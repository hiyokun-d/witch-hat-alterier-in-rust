//! Advancing the world by one tick: motion, heat, repetition, expiry.
//!
//! Everything here is **ours** (§2.6). Canon spells are discrete effects with no
//! mechanism behind them; a continuous simulation with gravity and heat transfer
//! is our extension, and every constant in [`SimRules`] is a number canon does
//! not give.
//!
//! What is *not* ours is the shape of it. Repetition resetting an object to the
//! state it held when the spell took hold — temperature included — is canon
//! (§2.2), and it falls out of one branch here.
//!
//! # Determinism (§4.3)
//!
//! Fixed timestep, no clock, no randomness, and parcels walked in slice order.
//! `Field::settle` runs once before the pass and once after, so a caller reading
//! cells always sees them agreeing with the parcels.

use super::field::Field;
use super::material::Materials;
use super::parcel::AMBIENT;
use super::vec2::Vec2;

/// The knobs, all of them invented (§2.6).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimRules {
    /// Seconds per tick. Fixed, because §4.3 has the same inputs producing the
    /// same outputs and a variable step makes that impossible.
    pub dt: f32,
    /// Constant acceleration, in pixels per second squared. Negative `y`
    /// because the pad's `y` grows upward.
    pub gravity: Vec2,
    /// Fraction of speed shed per second. Air, roughly.
    pub drag: f32,
    /// How fast a parcel takes its cell's temperature, per second.
    pub mixing: f32,
    /// How fast everything drifts back to [`AMBIENT`], per second.
    pub cooling: f32,
    /// Mass in a cell above which a stiff substance starts pushing back.
    ///
    /// A stand-in for a rest density: below it a liquid is happy, above it the
    /// parcels crowd and shove apart. **Ours**, and the one number that decides
    /// whether a pool looks like a pool.
    pub rest_density: f32,
    /// Whether the field has walls at all.
    ///
    /// On, nothing escapes and mass is conserved exactly — which is what makes
    /// every conservation property in M6.8 testable. Off, a parcel that leaves
    /// is *gone*, and the total falls. Both are honest; only one is measurable,
    /// which is why walls are the default.
    pub walls: bool,
    /// How much speed survives hitting a wall, `0..=1`. Never `1.0` — a
    /// perfect wall is a trampoline and nothing ever comes to rest.
    pub restitution: f32,
    /// How strongly a repetition anchor pulls a parcel back per second, `0..=1`
    /// where `1.0` is an instant snap.
    pub repetition: f32,
}

impl Default for SimRules {
    fn default() -> Self {
        SimRules {
            // 60Hz. Matches what the shell drives its fixed schedule at, and a
            // mismatch there is a bug rather than a taste.
            dt: 1.0 / 60.0,
            gravity: Vec2::new(0.0, -900.0),
            drag: 0.6,
            mixing: 3.0,
            cooling: 0.4,
            walls: true,
            rest_density: 6.0,
            restitution: 0.25,
            // Stiff on purpose. At 6.0 a held parcel sagged 19px under gravity
            // before the anchor balanced it, which is a sag, not a reset —
            // canon calls repetition spring-like and has it holding rot and
            // damage off entirely. 20.0 settles inside a pixel.
            repetition: 20.0,
        }
    }
}

/// Deterministic swirl.
///
/// Fire that rises in a straight line does not look like fire, and the thing
/// that is missing is turbulence. §4.3 forbids randomness outright, so this is
/// not random: it is a **pure function** of where the parcel is and which tick
/// it is, hashed. Same seal, same frame, same swirl, every run and every
/// machine — which is what makes it legal here at all.
///
/// An integer hash rather than a smooth noise field because the result is
/// multiplied by a small number and integrated over sixty ticks a second; the
/// eye sees the integral, and the integral of hash noise is already smooth.
fn swirl(x: f32, y: f32, tick: u64) -> Vec2 {
    // Quantised to a few pixels, so nearby parcels swirl together rather than
    // each doing its own thing — that is the difference between a flame and
    // static.
    let cell_x = (x * 0.15) as i32 as u32;
    let cell_y = (y * 0.15) as i32 as u32;
    // Time moves slower than space, so a flame licks rather than flickers.
    let step = (tick / 6) as u32;

    let mix = |seed: u32| {
        // Wang hash. Cheap, no dependency, and well-behaved on small inputs.
        let mut h = seed.wrapping_mul(2_654_435_761)
            ^ cell_x.wrapping_mul(374_761_393)
            ^ cell_y.wrapping_mul(668_265_263)
            ^ step.wrapping_mul(2_246_822_519);
        h ^= h >> 15;
        h = h.wrapping_mul(2_246_822_519);
        h ^= h >> 13;
        // To -1..=1.
        (h as f32 / u32::MAX as f32) * 2.0 - 1.0
    };

    Vec2::new(mix(0x9E37_79B9), mix(0x85EB_CA6B))
}

/// Advances `field` by one tick.
///
/// Order matters and is the readable part: settle so every parcel can see the
/// cell it is in, then move and mix, then confine, then drop what expired, then
/// settle again so the cells describe where things ended up.
///
/// # The forces, and where each comes from
///
/// | Term | What it is |
/// | --- | --- |
/// | buoyancy | Archimedes, on a density that varies by the ideal gas law |
/// | drag | `-k v`, per substance. A gas is light and wide; a stone is not |
/// | cohesion | pull toward the cell's mean flow. Liquids hold together |
/// | turbulence | deterministic swirl, scaled by heat. Fire flickers |
/// | mixing | temperature toward the cell's, then toward the room |
/// | repetition | canon §2.2, the anchor |
/// | settling | a slow solid on the floor stops. **Ours** |
pub fn step(field: &mut Field, rules: &SimRules, materials: &Materials, tick: u64) {
    let dt = rules.dt;
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }

    field.settle();

    // Read every parcel's cell *before* mutating, because a parcel changing its
    // own cell mid-pass would make the answer depend on where it happened to
    // sit in the list — the same trap as M5.6.
    let seen: Vec<(f32, Vec2)> = field
        .parcels()
        .iter()
        .map(|parcel| {
            field
                .cell(parcel.at)
                .map(|cell| (cell.temperature, cell.flow))
                .unwrap_or((AMBIENT, Vec2::ZERO))
        })
        .collect();

    // Pressure needs to know what is *around* a parcel, not just what is in its
    // own cell, so the four neighbours are read up front alongside the cell —
    // and read before anything moves, for the same reason everything else here
    // is: a parcel that shoved its neighbour first would make the answer depend
    // on where it sat in the list.
    let crowding: Vec<(Vec2, f32)> = field
        .parcels()
        .iter()
        .map(|parcel| {
            let Some((col, row)) = field.cell_at(parcel.at) else {
                return (Vec2::ZERO, 0.0);
            };
            let Some(cell) = field.cell_by(col, row) else {
                return (Vec2::ZERO, 0.0);
            };
            let here = cell.density;
            let size = field.cell_size();

            // Two terms, and both are needed.
            //
            // The **gradient** is what levels a pool: away from whichever
            // neighbour is fuller. On its own it is useless for a heap, because
            // a lone overdense cell surrounded by empty ones pushes equally in
            // all four directions and therefore not at all — the sum cancels
            // exactly, which is how the first version came to move a pile 0.55
            // pixels in ninety ticks.
            let mut push = Vec2::ZERO;
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let (nc, nr) = (col as i32 + dx, row as i32 + dy);
                if nc < 0 || nr < 0 {
                    continue;
                }
                let there = field
                    .cell_by(nc as usize, nr as usize)
                    .map_or(0.0, |cell| cell.density);
                let difference = here - there;
                if difference > 0.0 {
                    push += Vec2::new(-dx as f32, -dy as f32) * (difference / size);
                }
            }

            // The **centroid** term is what breaks a heap: away from where the
            // mass in this cell actually sits. It vanishes in an even pool,
            // because there a parcel already is where the mass is.
            let apart = (parcel.at - cell.centroid).normalize_or_zero();
            (push + apart * (here / size), here)
        })
        .collect();

    let gravity = rules.gravity;
    let mixing = (rules.mixing * dt).clamp(0.0, 1.0);
    let cooling = (rules.cooling * dt).clamp(0.0, 1.0);
    let pull = (rules.repetition * dt).clamp(0.0, 1.0);
    let floor = field.bounds().0.y + field.cell_size();

    let rest = rules.rest_density.max(f32::EPSILON);
    for ((parcel, (cell_temperature, cell_flow)), (crowd, density)) in
        field.parcels_mut().iter_mut().zip(seen).zip(crowding)
    {
        let material = materials.get(parcel.substance.as_str());
        parcel.life -= dt;

        // Archimedes. Positive falls, negative climbs, and nothing anywhere
        // says "flame goes up" — it comes out of the density.
        let lift = materials.buoyancy(parcel.substance.as_str(), parcel.temperature);
        parcel.velocity += gravity * (lift * dt);

        // Pressure. A liquid is nearly incompressible, and without a term
        // saying so water has no surface and no level — it collapses into
        // whichever cell is lowest and sits there as a dot. Scaled by how far
        // past the rest density the cell has been pushed, so a thin scatter
        // feels nothing and a heap pushes hard.
        // Only once the cell is actually crowded. Below the rest density a
        // liquid is happy and pushes on nothing, which is what stops a thin
        // scatter of droplets flying apart.
        if material.stiffness > 0.0 && density > rest {
            let over = ((density - rest) / rest).min(4.0);
            parcel.velocity += crowd.normalize_or_zero() * (material.stiffness * over * dt);
        }

        // Toward the mean flow of the cell. This is the liquid/gas difference:
        // water matches its neighbours and moves as a body, flame does not.
        parcel.velocity += (cell_flow - parcel.velocity) * (material.cohesion * dt).clamp(0.0, 1.0);

        // Swirl, scaled by how far above the room the parcel is. A cold flame
        // stops flickering, which is the same reason it stops rising.
        if material.turbulence > 0.0 {
            let heat = ((parcel.temperature - AMBIENT) / 200.0).clamp(0.0, 3.0);
            let noise = swirl(parcel.at.x, parcel.at.y, tick);
            parcel.velocity += noise * (material.turbulence * heat * dt);
        }

        parcel.velocity = parcel.velocity * (1.0 - material.drag * dt).clamp(0.0, 1.0);
        parcel.at += parcel.velocity * dt;

        // A solid that has come to rest on the floor stays there. **Ours**
        // (§2.6) and a shortcut: there is no contact model, so "slow and near
        // the bottom" stands in for "supported". Without it a heap of sand
        // jitters forever on the boundary and reads as broken.
        if material.phase.settles()
            && parcel.at.y <= floor
            && parcel.velocity.length_squared() < 400.0
        {
            parcel.velocity = parcel.velocity * (1.0 - material.friction).clamp(0.0, 1.0);
        }

        parcel.temperature += (cell_temperature - parcel.temperature) * mixing;
        parcel.temperature += (AMBIENT - parcel.temperature) * cooling;

        // Canon (§2.2): repetition "continuously resets affected objects to the
        // state they held when the spell took hold, temperature included". A
        // pull rather than a snap, so a spring is what it feels like — which is
        // canon's own word for it.
        if let Some(anchor) = parcel.anchor {
            parcel.at += (anchor.at - parcel.at) * pull;
            parcel.velocity += (anchor.velocity - parcel.velocity) * pull;
            parcel.temperature += (anchor.temperature - parcel.temperature) * pull;
        }
    }

    if rules.walls {
        field.confine(rules.restitution);
    } else {
        // No walls: what leaves is gone. Reported by the falling mass rather
        // than hidden, because silently losing matter is the one thing §3.2
        // cannot survive.
        field.spill();
    }
    field.sweep();
    // Merge before settling, so the cells describe the parcels that will
    // actually be there next tick — and so a reacting world stops growing.
    field.coalesce();
    field.scrub();
    field.settle();
}

#[cfg(test)]
#[path = "../tests/step.rs"]
mod tests;
