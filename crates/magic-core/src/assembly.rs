//! Finding rings in a pad full of ink.
//!
//! [`circle`](crate::circle) answers "what circle is this stroke", which is
//! the wrong question to ask on its own. A ring is not a stroke. Canon rule 3
//! puts half a seal on each of two objects; rule 2's prepared spell is closed
//! later by a separate dot of ink; and a plain arc finished off by a short
//! closing line is two strokes making one ring. Ask a stroke at a time and the
//! closing line becomes a ring of its own — with a meaningless radius, because
//! a 6%-of-a-turn arc fits any circle you like — while the ring it closed
//! stays open forever.
//!
//! So grouping is a query over the finished pad, exactly as §3.3 requires:
//! find the strokes that curve far enough to name a circle, then sweep up
//! every other stroke whose ink lies on that circle. Drawing order is not an
//! input, and no stroke is ever rejected for arriving early.
//!
//! **Closure is decided by endpoints, not by angle.** Angular coverage cannot
//! see the difference between a ring whose ends meet and two arcs that overlap
//! in angle without touching — both look like a full turn from the centre.
//! Canon is physical about this: the glowstone halves complete a spell when
//! they *touch*. So a ring is closed when every loose end has another end
//! within reach, and open where one does not.

use std::collections::BTreeSet;

use crate::circle::{self, CircleFit, Coverage};
use crate::Point;

/// A stroke joins a ring when at least this share of its points lie on it.
///
/// Not all of them: the hook a pen leaves on lifting is a couple of samples
/// flicked off the line, and a closing stroke should not be disowned for it.
const MIN_MEMBER_FRACTION: f32 = 0.8;

/// How forgiving the search is. All distances are in the input's units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingSearch {
    /// How far a point may sit off a circle and still count as lying on it.
    ///
    /// This is what lets a separately drawn closing line be recognised as part
    /// of the ring it closes, so it wants to be around the width of the pen.
    pub on_ring: f32,
    /// How close two loose ends must be to count as touching.
    ///
    /// Canon rule 3's toggle: the halves of a split seal complete the ring on
    /// contact and break it again when parted.
    pub join: f32,
    /// Least share of a full turn a stroke must cover before its own fit is
    /// trusted enough to start a ring from.
    ///
    /// A short stroke fits an enormous circle on the strength of a slight
    /// curve, and that circle is noise. Half a ring (rule 3) has to qualify,
    /// so this cannot go above `0.5`.
    pub min_span: f32,
}

impl Default for RingSearch {
    /// Tuned for a pen a few pixels wide on a screen-sized pad.
    fn default() -> Self {
        RingSearch {
            on_ring: 12.0,
            join: 16.0,
            min_span: 0.35,
        }
    }
}

/// One ring found in the ink, and everything that makes it up.
#[derive(Debug, Clone, PartialEq)]
pub struct RingCandidate {
    /// The circle fitted to every stroke in the ring at once.
    pub fit: CircleFit,
    /// How the combined ink is spread around that circle.
    pub coverage: Coverage,
    /// Ids of the strokes forming this ring, ascending.
    ///
    /// More than one is normal, not exceptional — see the module docs.
    pub strokes: Vec<u32>,
    /// Whether every loose end meets another. Canon rule 2's activation test.
    pub closed: bool,
    /// The ends that meet nothing — where the ring is still open.
    ///
    /// Empty exactly when [`RingCandidate::closed`]. Worth having as positions
    /// rather than a count: this is where a dot of ink would finish the spell.
    pub open_ends: Vec<Point>,
}

/// Finds every ring in `points`.
///
/// `points` must have the points of each stroke contiguous, which is how ink
/// is captured. Strokes belonging to no ring are simply absent from the result
/// — inert, not invalid (§3.3). An empty pad yields an empty `Vec`.
///
/// Deterministic: strokes are considered longest-first with the stroke's own
/// position breaking ties, so the same ink always yields the same rings in the
/// same order (§4.3).
pub fn find_rings(points: &[Point], search: &RingSearch) -> Vec<RingCandidate> {
    let strokes: Vec<&[Point]> = points
        .chunk_by(|a, b| a.stroke_id == b.stroke_id)
        .collect();

    // A stroke can start a ring only if it curves far enough round that its
    // own fit means something. Everything else can still *join* one.
    let mut seeds: Vec<(usize, CircleFit)> = strokes
        .iter()
        .enumerate()
        .filter_map(|(index, stroke)| {
            let fit = circle::fit_trimmed(stroke)?;
            let coverage = circle::coverage(stroke, &fit)?;
            (coverage.spanned >= search.min_span).then_some((index, fit))
        })
        .collect();

    // Longest first: the most-drawn stroke is the likeliest true ring, and its
    // fit is the steadiest. The index breaks ties so nothing depends on
    // anything but the ink itself.
    seeds.sort_by_key(|(index, _)| (std::cmp::Reverse(strokes[*index].len()), *index));

    let mut claimed = BTreeSet::new();
    let mut rings = Vec::new();

    for (seed, seed_fit) in seeds {
        if claimed.contains(&seed) {
            continue;
        }

        let mut members = vec![seed];
        for (index, stroke) in strokes.iter().enumerate() {
            if index == seed || claimed.contains(&index) {
                continue;
            }
            if lies_on(stroke, &seed_fit, search.on_ring) {
                members.push(index);
            }
        }
        members.sort_unstable();

        let mut union: Vec<Point> = Vec::new();
        for &index in &members {
            union.extend_from_slice(strokes[index]);
        }

        // Refit on everything: a closing line adds real information about
        // where the circle is, and the seed's own fit was only ever a probe.
        let Some(fit) = circle::fit_trimmed(&union) else {
            continue;
        };
        let Some(coverage) = circle::coverage(&union, &fit) else {
            continue;
        };

        claimed.extend(members.iter().copied());

        let open_ends = loose_ends(&members, &strokes, search.join);
        rings.push(RingCandidate {
            fit,
            coverage,
            strokes: members.iter().map(|&i| strokes[i][0].stroke_id).collect(),
            closed: open_ends.is_empty(),
            open_ends,
        });
    }

    rings
}

/// Whether enough of `stroke` sits on the circle to call it part of the ring.
fn lies_on(stroke: &[Point], fit: &CircleFit, on_ring: f32) -> bool {
    if stroke.is_empty() {
        return false;
    }

    let on = stroke
        .iter()
        .filter(|p| (p.dist(&fit.center) - fit.radius).abs() <= on_ring)
        .count();

    on as f32 >= stroke.len() as f32 * MIN_MEMBER_FRACTION
}

/// Collects the stroke ends that touch no other end.
///
/// Each stroke contributes two ends, its first and last point. An end is
/// satisfied by *any* other end within `join` — including the far end of its
/// own stroke, which is how a ring drawn in one unbroken loop closes.
///
/// Ends are compared by their slot rather than their position, so a
/// single-point stroke does not satisfy itself twice over.
fn loose_ends(members: &[usize], strokes: &[&[Point]], join: f32) -> Vec<Point> {
    let mut ends: Vec<Point> = Vec::with_capacity(members.len() * 2);
    for &index in members {
        let stroke = strokes[index];
        if let (Some(first), Some(last)) = (stroke.first(), stroke.last()) {
            ends.push(*first);
            ends.push(*last);
        }
    }

    // A lone stroke of one point is a dot: its two ends are the same place, so
    // they meet, and the dot reads as a closed ring of radius nothing. Nothing
    // downstream will accept that, but say so here rather than leave it to
    // luck.
    if ends.len() < 2 {
        return ends;
    }

    ends.iter()
        .enumerate()
        .filter(|(slot, end)| {
            !ends
                .iter()
                .enumerate()
                .any(|(other, candidate)| other != *slot && end.dist(candidate) <= join)
        })
        .map(|(_, end)| *end)
        .collect()
}

#[cfg(test)]
#[path = "tests/assembly.rs"]
mod tests;
