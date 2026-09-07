//! Tests for `sim::vec2`.

use super::*;

#[test]
fn a_zero_vector_normalizes_to_zero_rather_than_nan() {
    assert_eq!(Vec2::ZERO.normalize_or_zero(), Vec2::ZERO);
}

#[test]
fn a_vector_of_infinities_normalizes_to_zero() {
    let wild = Vec2::new(f32::INFINITY, 1.0);
    assert_eq!(wild.normalize_or_zero(), Vec2::ZERO);
}

#[test]
fn a_zero_vector_has_no_heading() {
    // `atan2(-0.0, -0.0)` is -pi. A vector with no direction must not report a
    // firm one; `balance` shipped exactly this bug.
    assert_eq!(Vec2::ZERO.to_angle(), 0.0);
    assert_eq!(Vec2::new(-0.0, -0.0).to_angle(), 0.0);
}

#[test]
fn an_angle_survives_a_round_trip_through_a_vector() {
    for step in 0..12 {
        let angle = step as f32 * 0.5 - 3.0;
        let back = Vec2::from_angle(angle).to_angle();
        assert!((back - angle).abs() < 1e-4, "{angle} came back as {back}");
    }
}

#[test]
fn a_normalized_vector_has_length_one() {
    let unit = Vec2::new(3.0, -4.0).normalize_or_zero();
    assert!((unit.length() - 1.0).abs() < 1e-6);
}

#[test]
fn adding_a_vector_and_its_negation_gives_zero() {
    let v = Vec2::new(12.0, -5.0);
    assert_eq!(v + -v, Vec2::ZERO);
}

#[test]
fn length_and_length_squared_agree() {
    let v = Vec2::new(3.0, 4.0);
    assert_eq!(v.length(), 5.0);
    assert_eq!(v.length_squared(), 25.0);
}

#[test]
fn scaling_by_zero_gives_the_zero_vector() {
    assert_eq!(Vec2::new(9.0, 9.0) * 0.0, Vec2::ZERO);
}

#[test]
fn the_perpendicular_is_a_quarter_turn_and_keeps_its_length() {
    let v = Vec2::new(3.0, 4.0);
    let p = v.perpendicular();
    assert!(v.dot(p).abs() < 1e-5);
    assert!((p.length() - v.length()).abs() < 1e-5);
}

#[test]
fn add_assign_matches_add() {
    let mut a = Vec2::new(1.0, 2.0);
    a += Vec2::new(3.0, 4.0);
    assert_eq!(a, Vec2::new(1.0, 2.0) + Vec2::new(3.0, 4.0));
}
