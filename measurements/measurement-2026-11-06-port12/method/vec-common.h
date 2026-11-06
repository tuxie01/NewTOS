/* vec-common.h - PORT12 Q5 probe: one line per Fopen/Fwrite/Fclose, so a fault leaves the log so far. */
#include <osbind.h>
#include <mint/mintbind.h>
#include <mint/basepage.h>
#include <stdio.h>
#include <string.h>
#define COOKIE_XGTS 0x58475453L  /* 'XGTS': test only */
#ifndef S_SETCOOKIE
#define S_SETCOOKIE 9
#endif
#ifndef S_GETCOOKIE
#define S_GETCOOKIE 8
#endif
struct shared { long magic; long (*fn)(void); volatile long *count; char *priv; char *glob; };
static char lf_name[20];
static void lg(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
static void lg(const char *fmt, ...)
{
    static char b[240];
    __builtin_va_list ap;
    long h;
    int n;
    __builtin_va_start(ap, fmt); n = vsnprintf(b, sizeof b, fmt, ap); __builtin_va_end(ap);
    h = Fopen(lf_name, 1);
    if (h < 0) h = Fcreate(lf_name, 0);
    if (h < 0) return;
    Fseek(0, (short)h, 2);
    Fwrite((short)h, n, b);
    Fclose((short)h);
}
#define SETX(n, v) ((long)Setexc((n), (v)))
