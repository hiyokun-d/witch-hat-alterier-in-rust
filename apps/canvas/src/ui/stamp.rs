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
        Shape::Cross => cross(center, radius * 0.45),
        Shape::Sign => sign(center, radius * 0.45, facing),
        Shape::Glaive => glaive(center, radius * 0.35, facing),
        Shape::Bar => bar(center, radius * 0.45, facing),
        Shape::Triangle => triangle(center, radius * 0.35, facing),
        Shape::Spiral => spiral(center, radius * 0.40),
        // A link is drawn from where you pressed to where you let go, not out
        // from a centre — it joins two rings (canon rule 5), so both ends
        // matter and neither is the middle of anything.
        Shape::Link => link(center, radius, facing),
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

/// Two crossed strokes, standing in for a sigil until real runes are traced.
///
/// Deliberately not claiming to be any canon sigil — `templates.ron` is empty
/// and inventing a fire glyph here would be exactly the thing §2 forbids. This
/// is ink of a known shape to put *inside* a ring, so that
/// `RingCandidate::contents` has something to sort and the intensity ratio has
/// something to measure.
pub fn cross(center: Vec2, reach: f32) -> Vec<Vec<Vec2>> {
    let arm = |from: Vec2, to: Vec2| -> Vec<Vec2> {
        (0..SAMPLES_PER_ARM)
            .map(|i| from.lerp(to, i as f32 / (SAMPLES_PER_ARM - 1) as f32))
            .collect()
    };

    vec![
        arm(center - Vec2::X * reach, center + Vec2::X * reach),
        arm(center - Vec2::Y * reach, center + Vec2::Y * reach),
    ]
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

/// A straight keystone: length is power, direction is aim (§2.4).
///
/// The plainest sign there is, and the one to reach for when what is being
/// tested is *balance* rather than shape — four equal bars sum to zero drift,
/// and lengthening one steers the whole seal.
pub fn bar(center: Vec2, reach: f32, facing: f32) -> Vec<Vec<Vec2>> {
    let along = Vec2::from_angle(facing);
    let steps = 12;
    vec![
        (0..=steps)
            .map(|i| {
                let t = i as f32 / steps as f32 * 2.0 - 1.0;
                center + along * (reach * t)
            })
            .collect(),
    ]
}

/// A three-sided mark.
///
/// Canon notes whorling wind is three-sided and wonders whether that hints at
/// a fire connection - heating the air the way a hot-air balloon does. This is
/// a stand-in for that shape, not a claim to be it.
pub fn triangle(center: Vec2, reach: f32, facing: f32) -> Vec<Vec<Vec2>> {
    let corner = |k: usize| {
        let angle = facing + std::f32::consts::TAU * k as f32 / 3.0;
        center + Vec2::from_angle(angle) * reach
    };
    // One closed stroke. The fitter will not mistake it for a ring: three
    // corners put the circle fit's rms well past `min_roundness`.
    vec![vec![corner(0), corner(1), corner(2), corner(0)]]
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
