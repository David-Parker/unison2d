#pragma once
#include <stddef.h>

// mlua overrides Lua's allocator via lua_newstate, so malloc/free/realloc
// are never called directly from Lua core — but we need declarations for compilation.

void *malloc(size_t n);
void *calloc(size_t nmemb, size_t size);
void *realloc(void *ptr, size_t n);
void  free(void *ptr);

void  abort(void);
void  exit(int status);
int   atexit(void (*fn)(void));

long  strtol(const char *s, char **endptr, int base);
long long strtoll(const char *s, char **endptr, int base);
unsigned long strtoul(const char *s, char **endptr, int base);
unsigned long long strtoull(const char *s, char **endptr, int base);
double strtod(const char *s, char **endptr);
float  strtof(const char *s, char **endptr);

int   abs(int n);
long  labs(long n);
long long llabs(long long n);

char *getenv(const char *name);
int   system(const char *cmd);

void *bsearch(const void *key, const void *base, size_t n, size_t size, int (*cmp)(const void *, const void *));
void  qsort(void *base, size_t n, size_t size, int (*cmp)(const void *, const void *));

int   rand(void);
void  srand(unsigned int seed);

#define EXIT_SUCCESS 0
#define EXIT_FAILURE 1
#define RAND_MAX 2147483647
