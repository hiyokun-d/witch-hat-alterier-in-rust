pub mod glyph;
pub use glyph::{Element, Glyph, GlyphId, Ring, Sign, SignKind};

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
mod tests {
    use super::*;

    fn p(x: f32, y: f32) -> Point {
        Point { x, y, stroke_id: 0 }
    }

    #[test]
    fn dist_same_point_is_zero() {
        assert_eq!(p(3.0, 7.0).dist(&p(3.0, 7.0)), 0.0);
    }

    #[test]
    fn dist_3_4_5_triangle() {
        assert_eq!(p(0.0, 0.0).dist(&p(3.0, 4.0)), 5.0);
    }

    #[test]
    fn dist_is_symmetric() {
        let a = p(1.5, -2.0);
        let b = p(-4.0, 6.25);
        assert_eq!(a.dist(&b), b.dist(&a));
    }

    #[test]
    fn dist_ignores_stroke_id() {
        let a = Point {
            x: 0.0,
            y: 0.0,
            stroke_id: 0,
        };
        let b = Point {
            x: 0.0,
            y: 0.0,
            stroke_id: 9,
        };
        assert_eq!(a.dist(&b), 0.0);
    }
}
