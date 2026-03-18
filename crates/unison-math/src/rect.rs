//! Axis-aligned rectangle type shared across engine crates.

use crate::Vec2;

/// An axis-aligned rectangle defined by min and max corners.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub min: Vec2,
    pub max: Vec2,
}

impl Rect {
    /// Create a rect from min/max corners.
    #[inline]
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Create a rect from center position and full size (width, height).
    #[inline]
    pub fn from_center(center: Vec2, size: Vec2) -> Self {
        let half = size * 0.5;
        Self {
            min: center - half,
            max: center + half,
        }
    }

    /// Create a rect from position (bottom-left) and size.
    #[inline]
    pub fn from_position(position: Vec2, size: Vec2) -> Self {
        Self {
            min: position,
            max: position + size,
        }
    }

    /// Width of the rect.
    #[inline]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height of the rect.
    #[inline]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Size as a Vec2 (width, height).
    #[inline]
    pub fn size(&self) -> Vec2 {
        self.max - self.min
    }

    /// Center point.
    #[inline]
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }

    /// Check if a point is inside the rect.
    #[inline]
    pub fn contains(&self, point: Vec2) -> bool {
        point.x >= self.min.x && point.x <= self.max.x
            && point.y >= self.min.y && point.y <= self.max.y
    }

    /// Check if a circle intersects this rect.
    #[inline]
    pub fn intersects_circle(&self, center: Vec2, radius: f32) -> bool {
        let closest = center.clamp(self.min, self.max);
        center.distance_squared(closest) <= radius * radius
    }

    /// Check if two rects overlap.
    #[inline]
    pub fn intersects(&self, other: &Rect) -> bool {
        self.min.x <= other.max.x && self.max.x >= other.min.x
            && self.min.y <= other.max.y && self.max.y >= other.min.y
    }
}

/// Convert from the (min_x, min_y, max_x, max_y) tuple pattern used by Camera::bounds().
impl From<(f32, f32, f32, f32)> for Rect {
    fn from((min_x, min_y, max_x, max_y): (f32, f32, f32, f32)) -> Self {
        Self {
            min: Vec2::new(min_x, min_y),
            max: Vec2::new(max_x, max_y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_center() {
        let r = Rect::from_center(Vec2::new(5.0, 5.0), Vec2::new(10.0, 6.0));
        assert_eq!(r.min, Vec2::new(0.0, 2.0));
        assert_eq!(r.max, Vec2::new(10.0, 8.0));
        assert_eq!(r.width(), 10.0);
        assert_eq!(r.height(), 6.0);
    }

    #[test]
    fn test_contains() {
        let r = Rect::from_center(Vec2::ZERO, Vec2::new(10.0, 10.0));
        assert!(r.contains(Vec2::ZERO));
        assert!(r.contains(Vec2::new(5.0, 5.0)));
        assert!(!r.contains(Vec2::new(6.0, 0.0)));
    }

    #[test]
    fn test_intersects_circle() {
        let r = Rect::from_center(Vec2::ZERO, Vec2::new(4.0, 4.0));
        assert!(r.intersects_circle(Vec2::new(3.0, 0.0), 2.0));
        assert!(!r.intersects_circle(Vec2::new(10.0, 0.0), 1.0));
    }

    #[test]
    fn test_from_bounds_tuple() {
        let r: Rect = (-5.0, -3.0, 5.0, 3.0).into();
        assert_eq!(r.min, Vec2::new(-5.0, -3.0));
        assert_eq!(r.max, Vec2::new(5.0, 3.0));
    }
}
