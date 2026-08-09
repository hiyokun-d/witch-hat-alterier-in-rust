pub mod arrangement;
pub mod assembly;
pub mod catalog;
pub mod circle;
pub mod glyph;

pub use arrangement::{RegionArrangement, Symmetry};
pub use assembly::{RingCandidate, RingSearch, find_rings};
pub use catalog::{Capabilities, Catalog, CatalogError, RegionPattern, SigilId, SignId};
pub use circle::{CircleFit, Coverage};
pub use glyph::{Glyph, GlyphId, Ring, Sign};

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
