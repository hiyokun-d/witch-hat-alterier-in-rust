//! The recorder: turning what is on the pad into a `templates.ron` entry.
//!
//! Nothing can be recognised until the rune shapes exist, and they cannot be
//! invented — §2 is explicit that the shapes belong to the manga and have to be
//! traced. So the missing piece before the recognizer is useful was never code;
//! it was a way to get a traced shape *out* of the app and into the catalogue.
//!
//! This is that way: press the button, and every stroke that is not part of a
//! ring is written to a `.ron` file beside the app.
//!
//! # Why a whole file and not a fragment
//!
//! The first version wrote a bare `( id: ..., strokes: [...] )` to paste into
//! `the-magic-assets/templates.ron`, which meant a traced rune could not be
//! *matched* until it had been pasted, named, and the app rebuilt. Writing a
//! complete, valid template file instead costs six lines of wrapper and buys
//! the whole loop: the overlay reads this file live, so a rune traced now is
//! ranked against ink a second later. Pasting still works — the entries in the
//! middle are exactly what `templates.ron` wants.
//!
//! Recordings **accumulate**. The file is carved back open between its two
//! marker lines and the new rune appended, so several runes can be traced in
//! one sitting and compared against each other.
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
//!
//! The placeholders are numbered rather than all `RENAME_ME`, because
//! `templates::parse` rejects a duplicate id within a kind — and it is right
//! to: two runes with one name means one of them can never win a match.

use bevy::prelude::*;
use magic_core::assembly;

use crate::InkPad;

/// Where the runes land, relative to wherever the app was launched from.
pub const OUTFILE: &str = "recorded-gesture.ron";

/// The region between these is ours to rewrite; everything outside is wrapper.
///
/// Carving on markers we wrote ourselves rather than parsing the file back:
/// `Recorded` holds a *normalised* cloud, so a parsed entry can no longer emit
/// the points it came from. The text is the only lossless copy.
const BEGIN: &str = "//<<< recorded runes — everything below is appended";
const END: &str = "//>>> end of recorded runes";

/// What the last recording did, so the panel can say so.
#[derive(Resource, Debug, Default, Clone)]
pub struct LastRecording(pub Option<String>);

/// Every non-ring stroke on the pad, as one gesture.
///
/// Rings are excluded on purpose: a ring is the activator, not the rune, and
/// the recogniser is never asked to identify one — `circle.rs` does that with
/// geometry instead. What is left is the marks inside, which is exactly what a
/// template is.
fn gestures(pad: &InkPad) -> Result<Vec<Vec<(f32, f32)>>, String> {
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
        return Err("only rings on the pad - draw the rune inside one".to_string());
    }

    Ok(strokes)
}

/// One gesture as a `templates.ron` entry, indented to sit in the list.
fn entry(id: &str, strokes: &[Vec<(f32, f32)>]) -> String {
    let body: String = strokes
        .iter()
        .map(|stroke| {
            let pts: Vec<String> = stroke
                .iter()
                .map(|(x, y)| format!("({x:.1}, {y:.1})"))
                .collect();
            format!("                [{}],\n", pts.join(", "))
        })
        .collect();

    format!(
        "        (\n            id: \"{id}\",\n            kind: Sigil,\n            strokes: [\n{body}            ],\n        ),\n"
    )
}

/// Wraps recorded entries in the file `templates::parse` accepts.
fn wrap(entries: &str) -> String {
    format!(
        "// Traced in the canvas, and read back by the overlay while it runs.\n\
         //\n\
         // To make a rune permanent: change its `id` to a real SigilId or SignId,\n\
         // set `kind`, and paste the entry into the-magic-assets/templates.ron.\n\
         (\n    version: {},\n    templates: [\n{BEGIN}\n{entries}{END}\n    ],\n)\n",
        magic_core::templates::FORMAT_VERSION
    )
}

/// The entries already recorded, or `None` if the file is absent or mangled.
fn carve(existing: &str) -> Option<&str> {
    let (_, rest) = existing.split_once(BEGIN)?;
    let (kept, _) = rest.split_once(END)?;
    Some(kept.trim_start_matches('\n'))
}

/// Appends the pad's rune to [`OUTFILE`], and says what it wrote.
pub fn record(pad: &InkPad) -> Result<String, String> {
    let strokes = gestures(pad)?;

    let existing = std::fs::read_to_string(OUTFILE).unwrap_or_default();
    let kept = match carve(&existing) {
        Some(kept) => kept,
        None => {
            // A file we cannot carve is either absent or something we did not
            // write — most likely a bare fragment from the older recorder. It
            // still holds a traced rune, and tracing one is the expensive half
            // of this whole project, so it gets moved aside rather than
            // overwritten. Failing to move it is not a reason to refuse the
            // recording; the worst case is an empty file we were about to
            // replace anyway.
            if !existing.is_empty() {
                let _ = std::fs::rename(OUTFILE, format!("{OUTFILE}.bak"));
            }
            ""
        }
    };
    // Counting the ids we emit, not parsing: the number only has to be unique.
    let next = kept.matches("            id: \"").count() + 1;

    let entries = format!("{kept}{}", entry(&format!("RENAME_ME_{next}"), &strokes));
    std::fs::write(OUTFILE, wrap(&entries)).map_err(|e| e.to_string())?;

    Ok(format!(
        "rune {next}: {} stroke(s), {} pts -> {OUTFILE}",
        strokes.len(),
        strokes.iter().map(Vec::len).sum::<usize>()
    ))
}

#[cfg(test)]
#[path = "../tests/record.rs"]
mod tests;
