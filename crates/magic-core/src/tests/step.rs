//! Tests for `sim::step`.

use super::*;
use crate::sim::material::Materials;
use crate::sim::parcel::{AMBIENT, Parcel, SubstanceId};
use crate::sim::vec2::Vec2;

const MATERIALS: &str = include_str!("../../the-magic-assets/materials.ron");

fn stuff() -> Materials {
    Materials::parse("materials.ron", MATERIALS).expect("shipped materials must load")
}

fn grid() -> Field {
    Field::new(16, 16, 20.0, Vec2::new(-160.0, -160.0)).expect("valid grid")
}

/// A plain falling parcel.
///
/// Water rather than air, and the change is the point: since `material.rs`,
/// air at room temperature is neutrally buoyant *against air* and does not
/// fall at all. Three tests here used `air` as a stand-in for "a thing" and
/// started failing the moment substances became specific — which is exactly
/// what was wanted.
fn drop_of(at: Vec2, mass: f32) -> Parcel {
    Parcel::new(SubstanceId::new("water"), at, mass)
}

fn made_of(name: &str, at: Vec2, mass: f32, temperature: f32) -> Parcel {
    let mut parcel = Parcel::new(SubstanceId::new(name), at, mass);
    parcel.temperature = temperature;
    parcel
}

#[test]
fn gravity_pulls_a_still_parcel_downward() {
    let mut field = grid();
    field.add(drop_of(Vec2::ZERO, 1.0));
    step(&mut field, &SimRules::default(), &stuff(), 0);
    assert!(field.parcels()[0].velocity.y < 0.0);
}

#[test]
fn a_step_of_zero_length_changes_nothing() {
    let mut field = grid();
    field.add(drop_of(Vec2::ZERO, 1.0));
    let before = field.clone();
    let rules = SimRules {
        dt: 0.0,
        ..SimRules::default()
    };
    step(&mut field, &rules, &stuff(), 0);
    assert_eq!(field, before);
}

#[test]
fn the_same_field_stepped_twice_lands_in_the_same_place() {
    // §4.3: same inputs, same outputs, every run.
    let mut a = grid();
    let mut b = grid();
    for i in 0..5 {
        a.add(drop_of(Vec2::new(i as f32 * 3.0, 10.0), 1.0 + i as f32));
        b.add(drop_of(Vec2::new(i as f32 * 3.0, 10.0), 1.0 + i as f32));
    }
    for _ in 0..120 {
        step(&mut a, &SimRules::default(), &stuff(), 0);
        step(&mut b, &SimRules::default(), &stuff(), 0);
    }
    assert_eq!(a, b);
}

#[test]
fn simulation_state_at_frame_n_is_identical_across_runs() {
    let run = || {
        let mut field = grid();
        for i in 0..8 {
            let mut p = drop_of(Vec2::new(i as f32 * 7.0 - 20.0, 40.0), 1.0);
            p.velocity = Vec2::new(i as f32 * 3.0, -10.0);
            field.add(p);
        }
        for _ in 0..90 {
            step(&mut field, &SimRules::default(), &stuff(), 0);
        }
        field
    };
    assert_eq!(run(), run());
}

#[test]
fn stepping_never_creates_or_loses_mass() {
    let mut field = grid();
    field.add(drop_of(Vec2::new(0.0, 100.0), 3.0));
    field.add(drop_of(Vec2::new(40.0, 20.0), 2.0));
    for _ in 0..200 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!((field.mass() - 5.0).abs() < 1e-3, "mass {}", field.mass());
}

#[test]
fn a_parcel_never_leaves_the_field() {
    let mut field = grid();
    let mut fast = drop_of(Vec2::ZERO, 1.0);
    fast.velocity = Vec2::new(9000.0, 9000.0);
    field.add(fast);
    for _ in 0..60 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(field.contains(field.parcels()[0].at));
}

#[test]
fn a_parcel_whose_life_runs_out_is_swept_away() {
    let mut field = grid();
    let mut brief = drop_of(Vec2::ZERO, 1.0);
    brief.life = 0.05;
    field.add(brief);
    for _ in 0..10 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(field.parcels().is_empty());
}

#[test]
fn a_hot_parcel_cools_toward_the_room() {
    let mut field = grid();
    let mut hot = drop_of(Vec2::ZERO, 1.0);
    hot.temperature = 500.0;
    field.add(hot);
    // Twenty seconds. Cooling is 0.4/s toward ambient, so ten seconds still
    // leaves nine degrees on it — the test was measuring impatience.
    for _ in 0..1200 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
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
    let mut hot = drop_of(Vec2::new(0.0, 0.0), 1.0);
    hot.temperature = 200.0;
    let mut cold = drop_of(Vec2::new(1.0, 0.0), 1.0);
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
        step(&mut field, &rules, &stuff(), 0);
    }
    let gap = (field.parcels()[0].temperature - field.parcels()[1].temperature).abs();
    assert!(gap < 20.0, "still {gap} apart");
}

#[test]
fn a_repetition_anchor_returns_a_parcel_to_where_it_started() {
    // Canon (§2.2): resets affected objects to the state they held when the
    // spell took hold, temperature included.
    let mut field = grid();
    let mut held = drop_of(Vec2::new(10.0, 10.0), 1.0);
    held.temperature = 60.0;
    held.anchor_here();
    field.add(held);

    for _ in 0..120 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
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
    field.add(drop_of(Vec2::new(10.0, 10.0), 1.0));
    for _ in 0..120 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(field.parcels()[0].at.y < 0.0);
}

#[test]
fn cells_agree_with_parcels_after_a_step() {
    let mut field = grid();
    field.add(drop_of(Vec2::new(0.0, 60.0), 4.0));
    step(&mut field, &SimRules::default(), &stuff(), 0);
    let total: f32 = (0..field.width())
        .flat_map(|col| (0..field.height()).map(move |row| (col, row)))
        .map(|(col, row)| field.cell_by(col, row).unwrap().density)
        .sum();
    assert!((total - field.mass()).abs() < 1e-3);
}

// ---- what materials changed -----------------------------------------------

#[test]
fn air_at_room_temperature_neither_rises_nor_falls() {
    // Archimedes on a parcel of air, in air: the bracket is exactly zero. This
    // is the fact that broke three older tests, and it is correct.
    let mut field = grid();
    field.add(made_of("air", Vec2::ZERO, 1.0, AMBIENT));
    for _ in 0..60 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(
        field.parcels()[0].at.y.abs() < 30.0,
        "drifted to {}",
        field.parcels()[0].at.y
    );
}

#[test]
fn hot_flame_climbs_against_gravity() {
    // Nothing in the code says "flame goes up". It comes out of the density.
    let mut field = grid();
    field.add(made_of("flame", Vec2::ZERO, 1.0, 800.0));
    for _ in 0..20 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(field.parcels()[0].at.y > 0.0, "flame did not rise");
}

#[test]
fn water_falls_while_flame_rises_from_the_same_place() {
    // The whole complaint, as one assertion: two substances, one starting
    // point, opposite behaviour, and no rule anywhere naming either of them.
    let mut field = grid();
    field.add(made_of("water", Vec2::ZERO, 1.0, AMBIENT));
    field.add(made_of("flame", Vec2::ZERO, 1.0, 800.0));
    for _ in 0..25 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(field.parcels()[0].at.y < field.parcels()[1].at.y);
}

#[test]
fn steam_rises_even_after_it_has_cooled() {
    // Not because it is hot. Because a water molecule is lighter than the air
    // it displaces.
    let mut field = grid();
    field.add(made_of("steam", Vec2::ZERO, 1.0, AMBIENT));
    for _ in 0..60 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(field.parcels()[0].at.y > 0.0);
}

#[test]
fn a_flame_that_cools_stops_rising() {
    // The consequence worth having: the effect fades on its own, because the
    // cause fades, and not because anything counts it down.
    //
    // Two fields rather than one flame measured twice — the first version let
    // the hot flame reach the ceiling, where the wall turned its velocity
    // negative and made "slower than before" quietly false.
    let height = |temperature: f32| {
        let mut field = grid();
        field.add(made_of("flame", Vec2::new(0.0, -100.0), 1.0, temperature));
        for tick in 0..40 {
            step(&mut field, &SimRules::default(), &stuff(), tick);
        }
        field.parcels()[0].at.y
    };
    assert!(height(AMBIENT) < height(800.0));
}

#[test]
fn a_stone_lands_and_stays_put() {
    let mut field = grid();
    field.add(made_of("stone", Vec2::new(0.0, 100.0), 1.0, AMBIENT));
    for _ in 0..240 {
        step(&mut field, &SimRules::default(), &stuff(), 0);
    }
    assert!(
        field.parcels()[0].velocity.length() < 30.0,
        "still moving at {}",
        field.parcels()[0].velocity.length()
    );
}

#[test]
fn turbulence_is_deterministic_not_random() {
    // §4.3. The swirl is a pure function of position and tick, so the same
    // frame is the same frame - which is the only reason a flame is allowed to
    // flicker at all.
    let run = || {
        let mut field = grid();
        field.add(made_of("flame", Vec2::ZERO, 1.0, 800.0));
        for tick in 0..90 {
            step(&mut field, &SimRules::default(), &stuff(), tick);
        }
        field
    };
    assert_eq!(run(), run());
}

#[test]
fn a_flame_wanders_where_a_stream_of_water_does_not() {
    // Turbulence, seen. Both start still; only one has any reason to move
    // sideways.
    let sideways = |name: &str, temperature: f32| {
        let mut field = grid();
        field.add(made_of(name, Vec2::ZERO, 1.0, temperature));
        for tick in 0..90 {
            step(&mut field, &SimRules::default(), &stuff(), tick);
        }
        field.parcels().first().map_or(0.0, |p| p.at.x.abs())
    };
    assert!(sideways("flame", 800.0) > sideways("water", AMBIENT));
}
