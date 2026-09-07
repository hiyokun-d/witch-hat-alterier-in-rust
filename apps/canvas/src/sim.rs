//! The simulation, running inside the app.
//!
//! Thin, like every shell module has to be (§4.2): it owns a `magic_core::Sim`,
//! ticks it on a fixed schedule, and turns "the ink on the pad" into "cast every
//! spell it compiles to". Nothing here decides what magic means.
//!
//! # Why `FixedUpdate`
//!
//! §4.3 wants the same inputs to give the same outputs on every machine, and a
//! step whose length depends on how long the last frame took cannot. Bevy's
//! fixed schedule runs at a rate we set, catching up with as many ticks as it
//! needs, so a slow frame produces the same simulation as a fast one — just
//! later. The rate here and `SimRules::dt` in core must agree; a mismatch is a
//! bug, not a taste, so both are 60Hz.

use bevy::prelude::*;
use magic_core::compiler::{Driver, Spell, Warning};
use magic_core::sim::{
    CastOutcome, Field, Materials, Parcel, ReactionBook, Sim, SubstanceId, Vec2 as SimVec2,
};
use magic_core::{Catalog, CompileRules, SigilId, assembly};

use crate::InkPad;
use crate::reading::RULES;
use crate::ui::ToolState;

/// How wide the world is, in cells, and how big a cell is in pixels.
///
/// Fixed rather than fitted to the window: a field that resized itself would
/// make the same seal behave differently on two screens, which is the same
/// determinism argument as the fixed timestep.
const CELLS_ACROSS: usize = 48;
const CELL_SIZE: f32 = 24.0;

/// How close a stroke has to come to a ring to count as touching it. Pixels, so
/// it belongs to the shell (§4.4's units rule).
const ON_RING_TOLERANCE: f32 = crate::INK_WIDTH * 2.5;

/// A seal the watcher has already fired, so it does not fire again every frame.
///
/// Matched by where it is rather than by index, because the ring list is
/// rebuilt from the pad every frame and a ring added earlier in the list shifts
/// every index after it. Position and radius are what actually identify a seal
/// on the paper.
#[derive(Debug, Clone, Copy)]
pub struct Armed {
    pub center: SimVec2,
    pub radius: f32,
    /// Whether it was firing last time we looked.
    pub fired: bool,
}

/// How far a ring may drift between frames and still be the same ring.
const SAME_RING: f32 = 24.0;

/// The world, the catalogue it is cast from, and whether it is running.
#[derive(Resource)]
pub struct Simulation {
    pub sim: Sim,
    /// The catalogue. Its own copy rather than the overlay's, because §0 keeps
    /// `debug.rs` deletable and this must keep working when it is gone.
    pub lore: Option<Catalog>,
    pub running: bool,
    /// What the last cast did, for the panel's hint line.
    pub last: Option<String>,
    /// Every seal on the pad and whether it has already gone off.
    pub armed: Vec<Armed>,
    /// Where the pointer last was on the paper.
    ///
    /// So `pour` drops what you asked for *where you are looking*, rather than
    /// always in the middle of the world. Kept here rather than threaded
    /// through `ui::run`, which already takes as many arguments as anything
    /// should.
    pub focus: SimVec2,
}

impl Default for Simulation {
    fn default() -> Self {
        let mut world = Simulation {
            sim: Sim::new(
                Field::centered(CELLS_ACROSS, CELL_SIZE, SimVec2::ZERO)
                    .expect("a fixed, non-zero grid"),
            ),
            lore: Catalog::parse(
                include_str!("../../../crates/magic-core/the-magic-assets/sigils.ron"),
                include_str!("../../../crates/magic-core/the-magic-assets/signs.ron"),
                include_str!("../../../crates/magic-core/the-magic-assets/spells.ron"),
            )
            .ok(),
            running: false,
            last: None,
            armed: Vec::new(),
            focus: SimVec2::ZERO,
        };

        // The rules are data (§4.4) and core has no filesystem (§4.1), so the
        // shell is what hands them over — the same arrangement as the catalogue.
        world.sim.materials = Materials::parse(
            "materials.ron",
            include_str!("../../../crates/magic-core/the-magic-assets/materials.ron"),
        )
        .unwrap_or_default();
        world.sim.reactions = ReactionBook::parse(
            "reactions.ron",
            include_str!("../../../crates/magic-core/the-magic-assets/reactions.ron"),
        )
        .unwrap_or_default();
        world.fill_air();
        world
    }
}

impl Simulation {
    /// Compiles everything on the pad and casts each spell at its own ring.
    ///
    /// `compile_all` rather than `compile`, because canon rules 4, 5 and 6 are
    /// questions about several glyphs at once and one seal can never answer
    /// them about itself.
    pub fn cast_pad(&mut self, pad: &InkPad, tools: &ToolState) -> String {
        let Some(catalog) = self.lore.as_ref() else {
            return "catalogue failed to load".to_string();
        };

        let rings = assembly::find_rings(&pad.points, &assembly::RingSearch::default());
        if rings.is_empty() {
            return "no ring on the pad - nothing to cast".to_string();
        }

        let glyphs = assembly::glyphs(&rings, &pad.points, ON_RING_TOLERANCE, &RULES);
        let spells = magic_core::compile_all(&glyphs, catalog, &CompileRules::default());

        let mut fired = 0;
        let mut spawned = 0;
        let mut notes: Vec<String> = Vec::new();

        for (glyph, spell) in glyphs.iter().zip(&spells) {
            if let Some(why) = refuses(spell, tools.only_traced) {
                notes.push(why);
                continue;
            }
            let center = SimVec2::new(glyph.ring.center().x, glyph.ring.center().y);
            let report = self.sim.cast(spell, center);
            match &report.outcome {
                CastOutcome::Fired => {
                    fired += 1;
                    spawned += report.spawned;
                    if report.shortfall > 0.0 {
                        notes.push(format!("short {:.1}", report.shortfall));
                    }
                }
                CastOutcome::Discharged => {
                    fired += 1;
                    notes.push(format!("discharge shoved {}", report.pushed));
                }
                CastOutcome::NotFiring(firing) => notes.push(format!("{firing:?}")),
                CastOutcome::Cancelled => notes.push("cancelled by its twin".to_string()),
                CastOutcome::NothingFound { substance, .. } => {
                    notes.push(format!("no {substance} in reach"))
                }
            }
        }

        // Casting starts the clock. Nothing to look at otherwise.
        if fired > 0 {
            self.running = true;
        }

        let detail = if notes.is_empty() {
            String::new()
        } else {
            format!(" | {}", notes.join(", "))
        };
        format!(
            "cast {fired}/{} seal(s), {spawned} parcel(s){detail}",
            spells.len()
        )
    }
}

/// Why a compiled seal will not be cast, or `None` if it will.
///
/// **A tool policy, not a canon rule, and the distinction is load-bearing.**
/// Canon rule 9 is unambiguous — "if a ring is the only thing drawn, the spell
/// generated will simply be a rapid discharge of energy, i.e. an explosion" —
/// and `compiler.rs` still says exactly that. Core is not changed and the
/// discharge tests all still pass. What changed is that *this app* declines to
/// fire one.
///
/// The reason is the workshop rather than the fiction. Every ring that holds
/// ink the recogniser cannot read compiles to the same discharge as an empty
/// one, so while the vocabulary is four runes deep, "blast" is what almost
/// every misread drawing does — and a spell that goes off when you have drawn
/// something it could not understand teaches you nothing and buries the thing
/// you were trying to see. A seal fires when it was *understood*.
///
/// Turning `only_traced` off puts the discharge back, which is the honest way
/// to hold a canon rule and a working tool at the same time.
pub fn refuses(spell: &Spell, only_traced: bool) -> Option<String> {
    if !only_traced {
        return None;
    }
    match spell.driver {
        Driver::Discharge => Some(
            match spell.warnings.iter().find_map(|warning| match warning {
                Warning::Unreadable { strokes } => Some(*strokes),
                _ => None,
            }) {
                Some(strokes) => {
                    format!("{strokes} mark(s) in this ring are not a traced rune - nothing cast")
                }
                None => "bare ring - blast is off, put a traced sigil in it".to_string(),
            },
        ),
        _ => None,
    }
}

/// Everything that can be traced, sigils first and then signs.
///
/// One list rather than two buttons: tracing is a sitting, and stepping through
/// a single list is how a sitting works. The `kind` rides along because
/// `templates.ron` keeps the two vocabularies apart — nothing stops a sigil and
/// a sign sharing a name, and the catalogue would answer differently for each.
pub fn traceable(catalog: &Catalog) -> Vec<(String, &'static str)> {
    catalog
        .sigils()
        .map(|def| (def.id.as_str().to_string(), "Sigil"))
        .chain(
            catalog
                .signs()
                .map(|def| (def.id.as_str().to_string(), "Sign")),
        )
        .collect()
}

/// One line saying what a sigil actually does, from the catalogue.
///
/// The panel used to answer `sigil 7/34: aeriforms` and stop there, which tells
/// you which button you pressed and nothing about the magic. The catalogue has
/// known this all along — `affects` is what the sigil acts on and `caps` is what
/// it may do to it — and §3.2 makes the difference between *makes* and *moves*
/// the most load-bearing fact in the whole engine.
pub fn describe_sigil(catalog: &Catalog, id: &SigilId) -> String {
    let Some(def) = catalog.sigil(id) else {
        return format!("{} - not in the catalogue", id.as_str());
    };

    let acts_on = if def.affects.is_empty() {
        "no substance".to_string()
    } else {
        def.affects.join(", ")
    };
    let verb = match (def.caps.create, def.caps.move_, def.caps.manipulate) {
        (true, _, _) => "MAKES",
        (false, true, _) => "MOVES",
        (false, false, true) => "SHAPES",
        _ => "acts on",
    };
    // The refusal is the interesting half, so it is spelled out rather than
    // left as the absence of a word.
    let cannot = match (def.caps.create, def.caps.move_) {
        (false, true) => " (cannot create it)",
        (true, false) => " (cannot move it)",
        (false, false) => " (cannot create it)",
        _ => "",
    };
    format!("{} - {verb} {acts_on}{cannot}", id.as_str())
}

/// One line saying what a fixture does, from the catalogue's own effect text.
pub fn describe_fixture(catalog: &Catalog, id: &str) -> String {
    let Some(def) = catalog.spell(id) else {
        return format!("{id} - not in the catalogue");
    };
    // The effect is written across several lines in the data; the panel has one.
    let effect: String = def.effect.split_whitespace().collect::<Vec<_>>().join(" ");
    let signs = if def.signs.is_empty() {
        "no signs".to_string()
    } else {
        def.signs
            .iter()
            .map(|(sign, _)| sign.as_str())
            .collect::<Vec<_>>()
            .join("+")
    };
    format!(
        "{id} [{}] {signs} - {effect}",
        def.sigil
            .as_ref()
            .map_or("no sigil", |sigil| sigil.as_str())
    )
}

/// How many cells apart the ambient air parcels sit, and how heavy each is.
///
/// **The room has air in it.** This was the bug behind "nothing happens when I
/// cast a closed ring": canon rule 9 makes a bare ring an explosion, and an
/// explosion is *energy* — it shoves and heats what is already there and
/// creates nothing. In a vacuum that is, correctly, nothing at all. So the
/// world was working exactly as written and looked broken, because a room with
/// no air in it is not a room.
///
/// It fixes three things at once. A discharge now has something to throw. Wind
/// can find the air it is forbidden from creating (§3.2). And fire has fuel, so
/// `fan` and `burnout` become visible rather than theoretical.
///
/// Every second cell rather than every cell: 48x48 is 2304, and the medium only
/// has to be dense enough to carry a blast.
const AIR_SPACING: usize = 2;
const AIR_MASS: f32 = 0.5;

/// `(fixture id, what it does, keystones, ring left open, keystones aimed in)`
///
/// The open ones are canon rule 2 made into a tool: "leaving a gap prepares a
/// spell to be fired later by closing it". They land inert, and the last stroke
/// is yours — which is the moment the whole engine is built around and the one
/// One ready-made seal the panel can lay down.
///
/// A struct now rather than a five-field tuple, because it grew a sixth: which
/// **rune** goes in the middle. That was the fix for the thing that made a
/// preset dishonest — it used to stamp a generic mark and then *force* the
/// spell's name, which meant the one button guaranteed to produce a working
/// spell was also the one button that bypassed the recogniser entirely.
///
/// Every entry names a sigil, and `preset` refuses to place one whose sigil has
/// not been traced. A preset you cannot read is a preset that would fire
/// something you did not draw.
#[derive(Debug, Clone, Copy)]
pub struct Preset {
    /// The fixture in `spells.ron` this is built after, for the blurb.
    pub id: &'static str,
    /// The rune that goes in the middle. Must be traced or the preset is
    /// refused.
    pub sigil: &'static str,
    pub blurb: &'static str,
    /// How many keystones ring it.
    pub signs: usize,
    /// Canon rule 2: left with a gap, prepared rather than fired.
    pub open: bool,
    /// Canon's `AllInward` — arrows at the middle rather than away from it.
    pub inward: bool,
}

/// Every seal `preset` can lay down.
///
/// Only the five elemental sigils, because those are the five a person can
/// realistically trace and every one of them does something visible. The list
/// used to carry `everlasting`, built on the repetition sigil — a fine spell
/// and one nobody has a rune for, so it stamped a seal that could only ever
/// compile to a discharge.
pub const PRESETS: &[Preset] = &[
    Preset {
        id: "flamespout",
        sigil: "fire",
        blurb: "fire + four columns - a column of flame, straight up",
        signs: 4,
        open: false,
        inward: false,
    },
    Preset {
        id: "flamespout",
        sigil: "fire",
        blurb: "the same seal, PREPARED - close the gap yourself",
        signs: 4,
        open: true,
        inward: false,
    },
    Preset {
        id: "fire_shot",
        sigil: "fire",
        blurb: "fire + three signs - thrown the way they point",
        signs: 3,
        open: false,
        inward: false,
    },
    // Four arrows pointing at the middle. Canon's `AllInward`: "manifests only
    // inside the ring" — which is exactly how a ball of water differs from a
    // fountain of it. Same sigil, arrows reversed, opposite spell, and the
    // arrangement is doing all of the work.
    Preset {
        id: "water_orb",
        sigil: "water",
        blurb: "water + four arrows pointing IN - held as a ball",
        signs: 4,
        open: false,
        inward: true,
    },
    Preset {
        id: "water_orb",
        sigil: "water",
        blurb: "the orb, PREPARED - close it and it forms",
        signs: 4,
        open: true,
        inward: true,
    },
    Preset {
        id: "raincleaver",
        sigil: "water",
        blurb: "water, eight signs - Qifrey's cutting spell",
        signs: 8,
        open: false,
        inward: false,
    },
    Preset {
        id: "mistveil",
        sigil: "water",
        blurb: "water as a cloud - six signs, spread wide",
        signs: 6,
        open: false,
        inward: false,
    },
    Preset {
        id: "windrider",
        sigil: "wind",
        blurb: "wind - finds air, or does nothing at all",
        signs: 4,
        open: false,
        inward: false,
    },
    Preset {
        id: "earth_wall",
        sigil: "earth",
        blurb: "earth + arrows IN - gathers stone and packs it rigid",
        signs: 4,
        open: false,
        inward: true,
    },
    Preset {
        id: "lightfall",
        sigil: "light",
        blurb: "light - a lamp above the seal, not a beam",
        signs: 4,
        open: false,
        inward: false,
    },
];

/// Substances a person can drop by hand, to watch two of them meet.
///
/// Casting a spell is the real way to put matter in the world, and it needs a
/// named sigil to do it. This is the short way: reactions are the interesting
/// half of M7 and they should be reachable in two clicks, not four.
pub const POURABLE: &[(&str, f32)] = &[
    ("water", 20.0),
    ("flame", 400.0),
    ("air", 20.0),
    ("ice", -20.0),
    ("stone", 20.0),
    ("steam", 130.0),
];

impl Simulation {
    /// Fills the world with still air at room temperature.
    ///
    /// Called at startup and after `empty`, because an empty world is not a
    /// neutral starting state — it is a vacuum, and almost nothing in canon
    /// does anything in one.
    pub fn fill_air(&mut self) {
        let field = &mut self.sim.field;
        let (width, height) = (field.width(), field.height());
        for col in (0..width).step_by(AIR_SPACING) {
            for row in (0..height).step_by(AIR_SPACING) {
                let at = field.cell_center(col, row);
                field.add(Parcel::new(SubstanceId::new("air"), at, AIR_MASS));
            }
        }
        field.settle();
    }

    /// Drops a blob of one substance at `at`.
    pub fn pour(&mut self, which: usize, at: SimVec2) -> String {
        let Some((name, temperature)) = POURABLE.get(which) else {
            return "nothing to pour".to_string();
        };
        // A ring of parcels rather than one, so it lands in several cells and
        // has something to meet.
        for slot in 0..8 {
            let turn = std::f32::consts::TAU * slot as f32 / 8.0;
            let offset = SimVec2::from_angle(turn) * 18.0;
            let mut parcel = Parcel::new(SubstanceId::new(*name), at + offset, 2.0);
            parcel.temperature = *temperature;
            self.sim.field.add(parcel);
        }
        self.sim.field.settle();
        format!("poured {name} at {:.0} deg", temperature)
    }
}

/// Remembers where the pointer is, so `pour` lands under it.
fn track_focus(
    pointer: Res<crate::ui::Pointer>,
    tools: Option<Res<ToolState>>,
    mut world: ResMut<Simulation>,
) {
    if let (Some(at), false) = (pointer.at, pointer.over_ui) {
        world.focus = SimVec2::new(at.x, at.y);
    }
    // The panel owns the switch; the rules own the behaviour.
    if let Some(state) = tools.as_deref() {
        world.sim.rules.walls = state.walls;
    }
}

/// Fires a seal the moment its ring closes, and never twice for one closing.
///
/// **Canon rule 2 is a state machine, not an event.** An open ring is a
/// *prepared* spell; closing it activates it; opening it again disarms it. Rule
/// 3's glowstone path is the same mechanism with the two halves of one seal on
/// separate objects — step on them and the ring completes.
///
/// So this compares what is firing now against what was firing last frame and
/// casts on the rising edge only. Nothing listens for a "ring closed" event
/// because there is no such event to listen for: `assembly` recomputes closure
/// from the ink every frame, which is exactly what makes toggling free.
fn watch_the_pad(pad: Res<InkPad>, tools: Option<Res<ToolState>>, mut world: ResMut<Simulation>) {
    // Canon rule 2, taken literally: "a spell activates only when its ring is
    // complete. Leaving a gap prepares a spell to be fired later by closing
    // it." A button was always the wrong shape for that.
    if !tools.as_deref().is_none_or(|state| state.auto) {
        world.armed.clear();
        return;
    }

    // The compile phase borrows the catalogue; the cast phase mutates the
    // world. Scoped so the first is finished before the second begins, rather
    // than cloning a catalogue sixty times a second to dodge the borrow.
    let (next, fired, refused) = {
        let Some(catalog) = world.lore.as_ref() else {
            return;
        };

        let rings = assembly::find_rings(&pad.points, &assembly::RingSearch::default());
        if rings.is_empty() {
            world.armed.clear();
            return;
        }

        let glyphs = assembly::glyphs(&rings, &pad.points, ON_RING_TOLERANCE, &RULES);
        let spells = magic_core::compile_all(&glyphs, catalog, &CompileRules::default());
        let only_traced = tools.as_deref().is_none_or(|state| state.only_traced);

        let mut next: Vec<Armed> = Vec::with_capacity(spells.len());
        let mut fired: Vec<(magic_core::Spell, SimVec2)> = Vec::new();
        let mut refused: Vec<String> = Vec::new();

        for (glyph, spell) in glyphs.iter().zip(&spells) {
            let center = SimVec2::new(glyph.ring.center().x, glyph.ring.center().y);
            let radius = glyph.ring.radius();
            // **The edge is about the ring, not about the cast.** `fired`
            // tracks whether the circuit was closed last frame, so a seal that
            // closes and is then refused has still had its one moment — it does
            // not re-ask every frame, and re-opening the ring re-arms it.
            //
            // Getting this wrong the other way was the bug: a refused seal that
            // never set the flag would be refused sixty times a second, and a
            // refusal you cannot read because it is being reprinted is the same
            // as no refusal at all.
            let closed = spell.fires();
            let refusal = refuses(spell, only_traced);

            // Was this seal on the pad last frame, and had it already closed?
            let before = world
                .armed
                .iter()
                .find(|seal| {
                    (seal.center - center).length() < SAME_RING
                        && (seal.radius - radius).abs() < SAME_RING
                })
                .map(|seal| seal.fired)
                .unwrap_or(false);

            if closed && !before {
                match refusal {
                    None => fired.push((spell.clone(), center)),
                    // **Silence was the bug.** A ring that closes and does
                    // nothing, with nothing on screen saying why, is
                    // indistinguishable from a ring that closed and the app
                    // missed it — which is exactly how this was reported:
                    // "some of the particle won't run even tho the ring is
                    // closed". The circuit closed. The app declined. Say so.
                    Some(why) => refused.push(why),
                }
            }
            next.push(Armed {
                center,
                radius,
                fired: closed,
            });
        }
        (next, fired, refused)
    };

    world.armed = next;
    for (spell, center) in fired {
        let report = world.sim.cast(&spell, center);
        world.last = Some(describe(&report));
        world.running = true;
    }
    // After the casts, so a frame that both fired one seal and refused another
    // leaves the refusal on screen — that is the half you did not expect.
    if let Some(why) = refused.into_iter().next() {
        world.last = Some(format!("ring closed, nothing cast - {why}"));
    }
}

/// One cast, in a line a panel can print.
fn describe(report: &magic_core::sim::CastReport) -> String {
    match &report.outcome {
        CastOutcome::Fired => format!(
            "fired: {} parcel(s), {:.1} made, {:.1} found",
            report.spawned, report.created, report.found
        ),
        CastOutcome::Discharged => format!("discharge shoved {}", report.pushed),
        CastOutcome::NotFiring(firing) => format!("{firing:?}"),
        CastOutcome::Cancelled => "cancelled by its twin".to_string(),
        CastOutcome::NothingFound { substance, .. } => format!("no {substance} in reach"),
    }
}

/// Keeps the world the size of the window.
///
/// The field was a fixed 48x48 cells of 24px — 1152 square — against a 900px
/// tall window, so a third of it was off screen. The walls were working
/// perfectly and parcels were bouncing off a boundary nobody could see, which
/// is indistinguishable from them escaping.
///
/// Rebuilt rather than resized because `Field`'s cells are a flat `Vec` sized
/// at construction; the parcels move across and are confined into the new
/// bounds, so nothing is lost even when the window shrinks.
///
/// Cell *size* stays fixed. Only the count changes, so a spell behaves the same
/// on any screen — it simply has more or less room. A field that scaled its
/// cells would change what "one cell of density" means and quietly retune every
/// reaction with the window.
fn fit_world(window: Single<&Window>, mut world: ResMut<Simulation>) {
    let wide = ((window.width() / CELL_SIZE).ceil() as usize).max(8);
    let tall = ((window.height() / CELL_SIZE).ceil() as usize).max(8);
    if world.sim.field.width() == wide && world.sim.field.height() == tall {
        return;
    }

    let half = SimVec2::new(wide as f32 * CELL_SIZE * 0.5, tall as f32 * CELL_SIZE * 0.5);
    let Some(mut fresh) = Field::new(wide, tall, CELL_SIZE, SimVec2::ZERO - half) else {
        return;
    };
    for parcel in world.sim.field.parcels() {
        fresh.add(parcel.clone());
    }
    fresh.confine(world.sim.rules.restitution);
    fresh.settle();
    world.sim.field = fresh;
}

/// How the desk is lit, and by what.
///
/// A light spell that only draws bright dots is a light spell you cannot see —
/// the paper is already pale, so *more pale* reads as nothing. Light has to
/// change the room: the desk darkens as the spell takes hold, the light itself
/// burns against it, and the room comes back when the spell is spent.
///
/// **Ours** (§2.6), like every other number in the simulation. Canon says light
/// "manifests magic as light" and stops there.
#[derive(Resource, Debug)]
pub struct Lighting {
    /// How dark the room is now, `0.0` normal to `1.0` fully dimmed.
    pub dim: f32,
    /// How much light is in the world, smoothed.
    pub glow: f32,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            dim: 0.0,
            glow: 0.0,
        }
    }
}

/// Mass of light at which the room is as dark as it gets.
const LIGHT_FULL: f32 = 8.0;

/// How fast the room dims and recovers, per second. Dimming is quicker than
/// recovery on purpose: a light flaring should feel sudden and its absence
/// should feel like your eyes adjusting.
const DIM_RATE: f32 = 2.2;
const LIFT_RATE: f32 = 0.9;

/// Darkens the room while a light spell burns, and brings it back after.
///
/// **Both halves, and neither works alone.** The desk goes dark here; the halos
/// that make the lit part visible are `particles::draw_glow`. Dimming without
/// them is a room that goes dark for no reason, and halos without the dimming
/// are pale dots on parchment that read as nothing — which is exactly how the
/// first version of this failed.
///
/// The paper darkens too now, and that is a reversal worth stating: it was
/// deliberately left alone on the grounds that dimming it hides the ink. True,
/// and it also meant the brightest thing on screen during a light spell was the
/// *unlit* page, which is backwards. So it dims, but only part way — far less
/// than the desk, and floored well above black, because iron-gall ink on mid
/// parchment is still perfectly readable while ink on a dark page is not.
fn light_the_room(
    time: Res<Time>,
    world: Res<Simulation>,
    mut lighting: ResMut<Lighting>,
    mut clear: ResMut<ClearColor>,
    paper: Query<&MeshMaterial2d<ColorMaterial>, With<crate::Paper>>,
    mut palette: ResMut<Assets<ColorMaterial>>,
) {
    let dt = time.delta_secs();
    let lit = world
        .sim
        .field
        .mass_of(&SubstanceId::new("light"))
        .max(world.sim.field.mass_of(&SubstanceId::new("electricity")));

    let wanted = (lit / LIGHT_FULL).clamp(0.0, 1.0);
    // Toward the target at different speeds each way.
    let rate = if wanted > lighting.dim {
        DIM_RATE
    } else {
        LIFT_RATE
    };
    lighting.dim += (wanted - lighting.dim) * (rate * dt).clamp(0.0, 1.0);
    lighting.glow = lighting.dim;

    // The desk, not the paper: the paper is a mesh the app draws and dimming it
    // would hide the ink. Darkening what is *around* the paper is what makes a
    // light spell read as light without taking the drawing away.
    let deep = Color::srgb(0.03, 0.03, 0.05);
    clear.0 = crate::DESK.mix(&deep, lighting.dim);

    // A third of the way toward a dark parchment at full dim, never further.
    // The number is the whole compromise: enough that the page stops being the
    // brightest thing in a dark room, little enough that the drawing survives.
    let shaded = crate::PAPER.mix(&Color::srgb(0.18, 0.15, 0.13), lighting.dim * 0.55);
    for handle in &paper {
        if let Some(mut material) = palette.get_mut(&handle.0) {
            material.color = shaded;
        }
    }
}

/// Adds the simulation. Deleting this line and `mod sim;` leaves the pad, the
/// recognizer and the compiler working exactly as before.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Simulation>()
            // Must match `SimRules::dt`. Core cannot check it, so this line is
            // the whole guarantee.
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .init_resource::<Lighting>()
            .add_systems(Update, (fit_world, light_the_room))
            .add_systems(FixedUpdate, advance)
            // After capture, so a stroke that closes a ring fires on the frame
            // the pen lifts rather than the frame after.
            .add_systems(
                Update,
                (track_focus, watch_the_pad)
                    .chain()
                    .after(crate::capture_stroke),
            );
    }
}

fn advance(mut world: ResMut<Simulation>) {
    if world.running {
        world.sim.step();
    }
}

#[cfg(test)]
#[path = "tests/sim.rs"]
mod tests;
