//
//  GameViewController.swift
//  UnisoniOS
//
//  Generic UIViewController for any Unison 2D game.
//  Sets up MTKView and forwards touch events to Rust via FFI.
//

import UIKit
import MetalKit
import UnisonGameFFI

open class GameViewController: UIViewController {

    public var mtkView: MTKView!
    public var renderer: Renderer!
    public private(set) var joystickView: JoystickView!

    open override func viewDidLoad() {
        super.viewDidLoad()

        guard let mtkView = view as? MTKView else {
            print("[UnisoniOS] View is not an MTKView")
            return
        }

        guard let device = MTLCreateSystemDefaultDevice() else {
            print("[UnisoniOS] Metal is not supported on this device")
            return
        }

        mtkView.device = device
        mtkView.colorPixelFormat = .bgra8Unorm
        mtkView.depthStencilPixelFormat = .invalid
        mtkView.backgroundColor = UIColor.black

        guard let newRenderer = Renderer(metalKitView: mtkView) else {
            print("[UnisoniOS] Renderer cannot be initialized")
            return
        }

        renderer = newRenderer
        mtkView.delegate = renderer
        self.mtkView = mtkView

        view.isMultipleTouchEnabled = true
    }

    /// Install the built-in virtual joystick overlay (bottom-left). Call from
    /// a subclass's `viewDidLoad` (after `super.viewDidLoad()`) when the game
    /// wants the default joystick UI. Not installed by default — games that
    /// handle touch input directly (e.g. via `input.pointer_position`) don't
    /// need it and shouldn't pay for the overlay.
    open func installJoystick() {
        guard let renderer = renderer else { return }
        let size = JoystickView.defaultSize
        let joystick = JoystickView()
        joystick.gameState = renderer.gameState
        joystick.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(joystick)
        NSLayoutConstraint.activate([
            joystick.widthAnchor.constraint(equalToConstant: size),
            joystick.heightAnchor.constraint(equalToConstant: size),
            joystick.leadingAnchor.constraint(equalTo: view.safeAreaLayoutGuide.leadingAnchor, constant: 20),
            joystick.bottomAnchor.constraint(equalTo: view.safeAreaLayoutGuide.bottomAnchor, constant: -20),
        ])
        joystickView = joystick
    }

    // MARK: - Touch forwarding to Rust

    open override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let state = renderer?.gameState else { return }
        for touch in touches {
            let loc = touch.location(in: view)
            game_touch_began(state, UInt64(touch.hash), Float(loc.x), Float(loc.y))
        }
    }

    open override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let state = renderer?.gameState else { return }
        for touch in touches {
            let loc = touch.location(in: view)
            game_touch_moved(state, UInt64(touch.hash), Float(loc.x), Float(loc.y))
        }
    }

    open override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let state = renderer?.gameState else { return }
        for touch in touches {
            game_touch_ended(state, UInt64(touch.hash))
        }
    }

    open override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        guard let state = renderer?.gameState else { return }
        for touch in touches {
            game_touch_cancelled(state, UInt64(touch.hash))
        }
    }
}
