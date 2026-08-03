//! What each keyboard shortcut does to the [`InkPad`].
//!
//! Plain functions on `&mut InkPad`, not systems — the `keyboard_shortcut`
//! system in `main.rs` decides which keys mean what, and calls in here for the
//! effect. Adding a shortcut means one fn here and one arm there.

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
