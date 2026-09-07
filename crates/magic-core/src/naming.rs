//! Turning the ink inside a ring into a sigil and some signs.
//!
//! This is the step the whole engine has been missing. `assembly::glyphs` finds
//! rings and what they hold; the recognizer can tell one shape from another;
//! the compiler knows what every sigil and sign does. Nothing joined them, so
//! every seal drawn by hand compiled to canon rule 9's discharge and the
//! compiler may as well not have existed.
//!
//! # The hard part is not recognising, it is segmenting
//!
//! A fire sigil is four strokes and a column sign is two, so "one stroke, one
//! mark" is wrong for almost everything. The first two attempts both tried to
//! settle it with a distance: strokes closer than *this* are one mark. Both
//! failed, and the traced runes say why.
//!
//! Measured on the runes actually recorded in this app, the widest gap inside
//! one mark is **35px** — water is three teardrops drawn well apart. The gap
//! between a sigil and a keystone beside it, in a seal that must segment
//! correctly, is **32px**. There is no threshold between 35 and 32. Any single
//! number either splits water into three unreadable pieces or glues a sign onto
//! the sigil, and clustering on stroke *centres* instead of ink only moves
//! which case breaks.
//!
//! So the segmentation is not decided by geometry alone. Geometry proposes and
//! **the recognizer disposes**:
//!
//! 1. Merge strokes agglomeratively by the gap between their ink, cheapest join
//!    first. That gives a binary tree whose leaves are strokes and whose root is
//!    everything the ring holds — every *spatially sensible* grouping, and no
//!    others, in `2n-1` nodes rather than `2^n` subsets.
//! 2. Score every node by asking the recognizer what it is: a clear match
//!    scores by how clear, ink nobody can read scores a penalty.
//! 3. Take the best cut of the tree by dynamic programming — each node is
//!    either one mark or the best its two halves can do apart.
//!
//! A blob that reads as nothing loses to its parts when the parts read as
//! something, and wins when they do not. Water's three teardrops merge because
//! separately they are nothing and together they are water; the sigil and the
//! keystone stay apart because together they are nothing and separately they
//! are two marks. Same tree, same rule, opposite answers — which is what a
//! threshold could never do.
//!
//! # What it still has to decide
//!
//! **Which mark is the sigil.** Not by position — §2.2 is explicit that a
//! sigil's location does not change its behaviour, so choosing "the middle one"
//! would be inventing a rule canon denies. The *template* says: a shape
//! recorded as a sigil names a sigil, one recorded as a sign names a sign.
//!
//! **Where a sign sits.** `placement` is the angle from the ring's centre and
//! `orientation` comes out of the mark's own long axis, because §2.4 makes the
//! tilt between them the difference between reach and spin.

use crate::Point;
use crate::assembly::RingCandidate;
use crate::catalog::{SigilId, SignId};
use crate::glyph::{Glyph, Sign};
use crate::recognizer::{self, Template};
use crate::stroke;
use crate::templates::{Kind, Recorded};

/// How close is close enough to call a drawing that shape.
///
/// **Ours.** `recognizer::classify` is deliberately unthresholded — how close is
/// close enough is a question about magic, and this is where it finally has to
/// be answered. Distance is the mean miss in gesture widths, so `0.32` means the
/// average point lands within a third of the drawing's own size: loose, because
/// a hand is not a plotter and a sloppy fire sigil that is obviously fire should
/// work.
pub const NEAR_ENOUGH: f32 = 0.32;

/// How much closer the winner must be than the nearest *different* rune.
///
/// **Ours**, and the half a distance threshold alone cannot do. `$P` always
/// returns a nearest template, so on its own "nearest, and near enough" will
/// happily call a scribble a fire sigil — which is exactly what
/// `ink_nobody_recognises_is_counted_rather_than_guessed` caught at 0.5.
///
/// Real ink is *decisively* nearer one shape than the others; noise is roughly
/// as far from everything. So the shape has to win, and win clearly, or the
/// mark stays unread. The built-ins sit at least 0.15 apart from each other by
/// test, so a margin of a third of that is a real bar and not a formality.
pub const CLEAR_MARGIN: f32 = 0.05;

/// What a mark nobody can read costs, against the `0.0..=1.0` a readable one
/// earns.
///
/// **Ours**, and the one number that tunes the segmentation. It is the exchange
/// rate between "read fewer things well" and "read more things badly": at
/// `0.5`, splitting a blob in two is worth it as soon as one half reads at all
/// clearly, and never worth it when both halves read as nothing.
const UNREAD_COST: f32 = 0.5;

/// What each *extra* mark costs, on the same scale.
///
/// **Ours**, and a parsimony prior: prefer the simplest reading that explains
/// the ink. Without it a mark can be read twice over — the built-in light sigil
/// is a diamond inside a square, and the square *alone* matches `light` nearly
/// as well as the whole thing does, so two marks each called light scored
/// higher than one and the seal reported a sigil it could not place. A part of
/// a rune resembling the rune is a discovery about the shape, not a second
/// rune, and this is the term that says so.
const SPLIT_COST: f32 = 0.25;

/// How many points of a stroke the proximity test looks at.
///
/// The merge order is quadratic in points, so a long stroke would dominate it.
/// Subsampled by a fixed stride, which keeps the answer the same on every
/// machine (§4.3) and cannot miss a join: a gap wide enough to matter is wide
/// compared to the spacing between one stroke's own samples.
const PROBES: usize = 24;

/// The mean position of a run of points.
fn centre(points: &[Point]) -> (f32, f32) {
    let n = points.len().max(1) as f32;
    (
        points.iter().map(|p| p.x).sum::<f32>() / n,
        points.iter().map(|p| p.y).sum::<f32>() / n,
    )
}

/// A stroke thinned to about [`PROBES`] points, both ends kept.
///
/// The ends matter more than the middle: a bar meets a chevron at a tip, so
/// dropping the last sample is exactly how a join goes missing.
fn probes(run: &[Point]) -> Vec<(f32, f32)> {
    if run.len() <= PROBES {
        return run.iter().map(|p| (p.x, p.y)).collect();
    }
    let stride = run.len().div_ceil(PROBES);
    let mut out: Vec<(f32, f32)> = run.iter().step_by(stride).map(|p| (p.x, p.y)).collect();
    if let Some(last) = run.last() {
        out.push((last.x, last.y));
    }
    out
}

/// The gap between two strokes: the distance between their nearest points.
fn gap(a: &[(f32, f32)], b: &[(f32, f32)]) -> f32 {
    let mut best = f32::MAX;
    for (ax, ay) in a {
        for (bx, by) in b {
            let (dx, dy) = (ax - bx, ay - by);
            best = best.min(dx * dx + dy * dy);
        }
    }
    best.sqrt()
}

/// One candidate mark: some strokes, and what the recognizer made of them.
#[derive(Debug, Clone)]
pub struct Mark {
    /// Which strokes it is made of, in ascending order.
    pub strokes: Vec<u32>,
    /// Which recorded shape it was read as. `None` is ink nobody can name, and
    /// is a perfectly legal answer (§3.3).
    pub read: Option<usize>,
    /// The size the recognizer divided out — a sigil's extent, a sign's power
    /// (§2.4, §3.1). Captured here because nothing downstream could recover it.
    pub scale: f32,
    /// Radians the drawing had to be turned to meet the template it matched.
    /// Always zero for a sigil, which is matched upright.
    pub turn: f32,
}

/// A node of the merge tree: some strokes, and the two halves it came from.
struct Node {
    strokes: Vec<usize>,
    halves: Option<(usize, usize)>,
}

/// Reads one group of strokes, if the recognizer can name it clearly.
/// What one group of strokes turned out to be.
#[derive(Debug, Clone, Copy, Default)]
struct Read {
    /// Which recorded shape, if the recogniser could name it clearly.
    found: Option<usize>,
    /// The size that was divided out — a sigil's extent, a sign's power.
    scale: f32,
    /// How far off the winner was, whether or not it was accepted. The DP needs
    /// this even for a rejected mark.
    distance: f32,
    /// Radians the drawing had to be turned to meet the template it matched.
    /// Zero for a sigil, which is never turned (§2.2, canon rule 6).
    turn: f32,
}

/// Reads one group of strokes, if the recognizer can name it clearly.
///
/// # Sigils are matched upright; signs are matched turned
///
/// This is the one place the two vocabularies are treated differently, and both
/// halves are canon.
///
/// A **sigil** is scored upright. §2.2 says a sigil's shape is what names it,
/// and canon rule 6 makes a reversed mark do the opposite of an upright one, so
/// letting a sigil rotate freely would erase a distinction the engine is built
/// on.
///
/// A **sign** is scored against every template at that template's own angle,
/// both ways round. Four keystones arranged around a ring point in four
/// different directions — that is not an edge case, it is what §2.4's entire
/// balance mechanic is *made of* — so at most one of them could ever sit at the
/// template's recorded angle. A recogniser that cannot read a turned keystone
/// cannot read a real seal, and this was exactly the bug: a stamped water orb
/// segmented into "column, wind" instead of "water and four levitation signs",
/// because three of its four arrows were unreadable at the angle they were
/// drawn.
fn read(ink: &[Point], shapes: &[Recorded], all: &[Template]) -> Read {
    let Some(cloud) = recognizer::normalize(ink, stroke::MATCH_POINTS) else {
        return Read {
            distance: NEAR_ENOUGH,
            ..Read::default()
        };
    };
    let miss = Read {
        scale: cloud.scale,
        distance: NEAR_ENOUGH,
        ..Read::default()
    };

    // Both rankings, and the better answer wins. Running only the aligned one
    // would let a rotated *sigil* through, and running only the upright one is
    // the bug above. A sigil that happens to score well aligned still has to
    // beat its own upright score to be chosen, and it never can — aligning can
    // only ever lower a distance.
    let upright = recognizer::rank(&cloud, all);
    let aligned = recognizer::rank_aligned(&cloud, all);

    // Ranked rather than classified: the runner-up is what says whether the
    // winner actually won.
    let mut ranked: Vec<(usize, f32, f32)> = shapes
        .iter()
        .enumerate()
        .filter_map(|(index, rune)| match rune.kind {
            Kind::Sigil => upright
                .iter()
                .find(|found| found.index == index)
                .map(|found| (index, found.distance, 0.0)),
            Kind::Sign => aligned
                .iter()
                .find(|found| found.index == index)
                .map(|found| (index, found.distance, found.turn)),
        })
        .collect();
    ranked.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));

    let Some(&(index, distance, turn)) = ranked.first() else {
        return miss;
    };
    let winner = &shapes[index];
    let rival = ranked.iter().skip(1).find(|(other, _, _)| {
        let theirs = &shapes[*other];
        theirs.kind != winner.kind || theirs.template.name != winner.template.name
    });
    let clear = rival.is_none_or(|(_, other, _)| other - distance >= CLEAR_MARGIN);
    if distance > NEAR_ENOUGH || !clear {
        return Read {
            scale: cloud.scale,
            distance,
            ..Read::default()
        };
    }
    Read {
        found: Some(index),
        scale: cloud.scale,
        distance,
        turn,
    }
}

/// Splits a set of strokes into the marks that read best.
///
/// See the module docs for why this is a search over a merge tree rather than a
/// distance threshold. Deterministic throughout (§4.3): merges are taken
/// cheapest-gap first with the stroke order breaking ties, and a node that
/// scores equal to its two halves stays whole.
pub fn segment(points: &[Point], ids: &[u32], shapes: &[Recorded]) -> Vec<Mark> {
    let mut strokes: Vec<(u32, Vec<Point>)> = Vec::new();
    for run in points.chunk_by(|a, b| a.stroke_id == b.stroke_id) {
        let Some(id) = run.first().map(|p| p.stroke_id) else {
            continue;
        };
        if ids.contains(&id) {
            strokes.push((id, run.to_vec()));
        }
    }
    if strokes.is_empty() || shapes.is_empty() {
        return Vec::new();
    }
    let sampled: Vec<Vec<(f32, f32)>> = strokes.iter().map(|(_, run)| probes(run)).collect();

    // Every stroke against every other, once. Single-link between two clusters
    // is then the smallest entry across their members.
    let n = strokes.len();
    let mut between = vec![f32::MAX; n * n];
    for one in 0..n {
        for two in (one + 1)..n {
            let d = gap(&sampled[one], &sampled[two]);
            between[one * n + two] = d;
            between[two * n + one] = d;
        }
    }

    // Agglomerate. Leaves first, so a node's halves always have a lower index
    // than the node itself and one forward pass can score the whole tree.
    let mut nodes: Vec<Node> = (0..n)
        .map(|index| Node {
            strokes: vec![index],
            halves: None,
        })
        .collect();
    let mut open: Vec<usize> = (0..n).collect();
    while open.len() > 1 {
        let mut nearest = (f32::MAX, 0usize, 1usize);
        for left in 0..open.len() {
            for right in (left + 1)..open.len() {
                let d = nodes[open[left]]
                    .strokes
                    .iter()
                    .flat_map(|&one| {
                        nodes[open[right]]
                            .strokes
                            .iter()
                            .map(move |&two| (one, two))
                    })
                    .map(|(one, two)| between[one * n + two])
                    .fold(f32::MAX, f32::min);
                if d < nearest.0 {
                    nearest = (d, left, right);
                }
            }
        }
        let (_, left, right) = nearest;
        let (a, b) = (open[left], open[right]);
        let mut merged = nodes[a].strokes.clone();
        merged.extend(nodes[b].strokes.iter().copied());
        merged.sort_unstable();
        nodes.push(Node {
            strokes: merged,
            halves: Some((a, b)),
        });
        // `right > left`, so removing it first leaves `left` where it was.
        open.remove(right);
        open[left] = nodes.len() - 1;
    }

    let all: Vec<Template> = shapes.iter().map(|r| r.template.clone()).collect();

    // Score every node once, then take the best cut. `best` holds what a node
    // is worth and which nodes that answer is made of.
    let mut best: Vec<(f32, Vec<usize>)> = Vec::with_capacity(nodes.len());
    let mut marks: Vec<Mark> = Vec::with_capacity(nodes.len());
    for (index, node) in nodes.iter().enumerate() {
        let ink: Vec<Point> = node
            .strokes
            .iter()
            .flat_map(|&which| strokes[which].1.iter().copied())
            .collect();
        // The distance comes back with the answer. It used to be recovered by
        // running `normalize` and `rank` a *second* time for the score, which
        // doubled the cost of the whole search for a number the first call
        // already had in its hand.
        let said = read(&ink, shapes, &all);
        marks.push(Mark {
            strokes: node.strokes.iter().map(|&which| strokes[which].0).collect(),
            read: said.found,
            scale: said.scale,
            turn: said.turn,
        });

        // A clear match earns by how clear it is; ink nobody can read costs.
        let alone = match said.found {
            Some(_) => 1.0 - said.distance / NEAR_ENOUGH,
            None => -UNREAD_COST,
        };
        // `>=` keeps a node whole on a tie: fewer, larger marks is the reading
        // that matches how a person draws.
        match node.halves {
            Some((a, b)) if best[a].0 + best[b].0 - SPLIT_COST > alone => {
                let mut parts = best[a].1.clone();
                parts.extend(best[b].1.iter().copied());
                best.push((best[a].0 + best[b].0 - SPLIT_COST, parts));
            }
            _ => best.push((alone, vec![index])),
        }
    }

    let root = best.len() - 1;
    let mut picked = best[root].1.clone();
    picked.sort_unstable();
    picked
        .into_iter()
        .map(|index| marks[index].clone())
        .collect()
}

/// The angle of a mark's longest axis, for a sign's `orientation`.
///
/// The direction from the mark's centre to the point furthest from it. Crude,
/// and right for the shapes that matter: a column and a levitation sign are
/// both long in the direction they point.
fn long_axis(points: &[Point]) -> f32 {
    let (cx, cy) = centre(points);
    let far = points
        .iter()
        .max_by(|a, b| {
            let da = (a.x - cx).powi(2) + (a.y - cy).powi(2);
            let db = (b.x - cx).powi(2) + (b.y - cy).powi(2);
            da.total_cmp(&db)
        })
        .copied();
    match far {
        Some(p) if (p.y - cy) != 0.0 || (p.x - cx) != 0.0 => (p.y - cy).atan2(p.x - cx),
        _ => 0.0,
    }
}

/// Names everything the ring holds that it can, and counts what it cannot.
///
/// Total, like the compiler: ink nobody recognises is not an error, it stays in
/// `unnamed` and the seal reports how much of itself could not be read.
pub fn name(
    glyph: &mut Glyph,
    ring: &RingCandidate,
    points: &[Point],
    shapes: &[Recorded],
    on_ring: f32,
) {
    let held = ring.contents(points, on_ring);
    let mut ids = held.inside.clone();
    ids.extend(held.touching.iter().copied());
    if ids.is_empty() || shapes.is_empty() {
        return;
    }

    glyph.sigil = None;
    glyph.signs.clear();
    let mut unread = 0;

    for mark in segment(points, &ids, shapes) {
        let Some(found) = mark.read else {
            unread += 1;
            continue;
        };
        let named = &shapes[found];
        match named.kind {
            // The first sigil wins. Canon has "the vast majority" of seals
            // holding one, and says nothing about what two would mean, so a
            // second is left unread rather than guessed at.
            Kind::Sigil if glyph.sigil.is_none() => {
                glyph.sigil = Some(SigilId::from(named.template.name.as_str()));
                glyph.sigil_extent = mark.scale;
            }
            Kind::Sigil => unread += 1,
            Kind::Sign => {
                let ink: Vec<Point> = points
                    .iter()
                    .copied()
                    .filter(|p| mark.strokes.contains(&p.stroke_id))
                    .collect();
                let (cx, cy) = centre(&ink);
                let (rx, ry) = (ring.fit.center.x, ring.fit.center.y);
                glyph.signs.push(Sign {
                    kind: SignId::from(named.template.name.as_str()),
                    // Where it sits around the ring — §3.3: inward and outward
                    // are questions about direction *relative to position*.
                    placement: (cy - ry).atan2(cx - rx),
                    orientation: long_axis(&ink),
                    // Size is power (§2.4), in the ring's own units so the two
                    // are comparable.
                    size: mark.scale,
                    reversed: false,
                });
            }
        }
    }

    glyph.unnamed = unread;
}

#[cfg(test)]
#[path = "tests/naming.rs"]
mod tests;

/// Names marks that belong to no ring, for a readout.
///
/// The same segmentation and the same thresholds as [`name`], deliberately: a
/// mark must not change its mind about what it is the moment a ring is drawn
/// around it. Returns each mark's centre and what it was read as — or that it
/// was not read, because "I could not tell" is the answer worth showing.
///
/// Nothing here affects a spell. Canon rule 1 is clear that ink outside every
/// ring contributes nothing, and §3.3 that it is inert rather than invalid.
pub fn loose(points: &[Point], ids: &[u32], shapes: &[Recorded]) -> Vec<((f32, f32), String)> {
    segment(points, ids, shapes)
        .into_iter()
        .map(|mark| {
            let ink: Vec<Point> = points
                .iter()
                .copied()
                .filter(|p| mark.strokes.contains(&p.stroke_id))
                .collect();
            let said = match mark.read {
                Some(found) => format!("{} (no ring)", shapes[found].template.name),
                None => "unread".to_string(),
            };
            (centre(&ink), said)
        })
        .collect()
}
