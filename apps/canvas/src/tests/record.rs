//! Tests for `record`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use crate::Point;

fn pad_with(strokes: &[Vec<(f32, f32)>]) -> InkPad {
    let mut pad = InkPad::default();
    for (id, stroke) in strokes.iter().enumerate() {
        for (x, y) in stroke {
            pad.points.push(Point {
                x: *x,
                y: *y,
                stroke_id: id as u32,
            });
        }
        pad.stroke_id = id as u32 + 1;
    }
    pad
}

#[test]
fn an_empty_pad_records_nothing() {
    assert!(record(&InkPad::default()).is_err());
}

#[test]
fn a_pad_holding_only_a_ring_records_nothing() {
    // A ring is the activator, not a rune. The recogniser is never asked to
    // identify one — `circle.rs` does that with geometry.
    let ring: Vec<(f32, f32)> = (0..200)
        .map(|i| {
            let a = i as f32 / 199.0 * std::f32::consts::TAU;
            (120.0 * a.cos(), 120.0 * a.sin())
        })
        .collect();
    assert!(record(&pad_with(&[ring])).is_err());
}

#[test]
fn a_single_point_stroke_is_not_a_gesture() {
    assert!(record(&pad_with(&[vec![(0.0, 0.0)]])).is_err());
}
