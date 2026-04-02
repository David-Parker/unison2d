//! Event system bindings — string-keyed events + collision callbacks.
//!
//! ```lua
//! events.on("score", function(data)
//!     print("Score: " .. data.points)
//! end)
//!
//! events.emit("score", { points = 10 })
//!
//! events.on_collision(function(a, b, info)
//!     print("collision between " .. a .. " and " .. b)
//! end)
//!
//! events.on_collision_for(donut, function(other, info)
//!     print("donut hit " .. other)
//! end)
//!
//! events.on_collision_between(donut, platform, function(info)
//!     print("donut landed!")
//! end)
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use mlua::prelude::*;
use unison2d::World;


// ===================================================================
// Event system state (shared between Lua closures)
// ===================================================================

/// Holds all event registrations and pending events.
pub struct EventSystem {
    /// String-keyed event handlers: event_name → list of Lua functions.
    handlers: HashMap<String, Vec<LuaRegistryKey>>,
    /// Pending events to dispatch on next flush.
    pending: Vec<(String, Option<LuaRegistryKey>)>,
    /// Global collision callbacks.
    collision_handlers: Vec<LuaRegistryKey>,
    /// Per-object collision callbacks: object_id → list of Lua functions.
    collision_for: HashMap<u64, Vec<LuaRegistryKey>>,
    /// Pair collision callbacks: (min_id, max_id) → list of Lua functions.
    collision_between: HashMap<(u64, u64), Vec<LuaRegistryKey>>,
    /// Whether collision events are enabled on the World.
    collision_enabled: bool,
}

impl EventSystem {
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
            pending: Vec::new(),
            collision_handlers: Vec::new(),
            collision_for: HashMap::new(),
            collision_between: HashMap::new(),
            collision_enabled: false,
        }
    }

    fn ensure_collisions(&mut self, world: &mut World) {
        if !self.collision_enabled {
            world.objects.set_collision_events_enabled(true);
            self.collision_enabled = true;
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

/// Flush collision events from the World into the Lua event system.
/// Called by ScriptedGame::update() after world:step().
pub fn flush_collision_events(lua: &Lua, world: &mut World) {
    let sys = EVENT_SYSTEM.with(|cell| cell.borrow().clone());
    let sys = match sys {
        Some(s) => s,
        None => return,
    };

    let es = sys.borrow();
    if !es.collision_enabled {
        return;
    }

    let events = world.objects.translate_collision_events();
    if events.is_empty() {
        return;
    }
    drop(es); // Release borrow before calling Lua functions

    for event in &events {
        let a = event.object_a.raw();
        let b = event.object_b.raw();

        // Create info table
        let info: LuaTable = match lua.create_table() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let _ = info.set("normal_x", event.normal.x);
        let _ = info.set("normal_y", event.normal.y);
        let _ = info.set("penetration", event.penetration);
        let _ = info.set("contact_x", event.contact_point.x);
        let _ = info.set("contact_y", event.contact_point.y);

        let es = sys.borrow();

        // Global collision handlers: fn(a, b, info)
        for key in &es.collision_handlers {
            if let Ok(func) = lua.registry_value::<LuaFunction>(key) {
                let _ = func.call::<()>((a, b, info.clone()));
            }
        }

        // Per-object handlers: fn(other, info)
        if let Some(handlers) = es.collision_for.get(&a) {
            for key in handlers {
                if let Ok(func) = lua.registry_value::<LuaFunction>(key) {
                    let _ = func.call::<()>((b, info.clone()));
                }
            }
        }
        if let Some(handlers) = es.collision_for.get(&b) {
            for key in handlers {
                if let Ok(func) = lua.registry_value::<LuaFunction>(key) {
                    let _ = func.call::<()>((a, info.clone()));
                }
            }
        }

        // Pair handlers: fn(info)
        let pair = (a.min(b), a.max(b));
        if let Some(handlers) = es.collision_between.get(&pair) {
            for key in handlers {
                if let Ok(func) = lua.registry_value::<LuaFunction>(key) {
                    let _ = func.call::<()>(info.clone());
                }
            }
        }
    }
}

/// Flush string-keyed events. Called by ScriptedGame::update() after collision flush.
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

    for (name, data_key) in &pending {
        let es = sys.borrow();
        if let Some(handlers) = es.handlers.get(name) {
            for key in handlers {
                if let Ok(func) = lua.registry_value::<LuaFunction>(key) {
                    if let Some(dk) = data_key {
                        if let Ok(data) = lua.registry_value::<LuaValue>(dk) {
                            let _ = func.call::<()>(data);
                        }
                    } else {
                        let _ = func.call::<()>(());
                    }
                }
            }
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

pub fn register(lua: &Lua) -> LuaResult<()> {
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

    // events.on_collision(fn(a, b, info))
    let sys_ref = sys.clone();
    events.set("on_collision", lua.create_function(move |lua, func: LuaFunction| {
        let key = lua.create_registry_value(func)?;
        sys_ref.borrow_mut().collision_handlers.push(key);
        Ok(())
    })?)?;

    // events.on_collision_for(id, fn(other, info))
    let sys_ref = sys.clone();
    events.set("on_collision_for", lua.create_function(move |lua, (id, func): (u64, LuaFunction)| {
        let key = lua.create_registry_value(func)?;
        sys_ref.borrow_mut().collision_for.entry(id).or_default().push(key);
        Ok(())
    })?)?;

    // events.on_collision_between(a, b, fn(info))
    let sys_ref = sys.clone();
    events.set("on_collision_between", lua.create_function(move |lua, (a, b, func): (u64, u64, LuaFunction)| {
        let key = lua.create_registry_value(func)?;
        let pair = (a.min(b), a.max(b));
        sys_ref.borrow_mut().collision_between.entry(pair).or_default().push(key);
        Ok(())
    })?)?;

    lua.globals().set("events", events)?;
    Ok(())
}

/// Enable collision tracking on a world. Called when collision handlers are registered
/// and we have access to the world.
pub fn enable_collisions_for_world(world: &mut World) {
    EVENT_SYSTEM.with(|cell| {
        if let Some(sys) = cell.borrow().as_ref() {
            sys.borrow_mut().ensure_collisions(world);
        }
    });
}
