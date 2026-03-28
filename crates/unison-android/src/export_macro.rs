//! `export_game!` macro — generates the 9 JNI entry functions
//! that bridge a concrete `Game` type to the Kotlin host app.
//!
//! This eliminates per-game JNI boilerplate. Instead of writing 100+ lines
//! of unsafe JNI code, games just write:
//!
//! ```ignore
//! unison_android::export_game!(MyGame, MyGame::new());
//! ```
//!
//! The macro generates 9 JNI functions matching `com.unison2d.UnisonNative`:
//! `gameInit`, `gameFrame`, `gameResize`, `gameTouchBegan`, `gameTouchMoved`,
//! `gameTouchEnded`, `gameTouchCancelled`, `gameSetAxis`, and `gameDestroy`.

/// Generate all Android JNI entry points for a concrete `Game` type.
///
/// # Arguments
/// - `$game_type` — the concrete struct that implements `unison2d::Game`
/// - `$constructor` — an expression that creates a new instance (e.g., `MyGame::new()`)
///
/// # Example
/// ```ignore
/// // In your game crate's android_ffi.rs (or lib.rs):
/// unison_android::export_game!(DonutGame, new_donut_game());
/// ```
///
/// This generates 9 `#[no_mangle] pub unsafe extern "system"` JNI functions
/// for the `com.unison2d.UnisonNative` Kotlin class.
#[macro_export]
macro_rules! export_game {
    ($game_type:ty, $constructor:expr) => {
        type __UnisonGameState = $crate::GameState<$game_type>;

        /// Initialize the game. Called from `UnisonNative.gameInit()`.
        /// The GL context is already current on this thread (GLSurfaceView GL thread).
        /// Returns an opaque pointer to `GameState` as a `jlong`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameInit(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            width: jni::sys::jfloat,
            height: jni::sys::jfloat,
        ) -> jni::sys::jlong {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let renderer = $crate::GlesRenderer::new(width, height)
                    .expect("Failed to create GLES renderer");

                let mut state = $crate::GameState::new(renderer, $constructor);
                state.init();

                Box::into_raw(Box::new(state)) as jni::sys::jlong
            }));
            match result {
                Ok(ptr) => ptr,
                Err(_) => 0,
            }
        }

        /// Run one display frame. Called from `UnisonNative.gameFrame()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameFrame(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
            dt: jni::sys::jfloat,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            state.frame(dt);
        }

        /// Update screen size. Called from `UnisonNative.gameResize()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameResize(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
            width: jni::sys::jfloat,
            height: jni::sys::jfloat,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            if let Some(renderer) = state.engine_mut().renderer_mut() {
                renderer.set_screen_size(width, height);
            }
        }

        /// Feed a touch-began event. Called from `UnisonNative.gameTouchBegan()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameTouchBegan(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
            id: jni::sys::jlong,
            x: jni::sys::jfloat,
            y: jni::sys::jfloat,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_began(state.input_mut(), id as u64, x, y);
        }

        /// Feed a touch-moved event. Called from `UnisonNative.gameTouchMoved()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameTouchMoved(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
            id: jni::sys::jlong,
            x: jni::sys::jfloat,
            y: jni::sys::jfloat,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_moved(state.input_mut(), id as u64, x, y);
        }

        /// Feed a touch-ended event. Called from `UnisonNative.gameTouchEnded()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameTouchEnded(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
            id: jni::sys::jlong,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_ended(state.input_mut(), id as u64);
        }

        /// Feed a touch-cancelled event. Called from `UnisonNative.gameTouchCancelled()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameTouchCancelled(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
            id: jni::sys::jlong,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::touch_cancelled(state.input_mut(), id as u64);
        }

        /// Set the virtual joystick axis. Called from `UnisonNative.gameSetAxis()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameSetAxis(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
            x: jni::sys::jfloat,
            y: jni::sys::jfloat,
        ) {
            let state = &mut *(state as *mut __UnisonGameState);
            $crate::input::set_axis(state.input_mut(), x, y);
        }

        /// Destroy the game state. Called from `UnisonNative.gameDestroy()`.
        #[no_mangle]
        pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_gameDestroy(
            _env: jni::JNIEnv,
            _class: jni::objects::JClass,
            state: jni::sys::jlong,
        ) {
            if state != 0 {
                drop(Box::from_raw(state as *mut __UnisonGameState));
            }
        }
    };
}
