/* xos_newtos.c - XGDOS OS layer (upstream src/xos.h) on the NewTOS GEMDOS, through
 * the SDK's mintlib bindings. Replaces upstream src/xos_tos.c + src/tos.h + src/traps.S,
 * which push m68k argument blocks and read the DTA big-endian. PORT12 stage 1.
 * SPDX-License-Identifier: GPL-2.0-only (derived from upstream src/xos_tos.c,
 * Copyright (c) 2026 Francis C Bullen). */
#include <osbind.h>
#include <mint/ostruct.h>
#include "xos.h"
#include "xstd.h"

s32 xos_open(const char *p, int mode) { return (s32)Fopen(p, mode); }
s32 xos_create(const char *p) { return (s32)Fcreate(p, 0); }
s32 xos_read(s32 h, s32 n, void *b) { return (s32)Fread((short)h, n, b); }
s32 xos_write(s32 h, s32 n, const void *b) { return (s32)Fwrite((short)h, n, b); }
s32 xos_seek(s32 h, s32 off, int wh) { return (s32)Fseek(off, (short)h, wh); }
void xos_close(s32 h) { if (h >= 0) Fclose((short)h); }
s32 xos_delete(const char *p) { return (s32)Fdelete(p); }

s32 xos_filesize(const char *p)
{
    s32 h = (s32)Fopen(p, 0), n;
    if (h < 0) return h;
    n = (s32)Fseek(0, (short)h, 2);
    Fclose((short)h);
    return n;
}

static void fill(XosFind *f)
{
    const _DTA *d = (const _DTA *)f->dta;
    xstrncpy0(f->name, d->dta_name, 14);
    f->size = d->dta_size;
    f->attr = (u8)d->dta_attribute;
}

int xos_findfirst(XosFind *f, const char *pattern)
{
    f->olddta = Fgetdta();
    Fsetdta((_DTA *)f->dta);
    if (Fsfirst(pattern, 0) != 0) return 0;
    fill(f);
    return 1;
}

int xos_findnext(XosFind *f)
{
    Fsetdta((_DTA *)f->dta);
    if (Fsnext() != 0) return 0;
    fill(f);
    return 1;
}

void xos_findend(XosFind *f)
{
    if (f->olddta) Fsetdta(f->olddta);
    f->olddta = NULL;
}

void *xos_alloc(s32 n)
{
    long p = Mxalloc(n, 3);
    if (p == -32 || p == 0) p = Malloc(n);
    return p > 0 ? (void *)p : 0;
}
void xos_free(void *p) { if (p) Mfree(p); }

void xos_getcwd(char *out, int max)
{
    char b[256];
    long d = Dgetdrv();
    int n;
    b[0] = 0;
    if (Dgetpath(b, 0) < 0) b[0] = 0;
    out[0] = (char)('A' + (d & 31)); out[1] = ':'; out[2] = 0;
    if (b[0] != '\\') xstrcat(out, "\\");
    n = (int)xstrlen(out);
    xstrncpy0(out + n, b, max - n - 1);
    n = (int)xstrlen(out);
    if (n == 0 || out[n - 1] != '\\') { out[n] = '\\'; out[n + 1] = 0; }
}

/* one process: no MiNT behaviour in the stage 1 test programs */
void xos_yield(void) {}
s32 xos_getpid(void) { return 0; }
int xos_pid_alive(s32 pid) { (void)pid; return 1; }

static long bootdev_s(void) { return *(volatile short *)0x446L; }
int xos_bootdrive(void) { return (int)Supexec(bootdev_s) & 31; }

int xos_screen_dev(void)
{
    long r = Getrez();
    return (r >= 0 && r < 8) ? (int)r + 2 : 1;
}

void xos_log(const char *s) { (void)s; }
