/*
 * usbprobe3.c -- TESTER measurement probe (NOT a product program) for boards WITHOUT a USB controller (raspi4b):
 * Ssystem(2022) with a well-formed request must return ERROR (-1) for every op and leave the buffer byte-for-byte as
 * the caller wrote it (spec §2.3 #5); header-validation errors (#1-#4) come BEFORE the controller test and stay -25.
 * Prints "P3 <name>: OK|FAIL ..." and ends with "P3 done" (it does not wait for any clock, unlike usbprobe.c).
 */
#include <newtos.h>
static inline NLONG gemdos3(NLONG fn, NLONG a0, NLONG a1, NLONG a2)
{
    register NLONG x8 __asm__("x8") = fn;
    register NLONG r0 __asm__("x0") = a0;
    register NLONG r1 __asm__("x1") = a1;
    register NLONG r2 __asm__("x2") = a2;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2) : "memory");
    return r0;
}
static unsigned char buf[512], ref[512];
static int sapp(char *d, int j, const char *s) { while (*s) d[j++] = *s++; d[j] = 0; return j; }
static int napp(char *d, int j, long v)
{
    char t[24]; int i = 0; unsigned long u;
    if (v < 0) { d[j++] = '-'; u = (unsigned long)(-v); } else u = (unsigned long)v;
    do { t[i++] = (char)('0' + (int)(u % 10)); u /= 10; } while (u);
    while (i) d[j++] = t[--i];
    d[j] = 0; return j;
}
static void out(const char *s) { Cconws(s); Cconws("\r\n"); }
static void setup(unsigned version, unsigned op, unsigned index, unsigned arg)
{
    int k;
    for (k = 0; k < 512; k++) buf[k] = (unsigned char)(0xA5 ^ k);   /* a pattern, not a constant */
    buf[0] = 'U'; buf[1] = 'S'; buf[2] = 'B'; buf[3] = 'S'; buf[4] = (unsigned char)version; buf[5] = 0;
    buf[8] = (unsigned char)op; buf[9] = 0; buf[12] = (unsigned char)index; buf[16] = (unsigned char)arg; buf[17] = (unsigned char)(arg >> 8);
    for (k = 0; k < 512; k++) ref[k] = buf[k];
}
static int same(void) { int k; for (k = 0; k < 512; k++) if (buf[k] != ref[k]) return 0; return 1; }
static void rep(const char *name, long rc, long want, int unchanged)
{
    char l[120]; int j = sapp(l, 0, "P3 "); j = sapp(l, j, name); j = sapp(l, j, ": ");
    j = sapp(l, j, (rc == want && unchanged) ? "OK" : "FAIL"); j = sapp(l, j, " rc "); j = napp(l, j, rc);
    j = sapp(l, j, " buffer "); sapp(l, j, unchanged ? "untouched" : "CHANGED"); out(l);
}
int main(void)
{
    long rc; unsigned op;
    out("P3 start");
    for (op = 1; op <= 4; op++) {
        char n[40]; int j = sapp(n, 0, "well-formed op "); j = napp(n, j, op); n[j] = 0;
        setup(1, op, 0, op == 4 ? 100 : 0);
        rc = gemdos3(0x154, 2022, (NLONG)buf, 512);
        rep(n, rc, -1, same());
    }
    setup(1, 2, 3, 0); rc = gemdos3(0x154, 2022, (NLONG)buf, 512); rep("DEVICE index 3 (no device model)", rc, -1, same());
    setup(1, 1, 0, 0); rc = gemdos3(0x154, 2022, (NLONG)buf, 32); rep("LIST len 32 (would be TRUNCATED elsewhere)", rc, -1, same());
    setup(1, 1, 0, 0); rc = gemdos3(0x154, 2022, (NLONG)buf, 31); rep("len 31 -> -25 before the controller test", rc, -25, same());
    setup(1, 1, 0, 0); buf[0] = 'X'; ref[0] = 'X'; rc = gemdos3(0x154, 2022, (NLONG)buf, 512); rep("bad magic -> -25", rc, -25, same());
    setup(2, 1, 0, 0); rc = gemdos3(0x154, 2022, (NLONG)buf, 512);
    { int ok = buf[4] == 1 && buf[6] == 32; char l[100]; int j = sapp(l, 0, "P3 version 2: rc "); j = napp(l, j, rc);
      j = sapp(l, j, ok ? " header v1 hdr_len 32 written: OK" : " header NOT written: FAIL"); out(l); }
    setup(1, 9, 0, 0); rc = gemdos3(0x154, 2022, (NLONG)buf, 512);
    { char l[100]; int j = sapp(l, 0, "P3 op 9: rc "); j = napp(l, j, rc); sapp(l, j, rc == -25 ? " OK" : " FAIL"); out(l); }
    rc = gemdos3(0x154, 2023, (NLONG)buf, 512);
    { char l[100]; int j = sapp(l, 0, "P3 mode 2023 (first free number): rc "); j = napp(l, j, rc); sapp(l, j, rc == -32 ? " OK (EINVFN)" : " FAIL"); out(l); }
    out("P3 done");
    return 0;
}
