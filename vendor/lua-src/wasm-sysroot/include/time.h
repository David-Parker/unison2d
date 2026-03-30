#pragma once
#include <stddef.h>

typedef long time_t;
typedef long clock_t;

struct tm {
    int tm_sec;
    int tm_min;
    int tm_hour;
    int tm_mday;
    int tm_mon;
    int tm_year;
    int tm_wday;
    int tm_yday;
    int tm_isdst;
};

time_t time(time_t *t);
clock_t clock(void);
struct tm *localtime(const time_t *t);
struct tm *gmtime(const time_t *t);
char *asctime(const struct tm *tm);
char *ctime(const time_t *t);
size_t strftime(char *buf, size_t max, const char *fmt, const struct tm *tm);
double difftime(time_t t1, time_t t0);
time_t mktime(struct tm *tm);

#define CLOCKS_PER_SEC 1000000
