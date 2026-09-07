//! What is on the pad, worked out once a frame.
//!
//! Reading the pad is not cheap: `find_rings` fits a circle to every stroke by
//! Taubin, measures winding and coverage on the survivors, then asks each ring
//! what it contains. `compile_all` follows. Doing that once is fine — the
//! problem was that **eight** places were each doing it for themselves every
//! frame: the inspector, the fit line, the ring captions, the recogniser board,
//! the fit drawing, the auto-cast watcher, and `cast_pad` twice over.
//!
//! At four hundred points that is the difference between drawing feeling
//! immediate and drawing feeling broken, which is exactly how it was described.
//!
//! So it is read here, once, and everyone reads the answer. Nothing recomputes
//! unless the ink or the naming actually changed — and the cheap way to know
//! that is to remember what the pad looked like, not to trust a `Changed`
//! filter that fires on any write.
//!
//! # Why the thresholds live here
//!
//! §4.4's units rule: anything measured in **pixels** is a fact about this
//! screen and this pen, so it belongs to the shell. `RingSearch` is all pixels.
//! `RingRules` is dimensionless and belongs to core, but *choosing* the numbers
//! is still ours, so the chosen ones sit beside the search that uses them.

use bevy::prelude::*;
use magic_core::{Catalog, CompileRules, Glyph, Recorded, Spell, assembly, naming, templates};

use crate::InkPad;
use crate::ui::ToolState;

/// How close two ends must come to count as joined. Rule 3 is physical about
/// it: the glowstone halves complete a spell when they *touch*.
pub const CLOSURE_TOLERANCE: f32 = crate::INK_WIDTH * 4.0;

/// How close ink must come to a fitted circle to be part of that ring, and how
/// close a mark must come to count as *connecting to* it (canon rule 1).
pub const ON_RING_TOLERANCE: f32 = crate::INK_WIDTH * 2.5;

/// What a ring must satisfy to close a circuit.
///
/// Every number here is a **ratio**, which is why the type is core's to hold: a
/// ratio means the same thing on any screen, where a pixel does not. Choosing
/// the values is still ours — `min_quality` is the one canon declines to give,
/// so it sits beside the pixel thresholds that use it rather than pretending to
/// be canon somewhere else.
///
/// `simple_tolerance` is how far turning may disagree with coverage before the
/// ink is not a simple arc at all.
pub const RULES: assembly::RingRules = assembly::RingRules {
    simple_tolerance: 0.08,
    min_quality: 0.95,
};

/// The search, in this screen's pixels.
pub fn search() -> assembly::RingSearch {
    assembly::RingSearch {
        on_ring: ON_RING_TOLERANCE,
        join: CLOSURE_TOLERANCE,
        ..default()
    }
}

/// Enough of the pad's state to know whether the answer can be reused.
///
/// Point count and stroke id together catch every edit: drawing adds points,
/// undo removes them, clearing resets both. The naming is in here because it
/// changes what the *same* ink compiles to.
#[derive(Debug, Default, PartialEq, Clone, Copy)]
struct Stamp {
    points: usize,
    stroke_id: u32,
    only_traced: bool,
}

/// The pad, read: its rings, the glyphs they make, and what those compile to.
///
/// The three lists are index-aligned — one glyph and one spell per ring — so a
/// caller holding a ring index can ask any of them.
#[derive(Resource, Default)]
pub struct Reading {
    pub rings: Vec<assembly::RingCandidate>,
    pub glyphs: Vec<Glyph>,
    pub spells: Vec<Spell>,
    /// Marks that belong to no ring, and what the recognizer made of them.
    ///
    /// Canon rule 1: ink outside every ring contributes nothing, and §3.3 is
    /// explicit that such strokes are *inert, not invalid* — a player halfway
    /// through a seal has drawn nothing wrong. But inert should not mean
    /// invisible: knowing the app read your fire sigil correctly is exactly
    /// what you want *before* committing to a ring around it.
    pub loose: Vec<(magic_core::Vec2, String)>,
    /// The shapes the recognizer matches against — **the only such list in the
    /// app**.
    ///
    /// There used to be two. `debug.rs` kept its own `Runes` resource holding
    /// the shipped file plus the recorder's, and this one held those *plus* the
    /// built-in reconstructions. Two lists, ranked separately, so the board and
    /// the compiler were never scoring against the same vocabulary — the board
    /// would call a drawing `earth` while the seal beneath it compiled to
    /// something else, and neither was wrong about its own list. A readout that
    /// does not describe the thing it sits next to is worse than no readout.
    pub shapes: Vec<Recorded>,
    /// Where each shape came from, index-aligned with `shapes`: `recorded`,
    /// `traced` or `built-in`. What the board prints in its source column, and
    /// what `traced_only` filters on.
    pub sources: Vec<&'static str>,
    /// What happened to the recorder's file last time it was read. A count, or
    /// why not.
    pub note: String,
    stamp: Option<Stamp>,
    /// Whether built-in sigils were left out last time the shapes were loaded.
    /// Part of the stamp: changing the switch has to force a reload.
    only_traced: bool,
    /// When the recorder's file was last seen. `None` means never looked,
    /// which is not the same as looked and found nothing.
    #[cfg(not(target_arch = "wasm32"))]
    traced: Option<Option<std::time::SystemTime>>,
}

impl Reading {
    /// The spell for a ring, if there is one.
    pub fn spell(&self, slot: usize) -> Option<&Spell> {
        self.spells.get(slot)
    }

    /// Every shape and where it came from, for a readout.
    pub fn catalogued(&self) -> impl Iterator<Item = (&'static str, &Recorded)> {
        self.sources.iter().copied().zip(self.shapes.iter())
    }

    /// Whether a rune of this name was actually traced by hand.
    ///
    /// The question the cast gate asks. A built-in reconstruction is a stand-in
    /// (§12) and casting one by accident is exactly what it should not allow.
    pub fn is_traced(&self, name: &str) -> bool {
        self.catalogued()
            .any(|(source, rune)| source != "built-in" && rune.template.name == name)
    }

    /// The traced sigils, by name, in catalogue order and without repeats.
    pub fn traced_sigils(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (source, rune) in self.catalogued() {
            if source == "built-in" || rune.kind != templates::Kind::Sigil {
                continue;
            }
            if !out.iter().any(|had| had == &rune.template.name) {
                out.push(rune.template.name.clone());
            }
        }
        out
    }
}

/// Rereads the pad when, and only when, something about it changed.
pub fn read_pad(
    pad: Res<InkPad>,
    tools: Option<Res<ToolState>>,
    lore: Option<Res<crate::sim::Simulation>>,
    mut reading: ResMut<Reading>,
) {
    let now = Stamp {
        points: pad.points.len(),
        stroke_id: pad.stroke_id,
        only_traced: tools.as_deref().is_none_or(|state| state.only_traced),
    };
    if reading.stamp == Some(now) {
        return;
    }
    reading.stamp = Some(now);

    load_shapes(&mut reading, now.only_traced);

    reading.rings = assembly::find_rings(&pad.points, &search());
    reading.glyphs = assembly::glyphs(&reading.rings, &pad.points, ON_RING_TOLERANCE, &RULES);

    // The step that makes the whole compiler reachable from a drawing: ink
    // becomes a sigil and some signs. `assembly` deliberately names nothing —
    // identifying a mark is the recognizer's job, and this is where the two
    // finally meet.
    // Destructured so the three fields are borrowed separately: the glyphs are
    // being written while the rings and shapes are being read, and borrowing
    // the whole resource at once cannot express that.
    let Reading {
        glyphs,
        rings,
        shapes,
        ..
    } = &mut *reading;
    for (glyph, ring) in glyphs.iter_mut().zip(rings.iter()) {
        naming::name(glyph, ring, &pad.points, shapes, ON_RING_TOLERANCE);
    }

    let catalog: Option<&Catalog> = lore.as_deref().and_then(|world| world.lore.as_ref());
    let Some(catalog) = catalog else {
        reading.spells.clear();
        return;
    };
    reading.loose = loose_marks(&pad, &reading);

    // No override any more. The drawing speaks for itself, always — which it
    // could not while `templates.ron` was empty, and can now that runes are
    // traced. See `sim::refuses` for what happens to a seal nobody can read.
    reading.spells = magic_core::compile_all(&reading.glyphs, catalog, &CompileRules::default());
}

/// Adds the reading. Everything that looks at the pad depends on it.
pub struct ReadingPlugin;

impl Plugin for ReadingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Reading>()
            .add_systems(Update, read_pad.after(crate::capture_stroke));
    }
}

/// Names every mark that belongs to no ring.
///
/// Uses the same clustering and the same threshold as `naming` does inside a
/// ring, so a mark does not change its mind about what it is the moment a ring
/// is drawn round it — which would be the worst possible behaviour for a
/// readout whose whole job is to promise the shape was understood.
fn loose_marks(pad: &InkPad, reading: &Reading) -> Vec<(magic_core::Vec2, String)> {
    if pad.points.is_empty() || reading.shapes.is_empty() {
        return Vec::new();
    }

    // Everything any ring has a claim on: its own strokes, and what it holds.
    let mut spoken_for: Vec<u32> = Vec::new();
    for ring in &reading.rings {
        spoken_for.extend(ring.strokes.iter().copied());
        let held = ring.contents(&pad.points, ON_RING_TOLERANCE);
        spoken_for.extend(held.inside.iter().copied());
        spoken_for.extend(held.touching.iter().copied());
    }

    let free: Vec<u32> = pad
        .points
        .chunk_by(|a, b| a.stroke_id == b.stroke_id)
        .filter_map(|run| run.first().map(|p| p.stroke_id))
        .filter(|id| !spoken_for.contains(id))
        .collect();
    if free.is_empty() {
        return Vec::new();
    }

    naming::loose(&pad.points, &free, &reading.shapes)
        .into_iter()
        .map(|(at, said)| (magic_core::Vec2::new(at.0, at.1), said))
        .collect()
}

/// Loads the shapes the recognizer matches against, newest source first.
///
/// **Three sources, and the order is the whole point.**
///
/// 1. `recorded-gesture.ron` — what you have traced *in this app*, reread
///    whenever the file changes so a rune recorded thirty seconds ago is
///    already being matched.
/// 2. `the-magic-assets/templates.ron` — traced runes made permanent. Ships
///    empty (§2: the shapes belong to the manga and have to be traced).
/// 3. `shapes::built_in` — the reconstructions, so the engine is reachable at
///    all before anybody has traced anything.
///
/// Each source only fills gaps the ones before it left, so a traced rune always
/// outranks the built-in of the same name (§12).
///
/// **`only_traced` drops the built-in *sigils*, and keeps the built-in signs.**
/// The asymmetry is the point rather than a compromise. A sigil decides *what*
/// the magic is, so a reconstruction being mistaken for one means casting fire
/// when you drew nothing of the sort — and §12 is explicit that these shapes
/// are stand-ins, reconstructions of a community project's reconstructions,
/// which is not a thing to fire a spell on by accident. A sign decides what
/// *shape* the magic takes, and a misread keystone gives you the wrong spout,
/// not the wrong element. Keeping them is also what lets a seal be built at all
/// before anybody has sat down to trace forty-four keystones.
fn load_shapes(reading: &mut Reading, only_traced: bool) {
    let n = magic_core::stroke::MATCH_POINTS;

    #[cfg(not(target_arch = "wasm32"))]
    let (live, note) = {
        let now = std::fs::metadata(crate::ui::record::OUTFILE)
            .and_then(|meta| meta.modified())
            .ok();
        let fresh = reading.traced != Some(now) || reading.only_traced != only_traced;
        if !fresh && !reading.shapes.is_empty() {
            return;
        }
        reading.traced = Some(now);
        match std::fs::read_to_string(crate::ui::record::OUTFILE) {
            Err(_) => (Vec::new(), "not written yet".to_string()),
            Ok(source) => match templates::parse("recorded-gesture.ron", &source, n) {
                Ok(found) => {
                    let note = format!("{} sample(s)", found.len());
                    (found, note)
                }
                // The message says what fixes it. A parser dump is true,
                // unreadable, and not actionable.
                Err(_) => (
                    Vec::new(),
                    "unreadable - press record to rewrite it".to_string(),
                ),
            },
        }
    };
    #[cfg(target_arch = "wasm32")]
    let (live, note) = {
        if !reading.shapes.is_empty() && reading.only_traced == only_traced {
            return;
        }
        (Vec::new(), "the browser has no files to watch".to_string())
    };
    reading.only_traced = only_traced;
    reading.note = note;

    let taken = |have: &[Recorded], rune: &Recorded| {
        have.iter()
            .any(|had| had.kind == rune.kind && had.template.name == rune.template.name)
    };

    let mut shapes = live;
    let mut sources: Vec<&'static str> = vec!["recorded"; shapes.len()];

    let permanent = templates::parse(
        "templates.ron",
        include_str!("../../../crates/magic-core/the-magic-assets/templates.ron"),
        n,
    )
    .unwrap_or_default();
    for rune in permanent {
        if !taken(&shapes, &rune) {
            shapes.push(rune);
            sources.push("traced");
        }
    }

    for rune in magic_core::templates::built_ins(n) {
        if only_traced && rune.kind == templates::Kind::Sigil {
            continue;
        }
        if !taken(&shapes, &rune) {
            shapes.push(rune);
            sources.push("built-in");
        }
    }

    reading.shapes = shapes;
    reading.sources = sources;
}
