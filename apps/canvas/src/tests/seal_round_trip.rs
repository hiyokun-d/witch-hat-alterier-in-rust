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
    let owned = assembly::owned(&rings, points, ON_RING_TOLERANCE);
    for ((glyph, ring), ids) in glyphs.iter_mut().zip(rings.iter()).zip(owned.iter()) {
        naming::name(glyph, ring, points, ids, shapes);
    }
    glyphs
}

#[test]
fn every_closed_preset_reads_back_as_the_spell_it_stamped() {
    let shapes = shapes();
    let catalog = catalog();

    // Nested workings have their own test: they lay down three rings on
    // purpose, and "one ring, one spell" is not the claim being made about them.
    for preset in PRESETS
        .iter()
        .filter(|preset| !preset.open && preset.with.is_empty())
    {
        // Skip anything whose rune nobody has traced — `preset` refuses those
        // in the app, so a failure here would be the fixture's fault, not the
        // geometry's.
        let traced = shapes
            .iter()
            .any(|rune| rune.kind == templates::Kind::Sigil && rune.template.name == preset.sigil);
        if !traced {
            continue;
        }

        let points = ink(stamp::working(&shapes, preset, Vec2::ZERO, 140.0));
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
        let points = ink(stamp::working(&shapes, preset, Vec2::ZERO, 140.0));
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

#[test]
fn the_shell_has_exactly_one_ring_search() {
    // **The bug this file keeps finding, in its purest form.** `watch_the_pad`,
    // `cast_pad` and the recorder each ran their own `find_rings` with
    // `assembly::RingSearch::default()` — `join: 16.0` — while `reading.rs`
    // searches with the shell's own `CLOSURE_TOLERANCE` of `20.0`. A ring whose
    // ends sat eighteen pixels apart was therefore **closed to the caption and
    // open to the watcher**: the seal read `ACTIVE - circuit closed` and nothing
    // ever fired, which is how it was reported — "the particle is not showing
    // up", and it was never the particles.
    //
    // Grepping the source is a blunt test and the right one. The failure is not
    // that some function returns a wrong number; it is that a *second* reading
    // exists at all, and no assertion about behaviour can see that.
    let shell = [
        include_str!("../sim.rs"),
        include_str!("../debug.rs"),
        include_str!("../props.rs"),
        include_str!("../particles.rs"),
        include_str!("../ui/mod.rs"),
        include_str!("../ui/bar.rs"),
        include_str!("../ui/place.rs"),
        include_str!("../ui/stamp.rs"),
        include_str!("../ui/record.rs"),
        include_str!("../ui/tutor.rs"),
    ];
    for source in shell {
        for (n, line) in source.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            assert!(
                !code.contains("find_rings"),
                "line {} searches for rings itself: {}\n\
                 The pad is read once, in reading.rs. Use `Reading`.",
                n + 1,
                line.trim()
            );
            assert!(
                !code.contains("RingSearch::default()"),
                "line {} uses core's default tolerances rather than the shell's: {}",
                n + 1,
                line.trim()
            );
        }
    }
}

#[test]
fn the_shells_tolerances_are_wider_than_cores_defaults() {
    // Why the drift mattered rather than merely existed. Core's defaults are
    // tuned for "a pen a few pixels wide"; this shell draws a five-pixel pen and
    // needs more slack. The two were never going to agree, so the only safe
    // arrangement is one of them being unreachable.
    let ours = crate::reading::search();
    let theirs = assembly::RingSearch::default();
    assert!(
        ours.join > theirs.join,
        "the shell joins at {} and core at {} - if these ever match, this test \
         stops proving anything and the one above is what still matters",
        ours.join,
        theirs.join
    );
}

#[test]
fn a_stamped_water_orb_gathers_rather_than_spreads() {
    // **The bug the screenshot showed, end to end.** Four arrows drawn at the
    // middle, and the engine read the seal as *diverging* — so the gathering
    // pull never engaged and the water fell straight through the ring.
    //
    // The cause was in `naming`: orientation was "the direction to the furthest
    // point", and an arrowhead drags the centroid toward the tip, which makes
    // the furthest point the **tail**. Every keystone pointed backwards.
    //
    // Asserted on the compiled spell rather than on the sign angles, because
    // the angles are an intermediate and `Convergence` is the thing the
    // simulation actually obeys.
    use magic_core::arrangement::Convergence;

    let shapes = shapes();
    let catalog = catalog();
    let orb = PRESETS
        .iter()
        .find(|preset| preset.inward && !preset.open)
        .expect("a gathering preset");

    let points = ink(stamp::working(&shapes, orb, Vec2::ZERO, 140.0));
    let glyphs = read(&points, &shapes);
    let spell = magic_core::compile(&glyphs[0], &catalog, &CompileRules::default());

    assert_eq!(
        spell.focus.convergence,
        Convergence::Converging,
        "arrows drawn at the middle read as {:?}",
        spell.focus.convergence
    );
    assert!(
        spell.focus.offset() < 60.0,
        "they gather {}px off the centre of a 140px ring",
        spell.focus.offset()
    );
}

#[test]
fn reversing_the_arrows_reverses_the_spell() {
    // The other half, and the one a fix could quietly break: the same sigil with
    // its arrows turned outward is a fountain, and if *both* arrangements read
    // as converging the reading is worth nothing. §2.3's whole point is that the
    // arrangement does the work.
    use magic_core::arrangement::Convergence;

    let shapes = shapes();
    let catalog = catalog();
    let orb = PRESETS
        .iter()
        .find(|preset| preset.inward && !preset.open)
        .expect("a gathering preset");

    // The same seal with its arrows turned the other way.
    let mut fountain = *orb;
    fountain.inward = false;
    let points = ink(stamp::working(&shapes, &fountain, Vec2::ZERO, 140.0));
    let glyphs = read(&points, &shapes);
    let spell = magic_core::compile(&glyphs[0], &catalog, &CompileRules::default());

    assert_eq!(
        spell.focus.convergence,
        Convergence::Diverging,
        "arrows drawn outward read as {:?}",
        spell.focus.convergence
    );
}

#[test]
fn a_nested_working_reads_back_as_two_gated_seals() {
    // **Canon rule 4, drawn for the first time.** The engine has handled nesting
    // since M5.2 and nothing in the app had ever laid one down, so the rule was
    // reachable only from tests.
    //
    // Two seals inside one ring: fire below throwing up, water held above. The
    // compiler is told nothing about steam — the elements meet in the *world*,
    // and `reactions.ron` decides what that means.
    let shapes = shapes();
    let catalog = catalog();
    let kettle = PRESETS
        .iter()
        .find(|preset| !preset.with.is_empty())
        .expect("a nested preset");

    let points = ink(stamp::working(&shapes, kettle, Vec2::ZERO, 200.0));
    let glyphs = read(&points, &shapes);
    assert_eq!(glyphs.len(), 3, "expected an outer ring and two seals");

    // Exactly one ring has no parent, and the other two answer to it.
    let outer: Vec<usize> = (0..glyphs.len())
        .filter(|&i| glyphs[i].parent.is_none())
        .collect();
    assert_eq!(outer.len(), 1, "nesting did not resolve to one gate");
    let gate = magic_core::glyph::GlyphId(outer[0] as u32);
    for (i, glyph) in glyphs.iter().enumerate() {
        if i != outer[0] {
            assert_eq!(glyph.parent, Some(gate), "seal {i} is not gated");
        }
    }

    // Both elements are named, and the outer ring claims neither of them —
    // canon rule 4 puts its spell in the *gap*, and `assembly::owned` is what
    // stops it swallowing what it merely encloses.
    let named: Vec<&str> = glyphs
        .iter()
        .filter_map(|g| g.sigil.as_ref().map(|id| id.as_str()))
        .collect();
    assert!(named.contains(&"fire"), "no fire in {named:?}");
    assert!(named.contains(&"water"), "no water in {named:?}");
    assert_eq!(
        glyphs[outer[0]].sigil, None,
        "the outer ring claimed a sigil it only encloses"
    );

    // And it compiles: the gate is closed, so both inner spells fire.
    let spells = magic_core::compile_all(&glyphs, &catalog, &CompileRules::default());
    let firing = spells.iter().filter(|s| s.fires()).count();
    assert!(firing >= 2, "only {firing} of the working fired");
}

#[test]
fn an_open_gate_holds_the_whole_working_back() {
    // Rule 4's settled half: "the inner ring will only activate if the outer
    // ring is completed, even if there is no gap in the inner ring". The inner
    // seals here are drawn perfectly closed, and must still wait.
    let shapes = shapes();
    let catalog = catalog();
    let kettle = PRESETS
        .iter()
        .find(|preset| !preset.with.is_empty())
        .expect("a nested preset");

    let mut opened = *kettle;
    opened.open = true;
    let points = ink(stamp::working(&shapes, &opened, Vec2::ZERO, 200.0));
    let glyphs = read(&points, &shapes);
    let spells = magic_core::compile_all(&glyphs, &catalog, &CompileRules::default());

    assert!(
        spells.iter().all(|spell| !spell.fires()),
        "a working fired through an open gate"
    );
}
