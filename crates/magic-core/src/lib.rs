pub mod arrangement;
pub mod assembly;
pub mod catalog;
pub mod circle;
pub mod compiler;
pub mod glyph;
pub mod naming;
pub mod recognizer;
pub mod shapes;
pub mod sim;
pub mod stroke;
pub mod templates;

pub use arrangement::{RegionArrangement, Symmetry};
pub use assembly::{
    Activation, RingCandidate, RingContents, RingRules, RingSearch, find_rings, glyphs, links,
    nesting,
};
pub use catalog::{Capabilities, Catalog, CatalogError, RegionPattern, SigilId, SignId};
pub use circle::{CircleFit, Coverage, Winding};
pub use compiler::{CompileRules, Demand, Driver, Firing, Spell, Warning, compile, compile_all};
pub use glyph::{Glaive, Glyph, GlyphId, Ring, Sign};
pub use recognizer::{Cloud, Match, Template, classify, cloud_distance, normalize, rank};
pub use sim::{
    CastReport, CastRules, Field, MaterialDef, Materials, Parcel, Phase, ReactionBook,
    ReactionReport, Sim, SimRules, SubstanceId, Vec2,
};
pub use stroke::{path_length, resample};
pub use templates::{Kind, Recorded};

// The point struct that will have x, y also stroke_id for each of stroke that in the magic
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub stroke_id: u32,
}

impl Point {
    /// Euclidean distance to `other`, ignoring stroke membership.
    pub fn dist(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
