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
use magic_core::sim::{CastOutcome, Field, Sim, Vec2 as SimVec2};
use magic_core::{Catalog, CompileRules, assembly};

use crate::InkPad;

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
        Simulation {
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
        }
    }
}

impl Simulation {
    /// Compiles everything on the pad and casts each spell at its own ring.
    ///
    /// `compile_all` rather than `compile`, because canon rules 4, 5 and 6 are
    /// questions about several glyphs at once and one seal can never answer
    /// them about itself.
    pub fn cast_pad(&mut self, pad: &InkPad) -> String {
        let Some(catalog) = self.lore.as_ref() else {
            return "catalogue failed to load".to_string();
        };

        let rings = assembly::find_rings(&pad.points, &assembly::RingSearch::default());
        if rings.is_empty() {
            return "no ring on the pad - nothing to cast".to_string();
        }

        let glyphs = assembly::glyphs(&rings, &pad.points, ON_RING_TOLERANCE);
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
