//! Tests for `main`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

/// Bevy's default window: 1280×720, so half-extents are 640×360 and the
/// disc's radius is 360 − `PAPER_MARGIN`.
fn window() -> Window {
    Window::default()
}

fn disc_radius() -> f32 {
    360.0 - PAPER_MARGIN
}

#[test]
fn disc_radius_follows_the_shorter_axis() {
    let extent = PaperShape::Disc.extent(&window());
    assert_eq!(extent, Vec2::splat(disc_radius()));
}

#[test]
fn full_sheet_covers_the_whole_window() {
    assert_eq!(PaperShape::Full.extent(&window()), Vec2::new(640.0, 360.0));
}

#[test]
fn disc_takes_ink_at_its_centre() {
    assert!(PaperShape::Disc.accepts(&window(), Vec2::ZERO));
}

#[test]
fn disc_refuses_ink_past_its_edge() {
    let just_out = Vec2::new(disc_radius() + 1.0, 0.0);
    assert!(!PaperShape::Disc.accepts(&window(), just_out));
}

/// The corner of the window is inside the *window* but outside the *disc*.
/// This is the case the whole feature exists for.
#[test]
fn disc_refuses_ink_in_the_window_corners() {
    let corner = Vec2::new(600.0, 340.0);
    assert!(!PaperShape::Disc.accepts(&window(), corner));
    assert!(PaperShape::Full.accepts(&window(), corner));
}

#[test]
fn disc_edge_itself_still_takes_ink() {
    let on_edge = Vec2::new(0.0, disc_radius());
    assert!(PaperShape::Disc.accepts(&window(), on_edge));
}

#[test]
fn full_sheet_refuses_ink_outside_the_window() {
    assert!(!PaperShape::Full.accepts(&window(), Vec2::new(641.0, 0.0)));
    assert!(!PaperShape::Full.accepts(&window(), Vec2::new(0.0, -361.0)));
}

#[test]
fn toggling_twice_returns_the_same_sheet() {
    let start = PaperShape::default();
    assert_eq!(start.toggled().toggled(), start);
    assert_ne!(start.toggled(), start);
}

/// A window narrower than twice the margin would ask for a negative radius
/// and turn the mesh inside out.
#[test]
fn a_tiny_window_still_leaves_a_pixel_of_paper() {
    let mut tiny = Window::default();
    tiny.resolution.set(10.0, 10.0);
    assert!(PaperShape::Disc.extent(&tiny).x >= 1.0);
}
