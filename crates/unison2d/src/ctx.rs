//! Ctx — unified context passed to levels during update and render.
//!
//! Replaces the split `LevelContext` / `RenderContext` with a single struct
//! that has everything a level needs: input, renderer, events, assets, and
//! shared state.
//!
//! ```ignore
//! impl Level<SharedState> for MyLevel {
//!     fn update(&mut self, ctx: &mut Ctx<SharedState>) {
//!         // Input
//!         let input = ctx.input;
//!         let dt = ctx.dt;
//!
//!         // Events
//!         ctx.events.emit(MyEvent { score: 100 });
//!
//!         // Renderer
//!         let screen = ctx.renderer.screen_size();
//!
//!         // Shared state
//!         ctx.shared.score += 1;
//!
//!         self.world.step(dt);
//!     }
//! }
//! ```

use unison_input::InputState;
use unison_render::{Camera, Color, DrawSprite, RenderCommand, Renderer, RenderTargetId, TextureId};

use crate::event_bus::{EventBus, HandlerId};
use crate::object::ObjectId;
use crate::object_system::CollisionEvent;
use crate::World;

/// Unified context passed to levels for both update and render.
///
/// Contains everything a level needs: input, renderer, events, assets,
/// delta time, and shared state. Built by `Engine::ctx()` each frame.
pub struct Ctx<'a, S = ()> {
    /// Raw input state for this frame.
    pub input: &'a InputState,
    /// Fixed timestep delta (seconds).
    pub dt: f32,
    /// Shared state provided by the Game. Levels can read/write
    /// shared state (e.g., score, inventory, events) without owning it.
    pub shared: &'a mut S,
    /// The renderer for this frame.
    pub renderer: &'a mut dyn Renderer<Error = String>,
    /// The event bus for registering handlers and emitting events.
    pub events: &'a mut EventBus<World>,
}

impl<'a, S> Ctx<'a, S> {
    /// Create an offscreen render target.
    ///
    /// Returns `(target_id, texture_id)`. Use the texture with sprite
    /// drawing to composite the result on screen.
    pub fn create_render_target(&mut self, width: u32, height: u32) -> Result<(RenderTargetId, TextureId), String> {
        self.renderer.create_render_target(width, height)
    }

    /// Bind a render target for subsequent draw calls.
    pub fn bind_render_target(&mut self, target: RenderTargetId) {
        self.renderer.bind_render_target(target);
    }

    /// Destroy an offscreen render target.
    pub fn destroy_render_target(&mut self, target: RenderTargetId) {
        self.renderer.destroy_render_target(target);
    }

    /// Get the screen/canvas size in pixels.
    pub fn screen_size(&self) -> (f32, f32) {
        self.renderer.screen_size()
    }

    /// Draw a texture as a screen-space overlay.
    ///
    /// Coordinates are in normalized screen space: (0,0) is bottom-left,
    /// (1,1) is top-right.
    pub fn draw_overlay(&mut self, texture: TextureId, position: [f32; 2], size: [f32; 2]) {
        let cx = position[0] + size[0] / 2.0;
        let cy = position[1] + size[1] / 2.0;

        let mut cam = Camera::new(1.0, 1.0);
        cam.set_position(0.5, 0.5);

        let uv = if self.renderer.fbo_origin_top_left() {
            [0.0, 0.0, 1.0, 1.0]
        } else {
            [0.0, 1.0, 1.0, 0.0]
        };

        self.renderer.bind_render_target(RenderTargetId::SCREEN);
        self.renderer.begin_frame(&cam);
        self.renderer.draw(RenderCommand::Sprite(DrawSprite {
            texture,
            position: [cx, cy],
            size,
            rotation: 0.0,
            uv,
            color: Color::WHITE,
        }));
        self.renderer.end_frame();
    }

    /// Draw a texture as a screen-space overlay with a colored border.
    pub fn draw_overlay_bordered(
        &mut self,
        texture: TextureId,
        position: [f32; 2],
        size: [f32; 2],
        border_width: f32,
        border_color: Color,
    ) {
        let cx = position[0] + size[0] / 2.0;
        let cy = position[1] + size[1] / 2.0;

        let mut cam = Camera::new(1.0, 1.0);
        cam.set_position(0.5, 0.5);

        let uv = if self.renderer.fbo_origin_top_left() {
            [0.0, 0.0, 1.0, 1.0]
        } else {
            [0.0, 1.0, 1.0, 0.0]
        };

        self.renderer.bind_render_target(RenderTargetId::SCREEN);
        self.renderer.begin_frame(&cam);

        self.renderer.draw(RenderCommand::Rect {
            position: [
                position[0] - border_width,
                position[1] - border_width,
            ],
            size: [
                size[0] + border_width * 2.0,
                size[1] + border_width * 2.0,
            ],
            color: border_color,
        });

        self.renderer.draw(RenderCommand::Sprite(DrawSprite {
            texture,
            position: [cx, cy],
            size,
            rotation: 0.0,
            uv,
            color: Color::WHITE,
        }));

        self.renderer.end_frame();
    }

    // ── UI factory ──

    /// Create a UI system pre-wired to the event bus.
    ///
    /// Events from button clicks route through the `EventBus` automatically.
    ///
    /// ```ignore
    /// let ui = ctx.create_ui::<MenuAction>(font_bytes)?;
    /// ```
    pub fn create_ui<E: Clone + 'static>(&mut self, font_bytes: Vec<u8>) -> Result<unison_ui::facade::Ui<E>, String> {
        let sink = self.events.create_sink();
        unison_ui::facade::Ui::new(font_bytes, self.renderer, sink)
    }

    // ── Event system ──

    /// Translate raw physics collision events and fire all pending event handlers.
    ///
    /// Call once per frame after `world.step()`:
    ///
    /// ```ignore
    /// self.world.step(ctx.dt);
    /// ctx.flush_events(&mut self.world);
    /// ```
    ///
    /// This:
    /// 1. Drains raw collision events from the physics engine
    /// 2. Translates `BodyHandle` → `ObjectId` and emits `CollisionEvent`s
    /// 3. Drains all event sinks and fires registered handlers
    pub fn flush_events(&mut self, world: &mut World) {
        // 1. Translate raw collision events and emit directly into bus
        for event in world.objects.translate_collision_events() {
            self.events.emit(event);
        }

        // 2. Flush all sinks and fire handlers
        let mut bus = std::mem::take(self.events);
        bus.flush(world);
        // Absorb any events emitted by handlers during flush
        let mut replacement = std::mem::take(self.events);
        bus.absorb(&mut replacement);
        *self.events = bus;
    }

    // ── Collision convenience API ──

    /// Register a handler for all collision events.
    ///
    /// Auto-enables collision event recording in the physics engine.
    ///
    /// ```ignore
    /// ctx.on_collision(|event, world| {
    ///     println!("{:?} hit {:?}", event.object_a, event.object_b);
    /// });
    /// ```
    pub fn on_collision(&mut self, world: &mut World, handler: impl FnMut(&CollisionEvent, &mut World) + 'static) -> HandlerId {
        world.objects.set_collision_events_enabled(true);
        self.events.on::<CollisionEvent>(handler)
    }

    /// Register a handler for collisions involving a specific object.
    ///
    /// The handler fires whenever `object` is either `object_a` or `object_b`.
    /// Auto-enables collision event recording.
    pub fn on_collision_for(&mut self, world: &mut World, object: ObjectId, mut handler: impl FnMut(&CollisionEvent, &mut World) + 'static) -> HandlerId {
        world.objects.set_collision_events_enabled(true);
        self.events.on::<CollisionEvent>(move |event, world| {
            if event.object_a == object || event.object_b == object {
                handler(event, world);
            }
        })
    }

    /// Register a handler for collisions between two specific objects.
    ///
    /// Fires regardless of which is `object_a` vs `object_b` (symmetric).
    /// Auto-enables collision event recording.
    pub fn on_collision_between(&mut self, world: &mut World, a: ObjectId, b: ObjectId, mut handler: impl FnMut(&CollisionEvent, &mut World) + 'static) -> HandlerId {
        world.objects.set_collision_events_enabled(true);
        self.events.on::<CollisionEvent>(move |event, world| {
            let pair = (event.object_a == a && event.object_b == b)
                || (event.object_a == b && event.object_b == a);
            if pair {
                handler(event, world);
            }
        })
    }
}
