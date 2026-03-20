# Unison 2D — User Guide

Patterns and best practices for building games with Unison 2D. For the full type/method reference, see [API.md](../API.md).

## Guides

| Guide | What you'll learn |
|-------|-------------------|
| [Getting Started](getting-started.md) | Minimal game, project setup, first soft body on screen |
| [Levels](levels.md) | Level trait, shared state, events, transitions, lifecycle hooks |
| [Prefabs & Shared Code](prefabs.md) | Reusable spawn templates, shared helpers across levels |
| [Patterns](patterns.md) | Platformer movement, spawning, cameras, PiP, despawning |

## When to use what

**Building a single-scene game?** Start with [Getting Started](getting-started.md). Put everything in your `Game` struct, use `World` directly.

**Building a multi-level game?** Read [Getting Started](getting-started.md) first, then [Levels](levels.md). Use `Level<SharedState>` for each scene, signal transitions through events.

**Duplicating code across levels?** Read [Prefabs & Shared Code](prefabs.md). Extract shared spawning into `Prefab` impls and shared setup into helper functions.

**Looking for a specific recipe?** Check [Patterns](patterns.md) for common gameplay implementations.
