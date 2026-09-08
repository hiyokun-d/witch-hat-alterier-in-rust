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

// ---- where the magic goes, and what decides it ----------------------------

/// A seal with `n` keystones evenly spaced, each turned by `turn` from outward.
fn with_signs(sigil: &str, n: usize, turn: f32) -> Glyph {
    let mut glyph = seal(Some(sigil));
    glyph.signs = (0..n)
        .map(|slot| {
            let placement = std::f32::consts::TAU * slot as f32 / n as f32;
            crate::glyph::Sign {
                kind: "column".into(),
                placement,
                orientation: placement + turn,
                size: 20.0,
                reversed: false,
            }
        })
        .collect();
    glyph
}

/// How far the raised parcels sit from a point, on average.
fn spread_about(field: &Field, about: Vec2) -> f32 {
    let n = field.parcels().len().max(1) as f32;
    field
        .parcels()
        .iter()
        .map(|p| (p.at - about).length())
        .sum::<f32>()
        / n
}

#[test]
fn a_seal_with_no_signs_manifests_at_its_own_centre() {
    // **Reported from a screenshot**: a hand-drawn fire seal put its flame above
    // the ring rather than inside it. `placement`'s fallback aimed straight up,
    // quoting canon's line about column signs "all the same size, and as such,
    // the same power" — which is a claim about a seal with *balanced* signs and
    // not about a seal with **none**. Nothing configured this spell, so there is
    // no direction to obey.
    let mut field = grid();
    cast(
        &spell_for(Some("fire")),
        Vec2::ZERO,
        &mut field,
        &CastRules::default(),
    );
    assert!(
        spread_about(&field, Vec2::ZERO) < 40.0,
        "raised {}px from the centre of a 100px ring",
        spread_about(&field, Vec2::ZERO)
    );
    // And with no velocity imposed on it: fire rises because it is hot, which
    // is the whole reason buoyancy is modelled.
    let pushed = field
        .parcels()
        .iter()
        .map(|p| p.velocity.length())
        .fold(0.0f32, f32::max);
    assert!(
        pushed < 1.0,
        "a seal that says nothing pushed at {pushed}px/s"
    );
}

#[test]
fn arrows_pointing_inward_raise_the_magic_at_the_middle() {
    // The water orb's opening arrangement. `Balance` says this seal goes
    // nowhere — correctly, four arrows cancel — so until `focus` existed the
    // simulation had no way to tell it from a seal with no signs at all.
    let mut field = grid();
    let inward = compile(
        &with_signs("water", 4, std::f32::consts::PI),
        &catalog(),
        &CompileRules::default(),
    );
    cast(&inward, Vec2::ZERO, &mut field, &CastRules::default());

    for parcel in field.parcels() {
        assert!(
            parcel.at.length() < 100.0,
            "a gathering seal put substance {}px out, past its own ring",
            parcel.at.length()
        );
    }
}

#[test]
fn arrows_pointing_outward_throw_it_past_the_ring() {
    // The same sigil, arrows reversed, opposite spell — and the arrangement is
    // doing every bit of the work (§2.3).
    let mut field = grid();
    let outward = compile(
        &with_signs("water", 4, 0.0),
        &catalog(),
        &CompileRules::default(),
    );
    cast(&outward, Vec2::ZERO, &mut field, &CastRules::default());

    // Measured after it has travelled and against the opposite arrangement,
    // because where a fountain *starts* is not the interesting half and an
    // absolute distance would only be measuring how fast water falls.
    let reach_of = |spell: &Spell| {
        let mut sim = crate::sim::Sim::new(grid());
        sim.cast(spell, Vec2::ZERO);
        for _ in 0..60 {
            sim.step();
        }
        spread_about(&sim.field, Vec2::ZERO)
    };
    let gathering = compile(
        &with_signs("water", 4, std::f32::consts::PI),
        &catalog(),
        &CompileRules::default(),
    );
    let (out, in_) = (reach_of(&outward), reach_of(&gathering));
    assert!(
        out > in_ * 1.5,
        "arrows out reached {out:.0}px and arrows in {in_:.0}px - the \
         arrangement is not deciding anything"
    );
}

#[test]
fn a_gathering_spell_holds_its_substance_while_it_runs() {
    // **The orb, and the half placement cannot do.** Water falls; a seal that
    // only decides where it *appears* has lost it a second later. The pull is
    // what makes gathering a lasting state.
    let mut sim = crate::sim::Sim::new(grid());
    let orb = compile(
        &with_signs("water", 4, std::f32::consts::PI),
        &catalog(),
        &CompileRules::default(),
    );
    sim.cast(&orb, Vec2::ZERO);
    assert!(!sim.channels.is_empty(), "the orb has to keep running");

    for _ in 0..120 {
        sim.step();
    }
    let held = spread_about(&sim.field, Vec2::ZERO);
    assert!(
        held < 90.0,
        "the orb spread to {held}px after two seconds - it is not being held"
    );
}

#[test]
fn nothing_is_held_once_the_spell_has_finished() {
    // The honest ending, and the reason the pull lives on the **channel**
    // rather than on the parcel: a spell that held its water forever would be a
    // spell with no duration, and canon rule 8 grades every seal on how long it
    // lasts. Asserted on the force rather than on where the water ended up,
    // because a parcel's own lifetime expires long before a channel's does and
    // an empty field would pass a falls-to-the-floor test for the wrong reason.
    let mut sim = crate::sim::Sim::new(grid());
    sim.field.add(parcel("water", Vec2::new(60.0, 0.0), 1.0));

    let held = {
        let mut running = sim.clone();
        let orb = compile(
            &with_signs("water", 4, std::f32::consts::PI),
            &catalog(),
            &CompileRules::default(),
        );
        running.cast(&orb, Vec2::ZERO);
        running.step();
        running.field.parcels()[0].velocity.x
    };
    assert!(held < -1.0, "a running orb should pull inward, got {held}");

    // The same world with no spell in it leaves the water alone sideways.
    sim.step();
    assert!(
        sim.field.parcels()[0].velocity.x.abs() < 0.5,
        "something pulled with no spell running"
    );
}

#[test]
fn stirring_moves_what_is_there_and_makes_nothing() {
    // §3.2's rule, applied to the one liberty we take with canon: the pointer
    // may push what is already in the world and may not add to it.
    let mut sim = crate::sim::Sim::new(grid());
    sim.field.add(parcel("water", Vec2::new(10.0, 0.0), 1.0));
    let before = sim.field.mass();

    sim.stir(Vec2::ZERO, 60.0, Vec2::new(50.0, 0.0));
    assert_eq!(sim.field.mass(), before, "stirring created matter");
    assert!(sim.field.parcels()[0].velocity.x > 0.0);
}

#[test]
fn stirring_reaches_only_as_far_as_it_says() {
    let mut sim = crate::sim::Sim::new(grid());
    sim.field.add(parcel("water", Vec2::new(200.0, 0.0), 1.0));
    sim.stir(Vec2::ZERO, 60.0, Vec2::new(50.0, 0.0));
    assert_eq!(sim.field.parcels()[0].velocity, Vec2::ZERO);
}

#[test]
fn a_weak_seal_still_holds_its_water_up() {
    // **The screenshot's actual seal.** A sigil filling a fifth of its ring has
    // an intensity near `0.15`, and the first attempt scaled the holding force
    // by exactly that — so it pulled at 225px/s2 against 900 of gravity and the
    // water fell straight through the ring while the reading beside it said
    // `GATHERS at the centre`.
    //
    // Canon never described a tug of war. `water_orb`'s own catalogue entry
    // says "gravity is reduced and the water is held in suspension as a ball",
    // and that is a claim about weight being *taken away*.
    let mut glyph = with_signs("water", 4, std::f32::consts::PI);
    glyph.ring = Ring::new(at(0.0, 0.0), 200.0, true, 0.99);
    glyph.sigil_extent = 30.0; // a fifth of the ring: a weak spell
    let weak = compile(&glyph, &catalog(), &CompileRules::default());
    assert!(
        weak.strength() < 0.25,
        "the fixture stopped being a weak seal: {}",
        weak.strength()
    );

    let mut sim = crate::sim::Sim::new(grid());
    sim.cast(&weak, Vec2::ZERO);
    for _ in 0..180 {
        sim.step();
    }

    // The water's centre of mass, not its lowest speck: parcels expire and are
    // replaced constantly, so the lowest one is mostly a measure of how recently
    // somebody died on the way down. Where the *body* of water sits is the
    // question a person watching is asking.
    assert!(
        settled_height(&sim) > -60.0,
        "the water sank to {} in a 200px ring - it is not being held",
        settled_height(&sim)
    );
}

/// Where the mass of water actually sits, vertically.
fn settled_height(sim: &crate::sim::Sim) -> f32 {
    let mass: f32 = sim.field.parcels().iter().map(|p| p.mass).sum();
    if mass <= 0.0 {
        return 0.0;
    }
    sim.field
        .parcels()
        .iter()
        .map(|p| p.at.y * p.mass)
        .sum::<f32>()
        / mass
}

#[test]
fn without_suspension_the_same_seal_drops_its_water() {
    // The control, and the one that says *which* term is doing the work. A test
    // that only showed the water staying up could be passed by a stronger
    // spring, and a spring was the thing that was wrong.
    let mut glyph = with_signs("water", 4, std::f32::consts::PI);
    glyph.ring = Ring::new(at(0.0, 0.0), 200.0, true, 0.99);
    glyph.sigil_extent = 30.0;
    let weak = compile(&glyph, &catalog(), &CompileRules::default());

    let height = |suspend: f32| {
        let mut sim = crate::sim::Sim::new(grid());
        sim.cast_rules.suspend = suspend;
        sim.cast(&weak, Vec2::ZERO);
        for _ in 0..180 {
            sim.step();
        }
        settled_height(&sim)
    };
    let (carried, dropped) = (height(1.0), height(0.0));
    assert!(
        carried.abs() < 15.0,
        "a carried orb should sit at the focus, not {carried}"
    );
    assert!(
        carried > dropped + 20.0,
        "carrying the weight changed nothing: held at {carried}, dropped at {dropped}"
    );
}

#[test]
fn a_running_spell_does_not_pour_down_one_line() {
    // `cast` places by slot and a channel casts a *single* parcel per tick, so
    // slot is always zero — every drop of a long spell entered the world at the
    // same point, which is why a gathering seal poured a thin vertical stream
    // through the middle of its own ring instead of filling it.
    let mut sim = crate::sim::Sim::new(grid());
    let orb = compile(
        &with_signs("water", 4, std::f32::consts::PI),
        &catalog(),
        &CompileRules::default(),
    );
    sim.cast(&orb, Vec2::ZERO);
    for _ in 0..40 {
        sim.step();
    }

    let spread = |axis: fn(&Parcel) -> f32| {
        let (lo, hi) = sim
            .field
            .parcels()
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                (lo.min(axis(p)), hi.max(axis(p)))
            });
        hi - lo
    };
    assert!(
        spread(|p| p.at.x) > 30.0,
        "the whole spell landed in a {}px-wide line",
        spread(|p| p.at.x)
    );
}
