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
    sigil: Option<usize>,
    fixture: Option<usize>,
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
    /// The shapes the recognizer matches against: whatever `templates.ron`
    /// holds, then the built-in reconstructions as a fallback.
    pub shapes: Vec<Recorded>,
    stamp: Option<Stamp>,
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
        sigil: tools.as_deref().and_then(|state| state.sigil),
        fixture: tools.as_deref().and_then(|state| state.fixture),
    };
    if reading.stamp == Some(now) {
        return;
    }
    reading.stamp = Some(now);

    load_shapes(&mut reading);

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

    // The panel's override still wins where it is set — it is how a seal gets
    // named as a spell fixture, and how anything the recognizer cannot read yet
    // can still be tested. Where it is not set, the drawing speaks for itself.
    if let Some(state) = tools.as_deref() {
        crate::sim::apply_naming(&mut reading.glyphs, catalog, state);
    }
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
/// `with_built_ins` skips a name already taken, so each source only fills gaps
/// the ones before it left.
///
/// **This was the bug.** Only the shipped file was read, and it is empty — so
/// every rune a person traced went into `recorded-gesture.ron` and was seen by
/// nothing but the recogniser board. A water sigil drawn perfectly still
/// compiled to canon rule 9's discharge, because the thing that names ink had
/// never been shown the shapes.
fn load_shapes(reading: &mut Reading) {
    let n = magic_core::stroke::MATCH_POINTS;

    #[cfg(not(target_arch = "wasm32"))]
    let live = {
        let now = std::fs::metadata(crate::ui::record::OUTFILE)
            .and_then(|meta| meta.modified())
            .ok();
        if reading.traced == Some(now) && !reading.shapes.is_empty() {
            return;
        }
        reading.traced = Some(now);
        std::fs::read_to_string(crate::ui::record::OUTFILE)
            .ok()
            .and_then(|source| templates::parse("recorded-gesture.ron", &source, n).ok())
            .unwrap_or_default()
    };
    #[cfg(target_arch = "wasm32")]
    let live = {
        if !reading.shapes.is_empty() {
            return;
        }
        Vec::new()
    };

    let mut shapes = live;
    let permanent = templates::parse(
        "templates.ron",
        include_str!("../../../crates/magic-core/the-magic-assets/templates.ron"),
        n,
    )
    .unwrap_or_default();
    for rune in permanent {
        let taken = shapes
            .iter()
            .any(|have| have.kind == rune.kind && have.template.name == rune.template.name);
        if !taken {
            shapes.push(rune);
        }
    }

    reading.shapes = templates::with_built_ins(shapes, n);
}
