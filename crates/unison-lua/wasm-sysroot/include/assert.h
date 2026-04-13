#pragma once

#ifdef NDEBUG
#define assert(x) ((void)(x))
#else
void __assert_fail(const char *expr, const char *file, int line, const char *func);
#define assert(x) ((x) ? (void)0 : __assert_fail(#x, __FILE__, __LINE__, __func__))
#endif
