//! Image decoding utility — converts raw image bytes into a [`TextureDescriptor`].
//!
//! Supports PNG, JPEG, GIF, BMP, and WebP via the `image` crate.

use crate::texture::{TextureDescriptor, TextureFormat, TextureFilter};

/// Decode image bytes (PNG, JPEG, GIF, BMP, WebP) into a [`TextureDescriptor`]
/// ready for [`Renderer::create_texture`].
///
/// The format is auto-detected from the file contents.
///
/// ```ignore
/// let bytes = engine.assets().get("textures/donut-pink.png").unwrap();
/// let desc = unison_render::decode_image(bytes)?;
/// let texture_id = renderer.create_texture(&desc)?;
/// ```
pub fn decode_image(data: &[u8]) -> Result<TextureDescriptor, String> {
    let img = image::load_from_memory(data)
        .map_err(|e| format!("Image decode error: {}", e))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    Ok(TextureDescriptor::new(
        width,
        height,
        TextureFormat::Rgba8,
        rgba.into_raw(),
    ).with_filter(TextureFilter::LinearMipmap))
}
