//! Tests for `sim::material` — and the one formula that makes fire rise.

use super::*;

const SHIPPED: &str = include_str!("../../the-magic-assets/materials.ron");

fn book() -> Materials {
    Materials::parse("materials.ron", SHIPPED).expect("shipped materials must load")
}

#[test]
fn the_shipped_materials_load() {
    assert!(!book().is_empty());
}

#[test]
fn a_file_from_another_version_is_refused() {
    assert!(
        Materials::parse(
            "test.ron",
            "(version: 9, ambient_density: 1.2, reference_temp: 20.0, materials: [])"
        )
        .is_err()
    );
}

#[test]
fn a_falling_substance_with_no_density_is_refused() {
    // It would divide by zero in the buoyancy term.
    let bad = r#"(version: 1, ambient_density: 1.2, reference_temp: 20.0, materials: [
        (id: "void", density: 0.0, phase: Solid, drag: 1.0, cohesion: 1.0)
    ])"#;
    assert!(Materials::parse("test.ron", bad).is_err());
}

#[test]
fn a_substance_named_twice_is_refused() {
    let bad = r#"(version: 1, ambient_density: 1.2, reference_temp: 20.0, materials: [
        (id: "water", density: 1000.0, phase: Liquid, drag: 1.0, cohesion: 1.0),
        (id: "water", density: 900.0, phase: Liquid, drag: 1.0, cohesion: 1.0)
    ])"#;
    assert!(Materials::parse("test.ron", bad).is_err());
}

#[test]
fn an_unknown_substance_still_behaves_like_something() {
    // Silently not moving is the hardest kind of bug to see.
    let book = book();
    assert!(!book.describes("mithril"));
    assert!(book.get("mithril").density > 0.0);
}

// ---- the ideal gas law ----------------------------------------------------

#[test]
fn a_gas_thins_as_it_heats() {
    let book = book();
    let flame = book.get("flame");
    let cold = flame.density_at(20.0, book.reference_temp);
    let hot = flame.density_at(800.0, book.reference_temp);
    assert!(hot < cold, "{hot} should be thinner than {cold}");
}

#[test]
fn a_liquid_does_not() {
    // Water does expand with heat, by a fraction of a percent. Modelling it
    // would be noise dressed as rigour.
    let book = book();
    let water = book.get("water");
    assert_eq!(
        water.density_at(20.0, book.reference_temp),
        water.density_at(90.0, book.reference_temp)
    );
}

#[test]
fn a_gas_at_the_reference_temperature_is_at_its_quoted_density() {
    let book = book();
    let air = book.get("air");
    assert!((air.density_at(book.reference_temp, book.reference_temp) - 1.2).abs() < 1e-3);
}

#[test]
fn flame_at_eight_hundred_degrees_is_about_a_third_of_the_room() {
    // The number the whole effect rests on: 293K over 1073K.
    let book = book();
    let hot = book.get("flame").density_at(800.0, book.reference_temp);
    assert!((0.25..0.40).contains(&hot), "got {hot}");
}

// ---- Archimedes -----------------------------------------------------------

#[test]
fn water_falls_at_very_nearly_full_gravity() {
    let lift = book().buoyancy("water", 20.0);
    assert!((lift - 1.0).abs() < 0.01, "got {lift}");
}

#[test]
fn stone_falls_faster_than_water_does_not() {
    // Both fall at g. Buoyancy is a *ratio*, not a mass, and Galileo was right.
    let book = book();
    assert!(book.buoyancy("stone", 20.0) > 0.99);
    assert!(book.buoyancy("water", 20.0) > 0.99);
}

#[test]
fn hot_flame_climbs() {
    assert!(book().buoyancy("flame", 800.0) < 0.0);
}

#[test]
fn flame_climbs_harder_the_hotter_it_is() {
    let book = book();
    assert!(book.buoyancy("flame", 900.0) < book.buoyancy("flame", 300.0));
}

#[test]
fn flame_that_has_cooled_to_room_temperature_stops_climbing() {
    // The consequence worth having: nothing says "flame goes up", so nothing
    // has to say "flame stops going up" either.
    let lift = book().buoyancy("flame", 20.0);
    assert!(lift.abs() < 0.01, "got {lift}");
}

#[test]
fn steam_rises_even_once_it_is_cold() {
    // Not because it is hot - because a water molecule is lighter than the air
    // it displaces. This is the test that proves buoyancy is not a heat rule.
    assert!(book().buoyancy("steam", 20.0) < 0.0);
}

#[test]
fn ice_is_lighter_than_water_which_is_why_it_floats() {
    let book = book();
    assert!(book.get("ice").density < book.get("water").density);
}

#[test]
fn light_neither_falls_nor_floats() {
    assert_eq!(book().buoyancy("light", 500.0), 0.0);
}

#[test]
fn buoyancy_is_clamped_so_nothing_leaves_the_screen() {
    // Ours: the physics has no such limit, the display does.
    assert!(book().buoyancy("flame", 5000.0) >= -6.0);
}

// ---- phases ---------------------------------------------------------------

#[test]
fn a_liquid_holds_together_harder_than_a_gas() {
    // The whole difference between water pooling and flame dispersing.
    let book = book();
    assert!(book.get("water").cohesion > book.get("flame").cohesion);
}

#[test]
fn only_fire_and_its_relatives_are_turbulent() {
    let book = book();
    assert!(book.get("flame").turbulence > 0.0);
    assert_eq!(book.get("water").turbulence, 0.0);
    assert_eq!(book.get("stone").turbulence, 0.0);
}

#[test]
fn a_gas_sheds_speed_faster_than_a_stone() {
    let book = book();
    assert!(book.get("flame").drag > book.get("stone").drag);
}

#[test]
fn only_solids_and_granulars_settle() {
    assert!(Phase::Solid.settles());
    assert!(Phase::Granular.settles());
    assert!(!Phase::Liquid.settles());
    assert!(!Phase::Gas.settles());
}
