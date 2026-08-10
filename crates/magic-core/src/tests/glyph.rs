//! Tests for `glyph`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use core::f32::consts::{FRAC_PI_2, PI};

fn sign(kind: &str) -> Sign {
    Sign {
        kind: kind.into(),
        placement: 0.0,
        orientation: 0.0,
        size: 1.0,
        reversed: false,
    }
}

fn at(x: f32, y: f32) -> Point {
    Point { x, y, stroke_id: 0 }
}

fn ring(quality: f32) -> Ring {
    Ring::new(at(0.0, 0.0), 40.0, true, quality)
}

#[test]
fn reversing_twice_returns_original() {
    let s = sign("enlarge");
    assert_eq!(s.reverse().reverse(), s);
}

#[test]
fn reversed_sign_differs_from_normal() {
    let s = sign("enlarge");
    assert_ne!(s.reverse(), s);
}

#[test]
fn reverse_preserves_kind_placement_and_orientation() {
    let s = Sign {
        placement: 0.5,
        orientation: 1.25,
        ..sign("region")
    };
    let r = s.reverse();
    assert_eq!(r.kind, s.kind);
    assert_eq!(r.placement, s.placement);
    assert_eq!(r.orientation, s.orientation);
}

#[test]
fn reverse_of_reversed_sign_is_normal() {
    let s = Sign {
        reversed: true,
        ..sign("column")
    };
    assert!(!s.reverse().reversed);
}

#[test]
fn sign_pointing_away_from_center_is_outward() {
    let s = Sign {
        placement: FRAC_PI_2,
        orientation: FRAC_PI_2,
        ..sign("region")
    };
    assert!(s.points_outward(0.1));
    assert!(!s.points_inward(0.1));
}

#[test]
fn sign_pointing_back_at_center_is_inward() {
    let s = Sign {
        placement: FRAC_PI_2,
        orientation: FRAC_PI_2 + PI,
        ..sign("region")
    };
    assert!(s.points_inward(0.1));
    assert!(!s.points_outward(0.1));
}

#[test]
fn sign_tangent_to_ring_is_neither_in_nor_out() {
    let s = Sign {
        placement: 0.0,
        orientation: FRAC_PI_2,
        ..sign("region")
    };
    assert!(!s.points_inward(0.1));
    assert!(!s.points_outward(0.1));
}

#[test]
fn ring_quality_clamps_above_one() {
    assert_eq!(ring(47.0).quality(), 1.0);
}

#[test]
fn ring_quality_clamps_below_zero() {
    assert_eq!(ring(-3.0).quality(), 0.0);
}

#[test]
fn ring_quality_in_range_is_unchanged() {
    assert_eq!(ring(0.62).quality(), 0.62);
}

#[test]
fn ring_keeps_the_rest_of_its_fields() {
    let r = ring(0.5);
    assert_eq!(r.radius(), 40.0);
    assert!(r.is_closed());
    assert_eq!(r.center().x, 0.0);
}

#[test]
fn ring_contains_a_point_inside_it() {
    assert!(ring(1.0).contains(at(10.0, 0.0), 0.0));
}

/// Canon rule 1: a sign touching the ring counts toward the spell, so
/// containment cannot be a plain point-in-circle test.
#[test]
fn ring_contains_a_point_touching_it_from_outside() {
    assert!(ring(1.0).contains(at(42.0, 0.0), 3.0));
}

#[test]
fn ring_excludes_a_point_beyond_tolerance() {
    assert!(!ring(1.0).contains(at(50.0, 0.0), 3.0));
}

#[test]
fn new_glyph_starts_with_no_signs() {
    let g = Glyph::new(GlyphId(0), Some("fire".into()), ring(1.0));
    assert!(g.signs.is_empty());
}

#[test]
fn new_glyph_starts_unnested_and_unlinked() {
    let g = Glyph::new(GlyphId(0), Some("fire".into()), ring(1.0));
    assert_eq!(g.parent, None);
    assert!(g.linked.is_empty());
}

/// CLAUDE.md §2.1: billow, repetition, and vision drive a spell from the
/// centre with no sigil at all.
#[test]
fn glyph_with_no_sigil_constructs() {
    let g = Glyph::new(GlyphId(7), None, ring(1.0));
    assert_eq!(g.sigil, None);
}

#[test]
fn glyph_ids_with_the_same_number_are_equal() {
    assert_eq!(GlyphId(3), GlyphId(3));
    assert_ne!(GlyphId(3), GlyphId(4));
}
