//! Spell vocabulary: the sigils, signs, and rings a glyph is built from.
//!
//! Canon names only (see CLAUDE.md §2) — `Sigil`, `Sign`, `Ring`, `Glyph`.
//!
//! What a sigil or sign *is* lives here. What it *does* does not: behaviour is
//! data, and every question of the form "can this create air?" or "is this
//! decorative?" is answered by [`crate::catalog::Catalog`] reading the `.ron`
//! files. A glyph names its parts by id and knows nothing about them.

use crate::Point;
use crate::arrangement::Symmetry;
use crate::catalog::{SigilId, SignId};

/// One keystone placed around a sigil: *how* the spell behaves.
///
/// `orientation` and `reversed` are not decoration — rule 6 (a reversed sign
/// inverts its effect) and rule 7 (symmetry) are unimplementable without them.
#[derive(Debug, Clone, PartialEq)]
pub struct Sign {
    /// Which keystone this is. An id into the catalog, not an enum — the set of
    /// signs is data, and a new one must not need a recompile (§4.4).
    pub kind: SignId,
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
    /// A mirrored sign inverts its effect: enlarge becomes shrink.
    pub reversed: bool,
}

impl Sign {
    /// The same sign with its effect inverted. Kind, placement, and orientation
    /// are untouched — a reversed enlarge is still an enlarge.
    pub fn reverse(&self) -> Sign {
        Sign {
            reversed: !self.reversed,
            ..self.clone()
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
    /// The sigil at the centre. `None` is legal: billow, repetition, and vision
    /// can drive a spell with no element at all (CLAUDE.md §2.1).
    pub sigil: Option<SigilId>,
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
    pub fn new(id: GlyphId, sigil: Option<SigilId>, ring: Ring) -> Glyph {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::f32::consts::{FRAC_PI_2, PI};

    fn sign(kind: &str) -> Sign {
        Sign {
            kind: kind.into(),
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
        let s = sign("enlarge");
        assert_eq!(s.reverse().reverse(), s);
    }

    #[test]
    fn reversed_sign_differs_from_normal() {
        let s = sign("enlarge");
        assert_ne!(s.reverse(), s);
    }

    #[test]
    fn reverse_preserves_kind_placement_and_orientation() {
        let s = Sign {
            placement: 0.5,
            orientation: 1.25,
            ..sign("region")
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
            ..sign("column")
        };
        assert!(!s.reverse().reversed);
    }

    #[test]
    fn sign_pointing_away_from_center_is_outward() {
        let s = Sign {
            placement: FRAC_PI_2,
            orientation: FRAC_PI_2,
            ..sign("region")
        };
        assert!(s.points_outward(0.1));
        assert!(!s.points_inward(0.1));
    }

    #[test]
    fn sign_pointing_back_at_center_is_inward() {
        let s = Sign {
            placement: FRAC_PI_2,
            orientation: FRAC_PI_2 + PI,
            ..sign("region")
        };
        assert!(s.points_inward(0.1));
        assert!(!s.points_outward(0.1));
    }

    #[test]
    fn sign_tangent_to_ring_is_neither_in_nor_out() {
        let s = Sign {
            placement: 0.0,
            orientation: FRAC_PI_2,
            ..sign("region")
        };
        assert!(!s.points_inward(0.1));
        assert!(!s.points_outward(0.1));
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
        let g = Glyph::new(GlyphId(0), Some("fire".into()), ring(1.0));
        assert!(g.signs.is_empty());
    }

    #[test]
    fn new_glyph_starts_unnested_and_unlinked() {
        let g = Glyph::new(GlyphId(0), Some("fire".into()), ring(1.0));
        assert_eq!(g.parent, None);
        assert!(g.linked.is_empty());
    }

    /// CLAUDE.md §2.1: billow, repetition, and vision drive a spell from the
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
