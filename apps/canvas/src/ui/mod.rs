//! The toolbar: everything you can do to the pad without drawing it.
//!
//! Drawing a clean ring by hand is a skill, and needing that skill before the
//! engine will do anything is a bad way to build the engine. These tools stamp
//! exact ink, lay down guides to draw along, and switch the overlays on and
//! off — so a seal can be assembled and compiled by someone who cannot draw a
//! circle, and so a bug can be reproduced from a button instead of a steady
//! hand.
//!
//! # Adding a tool
//!
//! Two edits, both in this directory:
//!
//! 1. Add a [`Tool`] to [`TOOLS`] below, naming a [`Toggle`] or a [`Command`].
//! 2. Give it behaviour — a field on [`ToolState`] for a toggle, or an arm in
//!    [`run`] for a command.
//!
//! Nothing else needs touching. The bar lays itself out from `TOOLS`, so a new
//! entry appears, wraps onto a second row if it must, and becomes clickable
//! without any layout work.
//!
//! # What lives where
//!
//! | File | Holds |
//! | --- | --- |
//! | `mod.rs` | the plugin, [`ToolState`], the tool table, what a click does |
//! | `bar.rs` | where the buttons are, how they draw, hit-testing |
//! | `guides.rs` | the helper lines you draw along |
//! | `stamp.rs` | generating exact ink and putting it on the pad |
//!
//! # Why not `bevy_ui`
//!
//! The shell pulls in `2d`, `bevy_text` and a default font, and nothing else.
//! A row of rectangles hit-tested against the cursor needs none of the layout
//! engine, and the overlay already proves the pattern works. If the toolbar
//! ever grows scrolling or text entry, that is the moment to reconsider.

use bevy::prelude::*;

use crate::InkPad;
use crate::shortcuts;

pub mod bar;
pub mod guides;
pub mod stamp;

/// Everything the toolbar can switch on and off.
///
/// A plain resource of flags rather than a signal to each feature, because the
/// bar has to *show* the current state as well as change it, and one place
/// holding the truth is the only way those two cannot disagree.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ToolState {
    /// Whether the bar itself is showing. Tab, or the handle at the left.
    pub bar: bool,
    /// The debug overlay. Read by `debug.rs`, which ANDs it with its own F1.
    pub debug_overlay: bool,
    /// Guide rings and spokes to draw along.
    pub guides: bool,
    /// Radius the stamps are laid down at.
    pub stamp_radius: f32,
    /// Size of the hole a stamped arc leaves, in degrees.
    pub stamp_gap: f32,
}

impl Default for ToolState {
    fn default() -> Self {
        ToolState {
            // Open on first run: a toolbar nobody knows about helps nobody.
            bar: true,
            debug_overlay: true,
            guides: false,
            stamp_radius: 140.0,
            stamp_gap: 40.0,
        }
    }
}

/// Least and most a stamp may be, and how much a nudge moves it.
const RADIUS_RANGE: (f32, f32) = (30.0, 400.0);
const RADIUS_STEP: f32 = 20.0;
const GAP_RANGE: (f32, f32) = (0.0, 180.0);
const GAP_STEP: f32 = 10.0;

/// Where the pointer is, and whether the toolbar has claimed it.
///
/// Recomputed before capture every frame so a click on a button never also
/// lands a blot of ink underneath it.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Pointer {
    pub at: Option<Vec2>,
    pub over_ui: bool,
}

/// Run condition: the pointer is over the pad rather than over a button.
///
/// `main.rs` gates `capture_stroke` on this.
pub fn pointer_free(pointer: Res<Pointer>) -> bool {
    !pointer.over_ui
}

/// A flag a button flips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Bar,
    Debug,
    Guides,
}

/// A one-shot a button fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    StampRing,
    StampArc,
    StampCross,
    Bigger,
    Smaller,
    WiderGap,
    NarrowerGap,
    Undo,
    Redo,
    Clear,
    ClearAll,
    SwapPaper,
}

/// What a button does when clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Toggle(Toggle),
    Run(Command),
}

/// One button.
#[derive(Debug, Clone, Copy)]
pub struct Tool {
    /// What the button says. Kept short — the bar sizes itself off these.
    pub label: &'static str,
    /// One line of what it does, shown when the pointer is over it.
    pub hint: &'static str,
    pub action: Action,
}

/// Every button on the bar, in the order they appear.
///
/// Grouped by what they act on: ink first, because that is why the bar exists;
/// then the size of what gets stamped; then the pad; then the overlays.
pub const TOOLS: &[Tool] = &[
    Tool {
        label: "ring",
        hint: "a perfect closed ring — activates on its own",
        action: Action::Run(Command::StampRing),
    },
    Tool {
        label: "arc",
        hint: "a ring with a deliberate hole — canon rule 2's prepared spell",
        action: Action::Run(Command::StampArc),
    },
    Tool {
        label: "cross",
        hint: "two strokes inside the ring, standing in for a sigil",
        action: Action::Run(Command::StampCross),
    },
    Tool {
        label: "radius +",
        hint: "stamp bigger — canon rule 8, larger seals are stronger",
        action: Action::Run(Command::Bigger),
    },
    Tool {
        label: "radius -",
        hint: "stamp smaller",
        action: Action::Run(Command::Smaller),
    },
    Tool {
        label: "gap +",
        hint: "widen the hole an arc leaves",
        action: Action::Run(Command::WiderGap),
    },
    Tool {
        label: "gap -",
        hint: "narrow the hole — take it to zero and the arc closes",
        action: Action::Run(Command::NarrowerGap),
    },
    Tool {
        label: "undo",
        hint: "lift the last stroke off the pad  (⌘Z)",
        action: Action::Run(Command::Undo),
    },
    Tool {
        label: "redo",
        hint: "put it back  (⇧⌘Z)",
        action: Action::Run(Command::Redo),
    },
    Tool {
        label: "clear",
        hint: "empty the pad, recoverably  (⌘⌫)",
        action: Action::Run(Command::Clear),
    },
    Tool {
        label: "wipe",
        hint: "empty the pad and its history  (⇧⌘⌫)",
        action: Action::Run(Command::ClearAll),
    },
    Tool {
        label: "paper",
        hint: "full sheet or a round one  (F three times)",
        action: Action::Run(Command::SwapPaper),
    },
    Tool {
        label: "guides",
        hint: "rings and spokes to draw along",
        action: Action::Toggle(Toggle::Guides),
    },
    Tool {
        label: "debug",
        hint: "the measurement overlay  (F1)",
        action: Action::Toggle(Toggle::Debug),
    },
];

/// Adds the toolbar. Remove this line and `mod ui;` and drawing still works —
/// everything here is a convenience over things the keyboard already does.
pub struct ToolbarPlugin;

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ToolState>()
            .init_resource::<Pointer>()
            .add_systems(Startup, bar::spawn)
            .add_systems(
                Update,
                // Before capture, so a click that lands on a button is known to
                // be a button press before the pad gets a chance to ink it.
                (bar::track_pointer, bar::click, toggle_with_keyboard)
                    .chain()
                    .before(crate::capture_stroke),
            )
            .add_systems(
                Update,
                (bar::layout, bar::draw, guides::draw).after(crate::capture_stroke),
            );
    }
}

/// Tab shows and hides the bar.
///
/// Bare, like `F` for the paper, and deliberately not a ⌘ chord: the bar is
/// something you flick away while drawing, not a command.
fn toggle_with_keyboard(keys: Res<ButtonInput<KeyCode>>, mut tools: ResMut<ToolState>) {
    if keys.just_pressed(KeyCode::Tab) && !shortcuts::command_held(&keys) {
        tools.bar = !tools.bar;
    }
}

/// Carries out one button press.
///
/// Everything a command does, some keyboard shortcut already did. That is on
/// purpose — the bar is a second way in, never the only way, so nothing here
/// can become the sole route to a feature.
pub fn run(
    command: Command,
    tools: &mut ToolState,
    pad: &mut InkPad,
    shape: &mut crate::PaperShape,
    window: &Window,
) {
    match command {
        Command::StampRing => stamp::place(pad, stamp::ring(tools.stamp_radius)),
        Command::StampArc => {
            stamp::place(pad, stamp::arc(tools.stamp_radius, tools.stamp_gap));
        }
        Command::StampCross => {
            // Sized against the ring rather than the window, so it lands
            // inside whatever was stamped last and reads as its contents.
            stamp::place(pad, stamp::cross(tools.stamp_radius * 0.45));
        }

        Command::Bigger => {
            // Clamped to the sheet as well as to its own range: a stamp bigger
            // than the paper would be silently refused by `PaperShape::accepts`
            // and look like a broken button.
            let room = shape.extent(window).min_element();
            tools.stamp_radius =
                (tools.stamp_radius + RADIUS_STEP).clamp(RADIUS_RANGE.0, RADIUS_RANGE.1.min(room));
        }
        Command::Smaller => {
            tools.stamp_radius = (tools.stamp_radius - RADIUS_STEP).max(RADIUS_RANGE.0);
        }
        Command::WiderGap => {
            tools.stamp_gap = (tools.stamp_gap + GAP_STEP).clamp(GAP_RANGE.0, GAP_RANGE.1);
        }
        Command::NarrowerGap => {
            tools.stamp_gap = (tools.stamp_gap - GAP_STEP).clamp(GAP_RANGE.0, GAP_RANGE.1);
        }

        Command::Undo => shortcuts::undo(pad),
        Command::Redo => shortcuts::redo(pad),
        Command::Clear => shortcuts::clear(pad),
        Command::ClearAll => shortcuts::clear_all(pad),

        Command::SwapPaper => {
            // Swapping wipes the pad, exactly as the keyboard path does — ink
            // drawn on a full sheet has no meaning on a round one.
            *shape = match *shape {
                crate::PaperShape::Full => crate::PaperShape::Disc,
                crate::PaperShape::Disc => crate::PaperShape::Full,
            };
            shortcuts::clear_all(pad);
        }
    }
}

/// Reads a toggle's current value, so the bar can show it lit.
pub fn is_on(toggle: Toggle, tools: &ToolState) -> bool {
    match toggle {
        Toggle::Bar => tools.bar,
        Toggle::Debug => tools.debug_overlay,
        Toggle::Guides => tools.guides,
    }
}

/// Flips a toggle.
pub fn flip(toggle: Toggle, tools: &mut ToolState) {
    match toggle {
        Toggle::Bar => tools.bar = !tools.bar,
        Toggle::Debug => tools.debug_overlay = !tools.debug_overlay,
        Toggle::Guides => tools.guides = !tools.guides,
    }
}
