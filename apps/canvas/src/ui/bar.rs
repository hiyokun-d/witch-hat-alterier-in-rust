//! Where the panel's pieces are, how they draw, and what the pointer is over.
//!
//! One function — [`rows`] — decides where every piece sits, and the layout,
//! the drawing and the hit-testing all ask it. That is the only way the three
//! cannot drift apart: a button you can see but not click is the classic bug
//! here, and it comes from two of them computing the same rectangle twice.
//!
//! The panel is a column down the right-hand edge, sectioned by [`Tool`]'s
//! `section` field. Adding a tool moves everything below it down; adding one
//! with a new section name grows a new heading. Neither needs anything here to
//! change.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::FontSource;

use super::{Action, Mode, Pointer, TOOLS, Tool, ToolState};
use crate::{InkPad, PaperShape, cursor_world};

/// Panel geometry, in pixels.
const PANEL_W: f32 = 152.0;
const BUTTON_H: f32 = 26.0;
const HEADER_H: f32 = 22.0;
const ROW_GAP: f32 = 4.0;
/// Breathing room between the slab's edge and the buttons inside it.
const PAD: f32 = 9.0;
/// Distance from the window's edge to the slab.
const EDGE: f32 = 14.0;

// The palette is one idea: a workbench, not an interface. Every colour is a
// thing that exists on a desk in a workshop — tanned hide, iron-gall ink,
// candle-brass, verdigris — and nothing is a hue a dye could not have made.
//
/// The slab everything sits on: dark tooled leather. Translucent enough that
/// ink underneath is not lost.
const PANEL: Color = Color::srgba(0.13, 0.10, 0.08, 0.94);
/// A hairline of brass around the slab, the way a book board is tooled.
const PANEL_EDGE: Color = Color::srgba(0.62, 0.48, 0.24, 0.55);
/// A button at rest: the leather again, one shade up so it reads as raised.
const FILL_IDLE: Color = Color::srgba(0.24, 0.19, 0.15, 1.0);
const FILL_HOVER: Color = Color::srgba(0.36, 0.29, 0.22, 1.0);
/// Armed: the pad is about to do this. Candle-brass, because it is the loud
/// state and warm light is what a workshop is lit by.
const FILL_ARMED: Color = Color::srgba(0.80, 0.56, 0.20, 1.0);
/// A toggle that is on. Verdigris — quiet, because it is a background
/// condition rather than an announcement.
const FILL_ON: Color = Color::srgba(0.16, 0.42, 0.33, 1.0);

/// Chalk on leather.
const LABEL: Color = Color::srgb(0.94, 0.90, 0.80);
/// Dark text for the one fill bright enough to need it.
const LABEL_ARMED: Color = Color::srgb(0.14, 0.10, 0.06);
/// Section headings, in the brass the slab is tooled with.
const HEADING: Color = Color::srgb(0.72, 0.58, 0.34);
// Bright, and on its own plate. The hint is the only line that says what the
// panel just *did*, and at low contrast under a two-column slab it was being
// missed entirely.
const HINT: Color = Color::srgb(0.99, 0.95, 0.86);
const HINT_PLATE: Color = Color::srgba(0.08, 0.07, 0.06, 0.92);
const HINT_SIZE: f32 = 16.0;

/// Above the ink and the paper.
const PANEL_Z: f32 = 50.0;

/// Which piece of the panel an entity is.
///
/// One component for every piece rather than one per kind, so laying the panel
/// out is a single query with no `Without` filters. A button's background and
/// its label share a slot because they share a position — that is the point.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    /// The slab behind everything.
    Panel,
    /// The show/hide handle. Stays put whether the panel is open or closed.
    Handle,
    /// A section heading, by its order down the panel.
    Heading(usize),
    /// A button, by its index in [`TOOLS`].
    Button(usize),
    /// The line that explains whatever the pointer is over.
    Hint,
    /// The plate behind it, so pale text is not sitting on parchment.
    HintPlate,
    /// A hairline of brass just outside the slab, the way a book board is
    /// tooled. Drawn as a slightly larger sprite *behind* the slab rather than
    /// as an outline, because a sprite has no stroke and four thin rectangles
    /// would be four more things to keep in step with a resizing window.
    PanelEdge,
}

/// A row of the panel: what it is, and the rectangle it occupies.
struct Row {
    slot: Slot,
    rect: Rect,
}

/// Section names, in the order they first appear in [`TOOLS`].
fn headings() -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    for tool in TOOLS {
        if found.last() != Some(&tool.section) {
            found.push(tool.section);
        }
    }
    found
}

/// Every row of the panel, top to bottom — and left again when it runs out.
///
/// The single source of truth for the panel's geometry. Cheap enough to
/// rebuild each frame and rebuilding beats caching something that has to
/// follow a resizing window.
///
/// **It wraps into columns.** Twenty-odd tools is taller than a 900px window,
/// and the first version simply drew the overflow past the bottom edge where
/// `runes` and `debug` could never be clicked. A tool nobody can reach is worse
/// than no tool, so a column that will not fit starts a new one to its left.
fn rows(window: &Window) -> Vec<Row> {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    let floor = -half.y + EDGE + PAD;
    let width = PANEL_W - PAD * 2.0;

    let mut out: Vec<Row> = Vec::with_capacity(TOOLS.len() + 6);
    let mut column = 0usize;
    let mut top = half.y - EDGE - PAD;

    let mut place = |slot: Slot, height: f32, column: &mut usize, top: &mut f32| {
        // The handle always stays in the first column: it is how the panel is
        // opened, so it cannot be the thing that moved.
        if *top - height < floor && !matches!(slot, Slot::Handle) {
            *column += 1;
            *top = half.y - EDGE - PAD;
        }
        let x = half.x - EDGE - PANEL_W * 0.5 - *column as f32 * (PANEL_W + PAD);
        out.push(Row {
            slot,
            rect: Rect::from_center_size(
                Vec2::new(x, *top - height * 0.5),
                Vec2::new(width, height),
            ),
        });
        *top -= height + ROW_GAP;
    };

    place(Slot::Handle, BUTTON_H, &mut column, &mut top);

    let mut section = "";
    let mut heading = 0;
    for (index, tool) in TOOLS.iter().enumerate() {
        if tool.section != section {
            section = tool.section;
            place(Slot::Heading(heading), HEADER_H, &mut column, &mut top);
            heading += 1;
        }
        place(Slot::Button(index), BUTTON_H, &mut column, &mut top);
    }

    out
}

/// The slab, sized to hold whatever [`rows`] produced.
fn panel_rect(window: &Window) -> Rect {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    let laid_out = rows(window);

    // Across every column, not just the last row: the slab has to cover a
    // second column that starts at the top again.
    let left = laid_out
        .iter()
        .map(|row| row.rect.min.x)
        .fold(half.x - EDGE - PANEL_W, f32::min);
    let bottom = laid_out
        .iter()
        .map(|row| row.rect.min.y)
        .fold(half.y - EDGE, f32::min);

    Rect::from_corners(
        Vec2::new(left - PAD, bottom - PAD),
        Vec2::new(half.x - EDGE, half.y - EDGE),
    )
}

/// Where the handle sits. Always the first row, open or closed.
fn handle_rect(window: &Window) -> Rect {
    rows(window)
        .into_iter()
        .find(|row| row.slot == Slot::Handle)
        .map(|row| row.rect)
        .unwrap_or(Rect::EMPTY)
}

/// Where the hint sits: the bottom edge, clear of the slab.
///
/// Three things were wrong with it at once. It was anchored at the window's
/// right edge, which is *underneath* the panel, so the text ran behind the
/// buttons. It was anchored at its **top**, so a second line grew downward off
/// the bottom of the screen. And the plate behind it was sized for one line.
///
/// So: anchored bottom-right, shifted left past the slab, and grown upward.
fn hint_at(window: &Window) -> Vec2 {
    let half = Vec2::new(window.width(), window.height()) * 0.5;
    Vec2::new(half.x - EDGE - PANEL_W - PAD * 2.0, -half.y + EDGE)
}

/// How tall the plate is: two lines and a little air.
fn hint_height() -> f32 {
    HINT_SIZE * 2.0 * 1.35 + PAD * 2.0
}

pub fn spawn(mut commands: Commands, hand: Res<crate::hand::Hand>) {
    // The script hand for everything on the panel: these are words a person
    // reads, not columns they scan. Bumped a point as well — Garamond's
    // x-height is small, which is exactly what makes it look like a book and
    // exactly what makes it need the extra size.
    let label_font = TextFont {
        font: FontSource::Handle(hand.script.clone()),
        font_size: FontSize::Px(15.0),
        ..default()
    };
    let small_font = TextFont {
        font: FontSource::Handle(hand.script.clone()),
        font_size: FontSize::Px(13.0),
        ..default()
    };

    commands.spawn((
        Sprite {
            color: PANEL_EDGE,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, PANEL_Z - 2.0),
        Slot::PanelEdge,
    ));

    commands.spawn((
        Sprite {
            color: PANEL,
            ..default()
        },
        // A little behind the buttons so it never covers them.
        Transform::from_xyz(0.0, 0.0, PANEL_Z - 1.0),
        Slot::Panel,
    ));

    commands.spawn((
        Sprite {
            color: FILL_IDLE,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, PANEL_Z),
        Slot::Handle,
    ));
    commands.spawn((
        Text2d::new("tools"),
        small_font.clone(),
        TextColor(HEADING),
        Anchor::CENTER,
        Transform::from_xyz(0.0, 0.0, PANEL_Z + 1.0),
        Slot::Handle,
    ));

    for (order, name) in headings().into_iter().enumerate() {
        commands.spawn((
            Text2d::new(name),
            small_font.clone(),
            TextColor(HEADING),
            // Left-aligned so the headings read as a spine down the panel
            // rather than as more buttons.
            Anchor::CENTER_LEFT,
            Transform::from_xyz(0.0, 0.0, PANEL_Z + 1.0),
            Slot::Heading(order),
        ));
    }

    for (index, tool) in TOOLS.iter().enumerate() {
        commands.spawn((
            Sprite {
                color: FILL_IDLE,
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, PANEL_Z),
            Slot::Button(index),
        ));
        commands.spawn((
            Text2d::new(tool.label),
            label_font.clone(),
            TextColor(LABEL),
            Anchor::CENTER,
            Transform::from_xyz(0.0, 0.0, PANEL_Z + 1.0),
            Slot::Button(index),
        ));
    }

    // A plate behind it, because the hint sits over the paper and pale text on
    // parchment is not text.
    commands.spawn((
        Sprite::from_color(HINT_PLATE, Vec2::ONE),
        Transform::from_xyz(0.0, 0.0, PANEL_Z),
        Slot::HintPlate,
    ));

    commands.spawn((
        Text2d::new(""),
        TextFont {
            font: FontSource::Handle(hand.script.clone()),
            font_size: FontSize::Px(HINT_SIZE),
            ..default()
        },
        TextColor(HINT),
        // Bottom-right: the block's *lower* edge is pinned, so a second line
        // grows upward into the page rather than downward off the screen.
        Anchor::BOTTOM_RIGHT,
        Transform::from_xyz(0.0, 0.0, PANEL_Z + 1.0),
        Slot::Hint,
    ));
}

/// Which entry in [`TOOLS`] carries `label`, if any.
///
/// Looked up rather than written down: the table is edited constantly and an
/// index copied into a second file is an index that goes stale silently.
pub fn tool_named(label: &str) -> Option<usize> {
    TOOLS.iter().position(|tool| tool.label == label)
}

/// Works out where the pointer is and whether the panel has claimed it.
///
/// Runs before capture, so a click that lands on the panel is known to be a
/// button press before the pad has a chance to ink it.
pub fn track_pointer(
    window: Single<&Window>,
    camera: Single<(&Camera, &GlobalTransform)>,
    tools: Res<ToolState>,
    mut pointer: ResMut<Pointer>,
) {
    let (camera, camera_transform) = *camera;
    pointer.at = cursor_world(&window, camera, camera_transform);

    pointer.over_ui = match pointer.at {
        // The whole slab claims the pointer, not just the buttons — ink placed
        // in the gaps between them would be ink you cannot see.
        Some(at) if tools.open => panel_rect(&window).contains(at),
        Some(at) => handle_rect(&window).contains(at),
        None => false,
    };

    pointer.over_tool = pointer.at.and_then(|at| match hit(at, &window, &tools) {
        Some(Slot::Button(index)) => Some(index),
        _ => None,
    });
}

/// Which button, if any, is under `at`.
fn hit(at: Vec2, window: &Window, tools: &ToolState) -> Option<Slot> {
    rows(window)
        .into_iter()
        .find(|row| match row.slot {
            Slot::Handle => row.rect.contains(at),
            // Closed, only the handle answers and the pad runs up to it.
            Slot::Button(_) => tools.open && row.rect.contains(at),
            _ => false,
        })
        .map(|row| row.slot)
}

/// Everything a button press is allowed to change.
///
/// Bundled rather than listed one by one: a system taking eight resources is
/// a system nobody can read the signature of, and the panel's commands only
/// grow from here.
#[derive(SystemParam)]
pub struct Controls<'w> {
    tools: ResMut<'w, ToolState>,
    pad: ResMut<'w, InkPad>,
    shape: ResMut<'w, PaperShape>,
    last: ResMut<'w, super::record::LastRecording>,
    world: ResMut<'w, crate::sim::Simulation>,
    /// What the pad currently reads as, and which runes are traced. `preset`
    /// needs both: it draws the traced rune, and refuses when there is none.
    reading: Res<'w, crate::reading::Reading>,
}

/// Turns a press into a tool.
pub fn click(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window>,
    pointer: Res<Pointer>,
    mut controls: Controls,
) {
    // `just_pressed`, not `pressed`: a held button would fire every frame.
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    let Some(at) = pointer.at else {
        return;
    };

    match hit(at, &window, &controls.tools) {
        Some(Slot::Handle) => super::flip(super::Toggle::Panel, &mut controls.tools),
        Some(Slot::Button(index)) => match TOOLS[index].action {
            // Picking what is already armed disarms it, so the same button
            // both takes the tool up and puts it down.
            Action::Pick(mode) => {
                controls.tools.mode = if controls.tools.mode == mode {
                    Mode::Pen
                } else {
                    mode
                };
            }
            Action::Toggle(toggle) => super::flip(toggle, &mut controls.tools),
            Action::Run(command) => super::run(
                command,
                &mut controls.tools,
                &mut controls.pad,
                &mut controls.shape,
                &mut controls.last,
                &mut controls.world,
                &super::Bench {
                    window: &window,
                    reading: &controls.reading,
                },
            ),
        },
        _ => {}
    }
}

/// Puts every piece where [`rows`] says it goes, and hides all but the handle
/// when the panel is closed.
pub fn layout(
    window: Single<&Window>,
    tools: Res<ToolState>,
    mut pieces: Query<(&Slot, &mut Transform, &mut Visibility, Option<&mut Sprite>)>,
) {
    let laid_out = rows(&window);
    let slab = panel_rect(&window);
    let hint = hint_at(&window);

    let shown = if tools.open {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };

    for (slot, mut transform, mut visibility, sprite) in &mut pieces {
        let (rect, seen) = match slot {
            Slot::Panel => (slab, shown),
            // Two pixels proud on every side.
            Slot::PanelEdge => (
                Rect::from_corners(slab.min - Vec2::splat(2.0), slab.max + Vec2::splat(2.0)),
                shown,
            ),
            // The hint is the only readout when the panel is shut, so it
            // stays: hovering the handle still explains itself.
            Slot::Hint => (
                Rect::from_center_size(hint, Vec2::ZERO),
                Visibility::Inherited,
            ),
            // Wide enough to cover a long hint. Sized here rather than measured
            // from the text because Bevy will not tell us how wide a `Text2d`
            // came out until after it has drawn it, and a plate that lags the
            // text by a frame flickers.
            // Wide enough for a long hint and tall enough for two lines. Sized
            // here rather than measured from the text because Bevy will not say
            // how wide a `Text2d` came out until after it has drawn it, and a
            // plate that lags the text by a frame flickers.
            Slot::HintPlate => (
                Rect::from_corners(
                    Vec2::new(hint.x - 1100.0, hint.y - PAD),
                    Vec2::new(hint.x + PAD, hint.y + hint_height()),
                ),
                Visibility::Inherited,
            ),
            // The handle is how you get the panel back, so it never hides.
            Slot::Handle => (handle_rect(&window), Visibility::Inherited),
            other => (
                laid_out
                    .iter()
                    .find(|row| row.slot == *other)
                    .map(|row| row.rect)
                    .unwrap_or(Rect::EMPTY),
                shown,
            ),
        };

        let at = match slot {
            // Headings are left-aligned, so they are placed by their left edge
            // rather than their middle.
            Slot::Heading(_) => Vec2::new(rect.min.x, rect.center().y),
            _ => rect.center(),
        };
        transform.translation.x = at.x;
        transform.translation.y = at.y;

        // Sprites take their size here rather than at spawn, so a resized
        // window restretches the slab instead of leaving it the old shape.
        if let Some(mut sprite) = sprite {
            sprite.custom_size = Some(rect.size());
        }

        *visibility = seen;
    }
}

/// Colours each button by what it is doing, and says what the pointer is over.
/// What the panel reads to decide what to say. A bundle for the same reason
/// [`super::Bench`] is one: a run of positional resources is a run nobody reads.
#[derive(SystemParam)]
pub struct Readouts<'w> {
    tools: Res<'w, ToolState>,
    last: Res<'w, super::record::LastRecording>,
    tutor: Res<'w, super::tutor::Tutor>,
    world: Res<'w, crate::sim::Simulation>,
}

pub fn draw(
    window: Single<&Window>,
    pointer: Res<Pointer>,
    said: Readouts,
    mut fills: Query<(&Slot, &mut Sprite)>,
    mut texts: Query<(&Slot, &mut Text2d, &mut TextColor)>,
) {
    let Readouts {
        tools,
        last,
        tutor,
        world,
    } = said;
    let hovered = pointer.at.and_then(|at| hit(at, &window, &tools));

    for (slot, mut sprite) in &mut fills {
        sprite.color = match slot {
            Slot::Handle if hovered == Some(Slot::Handle) => FILL_HOVER,
            Slot::Handle => FILL_IDLE,
            Slot::Button(index) => fill_for(*index, hovered, &tools),
            _ => continue,
        };
    }

    // Two lines, always. The old version showed the hovered button's hint *or*
    // the last result, and cycling a list is exactly the case where you are
    // still hovering the button that produced the answer you wanted to read —
    // so `sigil >` looked like it was doing nothing at all.
    let doing = match hovered {
        // The preset buttons answer with *which* preset, not just what the
        // button does. Hovering `preset` and being told "lay down a seal" is
        // no help at all when the question is which seal you are about to lay
        // down — and that is the moment you are hovering it.
        Some(Slot::Button(index))
            if Some(index) == tool_named("seal") || Some(index) == tool_named("seal >") =>
        {
            match crate::sim::PRESETS.get(tools.preset) {
                Some(preset) => format!(
                    "{}  ({}/{})  {}  |  {} keystone(s) aimed {}, ring {}  |  {}",
                    preset.id,
                    tools.preset + 1,
                    crate::sim::PRESETS.len(),
                    preset.blurb,
                    preset.signs,
                    if preset.inward { "IN" } else { "out" },
                    if preset.open {
                        "OPEN - you close it"
                    } else {
                        "closed"
                    },
                    // The fixture's own effect line, straight from
                    // `spells.ron`. The preset's blurb says how it is built;
                    // this says what it does.
                    match world.lore.as_ref() {
                        Some(catalog) => crate::sim::describe_fixture(catalog, preset.id),
                        None => "catalogue failed to load".to_string(),
                    },
                ),
                None => TOOLS[index].hint.to_string(),
            }
        }
        Some(Slot::Button(index)) => TOOLS[index].hint.to_string(),
        Some(Slot::Handle) => "show and hide the tools  (Tab)".to_string(),
        // Nothing hovered: say what a click on the paper would do right now.
        // It is the one piece of state no label can show.
        _ => match tools.mode {
            Mode::Pen => format!(
                "pen | drag to draw | shapes would land at r{:.0}, gap {:.0}deg",
                tools.stamp_radius, tools.stamp_gap
            ),
            Mode::Place(shape) => format!(
                "{shape:?} armed | click to place, drag to size | r{:.0}, gap {:.0}deg",
                tools.stamp_radius, tools.stamp_gap
            ),
        },
    };

    // The lesson outranks both. While the guide is on, the one line that
    // matters is what to draw next — a hover hint about a button you are not
    // pressing is noise on top of an instruction.
    let hint = match (tutor.placed, last.0.as_deref()) {
        (true, _) => {
            // The guide teaches whatever `preset >` has chosen. It used to
            // have a second selector of its own, which meant two lists to keep
            // in step and one more button to find.
            let (lesson, sigil, signs) = crate::sim::PRESETS
                .get(tools.preset)
                .map_or(("", "", 0), |preset| {
                    (preset.id, preset.sigil, preset.signs)
                });
            format!(
                "guide: {lesson}\n{}",
                tutor.step.asks(lesson, Some(sigil), signs)
            )
        }
        (false, Some(said)) => format!("{doing}\n{said}"),
        (false, None) => doing,
    };

    for (slot, mut text, mut color) in &mut texts {
        match slot {
            Slot::Hint => text.0 = hint.clone(),
            // Dark text on the one fill bright enough to swallow pale text.
            Slot::Button(index) => {
                color.0 = if fill_for(*index, hovered, &tools) == FILL_ARMED {
                    LABEL_ARMED
                } else {
                    LABEL
                };
            }
            _ => {}
        }
    }
}

/// The colour a button should be, given what is armed and what is hovered.
///
/// Its own function because [`draw`] asks twice — once for the fill and once
/// to decide whether the label needs to be dark against it.
fn fill_for(index: usize, hovered: Option<Slot>, tools: &ToolState) -> Color {
    let tool: &Tool = &TOOLS[index];

    let lit = match tool.action {
        Action::Pick(mode) => tools.mode == mode,
        Action::Toggle(toggle) => super::is_on(toggle, tools),
        Action::Run(_) => false,
    };

    match (lit, tool.action, hovered) {
        (true, Action::Pick(_), _) => FILL_ARMED,
        (true, _, _) => FILL_ON,
        (_, _, Some(Slot::Button(over))) if over == index => FILL_HOVER,
        _ => FILL_IDLE,
    }
}
