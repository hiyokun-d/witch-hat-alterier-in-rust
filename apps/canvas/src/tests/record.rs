//! Tests for `record`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

// The file-writing half lives in `record::desktop`, which is where the
// filesystem is allowed to exist. These reach into it directly because what
// they test is the format, not the panel.
use super::desktop::{carve, entry, record, wrap};

use crate::Point;

fn pad_with(strokes: &[Vec<(f32, f32)>]) -> InkPad {
    let mut pad = InkPad::default();
    for (id, stroke) in strokes.iter().enumerate() {
        for (x, y) in stroke {
            pad.points.push(Point {
                x: *x,
                y: *y,
                stroke_id: id as u32,
            });
        }
        pad.stroke_id = id as u32 + 1;
    }
    pad
}

#[test]
fn an_empty_pad_records_nothing() {
    assert!(record(&InkPad::default(), "fire", "Sigil").is_err());
}

#[test]
fn a_pad_holding_only_a_ring_records_nothing() {
    // A ring is the activator, not a rune. The recogniser is never asked to
    // identify one — `circle.rs` does that with geometry.
    let ring: Vec<(f32, f32)> = (0..200)
        .map(|i| {
            let a = i as f32 / 199.0 * std::f32::consts::TAU;
            (120.0 * a.cos(), 120.0 * a.sin())
        })
        .collect();
    assert!(record(&pad_with(&[ring]), "fire", "Sigil").is_err());
}

#[test]
fn a_single_point_stroke_is_not_a_gesture() {
    assert!(record(&pad_with(&[vec![(0.0, 0.0)]]), "fire", "Sigil").is_err());
}

/// A stroke shaped enough to normalise: a short arc, ten points.
fn mark() -> Vec<(f32, f32)> {
    (0..10)
        .map(|i| (i as f32 * 3.0, (i as f32 * 0.4).sin() * 20.0))
        .collect()
}

#[test]
fn a_recorded_rune_parses_back_as_a_template() {
    // The round trip is the only thing that makes the file worth writing:
    // core's parser has to accept what the shell emits, or the overlay cannot
    // read a traced rune back and neither can `templates.ron`.
    let file = wrap(&entry("RENAME_ME_1", "Sigil", &[mark(), mark()]));
    let parsed = magic_core::templates::parse("test", &file, magic_core::stroke::MATCH_POINTS)
        .expect("recorded file must parse");

    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0].template.name, "RENAME_ME_1");
    assert_eq!(
        parsed[0].template.cloud.points.len(),
        magic_core::stroke::MATCH_POINTS
    );
}

#[test]
fn recording_twice_keeps_both_runes() {
    // Accumulating is what lets several runes be traced in one sitting and
    // compared against each other. Overwriting would make the board a mirror
    // of the last stroke instead of a catalogue.
    let first = wrap(&entry("RENAME_ME_1", "Sigil", &[mark()]));
    let kept = carve(&first).expect("markers must survive wrapping");
    let both = wrap(&format!(
        "{kept}{}",
        entry("RENAME_ME_2", "Sigil", &[mark()])
    ));

    let parsed = magic_core::templates::parse("test", &both, magic_core::stroke::MATCH_POINTS)
        .expect("two runes must parse");
    assert_eq!(parsed.len(), 2);
}

#[test]
fn a_mangled_file_is_carved_as_nothing_rather_than_guessed() {
    // Losing a recording is bad; appending into a file whose shape we no
    // longer recognise is worse, because the result would not parse at all.
    assert!(carve("(version: 1, templates: [])").is_none());
}

#[test]
fn the_placeholder_id_counts_what_is_already_there() {
    let file = wrap(&entry("RENAME_ME_1", "Sigil", &[mark()]));
    let kept = carve(&file).unwrap();
    assert_eq!(kept.matches("            id: \"").count(), 1);
}
