package com.unison2d

import android.content.Context
import android.opengl.GLSurfaceView
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.opengles.GL10

/**
 * GLSurfaceView that hosts the Rust game engine's OpenGL ES 3.0 renderer.
 *
 * Mirrors the role of `Renderer.swift` (MTKViewDelegate) on iOS:
 * creates the GL surface, initializes the Rust game state, and drives
 * the frame loop via [GLSurfaceView.Renderer] callbacks.
 */
class GameSurfaceView(context: Context) : GLSurfaceView(context) {

    /** Opaque handle to the Rust GameState (set on the GL thread). */
    @Volatile
    var gameState: Long = 0L
        private set

    private var lastFrameTime: Long = 0L

    init {
        setEGLContextClientVersion(3) // Request OpenGL ES 3.0
        setRenderer(GameRenderer())
        renderMode = RENDERMODE_CONTINUOUSLY // Continuous rendering (like MTKView)
    }

    private inner class GameRenderer : Renderer {

        override fun onSurfaceCreated(gl: GL10?, config: EGLConfig?) {
            // GL context is now current on this thread.
            // If we had a previous state (GL context loss), destroy it.
            if (gameState != 0L) {
                UnisonNative.gameDestroy(gameState)
                gameState = 0L
            }

            val w = width.toFloat()
            val h = height.toFloat()
            gameState = UnisonNative.gameInit(w, h)
            lastFrameTime = System.nanoTime()

            // Also send logical (dp) size for coordinate system
            val density = resources.displayMetrics.density
            UnisonNative.gameResize(gameState, w / density, h / density)
        }

        override fun onSurfaceChanged(gl: GL10?, width: Int, height: Int) {
            val state = gameState
            if (state == 0L) return
            // Pass logical (dp) dimensions — the game coordinate system should
            // match touch coordinates in dp, not physical pixels.
            val density = resources.displayMetrics.density
            UnisonNative.gameResize(state, width.toFloat() / density, height.toFloat() / density)
        }

        override fun onDrawFrame(gl: GL10?) {
            val state = gameState
            if (state == 0L) return

            val now = System.nanoTime()
            val dt = if (lastFrameTime == 0L) {
                1.0f / 60.0f
            } else {
                (now - lastFrameTime) / 1_000_000_000.0f
            }
            lastFrameTime = now

            UnisonNative.gameFrame(state, dt)
        }
    }
}
