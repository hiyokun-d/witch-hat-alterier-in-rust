//! Tests for `sim` itself — and M6.8's conservation audit, which is the only
//! reason the capability model in §3.2 was worth encoding.

use super::*;

use crate::catalog::{Catalog, Family, SigilId};
use crate::compiler::{CompileRules, compile};
use crate::glyph::{Glyph, GlyphId, Ring};
use crate::{Point, sim::cast::CastOutcome};

fn catalog() -> Catalog {
    Catalog::parse(
        include_str!("../../the-magic-assets/sigils.ron"),
        include_str!("../../the-magic-assets/signs.ron"),
        include_str!("../../the-magic-assets/spells.ron"),
    )
    .expect("shipped assets must load")
}

fn spell_for(sigil: Option<&str>) -> Spell {
    let mut glyph = Glyph::new(
        GlyphId(0),
        sigil.map(SigilId::from),
        Ring::new(
            Point {
                x: 0.0,
                y: 0.0,
                stroke_id: 0,
            },
            100.0,
            true,
            0.99,
        ),
    );
    glyph.sigil_extent = 50.0;
    compile(&glyph, &catalog(), &CompileRules::default())
}

fn world() -> Sim {
    Sim::new(Field::centered(40, 20.0, Vec2::ZERO).expect("valid grid"))
}

#[test]
fn a_new_world_has_run_for_no_time_at_all() {
    let sim = world();
    assert_eq!(sim.ticks, 0);
    assert_eq!(sim.elapsed(), 0.0);
}

#[test]
fn elapsed_time_comes_from_the_tick_count_not_a_clock() {
    let mut sim = world();
    for _ in 0..60 {
        sim.step();
    }
    assert_eq!(sim.ticks, 60);
    assert!((sim.elapsed() - 1.0).abs() < 1e-4);
}

#[test]
fn resetting_empties_the_world_and_the_clock() {
    let mut sim = world();
    sim.cast(&spell_for(Some("aeriforms")), Vec2::ZERO);
    sim.step();
    sim.reset();
    assert!(sim.field.parcels().is_empty());
    assert_eq!(sim.ticks, 0);
}

// ---- M6.8: the conservation audit -----------------------------------------

#[test]
fn only_a_sigil_that_can_create_ever_creates() {
    // The audit that makes §3.2 more than documentation: walk every elemental
    // sigil in the catalogue, cast it into an empty room, and check that mass
    // appears from nothing only where the wiki says it may.
    let catalog = catalog();
    for def in catalog.sigils() {
        if !matches!(
            def.family,
            Family::Fire | Family::Water | Family::Earth | Family::Air
        ) {
            continue;
        }

        let mut sim = world();
        let spell = spell_for(Some(def.id.as_str()));
        let report = sim.cast(&spell, Vec2::ZERO);

        if report.created > 0.0 {
            assert!(
                def.caps.create,
                "{} created {} from nothing but cannot create",
                def.id.as_str(),
                report.created
            );
        }
        if !def.caps.create {
            assert!(
                matches!(
                    report.outcome,
                    CastOutcome::NothingFound { .. } | CastOutcome::Discharged
                ),
                "{} found substance in an empty room: {:?}",
                def.id.as_str(),
                report.outcome
            );
        }
    }
}

#[test]
fn a_world_stops_growing_once_its_spells_have_finished() {
    // The old form of this asserted mass never grows after a cast at all, and
    // channels made that false on purpose: a spell now keeps summoning for a
    // while, which is the whole difference between a firework and a fountain.
    //
    // The invariant that survives is the one that matters — when nothing is
    // running any more, nothing is being made any more.
    let mut sim = world();
    sim.cast(&spell_for(Some("aeriforms")), Vec2::ZERO);

    // Long enough for every channel to have run out. `CastRules::channel_bounds`
    // puts a floor of fifteen simulated seconds under any seal that fires, so
    // "long enough" is now the better part of a minute rather than ten seconds.
    for _ in 0..3600 {
        sim.step();
    }
    assert!(sim.channels.is_empty(), "a channel never finished");

    let settled = sim.field.mass();
    for _ in 0..120 {
        sim.step();
    }
    assert!(
        sim.field.mass() <= settled + 1e-3,
        "mass grew from {settled} to {} with nothing casting",
        sim.field.mass()
    );
}

#[test]
fn a_spell_keeps_summoning_after_the_first_burst() {
    let mut sim = world();
    sim.cast(&spell_for(Some("aeriforms")), Vec2::ZERO);
    let first = sim.field.mass();
    assert!(first > 0.0);
    assert!(!sim.channels.is_empty(), "nothing was left running");

    for _ in 0..30 {
        sim.step();
    }
    assert!(
        sim.field.mass() > first,
        "the spell stopped at its first burst: {first} then {}",
        sim.field.mass()
    );
}

#[test]
fn casting_the_same_seal_again_refreshes_rather_than_stacks() {
    // Otherwise a held button is an ocean.
    let mut sim = world();
    let spell = spell_for(Some("aeriforms"));
    for _ in 0..8 {
        sim.cast(&spell, Vec2::ZERO);
    }
    assert_eq!(sim.channels.len(), 1);
}

#[test]
fn a_neater_ring_summons_for_longer() {
    // Canon rule 8: "neatly drawn seals are more stable and long-lasting than
    // messy ones" — and that is a continuous claim, not a pass/fail one.
    let rules = crate::sim::CastRules::default();
    let neat = spell_for(Some("aeriforms"));

    let mut rough_glyph = Glyph::new(
        GlyphId(0),
        Some(SigilId::from("aeriforms")),
        Ring::new(
            Point {
                x: 0.0,
                y: 0.0,
                stroke_id: 0,
            },
            100.0,
            true,
            0.10,
        ),
    );
    rough_glyph.sigil_extent = 50.0;
    let rough = compile(&rough_glyph, &catalog(), &CompileRules::default());

    assert!(
        crate::sim::cast::channel_for(&rough, &rules)
            < crate::sim::cast::channel_for(&neat, &rules)
    );
}

#[test]
fn a_spell_that_cannot_fire_never_starts_a_channel() {
    let mut sim = world();
    let mut open = Glyph::new(
        GlyphId(0),
        Some(SigilId::from("aeriforms")),
        Ring::new(
            Point {
                x: 0.0,
                y: 0.0,
                stroke_id: 0,
            },
            100.0,
            false,
            0.99,
        ),
    );
    open.sigil_extent = 50.0;
    let spell = compile(&open, &catalog(), &CompileRules::default());

    sim.cast(&spell, Vec2::ZERO);
    assert!(sim.channels.is_empty());
}

#[test]
fn a_world_is_reproducible_from_the_same_inputs() {
    // §5's table: simulation state at frame N is identical across runs.
    let run = || {
        let mut sim = world();
        sim.cast(&spell_for(Some("aeriforms")), Vec2::ZERO);
        for tick in 0..90 {
            if tick == 30 {
                sim.cast(&spell_for(None), Vec2::new(20.0, 0.0));
            }
            sim.step();
        }
        sim
    };
    assert_eq!(run(), run());
}

#[test]
fn a_discharge_never_adds_mass_however_often_it_fires() {
    let mut sim = world();
    sim.cast(&spell_for(Some("aeriforms")), Vec2::ZERO);
    let raised = sim.field.mass();

    for _ in 0..20 {
        sim.cast(&spell_for(None), Vec2::ZERO);
    }
    assert!((sim.field.mass() - raised).abs() < 1e-3);
}

#[test]
fn nothing_ever_escapes_the_field() {
    let mut sim = world();
    sim.cast(&spell_for(Some("aeriforms")), Vec2::ZERO);
    for _ in 0..30 {
        sim.cast(&spell_for(None), Vec2::ZERO);
        sim.step();
    }
    assert!(
        sim.field.parcels().iter().all(|p| sim.field.contains(p.at)),
        "a parcel left the world"
    );
}
