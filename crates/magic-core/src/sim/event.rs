//! Things that *happened* this tick, for whoever is drawing the world.
//!
//! # Why core owns these and not the pixels
//!
//! §4.2 keeps every decision about what magic means in core and every decision
//! about how it looks in the shell. "A prop caught fire" is the first of those;
//! "an ignition throws eleven orange sparks that live half a second" is the
//! second. Without this list the shell can only draw *state* — where the mass
//! is, how hot — and state cannot show a moment. A blast is over in one tick:
//! by the frame after, the only trace is that some parcels are moving outward,
//! which is not something a person can see.
//!
//! So the simulation says what occurred and the shell decides what that looks
//! like. Every variant here is something a person would name if they were
//! watching, and nothing here is read back by the simulation — draining the
//! list changes no outcome, which is what makes it safe for a shell to ignore.

use super::parcel::SubstanceId;
use super::vec2::Vec2;

/// One thing that happened, somewhere.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Canon rule 9: a bare ring went off. `strength` is the spell's own.
    Blast { at: Vec2, strength: f32 },
    /// A spell raised matter — created or found, §3.2 decides which.
    Summon {
        at: Vec2,
        substance: SubstanceId,
        mass: f32,
        /// True when the mass came out of nothing, false when it was gathered.
        /// The difference is the whole capability model, and it deserves to
        /// look different.
        created: bool,
    },
    /// A spell wanted a substance the world did not have (§3.2). Nothing was
    /// raised, and the honest thing is to show the refusal rather than nothing.
    Refused { at: Vec2, substance: SubstanceId },
    /// A prop caught light.
    Ignite { at: Vec2 },
    /// A burning prop was put out.
    Douse { at: Vec2 },
    /// A prop was consumed, and what it was made of.
    Spent { at: Vec2, substance: SubstanceId },
    /// Light flared. Separate from `Summon` because light is the one substance
    /// whose effect is on the *room* rather than on the place it appeared.
    Flash { at: Vec2, strength: f32 },
    /// Two substances met and became a third.
    Reacted {
        at: Vec2,
        product: SubstanceId,
        mass: f32,
    },
}

impl Event {
    /// Where it happened. Every event has a place; that is most of the point.
    pub fn at(&self) -> Vec2 {
        match self {
            Event::Blast { at, .. }
            | Event::Summon { at, .. }
            | Event::Refused { at, .. }
            | Event::Ignite { at }
            | Event::Douse { at }
            | Event::Spent { at, .. }
            | Event::Flash { at, .. }
            | Event::Reacted { at, .. } => *at,
        }
    }

    /// A short name, for a readout. The shell picks colours; this picks words.
    pub fn label(&self) -> &'static str {
        match self {
            Event::Blast { .. } => "blast",
            Event::Summon { created: true, .. } => "created",
            Event::Summon { .. } => "gathered",
            Event::Refused { .. } => "refused",
            Event::Ignite { .. } => "ignite",
            Event::Douse { .. } => "douse",
            Event::Spent { .. } => "spent",
            Event::Flash { .. } => "flash",
            Event::Reacted { .. } => "reacted",
        }
    }
}
