package com.unison2d

/**
 * JNI bridge to the Rust game engine.
 *
 * These native methods are implemented by the `export_game!` macro in the
 * game's Rust crate. The shared library (e.g., libdonut_game.so) must be
 * loaded before calling any of these methods — [GameActivity] handles this.
 */
object UnisonNative {

    /**
     * Initialize the game engine. Must be called on the GL thread
     * (after the EGL context is current).
     *
     * @param width  Initial surface width in physical pixels.
     * @param height Initial surface height in physical pixels.
     * @return Opaque handle to the Rust GameState.
     */
    external fun gameInit(width: Float, height: Float): Long

    /**
     * Run one display frame: fixed-timestep updates + render.
     *
     * @param state Opaque handle returned by [gameInit].
     * @param dt    Time since last frame in seconds.
     */
    external fun gameFrame(state: Long, dt: Float)

    /**
     * Notify the game of a surface resize.
     *
     * @param state  Opaque handle returned by [gameInit].
     * @param width  New width in logical (dp) points.
     * @param height New height in logical (dp) points.
     */
    external fun gameResize(state: Long, width: Float, height: Float)

    /** Feed a touch-began (ACTION_DOWN) event. */
    external fun gameTouchBegan(state: Long, id: Long, x: Float, y: Float)

    /** Feed a touch-moved (ACTION_MOVE) event. */
    external fun gameTouchMoved(state: Long, id: Long, x: Float, y: Float)

    /** Feed a touch-ended (ACTION_UP) event. */
    external fun gameTouchEnded(state: Long, id: Long)

    /** Feed a touch-cancelled (ACTION_CANCEL) event. */
    external fun gameTouchCancelled(state: Long, id: Long)

    /**
     * Set the virtual joystick axis.
     *
     * @param x Horizontal axis, -1.0 (left) to 1.0 (right).
     * @param y Vertical axis, -1.0 (down) to 1.0 (up).
     */
    external fun gameSetAxis(state: Long, x: Float, y: Float)

    /** Destroy the game state and free memory. Do not use [state] after this. */
    external fun gameDestroy(state: Long)
}
