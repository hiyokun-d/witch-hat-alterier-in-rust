//! What each keyboard shortcut does, and the machinery a shortcut needs.
//!
//! Plain functions on `&mut InkPad`, not systems — the `keyboard_shortcut`
//! system in `main.rs` decides which keys mean what, and calls in here for the
//! effect. Adding a shortcut means one fn here and one arm there.
//!
//! [`TapCounter`] is the exception to "operates on `InkPad`": it is shortcut
//! *mechanics* rather than an effect, and it lives here so it can be tested
//! without a running app.

use bevy::prelude::*;

use crate::InkPad;

/// ⌘ on macOS, Ctrl elsewhere. Accepting both keeps muscle memory working
/// whichever machine this runs on.
pub fn command_held(keys: &ButtonInput<KeyCode>) -> bool {
    keys.any_pressed([
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
    ])
}

/// Lifts the most recent stroke off the pad and onto the undo stack.
pub fn undo(pad: &mut InkPad) {
    let Some(last_id) = pad.points.last().map(|point| point.stroke_id) else {
        return;
    };

    // Points are appended in draw order and ids only climb, so the first point
    // carrying `last_id` is where that stroke begins.
    let Some(start) = pad
        .points
        .iter()
        .position(|point| point.stroke_id == last_id)
    else {
        return;
    };

    let stroke = pad.points.split_off(start);
    pad.undone.push(stroke);
    // That id is free again, so the next stroke drawn reuses it.
    pad.stroke_id = last_id;
}

/// Puts back whichever stroke undo lifted off most recently.
pub fn redo(pad: &mut InkPad) {
    let Some(stroke) = pad.undone.pop() else {
        return;
    };

    if let Some(last) = stroke.last() {
        pad.stroke_id = last.stroke_id + 1;
    }
    pad.points.extend(stroke);
}

/// Empties the pad, recoverably. One undo brings the whole drawing back.
///
/// The pad goes onto `undone` as a **single** entry rather than one per stroke,
/// so ⌘Z after a clear restores everything at once. Undoing a clear stroke by
/// stroke would be technically tidier and feel broken.
///
/// Older redo entries are dropped, exactly as starting a new stroke drops them
/// in `capture_stroke`. A clear is an edit, and an edit invalidates anything
/// further forward in the history — without this, a second redo would splice a
/// previously undone stroke back in under an id the restored pad is already
/// using.
///
/// `stroke_id` is deliberately left climbing. Resetting it to zero would let a
/// stroke drawn after the clear collide with one the undo stack still holds.
pub fn clear(pad: &mut InkPad) {
    // Nothing to recover, and an empty entry on the stack would make ⌘Z look
    // broken — it would consume a press and change nothing on screen.
    if pad.points.is_empty() {
        return;
    }

    // `mem::take` rather than `clear`: `undone` has to *own* these points, and
    // a field cannot be moved out of a struct held by `&mut`.
    let cleared = std::mem::take(&mut pad.points);
    pad.undone.clear();
    pad.undone.push(cleared);
}

/// Wipes the pad back to the state it had at startup. **Not recoverable.**
///
/// The deliberate counterpart to [`clear`]: that one banks the drawing so ⌘Z
/// brings it back, this one takes the history with it. Because nothing survives,
/// `stroke_id` can safely restart at zero — the collision [`clear`] guards
/// against needs an undo stack still holding the old ids, and there isn't one.
///
/// Unconditional, unlike [`clear`]. Wiping an already-empty pad is a no-op that
/// costs nothing, and there is no undo entry to be careful about creating.
pub fn clear_all(pad: &mut InkPad) {
    pad.points.clear();
    pad.undone.clear();
    pad.stroke_id = 0;
}

/// Counts repeated presses of one key inside a time window.
///
/// A tap run rather than a single press for actions worth being sure about —
/// three F presses in two seconds cannot be hit by accident the way one can.
/// The counter knows nothing about which key it is watching; `main.rs` decides
/// that, the same way it decides every other binding.
#[derive(Resource, Debug, Clone, Copy)]
pub struct TapCounter {
    /// How many presses complete a run.
    needed: u32,
    /// Seconds from the first press in which the run must finish.
    window: f32,
    /// Presses banked so far. Zero means no run is open.
    count: u32,
    /// When the open run started, in seconds since app start.
    first_at: f32,
}

impl Default for TapCounter {
    fn default() -> Self {
        TapCounter {
            needed: 3,
            window: 2.0,
            count: 0,
            first_at: 0.0,
        }
    }
}

impl TapCounter {
    /// Banks a press. Returns `true` exactly once, on the press that completes
    /// a run, and resets so the next press starts a fresh one.
    ///
    /// `now` is seconds since app start rather than a wall clock — the shell may
    /// read a clock, but nothing here should depend on what time of day it is.
    pub fn tap(&mut self, now: f32) -> bool {
        // A lapsed run does not fail, it *restarts*. Otherwise a slow first two
        // presses would poison the next two and the third press would feel dead.
        if self.count == 0 || now - self.first_at > self.window {
            self.count = 1;
            self.first_at = now;
        } else {
            self.count += 1;
        }

        if self.count >= self.needed {
            self.count = 0;
            return true;
        }

        false
    }

    /// Drops an open run whose window has passed.
    ///
    /// Only cosmetic: [`TapCounter::tap`] already restarts a lapsed run. This
    /// exists so a progress readout shows the run expiring as it happens rather
    /// than staying lit until the next press.
    pub fn expire(&mut self, now: f32) {
        if self.count > 0 && now - self.first_at > self.window {
            self.count = 0;
        }
    }

    /// Presses banked in the open run.
    pub fn count(&self) -> u32 {
        self.count
    }

    /// How many presses complete a run.
    pub fn needed(&self) -> u32 {
        self.needed
    }

    /// Seconds left before the open run lapses, or `None` when none is open.
    pub fn remaining(&self, now: f32) -> Option<f32> {
        (self.count > 0).then(|| (self.window - (now - self.first_at)).max(0.0))
    }
}

#[cfg(test)]
mod tests {
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
}
