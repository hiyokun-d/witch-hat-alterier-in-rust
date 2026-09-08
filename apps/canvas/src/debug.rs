//! THIS IS CLAUDE WORKS AND I JUST ASKED FOR IT!
//! On-screen debug overlay. Scaffolding, and deliberately quarantined.
//!
//! Nothing outside this file knows the overlay exists. To delete it: remove
//! this file, the `mod debug;` line, and the `add_plugins` line in `main.rs`.
//! Nothing else changes — the overlay only ever reads.
//!
//! It reads `ui::ToolState` when the toolbar is present, so the "debug" button
//! and F1 are the same switch rather than two that can disagree. The
//! dependency runs this way round on purpose: every use of it is an
//! `Option<Res<..>>`, so deleting `ui/` costs two parameters here and nothing
//! else, while deleting *this* file still costs nothing anywhere.
//!
//! It goes away once ink rendering shows the same information implicitly.

// Bevy systems declare their dependencies as parameters, so a readout that
// looks at the pad, the reading, the panel's flags, the world and the
// catalogue has six before it has said anything. Clippy's limit of seven is
// aimed at ordinary functions, where a long list means a muddled abstraction;
// here it means a system that reads five things, which is what a readout is.
//
// Module-level rather than six copies of the same `allow`: the reason is the
// same in every case, and stating it once is stating it honestly.
#![allow(clippy::too_many_arguments)]

use bevy::gizmos::config::GizmoConfigStore;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::window::WindowFocused;

use magic_core::{assembly, circle, recognizer, stroke};

use crate::shortcuts::TapCounter;
use crate::ui::ToolState;
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

/// The template's cloud in the preview box. The drawing is teal, what it is
/// being compared against is gold — one picture, two clouds, so a bad match is
/// something you can see rather than only read.
const TEMPLATE_MARK: Color = Color::srgba(0.92, 0.74, 0.28, 0.85);

/// The grid the parcels live in. Faint - it is a ruler, not content.
const FIELD_GRID: Color = Color::srgba(0.35, 0.45, 0.55, 0.16);

/// A cell holding more than a breath of air.
const FIELD_FULL: Color = Color::srgba(0.30, 0.60, 0.75, 0.45);

/// Density below which a cell is not worth shading.
///
/// The room is full of ambient air now, so "has any mass in it" is true of
/// every cell on the board — and shading all of them turned the pad into a
/// grid of boxes with the drawing lost inside it. Ambient air is one parcel of
/// half a unit per cell, so anything above this is something that *arrived*.
const DENSE_ENOUGH: f32 = 1.2;

/// Density at which a cell is drawn at full strength.
const PACKED_AT: f32 = 12.0;

/// Degrees above ambient at which a parcel is drawn white-hot.
///
/// **Ours**, and only a colour ramp — nothing reads it back.
const HOT_AT: f32 = 300.0;

/// Smallest and largest a parcel is drawn, in pixels.
const PARCEL_DOT: (f32, f32) = (5.0, 22.0);

/// Parcels sit above the paper and below the panel.
const PARCEL_Z: f32 = 5.0;

/// What each substance looks like.
///
/// A `match` on a name, in a *shell* file, and that is the right place for it:
/// what colour water is on this screen is not a fact about magic (§4.2), and
/// `sim` must never learn one. An unknown substance gets a neutral grey rather
/// than a panic or a guess, so adding one to `reactions.ron` shows up as
/// something visible and plain rather than as nothing at all.
use crate::particles::substance_color;

/// One parcel's dot on screen, carrying its slot in the pool.
///
/// A sprite rather than a gizmo circle, and that was the whole bug: gizmos draw
/// **outlines** at one pixel, so a field of parcels was a field of hairline
/// rings on parchment — technically drawn, practically invisible. A sprite is
/// filled, and filled is what reads as a substance.
///
/// Pooled and reused, the same way `RingLabel` is: parcels come and go every
/// tick and spawning an entity per parcel per frame is not a thing to do.
#[derive(Component)]
struct ParcelDot(usize);

/// How much of the right edge the tool panel claims while it is open.
///
/// Mirrors `ui::bar`'s own `EDGE + PANEL_W`, and duplicated on purpose. The
/// panel is twenty-odd rows tall, so when it is open it owns the whole right
/// edge and anything drawn there is drawn underneath it. §0 keeps this file
/// deletable, and an accessor added to `bar.rs` purely so the overlay could
/// read this would be a change to real code that exists only for the overlay —
/// which §0 says is the wrong change. If the panel is resized and this drifts,
/// the cost is a preview box twenty pixels off, never a wrong answer.
const PANEL_RESERVE: f32 = 14.0 + 152.0;

/// How many ranked runes the board lists before it stops counting.
const RANKED_SHOWN: usize = 10;

/// A gap this wide between first place and second reads as a confident call.
///
/// **Ours, not canon** (§2.6), and parked here rather than in core for exactly
/// that reason. `recognizer::classify` is deliberately unthresholded — how
/// close is close enough is a question about magic, and it belongs to whoever
/// compiles a spell. This number only puts a word beside a number on screen.
const CONFIDENT_MARGIN: f32 = 0.05;

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
use crate::reading::{ON_RING_TOLERANCE, RULES, Reading};

/// Segments used to draw the gap arc.
const GAP_RESOLUTION: usize = 24;

/// The ring being inspected, highlighted so the panel and the pad agree.
const INSPECT_MARK: Color = Color::srgba(0.55, 0.30, 0.75, 0.85);
/// How near a ring's edge the cursor must come to inspect it.
const INSPECT_REACH: f32 = 60.0;

/// Cells in the quality bar.
const QUALITY_CELLS: usize = 10;

/// How far outside a ring its own label floats.
const LABEL_OFFSET: f32 = 24.0;
/// Labels are smaller than the corner readouts — there can be a dozen of them
/// on the pad at once, and they sit on top of the drawing.
const LABEL_SIZE: f32 = 17.0;

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
#[derive(Resource, Default)]
struct OverlayVisible(bool);

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
            .init_resource::<Lore>()
            .add_systems(
                Update,
                (
                    probe_input,
                    sample_frame_time,
                    toggle_overlay,
                    apply_visibility,
                    // Outside the visible group on purpose: a rune recorded
                    // while the overlay is hidden should already be loaded when
                    // it comes back, not one frame behind.
                )
                    .chain(),
            );

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
                    place_inspector,
                    update_inspector,
                    place_matches,
                    update_matches,
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
        // The ring caption and the parcels are NOT debug. "What will this seal
        // do" and "what is in the world" are the two questions a person has
        // while drawing, and hiding them behind the measurement overlay meant
        // turning the overlay off left the app unusable rather than clean.
        // They ride their own toggles — `spell` and `world` — and still vanish
        // with this file, which is what §0 asks.
        app.add_systems(
            Update,
            (
                update_ring_labels,
                update_loose_labels,
                draw_focus,
                draw_parcels,
                draw_world,
            )
                .after(crate::capture_stroke)
                .run_if(spell_or_world),
        );
        app.add_systems(
            Update,
            (clear_ring_labels, clear_loose_labels, clear_parcels).run_if(not(spell_or_world)),
        );

        // The tracing board rides its own toggle too, and for the same reason
        // the ring caption does: tracing runes is a *sitting*, and asking
        // somebody to keep the whole measurement overlay up to see whether
        // their sample was any good is asking them to read six blocks to find
        // one number.
        app.add_systems(
            Update,
            (place_trace, update_trace)
                .chain()
                .after(crate::capture_stroke)
                .run_if(tracing_showing),
        );
        app.add_systems(Update, clear_trace.run_if(not(tracing_showing)));
    }
}

/// Run condition: is the tracing board showing?
fn tracing_showing(tools: Option<Res<ToolState>>) -> bool {
    tools.is_none_or(|state| state.trace)
}

/// Blanks the tracing board when it is switched off.
fn clear_trace(mut panel: Single<&mut Text2d, With<TraceLine>>) {
    panel.0.clear();
}

/// Hairlines for the overlay's own group. One pixel, no joints — these are
/// rulers, and a ruler with weight reads as content.
fn thin_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DebugGizmos>();
    config.line.width = 1.0;
}

/// Run condition: is anything that is *not* a measurement showing?
///
/// The ring caption and the world are gated on their own toggles rather than on
/// F1, so a person who wants the numbers gone can still see what their seal is
/// about to do.
fn spell_or_world(tools: Option<Res<ToolState>>) -> bool {
    tools.is_none_or(|state| state.spell || state.sim)
}

/// Run condition: is the overlay showing?
///
/// The toolbar owns the answer when it exists, so the button and F1 cannot
/// drift apart. Without it, this file's own flag is the truth.
fn overlay_visible(visible: Res<OverlayVisible>, tools: Option<Res<ToolState>>) -> bool {
    tools.map_or(visible.0, |tools| tools.debug_overlay)
}

/// Every piece of text the overlay owns.
///
/// The inspector is pinned by its own system rather than the `lift` stack, so
/// it carries no [`OverlayLine`] and has to be named separately.
type AnyReadout = Or<(With<OverlayLine>, With<InspectorLine>, With<MatchLine>)>;

/// F1 shows and hides everything this file draws.
///
/// The text entities get their `Visibility` flipped; the gizmos stop because
/// their systems stop running. Two mechanisms because gizmos are immediate mode
/// and have no entity to hide.
fn toggle_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<OverlayVisible>,
    tools: Option<ResMut<ToolState>>,
) {
    if !keys.just_pressed(KeyCode::F1) {
        return;
    }

    match tools {
        Some(mut tools) => tools.debug_overlay = !tools.debug_overlay,
        None => visible.0 = !visible.0,
    }
}

/// Shows and hides the text, from whichever flag is in charge.
///
/// Its own system rather than part of [`toggle_overlay`], because the toolbar
/// can change the answer too and nothing here sees that keypress. Runs every
/// frame and unconditionally — a system skipped while hidden could never
/// un-hide itself.
fn apply_visibility(
    visible: Res<OverlayVisible>,
    tools: Option<Res<ToolState>>,
    mut lines: Query<&mut Visibility, AnyReadout>,
) {
    let on = tools.map_or(visible.0, |tools| tools.debug_overlay);
    for mut visibility in &mut lines {
        *visibility = if on {
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

/// Marks the recogniser board, pinned under the cloud preview.
#[derive(Component)]
struct MatchLine;

/// Marks the tracing board, pinned to the bottom-right.
///
/// Its own block rather than more rows on the recogniser board, because the two
/// answer different questions. The board asks *what did the engine make of this
/// ring*; tracing asks *is the sample I am about to save any good, and how many
/// have I got*. Reading past one to reach the other is what made the corner
/// unusable, and they are on separate toggles for the same reason.
#[derive(Component)]
struct TraceLine;

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
        Text2d::new("cursor: -"),
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
        Text2d::new("ink: -"),
        font.clone(),
        TextColor(Color::srgb(0.88, 0.72, 0.44)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 0.0 },
        InkLine,
    ));

    commands.spawn((
        Text2d::new("input: -"),
        font.clone(),
        TextColor(Color::srgb(0.72, 0.58, 0.92)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 4.0 },
        InputLine,
    ));

    commands.spawn((
        Text2d::new("layout: -"),
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
        Text2d::new("frame: -"),
        font.clone(),
        TextColor(Color::srgb(0.72, 0.70, 0.64)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_LEFT,
        OverlayLine { lift: 3.0 },
        FrameLine,
    ));

    commands.spawn((
        Text2d::new("strokes: -"),
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
        Text2d::new("fit: -"),
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

    // Directly under the cloud preview, in the same corner. The picture and
    // the numbers that explain it should not be at opposite ends of the window.
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(13.0),
            ..default()
        },
        TextColor(Color::srgb(0.90, 0.78, 0.46)),
        TextLayout::justify(Justify::Left),
        // Right-anchored so the block's edge stays put as rows change width.
        Anchor::TOP_RIGHT,
        MatchLine,
    ));

    // The opposite corner from the recogniser board, on purpose: while tracing
    // you are looking at the shape under your hand, not at the ranking.
    commands.spawn((
        Text2d::new(""),
        TextFont {
            font_size: FontSize::Px(14.0),
            ..default()
        },
        TextColor(Color::srgb(0.55, 0.88, 0.62)),
        TextLayout::justify(Justify::Left),
        Anchor::BOTTOM_RIGHT,
        TraceLine,
    ));
}

/// How close a distance is, as a percentage a person can read.
///
/// `100%` is an exact match and `0%` is [`naming::NEAR_ENOUGH`], the distance at
/// which the compiler stops believing it. So the number is not a probability and
/// does not pretend to be one — it is *where this drawing sits between perfect
/// and rejected*, which is the question somebody tracing a rune is actually
/// asking. Naming the threshold beside it is what keeps that honest.
fn closeness(distance: f32) -> f32 {
    (1.0 - distance / magic_core::naming::NEAR_ENOUGH).clamp(0.0, 1.0) * 100.0
}

/// The ink `record` would save right now: everything that is not a ring.
///
/// The same rule `ui::record::gestures` applies, and it has to be the same or
/// the percentage would describe a different drawing from the one that gets
/// written. Rings are excluded because a ring is the activator, not a rune —
/// but only ink that really is a ring, so water's teardrops and light's diamond
/// survive being part of their own sigil.
fn tracing_ink(pad: &InkPad, reading: &Reading) -> Vec<crate::Point> {
    let rings = reading.ring_strokes();
    pad.points
        .iter()
        .copied()
        .filter(|point| !rings.contains(&point.stroke_id))
        .collect()
}

/// Pins the tracing board to the bottom-right, above the hint line.
fn place_trace(
    window: Single<&Window>,
    tools: Option<Res<ToolState>>,
    mut panel: Single<&mut Transform, With<TraceLine>>,
) {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    panel.translation.x = half.x - MARGIN - panel_reserve(tools.as_deref());
    // Clear of the hint line, which runs along the very bottom at 16px.
    panel.translation.y = -half.y + MARGIN + 34.0;
}

/// The tracing board: what `record` will save, and how good it is.
///
/// Decides nothing (§4.2). It reads the pad, asks the recognizer, and prints —
/// the one judgement on screen is [`closeness`], which names its own threshold.
fn update_trace(
    pad: Res<InkPad>,
    tools: Option<Res<ToolState>>,
    lore: Res<Lore>,
    reading: Res<Reading>,
    mut panel: Single<&mut Text2d, With<TraceLine>>,
) {
    if !tools.as_deref().is_none_or(|state| state.trace) {
        panel.0.clear();
        return;
    }

    let all: Vec<(&'static str, &magic_core::Recorded)> = reading.catalogued().collect();
    let catalog = lore.0.as_ref();

    // What the next recording will be filed as.
    let target = tools
        .as_deref()
        .and_then(|state| state.tracing)
        .zip(catalog)
        .and_then(|(at, catalog)| {
            let list = crate::sim::traceable(catalog);
            list.get(at)
                .map(|(name, kind)| (name.clone(), *kind, at + 1, list.len()))
        });

    let mut out = String::from("-- tracing --\n");
    out.push_str(&match &target {
        Some((name, kind, at, of)) => {
            // What the rune you are tracing actually *does*, from the
            // catalogue. Tracing is a long sitting, and which of the
            // thirty-four is worth the next twenty minutes is the question.
            let does = match (*kind == "Sigil", catalog) {
                (true, Some(catalog)) => {
                    crate::sim::describe_sigil(catalog, &magic_core::SigilId::from(name.as_str()))
                }
                _ => format!("{name} - a keystone: it shapes what the sigil does"),
            };
            format!(
                "target     {name}  ({} {at}/{of})\n\
                 does       {does}\n",
                kind.to_lowercase()
            )
        }
        None => "target     none - press 'trace >' to choose a rune\n".to_string(),
    });

    // How many samples of each rune are on file. Several samples of one rune is
    // how a hand-drawn shape gets recognised reliably, so the count is the
    // number a person tracing is working *toward* and belongs on screen.
    let mut tally: Vec<(String, usize)> = Vec::new();
    for (_, rune) in &all {
        let name = rune.template.name.clone();
        match tally.iter_mut().find(|(had, _)| *had == name) {
            Some((_, n)) => *n += 1,
            None => tally.push((name, 1)),
        }
    }
    tally.sort();
    out.push_str(&format!(
        "on file    {} sample(s), {} rune(s)\n",
        all.len(),
        tally.len()
    ));
    for chunk in tally.chunks(4) {
        let row: Vec<String> = chunk
            .iter()
            .map(|(name, n)| format!("{name} x{n}"))
            .collect();
        out.push_str(&format!("           {}\n", row.join("  ")));
    }

    let ink = tracing_ink(&pad, &reading);
    if ink.is_empty() {
        panel.0 = plain(&(out + "ink        nothing to save - the pad holds only rings\n"));
        return;
    }
    let strokes = ink.chunk_by(|a, b| a.stroke_id == b.stroke_id).count();
    out.push_str(&format!(
        "ink        {strokes} stroke(s), {} pts - this is what record saves\n",
        ink.len()
    ));

    let Some(cloud) = recognizer::normalize(&ink, stroke::MATCH_POINTS) else {
        panel.0 = plain(&(out + "match      nothing to normalise\n"));
        return;
    };
    if all.is_empty() {
        panel.0 = plain(&(out + "match      no rune has a shape yet - press record\n"));
        return;
    }

    let shapes: Vec<recognizer::Template> =
        all.iter().map(|(_, rune)| rune.template.clone()).collect();
    let ranked = recognizer::rank(&cloud, &shapes);

    for (place, found) in ranked.iter().take(3).enumerate() {
        let rune = all[found.index].1;
        let label = if place == 0 {
            "match     "
        } else {
            "           "
        };
        out.push_str(&format!(
            "{label} {:<12} {:>3.0}%   miss {:.3}\n",
            rune.template.name,
            closeness(found.distance),
            found.distance,
        ));
    }

    // Against the rune you said you were tracing, which is not always the one
    // that won — and when it is not, that is the whole thing worth knowing.
    out.push_str(&match (&target, ranked.first()) {
        (Some((name, _, _, _)), Some(best)) => {
            let mine = ranked
                .iter()
                .find(|found| all[found.index].1.template.name == *name);
            let winner = &all[best.index].1.template.name;
            match mine {
                _ if winner == name => format!(
                    "verdict    reads as {name} at {:.0}% - a good sample\n",
                    closeness(best.distance)
                ),
                Some(found) => format!(
                    "verdict    reads as {winner}, not {name} ({:.0}% vs {:.0}%)\n",
                    closeness(best.distance),
                    closeness(found.distance)
                ),
                None => format!("verdict    reads as {winner} - no {name} on file yet\n"),
            }
        }
        (None, Some(best)) => format!(
            "verdict    reads as {} at {:.0}%\n",
            all[best.index].1.template.name,
            closeness(best.distance)
        ),
        _ => "verdict    nothing to compare against\n".to_string(),
    });
    out.push_str("scale      100% is exact, 0% is the distance the compiler rejects");
    panel.0 = plain(&out);
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

/// Flattens text to what the shipped font can actually draw.
///
/// The default font has no box-drawing, no typographic dashes, no `deg` sign —
/// they all render as an empty box, which is how `bare ring [] this is an
/// explosion` reached the screen. Sanitising *here* rather than in `magic-core`
/// is the same call §4.4 makes about pixels: what glyphs a font has is a fact
/// about this shell, and core is entitled to write `—` in a `Display` impl.
///
/// Applied to whole readouts, so a warning added to core years from now cannot
/// reintroduce the bug.
fn plain(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '\u{2014}' | '\u{2013}' | '\u{2212}' | '\u{2500}' => '-',
            '\u{00b7}' => '|',
            '\u{00d7}' => 'x',
            '\u{2588}' => '#',
            '\u{2591}' => '.',
            other if other.is_ascii() => other,
            // Anything unforeseen becomes a visible marker rather than a box,
            // so the next one of these is reported instead of puzzled over.
            _ => '?',
        })
        .collect()
}

/// What the world is doing, appended under the spell block.
///
/// Reads the simulation and prints it. Decides nothing (§4.2) — every number
/// here is one `magic_core::sim` already computed.
fn world_report(
    world: Option<&crate::sim::Simulation>,
    tools: Option<&ToolState>,
    reading: &Reading,
) -> String {
    if !tools.is_none_or(|state| state.sim) {
        return String::new();
    }
    let Some(world) = world else {
        return String::new();
    };

    let field = &world.sim.field;
    let hottest = field
        .parcels()
        .iter()
        .map(|parcel| parcel.temperature)
        .fold(f32::NEG_INFINITY, f32::max);

    format!(
        "\n-- world --\n         naming     {}\n         pour       {}\n         state      {}   tick {}   t {:.2}s\n         grid       {}x{} cells of {:.0}px\n         parcels    {}   mass {:.2}   heat {:.0}\n         props      {}\n         motion     momentum {:.0}, {:.0}   hottest {}\n         holds      {}\n         rules      {} loaded   over {}\n         reacting   {}\n         summoning  {}\n         last cast  {}",
        // What the `name` section has the pad's seals set to. Worth a line of
        // its own: with `templates.ron` empty this is the *only* thing that
        // makes a seal compile to anything but a discharge, so a person who
        // forgets they left it on fire has no other way to find out.
        match world.lore.as_ref() {
            None => "catalogue failed to load".to_string(),
            // No override left to report. The panel's "treat this ring as
            // fire" buttons are gone, so the only thing that can name a mark is
            // the recogniser — which is what `only_traced` is about.
            Some(_) => {
                let traced = reading.traced_sigils();
                format!(
                    "the drawing speaks, over {} shape(s){} | traced sigils: {}",
                    reading.shapes.len(),
                    match tools.is_none_or(|state| state.only_traced) {
                        true => " (traced only)",
                        false => " (built-ins allowed)",
                    },
                    match traced.is_empty() {
                        true => "none yet - press record".to_string(),
                        false => traced.join(", "),
                    },
                )
            }
        },
        match tools.map(|state| state.pour) {
            Some(which) => crate::sim::POURABLE
                .get(which)
                .map(|(name, temperature)| format!("{name} at {temperature:.0} deg"))
                .unwrap_or_else(|| "-".to_string()),
            None => "-".to_string(),
        },
        if world.running { "RUNNING" } else { "paused" },
        world.sim.ticks,
        world.sim.elapsed(),
        field.width(),
        field.height(),
        field.cell_size(),
        field.parcels().len(),
        field.mass(),
        field.heat(),
        // Props are mass the field cannot see, and burning moves mass from one
        // to the other — so watching only `field.mass()` shows a world that
        // invents matter. Both totals, side by side.
        if world.sim.props.is_empty() {
            "none on the paper - press kindle".to_string()
        } else {
            let alight = world.sim.props.iter().filter(|p| p.burning).count();
            let wet = world
                .sim
                .props
                .iter()
                .filter(|p| p.wetness >= world.sim.prop_rules.quench)
                .count();
            let lit = world.sim.props.iter().filter(|p| p.lit > 0.35).count();
            format!(
                "{}   mass {:.2}   {alight} alight   {wet} soaked   {lit} lit",
                world.sim.props.len(),
                world.sim.prop_mass(),
            )
        },
        field.momentum().x,
        field.momentum().y,
        if hottest.is_finite() {
            format!("{hottest:.0}")
        } else {
            "-".to_string()
        },
        // What is actually in the world, by substance. Sorted and totalled, so
        // a reaction turning one into another is legible as it happens.
        {
            let mut tally: Vec<(String, f32)> = Vec::new();
            for parcel in field.parcels() {
                match tally
                    .iter_mut()
                    .find(|(name, _)| name == parcel.substance.as_str())
                {
                    Some((_, mass)) => *mass += parcel.mass,
                    None => tally.push((parcel.substance.as_str().to_string(), parcel.mass)),
                }
            }
            tally.sort_by(|a, b| a.0.cmp(&b.0));
            if tally.is_empty() {
                "nothing".to_string()
            } else {
                tally
                    .iter()
                    .map(|(name, mass)| format!("{name} {mass:.1}"))
                    .collect::<Vec<_>>()
                    .join("   ")
            }
        },
        world.sim.reactions.rules().len(),
        {
            let named = world.sim.reactions.substances();
            if named.is_empty() {
                "nothing".to_string()
            } else {
                named.join(", ")
            }
        },
        {
            let last = &world.sim.last_reaction;
            if last.happened() {
                format!(
                    "{:.2} mass, {:+.0} heat, {}",
                    last.converted,
                    last.heat,
                    last.fired
                        .iter()
                        .map(|shot| format!("{} {:.2}", shot.rule, shot.mass))
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            } else {
                "nothing this tick".to_string()
            }
        },
        // How long each running spell has left. The question a person actually
        // has while watching one is "is this about to stop", and until now the
        // only answer was to keep watching.
        {
            let running = &world.sim.channels;
            if running.is_empty() {
                "nothing running".to_string()
            } else {
                running
                    .iter()
                    .map(|open| {
                        let name = match &open.spell.driver {
                            magic_core::Driver::Sigil(id) => id.as_str().to_string(),
                            magic_core::Driver::Substitute(id) => id.as_str().to_string(),
                            magic_core::Driver::Discharge => "discharge".to_string(),
                        };
                        format!("{name} {:.1}s left", open.left)
                    })
                    .collect::<Vec<_>>()
                    .join("   ")
            }
        },
        world.last.as_deref().unwrap_or("nothing cast yet"),
    )
}

/// Draws the world: the grid, whatever is in each cell, and every parcel.
fn draw_world(
    world: Option<Res<crate::sim::Simulation>>,
    tools: Option<Res<ToolState>>,
    mut gizmos: Gizmos<DebugGizmos>,
) {
    if !tools.is_none_or(|state| state.sim) {
        return;
    }
    let Some(world) = world else {
        return;
    };
    let field = &world.sim.field;

    let (min, max) = field.bounds();
    gizmos.rect_2d(
        Isometry2d::from_translation(Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5)),
        Vec2::new(max.x - min.x, max.y - min.y),
        FIELD_GRID,
    );

    // Only cells holding something. Drawing all 2304 every frame would be a
    // grey wash that says nothing.
    let size = field.cell_size();
    for col in 0..field.width() {
        for row in 0..field.height() {
            let Some(cell) = field.cell_by(col, row) else {
                continue;
            };
            if cell.density < DENSE_ENOUGH {
                continue;
            }
            // Shaded by how packed it is, so a pool reads as a pool rather than
            // as a uniform stencil. Square-rooted because the eye compares
            // brightness on something closer to a curve than a line.
            let packed = (cell.density / PACKED_AT).clamp(0.0, 1.0).sqrt();
            let at = field.cell_center(col, row);
            gizmos.rect_2d(
                Isometry2d::from_translation(Vec2::new(at.x, at.y)),
                Vec2::splat(size * 0.92),
                FIELD_FULL.with_alpha(FIELD_FULL.alpha() * packed),
            );
        }
    }

    // Where each parcel is going. The dots themselves are sprites — see
    // `draw_parcels` — because a gizmo circle is an outline and an outline is
    // not a substance.
    for parcel in field.parcels() {
        let travel = Vec2::new(parcel.velocity.x, parcel.velocity.y) * 0.05;
        if travel.length_squared() > 4.0 {
            let from = Vec2::new(parcel.at.x, parcel.at.y);
            gizmos.line_2d(
                from,
                from + travel,
                parcel_color(parcel, &world.sim.materials),
            );
        }
    }
}

/// A parcel's colour: its substance, brightened as it heats.
///
/// Hue says *what*, brightness says *how hot*. Mixing toward white rather than
/// toward orange keeps the two readable at once — a hot rock and a flame are
/// still different colours, which they would not be on a single cold-to-hot
/// ramp. That ramp was the first version and it made every substance look the
/// same at temperature.
fn parcel_color(parcel: &magic_core::sim::Parcel, materials: &magic_core::sim::Materials) -> Color {
    use magic_core::sim::Phase;

    let material = materials.get(parcel.substance.as_str());
    let heat = ((parcel.temperature - magic_core::sim::AMBIENT) / HOT_AT).clamp(0.0, 1.0);

    // Glow is a property of the substance, heat is a property of this parcel.
    // Both brighten, and keeping them separate is what lets a cold light still
    // read as light and a hot stone still read as stone.
    let lift = (heat * 0.7 + material.glow * 0.3).clamp(0.0, 0.95);
    let color = substance_color(parcel.substance.as_str()).mix(&Color::WHITE, lift);

    // A gas you can see through, a solid you cannot. Without this, steam and
    // stone are the same object in different colours — which was the complaint.
    let alpha = match material.phase {
        Phase::Gas => 0.30 + heat * 0.45,
        Phase::Liquid => 0.90,
        Phase::Solid | Phase::Granular => 1.0,
        Phase::Radiant => 0.75 + heat * 0.25,
    };
    color.with_alpha(alpha)
}

/// Draws every parcel as a filled dot, reusing a pool of sprites.
fn draw_parcels(
    mut commands: Commands,
    world: Option<Res<crate::sim::Simulation>>,
    tools: Option<Res<ToolState>>,
    mut dots: Query<(&ParcelDot, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let showing = tools.is_none_or(|state| state.sim);
    let parcels: &[magic_core::sim::Parcel] = match (&world, showing) {
        (Some(world), true) => world.sim.field.parcels(),
        _ => &[],
    };

    let Some(world) = world.as_ref() else {
        for (_, _, _, mut visible) in &mut dots {
            *visible = Visibility::Hidden;
        }
        return;
    };

    let mut seen = 0usize;
    for (slot, mut sprite, mut transform, mut visible) in &mut dots {
        seen = seen.max(slot.0 + 1);
        match parcels.get(slot.0) {
            Some(parcel) => {
                // Side from mass, square-rooted: the eye compares area, not
                // width, so a parcel twice as heavy should not be twice as wide.
                //
                // Then scaled by phase. A gas occupies far more room per unit
                // mass than a liquid does, which is the difference between a
                // puff of steam and a bead of water — and it is the same fact
                // as the density the simulation is already using.
                let materials = &world.sim.materials;
                let spread = match materials.get(parcel.substance.as_str()).phase {
                    magic_core::sim::Phase::Gas => 2.4,
                    magic_core::sim::Phase::Radiant => 1.8,
                    magic_core::sim::Phase::Liquid => 1.0,
                    _ => 0.85,
                };
                let side =
                    (PARCEL_DOT.0 + parcel.mass.max(0.0).sqrt() * 6.0 * spread).min(PARCEL_DOT.1);
                sprite.color = parcel_color(parcel, materials);
                sprite.custom_size = Some(Vec2::splat(side));
                transform.translation = Vec3::new(parcel.at.x, parcel.at.y, PARCEL_Z);
                *visible = Visibility::Inherited;
            }
            // Hidden rather than despawned: the count swings every tick and a
            // pool that churns entities is a pool that stutters.
            None => *visible = Visibility::Hidden,
        }
    }

    for slot in seen..parcels.len() {
        commands.spawn((
            Sprite::default(),
            Transform::from_xyz(0.0, 0.0, PARCEL_Z),
            ParcelDot(slot),
        ));
    }
}

/// Drops the parcel pool when the overlay is hidden, exactly as the ring
/// captions are dropped — sprites are not gizmos and do not stop on their own.
fn clear_parcels(mut commands: Commands, dots: Query<Entity, With<ParcelDot>>) {
    for entity in &dots {
        commands.entity(entity).despawn();
    }
}

/// Pins the recogniser board below the cloud preview it belongs to.
fn place_matches(
    window: Single<&Window>,
    tools: Option<Res<ToolState>>,
    mut panel: Single<&mut Transform, With<MatchLine>>,
) {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    panel.translation.x = half.x - MARGIN - panel_reserve(tools.as_deref());
    panel.translation.y = half.y - PREVIEW_SIZE - PREVIEW_MARGIN * 2.0;
}

/// How far in from the right edge the overlay has to start drawing.
fn panel_reserve(tools: Option<&ToolState>) -> f32 {
    match tools {
        Some(state) if state.open => PANEL_RESERVE,
        _ => 0.0,
    }
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
    inset: f32,
    cloud: &recognizer::Cloud,
    best: Option<&recognizer::Cloud>,
) {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    let center = Vec2::new(
        half.x - inset - PREVIEW_MARGIN - PREVIEW_SIZE * 0.5,
        half.y - PREVIEW_MARGIN - PREVIEW_SIZE * 0.5,
    );

    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        Vec2::splat(PREVIEW_SIZE),
        PREVIEW_FRAME,
    );
    // The origin the cloud was centred on. Every cloud's points average to it.
    cross(gizmos, center, PREVIEW_FRAME);

    // The nearest rune, drawn first so the drawing sits on top of it. Both
    // clouds are already in the same units — that is what normalisation was
    // for — so the gap you see here is the distance the board reports.
    if let Some(best) = best {
        for p in &best.points {
            gizmos.circle_2d(
                Isometry2d::from_translation(center + Vec2::new(p.x, p.y) * PREVIEW_SIZE),
                SAMPLE_DOT * 1.7,
                TEMPLATE_MARK,
            );
        }
    }

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

/// The ink the recogniser is being shown for one ring.
///
/// What the ring holds — rule 1's *inside* and *touching* — falling back to the
/// ring's own strokes so the preview is never blank while there is ink to show.
/// One helper because the board, the preview and the inspector must all be
/// looking at the same gesture, or the numbers describe different drawings.
fn subject_ink(pad: &InkPad, ring: &assembly::RingCandidate) -> Vec<crate::Point> {
    let held = ring.contents(&pad.points, ON_RING_TOLERANCE);
    let mut ids = held.inside.clone();
    ids.extend(held.touching.iter().copied());
    if ids.is_empty() {
        ids = ring.strokes.clone();
    }
    gesture(&pad.points, &ids)
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
        return "figure-eight - lobes cancel";
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
    tools: Option<Res<ToolState>>,
    lore: Res<Lore>,
    world: Option<Res<crate::sim::Simulation>>,
    mut panel: Single<&mut Text2d, With<InspectorLine>>,
    reading: Res<Reading>,
) {
    let rings = &reading.rings;
    let (camera, camera_transform) = *camera;
    let cursor = cursor_world(&window, camera, camera_transform);

    let Some((slot, by_hover)) = inspected(rings, cursor) else {
        panel.0 = "-- no ring --\ndraw a loop".to_string();
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
        .map(|i| if i < filled { '#' } else { '.' })
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
        assembly::Activation::Malformed => format!("NOT A RING - {}", shape_of(ring)),
        assembly::Activation::Armed => {
            format!("ARMED - {} end(s) to join", ring.open_ends.len())
        }
        assembly::Activation::Fleeting => format!(
            "FLEETING - not circular enough (needs q {:.2})",
            RULES.min_quality
        ),
        assembly::Activation::Active => "ACTIVE - circuit closed".to_string(),
    };

    panel.0 = plain(&format!(
        "-- ring {slot}/{} {:?} -- {}\n\
         geometry   centre {:.0}, {:.0}      radius {:.1}\n\
         \x20          circumference {circumference:.0}px   area {:.0}px^2\n\
         ink        {} stroke(s)   {} pts   drawn {:.0}px   x{:.2} of the ring\n\
         fit        rms {:.2}px   worst {:.2}px   trimmed {}   centroid off {centroid_off:.1}px\n\
         coverage   spanned {:.1}%   gap {:.0}px / {:.0}deg   facing {:.0}deg\n\
         turning    {:.2} turns   sweep {:.2}   backtrack {:.2}   {}\n\
         closure    {}   loose ends {}\n\
         holds      inside {:?}   touching {:?}   ignored {:?}\n\
         resample   {} pts/stroke   ring @ {:.1}px   held @ {}\n\
         cloud      {}   rotation kept - canon rule 6\n\
         \x20          {} pts   reach {:.0}px   x{fill:.2} of the ring   element: ?\n\
         quality    {quality:.3}  {bar}\n\
         canon      r{:.0} -> strength    neat {quality:.2} -> duration\n\
         verdict    {verdict}{}{}",
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
            0 => "-".to_string(),
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
                None => "- nothing to normalise".to_string(),
            }
        },
        held.points,
        held.extent,
        fit.radius,
        spell_report(&reading, slot, &lore, tools.as_deref()),
        world_report(world.as_deref(), tools.as_deref(), &reading),
    ));
}

/// The catalogue, parsed once at startup.
///
/// Baked in with `include_str!` rather than read from disk: core has no
/// filesystem (§4.1), the shell would need an asset path that survives being
/// bundled into a `.app`, and the overlay is a debug tool. `Catalog::parse` over
/// strings is the shipped path either way.
#[derive(Resource)]
struct Lore(Option<magic_core::Catalog>);

impl Default for Lore {
    fn default() -> Self {
        Lore(
            magic_core::Catalog::parse(
                include_str!("../../../crates/magic-core/the-magic-assets/sigils.ron"),
                include_str!("../../../crates/magic-core/the-magic-assets/signs.ron"),
                include_str!("../../../crates/magic-core/the-magic-assets/spells.ron"),
            )
            .ok(),
        )
    }
}

/// The recogniser board: every rune scored against the ink in the ring.
///
/// The circle fitter gets an overlay showing what it measured; this is the same
/// courtesy for `$P`. A single "best match" tells you nothing about *why* — the
/// interesting question is always what came second and by how much, because
/// that is what decides whether a rune shape is worth keeping in the catalogue
/// at all (M4.8 found a square and a ring only 16% apart).
///
/// Decides nothing (§4.2): it normalises, calls `rank`, and prints. The one
/// judgement on screen is [`CONFIDENT_MARGIN`], which is marked as ours.
fn update_matches(
    pad: Res<InkPad>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    tools: Option<Res<ToolState>>,
    lore: Res<Lore>,
    mut panel: Single<&mut Text2d, With<MatchLine>>,
    reading: Res<Reading>,
) {
    if !tools.is_none_or(|state| state.runes) {
        panel.0.clear();
        return;
    }

    let all: Vec<(&'static str, &magic_core::Recorded)> = reading.catalogued().collect();
    let vocabulary = lore.0.as_ref().map_or_else(
        || "catalogue failed to load".to_string(),
        |catalog| {
            format!(
                "{} sigils, {} signs, {} with a shape",
                catalog.sigils().count(),
                catalog.signs().count(),
                all.len(),
            )
        },
    );

    // Every line is kept short on purpose. The block is right-anchored, so its
    // width is set by its longest line — one long value and the whole panel
    // slides off the left edge of the window.
    // Counted off the one shared list rather than off a private copy. There
    // used to be two, and they disagreed — see `Reading::shapes`.
    let mut tally = (0usize, 0usize, 0usize);
    for (source, _) in &all {
        match *source {
            "recorded" => tally.0 += 1,
            "traced" => tally.1 += 1,
            _ => tally.2 += 1,
        }
    }

    let mut out = format!(
        "-- recogniser --  $P, {} pts, rotation kept\n\
         source           {}\n\
         shapes           {} recorded, {} in templates.ron, {} built-in\n\
         catalogue        {vocabulary}\n",
        stroke::MATCH_POINTS,
        reading.note,
        tally.0,
        tally.1,
        tally.2,
    );

    let rings = &reading.rings;
    let (camera, camera_transform) = *camera;
    let cursor = cursor_world(&window, camera, camera_transform);

    let Some((slot, _)) = inspected(rings, cursor) else {
        panel.0 = plain(&out) + "gesture    - no ring to look inside\n";
        return;
    };

    let ink = subject_ink(&pad, &rings[slot]);
    let Some(cloud) = recognizer::normalize(&ink, stroke::MATCH_POINTS) else {
        panel.0 = plain(&out) + "gesture    - nothing to normalise\n";
        return;
    };

    out.push_str(&format!(
        "gesture    ring {slot}   {} mark(s)   {} pts   scale {:.0}px divided out\n",
        ink.chunk_by(|a, b| a.stroke_id == b.stroke_id).count(),
        cloud.points.len(),
        cloud.scale,
    ));

    if all.is_empty() {
        panel.0 = plain(&out)
            + "ranked     nothing to rank - no rune has a shape yet\n\
               \x20          trace one inside the ring, press record, and it\n\
               \x20          is scored here on the next frame\n";
        return;
    }

    let shapes: Vec<recognizer::Template> =
        all.iter().map(|(_, rune)| rune.template.clone()).collect();
    let ranked = recognizer::rank(&cloud, &shapes);

    // The bar is *relative* — full for the nearest rune in this ranking, empty
    // for the furthest. An absolute bar would say nothing: every plausible
    // distance sits in a narrow band near zero, so ten cells would fill for a
    // good match and a bad one alike. The number beside it is the absolute one.
    let nearest = ranked.first().map_or(0.0, |found| found.distance);
    let furthest = ranked.last().map_or(0.0, |found| found.distance);
    let spread = (furthest - nearest).max(f32::EPSILON);

    out.push_str("ranked     % is 100 at exact, 0 at the distance the compiler rejects\n");
    for (place, found) in ranked.iter().take(RANKED_SHOWN).enumerate() {
        let (source, rune) = all[found.index];
        let standing = 1.0 - (found.distance - nearest) / spread;
        let filled = ((standing * QUALITY_CELLS as f32).round() as usize).min(QUALITY_CELLS);
        let bar: String = (0..QUALITY_CELLS)
            .map(|cell| if cell < filled { '#' } else { '.' })
            .collect();
        out.push_str(&format!(
            "{:>3}  {:<16} {:<6} {source:<6} {:>3.0}% {:.3}  {bar}\n",
            place + 1,
            rune.template.name,
            format!("{:?}", rune.kind),
            closeness(found.distance),
            found.distance,
        ));
    }
    if ranked.len() > RANKED_SHOWN {
        out.push_str(&format!("     ... {} more\n", ranked.len() - RANKED_SHOWN));
    }

    out.push_str(&match (ranked.first(), ranked.get(1)) {
        (Some(best), Some(second)) => {
            let margin = second.distance - best.distance;
            format!(
                "margin     {margin:.3} clear of {} - {}\n",
                all[second.index].1.template.name,
                if margin >= CONFIDENT_MARGIN {
                    "a clear call (ours: >=0.05)"
                } else {
                    "too close to call (ours: <0.05)"
                },
            )
        }
        (Some(_), None) => "margin     only one rune to compare against\n".to_string(),
        _ => "margin     nothing comparable - sample counts differ\n".to_string(),
    });

    out.push_str("threshold  none - how close is close enough is the compiler's call");
    panel.0 = plain(&out);
}

/// What the ring compiles to, appended to the inspector.
///
/// Only the ring and what it holds are real inputs — `templates.ron` is empty,
/// so nothing on the pad can be *named* a sigil yet and every seal currently
/// compiles as canon rule 9's discharge. That is the honest answer rather than
/// a placeholder, and the moment a rune is traced this block starts saying
/// something different without changing.
///
/// The shell decides nothing here: it builds a `Glyph`, calls `compile`, and
/// prints what comes back (§4.2).
fn spell_report(reading: &Reading, slot: usize, lore: &Lore, tools: Option<&ToolState>) -> String {
    if !tools.is_none_or(|t| t.spell) {
        return String::new();
    }
    if lore.0.is_none() {
        return "\nspell      catalogue failed to load".to_string();
    }

    // **Read, never recompiled.** Two earlier versions of this block built their
    // own glyphs, and both were wrong in the same way. The first hand-assembled
    // a `Glyph` from one ring, so it never carried `unnamed` and reported canon
    // rule 9's *bare* ring over a seal covered in ink. The second ran
    // `assembly::glyphs` + `compile_all` again — closer, and still a second
    // reading, because `naming` never touched it: the caption beside it said
    // `water - MAKES water` while this block reported a discharge.
    //
    // `reading.rs` exists precisely so there is one answer. A readout that
    // recomputes is a readout that will eventually describe a different drawing
    // from the one on the paper.
    let (Some(glyph), Some(spell)) = (reading.glyphs.get(slot), reading.spells.get(slot)) else {
        return "\nspell      no glyph for this ring".to_string();
    };
    let warnings = if spell.warnings.is_empty() {
        "none".to_string()
    } else {
        spell
            .warnings
            .iter()
            .map(|w| w.to_string())
            .collect::<Vec<_>>()
            .join("  |  ")
    };

    format!(
        "\n-- spell --\n         driver     {:?}   firing {:?}   fires {}\n         strength   {:.2}   intensity {:.2}   x{:.2} linked   scale r{:.0}\n         shape      {} sign(s)   embed {:.2}   {}   region {:?}\n         pad        nested in {}   linked to {:?}\n         balance    lean {:.2} -> {:.0}deg   power {:.1}   spin {:.2} / reach {:.2}\n         focus      {}\n         needs      {}\n         warnings   {warnings}",
        spell.driver,
        spell.firing,
        spell.fires(),
        spell.strength(),
        spell.intensity,
        spell.amplification,
        spell.scale,
        spell.sign_count,
        spell.embedding,
        // Symmetry classifies an *arrangement*, and a seal with no signs has
        // none to classify — `Radial` there is vacuously true and reads as a
        // measurement. Wording only; core still answers what it answers (§4.2).
        match spell.sign_count {
            0 => "-".to_string(),
            _ => format!("{:?}", spell.symmetry),
        },
        spell.region,
        match glyph.parent {
            Some(id) => format!("#{}", id.0),
            None => "nothing".to_string(),
        },
        glyph.linked.iter().map(|id| id.0).collect::<Vec<_>>(),
        spell.balance.lean(),
        spell.balance.heading.to_degrees(),
        spell.balance.power,
        spell.spin.spin,
        spell.spin.reach,
        // Where the signs *aim*, as against which way they push. Four arrows at
        // the middle cancel to zero lean — `balance` is right that the spell
        // goes nowhere, and cannot say that all four are pushing at one point.
        // That is the whole difference between a water orb and a fountain.
        {
            let focus = &spell.focus;
            match focus.convergence {
                magic_core::arrangement::Convergence::Unaimed => match focus.aimed {
                    0 => "no sign steers this seal - nothing aims anywhere".to_string(),
                    _ => "one steering sign - nothing for it to meet".to_string(),
                },
                magic_core::arrangement::Convergence::Parallel => format!(
                    "{} sign(s) all pointing one way - a beam, no focal point",
                    focus.aimed
                ),
                other => format!(
                    "{} {} of the ring, {:.0}px off centre, {:.0}% tight  ({})",
                    match other {
                        magic_core::arrangement::Convergence::Converging => "GATHERS at",
                        magic_core::arrangement::Convergence::Diverging => "SPREADS from",
                        _ => "meets on",
                    },
                    match focus.offset() < spell.scale * 0.15 {
                        true => "the centre",
                        false => "a point inside",
                    },
                    focus.offset(),
                    focus.tightness(spell.scale) * 100.0,
                    match other.as_region() {
                        Some(pattern) => format!("canon {pattern:?}"),
                        None => "no canon case".to_string(),
                    },
                ),
            }
        },
        match &spell.demand {
            Some(d) if d.must_find => format!("must find {:?}", d.substance),
            Some(d) => format!("may create {:?}", d.substance),
            None => "nothing - a discharge conserves no substance".to_string(),
        },
    )
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
        None => format!("F x{} swaps paper", taps.needed()),
    };

    line.0 = format!(
        "focus field {}  events {focus_by_event}   keyev {}  last {}\nheld [{}]  mouse [{}]  F1 hides this  {tap_run}",
        window.focused,
        probe.key_events,
        if probe.last_key.is_empty() {
            "-"
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
    hand: Option<Res<crate::hand::Hand>>,
    tools: Option<Res<ToolState>>,
    world: Option<Res<crate::sim::Simulation>>,
    mut labels: Query<(
        Entity,
        &RingLabel,
        &mut Text2d,
        &mut TextColor,
        &mut Transform,
    )>,
    reading: Res<Reading>,
) {
    let rings = &reading.rings;

    // Compiled once for every caption rather than once per caption. Naming is
    // applied first, so the label says what will actually happen when this seal
    // fires — not what it would do if nobody had picked a sigil.
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
                // The caption is the one thing on the paper a person *reads*
                // rather than measures, so it gets the script hand.
                font: hand.as_ref().map_or_else(Default::default, |h| {
                    bevy::text::FontSource::Handle(h.script.clone())
                }),
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

    let only_traced = tools.as_deref().is_none_or(|state| state.only_traced);
    for (_, slot, mut text, mut color, mut transform) in &mut labels {
        let Some(ring) = rings.get(slot.0) else {
            continue;
        };

        let activation = ring.activation(&RULES);
        let state = match activation {
            assembly::Activation::Malformed => shape_of(ring).to_string(),
            assembly::Activation::Armed => {
                format!("ARMED - {} loose end(s)", ring.open_ends.len())
            }
            assembly::Activation::Fleeting => "FLEETING - too rough".to_string(),
            assembly::Activation::Active => "ACTIVE - circuit closed".to_string(),
        };

        // Member strokes are printed because a ring made of several strokes is
        // the normal case, and knowing which ones it swallowed is the whole
        // reason a closing line no longer looks like a ring of its own.
        // What this seal will actually produce, over the ring it belongs to.
        // A number nobody can name is a measurement; a name is an answer, and
        // the question a person actually has in front of a drawing is "is this
        // going to be fire or water".
        let makes = reading
            .spell(slot.0)
            .map(|spell| match (&spell.driver, &spell.demand) {
                // The caption over the seal is the closest thing to the
                // drawing, so it is where a refusal has to appear. It used to
                // say "DISCHARGE - a blast" for a ring the app was quietly
                // declining to fire, which is the caption confidently naming a
                // spell that was never going to happen.
                (magic_core::Driver::Discharge, _) => {
                    match crate::sim::refuses(spell, only_traced) {
                        Some(why) => format!("NOT CAST - {why}"),
                        None => "DISCHARGE - a blast".to_string(),
                    }
                }
                (driver, Some(demand)) if demand.substance.is_empty() => {
                    // Guidance, obliviation and doorways act on no substance at
                    // all, and `MOVES ` with nothing after it is a sentence
                    // that stops halfway.
                    let name = match driver {
                        magic_core::Driver::Sigil(id) => id.as_str().to_string(),
                        magic_core::Driver::Substitute(id) => id.as_str().to_string(),
                        magic_core::Driver::Discharge => "discharge".to_string(),
                    };
                    format!("{name} - acts on no substance")
                }
                (driver, Some(demand)) => {
                    let name = match driver {
                        magic_core::Driver::Sigil(id) => id.as_str().to_string(),
                        magic_core::Driver::Substitute(id) => id.as_str().to_string(),
                        magic_core::Driver::Discharge => "discharge".to_string(),
                    };
                    let verb = if demand.must_find { "MOVES" } else { "MAKES" };
                    format!("{name} - {verb} {}", demand.substance.join(", "))
                }
                (driver, None) => format!("{driver:?} - no substance"),
            })
            .unwrap_or_else(|| "unread".to_string());

        // With the measurement overlay off, the caption is the *answer* and
        // nothing else. Stroke ids and rms are what you want while debugging
        // the fitter, and noise while drawing a spell.
        let measuring = tools.as_deref().is_none_or(|state| state.debug_overlay);
        // A seal still summoning says so, over the seal. Everything else about
        // a running spell is in the world block; this is the one fact you want
        // without looking away from the drawing.
        let running = world
            .as_ref()
            .and_then(|world| {
                let centre = magic_core::sim::Vec2::new(ring.fit.center.x, ring.fit.center.y);
                world
                    .sim
                    .channels
                    .iter()
                    .find(|open| (open.at - centre).length() < ring.fit.radius)
            })
            .map(|open| format!("\nsummoning - {:.1}s left", open.left))
            .unwrap_or_default();

        text.0 = if measuring {
            format!(
                "{makes}\n{:?} | r {:.0} | q {:.2}\n{state}{running}",
                ring.strokes,
                ring.fit.radius,
                ring.fit.quality()
            )
        } else {
            format!("{makes}\n{state}{running}")
        };
        color.0 = activation_color(activation);

        // An armed ring puts its caption at the widest hole, which is where
        // the eye wants to go. A closed ring has no meaningful gap direction —
        // its widest gap is just the sampling step and wanders frame to frame
        // — so those sit at the *bottom*. The top is where the inspector's own
        // lines run, and a caption there landed on top of them.
        let angle = if ring.closed {
            -std::f32::consts::FRAC_PI_2
        } else {
            ring.coverage.gap_heading
        };
        let out = Vec2::from_angle(angle) * (ring.fit.radius + LABEL_OFFSET);
        transform.translation.x = ring.fit.center.x + out.x;
        transform.translation.y = ring.fit.center.y + out.y;
    }
}

/// A mark outside every ring: named, and inert.
#[derive(Component)]
struct LooseLabel(usize);

/// Captions each mark that belongs to no ring.
///
/// Canon rule 1 makes these contribute nothing, and §3.3 makes them inert
/// rather than invalid — but inert should not mean invisible. Knowing the app
/// read your fire sigil correctly is exactly what you want *before* drawing a
/// ring around it, and until now the only way to find out was to draw one.
fn update_loose_labels(
    mut commands: Commands,
    hand: Option<Res<crate::hand::Hand>>,
    tools: Option<Res<ToolState>>,
    reading: Res<Reading>,
    mut labels: Query<(&LooseLabel, &mut Text2d, &mut Transform, &mut Visibility)>,
) {
    let showing = tools.is_none_or(|state| state.spell);
    let marks: &[(magic_core::Vec2, String)] = if showing { &reading.loose } else { &[] };

    let mut seen = 0usize;
    for (slot, mut text, mut transform, mut visible) in &mut labels {
        seen = seen.max(slot.0 + 1);
        match marks.get(slot.0) {
            Some((at, said)) => {
                text.0 = plain(said);
                transform.translation = Vec3::new(at.x, at.y - LABEL_OFFSET, 1.0);
                *visible = Visibility::Inherited;
            }
            None => *visible = Visibility::Hidden,
        }
    }

    for slot in seen..marks.len() {
        commands.spawn((
            Text2d::new(""),
            TextFont {
                font: hand.as_ref().map_or_else(Default::default, |h| {
                    bevy::text::FontSource::Handle(h.script.clone())
                }),
                font_size: FontSize::Px(LABEL_SIZE * 0.8),
                ..default()
            },
            // Faint: it is a note about ink that is doing nothing.
            TextColor(Color::srgba(0.45, 0.42, 0.38, 0.85)),
            TextLayout::justify(Justify::Center),
            Transform::from_xyz(0.0, 0.0, 1.0),
            LooseLabel(slot),
        ));
    }
}

/// Drops the loose captions with everything else.
fn clear_loose_labels(mut commands: Commands, labels: Query<Entity, With<LooseLabel>>) {
    for entity in &labels {
        commands.entity(entity).despawn();
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
        None => "-".to_string(),
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
                format!("FULL {:.0}x{:.0}", paper.scale.x, paper.scale.y)
            }
        },
        None => "-".to_string(),
    };

    line.0 = format!(
        "window {:.0}x{:.0}   half +/-{:.0}, +/-{:.0}   dpi x{:.2}   {}\ncredit {credit_at}   paper {paper_at}   margins {MARGIN:.0}/{:.0}",
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
        _ => "-".to_string(),
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
fn update_fit_line(
    pad: Res<InkPad>,
    reading: Res<Reading>,
    mut line: Single<&mut Text2d, With<FitLine>>,
) {
    let strokes = pad
        .points
        .chunk_by(|a, b| a.stroke_id == b.stroke_id)
        .count();
    let rings = &reading.rings;

    let (head, state) = match rings.first() {
        Some(r) => (
            format!(
                "ring 1/{}   strokes {:?}   c {:.0},{:.0}   r {:.1}   rms {:.2}px   q {:.3}   trim {}   dev x{DEVIATION_GAIN:.0}",
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
                "gap {:.0}px ({:.0}deg)   spanned {:.1}%   turns {:.2}   loose ends {}   {}",
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
            "no rings - ink so far names no circle".to_string(),
            "-".to_string(),
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
                if r.closed { "o" } else { "(" }
            )
        })
        .collect();

    line.0 = format!(
        "{head}\n{state}\nrings {} from {strokes} strokes   [{}]",
        rings.len(),
        listed.join("] ["),
    );
}

/// Draws the fit itself: the circle, its centre, and where the ink misses it.
///
/// Every stroke gets its circle. Only the newest gets the band, the centroid
/// mark and the whiskers — all of them at once is unreadable, and the newest
/// stroke is the one being judged.
///
/// Reads `Reading` rather than refitting. This used to say a cache would be "a
/// second copy of the truth" and refit here every frame — which was true of one
/// readout and wrong once there were eight, because eight copies of the work is
/// not one copy of the truth. `reading.rs` is the single copy now, and the
/// argument that a cache goes stale is answered by recomputing it whenever the
/// ink or the naming changes rather than by not having one.
fn draw_fits(
    pad: Res<InkPad>,
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    tools: Option<Res<ToolState>>,
    mut gizmos: Gizmos<DebugGizmos>,
    reading: Res<Reading>,
) {
    let inset = panel_reserve(tools.as_deref());
    let rings = &reading.rings;
    let (camera, camera_transform) = *camera;
    let focus = inspected(rings, cursor_world(&window, camera, camera_transform)).map(|(s, _)| s);

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
        if let Some(cloud) =
            recognizer::normalize(&subject_ink(&pad, candidate), stroke::MATCH_POINTS)
        {
            let all: Vec<(&'static str, &magic_core::Recorded)> = reading.catalogued().collect();
            let shapes: Vec<recognizer::Template> =
                all.iter().map(|(_, rune)| rune.template.clone()).collect();
            let best =
                recognizer::classify(&cloud, &shapes).map(|found| &shapes[found.index].cloud);
            draw_cloud_preview(&mut gizmos, &window, inset, &cloud, best);
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
        "screen {:.0}, {:.0}   ->   world {:.0}, {:.0}",
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
        None => "-".to_string(),
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

/// Draws where each seal's power gathers.
///
/// Asked for directly: "add a debug screen to show where the magic goes like
/// focusing the power, if the arrows all goes to the middle then focus all of
/// it power goes to middle".
///
/// The `focus` line in the spell block already says it in words, and a number
/// beside a drawing is not the same as a mark *on* it — the whole question is
/// spatial, and the answer belongs where you are looking. So the focal point
/// gets a crosshair, its tightness gets a circle, and every steering sign gets
/// a spoke showing what it is aiming at.
///
/// Rides the `spell` toggle rather than F1, because "where will this go" is a
/// question you have while drawing (§0's own reasoning for the ring caption).
fn draw_focus(reading: Res<Reading>, tools: Option<Res<ToolState>>, mut gizmos: Gizmos) {
    if !tools.is_none_or(|state| state.spell) {
        return;
    }

    for (glyph, spell) in reading.glyphs.iter().zip(reading.spells.iter()) {
        let focus = &spell.focus;
        // A beam and an unaimed seal have no point to draw, and inventing one
        // would be the readout lying about the thing it exists to report.
        let colour = match focus.convergence {
            magic_core::arrangement::Convergence::Converging => Color::srgba(0.30, 0.80, 0.95, 0.9),
            magic_core::arrangement::Convergence::Diverging => Color::srgba(0.98, 0.55, 0.20, 0.9),
            magic_core::arrangement::Convergence::Split => Color::srgba(0.75, 0.55, 0.95, 0.9),
            _ => continue,
        };

        let centre = Vec2::new(glyph.ring.center().x, glyph.ring.center().y);
        let at = centre + Vec2::new(focus.at.0, focus.at.1);
        let radius = glyph.ring.radius();

        // The crosshair. Six pixels, because this marks a point rather than
        // covering one.
        gizmos.line_2d(at - Vec2::X * 6.0, at + Vec2::X * 6.0, colour);
        gizmos.line_2d(at - Vec2::Y * 6.0, at + Vec2::Y * 6.0, colour);

        // How tightly the rays actually meet. A wide circle is a seal whose
        // signs nearly agree; a tight one is an orb.
        let spread = focus.spread.max(2.0);
        gizmos.circle_2d(
            Isometry2d::from_translation(at),
            spread,
            colour.with_alpha(0.35),
        );

        // What each steering sign is aiming at. This is the part that makes the
        // reading obvious rather than merely correct: four spokes converging on
        // one crosshair *is* the water orb, drawn.
        for sign in &spell.effective {
            let Some(drawn) = glyph.signs.iter().find(|s| &s.kind == sign) else {
                continue;
            };
            let from = centre + Vec2::from_angle(drawn.placement) * radius;
            gizmos.line_2d(from, at, colour.with_alpha(0.25));
        }
    }
}
