use bevy::gizmos::config::{DefaultGizmoConfigGroup, GizmoConfigStore, GizmoLineJoint};
use bevy::prelude::*;
use bevy::sprite::Anchor;
use magic_core::Point;

mod debug;
mod hand;
mod particles;
mod props;
mod reading;
mod shortcuts;
mod sim;
mod ui;

use shortcuts::{TapCounter, clear, clear_all, command_held, redo, undo};

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
pub(crate) const PAPER: Color = Color::srgb_u8(0xE8, 0xDC, 0xC4);

/// The surface the sheet lies on. Dark enough that the parchment reads as lit,
/// warm enough that it does not look like a UI panel.
pub(crate) const DESK: Color = Color::srgb_u8(0x3A, 0x2E, 0x22);

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

/// Marks the sheet of parchment so [`fit_paper`] can resize it.
///
/// Public so the debug overlay can read where the sheet is without the sheet
/// having to know the overlay exists.
#[derive(Component)]
pub struct Paper;

/// Which sheet is on the desk. Toggled with `F`.
///
/// This is not decoration: the sheet's edge is the boundary the pen honours, so
/// the shape decides where a stroke is allowed to exist.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PaperShape {
    /// Parchment wall to wall. Anywhere in the window takes ink.
    ///
    /// The default, so nothing about the pen surprises anyone who just opened
    /// the app — [`PaperShape::Disc`] is there to be found.
    #[default]
    Full,
    /// A round sheet lying on the desk. The pen stops at its edge.
    Disc,
}

impl PaperShape {
    /// The other one. A named method rather than an inline `match` at the call
    /// site, so adding a third sheet later is one edit here.
    fn toggled(self) -> PaperShape {
        match self {
            PaperShape::Disc => PaperShape::Full,
            PaperShape::Full => PaperShape::Disc,
        }
    }

    /// Half-extents of the sheet in world units: the disc's radius in both
    /// axes, or half the window for full-bleed.
    ///
    /// One function so [`fit_paper`] and [`PaperShape::accepts`] can never
    /// disagree about where the edge is — a sheet you can draw off the side of
    /// would be exactly that disagreement.
    pub fn extent(self, window: &Window) -> Vec2 {
        let half = Vec2::new(window.width(), window.height()) * 0.5;

        match self {
            // Driven by the shorter axis, which is what keeps it a full circle
            // rather than letting a wide window push its top and bottom off
            // screen. A window narrower than twice the margin would ask for a
            // negative radius and turn the mesh inside out, hence the floor.
            PaperShape::Disc => Vec2::splat((half.min_element() - PAPER_MARGIN).max(1.0)),
            PaperShape::Full => half,
        }
    }

    /// Whether the sheet takes ink at `point`, in world space.
    pub fn accepts(self, window: &Window, point: Vec2) -> bool {
        let extent = self.extent(window);

        match self {
            PaperShape::Disc => point.length() <= extent.x,
            PaperShape::Full => point.abs().cmple(extent).all(),
        }
    }
}

/// The two sheets, built once at startup.
///
/// Meshes are vertex buffers on the GPU. Toggling swaps a handle; it does not
/// build geometry, and neither does resizing — that is a scale on the transform.
#[derive(Resource)]
struct PaperMeshes {
    disc: Handle<Mesh>,
    full: Handle<Mesh>,
}

impl PaperMeshes {
    /// The mesh for a given sheet.
    fn of(&self, shape: PaperShape) -> Handle<Mesh> {
        match shape {
            PaperShape::Disc => self.disc.clone(),
            PaperShape::Full => self.full.clone(),
        }
    }
}

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
        // The typeface, before anything spawns text.
        .add_plugins(hand::HandPlugin)
        // Scaffolding. Delete this line and `mod debug;` and nothing breaks.
        .add_plugins(debug::DebugOverlayPlugin)
        // Ways to build a seal without drawing one. Same deal — delete this
        // line and `mod ui;` and the pen still works.
        .add_plugins(ui::ToolbarPlugin)
        // What a compiled spell actually does. Delete this line and `mod sim;`
        // and the pad, the recognizer and the compiler are untouched.
        // Reads the pad once a frame so nothing else has to. Must come before
        // the simulation and the overlay, which both consume the answer.
        .add_plugins(reading::ReadingPlugin)
        .add_plugins(sim::SimPlugin)
        // World content, not measurement: props are the thing a spell is aimed
        // at and motes are what a moment looks like, so neither hides with F1.
        .add_plugins(props::PropsPlugin)
        .add_plugins(particles::MotesPlugin)
        .insert_resource(ClearColor(DESK))
        .init_resource::<InkPad>()
        .init_resource::<PaperShape>()
        .init_resource::<TapCounter>()
        .add_systems(Startup, setup)
        // Shortcuts run after capture so an undo pressed mid-drag wins the
        // frame, rather than leaving behind the point captured a moment
        // earlier. Drawing comes last so the pad it renders is this frame's.
        .add_systems(
            Update,
            (
                // Not while the pointer is over a button: a click on the
                // toolbar must not also land a blot of ink underneath it.
                capture_stroke.run_if(ui::pointer_free),
                keyboard_shortcut,
                draw_ink,
                place_credit,
                // After `keyboard_shortcut`, so a sheet swapped this frame is
                // rescaled in the same frame rather than flashing at the old
                // size.
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

    // Both sheets are unit-sized and scaled by `fit_paper` rather than rebuilt.
    // A `Rectangle` of 1×1 scales to any window; a `Circle` of radius 1 scales
    // to any radius. Regenerating either every frame would be the expensive way
    // to do a multiply.
    let paper_meshes = PaperMeshes {
        disc: meshes.add(Circle::new(1.0)),
        full: meshes.add(Rectangle::new(1.0, 1.0)),
    };

    commands.spawn((
        Mesh2d(paper_meshes.of(PaperShape::default())),
        MeshMaterial2d(materials.add(PAPER)),
        Transform::from_xyz(0.0, 0.0, PAPER_Z),
        Paper,
    ));

    commands.insert_resource(paper_meshes);

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
#[allow(clippy::too_many_arguments)]
fn keyboard_shortcut(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    meshes: Res<PaperMeshes>,
    mut taps: ResMut<TapCounter>,
    mut shape: ResMut<PaperShape>,
    mut paper: Single<&mut Mesh2d, With<Paper>>,
    mut pad: ResMut<InkPad>,
) {
    let now = time.elapsed_secs();
    // Ages an open tap run even on frames where nothing is pressed, so the
    // readout shows it lapse rather than staying lit until the next press.
    taps.expire(now);

    // Editing history while a stroke is still open corrupts it: undo would lift
    // the in-progress stroke onto the stack, the drag would keep appending
    // under the same freed id, and a later redo would splice the two together
    // as one stroke with a jump in the middle. Swapping sheets mid-stroke has
    // the same problem from the other side — it moves the boundary under a pen
    // already committed to a line.
    if mouse.pressed(MouseButton::Left) {
        return;
    }

    // ignore the ctrl shortcuts
    // This key will be used to change like tools
    if keys.just_pressed(KeyCode::KeyA) {
        println!("Changing brushes or something i don't know")
    }

    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    // using ctrl for the shortcuts
    if command_held(&keys) {
        // Redo first: ⇧⌘Z also satisfies the plain undo test, so checking undo
        // first would swallow it.
        if (shift && keys.just_pressed(KeyCode::KeyZ)) || keys.just_pressed(KeyCode::KeyY) {
            redo(&mut pad);
        } else if keys.just_pressed(KeyCode::KeyZ) {
            undo(&mut pad);
        } else if keys.just_pressed(KeyCode::Backspace) {
            clear(&mut pad);
        }

        // A modifier is held, so nothing below should also fire. ⌘F stays free.
        return;
    }

    // Three bare F presses inside the counter's window swap the sheet. A single
    // press would be far too easy to hit while reaching for anything else, and
    // the swap wipes the pad.
    if keys.just_pressed(KeyCode::KeyF) && taps.tap(now) {
        // Ink outside the new sheet would be stranded — unreachable by the pen
        // and uneditable — so the swap starts from a blank pad rather than
        // leaving strokes the boundary no longer admits.
        clear_all(&mut pad);
        *shape = shape.toggled();
        // Only the handle changes. The transform is left alone; `fit_paper`
        // runs later this frame and rescales it for the new shape.
        paper.0 = meshes.of(*shape);
    }
}

/// Turns press, drag, and release into a stroke of [`Point`]s on [`InkPad`].
pub fn capture_stroke(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    shape: Res<PaperShape>,
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

            // The sheet's edge is where ink can exist, so a sample off the paper
            // is dropped exactly as a sample off the window is. The stroke stays
            // open: drag back onto the sheet and it carries on, the way a nib
            // lifted over the edge and set down again would.
            if !too_close && shape.accepts(&window, world) {
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

/// Sizes the sheet to the window.
///
/// Both meshes are unit-sized, so fitting is a scale, never a new mesh. The
/// numbers come from [`PaperShape::extent`], the same function the pen consults
/// — one source of truth for where the edge is.
fn fit_paper(
    window: Single<&Window>,
    shape: Res<PaperShape>,
    mut paper: Single<&mut Transform, With<Paper>>,
) {
    let extent = shape.extent(&window);

    // The disc mesh has radius 1 and the rectangle is 1×1, so a disc scales by
    // its radius and a full sheet by its full size. Doubling here rather than in
    // `extent` keeps that function talking in half-extents throughout.
    let scale = match *shape {
        PaperShape::Disc => extent,
        PaperShape::Full => extent * 2.0,
    };

    // z stays 1.0 so `PAPER_Z` keeps its meaning.
    paper.scale = Vec3::new(scale.x, scale.y, 1.0);
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

#[cfg(test)]
#[path = "tests/main.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/seal_round_trip.rs"]
mod seal_round_trip;
