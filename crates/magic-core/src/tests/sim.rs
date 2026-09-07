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
fn running_a_world_forward_never_changes_its_mass() {
    let mut sim = world();
    sim.cast(&spell_for(Some("aeriforms")), Vec2::ZERO);
    let raised = sim.field.mass();
    assert!(raised > 0.0);

    // Far enough that everything has settled against a wall or expired.
    for _ in 0..120 {
        sim.step();
    }
    let now = sim.field.mass();
    assert!(
        now <= raised + 1e-3,
        "mass grew from {raised} to {now} with nothing casting"
    );
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
