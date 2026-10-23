/*
 * keyrep.c -- MEASUREMENT probe (tester, not a product program): what the console queue and graf_mkstate see for
 * held keys, taps, Shift+key and mouse buttons, with and without a MUTE. Same binary on the base kernel (b5c8da80)
 * and on the head kernel; the two logs are compared. Runs on qemu-virt as NEWDESK.NAP of a scratch kernel,
 * driven by qmp_keyrep.py. Output: "KR: KEY <hex> t=<tick>" for every key read, "KR: BTN <n> t=<tick>" for every
 * change of the graf_mkstate button state. Phases (markers "KR: P<n> ... now"):
 *   P1 idle 4 s            nothing is pressed: no event may appear
 *   P2 hold 'a' 3 s         auto-repeat
 *   P3 tap 'b', Shift+'c', left hold 1 s, right click
 *   P4 MUTE 300 ticks with NOTHING held (idle): no event may come out of the mute start or end
 *   P5 MUTE 300 ticks, 'z' and left button held across the mute start, released DURING the mute: after the mute no event
 *   P6 plain typing again after the mutes (tap 'd', hold 'e' 2 s)
 * Build: sdk/build.sh method/keyrep.c KEYREP.NAP
 */
#include <newtos.h>
typedef struct { short *control, *global, *intin, *intout; long *addrin, *addrout; } AESPB;
static short ctl[6], glb[16], iin[16], iout[16]; static long ain[4], aout[4];
static short aes(short op, short nin, short nout, short nai)
{
    AESPB pb; int i;
    ctl[0] = op; ctl[1] = nin; ctl[2] = nout; ctl[3] = nai; ctl[4] = 0;
    for (i = 0; i < 16; i++) iout[i] = 0;
    pb.control = ctl; pb.global = glb; pb.intin = iin; pb.intout = iout; pb.addrin = ain; pb.addrout = aout;
    { register long x0 __asm__("x0") = 200; register long x1 __asm__("x1") = (long)&pb;
      __asm__ __volatile__("svc\t#2" : "+r"(x0) : "r"(x1) : "memory", "cc"); }
    return iout[0];
}
static long g3(long fn, long a0, long a1, long a2)
{
    register long x8 __asm__("x8") = fn; register long r0 __asm__("x0") = a0;
    register long r1 __asm__("x1") = a1; register long r2 __asm__("x2") = a2;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2) : "memory");
    return r0;
}
static unsigned char b[64];
static unsigned rd32(int o) { return b[o] | b[o + 1] << 8 | b[o + 2] << 16 | (unsigned)b[o + 3] << 24; }
static long snap(unsigned op, unsigned arg, long len)
{
    int k; for (k = 0; k < 32; k++) b[k] = 0;
    b[0] = 'U'; b[1] = 'S'; b[2] = 'B'; b[3] = 'S'; b[4] = 1; b[8] = (unsigned char)op;
    b[16] = (unsigned char)arg; b[17] = (unsigned char)(arg >> 8);
    return g3(0x154, 2022, (long)b, len);
}
static unsigned now(void) { snap(1, 0, 32); return rd32(28); }
static char l[160]; static int n;
static void s(const char *p) { while (*p) l[n++] = *p++; }
static void d(long v) { char t[24]; int i = 0; unsigned long u = v < 0 ? -v : v; if (v < 0) l[n++] = '-';
    do { t[i++] = '0' + u % 10; u /= 10; } while (u); while (i) l[n++] = t[--i]; }
static void h(unsigned long v) { static const char x[] = "0123456789abcdef"; int i; s("0x"); for (i = 28; i >= 0; i -= 4) l[n++] = x[(v >> i) & 15]; }
static void out(void) { l[n++] = '\r'; l[n++] = '\n'; l[n] = 0; Cconws(l); n = 0; }
static short lastb;
static int keys, btns;
/* poll for `ticks`, logging every key and button change */
static void watch(unsigned ticks)
{
    unsigned t0 = now(), t;
    while ((t = now()) - t0 < ticks) {
        if (g3(0x0B, 0, 0, 0)) { long c = g3(0x08, 0, 0, 0); s("KR: KEY "); h((unsigned long)c); s(" t="); d(t - t0); out(); keys++; }
        aes(79, 0, 5, 0);
        if (iout[3] != lastb) { lastb = iout[3]; s("KR: BTN "); d(lastb); s(" t="); d(t - t0); out(); btns++; }
    }
}
static void phase(const char *p, unsigned ticks)
{
    int k0 = keys, b0 = btns;
    s("KR: "); s(p); s(" now"); out();
    watch(ticks);
    s("KR: "); s(p); s(" done keys "); d(keys - k0); s(" button changes "); d(btns - b0); out();
}
int main(void)
{
    aes(10, 0, 1, 0);
    s("KR: start now "); d(now()); out();
    phase("P1 idle", 800);
    phase("P2 hold-a", 1000);
    phase("P3 tap-shift-buttons", 1600);
    snap(4, 300, 64); s("KR: P4 mute-idle set"); out();
    phase("P4 mute-idle", 500);
    phase("P5a hold-z-and-left", 400);
    snap(4, 300, 64); s("KR: P5 mute-held set"); out();
    phase("P5 mute-held", 700);
    phase("P6 typing", 1400);
    s("KR: END"); out();
    for (;;) watch(100);
}
