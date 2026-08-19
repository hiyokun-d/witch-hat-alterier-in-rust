//! Tests for `compiler`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use crate::Point;
use crate::catalog::{Family, RegionPattern, Tier};
use crate::glyph::{Glaive, Ring, Sign};
use core::f32::consts::{FRAC_PI_2, PI};

/// The real asset files. The compiler's whole job is asking the catalogue
/// questions, so testing it against a hand-made stub would test the stub.
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

/// A ring good enough to fire: closed, and neater than `min_quality`.
fn good_ring() -> Ring {
    Ring::new(at(0.0, 0.0), 100.0, true, 0.99)
}

/// A glyph with a sigil filling most of its ring and no signs.
fn glyph(sigil: Option<&str>) -> Glyph {
    let mut g = Glyph::new(GlyphId(1), sigil.map(SigilId::from), good_ring());
    g.sigil_extent = 50.0;
    g
}

/// One sign, placed on the ring at `placement` and aimed at `orientation`.
fn sign(kind: &str, placement: f32, orientation: f32) -> Sign {
    Sign {
        kind: kind.into(),
        placement,
        orientation,
        size: 10.0,
        reversed: false,
    }
}

fn build(glyph: &Glyph) -> Spell {
    compile(glyph, &catalog(), &CompileRules::default())
}

fn has(spell: &Spell, warning: &Warning) -> bool {
    spell.warnings.contains(warning)
}

// ---- rule 9: the empty case is not the trivial case -----------------------

#[test]
fn bare_ring_compiles_to_a_discharge() {
    let spell = build(&glyph(None));
    assert_eq!(spell.driver, Driver::Discharge);
    assert!(spell.is_discharge());
}

#[test]
fn bare_ring_fires() {
    // The trap this guards: "nothing in the ring" reading as "nothing to do".
    let spell = build(&glyph(None));
    assert!(spell.fires());
    assert_eq!(spell.firing, Firing::Active);
}

#[test]
fn bare_ring_is_warned_about_but_not_failed() {
    let spell = build(&glyph(None));
    assert!(has(&spell, &Warning::BareRing));
}

#[test]
fn bare_ring_is_full_intensity() {
    // There is nothing smaller than the ring for a discharge to be a fraction
    // of, so the sigil-size ratio does not apply.
    assert_eq!(build(&glyph(None)).intensity, 1.0);
}

#[test]
fn signs_without_a_driver_shape_a_discharge() {
    let mut g = glyph(None);
    g.signs.push(sign("cool", 0.0, 0.0));
    let spell = build(&g);
    assert_eq!(spell.driver, Driver::Discharge);
    assert!(has(&spell, &Warning::NoDriver));
    assert!(!has(&spell, &Warning::BareRing));
}

// ---- rule 2: an open ring is prepared, not broken -------------------------

#[test]
fn open_ring_does_not_fire() {
    let mut g = glyph(Some("fire"));
    g.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.99);
    let spell = build(&g);
    assert_eq!(spell.firing, Firing::Inert);
    assert!(!spell.fires());
}

#[test]
fn open_ring_still_compiles_its_contents() {
    // The whole point of rule 2: the spell is fully prepared and waiting on one
    // stroke, so everything about it must already be known.
    let mut g = glyph(Some("water"));
    g.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.99);
    g.signs.push(sign("column", FRAC_PI_2, FRAC_PI_2));
    let spell = build(&g);
    assert_eq!(spell.driver, Driver::Sigil("water".into()));
    assert_eq!(spell.effective, vec![SignId::from("column")]);
    assert!(has(&spell, &Warning::RingOpen));
}

#[test]
fn open_ring_is_not_judged_on_neatness() {
    // Structure before craft: how roughly an unfinished ring was inked is not
    // yet a question.
    let mut g = glyph(Some("fire"));
    g.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.10);
    let spell = build(&g);
    assert_eq!(spell.firing, Firing::Inert);
    assert!(
        !spell
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::RoughRing { .. }))
    );
}

// ---- rule 8: quality gates the closed ring -------------------------------

#[test]
fn rough_closed_ring_is_fleeting_not_dead() {
    let mut g = glyph(Some("fire"));
    g.ring = Ring::new(at(0.0, 0.0), 100.0, true, 0.50);
    let spell = build(&g);
    assert_eq!(spell.firing, Firing::Fleeting);
    // The wiki says a rough ring gives a fleeting effect *or* fails, so it does
    // something briefly rather than nothing.
    assert!(spell.fires());
}

// ---- drivers -------------------------------------------------------------

#[test]
fn a_known_sigil_drives_the_spell() {
    let spell = build(&glyph(Some("fire")));
    assert_eq!(spell.driver, Driver::Sigil("fire".into()));
}

#[test]
fn an_unknown_sigil_is_reported_not_invented() {
    let spell = build(&glyph(Some("blancmange")));
    assert!(has(&spell, &Warning::UnknownSigil("blancmange".into())));
    assert_eq!(spell.driver, Driver::Discharge);
}

#[test]
fn a_substitute_sign_can_stand_in_for_a_sigil() {
    let mut g = glyph(None);
    g.signs.push(sign("billow", 0.0, 0.0));
    let spell = build(&g);
    assert_eq!(spell.driver, Driver::Substitute("billow".into()));
    assert!(!has(&spell, &Warning::BareRing));
}

#[test]
fn every_substitute_the_catalogue_names_can_drive_a_spell() {
    let c = catalog();
    let substitutes: Vec<SignId> = c
        .signs()
        .filter(|def| def.can_substitute_as_sigil)
        .map(|def| def.id.clone())
        .collect();
    assert!(!substitutes.is_empty());
    for id in substitutes {
        let mut g = glyph(None);
        g.signs.push(sign(id.as_str(), 0.0, 0.0));
        let spell = compile(&g, &c, &CompileRules::default());
        assert_eq!(spell.driver, Driver::Substitute(id.clone()), "{id:?}");
    }
}

#[test]
fn a_real_sigil_outranks_a_substitute() {
    let mut g = glyph(Some("water"));
    g.signs.push(sign("billow", 0.0, 0.0));
    assert_eq!(build(&g).driver, Driver::Sigil("water".into()));
}

// ---- capabilities: the conservation rules -------------------------------

#[test]
fn wind_moves_air_but_cannot_create_it() {
    let caps = build(&glyph(Some("wind"))).caps.expect("wind is a sigil");
    assert!(caps.move_);
    assert!(!caps.create);
}

#[test]
fn aeriforms_creates_air_but_cannot_move_it() {
    let caps = build(&glyph(Some("aeriforms")))
        .caps
        .expect("aeriforms is a sigil");
    assert!(caps.create);
    assert!(!caps.move_);
}

#[test]
fn a_discharge_has_no_capabilities_to_conserve() {
    assert_eq!(build(&glyph(None)).caps, None);
}

// ---- intensity: sigil size against the ring -----------------------------

#[test]
fn a_sigil_filling_its_ring_is_full_strength() {
    let mut g = glyph(Some("fire"));
    g.sigil_extent = 100.0;
    assert!((build(&g).intensity - 1.0).abs() < 1e-5);
}

#[test]
fn a_small_sigil_in_a_large_ring_is_weak() {
    let mut g = glyph(Some("fire"));
    g.sigil_extent = 20.0;
    assert!((build(&g).intensity - 0.2).abs() < 1e-5);
}

#[test]
fn an_unmeasured_sigil_says_so_rather_than_reporting_zero() {
    let mut g = glyph(Some("fire"));
    g.sigil_extent = 0.0;
    let spell = build(&g);
    assert!(has(&spell, &Warning::IntensityUnmeasured));
    assert_eq!(spell.intensity, 1.0);
}

// ---- signs ---------------------------------------------------------------

#[test]
fn an_unknown_sign_is_reported_and_contributes_nothing() {
    let mut g = glyph(Some("fire"));
    g.signs.push(sign("squiggle", 0.0, 0.0));
    let spell = build(&g);
    assert!(has(&spell, &Warning::UnknownSign("squiggle".into())));
    assert!(spell.effective.is_empty());
}

#[test]
fn a_decorative_sign_is_dropped_silently() {
    // Nothing in signs.ron is decorative since the chapter 78 retcon, so this
    // asserts the retcon rather than the filter — if a decorative sign ever
    // comes back, the filter is already there.
    let c = catalog();
    assert_eq!(c.signs().filter(|d| d.tier == Tier::Decorative).count(), 0);
}

#[test]
fn reversing_a_non_directional_sign_is_reported() {
    // §2.3: a radial sign has no front, so there is no way to point it inward.
    let mut g = glyph(Some("water"));
    let mut s = sign("cool", 0.0, 0.0);
    s.reversed = true;
    g.signs.push(s);
    assert!(has(&build(&g), &Warning::NotReversible("cool".into())));
}

#[test]
fn reversing_a_directional_sign_is_fine() {
    let mut g = glyph(Some("water"));
    let mut s = sign("column", 0.0, 0.0);
    s.reversed = true;
    g.signs.push(s);
    assert!(
        !build(&g)
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::NotReversible(_)))
    );
}

// ---- balance: where the spell actually goes (§2.4) -----------------------

#[test]
fn four_equal_columns_shoot_straight() {
    let mut g = glyph(Some("fire"));
    for i in 0..4 {
        let angle = i as f32 * FRAC_PI_2;
        g.signs.push(sign("column", angle, angle));
    }
    let spell = build(&g);
    assert!(spell.balance.lean() < 0.01);
    assert!(
        !spell
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::Unbalanced { .. }))
    );
}

#[test]
fn one_oversized_column_steers_the_spell() {
    // Canon's own example: identical columns shoot straight up, one much longer
    // "has more power, causing uneven pressure which makes the spell shoot off
    // to the side".
    let mut g = glyph(Some("fire"));
    for i in 0..4 {
        let angle = i as f32 * FRAC_PI_2;
        let mut s = sign("column", angle, angle);
        if i == 0 {
            s.size = 60.0;
        }
        g.signs.push(s);
    }
    let spell = build(&g);
    assert!(spell.balance.lean() > 0.1);
    assert!(
        spell
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::Unbalanced { .. }))
    );
}

#[test]
fn an_uneven_non_directional_sign_is_strong_not_lopsided() {
    // Semi- and non-directional signs change strength only, so a seal of
    // mismatched crush signs must not be reported as steering anywhere.
    let mut g = glyph(Some("earth"));
    for i in 0..4 {
        let angle = i as f32 * FRAC_PI_2;
        let mut s = sign("cool", angle, angle);
        if i == 0 {
            s.size = 60.0;
        }
        g.signs.push(s);
    }
    assert_eq!(build(&g).balance.lean(), 0.0);
}

// ---- region: computed collectively, not per sign ------------------------

#[test]
fn region_signs_all_pointing_inward_confine_the_spell() {
    let mut g = glyph(Some("water"));
    for i in 0..4 {
        let angle = i as f32 * FRAC_PI_2;
        // Aimed back at the centre from wherever it sits.
        g.signs.push(sign("region", angle, angle + PI));
    }
    assert_eq!(build(&g).region.pattern(), Some(RegionPattern::AllInward));
}

#[test]
fn region_signs_all_pointing_outward_leave_the_seal_empty() {
    let mut g = glyph(Some("water"));
    for i in 0..4 {
        let angle = i as f32 * FRAC_PI_2;
        g.signs.push(sign("region", angle, angle));
    }
    assert_eq!(build(&g).region.pattern(), Some(RegionPattern::AllOutward));
}

#[test]
fn no_region_signs_means_absent_not_a_guess() {
    let mut g = glyph(Some("fire"));
    g.signs.push(sign("column", 0.0, 0.0));
    assert_eq!(build(&g).region, RegionArrangement::Absent);
}

// ---- determinism (§5) ----------------------------------------------------

#[test]
fn the_same_glyph_compiles_identically_twice() {
    let mut g = glyph(Some("fire"));
    for i in 0..3 {
        let angle = i as f32 * 2.0;
        g.signs.push(sign("column", angle, angle));
    }
    assert_eq!(build(&g), build(&g));
}

#[test]
fn compiling_never_panics_on_a_degenerate_ring() {
    // No panics in core (§4.7). A zero-radius ring is reachable from a
    // single-point stroke and must report rather than divide by nothing.
    let mut g = glyph(Some("fire"));
    g.ring = Ring::new(at(0.0, 0.0), 0.0, true, 1.0);
    let spell = build(&g);
    assert!(spell.intensity.is_finite());
}

#[test]
fn every_warning_renders() {
    // The overlay prints these, so an unreachable arm would surface as a blank
    // line rather than a build error.
    let mut g = glyph(Some("nonesuch"));
    g.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.1);
    g.sigil_extent = 0.0;
    g.signs.push(sign("squiggle", 0.0, 0.0));
    for warning in &build(&g).warnings {
        assert!(!warning.to_string().is_empty());
    }
}

// ---- rule 4: the outermost ring gates everything inside it ---------------

/// `outer` holds `inner`. Rings nest by containment, so the ids are what say so.
fn nested(outer_closed: bool) -> Vec<Glyph> {
    let mut outer = Glyph::new(GlyphId(1), Some("fire".into()), good_ring());
    outer.ring = Ring::new(at(0.0, 0.0), 200.0, outer_closed, 0.99);
    let mut inner = glyph(Some("water"));
    inner.id = GlyphId(2);
    inner.parent = Some(GlyphId(1));
    vec![outer, inner]
}

fn build_all(glyphs: &[Glyph]) -> Vec<Spell> {
    compile_all(glyphs, &catalog(), &CompileRules::default())
}

#[test]
fn an_open_outer_ring_holds_a_closed_inner_one_shut() {
    // Canon: "the inner ring will only activate if the outer ring is completed,
    // even if there is no gap in the inner ring."
    let spells = build_all(&nested(false));
    assert_eq!(spells[1].firing, Firing::Inert);
    assert!(has(&spells[1], &Warning::OuterRingOpen(GlyphId(1))));
}

#[test]
fn a_closed_outer_ring_lets_the_inner_one_fire() {
    let spells = build_all(&nested(true));
    assert_eq!(spells[1].firing, Firing::Active);
}

#[test]
fn gating_reaches_through_a_chain_of_rings() {
    let mut glyphs = nested(false);
    let mut deepest = glyph(Some("earth"));
    deepest.id = GlyphId(3);
    deepest.parent = Some(GlyphId(2));
    glyphs.push(deepest);
    let spells = build_all(&glyphs);
    // Two levels out, and still held shut.
    assert_eq!(spells[2].firing, Firing::Inert);
}

#[test]
fn a_nesting_cycle_reports_rather_than_hangs() {
    let mut a = glyph(Some("fire"));
    a.id = GlyphId(1);
    a.parent = Some(GlyphId(2));
    let mut b = glyph(Some("fire"));
    b.id = GlyphId(2);
    b.parent = Some(GlyphId(1));
    assert!(has(&build_all(&[a, b])[0], &Warning::NestingCycle));
}

#[test]
fn a_parent_that_is_not_on_the_pad_is_reported_not_fatal() {
    let mut lone = glyph(Some("fire"));
    lone.parent = Some(GlyphId(99));
    let spells = build_all(&[lone]);
    assert!(has(&spells[0], &Warning::DanglingLink(GlyphId(99))));
    assert_eq!(spells[0].firing, Firing::Active);
}

// ---- rule 5: identical linked seals amplify -----------------------------

fn linked_pair(second_reversed: bool) -> Vec<Glyph> {
    let mut a = glyph(Some("fire"));
    a.id = GlyphId(1);
    a.signs.push(sign("column", 0.0, 0.0));
    a.linked = vec![GlyphId(2)];

    let mut b = glyph(Some("fire"));
    b.id = GlyphId(2);
    let mut s = sign("column", 0.0, 0.0);
    s.reversed = second_reversed;
    b.signs.push(s);
    vec![a, b]
}

#[test]
fn identical_linked_seals_amplify_each_other() {
    let spells = build_all(&linked_pair(false));
    assert!(spells[0].amplification > 1.0);
    assert!(spells[0].strength() > spells[0].intensity);
}

#[test]
fn amplification_beats_drawing_one_bigger_seal() {
    // The whole point of rule 5: the total must grow faster than the count.
    let total: f32 = 4.0 * amplification(4);
    assert!(total > 4.0);
}

#[test]
fn amplification_of_a_lone_seal_is_one() {
    assert_eq!(amplification(1), 1.0);
    assert_eq!(build_all(&[glyph(Some("fire"))])[0].amplification, 1.0);
}

#[test]
fn a_link_is_mutual_even_recorded_one_way() {
    // A drawn line has no direction, and which glyph was assembled first must
    // not change the answer (§4.3).
    let spells = build_all(&linked_pair(false));
    assert_eq!(spells[0].amplification, spells[1].amplification);
}

#[test]
fn different_spells_linked_together_do_not_amplify() {
    let mut pair = linked_pair(false);
    pair[1].sigil = Some("water".into());
    assert_eq!(build_all(&pair)[0].amplification, 1.0);
}

// ---- rule 6: a seal and its reversed twin cancel ------------------------

#[test]
fn a_seal_and_its_reversed_twin_cancel_completely() {
    let spells = build_all(&linked_pair(true));
    assert!(spells[0].cancelled);
    assert!(spells[1].cancelled);
    assert_eq!(spells[0].strength(), 0.0);
    assert_eq!(spells[1].strength(), 0.0);
}

#[test]
fn cancellation_names_the_twin_that_did_it() {
    assert!(has(
        &build_all(&linked_pair(true))[0],
        &Warning::Cancelled(GlyphId(2))
    ));
}

#[test]
fn a_twin_cancels_rather_than_amplifies() {
    // Same spell, so `same_spell` is true for both — cancellation has to be
    // asked first or a twin would read as a copy.
    let spells = build_all(&linked_pair(true));
    assert_eq!(spells[0].amplification, 1.0);
}

#[test]
fn a_partly_reversed_seal_is_not_a_twin() {
    // Canon says a spell and its reversed twin "cancel out completely", which
    // only follows if the inversion is total.
    let mut pair = linked_pair(true);
    pair[0].signs.push(sign("levitation", 1.0, 1.0));
    pair[1].signs.push(sign("levitation", 1.0, 1.0));
    assert!(!build_all(&pair)[0].cancelled);
}

#[test]
fn reversing_a_sign_that_cannot_be_reversed_makes_no_twin() {
    // A flag nobody can act on must not make a seal look like somebody's twin.
    let mut a = glyph(Some("water"));
    a.id = GlyphId(1);
    a.signs.push(sign("cool", 0.0, 0.0));
    a.linked = vec![GlyphId(2)];
    let mut b = glyph(Some("water"));
    b.id = GlyphId(2);
    let mut s = sign("cool", 0.0, 0.0);
    s.reversed = true;
    b.signs.push(s);
    let spells = build_all(&[a, b]);
    assert!(!spells[0].cancelled);
    assert!(has(&spells[1], &Warning::NotReversible("cool".into())));
}

#[test]
fn compiling_a_pad_is_deterministic() {
    let pad = linked_pair(false);
    assert_eq!(build_all(&pad), build_all(&pad));
}

#[test]
fn an_empty_pad_compiles_to_nothing() {
    assert!(build_all(&[]).is_empty());
}

// ---- M5.3: what a sign needs beside it (§2.3 data) ----------------------

#[test]
fn billow_does_nothing_without_collection() {
    // The catalogue's own words: billow "needs collection to gather the
    // material first". Without it there is nothing for the sign to act on.
    let mut g = glyph(None);
    g.signs.push(sign("billow", 0.0, 0.0));
    let spell = build(&g);
    assert!(has(
        &spell,
        &Warning::Unsatisfied {
            sign: "billow".into(),
            needs: "collection".into(),
        }
    ));
    assert!(!spell.effective.contains(&"billow".into()));
}

#[test]
fn billow_works_once_collection_is_drawn() {
    let mut g = glyph(None);
    g.signs.push(sign("billow", 0.0, 0.0));
    g.signs.push(sign("collection", PI, PI));
    let spell = build(&g);
    assert!(spell.effective.contains(&"billow".into()));
    assert!(
        !spell
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::Unsatisfied { .. }))
    );
}

#[test]
fn enlarge_needs_either_selection_or_diamond() {
    let mut g = glyph(Some("earth"));
    g.signs.push(sign("enlarge", 0.0, 0.0));
    assert!(
        build(&g)
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::UnsatisfiedOneOf { .. }))
    );

    // Either one satisfies it — that is what `requires_one_of` means.
    for companion in ["selection", "diamond"] {
        let mut g = glyph(Some("earth"));
        g.signs.push(sign("enlarge", 0.0, 0.0));
        g.signs.push(sign(companion, PI, PI));
        assert!(
            !build(&g)
                .warnings
                .iter()
                .any(|w| matches!(w, Warning::UnsatisfiedOneOf { .. })),
            "{companion}"
        );
    }
}

#[test]
fn a_pairing_is_a_note_not_a_requirement() {
    // Bolt is "paired with region to aim it", but the wiki never says it fails
    // alone — so it is reported and kept.
    let mut g = glyph(Some("water"));
    g.signs.push(sign("bolt", 0.0, 0.0));
    let spell = build(&g);
    assert!(has(
        &spell,
        &Warning::Unpaired {
            sign: "bolt".into(),
            usually_with: "region".into(),
        }
    ));
    assert!(spell.effective.contains(&"bolt".into()));
}

#[test]
fn every_requirement_in_the_catalogue_can_be_satisfied() {
    // A requirement naming a sign nobody can draw would be an unbuildable
    // spell, and it would look like a compiler bug rather than a data one.
    let c = catalog();
    for def in c.signs() {
        for want in def.requires.iter().chain(def.requires_one_of.iter()) {
            assert!(c.sign(want).is_some(), "{:?} needs {want:?}", def.id);
            assert_ne!(&def.id, want, "{:?} requires itself", def.id);
        }
    }
}

// ---- §2.1: glaives, the fourth kind of mark -----------------------------

#[test]
fn a_seal_with_no_glaives_embeds_nothing() {
    assert_eq!(build(&glyph(Some("fire"))).embedding, 0.0);
}

#[test]
fn glaives_set_how_firmly_the_spell_embeds() {
    let mut g = glyph(Some("obliviation"));
    g.glaives.push(Glaive {
        placement: 0.0,
        size: 40.0,
    });
    g.glaives.push(Glaive {
        placement: PI,
        size: 30.0,
    });
    // Against a ring of 100.
    assert!((build(&g).embedding - 0.7).abs() < 1e-5);
}

#[test]
fn a_glaive_outside_the_ring_still_counts() {
    // Rule 1's one exception: "glaives can be drawn outside of the ring as long
    // as they are still connected to it".
    let mut g = glyph(Some("obliviation"));
    g.glaives.push(Glaive {
        placement: 0.0,
        size: 50.0,
    });
    assert!(build(&g).embedding > 0.0);
}

#[test]
fn glaives_do_not_steer_the_spell() {
    // Neither sign nor sigil, so they must stay out of balance and symmetry.
    let mut g = glyph(Some("fire"));
    g.glaives.push(Glaive {
        placement: 0.0,
        size: 90.0,
    });
    assert_eq!(build(&g).balance.power, 0.0);
}

// ---- §2.4 / rule 8: three knobs, kept apart -----------------------------

#[test]
fn absolute_size_is_kept_apart_from_the_sigil_ratio() {
    // Rule 8 ("larger seals are more powerful") and §2.2 ("the size of a sigil
    // in relation to the ring") are two claims, not one.
    let mut small = glyph(Some("fire"));
    small.ring = Ring::new(at(0.0, 0.0), 50.0, true, 0.99);
    small.sigil_extent = 50.0;

    let mut large = glyph(Some("fire"));
    large.ring = Ring::new(at(0.0, 0.0), 400.0, true, 0.99);
    large.sigil_extent = 400.0;

    let (a, b) = (build(&small), build(&large));
    assert_eq!(a.intensity, b.intensity);
    assert!(b.scale > a.scale);
}

#[test]
fn sign_count_is_read_separately_from_sign_size() {
    // "The amount of signs will affect the range or quantity of magic
    // generated" — a different knob from power.
    let mut few = glyph(Some("fire"));
    few.signs.push(sign("column", 0.0, 0.0));
    let mut many = glyph(Some("fire"));
    for i in 0..4 {
        let a = i as f32 * FRAC_PI_2;
        many.signs.push(sign("column", a, a));
    }
    assert_eq!(build(&few).sign_count, 1);
    assert_eq!(build(&many).sign_count, 4);
}

// ---- M5.4: conservation, from the capability model (§3.2) ---------------

#[test]
fn wind_must_find_its_air() {
    let demand = build(&glyph(Some("wind"))).demand.expect("wind is a sigil");
    assert!(demand.must_find);
    assert!(!demand.substance.is_empty());
}

#[test]
fn aeriforms_may_make_its_own() {
    assert!(
        !build(&glyph(Some("aeriforms")))
            .demand
            .expect("aeriforms is a sigil")
            .must_find
    );
}

#[test]
fn earth_manipulates_but_never_creates() {
    assert!(
        build(&glyph(Some("earth")))
            .demand
            .expect("earth is a sigil")
            .must_find
    );
}

#[test]
fn every_sigil_that_cannot_create_names_what_it_needs() {
    // A conservation rule with no substance attached is unenforceable — the
    // simulation would know a spell must find something and not what.
    let c = catalog();
    //
    // Only the elemental families. Guidance attracts objects, obliviation
    // erases memory, doorways names a place — those act on no substance at all,
    // and an empty `affects` is the right answer for them rather than a hole.
    let elemental = [Family::Fire, Family::Water, Family::Earth, Family::Air];
    let silent: Vec<&str> = c
        .sigils()
        .filter(|def| elemental.contains(&def.family))
        .filter(|def| !def.caps.create && def.affects.is_empty())
        .map(|def| def.id.as_str())
        .collect();
    assert!(
        silent.is_empty(),
        "cannot create, and says nothing: {silent:?}"
    );
}

#[test]
fn a_discharge_demands_nothing() {
    assert_eq!(build(&glyph(None)).demand, None);
}

// ---- M5.6: the invariants from §5's table -------------------------------

#[test]
fn the_same_seal_compiles_the_same_ring_first_and_ring_last() {
    // §2.5: canon never constrains drawing order, so order must not be an
    // input. Signs pushed in opposite sequences are the same seal.
    let mut first = glyph(Some("water"));
    let mut last = glyph(Some("water"));
    let placements = [0.0f32, 1.5, 3.0, 4.5];
    for a in placements {
        first.signs.push(sign("column", a, a));
    }
    for a in placements.into_iter().rev() {
        last.signs.push(sign("column", a, a));
    }
    let (a, b) = (build(&first), build(&last));
    assert_eq!(a.balance.lean(), b.balance.lean());
    assert_eq!(a.driver, b.driver);
    assert_eq!(a.firing, b.firing);
    assert_eq!(a.sign_count, b.sign_count);
}

#[test]
fn an_open_ring_never_produces_an_effect() {
    // Canon rule 2, as a property rather than one case: whatever is in the
    // ring, an open one delivers nothing.
    for sigil in ["fire", "water", "earth", "wind"] {
        for signs in 0..4 {
            let mut g = glyph(Some(sigil));
            g.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.99);
            for i in 0..signs {
                let a = i as f32 * FRAC_PI_2;
                g.signs.push(sign("column", a, a));
            }
            assert_eq!(build(&g).strength(), 0.0, "{sigil} with {signs} signs");
        }
    }
}

#[test]
fn a_spell_and_its_reversed_twin_produce_zero_net_effect() {
    // Canon rule 6, over every reversible sign the catalogue has.
    let c = catalog();
    let reversible: Vec<SignId> = c
        .signs()
        .filter(|def| c.is_reversible(&def.id) == Some(true))
        .filter(|def| def.requires.is_empty() && def.requires_one_of.is_empty())
        .map(|def| def.id.clone())
        .collect();
    assert!(reversible.len() > 5, "expected a real sample");

    for kind in reversible {
        let mut a = glyph(Some("fire"));
        a.id = GlyphId(1);
        a.signs.push(sign(kind.as_str(), 0.0, 0.0));
        a.linked = vec![GlyphId(2)];

        let mut b = glyph(Some("fire"));
        b.id = GlyphId(2);
        let mut s = sign(kind.as_str(), 0.0, 0.0);
        s.reversed = true;
        b.signs.push(s);

        let spells = compile_all(&[a, b], &c, &CompileRules::default());
        let net: f32 = spells.iter().map(|s| s.strength()).sum();
        assert_eq!(net, 0.0, "{kind:?} did not cancel");
    }
}

#[test]
fn closing_a_split_ring_gives_the_same_spell_as_drawing_it_whole() {
    // Canon rule 3. Closure is state, so the only difference between the two is
    // the flag itself.
    let mut split = glyph(Some("water"));
    split.ring = Ring::new(at(0.0, 0.0), 100.0, false, 0.99);
    split.signs.push(sign("column", 0.0, 0.0));

    let mut whole = split.clone();
    whole.ring = Ring::new(at(0.0, 0.0), 100.0, true, 0.99);

    let joined = {
        let mut g = split.clone();
        g.ring = Ring::new(at(0.0, 0.0), 100.0, true, 0.99);
        build(&g)
    };
    assert_eq!(joined.strength(), build(&whole).strength());
    assert_eq!(joined.driver, build(&whole).driver);
}

// ---- M5.5: the recorded spells (§5 golden) ------------------------------

#[test]
fn every_spell_fixture_names_parts_the_catalogue_has() {
    // The fixtures are what M5.5 will compile against. A fixture naming a sign
    // nobody has is a broken test waiting to look like a compiler bug.
    let c = catalog();
    for spell in c.spells() {
        if let Some(sigil) = &spell.sigil {
            assert!(c.sigil(sigil).is_some(), "{}: {sigil:?}", spell.id);
        }
        for (sign, _) in &spell.signs {
            assert!(c.sign(sign).is_some(), "{}: {sign:?}", spell.id);
        }
    }
}

#[test]
fn every_fixture_compiles_to_something_that_fires() {
    // Built from the recorded parts, closed and neat, each fixture must produce
    // a spell — no fixture may be unbuildable by the compiler that serves it.
    let c = catalog();
    let rules = CompileRules::default();
    for fixture in c.spells() {
        let mut g = Glyph::new(GlyphId(1), fixture.sigil.clone(), good_ring());
        g.sigil_extent = 50.0;
        for (i, (kind, _)) in fixture.signs.iter().enumerate() {
            let a = i as f32 * 0.7;
            let mut s = sign(kind.as_str(), a, a);
            s.reversed = fixture.reversed_signs.contains(kind);
            g.signs.push(s);
        }
        let spell = compile(&g, &c, &rules);
        assert!(spell.fires(), "{} does not fire", fixture.id);
        if fixture.sigil.is_some() {
            assert!(!spell.is_discharge(), "{} lost its sigil", fixture.id);
        }
    }
}

// ---- honesty: unread is not the same as empty ---------------------------

#[test]
fn a_ring_full_of_unreadable_marks_is_not_called_bare() {
    // The overlay was reporting "bare ring — this is an explosion" over a seal
    // covered in ink. Rule 9 is about a ring with *nothing* in it.
    let mut g = glyph(None);
    g.unnamed = 7;
    let spell = build(&g);
    assert!(has(&spell, &Warning::Unreadable { strokes: 7 }));
    assert!(!has(&spell, &Warning::BareRing));
}

#[test]
fn a_genuinely_empty_ring_is_still_rule_9() {
    let spell = build(&glyph(None));
    assert!(has(&spell, &Warning::BareRing));
    assert!(
        !spell
            .warnings
            .iter()
            .any(|w| matches!(w, Warning::Unreadable { .. }))
    );
}
