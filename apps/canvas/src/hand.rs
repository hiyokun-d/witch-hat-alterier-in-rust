//! The scribe's hand: the typeface the app writes in.
//!
//! Bevy ships one font — a monospace — and it has two problems here. It has no
//! box-drawing, no typographic dashes and no degree sign, which is why every
//! string this app prints had to be flattened to ASCII. And it looks like a
//! terminal, which is the opposite of what a book of spells looks like.
//!
//! So there are two hands, and which one to use is a real decision each time:
//!
//! - **`script`** — EB Garamond, an old-style humanist serif cut after Claude
//!   Garamond's sixteenth-century romans. For anything a person *reads*:
//!   buttons, hints, the caption over a seal.
//! - **`plain`** — Bevy's own monospace, kept for anything that lines up in
//!   columns. A proportional serif would make the measurement overlay's tables
//!   ragged, and a table you cannot scan is worse than an ugly one.
//!
//! # Why the bytes are baked in
//!
//! `include_bytes!` rather than the asset server. `run.sh` builds a real `.app`
//! bundle and an asset path that survives being bundled is one more thing to get
//! wrong; the wasm build in M8 would need a second answer again. Eight hundred
//! kilobytes in the binary buys one answer that works everywhere.
//!
//! # Licence
//!
//! EB Garamond is under the SIL Open Font License 1.1, which permits embedding
//! and redistribution. The licence text ships beside the font in
//! `assets/fonts/OFL.txt` and must stay there — that is the condition.

use bevy::prelude::*;
use bevy::text::Font;

/// The typeface the app writes prose in.
///
/// Only the one handle. Bevy's own monospace is `Handle::default()`, so the
/// tables do not need a field to name it — and a field nobody reads is a field
/// that drifts.
#[derive(Resource, Debug, Clone)]
pub struct Hand {
    /// EB Garamond. For anything a person reads rather than scans.
    pub script: Handle<Font>,
}

/// Loads the hand before anything spawns text.
///
/// Done in `build` rather than in a `Startup` system because half the app spawns
/// its text at `Startup` too, and "before the other startup systems" is a
/// scheduling constraint nobody should have to remember.
pub struct HandPlugin;

impl Plugin for HandPlugin {
    fn build(&self, app: &mut App) {
        let script = {
            let mut fonts = app.world_mut().resource_mut::<Assets<Font>>();
            fonts.add(Font::from_bytes(
                include_bytes!("../assets/fonts/EBGaramond-Regular.ttf").to_vec(),
            ))
        };
        app.insert_resource(Hand { script });
    }
}
