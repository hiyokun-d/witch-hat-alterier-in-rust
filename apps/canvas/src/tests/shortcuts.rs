//! Tests for `shortcuts`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use magic_core::Point;

/// A pad holding `strokes` strokes of two points each.
fn pad_with(strokes: u32) -> InkPad {
    let points = (0..strokes)
        .flat_map(|stroke_id| {
            (0..2).map(move |i| Point {
                x: i as f32,
                y: stroke_id as f32,
                stroke_id,
            })
        })
        .collect();

    InkPad {
        points,
        stroke_id: strokes,
        undone: Vec::new(),
    }
}

#[test]
fn clear_empties_the_pad() {
    let mut pad = pad_with(3);
    clear(&mut pad);
    assert!(pad.points.is_empty());
}

#[test]
fn one_redo_after_clear_restores_the_whole_drawing() {
    let mut pad = pad_with(3);
    let before = pad.points.clone();

    clear(&mut pad);
    redo(&mut pad);

    assert_eq!(pad.points, before);
}

/// A clear is an edit, so anything further forward in the history is stale.
/// Otherwise the second redo would splice a stroke back in under an id the
/// restored pad is already using.
#[test]
fn clear_drops_redo_entries_from_before_it() {
    let mut pad = pad_with(3);
    undo(&mut pad);
    assert_eq!(pad.undone.len(), 1);

    clear(&mut pad);
    assert_eq!(pad.undone.len(), 1, "only the cleared pad should remain");
}

#[test]
fn stroke_ids_stay_unique_across_a_clear_and_restore() {
    let mut pad = pad_with(3);
    clear(&mut pad);
    redo(&mut pad);

    let mut ids: Vec<u32> = pad.points.iter().map(|p| p.stroke_id).collect();
    ids.dedup();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids, sorted, "ids must stay ascending and unrepeated");
    assert!(pad.points.iter().all(|p| p.stroke_id < pad.stroke_id));
}

#[test]
fn clear_all_empties_the_pad_and_the_history() {
    let mut pad = pad_with(3);
    undo(&mut pad);
    clear_all(&mut pad);

    assert!(pad.points.is_empty());
    assert!(pad.undone.is_empty());
    assert_eq!(pad.stroke_id, 0);
}

#[test]
fn nothing_survives_clear_all() {
    let mut pad = pad_with(3);
    clear_all(&mut pad);
    redo(&mut pad);
    undo(&mut pad);
    assert!(pad.points.is_empty());
}

#[test]
fn clear_all_on_an_empty_pad_is_harmless() {
    let mut pad = pad_with(0);
    clear_all(&mut pad);
    assert!(pad.points.is_empty());
    assert_eq!(pad.stroke_id, 0);
}

/// Three taps inside the window fire once, on the third.
#[test]
fn a_full_run_of_taps_fires_on_the_last_one() {
    let mut taps = TapCounter::default();
    assert!(!taps.tap(0.0));
    assert!(!taps.tap(0.3));
    assert!(taps.tap(0.6));
}

#[test]
fn a_completed_run_resets_rather_than_firing_again() {
    let mut taps = TapCounter::default();
    taps.tap(0.0);
    taps.tap(0.1);
    assert!(taps.tap(0.2));
    // The fourth press starts a fresh run, it does not re-fire.
    assert!(!taps.tap(0.3));
    assert_eq!(taps.count(), 1);
}

/// A lapsed run restarts from the late press rather than failing, so a slow
/// first two presses cannot poison the next attempt.
#[test]
fn a_tap_after_the_window_starts_a_new_run() {
    let mut taps = TapCounter::default();
    taps.tap(0.0);
    taps.tap(0.5);
    assert!(!taps.tap(9.0), "too late to complete the first run");
    assert_eq!(taps.count(), 1);

    assert!(!taps.tap(9.1));
    assert!(taps.tap(9.2), "the restarted run completes normally");
}

#[test]
fn taps_exactly_on_the_window_edge_still_count() {
    let mut taps = TapCounter::default();
    taps.tap(0.0);
    taps.tap(1.0);
    assert!(taps.tap(2.0), "2.0s is within a 2.0s window, not past it");
}

#[test]
fn expire_drops_a_stale_run() {
    let mut taps = TapCounter::default();
    taps.tap(0.0);
    assert_eq!(taps.count(), 1);

    taps.expire(0.5);
    assert_eq!(taps.count(), 1, "still inside the window");

    taps.expire(5.0);
    assert_eq!(taps.count(), 0);
}

#[test]
fn remaining_is_none_when_no_run_is_open() {
    let mut taps = TapCounter::default();
    assert_eq!(taps.remaining(0.0), None);

    // Run opened at 1.0s with a 2.0s window, so half a second later there
    // is still 1.5s of it left.
    taps.tap(1.0);
    assert_eq!(taps.remaining(1.5), Some(1.5));
    assert_eq!(taps.remaining(3.0), Some(0.0), "never goes negative");
}

#[test]
fn clearing_an_empty_pad_banks_nothing() {
    let mut pad = pad_with(0);
    clear(&mut pad);
    assert!(pad.points.is_empty());
    assert!(
        pad.undone.is_empty(),
        "an empty entry would make the next undo look broken"
    );
}
