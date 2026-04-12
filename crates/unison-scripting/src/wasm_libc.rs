//! Stub implementations of C stdlib symbols for WASM builds.
//!
//! The Lua 5.4 C library references many libc symbols that aren't provided by
//! the `wasm32-unknown-unknown` target.  This module provides `#[no_mangle]`
//! implementations so the linker can resolve all "env" imports internally,
//! leaving the final WASM binary with zero unexpected host imports.
//!
//! All types match the exact WASM function signatures confirmed by inspecting
//! the import section of a real `wasm32-unknown-unknown` cdylib that embeds
//! Lua via `mlua`.  Where a correct implementation is required for Lua to
//! function (string ops, char classification, frexp, strtod …) a proper
//! implementation is provided.  Pure I/O, file, locale, and time functions
//! return safe sentinel values (0 / null) because Lua only calls them in code
//! paths that are never reached from WASM game scripts.
//!
//! This module is compiled only under `target_arch = "wasm32"` and its symbols
//! are linked into the final cdylib via standard rlib static-archive semantics:
//! the Lua C static library references these symbols during link, which pulls
//! in the enclosing object files from this rlib.  The module is not part of
//! `unison-scripting`'s public Rust API — it exists purely to satisfy the
//! linker.

#![allow(unused_variables, unused_mut, dead_code, non_camel_case_types, static_mut_refs)]

use core::ffi::c_void;

#[cfg(feature = "web")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "web")]
use wasm_bindgen::JsCast;
#[cfg(feature = "web")]
use web_sys::js_sys;

// ---------------------------------------------------------------------------
// Type aliases that make signatures more readable
// ---------------------------------------------------------------------------

type CInt = i32;
type CDouble = f64;
type CPtr = i32; // All pointers are i32 in WASM32

// ---------------------------------------------------------------------------
// Helper: read a null-terminated C string from WASM linear memory.
// Safety: The caller must guarantee `ptr` points to a valid C string.
// ---------------------------------------------------------------------------

unsafe fn cstr_len(ptr: *const u8) -> usize {
    let mut n = 0usize;
    while *ptr.add(n) != 0 {
        n += 1;
    }
    n
}

unsafe fn cstr_bytes(ptr: *const u8) -> &'static [u8] {
    core::slice::from_raw_parts(ptr, cstr_len(ptr))
}

// ============================================================================
// Assertion / abort
// ============================================================================

/// Lua calls `__assert_fail` when an internal assertion fires.
/// Signature (wasm): (i32, i32, i32, i32) -> ()
#[no_mangle]
pub unsafe extern "C" fn __assert_fail(
    expr: *const u8,
    file: *const u8,
    line: u32,
    func: *const u8,
) -> ! {
    panic!("assertion failed (wasm libc stub)");
}

/// Signature (wasm): () -> ()
#[no_mangle]
pub unsafe extern "C" fn abort() -> ! {
    panic!("abort() called in WASM");
}

/// Signature (wasm): (i32) -> ()
#[no_mangle]
pub unsafe extern "C" fn exit(code: i32) -> ! {
    panic!("exit({}) called in WASM", code);
}

// ============================================================================
// Memory
// ============================================================================

/// free — Lua uses its own allocator hook, so the C-level free is a no-op.
/// Signature (wasm): (i32) -> ()
#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    // mlua overrides Lua's allocator so Lua never calls the system free.
    // If somehow called, just do nothing.
}

/// realloc — return `ptr` unchanged (safe no-op; see note on free above).
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    ptr
}

// ============================================================================
// String functions — correct implementations required
// ============================================================================

/// strcmp — lexicographic comparison of two C strings.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strcmp(s1: *const u8, s2: *const u8) -> i32 {
    let mut i = 0usize;
    loop {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return (a as i32) - (b as i32);
        }
        if a == 0 {
            return 0;
        }
        i += 1;
    }
}

/// strcoll — locale-unaware, same as strcmp.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strcoll(s1: *const u8, s2: *const u8) -> i32 {
    strcmp(s1, s2)
}

/// strcpy — copy src into dst (including NUL terminator), return dst.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strcpy(dst: *mut u8, src: *const u8) -> *mut u8 {
    let mut i = 0usize;
    loop {
        let b = *src.add(i);
        *dst.add(i) = b;
        if b == 0 {
            break;
        }
        i += 1;
    }
    dst
}

/// strchr — find the first occurrence of `c` in `s`, return ptr or NULL.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strchr(s: *const u8, c: i32) -> *const u8 {
    let ch = (c & 0xFF) as u8;
    let mut i = 0usize;
    loop {
        let b = *s.add(i);
        if b == ch {
            return s.add(i);
        }
        if b == 0 {
            return core::ptr::null();
        }
        i += 1;
    }
}

/// strstr — find the first occurrence of `needle` in `haystack`.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strstr(haystack: *const u8, needle: *const u8) -> *const u8 {
    if *needle == 0 {
        return haystack;
    }
    let nlen = cstr_len(needle);
    let hlen = cstr_len(haystack);
    if nlen > hlen {
        return core::ptr::null();
    }
    let limit = hlen - nlen;
    let mut i = 0usize;
    loop {
        if i > limit {
            return core::ptr::null();
        }
        if core::slice::from_raw_parts(haystack.add(i), nlen)
            == core::slice::from_raw_parts(needle, nlen)
        {
            return haystack.add(i);
        }
        i += 1;
    }
}

/// strspn — length of leading segment of `s` consisting of chars in `accept`.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strspn(s: *const u8, accept: *const u8) -> i32 {
    let accept_bytes = cstr_bytes(accept);
    let mut n = 0usize;
    loop {
        let b = *s.add(n);
        if b == 0 || !accept_bytes.contains(&b) {
            return n as i32;
        }
        n += 1;
    }
}

/// strpbrk — find the first character in `s` that is in `accept`.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strpbrk(s: *const u8, accept: *const u8) -> *const u8 {
    let accept_bytes = cstr_bytes(accept);
    let mut i = 0usize;
    loop {
        let b = *s.add(i);
        if b == 0 {
            return core::ptr::null();
        }
        if accept_bytes.contains(&b) {
            return s.add(i);
        }
        i += 1;
    }
}

/// memchr — find byte `c` in the first `n` bytes of `s`.
/// Signature (wasm): (i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn memchr(s: *const u8, c: i32, n: usize) -> *const u8 {
    let ch = (c & 0xFF) as u8;
    for i in 0..n {
        if *s.add(i) == ch {
            return s.add(i);
        }
    }
    core::ptr::null()
}

/// strerror — return a pointer to a static error string.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strerror(errnum: i32) -> *const u8 {
    b"error\0".as_ptr()
}

// ============================================================================
// Character classification — correct ASCII-range implementations
// ============================================================================

#[no_mangle]
pub unsafe extern "C" fn isalnum(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_alphanumeric() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn isalpha(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_alphabetic() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn iscntrl(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_control() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn isgraph(c: i32) -> i32 {
    let c = c as u8;
    // printable and not space
    if c > 0x20 && c < 0x7F { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn islower(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_lowercase() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn ispunct(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_punctuation() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn isspace(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_whitespace() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn isupper(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_uppercase() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn isxdigit(c: i32) -> i32 {
    let c = c as u8;
    if c.is_ascii_hexdigit() { 1 } else { 0 }
}

#[no_mangle]
pub unsafe extern "C" fn tolower(c: i32) -> i32 {
    (c as u8).to_ascii_lowercase() as i32
}

#[no_mangle]
pub unsafe extern "C" fn toupper(c: i32) -> i32 {
    (c as u8).to_ascii_uppercase() as i32
}

// ============================================================================
// Math
// ============================================================================

/// frexp — split a float into mantissa in [0.5, 1.0) and a base-2 exponent.
/// Signature (wasm): (f64, i32) -> f64
#[no_mangle]
pub unsafe extern "C" fn frexp(x: f64, exp_ptr: *mut i32) -> f64 {
    if x == 0.0 || x.is_nan() || x.is_infinite() {
        if !exp_ptr.is_null() {
            *exp_ptr = 0;
        }
        return x;
    }
    // Use the bit representation to extract the exponent.
    let bits = x.to_bits();
    let biased_exp = ((bits >> 52) & 0x7FF) as i32;
    if biased_exp == 0 {
        // Subnormal: normalise first.
        let normalised = x * (f64::from_bits(0x4350000000000000u64)); // * 2^54
        let nbits = normalised.to_bits();
        let n_exp = ((nbits >> 52) & 0x7FF) as i32 - 54;
        let exp = n_exp - 1022;
        if !exp_ptr.is_null() {
            *exp_ptr = exp;
        }
        let mantissa_bits = (nbits & 0x000FFFFFFFFFFFFF) | 0x3FE0000000000000;
        return f64::from_bits(mantissa_bits).copysign(x);
    }
    let exp = biased_exp - 1022;
    if !exp_ptr.is_null() {
        *exp_ptr = exp;
    }
    // Set exponent field to 1022 (biased) → mantissa in [0.5, 1.0).
    let mantissa_bits = (bits & 0x800FFFFFFFFFFFFF) | 0x3FE0000000000000;
    f64::from_bits(mantissa_bits)
}

/// strtod — parse a decimal (or hex) floating-point number from a C string.
/// Signature (wasm): (i32, i32) -> f64
///
/// This is a reasonably complete implementation that handles the formats Lua
/// generates and parses: decimal integers, decimal floats with optional
/// exponent, and `0x`/`0X` hex floats.  It does not handle every edge case
/// mandated by C99, but it is correct for all values Lua itself produces.
#[no_mangle]
pub unsafe extern "C" fn strtod(nptr: *const u8, endptr: *mut *const u8) -> f64 {
    let mut p = nptr;

    // Skip leading whitespace.
    while isspace(*p as i32) != 0 {
        p = p.add(1);
    }

    let start = p;
    let mut sign = 1.0f64;

    if *p == b'+' {
        p = p.add(1);
    } else if *p == b'-' {
        sign = -1.0;
        p = p.add(1);
    }

    // "inf" / "infinity" / "nan" (case-insensitive)
    {
        let upper = |b: u8| b.to_ascii_uppercase();
        if upper(*p) == b'I' && upper(*p.add(1)) == b'N' && upper(*p.add(2)) == b'F' {
            p = p.add(3);
            if upper(*p) == b'I' {
                p = p.add(5); // "INITY"
            }
            if !endptr.is_null() { *endptr = p; }
            return sign * f64::INFINITY;
        }
        if upper(*p) == b'N' && upper(*p.add(1)) == b'A' && upper(*p.add(2)) == b'N' {
            p = p.add(3);
            if !endptr.is_null() { *endptr = p; }
            return f64::NAN;
        }
    }

    let result: f64;

    if *p == b'0' && ((*p.add(1)) | 0x20) == b'x' {
        // Hex float: 0x[digits][.digits][p[+-]digits]
        p = p.add(2);
        let mut int_part = 0u64;
        let mut frac_part = 0u64;
        let mut frac_digits = 0i32;
        let mut has_digits = false;

        while let Some(d) = hex_digit(*p) {
            int_part = int_part.wrapping_mul(16).wrapping_add(d as u64);
            has_digits = true;
            p = p.add(1);
        }
        if *p == b'.' {
            p = p.add(1);
            while let Some(d) = hex_digit(*p) {
                frac_part = frac_part.wrapping_mul(16).wrapping_add(d as u64);
                frac_digits += 1;
                has_digits = true;
                p = p.add(1);
            }
        }
        if !has_digits {
            if !endptr.is_null() { *endptr = start; }
            return 0.0;
        }
        let mut val = int_part as f64 + frac_part as f64 / 16f64.powi(frac_digits);
        if (*p | 0x20) == b'p' {
            p = p.add(1);
            let (exp, new_p) = parse_int_digits(p);
            p = new_p;
            val *= (2.0f64).powi(exp);
        }
        result = sign * val;
    } else {
        // Decimal float
        let mut int_val = 0u64;
        let mut frac_val = 0u64;
        let mut frac_digits = 0i32;
        let mut has_digits = false;

        while (*p).is_ascii_digit() {
            int_val = int_val.wrapping_mul(10).wrapping_add((*p - b'0') as u64);
            has_digits = true;
            p = p.add(1);
        }
        if *p == b'.' {
            p = p.add(1);
            while (*p).is_ascii_digit() {
                frac_val = frac_val.wrapping_mul(10).wrapping_add((*p - b'0') as u64);
                frac_digits += 1;
                has_digits = true;
                p = p.add(1);
            }
        }
        if !has_digits {
            if !endptr.is_null() { *endptr = start; }
            return 0.0;
        }
        let mut val = int_val as f64 + frac_val as f64 / 10f64.powi(frac_digits);
        if (*p | 0x20) == b'e' {
            p = p.add(1);
            let (exp, new_p) = parse_int_digits(p);
            p = new_p;
            val *= 10f64.powi(exp);
        }
        result = sign * val;
    }

    if !endptr.is_null() {
        *endptr = p;
    }
    result
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Parse an optional sign then decimal digits into an i32.
/// Returns `(value, new_ptr)`.
unsafe fn parse_int_digits(mut p: *const u8) -> (i32, *const u8) {
    let mut sign = 1i32;
    if *p == b'+' {
        p = p.add(1);
    } else if *p == b'-' {
        sign = -1;
        p = p.add(1);
    }
    let mut val = 0i32;
    while (*p).is_ascii_digit() {
        val = val.wrapping_mul(10).wrapping_add((*p - b'0') as i32);
        p = p.add(1);
    }
    (sign * val, p)
}

// ============================================================================
// setjmp / longjmp
//
// With -mllvm -wasm-enable-sjlj in the C compiler flags, clang transforms
// setjmp/longjmp calls in the Lua C source into WASM exception handling
// instructions at compile time.  The Rust stubs below are safety nets —
// they should NOT be called by the compiled Lua code (clang inlines the
// sjlj transform), but other C code that references these symbols will
// link against them.
// ============================================================================

/// setjmp — fallback stub.  Returns 0 (first-call semantics).
#[no_mangle]
pub unsafe extern "C" fn setjmp(_env: *mut u8) -> i32 {
    0
}

/// longjmp — fallback stub.  Should not be reached for Lua error handling
/// (clang's sjlj transform handles it), but panics if called unexpectedly.
#[no_mangle]
pub unsafe extern "C" fn longjmp(_env: *mut u8, _val: i32) -> ! {
    panic!("longjmp called — clang sjlj transform should handle this");
}

// ============================================================================
// I/O (stdio) — stubs that return 0 / NULL
// ============================================================================

/// fopen — always return NULL (file system not available).
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fopen(path: *const u8, mode: *const u8) -> *mut c_void {
    core::ptr::null_mut()
}

/// freopen — always return NULL.
/// Signature (wasm): (i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn freopen(
    path: *const u8,
    mode: *const u8,
    stream: *mut c_void,
) -> *mut c_void {
    core::ptr::null_mut()
}

/// fclose — return 0 (EOF constant in C is -1, success is 0).
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fclose(stream: *mut c_void) -> i32 {
    0
}

/// fflush — return 0.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fflush(stream: *mut c_void) -> i32 {
    0
}

/// fread — return 0 (nothing read).
/// Signature (wasm): (i32, i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fread(
    ptr: *mut c_void,
    size: usize,
    nmemb: usize,
    stream: *mut c_void,
) -> usize {
    0
}

/// fwrite — pretend all bytes were written.
/// Signature (wasm): (i32, i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fwrite(
    ptr: *const c_void,
    size: usize,
    nmemb: usize,
    stream: *mut c_void,
) -> usize {
    nmemb
}

/// fgets — return NULL.
/// Signature (wasm): (i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fgets(
    s: *mut u8,
    n: i32,
    stream: *mut c_void,
) -> *mut u8 {
    core::ptr::null_mut()
}

/// fputc — return the character unchanged (success).
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fputc(c: i32, stream: *mut c_void) -> i32 {
    c
}

/// fputs — return 0.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fputs(s: *const u8, stream: *mut c_void) -> i32 {
    0
}

/// fprintf — write nothing, return 0.
/// Signature (wasm): (i32, i32, i32) -> i32
///
/// The third argument is a pointer to the varargs shadow-stack area; we just
/// ignore all arguments and return 0.
#[no_mangle]
pub unsafe extern "C" fn fprintf(
    stream: *mut c_void,
    fmt: *const u8,
    args: *const c_void,
) -> i32 {
    0
}

/// snprintf — write nothing (or just NUL-terminate), return 0.
/// Signature (wasm): (i32, i32, i32, i32) -> i32
///
/// All four params are i32.  The fourth is the varargs shadow-stack pointer.
/// For now we just NUL-terminate the output buffer and return 0; this is
/// sufficient for the subset of snprintf calls Lua makes during script load
/// (pattern error messages etc. are only shown if Lua can print, which it
/// can't in this stub environment).
#[no_mangle]
pub unsafe extern "C" fn snprintf(
    buf: *mut u8,
    size: usize,
    fmt: *const u8,
    args: *const c_void,
) -> i32 {
    if !buf.is_null() && size > 0 {
        *buf = 0;
    }
    0
}

/// getc — return EOF (-1).
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn getc(stream: *mut c_void) -> i32 {
    -1 // EOF
}

/// ungetc — return EOF.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn ungetc(c: i32, stream: *mut c_void) -> i32 {
    -1 // EOF
}

/// feof — always return non-zero (stream is at EOF / invalid).
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn feof(stream: *mut c_void) -> i32 {
    1
}

/// ferror — always return non-zero.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn ferror(stream: *mut c_void) -> i32 {
    1
}

/// clearerr — no-op.
/// Signature (wasm): (i32) -> ()
#[no_mangle]
pub unsafe extern "C" fn clearerr(stream: *mut c_void) {}

/// setvbuf — return 0.
/// Signature (wasm): (i32, i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn setvbuf(
    stream: *mut c_void,
    buf: *mut u8,
    mode: i32,
    size: usize,
) -> i32 {
    0
}

/// fseek — return -1 (error).
/// Signature (wasm): (i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn fseek(stream: *mut c_void, offset: i32, whence: i32) -> i32 {
    -1
}

/// ftell — return -1.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn ftell(stream: *mut c_void) -> i32 {
    -1
}

/// tmpfile — return NULL.
/// Signature (wasm): () -> i32
#[no_mangle]
pub unsafe extern "C" fn tmpfile() -> *mut c_void {
    core::ptr::null_mut()
}

/// tmpnam — return NULL.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn tmpnam(s: *mut u8) -> *const u8 {
    core::ptr::null()
}

/// remove — return -1.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn remove(path: *const u8) -> i32 {
    -1
}

/// rename — return -1.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn rename(old: *const u8, new: *const u8) -> i32 {
    -1
}

// ============================================================================
// Environment
// ============================================================================

/// getenv — always return NULL.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn getenv(name: *const u8) -> *const u8 {
    core::ptr::null()
}

/// system — return -1 (no shell available).
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn system(command: *const u8) -> i32 {
    -1
}

// ============================================================================
// Time
// ============================================================================

/// time — return 0.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn time(t: *mut i32) -> i32 {
    if !t.is_null() {
        *t = 0;
    }
    0
}

/// clock — return 0.
/// Signature (wasm): () -> i32
#[no_mangle]
pub unsafe extern "C" fn clock() -> i32 {
    0
}

/// difftime — return (b - a) as f64.
/// Signature (wasm): (i32, i32) -> f64
#[no_mangle]
pub unsafe extern "C" fn difftime(time1: i32, time0: i32) -> f64 {
    (time1 - time0) as f64
}

/// mktime — return -1.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn mktime(tm: *mut c_void) -> i32 {
    -1
}

/// gmtime — return NULL.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn gmtime(timer: *const i32) -> *const c_void {
    core::ptr::null()
}

/// localtime — return NULL.
/// Signature (wasm): (i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn localtime(timer: *const i32) -> *const c_void {
    core::ptr::null()
}

/// strftime — write nothing, return 0.
/// Signature (wasm): (i32, i32, i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn strftime(
    s: *mut u8,
    max: usize,
    fmt: *const u8,
    tm: *const c_void,
) -> usize {
    0
}

// ============================================================================
// Locale
// ============================================================================

/// setlocale — return a pointer to the static "C" locale name.
/// Signature (wasm): (i32, i32) -> i32
#[no_mangle]
pub unsafe extern "C" fn setlocale(category: i32, locale: *const u8) -> *const u8 {
    b"C\0".as_ptr()
}

/// The C `lconv` struct layout (only the fields Lua actually reads).
///
/// Raw-pointer fields prevent this from implementing `Sync`, so we cannot
/// use a plain `static`.  Instead we store the data as a `static mut [u8]`
/// (a plain byte array, always `Sync`) and cast to `*const Lconv` at the
/// call site.  The byte array is initialised lazily on first call.
///
/// Layout (wasm32, 4-byte pointers):
///   offset  0: decimal_point      *const u8  → points to ".\0"
///   offset  4: thousands_sep      *const u8  → points to "\0"
///   offset  8: grouping           *const u8  → points to "\0"
///   offset 12: int_curr_symbol    *const u8  → points to "\0"
///   offset 16: currency_symbol    *const u8  → points to "\0"
///   offset 20: mon_decimal_point  *const u8  → points to "\0"
///   offset 24: mon_thousands_sep  *const u8  → points to "\0"
///   offset 28: mon_grouping       *const u8  → points to "\0"
///   offset 32: positive_sign      *const u8  → points to "\0"
///   offset 36: negative_sign      *const u8  → points to "\0"
///   offset 40..53: char fields    u8         → all 127 (CHAR_MAX)
#[allow(static_mut_refs)]
static mut LCONV_BUF: [u8; 56] = [0u8; 56];
static mut LCONV_INIT: bool = false;

static DECIMAL_POINT: &[u8] = b".\0";
static EMPTY_STR: &[u8] = b"\0";

unsafe fn get_lconv() -> *const u8 {
    if !LCONV_INIT {
        // Write pointer fields (4 bytes each at known offsets).
        let write_ptr = |offset: usize, ptr: *const u8| {
            let addr = ptr as u32;
            LCONV_BUF[offset]   = (addr & 0xFF) as u8;
            LCONV_BUF[offset+1] = ((addr >> 8)  & 0xFF) as u8;
            LCONV_BUF[offset+2] = ((addr >> 16) & 0xFF) as u8;
            LCONV_BUF[offset+3] = ((addr >> 24) & 0xFF) as u8;
        };
        let dp = DECIMAL_POINT.as_ptr();
        let ep = EMPTY_STR.as_ptr();
        write_ptr( 0, dp); // decimal_point
        write_ptr( 4, ep); // thousands_sep
        write_ptr( 8, ep); // grouping
        write_ptr(12, ep); // int_curr_symbol
        write_ptr(16, ep); // currency_symbol
        write_ptr(20, ep); // mon_decimal_point
        write_ptr(24, ep); // mon_thousands_sep
        write_ptr(28, ep); // mon_grouping
        write_ptr(32, ep); // positive_sign
        write_ptr(36, ep); // negative_sign
        // char fields at offsets 40..53 — set to 127 (CHAR_MAX)
        for i in 40..54 {
            LCONV_BUF[i] = 127;
        }
        LCONV_INIT = true;
    }
    LCONV_BUF.as_ptr()
}

/// localeconv — return a pointer to the static lconv struct.
/// Signature (wasm): () -> i32
#[no_mangle]
pub unsafe extern "C" fn localeconv() -> *const u8 {
    get_lconv()
}

// ============================================================================
// Lua WASM error handling — JS exception bridge
// ============================================================================
//
// In wasm32-unknown-unknown, setjmp/longjmp cannot save/restore the native
// call stack.  Instead, ldo.c is patched to call these two extern "C"
// functions:
//
//   wasm_lua_throw   — throws a JS exception (via wasm_bindgen::throw_str)
//   wasm_protected_call — invokes f(L, ud) through a JS wrapper whose
//                         try/catch converts the thrown exception into a
//                         return value.
//
// js_sys::Function::call3 has #[wasm_bindgen(catch)], so it wraps the JS
// call in try { ... } catch (e) { return Err(e); }.  When wasm_lua_throw
// fires throw_str, the exception propagates through WASM frames until it
// hits the JS boundary where call3's try/catch catches it.

/// The JS Function wrapper that invokes a C function pointer.  Created once
/// by `init_lua_error_handler()` and reused for every protected call.
#[cfg(feature = "web")]
static mut INVOKER: Option<js_sys::Function> = None;

/// Must be called once at startup (before any Lua code runs) to create the
/// JS invoker closure.
#[cfg(feature = "web")]
pub fn init_lua_error_handler() {
    let closure = Closure::<dyn Fn(u32, u32, u32)>::new(move |f: u32, l: u32, ud: u32| {
        unsafe {
            type Pfunc = unsafe extern "C" fn(*mut c_void, *mut c_void);
            let func: Pfunc = core::mem::transmute(f as usize);
            func(l as *mut c_void, ud as *mut c_void);
        }
    });
    let js_fn: js_sys::Function = closure.as_ref().unchecked_ref::<js_sys::Function>().clone();
    closure.forget(); // leak — lives for the entire program
    unsafe { INVOKER = Some(js_fn); }
}

/// Called from patched ldo.c `luaD_throw` on WASM.  Throws a JS exception
/// that will be caught by `wasm_protected_call`'s JS try/catch boundary.
#[cfg(feature = "web")]
#[no_mangle]
pub unsafe extern "C" fn wasm_lua_throw() -> ! {
    wasm_bindgen::throw_str("__lua_error__")
}

/// Called from patched ldo.c `luaD_rawrunprotected` on WASM.  Invokes
/// f(L, ud) through a JS function wrapper so that if wasm_lua_throw fires,
/// the JS try/catch in call3 catches the exception and returns Err.
#[cfg(feature = "web")]
#[no_mangle]
pub unsafe extern "C" fn wasm_protected_call(
    f: unsafe extern "C" fn(*mut c_void, *mut c_void),
    l: *mut c_void,
    ud: *mut c_void,
) -> i32 {
    let invoker = INVOKER.as_ref().expect("lua error handler not initialized — call init_lua_error_handler() first");
    let result = invoker.call3(
        &JsValue::NULL,
        &JsValue::from(f as u32),
        &JsValue::from(l as u32),
        &JsValue::from(ud as u32),
    );
    match result {
        Ok(_) => 0,  // no error
        Err(_) => 1, // error was thrown — lj.status already set by luaD_throw
    }
}

// Non-web fallbacks so the symbols exist for any wasm32 build.
#[cfg(not(feature = "web"))]
#[no_mangle]
pub unsafe extern "C" fn wasm_lua_throw() -> ! {
    panic!("wasm_lua_throw: no JS environment available")
}

#[cfg(not(feature = "web"))]
#[no_mangle]
pub unsafe extern "C" fn wasm_protected_call(
    _f: unsafe extern "C" fn(*mut c_void, *mut c_void),
    _l: *mut c_void,
    _ud: *mut c_void,
) -> i32 {
    panic!("wasm_protected_call: no JS environment available")
}
