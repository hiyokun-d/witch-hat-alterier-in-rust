//! Tests for `shapes` — the built-in marks.

use super::*;

use crate::Point;
use crate::catalog::{Catalog, SigilId, SignId};
use crate::recognizer::{classify, normalize};
use crate::stroke::MATCH_POINTS;
use crate::templates::{self, Kind};

fn catalog() -> Catalog {
    Catalog::parse(
        include_str!("../../the-magic-assets/sigils.ron"),
        include_str!("../../the-magic-assets/signs.ron"),
        include_str!("../../the-magic-assets/spells.ron"),
    )
    .expect("shipped assets must load")
}

fn points(strokes: &Strokes) -> Vec<Point> {
    strokes
        .iter()
        .enumerate()
        .flat_map(|(stroke_id, stroke)| {
            stroke.iter().map(move |&(x, y)| Point {
                x,
                y,
                stroke_id: stroke_id as u32,
            })
        })
        .collect()
}

#[test]
fn every_built_in_names_something_the_catalogue_defines() {
    // A shape naming an id nothing defines is a rune nobody could ever draw.
    let c = catalog();
    for (id, kind, _) in built_in() {
        let known = match kind {
            Kind::Sigil => c.sigil(&SigilId::from(id)).is_some(),
            Kind::Sign => c.sign(&SignId::from(id)).is_some(),
        };
        assert!(known, "{id:?} is not in the catalogue");
    }
}

#[test]
fn every_built_in_has_enough_ink_to_normalise() {
    for (id, _, strokes) in built_in() {
        assert!(
            normalize(&points(&strokes), MATCH_POINTS).is_some(),
            "{id:?} has no shape to record"
        );
    }
}

#[test]
fn every_built_in_recognises_as_itself() {
    // The whole point: draw a fire sigil and the recognizer says fire.
    let shapes = templates::built_ins(MATCH_POINTS);
    let all = templates::templates(&shapes);

    for (id, _, strokes) in built_in() {
        let cloud = normalize(&points(&strokes), MATCH_POINTS).expect("normalises");
        let found = classify(&cloud, &all).expect("something matched");
        assert_eq!(all[found.index].name, id, "{id:?} matched the wrong shape");
    }
}

#[test]
fn no_two_built_ins_are_the_same_drawing() {
    // Two shapes a hand could not tell apart is a catalogue that cannot be
    // used. M4.8 found a square and a ring only 16% apart and said so; this is
    // the same check, run over the shapes we actually ship.
    let shapes = templates::built_ins(MATCH_POINTS);
    let all = templates::templates(&shapes);

    for (i, one) in all.iter().enumerate() {
        for other in all.iter().skip(i + 1) {
            let apart = crate::recognizer::cloud_distance(&one.cloud, &other.cloud)
                .expect("same sample count");
            assert!(
                apart > 0.15,
                "{:?} and {:?} are only {apart:.3} apart",
                one.name,
                other.name
            );
        }
    }
}

#[test]
fn a_recorded_rune_outranks_the_built_in_of_the_same_name() {
    // §2's rule survives: these are reconstructions, and a traced shape wins.
    let traced = templates::built_ins(MATCH_POINTS)
        .into_iter()
        .take(1)
        .collect::<Vec<_>>();
    let both = templates::with_built_ins(traced, MATCH_POINTS);

    let fires = both
        .iter()
        .filter(|r| r.template.name == "fire" && r.kind == Kind::Sigil)
        .count();
    assert_eq!(fires, 1, "the built-in competed with the recorded rune");
}

// ---- several samples of one rune ------------------------------------------

#[test]
fn a_rune_may_have_several_recorded_samples() {
    // Rejected as a duplicate id once, on the reasoning that one of them could
    // never win. That is wrong for a nearest-neighbour recognizer: both
    // compete, the nearer wins, and they carry the same name so the *answer* is
    // identical. Recording your hand three to five times is the cheapest
    // accuracy there is.
    let file = r#"(version: 1, templates: [
        (id: "fire", kind: Sigil, strokes: [[(0.0, 0.0), (10.0, 20.0), (20.0, 0.0), (0.0, 0.0)]]),
        (id: "fire", kind: Sigil, strokes: [[(0.0, 0.0), (11.0, 19.0), (21.0, 1.0), (0.0, 0.0)]]),
        (id: "fire", kind: Sigil, strokes: [[(1.0, 0.0), (9.0, 21.0), (19.0, 0.0), (1.0, 0.0)]]),
    ])"#;
    let loaded = templates::parse("test.ron", file, MATCH_POINTS).expect("samples must load");
    assert_eq!(loaded.len(), 3);
    assert!(loaded.iter().all(|r| r.template.name == "fire"));
}

#[test]
fn several_samples_of_a_rune_do_not_make_it_unreadable() {
    // The trap this creates: five tracings of fire sit very close together, so
    // a margin measured against the *second nearest overall* would have every
    // one of them fail and leave the mark unread. The margin is measured
    // against the nearest differently-named rune instead — the question is
    // "fire rather than water", never "this tracing rather than that one".
    let mut shapes = templates::built_ins(MATCH_POINTS);
    let fire = shapes
        .iter()
        .find(|r| r.template.name == "fire")
        .expect("a fire built-in")
        .clone();
    // Four more, near-identical.
    for _ in 0..4 {
        shapes.push(fire.clone());
    }

    let mut ink = points(&fire_shape());
    for p in &mut ink {
        p.x += 0.01;
    }
    let ring = crate::assembly::find_rings(&[], &crate::assembly::RingSearch::default());
    assert!(ring.is_empty());

    let cloud = normalize(&ink, MATCH_POINTS).expect("normalises");
    let ranked = crate::recognizer::rank(&cloud, &templates::templates(&shapes));
    let best = ranked.first().expect("something matched");
    assert_eq!(shapes[best.index].template.name, "fire");
}

/// The fire shape, for the test above.
fn fire_shape() -> Strokes {
    fire()
}
