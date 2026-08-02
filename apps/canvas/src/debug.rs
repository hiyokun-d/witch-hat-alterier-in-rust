//! THIS IS CLAUDE WORKS AND I JUST ASKED FOR IT!
//! On-screen debug overlay. Scaffolding, and deliberately quarantined.
//!
//! Nothing outside this file knows the overlay exists. To delete it: remove
//! this file, the `mod debug;` line, and the `add_plugins` line in `main.rs`.
//! Nothing else changes — the overlay only ever reads.
//!
//! It goes away once ink rendering shows the same information implicitly.

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::WindowFocused;

use crate::{InkPad, cursor_world};

/// Height of one text row, at the font size below.
const LINE_GAP: f32 = 22.0;
/// Gap between the overlay and the window edges.
const MARGIN: f32 = 14.0;

/// Draws the live readouts. Add it, remove it, nothing else reacts.
pub struct DebugOverlayPlugin;

/// Raw event counts, so a dead shortcut can be blamed on the right layer.
///
/// `Window::focused` starts `true` and only changes when a focus event
/// arrives, so a window that never got focus still reports `true`. Counting
/// events instead removes the guesswork.
#[derive(Resource, Default)]
struct InputProbe {
    key_events: u32,
    last_key: String,
    focus_events: u32,
    focused_by_event: bool,
}

impl Plugin for DebugOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputProbe>()
            .add_systems(Update, probe_input);

        app.add_systems(Startup, spawn_overlay).add_systems(
            Update,
            // After capture, so the numbers describe this frame rather than
            // trailing it by one.
            (
                place_overlay,
                update_cursor_line,
                update_ink_line,
                update_input_line,
            )
                .after(crate::capture_stroke),
        );
    }
}

/// Pins a readout to the bottom-left corner, `lift` rows above the bottom.
#[derive(Component)]
struct OverlayLine {
    lift: f32,
}

/// Marks the cursor position line.
#[derive(Component)]
struct CursorLine;

/// Marks the ink statistics block.
#[derive(Component)]
struct InkLine;

/// Marks the keyboard/focus line — is the app even receiving key events?
#[derive(Component)]
struct InputLine;

fn spawn_overlay(mut commands: Commands) {
    let font = TextFont {
        font_size: FontSize::Px(16.0),
        ..default()
    };

    // `Anchor::BOTTOM_LEFT` makes the transform the block's bottom-left corner
    // instead of its center, so text grows right and up, away from the edge.
    // `place_overlay` supplies the position — resizing the window keeps it put.
    commands.spawn((
        Text2d::new("cursor: —"),
        font.clone(),
        TextColor(Color::srgb(0.45, 0.85, 0.75)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        // Two rows up: the ink block below it is two lines tall.
        OverlayLine { lift: 2.0 },
        CursorLine,
    ));

    commands.spawn((
        Text2d::new("ink: —"),
        font.clone(),
        TextColor(Color::srgb(0.85, 0.70, 0.45)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 0.0 },
        InkLine,
    ));

    commands.spawn((
        Text2d::new("input: —"),
        font,
        TextColor(Color::srgb(0.70, 0.55, 0.90)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 4.0 },
        InputLine,
    ));
}

/// Keeps the overlay in the bottom-left corner as the window resizes.
///
/// Runs every frame rather than on a `Changed<Window>` filter — three entities,
/// and it removes any question of whether the first frame gets placed.
fn place_overlay(window: Single<&Window>, mut lines: Query<(&OverlayLine, &mut Transform)>) {
    let corner = Vec2::new(window.width(), window.height()) * -0.5;

    for (line, mut transform) in &mut lines {
        transform.translation.x = corner.x + MARGIN;
        transform.translation.y = corner.y + MARGIN + line.lift * LINE_GAP;
    }
}

/// The mouse reports pixels from the top-left with Y growing downward; the
/// world puts its origin at the center with Y growing upward. Showing both is
/// the whole point of this line.
fn update_cursor_line(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut line: Single<&mut Text2d, With<CursorLine>>,
) {
    let (camera, camera_transform) = *camera;
    line.0 = cursor_text(&window, camera, camera_transform);
}

fn update_ink_line(
    pad: Res<InkPad>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut line: Single<&mut Text2d, With<InkLine>>,
) {
    line.0 = ink_text(&pad, &mouse);
}

/// Taps the raw event streams before `ButtonInput` ever sees them.
fn probe_input(
    mut keyboard: MessageReader<KeyboardInput>,
    mut focus: MessageReader<WindowFocused>,
    mut probe: ResMut<InputProbe>,
) {
    for event in keyboard.read() {
        probe.key_events += 1;
        probe.last_key = format!("{:?}/{:?}", event.key_code, event.state);
    }

    for event in focus.read() {
        probe.focus_events += 1;
        probe.focused_by_event = event.focused;
    }
}

/// Answers the only question that matters when a shortcut "does nothing":
/// is this window focused, and are keys arriving at all?
fn update_input_line(
    window: Single<&Window>,
    keys: Res<ButtonInput<KeyCode>>,
    probe: Res<InputProbe>,
    mut line: Single<&mut Text2d, With<InputLine>>,
) {
    let held: Vec<String> = keys.get_pressed().map(|key| format!("{key:?}")).collect();

    // Two focus readings on purpose: the field, and what events actually said.
    // They disagreeing is itself the answer.
    let focus_by_event = if probe.focus_events == 0 {
        "never".to_string()
    } else {
        format!("{}", probe.focused_by_event)
    };

    line.0 = format!(
        "focus field {}  events {focus_by_event}   keyev {}  last {}\nheld [{}]",
        window.focused,
        probe.key_events,
        if probe.last_key.is_empty() {
            "—"
        } else {
            &probe.last_key
        },
        held.join(" "),
    );
}

fn cursor_text(window: &Window, camera: &Camera, camera_transform: &GlobalTransform) -> String {
    // Absent whenever the pointer is outside the window — normal, not an error.
    let Some(cursor) = window.cursor_position() else {
        return "cursor: off-window".to_string();
    };

    let Some(world) = cursor_world(window, camera, camera_transform) else {
        return "cursor: conversion failed".to_string();
    };

    format!(
        "screen {:.0}, {:.0}   →   world {:.0}, {:.0}",
        cursor.x, cursor.y, world.x, world.y
    )
}

fn ink_text(pad: &InkPad, mouse: &ButtonInput<MouseButton>) -> String {
    // `stroke_id` names the stroke in progress and only advances on release, so
    // it doubles as the count of strokes already finished.
    let finished = pad.stroke_id;
    let in_current = pad
        .points
        .iter()
        .filter(|point| point.stroke_id == pad.stroke_id)
        .count();

    let last = match pad.points.last() {
        Some(point) => format!("{:.0}, {:.0}", point.x, point.y),
        None => "—".to_string(),
    };

    format!(
        "pen {}   strokes {finished}   points {}\nin stroke {in_current}   undone {}   last {last}",
        if mouse.pressed(MouseButton::Left) {
            "DOWN"
        } else {
            "up"
        },
        pad.points.len(),
        pad.undone.len(),
    )
}
