//! Tests for `particles` — the look table, and the one rule it must satisfy.

use super::*;
use magic_core::Vec2 as CoreVec2;
use magic_core::sim::{Event, SubstanceId};

fn every_event() -> Vec<Event> {
    vec![
        Event::Blast {
            at: CoreVec2::ZERO,
            strength: 1.0,
        },
        Event::Summon {
            at: CoreVec2::ZERO,
            substance: SubstanceId::new("water"),
            mass: 2.0,
            created: true,
        },
        Event::Summon {
            at: CoreVec2::ZERO,
            substance: SubstanceId::new("air"),
            mass: 2.0,
            created: false,
        },
        Event::Refused {
            at: CoreVec2::ZERO,
            substance: SubstanceId::new("air"),
        },
        Event::Ignite { at: CoreVec2::ZERO },
        Event::Douse { at: CoreVec2::ZERO },
        Event::Spent {
            at: CoreVec2::ZERO,
            substance: SubstanceId::new("wood"),
        },
        Event::Flash {
            at: CoreVec2::ZERO,
            strength: 1.0,
        },
        Event::Reacted {
            at: CoreVec2::ZERO,
            product: SubstanceId::new("steam"),
            mass: 1.0,
        },
    ]
}

#[test]
fn every_event_produces_a_visible_burst() {
    // The point of a complete look table: a variant nothing emits *yet* still
    // has to be ready, because a hole in it fails silently — the effect simply
    // does not appear and nobody can tell a missing rule from a missing colour.
    for event in every_event() {
        let burst = burst_for(&event);
        assert!(burst.count > 0, "{:?} draws nothing", event.label());
        assert!(burst.life > 0.0, "{:?} lives no time", event.label());
        assert!(burst.size > 0.0, "{:?} has no size", event.label());
        assert!(
            burst.color.alpha() > 0.0,
            "{:?} is invisible",
            event.label()
        );
    }
}

#[test]
fn gathering_draws_inward_and_creating_draws_outward() {
    // §3.2 made visible: you can see whether a spell made its substance or took
    // it from the room, without reading a number.
    let made = burst_for(&Event::Summon {
        at: CoreVec2::ZERO,
        substance: SubstanceId::new("water"),
        mass: 2.0,
        created: true,
    });
    let found = burst_for(&Event::Summon {
        at: CoreVec2::ZERO,
        substance: SubstanceId::new("air"),
        mass: 2.0,
        created: false,
    });
    assert!(made.speed > 0.0);
    assert!(found.speed < 0.0);
}

#[test]
fn every_shipped_substance_has_a_colour_of_its_own() {
    let unknown = substance_color("gravitonium");
    for name in [
        "air",
        "flame",
        "heat",
        "steam",
        "smoke",
        "sound",
        "water",
        "ice",
        "crystal",
        "stone",
        "sand",
        "soil",
        "wood",
        "cloth",
        "light",
        "electricity",
    ] {
        assert_ne!(
            substance_color(name),
            unknown,
            "{name} is in materials.ron but falls through to the grey"
        );
    }
}

#[test]
fn motes_never_grow_past_the_cap() {
    let mut motes = Motes::default();
    for _ in 0..400 {
        motes.scatter(
            Vec2::ZERO,
            burst_for(&Event::Blast {
                at: CoreVec2::ZERO,
                strength: 4.0,
            }),
        );
    }
    assert!(
        motes.live.len() <= MAX_MOTES,
        "{} motes alive",
        motes.live.len()
    );
}
