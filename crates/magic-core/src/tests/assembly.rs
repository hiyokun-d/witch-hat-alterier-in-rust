//! Tests for `assembly`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use std::f32::consts::TAU;

/// Points along an arc, inclusive of both ends, tagged with a stroke id.
fn arc(id: u32, cx: f32, cy: f32, r: f32, from_deg: f32, to_deg: f32, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            let a = (from_deg + (to_deg - from_deg) * t).to_radians();
            Point {
                x: cx + r * a.cos(),
                y: cy + r * a.sin(),
                stroke_id: id,
            }
        })
        .collect()
}

/// A full ring drawn as one unbroken stroke, ends meeting.
fn ring(id: u32, cx: f32, cy: f32, r: f32, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let a = (i as f32 / (n - 1) as f32) * TAU;
            Point {
                x: cx + r * a.cos(),
                y: cy + r * a.sin(),
                stroke_id: id,
            }
        })
        .collect()
}

fn search() -> RingSearch {
    RingSearch::default()
}

#[test]
fn an_empty_pad_has_no_rings() {
    assert!(find_rings(&[], &search()).is_empty());
}

#[test]
fn one_unbroken_loop_is_one_closed_ring() {
    let rings = find_rings(&ring(0, 0.0, 0.0, 120.0, 200), &search());

    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].strokes, vec![0]);
    assert!(rings[0].closed, "ends meet, so the ring is closed");
    assert!(rings[0].open_ends.is_empty());
    assert!((rings[0].fit.radius - 120.0).abs() < 1.0);
}

/// Canon rule 2 — the prepared spell. A deliberate hole leaves two loose ends.
#[test]
fn a_ring_with_a_gap_is_open_and_reports_both_ends() {
    let rings = find_rings(&arc(0, 0.0, 0.0, 120.0, 0.0, 300.0, 150), &search());

    assert_eq!(rings.len(), 1);
    assert!(!rings[0].closed);
    assert_eq!(rings[0].open_ends.len(), 2, "one end each side of the hole");
}

/// The bug from the screenshot. An arc plus the short line that closes it is
/// **one** ring, and it is closed. Per-stroke fitting made the line a ring of
/// its own and left the arc open forever.
#[test]
fn a_closing_line_joins_the_ring_it_closes() {
    let mut ink = arc(0, 0.0, 0.0, 120.0, 0.0, 320.0, 160);
    // A short stroke laid along the missing 40°, meeting both ends.
    ink.extend(arc(1, 0.0, 0.0, 120.0, 318.0, 362.0, 17));

    let rings = find_rings(&ink, &search());

    assert_eq!(rings.len(), 1, "one ring, not two");
    assert_eq!(rings[0].strokes, vec![0, 1]);
    assert!(rings[0].closed, "the line closed it");
    assert!(rings[0].open_ends.is_empty());
}

/// The other half of the same bug: on its own, that closing line is not a ring.
/// It spans 6% of a turn and fits whatever circle the noise suggests.
#[test]
fn a_short_line_alone_is_not_a_ring() {
    let ink = arc(0, 0.0, 0.0, 120.0, 318.0, 340.0, 17);
    assert!(
        find_rings(&ink, &search()).is_empty(),
        "a 22° scratch is not a ring"
    );
}

/// Canon rule 3 — half a seal on each of two objects, completed by contact.
#[test]
fn two_halves_that_touch_make_one_closed_ring() {
    let mut ink = arc(0, 0.0, 0.0, 100.0, 0.0, 180.0, 90);
    ink.extend(arc(1, 0.0, 0.0, 100.0, 180.0, 360.0, 90));

    let rings = find_rings(&ink, &search());

    assert_eq!(rings.len(), 1);
    assert_eq!(rings[0].strokes, vec![0, 1]);
    assert!(rings[0].closed);
}

/// The second bug from the screenshot: angular coverage alone called this
/// closed. Two arcs can cover every direction and still not touch.
#[test]
fn two_halves_that_do_not_touch_stay_open() {
    let mut ink = arc(0, 0.0, 0.0, 100.0, 5.0, 175.0, 90);
    ink.extend(arc(1, 0.0, 0.0, 100.0, 185.0, 355.0, 90));

    let rings = find_rings(&ink, &search());
    assert_eq!(rings.len(), 1);

    // Angle says the ring is complete — every direction has ink in it.
    assert!(
        rings[0].coverage.spans_full_turn(20.0),
        "this is exactly the illusion endpoints exist to see through"
    );
    // Endpoints say otherwise, and endpoints are right.
    assert!(!rings[0].closed);
    assert_eq!(rings[0].open_ends.len(), 4);
}

#[test]
fn separating_two_halves_reopens_the_ring() {
    let joined = {
        let mut ink = arc(0, 0.0, 0.0, 100.0, 0.0, 180.0, 90);
        ink.extend(arc(1, 0.0, 0.0, 100.0, 180.0, 360.0, 90));
        find_rings(&ink, &search())
    };
    // The same seal with one half slid away along the ring.
    let parted = {
        let mut ink = arc(0, 0.0, 0.0, 100.0, 0.0, 170.0, 90);
        ink.extend(arc(1, 0.0, 0.0, 100.0, 190.0, 350.0, 90));
        find_rings(&ink, &search())
    };

    assert!(joined[0].closed);
    assert!(!parted[0].closed, "rule 3 toggles both ways");
}

/// Canon rule 4 — a spell wrapped in a second ring. Two rings, not one blur.
#[test]
fn nested_rings_stay_separate() {
    let mut ink = ring(0, 0.0, 0.0, 60.0, 120);
    ink.extend(ring(1, 0.0, 0.0, 150.0, 200));

    let mut rings = find_rings(&ink, &search());
    assert_eq!(rings.len(), 2);

    rings.sort_by(|a, b| a.fit.radius.total_cmp(&b.fit.radius));
    assert!((rings[0].fit.radius - 60.0).abs() < 2.0);
    assert!((rings[1].fit.radius - 150.0).abs() < 2.0);
    assert!(rings.iter().all(|r| r.strokes.len() == 1));
}

/// Canon rule 5 — two glyphs joined by a line. Separate rings, and the link is
/// not swallowed by either.
#[test]
fn two_rings_side_by_side_are_two_rings() {
    let mut ink = ring(0, -200.0, 0.0, 80.0, 120);
    ink.extend(ring(1, 200.0, 0.0, 80.0, 120));

    let rings = find_rings(&ink, &search());
    assert_eq!(rings.len(), 2);
    assert!(rings.iter().all(|r| r.closed));
}

/// A sigil sits inside the ring, nowhere near the circumference, so it must
/// not be absorbed into it. §3.3 — strokes belonging to no ring are inert.
#[test]
fn ink_inside_the_ring_is_not_part_of_it() {
    let mut ink = ring(0, 0.0, 0.0, 150.0, 200);
    ink.extend(arc(1, 0.0, 0.0, 20.0, 0.0, 300.0, 30));

    let rings = find_rings(&ink, &search());
    let outer = rings.iter().find(|r| r.fit.radius > 100.0).unwrap();
    assert_eq!(outer.strokes, vec![0], "the sigil is not part of the ring");
}

#[test]
fn a_stroke_may_belong_to_only_one_ring() {
    let mut ink = ring(0, 0.0, 0.0, 100.0, 150);
    ink.extend(arc(1, 0.0, 0.0, 100.0, 40.0, 300.0, 120));

    let rings = find_rings(&ink, &search());
    let mut seen: Vec<u32> = rings.iter().flat_map(|r| r.strokes.clone()).collect();
    let before = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(before, seen.len(), "a stroke was claimed twice");
}

#[test]
fn drawing_order_does_not_change_the_result() {
    let arc_first = {
        let mut ink = arc(0, 0.0, 0.0, 120.0, 0.0, 320.0, 160);
        ink.extend(arc(1, 0.0, 0.0, 120.0, 318.0, 362.0, 17));
        find_rings(&ink, &search())
    };
    // The same two strokes, the closing line drawn first.
    let line_first = {
        let mut ink = arc(0, 0.0, 0.0, 120.0, 318.0, 362.0, 17);
        ink.extend(arc(1, 0.0, 0.0, 120.0, 0.0, 320.0, 160));
        find_rings(&ink, &search())
    };

    assert_eq!(arc_first.len(), line_first.len());
    assert_eq!(arc_first[0].closed, line_first[0].closed);
    assert!((arc_first[0].fit.radius - line_first[0].fit.radius).abs() < 1.0);
}

#[test]
fn the_same_ink_yields_the_same_rings_twice() {
    let mut ink = ring(0, 13.0, -7.0, 88.0, 130);
    ink.extend(arc(1, 13.0, -7.0, 88.0, 30.0, 200.0, 40));
    ink.extend(ring(2, 400.0, 400.0, 50.0, 90));

    assert_eq!(find_rings(&ink, &search()), find_rings(&ink, &search()));
}

#[test]
fn a_scribble_is_not_a_ring() {
    let ink: Vec<Point> = (0..60)
        .map(|i| Point {
            x: (i as f32) * 4.0,
            y: ((i % 7) as f32) * 9.0,
            stroke_id: 0,
        })
        .collect();
    assert!(find_rings(&ink, &search()).is_empty());
}

#[test]
fn a_wider_join_tolerance_closes_a_narrower_gap() {
    let ink = arc(0, 0.0, 0.0, 100.0, 0.0, 340.0, 150);

    let strict = RingSearch {
        join: 4.0,
        ..search()
    };
    let loose = RingSearch {
        join: 80.0,
        ..search()
    };

    assert!(!find_rings(&ink, &strict)[0].closed);
    assert!(find_rings(&ink, &loose)[0].closed);
}

#[test]
fn ink_length_matches_the_circumference_of_a_full_ring() {
    let rings = find_rings(&ring(0, 0.0, 0.0, 100.0, 400), &search());
    let circumference = TAU * rings[0].fit.radius;
    assert!(
        (rings[0].ink_length / circumference - 1.0).abs() < 0.01,
        "drew {} of {}",
        rings[0].ink_length,
        circumference
    );
}

/// Going round twice lays down twice the ink. Not a substitute for the turning
/// number, but it is the one signal already available that notices.
#[test]
fn ink_length_doubles_when_the_ring_is_drawn_twice() {
    let rings = find_rings(&arc(0, 0.0, 0.0, 100.0, 0.0, 720.0, 400), &search());
    let circumference = TAU * rings[0].fit.radius;
    assert!(
        rings[0].ink_length > circumference * 1.9,
        "drew {} of {}",
        rings[0].ink_length,
        circumference
    );
}

#[test]
fn points_counts_every_member_stroke() {
    let mut ink = arc(0, 0.0, 0.0, 120.0, 0.0, 320.0, 160);
    ink.extend(arc(1, 0.0, 0.0, 120.0, 318.0, 362.0, 17));

    let rings = find_rings(&ink, &search());
    assert_eq!(rings[0].points, 177);
}

#[test]
fn a_plain_ring_is_simple() {
    let rings = find_rings(&ring(0, 0.0, 0.0, 120.0, 200), &search());
    assert!(rings[0].is_simple(0.08));
    assert!((rings[0].winding.turns - 1.0).abs() < 0.03);
}

/// Canon rule 2's prepared spell must stay legal. This is why `is_simple`
/// compares turning against coverage instead of against a full turn.
#[test]
fn a_ring_with_a_deliberate_gap_is_still_simple() {
    let rings = find_rings(&arc(0, 0.0, 0.0, 120.0, 0.0, 300.0, 150), &search());

    assert!(!rings[0].closed, "it is open, as intended");
    assert!(rings[0].is_simple(0.08), "but it is still one clean arc");
    assert!((rings[0].winding.turns - 0.83).abs() < 0.05);
}

/// The blind spot `coverage_is_fooled_by_a_double_loop` records. Turning sees
/// straight through it.
#[test]
fn winding_catches_the_double_loop_coverage_misses() {
    let rings = find_rings(&arc(0, 0.0, 0.0, 100.0, 0.0, 720.0, 400), &search());

    assert!(rings[0].coverage.spans_full_turn(20.0), "still fooled");
    assert!((rings[0].winding.turns - 2.0).abs() < 0.06);
    assert!(
        !rings[0].is_simple(0.08),
        "two turns over one turn of coverage"
    );
}

#[test]
fn winding_catches_a_figure_eight() {
    let ink: Vec<Point> = (0..200)
        .map(|i| {
            let t = (i as f32 / 200.0) * TAU;
            Point {
                x: 90.0 * t.sin(),
                y: 45.0 * (2.0 * t).sin(),
                stroke_id: 0,
            }
        })
        .collect();

    for candidate in find_rings(&ink, &search()) {
        assert!(
            !candidate.is_simple(0.08),
            "a figure-eight is not one clean loop"
        );
    }
}

/// A split seal inked in opposite directions is perfectly ordinary. The two
/// strokes must not cancel each other out.
#[test]
fn two_halves_drawn_opposite_ways_are_still_simple() {
    let mut ink = arc(0, 0.0, 0.0, 100.0, 0.0, 180.0, 90);
    ink.extend(
        arc(1, 0.0, 0.0, 100.0, 180.0, 360.0, 90)
            .into_iter()
            .rev()
            .collect::<Vec<_>>(),
    );

    let rings = find_rings(&ink, &search());
    assert_eq!(rings.len(), 1);
    assert!(rings[0].closed);
    assert!(rings[0].is_simple(0.08));
}

/// A short scratch standing in for a sigil, `n` points around `(cx, cy)`.
fn blob(id: u32, cx: f32, cy: f32, size: f32, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let a = (i as f32 / n as f32) * TAU;
            Point {
                x: cx + size * a.cos(),
                y: cy + size * (2.0 * a).sin(),
                stroke_id: id,
            }
        })
        .collect()
}

#[test]
fn a_sigil_inside_the_ring_is_listed_as_contents() {
    let mut ink = ring(0, 0.0, 0.0, 150.0, 200);
    ink.extend(blob(1, 0.0, 0.0, 30.0, 40));

    let rings = find_rings(&ink, &search());
    let held = rings[0].contents(&ink, 12.0);

    assert_eq!(held.inside, vec![1]);
    assert!(held.touching.is_empty());
    assert!(held.outside.is_empty());
    assert_eq!(held.points, 40);
}

/// The ring's own strokes are the ring, not its contents.
#[test]
fn a_rings_own_strokes_are_not_its_contents() {
    let mut ink = arc(0, 0.0, 0.0, 120.0, 0.0, 320.0, 160);
    ink.extend(arc(1, 0.0, 0.0, 120.0, 318.0, 362.0, 17));

    let rings = find_rings(&ink, &search());
    let held = rings[0].contents(&ink, 12.0);

    assert_eq!(rings[0].strokes, vec![0, 1]);
    assert!(held.inside.is_empty());
    assert!(held.touching.is_empty());
    assert_eq!(held.points, 0);
}

/// Rule 1's *connecting to* clause. A stroke crossing the ring counts toward
/// the spell even though it is not within it — and this is how rule 5 links
/// two glyphs with a line.
#[test]
fn a_stroke_crossing_the_ring_counts_as_touching() {
    let mut ink = ring(0, 0.0, 0.0, 100.0, 150);
    // A line from well inside to well outside.
    ink.extend((0..30).map(|i| Point {
        x: 20.0 + i as f32 * 6.0,
        y: 0.0,
        stroke_id: 1,
    }));

    let rings = find_rings(&ink, &search());
    let held = rings[0].contents(&ink, 12.0);

    assert_eq!(held.touching, vec![1]);
    assert!(held.inside.is_empty());
}

/// Rule 1 again: ink outside the ring does not count toward the spell.
#[test]
fn ink_outside_the_ring_does_not_count() {
    let mut ink = ring(0, 0.0, 0.0, 100.0, 150);
    ink.extend(blob(1, 500.0, 500.0, 20.0, 30));

    let rings = find_rings(&ink, &search());
    let held = rings[0].contents(&ink, 12.0);

    assert_eq!(held.outside, vec![1]);
    assert_eq!(held.points, 0, "outside ink contributes nothing");
    assert_eq!(held.extent, 0.0);
}

/// The number the wiki ties to a spell's intensity, recorded before the
/// recognizer throws scale away.
#[test]
fn extent_tracks_the_size_of_the_contents_against_the_ring() {
    let outer = ring(0, 0.0, 0.0, 200.0, 200);

    let small = {
        let mut ink = outer.clone();
        ink.extend(blob(1, 0.0, 0.0, 20.0, 40));
        let rings = find_rings(&ink, &search());
        rings[0].contents(&ink, 12.0).extent / rings[0].fit.radius
    };
    let large = {
        let mut ink = outer.clone();
        ink.extend(blob(1, 0.0, 0.0, 120.0, 40));
        let rings = find_rings(&ink, &search());
        rings[0].contents(&ink, 12.0).extent / rings[0].fit.radius
    };

    assert!(small < 0.2, "small sigil filled {small}");
    assert!(large > 0.5, "large sigil filled {large}");
}

/// Canon rule 4 — the inner ring of a nested seal is contents of the outer.
#[test]
fn a_nested_ring_shows_up_in_the_outer_rings_contents() {
    let mut ink = ring(0, 0.0, 0.0, 60.0, 120);
    ink.extend(ring(1, 0.0, 0.0, 180.0, 200));

    let rings = find_rings(&ink, &search());
    let outer = rings.iter().find(|r| r.fit.radius > 100.0).unwrap();
    let inner = rings.iter().find(|r| r.fit.radius < 100.0).unwrap();

    assert_eq!(outer.contents(&ink, 12.0).inside, inner.strokes);
    assert!(inner.contents(&ink, 12.0).inside.is_empty());
    assert_eq!(inner.contents(&ink, 12.0).outside, outer.strokes);
}

/// A ring drawn badly enough to fall below `min_quality`, but still closed.
fn rough_ring(id: u32, r: f32, wobble: f32, n: usize) -> Vec<Point> {
    let mut points: Vec<Point> = (0..n)
        .map(|i| {
            let a = (i as f32 / (n - 1) as f32) * TAU;
            let push = if i % 2 == 0 {
                1.0 + wobble
            } else {
                1.0 - wobble
            };
            Point {
                x: r * push * a.cos(),
                y: r * push * a.sin(),
                stroke_id: id,
            }
        })
        .collect();

    // The wobble alternates, so with an even `n` the last point lands on the
    // opposite side of the wobble from the first and the ends miss each other
    // by twice the amplitude. The drawing is meant to be rough, not open.
    if let Some(first) = points.first().copied() {
        *points.last_mut().unwrap() = first;
    }
    points
}

#[test]
fn a_neat_closed_ring_is_active() {
    let ink = ring(0, 0.0, 0.0, 120.0, 200);
    let rings = find_rings(&ink, &search());
    assert_eq!(
        rings[0].activation(&RingRules::default()),
        Activation::Active
    );
}

/// Canon rule 2. A gap is a prepared spell, and armed is not a failure state.
#[test]
fn a_ring_with_a_gap_is_armed_not_broken() {
    let ink = arc(0, 0.0, 0.0, 120.0, 0.0, 300.0, 150);
    let rings = find_rings(&ink, &search());
    assert_eq!(
        rings[0].activation(&RingRules::default()),
        Activation::Armed
    );
}

/// The wiki's second gate: closed, but not circular enough to hold.
#[test]
fn a_rough_closed_ring_is_only_fleeting() {
    let ink = rough_ring(0, 120.0, 0.09, 200);
    let rings = find_rings(&ink, &search());

    assert!(rings[0].closed, "it is finished, just badly");
    assert!(rings[0].fit.quality() < 0.95);
    assert_eq!(
        rings[0].activation(&RingRules::default()),
        Activation::Fleeting
    );
}

/// Neatness is not asked about until the ring is finished — an unfinished
/// seal is armed however roughly it was inked.
#[test]
fn structure_is_judged_before_craft() {
    let ink = rough_ring(0, 120.0, 0.09, 200)
        .into_iter()
        .take(160)
        .collect::<Vec<_>>();
    let rings = find_rings(&ink, &search());

    assert!(rings[0].fit.quality() < 0.95, "rough");
    assert!(!rings[0].closed, "and unfinished");
    assert_eq!(
        rings[0].activation(&RingRules::default()),
        Activation::Armed
    );
}

/// Closed and neat are meaningless questions about a shape that never went
/// round, so being a ring at all is asked first.
#[test]
fn a_figure_eight_is_malformed_whatever_else_it_is() {
    let ink: Vec<Point> = (0..200)
        .map(|i| {
            let t = (i as f32 / 200.0) * TAU;
            Point {
                x: 90.0 * t.sin(),
                y: 45.0 * (2.0 * t).sin(),
                stroke_id: 0,
            }
        })
        .collect();

    for candidate in find_rings(&ink, &search()) {
        assert_eq!(
            candidate.activation(&RingRules::default()),
            Activation::Malformed
        );
    }
}

#[test]
fn to_ring_carries_the_measurements_across() {
    let ink = ring(0, 40.0, -25.0, 130.0, 200);
    let rings = find_rings(&ink, &search());
    let ring = rings[0].to_ring();

    assert_eq!(ring.center(), rings[0].fit.center);
    assert_eq!(ring.radius(), rings[0].fit.radius);
    assert_eq!(ring.is_closed(), rings[0].closed);
    assert_eq!(ring.quality(), rings[0].fit.quality());
}

/// A ring drawn open compiles to an open `Ring`, not to nothing. Rule 2 needs
/// the unfired spell to exist.
#[test]
fn an_armed_candidate_still_compiles_to_a_ring() {
    let ink = arc(0, 0.0, 0.0, 120.0, 0.0, 300.0, 150);
    let ring = find_rings(&ink, &search())[0].to_ring();
    assert!(!ring.is_closed());
    assert!(ring.radius() > 100.0);
}

/// Rule 1's containment test has to agree with the assembly that produced it,
/// or ink judged inside here would fall outside once compiled.
#[test]
fn the_compiled_ring_contains_what_assembly_said_it_held() {
    let mut ink = ring(0, 0.0, 0.0, 150.0, 200);
    ink.extend(blob(1, 0.0, 0.0, 30.0, 40));

    let rings = find_rings(&ink, &search());
    let held = rings[0].contents(&ink, 12.0);
    let compiled = rings[0].to_ring();

    assert_eq!(held.inside, vec![1]);
    for p in ink.iter().filter(|p| held.inside.contains(&p.stroke_id)) {
        assert!(compiled.contains(*p, 0.0), "{p:?} fell outside the ring");
    }
}
