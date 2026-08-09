//! Tests for `stroke`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use std::f32::consts::TAU;

fn p(x: f32, y: f32) -> Point {
    Point { x, y, stroke_id: 0 }
}

/// A straight run of `n` points from `(0,0)` to `(len, 0)`.
fn line(len: f32, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| p(len * i as f32 / (n - 1) as f32, 0.0))
        .collect()
}

/// A ring of `n` points, ends meeting.
fn ring(r: f32, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let a = (i as f32 / (n - 1) as f32) * TAU;
            p(r * a.cos(), r * a.sin())
        })
        .collect()
}

fn spacings(stroke: &[Point]) -> Vec<f32> {
    stroke
        .windows(2)
        .map(|pair| pair[0].dist(&pair[1]))
        .collect()
}

#[test]
fn path_length_of_a_straight_line_is_its_length() {
    assert!((path_length(&line(100.0, 5)) - 100.0).abs() < 1e-3);
}

/// The distance the pen travelled, not the distance between the ends.
#[test]
fn path_length_follows_the_path_not_the_shortcut() {
    let there_and_back = vec![p(0.0, 0.0), p(50.0, 0.0), p(0.0, 0.0)];
    assert!((path_length(&there_and_back) - 100.0).abs() < 1e-3);
}

#[test]
fn path_length_of_a_ring_is_its_circumference() {
    let drawn = path_length(&ring(100.0, 400));
    assert!((drawn / (TAU * 100.0) - 1.0).abs() < 0.01, "drew {drawn}");
}

#[test]
fn path_length_of_fewer_than_two_points_is_zero() {
    assert_eq!(path_length(&[]), 0.0);
    assert_eq!(path_length(&[p(3.0, 4.0)]), 0.0);
}

#[test]
fn resample_returns_exactly_the_count_asked_for() {
    for n in [2, 3, 16, 32, 64, 257] {
        let out = resample(&ring(80.0, 37), n).unwrap();
        assert_eq!(out.len(), n, "asked for {n}");
    }
}

#[test]
fn resample_keeps_both_ends_where_they_were_drawn() {
    let stroke = ring(120.0, 53);
    let out = resample(&stroke, 32).unwrap();

    assert_eq!(*out.first().unwrap(), *stroke.first().unwrap());
    assert_eq!(*out.last().unwrap(), *stroke.last().unwrap());
}

#[test]
fn resample_spaces_a_straight_line_evenly() {
    let out = resample(&line(100.0, 3), 11).unwrap();
    for gap in spacings(&out) {
        assert!((gap - 10.0).abs() < 0.01, "gap was {gap}");
    }
}

/// The reason this exists. A stroke that bunches at one end must come out even.
#[test]
fn resample_evens_out_a_stroke_that_bunched() {
    // Twenty points crammed into the first tenth, five spread over the rest —
    // a hand that hesitated at the start and then swept away.
    let mut bunched: Vec<Point> = (0..20).map(|i| p(i as f32 * 0.5, 0.0)).collect();
    bunched.extend((1..=5).map(|i| p(10.0 + i as f32 * 18.0, 0.0)));

    let before = spacings(&bunched);
    let widest_before = before.iter().cloned().fold(0.0f32, f32::max);
    let tightest_before = before.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(
        widest_before / tightest_before > 30.0,
        "fixture is not uneven"
    );

    let after = spacings(&resample(&bunched, 40).unwrap());
    let widest = after.iter().cloned().fold(0.0f32, f32::max);
    let tightest = after.iter().cloned().fold(f32::INFINITY, f32::min);
    assert!(
        widest / tightest < 1.05,
        "still uneven: {tightest} to {widest}"
    );
}

/// Every point produced lies on a segment the pen actually drew — resampling
/// smooths the sampling, never the shape.
#[test]
fn resample_never_leaves_the_drawn_path() {
    let stroke = ring(100.0, 90);
    for point in resample(&stroke, 64).unwrap() {
        let radius = (point.x * point.x + point.y * point.y).sqrt();
        // Chords cut inside the circle by at most the sagitta of one segment.
        assert!(
            (radius - 100.0).abs() < 1.0,
            "landed at radius {radius}, off the ink"
        );
    }
}

#[test]
fn resampling_an_even_stroke_barely_changes_it() {
    let stroke = ring(90.0, 33);
    let out = resample(&stroke, 33).unwrap();

    for (before, after) in stroke.iter().zip(out.iter()) {
        assert!(before.dist(after) < 1.5, "{before:?} moved to {after:?}");
    }
}

#[test]
fn resample_preserves_stroke_id() {
    let stroke: Vec<Point> = line(60.0, 9)
        .into_iter()
        .map(|q| Point { stroke_id: 7, ..q })
        .collect();

    assert!(
        resample(&stroke, 20)
            .unwrap()
            .iter()
            .all(|q| q.stroke_id == 7)
    );
}

#[test]
fn resample_is_deterministic() {
    let stroke = ring(77.0, 41);
    assert_eq!(resample(&stroke, 32), resample(&stroke, 32));
}

#[test]
fn resample_of_too_few_points_is_none() {
    assert!(resample(&[], 32).is_none());
    assert!(resample(&[p(1.0, 1.0)], 32).is_none());
}

#[test]
fn resample_of_too_few_samples_is_none() {
    let stroke = line(50.0, 10);
    assert!(resample(&stroke, 0).is_none());
    assert!(resample(&stroke, 1).is_none());
    assert!(resample(&stroke, 2).is_some());
}

/// A dot has no path to walk along.
#[test]
fn resample_of_stacked_points_is_none() {
    let stacked = vec![p(4.0, 4.0); 30];
    assert!(resample(&stacked, 32).is_none());
}

/// Duplicate points inside a stroke are a real capture artefact and must not
/// divide by the zero-length segment between them.
#[test]
fn resample_survives_duplicate_points_mid_stroke() {
    let mut stroke = vec![p(0.0, 0.0); 5];
    stroke.extend(vec![p(50.0, 0.0); 5]);
    stroke.extend(vec![p(50.0, 50.0); 5]);

    let out = resample(&stroke, 16).unwrap();
    assert_eq!(out.len(), 16);
    assert!(out.iter().all(|q| q.x.is_finite() && q.y.is_finite()));
}

/// Resampling to more points than were captured is normal — a short stroke
/// still has to become a `MATCH_POINTS` cloud.
#[test]
fn resample_can_add_points() {
    let out = resample(&line(100.0, 3), MATCH_POINTS).unwrap();
    assert_eq!(out.len(), MATCH_POINTS);
    for gap in spacings(&out) {
        assert!((gap - 100.0 / (MATCH_POINTS - 1) as f32).abs() < 0.01);
    }
}
