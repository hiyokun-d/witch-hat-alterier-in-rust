//! A guide for drawing a seal by hand.
//!
//! The panel can stamp a finished seal, and that is useful for testing and
//! useless for learning: it puts the drawing on the paper without ever putting
//! it in your hand. This is the other half — the same seal, one stroke at a
//! time, with the next stroke shown faintly where it goes and the pad watched
//! to see when you have drawn it.
//!
//! # It teaches whatever `preset >` has selected
//!
//! Deliberately not its own list of lessons. A second list is a second thing to
//! keep in step, and the interesting property is that the button which *stamps*
//! a seal and the guide which *teaches* it can never disagree — both read
//! `PRESETS`, and both draw with [`stamp::seal`].
//!
//! # The steps are canon's own order of assembly, not ours
//!
//! Ring, centre, keystones, close. §2.5 is explicit that canon never constrains
//! drawing *order* — an unclosed ring is a fully prepared spell, and half a
//! split seal is drawn with no ring at all — so this is a teaching order and
//! nothing more. The engine does not care, and the guide says so at the end
//! rather than letting anyone infer a rule that is not there.

use bevy::prelude::*;

use super::{ToolState, stamp};
use crate::reading::Reading;

/// The ghost of the stroke you are being asked to draw next.
const SHOW: Color = Color::srgba(0.30, 0.55, 0.42, 0.75);

/// Already drawn: what the guide has watched you finish.
const DONE: Color = Color::srgba(0.45, 0.42, 0.38, 0.28);

/// How far off the guide's radius your ring may be and still count.
///
/// Generous on purpose. The lesson is *where the parts of a seal go*, not
/// whether you can hit a radius by hand — and canon grades a seal on how round
/// it is, which `quality` already measures, never on how big.
const RADIUS_SLACK: f32 = 55.0;

/// One thing to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Step {
    /// The activator. Canon rule 9: on its own it is already a spell.
    #[default]
    Ring,
    /// The sigil — *what* the spell is.
    Centre,
    /// The keystones — *how* it behaves.
    Keystones,
    /// Only for a prepared seal: the last stroke.
    Close,
    /// Nothing left.
    Done,
}

impl Step {
    /// What to say while this step is the one to do.
    ///
    /// Names the actual mark where there is one. "Draw the sigil" is an
    /// instruction you cannot follow; "draw WATER" is one you can, and the
    /// ghost beside it shows the shape.
    pub fn asks(self, lesson: &str, sigil: Option<&str>, signs: usize) -> String {
        let keystone = sign_of(lesson);
        match self {
            Step::Ring => "1  the ring: one loop, all the way round. The activator".to_string(),
            Step::Centre => match sigil {
                Some(which) => format!(
                    "2  the sigil: draw {} in the middle. It decides WHAT the spell is",
                    which.to_uppercase()
                ),
                None => "2  the sigil: a mark in the middle. It decides WHAT".to_string(),
            },
            Step::Keystones => match (signs, keystone) {
                (0, _) => "3  no keystones in this one".to_string(),
                (n, Some(which)) => format!(
                    "3  {n} x {}, all the same size. They decide HOW",
                    which.to_uppercase()
                ),
                (n, None) => format!("3  {n} keystone(s), equal length. They decide HOW"),
            },
            Step::Close => {
                "4  close the gap. Canon rule 2: the ring completing IS the trigger".to_string()
            }
            Step::Done => {
                "done - and canon never asked for that order. Any order draws the same seal"
                    .to_string()
            }
        }
    }
}

/// What each teachable seal's keystone is, for the ghost and for the wording.
///
/// A small table rather than a read of `spells.ron`, because the guide can only
/// ghost a shape the app actually has. The *sigil* used to have a twin of this
/// table and does not need one any more — `Preset::sigil` carries it, so the
/// button that stamps a seal and the guide that teaches it cannot name
/// different runes.
const FIXTURE_SIGNS: &[(&str, &str)] = &[
    ("flamespout", "column"),
    ("watershot_seal", "column"),
    ("fire_shot", "column"),
    ("raincleaver", "column"),
    ("water_orb", "levitation"),
    ("lightfall", "levitation"),
    ("earth_wall", "convergence"),
];

/// The keystone a fixture calls for, if core has a shape for it.
fn sign_of(fixture: &str) -> Option<&'static str> {
    FIXTURE_SIGNS
        .iter()
        .find(|(f, _)| *f == fixture)
        .map(|(_, sign)| *sign)
}

/// Which lesson, and how far through it you are.
#[derive(Resource, Debug, Default)]
pub struct Tutor {
    pub step: Step,
    /// The radius the lesson is being taught at, fixed when it starts so the
    /// ghost does not move while you are tracing it.
    pub radius: f32,
    /// Where the lesson sits. Placed by clicking with the `guide` tool.
    ///
    /// One lesson, not many: clicking again *moves* this rather than adding a
    /// second. Two half-finished lessons on one page would be two sets of ghost
    /// strokes with no way to tell which step belongs to which.
    pub at: Vec2,
    /// Whether a lesson has been placed at all.
    pub placed: bool,
}

/// Reads the pad and decides which step you are on.
///
/// Watches rather than asks. There is no "next" button because there is nothing
/// to press: you have either drawn the thing or you have not, and the pad
/// already knows which.
pub fn follow(reading: Res<Reading>, tools: Res<ToolState>, mut tutor: ResMut<Tutor>) {
    if !tutor.placed {
        tutor.step = Step::Ring;
        tutor.radius = tools.stamp_radius;
        return;
    }
    if tutor.radius <= 0.0 {
        tutor.radius = tools.stamp_radius;
    }

    let Some(preset) = crate::sim::PRESETS.get(tools.preset).copied() else {
        return;
    };

    // The ring the lesson is about: the one nearest the size being taught,
    // rather than simply the first, so a stray loop elsewhere on the paper does
    // not hijack the guide.
    let ring = reading
        .rings
        .iter()
        .enumerate()
        .filter(|(_, ring)| {
            (ring.fit.radius - tutor.radius).abs() < RADIUS_SLACK
                && Vec2::new(ring.fit.center.x, ring.fit.center.y).distance(tutor.at) < tutor.radius
        })
        .min_by(|a, b| {
            (a.1.fit.radius - tutor.radius)
                .abs()
                .total_cmp(&(b.1.fit.radius - tutor.radius).abs())
        });

    let Some((slot, ring)) = ring else {
        tutor.step = Step::Ring;
        return;
    };

    // What the ring holds. The guide counts marks, not meanings — naming ink is
    // the recognizer's job and `templates.ron` is empty, so counting is the
    // only honest measure available and it is enough to teach placement.
    let held = reading
        .glyphs
        .get(slot)
        .map_or(0, |glyph| glyph.unnamed + glyph.signs.len());

    tutor.step = if held == 0 {
        Step::Centre
    } else if held < preset.signs + 1 {
        Step::Keystones
    } else if preset.open && !ring.closed {
        Step::Close
    } else {
        Step::Done
    };
}

/// Draws the step you are on, and faintly what you have already done.
pub fn show(
    tools: Res<ToolState>,
    tutor: Res<Tutor>,
    reading: Res<crate::reading::Reading>,
    mut gizmos: Gizmos,
) {
    if !tutor.placed {
        return;
    }
    let Some(preset) = crate::sim::PRESETS.get(tools.preset).copied() else {
        return;
    };

    // Where it was placed, and it stays there. The first version followed the
    // pointer, which is unusable — the thing you are trying to trace moves as
    // you reach for it. A guide has to hold still or it is not a guide; a
    // *click* is how you say where still should be.
    let at = tutor.at;
    let radius = tutor.radius;

    let ring = if preset.open {
        stamp::arc(at, radius, 34.0, std::f32::consts::FRAC_PI_2)
    } else {
        stamp::ring(at, radius)
    };
    // The rune itself, not a stand-in. A guide whose centre is a cross teaches
    // you to draw a cross.
    let centre = stamp::glyph_mark(&reading.shapes, preset.sigil, at, radius * 0.25);
    let keystones: Vec<Vec<Vec2>> = (0..preset.signs)
        .flat_map(|slot| {
            let around = std::f32::consts::TAU * slot as f32 / preset.signs.max(1) as f32;
            let aim = if preset.inward {
                around + std::f32::consts::PI
            } else {
                around
            };
            stamp::sign(
                at + Vec2::from_angle(around) * (radius * 0.62),
                radius * 0.22,
                aim,
            )
        })
        .collect();

    // Bright for the step being asked for, faint for what is behind you. A
    // guide that shows everything at full strength is a picture, not a lesson.
    let (bright, dim): (Vec<Vec<Vec2>>, Vec<Vec<Vec2>>) = match tutor.step {
        Step::Ring => (ring, Vec::new()),
        Step::Centre => (centre, ring),
        Step::Keystones => (keystones, [ring, centre].concat()),
        Step::Close | Step::Done => (Vec::new(), [ring, centre, keystones].concat()),
    };

    for (strokes, color) in [(dim, DONE), (bright, SHOW)] {
        for stroke in strokes {
            for pair in stroke.windows(2) {
                gizmos.line_2d(pair[0], pair[1], color);
            }
        }
    }
}
