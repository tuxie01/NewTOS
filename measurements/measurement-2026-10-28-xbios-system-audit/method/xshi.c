/*
 * xshi.c -- probe for the audit docs/audits/2026-10-28-xbios-system.md:
 * the high half of Gettime's LONG for a date whose packed value has bit 31
 * set (year offset >= 64, i.e. 2044..2107).  EmuTOS: LONG gettime(void)
 * (bios/clock.c:1266); the dumper's rule for the high half is the sign
 * extension of d0 (docs/TOS-REFERENCE-DUMPER.md section 3, ret_hi), which an
 * LP64 caller sees in x0.  Settime(2050-06-15 12:00:00), Gettime, then the
 * clock is put back (start value + elapsed _hz_200 time / 200, as in
 * xsprobe.c).  On the oracle Gettime reads Hatari's RTC, which takes no
 * write (audit section 6), so only the NewTOS run speaks for the date.
 * Output: XSHI.LOG.
 */
#include <osbind.h>
#include <stdio.h>

static volatile long sv_c;
static long hz200(void)
{
    sv_c = *(volatile long *)0x4BA;
    return sv_c;
}
static unsigned long lo32(long r) { return (unsigned long)r & 0xFFFFFFFFUL; }
static unsigned long hi32(long r)
{
    if (sizeof(long) > 4)
        return ((unsigned long)r >> 16 >> 16) & 0xFFFFFFFFUL;
    return r < 0 ? 0xFFFFFFFFUL : 0UL;
}

int main(void)
{
    char out[512];
    int n = 0;
    long fh, t0 = Supexec(hz200), g0, g, el;
    unsigned long v = ((unsigned long)((2050 - 1980) << 9 | 6 << 5 | 15) << 16) | (12UL << 11);
    unsigned d, t, s;

    g0 = (long)Gettime();
    Settime((long)v);
    g = (long)Gettime();
    n += sprintf(out + n, "H01 Settime(%08lx) Gettime ret=%08lx hi=%08lx negative-as-long=%s\n",
                 v, lo32(g), hi32(g), g < 0 ? "yes" : "no");
    /* restore: same-day seconds only (the probe takes a few seconds) */
    el = (Supexec(hz200) - t0) / 200;
    d = (unsigned)(lo32(g0) >> 16);
    t = (unsigned)lo32(g0) & 0xFFFFU;
    s = (t >> 11) * 3600 + ((t >> 5) & 63) * 60 + (t & 31) * 2 + (unsigned)el;
    if (s < 86400)
        t = (s / 3600) << 11 | (s / 60 % 60) << 5 | (s % 60) / 2;
    Settime((long)(((unsigned long)d << 16) | t));
    n += sprintf(out + n, "H02 info restored date=%04x time=%04x\n", d, t);
    fh = Fcreate("XSHI.LOG", 0);
    if (fh >= 0) {
        Fwrite((short)fh, (long)n, out);
        Fclose((short)fh);
    }
    return 0;
}
