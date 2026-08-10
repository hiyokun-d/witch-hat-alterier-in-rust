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

/// Where a seal's magic will actually go.
///
/// Canon treats a sign set as a set of pushes, not a set of flags: "column
/// signs which are all the same size, and as such, the same power… results in
/// a balanced spell that shoots straight up", while one sign "far longer than
/// the others… has more power, causing uneven pressure which makes the spell
/// shoot off to the side" (§2.4).
///
/// So every sign contributes a vector — magnitude from its size, direction
/// from where it sits — and their sum is the answer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Balance {
    /// Which way the spell leans, in radians. Meaningless when
    /// [`Balance::lean`] is near zero, and callers should check that first.
    pub heading: f32,
    /// How hard it leans, in the same units as sign size.
    pub drift: f32,
    /// Every sign's power added up regardless of direction.
    ///
    /// The denominator that makes `drift` comparable between a seal of three
    /// signs and a seal of thirty.
    pub power: f32,
}

impl Balance {
    /// How far off centre the seal is, `0.0..=1.0`.
    ///
    /// Zero is perfectly balanced — the pushes cancel and the spell goes
    /// straight up. One is every sign pulling the same way. This is what
    /// "adding more signs to a spell can help to average them out" does: more
    /// signs of equal size drive the numerator toward zero while the
    /// denominator keeps growing.
    pub fn lean(&self) -> f32 {
        if self.power <= f32::EPSILON {
            return 0.0;
        }
        (self.drift / self.power).clamp(0.0, 1.0)
    }

    /// Whether the seal is balanced enough to shoot where it is aimed.
    ///
    /// `tolerance` is a share of total power and is **ours** — canon says an
    /// unbalanced spell "will shoot off in unexpected directions" and gives no
    /// number (§2.6).
    pub fn is_balanced(&self, tolerance: f32) -> bool {
        self.lean() <= tolerance
    }
}

/// Sums a sign set into the push it will produce.
///
/// Each sign pushes along the direction it is *drawn* — its orientation — with
/// its size for magnitude. Reversed signs push the opposite way, which is what
/// makes canon rule 6's cancellation fall out: a seal and its mirrored twin
/// sum to nothing.
///
/// **Only directional signs steer.** §2.3 is explicit that for a
/// semi-directional sign "changing their size will only alter the strength of
/// their effect, not direction", and a non-directional one has no direction at
/// all. So a seal of crush signs, however uneven, is strong rather than
/// lopsided — they still count toward `power`, just not toward `drift`.
///
/// `directional` answers whether a given sign steers. It is a predicate rather
/// than a `&Catalog` so the geometry can be tested without loading the whole
/// vocabulary; real callers pass a lookup into `signs.ron`.
///
/// An empty set is perfectly balanced with no power, which is correct: a ring
/// with no signs shoots nowhere in particular (and, per rule 9, explodes).
pub fn balance(signs: &[Sign], directional: impl Fn(&SignId) -> bool) -> Balance {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut power = 0.0;

    for sign in signs {
        power += sign.size.abs();
        if !directional(&sign.kind) {
            continue;
        }

        let push = if sign.reversed { -sign.size } else { sign.size };
        x += push * sign.orientation.cos();
        y += push * sign.orientation.sin();
    }

    Balance {
        heading: y.atan2(x),
        drift: (x * x + y * y).sqrt(),
        power,
    }
}

/// What a seal's tilt buys and what it costs.
///
/// Canon: "By tilting the signs within a seal, it is possible to produce a
/// spell that rotates. The more tilted the signs, the more spin but less reach
/// the spell will have."
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spin {
    /// Mean tilt off the radial direction, in radians, `0..=π/2`.
    pub tilt: f32,
    /// Share of the spell's effort going into rotation, `0.0..=1.0`.
    pub spin: f32,
    /// Share left for distance, `0.0..=1.0`.
    pub reach: f32,
}

/// Reads the spin a sign set will impart.
///
/// **The exchange rate is ours.** Canon states the tradeoff and gives no
/// numbers, so we take `spin = sin(tilt)` and `reach = cos(tilt)` — the two
/// components of the same push, which keeps `spin² + reach² = 1` and makes a
/// sign pointing straight out pure reach and one lying tangent pure spin.
/// Marked here rather than buried, per §2.6.
///
/// Weighted by size, because a large tilted sign turns the spell more than a
/// small one does — size is power everywhere else and it is power here too.
///
/// Only directional signs are counted, for the same reason [`balance`] counts
/// only those: rotating a sign with no direction changes nothing.
pub fn spin(signs: &[Sign], directional: impl Fn(&SignId) -> bool) -> Spin {
    let steering: Vec<&Sign> = signs
        .iter()
        .filter(|sign| directional(&sign.kind))
        .collect();
    let signs = &steering;

    let power: f32 = signs.iter().map(|sign| sign.size.abs()).sum();
    if power <= f32::EPSILON {
        return Spin {
            tilt: 0.0,
            spin: 0.0,
            reach: 1.0,
        };
    }

    let tilt = signs
        .iter()
        .map(|sign| sign.tilt().min(core::f32::consts::PI - sign.tilt()) * sign.size.abs())
        .sum::<f32>()
        / power;

    Spin {
        tilt,
        spin: tilt.sin().abs().clamp(0.0, 1.0),
        reach: tilt.cos().abs().clamp(0.0, 1.0),
    }
}

#[cfg(test)]
#[path = "tests/arrangement.rs"]
mod tests;
