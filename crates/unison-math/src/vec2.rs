//! 2D vector type shared across engine crates.

use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 2D vector with f32 components.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub const ONE: Self = Self { x: 1.0, y: 1.0 };
    pub const UP: Self = Self { x: 0.0, y: 1.0 };
    pub const DOWN: Self = Self { x: 0.0, y: -1.0 };
    pub const LEFT: Self = Self { x: -1.0, y: 0.0 };
    pub const RIGHT: Self = Self { x: 1.0, y: 0.0 };

    /// Create a new Vec2.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Create a Vec2 with both components set to the same value.
    #[inline]
    pub const fn splat(v: f32) -> Self {
        Self { x: v, y: v }
    }

    /// Squared length (avoids sqrt).
    #[inline]
    pub fn length_squared(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Length of the vector.
    #[inline]
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Normalize to unit length. Returns ZERO for zero-length vectors.
    #[inline]
    pub fn normalized(self) -> Self {
        let len = self.length();
        if len < 1e-10 {
            Self::ZERO
        } else {
            self / len
        }
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    /// 2D cross product (returns scalar: the z-component of the 3D cross product).
    #[inline]
    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    /// Distance to another point.
    #[inline]
    pub fn distance(self, other: Self) -> f32 {
        (self - other).length()
    }

    /// Squared distance to another point (avoids sqrt).
    #[inline]
    pub fn distance_squared(self, other: Self) -> f32 {
        (self - other).length_squared()
    }

    /// Linear interpolation between self and other.
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }

    /// Per-component minimum.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        Self {
            x: self.x.min(other.x),
            y: self.y.min(other.y),
        }
    }

    /// Per-component maximum.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        Self {
            x: self.x.max(other.x),
            y: self.y.max(other.y),
        }
    }

    /// Per-component clamp.
    #[inline]
    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            x: self.x.clamp(min.x, max.x),
            y: self.y.clamp(min.y, max.y),
        }
    }

    /// Convert to array.
    #[inline]
    pub const fn to_array(self) -> [f32; 2] {
        [self.x, self.y]
    }

    /// Convert to tuple.
    #[inline]
    pub const fn to_tuple(self) -> (f32, f32) {
        (self.x, self.y)
    }
}

// --- Operator impls ---

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl SubAssign for Vec2 {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f32) -> Self {
        Self { x: self.x * rhs, y: self.y * rhs }
    }
}

impl Mul<Vec2> for f32 {
    type Output = Vec2;
    #[inline]
    fn mul(self, rhs: Vec2) -> Vec2 {
        Vec2 { x: self * rhs.x, y: self * rhs.y }
    }
}

impl MulAssign<f32> for Vec2 {
    #[inline]
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
    }
}

impl Div<f32> for Vec2 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f32) -> Self {
        Self { x: self.x / rhs, y: self.y / rhs }
    }
}

impl DivAssign<f32> for Vec2 {
    #[inline]
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self { x: -self.x, y: -self.y }
    }
}

// --- Conversions ---

impl From<[f32; 2]> for Vec2 {
    #[inline]
    fn from(arr: [f32; 2]) -> Self {
        Self { x: arr[0], y: arr[1] }
    }
}

impl From<Vec2> for [f32; 2] {
    #[inline]
    fn from(v: Vec2) -> Self {
        [v.x, v.y]
    }
}

impl From<(f32, f32)> for Vec2 {
    #[inline]
    fn from((x, y): (f32, f32)) -> Self {
        Self { x, y }
    }
}

impl From<Vec2> for (f32, f32) {
    #[inline]
    fn from(v: Vec2) -> Self {
        (v.x, v.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    fn assert_close(a: f32, b: f32, msg: &str) {
        assert!((a - b).abs() < EPSILON, "{}: expected {}, got {}", msg, b, a);
    }

    #[test]
    fn test_basic_ops() {
        let a = Vec2::new(3.0, 4.0);
        let b = Vec2::new(1.0, 2.0);

        let sum = a + b;
        assert_close(sum.x, 4.0, "add x");
        assert_close(sum.y, 6.0, "add y");

        let diff = a - b;
        assert_close(diff.x, 2.0, "sub x");
        assert_close(diff.y, 2.0, "sub y");

        let scaled = a * 2.0;
        assert_close(scaled.x, 6.0, "mul x");
        assert_close(scaled.y, 8.0, "mul y");

        let divided = a / 2.0;
        assert_close(divided.x, 1.5, "div x");
        assert_close(divided.y, 2.0, "div y");
    }

    #[test]
    fn test_length_and_normalize() {
        let v = Vec2::new(3.0, 4.0);
        assert_close(v.length(), 5.0, "length");
        assert_close(v.length_squared(), 25.0, "length_squared");

        let n = v.normalized();
        assert_close(n.length(), 1.0, "normalized length");
        assert_close(n.x, 0.6, "normalized x");
        assert_close(n.y, 0.8, "normalized y");
    }

    #[test]
    fn test_dot_and_cross() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert_close(a.dot(b), 0.0, "perpendicular dot");
        assert_close(a.cross(b), 1.0, "cross");
    }

    #[test]
    fn test_conversions() {
        let v = Vec2::new(1.0, 2.0);

        let arr: [f32; 2] = v.into();
        assert_eq!(arr, [1.0, 2.0]);

        let back: Vec2 = arr.into();
        assert_eq!(back, v);

        let tup: (f32, f32) = v.into();
        assert_eq!(tup, (1.0, 2.0));

        let back2: Vec2 = tup.into();
        assert_eq!(back2, v);
    }

    #[test]
    fn test_lerp() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 20.0);
        let mid = a.lerp(b, 0.5);
        assert_close(mid.x, 5.0, "lerp x");
        assert_close(mid.y, 10.0, "lerp y");
    }
}
