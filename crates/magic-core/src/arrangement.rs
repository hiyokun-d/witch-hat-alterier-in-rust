//! How a glyph's signs are arranged, and what that arrangement means.
//!
//! Two questions live here, and both are properties of the *whole* sign set
//! rather than of any single sign:
//!
//! - [`Symmetry`] — canon rule 7. Radial and bilateral arrangements are the
//!   normal case; asymmetric ones are legal but sometimes unstable.
//! - [`RegionArrangement`] — the four cases in CLAUDE.md §2.3. Where a spell
//!   manifests is decided by where every region sign points *collectively*, so
//!   no per-sign answer exists.

use core::f32::consts::{PI, TAU};

use crate::catalog::{RegionPattern, SignId};
use crate::glyph::Sign;

/// Folds an angle into `0.0..TAU`.
///
/// `rem_euclid` rather than a subtraction loop: it is branchless, and negative
/// orientations arrive routinely from anything that works in screen space.
fn normalize(angle: f32) -> f32 {
    angle.rem_euclid(TAU)
}

/// The shortest angular distance between two directions, in `0.0..=PI`.
fn angular_distance(a: f32, b: f32) -> f32 {
    let d = normalize(a - b);
    if d > PI { TAU - d } else { d }
}

/// How a glyph's signs are laid out around the ring (canon rule 7).
///
/// Classification looks at both where each sign sits and which kind it is. A
/// ring of twelve evenly spaced signs alternating triangle and arrow is radial
/// — order six, not twelve — and getting that right needs the kinds, not just
/// the angles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Symmetry {
    /// Evenly spaced around the ring, and the kinds repeat with that spacing.
    Radial,
    /// Mirror-symmetric about some axis through the centre.
    Bilateral,
    /// Neither. A perfectly legal spell, but the one canon calls unstable, and
    /// the one whose effect biases toward whichever side has more signs.
    Asymmetric,
}

impl Symmetry {
    /// Classifies an arrangement. `tolerance` is in radians and absorbs drawing
    /// slop — hands are not protractors, so an exact test would call almost
    /// every real seal asymmetric.
    ///
    /// A glyph with no signs is [`Symmetry::Radial`]: there is nothing to be off
    /// balance. A single sign is [`Symmetry::Bilateral`], mirrored about the
    /// axis it sits on.
    pub fn classify(signs: &[Sign], tolerance: f32) -> Symmetry {
        match signs.len() {
            0 => return Symmetry::Radial,
            1 => return Symmetry::Bilateral,
            _ => {}
        }

        let mut ordered: Vec<Sign> = signs.to_vec();
        // `total_cmp` rather than `partial_cmp().unwrap()`: core must not panic,
        // and a NaN placement from a degenerate stroke should sort, not abort.
        ordered.sort_by(|a, b| normalize(a.placement).total_cmp(&normalize(b.placement)));

        if is_radial(&ordered, tolerance) {
            Symmetry::Radial
        } else if is_bilateral(&ordered, tolerance) {
            Symmetry::Bilateral
        } else {
            Symmetry::Asymmetric
        }
    }
}

/// Evenly spaced *and* periodic in kind.
///
/// Even spacing alone is not enough: three different signs at 120° apart look
/// balanced but repeat nothing, so rotating the seal produces a different seal.
fn is_radial(ordered: &[Sign], tolerance: f32) -> bool {
    let n = ordered.len();
    let expected = TAU / n as f32;

    let evenly_spaced = (0..n).all(|i| {
        let here = normalize(ordered[i].placement);
        let next = normalize(ordered[(i + 1) % n].placement);
        // The wrap-around gap is the only one that can come out negative before
        // normalizing, which is exactly why the subtraction is normalized too.
        let gap = normalize(next - here);
        (gap - expected).abs() <= tolerance
    });

    if !evenly_spaced {
        return false;
    }

    // Rotating by `step` positions must land every sign on one of the same kind.
    // `step == n` is the identity and proves nothing, so it is excluded.
    (1..n).any(|step| {
        n.is_multiple_of(step) && (0..n).all(|i| ordered[i].kind == ordered[(i + step) % n].kind)
    })
}

/// Mirror-symmetric about some axis through the centre.
///
/// Candidate axes are the midpoints between every pair of signs, plus each
/// midpoint's perpendicular. Any real mirror axis of a finite point set on a
/// circle passes through either a sign or the midpoint of two, so the true axis
/// is always in this set if one exists.
fn is_bilateral(ordered: &[Sign], tolerance: f32) -> bool {
    let mut axes: Vec<f32> = Vec::with_capacity(ordered.len() * ordered.len());
    for a in ordered {
        for b in ordered {
            let mid = normalize((normalize(a.placement) + normalize(b.placement)) / 2.0);
            axes.push(mid);
            axes.push(normalize(mid + PI / 2.0));
        }
    }

    axes.iter()
        .any(|&axis| mirrors_onto_itself(ordered, axis, tolerance))
}

/// Whether reflecting every sign across `axis` lands it on a sign of the same
/// kind.
///
/// Reflection is its own inverse, so checking that each sign *has* a partner is
/// enough — no pairing pass is needed.
fn mirrors_onto_itself(signs: &[Sign], axis: f32, tolerance: f32) -> bool {
    signs.iter().all(|sign| {
        let reflected = normalize(2.0 * axis - sign.placement);
        signs.iter().any(|other| {
            other.kind == sign.kind
                && angular_distance(normalize(other.placement), reflected) <= tolerance
        })
    })
}

/// Where a spell manifests, decided by every region sign together.
///
/// The four canon patterns plus two honest non-answers. The patterns reuse
/// [`RegionPattern`] rather than redeclaring them, so a computed arrangement can
/// be compared directly against the one a spell fixture records.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegionArrangement {
    /// No region signs. The spell manifests wherever its other signs say.
    Absent,
    /// One of the four canon patterns. `heading` is the mean direction in
    /// radians, and is only meaningful for [`RegionPattern::AllSameSide`].
    Canon {
        pattern: RegionPattern,
        heading: Option<f32>,
    },
    /// Region signs are present but all lie tangent to the ring, pointing
    /// neither in nor out nor together. Canon describes no such spell, so this
    /// is reported rather than guessed at.
    Indeterminate,
}

impl RegionArrangement {
    /// The canon pattern this arrangement matches, if any.
    pub fn pattern(&self) -> Option<RegionPattern> {
        match self {
            RegionArrangement::Canon { pattern, .. } => Some(*pattern),
            _ => None,
        }
    }
}

impl RegionArrangement {
    /// Reads the arrangement off a glyph's signs.
    ///
    /// `region` names which sign kind is the region sign, rather than the id
    /// being baked in here — the sign set is data, and the catalog is what knows
    /// its ids. `deadband` is how far from tangent a sign must point before it
    /// counts as in or out; `spread` is how tightly headings must agree to read
    /// as one direction. Both in radians.
    pub fn classify(
        signs: &[Sign],
        region: &SignId,
        deadband: f32,
        spread: f32,
    ) -> RegionArrangement {
        let region: Vec<&Sign> = signs.iter().filter(|sign| &sign.kind == region).collect();

        if region.is_empty() {
            return RegionArrangement::Absent;
        }

        // Checked before in/out, because signs that all point one way are only
        // *incidentally* inward or outward depending on where they happen to
        // sit, and canon treats "all one side" as its own case.
        if let Some(direction) = common_heading(&region, spread) {
            return RegionArrangement::Canon {
                pattern: RegionPattern::AllSameSide,
                heading: Some(direction),
            };
        }

        let any_inward = region.iter().any(|sign| sign.points_inward(deadband));
        let any_outward = region.iter().any(|sign| sign.points_outward(deadband));

        let pattern = match (any_inward, any_outward) {
            (true, true) => RegionPattern::Opposed,
            (true, false) => RegionPattern::AllInward,
            (false, true) => RegionPattern::AllOutward,
            (false, false) => return RegionArrangement::Indeterminate,
        };

        RegionArrangement::Canon {
            pattern,
            heading: None,
        }
    }
}

/// The shared heading of a set of signs, if every one of them points within
/// `spread` of the circular mean.
///
/// Averaged as unit vectors rather than as raw angles: the naive mean of 350°
/// and 10° is 180°, which points the opposite way from both.
fn common_heading(signs: &[&Sign], spread: f32) -> Option<f32> {
    let (sum_x, sum_y) = signs.iter().fold((0.0f32, 0.0f32), |(x, y), sign| {
        (x + sign.orientation.cos(), y + sign.orientation.sin())
    });

    // Headings that cancel out leave no meaningful mean to compare against.
    if sum_x.hypot(sum_y) < f32::EPSILON {
        return None;
    }

    let mean = sum_y.atan2(sum_x);
    signs
        .iter()
        .all(|sign| angular_distance(sign.orientation, mean) <= spread)
        .then_some(normalize(mean))
}

#[cfg(test)]
mod tests {
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
}
