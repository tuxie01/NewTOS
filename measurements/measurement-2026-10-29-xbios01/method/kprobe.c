/*
 * kprobe.c -- Bconin(2)/Bconstat(2) with real key input, for the audit
 * docs/audits/2026-10-28-bios-core.md.  Same source on both systems (see
 * bcprobe.c).  The harness feeds six key strokes after "KEYREADY" appears on
 * the console (EmuTOS: Hatari --cmd-fifo "hatari-event keydown/keyup <ST
 * scancode>"; NewTOS: tools/inject-arm.py IKBD bytes, launch:KPROBE):
 *   1 a   2 Shift+a   3 F1   4 Ctrl+c   5 Return   6 Shift+b
 * The probe waits (polling Bconstat(2), at most 6000 hz_200 ticks) until the
 * keys are queued and Shift is released, then reads keys 1-5 with the default
 * conterm, sets conterm bit 3 ($484, super state) and reads key 6: bits 24-31
 * = the shift state (EmuTOS: stored at the key stroke, bios/ikbd.c:279-280,900).
 * Output KPROBE.LOG, one line per step.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>

static char out[4000];
static int olen;
static void emit(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
    va_end(ap);
}
static volatile long hz, ct;
static long rd(void) { hz = *(volatile long *)0x4BA; ct = *(volatile unsigned char *)0x484; return 0; }
static long ct_on(void) { *(volatile unsigned char *)0x484 |= 8; return 0; }
static long ct_off(void) { *(volatile unsigned char *)0x484 &= ~8; return 0; }
static unsigned long lo32(long r) { return (unsigned long)r & 0xFFFFFFFFUL; }

int main(void)
{
    long t0, s, n = 0, i, k;
    long fh;
    Supexec(rd); t0 = hz;
    emit("K00 conterm=%02lx Bconstat(2)=%08lx Kbshift=%08lx\n", ct, lo32(Bconstat(2)), lo32(Kbshift(-1)));
    Cconws("KEYREADY\r\n");
    /* wait for the first key, then for the stream to finish (2 s of silence) */
    do { s = trap_13_ww((short)1, (short)2); Supexec(rd); n++; } while (s == 0 && hz - t0 < 6000);
    emit("K01 first Bconstat(2)!=0 after %ld ticks (%ld polls): ret=%08lx\n", hz - t0, n, lo32(s));
    t0 = hz; do { Supexec(rd); } while (hz - t0 < 400);
    emit("K02 Kbshift(-1) before reading=%08lx\n", lo32(Kbshift(-1)));
    for (i = 1; i <= 5; i++) {
        s = trap_13_ww((short)1, (short)2);
        if (s == 0) { emit("K1%ld no key queued (Bconstat 0)\n", i); continue; }
        k = trap_13_ww((short)2, (short)2);
        emit("K1%ld Bconstat=%08lx Bconin(2)=%08lx\n", i, lo32(s), lo32(k));
    }
    Supexec(ct_on);
    s = trap_13_ww((short)1, (short)2);
    if (s == 0) emit("K16 no key queued (Bconstat 0)\n");
    else { k = trap_13_ww((short)2, (short)2); emit("K16 conterm|8 Bconstat=%08lx Bconin(2)=%08lx\n", lo32(s), lo32(k)); }
    Supexec(ct_off);
    emit("K20 after: Bconstat(2)=%08lx\n", lo32(trap_13_ww((short)1, (short)2)));
    fh = Fcreate("KPROBE.LOG", 0);
    if (fh >= 0) { Fwrite((short)fh, (long)olen, out); Fclose((short)fh); }
    Cconws("KPROBE done\r\n");
    return 0;
}
