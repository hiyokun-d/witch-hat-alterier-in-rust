//! Spell vocabulary: the sigils, signs, and rings a glyph is built from.
//!
//! Canon names only (see CLAUDE.md §2) — `Sigil`, `Sign`, `Ring`, `Glyph`.
//! Where the wiki records a name but no behaviour, this module records the name
//! and no behaviour. An honest hole beats a plausible guess.

use crate::Point;
use crate::arrangement::Symmetry;

/// Which family a sigil belongs to.
///
/// Grouping only. Every behavioural question is answered by [`Sigil`], never by
/// the family — wind and aeriforms are both `Air` and can do opposite things.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SigilFamily {
    /// Flame, heat, and light.
    Fire,
    Water,
    Earth,
    Air,
    Time,
    /// Sigils that belong to no family the wiki names.
    Misc,
}

/// What a sigil is *able to do* to its element.
///
/// Read straight off the wiki's own verbs rather than invented: wind moves air
/// but cannot create it, aeriforms creates air but cannot move it, earth
/// manipulates stone but never makes it. Encoding this gives the simulation
/// conservation rules — a wind spell in a sealed room has to find its air
/// somewhere — instead of letting every spell spawn matter from nothing.
///
/// A capability the wiki does not claim is `false`. That is deliberate: absence
/// of evidence is modelled as absence of the power, so a spell that seems to
/// need it shows up as a design question rather than passing silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Capabilities {
    /// Brings the element into being where there was none.
    pub can_create: bool,
    /// Changes the element in place — shape, state, intensity.
    pub can_manipulate: bool,
    /// Transports the element from one place to another.
    pub can_move: bool,
    /// Gathers the element from its surroundings rather than making it.
    pub can_collect: bool,
}

impl Capabilities {
    /// All four denied. The base every sigil's entry is built from, so adding a
    /// capability is a visible act rather than a default.
    const NONE: Capabilities = Capabilities {
        can_create: false,
        can_manipulate: false,
        can_move: false,
        can_collect: false,
    };
}

/// A sigil variant: the thing at the centre of a glyph that decides *what* the
/// spell is made of.
///
/// Canonically a sigil's size and position inside the seal do not change its
/// behaviour, so neither is stored here, and the recognizer must score sigils on
/// shape alone (CLAUDE.md §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Sigil {
    /// Creates and manipulates flame or heat.
    Fire,
    /// Heatless flame — the phantasmal fireball. The wiki gives no mechanism,
    /// and notes it may need supporting signs to work at all.
    UnburningFlames,
    /// Manifests magic as light. A fire variant, split out only because there
    /// are enough light spells to warrant it.
    Light,
    /// Manipulates, collects, and creates water. Long-duration water spells
    /// collect rather than create, implying creation costs more energy.
    Water,
    /// Manipulates wood, stone, sand, and soil. Cannot create them.
    Earth,
    /// Moves and manipulates air. Cannot create it.
    Wind,
    /// Creates and manipulates air. Cannot move it.
    Aeriforms,
    /// Supports solid objects suspended in air — an air platform. Mechanism
    /// unclear.
    WindUnderfoot,
    /// Manipulates air through rotation. Looks visually unlike the other air
    /// sigils, for reasons the wiki does not give.
    WhorlingWinds,
    /// Continuously resets affected objects to the state they held when the
    /// spell took hold. Spring-like: makes soft things elastic, stops rot.
    Repetition,
    /// Halts time outright for affected objects. Paired with another sigil it
    /// stops one aspect only — with fire, heat stops changing.
    Stop,
    /// Creates and manipulates crystalline objects. Only Richeh uses it.
    Crystal,
    /// Attracts objects matching parameters set by the rest of the spell.
    Guidance,
}

impl Sigil {
    /// Every sigil, in declaration order. Lets tests and tooling enumerate the
    /// set without a derive macro, which core has no dependencies for.
    pub const ALL: [Sigil; 13] = [
        Sigil::Fire,
        Sigil::UnburningFlames,
        Sigil::Light,
        Sigil::Water,
        Sigil::Earth,
        Sigil::Wind,
        Sigil::Aeriforms,
        Sigil::WindUnderfoot,
        Sigil::WhorlingWinds,
        Sigil::Repetition,
        Sigil::Stop,
        Sigil::Crystal,
        Sigil::Guidance,
    ];

    /// Which family this sigil sits in.
    pub fn family(&self) -> SigilFamily {
        match self {
            Sigil::Fire | Sigil::UnburningFlames | Sigil::Light => SigilFamily::Fire,
            Sigil::Water => SigilFamily::Water,
            Sigil::Earth => SigilFamily::Earth,
            Sigil::Wind | Sigil::Aeriforms | Sigil::WindUnderfoot | Sigil::WhorlingWinds => {
                SigilFamily::Air
            }
            Sigil::Repetition | Sigil::Stop => SigilFamily::Time,
            Sigil::Crystal | Sigil::Guidance => SigilFamily::Misc,
        }
    }

    /// What this sigil can do to its element.
    ///
    /// Each arm mirrors one wiki sentence. Where the wiki calls a mechanism
    /// unknown, the entry claims as little as the text supports.
    pub fn capabilities(&self) -> Capabilities {
        match self {
            Sigil::Fire | Sigil::Light | Sigil::Crystal => Capabilities {
                can_create: true,
                can_manipulate: true,
                ..Capabilities::NONE
            },
            // "Involved in" heatless flame, mechanism unknown. Something is
            // produced, so creation is claimed and nothing else is.
            Sigil::UnburningFlames => Capabilities {
                can_create: true,
                ..Capabilities::NONE
            },
            Sigil::Water => Capabilities {
                can_create: true,
                can_manipulate: true,
                can_collect: true,
                ..Capabilities::NONE
            },
            Sigil::Earth => Capabilities {
                can_manipulate: true,
                ..Capabilities::NONE
            },
            Sigil::Wind => Capabilities {
                can_manipulate: true,
                can_move: true,
                ..Capabilities::NONE
            },
            Sigil::Aeriforms => Capabilities {
                can_create: true,
                can_manipulate: true,
                ..Capabilities::NONE
            },
            // Holding an object up is doing *something* to the air beneath it,
            // but the wiki calls the mechanism unclear, so only the in-place
            // change is claimed.
            //
            // Whorling winds is the same shape of problem: rotation is how it
            // manipulates, and whether spinning air also counts as moving it is
            // left open rather than guessed.
            Sigil::WindUnderfoot | Sigil::WhorlingWinds | Sigil::Repetition | Sigil::Stop => {
                Capabilities {
                    can_manipulate: true,
                    ..Capabilities::NONE
                }
            }
            Sigil::Guidance => Capabilities {
                can_move: true,
                ..Capabilities::NONE
            },
        }
    }
}

/// How well established a sign's behaviour is.
///
/// Kept because it answers bug reports: a wrong effect on an
/// [`CanonTier::Unofficial`] sign is a reconstruction that missed, not a defect
/// in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CanonTier {
    /// Named in the source. Behaviour is established, or explicitly unknown.
    Official,
    /// Fan-reconstructed by comparing spells. Plausible, not established.
    Unofficial,
    /// Named, drawn, and mechanically inert.
    Decorative,
}

/// What a sign *does* to the sigil at the centre of the glyph.
///
/// The full canon list from CLAUDE.md §2.3, all three provenance tiers. Some
/// variants are documented names with no recorded effect; they exist so a
/// drawing can be recognised and reported rather than silently rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SignKind {
    // ── Official ─────────────────────────────────────────────────────────
    /// Manifests as a column or beam above the glyph. Unbalanced signs bias it
    /// toward whichever side has more; radial symmetry is not unbalanced.
    Column,
    /// A column that leaks in all directions instead of beaming.
    Dispersion,
    /// Magic floats above the glyph, or moves the object it is drawn on.
    Levitation,
    /// Pulls matter of the same kind toward the glyph when arrows point inward.
    /// Angled signs make the pull twist; inverted likely pushes.
    Pull,
    /// Disintegrates objects, or reassembles them when reversed. Only ever seen
    /// with earth.
    Crush,
    /// The object it is drawn on floats regardless of gravity. Partly retconned
    /// into levitation; surviving uses are float-only.
    Float,
    /// Decides *where* magic manifests relative to the glyph. Meaningless alone
    /// — the effect comes from where every region sign points collectively, see
    /// [`crate::arrangement::RegionArrangement`].
    Region,
    /// Focuses magic to a single point; packs loose particles rigid and compact.
    Convergence,
    /// Collects material from above and around the glyph for the spell to use.
    Collection,
    /// Converts material into cloud. Needs [`SignKind::Collection`] to gather
    /// the material first, and some materials cannot convert at all.
    Billow,
    /// Resets affected objects to a previous state.
    Repetition,
    /// Turns solid objects into long flexible ribbons on contact. Invented by
    /// Richeh. Must surround the central sigil.
    Weave,

    // Official names the wiki records with no description. Do not invent one.
    Cool,
    Strengthen,
    SightsSet,
    Entwine,
    SignOfWind,
    AeriformsDefined,
    Glaives,

    // ── Unofficial ───────────────────────────────────────────────────────
    /// Restricts the spell to the object it is drawn on.
    Window,
    /// Restricts the spell to nearby objects, excluding the one it is drawn on.
    Diamond,
    /// Grows objects (corners out) or shrinks them (corners in). Window versus
    /// Diamond decides self versus nearby. Goes in the centre.
    Enlarge,
    /// From rainflinger. Three candidate functions and no way yet to tell them
    /// apart: erase matching magic, restrict manifestation to inside matching
    /// objects, or define an area of effect.
    Crosshair,
    /// From snugstone. Likely weakens the spell — fire becomes gentle heat.
    Radial,
    /// Manifests magic as bolt-like projectiles. With region to aim it, fires at
    /// dangerous speed.
    Bolt,
    /// Appears alongside vision. Never seen alone, so its solo effect is
    /// unknown.
    Eye,
    /// Relates to sight and perception. With [`SignKind::Eye`] it creates
    /// illusions; without, it aids sight.
    Vision,
    /// Bends or alters reality — vision, physical objects, or reality itself.
    Bend,
    /// Produces the sigil's magic as rainfall over the immediate area.
    /// Surrounds the central sigil.
    Rain,
    /// Lets a user steer the object the spell is drawn on, apparently by mind.
    /// Movement type follows the sigil — wind puppets only move through air.
    Puppet,

    // Unofficial names with no recorded behaviour.
    Bind,
    Orb,
    Link,

    // ── Decorative ───────────────────────────────────────────────────────
    /// Projects a bird of the glyph's magic that flies around. Sigil goes in its
    /// centre. No mechanical effect.
    Bird,
    /// Zozah Peninsula animal shapes. Hobby and decoration; use is declining.
    AnimalSign,
}

impl SignKind {
    /// How well established this sign's behaviour is.
    pub fn canon(&self) -> CanonTier {
        match self {
            SignKind::Column
            | SignKind::Dispersion
            | SignKind::Levitation
            | SignKind::Pull
            | SignKind::Crush
            | SignKind::Float
            | SignKind::Region
            | SignKind::Convergence
            | SignKind::Collection
            | SignKind::Billow
            | SignKind::Repetition
            | SignKind::Weave
            | SignKind::Cool
            | SignKind::Strengthen
            | SignKind::SightsSet
            | SignKind::Entwine
            | SignKind::SignOfWind
            | SignKind::AeriformsDefined
            | SignKind::Glaives => CanonTier::Official,

            SignKind::Window
            | SignKind::Diamond
            | SignKind::Enlarge
            | SignKind::Crosshair
            | SignKind::Radial
            | SignKind::Bolt
            | SignKind::Eye
            | SignKind::Vision
            | SignKind::Bend
            | SignKind::Rain
            | SignKind::Puppet
            | SignKind::Bind
            | SignKind::Orb
            | SignKind::Link => CanonTier::Unofficial,

            SignKind::Bird | SignKind::AnimalSign => CanonTier::Decorative,
        }
    }

    /// Whether this sign can occupy the centre and drive a spell with no sigil
    /// at all (CLAUDE.md §2.1).
    ///
    /// Exactly three can. Everything else needs something at the centre.
    pub fn can_be_sigil(&self) -> bool {
        matches!(
            self,
            SignKind::Repetition | SignKind::Billow | SignKind::Vision
        )
    }

    /// Whether this sign contributes any behaviour to a compiled spell.
    ///
    /// Decorative signs are drawn, recognised, and then ignored — modelling them
    /// keeps a bird from being reported as an unrecognised stroke.
    pub fn is_decorative(&self) -> bool {
        self.canon() == CanonTier::Decorative
    }
}

/// One keystone placed around a sigil: *how* the spell behaves.
///
/// `orientation` and `reversed` are not decoration — rule 6 (a reversed sign
/// inverts its effect) and rule 7 (symmetry) are unimplementable without them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sign {
    /// Which keystone this is.
    pub kind: SignKind,
    /// Where the sign sits around the ring: the angle from the glyph's centre to
    /// the sign, in radians, counter-clockwise from the +x axis.
    ///
    /// Stored because "pointing inward" and "pointing outward" are questions
    /// about a sign's direction *relative to where it sits*, and rule 1's
    /// containment test needs a position too. A sign at the top of the ring
    /// pointing down and one at the bottom pointing down are different spells.
    pub placement: f32,
    /// Which way the drawn sign faces, in radians, in the same frame as
    /// `placement` — not relative to it. Compare the two with
    /// [`Sign::radial_alignment`].
    pub orientation: f32,
    /// A mirrored sign inverts its effect: Enlarge becomes Shrink.
    pub reversed: bool,
}

impl Sign {
    /// The same sign with its effect inverted. Kind, placement, and orientation
    /// are untouched — a reversed Enlarge is still an Enlarge.
    pub fn reverse(&self) -> Sign {
        Sign {
            reversed: !self.reversed,
            ..*self
        }
    }

    /// How much this sign points away from the glyph's centre: `1.0` straight
    /// out, `-1.0` straight in, `0.0` tangent to the ring.
    ///
    /// A cosine rather than a raw angle difference so callers compare against a
    /// deadband without worrying which side of ±π they landed on.
    pub fn radial_alignment(&self) -> f32 {
        (self.orientation - self.placement).cos()
    }

    /// Whether the sign points away from the centre, beyond `deadband`.
    ///
    /// A sign lying tangent to the ring is neither in nor out, and forcing it
    /// into one bucket would flip a spell's meaning on a degree of drawing
    /// slop — so both this and [`Sign::points_inward`] can answer `false`.
    pub fn points_outward(&self, deadband: f32) -> bool {
        self.radial_alignment() > deadband
    }

    /// Whether the sign points toward the centre, beyond `deadband`.
    pub fn points_inward(&self, deadband: f32) -> bool {
        self.radial_alignment() < -deadband
    }
}

/// The circle enclosing a glyph — the thing that actually fires the spell.
///
/// Fields are private because `quality` has to stay inside `0.0..=1.0`, and a
/// public field cannot promise that. Build one with [`Ring::new`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    center: Point,
    radius: f32,
    closed: bool,
    quality: f32,
}

impl Ring {
    /// Builds a ring, forcing `quality` into `0.0..=1.0`.
    ///
    /// Out-of-range quality is a caller mistake rather than something a player
    /// can cause, so it is clamped rather than reported.
    pub fn new(center: Point, radius: f32, closed: bool, quality: f32) -> Ring {
        Ring {
            center,
            radius,
            closed,
            quality: quality.clamp(0.0, 1.0),
        }
    }

    /// Where the ring sits on the canvas.
    pub fn center(&self) -> Point {
        self.center
    }

    /// Distance from the center to the drawn line. Bigger seals are stronger.
    pub fn radius(&self) -> f32 {
        self.radius
    }

    /// Whether the circuit is complete. An open ring is inert but primed —
    /// close the gap later and the spell fires immediately (canon rule 2).
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// How neatly the ring was drawn, `0.0..=1.0`. Neater seals last longer.
    pub fn quality(&self) -> f32 {
        self.quality
    }

    /// Whether a point counts as part of this ring's spell: inside the circle,
    /// or touching it from outside within `tolerance` (canon rule 1).
    ///
    /// The *connecting to* clause is why this is not a point-in-circle test. A
    /// sign drawn against the outside of the ring is part of the spell, and
    /// dropping it would silently change what the seal does.
    pub fn contains(&self, point: Point, tolerance: f32) -> bool {
        self.center.dist(&point) <= self.radius + tolerance.max(0.0)
    }
}

/// Identifies one glyph. A newtype rather than a bare `u32` so the compiler
/// refuses to let a stroke id, a template id, or an array index stand in for
/// one. Costs nothing at runtime — the wrapper is gone after compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlyphId(pub u32);

/// One complete spell drawing: an optional sigil, the signs around it, and the
/// ring that encloses them.
///
/// Constructing a `Glyph` never fails and never validates. Whether it is a
/// *legal* spell is the compiler's question (M5), and keeping the two apart is
/// what makes either one testable.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// This glyph's identity, for parent and link references.
    pub id: GlyphId,
    /// The sigil at the centre. `None` is legal: Repetition, Billow, and Vision
    /// can drive a spell with no element at all (CLAUDE.md §2.1).
    pub sigil: Option<Sigil>,
    /// The keystones arranged around the sigil.
    pub signs: Vec<Sign>,
    /// The enclosing circle.
    pub ring: Ring,
    /// The glyph this one is nested inside, if any (canon rule 4).
    pub parent: Option<GlyphId>,
    /// Glyphs joined to this one by a drawn line (canon rule 5). Identical
    /// linked glyphs amplify each other.
    pub linked: Vec<GlyphId>,
}

impl Glyph {
    /// A bare glyph: sigil and ring, nothing else.
    ///
    /// This is the shape a glyph has the moment its ring is recognized. Signs,
    /// nesting, and links are filled in afterwards as the drawing continues,
    /// so they start empty rather than being demanded up front.
    pub fn new(id: GlyphId, sigil: Option<Sigil>, ring: Ring) -> Glyph {
        Glyph {
            id,
            sigil,
            ring,
            signs: Vec::new(),
            parent: None,
            linked: Vec::new(),
        }
    }

    /// How this glyph's signs are arranged (canon rule 7).
    ///
    /// Derived rather than stored: signs keep arriving while the drawing
    /// continues, and a cached classification would be stale between strokes.
    pub fn symmetry(&self, tolerance: f32) -> Symmetry {
        Symmetry::classify(&self.signs, tolerance)
    }

    /// Whether this glyph is stable. Asymmetric arrangements are perfectly legal
    /// spells — canon says they are only *sometimes* unstable — so this is a
    /// warning for the compiler to carry, never a failure.
    pub fn is_stable(&self, tolerance: f32) -> bool {
        self.symmetry(tolerance) != Symmetry::Asymmetric
    }

    /// The signs that actually contribute behaviour. Decorative signs are drawn
    /// and then ignored.
    pub fn effective_signs(&self) -> impl Iterator<Item = &Sign> {
        self.signs.iter().filter(|sign| !sign.kind.is_decorative())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::{FRAC_PI_2, PI};

    fn sign(kind: SignKind) -> Sign {
        Sign {
            kind,
            placement: 0.0,
            orientation: 0.0,
            reversed: false,
        }
    }

    fn at(x: f32, y: f32) -> Point {
        Point { x, y, stroke_id: 0 }
    }

    fn ring(quality: f32) -> Ring {
        Ring::new(at(0.0, 0.0), 40.0, true, quality)
    }

    #[test]
    fn reversing_twice_returns_original() {
        let s = sign(SignKind::Enlarge);
        assert_eq!(s.reverse().reverse(), s);
    }

    #[test]
    fn reversed_sign_differs_from_normal() {
        let s = sign(SignKind::Enlarge);
        assert_ne!(s.reverse(), s);
    }

    #[test]
    fn reverse_preserves_kind_placement_and_orientation() {
        let s = Sign {
            kind: SignKind::Region,
            placement: 0.5,
            orientation: 1.25,
            reversed: false,
        };
        let r = s.reverse();
        assert_eq!(r.kind, s.kind);
        assert_eq!(r.placement, s.placement);
        assert_eq!(r.orientation, s.orientation);
    }

    #[test]
    fn reverse_of_reversed_sign_is_normal() {
        let s = Sign {
            reversed: true,
            ..sign(SignKind::Column)
        };
        assert!(!s.reverse().reversed);
    }

    #[test]
    fn sign_pointing_away_from_center_is_outward() {
        let s = Sign {
            placement: FRAC_PI_2,
            orientation: FRAC_PI_2,
            ..sign(SignKind::Region)
        };
        assert!(s.points_outward(0.1));
        assert!(!s.points_inward(0.1));
    }

    #[test]
    fn sign_pointing_back_at_center_is_inward() {
        let s = Sign {
            placement: FRAC_PI_2,
            orientation: FRAC_PI_2 + PI,
            ..sign(SignKind::Region)
        };
        assert!(s.points_inward(0.1));
        assert!(!s.points_outward(0.1));
    }

    #[test]
    fn sign_tangent_to_ring_is_neither_in_nor_out() {
        let s = Sign {
            placement: 0.0,
            orientation: FRAC_PI_2,
            ..sign(SignKind::Region)
        };
        assert!(!s.points_inward(0.1));
        assert!(!s.points_outward(0.1));
    }

    #[test]
    fn wind_moves_air_but_cannot_create_it() {
        let c = Sigil::Wind.capabilities();
        assert!(c.can_move);
        assert!(!c.can_create);
    }

    #[test]
    fn aeriforms_creates_air_but_cannot_move_it() {
        let c = Sigil::Aeriforms.capabilities();
        assert!(c.can_create);
        assert!(!c.can_move);
    }

    #[test]
    fn earth_manipulates_but_never_creates() {
        let c = Sigil::Earth.capabilities();
        assert!(c.can_manipulate);
        assert!(!c.can_create);
    }

    #[test]
    fn water_is_the_only_sigil_that_collects() {
        let collectors: Vec<Sigil> = Sigil::ALL
            .into_iter()
            .filter(|s| s.capabilities().can_collect)
            .collect();
        assert_eq!(collectors, vec![Sigil::Water]);
    }

    #[test]
    fn every_air_sigil_is_in_the_air_family() {
        for s in [
            Sigil::Wind,
            Sigil::Aeriforms,
            Sigil::WindUnderfoot,
            Sigil::WhorlingWinds,
        ] {
            assert_eq!(s.family(), SigilFamily::Air);
        }
    }

    #[test]
    fn light_is_a_fire_variant() {
        assert_eq!(Sigil::Light.family(), SigilFamily::Fire);
    }

    #[test]
    fn every_sigil_can_do_something() {
        for s in Sigil::ALL {
            let c = s.capabilities();
            assert!(
                c.can_create || c.can_manipulate || c.can_move || c.can_collect,
                "{s:?} has no capabilities at all"
            );
        }
    }

    #[test]
    fn only_three_signs_can_replace_a_sigil() {
        for kind in [SignKind::Repetition, SignKind::Billow, SignKind::Vision] {
            assert!(kind.can_be_sigil(), "{kind:?} should be able to");
        }
        for kind in [SignKind::Column, SignKind::Region, SignKind::Bird] {
            assert!(!kind.can_be_sigil(), "{kind:?} should not be able to");
        }
    }

    #[test]
    fn signs_are_sorted_into_their_canon_tiers() {
        assert_eq!(SignKind::Region.canon(), CanonTier::Official);
        assert_eq!(SignKind::Enlarge.canon(), CanonTier::Unofficial);
        assert_eq!(SignKind::Bird.canon(), CanonTier::Decorative);
    }

    #[test]
    fn undescribed_official_signs_are_still_official() {
        for kind in [SignKind::Cool, SignKind::Glaives, SignKind::Entwine] {
            assert_eq!(kind.canon(), CanonTier::Official);
        }
    }

    #[test]
    fn decorative_signs_are_dropped_from_effective_signs() {
        let mut g = Glyph::new(GlyphId(0), Some(Sigil::Fire), ring(1.0));
        g.signs = vec![sign(SignKind::Column), sign(SignKind::Bird)];
        let kept: Vec<SignKind> = g.effective_signs().map(|s| s.kind).collect();
        assert_eq!(kept, vec![SignKind::Column]);
    }

    #[test]
    fn ring_quality_clamps_above_one() {
        assert_eq!(ring(47.0).quality(), 1.0);
    }

    #[test]
    fn ring_quality_clamps_below_zero() {
        assert_eq!(ring(-3.0).quality(), 0.0);
    }

    #[test]
    fn ring_quality_in_range_is_unchanged() {
        assert_eq!(ring(0.62).quality(), 0.62);
    }

    #[test]
    fn ring_keeps_the_rest_of_its_fields() {
        let r = ring(0.5);
        assert_eq!(r.radius(), 40.0);
        assert!(r.is_closed());
        assert_eq!(r.center().x, 0.0);
    }

    #[test]
    fn ring_contains_a_point_inside_it() {
        assert!(ring(1.0).contains(at(10.0, 0.0), 0.0));
    }

    /// Canon rule 1: a sign touching the ring counts toward the spell, so
    /// containment cannot be a plain point-in-circle test.
    #[test]
    fn ring_contains_a_point_touching_it_from_outside() {
        assert!(ring(1.0).contains(at(42.0, 0.0), 3.0));
    }

    #[test]
    fn ring_excludes_a_point_beyond_tolerance() {
        assert!(!ring(1.0).contains(at(50.0, 0.0), 3.0));
    }

    #[test]
    fn new_glyph_starts_with_no_signs() {
        let g = Glyph::new(GlyphId(0), Some(Sigil::Fire), ring(1.0));
        assert!(g.signs.is_empty());
    }

    #[test]
    fn new_glyph_starts_unnested_and_unlinked() {
        let g = Glyph::new(GlyphId(0), Some(Sigil::Fire), ring(1.0));
        assert_eq!(g.parent, None);
        assert!(g.linked.is_empty());
    }

    /// CLAUDE.md §2.1: Repetition, Billow, and Vision drive a spell from the
    /// centre with no sigil at all.
    #[test]
    fn glyph_with_no_sigil_constructs() {
        let g = Glyph::new(GlyphId(7), None, ring(1.0));
        assert_eq!(g.sigil, None);
    }

    #[test]
    fn glyph_ids_with_the_same_number_are_equal() {
        assert_eq!(GlyphId(3), GlyphId(3));
        assert_ne!(GlyphId(3), GlyphId(4));
    }
}
