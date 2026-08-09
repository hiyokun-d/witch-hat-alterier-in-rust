//! Operations on one raw stroke, before anything tries to recognise it.
//!
//! A captured stroke is not evenly sampled and never can be. Capture drops a
//! point every time the pen has moved `MIN_POINT_SPACING`, which sounds even
//! and is not: a hand slows at curves and corners, so points bunch exactly
//! where the drawing is most interesting and thin out along the easy runs.
//!
//! That unevenness is a bias, not just untidiness. Least squares weights by
//! point *count*, so a slow patch pulls a circle fit toward itself; and `$P`
//! matches point clouds by nearest-neighbour distance, which is meaningless if
//! one cloud is dense where the other is sparse. Both want the same fix, so it
//! lives here rather than inside either of them.
//!
//! Resampling walks the path the pen actually took and drops a new point every
//! fixed fraction of its length. It never invents shape — every point it
//! produces lies on a segment the pen drew.

use crate::Point;

/// Points a stroke is resampled to before matching.
///
/// The `$P` paper's own figure. Thirty-two is enough to tell the gestures in
/// its test sets apart while keeping the greedy match — which is `O(n²)` per
/// template — inside a millisecond across a full catalogue.
pub const MATCH_POINTS: usize = 32;

/// A stroke shorter than this is a dot, not a path, and has no direction to
/// resample along.
const MIN_PATH_LENGTH: f32 = 1e-4;

/// Total length of the path the pen took, following the points in order.
///
/// Not the same as the distance between the ends, and not the same as the
/// perimeter of anything — this is how far the nib actually travelled.
///
/// Segments spanning two different strokes are skipped. A gesture drawn as
/// three separate marks is one gesture, and the leaps between them are the pen
/// in the air.
pub fn path_length(points: &[Point]) -> f32 {
    points
        .windows(2)
        .filter(|pair| pair[0].stroke_id == pair[1].stroke_id)
        .map(|pair| pair[0].dist(&pair[1]))
        .sum()
}

/// Resamples to exactly `n` points spaced evenly along the drawn path.
///
/// Accepts a whole gesture, not just one stroke: points of different strokes
/// are never interpolated between, so a three-mark sigil resamples as one
/// gesture without inventing ink across the gaps. A single stroke is simply
/// the case where there is one such run.
///
/// Returns `None` when there is no path to walk: fewer than two points, fewer
/// than two requested, or every point stacked on one spot.
///
/// The first and last points are always the originals, so the ends stay
/// exactly where they were drawn — endpoints decide ring closure, and nudging
/// them by a fraction of a pixel is not this function's business.
pub fn resample(stroke: &[Point], n: usize) -> Option<Vec<Point>> {
    if n < 2 || stroke.len() < 2 {
        return None;
    }

    let total = path_length(stroke);
    if !total.is_finite() || total < MIN_PATH_LENGTH {
        return None;
    }

    let interval = total / (n - 1) as f32;
    let mut out = Vec::with_capacity(n);
    out.push(stroke[0]);

    // Distance walked since the last point was emitted.
    let mut carried = 0.0;
    // Where along the path we currently stand. Not necessarily an original
    // point — after emitting, we resume from the sample just placed, because a
    // single long segment can hold several of them.
    let mut here = stroke[0];
    let mut next = 1;

    while next < stroke.len() {
        let ahead = stroke[next];

        // The pen lifted here. Jump the gap without banking it as travel, and
        // resume measuring from the new stroke's first point.
        if ahead.stroke_id != here.stroke_id {
            here = ahead;
            next += 1;
            continue;
        }

        let span = here.dist(&ahead);

        if span > 0.0 && carried + span >= interval {
            // The next sample falls inside this segment. Step exactly the
            // remaining distance into it.
            let t = (interval - carried) / span;
            let sample = Point {
                x: here.x + (ahead.x - here.x) * t,
                y: here.y + (ahead.y - here.y) * t,
                // Resampling never moves ink between strokes.
                stroke_id: here.stroke_id,
            };
            out.push(sample);
            here = sample;
            carried = 0.0;
            // `next` deliberately does not advance.
        } else {
            carried += span;
            here = ahead;
            next += 1;
        }
    }

    // Rounding decides whether the final sample lands just before the end of
    // the path or just after it, so the count can come up one short. Repeating
    // the true last point is right either way: it is where the stroke ends.
    let last = *stroke.last()?;
    while out.len() < n {
        out.push(last);
    }
    out.truncate(n);
    // The end must be the drawn end, not a sample that landed near it.
    if let Some(end) = out.last_mut() {
        *end = last;
    }

    Some(out)
}

#[cfg(test)]
#[path = "tests/stroke.rs"]
mod tests;
