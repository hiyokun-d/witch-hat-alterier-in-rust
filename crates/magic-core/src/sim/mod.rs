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
pub mod event;
pub mod field;
pub mod material;
pub mod parcel;
pub mod prop;
pub mod reaction;
pub mod step;
pub mod vec2;

pub use cast::{CastOutcome, CastReport, CastRules, cast};
pub use event::Event;
pub use field::{Cell, Field};
pub use material::{MaterialDef, Materials, Phase};
pub use parcel::{AMBIENT, Anchor, Parcel, SubstanceId};
pub use prop::{Prop, PropRules, scatter, step_props};
pub use reaction::{ReactionBook, ReactionDef, ReactionReport, react};
pub use step::{SimRules, step};
pub use vec2::Vec2;

use crate::compiler::Spell;

/// A spell still running: still summoning, second by second.
///
/// The difference between a firework and a fountain. A cast used to be one
/// burst and done, so "make a lot of water" meant pressing a button a lot —
/// and canon's own long-duration water spells, which *collect* rather than
/// create, had nothing in the model to be long about.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub spell: Spell,
    pub at: Vec2,
    /// Seconds of summoning left.
    pub left: f32,
}

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
    /// Spells still running. Emptied as each finishes.
    pub channels: Vec<Channel>,
    /// Things on the paper for a spell to act on. **Ours** (§2.6) — canon has
    /// no objects, and a spell with nothing to affect is invisible.
    pub props: Vec<Prop>,
    pub prop_rules: PropRules,
    /// What happened this tick, for whoever is drawing. Cleared at the top of
    /// every `step` and appended to as the tick runs, so a shell that reads it
    /// once a frame sees the latest tick and never a stale one.
    ///
    /// Nothing in the simulation reads it back — draining it changes no
    /// outcome, which is what makes it safe for a shell to ignore entirely.
    pub events: Vec<Event>,
    /// Parcels the maths ruined and `Field::scrub` had to replace, in total.
    /// Non-zero means a bug, and it should be on screen rather than hidden.
    pub scrubbed: usize,
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
            channels: Vec::new(),
            props: Vec::new(),
            prop_rules: PropRules::default(),
            events: Vec::new(),
            scrubbed: 0,
            ticks: 0,
        }
    }

    /// Simulated seconds elapsed. Derived from the tick count and the fixed
    /// step, never from a wall clock (§4.1).
    pub fn elapsed(&self) -> f32 {
        self.ticks as f32 * self.rules.dt
    }

    pub fn step(&mut self) {
        // Not cleared here. A shell renders at its own rate and a fixed step
        // can run twice between two frames, so clearing at the top of a tick
        // would silently drop the first tick's events — and a blast that is
        // over in one tick is exactly the thing that would go missing. The
        // consumer drains instead, and the cap below is what stops a shell that
        // never drains from growing without bound.
        const KEEP: usize = 512;
        if self.events.len() > KEEP {
            self.events.drain(..self.events.len() - KEEP);
        }
        // Motion first, then reactions: substances have to be *moved* into the
        // same cell before they can be said to have met there. Reacting first
        // would let a parcel react with wherever it used to be.
        // Summon, then move: a parcel raised this tick should be carried by
        // this tick's physics rather than hanging still for a frame.
        self.pour_channels();
        step(&mut self.field, &self.rules, &self.materials, self.ticks);
        self.last_reaction = react(&mut self.field, &self.reactions, self.rules.dt);
        // Every reaction is something a person watching would name, so it gets
        // an event too. `react` reports where it happened for exactly this.
        for shot in &self.last_reaction.fired {
            let product = self
                .reactions
                .rules()
                .iter()
                .find(|rule| rule.id == shot.rule)
                .and_then(|rule| rule.outputs.first())
                .map(|(name, _)| SubstanceId::new(name.clone()))
                .unwrap_or_else(|| SubstanceId::new(shot.rule.clone()));
            self.events.push(Event::Reacted {
                at: shot.at,
                product,
                mass: shot.mass,
            });
        }
        // Props after reactions: a plank should be judged against the world as
        // it ends the tick, so water that has just become steam is no longer
        // wetting it.
        let mut happened = std::mem::take(&mut self.events);
        step_props(
            &mut self.props,
            &mut self.field,
            &self.prop_rules,
            &self.materials,
            self.rules.dt,
            &mut happened,
        );
        self.events = happened;
        self.scrubbed += self.field.scrub();
        self.ticks += 1;
    }

    pub fn cast(&mut self, spell: &Spell, at: Vec2) -> CastReport {
        // The first burst lands now, so a cast is never invisible even if the
        // world is paused. The rest arrives tick by tick.
        let report = cast(spell, at, &mut self.field, &self.cast_rules);
        self.events.extend(announce(spell, at, &report));
        let left = cast::channel_for(spell, &self.cast_rules);
        if left > 0.0 {
            // One channel per seal, not one per press: casting the same seal
            // again refreshes it rather than stacking, which is what stops a
            // held button becoming an ocean.
            match self
                .channels
                .iter_mut()
                .find(|open| (open.at - at).length() < 1.0)
            {
                Some(open) => {
                    open.spell = spell.clone();
                    open.left = left;
                }
                None => self.channels.push(Channel {
                    spell: spell.clone(),
                    at,
                    left,
                }),
            }
        }
        report
    }

    /// What every running spell adds this tick.
    ///
    /// Each channel casts again at `dt`-scaled strength, so the *rate* is what
    /// the seal decides and the timestep only decides how finely it is chopped.
    /// Halve `dt` and you get twice as many casts of half the size — the same
    /// water, which is what §4.3 requires of anything that runs on a clock.
    fn pour_channels(&mut self) {
        let dt = self.rules.dt;
        let mut trickle = self.cast_rules;
        trickle.mass_per_strength *= dt;
        // Fewer, smaller parcels per tick: a channel that emitted the seal's
        // full count sixty times a second would bury the field in a moment.
        trickle.base_parcels = 1;
        trickle.parcels_per_sign = 0;

        let running = std::mem::take(&mut self.channels);
        self.channels = running
            .into_iter()
            .filter_map(|mut open| {
                cast(&open.spell, open.at, &mut self.field, &trickle);
                open.left -= dt;
                (open.left > 0.0).then_some(open)
            })
            .collect();
    }

    /// The mass standing on the paper, which `Field::mass` cannot see.
    ///
    /// A conservation reading has to add the two: burning moves mass out of a
    /// prop and into the field, so watching only one of them shows a world that
    /// invents matter or one that loses it, and neither is what happened.
    pub fn prop_mass(&self) -> f32 {
        self.props.iter().map(|prop| prop.standing()).sum()
    }

    pub fn reset(&mut self) {
        self.field.clear();
        self.props.clear();
        self.events.clear();
        self.last_reaction = ReactionReport::default();
        self.channels.clear();
        self.scrubbed = 0;
        self.ticks = 0;
    }
}

/// Turns a cast's report into the events a shell would want to draw.
///
/// Here rather than in `cast` because it is a *translation*, not a decision:
/// `CastReport` already says everything, and this only says it in the vocabulary
/// a renderer speaks. Keeping `cast` free of it also keeps every conservation
/// test written against the report rather than against a side effect.
fn announce(spell: &Spell, at: Vec2, report: &CastReport) -> Vec<Event> {
    let strength = spell.strength();
    match &report.outcome {
        CastOutcome::Discharged => vec![Event::Blast { at, strength }],
        CastOutcome::NothingFound { substance, .. } => vec![Event::Refused {
            at,
            substance: SubstanceId::new(substance.clone()),
        }],
        CastOutcome::Fired => {
            let mass = report.created + report.found;
            if mass <= 0.0 {
                return Vec::new();
            }
            let substance = spell
                .demand
                .as_ref()
                .and_then(|demand| demand.substance.first())
                .map(|name| SubstanceId::new(name.clone()))
                .unwrap_or_else(|| SubstanceId::new("air"));
            let mut out = vec![Event::Summon {
                at,
                substance: substance.clone(),
                mass,
                created: report.created > 0.0,
            }];
            // Light is the one substance whose effect is on the room rather
            // than on the place it appeared, so it gets its own event and the
            // shell can dim for it without inspecting substance names.
            if substance.as_str() == "light" || substance.as_str() == "electricity" {
                out.push(Event::Flash { at, strength });
            }
            out
        }
        _ => Vec::new(),
    }
}

#[cfg(test)]
#[path = "../tests/sim.rs"]
mod tests;
