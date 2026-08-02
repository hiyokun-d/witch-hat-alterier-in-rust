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
