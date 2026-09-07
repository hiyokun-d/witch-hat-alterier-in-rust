//! What each mark looks like, as geometry.
//!
//! # Where these came from, and what they are not
//!
//! §2 is emphatic that the rune shapes belong to the manga and must be *traced*
//! rather than invented, and `templates.ron` ships empty for exactly that
//! reason. These are not traced from the manga. They are **reconstructions of
//! the community spell simulator's own reconstructions** — simple geometry
//! described in words and rebuilt here: fire is a triangle on a stem, water is
//! three teardrops, wind is an S with rays, earth is a bar over a chevron,
//! light is a diamond in a square with four spokes. Column is a stem with a
//! crossbar, levitation is that with an arrowhead, convergence is a triangle
//! pointing down.
//!
//! So they carry the same standing as a fan-named sign in `signs.ron`: good
//! enough to build on, honest about not being canon, and **superseded the
//! moment somebody traces the real thing** and records it. That is why they
//! live beside `templates.ron` rather than inside it — a traced rune of the
//! same name wins, because the catalogue is loaded after these.
//!
//! # Why in core rather than in the shell
//!
//! Because a shape is now load-bearing. The recognizer matches against these,
//! so what a fire sigil looks like decides what a drawing *means* — and §4.2 has
//! every decision about meaning living in core. The shell draws them; it does
//! not define them.
//!
//! Everything here is a list of strokes, each a list of `(x, y)` in a unit box
//! roughly `-1..=1`, so a caller scales by whatever radius it wants.

/// One mark: several strokes, each several points.
pub type Strokes = Vec<Vec<(f32, f32)>>;

/// Points along a quadratic bezier, for the curves the straight ones cannot do.
fn curve(from: (f32, f32), via: (f32, f32), to: (f32, f32), steps: usize) -> Vec<(f32, f32)> {
    (0..=steps)
        .map(|i| {
            let t = i as f32 / steps as f32;
            let u = 1.0 - t;
            (
                u * u * from.0 + 2.0 * u * t * via.0 + t * t * to.0,
                u * u * from.1 + 2.0 * u * t * via.1 + t * t * to.1,
            )
        })
        .collect()
}

/// A closed teardrop: round at the bottom, drawn to a point at the top.
fn teardrop(cx: f32, cy: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    let tip = (cx, cy + h);
    let mut points = curve(tip, (cx - w * 1.6, cy), (cx, cy - h * 0.7), 10);
    points.extend(curve((cx, cy - h * 0.7), (cx + w * 1.6, cy), tip, 10));
    points
}

/// **Fire** — a triangle standing on a stem, with the sides overshooting.
pub fn fire() -> Strokes {
    vec![
        vec![(-0.75, -0.35), (0.0, 0.85), (0.75, -0.35), (-0.75, -0.35)],
        // The stem, which is what tells it from a plain triangle.
        vec![(0.0, -0.35), (0.0, -1.0)],
        // The two flicks the sides throw off near the base.
        vec![(-0.5, 0.1), (-0.95, 0.25)],
        vec![(0.5, 0.1), (0.95, 0.25)],
    ]
}

/// **Water** — three drops, the middle one falling on a long tail.
pub fn water() -> Strokes {
    vec![
        teardrop(-0.62, 0.15, 0.30, 0.45),
        // The centre is a drop with an S-tail running out beneath it.
        {
            let mut middle = teardrop(0.0, 0.35, 0.26, 0.40);
            middle.extend(curve((0.0, -0.05), (0.45, -0.45), (-0.30, -0.95), 14));
            middle
        },
        teardrop(0.62, 0.15, 0.24, 0.40),
    ]
}

/// **Wind** — an S turning back on itself, with rays either side.
pub fn wind() -> Strokes {
    let mut spine = curve((0.55, 0.85), (-0.55, 0.60), (0.0, 0.10), 14);
    spine.extend(curve((0.0, 0.10), (0.55, -0.40), (-0.55, -0.85), 14));
    let mut strokes = vec![spine];
    // Three rays each side, the gust coming off the curl.
    for side in [-1.0f32, 1.0] {
        for step in 0..3 {
            let y = 0.55 - step as f32 * 0.45;
            strokes.push(vec![(side * 0.72, y), (side * 1.05, y + 0.12)]);
        }
    }
    strokes
}

/// **Earth** — a bar over a chevron, with a dot to either side.
pub fn earth() -> Strokes {
    vec![
        vec![(-0.85, 0.55), (0.85, 0.55)],
        vec![(0.0, 0.55), (0.0, -0.15)],
        vec![(-0.70, 0.05), (0.0, -0.85), (0.70, 0.05)],
        // The dots, drawn as very short strokes: a single point is not ink.
        vec![(-0.95, -0.30), (-0.88, -0.30)],
        vec![(0.95, -0.30), (0.88, -0.30)],
    ]
}

/// **Light** — a diamond inside a square, with four spokes leaving it.
pub fn light() -> Strokes {
    vec![
        vec![
            (-0.60, 0.60),
            (0.60, 0.60),
            (0.60, -0.60),
            (-0.60, -0.60),
            (-0.60, 0.60),
        ],
        vec![
            (0.0, 0.85),
            (0.85, 0.0),
            (0.0, -0.85),
            (-0.85, 0.0),
            (0.0, 0.85),
        ],
        vec![(0.0, 0.60), (0.0, 1.0)],
        vec![(0.0, -0.60), (0.0, -1.0)],
        vec![(0.60, 0.0), (1.0, 0.0)],
        vec![(-0.60, 0.0), (-1.0, 0.0)],
    ]
}

/// **Column** — a stem with a bar across its foot. Length is power (§2.4).
pub fn column() -> Strokes {
    vec![
        vec![(0.0, 1.0), (0.0, -0.7)],
        vec![(-0.6, -0.7), (0.6, -0.7)],
    ]
}

/// **Levitation** — a column with an arrowhead: it lifts rather than presses.
pub fn levitation() -> Strokes {
    vec![
        vec![(0.0, 1.0), (0.0, -0.7)],
        vec![(-0.6, -0.7), (0.6, -0.7)],
        vec![(-0.34, 0.62), (0.0, 1.0), (0.34, 0.62)],
    ]
}

/// **Convergence** — a triangle pointing down: everything to a point.
pub fn convergence() -> Strokes {
    vec![vec![(-0.8, 0.8), (0.8, 0.8), (0.0, -0.9), (-0.8, 0.8)]]
}

/// Every built-in shape, by the id it stands for and which vocabulary it is in.
///
/// The order is fixed so a caller listing them gets the same list every run
/// (§4.3), and the ids are the catalogue's own — a shape naming something
/// `sigils.ron` does not define would be a rune nobody could draw.
pub fn built_in() -> Vec<(&'static str, Kind, Strokes)> {
    vec![
        ("fire", Kind::Sigil, fire()),
        ("water", Kind::Sigil, water()),
        ("wind", Kind::Sigil, wind()),
        ("earth", Kind::Sigil, earth()),
        ("light", Kind::Sigil, light()),
        ("column", Kind::Sign, column()),
        ("levitation", Kind::Sign, levitation()),
        ("convergence", Kind::Sign, convergence()),
    ]
}

pub use crate::templates::Kind;

#[cfg(test)]
#[path = "tests/shapes.rs"]
mod tests;
