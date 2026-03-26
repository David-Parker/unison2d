//
//  JoystickView.swift
//  UnisoniOS
//
//  Virtual joystick overlay for touch-based movement input.
//  Sends axis values to the Rust game engine via FFI.
//

import UIKit
import UnisonGameFFI

open class JoystickView: UIView {

    /// Opaque pointer to the Rust GameState. Must be set before use.
    public var gameState: UnsafeMutableRawPointer?

    /// Diameter of the joystick base in points.
    public static let defaultSize: CGFloat = 120

    private let baseLayer = CAShapeLayer()
    private let thumbLayer = CAShapeLayer()

    private let thumbRadius: CGFloat = 20
    private var trackingTouch: UITouch?

    public override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    public required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        isMultipleTouchEnabled = false
        backgroundColor = .clear

        // Base circle
        baseLayer.fillColor = UIColor.white.withAlphaComponent(0.15).cgColor
        baseLayer.strokeColor = UIColor.white.withAlphaComponent(0.3).cgColor
        baseLayer.lineWidth = 2
        layer.addSublayer(baseLayer)

        // Thumb circle
        thumbLayer.fillColor = UIColor.white.withAlphaComponent(0.5).cgColor
        layer.addSublayer(thumbLayer)
    }

    open override func layoutSublayers(of layer: CALayer) {
        super.layoutSublayers(of: layer)
        guard layer == self.layer else { return }

        let center = CGPoint(x: bounds.midX, y: bounds.midY)
        let baseRadius = min(bounds.width, bounds.height) / 2

        baseLayer.path = UIBezierPath(
            arcCenter: center,
            radius: baseRadius,
            startAngle: 0,
            endAngle: .pi * 2,
            clockwise: true
        ).cgPath

        // Thumb starts at center
        if trackingTouch == nil {
            updateThumbPosition(center)
        }
    }

    // MARK: - Touch handling

    open override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard trackingTouch == nil, let touch = touches.first else { return }
        trackingTouch = touch
        let loc = touch.location(in: self)
        handleThumbMove(to: loc)
    }

    open override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = trackingTouch, touches.contains(touch) else { return }
        let loc = touch.location(in: self)
        handleThumbMove(to: loc)
    }

    open override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = trackingTouch, touches.contains(touch) else { return }
        resetThumb()
    }

    open override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let touch = trackingTouch, touches.contains(touch) else { return }
        resetThumb()
    }

    // MARK: - Axis computation

    private func handleThumbMove(to point: CGPoint) {
        let center = CGPoint(x: bounds.midX, y: bounds.midY)
        let maxRadius = min(bounds.width, bounds.height) / 2

        var dx = point.x - center.x
        var dy = point.y - center.y
        let distance = sqrt(dx * dx + dy * dy)

        // Clamp to circle
        if distance > maxRadius {
            dx = dx / distance * maxRadius
            dy = dy / distance * maxRadius
        }

        // Normalized axis: -1 to 1
        let axisX = Float(dx / maxRadius)

        // Update visuals
        updateThumbPosition(CGPoint(x: center.x + dx, y: center.y + dy))

        // Send horizontal axis to Rust (y = 0 for side-scrolling platformer)
        if let state = gameState {
            game_set_axis(state, axisX, 0)
        }
    }

    private func resetThumb() {
        trackingTouch = nil
        let center = CGPoint(x: bounds.midX, y: bounds.midY)
        updateThumbPosition(center)

        if let state = gameState {
            game_set_axis(state, 0, 0)
        }
    }

    private func updateThumbPosition(_ center: CGPoint) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        thumbLayer.path = UIBezierPath(
            arcCenter: center,
            radius: thumbRadius,
            startAngle: 0,
            endAngle: .pi * 2,
            clockwise: true
        ).cgPath
        CATransaction.commit()
    }

    // MARK: - Hit testing

    open override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
        // Accept touches within the circular base area
        let center = CGPoint(x: bounds.midX, y: bounds.midY)
        let radius = min(bounds.width, bounds.height) / 2
        let dx = point.x - center.x
        let dy = point.y - center.y
        if dx * dx + dy * dy <= radius * radius {
            return self
        }
        return nil
    }
}
