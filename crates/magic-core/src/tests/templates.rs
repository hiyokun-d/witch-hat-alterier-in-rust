//! Tests for `templates`. Kept out of the source file for room to
//! breathe; still a child module of it, so private items stay reachable.

use super::*;

use crate::stroke::MATCH_POINTS;

/// The fixture set: plain geometry, not canon runes. See the file's header.
const FIXTURES: &str = include_str!("../../tests/data/templates.ron");
/// The same shapes drawn by hand — §5's golden gestures.
const GOLDEN: &str = include_str!("../../tests/data/golden.ron");

/// The real file, which ships empty until the shapes are traced.
const SHIPPED: &str = include_str!("../../the-magic-assets/templates.ron");

fn fixtures() -> Vec<Recorded> {
    parse("templates.ron", FIXTURES, MATCH_POINTS).unwrap()
}

#[test]
fn the_fixture_set_loads() {
    let loaded = fixtures();
    assert_eq!(loaded.len(), 6);
    assert!(loaded.iter().all(|r| r.kind == Kind::Sign));
    assert!(
        loaded
            .iter()
            .all(|r| r.template.cloud.points.len() == MATCH_POINTS)
    );
}

/// The shipped file is empty on purpose — the shapes have to be traced from
/// the manga, and a made-up sigil would be worse than none. This test exists
/// so that emptiness stays a decision rather than becoming an accident.
#[test]
fn the_shipped_template_file_parses_and_is_still_empty() {
    let loaded = parse("templates.ron", SHIPPED, MATCH_POINTS).unwrap();
    assert!(
        loaded.is_empty(),
        "shapes have been traced — delete this test and check them against the catalogue instead"
    );
}

/// Every id in a real template file must name something the catalogue knows,
/// or a typo becomes a rune nobody can draw.
#[test]
fn a_template_naming_nothing_is_rejected() {
    let catalog = crate::catalog::Catalog::parse(
        include_str!("../../the-magic-assets/sigils.ron"),
        include_str!("../../the-magic-assets/signs.ron"),
        include_str!("../../the-magic-assets/spells.ron"),
    )
    .unwrap();

    // The fixtures are geometry, not runes, so none of their names exist.
    assert!(check(&fixtures(), &catalog).is_err());

    let real = parse(
        "templates.ron",
        r#"(version: 1, templates: [(id: "fire", kind: Sigil,
             strokes: [[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]])])"#,
        MATCH_POINTS,
    )
    .unwrap();
    assert!(check(&real, &catalog).is_ok(), "fire is a real sigil");
}

/// Sigils and signs are separate namespaces, so the same name in each is fine.
#[test]
fn the_same_name_in_both_vocabularies_is_allowed() {
    let source = r#"(version: 1, templates: [
        (id: "vision", kind: Sigil, strokes: [[(0.0, 0.0), (9.0, 1.0), (4.0, 8.0)]]),
        (id: "vision", kind: Sign,  strokes: [[(0.0, 0.0), (1.0, 9.0), (8.0, 4.0)]]),
    ])"#;
    assert_eq!(parse("t.ron", source, MATCH_POINTS).unwrap().len(), 2);
}

/// Twice in one vocabulary is a **second sample**, and that is the point.
///
/// This used to be rejected, on the reasoning that "which one wins would be
/// decided by file order and nothing else". That is wrong for a
/// nearest-neighbour recognizer: both compete, the *nearer* wins, and since
/// they carry the same name the answer is identical either way. What varies is
/// which sample of a person's handwriting the drawing lands closest to — which
/// is the entire reason to record a rune more than once.
#[test]
fn the_same_name_twice_in_one_vocabulary_is_a_second_sample() {
    let source = r#"(version: 1, templates: [
        (id: "fire", kind: Sigil, strokes: [[(0.0, 0.0), (9.0, 1.0), (4.0, 8.0)]]),
        (id: "fire", kind: Sigil, strokes: [[(0.0, 0.0), (1.0, 9.0), (8.0, 4.0)]]),
    ])"#;
    let loaded = parse("t.ron", source, MATCH_POINTS).expect("two samples must load");
    assert_eq!(loaded.len(), 2);
    assert!(loaded.iter().all(|r| r.template.name == "fire"));
}

/// A rune that cannot be normalised would never match anything and never say
/// why, so it is a load error rather than a silent skip.
#[test]
fn a_gesture_with_no_shape_is_a_load_error() {
    let source = r#"(version: 1, templates: [
        (id: "dot", kind: Sign, strokes: [[(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)]]),
    ])"#;
    let failed = parse("t.ron", source, MATCH_POINTS).unwrap_err();
    assert!(failed.to_string().contains("dot"), "{failed}");
}

#[test]
fn a_file_from_another_version_is_refused() {
    let source = r#"(version: 99, templates: [])"#;
    let failed = parse("t.ron", source, MATCH_POINTS).unwrap_err();
    assert!(failed.to_string().contains("version"), "{failed}");
}

#[test]
fn a_malformed_file_says_which_file_it_was() {
    let failed = parse("mine.ron", "(this is not ron", MATCH_POINTS).unwrap_err();
    assert!(failed.to_string().starts_with("mine.ron"), "{failed}");
}

/// Several marks are one gesture, and the recogniser never sees the gaps.
#[test]
fn a_multi_stroke_gesture_loads_as_one_template() {
    let loaded = fixtures();
    let cross = loaded
        .iter()
        .find(|r| r.template.name == "cross")
        .expect("the fixture set has a two-stroke cross");

    assert_eq!(cross.template.cloud.points.len(), MATCH_POINTS);
    assert!(cross.template.cloud.points.iter().any(|p| p.stroke_id == 0));
    assert!(cross.template.cloud.points.iter().any(|p| p.stroke_id == 1));
}

// ── golden ──────────────────────────────────────────────────────────────────

/// §5. Six shapes drawn as a hand draws them — moved, resized, turned a few
/// degrees, noisy, several of them backwards or with the strokes in another
/// order — each must still come back as itself.
#[test]
fn every_golden_gesture_recognises_as_itself() {
    let loaded = fixtures();
    let cases = parse("golden.ron", GOLDEN, MATCH_POINTS).unwrap();
    let all = templates(&loaded);

    for case in &cases {
        let found =
            crate::recognizer::classify(&case.template.cloud, &all).expect("something to match");
        assert_eq!(
            all[found.index].name, case.template.name,
            "{:?} was read as {:?} at distance {}",
            case.template.name, all[found.index].name, found.distance
        );
    }
}

/// A ranking is only useful if the gap between first and second means
/// something. Without this, every golden case could pass by a hair.
///
/// The bar is deliberately low, because measuring it turned up something worth
/// knowing: **a square and a ring are close neighbours at 32 points** — 0.110
/// against 0.127, a margin of 16%. Both are closed convex loops of roughly
/// even radius, and thirty-two samples is not many to tell "has corners" from
/// "does not".
///
/// It is not a bug and it is not fixable here. Doubling `MATCH_POINTS` would
/// separate them and would also cost `2^2.5` — call it 2.3ms against §7's 1ms
/// budget. The real lesson is for whoever traces the runes: two that differ
/// only in roundness will be hard to tell apart, and the catalogue is better
/// off not containing such a pair.
#[test]
fn every_golden_gesture_wins_by_a_margin() {
    let loaded = fixtures();
    let cases = parse("golden.ron", GOLDEN, MATCH_POINTS).unwrap();
    let all = templates(&loaded);

    for case in &cases {
        let ranked = crate::recognizer::rank(&case.template.cloud, &all);
        assert!(
            ranked[1].distance > ranked[0].distance * 1.1,
            "{:?}: best {:.4}, runner-up {:?} at {:.4}",
            case.template.name,
            ranked[0].distance,
            all[ranked[1].index].name,
            ranked[1].distance
        );
    }
}

#[test]
fn identify_names_the_nearest_template() {
    let loaded = fixtures();
    let drawn: Vec<Point> = (0..40)
        .map(|i| Point {
            x: 300.0 + i as f32 * 4.0,
            y: -200.0,
            stroke_id: 0,
        })
        .collect();

    let (name, _) = identify(&drawn, &loaded, MATCH_POINTS).unwrap();
    assert_eq!(name, "stripe");
}

#[test]
fn identify_of_nothing_is_none() {
    assert!(identify(&[], &fixtures(), MATCH_POINTS).is_none());
}

#[test]
fn loading_is_deterministic() {
    assert_eq!(fixtures(), fixtures());
}
