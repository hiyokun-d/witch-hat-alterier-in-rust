//! Tests for `circle`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use std::f32::consts::TAU;

fn close(a: f32, b: f32, eps: f32) -> bool {
    (a - b).abs() <= eps
}

/// `n` points evenly spaced around a full circle. Exclusive of the closing
/// point, so no sample is counted twice.
fn circle(cx: f32, cy: f32, r: f32, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = (i as f32 / n as f32) * TAU;
            Point {
                x: cx + r * t.cos(),
                y: cy + r * t.sin(),
                stroke_id: 0,
            }
        })
        .collect()
}

/// `n` points along an arc, inclusive of both ends.
fn arc(cx: f32, cy: f32, r: f32, from_deg: f32, to_deg: f32, n: usize) -> Vec<Point> {
    (0..n)
        .map(|i| {
            let t = i as f32 / (n - 1) as f32;
            let a = (from_deg + (to_deg - from_deg) * t).to_radians();
            Point {
                x: cx + r * a.cos(),
                y: cy + r * a.sin(),
                stroke_id: 0,
            }
        })
        .collect()
}

#[test]
fn fit_of_a_perfect_circle_recovers_its_centre_and_radius() {
    let f = fit(&circle(100.0, -40.0, 55.0, 64)).unwrap();
    assert!(close(f.center.x, 100.0, 0.01), "x was {}", f.center.x);
    assert!(close(f.center.y, -40.0, 0.01), "y was {}", f.center.y);
    assert!(close(f.radius, 55.0, 0.01), "r was {}", f.radius);
}

#[test]
fn fit_is_translation_invariant() {
    let here = fit(&circle(0.0, 0.0, 30.0, 48)).unwrap();
    let there = fit(&circle(700.0, -250.0, 30.0, 48)).unwrap();

    assert!(close(here.radius, there.radius, 0.01));
    assert!(close(there.center.x - here.center.x, 700.0, 0.02));
    assert!(close(there.center.y - here.center.y, -250.0, 0.02));
}

#[test]
fn fit_is_rotation_invariant() {
    let points = arc(20.0, 10.0, 45.0, 0.0, 200.0, 40);
    let turned: Vec<Point> = points
        .iter()
        .map(|p| {
            let (s, c) = (0.7f32).sin_cos();
            Point {
                x: p.x * c - p.y * s,
                y: p.x * s + p.y * c,
                stroke_id: p.stroke_id,
            }
        })
        .collect();

    let a = fit(&points).unwrap();
    let b = fit(&turned).unwrap();
    assert!(close(a.radius, b.radius, 0.05));
    assert!(close(a.rms, b.rms, 0.05));
}

/// Doubling the drawing doubles the radius and the raw error, and leaves
/// the neatness score untouched. Canon rule 8 rests on exactly this.
#[test]
fn fit_is_scale_equivariant() {
    let small = arc(5.0, 5.0, 25.0, 0.0, 300.0, 50);
    let big: Vec<Point> = small
        .iter()
        .map(|p| Point {
            x: p.x * 4.0,
            y: p.y * 4.0,
            stroke_id: p.stroke_id,
        })
        .collect();

    let a = fit(&small).unwrap();
    let b = fit(&big).unwrap();

    assert!(close(b.radius, a.radius * 4.0, 0.05));
    assert!(close(b.quality(), a.quality(), 1e-3));
}

#[test]
fn fit_of_fewer_than_three_points_is_none() {
    let points = circle(0.0, 0.0, 10.0, 8);
    assert!(fit(&[]).is_none());
    assert!(fit(&points[..1]).is_none());
    assert!(fit(&points[..2]).is_none());
    assert!(fit(&points[..3]).is_some());
}

#[test]
fn fit_of_collinear_points_is_none() {
    let straight: Vec<Point> = (0..20)
        .map(|i| Point {
            x: i as f32 * 7.0,
            y: 3.0 + i as f32 * 7.0,
            stroke_id: 0,
        })
        .collect();
    assert!(fit(&straight).is_none());
}

#[test]
fn fit_of_identical_points_is_none() {
    let stacked = vec![
        Point {
            x: 12.0,
            y: -3.0,
            stroke_id: 0
        };
        30
    ];
    assert!(fit(&stacked).is_none());
}

#[test]
fn fit_ignores_stroke_id() {
    let plain = circle(0.0, 0.0, 20.0, 32);
    let mixed: Vec<Point> = plain
        .iter()
        .enumerate()
        .map(|(i, p)| Point {
            stroke_id: i as u32,
            ..*p
        })
        .collect();

    let a = fit(&plain).unwrap();
    let b = fit(&mixed).unwrap();
    assert!(close(a.radius, b.radius, 1e-4));
    assert!(close(a.center.x, b.center.x, 1e-4));
    assert!(close(a.center.y, b.center.y, 1e-4));
}

/// Canon rule 3 — half a ring on each of two objects. Both halves have to
/// land on the same circle or the two can never be recognised as one seal.
/// This is the test Kåsa fails.
#[test]
fn fit_of_a_half_arc_recovers_the_full_circle() {
    let f = fit(&arc(0.0, 0.0, 100.0, 0.0, 180.0, 60)).unwrap();
    assert!(close(f.radius, 100.0, 1.0), "r was {}", f.radius);
    assert!(close(f.center.x, 0.0, 1.0), "x was {}", f.center.x);
    assert!(close(f.center.y, 0.0, 1.0), "y was {}", f.center.y);
}

/// Canon rule 2 — a gapped ring is a prepared spell, and the gap can be
/// wide. Kåsa is off by ten percent or worse here.
#[test]
fn fit_of_a_quarter_arc_is_within_one_percent() {
    let f = fit(&arc(0.0, 0.0, 100.0, 0.0, 90.0, 60)).unwrap();
    assert!(close(f.radius, 100.0, 1.0), "r was {}", f.radius);
}

/// The test that catches a missing centring step. Without it the `z²`
/// terms overflow `f32`'s precision and the answer is noise.
#[test]
fn fit_of_a_huge_offset_circle_stays_accurate() {
    let f = fit(&circle(5000.0, 5000.0, 200.0, 64)).unwrap();
    assert!(close(f.radius, 200.0, 0.5), "r was {}", f.radius);
    assert!(close(f.center.x, 5000.0, 0.5), "x was {}", f.center.x);
}

#[test]
fn rms_of_a_perfect_circle_is_zero() {
    let f = fit(&circle(3.0, 8.0, 40.0, 72)).unwrap();
    assert!(f.rms < 0.01, "rms was {}", f.rms);
}

/// Monotonicity, not a magic number — more wobble must score worse.
#[test]
fn rms_grows_with_wobble() {
    let wobbled = |amount: f32| -> Vec<Point> {
        circle(0.0, 0.0, 60.0, 64)
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let push = if i % 2 == 0 { amount } else { -amount };
                let len = (p.x * p.x + p.y * p.y).sqrt();
                Point {
                    x: p.x * (len + push) / len,
                    y: p.y * (len + push) / len,
                    stroke_id: 0,
                }
            })
            .collect()
    };

    let calm = fit(&wobbled(0.5)).unwrap();
    let rough = fit(&wobbled(4.0)).unwrap();
    assert!(rough.rms > calm.rms, "{} !> {}", rough.rms, calm.rms);
    assert!(rough.quality() < calm.quality());
}

#[test]
fn quality_is_one_for_a_perfect_circle() {
    let f = fit(&circle(0.0, 0.0, 80.0, 90)).unwrap();
    assert!(close(f.quality(), 1.0, 1e-3), "quality {}", f.quality());
}

/// Normalisation pins the mean squared radius to one, and `m_z` must equal
/// `m_uu + m_vv` by construction. If either fails, `z` is being built from
/// raw coordinates instead of centred ones.
#[test]
fn normalised_moments_have_unit_mean_radius() {
    let m = moments(&arc(400.0, 900.0, 130.0, 0.0, 250.0, 40)).unwrap();
    assert!(close(m.m_z, 1.0, 1e-4), "m_z was {}", m.m_z);
    assert!(close(m.m_z, m.m_uu + m.m_vv, 1e-6));
}

/// One point yanked well off the ring. The trimmed fit should shrug it off.
fn circle_with_a_hook(hook: f32) -> Vec<Point> {
    let mut points = circle(0.0, 0.0, 100.0, 60);
    points[0].x += hook;
    points[0].y += hook;
    points
}

#[test]
fn trimming_a_clean_circle_changes_nothing() {
    let points = circle(10.0, 20.0, 70.0, 64);
    let plain = fit(&points).unwrap();
    let trimmed = fit_trimmed(&points).unwrap();

    assert_eq!(trimmed.trimmed, 0);
    assert!(close(plain.radius, trimmed.radius, 1e-3));
    assert!(close(plain.center.x, trimmed.center.x, 1e-3));
}

#[test]
fn trimming_pulls_the_centre_back_off_an_outlier() {
    let points = circle_with_a_hook(120.0);
    let plain = fit(&points).unwrap();
    let trimmed = fit_trimmed(&points).unwrap();

    assert!(trimmed.trimmed > 0, "nothing was trimmed");

    let plain_miss = plain.center.dist(&Point {
        x: 0.0,
        y: 0.0,
        stroke_id: 0,
    });
    let trimmed_miss = trimmed.center.dist(&Point {
        x: 0.0,
        y: 0.0,
        stroke_id: 0,
    });
    assert!(
        trimmed_miss < plain_miss,
        "trimmed centre {trimmed_miss} was no better than {plain_miss}"
    );
    assert!(
        close(trimmed.radius, 100.0, 1.0),
        "r was {}",
        trimmed.radius
    );
}

/// The whole point of separating the two passes. The outlier is excluded
/// from the fit and still counted in the score, because the mess is what
/// canon rule 8 grades.
#[test]
fn a_trimmed_point_still_counts_against_the_score() {
    let clean = fit_trimmed(&circle(0.0, 0.0, 100.0, 60)).unwrap();
    let hooked = fit_trimmed(&circle_with_a_hook(120.0)).unwrap();

    assert!(hooked.trimmed > 0);
    assert!(
        hooked.rms > clean.rms,
        "hooked rms {} did not exceed clean {}",
        hooked.rms,
        clean.rms
    );
    assert!(hooked.quality() < clean.quality());
}

/// A scribble is not a circle with strays on it. Carving an arc out of one
/// would report a confident fit for nonsense.
#[test]
fn trimming_refuses_when_it_would_drop_too_much() {
    // Half on a ring, half scattered far off it.
    let mut points = arc(0.0, 0.0, 100.0, 0.0, 180.0, 30);
    for i in 0..30 {
        points.push(Point {
            x: (i as f32) * 3.0 - 45.0,
            y: 400.0 + (i % 5) as f32 * 30.0,
            stroke_id: 0,
        });
    }

    let trimmed = fit_trimmed(&points).unwrap();
    let cap = (points.len() as f32 * MAX_TRIM_FRACTION) as usize;
    assert!(
        trimmed.trimmed <= cap,
        "dropped {} of {}",
        trimmed.trimmed,
        points.len()
    );
}

#[test]
fn trimming_never_leaves_fewer_than_three_points() {
    let points = vec![
        Point {
            x: 0.0,
            y: 0.0,
            stroke_id: 0,
        },
        Point {
            x: 10.0,
            y: 0.0,
            stroke_id: 0,
        },
        Point {
            x: 5.0,
            y: 9.0,
            stroke_id: 0,
        },
    ];
    let f = fit_trimmed(&points).unwrap();
    assert_eq!(f.trimmed, 0);
}

#[test]
fn a_full_ring_has_almost_no_gap() {
    let points = circle(0.0, 0.0, 100.0, 120);
    let f = fit(&points).unwrap();
    let c = coverage(&points, &f).unwrap();

    // 120 samples on a 628px circumference is a step of ~5px.
    assert!(c.gap_length < 8.0, "gap was {}px", c.gap_length);
    assert!(c.spanned > 0.98, "spanned {}", c.spanned);
    assert!(c.spans_full_turn(20.0));
}

/// Canon rule 2 — the prepared spell. A deliberate hole must read as open.
#[test]
fn a_ring_with_a_deliberate_gap_reads_as_open() {
    // 320° of a 100px ring leaves a 40° hole, about 70px of arc.
    let points = arc(0.0, 0.0, 100.0, 0.0, 320.0, 100);
    let f = fit(&points).unwrap();
    let c = coverage(&points, &f).unwrap();

    assert!(
        close(c.gap.to_degrees(), 40.0, 3.0),
        "gap {}°",
        c.gap.to_degrees()
    );
    assert!(close(c.gap_length, 69.8, 6.0), "gap {}px", c.gap_length);
    assert!(!c.spans_full_turn(20.0), "a 70px hole is not closed");
}

/// Closure is judged in pixels, so the same angular gap on a bigger ring is
/// a bigger hole. Rule 3 joins two halves by physical contact.
#[test]
fn the_same_angular_gap_is_a_wider_hole_on_a_bigger_ring() {
    let small = arc(0.0, 0.0, 50.0, 0.0, 350.0, 80);
    let large = arc(0.0, 0.0, 400.0, 0.0, 350.0, 80);

    let cs = coverage(&small, &fit(&small).unwrap()).unwrap();
    let cl = coverage(&large, &fit(&large).unwrap()).unwrap();

    assert!(close(cs.gap, cl.gap, 0.05), "angles should match");
    assert!(cl.gap_length > cs.gap_length * 7.0);
    assert!(cs.spans_full_turn(20.0), "a 9px hole is closed");
    assert!(!cl.spans_full_turn(20.0), "a 70px hole is not");
}

#[test]
fn the_gap_heading_points_at_the_hole() {
    // Ink from 100° round to 20°, so the hole is centred on 60°.
    let points = arc(0.0, 0.0, 100.0, 100.0, 380.0, 90);
    let f = fit(&points).unwrap();
    let c = coverage(&points, &f).unwrap();

    assert!(
        close(c.gap_heading.to_degrees(), 60.0, 6.0),
        "heading {}°",
        c.gap_heading.to_degrees()
    );
}

#[test]
fn the_gap_heading_stays_inside_one_turn() {
    for from in [0.0, 90.0, 170.0, 250.0, 359.0] {
        let points = arc(0.0, 0.0, 60.0, from, from + 300.0, 70);
        let f = fit(&points).unwrap();
        let c = coverage(&points, &f).unwrap();
        assert!(
            c.gap_heading > -std::f32::consts::PI && c.gap_heading <= std::f32::consts::PI,
            "heading {} out of range for start {from}",
            c.gap_heading
        );
    }
}

#[test]
fn coverage_of_fewer_than_two_points_is_none() {
    let f = fit(&circle(0.0, 0.0, 10.0, 8)).unwrap();
    assert!(coverage(&[], &f).is_none());
    assert!(coverage(&circle(0.0, 0.0, 10.0, 8)[..1], &f).is_none());
}

/// Recorded, not fixed. Going round twice passes through every angle, so
/// coverage sees a closed ring. Only the turning number can tell them
/// apart, and that is M4.3.
#[test]
fn coverage_is_fooled_by_a_double_loop() {
    let points = arc(0.0, 0.0, 100.0, 0.0, 720.0, 200);
    let f = fit(&points).unwrap();
    let c = coverage(&points, &f).unwrap();
    assert!(c.spans_full_turn(20.0), "documents a known blind spot");
}

#[test]
fn the_same_stroke_measures_identically_twice() {
    let points = arc(13.0, -7.0, 88.0, 20.0, 300.0, 77);

    let a = fit_trimmed(&points).unwrap();
    let b = fit_trimmed(&points).unwrap();
    assert_eq!(a, b);

    let ca = coverage(&points, &a).unwrap();
    let cb = coverage(&points, &b).unwrap();
    assert_eq!(ca, cb);
}

/// A figure-eight is not a ring, but it fits a circle perfectly happily.
/// Rejecting it needs the turning number, which arrives in M4.3 — this
/// test exists to record that the fit alone cannot do it.
#[test]
fn fit_accepts_a_figure_eight_because_rejecting_it_is_not_its_job() {
    let eight: Vec<Point> = (0..80)
        .map(|i| {
            let t = (i as f32 / 80.0) * TAU;
            Point {
                x: 50.0 * t.sin(),
                y: 50.0 * (2.0 * t).sin(),
                stroke_id: 0,
            }
        })
        .collect();
    assert!(fit(&eight).is_some());
}
