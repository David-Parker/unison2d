#pragma once

// WASM stub — only sig_atomic_t is used by Lua (for l_signalT in lstate.h).
// Signals are not available in wasm32-unknown-unknown.
typedef int sig_atomic_t;
