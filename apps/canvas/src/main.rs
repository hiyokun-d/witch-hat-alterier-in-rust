use bevy::prelude::*;
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
        .insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.09)))
        .init_resource::<InkPad>()
        .add_systems(Startup, setup)
        // Shortcuts run last so an undo pressed mid-drag wins the frame,
        // rather than leaving behind the point captured a moment earlier.
        .add_systems(Update, (capture_stroke, keyboard_shortcut).chain())
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text2d::new("This app made by HIYO"),
        TextFont {
            font_size: FontSize::Px(18.0),
            ..default()
        },
        TextColor(Color::srgb(0.35, 0.35, 0.40)),
        Transform::from_xyz(547.0, 332.0, 0.0),
    ));
}

/// ⌘Z / Ctrl+Z undoes a stroke, ⇧⌘Z or Ctrl+Y puts it back.
///
/// Modifiers use `pressed`, the action key uses `just_pressed`: a modifier is a
/// state you hold, the action is an edge. Asking `just_pressed` of both would
/// demand they go down on the same frame — a 16ms window nobody hits.
fn keyboard_shortcut(keys: Res<ButtonInput<KeyCode>>, mut pad: ResMut<InkPad>) {
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

    // No `just_pressed` arm: `pressed` is already true on the frame the button
    // goes down, so the click position becomes the stroke's first point without
    // a special case.
    // A fresh stroke is a new edit, and a new edit invalidates the redo stack —
    // otherwise redo would splice an old stroke in on top of newer work.
    if mouse.just_pressed(MouseButton::Left) {
        pad.undone.clear();
    }

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
