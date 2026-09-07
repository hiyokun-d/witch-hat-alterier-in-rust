//! Pick-then-place: the drag, the preview, and the commit.
//!
//! An armed shape behaves the way a shape tool behaves anywhere else. Press on
//! the paper and the press point is the centre; drag and the distance out is
//! the radius; release and it lands. A press with no drag places at whatever
//! radius the panel is set to, so the common case is still one click.
//!
//! The preview and the commit call the same functions in [`stamp`], so what
//! you are shown and what you get cannot drift apart. That is the only reason
//! a preview is worth having.

// Same reason as `debug.rs`: a Bevy system declares its dependencies as
// parameters, so a handler that reads the mouse, the keyboard, the sheet, the
// pointer, the tools, the drag state, the pad and the lesson has eight before
// it has done anything. Clippy's limit targets muddled abstractions; this is a
// list of things the drag genuinely touches.
#![allow(clippy::too_many_arguments)]

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use super::{Mode, Pointer, RADIUS_RANGE, Shape, ToolState, stamp};
use crate::{InkPad, PaperShape};

/// Below this, a drag was a click and the panel's radius is used instead.
///
/// Generous on purpose: a click always moves the mouse a pixel or two, and a
/// ring of radius four is not what anybody meant.
const DRAG_THRESHOLD: f32 = 12.0;

/// The shape being dragged out, if any.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Placing {
    /// Where the press landed — the centre of what will be placed.
    pub from: Option<Vec2>,
    /// Where the pointer is now.
    pub to: Vec2,
}

impl Placing {
    /// Radius the drag is asking for, or `None` if it has not moved far enough
    /// to be asking for anything.
    fn dragged_radius(&self) -> Option<f32> {
        let from = self.from?;
        let reach = from.distance(self.to);
        (reach >= DRAG_THRESHOLD).then_some(reach)
    }

    /// Which way the drag went, for aiming an arc's hole.
    fn heading(&self) -> Option<f32> {
        let from = self.from?;
        let out = self.to - from;
        (out.length() >= DRAG_THRESHOLD).then(|| out.y.atan2(out.x))
    }
}

/// What the current drag would place: centre, radius, and which way it faces.
///
/// Shared by the preview and the commit so the two cannot disagree.
fn pending(placing: &Placing, tools: &ToolState) -> Option<(Vec2, f32, f32)> {
    let center = placing.from?;
    let radius = placing
        .dragged_radius()
        .unwrap_or(tools.stamp_radius)
        .clamp(RADIUS_RANGE.0, RADIUS_RANGE.1);
    // Undragged, an arc's hole sits at the top, where it is easiest to see and
    // easiest to close.
    let facing = placing.heading().unwrap_or(std::f32::consts::FRAC_PI_2);
    Some((center, radius, facing))
}

/// What a press means: the window and the sheet it has to land on.
#[derive(SystemParam)]
pub struct Sheet<'w, 's> {
    window: Single<'w, 's, &'static Window>,
    shape: Res<'w, PaperShape>,
}

impl Sheet<'_, '_> {
    /// Whether the sheet would accept ink at `at`. A placed shape is ink like
    /// any other, so it obeys the same edge the pen does.
    fn accepts(&self, at: Vec2) -> bool {
        self.shape.accepts(&self.window, at)
    }
}

/// Turns a press-drag-release on the paper into ink.
pub fn drag(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    sheet: Sheet,
    pointer: Res<Pointer>,
    mut tools: ResMut<ToolState>,
    mut placing: ResMut<Placing>,
    mut pad: ResMut<InkPad>,
    mut tutor: ResMut<super::tutor::Tutor>,
    reading: Res<crate::reading::Reading>,
) {
    let Mode::Place(shape) = tools.mode else {
        placing.from = None;
        return;
    };

    // Escape mid-drag abandons it without placing anything. Checked before the
    // release, so letting go after Escape does not still commit.
    if keys.just_pressed(KeyCode::Escape) {
        placing.from = None;
        return;
    }

    let Some(at) = pointer.at else {
        return;
    };
    if pointer.over_ui {
        // Started or wandered onto the panel. Placing under the buttons would
        // put ink where it cannot be seen.
        return;
    }

    if mouse.just_pressed(MouseButton::Left) {
        if sheet.accepts(at) {
            placing.from = Some(at);
            placing.to = at;
        }
        return;
    }

    if mouse.pressed(MouseButton::Left) {
        placing.to = at;
        return;
    }

    if mouse.just_released(MouseButton::Left) {
        // The eraser acts on press-point alone: there is no size to drag out,
        // and asking for one would make rubbing out a dot into a gesture.
        if shape == Shape::Guide {
            // One lesson, moved rather than multiplied.
            let from = placing.from.unwrap_or(at);
            placing.from = None;
            tutor.at = from;
            tutor.radius = tools.stamp_radius;
            tutor.placed = true;
            return;
        }
        if shape == Shape::Erase {
            let from = placing.from.unwrap_or(at);
            placing.from = None;
            erase_at(&mut pad, from);
            return;
        }

        let Some((center, radius, facing)) = pending(&placing, &tools) else {
            return;
        };
        placing.from = None;

        stamp::place(
            &mut pad,
            stamp::draw_shape(
                shape,
                &reading.shapes,
                center,
                radius,
                facing,
                tools.stamp_gap,
            ),
        );

        // A drag says what size you wanted, so the next click agrees with the
        // last drag rather than snapping back to a number you did not choose.
        tools.stamp_radius = radius;
    }
}

/// Shows exactly what a release would place.
pub fn preview(
    pointer: Res<Pointer>,
    tools: Res<ToolState>,
    placing: Res<Placing>,
    reading: Res<crate::reading::Reading>,
    mut gizmos: Gizmos<super::guides::GuideGizmos>,
) {
    let Mode::Place(shape) = tools.mode else {
        return;
    };

    // Mid-drag: the real thing, ghosted. Otherwise a hint at the cursor of
    // what a click would drop there.
    let (center, radius, facing) = match pending(&placing, &tools) {
        Some(pending) => pending,
        None => match pointer.at {
            Some(at) if !pointer.over_ui => (at, tools.stamp_radius, std::f32::consts::FRAC_PI_2),
            _ => return,
        },
    };

    let ghost = if placing.from.is_some() {
        GHOST_LIVE
    } else {
        GHOST_IDLE
    };

    // Built by the same functions that will build the ink, so the preview is
    // the thing itself rather than a drawing of it.
    for stroke in stamp::draw_shape(
        shape,
        &reading.shapes,
        center,
        radius,
        facing,
        tools.stamp_gap,
    ) {
        for pair in stroke.windows(2) {
            gizmos.line_2d(pair[0], pair[1], ghost);
        }
    }

    // The drag itself: centre, and the line whose length is the radius.
    if let Some(from) = placing.from {
        gizmos.line_2d(from, placing.to, GHOST_REACH);
        gizmos.circle_2d(Isometry2d::from_translation(from), 3.0, GHOST_REACH);
    }
}

/// What a click would drop, following the cursor.
const GHOST_IDLE: Color = Color::srgba(0.35, 0.45, 0.62, 0.40);
/// What a release would place, while the drag is live.
const GHOST_LIVE: Color = Color::srgba(0.20, 0.40, 0.72, 0.85);
/// The drag's own centre-to-cursor line.
const GHOST_REACH: Color = Color::srgba(0.92, 0.58, 0.12, 0.70);

/// Colour of a seal the panel is offering but has not placed.
const GHOST: Color = Color::srgba(0.42, 0.34, 0.62, 0.60);

/// Shows the seal `preset` would lay down, while the pointer is over its button.
///
/// A button whose whole job is "put a complicated drawing on the paper" is the
/// one button you cannot guess the result of, and the hint line can only say
/// its name. Drawing the actual strokes costs nothing — [`stamp::seal`] is the
/// same function the button calls, so the preview cannot drift from the result.
pub fn preset_preview(
    pointer: Res<Pointer>,
    tools: Res<ToolState>,
    reading: Res<crate::reading::Reading>,
    mut gizmos: Gizmos,
) {
    let Some(over) = pointer.over_tool else {
        return;
    };
    // Both buttons, because `preset >` is the one you are on while *choosing*,
    // and choosing without seeing is the thing this exists to fix.
    let offered = super::bar::tool_named("preset") == Some(over)
        || super::bar::tool_named("preset >") == Some(over);
    if !offered {
        return;
    }

    let Some(preset) = crate::sim::PRESETS.get(tools.preset) else {
        return;
    };
    for stroke in stamp::seal(
        &reading.shapes,
        preset.sigil,
        Vec2::ZERO,
        tools.stamp_radius,
        preset.signs,
        preset.open,
        preset.inward,
    ) {
        for pair in stroke.windows(2) {
            gizmos.line_2d(pair[0], pair[1], GHOST);
        }
    }
}

/// How near a click must land to count as touching a stroke.
const RUB: f32 = 14.0;

/// Rubs out the stroke nearest `at`, if anything is near enough.
///
/// Nearest rather than every stroke in range: erasing more than you pointed at
/// is the thing that makes an eraser frightening to use.
///
/// The redo stack is dropped, because rubbing out is an edit and anything
/// further forward in the history is now about ink that no longer exists —
/// the same contract `stamp::place` follows.
pub fn erase_at(pad: &mut InkPad, at: Vec2) {
    let mut nearest: Option<(u32, f32)> = None;
    for run in pad.points.chunk_by(|a, b| a.stroke_id == b.stroke_id) {
        let Some(id) = run.first().map(|p| p.stroke_id) else {
            continue;
        };
        let close = run
            .iter()
            .map(|p| at.distance(Vec2::new(p.x, p.y)))
            .fold(f32::INFINITY, f32::min);
        if close <= RUB && nearest.is_none_or(|(_, best)| close < best) {
            nearest = Some((id, close));
        }
    }

    if let Some((id, _)) = nearest {
        pad.points.retain(|p| p.stroke_id != id);
        pad.undone.clear();
    }
}
