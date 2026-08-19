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
//! A ring found here is *reported*, never rejected: a double loop and a
//! figure-eight both come back as candidates with the numbers that give them
//! away ([`RingCandidate::is_simple`]). Deciding what to do about that belongs
//! to the compiler, not to a search.
//!
//! **Closure is decided by endpoints, not by angle.** Angular coverage cannot
//! see the difference between a ring whose ends meet and two arcs that overlap
//! in angle without touching — both look like a full turn from the centre.
//! Canon is physical about this: the glowstone halves complete a spell when
//! they *touch*. So a ring is closed when every loose end has another end
//! within reach, and open where one does not.

use std::collections::BTreeSet;

use crate::Point;
use crate::circle::{self, CircleFit, Coverage, Winding};
use crate::glyph::{Glyph, GlyphId, Ring};
use crate::stroke;

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
    /// Least roundness a stroke's own fit must have before it can start a ring,
    /// as [`CircleFit::quality`].
    ///
    /// **Not the same question as [`RingRules::min_quality`], and conflating
    /// the two was a bug.** This asks whether the ink is a circle at all;
    /// `min_quality` asks whether a ring is neat enough to hold. A sign's
    /// arrowhead — two short lines meeting at a point — clears `min_span`
    /// easily, because its fitted centre lands near the bend and its arms then
    /// sweep most of a turn about it. Nothing else in the search could see that
    /// it was a chevron rather than a small ring.
    ///
    /// Deliberately far below `min_quality`: a rough ring must still *be* a
    /// ring, and canon rule 8 grades it afterwards as `Fleeting`. Rejecting on
    /// size instead would be wrong — rule 5 links several small identical
    /// seals, and those are real rings.
    pub min_roundness: f32,
}

impl Default for RingSearch {
    /// Tuned for a pen a few pixels wide on a screen-sized pad.
    fn default() -> Self {
        RingSearch {
            on_ring: 12.0,
            join: 16.0,
            min_span: 0.35,
            // A hand-drawn ring lands near 0.95; the chevrons this rejects sat
            // at 0.77. Low enough that a genuinely wobbly ring survives to be
            // graded, high enough that a bend is not a circle.
            min_roundness: 0.85,
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
    /// Total length of ink laid down, following the strokes point to point.
    ///
    /// Against the circle's own circumference this says how much of the ring
    /// was actually drawn, and — over `1.0` — how much was drawn twice. A
    /// rough stand-in for the turning number until M4.3 computes it properly.
    pub ink_length: f32,
    /// How many points make up the ring.
    pub points: usize,
    /// How far the ink travels around the circle. The third signal.
    pub winding: Winding,
}

impl RingCandidate {
    /// Whether the ink goes round once and no further.
    ///
    /// Compares turning against direction coverage, because for **any** simple
    /// arc the two agree — half a ring turns half a turn and covers half the
    /// directions. What they catch is everything else:
    ///
    /// | Shape | `spanned` | `turns` |
    /// | --- | --- | --- |
    /// | ring | 1.0 | 1.0 |
    /// | ring with a gap | 0.9 | 0.9 |
    /// | ring drawn twice | 1.0 | 2.0 |
    /// | figure-eight | 1.0 | ~0 |
    ///
    /// Comparing against coverage rather than against `1.0` is what keeps
    /// canon rule 2's prepared spell legal: a ring with a deliberate hole is
    /// not a full turn and must not be rejected for it.
    ///
    /// `tolerance` is in turns. A hand needs a few percent.
    pub fn is_simple(&self, tolerance: f32) -> bool {
        (self.winding.turns - self.coverage.spanned).abs() <= tolerance
            && self.winding.backtrack() <= tolerance
    }

    /// Whether this ring closes the circuit.
    ///
    /// Order matters. Being a ring at all is asked first, because "closed" and
    /// "neat" are meaningless questions about a figure-eight. Then structure
    /// before craft: an open ring is *armed*, and how neatly it was drawn does
    /// not come into it until it is finished.
    pub fn activation(&self, rules: &RingRules) -> Activation {
        if !self.is_simple(rules.simple_tolerance) {
            return Activation::Malformed;
        }
        if !self.closed {
            return Activation::Armed;
        }
        if self.fit.quality() < rules.min_quality {
            return Activation::Fleeting;
        }
        Activation::Active
    }

    /// Compiles this candidate into the [`Ring`] the rest of the engine uses.
    ///
    /// The candidate keeps the evidence — every stroke, the turning numbers,
    /// where the loose ends are. `Ring` keeps only what a spell needs: where,
    /// how big, whether the circuit is closed, and how well it was drawn. The
    /// two exist separately so that a compiler warning can point back at the
    /// measurement it came from.
    ///
    /// A [`Activation::Malformed`] candidate still converts. Refusing here
    /// would leave the caller with no way to say *why* something failed, and
    /// §4.7 has core reporting rather than deciding.
    pub fn to_ring(&self) -> Ring {
        Ring::new(
            self.fit.center,
            self.fit.radius,
            self.closed,
            self.fit.quality(),
        )
    }
}

/// What a ring must satisfy to close a spell's circuit.
///
/// Every value here is **dimensionless**, and that is the whole reason this
/// type exists rather than a pile of constants in the shell. A ratio means the
/// same thing on any screen at any zoom, so it is a statement about magic and
/// belongs in core (§4.2). Anything measured in pixels — how near two ends
/// must be to touch, how wide the pen is — is the shell's business and arrives
/// through [`RingSearch`] instead.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RingRules {
    /// How far turning may stray from coverage and still read as one clean
    /// loop, in turns. See [`RingCandidate::is_simple`].
    pub simple_tolerance: f32,
    /// Least neatness a closed ring needs before it holds rather than fizzles.
    ///
    /// **Invented.** The wiki says a spell "will have a fleeting effect or may
    /// even fail to activate completely if the ring is not circular enough"
    /// and gives no number. This is ours, and it is the only value in this
    /// file canon does not back.
    pub min_quality: f32,
}

impl Default for RingRules {
    fn default() -> Self {
        RingRules {
            // A hand tracing a ring lands within a couple of percent; the
            // shapes this rejects are out by a whole turn.
            simple_tolerance: 0.08,
            min_quality: 0.95,
        }
    }
}

/// Whether a ring closes the circuit, and if not, why not.
///
/// Canon puts two gates on the ring, and they are not the same gate. Rule 2 is
/// structural — a spell fires only once its ring is complete. The Spells page
/// adds a second, about craft: a ring that is not circular enough gives a
/// fleeting effect or fails outright. A seal can pass either and fail the
/// other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    /// Not a ring at all — the ink fits a circle without going round it once.
    Malformed,
    /// Complete, and neat enough to hold. The circuit is closed.
    Active,
    /// A ring with a gap. **Not a mistake** — canon rule 2's prepared spell,
    /// waiting on its last stroke, and rule 3's split seal before contact.
    Armed,
    /// Closed, but too rough to hold. The wiki's own word for it.
    Fleeting,
}

/// What a ring encloses, sorted by canon rule 1.
///
/// Rule 1: every sigil and sign must be drawn inside the ring or touching it;
/// anything else does not count toward the spell. Note the *touching* clause —
/// a stroke that meets the ring is part of the seal even though it is not
/// within it, which is also how rule 5 links two glyphs with a line.
///
/// Identity only. What the enclosed ink *means* — which sigil, which sign — is
/// the recognizer's job (M4.5 onward); this says nothing about it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RingContents {
    /// Strokes lying wholly within the ring.
    pub inside: Vec<u32>,
    /// Strokes that cross or meet the ring. Count toward the spell.
    pub touching: Vec<u32>,
    /// Strokes wholly outside. Do not count — rule 1.
    pub outside: Vec<u32>,
    /// How many points the enclosed ink holds, touching strokes included.
    pub points: usize,
    /// How far the enclosed ink reaches from the ring's centre.
    ///
    /// Against the ring's own radius this is the ratio the wiki says sets a
    /// spell's intensity: "the size of a sigil in relation to the ring
    /// determines the intensity and strength of the spell's effect". Recorded
    /// now because it is unrecoverable later — the recognizer scores sigils on
    /// shape alone and deliberately never sees scale (§3.1).
    pub extent: f32,
}

impl RingCandidate {
    /// Sorts every stroke on the pad by where it sits relative to this ring.
    ///
    /// The ring's own strokes are excluded — they are the ring, not its
    /// contents. `tolerance` is the same slack used to decide a stroke lies on
    /// the ring, and belongs to the caller for the usual reason: core does not
    /// know how wide a pen is.
    pub fn contents(&self, points: &[Point], tolerance: f32) -> RingContents {
        let mut found = RingContents::default();
        let mut extent: f32 = 0.0;

        for stroke in points.chunk_by(|a, b| a.stroke_id == b.stroke_id) {
            let Some(id) = stroke.first().map(|p| p.stroke_id) else {
                continue;
            };
            if self.strokes.contains(&id) {
                continue;
            }

            let inner = self.fit.radius - tolerance;
            let outer = self.fit.radius + tolerance;
            let mut nearest = f32::INFINITY;
            let mut furthest: f32 = 0.0;
            for p in stroke {
                let d = p.dist(&self.fit.center);
                nearest = nearest.min(d);
                furthest = furthest.max(d);
            }

            if furthest <= inner {
                found.inside.push(id);
            } else if nearest >= outer {
                // Rule 1: does not count toward the spell.
                found.outside.push(id);
                continue;
            } else {
                found.touching.push(id);
            }

            found.points += stroke.len();
            extent = extent.max(furthest);
        }

        found.extent = extent;
        found
    }
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
    let strokes: Vec<&[Point]> = points.chunk_by(|a, b| a.stroke_id == b.stroke_id).collect();

    // A stroke can start a ring only if it curves far enough round that its
    // own fit means something. Everything else can still *join* one.
    let mut seeds: Vec<(usize, CircleFit)> = strokes
        .iter()
        .enumerate()
        .filter_map(|(index, stroke)| {
            let fit = circle::fit_trimmed(stroke)?;
            let coverage = circle::coverage(stroke, &fit)?;
            (coverage.spanned >= search.min_span && fit.quality() >= search.min_roundness)
                .then_some((index, fit))
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
            // Per stroke, so the jump from the end of one to the start of the
            // next is not counted as ink that was never drawn.
            ink_length: members
                .iter()
                .map(|&i| stroke::path_length(strokes[i]))
                .sum(),
            points: union.len(),
            winding: circle::winding(&union, fit.center),
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

/// Which ring each ring sits inside, as indices into `rings`.
///
/// Canon rule 4 gates an inner ring on its outer one, and rings nest by
/// **containment** — there is no other way to draw it. So this is geometry, not
/// bookkeeping: a ring is nested in the *smallest* ring that encloses it, which
/// is what makes a three-deep stack resolve to a chain rather than to everything
/// pointing at the outermost.
///
/// A ring is enclosed when its centre is inside the other and it fits within
/// the remaining room. Touching does not count — two rings that graze are two
/// seals, and rule 5's linking is what joins those.
pub fn nesting(rings: &[RingCandidate]) -> Vec<Option<usize>> {
    rings
        .iter()
        .enumerate()
        .map(|(i, inner)| {
            rings
                .iter()
                .enumerate()
                .filter(|(j, outer)| {
                    *j != i
                        && outer.fit.radius > inner.fit.radius
                        && inner.fit.center.dist(&outer.fit.center) + inner.fit.radius
                            <= outer.fit.radius
                })
                // The smallest enclosing ring is the parent; anything larger is
                // that ring's own parent.
                .min_by(|(_, a), (_, b)| a.fit.radius.total_cmp(&b.fit.radius))
                .map(|(j, _)| j)
        })
        .collect()
}

/// Pairs of rings joined by a stroke that touches both — canon rule 5.
///
/// > "Two glyphs joined by a line link their effects."
///
/// A joining line is ink belonging to neither ring's own strokes that comes
/// within `tolerance` of both. Pairs are returned once, lower index first, so
/// the result does not depend on which ring was found first (§4.3).
pub fn links(rings: &[RingCandidate], points: &[Point], tolerance: f32) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let strokes: Vec<&[Point]> = points.chunk_by(|a, b| a.stroke_id == b.stroke_id).collect();

    for stroke in strokes {
        let Some(first) = stroke.first() else {
            continue;
        };
        // A ring's own ink cannot be the line that links it to something.
        let touched: Vec<usize> = rings
            .iter()
            .enumerate()
            .filter(|(_, ring)| !ring.strokes.contains(&first.stroke_id))
            .filter(|(_, ring)| {
                stroke
                    .iter()
                    .any(|p| (p.dist(&ring.fit.center) - ring.fit.radius).abs() <= tolerance)
            })
            .map(|(i, _)| i)
            .collect();

        for (a, &left) in touched.iter().enumerate() {
            for &right in &touched[a + 1..] {
                let pair = (left.min(right), left.max(right));
                if !found.contains(&pair) {
                    found.push(pair);
                }
            }
        }
    }

    found.sort_unstable();
    found
}

/// Turns a pad of rings into the glyphs the compiler takes.
///
/// The last step before magic: every ring becomes a [`Glyph`] carrying what it
/// encloses, which ring it nests in (rule 4) and which it is linked to (rule 5).
/// Ids are the ring's index, so a warning naming `#2` points at the third ring
/// in the same list the overlay numbers.
///
/// **What it cannot do yet is name anything.** The sigil is always `None` and
/// the sign list is always empty, because identifying ink is the recognizer's
/// job and `templates.ron` is empty until the runes are traced. Every glyph
/// built here therefore compiles to rule 9's discharge, which is the correct
/// answer for a ring holding ink nobody can read — not a placeholder.
pub fn glyphs(rings: &[RingCandidate], points: &[Point], tolerance: f32) -> Vec<Glyph> {
    let parents = nesting(rings);
    let joined = links(rings, points, tolerance);

    rings
        .iter()
        .enumerate()
        .map(|(i, ring)| {
            let mut glyph = Glyph::new(GlyphId(i as u32), None, ring.to_ring());
            let held = ring.contents(points, tolerance);
            glyph.sigil_extent = held.extent;
            // Everything the ring holds, since nothing here can name any of it.
            // The day the recognizer can, this drops by one per mark it reads.
            glyph.unnamed = held.inside.len() + held.touching.len();
            glyph.parent = parents[i].map(|j| GlyphId(j as u32));
            glyph.linked = joined
                .iter()
                .filter_map(|&(a, b)| match (a, b) {
                    (a, b) if a == i => Some(GlyphId(b as u32)),
                    (a, b) if b == i => Some(GlyphId(a as u32)),
                    _ => None,
                })
                .collect();
            glyph
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/assembly.rs"]
mod tests;
