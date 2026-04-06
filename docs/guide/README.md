# Unison 2D — User Guide

Patterns and best practices for building games with Unison 2D. For the full type/method reference, see [API.md](../API.md).

## Guides

| Guide | What you'll learn |
|-------|-------------------|
| [Getting Started](getting-started.md) | Minimal game, project setup, first soft body on screen |
| [Levels](levels.md) | Level trait, shared state, events, transitions, lifecycle hooks |
| [Prefabs & Shared Code](prefabs.md) | Reusable spawn templates, shared helpers across levels |
| [Patterns](patterns.md) | Platformer movement, spawning, cameras, PiP, despawning |

## Scripting (Lua)

Build your game entirely in Lua — no Rust game code required. The scripting guides
assume you are using `ScriptedGame` from `unison-scripting`.

| Guide | What you'll learn |
|-------|-------------------|
| [Getting Started (Scripting)](../scripting/getting-started.md) | Setup, script lifecycle, minimal example, multi-file require |
| [API Reference](../scripting/api-reference.md) | All Lua globals: engine, input, events, World, Color, Rng, math, debug |
| [Migration Guide](../scripting/migration-guide.md) | Port a Rust game to Lua — before/after examples, gotchas |
| [Hot Reload](../scripting/hot-reload.md) | Level 1 vs Level 2 reload, ScriptWatcher, web strategy, error overlay |

## When to use what

**New game, want to move fast?** Use Lua scripting. Start with
[Getting Started (Scripting)](../scripting/getting-started.md).

**Existing Rust game?** Read [Migration Guide](../scripting/migration-guide.md) to port it.

**Building with Rust directly?** Start with [Getting Started](getting-started.md).
Put everything in your `Game` struct, use `World` directly.

**Building a multi-level game in Rust?** Read [Getting Started](getting-started.md)
first, then [Levels](levels.md). Use `Level<SharedState>` for each scene, signal
transitions through events.

**Duplicating code across levels?** Read [Prefabs & Shared Code](prefabs.md). Extract
shared spawning into `Prefab` impls and shared setup into helper functions.

**Looking for a specific recipe?** Check [Patterns](patterns.md) for common gameplay
implementations.
