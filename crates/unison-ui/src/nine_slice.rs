//! 9-slice sprite rendering for textured panel backgrounds.
//!
//! A 9-slice divides a texture into 9 regions:
//! ```text
//! ┌───┬──────┬───┐
//! │ TL│  TOP │ TR│  ← corners: fixed size
//! ├───┼──────┼───┤
//! │ L │CENTER│ R │  ← edges: stretch in one direction
//! ├───┼──────┼───┤
//! │ BL│ BOT  │ BR│  ← center: stretches both directions
//! └───┴──────┴───┘
//! ```

use unison_math::Color;
use unison_render::DrawSprite;

use crate::style::NineSlice;

/// Generate 9 `DrawSprite` commands for a 9-slice panel at the given pixel-space bounds.
///
/// `x, y` = top-left corner (pixel coords, Y-down).
/// Returns sprites in draw order (bottom-left first, top-right last).
pub fn render_nine_slice(
    nine: &NineSlice,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) -> Vec<DrawSprite> {
    let border = &nine.border;
    let tw = nine.texture_width;
    let th = nine.texture_height;

    // Clamp border to half of the target size to avoid negative inner dimensions
    let bl = border.left.min(width / 2.0);
    let br = border.right.min(width / 2.0);
    let bt = border.top.min(height / 2.0);
    let bb = border.bottom.min(height / 2.0);

    // Inner region dimensions
    let inner_w = (width - bl - br).max(0.0);
    let inner_h = (height - bt - bb).max(0.0);

    // UV boundaries (normalized 0..1)
    let u_left = bl / tw;
    let u_right = 1.0 - br / tw;
    let v_top = bt / th;
    let v_bottom = 1.0 - bb / th;

    let mut sprites = Vec::with_capacity(9);

    // Helper: create a DrawSprite centered at (cx, cy) with given size and UVs
    let sprite = |cx: f32, cy: f32, sw: f32, sh: f32, uv: [f32; 4]| -> DrawSprite {
        DrawSprite {
            texture: nine.texture,
            position: [cx, cy],
            size: [sw, sh],
            rotation: 0.0,
            uv,
            color,
        }
    };

    // ── Row 1: Top ──

    // Top-left corner
    if bl > 0.0 && bt > 0.0 {
        sprites.push(sprite(
            x + bl / 2.0, y + bt / 2.0,
            bl, bt,
            [0.0, 0.0, u_left, v_top],
        ));
    }

    // Top edge
    if inner_w > 0.0 && bt > 0.0 {
        sprites.push(sprite(
            x + bl + inner_w / 2.0, y + bt / 2.0,
            inner_w, bt,
            [u_left, 0.0, u_right, v_top],
        ));
    }

    // Top-right corner
    if br > 0.0 && bt > 0.0 {
        sprites.push(sprite(
            x + bl + inner_w + br / 2.0, y + bt / 2.0,
            br, bt,
            [u_right, 0.0, 1.0, v_top],
        ));
    }

    // ── Row 2: Middle ──

    // Left edge
    if bl > 0.0 && inner_h > 0.0 {
        sprites.push(sprite(
            x + bl / 2.0, y + bt + inner_h / 2.0,
            bl, inner_h,
            [0.0, v_top, u_left, v_bottom],
        ));
    }

    // Center
    if inner_w > 0.0 && inner_h > 0.0 {
        sprites.push(sprite(
            x + bl + inner_w / 2.0, y + bt + inner_h / 2.0,
            inner_w, inner_h,
            [u_left, v_top, u_right, v_bottom],
        ));
    }

    // Right edge
    if br > 0.0 && inner_h > 0.0 {
        sprites.push(sprite(
            x + bl + inner_w + br / 2.0, y + bt + inner_h / 2.0,
            br, inner_h,
            [u_right, v_top, 1.0, v_bottom],
        ));
    }

    // ── Row 3: Bottom ──

    // Bottom-left corner
    if bl > 0.0 && bb > 0.0 {
        sprites.push(sprite(
            x + bl / 2.0, y + bt + inner_h + bb / 2.0,
            bl, bb,
            [0.0, v_bottom, u_left, 1.0],
        ));
    }

    // Bottom edge
    if inner_w > 0.0 && bb > 0.0 {
        sprites.push(sprite(
            x + bl + inner_w / 2.0, y + bt + inner_h + bb / 2.0,
            inner_w, bb,
            [u_left, v_bottom, u_right, 1.0],
        ));
    }

    // Bottom-right corner
    if br > 0.0 && bb > 0.0 {
        sprites.push(sprite(
            x + bl + inner_w + br / 2.0, y + bt + inner_h + bb / 2.0,
            br, bb,
            [u_right, v_bottom, 1.0, 1.0],
        ));
    }

    sprites
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::{EdgeInsets, NineSlice};
    use unison_math::Color;
    use unison_render::TextureId;

    fn test_nine_slice() -> NineSlice {
        NineSlice {
            texture: TextureId(1),
            border: EdgeInsets::all(8.0),
            texture_width: 32.0,
            texture_height: 32.0,
        }
    }

    #[test]
    fn emits_nine_sprites() {
        let nine = test_nine_slice();
        let sprites = render_nine_slice(&nine, 0.0, 0.0, 100.0, 80.0, Color::WHITE);
        assert_eq!(sprites.len(), 9, "should emit exactly 9 sprites");
    }

    #[test]
    fn all_sprites_use_correct_texture() {
        let nine = test_nine_slice();
        let sprites = render_nine_slice(&nine, 0.0, 0.0, 100.0, 80.0, Color::WHITE);
        for s in &sprites {
            assert_eq!(s.texture, TextureId(1));
        }
    }

    #[test]
    fn corner_sizes_preserved() {
        let nine = test_nine_slice();
        let sprites = render_nine_slice(&nine, 0.0, 0.0, 200.0, 200.0, Color::WHITE);

        // First sprite is top-left corner — should be 8x8
        assert!((sprites[0].size[0] - 8.0).abs() < 0.001);
        assert!((sprites[0].size[1] - 8.0).abs() < 0.001);

        // Last sprite is bottom-right corner — should also be 8x8
        assert!((sprites[8].size[0] - 8.0).abs() < 0.001);
        assert!((sprites[8].size[1] - 8.0).abs() < 0.001);
    }

    #[test]
    fn edge_stretching() {
        let nine = test_nine_slice();
        let sprites = render_nine_slice(&nine, 0.0, 0.0, 100.0, 80.0, Color::WHITE);

        // Top edge (second sprite): width = 100 - 8 - 8 = 84, height = 8
        assert!((sprites[1].size[0] - 84.0).abs() < 0.001);
        assert!((sprites[1].size[1] - 8.0).abs() < 0.001);

        // Left edge (4th sprite): width = 8, height = 80 - 8 - 8 = 64
        assert!((sprites[3].size[0] - 8.0).abs() < 0.001);
        assert!((sprites[3].size[1] - 64.0).abs() < 0.001);
    }

    #[test]
    fn center_stretching() {
        let nine = test_nine_slice();
        let sprites = render_nine_slice(&nine, 0.0, 0.0, 100.0, 80.0, Color::WHITE);

        // Center (5th sprite): width = 84, height = 64
        assert!((sprites[4].size[0] - 84.0).abs() < 0.001);
        assert!((sprites[4].size[1] - 64.0).abs() < 0.001);
    }

    #[test]
    fn uv_correctness() {
        let nine = test_nine_slice();
        let sprites = render_nine_slice(&nine, 0.0, 0.0, 100.0, 80.0, Color::WHITE);

        // Top-left corner: UV = [0, 0, 0.25, 0.25] (8/32 = 0.25)
        assert!((sprites[0].uv[0] - 0.0).abs() < 0.001);
        assert!((sprites[0].uv[1] - 0.0).abs() < 0.001);
        assert!((sprites[0].uv[2] - 0.25).abs() < 0.001);
        assert!((sprites[0].uv[3] - 0.25).abs() < 0.001);

        // Center: UV = [0.25, 0.25, 0.75, 0.75]
        assert!((sprites[4].uv[0] - 0.25).abs() < 0.001);
        assert!((sprites[4].uv[1] - 0.25).abs() < 0.001);
        assert!((sprites[4].uv[2] - 0.75).abs() < 0.001);
        assert!((sprites[4].uv[3] - 0.75).abs() < 0.001);

        // Bottom-right corner: UV = [0.75, 0.75, 1.0, 1.0]
        assert!((sprites[8].uv[0] - 0.75).abs() < 0.001);
        assert!((sprites[8].uv[1] - 0.75).abs() < 0.001);
        assert!((sprites[8].uv[2] - 1.0).abs() < 0.001);
        assert!((sprites[8].uv[3] - 1.0).abs() < 0.001);
    }

    #[test]
    fn small_bounds_graceful() {
        let nine = test_nine_slice();
        // Bounds smaller than 2x border (16x16 with border=8 each side)
        let sprites = render_nine_slice(&nine, 0.0, 0.0, 10.0, 10.0, Color::WHITE);
        // Should still work — corners clamped to 5px each, no inner
        for s in &sprites {
            assert!(s.size[0] >= 0.0, "no negative widths");
            assert!(s.size[1] >= 0.0, "no negative heights");
        }
    }

    #[test]
    fn asymmetric_border() {
        let nine = NineSlice {
            texture: TextureId(1),
            border: EdgeInsets { top: 4.0, right: 12.0, bottom: 8.0, left: 6.0 },
            texture_width: 64.0,
            texture_height: 64.0,
        };
        let sprites = render_nine_slice(&nine, 10.0, 20.0, 200.0, 150.0, Color::WHITE);
        assert_eq!(sprites.len(), 9);

        // Top-left corner: width = 6, height = 4
        assert!((sprites[0].size[0] - 6.0).abs() < 0.001);
        assert!((sprites[0].size[1] - 4.0).abs() < 0.001);

        // Top-right corner: width = 12, height = 4
        assert!((sprites[2].size[0] - 12.0).abs() < 0.001);
        assert!((sprites[2].size[1] - 4.0).abs() < 0.001);

        // Center: width = 200 - 6 - 12 = 182, height = 150 - 4 - 8 = 138
        assert!((sprites[4].size[0] - 182.0).abs() < 0.001);
        assert!((sprites[4].size[1] - 138.0).abs() < 0.001);
    }

    #[test]
    fn sprite_positions_cover_bounds() {
        let nine = test_nine_slice();
        let x = 50.0;
        let y = 30.0;
        let w = 100.0;
        let h = 80.0;
        let sprites = render_nine_slice(&nine, x, y, w, h, Color::WHITE);

        // All sprite centers should be within the bounds
        for s in &sprites {
            let left = s.position[0] - s.size[0] / 2.0;
            let top = s.position[1] - s.size[1] / 2.0;
            let right = s.position[0] + s.size[0] / 2.0;
            let bottom = s.position[1] + s.size[1] / 2.0;
            assert!(left >= x - 0.001, "sprite extends left of bounds: {left} < {x}");
            assert!(top >= y - 0.001, "sprite extends above bounds: {top} < {y}");
            assert!(right <= x + w + 0.001, "sprite extends right of bounds");
            assert!(bottom <= y + h + 0.001, "sprite extends below bounds");
        }
    }
}
