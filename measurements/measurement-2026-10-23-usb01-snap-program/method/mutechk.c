/*
 * mutechk.c -- MEASUREMENT probe (not a product program) for the snapshot MUTE (Ssystem 2022 op 4,
 * docs/api/usb/usb-snapshot.md §2.6 and §9) as the AES sees it: pointer position and button state
 * through graf_mkstate (AES 79), keys through the GEMDOS console queue (Cconis/Cnecin).
 * Runs on qemu-virt as NEWDESK.NAP of a scratch kernel; method/qmp_mutechk.py drives the input.
 *   A  unmuted: QMP moves -> graf_mkstate position changes (the input path works)
 *   B  QMP holds the left button and key 'x'; graf_mkstate button 1; keys drained
 *   C  MUTE 1400 ticks (7 s); QMP releases button + key, moves, clicks: position unchanged, button 0
 *      (released by the kernel at the mute start), no key queued (no auto-repeat of the held 'x')
 *   D  after the deadline: QMP moves -> position changes again, still no ghost key
 *   E  "kill": MUTE 7000 (35 s) then a bad pointer -> the kernel ends the probe; QMP RETURN at +5 s must
 *      do nothing, RETURN at +40 s restarts the probe (second "MUTECHK: start")
 * Build: sdk/build.sh method/mutechk.c MUTECHK.NAP
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
static void waitt(unsigned t) { unsigned t0 = now(); while (now() - t0 < t) g3(0x0B, 0, 0, 0); }
static char l[160]; static int n;
static void s(const char *p) { while (*p) l[n++] = *p++; }
static void d(long v) { char t[24]; int i = 0; unsigned long u = v < 0 ? -v : v; if (v < 0) l[n++] = '-';
    do { t[i++] = '0' + u % 10; u /= 10; } while (u); while (i) l[n++] = t[--i]; }
static void out(void) { l[n++] = '\r'; l[n++] = '\n'; l[n] = 0; Cconws(l); n = 0; }
static void say(const char *p) { s("MUTECHK: "); s(p); out(); }
static short mx, my, mb;
static void mk(const char *tag) { aes(79, 0, 5, 0); mx = iout[1]; my = iout[2]; mb = iout[3];
    s("MUTECHK: "); s(tag); s(" x "); d(mx); s(" y "); d(my); s(" button "); d(mb); out(); }
static int drain(const char *tag) { int k = 0; long c = 0;
    while (g3(0x0B, 0, 0, 0)) { c = g3(0x08, 0, 0, 0); k++; }
    s("MUTECHK: "); s(tag); s(" keys queued "); d(k); if (k) { s(" last 0x"); d(c); } out(); return k; }
static int fails;
static void chk(int ok, const char *what) { s(ok ? "MUTECHK: PASS " : "MUTECHK: FAIL "); s(what); out(); fails += !ok; }
int main(void)
{
    short x0, y0, x1, y1, x2, y2;
    unsigned t;
    aes(10, 0, 1, 0);
    s("MUTECHK: start now "); d(now()); out();
    drain("A0");
    mk("A0"); x0 = mx; y0 = my;
    say("move now"); waitt(300);
    mk("A1"); x1 = mx; y1 = my;
    chk(x1 != x0 || y1 != y0, "A unmuted moves change the position");
    say("hold now"); waitt(200);
    mk("B1"); chk(mb == 1, "B held left button seen (button 1)");
    drain("B1");
    snap(4, 1400, 64); t = rd32(28);
    say("muted"); waitt(500);
    mk("C1"); x2 = mx; y2 = my;
    chk(mb == 0, "C button released at the mute start although still held/clicked");
    waitt(200);
    mk("C2");
    chk(mx == x2 && my == y2, "C muted moves do not change the position");
    chk(drain("C2") == 0, "C no key queued during the mute (no repeat of the held key)");
    while (now() - t < 1450) g3(0x0B, 0, 0, 0);
    say("unmuted"); waitt(300);
    mk("D1");
    chk(mx != x2 || my != y2, "D after the deadline moves change the position again");
    chk(drain("D1") == 0, "D no ghost key after the mute");
    s("MUTECHK: RESULT fails "); d(fails); out();
    snap(4, 7000, 64);
    s("MUTECHK: crash next (MUTE 7000 set) now "); d(rd32(28)); s(" until "); d(rd32(32)); out();
    g3(0x154, 2022, (long)0xDEAD0000, 64);
    say("survived the bad pointer (WRONG)");
    Pterm(1);
}
