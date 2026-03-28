package com.unison2d

import android.content.Context
import android.graphics.Canvas
import android.graphics.Paint
import android.view.MotionEvent
import android.view.View
import kotlin.math.min
import kotlin.math.sqrt

/**
 * Virtual joystick overlay for touch-based movement input.
 *
 * Mirrors `JoystickView.swift` on iOS. Renders a semi-transparent
 * circular base with a draggable thumb, and reports a normalized
 * horizontal axis value (-1 to 1) via [onAxisChanged].
 */
class JoystickView(context: Context) : View(context) {

    companion object {
        /** Default diameter of the joystick base in dp. */
        const val DEFAULT_SIZE_DP = 120
        /** Extra padding around the base circle so stroke/thumb aren't clipped by view bounds. */
        const val PADDING_DP = 22f
        /** Total view size in dp (base + padding on each side). */
        const val VIEW_SIZE_DP = DEFAULT_SIZE_DP + (PADDING_DP * 2).toInt()
    }

    /**
     * Called when the joystick axis changes.
     * @param x Horizontal axis, -1.0 (left) to 1.0 (right).
     * @param y Vertical axis (always 0 for side-scrolling platformer).
     */
    var onAxisChanged: ((Float, Float) -> Unit)? = null

    private val basePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0x26FFFFFF // white @ 15% alpha
        style = Paint.Style.FILL
    }

    private val baseStrokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0x4DFFFFFF // white @ 30% alpha
        style = Paint.Style.STROKE
        strokeWidth = 2f * context.resources.displayMetrics.density
    }

    private val thumbPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = 0x80FFFFFF.toInt() // white @ 50% alpha
        style = Paint.Style.FILL
    }

    private val thumbRadiusDp = 20f
    private var thumbX = 0f
    private var thumbY = 0f
    private var tracking = false
    private var trackingPointerId = -1

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        // Reset thumb to center
        thumbX = w / 2f
        thumbY = h / 2f
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)

        val cx = width / 2f
        val cy = height / 2f
        val paddingPx = PADDING_DP * resources.displayMetrics.density
        val baseRadius = min(width, height) / 2f - paddingPx

        // Base circle
        canvas.drawCircle(cx, cy, baseRadius, basePaint)
        canvas.drawCircle(cx, cy, baseRadius, baseStrokePaint)

        // Thumb circle
        val thumbRadiusPx = thumbRadiusDp * resources.displayMetrics.density
        canvas.drawCircle(thumbX, thumbY, thumbRadiusPx, thumbPaint)
    }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                if (!tracking) {
                    tracking = true
                    trackingPointerId = event.getPointerId(0)
                    handleThumbMove(event.x, event.y)
                    return true
                }
            }
            MotionEvent.ACTION_MOVE -> {
                if (tracking) {
                    val idx = event.findPointerIndex(trackingPointerId)
                    if (idx >= 0) {
                        handleThumbMove(event.getX(idx), event.getY(idx))
                    }
                    return true
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                if (tracking) {
                    resetThumb()
                    return true
                }
            }
            MotionEvent.ACTION_POINTER_UP -> {
                val idx = event.actionIndex
                if (tracking && event.getPointerId(idx) == trackingPointerId) {
                    resetThumb()
                    return true
                }
            }
        }
        return super.onTouchEvent(event)
    }

    private fun handleThumbMove(px: Float, py: Float) {
        val cx = width / 2f
        val cy = height / 2f
        val paddingPx = PADDING_DP * resources.displayMetrics.density
        val maxRadius = min(width, height) / 2f - paddingPx

        var dx = px - cx
        var dy = py - cy
        val distance = sqrt(dx * dx + dy * dy)

        // Clamp to circle
        if (distance > maxRadius) {
            dx = dx / distance * maxRadius
            dy = dy / distance * maxRadius
        }

        thumbX = cx + dx
        thumbY = cy + dy
        invalidate()

        // Normalized horizontal axis: -1 to 1 (y = 0 for side-scrolling platformer)
        val axisX = dx / maxRadius
        onAxisChanged?.invoke(axisX, 0f)
    }

    private fun resetThumb() {
        tracking = false
        trackingPointerId = -1
        thumbX = width / 2f
        thumbY = height / 2f
        invalidate()

        onAxisChanged?.invoke(0f, 0f)
    }
}
