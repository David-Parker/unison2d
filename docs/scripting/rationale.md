# Scripting Rationale

Design decisions behind Unison's scripting layer. Each section captures
the *why* in enough depth to prevent re-litigating the decision later.

## Contents

- [Why Lua 5.4 (and not 5.1)](#why-lua-54-and-not-51)
- [Why no LuaJIT](#why-no-luajit)
- [Why fork lua-src](#why-fork-lua-src)
- [Why mlua (and not raw mlua-sys or a hand-rolled binding)](#why-mlua-and-not-raw-mlua-sys-or-a-hand-rolled-binding)

---

## Why Lua 5.4 (and not 5.1)

### Why 5.1 is still dominant in the wider Lua world

Three overlapping forces keep Lua 5.1 the de facto version for most
performance-sensitive Lua users:

1. **LuaJIT.** Mike Pall's LuaJIT is Lua 5.1 API+ABI compatible. It
   supports optional 5.2 language/library extensions (`goto`, bitwise
   operators library, `__pairs`/`__ipairs` metamethods) and a handful of
   5.3 extensions (Unicode escapes, `table.move`, `coroutine.isyieldable`,
   integer division `//` on floats). It cannot add 5.3's integer type or
   5.4's `<const>`/`<close>` without breaking the 5.1 ABI it preserves.
   For performance-sensitive Lua users (WoW, Roblox, nginx/OpenResty,
   Redis, Torch, love2d historically), LuaJIT was the obvious runtime, so
   5.1 was the obvious target.
2. **Library ecosystem network effect.** Because LuaJIT ran 5.1, the
   LuaRocks ecosystem grew around 5.1. New adopters picked 5.1 to get
   those libraries. Self-reinforcing.
3. **Breaking changes across minor versions.** 5.2 replaced the module
   system with `_ENV`, removed `setfenv`/`getfenv`, added `goto`. 5.3 added
   an integer type and bitwise operators, changed string-to-number
   coercion. 5.4 added `<const>`/`<close>` and switched to generational
   GC. Every upgrade is a small migration, so existing codebases stick.

LuaJIT itself has been in a holding pattern since 2015, when Mike Pall
resigned from day-to-day maintenance, citing the lack of additional
maintainers. Only occasional patches have landed on the 2.1 branch since.
Forks (OpenResty, RaptorJIT) maintain their own patches but haven't moved
the semantic baseline off 5.1. So the "fast Lua" universe is effectively
stuck at 5.1 semantics, even though the Lua language itself has moved on.

### Why none of this applies to Unison

- **No library ecosystem dependency.** Game code is authored in
  TypeScript and transpiled via [TypeScriptToLua (TSTL)][tstl], which
  supports `JIT`, `5.4`, `5.3`, `5.2`, `5.1`, `5.0`, and `universal` as
  compilation targets. Targeting 5.4 costs us nothing on the authoring
  side. We don't use LuaRocks.
- **5.4's language improvements matter.** Integer type (no silent
  float-coercion for counters/IDs), `<const>` (catch mutation bugs at
  compile time), `<close>` (RAII-style deterministic cleanup, important
  for engine resource handles crossing the FFI boundary), generational GC
  (better for allocation-heavy game loops), and the xoshiro256** RNG are
  all worth taking.
- **No 5.4 JIT exists.** On any platform where we'd be running an
  interpreter (see next section), 5.4 and 5.1 both run through reference-
  interpreter-equivalent mechanics. The "5.1 because LuaJIT gives you
  10-100x" argument collapses into "5.1 because library ecosystem" —
  which doesn't apply to us.

Existing 5.4-compatible JIT projects are dialects (Ravi, with MIR-based
JIT) or research (Deegen / LuaJIT Remake, targeting 5.1 as a learning
vehicle). Neither is a drop-in replacement.

## Why no LuaJIT

Unison targets iOS, web (wasm), and Android. LuaJIT's JIT cannot run on
two of the three.

### iOS

Apple enforces W^X at the kernel level. Writing bytes into a memory page
and then executing them — which is what every JIT does — requires the
`dynamic-codesigning` entitlement, which gates `MAP_JIT` (the only way to
allocate a writable+executable page on iOS). Apple awards this entitlement
only to system processes that ship JavaScript engines (WebKit /
JavaScriptCore). Third-party apps cannot get it, and Apple's App Store
policies have historically rejected apps that rely on JIT execution. iOS
18.4 further tightened this by restricting even debugger-mediated JIT
paths that sideloading tools had relied on.

For emulators and other JIT-heavy software, this has been a well-known
limitation for years. For our purposes it means: any design that assumes
LuaJIT runs on iOS is a non-starter.

### Wasm (web)

The intuition "wasm is code, so couldn't a wasm-hosted LuaJIT just emit
more wasm at runtime?" is reasonable. The mechanism for runtime wasm
generation does exist. But the design of wasm prevents the LuaJIT
execution model from paying off through it.

**What wasm's security model forbids:** A wasm module's linear memory is
strictly non-executable. There is no instruction sequence in wasm that
jumps into dynamically-produced bytes stored in memory. Functions live in
a fixed indirect-call table created at module instantiation time. You can
call through the table by index, and you can grow the table
(`WebAssembly.Table.grow`), but new table entries must point at functions
that came from an *already-instantiated* wasm module. In-module code
generation is not a capability wasm offers.

**What wasm's execution model allows:** A wasm module can produce a wasm
binary (a sequence of bytes) in its linear memory, then call out to the
JavaScript host, which can invoke `new WebAssembly.Module(bytes)` →
`new WebAssembly.Instance(module)` and register the new module's exports
back into the caller's table. The browser's wasm compiler (V8 Liftoff +
TurboFan, SpiderMonkey baseline + IonMonkey, JSC BBQ + OMG) compiles the
new module to native code, and the caller can then invoke it via the
indirect-call table.

This mechanism is gated by the Content Security Policy directive
[`wasm-unsafe-eval`][wasm-csp] (a newer, more targeted directive than
legacy `unsafe-eval`). Many production sites disallow both, which breaks
dynamic instantiation entirely.

**Why this mechanism cannot sustain a LuaJIT-style trace compiler:**

1. **Granularity mismatch.** LuaJIT traces are tens to hundreds of
   machine instructions emitted in microseconds. The wasm module
   instantiation pipeline validates the binary, baseline-compiles it, and
   potentially tiers up to the optimizing compiler. That's milliseconds
   per trace minimum. For tight Lua loops that LuaJIT would normally
   trace-compile in a few microseconds and execute millions of times, the
   compile-per-trace cost through this path exceeds the savings.
2. **Boundary cost.** Each JIT-produced function call crosses wasm → JS
   → wasm. Native LuaJIT traces chain into each other directly via
   register calling conventions; wasm-module-per-trace can't match that.
3. **ABI fiction.** LuaJIT traces share the interpreter's state — VM
   registers, stack, GC write barriers — with the interpreter via raw
   pointers, as if they were inlined extensions of the inner dispatch
   loop. Between wasm modules you can import linear memory, but every
   trace has to re-derive pointers through imported helpers instead of
   inlining loads/stores. The "trace is fast because it's a natural
   extension of the interpreter" property that makes LuaJIT work is gone.
4. **Backend rewrite.** LuaJIT's x86/ARM machine-code emitter would need
   replacement with a wasm-bytecode emitter, and wasm's instruction set
   isn't a good match for LuaJIT's low-level idioms (tagged pointers,
   unaligned loads, NaN-boxing representation tricks).
5. **CSP exposure.** Even when all four of the above are acceptable,
   many production sites block `wasm-unsafe-eval` entirely.

No production Lua-in-wasm system attempts this. The consistent path has
been "ship an interpreter in wasm; let the browser JIT the interpreter;
accept the interpreter's performance characteristics."

**What you actually get for free:** When we ship `unison-lua` compiled to
wasm, the browser's wasm JIT compiles our interpreter's dispatch loop to
native code at module load. So the interpreter itself runs at native-ish
speeds, subject to wasm's constraints (no computed goto; all memory
accesses bounds-checked against linear memory base; limited escape
analysis; fixed-arity indirect calls). Academic benchmarks typically show
wasm at ~1.45–2.5× slower than native for general code; a Lua VM ported
to JavaScript in Firefox has measured at ~64% of native speed, and wasm
generally outperforms JS. We expect wasm Lua to land in a similar range.

User Lua code, though, is just *data* from the browser's perspective —
bytecode in our wasm heap. The browser can't JIT-compile it the way it
would JIT user JavaScript, because the browser doesn't know it's code.
That's why our performance lever isn't "get the browser to JIT user Lua"
— it's "ship the fastest interpreter we can."

### Android

Android doesn't have iOS's JIT restrictions, so LuaJIT would technically
run. But supporting LuaJIT on Android while interpreting on iOS + web
would cost us:

- **Different Lua semantics per platform.** TSTL would have to emit
  different code for the JIT target (5.1 + extensions) vs. 5.4. We'd lose
  the integer type, `<const>`/`<close>`, native bitwise operators, and
  the xoshiro RNG on Android-only, or else we'd constrain our whole
  codebase to LuaJIT-compatible features.
- **Different mlua binding surfaces.** mlua's `lua54` and `luajit`
  features produce different binding crates with different generated FFI.
- **Platform-dependent game behavior.** Floating-point coercion rules,
  integer operations, and GC timing would differ by platform. Subtle
  bugs.

High complexity tax for one platform's benefit.

### Therefore

The performance lever we actually have is the interpreter itself.
Porting LuaJIT interpreter techniques (NaN-boxing of tagged values,
tighter bytecode dispatch, threaded code where the target allows,
optimized instruction decoding) into 5.4's reference interpreter is the
realistic path. The `unison-lua` crate exists in part to make this work
possible. [Deegen / LuaJIT Remake][deegen] is concrete prior art worth
studying during that phase — it's 5.1-targeted, but the interpreter
construction techniques translate.

## Why fork lua-src

Three reasons:

1. **Wasm32 support.** Upstream `lua-src-rs` rejects
   `wasm32-unknown-unknown` with "don't know how to build Lua for this
   target." We ship a minimal C11 build using LLVM clang and a bundled
   libc sysroot (`wasm-sysroot/include/`). Upstream has not merged wasm
   support.
2. **Interpreter customization.** Future interpreter-speed work (see
   above) requires modifying Lua's C sources. An external fork keeps
   those changes local and fast to iterate without upstream coordination.
   Configured as a workspace member, the fork lives alongside the engine
   crates and participates in the same build/test cycle.
3. **Target matrix tuning.** Unison targets iOS, web, and Android. We
   want to tune `luaconf.h`, build flags, and potentially opcode sets for
   that specific matrix, without constraining ourselves to upstream's
   general-purpose defaults.

Practically, this also lets us:

- Trim to Lua 5.4 only (upstream ships 5.1/5.2/5.3/5.4 source trees).
  Smaller crate, simpler build logic.
- Control the Lua version cadence. Upstream rebases are deliberate
  rather than automatic.
- Bundle the `wasm-sysroot/` with the crate instead of asking consumers
  to manage it separately.

Upstream attribution: the fork preserves the original MIT license (see
`crates/unison-lua/LICENSE`) and maintains a `NOTICE` file crediting
khvzak/lua-src-rs.

## Why mlua (and not raw mlua-sys or a hand-rolled binding)

`mlua` is ~15k lines of carefully-audited Rust that provides:

- Safe Rust ↔ Lua FFI with proper error wrapping
- The `UserData` trait — exposing Rust types to Lua with correct
  GC anchoring (the hard part; a bug here = silent memory corruption
  during Lua GC cycles)
- `IntoLua` / `FromLua` — automatic conversion for Rust values crossing
  the boundary
- Lifetime and refcount tracking that prevents Rust from dereferencing
  Lua-owned memory that Lua has freed

The `unison-scripting` crate depends on this heavily: 15 binding files
(engine, world, events, input, ui, physics, etc.) built on `UserData`,
`create_function`, and `IntoLua`/`FromLua`. Replacing mlua with raw
`mlua-sys` or a hand-rolled binding layer would be weeks of work with
real risk.

Crucially, **the interpreter-speed work discussed above lives *below*
the Lua C API line.** mlua sits *above* that line. Mlua talks to Lua
through `lua_pushnumber`, `lua_pcall`, etc. — the standard C API, stable
since 1993. We can modify the Lua interpreter as aggressively as we want
(bytecode dispatch, opcodes, GC, memory layout) and mlua continues to
work as long as we preserve the C API function signatures. The two layers
are orthogonal.

### When we'd reconsider

- **mlua's `Arc<Mutex<Lua>>` model conflicts with Unison's threading.**
  Today our scripting runs single-threaded per VM; if that changes, mlua
  may push back.
- **Wasm binary size becomes a measurable concern.** mlua pulls in
  features (async, userdata) whose code may be dead weight for some
  targets. Tree-shaking only goes so far.
- **We've accumulated enough engine-specific binding ergonomics** that
  mlua's generality is pure overhead.

None of these are acute. Revisit after the interpreter-speed work
completes and the friction surface is known.

[tstl]: https://typescripttolua.github.io/docs/getting-started/
[wasm-csp]: https://github.com/WebAssembly/content-security-policy/blob/main/proposals/CSP.md
[deegen]: https://sillycross.github.io/2022/11/22/2022-11-22/
