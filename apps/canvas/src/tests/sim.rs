//! Tests for `sim` — the shell's half of casting, and what it declines to do.

use super::*;
use magic_core::CompileRules;
use magic_core::glyph::{Glyph, GlyphId, Ring};

fn origin() -> magic_core::Point {
    magic_core::Point {
        x: 0.0,
        y: 0.0,
        stroke_id: 0,
    }
}

fn catalog() -> Catalog {
    Catalog::parse(
        include_str!("../../../../crates/magic-core/the-magic-assets/sigils.ron"),
        include_str!("../../../../crates/magic-core/the-magic-assets/signs.ron"),
        include_str!("../../../../crates/magic-core/the-magic-assets/spells.ron"),
    )
    .expect("shipped assets must load")
}

/// A closed, neat ring with `sigil` in the middle and nothing else.
fn seal(sigil: Option<&str>) -> Spell {
    let ring = Ring::new(origin(), 140.0, true, 0.99);
    let mut glyph = Glyph::new(GlyphId(0), sigil.map(SigilId::from), ring);
    glyph.sigil_extent = 40.0;
    magic_core::compile(&glyph, &catalog(), &CompileRules::default())
}

#[test]
fn a_bare_ring_is_refused_while_only_traced_is_on() {
    // Canon rule 9 is untouched — core still compiles this to a discharge and
    // says so. What changed is that the app declines to fire one, because while
    // the vocabulary is a handful of runes deep, "blast" is what nearly every
    // *misread* drawing does, and a spell going off when the app did not
    // understand you teaches nothing and buries what you were looking at.
    let spell = seal(None);
    assert!(spell.fires(), "core must still say a bare ring fires");
    assert!(refuses(&spell, true).is_some());
}

#[test]
fn turning_only_traced_off_puts_canon_rule_9_back() {
    // The honest way to hold a canon rule and a working tool at once: the rule
    // is still in the engine and still reachable, it is just not the default.
    assert!(refuses(&seal(None), false).is_none());
}

#[test]
fn a_seal_with_a_named_sigil_is_never_refused() {
    for which in ["fire", "water", "wind", "earth", "light"] {
        let spell = seal(Some(which));
        assert!(
            refuses(&spell, true).is_none(),
            "{which} was refused: {:?}",
            refuses(&spell, true)
        );
    }
}

#[test]
fn the_refusal_says_which_case_it_was() {
    // A bare ring and a ring full of ink nobody could read both compile to the
    // same discharge, and they need different advice: one wants a sigil drawn,
    // the other wants the sigil it already has *traced*.
    let bare = refuses(&seal(None), true).expect("refused");
    assert!(bare.contains("bare ring"), "{bare}");

    let ring = Ring::new(origin(), 140.0, true, 0.99);
    let mut glyph = Glyph::new(GlyphId(0), None, ring);
    glyph.unnamed = 3;
    let unread = magic_core::compile(&glyph, &catalog(), &CompileRules::default());
    let said = refuses(&unread, true).expect("refused");
    assert!(said.contains("3 mark(s)"), "{said}");
}

#[test]
fn every_preset_names_a_sigil_the_catalogue_knows() {
    // A preset whose sigil is not in `sigils.ron` would stamp a rune nothing
    // could name — which is the bug the whole `Preset::sigil` field exists to
    // stop, so it is worth a standing check rather than a careful reading.
    let catalog = catalog();
    for preset in PRESETS {
        assert!(
            catalog.sigil(&SigilId::from(preset.sigil)).is_some(),
            "{} names a sigil the catalogue does not have: {}",
            preset.id,
            preset.sigil
        );
    }
}

#[test]
fn every_preset_is_built_on_a_sigil_somebody_could_trace() {
    // The five elemental runes are the ones a person realistically traces, and
    // a preset built on anything else is a button that can only ever be
    // refused. `everlasting` was exactly that: a fine spell on the repetition
    // sigil, which nobody has a rune for.
    for preset in PRESETS {
        assert!(
            ["fire", "water", "wind", "earth", "light"].contains(&preset.sigil),
            "{} is built on {}, which nobody is going to trace",
            preset.id,
            preset.sigil
        );
    }
}
