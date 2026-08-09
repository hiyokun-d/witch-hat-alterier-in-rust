//! Tests for `catalog`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

/// The real asset files, baked in at compile time.
///
/// `include_str!` is a test convenience only — the shipped path is
/// [`Catalog::parse`] over strings the shell read at runtime, which is what
/// keeps §4.4's "adding a spell must not require recompiling" true.
fn catalog() -> Catalog {
    Catalog::parse(
        include_str!("../../the-magic-assets/sigils.ron"),
        include_str!("../../the-magic-assets/signs.ron"),
        include_str!("../../the-magic-assets/spells.ron"),
    )
    .expect("shipped assets must load")
}

#[test]
fn the_shipped_assets_load() {
    let c = catalog();
    assert_eq!(c.sigils().count(), 13);
    assert_eq!(c.signs().count(), 35);
    assert_eq!(c.spells().count(), 13);
    assert_eq!(c.edge_cases().count(), 7);
}

#[test]
fn wind_moves_air_but_cannot_create_it() {
    let caps = catalog().capabilities(&"wind".into()).unwrap();
    assert!(caps.move_);
    assert!(!caps.create);
}

#[test]
fn aeriforms_creates_air_but_cannot_move_it() {
    let caps = catalog().capabilities(&"aeriforms".into()).unwrap();
    assert!(caps.create);
    assert!(!caps.move_);
}

#[test]
fn earth_manipulates_but_never_creates() {
    let caps = catalog().capabilities(&"earth".into()).unwrap();
    assert!(caps.manipulate);
    assert!(!caps.create);
}

#[test]
fn every_air_sigil_is_in_the_air_family() {
    let c = catalog();
    for id in ["wind", "aeriforms", "wind_underfoot", "whorling_winds"] {
        assert_eq!(c.sigil(&id.into()).unwrap().family, Family::Air);
    }
}

/// Rotation is canon and belongs to a sigil, which is why there is no
/// rotate sign to look up.
#[test]
fn whorling_winds_is_the_only_sigil_that_imparts_rotation() {
    let c = catalog();
    let spinners: Vec<&str> = c
        .sigils()
        .filter(|def| def.imparts_rotation)
        .map(|def| def.id.as_str())
        .collect();
    assert_eq!(spinners, vec!["whorling_winds"]);
    assert!(c.sign(&"rotate".into()).is_none());
}

#[test]
fn water_is_the_only_sigil_with_a_creation_surcharge() {
    let c = catalog();
    let surcharged: Vec<&str> = c
        .sigils()
        .filter(|def| def.create_cost_multiplier.is_some())
        .map(|def| def.id.as_str())
        .collect();
    assert_eq!(surcharged, vec!["water"]);
}

#[test]
fn exactly_three_signs_can_replace_a_sigil() {
    let c = catalog();
    let substitutes: Vec<&str> = c
        .signs()
        .filter(|def| def.can_substitute_as_sigil)
        .map(|def| def.id.as_str())
        .collect();
    assert_eq!(substitutes, vec!["billow", "repetition", "vision"]);
}

#[test]
fn region_is_official_and_enlarge_is_reconstructed() {
    let c = catalog();
    assert_eq!(c.sign(&"region".into()).unwrap().tier, Tier::Official);
    assert_eq!(c.sign(&"enlarge".into()).unwrap().tier, Tier::Unofficial);
    assert!(c.is_decorative(&"bird".into()));
}

/// A sign can be officially named and still have no known effect. The two
/// axes are independent, and collapsing them would lose the distinction.
#[test]
fn undescribed_official_signs_are_official_with_unknown_confidence() {
    let c = catalog();
    for id in ["cool", "glaives", "entwine"] {
        let def = c.sign(&id.into()).unwrap();
        assert_eq!(def.tier, Tier::Official);
        assert_eq!(def.confidence, Confidence::Unknown);
    }
}

#[test]
fn billow_requires_collection_to_gather_its_material() {
    let c = catalog();
    let billow = c.sign(&"billow".into()).unwrap();
    assert_eq!(billow.requires, vec![SignId::from("collection")]);
}

#[test]
fn pulling_reads_its_angle_as_a_pull_twist_mix() {
    let c = catalog();
    let pulling = c.sign(&"pulling".into()).unwrap();
    assert_eq!(pulling.angle_semantics, Some(AngleSemantics::PullTwistMix));
}

#[test]
fn region_records_all_four_canon_arrangements() {
    let c = catalog();
    let patterns: Vec<RegionPattern> = c
        .sign(&"region".into())
        .unwrap()
        .arrangements
        .iter()
        .map(|a| a.pattern)
        .collect();
    assert_eq!(
        patterns,
        vec![
            RegionPattern::AllSameSide,
            RegionPattern::AllInward,
            RegionPattern::AllOutward,
            RegionPattern::Opposed,
        ]
    );
}

/// Two spells, same sigil, same sign kind, different arrangement. The
/// fixture the region analyser exists to satisfy.
#[test]
fn water_bolt_and_floating_drops_differ_only_by_arrangement() {
    let c = catalog();
    let bolt = c.spell("water_bolt").unwrap();
    let drops = c.spell("floating_drops").unwrap();
    assert_eq!(bolt.sigil, drops.sigil);
    assert_eq!(bolt.region_arrangement, Some(RegionPattern::AllSameSide));
    assert_eq!(drops.region_arrangement, Some(RegionPattern::Opposed));
}

/// CLAUDE.md §2.1: a spell needs no sigil at all.
#[test]
fn some_canon_spells_have_no_sigil() {
    let c = catalog();
    let sigil_less: Vec<&str> = c
        .spells()
        .filter(|spell| spell.sigil.is_none())
        .map(|spell| spell.id.as_str())
        .collect();
    assert!(sigil_less.contains(&"serpents_bed_of_sand"));
    assert!(sigil_less.contains(&"floating_expansion"));
}

/// Canon rule 6: a spell and its reversed twin cancel. These two are the
/// documented pair.
#[test]
fn wall_breaker_and_integration_are_the_reversal_pair() {
    let c = catalog();
    let breaker = c.spell("wall_breaker_seal").unwrap();
    let integration = c.spell("integration").unwrap();
    assert_eq!(breaker.sigil, integration.sigil);
    assert_eq!(breaker.signs, integration.signs);
    assert!(breaker.reversed_signs.is_empty());
    assert_eq!(integration.reversed_signs, vec![SignId::from("crushing")]);
}

/// The most surprising rule in the research: an empty closed ring is not
/// inert, it detonates.
#[test]
fn an_empty_closed_ring_is_a_documented_edge_case() {
    let c = catalog();
    let case = c
        .edge_cases()
        .find(|e| e.id == "empty_closed_ring")
        .expect("empty ring must be documented");
    assert!(case.canon_effect.as_ref().unwrap().contains("shockwave"));
}

#[test]
fn lookups_of_unknown_ids_return_none_rather_than_guessing() {
    let c = catalog();
    assert!(c.sigil(&"plasma".into()).is_none());
    assert!(c.sign(&"direction".into()).is_none());
    assert!(c.capabilities(&"plasma".into()).is_none());
    assert!(!c.can_substitute_as_sigil(&"plasma".into()));
}

#[test]
fn iteration_is_sorted_and_therefore_deterministic() {
    let c = catalog();
    let once: Vec<&str> = c.signs().map(|d| d.id.as_str()).collect();
    let twice: Vec<&str> = c.signs().map(|d| d.id.as_str()).collect();
    assert_eq!(once, twice);
    assert!(once.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn a_malformed_file_reports_which_file_it_was() {
    let err = Catalog::parse("(sigils: [", "(signs: [])", "(spells: [])").unwrap_err();
    assert!(matches!(
        err,
        CatalogError::Parse {
            file: "sigils.ron",
            ..
        }
    ));
}

#[test]
fn a_dangling_sign_reference_is_rejected_at_load() {
    let signs = r#"(signs: [
        ( id: "billow", tier: Official, confidence: Canon, requires: ["nonexistent"] ),
    ])"#;
    let err = Catalog::parse("(sigils: [])", signs, "(spells: [])").unwrap_err();
    assert_eq!(
        err,
        CatalogError::UnknownReference {
            from: "billow".to_owned(),
            to: "nonexistent".to_owned(),
        }
    );
}

#[test]
fn a_duplicate_id_is_rejected_at_load() {
    let signs = r#"(signs: [
        ( id: "bird", tier: Decorative, confidence: Canon ),
        ( id: "bird", tier: Decorative, confidence: Canon ),
    ])"#;
    let err = Catalog::parse("(sigils: [])", signs, "(spells: [])").unwrap_err();
    assert_eq!(
        err,
        CatalogError::DuplicateId {
            file: "signs.ron",
            id: "bird".to_owned(),
        }
    );
}
