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
            // Stiff on purpose. At 6.0 a held parcel sagged 19px under gravity
            // before the anchor balanced it, which is a sag, not a reset —
            // canon calls repetition spring-like and has it holding rot and
            // damage off entirely. 20.0 settles inside a pixel.
            repetition: 20.0,
        }
    }
}

/// Advances `field` by one tick.
///
/// Order matters and is the readable part: settle so every parcel can see the
/// cell it is in, then move and mix, then confine, then drop what expired, then
/// settle again so the cells describe where things ended up.
pub fn step(field: &mut Field, rules: &SimRules) {
    let dt = rules.dt;
    if !dt.is_finite() || dt <= 0.0 {
        return;
    }

    field.settle();

    // Read every cell a parcel needs *before* mutating, because a parcel
    // changing its own cell mid-pass would make the answer depend on the order
    // the parcels happen to sit in — the same trap as M5.6.
    let ambient_cells: Vec<(f32, Vec2)> = field
        .parcels()
        .iter()
        .map(|parcel| {
            field
                .cell(parcel.at)
                .map(|cell| (cell.temperature, cell.flow))
                .unwrap_or((AMBIENT, Vec2::ZERO))
        })
        .collect();

    let gravity = rules.gravity;
    let drag = (1.0 - rules.drag * dt).clamp(0.0, 1.0);
    let mixing = (rules.mixing * dt).clamp(0.0, 1.0);
    let cooling = (rules.cooling * dt).clamp(0.0, 1.0);
    let pull = (rules.repetition * dt).clamp(0.0, 1.0);

    for (parcel, (cell_temperature, _cell_flow)) in
        field.parcels_mut().iter_mut().zip(ambient_cells)
    {
        parcel.life -= dt;

        parcel.velocity += gravity * dt;
        parcel.velocity = parcel.velocity * drag;
        parcel.at += parcel.velocity * dt;

        // Toward the cell it shares with its neighbours, then toward the room.
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

    field.confine();
    field.sweep();
    field.settle();
}

#[cfg(test)]
#[path = "../tests/step.rs"]
mod tests;
