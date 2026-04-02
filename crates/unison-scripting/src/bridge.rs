//! Low-level bridge utilities — render command buffer.
//!
//! Phase 1 used this module for the entire `engine` table. In Phase 2, the
//! engine bindings moved to `bindings::engine`. This module now only provides
//! the render command buffer used by the Phase 1 `engine.draw_rect()` compat
//! shim.

use unison2d::render::RenderCommand;

thread_local! {
    /// Render commands buffered during a Lua `render()` call.
    static PENDING_COMMANDS: std::cell::RefCell<Vec<RenderCommand>> =
        std::cell::RefCell::new(Vec::new());
}

/// Push a render command from Lua (used by `engine.draw_rect` compat).
pub fn push_render_command(cmd: RenderCommand) {
    PENDING_COMMANDS.with(|cell| cell.borrow_mut().push(cmd));
}

/// Drain and submit buffered render commands to the renderer.
pub fn flush_commands(renderer: &mut dyn unison2d::render::Renderer<Error = String>) {
    PENDING_COMMANDS.with(|cell| {
        let mut cmds = cell.borrow_mut();
        for cmd in cmds.drain(..) {
            renderer.draw(cmd);
        }
    });
}
