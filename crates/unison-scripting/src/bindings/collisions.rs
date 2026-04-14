//! Per-world collision handler registry.
//!
//! Collision handlers are now registered on the World userdata rather than on
//! the global `events` table. This module provides the backing storage, keyed
//! by a `WorldKey` derived from the `Rc<RefCell<World>>` pointer address.
//!
//! ```lua
//! world:on_collision(function(a, b, info)
//!     print("collision between " .. a .. " and " .. b)
//! end)
//!
//! world:on_collision_with(donut, function(other, info)
//!     print("donut hit " .. other)
//! end)
//!
//! world:on_collision_between(donut, platform, function(info)
//!     print("donut landed!")
//! end)
//! ```
//!
//! Handlers are cleared when the World userdata is dropped (via the `Drop` impl
//! on `LuaWorld`), so they never outlive the World.

use std::cell::RefCell;
use std::collections::HashMap;

use mlua::prelude::*;
use unison2d::World;

// ===================================================================
// World key — raw pointer address of the Rc<RefCell<World>>
// ===================================================================

/// Opaque identifier for a World instance in the collision registry.
/// We use the raw pointer address of the `Rc<RefCell<World>>` inner allocation.
pub type WorldKey = usize;

// ===================================================================
// Per-world collision state
// ===================================================================

struct WorldCollisions {
    /// Global collision callbacks: fn(a, b, info).
    handlers: Vec<LuaRegistryKey>,
    /// Per-object collision callbacks: object_id → list of fn(other, info).
    with: HashMap<u64, Vec<LuaRegistryKey>>,
    /// Pair collision callbacks: (min_id, max_id) → list of fn(info).
    between: HashMap<(u64, u64), Vec<LuaRegistryKey>>,
    /// Whether collision events are enabled on the World.
    enabled: bool,
}

impl WorldCollisions {
    fn new() -> Self {
        Self {
            handlers: Vec::new(),
            with: HashMap::new(),
            between: HashMap::new(),
            enabled: false,
        }
    }

    fn ensure_enabled(&mut self, world: &mut World) {
        if !self.enabled {
            world.objects.set_collision_events_enabled(true);
            self.enabled = true;
        }
    }
}

// ===================================================================
// Thread-local registry
// ===================================================================

thread_local! {
    static REGISTRY: RefCell<HashMap<WorldKey, WorldCollisions>> =
        RefCell::new(HashMap::new());
}

// ===================================================================
// Public API
// ===================================================================

/// Derive a `WorldKey` from the raw pointer of an `Rc<RefCell<World>>`.
pub fn key_of(rc: &std::rc::Rc<RefCell<World>>) -> WorldKey {
    std::rc::Rc::as_ptr(rc) as usize
}

/// Register a global collision handler for this world: fn(a, b, info).
pub fn register_any(
    lua: &Lua,
    world_key: WorldKey,
    world_rc: &std::rc::Rc<RefCell<World>>,
    cb: LuaFunction,
) -> LuaResult<()> {
    let key = lua.create_registry_value(cb)?;
    REGISTRY.with(|reg| {
        let mut map = reg.borrow_mut();
        let entry = map.entry(world_key).or_insert_with(WorldCollisions::new);
        entry.ensure_enabled(&mut world_rc.borrow_mut());
        entry.handlers.push(key);
    });
    Ok(())
}

/// Register a per-object collision handler: fn(other, info).
pub fn register_with(
    lua: &Lua,
    world_key: WorldKey,
    world_rc: &std::rc::Rc<RefCell<World>>,
    id: u64,
    cb: LuaFunction,
) -> LuaResult<()> {
    let key = lua.create_registry_value(cb)?;
    REGISTRY.with(|reg| {
        let mut map = reg.borrow_mut();
        let entry = map.entry(world_key).or_insert_with(WorldCollisions::new);
        entry.ensure_enabled(&mut world_rc.borrow_mut());
        entry.with.entry(id).or_default().push(key);
    });
    Ok(())
}

/// Register a pair collision handler: fn(info).
pub fn register_between(
    lua: &Lua,
    world_key: WorldKey,
    world_rc: &std::rc::Rc<RefCell<World>>,
    a: u64,
    b: u64,
    cb: LuaFunction,
) -> LuaResult<()> {
    let key = lua.create_registry_value(cb)?;
    let pair = (a.min(b), a.max(b));
    REGISTRY.with(|reg| {
        let mut map = reg.borrow_mut();
        let entry = map.entry(world_key).or_insert_with(WorldCollisions::new);
        entry.ensure_enabled(&mut world_rc.borrow_mut());
        entry.between.entry(pair).or_default().push(key);
    });
    Ok(())
}

/// Flush collision events for a world into its registered Lua callbacks.
///
/// Called by `ScriptedGame::update()` after `world:step()`.
pub fn flush(lua: &Lua, world_key: WorldKey, world: &mut World) {
    let enabled = REGISTRY.with(|reg| {
        reg.borrow().get(&world_key).map(|e| e.enabled).unwrap_or(false)
    });
    if !enabled {
        return;
    }

    let events = world.objects.translate_collision_events();
    if events.is_empty() {
        return;
    }

    for event in &events {
        let a = event.object_a.raw();
        let b = event.object_b.raw();

        // Build contact info table.
        let info: LuaTable = match lua.create_table() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let _ = info.set("normal_x", event.normal.x);
        let _ = info.set("normal_y", event.normal.y);
        let _ = info.set("penetration", event.penetration);
        let _ = info.set("contact_x", event.contact_point.x);
        let _ = info.set("contact_y", event.contact_point.y);

        // Collect all registry keys to call, releasing the borrow before
        // calling into Lua (a handler might re-enter the registry).
        let (global_keys, with_a_keys, with_b_keys, pair_keys): (
            Vec<_>,
            Vec<_>,
            Vec<_>,
            Vec<_>,
        ) = REGISTRY.with(|reg| {
            let map = reg.borrow();
            let Some(entry) = map.get(&world_key) else {
                return (vec![], vec![], vec![], vec![]);
            };

            let global = entry.handlers.iter()
                .filter_map(|k| lua.registry_value::<LuaFunction>(k).ok())
                .collect();

            let with_a = entry.with.get(&a).map(|ks| {
                ks.iter().filter_map(|k| lua.registry_value::<LuaFunction>(k).ok()).collect()
            }).unwrap_or_default();

            let with_b = entry.with.get(&b).map(|ks| {
                ks.iter().filter_map(|k| lua.registry_value::<LuaFunction>(k).ok()).collect()
            }).unwrap_or_default();

            let pair = (a.min(b), a.max(b));
            let between = entry.between.get(&pair).map(|ks| {
                ks.iter().filter_map(|k| lua.registry_value::<LuaFunction>(k).ok()).collect()
            }).unwrap_or_default();

            (global, with_a, with_b, between)
        });

        // Global: fn(a, b, info)
        for func in &global_keys {
            let _ = func.call::<()>((a, b, info.clone()));
        }

        // Per-object a: fn(other=b, info)
        for func in &with_a_keys {
            let _ = func.call::<()>((b, info.clone()));
        }

        // Per-object b: fn(other=a, info)
        for func in &with_b_keys {
            let _ = func.call::<()>((a, info.clone()));
        }

        // Pair: fn(info)
        for func in &pair_keys {
            let _ = func.call::<()>(info.clone());
        }
    }
}

/// Remove all collision handlers for a world and clean up registry keys.
///
/// Called from the `Drop` impl on `LuaWorld` (or from `ScriptedGame::drop`).
/// Note: we cannot call `lua.remove_registry_value` here without a `&Lua`
/// reference, so we simply evict the entry. The Lua GC will collect the
/// orphaned registry values when the Lua state is next collected or dropped.
pub fn clear_world(world_key: WorldKey) {
    REGISTRY.with(|reg| {
        reg.borrow_mut().remove(&world_key);
    });
}

/// Reset the entire registry (all worlds). Called from `ScriptedGame::drop()`.
pub fn reset() {
    REGISTRY.with(|reg| {
        reg.borrow_mut().clear();
    });
}
