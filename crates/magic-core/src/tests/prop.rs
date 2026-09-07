//! Tests for `prop` — the things on the paper a spell can act on.

use super::*;
use crate::sim::field::Field;
use crate::sim::material::Materials;

fn book() -> Materials {
    Materials::parse(
        "materials.ron",
        include_str!("../../the-magic-assets/materials.ron"),
    )
    .expect("shipped materials must load")
}

fn paper() -> Field {
    Field::centered(24, 24.0, Vec2::ZERO).expect("a grid")
}

fn plank() -> Prop {
    Prop::new(
        SubstanceId::new("wood"),
        Vec2::ZERO,
        Vec2::new(20.0, 14.0),
        6.0,
    )
}

/// A flame parcel sitting on top of something.
fn flame_at(at: Vec2, mass: f32) -> Parcel {
    Parcel {
        temperature: 800.0,
        ..Parcel::new(SubstanceId::new("flame"), at, mass)
    }
}

fn run(props: &mut Vec<Prop>, field: &mut Field, ticks: usize) -> Vec<Event> {
    let rules = PropRules::default();
    let materials = book();
    let mut events = Vec::new();
    for _ in 0..ticks {
        step_props(props, field, &rules, &materials, 1.0 / 60.0, &mut events);
    }
    events
}

#[test]
fn a_plank_in_a_flame_catches_fire() {
    let mut field = paper();
    field.add(flame_at(Vec2::ZERO, 2.0));
    let mut props = vec![plank()];

    let events = run(&mut props, &mut field, 60);
    assert!(props[0].burning, "the plank never caught");
    assert!(
        events.iter().any(|e| matches!(e, Event::Ignite { .. })),
        "nothing announced the ignition"
    );
}

#[test]
fn a_plank_in_a_cold_room_never_catches() {
    let mut field = paper();
    let mut props = vec![plank()];
    run(&mut props, &mut field, 600);
    assert!(!props[0].burning);
    assert_eq!(props[0].integrity, 1.0);
}

#[test]
fn a_wet_plank_refuses_to_light() {
    // The rule worth having: water does not put a fire out by cooling it, it
    // stops the fuel taking in the first place.
    let mut field = paper();
    field.add(flame_at(Vec2::ZERO, 2.0));
    field.add(Parcel::new(SubstanceId::new("water"), Vec2::ZERO, 4.0));
    let mut props = vec![plank()];

    run(&mut props, &mut field, 120);
    assert!(props[0].wetness > PropRules::default().quench);
    assert!(!props[0].burning, "a soaked plank caught fire");
}

#[test]
fn water_puts_out_a_burning_plank() {
    let mut field = paper();
    field.add(flame_at(Vec2::ZERO, 2.0));
    let mut props = vec![plank()];
    run(&mut props, &mut field, 60);
    assert!(props[0].burning, "precondition: it has to be alight first");

    field.clear();
    field.add(Parcel::new(SubstanceId::new("water"), Vec2::ZERO, 6.0));
    let events = run(&mut props, &mut field, 60);
    assert!(!props[0].burning, "water did not put it out");
    assert!(events.iter().any(|e| matches!(e, Event::Douse { .. })));
}

#[test]
fn a_stone_block_never_burns_however_hot_it_gets() {
    // `ignites_at` being absent is the load-bearing part: a substance that does
    // not burn is one the data says nothing about, not one with a high number.
    let mut field = paper();
    field.add(flame_at(Vec2::ZERO, 8.0));
    let mut props = vec![Prop::new(
        SubstanceId::new("stone"),
        Vec2::ZERO,
        Vec2::new(20.0, 14.0),
        6.0,
    )];

    run(&mut props, &mut field, 600);
    assert!(props[0].temperature > 400.0, "it should still get hot");
    assert!(!props[0].burning);
    assert_eq!(props[0].integrity, 1.0);
}

#[test]
fn burning_conserves_mass_into_the_field() {
    // §3.2 with objects in the world: a plank that burns away puts exactly its
    // own mass into the field as exhaust, and mints nothing.
    let mut field = paper();
    field.add(flame_at(Vec2::ZERO, 1.0));
    let mut props = vec![plank()];
    let before = field.mass() + props[0].standing();

    run(&mut props, &mut field, 240);
    let after = field.mass() + props.iter().map(|p| p.standing()).sum::<f32>();
    assert!(
        (after - before).abs() < 0.05,
        "mass went from {before} to {after}"
    );
}

#[test]
fn a_plank_burns_all_the_way_down_and_says_so() {
    let mut field = paper();
    field.add(flame_at(Vec2::ZERO, 2.0));
    let mut props = vec![plank()];
    let events = run(&mut props, &mut field, 1200);
    assert!(props.is_empty(), "it should be gone");
    assert!(events.iter().any(|e| matches!(e, Event::Spent { .. })));
}

#[test]
fn moving_air_pushes_a_prop_along() {
    let mut field = paper();
    for n in 0..6 {
        field.add(Parcel {
            velocity: Vec2::new(240.0, 0.0),
            ..Parcel::new(
                SubstanceId::new("air"),
                Vec2::new(-30.0 + n as f32 * 6.0, 0.0),
                1.0,
            )
        });
    }
    let mut props = vec![plank()];
    run(&mut props, &mut field, 60);
    assert!(props[0].at.x > 1.0, "the wind moved it {}", props[0].at.x);
}

#[test]
fn light_falling_on_a_prop_is_measured_and_changes_nothing_else() {
    let mut field = paper();
    field.add(Parcel::new(SubstanceId::new("light"), Vec2::ZERO, 4.0));
    let mut props = vec![plank()];
    run(&mut props, &mut field, 60);
    assert!(props[0].lit > 0.5, "lit {}", props[0].lit);
    assert!(!props[0].burning, "light is not heat");
    assert_eq!(props[0].integrity, 1.0);
}

#[test]
fn scattering_is_the_same_every_run() {
    // §4.3: "random on the paper" is a hashed pure function of the index, so a
    // scattered board is something a test can be written against at all.
    let one = scatter(
        8,
        Vec2::new(-200.0, -150.0),
        Vec2::new(200.0, 150.0),
        SubstanceId::new("wood"),
        7,
    );
    let two = scatter(
        8,
        Vec2::new(-200.0, -150.0),
        Vec2::new(200.0, 150.0),
        SubstanceId::new("wood"),
        7,
    );
    assert_eq!(one, two);
    assert_eq!(one.len(), 8);
    for prop in &one {
        assert!(prop.at.x.abs() <= 200.0 && prop.at.y.abs() <= 150.0);
        assert!(prop.mass > 0.0);
    }
}

#[test]
fn a_different_seed_lays_out_different_kindling() {
    let one = scatter(
        6,
        Vec2::new(-200.0, -150.0),
        Vec2::new(200.0, 150.0),
        SubstanceId::new("wood"),
        1,
    );
    let two = scatter(
        6,
        Vec2::new(-200.0, -150.0),
        Vec2::new(200.0, 150.0),
        SubstanceId::new("wood"),
        2,
    );
    assert_ne!(one, two);
}
