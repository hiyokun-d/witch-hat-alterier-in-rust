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
    /// The ring size a seal's power is quoted against.
    ///
    /// Canon rule 8: "larger seals are more powerful than smaller ones". That
    /// is a ratio with nothing on the other side of it until someone picks a
    /// reference, so this is the pick. **Ours.**
    pub reference_radius: f32,
    /// Seconds an `Active` seal keeps *summoning*, per unit of strength.
    ///
    /// Not how long a parcel lasts — how long the spell goes on making them.
    /// Canon points straight at this: long-duration water spells *collect*
    /// rather than create, which is a statement about a spell that runs for a
    /// while, and rule 8 has a neat seal "long-lasting". A spell that fires
    /// once and stops is a firework, not a fountain.
    pub channel_seconds: f32,
    /// The shortest and longest a channel may run, whatever the seal says.
    ///
    /// **Ours** (§2.6), and a floor rather than only a ceiling. Canon has rule
    /// 8's neat seal "long-lasting" and gives no seconds, so the numbers are a
    /// choice — but the *floor* is the interesting half: a spell that raises a
    /// splash and stops before you have looked at it is a spell you cannot
    /// learn anything from, however faithfully it was graded. A rough seal
    /// should be visibly worse than a neat one and still be a spell.
    ///
    /// A `Fleeting` seal is the one exception and keeps its short life: canon's
    /// own word for a ring too rough to hold has to mean something.
    pub channel_bounds: (f32, f32),
    /// Seconds an `Active` seal's parcels last, per unit of strength.
    pub life_active: f32,
    /// Seconds a `Fleeting` seal's last. Canon's word for a ring too rough to
    /// hold, so this is deliberately short rather than merely smaller.
    pub life_fleeting: f32,
    /// Speed a discharge imparts to what it catches, per unit of strength.
    pub blast: f32,
    /// Degrees a discharge adds at its centre.
    pub blast_heat: f32,
    /// How much of a held parcel's weight a converging seal carries, `0..=1`.
    ///
    /// **This is the mechanism, and the spring below is only the shaping.** The
    /// first version had no such term: it fought gravity with a spring, which
    /// cannot work and did not — a seal whose sigil fills a fifth of its ring
    /// has an intensity near `0.15`, so the pull came out at 225px/s² against
    /// 900 of gravity and the water fell straight through the ring while every
    /// reading beside it said `GATHERS at the centre`.
    ///
    /// The catalogue had been saying the right thing the whole time. `water_orb`
    /// reads: "A water sigil surrounded by levitation signs: **gravity is
    /// reduced** and the water is held in suspension as a ball." Canon does not
    /// describe a force that wins a tug of war with weight; it describes weight
    /// being *taken away*. So the spell carries it, and what is left over is a
    /// gentle pull that shapes what is already floating into a sphere.
    ///
    /// Not scaled by the seal's strength, deliberately. Canon's claim is about a
    /// levitation seal that is *balanced*, not a strong one — a small neat orb
    /// and a big one both float, and strength decides how tightly they ball up
    /// rather than whether they fall.
    pub suspend: f32,
    /// How hard a *converging* seal pulls its substance toward the point its
    /// signs aim at, per second, per unit of strength.
    ///
    /// **Ours** (§2.6) in its number and canon in its shape. Levitation "floats
    /// the target, often shaping it into a sphere when balanced" — a sphere is
    /// what you get when something is pulled toward one point from every side,
    /// so canon describes the force and we pick its strength.
    ///
    /// It is a *spring*, not a snap. Placement alone can never hold an orb
    /// together: fire rises because it is hot and water falls because it is
    /// heavy, so a seal that only decides where parcels *appear* has lost them
    /// a second later. The pull is what makes gathering a lasting state rather
    /// than an opening arrangement, and it lasts exactly as long as the spell
    /// does — let the channel run out and the orb falls, which is the honest
    /// ending.
    pub gather: f32,
    /// Speed a gathered parcel sheds per second, so it settles into the orb
    /// rather than orbiting it forever. Without this the spring is a planet.
    pub gather_damping: f32,
    /// Turns every placement by this much, so a channel trickling one parcel a
    /// tick does not stack them all on one spot.
    ///
    /// `cast` places by slot, and a channel casts a *single* parcel per tick —
    /// so slot is always zero and every drop of a long spell entered the world
    /// at the same point, which is why a gathering seal poured a thin vertical
    /// stream instead of filling its ring. `pour_channels` walks this round.
    pub phase: f32,
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
            reference_radius: 140.0,
            channel_seconds: 20.0,
            channel_bounds: (15.0, 45.0),
            life_active: 6.0,
            life_fleeting: 0.6,
            blast: 400.0,
            blast_heat: 300.0,
            phase: 0.0,
            // Against 900px/s2 of gravity: at the edge of a 140px ring this is
            // 4200px/s2, and it falls to gravity's own strength about 30px from
            // the focus. So an orb holds and sags a little, which is what a
            // held ball of water looks like.
            suspend: 1.0,
            gather: 30.0,
            // Under critical damping (2*sqrt(30) is about 11), on purpose: a
            // little bounce reads as something being *held* rather than glued.
            gather_damping: 6.0,
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

/// How long a spell keeps summoning, in seconds.
///
/// Rule 8 twice over: a seal too rough to hold gets a fraction of the time, and
/// within the seals that do hold, a neater one runs longer. Zero for anything
/// that does not fire.
pub fn channel_for(spell: &Spell, rules: &CastRules) -> f32 {
    if !spell.fires() || spell.cancelled {
        return 0.0;
    }
    let craft = spell.quality.clamp(0.1, 1.0);
    let wanted = rules.channel_seconds * craft * spell.strength().clamp(0.1, 4.0);
    let (floor, ceiling) = rules.channel_bounds;
    match spell.firing {
        // Fleeting is exempt from the floor. Canon's word for a ring too rough
        // to hold has to cost something, and a fifth of the *floor* is still a
        // spell you can see.
        Firing::Fleeting => (wanted * 0.2).min(ceiling),
        _ => wanted.clamp(floor.min(ceiling), ceiling),
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
    // Canon rule 8's other claim: a larger seal is a stronger one. Kept apart
    // from `intensity`, which is the sigil measured *against* its ring — a big
    // seal with a small sigil is powerful and unfocused, a small one with a
    // filling sigil is focused and weak, and both facts have to survive.
    let bulk = (spell.scale / rules.reference_radius).clamp(0.2, 4.0);
    let wanted = strength * rules.mass_per_strength * count as f32 * bulk;

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
    // Canon rule 8, both halves. `Fleeting` is the wiki's own word for a ring
    // too rough to hold at all; `quality` is the rest of the sentence — "neatly
    // drawn seals are more stable and long-lasting than messy ones" — and it is
    // continuous, so a good ring and a perfect one must not last the same time.
    let life = match spell.firing {
        Firing::Fleeting => rules.life_fleeting,
        _ => rules.life_active,
    } * strength.max(0.1)
        * spell.quality.clamp(0.1, 1.0);

    let speed = strength * rules.speed_per_strength;
    let lean = spell.balance.lean();
    let drift = if lean > 0.0 {
        Vec2::from_angle(spell.balance.heading) * lean
    } else {
        Vec2::ZERO
    };

    let share = available / count as f32;
    for slot in 0..count {
        let (place, outward) = placement(spell, slot, count, radius, rules.phase);
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
fn placement(spell: &Spell, slot: usize, count: usize, radius: f32, phase: f32) -> (Vec2, Vec2) {
    use crate::arrangement::{Convergence, RegionArrangement};

    let turn = std::f32::consts::TAU * slot as f32 / count as f32 + phase;
    let around = Vec2::from_angle(turn);

    // A narrow fan rather than a line: a jet has width, and one column of
    // parcels reads as a laser.
    let fan = |aim: f32, spread: f32| {
        let offset = (slot as f32 / count as f32 - 0.5) * spread;
        Vec2::from_angle(aim + offset)
    };

    // **Region signs first, because canon states them.** §2.3 gives the four
    // arrangements a meaning nothing else has — "only inside the ring", "no
    // effect within the seal" — and a column sign pointing the same way does
    // not say that. So a seal carrying region signs is answered by them.
    if let RegionArrangement::Canon { pattern, heading } = &spell.region {
        return match pattern {
            RegionPattern::AllSameSide => {
                let aim = heading.unwrap_or(spell.balance.heading);
                let direction = fan(aim, std::f32::consts::PI / 3.0);
                (direction * (radius * 0.5), direction)
            }
            RegionPattern::AllInward => (around * (radius * 0.45), -around),
            RegionPattern::AllOutward => (around * (radius * 1.2), around),
            RegionPattern::Opposed => (around * radius, around),
        };
    }

    // **Then wherever the keystones actually aim.** This is the half that was
    // missing, and it is the one a person notices: four arrows drawn at the
    // middle are unmistakably a spell that gathers there, and until now the
    // simulation read them only through `Balance` — which correctly reports
    // that they cancel, and therefore said "shoots straight up".
    //
    // `Focus::at` is measured from the ring's centre, which is exactly the
    // frame `place` is returned in.
    let focus = &spell.focus;
    let aimed = Vec2::new(focus.at.0, focus.at.1);
    match focus.convergence {
        // Gathering. Parcels ring the focal point and are aimed *at* it, so
        // they arrive rather than sit — canon's levitation "floats the target,
        // often shaping it into a sphere when balanced", and a sphere is what
        // arriving from every side makes.
        Convergence::Converging => {
            let ring = (radius * 0.45).min(spell.scale);
            (aimed + around * ring, -around)
        }
        // Spreading from a point: the fountain, and the exact reverse.
        Convergence::Diverging => (aimed + around * (radius * 0.2), around),
        // Canon's floating drops, on the ring itself.
        Convergence::Split => (around * radius, around),
        // Every sign pointing one way is a beam, and `Balance` is the right
        // reading of it — the signs agree, so their sum is their direction.
        Convergence::Parallel => {
            let direction = fan(spell.balance.heading, std::f32::consts::PI / 4.0);
            (direction * (radius * 0.5), direction)
        }
        // **Nothing configured this spell.** No sign steers it, so there is no
        // direction to obey, and the magic manifests *at the seal*.
        //
        // The previous answer here was "shoots straight up", quoting canon's
        // line about column signs that are "all the same size, and as such, the
        // same power". That is a claim about a seal with *balanced signs* — it
        // is not a claim about a seal with **none**, and reading it as one made
        // a bare fire seal spit its flame out of the top of its own ring
        // instead of burning in the middle of it.
        //
        // Scattered a little rather than stacked on one point, so a dozen
        // parcels are a body of substance instead of a dot, and given no
        // velocity at all: fire rises because it is hot and water falls because
        // it is heavy, which is the whole reason those are modelled.
        Convergence::Unaimed => (around * (radius * 0.18), Vec2::ZERO),
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
