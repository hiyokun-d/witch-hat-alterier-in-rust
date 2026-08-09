//! FOR AN DETAILED EXPLANATION OF HOW THE CODE WORKS IS ON THE DOCS/m4.1-circle-fit.md you can
//! just read that
//!
//! Fitting a circle to a stroke, and scoring how circular that stroke was.
//!
//! Canon makes circularity an activation gate, not a cosmetic score — a ring
//! that is not round enough produces a fleeting effect or fails outright. So
//! the number this module reports has to be *the player's* error and nothing
//! else, which rules out the obvious algorithm.
//!
//! The obvious one is Kåsa's: minimise `Σ(dᵢ² − r²)`, which is linear and
//! solves in closed form. It is also biased on arcs — the residual carries a
//! stray `(dᵢ + r)` factor that weights outer points harder, and on anything
//! less than a full circle that drags the centre into the arc and shrinks the
//! radius. Arcs are not an edge case here: canon rule 2 makes a gapped ring a
//! legal prepared spell, and rule 3 splits one ring across two objects. A bent
//! ruler cannot measure neatness.
//!
//! So this is Taubin's method (Taubin 1991, via Chernov's reference
//! derivation). It is the same one-pass moment accumulation as Kåsa with one
//! extra number subtracted from the diagonal of the solve — see
//! [`taubin_root`]. Cost over Kåsa is about twenty flops total, independent of
//! stroke length.
//!
//! Full working, with every symbol defined: `docs/m4.1-circle-fit.md`.

use std::f32::consts::TAU;

use crate::Point;

/// Distance in pixels below which two points are the same point.
///
/// Applied to the spread of the whole stroke, so it only fires when every
/// point is effectively stacked on one spot.
const COINCIDENT_EPSILON: f32 = 1e-4;

/// Zero threshold for the dimensionless quantities in the solve.
///
/// Safe as an absolute constant only because [`moments`] normalises the stroke
/// to unit scale first. On raw screen coordinates the same quantities run into
/// the billions and no fixed epsilon would mean anything.
const SOLVE_EPSILON: f32 = 1e-6;

/// Newton is quadratic and starts on the answer's doorstep, so this is a
/// runaway guard rather than a tuning knob — five steps is typical.
const MAX_NEWTON_STEPS: usize = 20;

/// A point missing the circle by more than this many times the RMS error is an
/// outlier, and is dropped before the fit is redone. See [`fit_trimmed`].
const TRIM_SIGMA: f32 = 2.5;

/// Refuse to trim if it would drop more than this share of the stroke.
///
/// Past it, the stroke is not a circle with a few strays on it — it is not a
/// circle. Trimming anyway would carve a plausible arc out of a scribble and
/// report a confident fit for it.
const MAX_TRIM_FRACTION: f32 = 0.3;

/// A circle fitted to a stroke, and how far the stroke strayed from it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircleFit {
    /// Centre of the fitted circle. `stroke_id` is carried over from the
    /// stroke it was fitted to; it takes no part in the arithmetic.
    pub center: Point,
    /// Radius of the fitted circle, in the same units as the input.
    pub radius: f32,
    /// Root-mean-square distance of the ink from the fitted circle, in pixels.
    ///
    /// Raw, not normalised, because the caller may care about either. Divide
    /// by [`CircleFit::radius`] for a size-independent figure, or use
    /// [`CircleFit::quality`].
    pub rms: f32,
    /// How many points [`fit_trimmed`] excluded from the fit. Always zero from
    /// [`fit`].
    ///
    /// Those points still count toward [`CircleFit::rms`] — they are dropped to
    /// find an honest centre, never to flatter the score.
    pub trimmed: usize,
    /// The worst single miss, in pixels.
    ///
    /// [`CircleFit::rms`] alone cannot tell a uniform wobble from one clean
    /// circle with a dent in it — same average, very different drawing. Canon
    /// grades a seal on its neatness, so the difference is worth keeping.
    pub max_miss: f32,
}

impl CircleFit {
    /// Neatness in `0.0..=1.0`, where `1.0` is a flawless circle.
    ///
    /// Normalised by radius on purpose. The raw error of a large seal is
    /// naturally bigger than that of a small one drawn with the same care, and
    /// canon has larger seals *stronger*, never sloppier — so "neat" has to
    /// mean the same thing at every size. Feeds `Ring::new`.
    pub fn quality(&self) -> f32 {
        if self.radius <= 0.0 || !self.radius.is_finite() {
            return 0.0;
        }
        (1.0 - self.rms / self.radius).clamp(0.0, 1.0)
    }
}

/// The stroke reduced to nine numbers, in a frame where the maths behaves.
///
/// Points are shifted so their centroid is the origin and scaled so their RMS
/// distance from it is one. Both steps are undone on the way out. Together
/// they are the difference between a fit that works at screen coordinates and
/// one that dissolves into rounding noise: at `(940, 512)` the `z²` terms
/// reach `10¹²`, and `f32` has seven digits to spend.
struct Moments {
    /// Centroid of the raw points — added back to the answer at the end.
    x_bar: f32,
    y_bar: f32,
    /// RMS distance from the centroid. Every `u`, `v` below is divided by it.
    scale: f32,

    m_uu: f32,
    m_vv: f32,
    m_uv: f32,
    m_uz: f32,
    m_vz: f32,
    m_zz: f32,
    /// `m_uu + m_vv`, which normalisation pins to `1.0`. Kept as a field
    /// because the solve reads it constantly and the name says what it means.
    m_z: f32,
}

/// Fits a circle to `points` by Taubin's method.
///
/// Returns `None` when the points cannot determine a circle: fewer than three
/// of them, all stacked on one spot, or lying on a straight line. Never
/// panics, and never returns a non-finite centre or radius (§4.7).
///
/// Stroke membership is ignored — this fits whatever slice it is handed.
/// Deciding which points form one ring is the caller's problem, which is what
/// lets a split seal (canon rule 3) be fitted one half at a time.
pub fn fit(points: &[Point]) -> Option<CircleFit> {
    let m = moments(points)?;
    let lambda = taubin_root(&m)?;

    let cov = m.m_uu * m.m_vv - m.m_uv * m.m_uv;

    // The determinant of the 2×2 solve. Vanishes when the points are collinear
    // — a straight line is a circle of infinite radius, and there is no
    // sensible answer to give.
    let det = lambda * lambda - lambda * m.m_z + cov;
    if !det.is_finite() || det.abs() < SOLVE_EPSILON {
        return None;
    }

    // Centre in the normalised frame. With `lambda` at zero these two lines
    // are exactly Cramer's rule on the Kåsa system; `lambda` is the whole
    // difference between the two methods.
    let u_c = (m.m_uz * (m.m_vv - lambda) - m.m_vz * m.m_uv) / (2.0 * det);
    let v_c = (m.m_vz * (m.m_uu - lambda) - m.m_uz * m.m_uv) / (2.0 * det);

    // Mean squared distance from the points to the centre is `m_z + |c|²`:
    // expanding `mean(|pᵢ − c|²)` kills the cross term because centring made
    // `mean(pᵢ)` zero. So the radius needs no second pass over the points.
    let radius = (u_c * u_c + v_c * v_c + m.m_z).sqrt() * m.scale;
    if !radius.is_finite() || radius <= 0.0 {
        return None;
    }

    let center = Point {
        x: u_c * m.scale + m.x_bar,
        y: v_c * m.scale + m.y_bar,
        stroke_id: points[0].stroke_id,
    };
    if !center.x.is_finite() || !center.y.is_finite() {
        return None;
    }

    let (rms, max_miss) = misses(points, center, radius);
    Some(CircleFit {
        center,
        radius,
        rms,
        trimmed: 0,
        max_miss,
    })
}

/// Fits a circle, ignoring the points that miss it worst, then scores the fit
/// against **every** point including those.
///
/// The hook a pen leaves at the start of a stroke is a handful of samples
/// nowhere near the ring, and least squares of any flavour will let them drag
/// the centre. Dropping them is worth a few percent of accuracy.
///
/// The two halves of that sentence must stay separate. Trimming decides *where
/// the circle is*; it must never decide *how neat the drawing was*, because
/// canon grades a seal on its mess and a score that quietly discards the mess
/// is a lie. So the refit uses the survivors and [`CircleFit::rms`] uses all of
/// them.
///
/// Deterministic — one pass, a fixed threshold, no sampling. RANSAC would do
/// this better and would also break §4.3.
///
/// Falls back to the untrimmed fit whenever trimming looks like the wrong
/// move: nothing was far enough out, too little would be left, or so much
/// would go that the stroke was never a circle to begin with.
pub fn fit_trimmed(points: &[Point]) -> Option<CircleFit> {
    let rough = fit(points)?;

    let limit = rough.rms * TRIM_SIGMA;
    // A clean stroke has an RMS near zero, and `|miss| <= 0` would then reject
    // almost every point to rounding error. Nothing to trim, so don't.
    if !limit.is_finite() || limit <= COINCIDENT_EPSILON {
        return Some(rough);
    }

    let kept: Vec<Point> = points
        .iter()
        .copied()
        .filter(|p| (p.dist(&rough.center) - rough.radius).abs() <= limit)
        .collect();

    let dropped = points.len() - kept.len();
    if dropped == 0 || kept.len() < 3 || dropped as f32 > points.len() as f32 * MAX_TRIM_FRACTION {
        return Some(rough);
    }

    let Some(refined) = fit(&kept) else {
        return Some(rough);
    };

    // Every point, survivors and outliers alike.
    let (rms, max_miss) = misses(points, refined.center, refined.radius);
    Some(CircleFit {
        rms,
        max_miss,
        trimmed: dropped,
        ..refined
    })
}

/// How a stroke is spread around the circle fitted to it.
///
/// A ring and a scribble can fit the same circle equally well; what separates
/// them is whether the ink actually goes round. This is the second of the
/// three signals — fit error, coverage, and (M4.3) turning number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coverage {
    /// The widest angular gap between neighbouring points, in radians.
    pub gap: f32,
    /// That gap measured along the circle, in the input's units.
    ///
    /// The number canon cares about. Rule 3 has two halves of a seal joining
    /// when the objects *touch*, and touching is physical — a hole is a hole at
    /// whatever radius, so closure is judged in pixels, not degrees.
    pub gap_length: f32,
    /// Direction from the centre to the middle of the gap, in `(-π, π]`.
    ///
    /// Where the opening faces. Useful for drawing it, and for M4.3 when two
    /// halves of a split ring have to be matched up.
    pub gap_heading: f32,
    /// Share of the circle lying between the two ends of the gap, in `0..=1`.
    ///
    /// Only the *largest* gap is subtracted, so a fully drawn ring reads near
    /// `1.0` however coarsely it was sampled.
    pub spanned: f32,
}

impl Coverage {
    /// Whether the ink reaches all the way round, leaving no angular hole
    /// wider than `tolerance`.
    ///
    /// **This is not the closure test.** Angle is measured from the centre, so
    /// two arcs that overlap in angle without touching each other look
    /// identical to a ring whose ends meet — both cover every direction. Canon
    /// is physical about closure: the halves of a split seal complete a spell
    /// when they *touch*. Use [`RingCandidate::closed`] for that.
    ///
    /// Still worth asking, because a ring can be joined up and still have a
    /// bite missing — a spiral, say. This is the second of the three signals.
    ///
    /// `tolerance` is in the input's units and belongs to the caller: core has
    /// no idea how wide a pen stroke is.
    ///
    /// [`RingCandidate::closed`]: crate::assembly::RingCandidate::closed
    pub fn spans_full_turn(&self, tolerance: f32) -> bool {
        self.gap_length <= tolerance
    }
}

/// Measures how a stroke wraps around `fit`.
///
/// Returns `None` for fewer than two points, which cannot have a gap between
/// them.
///
/// **Assumes the stroke goes round once.** Angles are sorted, so a figure-eight
/// or a double loop reports full coverage and a closed ring — both pass through
/// every angle. Rejecting those needs the turning number, which is M4.3; see
/// `coverage_is_fooled_by_a_double_loop`.
pub fn coverage(points: &[Point], fit: &CircleFit) -> Option<Coverage> {
    if points.len() < 2 {
        return None;
    }

    let mut angles: Vec<f32> = points
        .iter()
        .map(|p| (p.y - fit.center.y).atan2(p.x - fit.center.x))
        .collect();
    // `total_cmp` rather than `partial_cmp().unwrap()`: a total order, so the
    // sort is deterministic (§4.3) and cannot panic (§4.7).
    angles.sort_by(f32::total_cmp);

    let last = angles[angles.len() - 1];
    // Start from the wrap-around gap, the one `windows` cannot see: from the
    // largest angle forward past ±π to the smallest.
    let mut gap = angles[0] + TAU - last;
    let mut gap_starts_at = last;

    for pair in angles.windows(2) {
        let between = pair[1] - pair[0];
        if between > gap {
            gap = between;
            gap_starts_at = pair[0];
        }
    }

    // Back into `(-π, π]` the honest way. Adding or subtracting `TAU` by hand
    // needs a case for each direction and gets one of them wrong.
    let middle = gap_starts_at + gap * 0.5;
    let gap_heading = middle.sin().atan2(middle.cos());

    Some(Coverage {
        gap,
        gap_length: gap * fit.radius,
        gap_heading,
        spanned: ((TAU - gap) / TAU).clamp(0.0, 1.0),
    })
}

/// Reduces a stroke to its centred, unit-scaled moments.
fn moments(points: &[Point]) -> Option<Moments> {
    // Three points is the minimum that determines a circle.
    if points.len() < 3 {
        return None;
    }
    let n = points.len() as f32;

    let mut sum_x = 0.0;
    let mut sum_y = 0.0;
    for p in points {
        sum_x += p.x;
        sum_y += p.y;
    }
    let x_bar = sum_x / n;
    let y_bar = sum_y / n;
    if !x_bar.is_finite() || !y_bar.is_finite() {
        return None;
    }

    let mut sum_sq = 0.0;
    for p in points {
        let u = p.x - x_bar;
        let v = p.y - y_bar;
        sum_sq += u * u + v * v;
    }
    let scale = (sum_sq / n).sqrt();
    // Everything on one spot. Every circle through it fits equally well, so
    // there is no answer to pick.
    if !scale.is_finite() || scale < COINCIDENT_EPSILON {
        return None;
    }

    let mut m_uu = 0.0;
    let mut m_vv = 0.0;
    let mut m_uv = 0.0;
    let mut m_uz = 0.0;
    let mut m_vz = 0.0;
    let mut m_zz = 0.0;

    for p in points {
        let u = (p.x - x_bar) / scale;
        let v = (p.y - y_bar) / scale;
        // Note `z` is built from the *centred and scaled* coordinates. Using
        // the raw ones here is the classic way to break this file, and it
        // fails quietly — the fit still returns a plausible circle.
        let z = u * u + v * v;

        m_uu += u * u;
        m_vv += v * v;
        m_uv += u * v;
        m_uz += u * z;
        m_vz += v * z;
        m_zz += z * z;
    }

    m_uu /= n;
    m_vv /= n;
    m_uv /= n;
    m_uz /= n;
    m_vz /= n;
    m_zz /= n;

    Some(Moments {
        x_bar,
        y_bar,
        scale,
        m_uu,
        m_vv,
        m_uv,
        m_uz,
        m_vz,
        m_zz,
        m_z: m_uu + m_vv,
    })
}

/// Finds `λ`, the number that turns the Kåsa solve into the Taubin solve.
///
/// Taubin's constraint — that the mean gradient magnitude of the implicit
/// circle be one — turns the fit into a generalised eigenproblem whose
/// solution is the smallest root of one cubic in the moments. Subtracting that
/// root from the diagonal of the 2×2 system is the entire correction:
///
/// ```text
/// [ m_uu − λ    m_uv   ] [ 2a ]   [ m_uz ]
/// [   m_uv    m_vv − λ ] [ 2b ] = [ m_vz ]
/// ```
///
/// Newton starts at zero because zero *is* the Kåsa answer, and the root being
/// sought is the one nearest it — the iteration walks from one method to the
/// other. It cannot make things worse: the moment a step stops reducing the
/// residual the loop keeps the previous value, so the failure mode of this
/// function is "returns Kåsa", not "returns nonsense".
fn taubin_root(m: &Moments) -> Option<f32> {
    let cov = m.m_uu * m.m_vv - m.m_uv * m.m_uv;

    // Variance of `z`, not its second moment. `c2` wants the raw `m_zz`; `c1`
    // and `c0` want the mean subtracted off. Using `m_zz` in all three
    // compiles, runs, and fits full circles perfectly — it only goes wrong on
    // arcs, which is exactly the case Taubin exists to get right.
    let var_z = m.m_zz - m.m_z * m.m_z;

    let c3 = 4.0 * m.m_z;
    let c2 = -3.0 * m.m_z * m.m_z - m.m_zz;
    let c1 = var_z * m.m_z + 4.0 * cov * m.m_z - m.m_uz * m.m_uz - m.m_vz * m.m_vz;
    let c0 = m.m_uz * (m.m_uz * m.m_vv - m.m_vz * m.m_uv)
        + m.m_vz * (m.m_vz * m.m_uu - m.m_uz * m.m_uv)
        - var_z * cov;

    // Horner's form: fewer multiplications and less rounding than expanding
    // the powers out.
    let p = |l: f32| c0 + l * (c1 + l * (c2 + l * c3));
    let d_p = |l: f32| c1 + l * (2.0 * c2 + 3.0 * c3 * l);

    let mut lambda: f32 = 0.0;
    let mut value = c0;

    for _ in 0..MAX_NEWTON_STEPS {
        // A flat slope would send the step to infinity. Keep what we have.
        let slope = d_p(lambda);
        if slope.abs() < SOLVE_EPSILON {
            break;
        }

        let next = lambda - value / slope;
        // Exact float equality is the right test: it means the step was too
        // small to change the number, which is as converged as f32 gets.
        if !next.is_finite() || next == lambda {
            break;
        }

        let next_value = p(next);
        if next_value.abs() >= value.abs() {
            break;
        }

        lambda = next;
        value = next_value;
    }

    lambda.is_finite().then_some(lambda)
}

/// The RMS and the worst-case distance of the ink from the fitted circle, in
/// pixels, in one pass.
///
/// Deliberately infallible: by the time this runs the centre and radius are
/// already known good, and a function that cannot fail should not claim it
/// might.
///
/// Measured on the raw points. Once trimming arrives in M4.2 it will apply to
/// the *fit* only — outliers are dropped to find an honest centre, then scored
/// anyway, because the mess is the thing canon grades.
fn misses(points: &[Point], center: Point, radius: f32) -> (f32, f32) {
    let mut sum = 0.0;
    let mut worst: f32 = 0.0;
    for p in points {
        let miss = p.dist(&center) - radius;
        sum += miss * miss;
        worst = worst.max(miss.abs());
    }
    ((sum / points.len() as f32).sqrt(), worst)
}

#[cfg(test)]
#[path = "tests/circle.rs"]
mod tests;
