use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore, GizmoLineJoint};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use magic_core::Point;

mod debug;
mod shortcuts;

use shortcuts::{clear, command_held, redo, undo};

/// Closer than this and a sample is dropped. A motionless hand still fires
/// `pressed` every frame, and hundreds of identical points would skew every
/// average the recognizer takes later.
///
/// World units, and provisional — once quality scoring lands this probably
/// belongs in core, where it can be reasoned about as part of stroke neatness.
const MIN_POINT_SPACING: f32 = 2.0;

/// Gap between the credit and the window edge.
const CREDIT_MARGIN: f32 = 16.0;

/// Gap between the paper disc and the nearest window edge.
///
/// Wider than the credit's margin so the sheet reads as an object lying on the
/// desk rather than a viewport that happens to be round.
const PAPER_MARGIN: f32 = 28.0;

/// Behind everything. Ink is drawn with gizmos, which ignore z and always land
/// on top, but the credit is `Text2d` and would otherwise be hidden by a sheet
/// spawned after it.
const PAPER_Z: f32 = -10.0;

/// Stroke weight in pixels. Seals in the source run roughly 2–3% of the glyph's
/// diameter, so a palm-sized seal on a 1280px window lands near here.
const INK_WIDTH: f32 = 5.0;

/// Aged parchment. Witches in the source draw dark on warm paper, never on
/// white — the cream is what keeps ink from reading as harsh.
const PAPER: Color = Color::srgb_u8(0xE8, 0xDC, 0xC4);

/// The surface the sheet lies on. Dark enough that the parchment reads as lit,
/// warm enough that it does not look like a UI panel.
const DESK: Color = Color::srgb_u8(0x3A, 0x2E, 0x22);

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

/// Marks the signature in the corner so [`place_credit`] can find it.
///
/// Empty on purpose. A query filters by component *type*, so the type being
/// present on one entity and absent everywhere else is the whole payload —
/// without it, `Query<&mut Transform>` would also match the camera and every
/// debug line.
#[derive(Component)]
pub struct Credit;

/// Marks the round sheet of parchment so [`fit_paper`] can resize it.
///
/// Public so the debug overlay can read where the sheet is without the sheet
/// having to know the overlay exists.
#[derive(Component)]
pub struct Paper;

// all of the code will be start here just like C
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Witch hat atelier".to_string(),
                ..default()
            }),
            ..default()
        }))
        // Scaffolding. Delete this line and `mod debug;` and nothing breaks.
        .add_plugins(debug::DebugOverlayPlugin)
        .insert_resource(ClearColor(DESK))
        .init_resource::<InkPad>()
        .add_systems(Startup, setup)
        // Shortcuts run after capture so an undo pressed mid-drag wins the
        // frame, rather than leaving behind the point captured a moment
        // earlier. Drawing comes last so the pad it renders is this frame's.
        .add_systems(
            Update,
            (
                capture_stroke,
                keyboard_shortcut,
                draw_ink,
                place_credit,
                fit_paper,
            )
                .chain(),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut gizmo_config: ResMut<GizmoConfigStore>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2d);

    // A unit circle, scaled by `fit_paper` rather than rebuilt. Mesh assets are
    // vertex buffers uploaded to the GPU; regenerating one every frame to change
    // its size would be the expensive way to do a multiply.
    commands.spawn((
        Mesh2d(meshes.add(Circle::new(1.0))),
        MeshMaterial2d(materials.add(PAPER)),
        Transform::from_xyz(0.0, 0.0, PAPER_Z),
        Paper,
    ));

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
        // Anchored by its own bottom-right corner, so the transform is where the
        // text *ends*, not where it centres. Without this the string straddles
        // the point and half of it hangs off the window.
        Anchor::BOTTOM_RIGHT,
        // No `Transform` here: `place_credit` writes one every frame, and it
        // runs before the first frame is drawn.
        Credit,
    ));
}

/// ⌘Z / Ctrl+Z undoes a stroke, ⇧⌘Z or Ctrl+Y puts it back, ⌘⌫ wipes the pad.
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

    // ignore the ctrl shortcuts
    if keys.just_pressed(KeyCode::KeyA) {
        println!("Changing brushes or something i don't know")
    }

    // using ctrl for the shortcuts
    if command_held(&keys) {
        let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

        // Redo first: ⇧⌘Z also satisfies the plain undo test, so checking undo
        // first would swallow it.
        if (shift && keys.just_pressed(KeyCode::KeyZ)) || keys.just_pressed(KeyCode::KeyY) {
            redo(&mut pad);
        } else if keys.just_pressed(KeyCode::KeyZ) {
            undo(&mut pad);
        } else if shift && keys.just_pressed(KeyCode::Backspace) {
            clear(&mut pad);
        }
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

/// Sizes the parchment disc to the window, leaving [`PAPER_MARGIN`] of desk.
///
/// The sheet is a unit circle, so fitting it is a scale, not a new mesh. Driven
/// by the *shorter* window axis, which is what keeps it a full circle instead of
/// letting a wide window push its top and bottom off screen.
fn fit_paper(window: Single<&Window>, mut paper: Single<&mut Transform, With<Paper>>) {
    let shorter = window.width().min(window.height());

    // A window dragged smaller than twice the margin would ask for a negative
    // radius, which flips the mesh inside out. One pixel of paper is the floor.
    let radius = (shorter * 0.5 - PAPER_MARGIN).max(1.0);

    // Uniform scale on x and y only: z stays 1.0 so `PAPER_Z` keeps its meaning.
    paper.scale = Vec3::new(radius, radius, 1.0);
}

/// Keeps the signature in the bottom-right corner as the window resizes.
///
/// Runs every frame rather than only on a resize event: it is two float writes,
/// and reacting to events would mean handling the first frame, monitor changes,
/// and DPI shifts as separate cases.
fn place_credit(window: Single<&Window>, mut credit: Single<&mut Transform, With<Credit>>) {
    // The world puts its origin at the centre with +y up, so the bottom-right
    // corner is half the width to the right and half the height down. The two
    // axes need opposite signs, which is why this is not one multiply.
    let corner = Vec2::new(window.width(), -window.height()) * 0.5;

    // Pulled back in along both axes: x toward the centre, y up off the edge.
    credit.translation.x = corner.x - CREDIT_MARGIN;
    credit.translation.y = corner.y + CREDIT_MARGIN;
}
