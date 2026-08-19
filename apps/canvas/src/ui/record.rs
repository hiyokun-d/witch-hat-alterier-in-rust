//! The recorder: turning what is on the pad into a `templates.ron` entry.
//!
//! Nothing can be recognised until the rune shapes exist, and they cannot be
//! invented — §2 is explicit that the shapes belong to the manga and have to be
//! traced. So the missing piece before the recognizer is useful was never code;
//! it was a way to get a traced shape *out* of the app and into the catalogue.
//!
//! This is that way, and it is deliberately the smallest thing that works:
//! press the button, and every stroke that is not part of a ring is written to
//! a `.ron` fragment beside the app. Paste the fragment into
//! `the-magic-assets/templates.ron`, change the placeholder id to the real one,
//! and the recognizer can see it.
//!
//! # Why the shell and not core
//!
//! Core has no filesystem (§4.1) and never will. Writing files is exactly the
//! kind of platform business a shell exists for, and the format itself is
//! core's — `templates::parse` is what has to accept the result, so the round
//! trip is the test that matters.
//!
//! # Why the id is a placeholder
//!
//! The panel has no text entry, and adding one for this would be a lot of
//! machinery for a step that happens a few dozen times in the project's life.
//! Naming the rune is a decision about canon; making the file is not. So the
//! tool does the mechanical half and leaves the decision in a one-word edit.

use bevy::prelude::*;
use magic_core::assembly;
use std::io::Write;

use crate::InkPad;

/// Where the fragment lands, relative to wherever the app was launched from.
const OUTFILE: &str = "recorded-gesture.ron";

/// What the last recording did, so the panel can say so.
#[derive(Resource, Debug, Default, Clone)]
pub struct LastRecording(pub Option<String>);

/// Writes every non-ring stroke on the pad as one gesture.
///
/// Rings are excluded on purpose: a ring is the activator, not the rune, and
/// the recogniser is never asked to identify one — `circle.rs` does that with
/// geometry instead. What is left is the marks inside, which is exactly what a
/// template is.
pub fn record(pad: &InkPad) -> Result<String, String> {
    if pad.points.is_empty() {
        return Err("nothing on the pad".to_string());
    }

    let rings = assembly::find_rings(&pad.points, &assembly::RingSearch::default());
    let ring_strokes: Vec<u32> = rings.iter().flat_map(|r| r.strokes.clone()).collect();

    let mut strokes: Vec<Vec<(f32, f32)>> = Vec::new();
    let mut current = Vec::new();
    let mut at = None;
    for point in &pad.points {
        if ring_strokes.contains(&point.stroke_id) {
            continue;
        }
        if at != Some(point.stroke_id) {
            if current.len() >= 2 {
                strokes.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
            at = Some(point.stroke_id);
        }
        current.push((point.x, point.y));
    }
    if current.len() >= 2 {
        strokes.push(current);
    }

    if strokes.is_empty() {
        return Err("only rings on the pad — draw the rune inside one".to_string());
    }

    let body: String = strokes
        .iter()
        .map(|stroke| {
            let pts: Vec<String> = stroke
                .iter()
                .map(|(x, y)| format!("({x:.1}, {y:.1})"))
                .collect();
            format!("        [{}],\n", pts.join(", "))
        })
        .collect();

    let fragment = format!(
        "// Traced in the canvas. Change `id` to the real sigil or sign, set\n\
         // `kind`, and paste this into the-magic-assets/templates.ron.\n\
         (\n    id: \"RENAME_ME\",\n    kind: Sigil,\n    strokes: [\n{body}    ],\n),\n"
    );

    let mut file = std::fs::File::create(OUTFILE).map_err(|e| e.to_string())?;
    file.write_all(fragment.as_bytes())
        .map_err(|e| e.to_string())?;

    Ok(format!(
        "wrote {} stroke(s), {} pts → {OUTFILE}",
        strokes.len(),
        strokes.iter().map(Vec::len).sum::<usize>()
    ))
}

#[cfg(test)]
#[path = "../tests/record.rs"]
mod tests;
