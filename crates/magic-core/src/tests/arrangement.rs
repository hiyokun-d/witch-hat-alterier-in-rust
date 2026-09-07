//! Tests for `arrangement`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use core::f32::consts::{FRAC_PI_2, FRAC_PI_4, PI};

/// The catalog id of the region sign. A literal here rather than a lookup:
/// these tests exercise the geometry, not the loader.
const REGION: &str = "region";

/// A sign sitting at `placement`, pointing straight out from the centre.
///
/// All one size, so these fixtures test symmetry of *position*. Power is
/// tested separately — see the balance tests.
fn spoke(kind: &str, placement: f32) -> Sign {
    sized_spoke(kind, placement, 1.0)
}

/// A spoke of a chosen size, for the tests that care that size is power.
fn sized_spoke(kind: &str, placement: f32, size: f32) -> Sign {
    Sign {
        kind: kind.into(),
        placement,
        orientation: placement,
        size,
        reversed: false,
    }
}

fn region_at(placement: f32, orientation: f32) -> Sign {
    Sign {
        kind: REGION.into(),
        placement,
        orientation,
        size: 1.0,
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

// ── balance and spin ────────────────────────────────────────────────────────

/// Every sign in these fixtures steers. The class rule gets its own test.
fn all_directional(_: &SignId) -> bool {
    true
}

/// The wiki's own example: identical column signs shoot straight up.
#[test]
fn signs_of_equal_size_evenly_spaced_are_balanced() {
    let seal = evenly_spaced("column", 6);
    let found = balance(&seal, all_directional);

    assert!(found.lean() < 1e-5, "leaned {}", found.lean());
    assert!(found.is_balanced(0.05));
    assert!((found.power - 6.0).abs() < 1e-4);
}

/// The other half of the same example: one sign far longer than the others
/// carries more power and the spell shoots off toward it.
#[test]
fn one_oversized_sign_steers_the_spell_toward_itself() {
    let mut seal = evenly_spaced("column", 6);
    // The sign at placement 0, made three times the length of its neighbours.
    seal[0] = sized_spoke("column", 0.0, 3.0);

    let found = balance(&seal, all_directional);
    assert!(found.lean() > 0.2, "barely leaned: {}", found.lean());
    assert!(found.heading.abs() < 0.05, "should lean toward 0 rad");
    assert!(!found.is_balanced(0.05));
}

/// "Adding more signs to a spell can help to average them out."
#[test]
fn more_signs_average_an_imbalance_away() {
    let lean_of = |n: usize| {
        let mut seal = evenly_spaced("column", n);
        seal[0] = sized_spoke("column", 0.0, 3.0);
        balance(&seal, all_directional).lean()
    };

    assert!(lean_of(12) < lean_of(4), "more signs should even it out");
}

/// Canon rule 6 falls straight out of size being signed by `reversed`: a seal
/// and its mirrored twin sum to nothing.
#[test]
fn a_seal_and_its_reversed_twin_cancel() {
    let seal = evenly_spaced("column", 3);
    let mut both = seal.clone();
    both.extend(seal.iter().map(|sign| sign.reverse()));

    assert!(balance(&both, all_directional).drift < 1e-5);
}

#[test]
fn an_empty_seal_leans_nowhere() {
    let found = balance(&[], all_directional);
    assert_eq!(found.power, 0.0);
    assert_eq!(found.lean(), 0.0);
    assert!(found.is_balanced(0.0));
}

/// Signs pointing straight out are all reach and no spin.
#[test]
fn untilted_signs_impart_no_spin() {
    let found = spin(&evenly_spaced("column", 6), all_directional);
    assert!(found.spin < 1e-4, "spin {}", found.spin);
    assert!((found.reach - 1.0).abs() < 1e-4);
}

/// "The more tilted the signs, the more spin but less reach."
#[test]
fn tilting_signs_trades_reach_for_spin() {
    let tilted_by = |tilt: f32| {
        let seal: Vec<Sign> = (0..6)
            .map(|i| {
                let placement = TAU * i as f32 / 6.0;
                Sign {
                    kind: "column".into(),
                    placement,
                    orientation: placement + tilt,
                    size: 1.0,
                    reversed: false,
                }
            })
            .collect();
        spin(&seal, all_directional)
    };

    let gentle = tilted_by(0.2);
    let hard = tilted_by(1.0);

    assert!(hard.spin > gentle.spin, "more tilt should spin more");
    assert!(hard.reach < gentle.reach, "more tilt should reach less");
    // The two are the components of one push, so they stay on the unit circle.
    for found in [gentle, hard] {
        let sum = found.spin * found.spin + found.reach * found.reach;
        assert!((sum - 1.0).abs() < 1e-4, "spin² + reach² was {sum}");
    }
}

/// Signs lying tangent to the ring are all spin and no reach.
#[test]
fn fully_tangent_signs_are_all_spin() {
    let seal: Vec<Sign> = (0..4)
        .map(|i| {
            let placement = TAU * i as f32 / 4.0;
            Sign {
                kind: "column".into(),
                placement,
                orientation: placement + FRAC_PI_2,
                size: 1.0,
                reversed: false,
            }
        })
        .collect();

    let found = spin(&seal, all_directional);
    assert!((found.spin - 1.0).abs() < 1e-4, "spin {}", found.spin);
    assert!(found.reach < 1e-4, "reach {}", found.reach);
}

/// A big tilted sign turns the spell more than a small one, because size is
/// power here as everywhere else.
#[test]
fn a_large_tilted_sign_outweighs_a_small_straight_one() {
    let mixed = vec![
        Sign {
            kind: "column".into(),
            placement: 0.0,
            orientation: FRAC_PI_2,
            size: 4.0,
            reversed: false,
        },
        sized_spoke("column", PI, 1.0),
    ];

    assert!(
        spin(&mixed, all_directional).spin > 0.6,
        "the heavy tilted sign should dominate"
    );
}

/// §2.3: "Changing their size will only alter the strength of their effect,
/// not direction." A lopsided set of crush signs is strong, not lopsided.
#[test]
fn a_sign_that_does_not_steer_adds_power_without_drift() {
    let mut seal = evenly_spaced("crush", 6);
    seal[0] = sized_spoke("crush", 0.0, 5.0);

    let found = balance(&seal, |_| false);
    assert_eq!(
        found.drift, 0.0,
        "a non-steering sign must not lean the seal"
    );
    assert!(
        (found.power - 10.0).abs() < 1e-4,
        "but it still counts as power"
    );
    assert!(found.is_balanced(0.0));
}

/// A seal mixing both kinds leans only by its directional half.
#[test]
fn only_directional_signs_steer_a_mixed_seal() {
    let mut seal = evenly_spaced("crush", 4);
    seal.push(sized_spoke("column", 0.0, 2.0));

    let found = balance(&seal, |kind| kind.as_str() == "column");
    assert!(found.heading.abs() < 1e-4, "should lean toward the column");
    assert!((found.drift - 2.0).abs() < 1e-4);
    assert!(
        (found.power - 6.0).abs() < 1e-4,
        "crush still counts as power"
    );
}

/// Rotating a sign with no direction changes nothing.
#[test]
fn tilting_a_non_directional_sign_imparts_no_spin() {
    let seal: Vec<Sign> = (0..4)
        .map(|i| {
            let placement = TAU * i as f32 / 4.0;
            Sign {
                kind: "float".into(),
                placement,
                orientation: placement + FRAC_PI_2,
                size: 1.0,
                reversed: false,
            }
        })
        .collect();

    let found = spin(&seal, |_| false);
    assert_eq!(found.spin, 0.0);
    assert_eq!(found.reach, 1.0);
}

#[test]
fn an_empty_sign_set_leans_nowhere_at_all() {
    // The overlay was printing `lean 0.00 → -180°   power -0.0` for a bare
    // ring. A heading of -180° for a seal with no signs is noise dressed as a
    // measurement, and a negative zero is a sign something summed backwards.
    let b = balance(&[], |_| true);
    assert_eq!(b.power, 0.0);
    assert!(b.power.is_sign_positive(), "power came out as -0.0");
    assert_eq!(b.drift, 0.0);
    assert_eq!(b.heading, 0.0);
}

// ── focus ───────────────────────────────────────────────────────────────────
//
// Where the power gathers, as against `balance`'s "which way does it go".

/// A keystone at `placement` aimed along `orientation`.
fn aimed(placement: f32, orientation: f32, size: f32) -> Sign {
    Sign {
        kind: "column".into(),
        placement,
        orientation,
        size,
        reversed: false,
    }
}

/// Four keystones at the compass points, each turned by `turn` from outward.
fn four(turn: f32, size: f32) -> Vec<Sign> {
    (0..4)
        .map(|n| {
            let placement = n as f32 * FRAC_PI_2;
            aimed(placement, placement + turn, size)
        })
        .collect()
}

/// Everything steers, for the geometry tests.
fn steers(_: &SignId) -> bool {
    true
}

#[test]
fn four_arrows_pointing_inward_focus_on_the_centre() {
    // The water orb, and the reading that did not exist. `balance` says this
    // seal goes nowhere — correctly, the pushes cancel — and is silent about
    // the thing that makes it an orb: all four are aimed at one point.
    let found = focus(&four(PI, 1.0), 100.0, steers);
    assert_eq!(found.convergence, Convergence::Converging);
    assert!(found.offset() < 0.01, "focus at {:?}", found.at);
    assert!(found.spread < 0.01);
    assert_eq!(found.aimed, 4);
    assert!(found.tightness(100.0) > 0.99);
}

#[test]
fn four_arrows_pointing_outward_spread_from_the_centre() {
    let found = focus(&four(0.0, 1.0), 100.0, steers);
    assert_eq!(found.convergence, Convergence::Diverging);
    assert!(found.offset() < 0.01);
}

#[test]
fn arrows_all_pointing_one_way_are_a_beam_not_a_focus() {
    // Canon's "all pointing one side". Parallel rays have no meeting point, and
    // the solve is singular — detected rather than divided by.
    let signs: Vec<Sign> = (0..4)
        .map(|n| aimed(n as f32 * FRAC_PI_2, FRAC_PI_2, 1.0))
        .collect();
    let found = focus(&signs, 100.0, steers);
    assert_eq!(found.convergence, Convergence::Parallel);
    assert_eq!(found.at, (0.0, 0.0));
}

#[test]
fn opposed_arrows_read_as_split() {
    // Two in, two out: canon's opposed case, where the meeting point is not
    // something anything is actually aimed at.
    let signs = vec![
        aimed(0.0, PI, 1.0),
        aimed(PI, 0.0, 1.0),
        aimed(FRAC_PI_2, FRAC_PI_2, 1.0),
        aimed(-FRAC_PI_2, -FRAC_PI_2, 1.0),
    ];
    assert_eq!(focus(&signs, 100.0, steers).convergence, Convergence::Split);
}

#[test]
fn one_arrow_aims_at_nothing_in_particular() {
    // Not "focuses nowhere" — there is nothing to intersect, and saying so is
    // different from reporting a point.
    let found = focus(&[aimed(0.0, PI, 1.0)], 100.0, steers);
    assert_eq!(found.convergence, Convergence::Unaimed);
    assert_eq!(found.aimed, 1);
    assert_eq!(found.tightness(100.0), 0.0);
}

#[test]
fn a_longer_arrow_drags_the_focus_toward_what_it_aims_at() {
    // §2.4: size is power, and one sign longer than its neighbours steers the
    // whole spell. The same claim, read as a *place* rather than a direction.
    let mut signs = four(PI, 1.0);
    // Turn the eastern arrow so it aims north of the centre, and make it heavy.
    // `PI - 0.4` rather than `PI + 0.4`: the arrow sits due east and points back
    // west, so subtracting swings its aim upward and adding swings it down.
    signs[0].orientation = PI - 0.4;
    signs[0].size = 6.0;

    let found = focus(&signs, 100.0, steers);
    assert!(
        found.at.1 > 4.0,
        "the heavy arrow should pull the focus north, got {:?}",
        found.at
    );
}

#[test]
fn signs_that_do_not_steer_are_left_out() {
    // The same exclusion `balance` makes, for the same canon reason: a
    // non-directional sign has no front to point.
    let signs = four(PI, 1.0);
    let found = focus(&signs, 100.0, |kind| kind.as_str() != "column");
    assert_eq!(found.convergence, Convergence::Unaimed);
    assert_eq!(found.aimed, 0);
}

#[test]
fn a_focus_off_the_centre_is_reported_where_it_actually_is() {
    // Two arrows crossing north of the middle. Nothing here is symmetric, so a
    // "focus is always the centre" bug would pass every test above and fail
    // this one.
    let signs = vec![aimed(0.0, PI - FRAC_PI_4, 1.0), aimed(PI, FRAC_PI_4, 1.0)];
    let found = focus(&signs, 100.0, steers);
    assert_eq!(found.convergence, Convergence::Converging);
    assert!(found.at.0.abs() < 0.01, "should sit on the vertical axis");
    assert!(
        (found.at.1 - 100.0).abs() < 1.0,
        "should cross a radius above centre, got {:?}",
        found.at
    );
}

#[test]
fn sloppy_arrows_still_focus_but_less_tightly() {
    // Tightness is the number worth having: four arrows that nearly meet and
    // four that meet exactly are both "converging", and only one is an orb.
    let neat = focus(&four(PI, 1.0), 100.0, steers);

    let mut sloppy = four(PI, 1.0);
    for (n, sign) in sloppy.iter_mut().enumerate() {
        sign.orientation += if n % 2 == 0 { 0.35 } else { -0.35 };
    }
    let rough = focus(&sloppy, 100.0, steers);

    assert_eq!(rough.convergence, Convergence::Converging);
    assert!(
        rough.tightness(100.0) < neat.tightness(100.0),
        "neat {} vs rough {}",
        neat.tightness(100.0),
        rough.tightness(100.0)
    );
}

#[test]
fn focus_does_not_depend_on_the_order_the_signs_were_drawn() {
    // §4.3 and §3.3: stroke order is never an input. Float addition is not
    // associative, which is how `balance` broke this exact way.
    let signs = vec![
        aimed(0.0, PI + 0.2, 1.3),
        aimed(FRAC_PI_2, PI + FRAC_PI_2 - 0.1, 0.7),
        aimed(PI, 0.15, 2.1),
        aimed(-FRAC_PI_2, FRAC_PI_2 + 0.05, 1.9),
    ];
    let forward = focus(&signs, 137.0, steers);

    let mut backward = signs.clone();
    backward.reverse();
    assert_eq!(forward, focus(&backward, 137.0, steers));
}

#[test]
fn the_four_convergence_cases_are_the_four_canon_region_cases() {
    // A finding, not a definition: canon states its four cases for region signs
    // specifically, and they fall straight out of asking where *any* set of
    // directional signs points. That is what lets a water orb — four levitation
    // arrows and no region sign at all — be read against them.
    assert_eq!(
        Convergence::Converging.as_region(),
        Some(RegionPattern::AllInward)
    );
    assert_eq!(
        Convergence::Diverging.as_region(),
        Some(RegionPattern::AllOutward)
    );
    assert_eq!(
        Convergence::Parallel.as_region(),
        Some(RegionPattern::AllSameSide)
    );
    assert_eq!(Convergence::Split.as_region(), Some(RegionPattern::Opposed));
    assert_eq!(Convergence::Unaimed.as_region(), None);
}
