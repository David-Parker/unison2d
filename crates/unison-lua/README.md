# unison-lua

Unison 2D's build of the Lua 5.4 interpreter. Forked from
[khvzak/lua-src-rs][upstream]; lives here as a workspace crate so the
engine can tune and customize it for Unison's target matrix (iOS, web
via wasm, Android).

Consumed transparently by `mlua-sys` (and therefore `mlua`) through a
`[patch.crates-io]` entry on `lua-src`.

[upstream]: https://github.com/khvzak/lua-src-rs

## Why this exists

Three independent reasons:

1. **wasm32-unknown-unknown support.** Upstream does not build for wasm.
   We build Lua's C sources with LLVM clang and a bundled libc sysroot
   (`wasm-sysroot/include/`) so the engine's wasm target links cleanly.
2. **Interpreter customization.** Unison targets interpreters on every
   platform — LuaJIT is unavailable on iOS and wasm and not worth the
   complexity on Android alone. The performance lever we have is the
   interpreter itself, and we need to be able to iterate on Lua's C
   sources (bytecode dispatch, opcodes, GC, memory layout) without
   coordinating with upstream.
3. **Target-matrix tuning.** We want to tune `luaconf.h`, build flags,
   and potentially opcode sets for iOS/web/Android specifically, which
   upstream's general-purpose defaults don't allow.

See [docs/scripting/rationale.md](../../docs/scripting/rationale.md) for
the long-form engine-level rationale, including "why Lua 5.4 and not
5.1", "why no LuaJIT", and "why keep mlua on top".

## How it plugs in

The crate keeps `[package] name = "lua-src"` because Cargo's
`[patch.crates-io]` resolves substitutions by package name — so the
crate that replaces the published `lua-src` must still be called
`lua-src`. Unison branding is carried by the directory
(`crates/unison-lua/`), this README, and the `NOTICE` file.

The `[lib] name = "lua_src"` line is preserved independently so that
`mlua-sys`'s `use lua_src::...` imports keep working after the patch
substitution.

Consumers wire it up via:

```toml
[patch.crates-io]
lua-src = { git = "https://github.com/David-Parker/unison2d", tag = "..." }
```

Freshly scaffolded Unison games get this patch block automatically; the
`unison link` and `unison unlink` commands rewrite it uniformly
alongside the other engine dependencies.

## Relationship to mlua

`mlua` sits *above* the Lua C API; this crate sits *below* it. The C API
(`lua_pushnumber`, `lua_pcall`, …) has been stable since 1993, so we can
modify the interpreter as aggressively as we want (dispatch, opcodes,
GC, memory layout) without breaking mlua, as long as the C API function
signatures are preserved. See the rationale doc for the "when we'd
reconsider mlua" discussion.

## License & attribution

Forked from `lua-src-rs` by Aleksandr Orlenko (khvzak) under the MIT
license. Lua itself is © 1994–2024 Lua.org, PUC-Rio, also MIT. See
`LICENSE` and `NOTICE`.
