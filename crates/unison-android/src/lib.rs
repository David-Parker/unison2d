//! Unison Android — Android platform crate for the Unison 2D engine.
//!
//! Provides:
//! - OpenGL ES 3.0 renderer (implements the `Renderer` trait)
//! - Touch input helpers (feeds Android events into `InputBuffer`)
//! - `GameState<G>` frame loop (called from Kotlin via JNI)
//! - `export_game!` macro to generate all JNI boilerplate
//!
//! This crate is **generic** — it knows nothing about any specific game.
//!
//! # Usage
//!
//! ```ignore
//! // In your game crate — this is ALL the Android code you need:
//! #[cfg(feature = "android")]
//! unison_android::export_game!(MyGame, MyGame::new());
//! ```

mod renderer;
mod shaders;
pub mod input;
mod game_loop;
mod export_macro;

pub use renderer::GlesRenderer;
pub use game_loop::GameState;

// Re-export `jni` so the `export_game!` macro expansion can refer to it as
// `$crate::jni::...` without the game crate needing a direct `jni` dep.
#[doc(hidden)]
pub use jni;

// ─────────────────────────────────────────────────────────────────────────────
// Audio lifecycle JNI bridges
// ─────────────────────────────────────────────────────────────────────────────
//
// Kotlin-visible wrappers for the `engine_audio_*` C-ABI symbols defined in
// `unison2d::engine`. Kotlin only sees symbols named
// `Java_<package>_<class>_<method>`, so we emit explicit JNI-compatible
// thunks here. The underlying `engine_audio_*` are kept alive through the
// downstream `.so` link by the `__UNISON_ANDROID_AUDIO_FFI_KEEPALIVE` static
// below (analogous to `__UNISON_IOS_AUDIO_FFI_KEEPALIVE` in `unison-ios`).
//
// The Kotlin side lives on `com.unison2d.UnisonNative` — the same JNI class
// that already hosts `gameInit` / `gameFrame` / etc. (see `UnisonNative.kt`).

use jni::objects::JClass;
use jni::sys::jlong;
use jni::JNIEnv;

/// Suspend audio output (stop pulling frames). Call from `onPause` or on
/// `AUDIOFOCUS_LOSS` / `AUDIOFOCUS_LOSS_TRANSIENT`.
///
/// `engine_ptr` is an `Engine *` as returned by `UnisonNative.gameEnginePtr`.
#[no_mangle]
pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_audioSuspend(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    if engine_ptr == 0 {
        return;
    }
    unison2d::engine_audio_suspend(engine_ptr as *mut unison2d::Engine);
}

/// Resume audio output after a system-initiated suspension (lifecycle or
/// AudioFocus regain). Call from `onResume` or on `AUDIOFOCUS_GAIN`.
#[no_mangle]
pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_audioResumeSystem(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    if engine_ptr == 0 {
        return;
    }
    unison2d::engine_audio_resume_system(engine_ptr as *mut unison2d::Engine);
}

/// Arm audio after a user gesture. Exposed for parity with iOS and the web;
/// Android does not require a user-gesture arm step, but games can still call
/// this if they want to match the cross-platform lifecycle.
#[no_mangle]
pub unsafe extern "system" fn Java_com_unison2d_UnisonNative_audioArm(
    _env: JNIEnv,
    _class: JClass,
    engine_ptr: jlong,
) {
    if engine_ptr == 0 {
        return;
    }
    unison2d::engine_audio_arm(engine_ptr as *mut unison2d::Engine);
}

// Keep the audio FFI symbols alive through the downstream staticlib link.
// `#[no_mangle]` on `engine_audio_*` (in the `unison2d` rlib) only guarantees
// the symbol exists; without a reference from this crate, the linker's
// dead-code elimination will strip them from the final `.so`. The JNI thunks
// above already reference them, but keeping this array matches the iOS
// pattern and documents the invariant explicitly.
#[doc(hidden)]
#[allow(dead_code, non_upper_case_globals)]
pub static __UNISON_ANDROID_AUDIO_FFI_KEEPALIVE:
    [unsafe extern "C" fn(*mut unison2d::Engine); 3] = [
    unison2d::engine_audio_suspend,
    unison2d::engine_audio_resume_system,
    unison2d::engine_audio_arm,
];
