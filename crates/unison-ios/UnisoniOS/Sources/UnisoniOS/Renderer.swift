//
//  Renderer.swift
//  UnisoniOS
//
//  Thin MTKViewDelegate that delegates all rendering to the Rust game engine.
//  Metal device and CAMetalLayer are passed to Rust at init; Rust owns all
//  pipeline state, shaders, textures, and draw calls.
//

import Metal
import MetalKit
import UnisonGameFFI

public class Renderer: NSObject, MTKViewDelegate {

    /// Opaque pointer to the Rust GameState. Passed to all FFI functions.
    public private(set) var gameState: UnsafeMutableRawPointer?

    private var lastFrameTime: CFTimeInterval = 0

    @MainActor
    public init?(metalKitView: MTKView) {
        guard let device = metalKitView.device else { return nil }
        guard let layer = metalKitView.layer as? CAMetalLayer else { return nil }

        let size = metalKitView.drawableSize

        let devicePtr = Unmanaged.passUnretained(device).toOpaque()
        let layerPtr = Unmanaged.passUnretained(layer).toOpaque()

        gameState = game_init(devicePtr, layerPtr, Float(size.width), Float(size.height))
        guard gameState != nil else { return nil }

        // Set point-based screen size immediately so the first frame uses
        // the correct coordinate system (matching UIKit touch coordinates).
        let bounds = metalKitView.bounds.size
        game_resize(gameState!, Float(bounds.width), Float(bounds.height))

        super.init()
    }

    deinit {
        if let state = gameState {
            game_destroy(state)
        }
    }

    // MARK: - MTKViewDelegate

    public func draw(in view: MTKView) {
        guard let state = gameState else { return }
        guard let drawable = view.currentDrawable else { return }

        let now = CACurrentMediaTime()
        let dt: Float
        if lastFrameTime == 0 {
            dt = 1.0 / 60.0
        } else {
            dt = Float(now - lastFrameTime)
        }
        lastFrameTime = now

        let drawablePtr = Unmanaged.passUnretained(drawable).toOpaque()
        game_frame(state, dt, drawablePtr)
    }

    public func mtkView(_ view: MTKView, drawableSizeWillChange size: CGSize) {
        guard let state = gameState else { return }
        // Pass point-based size so the game coordinate system matches
        // UIKit touch coordinates. The renderer derives pixel dimensions
        // from drawableSize / points internally.
        let bounds = view.bounds.size
        game_resize(state, Float(bounds.width), Float(bounds.height))
    }
}
