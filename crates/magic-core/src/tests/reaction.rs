//! Tests for `sim::reaction` — M7, and the conservation it must not break.

use super::*;

use crate::sim::field::Field;
use crate::sim::parcel::{AMBIENT, Parcel, SubstanceId};
use crate::sim::vec2::Vec2;

const SHIPPED: &str = include_str!("../../the-magic-assets/reactions.ron");

fn book() -> ReactionBook {
    ReactionBook::parse("reactions.ron", SHIPPED).expect("shipped rules must load")
}

fn grid() -> Field {
    Field::new(8, 8, 20.0, Vec2::ZERO).expect("valid grid")
}

fn at(name: &str, at: Vec2, mass: f32, temperature: f32) -> Parcel {
    let mut parcel = Parcel::new(SubstanceId::new(name), at, mass);
    parcel.temperature = temperature;
    parcel
}

fn one_rule(source: &str) -> Result<ReactionBook, crate::CatalogError> {
    ReactionBook::parse("test.ron", source)
}

// ---- the loader is the guard, not the runtime -----------------------------

#[test]
fn the_shipped_rules_load() {
    assert!(!book().is_empty());
}

#[test]
fn a_rule_whose_shares_do_not_total_one_is_refused() {
    // The check the whole module rests on: a rule that mints mass would make
    // every conservation property in `sim` a lie.
    let bad = r#"(version: 1, reactions: [(
        id: "greedy", inputs: ["water"], outputs: [("steam", 1.5)],
        heat: 0.0, rate: 1.0,
    )])"#;
    assert!(one_rule(bad).is_err());
}

#[test]
fn a_rule_that_destroys_mass_is_refused_too() {
    let bad = r#"(version: 1, reactions: [(
        id: "leaky", inputs: ["water"], outputs: [("steam", 0.5)],
        heat: 0.0, rate: 1.0,
    )])"#;
    assert!(one_rule(bad).is_err());
}

#[test]
fn shares_across_several_products_may_total_one() {
    let fine = r#"(version: 1, reactions: [(
        id: "split", inputs: ["stone"], outputs: [("sand", 0.5), ("stone", 0.5)],
        heat: 0.0, rate: 1.0,
    )])"#;
    assert!(one_rule(fine).is_ok());
}

#[test]
fn a_rule_with_no_inputs_is_refused() {
    let bad = r#"(version: 1, reactions: [(
        id: "nothing", inputs: [], outputs: [("steam", 1.0)], heat: 0.0, rate: 1.0,
    )])"#;
    assert!(one_rule(bad).is_err());
}

#[test]
fn a_rule_that_can_never_fire_is_refused() {
    let bad = r#"(version: 1, reactions: [(
        id: "impossible", inputs: ["water"], outputs: [("steam", 1.0)],
        min_temp: Some(100.0), max_temp: Some(0.0), heat: 0.0, rate: 1.0,
    )])"#;
    assert!(one_rule(bad).is_err());
}

#[test]
fn a_file_from_another_version_is_refused() {
    assert!(one_rule("(version: 99, reactions: [])").is_err());
}

// ---- phase change: one input, a temperature window ------------------------

#[test]
fn water_above_boiling_becomes_steam() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 150.0));
    field.settle();
    let report = react(&mut field, &book(), 1.0);

    assert!(report.happened());
    assert!(field.mass_of(&SubstanceId::new("steam")) > 0.0);
}

#[test]
fn water_below_boiling_stays_water() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 40.0));
    field.settle();
    react(&mut field, &book(), 1.0);
    assert_eq!(field.mass_of(&SubstanceId::new("steam")), 0.0);
}

#[test]
fn water_below_freezing_becomes_ice() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, -20.0));
    field.settle();
    react(&mut field, &book(), 1.0);
    assert!(field.mass_of(&SubstanceId::new("ice")) > 0.0);
}

#[test]
fn ice_above_freezing_melts_back() {
    let mut field = grid();
    field.add(at("ice", Vec2::new(30.0, 30.0), 10.0, 20.0));
    field.settle();
    react(&mut field, &book(), 1.0);
    assert!(field.mass_of(&SubstanceId::new("water")) > 0.0);
}

// ---- reactions: two inputs, and only where they actually meet --------------

#[test]
fn water_thrown_on_flame_becomes_steam() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 200.0));
    field.add(at("flame", Vec2::new(32.0, 31.0), 10.0, 200.0));
    field.settle();
    let report = react(&mut field, &book(), 1.0);

    assert!(report.happened());
    assert!(field.mass_of(&SubstanceId::new("steam")) > 0.0);
}

#[test]
fn substances_in_different_cells_do_not_meet() {
    // A reaction is something that happens to a *place*.
    //
    // At 80 degrees so the water cannot simply boil on its own — the first
    // version of this test used 200 and passed for the wrong reason, measuring
    // a phase change rather than a meeting.
    let mut field = grid();
    field.add(at("water", Vec2::new(10.0, 10.0), 10.0, 80.0));
    field.add(at("flame", Vec2::new(150.0, 150.0), 10.0, 80.0));
    field.settle();
    react(&mut field, &book(), 1.0);
    assert_eq!(field.mass_of(&SubstanceId::new("steam")), 0.0);
}

#[test]
fn the_scarcer_input_sets_the_pace() {
    // One drop of water cannot consume a bonfire.
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 0.1, 200.0));
    field.add(at("flame", Vec2::new(30.0, 30.0), 100.0, 200.0));
    field.settle();
    react(&mut field, &book(), 1.0);
    assert!(field.mass_of(&SubstanceId::new("flame")) > 99.0);
}

#[test]
fn air_feeds_flame_rather_than_smothering_it() {
    let mut field = grid();
    field.add(at("flame", Vec2::new(30.0, 30.0), 5.0, 300.0));
    field.add(at("air", Vec2::new(30.0, 30.0), 5.0, 300.0));
    let before = field.mass_of(&SubstanceId::new("flame"));
    field.settle();
    react(&mut field, &book(), 1.0);
    assert!(field.mass_of(&SubstanceId::new("flame")) > before);
}

// ---- conservation ---------------------------------------------------------

#[test]
fn every_reaction_conserves_mass() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 200.0));
    field.add(at("flame", Vec2::new(30.0, 30.0), 10.0, 200.0));
    field.add(at("stone", Vec2::new(70.0, 70.0), 4.0, 20.0));
    field.add(at("air", Vec2::new(70.0, 70.0), 4.0, 20.0));
    field.settle();

    let before = field.mass();
    for _ in 0..50 {
        react(&mut field, &book(), 1.0 / 60.0);
    }
    assert!(
        (field.mass() - before).abs() < 1e-2,
        "{before} became {}",
        field.mass()
    );
}

#[test]
fn a_reaction_conserves_momentum() {
    // Two parcels meeting head-on leave as one going nowhere.
    let mut field = grid();
    let mut left = at("water", Vec2::new(30.0, 30.0), 10.0, 200.0);
    left.velocity = Vec2::new(50.0, 0.0);
    let mut right = at("flame", Vec2::new(30.0, 30.0), 10.0, 200.0);
    right.velocity = Vec2::new(-50.0, 0.0);
    field.add(left);
    field.add(right);
    field.settle();

    let before = field.momentum();
    react(&mut field, &book(), 1.0);
    let after = field.momentum();
    assert!(
        (after - before).length() < 1e-2,
        "{before:?} became {after:?}"
    );
}

#[test]
fn boiling_takes_heat_out_of_the_world() {
    // Latent heat is what makes a kettle plateau instead of racing away.
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 150.0));
    field.settle();
    let report = react(&mut field, &book(), 1.0);
    assert!(report.heat < 0.0, "boiling released heat: {}", report.heat);
}

#[test]
fn freezing_puts_heat_back_into_it() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, -20.0));
    field.settle();
    let report = react(&mut field, &book(), 1.0);
    assert!(report.heat > 0.0);
}

// ---- §4.3 -----------------------------------------------------------------

#[test]
fn reacting_is_deterministic() {
    let run = || {
        let mut field = grid();
        field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 200.0));
        field.add(at("flame", Vec2::new(31.0, 30.0), 7.0, 200.0));
        field.add(at("ice", Vec2::new(31.0, 31.0), 3.0, 200.0));
        field.settle();
        for _ in 0..40 {
            react(&mut field, &book(), 1.0 / 60.0);
        }
        field
    };
    assert_eq!(run(), run());
}

#[test]
fn an_empty_book_does_nothing() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 500.0));
    field.settle();
    let before = field.clone();
    assert!(!react(&mut field, &ReactionBook::default(), 1.0).happened());
    assert_eq!(field, before);
}

#[test]
fn a_step_of_no_length_reacts_nothing() {
    let mut field = grid();
    field.add(at("water", Vec2::new(30.0, 30.0), 10.0, 500.0));
    field.settle();
    assert!(!react(&mut field, &book(), 0.0).happened());
}

#[test]
fn every_substance_a_rule_names_is_listed_once() {
    let named = book().substances();
    let mut sorted = named.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(named, sorted);
    assert!(named.contains(&"steam".to_string()));
}

#[test]
fn an_untouched_substance_is_left_alone() {
    let mut field = grid();
    field.add(at("light", Vec2::new(30.0, 30.0), 10.0, AMBIENT));
    field.settle();
    assert!(!react(&mut field, &book(), 1.0).happened());
}
