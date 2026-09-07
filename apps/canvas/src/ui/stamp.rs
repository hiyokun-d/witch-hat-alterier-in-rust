//! Generating exact ink.
//!
//! A placed ring is real ink — the same `Vec<Point>` a pen would have
//! produced, going through the same assembly, the same fit, the same
//! activation test. Nothing downstream is told it was placed, and nothing
//! downstream should be: the moment placed ink took a shortcut, it would stop
//! being a way to reproduce what a drawn seal does.
//!
//! Everything here returns strokes in world coordinates and puts nothing
//! anywhere. [`place`] is the only function that touches the pad, which is
//! what lets the preview call the same generators without side effects.

use bevy::prelude::*;
use std::f32::consts::TAU;

use magic_core::templates::{Kind, Recorded};

use super::Shape;
use crate::{InkPad, Point};

/// Points per full turn of a placed curve.
///
/// Roughly what a steady hand produces at `MIN_POINT_SPACING` on a middling
/// ring, so the recognizer sees something with a familiar density rather than
/// an impossibly smooth one.
const SAMPLES_PER_TURN: usize = 180;

/// Samples along one straight arm.
const SAMPLES_PER_ARM: usize = 24;

/// Builds whichever shape is armed.
///
/// One entry point so the preview and the commit cannot end up calling
/// different things. `facing` aims an arc's hole and is ignored by the shapes
/// that have no direction.
pub fn draw_shape(
    shape: Shape,
    shapes: &[Recorded],
    center: Vec2,
    radius: f32,
    facing: f32,
    gap_degrees: f32,
) -> Vec<Vec<Vec2>> {
    match shape {
        Shape::Ring => ring(center, radius),
        Shape::Arc => arc(center, radius, gap_degrees, facing),
        // Sized against the ring it is meant to sit inside, so it lands as
        // contents rather than as another ring.
        Shape::Sign => sign(center, radius * 0.45, facing),
        Shape::Glaive => glaive(center, radius * 0.35, facing),
        Shape::Spiral => spiral(center, radius * 0.40),
        // A link is drawn from where you pressed to where you let go, not out
        // from a centre — it joins two rings (canon rule 5), so both ends
        // matter and neither is the middle of anything.
        Shape::Link => link(center, radius, facing),
        // A sigil or a sign from core's own shapes, sized by the drag like
        // everything else on the panel.
        Shape::Mark(which) => glyph_mark(shapes, which, center, radius * 0.30),
        // The guide places no ink: it moves where the lesson is drawn. Handled
        // in `place::drag` before it reaches here.
        Shape::Guide => Vec::new(),
        // The eraser draws nothing. `place::drag` handles it before it ever
        // reaches here, and returning no strokes is what makes that safe.
        Shape::Erase => Vec::new(),
    }
}

/// Puts finished strokes on the pad, as capture would have.
///
/// Follows the same contract as drawing: points appended, `stroke_id` bumped
/// once each stroke is complete, and the redo stack dropped because placing is
/// an edit and anything further forward in the history is now stale.
pub fn place(pad: &mut InkPad, strokes: Vec<Vec<Vec2>>) {
    for stroke in strokes {
        if stroke.len() < 2 {
            continue;
        }
        for at in stroke {
            pad.points.push(Point {
                x: at.x,
                y: at.y,
                stroke_id: pad.stroke_id,
            });
        }
        pad.stroke_id += 1;
    }
    pad.undone.clear();
}

/// A closed ring: one stroke whose ends meet exactly.
///
/// The ends *are* the same point, so `assembly`'s endpoint test finds nothing
/// loose and the ring reads as closed. That is the whole reason this exists —
/// a hand-drawn ring almost always leaves a few pixels between its ends.
pub fn ring(center: Vec2, radius: f32) -> Vec<Vec<Vec2>> {
    let n = SAMPLES_PER_TURN;
    let mut stroke: Vec<Vec2> = (0..n)
        .map(|i| center + Vec2::from_angle(i as f32 / n as f32 * TAU) * radius)
        .collect();
    // Closed properly rather than relying on the last sample landing near the
    // first.
    stroke.push(stroke[0]);
    vec![stroke]
}

/// A ring with a hole of `gap_degrees` left in it, centred on `facing` —
/// canon rule 2's prepared spell, ready to be finished later.
///
/// A zero gap gives a closed ring, so the gap control can be wound down to
/// nothing without the tool changing meaning underneath you.
pub fn arc(center: Vec2, radius: f32, gap_degrees: f32, facing: f32) -> Vec<Vec<Vec2>> {
    let gap = gap_degrees.to_radians().clamp(0.0, TAU * 0.99);
    if gap <= f32::EPSILON {
        return ring(center, radius);
    }

    let sweep = TAU - gap;
    let n = ((SAMPLES_PER_TURN as f32) * sweep / TAU).ceil().max(3.0) as usize;

    // The ink starts half a gap the far side of `facing` and runs the long way
    // round, which leaves the hole centred on exactly where the drag pointed.
    let start = facing + gap * 0.5;
    let stroke = (0..n)
        .map(|i| center + Vec2::from_angle(start + sweep * i as f32 / (n - 1) as f32) * radius)
        .collect();
    vec![stroke]
}
/// One keystone: a shaft with a head, pointing along `facing`.
///
/// Not any named sign — `templates.ron` is empty and inventing a column glyph
/// here is the thing §2 forbids. What it *is* is a mark with a length and a
/// direction, which is everything `arrangement::balance` and `spin` need: size
/// is power, tilt off the radial buys spin at the cost of reach (§2.4). So a
/// seal built from these can be compiled and steered before a single real rune
/// exists.
pub fn sign(center: Vec2, reach: f32, facing: f32) -> Vec<Vec<Vec2>> {
    let dir = Vec2::from_angle(facing);
    let tail = center - dir * reach * 0.5;
    let tip = center + dir * reach * 0.5;

    let line = |from: Vec2, to: Vec2| -> Vec<Vec2> {
        (0..SAMPLES_PER_ARM)
            .map(|i| from.lerp(to, i as f32 / (SAMPLES_PER_ARM - 1) as f32))
            .collect()
    };

    // A head rather than a bare line, so which end is the front survives being
    // drawn, recognised, and reversed (canon rule 6).
    let barb = reach * 0.3;
    let left = tip - Vec2::from_angle(facing + 0.5) * barb;
    let right = tip - Vec2::from_angle(facing - 0.5) * barb;

    vec![
        line(tail, tip),
        [line(left, tip), line(tip, right)].concat(),
    ]
}

/// A claw: three curved talons fanning out from a point.
///
/// Neither a sign nor a sigil (§2.1), and the one mark canon lets you draw
/// *outside* the ring so long as it still connects — so this is meant to be
/// placed straddling the line, not tucked inside it.
///
/// What it sets is how firmly the spell embeds in a body. Whether that means
/// depth or tenacity is unknown, and the tool does not pretend otherwise: the
/// only thing it varies is size, which is the only thing canon gives us.
pub fn glaive(center: Vec2, reach: f32, facing: f32) -> Vec<Vec<Vec2>> {
    let talon = |spread: f32| -> Vec<Vec2> {
        // Each talon curls further out the way it already leans, so the three
        // open into a claw. `signum().max(0.4)` was the bug: it turned the
        // left talon's -1 into +0.4 and curled all three the same way, giving
        // a fan instead of a claw.
        (0..SAMPLES_PER_ARM)
            .map(|i| {
                let t = i as f32 / (SAMPLES_PER_ARM - 1) as f32;
                let angle = facing + spread * (1.0 + t * 1.4);
                center + Vec2::from_angle(angle) * reach * t
            })
            .collect()
    };
    vec![talon(-0.45), talon(0.0), talon(0.45)]
}
/// A spiral - ink that turns more than once.
///
/// Deliberately something the ring search must *reject*: it covers one turn of
/// angle while winding two and a half, so `is_simple` fails and it stays as
/// contents rather than seeding a ring. A tool for reproducing that case
/// without a steady hand.
pub fn spiral(center: Vec2, reach: f32) -> Vec<Vec<Vec2>> {
    let turns = 2.5;
    let steps = 90;
    vec![
        (0..=steps)
            .map(|i| {
                let t = i as f32 / steps as f32;
                let angle = std::f32::consts::TAU * turns * t;
                center + Vec2::from_angle(angle) * (reach * (0.15 + 0.85 * t))
            })
            .collect(),
    ]
}

/// The line that joins two seals - canon rule 5.
///
/// Drawn from `from` outward, so a drag starting on one ring and ending on
/// another lands exactly where `assembly::links` looks for it: ink belonging to
/// neither ring that comes within tolerance of both. Until this existed, rule 5
/// could only be reached from a test.
pub fn link(from: Vec2, reach: f32, facing: f32) -> Vec<Vec<Vec2>> {
    let along = Vec2::from_angle(facing);
    let steps = 16;
    vec![
        (0..=steps)
            .map(|i| from + along * (reach * i as f32 / steps as f32))
            .collect(),
    ]
}

/// The whole seal a preset lays down: centre mark, ring, and its keystones.
///
/// One function because the button and the hover preview must draw the *same*
/// thing — a preview that lies is worse than no preview.
///
/// **The centre mark is not decoration.** Every seal in the source has a sigil
/// in the middle; a ring with arrows around an empty hole is not a seal anyone
/// would recognise, and the first version of `preset` drew exactly that. What
/// goes there is a stand-in, not a traced rune (§2 — the shapes belong to the
/// manga), but *something* has to occupy the centre or the drawing is wrong
/// before the compiler has said a word.
///
/// A triangle inside a cross rather than a small ring: a ring in the middle
/// would be found by the ring search and read as canon rule 4's nesting, which
/// would change what the seal compiles to. A three-sided mark cannot.
pub fn seal(
    shapes: &[Recorded],
    sigil: &str,
    center: Vec2,
    radius: f32,
    signs: usize,
    open: bool,
    inward: bool,
) -> Vec<Vec<Vec2>> {
    // An open seal is canon rule 2's prepared spell: everything drawn, waiting
    // on its last stroke. The gap faces up, where it is easiest to see and to
    // close by hand.
    let mut strokes = if open {
        arc(center, radius, 34.0, std::f32::consts::FRAC_PI_2)
    } else {
        ring(center, radius)
    };
    // **The seal's own sigil, not a stand-in.** This used to be a cross with a
    // triangle in it — a mark nobody could read, which is why the preset had to
    // force its own name to compile to anything. Drawing the rune the
    // recogniser knows means a stamped seal names itself exactly the way a
    // hand-drawn one does, and there is no override left to get wrong.
    // **A quarter of the ring, and the number is load-bearing.** The keystones
    // sit at `0.62` of the radius and are `0.22` long, so their inner ends
    // reach in to `0.51`. A sigil drawn any larger than about `0.3` touches
    // them, and `naming` segments on the gaps between strokes — so an oversized
    // sigil merges with a keystone, the blob reads as nothing, and the seal is
    // refused. A preset that stamps a spell it cannot then read is the exact
    // failure this whole path exists to remove.
    strokes.extend(glyph_mark(shapes, sigil, center, radius * 0.25));

    for slot in 0..signs.max(1) {
        let around = std::f32::consts::TAU * slot as f32 / signs.max(1) as f32;
        let at = center + Vec2::from_angle(around) * (radius * 0.62);
        // Outward throws the magic away from the seal; **inward** keeps it
        // inside, which is canon's `AllInward`: "manifests only inside the
        // ring". Four arrows pointing at the middle is how you get a ball
        // rather than a fountain, and it is the arrangement doing it, not the
        // sigil — the same water sigil with the arrows reversed is a spout.
        let aim = if inward {
            around + std::f32::consts::PI
        } else {
            around
        };
        strokes.extend(sign(at, radius * 0.22, aim));
    }
    strokes
}

/// One recorded rune, un-normalised back onto the pad.
///
/// `Cloud` divides out position and scale and keeps both, so putting a rune
/// back is the same arithmetic run backwards. `stroke_id` survives
/// normalisation untouched, which is what lets a four-stroke sigil come back as
/// four strokes rather than as one scribble — and that matters, because
/// `naming` segments on the gaps between strokes.
///
/// `reach` is the **furthest** point from the centre, not the RMS distance the
/// recognizer divided out. The caller is placing a mark inside a ring and needs
/// to know it will fit; RMS is a fact about the shape and says nothing about
/// where its tips land. Runes vary a lot here — a diamond in a square reaches
/// much further past its RMS than three teardrops do — so converting once, per
/// rune, is the only way `seal` can promise a gap between the sigil and the
/// keystones around it.
pub fn rune(
    shapes: &[Recorded],
    kind: Kind,
    name: &str,
    center: Vec2,
    reach: f32,
) -> Option<Vec<Vec<Vec2>>> {
    let found = shapes
        .iter()
        .find(|rune| rune.kind == kind && rune.template.name == name)?;

    let points = &found.template.cloud.points;
    let furthest = points
        .iter()
        .map(|p| (p.x * p.x + p.y * p.y).sqrt())
        .fold(0.0f32, f32::max);
    if furthest <= f32::EPSILON {
        return None;
    }
    let scale = reach / furthest;

    let mut strokes: Vec<Vec<Vec2>> = Vec::new();
    for run in points.chunk_by(|a, b| a.stroke_id == b.stroke_id) {
        strokes.push(
            run.iter()
                .map(|p| center + Vec2::new(p.x, p.y) * scale)
                .collect(),
        );
    }
    (!strokes.is_empty()).then_some(strokes)
}

/// A mark, placed and scaled — **the one you would actually be recognised as**.
///
/// Traced first, and that is the whole change. It used to draw
/// `magic_core::shapes::built_in` always, which meant the button that stamps a
/// fire sigil drew a *reconstruction* while the recogniser had your own traced
/// fire in front of it. The two are not the same drawing, so the app was
/// stamping something it could not read — and the preset covered for that by
/// forcing the name, which is exactly the accidental cast that has to stop.
///
/// Falls back to the built-in geometry for anything untraced, so the keystones
/// still work before anybody has sat down to trace forty-four of them (§12).
pub fn glyph_mark(shapes: &[Recorded], which: &str, center: Vec2, reach: f32) -> Vec<Vec<Vec2>> {
    if let Some(traced) = rune(shapes, Kind::Sigil, which, center, reach) {
        return traced;
    }
    if let Some(traced) = rune(shapes, Kind::Sign, which, center, reach) {
        return traced;
    }
    magic_core::shapes::built_in()
        .into_iter()
        .find(|(id, _, _)| *id == which)
        .map(|(_, _, strokes)| {
            strokes
                .iter()
                .map(|stroke| {
                    stroke
                        .iter()
                        .map(|&(x, y)| center + Vec2::new(x, y) * reach)
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default()
}
