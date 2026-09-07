//! Turning a compiled [`Spell`] into parcels — the step M5 was built for.
//!
//! Everything the compiler measured finally decides something here:
//!
//! | What the spell carries | What it decides |
//! | --- | --- |
//! | [`Spell::fires`] | whether anything happens at all (canon rule 2) |
//! | [`Spell::strength`] | how much substance, and how fast |
//! | [`Spell::demand`] | *what* substance, and whether it must be found (§3.2) |
//! | `balance` | which way the result leans (§2.4) |
//! | `spin` | how much of that effort is rotation instead of reach |
//! | `region` | *where* it manifests — canon's four cases (§2.3) |
//! | `sign_count` | how many parcels; canon ties count to quantity |
//! | `scale` | the radius everything is placed against |
//! | `firing` | how long it lasts — `Fleeting` is canon's own word |
//!
//! The numbers in [`CastRules`] are **ours** (§2.6). Canon says a longer sign
//! has more power and a neater seal lasts longer; it gives no units for either.

use crate::catalog::RegionPattern;
use crate::compiler::{Driver, Firing, Spell};

use super::field::Field;
use super::parcel::{Parcel, SubstanceId};
use super::vec2::Vec2;

/// The exchange rates between a compiled spell and a simulation. All ours.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CastRules {
    /// Mass raised per unit of [`Spell::strength`], per parcel.
    pub mass_per_strength: f32,
    /// Pixels per second per unit of strength.
    pub speed_per_strength: f32,
    /// Parcels a seal with no signs produces.
    pub base_parcels: usize,
    /// Extra parcels per sign. Canon: "the amount of signs will affect the
    /// range or quantity of magic generated" (§2.4), which is separate from
    /// size being power.
    pub parcels_per_sign: usize,
    /// Never more than this many, whatever the seal says.
    pub max_parcels: usize,
    /// How far out a spell may look for substance, as a multiple of its radius.
    pub reach: f32,
    /// Seconds an `Active` seal's parcels last, per unit of strength.
    pub life_active: f32,
    /// Seconds a `Fleeting` seal's last. Canon's word for a ring too rough to
    /// hold, so this is deliberately short rather than merely smaller.
    pub life_fleeting: f32,
    /// Speed a discharge imparts to what it catches, per unit of strength.
    pub blast: f32,
    /// Degrees a discharge adds at its centre.
    pub blast_heat: f32,
    /// Sigils whose spells anchor what they touch — canon's repetition (§2.2).
    ///
    /// Data on the rules rather than a `match` arm on an id (§4.4). It is still
    /// a content check living in code, and it belongs in `sigils.ron` the day
    /// that file grows a field for it.
    pub anchoring: &'static [&'static str],
}

impl Default for CastRules {
    fn default() -> Self {
        CastRules {
            mass_per_strength: 1.0,
            speed_per_strength: 120.0,
            base_parcels: 6,
            parcels_per_sign: 2,
            max_parcels: 64,
            reach: 3.0,
            life_active: 6.0,
            life_fleeting: 0.6,
            blast: 400.0,
            blast_heat: 300.0,
            anchoring: &["repetition"],
        }
    }
}

/// What happened when a spell was cast.
#[derive(Debug, Clone, PartialEq)]
pub enum CastOutcome {
    /// Parcels were raised.
    Fired,
    /// A bare ring: canon rule 9's explosion. No matter is created, because an
    /// explosion is energy — it shoves and heats what is already there.
    Discharged,
    /// The ring is open or too rough. Canon rule 2 makes this *prepared*, not
    /// broken, so it is an outcome rather than an error.
    NotFiring(Firing),
    /// Rule 6: a reversed twin cancelled it out completely.
    Cancelled,
    /// The sigil cannot create, and there was none of its substance in reach.
    /// The conservation model biting, exactly as §3.2 intends.
    NothingFound { substance: String, wanted: f32 },
}

/// What a cast did, in numbers a readout can print.
#[derive(Debug, Clone, PartialEq)]
pub struct CastReport {
    pub outcome: CastOutcome,
    /// Parcels added to the field.
    pub spawned: usize,
    /// Mass conjured out of nothing. Only ever non-zero for a sigil that
    /// `can_create` — this is the number a conservation audit watches.
    pub created: f32,
    /// Mass taken from parcels already present.
    pub found: f32,
    /// Mass the spell wanted and could not find.
    pub shortfall: f32,
    /// Parcels a discharge shoved.
    pub pushed: usize,
}

impl CastReport {
    fn nothing(outcome: CastOutcome) -> CastReport {
        CastReport {
            outcome,
            spawned: 0,
            created: 0.0,
            found: 0.0,
            shortfall: 0.0,
            pushed: 0,
        }
    }
}

/// Casts `spell` centred on `at`, into `field`.
///
/// Total, like [`crate::compile`]: there is no error case. A spell that cannot
/// fire, or cannot find its substance, returns a report saying so.
pub fn cast(spell: &Spell, at: Vec2, field: &mut Field, rules: &CastRules) -> CastReport {
    if spell.cancelled {
        return CastReport::nothing(CastOutcome::Cancelled);
    }
    if !spell.fires() {
        return CastReport::nothing(CastOutcome::NotFiring(spell.firing));
    }

    let strength = spell.strength().max(0.0);
    let radius = spell.scale.max(1.0);

    if matches!(spell.driver, Driver::Discharge) {
        return discharge(at, radius, strength, field, rules);
    }

    // A substitute drives a spell with no substance behind it — the wiki does
    // not say what a stability or billow seal is *made of*, and §2 says leave
    // an honest hole rather than invent one. So it shoves rather than raises.
    let Some(demand) = spell.demand.as_ref() else {
        return discharge(at, radius, strength, field, rules);
    };
    let Some(name) = demand.substance.first() else {
        return discharge(at, radius, strength, field, rules);
    };
    let substance = SubstanceId::new(name.clone());

    let count = (rules.base_parcels + rules.parcels_per_sign * spell.sign_count)
        .min(rules.max_parcels)
        .max(1);
    let wanted = strength * rules.mass_per_strength * count as f32;

    let (available, found, created) = if demand.must_find {
        let got = field.take(&substance, wanted, at, radius * rules.reach);
        if got <= 0.0 {
            return CastReport::nothing(CastOutcome::NothingFound {
                substance: name.clone(),
                wanted,
            });
        }
        (got, got, 0.0)
    } else {
        (wanted, 0.0, wanted)
    };

    let anchoring = match &spell.driver {
        Driver::Sigil(id) => rules.anchoring.contains(&id.as_str()),
        _ => false,
    };
    let life = match spell.firing {
        Firing::Fleeting => rules.life_fleeting,
        _ => rules.life_active,
    } * strength.max(0.1);

    let speed = strength * rules.speed_per_strength;
    let lean = spell.balance.lean();
    let drift = if lean > 0.0 {
        Vec2::from_angle(spell.balance.heading) * lean
    } else {
        Vec2::ZERO
    };

    let share = available / count as f32;
    for slot in 0..count {
        let (place, outward) = placement(spell, slot, count, radius);
        // §2.4's exchange rate, and the only place it becomes motion: reach
        // pushes outward, spin pushes along the tangent, and the two shares
        // always sum to one because `Spin` guarantees it.
        let heading = outward * spell.spin.reach + outward.perpendicular() * spell.spin.spin;
        let mut parcel = Parcel::new(substance.clone(), at + place, share);
        parcel.velocity = (heading.normalize_or_zero() + drift) * speed;
        parcel.life = life;
        if anchoring {
            parcel.anchor_here();
        }
        field.add(parcel);
    }

    CastReport {
        outcome: CastOutcome::Fired,
        spawned: count,
        created,
        found,
        shortfall: (wanted - available).max(0.0),
        pushed: 0,
    }
}

/// Where parcel `slot` of `count` goes, and which way is "outward" there.
///
/// Canon's four region cases (§2.3), computed by the compiler and finally
/// meaning something. `Absent` and `Indeterminate` both fall back to a ring on
/// the seal, because that is what a spell with nothing telling it otherwise
/// does everywhere else in the wiki.
fn placement(spell: &Spell, slot: usize, count: usize, radius: f32) -> (Vec2, Vec2) {
    use crate::arrangement::RegionArrangement;

    let turn = std::f32::consts::TAU * slot as f32 / count as f32;
    let around = Vec2::from_angle(turn);

    match &spell.region {
        RegionArrangement::Canon { pattern, heading } => match pattern {
            // Shoots the way the signs point: a narrow fan about that heading.
            RegionPattern::AllSameSide => {
                let aim = heading.unwrap_or(spell.balance.heading);
                let spread = (slot as f32 / count as f32 - 0.5) * (std::f32::consts::PI / 3.0);
                let direction = Vec2::from_angle(aim + spread);
                (direction * (radius * 0.5), direction)
            }
            // Only inside the ring.
            RegionPattern::AllInward => (around * (radius * 0.45), -around),
            // Only outside it, with no effect within the seal.
            RegionPattern::AllOutward => (around * (radius * 1.2), around),
            // Only on the ring itself — canon's floating drops.
            RegionPattern::Opposed => (around * radius, around),
        },
        _ => (around * (radius * 0.8), around),
    }
}

/// Canon rule 9, and the reason it is a first-class case.
///
/// A bare ring is "a rapid discharge of energy, i.e. an explosion". Energy, not
/// matter — so this creates nothing and conserves everything. It shoves what is
/// in reach outward and heats it, and if the room is empty then nothing visible
/// happens, which is the honest answer rather than a puff of invented smoke.
fn discharge(
    at: Vec2,
    radius: f32,
    strength: f32,
    field: &mut Field,
    rules: &CastRules,
) -> CastReport {
    let reach = radius * rules.reach;
    let mut pushed = 0;

    for parcel in field.parcels_mut() {
        let offset = parcel.at - at;
        let distance = offset.length();
        if distance > reach {
            continue;
        }
        // Linear falloff. Ours, and chosen for being readable on screen rather
        // than for being physical — an inverse square is invisible at the edge.
        let share = 1.0 - distance / reach;
        let outward = offset.normalize_or_zero();
        parcel.velocity += outward * (rules.blast * strength * share);
        parcel.temperature += rules.blast_heat * strength * share;
        pushed += 1;
    }

    CastReport {
        outcome: CastOutcome::Discharged,
        spawned: 0,
        created: 0.0,
        found: 0.0,
        shortfall: 0.0,
        pushed,
    }
}

#[cfg(test)]
#[path = "../tests/cast.rs"]
mod tests;
