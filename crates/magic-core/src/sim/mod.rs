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
pub mod parcel;
pub mod step;
pub mod vec2;

pub use cast::{CastOutcome, CastReport, CastRules, cast};
pub use field::{Cell, Field};
pub use parcel::{AMBIENT, Anchor, Parcel, SubstanceId};
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
            ticks: 0,
        }
    }

    /// Simulated seconds elapsed. Derived from the tick count and the fixed
    /// step, never from a wall clock (§4.1).
    pub fn elapsed(&self) -> f32 {
        self.ticks as f32 * self.rules.dt
    }

    pub fn step(&mut self) {
        step(&mut self.field, &self.rules);
        self.ticks += 1;
    }

    pub fn cast(&mut self, spell: &Spell, at: Vec2) -> CastReport {
        cast(spell, at, &mut self.field, &self.cast_rules)
    }

    pub fn reset(&mut self) {
        self.field.clear();
        self.ticks = 0;
    }
}

#[cfg(test)]
#[path = "../tests/sim.rs"]
mod tests;
