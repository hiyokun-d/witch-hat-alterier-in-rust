//! Tests for `recognizer`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use std::f32::consts::TAU;

use crate::stroke::MATCH_POINTS;

/// An ellipse, so the tests can tell uniform scaling from squashing.
fn oval(cx: f32, cy: f32, rx: f32, ry: f32, n: usize, id: u32) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let a = (i as f32 / (n - 1) as f32) * TAU;
            Point {
                x: cx + rx * a.cos(),
                y: cy + ry * a.sin(),
                stroke_id: id,
            }
        })
        .collect()
}

fn moved(points: &[Point], dx: f32, dy: f32) -> Vec<Point> {
    points
        .iter()
        .map(|p| Point {
            x: p.x + dx,
            y: p.y + dy,
            ..*p
        })
        .collect()
}

fn scaled(points: &[Point], by: f32) -> Vec<Point> {
    points
        .iter()
        .map(|p| Point {
            x: p.x * by,
            y: p.y * by,
            ..*p
        })
        .collect()
}

fn turned(points: &[Point], radians: f32) -> Vec<Point> {
    let (s, c) = radians.sin_cos();
    points
        .iter()
        .map(|p| Point {
            x: p.x * c - p.y * s,
            y: p.x * s + p.y * c,
            ..*p
        })
        .collect()
}

/// Largest distance between clouds compared point for point. Only meaningful
/// for clouds built from the same drawing, which is all these tests do.
fn drift(a: &Cloud, b: &Cloud) -> f32 {
    a.points
        .iter()
        .zip(b.points.iter())
        .map(|(p, q)| p.dist(q))
        .fold(0.0f32, f32::max)
}

#[test]
fn a_normalised_cloud_has_the_asked_for_point_count() {
    let c = normalize(&oval(0.0, 0.0, 50.0, 50.0, 71, 0), 32).unwrap();
    assert_eq!(c.points.len(), 32);
}

#[test]
fn a_normalised_cloud_is_centred_on_the_origin() {
    let c = normalize(&oval(400.0, -250.0, 60.0, 30.0, 91, 0), 32).unwrap();

    let n = c.points.len() as f32;
    let mean_x = c.points.iter().map(|p| p.x).sum::<f32>() / n;
    let mean_y = c.points.iter().map(|p| p.y).sum::<f32>() / n;

    assert!(mean_x.abs() < 1e-5, "mean x {mean_x}");
    assert!(mean_y.abs() < 1e-5, "mean y {mean_y}");
}

#[test]
fn a_normalised_cloud_has_unit_spread() {
    let c = normalize(&oval(0.0, 0.0, 130.0, 40.0, 81, 0), 32).unwrap();

    let n = c.points.len() as f32;
    let rms = (c.points.iter().map(|p| p.x * p.x + p.y * p.y).sum::<f32>() / n).sqrt();
    assert!((rms - 1.0).abs() < 1e-4, "rms was {rms}");
}

/// The reason for not using a bounding box. Turning a drawing must change its
/// angle and nothing else — §5 asks for ±15° of tolerance, and a scale that
/// shrank by a fifth over twelve degrees could not give it.
#[test]
fn turning_a_drawing_does_not_change_its_normalised_size() {
    let upright = square(0.0, 0.0, 100.0, 0);

    for degrees in [0.0f32, 7.0, 12.0, 23.0, 45.0] {
        let c = normalize(&turned(&upright, degrees.to_radians()), 32).unwrap();
        let n = c.points.len() as f32;
        let rms = (c.points.iter().map(|p| p.x * p.x + p.y * p.y).sum::<f32>() / n).sqrt();
        assert!((rms - 1.0).abs() < 1e-4, "{degrees}° gave rms {rms}");
    }
}

/// §2.2 — where a sigil is drawn does not change what it does.
#[test]
fn the_same_shape_drawn_elsewhere_normalises_the_same() {
    let ink = oval(0.0, 0.0, 60.0, 35.0, 81, 0);
    let here = normalize(&ink, 32).unwrap();
    let there = normalize(&moved(&ink, 900.0, -400.0), 32).unwrap();

    assert!(drift(&here, &there) < 1e-3);
    assert!((there.origin.x - here.origin.x - 900.0).abs() < 1.5);
}

/// §2.2 again — nor does how big it was drawn.
#[test]
fn the_same_shape_drawn_bigger_normalises_the_same() {
    let small = oval(0.0, 0.0, 20.0, 12.0, 81, 0);
    let large = scaled(&small, 7.0);

    let a = normalize(&small, 32).unwrap();
    let b = normalize(&large, 32).unwrap();

    assert!(drift(&a, &b) < 1e-4);
    assert!(
        (b.scale / a.scale - 7.0).abs() < 0.01,
        "scale must be recorded"
    );
}

/// Squashing each axis separately would make these the same drawing. Uniform
/// scaling keeps them apart, which is the point of scoring a sigil on shape.
#[test]
fn a_circle_and_an_ellipse_stay_different_shapes() {
    let round = normalize(&oval(0.0, 0.0, 50.0, 50.0, 81, 0), 32).unwrap();
    let squashed = normalize(&oval(0.0, 0.0, 50.0, 20.0, 81, 0), 32).unwrap();

    assert!(drift(&round, &squashed) > 0.1, "aspect ratio was flattened");
}

/// **Canon rule 6.** A reversed sign inverts its effect and a spell plus its
/// reversed twin cancel out, so orientation is meaning. Normalising rotation
/// away — which `$P` implementations often do — would delete that rule.
#[test]
fn rotation_is_not_normalised_away() {
    let upright = oval(0.0, 0.0, 60.0, 20.0, 81, 0);
    let onto_its_side = turned(&upright, std::f32::consts::FRAC_PI_2);

    let a = normalize(&upright, 32).unwrap();
    let b = normalize(&onto_its_side, 32).unwrap();

    assert!(
        drift(&a, &b) > 0.3,
        "a quarter turn must not vanish into the normalisation"
    );
}

/// §5 — hands are not protractors. A small turn has to stay a near match, and
/// it does so simply because a slightly turned cloud is a nearby cloud.
#[test]
fn a_small_rotation_stays_a_near_match() {
    let upright = oval(0.0, 0.0, 60.0, 25.0, 81, 0);
    let nudged = turned(&upright, 12.0f32.to_radians());

    let a = normalize(&upright, 32).unwrap();
    let b = normalize(&nudged, 32).unwrap();

    assert!(drift(&a, &b) < 0.5, "12° should barely move it");
}

/// A gesture is one drawing however many marks it took.
#[test]
fn several_strokes_normalise_as_one_gesture() {
    let mut ink = oval(0.0, 0.0, 50.0, 50.0, 41, 0);
    ink.extend((0..20).map(|i| Point {
        x: -20.0 + i as f32 * 2.0,
        y: 0.0,
        stroke_id: 1,
    }));

    let c = normalize(&ink, 32).unwrap();
    assert_eq!(c.points.len(), 32);
    assert!(c.points.iter().any(|p| p.stroke_id == 0));
    assert!(c.points.iter().any(|p| p.stroke_id == 1));
}

#[test]
fn the_origin_points_back_at_where_the_gesture_was() {
    let ink = oval(300.0, 120.0, 40.0, 40.0, 81, 0);
    let c = normalize(&ink, 32).unwrap();

    assert!((c.origin.x - 300.0).abs() < 1.5, "origin x {}", c.origin.x);
    assert!((c.origin.y - 120.0).abs() < 1.5, "origin y {}", c.origin.y);
}

#[test]
fn normalising_is_deterministic() {
    let ink = oval(13.0, -7.0, 44.0, 61.0, 77, 0);
    assert_eq!(normalize(&ink, 32), normalize(&ink, 32));
}

#[test]
fn a_dot_has_no_shape_to_normalise() {
    let stacked = vec![
        Point {
            x: 5.0,
            y: 5.0,
            stroke_id: 0
        };
        40
    ];
    assert!(normalize(&stacked, 32).is_none());
}

#[test]
fn too_little_ink_normalises_to_nothing() {
    assert!(normalize(&[], 32).is_none());
    assert!(normalize(&oval(0.0, 0.0, 10.0, 10.0, 20, 0), 1).is_none());
}

/// A straight line has no height, but it still has a shape.
#[test]
fn a_straight_line_still_normalises() {
    let line: Vec<Point> = (0..20)
        .map(|i| Point {
            x: i as f32 * 5.0,
            y: 0.0,
            stroke_id: 0,
        })
        .collect();

    // RMS radius of a segment of length L is L/√12 in the continuous case;
    // sampling it at n points instead of integrating adds √((n+1)/(n−1)).
    let c = normalize(&line, 32).unwrap();
    let expected = 95.0 / 12.0f32.sqrt() * (33.0f32 / 31.0).sqrt();
    assert!(
        (c.scale - expected).abs() < 0.05,
        "scale was {} not {expected}",
        c.scale
    );
    assert!(c.points.iter().all(|p| p.y.abs() < 1e-5));
}

// ── matching ────────────────────────────────────────────────────────────────

fn square(cx: f32, cy: f32, side: f32, id: u32) -> Vec<Point> {
    let h = side * 0.5;
    let corners = [(-h, -h), (h, -h), (h, h), (-h, h), (-h, -h)];
    let mut out = Vec::new();
    for pair in corners.windows(2) {
        for k in 0..20 {
            let t = k as f32 / 20.0;
            out.push(Point {
                x: cx + pair[0].0 + (pair[1].0 - pair[0].0) * t,
                y: cy + pair[0].1 + (pair[1].1 - pair[0].1) * t,
                stroke_id: id,
            });
        }
    }
    out
}

fn stripe(cx: f32, cy: f32, len: f32, id: u32) -> Vec<Point> {
    (0..40)
        .map(|i| Point {
            x: cx - len * 0.5 + len * i as f32 / 39.0,
            y: cy,
            stroke_id: id,
        })
        .collect()
}

fn cloud_of(points: &[Point]) -> Cloud {
    normalize(points, MATCH_POINTS).unwrap()
}

fn catalogue() -> Vec<Template> {
    vec![
        Template::record("ring", &oval(0.0, 0.0, 50.0, 50.0, 81, 0), MATCH_POINTS).unwrap(),
        Template::record("square", &square(0.0, 0.0, 100.0, 0), MATCH_POINTS).unwrap(),
        Template::record("stripe", &stripe(0.0, 0.0, 100.0, 0), MATCH_POINTS).unwrap(),
    ]
}

#[test]
fn a_cloud_is_no_distance_from_itself() {
    let c = cloud_of(&oval(0.0, 0.0, 40.0, 25.0, 71, 0));
    assert!(cloud_distance(&c, &c).unwrap() < 1e-5);
}

/// Greedy matching is not symmetric, which is exactly why `$P` takes the
/// nearer of both directions. The result of doing so is.
#[test]
fn distance_does_not_depend_on_which_cloud_comes_first() {
    let a = cloud_of(&oval(0.0, 0.0, 50.0, 30.0, 71, 0));
    let b = cloud_of(&square(0.0, 0.0, 100.0, 0));

    let there = cloud_distance(&a, &b).unwrap();
    let back = cloud_distance(&b, &a).unwrap();
    assert!((there - back).abs() < 1e-5, "{there} vs {back}");
}

#[test]
fn clouds_of_different_sizes_cannot_be_compared() {
    let a = normalize(&oval(0.0, 0.0, 40.0, 40.0, 71, 0), 32).unwrap();
    let b = normalize(&oval(0.0, 0.0, 40.0, 40.0, 71, 0), 16).unwrap();
    assert!(cloud_distance(&a, &b).is_none());
}

#[test]
fn a_ring_is_recognised_as_a_ring() {
    let drawn = cloud_of(&oval(0.0, 0.0, 63.0, 63.0, 55, 0));
    let found = classify(&drawn, &catalogue()).unwrap();
    assert_eq!(catalogue()[found.index].name, "ring");
}

#[test]
fn a_line_is_recognised_as_a_line() {
    let drawn = cloud_of(&stripe(0.0, 0.0, 240.0, 0));
    let found = classify(&drawn, &catalogue()).unwrap();
    assert_eq!(catalogue()[found.index].name, "stripe");
}

/// §2.2 — the same rune drawn elsewhere, at another size, is the same rune.
#[test]
fn position_and_size_do_not_change_the_answer() {
    let far_and_small = cloud_of(&square(800.0, -600.0, 19.0, 0));
    let found = classify(&far_and_small, &catalogue()).unwrap();
    assert_eq!(catalogue()[found.index].name, "square");
}

/// §5 — hands are not protractors.
#[test]
fn a_gesture_off_by_twelve_degrees_still_matches() {
    let drawn = cloud_of(&turned(&square(0.0, 0.0, 100.0, 0), 12.0f32.to_radians()));
    let found = classify(&drawn, &catalogue()).unwrap();
    assert_eq!(catalogue()[found.index].name, "square");
}

/// Canon rule 6 needs a turned shape to *be* a different drawing, so the
/// distance to its upright self must be real and not zero.
#[test]
fn a_quarter_turn_is_a_measurable_distance() {
    let upright = cloud_of(&stripe(0.0, 0.0, 100.0, 0));
    let sideways = cloud_of(&turned(
        &stripe(0.0, 0.0, 100.0, 0),
        std::f32::consts::FRAC_PI_2,
    ));

    let apart = cloud_distance(&upright, &sideways).unwrap();
    assert!(apart > 0.2, "a quarter turn measured only {apart}");
}

/// $P's whole point, and §5's property: order is not an input.
#[test]
fn drawing_a_gesture_backwards_matches_the_same() {
    let forward = square(0.0, 0.0, 100.0, 0);
    let backward: Vec<Point> = forward.iter().rev().copied().collect();

    let a = classify(&cloud_of(&forward), &catalogue()).unwrap();
    let b = classify(&cloud_of(&backward), &catalogue()).unwrap();

    assert_eq!(a.index, b.index);
    assert!((a.distance - b.distance).abs() < 0.05);
}

#[test]
fn rank_returns_every_comparable_template_nearest_first() {
    let drawn = cloud_of(&oval(0.0, 0.0, 60.0, 60.0, 61, 0));
    let ranked = rank(&drawn, &catalogue());

    assert_eq!(ranked.len(), 3);
    for pair in ranked.windows(2) {
        assert!(pair[0].distance <= pair[1].distance, "not sorted");
    }
    assert_eq!(catalogue()[ranked[0].index].name, "ring");
}

#[test]
fn rank_skips_templates_it_cannot_compare() {
    let mut catalogue = catalogue();
    catalogue.push(Template::record("coarse", &stripe(0.0, 0.0, 50.0, 0), 8).unwrap());

    let ranked = rank(&cloud_of(&stripe(0.0, 0.0, 90.0, 0)), &catalogue);
    assert_eq!(ranked.len(), 3, "the 8-point template is not a candidate");
}

#[test]
fn classify_of_an_empty_catalogue_is_none() {
    assert!(classify(&cloud_of(&stripe(0.0, 0.0, 40.0, 0)), &[]).is_none());
}

/// Two identical templates must always resolve the same way, or the same ink
/// would name different runes on different runs (§4.3).
#[test]
fn a_tie_always_goes_to_the_earlier_template() {
    let shape = oval(0.0, 0.0, 40.0, 40.0, 61, 0);
    let twins = vec![
        Template::record("first", &shape, MATCH_POINTS).unwrap(),
        Template::record("second", &shape, MATCH_POINTS).unwrap(),
    ];

    let drawn = cloud_of(&shape);
    assert_eq!(classify(&drawn, &twins).unwrap().index, 0);
    assert_eq!(rank(&drawn, &twins)[0].index, 0);
}

#[test]
fn matching_is_deterministic() {
    let drawn = cloud_of(&square(11.0, -3.0, 77.0, 0));
    assert_eq!(rank(&drawn, &catalogue()), rank(&drawn, &catalogue()));
}

#[test]
fn score_runs_from_one_downwards() {
    let exact = Match {
        index: 0,
        distance: 0.0,
    };
    let poor = Match {
        index: 0,
        distance: 1.0,
    };
    assert!((exact.score() - 1.0).abs() < 1e-6);
    assert!(poor.score() < exact.score());
    assert!(poor.score() > 0.0);
}

/// A drawing of the wrong shape must be measurably further away than the right
/// one — a ranking is only useful if the gap means something.
#[test]
fn the_right_template_wins_by_a_clear_margin() {
    let drawn = cloud_of(&stripe(0.0, 0.0, 120.0, 0));
    let ranked = rank(&drawn, &catalogue());

    assert!(
        ranked[1].distance > ranked[0].distance * 2.0,
        "best {} runner-up {}",
        ranked[0].distance,
        ranked[1].distance
    );
}
