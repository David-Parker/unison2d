//! RGBA color type shared across engine crates.

/// RGBA color with f32 components (0.0 to 1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const RED: Self = Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const GREEN: Self = Self { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };
    pub const BLUE: Self = Self { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };
    pub const TRANSPARENT: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 0.0 };

    /// Create a new color from RGBA values (0.0 to 1.0).
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a color from RGB values (0.0 to 1.0), alpha = 1.0.
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a color from u8 RGBA values (0 to 255).
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Create a color from hex value (0xRRGGBB or 0xRRGGBBAA).
    pub fn from_hex(hex: u32) -> Self {
        if hex > 0xFFFFFF {
            Self::from_rgba8(
                ((hex >> 24) & 0xFF) as u8,
                ((hex >> 16) & 0xFF) as u8,
                ((hex >> 8) & 0xFF) as u8,
                (hex & 0xFF) as u8,
            )
        } else {
            Self::from_rgba8(
                ((hex >> 16) & 0xFF) as u8,
                ((hex >> 8) & 0xFF) as u8,
                (hex & 0xFF) as u8,
                255,
            )
        }
    }

    /// Convert to [f32; 4] array.
    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Convert to [u8; 4] array.
    pub fn to_rgba8(self) -> [u8; 4] {
        [
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        ]
    }

    /// Convert to RGB tuple (discards alpha).
    pub const fn to_rgb_tuple(self) -> (f32, f32, f32) {
        (self.r, self.g, self.b)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl From<[f32; 4]> for Color {
    fn from(arr: [f32; 4]) -> Self {
        Self { r: arr[0], g: arr[1], b: arr[2], a: arr[3] }
    }
}

impl From<[f32; 3]> for Color {
    fn from(arr: [f32; 3]) -> Self {
        Self { r: arr[0], g: arr[1], b: arr[2], a: 1.0 }
    }
}

impl From<(f32, f32, f32)> for Color {
    fn from((r, g, b): (f32, f32, f32)) -> Self {
        Self { r, g, b, a: 1.0 }
    }
}

impl From<Color> for (f32, f32, f32) {
    fn from(c: Color) -> Self {
        (c.r, c.g, c.b)
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from((r, g, b, a): (f32, f32, f32, f32)) -> Self {
        Self { r, g, b, a }
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

impl From<Color> for [f32; 3] {
    fn from(c: Color) -> Self {
        [c.r, c.g, c.b]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(Color::WHITE, Color::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(Color::TRANSPARENT, Color::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_from_hex() {
        let c = Color::from_hex(0xFF0000);
        assert_eq!(c, Color::new(1.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn test_tuple_conversion() {
        let c = Color::rgb(1.0, 0.5, 0.0);
        let tup: (f32, f32, f32) = c.into();
        assert_eq!(tup, (1.0, 0.5, 0.0));

        let back: Color = tup.into();
        assert_eq!(back, Color::rgb(1.0, 0.5, 0.0));
    }

    #[test]
    fn test_array_conversion() {
        let c = Color::new(0.1, 0.2, 0.3, 0.4);
        let arr: [f32; 4] = c.into();
        assert_eq!(arr, [0.1, 0.2, 0.3, 0.4]);
    }
}
