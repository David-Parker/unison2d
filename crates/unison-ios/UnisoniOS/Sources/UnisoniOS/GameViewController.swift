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
