//! Texture types and descriptors

/// Opaque handle to a texture resource
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextureId(pub u32);

impl TextureId {
    /// A null/invalid texture ID
    pub const NONE: Self = Self(u32::MAX);

    /// Check if this is a valid texture ID
    pub fn is_valid(self) -> bool {
        self.0 != u32::MAX
    }
}

/// Texture pixel format
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextureFormat {
    /// 8-bit grayscale
    R8,
    /// 8-bit red + green
    Rg8,
    /// 24-bit RGB
    Rgb8,
    /// 32-bit RGBA
    #[default]
    Rgba8,
}

impl TextureFormat {
    /// Bytes per pixel for this format
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::R8 => 1,
            Self::Rg8 => 2,
            Self::Rgb8 => 3,
            Self::Rgba8 => 4,
        }
    }
}

/// Texture filtering mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextureFilter {
    /// Nearest-neighbor (pixelated)
    Nearest,
    /// Bilinear (smooth)
    #[default]
    Linear,
    /// Trilinear with mipmaps
    LinearMipmap,
}

/// Texture wrapping mode
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TextureWrap {
    /// Repeat the texture
    #[default]
    Repeat,
    /// Clamp to edge pixels
    ClampToEdge,
    /// Mirror and repeat
    MirroredRepeat,
}

/// Descriptor for creating a texture
#[derive(Clone, Debug)]
pub struct TextureDescriptor {
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Pixel format
    pub format: TextureFormat,
    /// Pixel data (row-major, top-to-bottom)
    pub data: Vec<u8>,
    /// Minification filter
    pub min_filter: TextureFilter,
    /// Magnification filter
    pub mag_filter: TextureFilter,
    /// Horizontal wrap mode
    pub wrap_u: TextureWrap,
    /// Vertical wrap mode
    pub wrap_v: TextureWrap,
}

impl TextureDescriptor {
    /// Create a new texture descriptor
    pub fn new(width: u32, height: u32, format: TextureFormat, data: Vec<u8>) -> Self {
        Self {
            width,
            height,
            format,
            data,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            wrap_u: TextureWrap::ClampToEdge,
            wrap_v: TextureWrap::ClampToEdge,
        }
    }

    /// Set filter mode
    pub fn with_filter(mut self, filter: TextureFilter) -> Self {
        self.min_filter = filter;
        self.mag_filter = filter;
        self
    }

    /// Set wrap mode
    pub fn with_wrap(mut self, wrap: TextureWrap) -> Self {
        self.wrap_u = wrap;
        self.wrap_v = wrap;
        self
    }

    /// Check if dimensions are power of two
    pub fn is_power_of_two(&self) -> bool {
        self.width.is_power_of_two() && self.height.is_power_of_two()
    }
}
