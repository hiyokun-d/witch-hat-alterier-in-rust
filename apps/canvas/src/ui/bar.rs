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

/// The slab everything sits on. Dark enough to read over parchment and over
/// the desk, translucent enough that ink underneath is not lost.
const PANEL: Color = Color::srgba(0.10, 0.08, 0.07, 0.90);
const FILL_IDLE: Color = Color::srgba(0.21, 0.18, 0.15, 1.0);
const FILL_HOVER: Color = Color::srgba(0.32, 0.27, 0.22, 1.0);
/// Armed: the pad is about to do this. Warm, because it is the loud state.
const FILL_ARMED: Color = Color::srgba(0.76, 0.49, 0.16, 1.0);
/// A toggle that is on. Quiet, because it is a background condition.
const FILL_ON: Color = Color::srgba(0.16, 0.44, 0.31, 1.0);

const LABEL: Color = Color::srgb(0.93, 0.89, 0.81);
/// Dark text for the one fill bright enough to need it.
const LABEL_ARMED: Color = Color::srgb(0.13, 0.10, 0.07);
const HEADING: Color = Color::srgb(0.58, 0.50, 0.40);
const HINT: Color = Color::srgb(0.72, 0.65, 0.54);

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
            rect: Rect::from_center_size(Vec2::new(x, *top - height * 0.5), Vec2::new(width, height)),
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

/// Where the hint sits — just under the slab, right-aligned to its edge.
fn hint_at(window: &Window) -> Vec2 {
    let panel = panel_rect(window);
    Vec2::new(panel.max.x, panel.min.y - 6.0)
}

pub fn spawn(mut commands: Commands) {
    let label_font = TextFont {
        font_size: FontSize::Px(13.0),
        ..default()
    };
    let small_font = TextFont {
        font_size: FontSize::Px(11.0),
        ..default()
    };

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

    commands.spawn((
        Text2d::new(""),
        small_font,
        TextColor(HINT),
        // Grows leftward from the panel's edge, so a long hint never runs off
        // the right of the window.
        Anchor::TOP_RIGHT,
        Transform::from_xyz(0.0, 0.0, PANEL_Z + 1.0),
        Slot::Hint,
    ));
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
                &window,
                &mut controls.last,
                &mut controls.world,
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
            // The hint is the only readout when the panel is shut, so it
            // stays: hovering the handle still explains itself.
            Slot::Hint => (
                Rect::from_center_size(hint, Vec2::ZERO),
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
pub fn draw(
    window: Single<&Window>,
    pointer: Res<Pointer>,
    tools: Res<ToolState>,
    last: Res<super::record::LastRecording>,
    mut fills: Query<(&Slot, &mut Sprite)>,
    mut texts: Query<(&Slot, &mut Text2d, &mut TextColor)>,
) {
    let hovered = pointer.at.and_then(|at| hit(at, &window, &tools));

    for (slot, mut sprite) in &mut fills {
        sprite.color = match slot {
            Slot::Handle if hovered == Some(Slot::Handle) => FILL_HOVER,
            Slot::Handle => FILL_IDLE,
            Slot::Button(index) => fill_for(*index, hovered, &tools),
            _ => continue,
        };
    }

    let hint = match hovered {
        Some(Slot::Button(index)) => TOOLS[index].hint.to_string(),
        // A recording outranks the idle hint until something is hovered, so the
        // one thing the button produces is not lost the frame after it happens.
        None if last.0.is_some() => last.0.clone().unwrap_or_default(),
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
