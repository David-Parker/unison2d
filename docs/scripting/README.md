# Writing Games for Unison 2D

Games for Unison 2D can be written in **Lua** or **TypeScript**. Both languages
produce the same Lua bytecode at runtime — TypeScript is transpiled to Lua at
build time via [TypeScriptToLua](https://typescripttolua.github.io/).

## Getting Started

- **[Lua](getting-started/lua.md)** — Direct scripting, no build step, instant hot reload.
- **[TypeScript](getting-started/typescript.md)** — Full type safety, IDE autocomplete, compile-time error checking.

## Guides

- **[Concepts](concepts.md)** — Lifecycle, scenes, events, worlds — language-neutral with dual-language examples.
- **[API Reference](api-reference.md)** — Every engine global with both Lua and TypeScript signatures.
- **[Hot Reload](hot-reload.md)** — Live code reloading for both languages.

## Which language should I use?

| | Lua | TypeScript |
|---|---|---|
| **Setup** | Zero — just write `.lua` files | Requires npm, tsconfig, TSTL |
| **Type safety** | None (dynamic) | Full (compile-time) |
| **IDE support** | Basic (Lua LSP) | Excellent (autocomplete, refactoring) |
| **Hot reload** | Direct file watch | TSTL `--watch` → file watch |
| **Best for** | Prototyping, small games | Larger projects, teams |
