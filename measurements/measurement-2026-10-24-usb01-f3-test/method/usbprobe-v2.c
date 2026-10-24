/*
 * usbprobe-v2.c (TESTER, derived from measurement-2026-10-23-usb01-snap-kernel/method/usbprobe.c) -- measurement probe (NOT a product program): calls
 * Ssystem(S_NEWTOS_USBSNAP = 2022) from EL0 and prints every answer as hex on
 * the console (serial on qemu-virt), so the run log can be compared with the
 * kernel's own [usb] lines. Spec: docs/api/usb/usb-snapshot.md §2.
 *
 * Sequence:
 *   1. header checks: len 16 (EINVAL), wrong magic (EINVAL), version 2
 *      (EINVAL + header), op 9 (EINVAL + header);
 *   2. LIST with 40 bytes (TRUNCATED, total_len), then with total_len;
 *   3. DEVICE for every listed device, full hex dump;
 *   4. index = n_devices (EINVAL), MUTE 12001 (EINVAL);
 *   5. "USBPROBE: move now" -> the QMP driver moves the mouse; the probe
 *      waits ~3 s (now_hz200 from the answers, Cconis pumps USB) and dumps
 *      REPORTS of every device from seq 0, once with a small buffer (MORE);
 *   6. MUTE 400 ticks: "USBPROBE: muted" -> moves; REPORTS from seq 20 (the
 *      first report after the 20 unmuted ones: entry flag bit 1);
 *      MUTE 0;
 *   7. last: Ssystem(2022) with buf = 0xDEAD0000 (unmapped in every process, as sdk/examples/uakid.c) -- the kernel must end
 *      this program (crash report), it must NOT print "survived".
 *
 * Built with sdk/build.sh, embedded in a SCRATCH kernel as NEWDESK.NAP (the
 * program the shell starts at boot), so it runs as an EL0 process on
 * qemu-virt, which has no card to start a program from.
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

#define SSYSTEM 0x154
#define USBSNAP 2022
#define CCONIS 0x0B

static unsigned char buf[16384];

static int sapp(char *d, int j, const char *s) { while (*s) d[j++] = *s++; d[j] = 0; return j; }
static int napp(char *d, int j, long v)
{
    char t[24]; int i = 0; unsigned long u;
    if (v < 0) { d[j++] = '-'; u = (unsigned long)(-v); } else u = (unsigned long)v;
    do { t[i++] = (char)('0' + (int)(u % 10)); u /= 10; } while (u);
    while (i) d[j++] = t[--i];
    d[j] = 0; return j;
}
static int xapp(char *d, int j, unsigned v, int digits)
{
    static const char hx[] = "0123456789abcdef";
    while (digits--) d[j++] = hx[(v >> (4 * digits)) & 15];
    d[j] = 0; return j;
}
static void out(const char *s) { Cconws(s); Cconws("\r\n"); }

static unsigned u16(int o) { return (unsigned)buf[o] | ((unsigned)buf[o + 1] << 8); }
static unsigned u32(int o) { return u16(o) | (u16(o + 2) << 16); }

static void req(unsigned version, unsigned op, unsigned index, unsigned arg)
{
    int k;
    for (k = 0; k < 32; k++) buf[k] = 0;
    buf[0] = 'U'; buf[1] = 'S'; buf[2] = 'B'; buf[3] = 'S';
    buf[4] = (unsigned char)version; buf[8] = (unsigned char)op;
    buf[12] = (unsigned char)index;
    buf[16] = (unsigned char)arg; buf[17] = (unsigned char)(arg >> 8);
    buf[18] = (unsigned char)(arg >> 16); buf[19] = (unsigned char)(arg >> 24);
}

static long call(const char *what, long len)
{
    char l[200]; int j;
    long rc = gemdos3(SSYSTEM, USBSNAP, (NLONG)buf, len);
    j = sapp(l, 0, "USBPROBE: "); j = sapp(l, j, what); j = sapp(l, j, " len ");
    j = napp(l, j, len); j = sapp(l, j, " -> rc "); j = napp(l, j, rc);
    if (buf[0] == 'U' && buf[1] == 'S') {
        j = sapp(l, j, " | ver "); j = napp(l, j, u16(4));
        j = sapp(l, j, " hdr "); j = napp(l, j, u16(6));
        j = sapp(l, j, " op "); j = napp(l, j, u16(8));
        j = sapp(l, j, " flags "); j = napp(l, j, u16(10));
        j = sapp(l, j, " total "); j = napp(l, j, u32(20));
        j = sapp(l, j, " used "); j = napp(l, j, u32(24));
        j = sapp(l, j, " now "); j = napp(l, j, u32(28));
    }
    out(l);
    return rc;
}

static void dump(int from, int to)
{
    char l[120]; int o, k, j;
    for (o = from; o < to; o += 16) {
        j = sapp(l, 0, "USBPROBE:   "); j = xapp(l, j, (unsigned)(o - from), 4); j = sapp(l, j, ":");
        for (k = o; k < o + 16 && k < to; k++) { l[j++] = ' '; j = xapp(l, j, buf[k], 2); }
        out(l);
    }
}

static unsigned now(void) { req(2, 1, 0, 0); gemdos3(SSYSTEM, USBSNAP, (NLONG)buf, 32); return u32(28); }

static void wait_ticks(unsigned t)
{
    unsigned t0 = now();
    while (now() - t0 < t) gemdos0(CCONIS);
}

static void reports(int n, const char *tag, unsigned from)
{
    int i, c;
    for (i = 0; i < n; i++) {
        char l[80]; int j = sapp(l, 0, tag); j = napp(l, j, i); l[j] = 0;
        req(1, 3, (unsigned)i, from);
        call(l, 32 + 16 + 3 * 80);
        c = (int)u16(32 + 12);
        dump(32, 48);
        dump(48, 48 + c * 80);
    }
}

int main(void)
{
    long rc; int n, i;
    out("USBPROBE2: start");
    req(1, 1, 0, 0); call("LIST version1", 64);
    dump(0, 32);
    req(3, 1, 0, 0); call("LIST version3", 64);
    req(0, 1, 0, 0); call("LIST version0", 64);
    req(2, 1, 0, 0); call("LIST version2 small", 40);
    rc = (long)u32(20);
    req(2, 1, 0, 0); call("LIST version2 full", rc);
    dump(32, (int)u32(24));
    n = (int)u16(32 + 4);
    for (i = 0; i < n; i++) {
        char l[80]; int j = sapp(l, 0, "USBPROBE2: entry "); j = napp(l, j, i);
        j = sapp(l, j, " role "); j = napp(l, j, buf[48 + 32 * i + 14]);
        j = sapp(l, j, " reason "); j = napp(l, j, buf[48 + 32 * i + 15]);
        j = sapp(l, j, " ring_if "); j = napp(l, j, buf[48 + 32 * i + 18]);
        j = sapp(l, j, " ring_ep "); j = napp(l, j, buf[48 + 32 * i + 19]);
        j = sapp(l, j, " n_funcs "); j = napp(l, j, buf[48 + 32 * i + 31]);
        out(l);
    }
    out("USBPROBE2: move now");
    wait_ticks(1200);
    for (i = 0; i < n; i++) {
        char l[80]; int j = sapp(l, 0, "REP dev "); j = napp(l, j, i); l[j] = 0;
        req(2, 3, (unsigned)i, 0);
        call(l, 32 + 16 + 4 * 80);
        { int c = (int)u16(32 + 12); dump(32, 48); dump(48, 48 + c * 80); }
    }
    out("USBPROBE2: done");
    for (;;) gemdos0(CCONIS);
    return 0;
}
