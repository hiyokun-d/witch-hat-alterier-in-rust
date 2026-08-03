use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore, GizmoLineJoint};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use magic_core::Point;

mod debug;
mod shortcuts;

use shortcuts::{command_held, redo, undo};

/// Closer than this and a sample is dropped. A motionless hand still fires
/// `pressed` every frame, and hundreds of identical points would skew every
/// average the recognizer takes later.
///
/// World units, and provisional — once quality scoring lands this probably
/// belongs in core, where it can be reasoned about as part of stroke neatness.
const MIN_POINT_SPACING: f32 = 2.0;

/// Half of `DefaultPlugins`' default window, in world units. Hardcoded because
/// the credit is spawned once at startup — resize the window and it stops
/// tracking the corner. Fine while the window is fixed; the day it isn't, this
/// becomes a system that reads `Window::resolution`.
const WINDOW_HALF: Vec2 = Vec2::new(640.0, 360.0);

/// Gap between the credit and the window edge.
const CREDIT_MARGIN: f32 = 16.0;

/// Stroke weight in pixels. Seals in the source run roughly 2–3% of the glyph's
/// diameter, so a palm-sized seal on a 1280px window lands near here.
const INK_WIDTH: f32 = 5.0;

/// Aged parchment. Witches in the source draw dark on warm paper, never on
/// white — the cream is what keeps ink from reading as harsh.
const PAPER: Color = Color::srgb_u8(0xE8, 0xDC, 0xC4);

/// Iron-gall black: as dark as the paper allows, biased brown rather than blue.
const INK: Color = Color::srgb_u8(0x22, 0x1C, 0x18);

/// The credit line. Same warm family as [`INK`], lifted toward the paper until
/// it recedes — a signature, not something to read while drawing.
const CREDIT: Color = Color::srgb_u8(0xA8, 0x9C, 0x88);

/// Every point drawn so far, across every stroke.
///
/// Flat rather than nested: `$P` consumes a point cloud where stroke membership
/// is an attribute of the point, so `Point::stroke_id` is the only thing that
/// needs to say which stroke a point belongs to.
#[derive(Resource, Default)]
pub struct InkPad {
    pub points: Vec<Point>,
    /// Id the stroke currently being drawn will carry. Bumped on release.
    pub stroke_id: u32,
    /// Strokes lifted off by undo, newest last. Nested where `points` is flat:
    /// this is edit history, not something the recognizer ever sees.
    pub undone: Vec<Vec<Point>>,
}

// all of the code will be start here just like C
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Scaffolding. Delete this line and `mod debug;` and nothing breaks.
        .add_plugins(debug::DebugOverlayPlugin)
        .insert_resource(ClearColor(PAPER))
        .init_resource::<InkPad>()
        .add_systems(Startup, setup)
        // Shortcuts run after capture so an undo pressed mid-drag wins the
        // frame, rather than leaving behind the point captured a moment
        // earlier. Drawing comes last so the pad it renders is this frame's.
        .add_systems(
            Update,
            (capture_stroke, keyboard_shortcut, draw_ink).chain(),
        )
        .run();
}

fn setup(mut commands: Commands, mut gizmo_config: ResMut<GizmoConfigStore>) {
    commands.spawn(Camera2d);

    // Gizmo lines default to 2px, which reads as a pencil sketch. Seals in the
    // source are inked with a broad nib — heavy enough that the ring and the
    // signs carry equal weight at a glance.
    let (config, _) = gizmo_config.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = INK_WIDTH;
    // A thick polyline is drawn as separate quads, so every direction change
    // leaves a notch on the outside of the turn. Round joints fill them, which
    // is also what a real nib does.
    config.line.joints = GizmoLineJoint::Round(8);

    commands.spawn((
        Text2d::new("This app made by HIYO"),
        TextFont {
            font_size: FontSize::Px(12.0),
            ..default()
        },
        TextColor(CREDIT),
        // Anchored by its own bottom-right corner, so the transform below is
        // where the text *ends*, not where it centres. Without this the string
        // straddles the point and half of it hangs off the window.
        Anchor::BOTTOM_RIGHT,
        Transform::from_xyz(
            WINDOW_HALF.x - CREDIT_MARGIN,
            -WINDOW_HALF.y + CREDIT_MARGIN,
            0.0,
        ),
    ));
}

/// ⌘Z / Ctrl+Z undoes a stroke, ⇧⌘Z or Ctrl+Y puts it back.
///
/// Modifiers use `pressed`, the action key uses `just_pressed`: a modifier is a
/// state you hold, the action is an edge. Asking `just_pressed` of both would
/// demand they go down on the same frame — a 16ms window nobody hits.
fn keyboard_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut pad: ResMut<InkPad>,
) {
    // Editing history while a stroke is still open corrupts it: undo would lift
    // the in-progress stroke onto the stack, the drag would keep appending
    // under the same freed id, and a later redo would splice the two together
    // as one stroke with a jump in the middle.
    if mouse.pressed(MouseButton::Left) {
        return;
    }

    if !command_held(&keys) {
        return;
    }

    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // Redo first: ⇧⌘Z also satisfies the plain undo test, so checking undo
    // first would swallow it.
    if (shift && keys.just_pressed(KeyCode::KeyZ)) || keys.just_pressed(KeyCode::KeyY) {
        redo(&mut pad);
    } else if keys.just_pressed(KeyCode::KeyZ) {
        undo(&mut pad);
    }
}

/// Turns press, drag, and release into a stroke of [`Point`]s on [`InkPad`].
pub fn capture_stroke(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut pad: ResMut<InkPad>,
) {
    let (camera, camera_transform) = *camera;

    // there's 2 different type of pressed and here's the different
    // just_pressed: is just for once or one time press but not hold
    // pressed: we can hold it down and still track where the cursor goes on the click while we drag

    // A fresh stroke is a new edit, and a new edit invalidates the redo stack —
    // otherwise redo would splice an old stroke in on top of newer work.
    if mouse.just_pressed(MouseButton::Left) {
        pad.undone.clear();
    }

    // No first-point special case: `pressed` is already true on the frame the
    // button goes down, so the click position lands here like any other sample.
    if mouse.pressed(MouseButton::Left) {
        // Off-window or a degenerate viewport. Skip the sample and leave the
        // stroke open — dragging back into the window resumes the same stroke.
        if let Some(world) = cursor_world(&window, camera, camera_transform) {
            // Copied out before the push: `pad.points` borrows `pad` mutably,
            // so `pad.stroke_id` can't be read inside the same expression.
            let stroke_id = pad.stroke_id;
            let point = Point {
                x: world.x,
                y: world.y,
                stroke_id,
            };

            let too_close = pad.points.last().is_some_and(|last| {
                last.stroke_id == stroke_id && last.dist(&point) < MIN_POINT_SPACING
            });

            if !too_close {
                pad.points.push(point);
            }
        }
    }

    // Bump only on release, so the next press can't merge into this stroke.
    if mouse.just_released(MouseButton::Left) {
        pad.stroke_id += 1;
    }
}

/// Where the cursor is in world space, or `None` when there is no answer.
///
/// Two ways to have no answer, and neither is an error worth distinguishing:
/// the pointer is outside the window, or the viewport is degenerate. Callers
/// that draw ink react to both the same way — by not drawing.
pub fn cursor_world(
    window: &Window,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Option<Vec2> {
    let cursor = window.cursor_position()?;
    camera.viewport_to_world_2d(camera_transform, cursor).ok()
}

/// Draws every stroke on the pad as a polyline.
///
/// Gizmos are immediate mode: nothing is spawned and nothing persists, so this
/// redraws the whole pad from scratch each frame. That is why undo needs no
/// cleanup — the points are gone, so the line is.
pub fn draw_ink(mut gizmos: Gizmos, pad: Res<InkPad>) {
    for pair in pad.points.windows(2) {
        let (start, end) = (pair[0], pair[1]);

        // Strokes sit back to back in one flat `Vec`, so consecutive points can
        // straddle a pen lift. Without this, the end of one stroke joins the
        // start of the next and draws a line across the canvas.
        if start.stroke_id != end.stroke_id {
            continue;
        }

        gizmos.line_2d(Vec2::new(start.x, start.y), Vec2::new(end.x, end.y), INK)
    }
}
