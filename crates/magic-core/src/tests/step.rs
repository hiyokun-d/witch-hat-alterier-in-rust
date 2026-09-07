//! Tests for `sim::step`.

use super::*;
use crate::sim::parcel::{AMBIENT, Parcel, SubstanceId};
use crate::sim::vec2::Vec2;

fn grid() -> Field {
    Field::new(16, 16, 20.0, Vec2::new(-160.0, -160.0)).expect("valid grid")
}

fn air(at: Vec2, mass: f32) -> Parcel {
    Parcel::new(SubstanceId::new("air"), at, mass)
}

#[test]
fn gravity_pulls_a_still_parcel_downward() {
    let mut field = grid();
    field.add(air(Vec2::ZERO, 1.0));
    step(&mut field, &SimRules::default());
    assert!(field.parcels()[0].velocity.y < 0.0);
}

#[test]
fn a_step_of_zero_length_changes_nothing() {
    let mut field = grid();
    field.add(air(Vec2::ZERO, 1.0));
    let before = field.clone();
    let rules = SimRules {
        dt: 0.0,
        ..SimRules::default()
    };
    step(&mut field, &rules);
    assert_eq!(field, before);
}

#[test]
fn the_same_field_stepped_twice_lands_in_the_same_place() {
    // §4.3: same inputs, same outputs, every run.
    let mut a = grid();
    let mut b = grid();
    for i in 0..5 {
        a.add(air(Vec2::new(i as f32 * 3.0, 10.0), 1.0 + i as f32));
        b.add(air(Vec2::new(i as f32 * 3.0, 10.0), 1.0 + i as f32));
    }
    for _ in 0..120 {
        step(&mut a, &SimRules::default());
        step(&mut b, &SimRules::default());
    }
    assert_eq!(a, b);
}

#[test]
fn simulation_state_at_frame_n_is_identical_across_runs() {
    let run = || {
        let mut field = grid();
        for i in 0..8 {
            let mut p = air(Vec2::new(i as f32 * 7.0 - 20.0, 40.0), 1.0);
            p.velocity = Vec2::new(i as f32 * 3.0, -10.0);
            field.add(p);
        }
        for _ in 0..90 {
            step(&mut field, &SimRules::default());
        }
        field
    };
    assert_eq!(run(), run());
}

#[test]
fn stepping_never_creates_or_loses_mass() {
    let mut field = grid();
    field.add(air(Vec2::new(0.0, 100.0), 3.0));
    field.add(air(Vec2::new(40.0, 20.0), 2.0));
    for _ in 0..200 {
        step(&mut field, &SimRules::default());
    }
    assert!((field.mass() - 5.0).abs() < 1e-3, "mass {}", field.mass());
}

#[test]
fn a_parcel_never_leaves_the_field() {
    let mut field = grid();
    let mut fast = air(Vec2::ZERO, 1.0);
    fast.velocity = Vec2::new(9000.0, 9000.0);
    field.add(fast);
    for _ in 0..60 {
        step(&mut field, &SimRules::default());
    }
    assert!(field.contains(field.parcels()[0].at));
}

#[test]
fn a_parcel_whose_life_runs_out_is_swept_away() {
    let mut field = grid();
    let mut brief = air(Vec2::ZERO, 1.0);
    brief.life = 0.05;
    field.add(brief);
    for _ in 0..10 {
        step(&mut field, &SimRules::default());
    }
    assert!(field.parcels().is_empty());
}

#[test]
fn a_hot_parcel_cools_toward_the_room() {
    let mut field = grid();
    let mut hot = air(Vec2::ZERO, 1.0);
    hot.temperature = 500.0;
    field.add(hot);
    // Twenty seconds. Cooling is 0.4/s toward ambient, so ten seconds still
    // leaves nine degrees on it — the test was measuring impatience.
    for _ in 0..1200 {
        step(&mut field, &SimRules::default());
    }
    assert!(
        (field.parcels()[0].temperature - AMBIENT).abs() < 5.0,
        "still at {}",
        field.parcels()[0].temperature
    );
}

#[test]
fn two_parcels_in_one_cell_even_out_their_temperatures() {
    let mut field = grid();
    let mut hot = air(Vec2::new(0.0, 0.0), 1.0);
    hot.temperature = 200.0;
    let mut cold = air(Vec2::new(1.0, 0.0), 1.0);
    cold.temperature = 0.0;
    // No gravity, so they stay in the same cell and only heat is under test.
    let rules = SimRules {
        gravity: Vec2::ZERO,
        cooling: 0.0,
        ..SimRules::default()
    };
    field.add(hot);
    field.add(cold);
    for _ in 0..60 {
        step(&mut field, &rules);
    }
    let gap = (field.parcels()[0].temperature - field.parcels()[1].temperature).abs();
    assert!(gap < 20.0, "still {gap} apart");
}

#[test]
fn a_repetition_anchor_returns_a_parcel_to_where_it_started() {
    // Canon (§2.2): resets affected objects to the state they held when the
    // spell took hold, temperature included.
    let mut field = grid();
    let mut held = air(Vec2::new(10.0, 10.0), 1.0);
    held.temperature = 60.0;
    held.anchor_here();
    field.add(held);

    for _ in 0..120 {
        step(&mut field, &SimRules::default());
    }

    let parcel = &field.parcels()[0];
    assert!(
        (parcel.at - Vec2::new(10.0, 10.0)).length() < 3.0,
        "drifted to {:?}",
        parcel.at
    );
    assert!(
        (parcel.temperature - 60.0).abs() < 5.0,
        "temperature {}",
        parcel.temperature
    );
}

#[test]
fn an_unanchored_parcel_does_fall_away() {
    // The control for the test above: without an anchor, gravity wins.
    let mut field = grid();
    field.add(air(Vec2::new(10.0, 10.0), 1.0));
    for _ in 0..120 {
        step(&mut field, &SimRules::default());
    }
    assert!(field.parcels()[0].at.y < 0.0);
}

#[test]
fn cells_agree_with_parcels_after_a_step() {
    let mut field = grid();
    field.add(air(Vec2::new(0.0, 60.0), 4.0));
    step(&mut field, &SimRules::default());
    let total: f32 = (0..field.width())
        .flat_map(|col| (0..field.height()).map(move |row| (col, row)))
        .map(|(col, row)| field.cell_by(col, row).unwrap().density)
        .sum();
    assert!((total - field.mass()).abs() < 1e-3);
}
