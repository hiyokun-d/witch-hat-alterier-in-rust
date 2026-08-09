//! Tests for `arrangement`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use core::f32::consts::{FRAC_PI_2, FRAC_PI_4};

/// The catalog id of the region sign. A literal here rather than a lookup:
/// these tests exercise the geometry, not the loader.
const REGION: &str = "region";

/// A sign sitting at `placement`, pointing straight out from the centre.
fn spoke(kind: &str, placement: f32) -> Sign {
    Sign {
        kind: kind.into(),
        placement,
        orientation: placement,
        reversed: false,
    }
}

fn region_at(placement: f32, orientation: f32) -> Sign {
    Sign {
        kind: REGION.into(),
        placement,
        orientation,
        reversed: false,
    }
}

/// `n` signs of one kind, evenly spaced.
fn evenly_spaced(kind: &str, n: usize) -> Vec<Sign> {
    (0..n)
        .map(|i| spoke(kind, TAU * i as f32 / n as f32))
        .collect()
}

const TOL: f32 = 0.05;

#[test]
fn no_signs_is_radial() {
    assert_eq!(Symmetry::classify(&[], TOL), Symmetry::Radial);
}

#[test]
fn one_sign_is_bilateral() {
    let signs = [spoke("column", 1.0)];
    assert_eq!(Symmetry::classify(&signs, TOL), Symmetry::Bilateral);
}

#[test]
fn four_identical_signs_evenly_spaced_are_radial() {
    let signs = evenly_spaced("column", 4);
    assert_eq!(Symmetry::classify(&signs, TOL), Symmetry::Radial);
}

#[test]
fn radial_survives_hand_drawn_slop() {
    let mut signs = evenly_spaced("column", 6);
    signs[2].placement += TOL / 2.0;
    assert_eq!(Symmetry::classify(&signs, TOL), Symmetry::Radial);
}

/// The reference seal: twelve signs alternating two kinds. Evenly spaced and
/// periodic with step 2, so it is radial of order six.
#[test]
fn alternating_kinds_evenly_spaced_are_radial() {
    let signs: Vec<Sign> = (0..12)
        .map(|i| {
            let kind = if i % 2 == 0 { "convergence" } else { "region" };
            spoke(kind, TAU * i as f32 / 12.0)
        })
        .collect();
    assert_eq!(Symmetry::classify(&signs, TOL), Symmetry::Radial);
}

/// Even spacing is not enough on its own — rotating this seal produces a
/// different seal, so it cannot be radial.
#[test]
fn evenly_spaced_signs_of_three_different_kinds_are_not_radial() {
    let signs = vec![
        spoke("column", 0.0),
        spoke("pulling", TAU / 3.0),
        spoke("crushing", 2.0 * TAU / 3.0),
    ];
    assert_ne!(Symmetry::classify(&signs, TOL), Symmetry::Radial);
}

#[test]
fn two_signs_mirrored_about_the_vertical_are_bilateral() {
    let signs = vec![
        spoke("column", FRAC_PI_2 - FRAC_PI_4),
        spoke("column", FRAC_PI_2 + FRAC_PI_4),
    ];
    assert_eq!(Symmetry::classify(&signs, TOL), Symmetry::Bilateral);
}

#[test]
fn a_lone_extra_sign_makes_the_arrangement_asymmetric() {
    let mut signs = evenly_spaced("column", 4);
    signs.push(spoke("pulling", 0.3));
    assert_eq!(Symmetry::classify(&signs, TOL), Symmetry::Asymmetric);
}

#[test]
fn mirrored_pair_of_different_kinds_is_asymmetric() {
    let signs = vec![
        spoke("column", FRAC_PI_2 - FRAC_PI_4),
        spoke("pulling", FRAC_PI_2 + FRAC_PI_4),
    ];
    assert_eq!(Symmetry::classify(&signs, TOL), Symmetry::Asymmetric);
}

#[test]
fn classification_ignores_the_order_signs_were_drawn_in() {
    let forward = evenly_spaced("column", 5);
    let mut backward = forward.clone();
    backward.reverse();
    assert_eq!(
        Symmetry::classify(&forward, TOL),
        Symmetry::classify(&backward, TOL)
    );
}

const DEADBAND: f32 = 0.2;
const SPREAD: f32 = 0.3;

#[test]
fn no_region_signs_is_absent() {
    let signs = evenly_spaced("column", 4);
    assert_eq!(
        RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD),
        RegionArrangement::Absent
    );
}

#[test]
fn region_signs_all_pointing_outward_manifest_outside() {
    let signs: Vec<Sign> = (0..4)
        .map(|i| {
            let p = TAU * i as f32 / 4.0;
            region_at(p, p)
        })
        .collect();
    assert_eq!(
        RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD),
        RegionArrangement::Canon {
            pattern: RegionPattern::AllOutward,
            heading: None
        }
    );
}

#[test]
fn region_signs_all_pointing_inward_manifest_inside() {
    let signs: Vec<Sign> = (0..4)
        .map(|i| {
            let p = TAU * i as f32 / 4.0;
            region_at(p, p + PI)
        })
        .collect();
    assert_eq!(
        RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD),
        RegionArrangement::Canon {
            pattern: RegionPattern::AllInward,
            heading: None
        }
    );
}

#[test]
fn opposed_region_signs_manifest_on_the_ring() {
    let signs = vec![
        region_at(0.0, 0.0),
        region_at(FRAC_PI_2, FRAC_PI_2),
        region_at(PI, 0.0),
        region_at(PI + FRAC_PI_2, FRAC_PI_2),
    ];
    assert_eq!(
        RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD),
        RegionArrangement::Canon {
            pattern: RegionPattern::Opposed,
            heading: None
        }
    );
}

#[test]
fn region_signs_all_pointing_one_way_shoot_that_way() {
    let signs = vec![
        region_at(0.0, FRAC_PI_2),
        region_at(FRAC_PI_2, FRAC_PI_2),
        region_at(PI, FRAC_PI_2),
    ];
    match RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD) {
        RegionArrangement::Canon {
            pattern: RegionPattern::AllSameSide,
            heading: Some(direction),
        } => {
            assert!(angular_distance(direction, FRAC_PI_2) <= SPREAD);
        }
        other => panic!("expected OneSide, got {other:?}"),
    }
}

/// The circular mean of headings either side of zero must not land opposite
/// them — the reason headings are averaged as vectors.
#[test]
fn one_side_survives_headings_that_straddle_zero() {
    let signs = vec![
        region_at(0.0, 0.1),
        region_at(FRAC_PI_2, TAU - 0.1),
        region_at(PI, 0.0),
    ];
    match RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD) {
        RegionArrangement::Canon {
            pattern: RegionPattern::AllSameSide,
            heading: Some(direction),
        } => {
            assert!(angular_distance(direction, 0.0) <= SPREAD);
        }
        other => panic!("expected OneSide, got {other:?}"),
    }
}

#[test]
fn region_signs_lying_tangent_are_indeterminate() {
    let signs = vec![region_at(0.0, FRAC_PI_2), region_at(PI, PI + FRAC_PI_2)];
    assert_eq!(
        RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD),
        RegionArrangement::Indeterminate
    );
}

#[test]
fn region_classification_ignores_non_region_signs() {
    let mut signs: Vec<Sign> = (0..4)
        .map(|i| {
            let p = TAU * i as f32 / 4.0;
            region_at(p, p)
        })
        .collect();
    signs.push(spoke("bird", 0.7));
    assert_eq!(
        RegionArrangement::classify(&signs, &REGION.into(), DEADBAND, SPREAD),
        RegionArrangement::Canon {
            pattern: RegionPattern::AllOutward,
            heading: None
        }
    );
}
