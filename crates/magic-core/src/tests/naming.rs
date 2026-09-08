//! Tests for `naming` — ink becoming a sigil and signs.

use super::*;

/// The rules these fixtures judge a ring by.
///
/// The same numbers the shell ships, so a test agrees with the app about what
/// counts as a ring — which is the whole point of the two being separate.
const RULES_FOR_TESTS: crate::assembly::RingRules = crate::assembly::RingRules {
    simple_tolerance: 0.08,
    min_quality: 0.95,
};

use crate::assembly::{RingSearch, find_rings};
use crate::glyph::GlyphId;
use crate::shapes;
use crate::templates;

const ON_RING: f32 = 12.0;

/// The shipped catalogue, for the tests that compile a seal.
fn catalog() -> crate::Catalog {
    crate::Catalog::parse(
        include_str!("../../the-magic-assets/sigils.ron"),
        include_str!("../../the-magic-assets/signs.ron"),
        include_str!("../../the-magic-assets/spells.ron"),
    )
    .expect("shipped assets must load")
}

fn shapes_book() -> Vec<Recorded> {
    templates::built_ins(stroke::MATCH_POINTS)
}

/// A ring of `radius` about the origin, as ink.
fn ring_ink(radius: f32, id: u32) -> Vec<Point> {
    (0..220)
        .map(|i| {
            let a = i as f32 / 219.0 * std::f32::consts::TAU;
            Point {
                x: radius * a.cos(),
                y: radius * a.sin(),
                stroke_id: id,
            }
        })
        .collect()
}

/// One of the built-in shapes, placed, scaled and turned.
///
/// Turned about its own centre, which is what a keystone drawn at the side of a
/// ring actually is — canon's `placement` is where it sits, and its rotation is
/// its own.
fn rotated(which: &str, at: (f32, f32), scale: f32, by: f32, first_id: u32) -> Vec<Point> {
    let (sin, cos) = by.sin_cos();
    mark(which, (0.0, 0.0), scale, first_id)
        .into_iter()
        .map(|p| Point {
            x: at.0 + p.x * cos - p.y * sin,
            y: at.1 + p.x * sin + p.y * cos,
            stroke_id: p.stroke_id,
        })
        .collect()
}

/// A keystone turned to point at the ring's centre from where it sits.
fn turned_mark(which: &str, at: (f32, f32), scale: f32, first_id: u32) -> Vec<Point> {
    rotated(which, at, scale, at.1.atan2(at.0), first_id)
}

/// One of the built-in shapes, placed and scaled.
fn mark(which: &str, at: (f32, f32), scale: f32, first_id: u32) -> Vec<Point> {
    let (_, _, strokes) = shapes::built_in()
        .into_iter()
        .find(|(id, _, _)| *id == which)
        .expect("a built-in by that name");
    strokes
        .iter()
        .enumerate()
        .flat_map(|(n, stroke)| {
            stroke.iter().map(move |&(x, y)| Point {
                x: at.0 + x * scale,
                y: at.1 + y * scale,
                stroke_id: first_id + n as u32,
            })
        })
        .collect()
}

/// Builds a pad, finds its ring, and names what the ring holds.
fn seal(points: Vec<Point>) -> Glyph {
    let rings = find_rings(&points, &RingSearch::default());
    // The biggest ring, not the only one: some marks are closed loops and the
    // search is entitled to consider them. Which ring is the *seal* is a
    // question of size, and this fixture always draws the seal largest.
    let ring = rings
        .iter()
        .max_by(|a, b| a.fit.radius.total_cmp(&b.fit.radius))
        .expect("the fixture must make at least one ring");
    let mut glyph = Glyph::new(GlyphId(0), None, ring.to_ring(&RULES_FOR_TESTS));
    let owned = crate::assembly::owned(std::slice::from_ref(ring), &points, ON_RING);
    name(&mut glyph, ring, &points, &owned[0], &shapes_book());
    glyph
}

#[test]
fn a_fire_sigil_drawn_inside_a_ring_is_named_fire() {
    // The whole point of the module: draw it, and the seal knows what it is.
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("fire", (0.0, 0.0), 40.0, 1));

    let glyph = seal(ink);
    assert_eq!(glyph.sigil.as_ref().map(|id| id.as_str()), Some("fire"));
    assert_eq!(glyph.unnamed, 0);
}

#[test]
fn a_water_sigil_is_not_mistaken_for_fire() {
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("water", (0.0, 0.0), 40.0, 1));
    assert_eq!(
        seal(ink).sigil.as_ref().map(|id| id.as_str()),
        Some("water")
    );
}

#[test]
fn every_built_in_sigil_is_named_when_it_is_drawn() {
    for which in ["fire", "water", "wind", "earth", "light"] {
        let mut ink = ring_ink(140.0, 0);
        ink.extend(mark(which, (0.0, 0.0), 40.0, 1));
        assert_eq!(
            seal(ink).sigil.as_ref().map(|id| id.as_str()),
            Some(which),
            "{which} was not recognised"
        );
    }
}

#[test]
fn signs_around_a_sigil_are_named_and_counted() {
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("fire", (0.0, 0.0), 34.0, 1));
    // Four columns, north east south west.
    for (n, (x, y)) in [(0.0, 88.0), (88.0, 0.0), (0.0, -88.0), (-88.0, 0.0)]
        .into_iter()
        .enumerate()
    {
        ink.extend(mark("column", (x, y), 22.0, 20 + n as u32 * 4));
    }

    let glyph = seal(ink);
    assert_eq!(glyph.sigil.as_ref().map(|id| id.as_str()), Some("fire"));
    assert_eq!(glyph.signs.len(), 4, "expected four keystones");
    assert!(glyph.signs.iter().all(|s| s.kind.as_str() == "column"));
}

#[test]
fn a_signs_placement_is_where_it_sits_around_the_ring() {
    // §3.3: inward and outward are questions about direction relative to
    // position, so the angle a sign sits at has to be measured, not assumed.
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("column", (0.0, 90.0), 22.0, 1));

    let glyph = seal(ink);
    assert_eq!(glyph.signs.len(), 1);
    let up = std::f32::consts::FRAC_PI_2;
    assert!(
        (glyph.signs[0].placement - up).abs() < 0.3,
        "placement {} should be near {up}",
        glyph.signs[0].placement
    );
}

#[test]
fn ink_nobody_recognises_is_counted_rather_than_guessed() {
    // Total, like the compiler: unreadable is a fact to report, not an error.
    let mut ink = ring_ink(140.0, 0);
    for i in 0..14 {
        ink.push(Point {
            x: -30.0 + i as f32 * 4.0,
            y: (i as f32 * 1.7).sin() * 25.0,
            stroke_id: 1,
        });
    }
    let glyph = seal(ink);
    assert!(glyph.sigil.is_none());
    assert_eq!(glyph.unnamed, 1);
}

#[test]
fn an_empty_ring_names_nothing_and_reports_nothing_unread() {
    let glyph = seal(ring_ink(140.0, 0));
    assert!(glyph.sigil.is_none());
    assert!(glyph.signs.is_empty());
    assert_eq!(glyph.unnamed, 0);
}

#[test]
fn the_parts_of_one_mark_are_not_read_as_several_marks() {
    // Fire is four strokes. Reading it as four marks would be four unreadable
    // scribbles instead of one sigil.
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("fire", (0.0, 0.0), 40.0, 1));
    let glyph = seal(ink);
    assert_eq!(glyph.signs.len(), 0);
    assert_eq!(glyph.unnamed, 0);
    assert!(glyph.sigil.is_some());
}

#[test]
fn naming_is_deterministic() {
    let build = || {
        let mut ink = ring_ink(140.0, 0);
        ink.extend(mark("water", (0.0, 0.0), 36.0, 1));
        ink.extend(mark("levitation", (0.0, 88.0), 22.0, 20));
        ink.extend(mark("levitation", (88.0, 0.0), 22.0, 30));
        seal(ink)
    };
    assert_eq!(build(), build());
}

// ---- the bug: a sigil drawn on its own was firing ------------------------

#[test]
fn a_sigil_drawn_alone_is_not_a_ring_and_does_not_fire() {
    // Reported from a screenshot: a fire sigil on an empty pad showed
    // "DISCHARGE - a blast, summoning 0.2s left". Its triangle is a closed
    // loop that fits a circle well enough to be found, and nothing downstream
    // ever asked whether the ink was *round*. Canon has no spell without a
    // ring, so there was no spell there at all.
    let ink = mark("fire", (0.0, 0.0), 60.0, 0);
    let rings = find_rings(&ink, &RingSearch::default());

    for ring in &rings {
        let mut glyph = Glyph::new(GlyphId(0), None, ring.to_ring(&RULES_FOR_TESTS));
        let owned = crate::assembly::owned(std::slice::from_ref(ring), &ink, ON_RING);
        name(&mut glyph, ring, &ink, &owned[0], &shapes_book());
        let spell = crate::compile(&glyph, &catalog(), &crate::CompileRules::default());
        assert!(!spell.fires(), "a bare {:?} fired as a seal", ring.strokes);
    }
}

#[test]
fn every_built_in_drawn_alone_stays_inert() {
    // The same trap for each of them: earth's chevron, light's square and
    // diamond, convergence's triangle are all closed loops.
    for (which, _, _) in shapes::built_in() {
        let ink = mark(which, (0.0, 0.0), 60.0, 0);
        for ring in find_rings(&ink, &RingSearch::default()) {
            let mut glyph = Glyph::new(GlyphId(0), None, ring.to_ring(&RULES_FOR_TESTS));
            let owned = crate::assembly::owned(std::slice::from_ref(&ring), &ink, ON_RING);
            name(&mut glyph, &ring, &ink, &owned[0], &shapes_book());
            let spell = crate::compile(&glyph, &catalog(), &crate::CompileRules::default());
            assert!(!spell.fires(), "{which} fired with no ring around it");
        }
    }
}

#[test]
fn a_real_ring_still_fires() {
    // The control. A fix that made everything inert would pass the two tests
    // above and be worthless.
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("fire", (0.0, 0.0), 40.0, 1));

    let rings = find_rings(&ink, &RingSearch::default());
    let ring = rings
        .iter()
        .max_by(|a, b| a.fit.radius.total_cmp(&b.fit.radius))
        .expect("a ring");
    let mut glyph = Glyph::new(GlyphId(0), None, ring.to_ring(&RULES_FOR_TESTS));
    let owned = crate::assembly::owned(std::slice::from_ref(ring), &ink, ON_RING);
    name(&mut glyph, ring, &ink, &owned[0], &shapes_book());

    let spell = crate::compile(&glyph, &catalog(), &crate::CompileRules::default());
    assert!(spell.fires(), "a proper seal stopped firing");
    assert_eq!(glyph.sigil.as_ref().map(|id| id.as_str()), Some("fire"));
}

#[test]
fn a_drawn_sigil_is_what_the_seal_is() {
    // Reported from a screenshot: a water sigil, drawn by hand and correctly
    // recognised, captioned `fire`. The panel's naming override was stomping
    // the recognizer rather than filling in for it — so the seal on the paper
    // and the label over it disagreed, which is the one thing a readout must
    // never do.
    //
    // The override lives in the shell, so what core can pin is the half it
    // owns: given a drawing it can read, `name` produces that reading and no
    // other, and it says so by leaving nothing unread for a caller to guess at.
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("water", (0.0, 0.0), 40.0, 1));

    let glyph = seal(ink);
    assert_eq!(glyph.sigil.as_ref().map(|id| id.as_str()), Some("water"));
    assert_eq!(
        glyph.unnamed, 0,
        "a seal that read itself must leave nothing for an override to fill"
    );
}

#[test]
fn a_multi_stroke_sigil_is_one_mark_not_several() {
    // The bug this file exists to keep out. Every built-in sigil is several
    // strokes, and the first two segmentations split them: a seal covered in
    // ink reported "3 marks nothing can name" and fell back to canon rule 9's
    // discharge, while the recogniser board — which scores the ring's whole
    // contents as one cloud — read the same drawing correctly on the same
    // frame.
    for which in ["fire", "water", "wind", "earth", "light"] {
        let mut ink = ring_ink(140.0, 0);
        ink.extend(mark(which, (0.0, 0.0), 40.0, 1));
        let glyph = seal(ink);
        assert_eq!(
            glyph.sigil.as_ref().map(|id| id.as_str()),
            Some(which),
            "{which} was not read as one mark"
        );
        assert_eq!(glyph.unnamed, 0, "{which} left pieces nobody could name");
    }
}

#[test]
fn a_sigil_and_a_keystone_beside_it_stay_two_marks() {
    // The other half, and the reason segmentation cannot be a distance. The
    // widest gap *inside* one traced rune is 35px; the gap between this sigil
    // and this sign is 32px. Both answers have to come out of the same rule.
    let mut ink = ring_ink(140.0, 0);
    ink.extend(mark("water", (0.0, 0.0), 34.0, 1));
    ink.extend(mark("column", (0.0, 88.0), 22.0, 20));

    let glyph = seal(ink);
    assert_eq!(glyph.sigil.as_ref().map(|id| id.as_str()), Some("water"));
    assert_eq!(
        glyph.signs.len(),
        1,
        "the keystone was swallowed by the sigil"
    );
    assert_eq!(glyph.signs[0].kind.as_str(), "column");
}

#[test]
fn keystones_are_read_at_whatever_angle_they_are_drawn() {
    // **The bug that made every real seal unreadable.** Four keystones around a
    // ring point in four different directions — §2.4's whole balance mechanic
    // is made of that — so at most one of them can ever sit at the template's
    // own recorded angle. `$P` keeps rotation deliberately (canon rule 6), and
    // for a sigil that is right; for a sign it meant a stamped water orb
    // segmented into "column, wind" instead of a sigil and four arrows, and the
    // seal was refused as unreadable.
    let mut ink = ring_ink(160.0, 0);
    ink.extend(mark("water", (0.0, 0.0), 34.0, 1));
    for (n, (x, y)) in [(0.0, 96.0), (96.0, 0.0), (0.0, -96.0), (-96.0, 0.0)]
        .into_iter()
        .enumerate()
    {
        ink.extend(turned_mark("column", (x, y), 22.0, 20 + n as u32 * 4));
    }

    let glyph = seal(ink);
    assert_eq!(glyph.sigil.as_ref().map(|id| id.as_str()), Some("water"));
    assert_eq!(glyph.signs.len(), 4, "keystones read: {:?}", glyph.signs);
    assert!(glyph.signs.iter().all(|s| s.kind.as_str() == "column"));
    assert_eq!(glyph.unnamed, 0);
}

#[test]
fn a_sigil_is_still_matched_upright() {
    // The other half, and it must not regress: §2.2 names a sigil by its shape
    // and canon rule 6 makes a reversed mark mean the opposite, so a sigil that
    // could rotate freely would erase a distinction the engine is built on.
    // A fire sigil turned on its head is not a fire sigil.
    let mut ink = ring_ink(140.0, 0);
    ink.extend(rotated("fire", (0.0, 0.0), 40.0, std::f32::consts::PI, 1));
    assert_ne!(
        seal(ink).sigil.as_ref().map(|id| id.as_str()),
        Some("fire"),
        "an upside-down fire sigil must not read as fire"
    );
}
