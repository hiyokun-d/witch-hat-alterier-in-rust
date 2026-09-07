//! Things on the paper for a spell to act *on*.
//!
//! # Entirely ours (§2.6)
//!
//! Canon has no objects. A seal in the manga is drawn on a thing and acts on
//! that thing, and the wiki never describes what a spell does to a plank. So
//! every rule in this module is our extension, and it exists for one reason:
//!
//! **A spell with nothing to affect is invisible.** A fire seal in an empty
//! room correctly does nothing — §3.2 is exactly why, since fire's own entry
//! says it creates flame and nothing says it creates fuel. `fill_air` fixed
//! half of that by giving a discharge something to shove. This fixes the other
//! half: something that can catch, soak, be blown about, and be lit.
//!
//! # A prop is not a parcel, on purpose
//!
//! It would be less code to model a plank as a hundred `wood` parcels, and it
//! would be wrong: parcels are a *fluid*, and a hundred of them would drift
//! apart within a second. The point of a prop is that it holds together until
//! something destroys it, so it is a rigid rectangle that the field acts on and
//! that acts back — one body, one temperature, one integrity.
//!
//! # What is conserved and what is not
//!
//! Burning converts a prop's own mass into flame and smoke parcels, at the rate
//! the prop is consumed. Nothing is minted: a plank of mass six leaves exactly
//! six units of exhaust behind it, which is what keeps §3.2 honest once objects
//! exist. Heat is *accounted* rather than conserved, the same call
//! `reaction.rs` makes — combustion releases energy the model never stored.

use super::event::Event;
use super::field::Field;
use super::material::Materials;
use super::parcel::{AMBIENT, Parcel, SubstanceId};
use super::vec2::Vec2;

/// A rectangle of some substance, sitting on the paper.
#[derive(Debug, Clone, PartialEq)]
pub struct Prop {
    /// Centre, in the pad's own pixel coordinates — the same frame as a parcel.
    pub at: Vec2,
    /// Half-width and half-height. Half rather than full so the four edges are
    /// `at ± half` with no factor of two to forget.
    pub half: Vec2,
    pub substance: SubstanceId,
    pub mass: f32,
    pub temperature: f32,
    /// How wet, `0..=1`. Water raises it, heat drives it off, and enough of it
    /// refuses to let the prop light.
    pub wetness: f32,
    /// Whether it is currently on fire.
    pub burning: bool,
    /// How much of it is left, `1.0` whole down to `0.0` consumed.
    pub integrity: f32,
    pub velocity: Vec2,
    /// How much light is falling on it, `0..=1`, smoothed. Rendering reads it;
    /// nothing in the simulation does.
    pub lit: f32,
}

impl Prop {
    /// A whole, dry, room-temperature prop.
    pub fn new(substance: SubstanceId, at: Vec2, half: Vec2, mass: f32) -> Prop {
        Prop {
            at,
            half,
            substance,
            mass,
            temperature: AMBIENT,
            wetness: 0.0,
            burning: false,
            integrity: 1.0,
            velocity: Vec2::ZERO,
            lit: 0.0,
        }
    }

    /// Whether a point is inside it.
    pub fn holds(&self, at: Vec2) -> bool {
        (at.x - self.at.x).abs() <= self.half.x && (at.y - self.at.y).abs() <= self.half.y
    }

    /// How much of its original mass is still standing.
    pub fn standing(&self) -> f32 {
        self.mass * self.integrity.clamp(0.0, 1.0)
    }

    /// Whether it is still in the world.
    pub fn alive(&self) -> bool {
        self.integrity > 0.0
    }
}

/// The numbers behind catching, soaking, drying and burning.
///
/// **Every one of them ours** (§2.6). They are picked so that the four sigils a
/// person can actually draw each do something visible to a prop within a second
/// or two — a demonstration has to be watchable, and canon gives no timings to
/// be faithful to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropRules {
    /// How fast a prop's temperature follows the air around it, per second.
    pub conduct: f32,
    /// Wetness gained per unit of water mass touching it, per second.
    pub soak: f32,
    /// Wetness lost per second once the prop is above `steaming`.
    pub dry: f32,
    /// The temperature at which a wet prop starts drying rather than soaking.
    pub steaming: f32,
    /// Wetness above which it will not light, and at which a burning prop goes
    /// out.
    pub quench: f32,
    /// Fraction of the prop consumed per second while burning.
    pub burn: f32,
    /// What a burning prop's own temperature climbs toward.
    pub flame_temp: f32,
    /// Of the mass a burning prop gives up, how much leaves as flame. The rest
    /// leaves as smoke, which is what makes a fire read as a fire rather than
    /// as a glow.
    pub ember_share: f32,
    /// How hard moving air pushes it, per second. A prop is heavy and the air
    /// is not, so this is small — a gale nudges a plank, it does not launch it.
    pub push: f32,
    /// Speed shed per second when nothing is pushing.
    pub drag: f32,
    /// How far light reaches to count as falling on a prop, in pixels.
    pub lamp_reach: f32,
    /// Mass of glowing substance within `lamp_reach` at which a prop is fully
    /// lit.
    pub lamp_full: f32,
}

impl Default for PropRules {
    fn default() -> Self {
        PropRules {
            conduct: 2.4,
            soak: 0.9,
            dry: 0.35,
            steaming: 70.0,
            quench: 0.3,
            burn: 0.16,
            flame_temp: 780.0,
            ember_share: 0.65,
            push: 0.5,
            drag: 1.8,
            lamp_reach: 90.0,
            lamp_full: 3.0,
        }
    }
}

/// What the field looks like where a prop is sitting.
struct Around {
    temperature: f32,
    flow: Vec2,
    water: f32,
    glow: f32,
}

/// Surveys the parcels touching a prop.
///
/// Read from the parcels rather than from `Field`'s cells because a prop is
/// rarely cell-aligned, and a cell-quantised answer would make a plank half in
/// a fire either fully alight or not at all. Mass-weighted, so a speck cannot
/// outvote a puddle — the same discipline `Field::settle` uses.
fn around(prop: &Prop, field: &Field, materials: &Materials, reach: f32) -> Around {
    let (mut mass, mut heat, mut momentum) = (0.0, 0.0, Vec2::ZERO);
    let (mut water, mut glow) = (0.0, 0.0);
    // A little wider than the prop itself: a fire beside a plank should still
    // warm it, and touching-only would mean a flame had to be inside the wood.
    let margin = Vec2::new(reach, reach);
    for parcel in field.parcels() {
        let near = (parcel.at.x - prop.at.x).abs() <= prop.half.x + margin.x
            && (parcel.at.y - prop.at.y).abs() <= prop.half.y + margin.y;
        if !near {
            continue;
        }
        mass += parcel.mass;
        heat += parcel.heat();
        momentum += parcel.momentum();
        if parcel.substance.as_str() == "water" {
            water += parcel.mass;
        }
        if materials.get(parcel.substance.as_str()).glow > 0.5 {
            glow += parcel.mass;
        }
    }

    Around {
        temperature: if mass > 0.0 { heat / mass } else { AMBIENT },
        flow: if mass > 0.0 {
            momentum * (1.0 / mass)
        } else {
            Vec2::ZERO
        },
        water,
        glow,
    }
}

/// Advances every prop one step, and says what happened.
///
/// Deterministic throughout (§4.3): props are walked in slice order, every
/// prop's surroundings are read before anything is written, and the parcels a
/// burning prop emits are appended in that same order.
pub fn step_props(
    props: &mut Vec<Prop>,
    field: &mut Field,
    rules: &PropRules,
    materials: &Materials,
    dt: f32,
    events: &mut Vec<Event>,
) {
    if props.is_empty() || dt <= 0.0 {
        return;
    }

    // Surveyed first, all of them, so a prop that emits flame this tick cannot
    // heat the prop beside it until the next one. Otherwise the answer would
    // depend on which prop happened to be first in the list.
    let seen: Vec<Around> = props
        .iter()
        .map(|prop| around(prop, field, materials, 18.0))
        .collect();

    let mut raised: Vec<Parcel> = Vec::new();

    for (prop, near) in props.iter_mut().zip(seen.iter()) {
        // ── heat ────────────────────────────────────────────────────────────
        let toward = if prop.burning {
            rules.flame_temp
        } else {
            near.temperature
        };
        prop.temperature += (toward - prop.temperature) * (rules.conduct * dt).clamp(0.0, 1.0);

        // ── water on it, and heat driving that water off ────────────────────
        if near.water > 0.0 {
            prop.wetness += near.water * rules.soak * dt;
        }
        if prop.temperature > rules.steaming {
            prop.wetness -= rules.dry * dt;
        }
        prop.wetness = prop.wetness.clamp(0.0, 1.0);

        // ── catching, and being put out ─────────────────────────────────────
        let kindles = materials.get(prop.substance.as_str()).ignites_at;
        match (prop.burning, kindles) {
            (false, Some(point))
                if prop.temperature >= point && prop.wetness < rules.quench && prop.alive() =>
            {
                prop.burning = true;
                events.push(Event::Ignite { at: prop.at });
            }
            (true, _) if prop.wetness >= rules.quench => {
                prop.burning = false;
                events.push(Event::Douse { at: prop.at });
                // What put it out leaves as steam. The water is already in the
                // field as parcels; this is the prop's own heat spent boiling.
                raised.push(Parcel {
                    temperature: 110.0,
                    life: 3.0,
                    ..Parcel::new(SubstanceId::new("steam"), prop.at, 0.3)
                });
                prop.temperature = prop.temperature.min(rules.steaming);
            }
            _ => {}
        }

        // ── burning: the prop's own mass becoming exhaust ───────────────────
        if prop.burning && prop.alive() {
            let spent = (rules.burn * dt).min(prop.integrity);
            prop.integrity -= spent;
            let mass = spent * prop.mass;
            let ember = mass * rules.ember_share;
            raised.push(Parcel {
                temperature: rules.flame_temp,
                life: 4.0,
                ..Parcel::new(SubstanceId::new("flame"), prop.at, ember)
            });
            raised.push(Parcel {
                temperature: rules.flame_temp * 0.4,
                life: 6.0,
                ..Parcel::new(SubstanceId::new("smoke"), prop.at, mass - ember)
            });
        }

        // ── the wind pushing it about ───────────────────────────────────────
        let pushed = (near.flow - prop.velocity) * (rules.push * dt);
        prop.velocity += pushed;
        prop.velocity = prop.velocity * (1.0 - (rules.drag * dt).clamp(0.0, 1.0));
        prop.at += prop.velocity * dt;

        // Kept on the paper. A prop that wandered off the field would still be
        // burning where nobody could see it.
        let (low, high) = field.bounds();
        prop.at.x = prop.at.x.clamp(low.x + prop.half.x, high.x - prop.half.x);
        prop.at.y = prop.at.y.clamp(low.y + prop.half.y, high.y - prop.half.y);

        // ── how lit it is. Rendering only. ──────────────────────────────────
        let wanted = (near.glow / rules.lamp_full).clamp(0.0, 1.0);
        prop.lit += (wanted - prop.lit) * (4.0 * dt).clamp(0.0, 1.0);
    }

    for parcel in raised {
        if parcel.mass > 0.0 {
            field.add(parcel);
        }
    }

    // Spent props leave, and say so. Done after the walk so an index is never
    // invalidated mid-loop.
    let gone: Vec<Event> = props
        .iter()
        .filter(|prop| !prop.alive())
        .map(|prop| Event::Spent {
            at: prop.at,
            substance: prop.substance.clone(),
        })
        .collect();
    events.extend(gone);
    props.retain(|prop| prop.alive());
}

/// Scatters `n` props across a rectangle, deterministically.
///
/// §4.3 forbids randomness outright, so "random on the paper" is a *hashed pure
/// function of the index* — the same seed lays the same kindling out on every
/// machine and every run, which is also what makes a scattered board something
/// a test can be written against.
pub fn scatter(n: usize, low: Vec2, high: Vec2, substance: SubstanceId, seed: u32) -> Vec<Prop> {
    let span = high - low;
    (0..n)
        .map(|index| {
            let mix = |salt: u32| {
                // The same Wang hash `step::swirl` uses, for the same reason:
                // cheap, no dependency, and well-behaved on small inputs.
                let mut h = (index as u32).wrapping_mul(2_654_435_761)
                    ^ salt.wrapping_mul(374_761_393)
                    ^ seed.wrapping_mul(668_265_263);
                h ^= h >> 15;
                h = h.wrapping_mul(2_246_822_519);
                h ^= h >> 13;
                h as f32 / u32::MAX as f32
            };
            // Inset so a prop is never half off the paper.
            let half = Vec2::new(14.0 + mix(3) * 12.0, 10.0 + mix(4) * 10.0);
            let at = Vec2::new(
                low.x + half.x + mix(1) * (span.x - half.x * 2.0).max(0.0),
                low.y + half.y + mix(2) * (span.y - half.y * 2.0).max(0.0),
            );
            // Mass from area, so a bigger plank takes longer to burn through.
            let mass = half.x * half.y * 0.02;
            Prop::new(substance.clone(), at, half, mass)
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/prop.rs"]
mod tests;
