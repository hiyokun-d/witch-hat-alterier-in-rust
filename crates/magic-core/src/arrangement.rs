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
    // Summed in a canonical order rather than the order the signs arrived in.
    //
    // Float addition is not associative, so the same seal drawn sign-by-sign in
    // two different sequences summed to two different numbers — a real §4.3
    // determinism break, and §3.3 is explicit that stroke order is never an
    // input. Sorting the contributions first costs nothing at these sizes and
    // makes the answer a function of the seal alone.
    let mut pushes: Vec<(f32, f32)> = Vec::with_capacity(signs.len());
    let mut sizes: Vec<f32> = Vec::with_capacity(signs.len());

    for sign in signs {
        sizes.push(sign.size.abs());
        if !directional(&sign.kind) {
            continue;
        }
        let push = if sign.reversed { -sign.size } else { sign.size };
        pushes.push((push * sign.orientation.cos(), push * sign.orientation.sin()));
    }

    pushes.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    sizes.sort_by(f32::total_cmp);

    // `+ 0.0` is not decoration. IEEE's additive identity is *negative* zero, so
    // an empty sum comes back as `-0.0` — which then reads as `power -0.0` and,
    // through `atan2(-0.0, -0.0)`, as a seal leaning firmly toward -180°. Adding
    // positive zero collapses the sign and changes nothing else.
    let x: f32 = pushes.iter().map(|p| p.0).sum::<f32>() + 0.0;
    let y: f32 = pushes.iter().map(|p| p.1).sum::<f32>() + 0.0;
    let power: f32 = sizes.iter().sum::<f32>() + 0.0;
    let drift = (x * x + y * y).sqrt();

    Balance {
        // A heading is meaningless without drift to point, and reporting one
        // anyway is how a bare ring came to claim it was shooting backwards.
        heading: if drift <= f32::EPSILON {
            0.0
        } else {
            y.atan2(x)
        },
        drift,
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

    let mut sizes: Vec<f32> = signs.iter().map(|sign| sign.size.abs()).collect();
    sizes.sort_by(f32::total_cmp);
    let power: f32 = sizes.iter().sum::<f32>() + 0.0;
    if power <= f32::EPSILON {
        return Spin {
            tilt: 0.0,
            spin: 0.0,
            reach: 1.0,
        };
    }

    // Sorted for the same reason [`balance`] sorts: order must not be an input.
    let mut weighted: Vec<f32> = signs
        .iter()
        .map(|sign| sign.tilt().min(core::f32::consts::PI - sign.tilt()) * sign.size.abs())
        .collect();
    weighted.sort_by(f32::total_cmp);
    let tilt = (weighted.iter().sum::<f32>() + 0.0) / power;

    Spin {
        tilt,
        spin: tilt.sin().abs().clamp(0.0, 1.0),
        reach: tilt.cos().abs().clamp(0.0, 1.0),
    }
}

#[cfg(test)]
#[path = "tests/arrangement.rs"]
mod tests;

/// What a seal's signs, taken together, aim *at*.
///
/// [`Balance`] answers "which way will it go"; this answers "where will it
/// land". Four arrows around a ring all pointing at the middle have zero drift
/// — they cancel perfectly — and `Balance` correctly reports that the spell
/// goes nowhere. It is also, just as correctly, silent about the thing that
/// makes a water orb a water orb: all four are pushing *at the same point*.
///
/// So a sign set is read a second way. Each directional sign is a ray, and
/// where those rays meet is where the power gathers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Convergence {
    /// The rays meet ahead of the signs: power gathers at a point.
    Converging,
    /// The rays meet behind them: power spreads from a point.
    Diverging,
    /// Some in, some out. Canon's opposed case, and the meeting point sits
    /// between them rather than being aimed at by anything.
    Split,
    /// The signs all point the same way, so nothing meets — a beam, not a
    /// focus. Canon's "all pointing one side".
    Parallel,
    /// Fewer than two signs that steer. There is nothing to intersect, which is
    /// not the same as a spell that focuses nowhere.
    Unaimed,
}

impl Convergence {
    /// The canon region case this geometry corresponds to (§2.3), if any.
    ///
    /// **A finding rather than a definition.** Canon's four cases are stated for
    /// region signs specifically, and they fall straight out of asking where
    /// *any* set of directional signs points: all-inward converges, all-outward
    /// diverges, all-one-side is parallel, opposed is split. So a seal with no
    /// region sign in it — a water orb is four levitation arrows — can still be
    /// read against the same four cases, which is exactly what
    /// `RegionArrangement::Absent` could never say.
    pub fn as_region(self) -> Option<RegionPattern> {
        match self {
            Convergence::Converging => Some(RegionPattern::AllInward),
            Convergence::Diverging => Some(RegionPattern::AllOutward),
            Convergence::Parallel => Some(RegionPattern::AllSameSide),
            Convergence::Split => Some(RegionPattern::Opposed),
            Convergence::Unaimed => None,
        }
    }
}

/// Where a seal's power gathers, and how tightly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Focus {
    /// The meeting point, **relative to the ring's centre**, in the same units
    /// as the radius it was measured with. Ring-relative rather than absolute
    /// so the answer is a fact about the seal and not about where on the page
    /// it was drawn (§3.1's reasoning about position, applied to a reading).
    ///
    /// `(0.0, 0.0)` when [`Focus::convergence`] is
    /// [`Convergence::Unaimed`] or [`Convergence::Parallel`] — there is no
    /// point, and a zero is the honest placeholder rather than a guess.
    pub at: (f32, f32),
    pub convergence: Convergence,
    /// How far each ray misses the meeting point, root-mean-square, in the same
    /// units. Zero means they all pass exactly through it.
    pub spread: f32,
    /// How many signs actually aimed. Signs that do not steer are excluded, the
    /// same way [`balance`] excludes them.
    pub aimed: usize,
}

impl Focus {
    /// How far the meeting point sits from the ring's centre.
    pub fn offset(&self) -> f32 {
        (self.at.0 * self.at.0 + self.at.1 * self.at.1).sqrt()
    }

    /// How tightly the power is gathered, `0.0..=1.0`, against a ring radius.
    ///
    /// One is every ray passing exactly through one point; zero is a spread as
    /// wide as the ring itself. **Ours** (§2.6) — canon has focus as a
    /// qualitative thing and gives no number, and the choice of the ring's own
    /// radius as the denominator is what makes it mean the same on any seal.
    pub fn tightness(&self, radius: f32) -> f32 {
        if radius <= f32::EPSILON || !matches!(
            self.convergence,
            Convergence::Converging | Convergence::Diverging | Convergence::Split
        ) {
            return 0.0;
        }
        (1.0 - self.spread / radius).clamp(0.0, 1.0)
    }
}

/// Reads where a sign set aims.
///
/// # The maths, and why it is this and not an average
///
/// Each steering sign is a ray: it sits at `placement` around a ring of
/// `radius`, and points along its `orientation`. Averaging those directions
/// gives [`Balance`] and, for four inward arrows, gives zero — the pushes
/// cancel, which is true and is not what a person watching wants to know.
///
/// The meeting point is the least-squares intersection instead: the point whose
/// total squared *perpendicular* distance to every ray is smallest. In two
/// dimensions each ray contributes `n nᵀ` to a `2×2` matrix, where `n` is the
/// ray's normal, and the solve is a determinant. A singular matrix means every
/// normal is parallel, which means every ray is — that is the beam case, and it
/// is detected rather than divided by.
///
/// **Weighted by size, because size is power** (§2.4). One arrow drawn longer
/// than its neighbours drags the focal point toward what it is aimed at, which
/// is the same claim canon makes about a longer column sign steering the whole
/// spell.
///
/// # What it cannot know
///
/// `orientation` from a hand-drawn mark is an *axis*, and which end of it is
/// the arrowhead is what `reversed` is for. The meeting point does not care —
/// a line and its reverse intersect at the same place — but converging and
/// diverging are exactly the pair that swap when a sign is read backwards. So
/// [`Focus::at`] is reliable and [`Focus::convergence`] inherits whatever the
/// recognizer decided about direction.
pub fn focus(signs: &[Sign], radius: f32, directional: impl Fn(&SignId) -> bool) -> Focus {
    let unaimed = Focus {
        at: (0.0, 0.0),
        convergence: Convergence::Unaimed,
        spread: 0.0,
        aimed: 0,
    };
    if !radius.is_finite() || radius <= 0.0 {
        return unaimed;
    }

    // Where each sign sits, which way it points, and how hard it pushes.
    let mut rays: Vec<((f32, f32), (f32, f32), f32)> = Vec::new();
    for sign in signs {
        if !directional(&sign.kind) {
            continue;
        }
        let weight = sign.size.abs();
        if weight <= f32::EPSILON {
            continue;
        }
        let at = (
            radius * sign.placement.cos(),
            radius * sign.placement.sin(),
        );
        let heading = if sign.reversed {
            sign.orientation + std::f32::consts::PI
        } else {
            sign.orientation
        };
        rays.push((at, (heading.cos(), heading.sin()), weight));
    }
    if rays.len() < 2 {
        return Focus {
            aimed: rays.len(),
            ..unaimed
        };
    }

    // Summed in a canonical order, not the order the signs arrived in — the
    // same §4.3 break `balance` was caught by, and the same fix.
    let mut terms: Vec<[f32; 5]> = rays
        .iter()
        .map(|&(at, dir, weight)| {
            let normal = (-dir.1, dir.0);
            let along = normal.0 * at.0 + normal.1 * at.1;
            [
                weight * normal.0 * normal.0,
                weight * normal.0 * normal.1,
                weight * normal.1 * normal.1,
                weight * normal.0 * along,
                weight * normal.1 * along,
            ]
        })
        .collect();
    terms.sort_by(|a, b| {
        a.iter()
            .zip(b.iter())
            .find_map(|(one, two)| match one.total_cmp(two) {
                std::cmp::Ordering::Equal => None,
                other => Some(other),
            })
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut m = [0.0f32; 5];
    for term in &terms {
        for (slot, value) in m.iter_mut().zip(term.iter()) {
            *slot += value;
        }
    }
    let (a00, a01, a11, b0, b1) = (m[0], m[1], m[2], m[3], m[4]);

    // A singular matrix means every normal points the same way, so every ray
    // does too. Compared against the trace rather than against an absolute
    // epsilon, because the entries scale with total sign power.
    let det = a00 * a11 - a01 * a01;
    let trace = a00 + a11;
    if !det.is_finite() || det.abs() <= 1e-4 * trace * trace {
        return Focus {
            at: (0.0, 0.0),
            convergence: Convergence::Parallel,
            spread: 0.0,
            aimed: rays.len(),
        };
    }

    let at = (
        (a11 * b0 - a01 * b1) / det,
        (a00 * b1 - a01 * b0) / det,
    );

    // Ahead of the sign or behind it — the difference between gathering power
    // at a point and spreading it from one.
    let (mut ahead, mut behind) = (0usize, 0usize);
    let (mut miss, mut total) = (0.0f32, 0.0f32);
    for &(from, dir, weight) in &rays {
        let to = (at.0 - from.0, at.1 - from.1);
        if to.0 * dir.0 + to.1 * dir.1 >= 0.0 {
            ahead += 1;
        } else {
            behind += 1;
        }
        let off = -dir.1 * to.0 + dir.0 * to.1;
        miss += weight * off * off;
        total += weight;
    }

    Focus {
        at,
        convergence: match (ahead, behind) {
            (_, 0) => Convergence::Converging,
            (0, _) => Convergence::Diverging,
            _ => Convergence::Split,
        },
        spread: if total > 0.0 {
            (miss / total).sqrt()
        } else {
            0.0
        },
        aimed: rays.len(),
    }
}
