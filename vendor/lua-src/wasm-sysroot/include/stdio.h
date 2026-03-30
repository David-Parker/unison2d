#pragma once
#include <stddef.h>
#include <stdarg.h>

// Minimal stdio stubs for Lua WASM build.
// Real I/O is not used — Lua's print() is overridden by mlua.
// These stubs prevent missing-symbol linker errors for standard libs we load but don't call.

typedef struct _FILE FILE;

extern FILE *stdin;
extern FILE *stdout;
extern FILE *stderr;

#define EOF (-1)
#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

int    printf(const char *fmt, ...);
int    fprintf(FILE *f, const char *fmt, ...);
int    sprintf(char *buf, const char *fmt, ...);
int    snprintf(char *buf, size_t n, const char *fmt, ...);
int    vsnprintf(char *buf, size_t n, const char *fmt, va_list ap);
int    vsprintf(char *buf, const char *fmt, va_list ap);
int    vfprintf(FILE *f, const char *fmt, va_list ap);

int    fputs(const char *s, FILE *f);
int    fputc(int c, FILE *f);
int    fgets(char *s, int n, FILE *f);
int    fgetc(FILE *f);
int    ungetc(int c, FILE *f);
size_t fread(void *buf, size_t sz, size_t n, FILE *f);
size_t fwrite(const void *buf, size_t sz, size_t n, FILE *f);
int    fflush(FILE *f);
int    fclose(FILE *f);
FILE  *fopen(const char *path, const char *mode);
FILE  *freopen(const char *path, const char *mode, FILE *f);
int    fseek(FILE *f, long offset, int whence);
long   ftell(FILE *f);
void   rewind(FILE *f);
int    ferror(FILE *f);
int    feof(FILE *f);
void   clearerr(FILE *f);
FILE  *tmpfile(void);
int    remove(const char *path);
int    rename(const char *oldp, const char *newp);
int    sscanf(const char *str, const char *fmt, ...);
int    fscanf(FILE *f, const char *fmt, ...);
int    setvbuf(FILE *f, char *buf, int mode, size_t size);

#define BUFSIZ 8192
#define _IOFBF 0
#define _IOLBF 1
#define _IONBF 2

#define L_tmpnam 20
char  *tmpnam(char *s);

int    getc(FILE *f);
int    putc(int c, FILE *f);
int    getchar(void);
int    putchar(int c);
int    puts(const char *s);
