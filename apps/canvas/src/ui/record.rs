//! The recorder: turning what is on the pad into a `templates.ron` entry.
//!
//! Nothing can be recognised until the rune shapes exist, and they cannot be
//! invented — §2 is explicit that the shapes belong to the manga and have to be
//! traced. `shapes.rs` ships reconstructions so the engine is reachable at all,
//! and this is how a real one replaces them: draw it, name it on the panel,
//! press record.
//!
//! # Why a whole file and not a fragment
//!
//! The first version wrote a bare entry to paste into `templates.ron`, which
//! meant a traced rune could not be *matched* until it had been pasted, named,
//! and the app rebuilt. Writing a complete, valid template file instead costs
//! six lines of wrapper and buys the whole loop: the overlay reads this file
//! live, so a rune traced now is ranked against ink a second later. Pasting
//! still works — the entries in the middle are what `templates.ron` wants.
//!
//! # Why the shell and not core
//!
//! Core has no filesystem (§4.1) and never will. Writing files is exactly the
//! platform business a shell exists for, and the format itself is core's —
//! `templates::parse` has to accept the result, so the round trip is the test
//! that matters.
//!
//! # The browser has no files
//!
//! `std::fs` *compiles* for `wasm32-unknown-unknown` and then fails at runtime,
//! which is the worst of the three options: it builds, it looks supported, and
//! it silently does nothing. So everything below the resource is desktop-only
//! and the web build says so out loud. Tracing is a desktop job anyway — the
//! reference is open beside the app.

use bevy::prelude::*;

use crate::InkPad;

/// What the last recording did, so the panel can say so.
#[derive(Resource, Debug, Default, Clone)]
pub struct LastRecording(pub Option<String>);

/// The browser cannot write files, and pretending otherwise is worse than
/// saying so.
#[cfg(target_arch = "wasm32")]
pub fn record(
    _pad: &InkPad,
    _ring_strokes: &[u32],
    _id: &str,
    _kind: &str,
) -> Result<String, String> {
    Err("recording needs the desktop build - the browser has no files".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn forget(_id: &str) -> Result<String, String> {
    Err("recording needs the desktop build - the browser has no files".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
pub use desktop::{OUTFILE, forget, record};

#[cfg(not(target_arch = "wasm32"))]
mod desktop {
    use super::*;

    /// Where the runes land, relative to wherever the app was launched from.
    pub const OUTFILE: &str = "recorded-gesture.ron";

    /// The region between these is ours to rewrite; everything outside is
    /// wrapper.
    ///
    /// Carving on markers we wrote ourselves rather than parsing the file back:
    /// a parsed `Recorded` holds a *normalised* cloud and can no longer emit the
    /// points it came from, so the text is the only lossless copy.
    const BEGIN: &str = "//<<< recorded runes - everything below is appended";
    const END: &str = "//>>> end of recorded runes";

    /// Every non-ring stroke on the pad, as one gesture.
    ///
    /// Rings are excluded on purpose: a ring is the activator, not the rune, and
    /// the recogniser is never asked to identify one — `circle.rs` does that with
    /// geometry instead.
    /// `ring_strokes` is handed in rather than found here, and that matters.
    ///
    /// This used to run its own `find_rings` with `RingSearch::default()`,
    /// whose `join` is 16px against the shell's 20px — so the recorder could
    /// disagree with the rest of the app about whether a stroke was ring ink,
    /// and quietly save a rune with a piece missing or a ring stroke welded on.
    /// One reading of the pad, shared (`reading.rs`), or the two drift.
    fn gestures(pad: &InkPad, ring_strokes: &[u32]) -> Result<Vec<Vec<(f32, f32)>>, String> {
        if pad.points.is_empty() {
            return Err("nothing on the pad".to_string());
        }

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
    pub(super) fn entry(id: &str, kind: &str, strokes: &[Vec<(f32, f32)>]) -> String {
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
            "        (\n            id: \"{id}\",\n            kind: {kind},\n            strokes: [\n{body}            ],\n        ),\n"
        )
    }

    /// Wraps recorded entries in the file `templates::parse` accepts.
    pub(super) fn wrap(entries: &str) -> String {
        format!(
            "// Traced in the canvas, and read back by the overlay while it runs.\n\
             //\n\
             // To make a rune permanent, paste its entry into\n\
             // the-magic-assets/templates.ron. A traced rune outranks the built-in\n\
             // shape of the same name.\n\
             (\n    version: {},\n    templates: [\n{BEGIN}\n{entries}{END}\n    ],\n)\n",
            magic_core::templates::FORMAT_VERSION
        )
    }

    /// The entries already recorded, or `None` if absent or mangled.
    pub(super) fn carve(existing: &str) -> Option<&str> {
        let (_, rest) = existing.split_once(BEGIN)?;
        let (kept, _) = rest.split_once(END)?;
        Some(kept.trim_start_matches('\n'))
    }

    /// Removes every entry recorded under `id`, for starting a rune over.
    ///
    /// Text rather than parsing, for the same reason `carve` is.
    pub(super) fn drop_named(entries: &str, id: &str) -> String {
        let marker = format!("            id: \"{id}\",");
        let Some(at) = entries.find(&marker) else {
            return entries.to_string();
        };
        let start = entries[..at].rfind("        (\n").unwrap_or(at);
        let end = entries[at..]
            .find("\n        ),\n")
            .map(|offset| at + offset + "\n        ),\n".len())
            .unwrap_or(entries.len());
        // Recursive, because a rune now has several samples and starting it
        // over has to clear all of them.
        drop_named(&format!("{}{}", &entries[..start], &entries[end..]), id)
    }

    /// Forgets every sample of one rune, so it can be traced afresh.
    pub fn forget(id: &str) -> Result<String, String> {
        let existing = std::fs::read_to_string(OUTFILE).unwrap_or_default();
        let Some(kept) = carve(&existing) else {
            return Err("nothing recorded yet".to_string());
        };
        let before = kept.matches(&format!("id: \"{id}\"")).count();
        if before == 0 {
            return Err(format!("no samples of {id} to forget"));
        }
        let entries = drop_named(kept, id);
        std::fs::write(OUTFILE, wrap(&entries)).map_err(|e| e.to_string())?;
        Ok(format!("forgot {before} sample(s) of {id}"))
    }

    /// Saves the pad's rune under `id`, and says what it wrote.
    ///
    /// `id` is the name chosen on the panel. It used to be `RENAME_ME_n` always,
    /// which meant every recording ended in a text editor — and the one workflow
    /// §2 actually asks for is a person tracing the manga's shapes into the app.
    /// Making that end in the app is the difference between a feature and a chore.
    pub fn record(
        pad: &InkPad,
        ring_strokes: &[u32],
        id: &str,
        kind: &str,
    ) -> Result<String, String> {
        let strokes = gestures(pad, ring_strokes)?;

        let existing = std::fs::read_to_string(OUTFILE).unwrap_or_default();
        let kept = match carve(&existing) {
            Some(kept) => kept.to_string(),
            None => {
                // A file we cannot carve is either absent or something we did
                // not write. It still holds a traced rune, and tracing one is
                // the expensive half of this whole project, so it is moved
                // aside rather than overwritten.
                if !existing.is_empty() {
                    let _ = std::fs::rename(OUTFILE, format!("{OUTFILE}.bak"));
                }
                String::new()
            }
        };

        // **Appended, not replaced.** Several samples of one rune is how a
        // hand-drawn shape gets recognised reliably: your fire is never twice
        // the same, and the recognizer picks whichever sample the drawing is
        // nearest. Three to five is the usual figure.
        //
        // The earlier version replaced, on the reasoning that re-tracing is how
        // a shape gets better — true, and it made "record fire five times" leave
        // exactly one fire.
        let samples = kept.matches(&format!("id: \"{id}\"")).count() + 1;
        let entries = format!("{kept}{}", entry(id, kind, &strokes));
        std::fs::write(OUTFILE, wrap(&entries)).map_err(|e| e.to_string())?;

        Ok(format!(
            "traced {id} as a {} - sample {samples}: {} stroke(s), {} pts",
            kind.to_lowercase(),
            strokes.len(),
            strokes.iter().map(Vec::len).sum::<usize>()
        ))
    }
}

// At the file's own level, not inside `mod desktop`. A `#[path]` in a nested
// inline module resolves against `src/ui/record/desktop/`, and `src/ui/record/`
// is not a real directory, so the `..` walk fails before it can normalise. Out
// here the base is `src/ui/`, which is one hop from `src/tests/`.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../tests/record.rs"]
mod tests;
