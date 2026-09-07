//! The simulation: what a compiled spell actually *does*.
//!
//! M5 ended with a `Spell` carrying everything a simulation could want — what
//! drives it, how strongly, which way it leans, how much of that is spin, what
//! substance it demands and whether it must find it. Nothing read any of it.
//! This module is what reads it.
//!
//! # What is canon here and what is ours
//!
//! §2.6 is explicit that canon has no mechanism behind magic, so a continuous
//! world with gravity and heat is **our extension** and every constant in
//! [`SimRules`] and [`CastRules`] is a number canon does not give.
//!
//! What is canon is the *shape*: a bare ring discharges rather than doing
//! nothing (rule 9), a sigil that cannot create has to find its substance
//! (§3.2), a repetition spell returns its target to the state it held when the
//! spell took hold (§2.2), and where magic manifests is one of four cases
//! decided by the region signs (§2.3). Each of those is a branch you can point
//! at in this module.
//!
//! # Determinism (§4.3)
//!
//! No clock, no randomness, no map iteration. A fixed timestep, parcels held in
//! a `Vec` and walked in slice order, and every sum that could depend on order
//! either sorted first or documented as safe because the order is fixed.

pub mod cast;
pub mod field;
pub mod material;
pub mod parcel;
pub mod reaction;
pub mod step;
pub mod vec2;

pub use cast::{CastOutcome, CastReport, CastRules, cast};
pub use field::{Cell, Field};
pub use material::{MaterialDef, Materials, Phase};
pub use parcel::{AMBIENT, Anchor, Parcel, SubstanceId};
pub use reaction::{ReactionBook, ReactionDef, ReactionReport, react};
pub use step::{SimRules, step};
pub use vec2::Vec2;

use crate::compiler::Spell;

/// A world, its rules, and how far it has run.
///
/// Thin on purpose: everything interesting is in [`step`] and [`cast`], and this
/// exists so a shell has one thing to own and one thing to tick.
#[derive(Debug, Clone, PartialEq)]
pub struct Sim {
    pub field: Field,
    pub rules: SimRules,
    pub cast_rules: CastRules,
    /// What each substance is, physically. Empty by default and filled by
    /// whoever loaded the file — the same arrangement as the reaction rules.
    /// With it empty every substance falls back to a middling liquid, which is
    /// obviously *something* rather than silently nothing.
    pub materials: Materials,
    /// The reaction rules. Empty by default and filled by whoever loaded the
    /// file — core has no filesystem (§4.1), exactly as with `Catalog`.
    pub reactions: ReactionBook,
    /// What the last tick's reactions did, for a readout.
    pub last_reaction: ReactionReport,
    /// Ticks run. Not a clock — a count, so §4.1 still holds and a replay of
    /// the same inputs lands on the same number.
    pub ticks: u64,
}

impl Sim {
    pub fn new(field: Field) -> Sim {
        Sim {
            field,
            rules: SimRules::default(),
            cast_rules: CastRules::default(),
            materials: Materials::default(),
            reactions: ReactionBook::default(),
            last_reaction: ReactionReport::default(),
            ticks: 0,
        }
    }

    /// Simulated seconds elapsed. Derived from the tick count and the fixed
    /// step, never from a wall clock (§4.1).
    pub fn elapsed(&self) -> f32 {
        self.ticks as f32 * self.rules.dt
    }

    pub fn step(&mut self) {
        // Motion first, then reactions: substances have to be *moved* into the
        // same cell before they can be said to have met there. Reacting first
        // would let a parcel react with wherever it used to be.
        step(&mut self.field, &self.rules, &self.materials, self.ticks);
        self.last_reaction = react(&mut self.field, &self.reactions, self.rules.dt);
        self.ticks += 1;
    }

    pub fn cast(&mut self, spell: &Spell, at: Vec2) -> CastReport {
        cast(spell, at, &mut self.field, &self.cast_rules)
    }

    pub fn reset(&mut self) {
        self.field.clear();
        self.last_reaction = ReactionReport::default();
        self.ticks = 0;
    }
}

#[cfg(test)]
#[path = "../tests/sim.rs"]
mod tests;
