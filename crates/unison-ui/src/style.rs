//! Style types for UI elements — anchoring, spacing, colors, text styles.

use unison_core::Color;
use unison_render::TextureId;

/// Screen anchor point for root-level containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Edge insets for padding and margins.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    /// Uniform insets on all sides.
    pub fn all(v: f32) -> Self {
        Self { top: v, right: v, bottom: v, left: v }
    }

    /// Symmetric insets (horizontal, vertical).
    pub fn xy(x: f32, y: f32) -> Self {
        Self { top: y, right: x, bottom: y, left: x }
    }

    /// Total horizontal inset (left + right).
    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    /// Total vertical inset (top + bottom).
    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }
}

impl From<f32> for EdgeInsets {
    fn from(v: f32) -> Self {
        Self::all(v)
    }
}

/// Text style properties.
#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font_size: f32,
    pub color: Color,
    pub bold: bool,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_size: 16.0,
            color: Color::WHITE,
            bold: false,
        }
    }
}

impl TextStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

/// Visual style for panels and containers.
#[derive(Clone, Debug)]
pub struct PanelStyle {
    pub background: Color,
    pub border_color: Color,
    pub border_width: f32,
}

impl Default for PanelStyle {
    fn default() -> Self {
        Self {
            background: Color::new(0.1, 0.1, 0.1, 0.8),
            border_color: Color::TRANSPARENT,
            border_width: 0.0,
        }
    }
}

impl PanelStyle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self
    }

    pub fn border(mut self, color: Color, width: f32) -> Self {
        self.border_color = color;
        self.border_width = width;
        self
    }
}

/// 9-slice sprite definition for textured panel backgrounds.
#[derive(Clone, Debug)]
pub struct NineSlice {
    /// The texture containing the 9-slice source image.
    pub texture: TextureId,
    /// Border insets defining the non-stretching regions (in texels).
    pub border: EdgeInsets,
    /// Full texture dimensions in pixels.
    pub texture_width: f32,
    pub texture_height: f32,
}
