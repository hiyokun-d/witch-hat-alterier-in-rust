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
use magic_core::catalog::Slot;
use magic_core::sim::{
    CastOutcome, Field, Materials, Parcel, ReactionBook, Sim, SubstanceId, Vec2 as SimVec2,
};
use magic_core::{Catalog, CompileRules, Glyph, SigilId, Sign, assembly};

use crate::InkPad;
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

        let mut glyphs = assembly::glyphs(&rings, &pad.points, ON_RING_TOLERANCE);
        apply_naming(&mut glyphs, catalog, tools);
        let spells = magic_core::compile_all(&glyphs, catalog, &CompileRules::default());

        let mut fired = 0;
        let mut spawned = 0;
        let mut notes: Vec<String> = Vec::new();

        for (glyph, spell) in glyphs.iter().zip(&spells) {
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

/// Every sigil the catalogue knows, in its own order.
///
/// Collected fresh rather than cached: the panel cycles through it a few times
/// a session, and a cached list is one more thing that can disagree with the
/// data it came from.
pub fn sigil_names(catalog: &Catalog) -> Vec<SigilId> {
    catalog.sigils().map(|def| def.id.clone()).collect()
}

/// Every canon spell fixture `spells.ron` records.
pub fn fixture_names(catalog: &Catalog) -> Vec<String> {
    catalog.spells().map(|def| def.id.clone()).collect()
}

/// Names the seals on the pad, because nothing else can yet.
///
/// **A testing override, and it must stay one.** `templates.ron` ships empty
/// because the rune shapes belong to the manga and §2 forbids inventing them,
/// so the recognizer names nothing and every seal compiles to rule 9's
/// discharge. That leaves the entire compiler unreachable from the app. Saying
/// "treat this ring as fire" is not a claim about what a fire sigil looks like;
/// it is the difference between a compiler you can use and one you can only
/// read tests about.
///
/// Returns what it applied, for the hint line.
pub fn apply_naming(glyphs: &mut [Glyph], catalog: &Catalog, tools: &ToolState) -> Option<String> {
    if let Some(index) = tools.fixture {
        let fixtures: Vec<_> = catalog.spells().cloned().collect();
        let def = fixtures.get(index)?;

        for glyph in glyphs.iter_mut() {
            glyph.sigil = def.sigil.clone();
            glyph.signs = fixture_signs(def, glyph.ring.radius());
            // A named seal is a read seal. Leaving the count would have the
            // compiler warn about marks it has just been told the meaning of.
            glyph.unnamed = 0;
        }
        return Some(format!("spell {}: {}", def.id, def.effect));
    }

    let index = tools.sigil?;
    let names = sigil_names(catalog);
    let id = names.get(index)?;
    for glyph in glyphs.iter_mut() {
        glyph.sigil = Some(id.clone());
        glyph.unnamed = glyph.unnamed.saturating_sub(1);
    }
    Some(format!("sigil {}", id.as_str()))
}

/// The signs a fixture calls for, spread evenly around its ring.
///
/// Canon constrains *which* signs a spell has and roughly where (`Slot`), never
/// the exact angles, so even spacing is the honest reading: §2.4's own advice is
/// bilateral symmetry, and evenly spaced signs of equal size sum to zero drift,
/// which is what canon says a balanced seal does.
fn fixture_signs(def: &magic_core::catalog::SpellDef, radius: f32) -> Vec<Sign> {
    let mut wanted: Vec<(magic_core::SignId, Slot)> = Vec::new();
    for (id, slot) in &def.signs {
        let count = def.sign_counts.get(id).copied().unwrap_or(1).max(1);
        for _ in 0..count {
            wanted.push((id.clone(), *slot));
        }
    }

    let total = wanted.len().max(1) as f32;
    wanted
        .iter()
        .enumerate()
        .map(|(i, (id, slot))| {
            let placement = std::f32::consts::TAU * i as f32 / total;
            Sign {
                kind: id.clone(),
                placement,
                // Radial: no tilt, so `spin` reads zero and the seal is
                // balanced unless the fixture's own signs make it otherwise.
                orientation: placement,
                // A tenth of the ring, in the ring's own units, so intensity
                // and balance stay comparable between fixtures.
                size: (radius * 0.1).max(1.0),
                reversed: def.reversed_signs.contains(id) || matches!(slot, Slot::Center) && false,
            }
        })
        .collect()
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

/// Seals the panel can lay down whole, so nothing has to be drawn by hand.
///
/// Each is a fixture id from `spells.ron` and the arrangement to ink for it.
/// Drawing a clean four-sign seal is a minute of careful work every single time
/// you want to test one, and needing that minute before the engine will do
/// anything is a bad way to build the engine — the same argument that put the
/// `place` tools on the panel in the first place.
pub const PRESETS: &[(&str, &str, usize)] = &[
    (
        "flamespout",
        "fire, four columns - shoots a column of flame up",
        4,
    ),
    (
        "watershot_seal",
        "water, four columns - the first spell Coco learned",
        4,
    ),
    (
        "raincleaver",
        "water, many signs - Qifrey's cutting spell",
        8,
    ),
    (
        "everlasting",
        "repetition - holds what it raises where it was raised",
        3,
    ),
    ("windrider", "wind - finds air or does nothing at all", 4),
    ("mistveil", "water as a cloud", 6),
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
    /// The fixture id a preset names, if the catalogue has it.
    pub fn preset_fixture(&self, which: usize) -> Option<usize> {
        let (id, _, _) = PRESETS.get(which)?;
        fixture_names(self.lore.as_ref()?)
            .iter()
            .position(|found| found == id)
    }

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

/// Adds the simulation. Deleting this line and `mod sim;` leaves the pad, the
/// recognizer and the compiler working exactly as before.
pub struct SimPlugin;

impl Plugin for SimPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Simulation>()
            // Must match `SimRules::dt`. Core cannot check it, so this line is
            // the whole guarantee.
            .insert_resource(Time::<Fixed>::from_hz(60.0))
            .add_systems(FixedUpdate, advance);
    }
}

fn advance(mut world: ResMut<Simulation>) {
    if world.running {
        world.sim.step();
    }
}
