//! `export_game!` macro — generates the 8 `extern "C"` FFI functions
//! that bridge a concrete `Game` type to the Swift host app.
//!
//! This eliminates per-game FFI boilerplate. Instead of writing 100+ lines
//! of unsafe FFI code, games just write:
//!
//! ```ignore
//! unison_ios::export_game!(MyGame, MyGame::new());
//! ```
//!
//! The macro generates 9 `extern "C"` functions: `game_init`, `game_frame`,
//! `game_resize`, `game_touch_began`, `game_touch_moved`, `game_touch_ended`,
//! `game_touch_cancelled`, `game_set_axis`, and `game_destroy`.

/// Generate all iOS FFI entry points for a concrete `Game` type.
///
/// # Arguments
/// - `$game_type` — the concrete struct that implements `unison2d::Game`
/// - `$constructor` — an expression that creates a new instance (e.g., `MyGame::new()`)
///
/// # Example
/// ```ignore
/// // In your game crate's ios_ffi.rs (or lib.rs):
/// unison_ios::export_game!(DonutGame, new_donut_game());
/// ```
///
/// This generates 9 `#[no_mangle] pub unsafe extern "C"` functions:
/// `game_init`, `game_frame`, `game_resize`, `game_touch_began`,
/// `game_touch_moved`, `game_touch_ended`, `game_touch_cancelled`,
/// `game_set_axis`, `game_destroy`.
#[macro_export]
macro_rules! export_game {
    ($game_type:ty, $constructor:expr) => {
        type __UnisonGameState = $crate::GameState<$game_type>;

        #[no_mangle]
        pub unsafe extern "C" fn game_init(
            device: *mut ::std::ffi::c_void,
            layer: *mut ::std::ffi::c_void,
            width: f32,
            height: f32,
        ) -> *mut ::std::ffi::c_void {
            let renderer = $crate::MetalRenderer::new(
                device as *mut _,
                layer as *mut _,
                width,
                height,
            )
            .expect("Failed to create Metal renderer");

            let mut state = $crate::GameState::new(renderer, $constructor);
            state.init();

            Box::into_raw(Box::new(state)) as *mut ::std::ffi::c_void
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_frame(
            state: *mut ::std::ffi::c_void,
            dt: f32,
            drawable: *mut ::std::ffi::c_void,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            state.frame(dt, drawable as *mut _);
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_resize(
            state: *mut ::std::ffi::c_void,
            width: f32,
            height: f32,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            if let Some(renderer) = state.engine_mut().renderer_mut() {
                renderer.set_screen_size(width, height);
            }
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_touch_began(
            state: *mut ::std::ffi::c_void,
            id: u64,
            x: f32,
            y: f32,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_began(state.input_mut(), id, x, y);
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_touch_moved(
            state: *mut ::std::ffi::c_void,
            id: u64,
            x: f32,
            y: f32,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_moved(state.input_mut(), id, x, y);
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_touch_ended(
            state: *mut ::std::ffi::c_void,
            id: u64,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_ended(state.input_mut(), id);
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_touch_cancelled(
            state: *mut ::std::ffi::c_void,
            id: u64,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_cancelled(state.input_mut(), id);
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_set_axis(
            state: *mut ::std::ffi::c_void,
            x: f32,
            y: f32,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::set_axis(state.input_mut(), x, y);
        }

        #[no_mangle]
        pub unsafe extern "C" fn game_destroy(state: *mut ::std::ffi::c_void) {
            if !state.is_null() {
                drop(Box::from_raw(state as *mut __UnisonGameState));
            }
        }

        /// Expose the `Engine` owned by `GameState` so Swift can pass it to
        /// the `engine_audio_*` FFI for AVAudioSession interruption handling.
        /// Returns null if `state` is null.
        #[no_mangle]
        pub unsafe extern "C" fn game_engine_ptr(
            state: *mut ::std::ffi::c_void,
        ) -> *mut ::std::ffi::c_void {
            if state.is_null() {
                return ::std::ptr::null_mut();
            }
            let state = &mut *(state as *mut __UnisonGameState);
            state.engine_mut() as *mut _ as *mut ::std::ffi::c_void
        }
    };
}
