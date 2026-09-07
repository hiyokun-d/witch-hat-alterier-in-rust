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
    /// How large the sign is drawn, in the same units as the ring's radius.
    ///
    /// **Size is power.** Canon is explicit: column signs all the same size
    /// give "the same power… a balanced spell that shoots straight up", while
    /// one sign "far longer than the others… has more power, causing uneven
    /// pressure which makes the spell shoot off to the side" (§2.4).
    ///
    /// Without this, symmetry can only ever be measured by *position*, and a
    /// seal with evenly spaced signs of wildly different lengths would read as
    /// perfectly balanced while shooting sideways.
    pub size: f32,
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

    /// How far this sign is tilted off the radial direction, in radians,
    /// `0..=π`.
    ///
    /// Zero points straight out or straight in; `π/2` lies tangent to the
    /// ring. Canon: "By tilting the signs within a seal, it is possible to
    /// produce a spell that rotates. The more tilted the signs, the more spin
    /// but less reach."
    pub fn tilt(&self) -> f32 {
        self.radial_alignment().clamp(-1.0, 1.0).acos()
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

/// A claw-shaped protrusion: how firmly a spell embeds itself in a body.
///
/// **Neither a sign nor a sigil.** The Magic page says so outright, so this is
/// a fourth kind of mark rather than a sign with an odd name — which is why it
/// has its own type and its own list on [`Glyph`] instead of a [`SignId`].
///
/// Canon: glaives "determine how firmly a spell will imbed itself into one's
/// body". Whether that means depth or tenacity is unclear, and stays unclear
/// here. Nearly forgotten since the Day of the Pact; seen on memory erasure and
/// slime rendering.
///
/// **The exception to rule 1.** "Unlike any other known sign, glaives can be
/// drawn outside of the ring as long as they are still connected to it." So a
/// glaive whose placement puts it beyond the ring is correctly drawn, not stray
/// ink, and nothing may drop it for being outside.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glaive {
    /// The angle it sits at around the ring, as [`Sign::placement`].
    pub placement: f32,
    /// How large it is drawn, in the same units as the ring's radius.
    ///
    /// Size is the only thing canon gives us to read here, and it reads the
    /// same way it does everywhere else in §2.4: bigger is more.
    pub size: f32,
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
    /// Whether the ink was a **ring** at all, as opposed to a triangle, a
    /// spiral or a figure-eight that happened to fit a circle.
    ///
    /// Kept apart from `closed` and `quality` because it answers a different
    /// question, and conflating the three is exactly how a fire sigil drawn on
    /// its own came to fire as canon rule 9's explosion: its triangle is a
    /// closed loop, it fits a circle well enough, and nothing downstream ever
    /// asked whether it was round.
    simple: bool,
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
            // Assumed, so every existing caller keeps its meaning. Only the
            // ring search knows otherwise, and it says so with `not_a_ring`.
            simple: true,
        }
    }

    /// Marks the ink as not a ring: tangled, doubled, or not round.
    pub fn not_a_ring(mut self) -> Ring {
        self.simple = false;
        self
    }

    /// Whether the ink is a ring rather than some other closed shape.
    pub fn is_simple(&self) -> bool {
        self.simple
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
    /// How far the sigil's ink reaches from the centre, in the same units as
    /// the ring's radius and as [`Sign::size`].
    ///
    /// **Size is intensity.** Canon: "the size of a sigil in relation to the
    /// ring determines the intensity and strength of the spell's effect, with
    /// larger sigils creating more powerful effects" (§2.2). It does *not*
    /// change what the sigil does — the recognizer scores shape alone and
    /// deliberately divides scale out (§3.1), which is exactly why this has to
    /// be carried here. Nothing downstream could recover it.
    ///
    /// Zero when there is no sigil, or when whoever built the glyph did not
    /// measure one; the compiler then falls back to the ring's own size rather
    /// than reporting an intensity it does not have.
    pub sigil_extent: f32,
    /// The keystones arranged around the sigil.
    pub signs: Vec<Sign>,
    /// Strokes inside the ring that nothing could put a name to.
    ///
    /// **Not the same as an empty ring, and conflating the two was a lie on
    /// screen.** Rule 9's discharge is what a ring with *nothing* in it does; a
    /// ring full of marks the recognizer cannot read is a spell we failed to
    /// read, and it must not be reported as a bare one. Until `templates.ron`
    /// holds traced runes this is every mark on the pad.
    pub unnamed: usize,
    /// Claw-shaped marks saying how firmly the spell embeds in a body.
    ///
    /// A separate list because a glaive is neither a sign nor a sigil (§2.1),
    /// and folding it into `signs` would put it into `balance`, `spin` and the
    /// symmetry classification, where canon never puts it.
    pub glaives: Vec<Glaive>,
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
            sigil_extent: 0.0,
            ring,
            signs: Vec::new(),
            unnamed: 0,
            glaives: Vec::new(),
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
#[path = "tests/glyph.rs"]
mod tests;
