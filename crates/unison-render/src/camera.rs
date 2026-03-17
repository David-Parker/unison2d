//! 2D Camera

/// 2D orthographic camera
#[derive(Debug, Clone)]
pub struct Camera {
    /// Camera position (center of view)
    pub x: f32,
    pub y: f32,
    /// Viewport size in world units
    pub width: f32,
    pub height: f32,
    /// Zoom level (1.0 = normal, 2.0 = zoomed in 2x)
    pub zoom: f32,
    /// Rotation in radians
    pub rotation: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 20.0,
            height: 15.0,
            zoom: 1.0,
            rotation: 0.0,
        }
    }
}

impl Camera {
    /// Create a new camera with the given viewport size
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width,
            height,
            ..Default::default()
        }
    }

    /// Set camera position
    pub fn set_position(&mut self, x: f32, y: f32) {
        self.x = x;
        self.y = y;
    }

    /// Move camera by offset
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.x += dx;
        self.y += dy;
    }

    /// Smoothly move camera toward target position
    pub fn move_toward(&mut self, target_x: f32, target_y: f32, smoothing: f32) {
        self.x += (target_x - self.x) * smoothing;
        self.y += (target_y - self.y) * smoothing;
    }

    /// Get visible bounds (min_x, min_y, max_x, max_y)
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let half_w = self.width / (2.0 * self.zoom);
        let half_h = self.height / (2.0 * self.zoom);
        (
            self.x - half_w,
            self.y - half_h,
            self.x + half_w,
            self.y + half_h,
        )
    }

    /// Check if a point is visible
    pub fn is_visible(&self, x: f32, y: f32) -> bool {
        let (min_x, min_y, max_x, max_y) = self.bounds();
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }

    /// Convert screen coordinates to world coordinates
    pub fn screen_to_world(&self, screen_x: f32, screen_y: f32, screen_width: f32, screen_height: f32) -> (f32, f32) {
        // Normalize to -1..1
        let nx = (screen_x / screen_width) * 2.0 - 1.0;
        let ny = 1.0 - (screen_y / screen_height) * 2.0; // Flip Y

        // Scale by viewport and zoom
        let world_x = self.x + nx * (self.width / (2.0 * self.zoom));
        let world_y = self.y + ny * (self.height / (2.0 * self.zoom));

        (world_x, world_y)
    }

    /// Convert world coordinates to screen coordinates
    pub fn world_to_screen(&self, world_x: f32, world_y: f32, screen_width: f32, screen_height: f32) -> (f32, f32) {
        // Offset from camera center
        let dx = world_x - self.x;
        let dy = world_y - self.y;

        // Scale by zoom and viewport
        let nx = dx / (self.width / (2.0 * self.zoom));
        let ny = dy / (self.height / (2.0 * self.zoom));

        // Convert to screen coordinates
        let screen_x = (nx + 1.0) * 0.5 * screen_width;
        let screen_y = (1.0 - ny) * 0.5 * screen_height; // Flip Y

        (screen_x, screen_y)
    }
}
