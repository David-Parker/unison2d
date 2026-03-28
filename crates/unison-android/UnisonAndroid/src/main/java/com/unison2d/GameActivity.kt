package com.unison2d

import android.app.Activity
import android.os.Bundle
import android.view.Gravity
import android.view.MotionEvent
import android.view.ViewGroup
import android.view.WindowManager
import android.widget.FrameLayout

/**
 * Base Activity for any Unison 2D game on Android.
 *
 * Mirrors the role of `GameViewController.swift` on iOS:
 * creates the GL surface, adds a virtual joystick overlay, and forwards
 * touch events to the Rust game engine via JNI.
 *
 * Games subclass this and override [nativeLibraryName] to specify
 * which `.so` to load. The subclass can be as minimal as:
 *
 * ```kotlin
 * class MainActivity : GameActivity() {
 *     override val nativeLibraryName = "donut_game"
 * }
 * ```
 */
open class GameActivity : Activity() {

    /** Name of the native library to load (without `lib` prefix or `.so` suffix). */
    open val nativeLibraryName: String = "donut_game"

    lateinit var gameSurfaceView: GameSurfaceView
        private set

    lateinit var joystickView: JoystickView
        private set

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Load the Rust game library
        System.loadLibrary(nativeLibraryName)

        // Fullscreen immersive
        @Suppress("DEPRECATION")
        window.setFlags(
            WindowManager.LayoutParams.FLAG_FULLSCREEN,
            WindowManager.LayoutParams.FLAG_FULLSCREEN
        )

        val root = FrameLayout(this)

        // GL surface (fills the screen)
        gameSurfaceView = GameSurfaceView(this)
        root.addView(
            gameSurfaceView,
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.MATCH_PARENT
        )

        // Virtual joystick overlay (bottom-left)
        joystickView = JoystickView(this)
        val joystickSizePx = (JoystickView.DEFAULT_SIZE_DP * resources.displayMetrics.density).toInt()
        val marginPx = (20 * resources.displayMetrics.density).toInt()
        val lp = FrameLayout.LayoutParams(joystickSizePx, joystickSizePx).apply {
            gravity = Gravity.BOTTOM or Gravity.START
            leftMargin = marginPx
            bottomMargin = marginPx
        }
        joystickView.onAxisChanged = { x, y ->
            val state = gameSurfaceView.gameState
            if (state != 0L) {
                gameSurfaceView.queueEvent {
                    UnisonNative.gameSetAxis(state, x, y)
                }
            }
        }
        root.addView(joystickView, lp)

        setContentView(root)
    }

    // ── Touch forwarding to Rust ──
    // All JNI calls are queued to the GL thread via queueEvent to ensure
    // thread safety (InputBuffer is accessed exclusively from the GL thread).

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val state = gameSurfaceView.gameState
        if (state == 0L) return super.onTouchEvent(event)

        val density = resources.displayMetrics.density
        val pointerIndex = event.actionIndex
        val pointerId = event.getPointerId(pointerIndex).toLong()

        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val x = event.getX(pointerIndex) / density
                val y = event.getY(pointerIndex) / density
                gameSurfaceView.queueEvent {
                    UnisonNative.gameTouchBegan(state, pointerId, x, y)
                }
            }
            MotionEvent.ACTION_MOVE -> {
                // ACTION_MOVE reports all pointers at once
                for (i in 0 until event.pointerCount) {
                    val id = event.getPointerId(i).toLong()
                    val x = event.getX(i) / density
                    val y = event.getY(i) / density
                    gameSurfaceView.queueEvent {
                        UnisonNative.gameTouchMoved(state, id, x, y)
                    }
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                gameSurfaceView.queueEvent {
                    UnisonNative.gameTouchEnded(state, pointerId)
                }
            }
            MotionEvent.ACTION_CANCEL -> {
                gameSurfaceView.queueEvent {
                    UnisonNative.gameTouchCancelled(state, pointerId)
                }
            }
        }
        return true
    }

    override fun onPause() {
        super.onPause()
        gameSurfaceView.onPause()
    }

    override fun onResume() {
        super.onResume()
        gameSurfaceView.onResume()
    }

    override fun onDestroy() {
        val state = gameSurfaceView.gameState
        if (state != 0L) {
            gameSurfaceView.queueEvent {
                UnisonNative.gameDestroy(state)
            }
        }
        super.onDestroy()
    }
}
