//! THIS IS CLAUDE WORKS AND I JUST ASKED FOR IT!
//! On-screen debug overlay. Scaffolding, and deliberately quarantined.
//!
//! Nothing outside this file knows the overlay exists. To delete it: remove
//! this file, the `mod debug;` line, and the `add_plugins` line in `main.rs`.
//! Nothing else changes — the overlay only ever reads.
//!
//! It goes away once ink rendering shows the same information implicitly.

use bevy::gizmos::config::GizmoConfigStore;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::WindowFocused;

use crate::{Credit, InkPad, Paper, cursor_world};

/// Height of one text row, at the font size below.
const LINE_GAP: f32 = 22.0;
/// Gap between the overlay and the window edges.
const MARGIN: f32 = 14.0;

/// Half-size of the marks drawn at stroke ends and anchor points.
const TICK: f32 = 7.0;

/// Guides: window edge, centre axes, the safe area inside the margins.
const GUIDE: Color = Color::srgba(0.78, 0.72, 0.62, 0.28);
/// Where a stroke begins.
const START_MARK: Color = Color::srgb(0.10, 0.48, 0.34);
/// Where a stroke ends.
const END_MARK: Color = Color::srgb(0.66, 0.22, 0.18);
/// The credit's anchor point — its true corner, read from its transform.
const ANCHOR_MARK: Color = Color::srgb(0.90, 0.70, 0.30);

/// Draws the live readouts. Add it, remove it, nothing else reacts.
pub struct DebugOverlayPlugin;

/// How many frames the frame-time average covers. Long enough that the number
/// stops flickering, short enough that a stall still shows up.
const FRAME_WINDOW: usize = 60;

/// How many strokes the stroke table lists, newest first.
const STROKES_SHOWN: usize = 6;

/// Whether the overlay is drawn at all. Toggled with F1.
///
/// The overlay covers a real fraction of the pad now, and judging ink you
/// cannot fully see is worse than having no readouts.
#[derive(Resource)]
struct OverlayVisible(bool);

impl Default for OverlayVisible {
    fn default() -> Self {
        OverlayVisible(true)
    }
}

/// A ring buffer of recent frame times.
///
/// Averaged rather than shown raw: a single frame's delta jitters far too much
/// to read, and the number you actually want is "is this holding 60".
#[derive(Resource, Default)]
struct FrameTimes {
    samples: Vec<f32>,
    next: usize,
}

impl FrameTimes {
    fn push(&mut self, seconds: f32) {
        if self.samples.len() < FRAME_WINDOW {
            self.samples.push(seconds);
        } else {
            self.samples[self.next] = seconds;
            self.next = (self.next + 1) % FRAME_WINDOW;
        }
    }

    /// Mean frame time in seconds, or `None` before any frame has run.
    fn mean(&self) -> Option<f32> {
        if self.samples.is_empty() {
            return None;
        }
        Some(self.samples.iter().sum::<f32>() / self.samples.len() as f32)
    }
}

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

/// A gizmo group of its own, so the overlay can draw hairlines while ink stays
/// broad. Line width is per config group, not per call, and sharing the default
/// group would mean guides as heavy as a pen stroke.
#[derive(Default, Reflect, GizmoConfigGroup)]
struct DebugGizmos;

impl Plugin for DebugOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<InputProbe>()
            .init_resource::<FrameTimes>()
            .init_resource::<OverlayVisible>()
            .add_systems(Update, (probe_input, sample_frame_time, toggle_overlay));

        app.init_gizmo_group::<DebugGizmos>();

        app.add_systems(Startup, (spawn_overlay, thin_gizmos))
            .add_systems(
                Update,
                // After capture, so the numbers describe this frame rather than
                // trailing it by one.
                (
                    place_overlay,
                    update_cursor_line,
                    update_ink_line,
                    update_input_line,
                    update_layout_line,
                    update_frame_line,
                    update_stroke_line,
                    draw_guides,
                    draw_stroke_ends,
                )
                    .after(crate::capture_stroke)
                    // Everything above is skipped outright when hidden, rather
                    // than each system testing the flag itself.
                    .run_if(overlay_visible),
            );
    }
}

/// Hairlines for the overlay's own group. One pixel, no joints — these are
/// rulers, and a ruler with weight reads as content.
fn thin_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DebugGizmos>();
    config.line.width = 1.0;
}

/// Run condition: is the overlay showing?
fn overlay_visible(visible: Res<OverlayVisible>) -> bool {
    visible.0
}

/// F1 shows and hides everything this file draws.
///
/// The text entities get their `Visibility` flipped; the gizmos stop because
/// their systems stop running. Two mechanisms because gizmos are immediate mode
/// and have no entity to hide.
fn toggle_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<OverlayVisible>,
    mut lines: Query<&mut Visibility, With<OverlayLine>>,
) {
    if !keys.just_pressed(KeyCode::F1) {
        return;
    }

    visible.0 = !visible.0;
    for mut visibility in &mut lines {
        *visibility = if visible.0 {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Banks this frame's duration.
///
/// Runs unconditionally, even while hidden, so the average is already warm when
/// the overlay comes back rather than reading zero for a second.
fn sample_frame_time(time: Res<Time>, mut frames: ResMut<FrameTimes>) {
    frames.push(time.delta_secs());
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

/// Marks the window-geometry line: size, corners, and where the credit sits.
#[derive(Component)]
struct LayoutLine;

/// Marks the frame-timing line.
#[derive(Component)]
struct FrameLine;

/// Marks the per-stroke breakdown — the shape of what the recognizer will see.
#[derive(Component)]
struct StrokeLine;

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
        // Lightened for the desk: the readouts sit in the bottom-left, which
        // is outside the paper disc at every window size. Each line keeps its
        // own hue so a glance finds the right block without reading it.
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
        TextColor(Color::srgb(0.88, 0.72, 0.44)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 0.0 },
        InkLine,
    ));

    commands.spawn((
        Text2d::new("input: —"),
        font.clone(),
        TextColor(Color::srgb(0.72, 0.58, 0.92)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 4.0 },
        InputLine,
    ));

    commands.spawn((
        Text2d::new("layout: —"),
        font.clone(),
        TextColor(Color::srgb(0.52, 0.74, 0.94)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        // Above the input block, which is two rows tall starting at 4.
        OverlayLine { lift: 6.0 },
        LayoutLine,
    ));

    // Row 3 is the gap the single-line cursor readout leaves above itself.
    commands.spawn((
        Text2d::new("frame: —"),
        font.clone(),
        TextColor(Color::srgb(0.72, 0.70, 0.64)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 3.0 },
        FrameLine,
    ));

    commands.spawn((
        Text2d::new("strokes: —"),
        font,
        TextColor(Color::srgb(0.92, 0.62, 0.64)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 8.0 },
        StrokeLine,
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
    mouse: Res<ButtonInput<MouseButton>>,
    probe: Res<InputProbe>,
    mut line: Single<&mut Text2d, With<InputLine>>,
) {
    let held: Vec<String> = keys.get_pressed().map(|key| format!("{key:?}")).collect();
    let buttons: Vec<String> = mouse
        .get_pressed()
        .map(|button| format!("{button:?}"))
        .collect();

    // Two focus readings on purpose: the field, and what events actually said.
    // They disagreeing is itself the answer.
    let focus_by_event = if probe.focus_events == 0 {
        "never".to_string()
    } else {
        format!("{}", probe.focused_by_event)
    };

    line.0 = format!(
        "focus field {}  events {focus_by_event}   keyev {}  last {}\nheld [{}]  mouse [{}]  F1 hides this",
        window.focused,
        probe.key_events,
        if probe.last_key.is_empty() {
            "—"
        } else {
            &probe.last_key
        },
        held.join(" "),
        buttons.join(" "),
    );
}

/// Draws the window edge, the centre axes, and the margin box.
///
/// The margin box is the useful one: anything pinned to a corner should land on
/// it, so a drifting element is visible as a gap rather than something you have
/// to measure.
fn draw_guides(
    window: Single<&Window>,
    credit: Option<Single<&Transform, With<Credit>>>,
    paper: Option<Single<&Transform, With<Paper>>>,
    mut gizmos: Gizmos<DebugGizmos>,
) {
    let half = Vec2::new(window.width(), window.height()) * 0.5;

    // Inset by one pixel: a rect drawn exactly on the edge is half outside the
    // viewport and reads as a three-sided box.
    gizmos.rect_2d(Isometry2d::IDENTITY, half * 2.0 - Vec2::splat(2.0), GUIDE);

    // Centre axes. Origin is where world coordinates are zero, which is worth
    // seeing when a transform lands somewhere unexpected.
    gizmos.line_2d(Vec2::new(-half.x, 0.0), Vec2::new(half.x, 0.0), GUIDE);
    gizmos.line_2d(Vec2::new(0.0, -half.y), Vec2::new(0.0, half.y), GUIDE);

    // The overlay and the credit use different margins, so both boxes are drawn
    // rather than one averaged guide that matches neither.
    for margin in [MARGIN, crate::CREDIT_MARGIN] {
        gizmos.rect_2d(
            Isometry2d::IDENTITY,
            (half - Vec2::splat(margin)) * 2.0,
            GUIDE,
        );
    }

    // Read from the transform, not recomputed from the window: the point is to
    // show where the credit *is*, so a wrong sign shows up as a mark in the
    // wrong corner instead of agreeing with the bug.
    if let Some(credit) = credit {
        cross(&mut gizmos, credit.translation.truncate(), ANCHOR_MARK);
    }

    // The sheet's edge, traced over its own fill. The mesh is a unit circle
    // scaled by its transform, so the drawn radius is the scale — reading it
    // back proves the fit rather than assuming it.
    if let Some(paper) = paper {
        gizmos.circle_2d(
            Isometry2d::from_translation(paper.translation.truncate()),
            paper.scale.x,
            GUIDE,
        );
        cross(&mut gizmos, paper.translation.truncate(), GUIDE);
    }
}

/// Marks the first and last point of every stroke.
///
/// Start is a circle, end is a cross — different shapes rather than different
/// colours only, so a one-point stroke still reads as both at once.
fn draw_stroke_ends(pad: Res<InkPad>, mut gizmos: Gizmos<DebugGizmos>) {
    for stroke in pad.points.chunk_by(|a, b| a.stroke_id == b.stroke_id) {
        let (Some(first), Some(last)) = (stroke.first(), stroke.last()) else {
            continue;
        };

        gizmos.circle_2d(
            Isometry2d::from_translation(Vec2::new(first.x, first.y)),
            TICK,
            START_MARK,
        );
        cross(&mut gizmos, Vec2::new(last.x, last.y), END_MARK);
    }
}

/// A small ✕ centred on `at`. Two lines rather than a glyph, because gizmos
/// have no text and this has to survive any zoom.
fn cross(gizmos: &mut Gizmos<DebugGizmos>, at: Vec2, color: Color) {
    let d = Vec2::splat(TICK);
    gizmos.line_2d(at - d, at + d, color);
    gizmos.line_2d(at - Vec2::new(d.x, -d.y), at + Vec2::new(d.x, -d.y), color);
}

/// Window size and the corners things are supposed to sit in.
fn update_layout_line(
    window: Single<&Window>,
    credit: Option<Single<&Transform, With<Credit>>>,
    paper: Option<Single<&Transform, With<Paper>>>,
    mut line: Single<&mut Text2d, With<LayoutLine>>,
) {
    let half = Vec2::new(window.width(), window.height()) * 0.5;

    let credit_at = match credit {
        Some(credit) => {
            let t = credit.translation;
            // The gap to each edge, which is the number that should equal
            // CREDIT_MARGIN. Printing the difference beats printing two
            // coordinates and doing the subtraction in your head.
            let gap = Vec2::new(half.x - t.x, t.y + half.y);
            format!("{:.0}, {:.0}  gap {:.0}, {:.0}", t.x, t.y, gap.x, gap.y)
        }
        None => "—".to_string(),
    };

    // Scale *is* the radius: the mesh is a unit circle. Also reported is how
    // much desk is left, which should hold at PAPER_MARGIN on the short axis.
    let paper_at = match paper {
        Some(paper) => {
            let radius = paper.scale.x;
            format!("r {radius:.0}  desk {:.0}", half.x.min(half.y) - radius)
        }
        None => "—".to_string(),
    };

    line.0 = format!(
        "window {:.0}×{:.0}   half ±{:.0}, ±{:.0}   dpi ×{:.2}   {}\ncredit {credit_at}   paper {paper_at}   margins {MARGIN:.0}/{:.0}",
        window.width(),
        window.height(),
        half.x,
        half.y,
        // Logical pixels versus physical. On a retina display this is 2.0, and
        // it is the usual reason a coordinate looks off by exactly double.
        window.scale_factor(),
        if window.focused {
            "focused"
        } else {
            "BACKGROUND"
        },
        crate::CREDIT_MARGIN,
    );
}

/// Frame budget, and how much of the pad is in memory.
///
/// Both on one line because they answer the same question: is drawing still
/// cheap as the point count climbs? `draw_ink` walks every point every frame,
/// so this is where that would first show.
fn update_frame_line(
    frames: Res<FrameTimes>,
    pad: Res<InkPad>,
    mut line: Single<&mut Text2d, With<FrameLine>>,
) {
    let timing = match frames.mean() {
        // 60fps is a 16.7ms budget; the percentage is easier to judge than
        // either number alone.
        Some(mean) if mean > 0.0 => format!(
            "{:.1} fps   {:.2} ms   {:.0}% of 16.7ms",
            1.0 / mean,
            mean * 1000.0,
            mean * 1000.0 / 16.7 * 100.0
        ),
        _ => "—".to_string(),
    };

    line.0 = format!(
        "{timing}   points {}/{} cap",
        pad.points.len(),
        pad.points.capacity()
    );
}

/// The pad broken down the way the recognizer will read it.
///
/// Point counts per stroke are the number that matters for $P: too few and a
/// gesture cannot be matched, wildly uneven and `MIN_POINT_SPACING` is wrong.
fn update_stroke_line(pad: Res<InkPad>, mut line: Single<&mut Text2d, With<StrokeLine>>) {
    let strokes: Vec<(u32, usize)> = pad
        .points
        .chunk_by(|a, b| a.stroke_id == b.stroke_id)
        .filter_map(|stroke| Some((stroke.first()?.stroke_id, stroke.len())))
        .collect();

    let listed: Vec<String> = strokes
        .iter()
        .rev()
        .take(STROKES_SHOWN)
        .map(|(id, count)| format!("#{id}:{count}"))
        .collect();

    let hidden = strokes.len().saturating_sub(STROKES_SHOWN);
    let more = if hidden > 0 {
        format!(" +{hidden} more")
    } else {
        String::new()
    };

    // Undo entries hold whole strokes after an undo and the whole pad after a
    // clear, so their sizes say which kind of entry each one is.
    let undone: Vec<String> = pad
        .undone
        .iter()
        .rev()
        .take(STROKES_SHOWN)
        .map(|stroke| stroke.len().to_string())
        .collect();

    line.0 = format!(
        "strokes {} newest [{}]{more}\nundo stack {} sizes [{}]",
        strokes.len(),
        listed.join(" "),
        pad.undone.len(),
        undone.join(" "),
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
