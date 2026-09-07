//! A two-component vector, and nothing else.
//!
//! `glam` would satisfy §4.1 perfectly — it is pure Rust and compiles to wasm
//! untouched. Two reasons it is not here: sixty lines is a small price against a
//! third dependency in a crate that has exactly two, and every operation the
//! simulation needs fits on one screen.
//!
//! [`crate::Point`] is not the answer either. It carries `stroke_id`, which is a
//! fact about ink, and a parcel of steam was never drawn.

/// A position, or a direction and a length. The same two numbers either way.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    /// The origin, and the additive identity.
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    /// The length, squared.
    ///
    /// Kept beside [`Vec2::length`] because comparing two lengths never needs
    /// the square root, and that observation is what took the recognizer from
    /// 0.87ms to 0.41ms in M4.7.
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn dot(self, other: Vec2) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// The same direction at length one, or [`Vec2::ZERO`] if there is no
    /// direction to keep.
    ///
    /// Never divides without checking. §4.7 forbids panics, and a `NaN` that
    /// escapes here poisons every sum it later touches — which is exactly how
    /// the `-0.0` bug in `balance` got as far as the screen.
    pub fn normalize_or_zero(self) -> Vec2 {
        let length = self.length();
        if length > 0.0 && length.is_finite() {
            Vec2::new(self.x / length, self.y / length)
        } else {
            Vec2::ZERO
        }
    }

    /// The unit vector at `radians`.
    ///
    /// The bridge between the compiler and the simulation: every `placement`,
    /// every `Balance::heading`, every region heading is an angle, and this is
    /// how an angle becomes a push.
    pub fn from_angle(radians: f32) -> Vec2 {
        Vec2::new(radians.cos(), radians.sin())
    }

    /// The angle this vector points along, in radians.
    ///
    /// Zero for the zero vector, and that is a decision rather than a fallout:
    /// `atan2(-0.0, -0.0)` is `-π`, so the honest-looking answer for "no
    /// direction at all" is a firm heading due west. `balance` shipped that bug
    /// once already.
    pub fn to_angle(self) -> f32 {
        if self == Vec2::ZERO || self.length_squared() == 0.0 {
            0.0
        } else {
            self.y.atan2(self.x)
        }
    }

    /// Turned a quarter turn anticlockwise.
    ///
    /// What tilt buys: §2.4 has a sign turned toward the tangent trading reach
    /// for spin, and the tangent is the radial direction rotated by 90°.
    pub fn perpendicular(self) -> Vec2 {
        Vec2::new(-self.y, self.x)
    }

    /// Every component finite — no `NaN`, no infinity.
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite()
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2::new(self.x * rhs, self.y * rhs)
    }
}

impl std::ops::AddAssign for Vec2 {
    fn add_assign(&mut self, rhs: Vec2) {
        *self = *self + rhs;
    }
}

#[cfg(test)]
#[path = "../tests/vec2.rs"]
mod tests;
