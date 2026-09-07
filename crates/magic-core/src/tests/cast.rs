//! Tests for `sim::cast` — the bridge from a compiled spell to matter.

use super::*;

use crate::catalog::{Catalog, SigilId};
use crate::compiler::{CompileRules, Firing, compile};
use crate::glyph::{Glyph, GlyphId, Ring};
use crate::sim::parcel::{Parcel, SubstanceId};
use crate::sim::vec2::Vec2;
use crate::{Point, sim::field::Field};

fn catalog() -> Catalog {
    Catalog::parse(
        include_str!("../../the-magic-assets/sigils.ron"),
        include_str!("../../the-magic-assets/signs.ron"),
        include_str!("../../the-magic-assets/spells.ron"),
    )
    .expect("shipped assets must load")
}

fn at(x: f32, y: f32) -> Point {
    Point { x, y, stroke_id: 0 }
}

/// A closed, neat ring holding a sigil that fills half of it.
fn seal(sigil: Option<&str>) -> Glyph {
    let mut g = Glyph::new(
        GlyphId(0),
        sigil.map(SigilId::from),
        Ring::new(at(0.0, 0.0), 100.0, true, 0.99),
    );
    g.sigil_extent = 50.0;
    g
}

fn spell_for(sigil: Option<&str>) -> Spell {
    compile(&seal(sigil), &catalog(), &CompileRules::default())
}

fn grid() -> Field {
    Field::new(40, 40, 20.0, Vec2::new(-400.0, -400.0)).expect("valid grid")
}

fn parcel(name: &str, at: Vec2, mass: f32) -> Parcel {
    Parcel::new(SubstanceId::new(name), at, mass)
}

// ---- canon rule 9: an explosion moves what is there, it does not make more --

#[test]
fn a_bare_ring_discharges_rather_than_raising_anything() {
    let mut field = grid();
    let report = cast(
        &spell_for(None),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert_eq!(report.outcome, CastOutcome::Discharged);
    assert_eq!(report.spawned, 0);
    assert_eq!(report.created, 0.0);
}

#[test]
fn a_discharge_conserves_mass_exactly() {
    let mut field = grid();
    field.add(parcel("air", Vec2::new(30.0, 0.0), 4.0));
    cast(
        &spell_for(None),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert_eq!(field.mass(), 4.0);
}

#[test]
fn a_discharge_shoves_what_is_in_reach_outward() {
    let mut field = grid();
    field.add(parcel("air", Vec2::new(30.0, 0.0), 1.0));
    cast(
        &spell_for(None),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(field.parcels()[0].velocity.x > 0.0);
}

#[test]
fn a_discharge_heats_what_it_catches() {
    let mut field = grid();
    field.add(parcel("air", Vec2::new(30.0, 0.0), 1.0));
    let before = field.parcels()[0].temperature;
    cast(
        &spell_for(None),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(field.parcels()[0].temperature > before);
}

#[test]
fn a_discharge_in_an_empty_room_does_nothing_visible() {
    // The honest answer, rather than a puff of invented smoke.
    let mut field = grid();
    let report = cast(
        &spell_for(None),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert_eq!(report.pushed, 0);
    assert!(field.parcels().is_empty());
}

#[test]
fn a_discharge_leaves_what_is_out_of_reach_alone() {
    let mut field = grid();
    field.add(parcel("air", Vec2::new(390.0, 0.0), 1.0));
    let before = field.parcels()[0].clone();
    cast(
        &spell_for(None),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert_eq!(field.parcels()[0], before);
}

// ---- canon rule 2: an open ring is prepared, not broken --------------------

#[test]
fn an_open_ring_raises_nothing() {
    let mut glyph = seal(Some("fire"));
    glyph.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.99);
    let spell = compile(&glyph, &catalog(), &CompileRules::default());

    let mut field = grid();
    let report = cast(&spell, Vec2::ZERO, &mut field, &CastRules::default());
    assert_eq!(report.outcome, CastOutcome::NotFiring(Firing::Inert));
    assert!(field.parcels().is_empty());
}

// ---- §3.2: the conservation model, which is the whole point ---------------

#[test]
fn a_sigil_that_can_create_raises_its_own_substance() {
    // Aeriforms creates air. It should need nothing to be there first.
    let mut field = grid();
    let report = cast(
        &spell_for(Some("aeriforms")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert_eq!(report.outcome, CastOutcome::Fired);
    assert!(report.created > 0.0);
    assert_eq!(report.found, 0.0);
    assert!(!field.parcels().is_empty());
}

#[test]
fn a_sigil_that_cannot_create_finds_nothing_in_an_empty_room() {
    // Wind moves air but cannot make it. This is the constraint earning its
    // keep: a wind spell in a vacuum does nothing at all.
    let mut field = grid();
    let report = cast(
        &spell_for(Some("wind")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(matches!(report.outcome, CastOutcome::NothingFound { .. }));
    assert_eq!(report.created, 0.0);
    assert!(field.parcels().is_empty());
}

#[test]
fn a_sigil_that_cannot_create_takes_what_is_already_there() {
    let mut field = grid();
    field.add(parcel("air", Vec2::new(20.0, 0.0), 50.0));
    let before = field.mass();

    let report = cast(
        &spell_for(Some("wind")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );

    assert_eq!(report.outcome, CastOutcome::Fired);
    assert!(report.found > 0.0);
    assert_eq!(report.created, 0.0);
    // Moved, never multiplied.
    assert!(
        (field.mass() - before).abs() < 1e-3,
        "mass {}",
        field.mass()
    );
}

#[test]
fn a_spell_that_finds_less_than_it_wanted_reports_the_shortfall() {
    let mut field = grid();
    field.add(parcel("air", Vec2::new(20.0, 0.0), 0.25));
    let report = cast(
        &spell_for(Some("wind")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(report.shortfall > 0.0);
    assert!((report.found - 0.25).abs() < 1e-4);
}

#[test]
fn substance_out_of_reach_might_as_well_not_exist() {
    let mut field = grid();
    field.add(parcel("air", Vec2::new(395.0, 395.0), 100.0));
    let report = cast(
        &spell_for(Some("wind")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(matches!(report.outcome, CastOutcome::NothingFound { .. }));
}

// ---- canon §2.4: count, size, lean and spin all decide something ----------

#[test]
fn more_signs_raise_more_parcels() {
    use crate::glyph::Sign;

    let plain = cast(
        &spell_for(Some("aeriforms")),
        Vec2::ZERO,
        &mut grid(),
        &CastRules::default(),
    );

    let mut crowded = seal(Some("aeriforms"));
    for i in 0..6 {
        let angle = i as f32;
        crowded.signs.push(Sign {
            kind: "column".into(),
            placement: angle,
            orientation: angle,
            size: 10.0,
            reversed: false,
        });
    }
    let spell = compile(&crowded, &catalog(), &CompileRules::default());
    let busy = cast(&spell, Vec2::ZERO, &mut grid(), &CastRules::default());

    assert!(busy.spawned > plain.spawned);
}

#[test]
fn a_fleeting_seal_produces_shorter_lived_parcels_than_an_active_one() {
    let rules = CastRules::default();

    let mut rough = seal(Some("aeriforms"));
    rough.ring = Ring::new(at(0.0, 0.0), 100.0, true, 0.10);
    let fleeting = compile(&rough, &catalog(), &CompileRules::default());
    assert_eq!(fleeting.firing, Firing::Fleeting);

    let mut a = grid();
    let mut b = grid();
    cast(&fleeting, Vec2::ZERO, &mut a, &rules);
    cast(&spell_for(Some("aeriforms")), Vec2::ZERO, &mut b, &rules);

    assert!(a.parcels()[0].life < b.parcels()[0].life);
}

#[test]
fn a_repetition_spell_anchors_what_it_raises() {
    // Canon §2.2, and the one sigil whose effect is a simulation rule outright.
    let mut field = grid();
    cast(
        &spell_for(Some("repetition")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(
        field.parcels().iter().all(|p| p.anchor.is_some()),
        "repetition must anchor every parcel it raises"
    );
}

#[test]
fn an_ordinary_spell_anchors_nothing() {
    let mut field = grid();
    cast(
        &spell_for(Some("aeriforms")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(field.parcels().iter().all(|p| p.anchor.is_none()));
}

// ---- §4.3 -----------------------------------------------------------------

#[test]
fn casting_the_same_spell_twice_gives_the_same_field() {
    let spell = spell_for(Some("aeriforms"));
    let build = || {
        let mut field = grid();
        cast(&spell, Vec2::ZERO, &mut field, &CastRules::default());
        field
    };
    assert_eq!(build(), build());
}

// ---- how long a channel runs, and the floor under it -----------------------

#[test]
fn every_firing_seal_channels_for_at_least_the_floor() {
    // **The floor is the interesting half.** Rule 8 grades a seal on how neatly
    // it was drawn, and grading it down to a splash that is over before you
    // have looked at it teaches nothing — a rough seal should be visibly worse
    // than a neat one and still be a spell. Ceiling and floor are both ours
    // (§2.6); canon gives no seconds at all.
    let rules = CastRules::default();
    let (floor, ceiling) = rules.channel_bounds;

    for sigil in ["fire", "water", "wind", "earth", "light", "aeriforms"] {
        let spell = spell_for(Some(sigil));
        let left = channel_for(&spell, &rules);
        assert!(
            (floor..=ceiling).contains(&left),
            "{sigil} channels for {left}s, outside {floor}..={ceiling}"
        );
    }
}

#[test]
fn a_seal_too_rough_to_hold_is_the_one_exception_to_the_floor() {
    // Canon's own word for a ring that cannot hold is `Fleeting`, and it has to
    // cost something or the grading is decoration.
    let mut glyph = seal(Some("water"));
    glyph.ring = Ring::new(at(0.0, 0.0), 100.0, true, 0.5);
    let rough = compile(&glyph, &catalog(), &CompileRules::default());
    assert_eq!(rough.firing, Firing::Fleeting);

    let rules = CastRules::default();
    let left = channel_for(&rough, &rules);
    assert!(left > 0.0, "a fleeting seal still casts");
    assert!(
        left < rules.channel_bounds.0,
        "fleeting ran for {left}s, which is not shorter than the floor"
    );
}

#[test]
fn a_seal_that_does_not_fire_channels_for_no_time_at_all() {
    // The bounds must not resurrect a spell canon says is not running: an open
    // ring is *prepared* (rule 2), and a floor applied before that check would
    // have given it fifteen seconds of summoning.
    let mut glyph = seal(Some("water"));
    glyph.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.99);
    let open = compile(&glyph, &catalog(), &CompileRules::default());
    assert!(!open.fires());
    assert_eq!(channel_for(&open, &CastRules::default()), 0.0);
}
