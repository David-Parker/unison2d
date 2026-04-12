# Unison 2D Samples

Headless-testable reference projects — no platform scaffolding, no Cargo.

Each sample is a minimal game used for:
- Regression testing via `unison-tests` (headless ScriptedGame smoke tests)
- Tutorial targets for `docs/scripting/getting-started/`

## Samples

- **lua-minimal/** — Pure Lua. One scene, one sprite, input handling, event subscription.
- **ts-minimal/** — TypeScript source, transpiled to Lua via TSTL. Same feature coverage as lua-minimal.

## Running

These samples have no platform shells. To run visually, drop the scripts into
a project that has platform scaffolding (e.g., donut-game). To test headlessly:

```bash
cd .. && cargo test -p unison-tests --test samples
```
