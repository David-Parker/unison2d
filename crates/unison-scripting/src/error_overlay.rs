//! Debug error overlay — captures Lua errors and renders an on-screen indicator.
//!
//! In **debug builds** (`#[cfg(debug_assertions)]`), a Lua runtime error is
//! captured here and rendered as a visual overlay each frame so the developer
//! can see at a glance that something went wrong.
//!
//! In **release builds** errors continue to be reported via `eprintln!` only.
//!
//! # Usage
//!
//! ```rust,ignore
//! // On error:
//! overlay.set(e.to_string());
//!
//! // At end of render():
//! #[cfg(debug_assertions)]
//! overlay.render(engine);
//! ```

use crate::NoAction;
use unison2d::Engine;

/// Holds the last Lua error message (debug builds only).
///
/// In release builds the struct is zero-sized and all methods are no-ops.
pub struct ErrorOverlay {
    #[cfg(debug_assertions)]
    message: Option<String>,
}

impl ErrorOverlay {
    pub fn new() -> Self {
        Self {
            #[cfg(debug_assertions)]
            message: None,
        }
    }

    /// Store an error message to display on-screen.
    #[allow(unused_variables)]
    pub fn set(&mut self, msg: impl Into<String>) {
        #[cfg(debug_assertions)]
        {
            self.message = Some(msg.into());
        }
    }

    /// Clear any stored error.
    pub fn clear(&mut self) {
        #[cfg(debug_assertions)]
        {
            self.message = None;
        }
    }

    /// Returns `true` if there is an active error.
    pub fn has_error(&self) -> bool {
        #[cfg(debug_assertions)]
        {
            self.message.is_some()
        }
        #[cfg(not(debug_assertions))]
        {
            false
        }
    }

    /// Returns the stored error message, if any.
    pub fn message(&self) -> Option<&str> {
        #[cfg(debug_assertions)]
        {
            self.message.as_deref()
        }
        #[cfg(not(debug_assertions))]
        {
            None
        }
    }

    /// Render the error overlay on screen (debug builds only).
    ///
    /// Draws a solid red rect at the top of the screen as a visual error
    /// indicator and logs the full message to stderr. Text rendering is not
    /// currently available through the renderer API, so the red bar serves
    /// as the visual cue while the full message appears in the console.
    ///
    /// The overlay is drawn as a separate compositing pass after all other
    /// rendering is complete, consistent with how PiP overlays are handled.
    #[allow(unused_variables)]
    pub fn render(&self, engine: &mut Engine<NoAction>) {
        #[cfg(debug_assertions)]
        {
            let msg = match &self.message {
                Some(m) => m,
                None => return,
            };

            // Log the full error to stderr so the developer can read the
            // stack trace even though we can't render text on screen.
            eprintln!("[unison-scripting] ERROR OVERLAY: {msg}");

            let r = match engine.renderer_mut() {
                Some(r) => r,
                None => return,
            };

            let (screen_w, screen_h) = r.screen_size();

            // Draw a dark semi-transparent background strip at the top of
            // the screen, then a bright red accent bar so the error is
            // immediately visible even without reading the console.
            use unison2d::render::{BlendMode, Camera, Color, RenderCommand};

            // Use a screen-space camera (1:1 pixel mapping, origin top-left
            // at 0,0, positive-Y down) so our pixel coordinates are exact.
            let cam = Camera::new(screen_w, screen_h);

            r.set_blend_mode(BlendMode::Alpha);
            r.begin_frame(&cam);

            // Dark overlay strip — full width, 60px tall.
            let bar_h = (screen_h * 0.08).max(40.0).min(80.0);
            r.draw(RenderCommand::Rect {
                position: [-screen_w * 0.5, screen_h * 0.5 - bar_h],
                size: [screen_w, bar_h],
                color: Color::new(0.0, 0.0, 0.0, 0.75),
            });

            // Red accent border at the very top of the bar (4px).
            let border = 4.0;
            r.draw(RenderCommand::Rect {
                position: [-screen_w * 0.5, screen_h * 0.5 - bar_h],
                size: [screen_w, border],
                color: Color::new(1.0, 0.15, 0.15, 1.0),
            });

            r.end_frame();
        }
    }
}
