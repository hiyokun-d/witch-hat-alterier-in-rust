//! The test a friend would run without knowing it: press `preset`, close the
//! ring, and get the spell it said you would get.
//!
//! **End to end on purpose.** Every piece below is covered by its own tests —
//! the stamp draws, the ring search finds, `naming` segments, the compiler
//! compiles — and the seal still failed, because the *geometry the stamp chose*
//! put the sigil close enough to the keystones that segmentation merged them.
//! No unit test could have caught it: each part did its job perfectly on input
//! the other part should never have produced.

use bevy::prelude::Vec2;
use magic_core::{CompileRules, assembly, naming, templates};

use crate::reading::{ON_RING_TOLERANCE, RULES, search};
use crate::sim::PRESETS;
use crate::ui::stamp;

/// The shapes the app matches against with `traced` on: whatever has been
/// traced, and the built-in *signs* only.
fn shapes() -> Vec<magic_core::Recorded> {
    let n = magic_core::stroke::MATCH_POINTS;
    let mut out = templates::parse(
        "recorded-gesture.ron",
        include_str!("../../../../recorded-gesture.ron"),
        n,
    )
    .unwrap_or_default();
    for rune in templates::built_ins(n) {
        let taken = out
            .iter()
            .any(|had| had.kind == rune.kind && had.template.name == rune.template.name);
        if !taken && rune.kind == templates::Kind::Sign {
            out.push(rune);
        }
    }
    out
}

fn catalog() -> magic_core::Catalog {
    magic_core::Catalog::parse(
        include_str!("../../../../crates/magic-core/the-magic-assets/sigils.ron"),
        include_str!("../../../../crates/magic-core/the-magic-assets/signs.ron"),
        include_str!("../../../../crates/magic-core/the-magic-assets/spells.ron"),
    )
    .expect("shipped assets must load")
}

/// Stamped strokes as the pad would hold them.
fn ink(strokes: Vec<Vec<Vec2>>) -> Vec<crate::Point> {
    let mut out = Vec::new();
    for (id, stroke) in strokes.iter().enumerate() {
        for point in stroke {
            out.push(crate::Point {
                x: point.x,
                y: point.y,
                stroke_id: id as u32,
            });
        }
    }
    out
}

/// Reads a stamped seal exactly as `reading.rs` does.
fn read(points: &[crate::Point], shapes: &[magic_core::Recorded]) -> Vec<magic_core::Glyph> {
    let rings = assembly::find_rings(points, &search());
    let mut glyphs = assembly::glyphs(&rings, points, ON_RING_TOLERANCE, &RULES);
    for (glyph, ring) in glyphs.iter_mut().zip(rings.iter()) {
        naming::name(glyph, ring, points, shapes, ON_RING_TOLERANCE);
    }
    glyphs
}

#[test]
fn every_closed_preset_reads_back_as_the_spell_it_stamped() {
    let shapes = shapes();
    let catalog = catalog();

    for preset in PRESETS.iter().filter(|preset| !preset.open) {
        // Skip anything whose rune nobody has traced — `preset` refuses those
        // in the app, so a failure here would be the fixture's fault, not the
        // geometry's.
        let traced = shapes
            .iter()
            .any(|rune| rune.kind == templates::Kind::Sigil && rune.template.name == preset.sigil);
        if !traced {
            continue;
        }

        let points = ink(stamp::seal(
            &shapes,
            preset.sigil,
            Vec2::ZERO,
            140.0,
            preset.signs,
            preset.open,
            preset.inward,
        ));
        let glyphs = read(&points, &shapes);
        assert_eq!(
            glyphs.len(),
            1,
            "{} laid down {} rings",
            preset.id,
            glyphs.len()
        );

        let glyph = &glyphs[0];
        assert_eq!(
            glyph.sigil.as_ref().map(|id| id.as_str()),
            Some(preset.sigil),
            "{} stamped a {} sigil the app could not read back",
            preset.id,
            preset.sigil
        );
        assert_eq!(
            glyph.unnamed, 0,
            "{} left {} mark(s) nobody could name",
            preset.id, glyph.unnamed
        );

        let spell = magic_core::compile(glyph, &catalog, &CompileRules::default());
        assert!(spell.fires(), "{} did not fire", preset.id);
        assert!(
            crate::sim::refuses(&spell, true).is_none(),
            "{} was refused: {:?}",
            preset.id,
            crate::sim::refuses(&spell, true)
        );
    }
}

#[test]
fn a_prepared_preset_is_armed_rather_than_firing() {
    // Canon rule 2. The open presets exist so the last stroke is yours, and a
    // geometry change that quietly closed them would take that away.
    let shapes = shapes();
    let catalog = catalog();

    for preset in PRESETS.iter().filter(|preset| preset.open) {
        let points = ink(stamp::seal(
            &shapes,
            preset.sigil,
            Vec2::ZERO,
            140.0,
            preset.signs,
            preset.open,
            preset.inward,
        ));
        let glyphs = read(&points, &shapes);
        let Some(glyph) = glyphs.first() else {
            continue;
        };
        let spell = magic_core::compile(glyph, &catalog, &CompileRules::default());
        assert!(
            !spell.fires(),
            "{} fired without anybody closing it",
            preset.id
        );
    }
}

#[test]
fn a_stamped_sigil_alone_reads_as_itself() {
    // The element buttons. They used to stamp a *reconstruction* while the
    // recogniser held your traced rune, so the app was placing a mark it could
    // not read.
    let shapes = shapes();
    for rune in shapes.iter().filter(|r| r.kind == templates::Kind::Sigil) {
        let name = rune.template.name.clone();
        let points = ink(stamp::glyph_mark(&shapes, &name, Vec2::ZERO, 40.0));
        assert!(!points.is_empty(), "{name} stamped nothing");

        let ids: Vec<u32> = points.iter().map(|p| p.stroke_id).collect();
        let read = naming::loose(&points, &ids, &shapes);
        assert_eq!(read.len(), 1, "{name} stamped {} marks", read.len());
        assert!(
            read[0].1.starts_with(&name),
            "{name} stamped something that reads as {:?}",
            read[0].1
        );
    }
}
