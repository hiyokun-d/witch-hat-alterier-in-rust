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

use magic_core::{assembly, circle, recognizer, stroke};

use crate::shortcuts::TapCounter;
use crate::{Credit, InkPad, Paper, PaperShape, cursor_world};

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

/// The circle `magic_core::circle::fit` found, and its centre.
const FIT_RING: Color = Color::srgba(0.15, 0.45, 0.85, 0.90);
/// The ±rms tolerance band around that circle.
const FIT_BAND: Color = Color::srgba(0.15, 0.45, 0.85, 0.30);
/// The raw centroid of the stroke — where the fitter shifts the origin to.
///
/// Drawn separately from the centre on purpose: on a full ring the two sit on
/// top of each other, and on an arc they pull apart. That gap is the bias
/// Taubin exists to correct, made visible.
const CENTROID_MARK: Color = Color::srgb(0.85, 0.45, 0.10);
/// Per-point miss distance, drawn as a whisker off the fitted circle.
const DEVIATION: Color = Color::srgba(0.80, 0.20, 0.30, 0.60);

/// Deviations are magnified by this much before being drawn.
///
/// A neat ring misses by two pixels out of three hundred, which is invisible
/// and also the whole point. The band and the whiskers share this gain so they
/// stay comparable, and the readout prints it so nothing here reads as literal.
const DEVIATION_GAIN: f32 = 5.0;

/// Draw a whisker every N points. Every point turns a wobbly stroke into a
/// solid red smear that says less than a comb does.
const WHISKER_STRIDE: usize = 3;

/// Segments in a fitted circle. The default is too few — a 300px ring drawn at
/// 32 segments reads as a polygon and looks like a bad fit when it is not.
const FIT_RESOLUTION: u32 = 96;

/// The untrimmed fit, drawn faintly behind the trimmed one.
///
/// Two circles rather than a count of dropped points: when they sit on top of
/// each other the trim did nothing, and when they separate the gap between
/// them *is* what the outlier was doing to the answer.
const FIT_RAW: Color = Color::srgba(0.45, 0.42, 0.38, 0.55);
/// The hole in a ring that is still open.
const GAP_OPEN: Color = Color::srgb(0.92, 0.58, 0.12);
/// The hole in a ring small enough to count as closed.
const GAP_CLOSED: Color = Color::srgb(0.18, 0.62, 0.36);
/// Evenly spaced samples taken along the ring itself.
const RESAMPLE_RING: Color = Color::srgba(0.20, 0.35, 0.70, 0.95);
/// Evenly spaced samples taken along whatever the ring encloses.
const RESAMPLE_HELD: Color = Color::srgba(0.10, 0.52, 0.44, 0.95);
/// Radius of a resample dot.
const SAMPLE_DOT: f32 = 2.6;

/// Side of the normalised-cloud preview, in pixels.
///
/// The cloud itself is unitless — that is the whole point of normalising — so
/// this number only decides how big the picture of it is.
const PREVIEW_SIZE: f32 = 150.0;
/// Gap between the preview and the window edges.
const PREVIEW_MARGIN: f32 = 24.0;
/// The preview's frame and axes.
const PREVIEW_FRAME: Color = Color::srgba(0.55, 0.30, 0.75, 0.45);

/// The reach of whatever the ring encloses — the future sigil's bubble.
const CONTENTS_MARK: Color = Color::srgba(0.20, 0.58, 0.50, 0.75);
/// Ink that fits a circle and covers it, and is still not a ring — a
/// figure-eight, a double loop, a stroke that doubles back.
const NOT_A_RING: Color = Color::srgb(0.72, 0.16, 0.22);
/// Closed, but too rough to hold.
const GAP_FLEETING: Color = Color::srgb(0.62, 0.52, 0.14);

/// One colour per activation state, so the captions, the gap arcs and the
/// panel can never disagree about what they are showing.
fn activation_color(state: assembly::Activation) -> Color {
    match state {
        assembly::Activation::Malformed => NOT_A_RING,
        assembly::Activation::Armed => GAP_OPEN,
        assembly::Activation::Fleeting => GAP_FLEETING,
        assembly::Activation::Active => GAP_CLOSED,
    }
}

/// How wide a hole may be and still count as a closed ring, in pixels.
///
/// Stays in the shell on purpose. Distances in pixels are a fact about this
/// screen and this pen, and core has no business knowing either — the split is
/// that ratios live in `RingRules` and lengths come from here (§4.2). Four pen
/// widths: wide enough to forgive the sampling step, far too narrow for a
/// deliberate gap.
const CLOSURE_TOLERANCE: f32 = crate::INK_WIDTH * 4.0;

/// How far ink may sit off a circle and still count as lying on it.
const ON_RING_TOLERANCE: f32 = crate::INK_WIDTH * 2.5;

/// Segments used to draw the gap arc.
const GAP_RESOLUTION: usize = 24;

/// The ring being inspected, highlighted so the panel and the pad agree.
const INSPECT_MARK: Color = Color::srgba(0.55, 0.30, 0.75, 0.85);
/// How near a ring's edge the cursor must come to inspect it.
const INSPECT_REACH: f32 = 60.0;

/// What core says a ring must satisfy to close a circuit.
///
/// Every number in here is a ratio, which is why it is core's to hold: a ratio
/// means the same on any screen. `min_quality` is the one value canon does not
/// give us — see `RingRules`.
const RULES: assembly::RingRules = assembly::RingRules {
    simple_tolerance: 0.08,
    min_quality: 0.95,
};

/// Cells in the quality bar.
const QUALITY_CELLS: usize = 10;

/// How far outside a ring its own label floats.
const LABEL_OFFSET: f32 = 24.0;
/// Labels are smaller than the corner readouts — there can be a dozen of them
/// on the pad at once, and they sit on top of the drawing.
const LABEL_SIZE: f32 = 13.0;

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
                    update_fit_line,
                    update_ring_labels,
                    place_inspector,
                    update_inspector,
                    draw_guides,
                    draw_stroke_ends,
                    draw_fits,
                )
                    .after(crate::capture_stroke)
                    // Everything above is skipped outright when hidden, rather
                    // than each system testing the flag itself.
                    .run_if(overlay_visible),
            );

        // Ring labels are spawned on demand, so unlike the fixed readouts there
        // is nothing for `toggle_overlay` to hide. They are dropped instead and
        // rebuilt on the next visible frame.
        app.add_systems(Update, clear_ring_labels.run_if(not(overlay_visible)));
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

/// Every piece of text the overlay owns.
///
/// The inspector is pinned by its own system rather than the `lift` stack, so
/// it carries no [`OverlayLine`] and has to be named separately.
type AnyReadout = Or<(With<OverlayLine>, With<InspectorLine>)>;

/// F1 shows and hides everything this file draws.
///
/// The text entities get their `Visibility` flipped; the gizmos stop because
/// their systems stop running. Two mechanisms because gizmos are immediate mode
/// and have no entity to hide.
fn toggle_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<OverlayVisible>,
    mut lines: Query<&mut Visibility, AnyReadout>,
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

/// Marks the circle-fit readout — what `magic_core::circle::fit` made of the ink.
#[derive(Component)]
struct FitLine;

/// Marks the full-detail panel for one ring, pinned to the top-left.
#[derive(Component)]
struct InspectorLine;

/// A caption floating beside one fitted ring, carrying its slot in the pool.
///
/// A seal can be several rings at once — nested (canon rule 4), linked (rule
/// 5), or a split ring across two objects (rule 3) — so one readout at the
/// bottom of the screen cannot say which ring is which. Each gets its own.
///
/// The index keeps labels stapled to the same slot between frames. Bevy query
/// order is not guaranteed, and without it the captions would shuffle every
/// frame.
#[derive(Component)]
struct RingLabel(usize);

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
        font.clone(),
        TextColor(Color::srgb(0.92, 0.62, 0.64)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 8.0 },
        StrokeLine,
    ));

    // Above the stroke block, which is two rows tall starting at 8. Blue to
    // match the circle the gizmos draw for it.
    commands.spawn((
        Text2d::new("fit: —"),
        font,
        TextColor(Color::srgb(0.42, 0.68, 0.96)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 10.0 },
        FitLine,
    ));

    // Its own corner, not the bottom-left stack. The panel is a dozen rows and
    // would push everything else off the pad.
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.62, 0.55, 0.86)),
        TextLayout::justify(Justify::Left),
        Anchor::TOP_LEFT,
        InspectorLine,
    ));
}

/// Pins the inspector to the top-left corner.
fn place_inspector(
    window: Single<&Window>,
    mut panel: Single<&mut Transform, With<InspectorLine>>,
) {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    panel.translation.x = -half.x + MARGIN;
    panel.translation.y = half.y - MARGIN;
}

/// Which ring the cursor is inspecting: the one whose edge it is nearest.
///
/// Falls back to the first ring so the panel is never blank while there is
/// something to say. Returns the slot and whether the cursor picked it.
fn inspected(rings: &[assembly::RingCandidate], cursor: Option<Vec2>) -> Option<(usize, bool)> {
    if rings.is_empty() {
        return None;
    }

    let hovered = cursor.and_then(|at| {
        rings
            .iter()
            .enumerate()
            .map(|(slot, ring)| {
                let center = Vec2::new(ring.fit.center.x, ring.fit.center.y);
                (slot, (at.distance(center) - ring.fit.radius).abs())
            })
            .filter(|(_, off)| *off <= INSPECT_REACH)
            // `total_cmp` keeps the pick deterministic when two rings overlap.
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(slot, _)| slot)
    });

    Some(match hovered {
        Some(slot) => (slot, true),
        None => (0, false),
    })
}

/// Draws a normalised cloud in a fixed box at the top-right of the window.
///
/// Worth its own corner rather than being drawn over the ink. The cloud has
/// had position and scale divided out, so it no longer belongs anywhere on the
/// pad — showing it in place would suggest a correspondence that normalisation
/// has just finished destroying.
///
/// Rotation is *not* divided out, so the picture here is turned the same way
/// the drawing was. That is canon rule 6 on screen: a reversed sign has to look
/// different from an upright one or it could not invert anything.
fn draw_cloud_preview(
    gizmos: &mut Gizmos<DebugGizmos>,
    window: &Window,
    cloud: &recognizer::Cloud,
) {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    let center = Vec2::new(
        half.x - PREVIEW_MARGIN - PREVIEW_SIZE * 0.5,
        half.y - PREVIEW_MARGIN - PREVIEW_SIZE * 0.5,
    );

    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        Vec2::splat(PREVIEW_SIZE),
        PREVIEW_FRAME,
    );
    // The origin the cloud was centred on. Every cloud's points average to it.
    cross(gizmos, center, PREVIEW_FRAME);

    for p in &cloud.points {
        gizmos.circle_2d(
            Isometry2d::from_translation(center + Vec2::new(p.x, p.y) * PREVIEW_SIZE),
            SAMPLE_DOT,
            RESAMPLE_HELD,
        );
    }
}

/// Mean gap between samples once `ids`' strokes are evenly resampled.
///
/// The number `$P` cares about: two clouds can only be compared by nearest
/// neighbour if their points are laid out at comparable density.
fn even_spacing(points: &[crate::Point], ids: &[u32]) -> f32 {
    let mut total = 0.0;
    let mut counted = 0;

    for run in points.chunk_by(|a, b| a.stroke_id == b.stroke_id) {
        let Some(id) = run.first().map(|p| p.stroke_id) else {
            continue;
        };
        if !ids.contains(&id) {
            continue;
        }
        if stroke::resample(run, stroke::MATCH_POINTS).is_some() {
            total += stroke::path_length(run) / (stroke::MATCH_POINTS - 1) as f32;
            counted += 1;
        }
    }

    if counted == 0 {
        0.0
    } else {
        total / counted as f32
    }
}

/// The points of every stroke in `ids`, in pad order.
fn gesture(points: &[crate::Point], ids: &[u32]) -> Vec<crate::Point> {
    points
        .iter()
        .copied()
        .filter(|p| ids.contains(&p.stroke_id))
        .collect()
}

/// Dots every stroke in `ids` at the positions `$P` will be handed.
///
/// Drawn from the resampler rather than from the captured points on purpose:
/// where these sit and where the ink sits are different things, and seeing the
/// difference is the only way to tell a resampling bug from a matching bug.
fn draw_resampled(
    gizmos: &mut Gizmos<DebugGizmos>,
    points: &[crate::Point],
    ids: &[u32],
    color: Color,
) {
    for run in points.chunk_by(|a, b| a.stroke_id == b.stroke_id) {
        let Some(id) = run.first().map(|p| p.stroke_id) else {
            continue;
        };
        if !ids.contains(&id) {
            continue;
        }
        let Some(even) = stroke::resample(run, stroke::MATCH_POINTS) else {
            continue;
        };
        for sample in even {
            gizmos.circle_2d(
                Isometry2d::from_translation(Vec2::new(sample.x, sample.y)),
                SAMPLE_DOT,
                color,
            );
        }
    }
}

/// A plain-English name for what the pen actually did.
///
/// Reads the two turning numbers against coverage. `turns` is how far round
/// the ink went; `spanned` is how much of the circle it reached. For any
/// simple arc those agree — the ways they can disagree are the shapes that
/// pass a fit and a coverage test while being nothing like a ring.
fn shape_of(ring: &assembly::RingCandidate) -> &'static str {
    let w = ring.winding;
    let spanned = ring.coverage.spanned;

    if ring.is_simple(RULES.simple_tolerance) {
        return if spanned > 0.98 {
            "one clean loop"
        } else {
            "one clean arc"
        };
    }
    if w.backtrack() > 0.25 && w.turns < 0.35 {
        // Travelled a long way and came back with nothing to show for it.
        return "figure-eight — lobes cancel";
    }
    if w.turns > spanned + 0.5 {
        return "drawn round more than once";
    }
    if w.backtrack() > RULES.simple_tolerance {
        return "doubles back on itself";
    }
    "tangled"
}

/// Everything known about one ring, in full.
///
/// The corner readout is a summary across every ring; this is one ring in
/// depth. Hover a ring's edge to switch to it.
fn update_inspector(
    pad: Res<InkPad>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut panel: Single<&mut Text2d, With<InspectorLine>>,
) {
    let rings = circle_search(&pad);
    let (camera, camera_transform) = *camera;
    let cursor = cursor_world(&window, camera, camera_transform);

    let Some((slot, by_hover)) = inspected(&rings, cursor) else {
        panel.0 = "── no ring ──\ndraw a loop".to_string();
        return;
    };

    let ring = &rings[slot];
    let fit = ring.fit;
    let quality = fit.quality();
    let circumference = std::f32::consts::TAU * fit.radius;

    // Where the fitter shifts its origin to. On a full ring it sits on the
    // centre; on an arc it slides toward the ink, and that offset is the bias
    // Taubin corrects for.
    let member: Vec<Vec2> = pad
        .points
        .iter()
        .filter(|p| ring.strokes.contains(&p.stroke_id))
        .map(|p| Vec2::new(p.x, p.y))
        .collect();
    let centroid_off = if member.is_empty() {
        0.0
    } else {
        let centroid = member.iter().fold(Vec2::ZERO, |sum, p| sum + *p) / member.len() as f32;
        centroid.distance(Vec2::new(fit.center.x, fit.center.y))
    };

    let filled = (quality * QUALITY_CELLS as f32).round() as usize;
    let bar: String = (0..QUALITY_CELLS)
        .map(|i| if i < filled { '█' } else { '░' })
        .collect();

    // Canon rule 2 gates on closure, and the Spells page gates on circularity
    // as well — a ring that is not round enough fizzles rather than fires.
    let held = ring.contents(&pad.points, ON_RING_TOLERANCE);
    let fill = if fit.radius > 0.0 {
        held.extent / fit.radius
    } else {
        0.0
    };

    // The verdict is core's call, not the overlay's — deciding what magic
    // means is never a shell's job (§4.2). All the shell adds is wording.
    let verdict = match ring.activation(&RULES) {
        assembly::Activation::Malformed => format!("NOT A RING — {}", shape_of(ring)),
        assembly::Activation::Armed => {
            format!("ARMED — {} end(s) to join", ring.open_ends.len())
        }
        assembly::Activation::Fleeting => format!(
            "FLEETING — not circular enough (needs q {:.2})",
            RULES.min_quality
        ),
        assembly::Activation::Active => "ACTIVE — circuit closed".to_string(),
    };

    panel.0 = format!(
        "── ring {slot}/{} {:?} ── {}\n\
         geometry   centre {:.0}, {:.0}      radius {:.1}\n\
         \x20          circumference {circumference:.0}px   area {:.0}px²\n\
         ink        {} stroke(s)   {} pts   drawn {:.0}px   ×{:.2} of the ring\n\
         fit        rms {:.2}px   worst {:.2}px   trimmed {}   centroid off {centroid_off:.1}px\n\
         coverage   spanned {:.1}%   gap {:.0}px / {:.0}°   facing {:.0}°\n\
         turning    {:.2} turns   sweep {:.2}   backtrack {:.2}   {}\n\
         closure    {}   loose ends {}\n\
         holds      inside {:?}   touching {:?}   ignored {:?}\n\
         resample   {} pts/stroke   ring @ {:.1}px   held @ {}\n\
         cloud      {}   rotation kept — canon rule 6\n\
         \x20          {} pts   reach {:.0}px   ×{fill:.2} of the ring   element: ?\n\
         quality    {quality:.3}  {bar}\n\
         canon      r{:.0} → strength    neat {quality:.2} → duration\n\
         verdict    {verdict}",
        rings.len(),
        ring.strokes,
        if by_hover { "hover" } else { "first" },
        fit.center.x,
        fit.center.y,
        fit.radius,
        std::f32::consts::PI * fit.radius * fit.radius,
        ring.strokes.len(),
        ring.points,
        ring.ink_length,
        ring.ink_length / circumference.max(1.0),
        fit.rms,
        fit.max_miss,
        fit.trimmed,
        ring.coverage.spanned * 100.0,
        ring.coverage.gap_length,
        ring.coverage.gap.to_degrees(),
        ring.coverage.gap_heading.to_degrees(),
        ring.winding.turns,
        ring.winding.sweep,
        ring.winding.backtrack(),
        shape_of(ring),
        if ring.closed { "CLOSED" } else { "OPEN" },
        ring.open_ends.len(),
        held.inside,
        held.touching,
        held.outside,
        stroke::MATCH_POINTS,
        even_spacing(&pad.points, &ring.strokes),
        match held.inside.len() + held.touching.len() {
            0 => "—".to_string(),
            _ => {
                let mut ids = held.inside.clone();
                ids.extend(held.touching.iter().copied());
                format!("{:.1}px", even_spacing(&pad.points, &ids))
            }
        },
        {
            let mut ids = held.inside.clone();
            ids.extend(held.touching.iter().copied());
            if ids.is_empty() {
                ids = ring.strokes.clone();
            }
            match recognizer::normalize(&gesture(&pad.points, &ids), stroke::MATCH_POINTS) {
                Some(c) => format!(
                    "{} pts   scale {:.0}px divided out   at {:.0},{:.0}",
                    c.points.len(),
                    c.scale,
                    c.origin.x,
                    c.origin.y
                ),
                None => "— nothing to normalise".to_string(),
            }
        },
        held.points,
        held.extent,
        fit.radius,
    );
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
    taps: Res<TapCounter>,
    time: Res<Time>,
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

    // A tap run in progress, so a swap that did not fire can be told apart from
    // one that was never started.
    let tap_run = match taps.remaining(time.elapsed_secs()) {
        Some(left) => format!("F {}/{} {left:.1}s", taps.count(), taps.needed()),
        None => format!("F ×{} swaps paper", taps.needed()),
    };

    line.0 = format!(
        "focus field {}  events {focus_by_event}   keyev {}  last {}\nheld [{}]  mouse [{}]  F1 hides this  {tap_run}",
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
    shape: Res<PaperShape>,
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

    // The sheet's edge, traced over its own fill. Read back from the transform
    // rather than recomputed, so a fit that disagrees with the pen's boundary
    // shows as a visible mismatch instead of quietly agreeing with the bug.
    if let Some(paper) = paper {
        let at = paper.translation.truncate();

        match *shape {
            PaperShape::Disc => {
                gizmos.circle_2d(Isometry2d::from_translation(at), paper.scale.x, GUIDE);
            }
            PaperShape::Full => {
                gizmos.rect_2d(
                    Isometry2d::from_translation(at),
                    paper.scale.truncate(),
                    GUIDE,
                );
            }
        }

        cross(&mut gizmos, at, GUIDE);
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

/// Captions every fitted ring in place, beside the ring itself.
///
/// A pad can hold several rings at once and they are not interchangeable: one
/// may be closed and firing while the one nested inside it is still armed.
/// Reading that off a list in the corner means counting rings and hoping the
/// order matches — so each caption sits on its own ring instead.
///
/// Entities are pooled by index rather than respawned. Spawning and despawning
/// text every frame churns the archetype and makes the labels flicker.
fn update_ring_labels(
    mut commands: Commands,
    pad: Res<InkPad>,
    mut labels: Query<(
        Entity,
        &RingLabel,
        &mut Text2d,
        &mut TextColor,
        &mut Transform,
    )>,
) {
    let rings = circle_search(&pad);

    let mut pool: Vec<(Entity, usize)> = labels
        .iter()
        .map(|(entity, slot, ..)| (entity, slot.0))
        .collect();
    pool.sort_by_key(|(_, slot)| *slot);

    for (entity, _) in pool.iter().skip(rings.len()) {
        commands.entity(*entity).despawn();
    }
    for slot in pool.len()..rings.len() {
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font_size: FontSize::Px(LABEL_SIZE),
                ..default()
            },
            TextColor(GAP_OPEN),
            TextLayout::justify(Justify::Center),
            // Above the paper and the ink, so a caption is never buried in a
            // dense drawing.
            Transform::from_xyz(0.0, 0.0, 1.0),
            RingLabel(slot),
        ));
    }

    for (_, slot, mut text, mut color, mut transform) in &mut labels {
        let Some(ring) = rings.get(slot.0) else {
            continue;
        };

        let activation = ring.activation(&RULES);
        let state = match activation {
            assembly::Activation::Malformed => shape_of(ring).to_string(),
            assembly::Activation::Armed => {
                format!("ARMED — {} loose end(s)", ring.open_ends.len())
            }
            assembly::Activation::Fleeting => "FLEETING — too rough".to_string(),
            assembly::Activation::Active => "ACTIVE — circuit closed".to_string(),
        };

        // Member strokes are printed because a ring made of several strokes is
        // the normal case, and knowing which ones it swallowed is the whole
        // reason a closing line no longer looks like a ring of its own.
        text.0 = format!(
            "{:?} · r {:.0} · q {:.2}\n{state}",
            ring.strokes,
            ring.fit.radius,
            ring.fit.quality()
        );
        color.0 = activation_color(activation);

        // An armed ring puts its caption at the widest hole, which is where
        // the eye wants to go. A closed ring has no meaningful gap direction —
        // its widest gap is just the sampling step and wanders frame to frame
        // — so those sit at the top, where nested rings stack tidily.
        let angle = if ring.closed {
            std::f32::consts::FRAC_PI_2
        } else {
            ring.coverage.gap_heading
        };
        let out = Vec2::from_angle(angle) * (ring.fit.radius + LABEL_OFFSET);
        transform.translation.x = ring.fit.center.x + out.x;
        transform.translation.y = ring.fit.center.y + out.y;
    }
}

/// Drops every ring caption while the overlay is hidden.
fn clear_ring_labels(mut commands: Commands, labels: Query<Entity, With<RingLabel>>) {
    for entity in &labels {
        commands.entity(entity).despawn();
    }
}

/// Traces the hole in a ring along the fitted circle, with a spoke at each end.
///
/// Amber for open, green for closed — the two states canon rule 2 turns on, so
/// they get the strongest colour difference in the overlay. Drawn as segments
/// rather than one arc call so the ends land exactly on the gap's edges.
fn draw_gap(
    gizmos: &mut Gizmos<DebugGizmos>,
    center: Vec2,
    radius: f32,
    candidate: &assembly::RingCandidate,
) {
    // Coloured by activation, so the arc says the same thing as the caption
    // beside it and the panel above it.
    let color = activation_color(candidate.activation(&RULES));

    let c = &candidate.coverage;
    let from = c.gap_heading - c.gap * 0.5;
    let on_ring = |angle: f32| center + Vec2::from_angle(angle) * radius;

    // Spokes first: on a nearly-closed ring the arc is a couple of pixels long
    // and these are the only part still visible.
    gizmos.line_2d(center, on_ring(from), color);
    gizmos.line_2d(center, on_ring(from + c.gap), color);

    for step in 0..GAP_RESOLUTION {
        let a = from + c.gap * (step as f32 / GAP_RESOLUTION as f32);
        let b = from + c.gap * ((step + 1) as f32 / GAP_RESOLUTION as f32);
        gizmos.line_2d(on_ring(a), on_ring(b), color);
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
    shape: Res<PaperShape>,
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
        // The disc mesh has radius 1 and the rectangle is 1×1, so scale reads as
        // radius for one and full size for the other. Printed with the units it
        // actually has rather than a single number that means two things.
        Some(paper) => match *shape {
            PaperShape::Disc => format!(
                "DISC r {:.0}  desk {:.0}",
                paper.scale.x,
                half.min_element() - paper.scale.x
            ),
            PaperShape::Full => {
                format!("FULL {:.0}×{:.0}", paper.scale.x, paper.scale.y)
            }
        },
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

/// What the ring finder made of the pad.
///
/// Reports **rings**, not strokes. A ring is often several strokes — an arc
/// plus the line that closes it, or two halves of a split seal — so a
/// per-stroke readout would call the closing line a ring of its own and leave
/// the ring it closed permanently open.
fn update_fit_line(pad: Res<InkPad>, mut line: Single<&mut Text2d, With<FitLine>>) {
    let strokes = pad
        .points
        .chunk_by(|a, b| a.stroke_id == b.stroke_id)
        .count();
    let rings = circle_search(&pad);

    let (head, state) = match rings.first() {
        Some(r) => (
            format!(
                "ring 1/{}   strokes {:?}   c {:.0},{:.0}   r {:.1}   rms {:.2}px   q {:.3}   trim {}   dev ×{DEVIATION_GAIN:.0}",
                rings.len(),
                r.strokes,
                r.fit.center.x,
                r.fit.center.y,
                r.fit.radius,
                r.fit.rms,
                r.fit.quality(),
                r.fit.trimmed,
            ),
            format!(
                "gap {:.0}px ({:.0}°)   spanned {:.1}%   turns {:.2}   loose ends {}   {}",
                r.coverage.gap_length,
                r.coverage.gap.to_degrees(),
                r.coverage.spanned * 100.0,
                r.winding.turns,
                r.open_ends.len(),
                // Rule 2: only a complete ring fires. A gap left on purpose is
                // a prepared spell, not a mistake, so open reads as ARMED.
                match r.activation(&RULES) {
                    assembly::Activation::Malformed => "NOT A RING",
                    assembly::Activation::Armed => "ARMED",
                    assembly::Activation::Fleeting => "FLEETING",
                    assembly::Activation::Active => "ACTIVE",
                },
            ),
        ),
        None => (
            "no rings — ink so far names no circle".to_string(),
            "—".to_string(),
        ),
    };

    let listed: Vec<String> = rings
        .iter()
        .take(STROKES_SHOWN)
        .map(|r| {
            format!(
                "r{:.0} q{:.2} {:?} {}",
                r.fit.radius,
                r.fit.quality(),
                r.strokes,
                if r.closed { "○" } else { "◜" }
            )
        })
        .collect();

    line.0 = format!(
        "{head}\n{state}\nrings {} from {strokes} strokes   [{}]",
        rings.len(),
        listed.join("] ["),
    );
}

/// The pad's rings, on this frame's ink.
///
/// Recomputed per system rather than cached in a resource: it costs
/// microseconds, and a cache would be a second copy of the truth that can go
/// stale between the readout and the gizmos.
fn circle_search(pad: &InkPad) -> Vec<assembly::RingCandidate> {
    assembly::find_rings(
        &pad.points,
        &assembly::RingSearch {
            // Ink this far off the circle still counts as on it, which is what
            // lets a separately drawn closing line join the ring it closes.
            on_ring: ON_RING_TOLERANCE,
            join: CLOSURE_TOLERANCE,
            ..default()
        },
    )
}

/// Draws the fit itself: the circle, its centre, and where the ink misses it.
///
/// Every stroke gets its circle. Only the newest gets the band, the centroid
/// mark and the whiskers — all of them at once is unreadable, and the newest
/// stroke is the one being judged.
///
/// Refits on every frame rather than caching. It costs a few microseconds
/// against a 16 700µs budget, and a cache is a second copy of the truth.
fn draw_fits(
    pad: Res<InkPad>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut gizmos: Gizmos<DebugGizmos>,
) {
    let rings = circle_search(&pad);
    let (camera, camera_transform) = *camera;
    let focus = inspected(&rings, cursor_world(&window, camera, camera_transform)).map(|(s, _)| s);

    for (index, candidate) in rings.iter().enumerate() {
        let fit = candidate.fit;
        let center = Vec2::new(fit.center.x, fit.center.y);
        let at_center = Isometry2d::from_translation(center);

        gizmos
            .circle_2d(at_center, fit.radius, FIT_RING)
            .resolution(FIT_RESOLUTION);

        // Every ring shows where its widest hole is, not just the first —
        // several rings on one pad is a normal seal, not an edge case.
        draw_gap(&mut gizmos, center, fit.radius, candidate);

        // Ends that meet nothing. These are the closure test made visible: a
        // dot of ink on each one finishes the spell.
        for end in &candidate.open_ends {
            gizmos.circle_2d(
                Isometry2d::from_translation(Vec2::new(end.x, end.y)),
                TICK * 1.4,
                GAP_OPEN,
            );
        }

        if focus != Some(index) {
            continue;
        }

        // A collar just outside the ring the panel is describing, so there is
        // never any doubt which one those numbers belong to.
        gizmos
            .circle_2d(at_center, fit.radius + LABEL_OFFSET * 0.4, INSPECT_MARK)
            .resolution(FIT_RESOLUTION);

        // How far the enclosed ink reaches. Against the ring this is the
        // ratio the wiki ties to a spell's intensity, so it is worth seeing
        // rather than reading.
        let held = candidate.contents(&pad.points, ON_RING_TOLERANCE);
        if held.extent > 0.0 {
            gizmos
                .circle_2d(at_center, held.extent, CONTENTS_MARK)
                .resolution(FIT_RESOLUTION);
        }

        draw_resampled(&mut gizmos, &pad.points, &candidate.strokes, RESAMPLE_RING);
        let mut held_ids = held.inside.clone();
        held_ids.extend(held.touching.iter().copied());
        draw_resampled(&mut gizmos, &pad.points, &held_ids, RESAMPLE_HELD);

        // What the ring encloses, as the recognizer will see it. Falls back to
        // the ring itself when nothing is inside yet, so the box is never
        // blank while there is ink to show.
        let subject = if held_ids.is_empty() {
            gesture(&pad.points, &candidate.strokes)
        } else {
            gesture(&pad.points, &held_ids)
        };
        if let Some(cloud) = recognizer::normalize(&subject, stroke::MATCH_POINTS) {
            draw_cloud_preview(&mut gizmos, &window, &cloud);
        }

        // Detail for the inspected ring only. All of it at once is a smear.
        let member: Vec<crate::Point> = pad
            .points
            .iter()
            .copied()
            .filter(|p| candidate.strokes.contains(&p.stroke_id))
            .collect();

        // The fit before trimming, faint. Hidden under the solid one unless an
        // outlier was pulling it somewhere.
        if fit.trimmed > 0
            && let Some(raw) = circle::fit(&member)
        {
            gizmos
                .circle_2d(
                    Isometry2d::from_translation(Vec2::new(raw.center.x, raw.center.y)),
                    raw.radius,
                    FIT_RAW,
                )
                .resolution(FIT_RESOLUTION);
        }

        // The band the ink sits in on average, magnified so a good fit is still
        // visible. Inner edge clamped: a ring messier than its own radius would
        // otherwise ask for a negative circle.
        let spread = fit.rms * DEVIATION_GAIN;
        gizmos
            .circle_2d(at_center, fit.radius + spread, FIT_BAND)
            .resolution(FIT_RESOLUTION);
        gizmos
            .circle_2d(at_center, (fit.radius - spread).max(0.0), FIT_BAND)
            .resolution(FIT_RESOLUTION);

        cross(&mut gizmos, center, FIT_RING);
        gizmos.circle_2d(at_center, TICK * 0.4, FIT_RING);

        // Mean of the ring's points — the origin the fitter shifts to before
        // it does anything else. Sits on the centre for a full ring, drifts
        // off it for an arc.
        if !member.is_empty() {
            let centroid = member
                .iter()
                .fold(Vec2::ZERO, |sum, p| sum + Vec2::new(p.x, p.y))
                / member.len() as f32;
            cross(&mut gizmos, centroid, CENTROID_MARK);
        }

        for p in member.iter().step_by(WHISKER_STRIDE) {
            let at = Vec2::new(p.x, p.y);
            let out = (at - center).normalize_or_zero();
            if out == Vec2::ZERO {
                continue;
            }
            // Anchored on the fitted circle, not on the ink, so every whisker
            // starts from the same reference and their lengths compare.
            let on_ring = center + out * fit.radius;
            let miss = at.distance(center) - fit.radius;
            gizmos.line_2d(on_ring, on_ring + out * miss * DEVIATION_GAIN, DEVIATION);
        }
    }
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
