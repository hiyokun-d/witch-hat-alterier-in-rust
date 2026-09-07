//! Tests for `sim::parcel`.

use super::*;

fn air(at: Vec2, mass: f32) -> Parcel {
    Parcel::new(SubstanceId::new("air"), at, mass)
}

#[test]
fn a_new_parcel_is_still_and_at_ambient_temperature() {
    let p = air(Vec2::ZERO, 1.0);
    assert_eq!(p.velocity, Vec2::ZERO);
    assert_eq!(p.temperature, AMBIENT);
    assert!(p.life.is_infinite());
    assert!(p.anchor.is_none());
}

#[test]
fn momentum_is_zero_for_a_parcel_that_is_not_moving() {
    assert_eq!(air(Vec2::ZERO, 7.0).momentum(), Vec2::ZERO);
}

#[test]
fn heat_is_mass_times_temperature_not_temperature_alone() {
    // The distinction that stops a speck cooling a boulder.
    let mut speck = air(Vec2::ZERO, 0.1);
    speck.temperature = 100.0;
    let mut boulder = air(Vec2::ZERO, 100.0);
    boulder.temperature = 20.0;
    assert!(boulder.heat() > speck.heat());
}

#[test]
fn a_parcel_with_no_mass_is_not_alive() {
    assert!(!air(Vec2::ZERO, 0.0).alive());
}

#[test]
fn a_parcel_whose_life_ran_out_is_not_alive() {
    let mut p = air(Vec2::ZERO, 1.0);
    p.life = 0.0;
    assert!(!p.alive());
}

#[test]
fn anchoring_records_the_state_held_at_that_moment() {
    // Canon: repetition resets to the state held when the spell took hold,
    // temperature included.
    let mut p = air(Vec2::new(5.0, 6.0), 1.0);
    p.velocity = Vec2::new(1.0, 2.0);
    p.temperature = 90.0;
    p.anchor_here();

    let anchor = p.anchor.expect("anchor was set");
    assert_eq!(anchor.at, Vec2::new(5.0, 6.0));
    assert_eq!(anchor.velocity, Vec2::new(1.0, 2.0));
    assert_eq!(anchor.temperature, 90.0);
}
