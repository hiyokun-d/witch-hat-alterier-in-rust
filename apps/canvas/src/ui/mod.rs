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
pub mod tutor;

/// A shape the panel can place for you.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A closed ring. Its ends meet exactly, so it reads as `Active` the
    /// moment it lands — which a hand-drawn ring almost never does.
    Ring,
    /// A ring with a deliberate hole. Canon rule 2's prepared spell.
    Arc,
    /// A keystone: a mark with a length and a direction, to arrange *around* a
    /// sigil. Drag aims it; §2.4 makes both the length and the aim matter.
    Sign,
    /// A claw. Neither sign nor sigil, and the only mark canon lets you draw
    /// outside the ring — place it straddling the line.
    Glaive,
    /// Ink that turns more than once. The ring search must *reject* this, and
    /// this is how to make one without a steady hand.
    Spiral,
    /// One of core's marks — a sigil or a sign — placed like any other shape.
    ///
    /// Drag sizes it, exactly as a ring or an arc. The element buttons used to
    /// stamp at the middle of the pad at a fixed size, which is a different
    /// interaction from every other thing on the panel for no reason anyone
    /// could give.
    Mark(&'static str),
    /// Where the guide's lesson sits. Only ever one: placing again moves it.
    Guide,
    /// Rubs out whatever stroke you click on.
    ///
    /// `clear` and `wipe` are all-or-nothing, and a seal is a dozen strokes —
    /// one bad keystone should not cost the ring. Erasing by stroke is the
    /// smallest edit the pad can express, since a stroke is what capture
    /// records and what `undo` removes.
    Erase,
    /// The line joining two seals — canon rule 5. Drag from one ring to
    /// another. Until this existed, rule 5 had no way in from the panel.
    Link,
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
    /// The tracing board. Its own panel and its own toggle, deliberately: the
    /// recogniser board answers "what did the engine make of this ring", and
    /// tracing asks "is the sample I am about to save any good". Two questions,
    /// two blocks, so neither has to be read past to reach the other.
    pub trace: bool,
    /// The simulation overlay: parcels, cell density, and what it is doing.
    pub sim: bool,
    /// Whether only runes that were actually traced may name a mark or fire a
    /// seal.
    ///
    /// On by default, and it replaces the naming override outright. That
    /// override existed because `templates.ron` shipped empty and the compiler
    /// was otherwise unreachable from the app — a real problem then, solved now
    /// that runes are traced, and its cost was that the one button guaranteed
    /// to produce a working spell was the one button that bypassed the
    /// recogniser. See `sim::refuses` and `reading::load_shapes`.
    pub only_traced: bool,
    /// Which substance the `pour` button drops.
    pub pour: usize,
    /// What the `kindle` button scatters, by position in
    /// [`crate::props::KINDLING`].
    pub kindling: usize,
    /// Which seal the `preset` button lays down.
    pub preset: usize,
    /// Which rune the next `record` is traced *as*, by position in the
    /// catalogue's sigils. `None` falls back to a numbered placeholder.
    pub tracing: Option<usize>,
    /// Whether the world has edges. Off, what leaves is gone and the mass
    /// total visibly falls — which is the honest way to show a leak.
    pub walls: bool,
    /// Fire a seal the moment its ring closes, with no button.
    ///
    /// Lives on the panel rather than on the world because it is a statement
    /// about how the *tool* behaves, not about the simulation.
    pub auto: bool,
}

impl Default for ToolState {
    fn default() -> Self {
        ToolState {
            // Open on first run: a panel nobody knows about helps nobody.
            open: true,
            mode: Mode::Pen,
            debug_overlay: false,
            guides: false,
            stamp_radius: 140.0,
            stamp_gap: 40.0,
            spell: true,
            runes: true,
            trace: true,
            sim: true,
            only_traced: true,
            pour: 0,
            kindling: 0,
            preset: 0,
            auto: true,
            walls: true,
            tracing: None,
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
    /// Which button the pointer is over, by index into [`TOOLS`].
    ///
    /// Recorded so something other than the hint line can react to a hover —
    /// the preset preview needs to know, and asking `bar` to hit-test again
    /// from a second place would be two answers that can disagree.
    pub over_tool: Option<usize>,
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
    /// The tracing board: what `record` will save, and how close it is.
    Trace,
    /// Only traced runes may name a mark or fire a seal.
    Traced,
    Sim,
    /// Fire a seal the moment its ring closes, with no button. Canon rule 2.
    Auto,
    /// The edges of the world.
    Walls,
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
    /// Lays a whole seal down and names it, so nothing has to be drawn.
    Preset,
    /// Chooses which preset `preset` lays down.
    NextPreset,
    /// Chooses which rune the next recording is traced as.
    NextTraced,
    /// Forgets every sample of the chosen rune, to trace it afresh.
    Forget,
    /// Steps through the substances that can be dropped by hand.
    NextPour,
    /// Drops the current one in the middle of the world.
    Pour,
    /// Scatters a handful of props across the paper, for a spell to act on.
    Kindle,
    /// Chooses what `kindle` scatters.
    NextKindling,
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
        section: "place",
        label: "spiral",
        hint: "ink that turns twice - the ring search must reject this",
        action: Action::Pick(Mode::Place(Shape::Spiral)),
    },
    Tool {
        section: "place",
        label: "erase",
        hint: "rub out one stroke - click the ink you want gone",
        action: Action::Pick(Mode::Place(Shape::Erase)),
    },
    Tool {
        section: "place",
        label: "link",
        hint: "drag from one ring to another - canon rule 5, linked seals",
        action: Action::Pick(Mode::Place(Shape::Link)),
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
        label: "trace >",
        hint: "choose which rune the next recording is traced as",
        action: Action::Run(Command::NextTraced),
    },
    Tool {
        section: "pad",
        label: "forget",
        hint: "throw away every sample of the chosen rune and start it over",
        action: Action::Run(Command::Forget),
    },
    Tool {
        section: "pad",
        label: "record",
        hint: "save the drawn rune under the chosen name - it goes live at once",
        action: Action::Run(Command::Record),
    },
    Tool {
        section: "element",
        label: "fire",
        hint: "place the fire sigil - drag to size. Makes and moves flame",
        action: Action::Pick(Mode::Place(Shape::Mark("fire"))),
    },
    Tool {
        section: "element",
        label: "water",
        hint: "place the water sigil - drag to size. Makes and moves water",
        action: Action::Pick(Mode::Place(Shape::Mark("water"))),
    },
    Tool {
        section: "element",
        label: "earth",
        hint: "place the earth sigil - drag to size. Moves stone, never makes it",
        action: Action::Pick(Mode::Place(Shape::Mark("earth"))),
    },
    Tool {
        section: "element",
        label: "wind",
        hint: "place the wind sigil - drag to size. Moves air, cannot make it",
        action: Action::Pick(Mode::Place(Shape::Mark("wind"))),
    },
    Tool {
        section: "element",
        label: "light",
        hint: "place the light sigil - drag to size. A fire variant, not a fifth",
        action: Action::Pick(Mode::Place(Shape::Mark("light"))),
    },
    Tool {
        section: "name",
        label: "guide",
        hint: "click where you want the lesson - one at a time, clicking moves it",
        action: Action::Pick(Mode::Place(Shape::Guide)),
    },
    Tool {
        section: "name",
        label: "preset",
        hint: "lay down a whole seal and name it - nothing to draw",
        action: Action::Run(Command::Preset),
    },
    Tool {
        section: "name",
        label: "preset >",
        hint: "choose which seal preset lays down",
        action: Action::Run(Command::NextPreset),
    },
    Tool {
        section: "cast",
        label: "auto",
        hint: "fire the moment a ring closes - canon rule 2, no button needed",
        action: Action::Toggle(Toggle::Auto),
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
        label: "walls",
        hint: "edges the world cannot escape - off, what leaves is gone and the mass falls",
        action: Action::Toggle(Toggle::Walls),
    },
    Tool {
        section: "cast",
        label: "pour",
        hint: "drop a blob of the chosen substance in the middle, to watch it react",
        action: Action::Run(Command::Pour),
    },
    Tool {
        section: "cast",
        label: "pour >",
        hint: "choose what pour drops - water, flame, air, ice, stone, steam",
        action: Action::Run(Command::NextPour),
    },
    Tool {
        section: "cast",
        label: "kindle",
        hint: "scatter things on the paper for a spell to burn, soak, blow or light",
        action: Action::Run(Command::Kindle),
    },
    Tool {
        section: "cast",
        label: "kindle >",
        hint: "choose what kindle scatters - wood, cloth, stone, ice, sand",
        action: Action::Run(Command::NextKindling),
    },
    Tool {
        section: "cast",
        label: "empty",
        hint: "clear the world, leaving the ink alone",
        action: Action::Run(Command::ClearWorld),
    },
    Tool {
        section: "view",
        label: "traced",
        hint: "only runes you traced can name a mark or fire a seal - off, built-ins and the blast come back",
        action: Action::Toggle(Toggle::Traced),
    },
    Tool {
        section: "view",
        label: "trace",
        hint: "the tracing board - what record will save, and how close it is",
        action: Action::Toggle(Toggle::Trace),
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
            .init_resource::<tutor::Tutor>()
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
                (
                    bar::layout,
                    bar::draw,
                    guides::draw,
                    place::preview,
                    place::preset_preview,
                    tutor::follow,
                    tutor::show,
                )
                    .after(crate::capture_stroke),
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
/// What a command may *look at* — as against what it may change.
///
/// Grouped because the two read-only halves arrived together and a run of eight
/// positional arguments is a run nobody reads. The mutable four stay separate:
/// which of them a command touches is the interesting part of its signature.
pub struct Bench<'a> {
    pub window: &'a Window,
    /// What the pad currently reads as, and which runes are traced.
    pub reading: &'a crate::reading::Reading,
}

pub fn run(
    command: Command,
    tools: &mut ToolState,
    pad: &mut InkPad,
    shape: &mut crate::PaperShape,
    last: &mut record::LastRecording,
    world: &mut crate::sim::Simulation,
    bench: &Bench,
) {
    let Bench { window, reading } = bench;
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

        // Every one of these replaces the ink wholesale, so the watcher's memory
        // of what has already gone off is about seals that no longer exist.
        //
        // This was the preset bug: `preset` clears and re-places in the *same*
        // frame, so the watcher never saw an empty pad, and the new ring landed
        // at the same centre and radius as the old one — which is exactly how it
        // decides two rings are the same ring. It matched a seal already marked
        // fired, so the rising edge never happened and nothing cast until you
        // wiped by hand first.
        Command::Undo => {
            shortcuts::undo(pad);
            world.armed.clear();
        }
        Command::Redo => {
            shortcuts::redo(pad);
            world.armed.clear();
        }
        Command::Clear => {
            shortcuts::clear(pad);
            world.armed.clear();
        }
        Command::ClearAll => {
            shortcuts::clear_all(pad);
            world.armed.clear();
        }

        Command::Cast => {
            // The result goes on the hint line for the same reason recording's
            // does: nobody is reading a terminal while drawing.
            let said = world.cast_pad(pad, tools);
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
            // Back to a room, not to a vacuum. `reset` empties the field, and
            // an empty field is the state where a discharge has nothing to
            // throw and a wind spell has nothing to find.
            world.fill_air();
            world.running = false;
            world.last = None;
            last.0 = Some("world emptied, air back".to_string());
        }

        Command::NextPreset => {
            tools.preset = (tools.preset + 1) % crate::sim::PRESETS.len();
            let preset = crate::sim::PRESETS[tools.preset];
            last.0 = Some(format!(
                "preset {}/{}: {} - {}",
                tools.preset + 1,
                crate::sim::PRESETS.len(),
                preset.id,
                preset.blurb
            ));
        }

        Command::Preset => {
            let preset = crate::sim::PRESETS[tools.preset];

            // **Refused rather than forced.** This used to stamp a generic mark
            // and then set a naming override, which made it the one button
            // guaranteed to produce a working spell *and* the one button that
            // never went through the recogniser. Now it draws the traced rune,
            // and if there is no traced rune it declines and says which one to
            // go and trace.
            if tools.only_traced && !reading.is_traced(preset.sigil) {
                last.0 = Some(format!(
                    "{} needs a traced {} sigil - trace one and press this again",
                    preset.id, preset.sigil
                ));
                return;
            }

            // A fresh pad, because a preset is a known state and leftover ink
            // would make it something else.
            shortcuts::clear(pad);
            world.armed.clear();

            stamp::place(
                pad,
                stamp::seal(
                    &reading.shapes,
                    preset.sigil,
                    Vec2::ZERO,
                    tools.stamp_radius,
                    preset.signs,
                    preset.open,
                    preset.inward,
                ),
            );

            last.0 = Some(match tools.auto {
                // With `auto` on, closing the ring *is* the trigger — telling
                // someone to press cast when the spell has already gone off is
                // advice from two versions ago.
                true => format!("{} placed - it fires as the ring closes", preset.id),
                false => format!("{} placed - press cast", preset.id),
            });
        }
        Command::NextPour => {
            tools.pour = (tools.pour + 1) % crate::sim::POURABLE.len();
            let (name, temperature) = crate::sim::POURABLE[tools.pour];
            last.0 = Some(format!("pour will drop {name} at {temperature:.0} deg"));
        }
        Command::Pour => {
            // Under the pointer, not in the middle of the world. Dropping
            // everything at the origin meant two substances could only ever be
            // made to meet in one place.
            let at = world.focus;
            let said = world.pour(tools.pour, at);
            world.last = Some(said.clone());
            last.0 = Some(said);
            world.running = true;
        }

        Command::Kindle => {
            let which = crate::props::KINDLING[tools.kindling];
            let said = crate::props::kindle(world, *shape, window, which);
            world.last = Some(said.clone());
            last.0 = Some(said);
        }
        Command::NextKindling => {
            tools.kindling = (tools.kindling + 1) % crate::props::KINDLING.len();
            let which = crate::props::KINDLING[tools.kindling];
            last.0 = Some(format!(
                "kindle will scatter {which} - {}",
                crate::props::describe_kindling(which)
            ));
        }
        Command::NextTraced => {
            // `traceable`, not `sigil_names` — and that was a real bug. `record`
            // has always indexed the combined list, sigils and then signs, while
            // this stepped modulo the *sigil* count: every sign in the
            // catalogue was unreachable from the panel, so a keystone could not
            // be traced at all.
            let all = world
                .lore
                .as_ref()
                .map(crate::sim::traceable)
                .unwrap_or_default();
            if all.is_empty() {
                last.0 = Some("catalogue failed to load".to_string());
            } else {
                let next = tools.tracing.map_or(0, |at| (at + 1) % all.len());
                tools.tracing = Some(next);
                let (name, kind) = &all[next];
                last.0 = Some(format!(
                    "record will trace as {name} - {} {}/{}",
                    kind.to_lowercase(),
                    next + 1,
                    all.len()
                ));
            }
        }
        Command::Forget => {
            let id = tools
                .tracing
                .and_then(|at| {
                    let all = crate::sim::traceable(world.lore.as_ref()?);
                    all.get(at).map(|(id, _)| id.clone())
                })
                .unwrap_or_else(|| "RENAME_ME".to_string());
            last.0 = Some(match record::forget(&id) {
                Ok(said) => said,
                Err(why) => why,
            });
        }
        Command::Record => {
            // The result goes on the panel rather than into a log: the point of
            // the tool is knowing whether the file got written, and a terminal
            // is not where anyone is looking while drawing.
            // The name chosen on the panel, or a numbered placeholder if
            // nobody has chosen one — the old behaviour, kept as the fallback
            // rather than as the only option.
            let (id, kind) = tools
                .tracing
                .and_then(|at| {
                    let all = crate::sim::traceable(world.lore.as_ref()?);
                    all.get(at).cloned()
                })
                .unwrap_or_else(|| ("RENAME_ME".to_string(), "Sigil"));
            last.0 = Some(match record::record(pad, &id, kind) {
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
        Toggle::Trace => tools.trace,
        Toggle::Traced => tools.only_traced,
        Toggle::Sim => tools.sim,
        Toggle::Auto => tools.auto,
        Toggle::Walls => tools.walls,
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
        Toggle::Trace => tools.trace = !tools.trace,
        Toggle::Traced => tools.only_traced = !tools.only_traced,
        Toggle::Sim => tools.sim = !tools.sim,
        Toggle::Auto => tools.auto = !tools.auto,
        Toggle::Walls => tools.walls = !tools.walls,
    }
}
