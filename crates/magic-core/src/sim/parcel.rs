//! One element instance: a lump of some substance, somewhere, doing something.
//!
//! The smallest thing the simulation can conserve. Everything §2.6 promises —
//! reactions, phase change, emergent behaviour — is a rule about how parcels
//! meet, so this struct decides how much of that is even expressible.

use super::vec2::Vec2;

/// Room temperature, and the state everything decays back toward.
///
/// **Ours** (§2.6). Canon gives no units and no numbers; degrees Celsius is a
/// choice made here so that every other constant can be read against it.
pub const AMBIENT: f32 = 20.0;

/// What a parcel is made of, as the catalogue names it.
///
/// A newtype over `String`, exactly like [`crate::SigilId`] — §3.1 and §4.4.
/// `Demand::substance` is already `Vec<String>` straight out of `sigils.ron`, so
/// an enum here would break the day someone adds a substance to the data.
///
/// `Ord` because sorting by substance is what keeps a later sum from depending
/// on the order parcels happened to be added in (§4.3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubstanceId(pub String);

impl SubstanceId {
    pub fn new(name: impl Into<String>) -> SubstanceId {
        SubstanceId(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The state a repetition spell keeps returning its target to.
///
/// Canon, and unusually literal about it: repetition "continuously resets
/// affected objects to the state they held when the spell took hold,
/// temperature included" (§2.2). So the anchor is a snapshot of exactly the
/// three fields that can drift, and nothing else.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Anchor {
    pub at: Vec2,
    pub velocity: Vec2,
    pub temperature: f32,
}

/// One lump of substance.
#[derive(Debug, Clone, PartialEq)]
pub struct Parcel {
    /// What it is made of. Conservation (§3.2) is unenforceable without this.
    pub substance: SubstanceId,
    /// Where it is, in the pad's own pixel coordinates.
    pub at: Vec2,
    /// How fast and which way, in pixels per second.
    pub velocity: Vec2,
    /// How much of it there is. The quantity `Demand::must_find` has to take
    /// from somewhere, and the one M6.8's conservation tests are written on.
    pub mass: f32,
    /// How hot, in the same units as [`AMBIENT`].
    pub temperature: f32,
    /// Seconds left before it expires. `f32::INFINITY` for a parcel that does
    /// not — infinity rather than `Option` so the step loop has no branch.
    pub life: f32,
    /// Set only by a repetition-driven spell. See [`Anchor`].
    pub anchor: Option<Anchor>,
}

impl Parcel {
    /// A still parcel at ambient temperature that lasts forever.
    pub fn new(substance: SubstanceId, at: Vec2, mass: f32) -> Parcel {
        Parcel {
            substance,
            at,
            velocity: Vec2::ZERO,
            mass,
            temperature: AMBIENT,
            life: f32::INFINITY,
            anchor: None,
        }
    }

    /// Momentum — `velocity * mass`. M6.8 is written against it.
    pub fn momentum(&self) -> Vec2 {
        self.velocity * self.mass
    }

    /// Heat content, in mass-degrees. What actually has to be conserved when
    /// two parcels exchange temperature — averaging temperatures alone would
    /// let a speck cool a boulder.
    pub fn heat(&self) -> f32 {
        self.mass * self.temperature
    }

    /// Whether this parcel is still in the world.
    pub fn alive(&self) -> bool {
        self.life > 0.0 && self.mass > 0.0
    }

    /// Pins the parcel's current state as the one repetition returns it to.
    pub fn anchor_here(&mut self) {
        self.anchor = Some(Anchor {
            at: self.at,
            velocity: self.velocity,
            temperature: self.temperature,
        });
    }
}

#[cfg(test)]
#[path = "../tests/parcel.rs"]
mod tests;
