//! Spell vocabulary: the sigils, signs, and rings a glyph is built from.
//!
//! Canon names only (see CLAUDE.md §2) — `Sigil`, `Sign`, `Ring`, `Glyph`.

use crate::Point;

/// The five canon sigils. A sigil sits at the center of a glyph and decides
/// *what* the spell is made of; the surrounding signs decide *how*.
///
/// Note there is no `Float` here — float is a sign, not an element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Element {
    /// YEAHHHHH ON FIRE!!!
    Fire,
    /// Bends water.
    Water,
    /// Earth, stone, and anything solid enough to stand on.
    Earth,
    /// *Directed* air — distinct from Aeriforms, which only sustains air
    /// that is already there.
    Wind,
    /// Controls how light travels: bending, blocking, revealing.
    Light,
}

/// What a sign *does* to the sigil at the center of the glyph.
///
/// Only the eight the compiler needs first; the canon list (CLAUDE.md §2.3)
/// is much longer, and adding a variant later costs nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SignKind {
    /// Magic rises as a beam above the glyph.
    Column,
    /// A column that leaks — magic pours out on every side.
    Dispersion,
    /// Magic floats above the glyph, or lifts whatever the glyph is drawn on.
    Levitation,
    /// Steers the manifestation. Meaningless without an `orientation`.
    Direction,
    /// Focuses magic to a point; packs loose particles into something solid.
    Convergence,
    /// Spins the manifestation around the center of the glyph.
    Rotate,
    /// Grows the effect — or shrinks it, when reversed.
    Enlarge,
    /// Whatever the glyph is drawn on ignores gravity.
    Float,
}

/// One keystone placed around a sigil: *how* the spell behaves.
///
/// `orientation` and `reversed` are not decoration — canon rule 5 (a reversed
/// sign inverts its effect) and rule 6 (symmetry decides which way the spell
/// biases) are unimplementable without them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sign {
    /// Which keystone this is.
    pub kind: SignKind,
    /// Rotation of the drawn sign, in radians.
    pub orientation: f32,
    /// A mirrored sign inverts its effect: Enlarge becomes Shrink.
    pub reversed: bool,
}

impl Sign {
    /// The same sign with its effect inverted. Kind and orientation are
    /// untouched — a reversed Enlarge is still an Enlarge.
    pub fn reverse(&self) -> Sign {
        Sign {
            reversed: !self.reversed,
            ..*self
        }
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
    /// close the gap later and the spell fires immediately (canon rule 1).
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// How neatly the ring was drawn, `0.0..=1.0`. Neater seals last longer.
    pub fn quality(&self) -> f32 {
        self.quality
    }
}

/// Identifies one glyph. A newtype rather than a bare `u32` so the compiler
/// refuses to let a stroke id, a template id, or an array index stand in for
/// one. Costs nothing at runtime — the wrapper is gone after compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GlyphId(pub u32);

/// One complete spell drawing: a sigil, the signs around it, and the ring
/// that encloses them.
///
/// Constructing a `Glyph` never fails and never validates. Whether it is a
/// *legal* spell is the compiler's question (M5), and keeping the two apart is
/// what makes either one testable.
#[derive(Debug, Clone, PartialEq)]
pub struct Glyph {
    /// This glyph's identity, for parent and link references.
    pub id: GlyphId,
    /// The element at the center. `None` is legal: canon rule 8 lets
    /// Repetition, Billowing, and Vision drive a spell with no element.
    pub sigil: Option<Element>,
    /// The keystones arranged around the sigil.
    pub signs: Vec<Sign>,
    /// The enclosing circle.
    pub ring: Ring,
    /// The glyph this one is nested inside, if any (canon rule 3).
    pub parent: Option<GlyphId>,
    /// Glyphs joined to this one by a drawn line (canon rule 4). Identical
    /// linked glyphs amplify each other.
    pub linked: Vec<GlyphId>,
}

impl Glyph {
    /// A bare glyph: sigil and ring, nothing else.
    ///
    /// This is the shape a glyph has the moment its ring is recognized. Signs,
    /// nesting, and links are filled in afterwards as the drawing continues,
    /// so they start empty rather than being demanded up front.
    pub fn new(id: GlyphId, sigil: Option<Element>, ring: Ring) -> Glyph {
        Glyph {
            id,
            sigil,
            ring,
            signs: Vec::new(),
            parent: None,
            linked: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sign(kind: SignKind) -> Sign {
        Sign {
            kind,
            orientation: 0.0,
            reversed: false,
        }
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
    fn reverse_preserves_kind_and_orientation() {
        let s = Sign {
            kind: SignKind::Direction,
            orientation: 1.25,
            reversed: false,
        };
        let r = s.reverse();
        assert_eq!(r.kind, s.kind);
        assert_eq!(r.orientation, s.orientation);
    }

    #[test]
    fn reverse_of_reversed_sign_is_normal() {
        let s = Sign {
            kind: SignKind::Column,
            orientation: 0.0,
            reversed: true,
        };
        assert!(!s.reverse().reversed);
    }

    fn ring(quality: f32) -> Ring {
        let center = Point {
            x: 0.0,
            y: 0.0,
            stroke_id: 0,
        };
        Ring::new(center, 40.0, true, quality)
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
    fn new_glyph_starts_with_no_signs() {
        let g = Glyph::new(GlyphId(0), Some(Element::Fire), ring(1.0));
        assert!(g.signs.is_empty());
    }

    #[test]
    fn new_glyph_starts_unnested_and_unlinked() {
        let g = Glyph::new(GlyphId(0), Some(Element::Fire), ring(1.0));
        assert_eq!(g.parent, None);
        assert!(g.linked.is_empty());
    }

    /// Canon rule 8: Repetition, Billowing, and Vision drive a spell from the
    /// center with no element at all.
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
