//! Event system bindings — string-keyed pub/sub.
//!
//! ```lua
//! events.on("score", function(data)
//!     print("Score: " .. data.points)
//! end)
//!
//! events.emit("score", { points = 10 })
//! ```
//!
//! Collision callbacks have moved to the World userdata — see `world:on_collision*`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::prelude::*;


// ===================================================================
// Event system state (shared between Lua closures)
// ===================================================================

/// Holds all string-keyed event registrations and pending events.
pub struct EventSystem {
    /// String-keyed event handlers: event_name → list of Lua functions.
    handlers: HashMap<String, Vec<LuaRegistryKey>>,
    /// Pending events to dispatch on next flush.
    pending: Vec<(String, Option<LuaRegistryKey>)>,
}

impl EventSystem {
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            pending: Vec::new(),
        }
    }
}

thread_local! {
    static EVENT_SYSTEM: RefCell<Option<Rc<RefCell<EventSystem>>>> = const { RefCell::new(None) };
}

/// Initialize the event system for this Lua VM instance. Called during registration.
fn init_event_system() -> Rc<RefCell<EventSystem>> {
    let sys = Rc::new(RefCell::new(EventSystem::new()));
    EVENT_SYSTEM.with(|cell| {
        *cell.borrow_mut() = Some(sys.clone());
    });
    sys
}

/// Queue a string-keyed event with no data. Used by native subsystems (e.g. the
/// UI binding) to route events into the Lua event system.
pub fn queue_string_event(name: &str) {
    let sys = EVENT_SYSTEM.with(|cell| cell.borrow().clone());
    if let Some(sys) = sys {
        sys.borrow_mut().pending.push((name.to_string(), None));
    }
}

/// Flush string-keyed events. Called by ScriptedGame::update().
pub fn flush_string_events(lua: &Lua) {
    let sys = EVENT_SYSTEM.with(|cell| cell.borrow().clone());
    let sys = match sys {
        Some(s) => s,
        None => return,
    };

    // Take pending events
    let pending: Vec<(String, Option<LuaRegistryKey>)> = {
        let mut es = sys.borrow_mut();
        std::mem::take(&mut es.pending)
    };

    // Iterate handlers by index, re-borrowing `sys` for each lookup and
    // releasing the borrow before calling into Lua. A handler may invoke
    // `events.clear()` (directly or via a scene's on_exit during
    // engine.switch_scene), which needs `sys.borrow_mut()`; holding a borrow
    // across the call would RefCell-panic. If `clear()` wipes the handler
    // list mid-dispatch, the next lookup returns None and we break out
    // cleanly. New handlers appended during dispatch are picked up by the
    // next iteration, consistent with a "dispatch snapshot via growing vec"
    // semantics.
    for (name, data_key) in &pending {
        let mut i = 0;
        loop {
            let func = {
                let es = sys.borrow();
                let Some(key) = es.handlers.get(name).and_then(|v| v.get(i)) else {
                    break;
                };
                lua.registry_value::<LuaFunction>(key).ok()
            };
            if let Some(func) = func {
                if let Some(dk) = data_key {
                    if let Ok(data) = lua.registry_value::<LuaValue>(dk) {
                        let _ = func.call::<()>(data);
                    }
                } else {
                    let _ = func.call::<()>(());
                }
            }
            i += 1;
        }
    }

    // Clean up registry keys for event data
    for (_, data_key) in pending {
        if let Some(dk) = data_key {
            lua.remove_registry_value(dk).ok();
        }
    }
}

// ===================================================================
// Registration
// ===================================================================

pub fn populate(lua: &Lua, unison: &LuaTable) -> LuaResult<()> {
    let sys = init_event_system();
    let events = lua.create_table()?;

    // events.on("name", callback)
    let sys_ref = sys.clone();
    events.set("on", lua.create_function(move |lua, (name, func): (String, LuaFunction)| {
        let key = lua.create_registry_value(func)?;
        let mut es = sys_ref.borrow_mut();
        es.handlers.entry(name).or_default().push(key);
        Ok(())
    })?)?;

    // events.emit("name", data)
    let sys_ref = sys.clone();
    events.set("emit", lua.create_function(move |lua, (name, data): (String, Option<LuaValue>)| {
        let data_key = match data {
            Some(d) => Some(lua.create_registry_value(d)?),
            None => None,
        };
        sys_ref.borrow_mut().pending.push((name, data_key));
        Ok(())
    })?)?;

    // events.clear() — remove all string-keyed event handlers.
    // Call this from scene on_exit() to prevent duplicate handlers when
    // re-entering a scene.
    let sys_ref = sys.clone();
    events.set("clear", lua.create_function(move |lua, ()| {
        let mut es = sys_ref.borrow_mut();
        // Remove all registry values for the string-keyed handlers.
        for (_, handlers) in es.handlers.drain() {
            for key in handlers {
                lua.remove_registry_value(key).ok();
            }
        }
        // Also clear any pending (undelivered) events with data keys.
        for (_, data_key) in es.pending.drain(..) {
            if let Some(dk) = data_key {
                lua.remove_registry_value(dk).ok();
            }
        }
        Ok(())
    })?)?;

    unison.set("events", events)?;
    Ok(())
}

/// Reset the event system — clears all handlers and pending events.
/// Called from `ScriptedGame::drop()` to avoid leaking thread-local state.
pub fn reset() {
    EVENT_SYSTEM.with(|cell| {
        *cell.borrow_mut() = None;
    });
}
