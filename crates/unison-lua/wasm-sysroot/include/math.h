#pragma once

// Math functions — available as LLVM intrinsics for wasm32
double sin(double x);
double cos(double x);
double tan(double x);
double sinh(double x);
double cosh(double x);
double tanh(double x);
double asin(double x);
double acos(double x);
double atan(double x);
double atan2(double y, double x);
double exp(double x);
double log(double x);
double log2(double x);
double log10(double x);
double pow(double x, double y);
double sqrt(double x);
double cbrt(double x);
double ceil(double x);
double floor(double x);
double fmod(double x, double y);
double fabs(double x);
double modf(double x, double *iptr);
double frexp(double x, int *exp);
double ldexp(double x, int exp);
double hypot(double x, double y);
double trunc(double x);
double round(double x);
double nearbyint(double x);
float  sqrtf(float x);
float  fabsf(float x);
float  floorf(float x);
float  ceilf(float x);
float  roundf(float x);
float  truncf(float x);
double huge_val(void);

#define HUGE_VAL __builtin_huge_val()
#define HUGE_VALF __builtin_huge_valf()
#define NAN __builtin_nanf("")
#define INFINITY __builtin_inff()
#define M_PI 3.14159265358979323846
