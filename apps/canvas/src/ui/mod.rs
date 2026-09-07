//! The tool panel: everything you can do to the pad without drawing it.
//!
//! Drawing a clean ring by hand is a skill, and needing that skill before the
//! engine will do anything is a bad way to build the engine. These tools place
//! exact ink, lay down guides to draw along, and switch the overlays on and
//! off — so a seal can be assembled and compiled by someone who cannot draw a
//! circle, and so a bug can be reproduced from a button instead of a steady
//! hand.
//!
//! # How it feels to use
//!
//! Pick a shape, then put it somewhere. Clicking `ring` does not stamp a ring;
//! it *arms* one. The pad then behaves like the shape tool in any drawing
//! program:
//!
//! - **click** — places it where you clicked, at the current radius
//! - **drag** — the press point is the centre and the distance is the radius,
//!   with a live preview of exactly what will land
//! - **Escape**, or picking `pen` — back to drawing by hand
//!
//! Arming rather than stamping is what makes the panel feel like a tool
//! instead of a vending machine: the same button can produce a ring anywhere,
//! at any size, and you can see it before you commit.
//!
//! # Adding a tool
//!
//! Two edits, both in this directory:
//!
//! 1. Add a [`Tool`] to [`TOOLS`] below, naming a [`Mode`], [`Toggle`] or
//!    [`Command`].
//! 2. Give it behaviour — a variant of [`Shape`] and an arm in
//!    [`stamp::draw_shape`], a field on [`ToolState`], or an arm in [`run`].
//!
//! Nothing else needs touching. The panel lays itself out from `TOOLS`, so a
//! new entry appears in its section, moves everything below it down, and
//! becomes clickable without any layout work.
//!
//! # What lives where
//!
//! | File | Holds |
//! | --- | --- |
//! | `mod.rs` | the plugin, [`ToolState`], the tool table, what a click does |
//! | `bar.rs` | where the buttons are, how they draw, hit-testing |
//! | `place.rs` | pick-then-place: the drag, the preview, the commit |
//! | `guides.rs` | the helper lines you draw along |
//! | `stamp.rs` | generating exact ink |
//!
//! # Why not `bevy_ui`
//!
//! The shell pulls in `2d`, `bevy_text` and a default font, and nothing else.
//! A column of rectangles hit-tested against the cursor needs none of the
//! layout engine, and the overlay already proves the pattern works. If the
//! panel ever grows scrolling or text entry, that is the moment to reconsider.

use bevy::prelude::*;

use crate::InkPad;
use crate::shortcuts;

pub mod bar;
pub mod guides;
pub mod place;
pub mod record;
pub mod stamp;

/// A shape the panel can place for you.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A closed ring. Its ends meet exactly, so it reads as `Active` the
    /// moment it lands — which a hand-drawn ring almost never does.
    Ring,
    /// A ring with a deliberate hole. Canon rule 2's prepared spell.
    Arc,
    /// Two crossed strokes to put *inside* a ring. Not a canon sigil, and not
    /// pretending to be — see [`stamp::cross`].
    Cross,
    /// A keystone: a mark with a length and a direction, to arrange *around* a
    /// sigil. Drag aims it; §2.4 makes both the length and the aim matter.
    Sign,
    /// A claw. Neither sign nor sigil, and the only mark canon lets you draw
    /// outside the ring — place it straddling the line.
    Glaive,
}

/// What the pad does with a press.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Ink follows the pen. The default, and what Escape returns to.
    #[default]
    Pen,
    /// A shape is armed, waiting to be clicked or dragged into place.
    Place(Shape),
}

/// Everything the panel can switch on and off.
///
/// A plain resource of flags rather than a signal to each feature, because the
/// panel has to *show* the current state as well as change it, and one place
/// holding the truth is the only way those two cannot disagree.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ToolState {
    /// Whether the panel is showing. Tab, or the handle at the top.
    pub open: bool,
    /// What a press on the pad means right now.
    pub mode: Mode,
    /// The debug overlay. Read by `debug.rs`, which falls back to its own F1.
    pub debug_overlay: bool,
    /// Guide rings and spokes to draw along.
    pub guides: bool,
    /// Radius a click places at, and the size a drag leaves behind.
    pub stamp_radius: f32,
    /// Size of the hole a placed arc leaves, in degrees.
    pub stamp_gap: f32,
    /// The compiled-spell readout: what the seal on the pad will actually do.
    pub spell: bool,
    /// The recogniser board: every recorded rune scored against the ink.
    pub runes: bool,
    /// The simulation overlay: parcels, cell density, and what it is doing.
    pub sim: bool,
}

impl Default for ToolState {
    fn default() -> Self {
        ToolState {
            // Open on first run: a panel nobody knows about helps nobody.
            open: true,
            mode: Mode::Pen,
            debug_overlay: true,
            guides: false,
            stamp_radius: 140.0,
            stamp_gap: 40.0,
            spell: true,
            runes: true,
            sim: true,
        }
    }
}

/// Least and most a placed shape may be, and how much a nudge moves it.
pub const RADIUS_RANGE: (f32, f32) = (24.0, 460.0);
const RADIUS_STEP: f32 = 20.0;
const GAP_RANGE: (f32, f32) = (0.0, 180.0);
const GAP_STEP: f32 = 10.0;

/// Where the pointer is, and whether the panel has claimed it.
///
/// Recomputed before capture every frame so a click on a button never also
/// lands a blot of ink underneath it.
#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct Pointer {
    pub at: Option<Vec2>,
    pub over_ui: bool,
}

/// Run condition: a press right now means ink.
///
/// False over a button, and false whenever a shape is armed — in that mode the
/// pad belongs to [`place`], and letting both have the press would leave a
/// scribble under every placed ring. `main.rs` gates `capture_stroke` on this.
pub fn pointer_free(pointer: Res<Pointer>, tools: Res<ToolState>) -> bool {
    !pointer.over_ui && tools.mode == Mode::Pen
}

/// A flag a button flips.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Panel,
    Debug,
    Guides,
    Spell,
    Runes,
    Sim,
}

/// A one-shot a button fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Bigger,
    Smaller,
    WiderGap,
    NarrowerGap,
    Undo,
    Redo,
    Clear,
    ClearAll,
    SwapPaper,
    /// Writes the pad's non-ring strokes out as a `templates.ron` fragment.
    Record,
    /// Compiles everything on the pad and casts it into the world.
    Cast,
    /// Runs the world, or stops it.
    PlayPause,
    /// One tick, so a frame can be read rather than watched.
    StepOnce,
    /// Empties the world without touching the ink.
    ClearWorld,
}

/// What a button does when clicked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Arms something. Lights up while it is what the pad will do.
    Pick(Mode),
    Toggle(Toggle),
    Run(Command),
}

/// One button.
#[derive(Debug, Clone, Copy)]
pub struct Tool {
    /// Which block of the panel it belongs to. A new name starts a new block.
    pub section: &'static str,
    /// What the button says. Kept short — the panel sizes itself off these.
    pub label: &'static str,
    /// One line of what it does, shown when the pointer is over it.
    pub hint: &'static str,
    pub action: Action,
}

/// Every button on the panel, top to bottom.
///
/// Order is the layout: entries are drawn in this sequence and a change of
/// `section` starts a new block with a heading.
pub const TOOLS: &[Tool] = &[
    Tool {
        section: "place",
        label: "pen",
        hint: "draw by hand  (Esc)",
        action: Action::Pick(Mode::Pen),
    },
    Tool {
        section: "place",
        label: "ring",
        hint: "click or drag out a closed ring - fires on its own",
        action: Action::Pick(Mode::Place(Shape::Ring)),
    },
    Tool {
        section: "place",
        label: "arc",
        hint: "a ring with a hole - drag to aim the hole. Rule 2's prepared spell",
        action: Action::Pick(Mode::Place(Shape::Arc)),
    },
    Tool {
        section: "place",
        label: "cross",
        hint: "two strokes to put inside a ring, standing in for a sigil",
        action: Action::Pick(Mode::Place(Shape::Cross)),
    },
    Tool {
        section: "place",
        label: "sign",
        hint: "a keystone - drag to aim it, set its length. Size is power (canon 2.4)",
        action: Action::Pick(Mode::Place(Shape::Sign)),
    },
    Tool {
        section: "place",
        label: "glaive",
        hint: "a claw - how firmly the spell embeds. Straddle the ring with it",
        action: Action::Pick(Mode::Place(Shape::Glaive)),
    },
    Tool {
        section: "size",
        label: "radius +",
        hint: "place bigger - canon rule 8, larger seals are stronger",
        action: Action::Run(Command::Bigger),
    },
    Tool {
        section: "size",
        label: "radius -",
        hint: "place smaller",
        action: Action::Run(Command::Smaller),
    },
    Tool {
        section: "size",
        label: "gap +",
        hint: "widen the hole an arc leaves",
        action: Action::Run(Command::WiderGap),
    },
    Tool {
        section: "size",
        label: "gap -",
        hint: "narrow the hole - take it to zero and the arc closes",
        action: Action::Run(Command::NarrowerGap),
    },
    Tool {
        section: "pad",
        label: "undo",
        hint: "lift the last stroke off the pad  (Cmd Z)",
        action: Action::Run(Command::Undo),
    },
    Tool {
        section: "pad",
        label: "redo",
        hint: "put it back  (Shift Cmd Z)",
        action: Action::Run(Command::Redo),
    },
    Tool {
        section: "pad",
        label: "clear",
        hint: "empty the pad, recoverably  (Cmd Del)",
        action: Action::Run(Command::Clear),
    },
    Tool {
        section: "pad",
        label: "wipe",
        hint: "empty the pad and its history  (Shift Cmd Del)",
        action: Action::Run(Command::ClearAll),
    },
    Tool {
        section: "pad",
        label: "paper",
        hint: "full sheet or a round one  (F three times)",
        action: Action::Run(Command::SwapPaper),
    },
    Tool {
        section: "pad",
        label: "record",
        hint: "write the rune on the pad to recorded-gesture.ron",
        action: Action::Run(Command::Record),
    },
    Tool {
        section: "view",
        label: "guides",
        hint: "rings and spokes to draw along",
        action: Action::Toggle(Toggle::Guides),
    },
    Tool {
        section: "view",
        label: "spell",
        hint: "what the seal compiles to - driver, firing, balance, warnings",
        action: Action::Toggle(Toggle::Spell),
    },
    Tool {
        section: "cast",
        label: "cast",
        hint: "compile every seal on the pad and fire it into the world",
        action: Action::Run(Command::Cast),
    },
    Tool {
        section: "cast",
        label: "run",
        hint: "start or stop the simulation - it steps at a fixed 60Hz",
        action: Action::Run(Command::PlayPause),
    },
    Tool {
        section: "cast",
        label: "tick",
        hint: "one step, so a frame can be read instead of watched",
        action: Action::Run(Command::StepOnce),
    },
    Tool {
        section: "cast",
        label: "empty",
        hint: "clear the world, leaving the ink alone",
        action: Action::Run(Command::ClearWorld),
    },
    Tool {
        section: "view",
        label: "world",
        hint: "the simulation overlay - parcels, density, what it is doing",
        action: Action::Toggle(Toggle::Sim),
    },
    Tool {
        section: "view",
        label: "runes",
        hint: "score the ink against every recorded rune - record adds one",
        action: Action::Toggle(Toggle::Runes),
    },
    Tool {
        section: "view",
        label: "debug",
        hint: "the measurement overlay  (F1)",
        action: Action::Toggle(Toggle::Debug),
    },
];

/// Adds the tool panel. Remove this line and `mod ui;` and drawing still
/// works — everything here is a convenience over things the pen already does.
pub struct ToolbarPlugin;

impl Plugin for ToolbarPlugin {
    fn build(&self, app: &mut App) {
        guides::setup(app);

        app.init_resource::<ToolState>()
            .init_resource::<Pointer>()
            .init_resource::<place::Placing>()
            .init_resource::<record::LastRecording>()
            .add_systems(Startup, bar::spawn)
            .add_systems(
                Update,
                // Before capture, so a press that lands on a button — or on the
                // pad with a shape armed — is claimed before the pad inks it.
                (bar::track_pointer, bar::click, place::drag, keyboard)
                    .chain()
                    .before(crate::capture_stroke),
            )
            .add_systems(
                Update,
                (bar::layout, bar::draw, guides::draw, place::preview).after(crate::capture_stroke),
            );
    }
}

/// Tab shows and hides the panel; Escape puts the pen back in your hand.
///
/// Both bare, like `F` for the paper, and deliberately not ⌘ chords: these are
/// things you reach for mid-drawing, not commands.
fn keyboard(keys: Res<ButtonInput<KeyCode>>, mut tools: ResMut<ToolState>) {
    if shortcuts::command_held(&keys) {
        return;
    }
    if keys.just_pressed(KeyCode::Tab) {
        tools.open = !tools.open;
    }
    if keys.just_pressed(KeyCode::Escape) {
        tools.mode = Mode::Pen;
    }
}

/// Carries out one button press.
///
/// Everything a command does, some keyboard shortcut already did. That is on
/// purpose — the panel is a second way in, never the only way, so nothing here
/// can become the sole route to a feature.
pub fn run(
    command: Command,
    tools: &mut ToolState,
    pad: &mut InkPad,
    shape: &mut crate::PaperShape,
    window: &Window,
    last: &mut record::LastRecording,
    world: &mut crate::sim::Simulation,
) {
    // Clamped against the sheet, not just its own range: the window can be
    // resized and the paper swapped for a smaller round one without anyone
    // touching the radius.
    let room = shape.extent(window).min_element();

    match command {
        Command::Bigger => {
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

        Command::Cast => {
            // The result goes on the hint line for the same reason recording's
            // does: nobody is reading a terminal while drawing.
            let said = world.cast_pad(pad);
            world.last = Some(said.clone());
            last.0 = Some(said);
        }
        Command::PlayPause => {
            world.running = !world.running;
            last.0 = Some(if world.running {
                "world running".to_string()
            } else {
                format!("world paused at tick {}", world.sim.ticks)
            });
        }
        Command::StepOnce => {
            // Stops first, so a click on `tick` always advances exactly one.
            world.running = false;
            world.sim.step();
            last.0 = Some(format!("tick {}", world.sim.ticks));
        }
        Command::ClearWorld => {
            world.sim.reset();
            world.running = false;
            world.last = None;
            last.0 = Some("world emptied".to_string());
        }

        Command::Record => {
            // The result goes on the panel rather than into a log: the point of
            // the tool is knowing whether the file got written, and a terminal
            // is not where anyone is looking while drawing.
            last.0 = Some(match record::record(pad) {
                Ok(message) => message,
                Err(why) => format!("nothing written - {why}"),
            });
        }
    }
}

/// Reads a toggle's current value, so the panel can show it lit.
pub fn is_on(toggle: Toggle, tools: &ToolState) -> bool {
    match toggle {
        Toggle::Panel => tools.open,
        Toggle::Debug => tools.debug_overlay,
        Toggle::Guides => tools.guides,
        Toggle::Spell => tools.spell,
        Toggle::Runes => tools.runes,
        Toggle::Sim => tools.sim,
    }
}

/// Flips a toggle.
pub fn flip(toggle: Toggle, tools: &mut ToolState) {
    match toggle {
        Toggle::Panel => tools.open = !tools.open,
        Toggle::Debug => tools.debug_overlay = !tools.debug_overlay,
        Toggle::Guides => tools.guides = !tools.guides,
        Toggle::Spell => tools.spell = !tools.spell,
        Toggle::Runes => tools.runes = !tools.runes,
        Toggle::Sim => tools.sim = !tools.sim,
    }
}
