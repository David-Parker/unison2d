//! Animation parameters — computes visual properties from widget state timers.
//!
//! Used by the render pipeline to apply enter/exit/hover animations.

use crate::state::{WidgetState, EXIT_DURATION};

/// Duration of the enter (fade-in) animation in seconds.
pub const ENTER_DURATION: f32 = 0.15;

/// Computed animation values for a single widget, used by the renderer.
#[derive(Clone, Debug)]
pub struct AnimationParams {
    /// Overall alpha multiplier (0.0 = invisible, 1.0 = fully visible).
    pub alpha: f32,
    /// Hover interpolation (0.0 = not hovered, 1.0 = fully hovered).
    pub hover_t: f32,
}

impl Default for AnimationParams {
    fn default() -> Self {
        Self {
            alpha: 1.0,
            hover_t: 0.0,
        }
    }
}

/// Compute animation parameters from widget state.
pub fn compute_animation(state: Option<&WidgetState>) -> AnimationParams {
    let state = match state {
        Some(s) => s,
        None => return AnimationParams::default(),
    };

    // Enter animation: fade in over ENTER_DURATION
    let enter_alpha = if state.animation_time < ENTER_DURATION {
        (state.animation_time / ENTER_DURATION).clamp(0.0, 1.0)
    } else {
        1.0
    };

    // Exit animation: fade out over remaining exit_time
    let exit_alpha = match state.exit_time {
        Some(t) => (t / EXIT_DURATION).clamp(0.0, 1.0),
        None => 1.0,
    };

    // Final alpha is the product of enter and exit
    let alpha = enter_alpha * exit_alpha;

    AnimationParams {
        alpha,
        hover_t: state.hover_time,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::WidgetState;

    #[test]
    fn enter_animation_start() {
        let mut ws = WidgetState::new();
        ws.animation_time = 0.0;
        let params = compute_animation(Some(&ws));
        assert!((params.alpha - 0.0).abs() < 0.01, "alpha should be ~0 at start");
    }

    #[test]
    fn enter_animation_midpoint() {
        let mut ws = WidgetState::new();
        ws.animation_time = ENTER_DURATION / 2.0;
        let params = compute_animation(Some(&ws));
        assert!((params.alpha - 0.5).abs() < 0.05, "alpha should be ~0.5 at midpoint: {}", params.alpha);
    }

    #[test]
    fn enter_animation_complete() {
        let mut ws = WidgetState::new();
        ws.animation_time = ENTER_DURATION;
        let params = compute_animation(Some(&ws));
        assert!((params.alpha - 1.0).abs() < 0.01, "alpha should be 1.0 after enter");
    }

    #[test]
    fn exit_animation_start() {
        let mut ws = WidgetState::new();
        ws.animation_time = 1.0; // past enter
        ws.exit_time = Some(EXIT_DURATION);
        let params = compute_animation(Some(&ws));
        assert!((params.alpha - 1.0).abs() < 0.01, "alpha should be 1.0 at exit start");
    }

    #[test]
    fn exit_animation_midpoint() {
        let mut ws = WidgetState::new();
        ws.animation_time = 1.0;
        ws.exit_time = Some(EXIT_DURATION / 2.0);
        let params = compute_animation(Some(&ws));
        assert!((params.alpha - 0.5).abs() < 0.05, "alpha should be ~0.5 at exit midpoint: {}", params.alpha);
    }

    #[test]
    fn exit_animation_end() {
        let mut ws = WidgetState::new();
        ws.animation_time = 1.0;
        ws.exit_time = Some(0.0);
        let params = compute_animation(Some(&ws));
        assert!((params.alpha - 0.0).abs() < 0.01, "alpha should be 0.0 at exit end");
    }

    #[test]
    fn hover_interpolation() {
        let mut ws = WidgetState::new();
        ws.animation_time = 1.0;
        ws.hover_time = 0.0;
        let p1 = compute_animation(Some(&ws));
        assert_eq!(p1.hover_t, 0.0);

        ws.hover_time = 0.5;
        let p2 = compute_animation(Some(&ws));
        assert!((p2.hover_t - 0.5).abs() < 0.001);

        ws.hover_time = 1.0;
        let p3 = compute_animation(Some(&ws));
        assert!((p3.hover_t - 1.0).abs() < 0.001);
    }

    #[test]
    fn no_state_returns_defaults() {
        let params = compute_animation(None);
        assert_eq!(params.alpha, 1.0);
        assert_eq!(params.hover_t, 0.0);
    }
}
