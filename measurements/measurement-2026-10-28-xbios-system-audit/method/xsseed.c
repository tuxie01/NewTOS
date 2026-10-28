/*
 * xsseed.c -- probe for the audit docs/audits/2026-10-28-xbios-system.md:
 * where Random's first seed comes from.  EmuTOS seeds rseed on the first
 * call with (hz_200 << 16) + hz_200 (bios/xbios.c:421-426).  Started as the
 * first program after boot, this reads _hz_200 ($4BA, through Supexec) right
 * before and after the first Random() and asks whether one of the hz_200
 * values in between, as that seed, gives the value Random returned.
 * "no" means: seeded from something else, or Random was called before this
 * program (the line says which values were tried).  Same source for both
 * systems (build-xsprobe.sh).  Output: XSSEED.LOG.
 */
#include <osbind.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static volatile long sv_c;
static long hz200(void)
{
    sv_c = *(volatile long *)0x4BA;
    return sv_c;
}

int main(void)
{
    char out[256];
    long h1, h2, h, r;
    int hit = 0, n;
    long fh;

    h1 = Supexec(hz200);
    r = (long)Random();
    h2 = Supexec(hz200);
    for (h = h1; h <= h2; h++) {
        uint32_t s = ((uint32_t)h << 16) + (uint32_t)h;
        s = (uint32_t)(s * (uint32_t)3141592621UL + 1U);
        if (((s >> 8) & 0xFFFFFFU) == ((uint32_t)r & 0xFFFFFFU))
            hit = 1;
    }
    n = sprintf(out, "X01 first Random() matches a seed (hz_200<<16)+hz_200 for hz_200 in the window: %s\n"
                     "X01 info hz_200 window=%ld..%ld random=%06lx\n",
                hit ? "yes" : "no", h1, h2, (unsigned long)r & 0xFFFFFFUL);
    fh = Fcreate("XSSEED.LOG", 0);
    if (fh >= 0) {
        Fwrite((short)fh, (long)n, out);
        Fclose((short)fh);
    }
    return 0;
}
