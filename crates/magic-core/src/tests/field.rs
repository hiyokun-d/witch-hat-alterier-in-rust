//! Tests for `sim::field`.

use super::*;
use crate::sim::parcel::AMBIENT;

fn grid() -> Field {
    Field::new(4, 4, 10.0, Vec2::ZERO).expect("a 4x4 grid is valid")
}

fn air(at: Vec2, mass: f32) -> Parcel {
    Parcel::new(SubstanceId::new("air"), at, mass)
}

#[test]
fn a_field_with_no_width_is_refused_rather_than_panicking() {
    assert!(Field::new(0, 4, 10.0, Vec2::ZERO).is_none());
    assert!(Field::new(4, 0, 10.0, Vec2::ZERO).is_none());
}

#[test]
fn a_field_with_no_cell_size_is_refused() {
    // The divide in `cell_at` would be by zero.
    assert!(Field::new(4, 4, 0.0, Vec2::ZERO).is_none());
    assert!(Field::new(4, 4, -3.0, Vec2::ZERO).is_none());
}

#[test]
fn a_position_on_the_origin_lands_in_cell_zero_zero() {
    assert_eq!(grid().cell_at(Vec2::ZERO), Some((0, 0)));
}

#[test]
fn a_position_left_of_the_origin_is_outside_the_grid() {
    // Casting -1.0 to usize saturates to 0 in Rust, which would file this into
    // the first column instead. The sign is checked before the cast.
    assert_eq!(grid().cell_at(Vec2::new(-1.0, 5.0)), None);
    assert_eq!(grid().cell_at(Vec2::new(5.0, -1.0)), None);
}

#[test]
fn a_position_past_the_far_edge_is_outside_the_grid() {
    assert_eq!(grid().cell_at(Vec2::new(40.0, 5.0)), None);
    assert_eq!(grid().cell_at(Vec2::new(39.9, 5.0)), Some((3, 0)));
}

#[test]
fn a_cells_centre_sits_half_a_cell_in_from_its_corner() {
    assert_eq!(grid().cell_center(0, 0), Vec2::new(5.0, 5.0));
    assert_eq!(grid().cell_center(3, 3), Vec2::new(35.0, 35.0));
}

#[test]
fn settling_puts_a_parcels_mass_in_the_cell_it_sits_in() {
    let mut field = grid();
    field.add(air(Vec2::new(15.0, 5.0), 2.0));
    field.settle();
    assert_eq!(field.cell_by(1, 0).unwrap().density, 2.0);
    assert_eq!(field.cell_by(0, 0).unwrap().density, 0.0);
}

#[test]
fn a_cell_holding_two_parcels_reports_their_combined_mass() {
    let mut field = grid();
    field.add(air(Vec2::new(5.0, 5.0), 2.0));
    field.add(air(Vec2::new(6.0, 6.0), 3.0));
    field.settle();
    assert_eq!(field.cell_by(0, 0).unwrap().density, 5.0);
}

#[test]
fn a_cells_temperature_is_weighted_by_mass_not_by_count() {
    let mut field = grid();
    let mut speck = air(Vec2::new(5.0, 5.0), 1.0);
    speck.temperature = 100.0;
    let mut boulder = air(Vec2::new(5.0, 5.0), 99.0);
    boulder.temperature = 0.0;
    field.add(speck);
    field.add(boulder);
    field.settle();

    let cell = field.cell_by(0, 0).unwrap();
    assert!(cell.temperature < 2.0, "got {}", cell.temperature);
}

#[test]
fn an_empty_cell_reads_as_ambient_and_still() {
    let mut field = grid();
    field.settle();
    let cell = field.cell_by(2, 2).unwrap();
    assert_eq!(cell.temperature, AMBIENT);
    assert_eq!(cell.flow, Vec2::ZERO);
    assert_eq!(cell.density, 0.0);
}

#[test]
fn a_parcel_outside_the_grid_contributes_to_no_cell() {
    let mut field = grid();
    field.add(air(Vec2::new(-50.0, -50.0), 5.0));
    field.settle();
    assert_eq!(field.mass(), 5.0);
    for col in 0..4 {
        for row in 0..4 {
            assert_eq!(field.cell_by(col, row).unwrap().density, 0.0);
        }
    }
}

#[test]
fn settling_twice_gives_the_same_field_as_settling_once() {
    // It runs every frame. Not being idempotent would compound silently.
    let mut field = grid();
    field.add(air(Vec2::new(5.0, 5.0), 2.0));
    field.add(air(Vec2::new(25.0, 15.0), 4.0));
    field.settle();
    let once = field.clone();
    field.settle();
    assert_eq!(field, once);
}

#[test]
fn total_mass_is_unchanged_by_settling() {
    let mut field = grid();
    field.add(air(Vec2::new(5.0, 5.0), 2.0));
    field.add(air(Vec2::new(25.0, 15.0), 4.0));
    let before = field.mass();
    field.settle();
    assert_eq!(field.mass(), before);
}

#[test]
fn an_empty_field_reports_positive_zero_mass() {
    // IEEE's additive identity is -0.0, and `-0.00` on screen reads as a
    // measurement rather than as nothing.
    assert!(grid().mass().is_sign_positive());
}

#[test]
fn taking_more_than_there_is_returns_only_what_was_there() {
    let mut field = grid();
    field.add(air(Vec2::new(5.0, 5.0), 3.0));
    let got = field.take(&SubstanceId::new("air"), 10.0, Vec2::new(5.0, 5.0), 100.0);
    assert_eq!(got, 3.0);
    assert_eq!(field.mass(), 0.0);
}

#[test]
fn taking_finds_nothing_when_the_substance_is_absent() {
    let mut field = grid();
    field.add(air(Vec2::new(5.0, 5.0), 3.0));
    let got = field.take(&SubstanceId::new("stone"), 1.0, Vec2::new(5.0, 5.0), 100.0);
    assert_eq!(got, 0.0);
    assert_eq!(field.mass(), 3.0);
}

#[test]
fn taking_ignores_substance_out_of_reach() {
    let mut field = grid();
    field.add(air(Vec2::new(35.0, 35.0), 3.0));
    let got = field.take(&SubstanceId::new("air"), 1.0, Vec2::ZERO, 5.0);
    assert_eq!(got, 0.0);
}

#[test]
fn taking_drains_the_nearest_parcel_first() {
    let mut field = grid();
    field.add(air(Vec2::new(35.0, 5.0), 5.0));
    field.add(air(Vec2::new(6.0, 5.0), 5.0));
    field.take(&SubstanceId::new("air"), 5.0, Vec2::new(5.0, 5.0), 100.0);

    // The far one is untouched; the near one is gone entirely.
    assert_eq!(field.parcels().len(), 1);
    assert_eq!(field.parcels()[0].at, Vec2::new(35.0, 5.0));
}

#[test]
fn confining_keeps_every_parcel_inside_the_grid() {
    let mut field = grid();
    field.add(air(Vec2::new(-10.0, 100.0), 1.0));
    field.confine();
    assert!(field.contains(field.parcels()[0].at));
}

#[test]
fn confining_conserves_mass() {
    // Walls rather than a void, precisely so nothing can vanish unnoticed.
    let mut field = grid();
    field.add(air(Vec2::new(-10.0, 100.0), 1.0));
    field.add(air(Vec2::new(999.0, -999.0), 4.0));
    field.confine();
    assert_eq!(field.mass(), 5.0);
}

#[test]
fn a_parcel_driven_off_an_edge_is_turned_back_inward() {
    let mut field = grid();
    let mut escaping = air(Vec2::new(-5.0, 20.0), 1.0);
    escaping.velocity = Vec2::new(-50.0, 0.0);
    field.add(escaping);
    field.confine();
    assert!(field.parcels()[0].velocity.x > 0.0);
}
