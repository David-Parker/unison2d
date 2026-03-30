#pragma once

// setjmp/longjmp for wasm32 via Clang's built-in exception handling.
// LUA_USE_LONGJMP is defined, so Lua uses these for error recovery.

typedef struct {
    // opaque state — size chosen to be safe on wasm32
    unsigned char _buf[200];
} jmp_buf[1];

int  setjmp(jmp_buf env);
void longjmp(jmp_buf env, int val);
