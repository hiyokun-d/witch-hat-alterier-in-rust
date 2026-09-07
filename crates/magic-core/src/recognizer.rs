//! `$P` point-cloud recognition (Vatavu, Anthony & Wobbrock, 2012).
//!
//! A drawn gesture and a stored template are both reduced to a fixed number of
//! evenly spaced points and then compared as unordered clouds. Order does not
//! enter into it, which is the whole reason for choosing `$P`: canon never
//! constrains the order a seal is drawn in (§2.4), so a recognizer that cared
//! would be wrong about the game before it was wrong about anything else.
//!
//! This module holds the normalisation — turning ink into a cloud that can be
//! compared. The comparison itself is M4.7.
//!
//! # What normalisation removes, and what it must not
//!
//! Two of the three transforms `$P` can divide out are exactly what canon
//! asks for, and the third would be a bug:
//!
//! - **Position** — removed. "A sigil's size and location within a seal do not
//!   change its behaviour" (§2.2), so where on the paper it was drawn cannot
//!   be allowed to affect the match.
//! - **Scale** — removed, *uniformly*, and by a different measure than the
//!   paper uses. `$P` divides by the longer side of the bounding box; we divide
//!   by the RMS distance of the points from their centroid. See below.
//! - **Rotation** — **kept.** `$P` implementations often rotate a cloud to a
//!   canonical angle, and doing that here would erase canon rule 6: a reversed
//!   sign inverts its effect, and a spell plus its reversed twin cancel out.
//!   Orientation is meaning, not noise. Small rotations still match because a
//!   slightly turned cloud is still a nearby cloud — which is exactly the ±15°
//!   tolerance §5 asks for, and no more.
//!
//! # Why not the bounding box
//!
//! A bounding box is not rotation-invariant. Turn a square by twelve degrees
//! and its box grows by nearly a fifth, so dividing by the box shrinks the
//! shape inside it — a small turn changes the drawing's *size* as well as its
//! angle. Most `$P` users never notice, because they normalise rotation away
//! before they get here. We deliberately do not, so we would notice: it cost a
//! rotated square its match against a square template in
//! `a_gesture_off_by_twelve_degrees_still_matches`.
//!
//! RMS distance from the centroid has no such preference for the axes, so a
//! turned drawing keeps its size and only changes its angle. That is what §5's
//! ±15° tolerance needs, and it costs nothing — the same quantity already
//! conditions the circle fitter.
//!
//! What is divided out is not thrown away: [`Cloud`] keeps the size and the
//! position it removed, because the wiki ties a spell's intensity to the size
//! of a sigil relative to its ring, and nothing downstream could recover it.

use crate::Point;
use crate::stroke;

/// A gesture smaller than this in both directions is a dot, and has no shape
/// to normalise.
const MIN_SIZE: f32 = 1e-4;

/// A gesture reduced to the form `$P` compares.
///
/// Position and scale are gone; rotation is deliberately intact.
#[derive(Debug, Clone, PartialEq)]
pub struct Cloud {
    /// The normalised points. The centroid sits at the origin and the RMS
    /// distance from it is exactly `1.0`.
    ///
    /// `stroke_id` is carried through untouched. The match ignores it, but a
    /// caller may still want to know which mark a point came from.
    pub points: Vec<Point>,
    /// The size that was divided out — the RMS distance of the points from
    /// their centroid, in the input's units.
    ///
    /// Kept because the recognizer must not see it and the compiler must. The
    /// wiki has "the size of a sigil in relation to the ring" setting a
    /// spell's intensity, so this number has to survive the step that exists
    /// to ignore it.
    pub scale: f32,
    /// Where the gesture's centroid sat before it was moved to the origin.
    pub origin: Point,
}

/// Reduces ink to a comparable cloud of `n` points.
///
/// Accepts a whole gesture — several strokes are one drawing, and the leaps
/// between them are not ink.
///
/// Returns `None` when there is nothing to normalise: too few points, too few
/// samples asked for, or a gesture with no extent in any direction.
pub fn normalize(points: &[Point], n: usize) -> Option<Cloud> {
    let mut sampled = stroke::resample(points, n)?;
    let count = sampled.len() as f32;

    let mut center = (0.0, 0.0);
    for p in &sampled {
        center.0 += p.x;
        center.1 += p.y;
    }
    center.0 /= count;
    center.1 /= count;

    for p in &mut sampled {
        p.x -= center.0;
        p.y -= center.1;
    }

    // RMS distance from the centroid, which — unlike a bounding box — does not
    // change when the drawing turns. Both axes are divided by the same number,
    // so proportions survive: a circle and an ellipse stay different shapes.
    let spread = sampled.iter().map(|p| p.x * p.x + p.y * p.y).sum::<f32>() / count;
    let size = spread.sqrt();
    if !size.is_finite() || size < MIN_SIZE {
        return None;
    }

    for p in &mut sampled {
        p.x /= size;
        p.y /= size;
    }

    Some(Cloud {
        points: sampled,
        scale: size,
        origin: Point {
            x: center.0,
            y: center.1,
            stroke_id: points[0].stroke_id,
        },
    })
}

/// A stored gesture to match ink against.
///
/// `name` is the id this shape stands for — a `SigilId` or `SignId` the
/// catalog knows. Kept as a string here because a template is loaded from
/// data (§4.4) and the recognizer has no business knowing which vocabulary it
/// came from; wiring it to the catalog is M4.8.
#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub name: String,
    pub cloud: Cloud,
}

impl Template {
    /// Records `points` as a template of `n` samples.
    ///
    /// Returns `None` for ink that cannot be normalised at all.
    pub fn record(name: impl Into<String>, points: &[Point], n: usize) -> Option<Template> {
        Some(Template {
            name: name.into(),
            cloud: normalize(points, n)?,
        })
    }
}

/// One template ranked against a drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Match {
    /// Which template, by position in the slice handed to [`rank`].
    pub index: usize,
    /// Weighted mean distance between matched points, in cloud units.
    ///
    /// Cloud units mean the gesture is one unit across, so `0.05` reads as
    /// "the average point sits five percent of the drawing's width away from
    /// where the template says it should". Smaller is better; zero is exact.
    pub distance: f32,
}

impl Match {
    /// The distance as a `0..=1` confidence, where `1.0` is exact.
    ///
    /// A convenience for showing a person, not something to threshold on —
    /// `distance` is the honest number and it is the one to compare.
    pub fn score(&self) -> f32 {
        1.0 / (1.0 + self.distance.max(0.0))
    }
}

/// How far apart two clouds are, by `$P`'s greedy match.
///
/// Returns `None` when the clouds cannot be compared: either is empty, or they
/// hold different numbers of points. `$P` pairs points one for one, so equal
/// counts are not a convenience but a requirement.
///
/// **One deviation from the paper.** It returns the weighted *mean* rather
/// than the weighted sum — the sum grows with `n`, which makes a distance
/// impossible to read without knowing the sample count. Dividing by the total
/// weight is dividing by a constant, so every ranking is unchanged and the
/// number becomes a fraction of the gesture's own size.
pub fn cloud_distance(a: &Cloud, b: &Cloud) -> Option<f32> {
    let n = a.points.len();
    if n == 0 || b.points.len() != n {
        return None;
    }

    let mut matched = vec![false; n];
    Some(greedy(&a.points, &b.points, &mut matched))
}

/// Ranks every template against `cloud`, nearest first.
///
/// Ties keep the order the templates were given in, so the same ink and the
/// same catalogue always produce the same ranking (§4.3). Templates whose
/// sample count differs from `cloud`'s are skipped rather than reported as
/// infinitely distant — they were never candidates.
pub fn rank(cloud: &Cloud, templates: &[Template]) -> Vec<Match> {
    let n = cloud.points.len();
    let mut matched = vec![false; n];

    let mut found: Vec<Match> = templates
        .iter()
        .enumerate()
        .filter(|(_, template)| template.cloud.points.len() == n && n > 0)
        .map(|(index, template)| Match {
            index,
            distance: greedy(&cloud.points, &template.cloud.points, &mut matched),
        })
        .collect();

    // `sort_by` is stable, so equal distances keep catalogue order. `total_cmp`
    // rather than `partial_cmp().unwrap()`: a total order cannot panic (§4.7).
    found.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    found
}

/// The nearest template, or `None` if none could be compared.
///
/// Deliberately unthresholded. How close is close enough is a question about
/// magic — a sloppy sigil that is still obviously fire should probably work —
/// and it belongs to whoever is compiling a spell, not to the matcher.
pub fn classify(cloud: &Cloud, templates: &[Template]) -> Option<Match> {
    let n = cloud.points.len();
    if n == 0 {
        return None;
    }

    let mut matched = vec![false; n];
    let mut best: Option<Match> = None;

    for (index, template) in templates.iter().enumerate() {
        if template.cloud.points.len() != n {
            continue;
        }
        // Handing the best distance so far down as a ceiling lets a hopeless
        // template be abandoned part-way instead of measured exactly. Most of
        // a catalogue is hopeless, so this is most of the saving — and it is
        // exact, not an approximation: an abandoned walk could only ever have
        // scored worse than the bound it broke.
        let ceiling = best.map_or(f32::INFINITY, |found: Match| found.distance);
        let distance = bounded(&cloud.points, &template.cloud.points, &mut matched, ceiling);
        // Strictly less than, so a tie keeps the earlier template.
        if distance < ceiling {
            best = Some(Match { index, distance });
        }
    }

    best
}

/// `$P`'s greedy cloud match: the nearer of the two directions, over several
/// starting points.
///
/// Both directions are needed because greedy matching is not symmetric —
/// pairing `a`'s points to `b`'s can strand a point that pairing the other way
/// would have placed well. Several starts are needed because the first points
/// matched are the ones matched most confidently, so where the walk begins
/// changes the answer.
///
/// The starts are spaced `n^(1−ε)` apart with `ε = 0.5`, the paper's own
/// figure: `√n` starts rather than all `n`, which is the difference between
/// `O(n³)` and `O(n^2.5)`.
///
/// Measured against §7's 1ms budget with 48 templates at `n = 32` — the real
/// catalogue's size, 13 sigils and 35 signs — the plain algorithm came in at
/// 0.87ms, which is not margin, it is luck. Two exact changes brought it to
/// 0.41ms: searching on squared distances and taking one root per step instead
/// of `n`, and abandoning a walk the moment it cannot beat the best already
/// found. Neither changes an answer.
fn greedy(a: &[Point], b: &[Point], matched: &mut [bool]) -> f32 {
    bounded(a, b, matched, f32::INFINITY)
}

/// [`greedy`], abandoning any walk that cannot beat `ceiling`.
///
/// Returns something at least as large as `ceiling` when every walk was
/// abandoned, which the caller reads as "not better than what I already had".
/// With `ceiling` at infinity this is the plain greedy match.
fn bounded(a: &[Point], b: &[Point], matched: &mut [bool], ceiling: f32) -> f32 {
    let n = a.len();
    let step = ((n as f32).sqrt().floor() as usize).max(1);

    // Every direction and every start tightens the bound for the ones after
    // it, so the later walks are the cheapest.
    let mut best = ceiling;
    let mut start = 0;
    while start < n {
        best = best.min(walk(a, b, start, matched, best));
        best = best.min(walk(b, a, start, matched, best));
        start += step;
    }
    best
}

/// One direction, from one starting point.
///
/// Walks `a` in order from `start`, and for each point claims the nearest
/// unclaimed point of `b`. Claiming is what makes this greedy and also what
/// makes it cheap: no point of `b` can absorb two points of `a`, so a cloud
/// bunched in one corner cannot pretend to match a spread-out one.
fn walk(a: &[Point], b: &[Point], start: usize, matched: &mut [bool], ceiling: f32) -> f32 {
    let n = a.len();
    matched[..n].fill(false);

    // The weights are `1 − k/n` for `k` in `0..n`, which always total
    // `(n+1)/2`. Knowing that up front turns the running sum into something
    // comparable against a ceiling before the walk has finished.
    let total_weight = (n as f32 + 1.0) * 0.5;
    let budget = ceiling * total_weight;

    let mut sum = 0.0;
    let mut i = start;

    loop {
        // Searching on the *square* of the distance and taking the root only
        // once the winner is known. Square root is monotone, so the nearest
        // point is the same either way — and this turns `n²` roots per walk
        // into `n`, which is most of what this loop used to cost.
        let mut nearest = f32::INFINITY;
        let mut claim = 0;
        let here = a[i];
        for (j, candidate) in b.iter().enumerate().take(n) {
            if matched[j] {
                continue;
            }
            let dx = here.x - candidate.x;
            let dy = here.y - candidate.y;
            let d = dx * dx + dy * dy;
            if d < nearest {
                nearest = d;
                claim = j;
            }
        }
        let nearest = nearest.sqrt();
        matched[claim] = true;

        // Confidence falls off along the walk: the first pairings had the
        // whole cloud to choose from, the last had whatever was left. Weighting
        // by how far into the walk we are is the paper's way of trusting them
        // accordingly.
        let travelled = (i + n - start) % n;
        let weight = 1.0 - travelled as f32 / n as f32;
        sum += weight * nearest;

        // Every remaining term is non-negative, so once the bound is broken it
        // can never be recovered.
        if sum > budget {
            return f32::INFINITY;
        }

        i = (i + 1) % n;
        if i == start {
            break;
        }
    }

    sum / total_weight
}

#[cfg(test)]
#[path = "tests/recognizer.rs"]
mod tests;

/// The direction a cloud is longest in, in radians, `0..π`.
///
/// The first eigenvector of the point covariance — the axis of greatest spread.
/// Order-invariant (it is a sum), scale-invariant (the cloud is normalised
/// already), and defined up to 180 degrees, which is exactly the ambiguity
/// canon rule 6 is *about*: a sign and its reversed twin lie on the same axis
/// and mean opposite things.
///
/// Returns `None` for a cloud with no direction at all — a ring, a dot, a
/// perfectly radial mark. That is a real answer and not a failure: such a mark
/// has no front to point, which §2.3 says outright of the non-directional
/// signs, and rotating it would be inventing an orientation it does not have.
pub fn principal_axis(cloud: &Cloud) -> Option<f32> {
    let (mut xx, mut xy, mut yy) = (0.0f32, 0.0f32, 0.0f32);
    for point in &cloud.points {
        xx += point.x * point.x;
        xy += point.x * point.y;
        yy += point.y * point.y;
    }
    // How far from circular the spread is. Below this the axis is noise, and a
    // noisy axis would spin a radial mark to a different angle every frame.
    let spread = ((xx - yy) * (xx - yy) + 4.0 * xy * xy).sqrt();
    let total = xx + yy;
    if total <= f32::EPSILON || spread / total < 0.05 {
        return None;
    }
    // Eigenvector of the larger eigenvalue, as an angle. The factor of two is
    // the usual one for an axis rather than a direction.
    Some(0.5 * (2.0 * xy).atan2(xx - yy))
}

/// The same cloud, turned about its centroid.
///
/// The centroid is already the origin after [`normalize`], so this is a plain
/// rotation and `scale` and `origin` ride through untouched — they describe
/// what was divided out, and turning a drawing changes neither.
pub fn turned(cloud: &Cloud, by: f32) -> Cloud {
    let (sin, cos) = by.sin_cos();
    Cloud {
        points: cloud
            .points
            .iter()
            .map(|point| Point {
                x: point.x * cos - point.y * sin,
                y: point.x * sin + point.y * cos,
                stroke_id: point.stroke_id,
            })
            .collect(),
        scale: cloud.scale,
        origin: cloud.origin,
    }
}

/// One template's best score against a gesture, and how far the gesture had to
/// be turned to get it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aligned {
    pub index: usize,
    pub distance: f32,
    /// Radians the *gesture* was turned by to meet the template. Negate it and
    /// you have how far the drawn mark sits from the template's own angle,
    /// which is what a sign's tilt is measured from (§2.4).
    pub turn: f32,
}

/// Ranks templates against a gesture, allowing the gesture to be turned.
///
/// # Why this exists, and why it is not the default
///
/// [`rank`] deliberately keeps rotation, because §2.2 says a sigil's *shape*
/// is what names it and canon rule 6 makes a reversed mark mean the opposite of
/// an upright one. For a **sigil** that is right and stays right.
///
/// For a **sign** it is unworkable, and the water orb is the proof: four
/// keystones arranged around a ring point in four different directions, so at
/// most one of them can ever sit at the template's own angle. Canon is explicit
/// that this is the normal way to draw a seal — §2.4's whole balance mechanic
/// is about signs at different angles — so a recogniser that cannot read a
/// turned keystone cannot read any real seal.
///
/// So the gesture is aligned to each template by its principal axis before
/// scoring, and **both ways round**, because an axis is defined up to 180
/// degrees and that ambiguity is precisely canon rule 6: a sign and its
/// reversed twin lie on the same axis and do opposite things. The winner
/// reports the turn it needed, so which of the two it was is an answer rather
/// than a coin toss.
///
/// Costs two distance computations per template instead of one. It does *not*
/// sweep angles — that would be twenty-odd times the work for an answer the
/// covariance already gives exactly.
pub fn rank_aligned(cloud: &Cloud, templates: &[Template]) -> Vec<Aligned> {
    let n = cloud.points.len();
    let mine = principal_axis(cloud);

    let mut found: Vec<Aligned> = templates
        .iter()
        .enumerate()
        .filter(|(_, template)| template.cloud.points.len() == n && n > 0)
        .filter_map(|(index, template)| {
            // A mark with no axis cannot be aligned to anything, and neither
            // can a template without one. Both fall back to the upright
            // comparison, which is the honest answer for a radial mark.
            let turn = match (mine, principal_axis(&template.cloud)) {
                (Some(mine), Some(theirs)) => theirs - mine,
                _ => 0.0,
            };
            let mut best: Option<(f32, f32)> = None;
            for by in [turn, turn + std::f32::consts::PI] {
                let moved = turned(cloud, by);
                let mut matched = vec![false; n];
                let distance = greedy(&moved.points, &template.cloud.points, &mut matched);
                if best.is_none_or(|(had, _)| distance < had) {
                    best = Some((distance, by));
                }
            }
            let (distance, turn) = best?;
            Some(Aligned {
                index,
                distance,
                turn,
            })
        })
        .collect();

    found.sort_by(|a, b| {
        a.distance
            .total_cmp(&b.distance)
            .then(a.index.cmp(&b.index))
    });
    found
}
