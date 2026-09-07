//! Tests for `props` — how a prop looks, and where a handful lands.

use super::*;
use magic_core::sim::{Prop, PropRules, SubstanceId, step_props};

fn plank() -> Prop {
    Prop::new(
        SubstanceId::new("wood"),
        magic_core::Vec2::ZERO,
        magic_core::Vec2::new(20.0, 14.0),
        6.0,
    )
}

#[test]
fn a_burning_prop_looks_hotter_than_a_cold_one() {
    let cold = look(&plank());
    let mut hot = plank();
    hot.burning = true;
    hot.temperature = 700.0;
    let alight = look(&hot);
    assert!(
        alight.to_srgba().red > cold.to_srgba().red,
        "a burning plank has to read warmer"
    );
}

#[test]
fn a_wet_prop_looks_darker_than_a_dry_one() {
    let dry = look(&plank());
    let mut soaked = plank();
    soaked.wetness = 1.0;
    let wet = look(&soaked);
    let lum = |c: Color| {
        let s = c.to_srgba();
        s.red + s.green + s.blue
    };
    assert!(lum(wet) < lum(dry), "wet timber is darker, not lighter");
}

#[test]
fn a_spent_prop_fades_out() {
    let mut worn = plank();
    worn.integrity = 0.05;
    assert!(look(&worn).alpha() < look(&plank()).alpha());
}

#[test]
fn an_unknown_substance_gets_a_plain_grey_rather_than_a_guess() {
    // So adding one to `materials.ron` shows up as something visible.
    let grey = prop_color("gravitonium").to_srgba();
    assert!((grey.red - grey.green).abs() < 0.15 && (grey.green - grey.blue).abs() < 0.15);
}

#[test]
fn props_do_not_move_or_burn_on_their_own() {
    // The property that makes them safe to leave on the paper: nothing happens
    // to a prop until a spell happens to it.
    let materials = magic_core::sim::Materials::parse(
        "materials.ron",
        include_str!("../../../../crates/magic-core/the-magic-assets/materials.ron"),
    )
    .expect("shipped materials");
    let mut field =
        magic_core::sim::Field::centered(24, 24.0, magic_core::Vec2::ZERO).expect("a grid");
    let mut props = vec![plank()];
    let before = props[0].clone();
    let mut events = Vec::new();
    for _ in 0..600 {
        step_props(
            &mut props,
            &mut field,
            &PropRules::default(),
            &materials,
            1.0 / 60.0,
            &mut events,
        );
    }
    assert_eq!(props[0].at, before.at);
    assert_eq!(props[0].integrity, 1.0);
    assert!(events.is_empty());
}
