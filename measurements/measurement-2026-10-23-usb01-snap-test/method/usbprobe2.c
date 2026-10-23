/*
 * usbprobe2.c -- TESTER measurement probe (NOT a product program), second probe for Ssystem(2022) edge cases that the
 * developer's usbprobe.c does not cover. Same embedding as usbprobe.c (scratch kernel, runs as NEWDESK.NAP on qemu-virt).
 * Every check prints "CHK <name>: OK" or "CHK <name>: FAIL <detail>". Spec: docs/api/usb/usb-snapshot.md §2.2/§2.3/§2.5/§2.6.
 * Buffer sentinel 0xA5: bytes past the answer's used_len must stay untouched on every path.
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
#define BUFSZ 8192
static unsigned char buf[BUFSZ];

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
static unsigned u16(int o) { return (unsigned)buf[o] | ((unsigned)buf[o + 1] << 8); }
static unsigned u32(int o) { return u16(o) | (u16(o + 2) << 16); }

static void fillbuf(void) { int k; for (k = 0; k < BUFSZ; k++) buf[k] = 0xA5; }
static void req(unsigned version, unsigned op, unsigned index, unsigned arg)
{
    int k;
    fillbuf();
    for (k = 0; k < 32; k++) buf[k] = 0;
    buf[0] = 'U'; buf[1] = 'S'; buf[2] = 'B'; buf[3] = 'S';
    buf[4] = (unsigned char)version; buf[8] = (unsigned char)op; buf[12] = (unsigned char)index;
    buf[16] = (unsigned char)arg; buf[17] = (unsigned char)(arg >> 8); buf[18] = (unsigned char)(arg >> 16); buf[19] = (unsigned char)(arg >> 24);
}
/* all bytes from `from` on are still the sentinel */
static int sentinel_from(int from) { int k; for (k = from; k < BUFSZ; k++) if (buf[k] != 0xA5) return 0; return 1; }
static int all_sentinel(void) { return sentinel_from(0); }

static void chk(const char *name, int ok, long a, long b)
{
    char l[160]; int j = sapp(l, 0, "CHK "); j = sapp(l, j, name);
    if (ok) { sapp(l, j, ": OK"); }
    else { j = sapp(l, j, ": FAIL "); j = napp(l, j, a); j = sapp(l, j, " / "); napp(l, j, b); }
    out(l);
}
static long call(long len) { return gemdos3(SSYSTEM, USBSNAP, (NLONG)buf, len); }
static unsigned now(void) { long rc; req(1, 1, 0, 0); rc = call(32); (void)rc; return u32(28); }
static void wait_ticks(unsigned t) { unsigned t0 = now(); while (now() - t0 < t) gemdos0(CCONIS); }

int main(void)
{
    long rc; unsigned total, used, flags; int n;
    out("USBPROBE2: start");

    /* --- §2.3 #1/#2: nothing written */
    req(1, 1, 0, 0); rc = gemdos3(SSYSTEM, USBSNAP, 0, 64);
    chk("buf NULL -> -25", rc == -25, rc, 0);
    req(1, 1, 0, 0); rc = call(31);
    chk("len 31 -> -25, untouched", rc == -25 && buf[0] == 'U' && sentinel_from(32), rc, 0);
    req(1, 1, 0, 0); rc = call(0);
    chk("len 0 -> -25", rc == -25, rc, 0);
    req(1, 1, 0, 0); rc = call(-1);
    chk("len -1 -> -25 (i32)", rc == -25, rc, 0);
    req(1, 1, 0, 0); buf[0] = 'u'; rc = call(BUFSZ);
    chk("bad magic -> -25, buffer untouched beyond request", rc == -25 && buf[0] == 'u' && u16(6) == 0 && sentinel_from(32), rc, 0);
    /* --- #3/#4/#6/#7: header only, 32 bytes */
    req(2, 1, 0, 0); rc = call(BUFSZ);
    chk("version 2 -> -25, header v1 used 32, rest untouched", rc == -25 && u16(4) == 1 && u16(6) == 32 && u32(24) == 32 && u32(20) == 32 && u16(10) == 0 && sentinel_from(32), rc, u32(24));
    req(0, 1, 0, 0); rc = call(BUFSZ);
    chk("version 0 -> -25", rc == -25 && u16(4) == 1, rc, u16(4));
    req(1, 0, 0, 0); rc = call(BUFSZ);
    chk("op 0 -> -25", rc == -25 && u16(8) == 0 && sentinel_from(32), rc, u16(8));
    req(1, 5, 0, 0); rc = call(BUFSZ);
    chk("op 5 -> -25", rc == -25 && u16(8) == 5 && sentinel_from(32), rc, u16(8));
    req(1, 2, 200, 0); rc = call(BUFSZ);
    chk("DEVICE index 200 -> -25 mirrored index", rc == -25 && u32(12) == 200 && u32(24) == 32 && sentinel_from(32), rc, u32(12));
    req(1, 3, 99, 0); rc = call(BUFSZ);
    chk("REPORTS index 99 -> -25", rc == -25 && u32(24) == 32 && sentinel_from(32), rc, 0);
    req(1, 4, 1, 100); rc = call(BUFSZ);
    chk("MUTE index 1 -> -25 (index must be 0)", rc == -25 && u32(24) == 32 && sentinel_from(32), rc, 0);
    req(1, 4, 0, 12001); rc = call(BUFSZ);
    chk("MUTE 12001 -> -25", rc == -25 && u32(16) == 12001, rc, 0);
    req(1, 4, 0, 0xFFFFFFFFu); rc = call(BUFSZ);
    chk("MUTE 0xFFFFFFFF -> -25", rc == -25, rc, 0);

    /* --- LIST sizes */
    req(1, 1, 0, 0); rc = call(32);
    flags = u16(10); total = u32(20); used = u32(24);
    chk("LIST len 32 -> 0 TRUNCATED used 32 total>32", rc == 0 && (flags & 1) && used == 32 && total > 32 && sentinel_from(32), rc, total);
    req(1, 1, 0, 0); rc = call((long)total - 1);
    chk("LIST len total-1 -> TRUNCATED header only", rc == 0 && (u16(10) & 1) && u32(24) == 32 && u32(20) == total && sentinel_from(32), rc, u32(24));
    req(1, 1, 0, 0); rc = call((long)total);
    chk("LIST len total -> full, flags 0, used==total", rc == 0 && u16(10) == 0 && u32(24) == total && sentinel_from((int)total), rc, u32(24));
    n = (int)u16(32 + 4);
    req(1, 1, 0, 0); rc = call(BUFSZ);
    chk("LIST len 8192 same total as exact", rc == 0 && u32(20) == total && u32(24) == total && sentinel_from((int)total), rc, u32(24));
    /* header mirrors request fields */
    req(1, 1, 0, 0); buf[10] = 0xff; buf[11] = 0xff; buf[6] = 7; buf[20] = 9; buf[24] = 9; buf[28] = 9; rc = call(BUFSZ);
    chk("LIST ignores hdr_len/flags/total/used/now in request", rc == 0 && u16(6) == 32 && u16(10) == 0 && u32(20) == total, rc, u16(10));

    /* --- DEVICE sizes */
    if (n > 0) {
        req(1, 2, 0, 0); rc = call(BUFSZ);
        total = u32(20); used = u32(24);
        chk("DEVICE 0 full (used==total, rest untouched)", rc == 0 && used == total && u16(10) == 0 && sentinel_from((int)total), rc, total);
        req(1, 2, 0, 0); rc = call((long)total - 1);
        chk("DEVICE len total-1 -> TRUNCATED, header only (no partial answer)", rc == 0 && (u16(10) & 1) && u32(24) == 32 && u32(20) == total && sentinel_from(32), rc, u32(24));
        req(1, 2, 0, 0); rc = call(100);
        chk("DEVICE len 100 -> TRUNCATED, header only", rc == 0 && (u16(10) & 1) && u32(24) == 32 && sentinel_from(32), rc, u32(24));
    }

    /* --- REPORTS sizes with nothing / something recorded */
    req(1, 3, 0, 0xFFFFFFFFu); rc = call(BUFSZ);
    chk("REPORTS arg 0xFFFFFFFF -> count 0, rc 0", rc == 0 && u16(32 + 12) == 0 && u32(24) == 48 && sentinel_from(48), rc, u16(32 + 12));
    req(1, 3, 0, 0); rc = call(47);
    chk("REPORTS len 47 -> TRUNCATED header only", rc == 0 && (u16(10) & 1) && u32(24) == 32 && sentinel_from(32), rc, u32(24));
    req(1, 3, 0, 0); rc = call(48);
    chk("REPORTS len 48 -> ok (rc 0)", rc == 0, rc, 0);

    out("USBPROBE2: move now");           /* QMP driver sends 300 moves */
    wait_ticks(900);
    {
        unsigned next, oldest, lost; int idx = 0, i, cnt;
        /* find the device with a filled ring */
        for (i = 0; i < n; i++) { req(1, 3, (unsigned)i, 0); call(48); if (u32(32 + 4) > 0) { idx = i; break; } }
        req(1, 3, (unsigned)idx, 0); rc = call(48);
        next = u32(32 + 4); oldest = u32(32); lost = u32(32 + 8);
        { char l[120]; int j = sapp(l, 0, "USBPROBE2: ring dev "); j = napp(l, j, idx); j = sapp(l, j, " oldest "); j = napp(l, j, oldest); j = sapp(l, j, " next "); j = napp(l, j, next); j = sapp(l, j, " lost "); j = napp(l, j, lost); out(l); }
        chk("REPORTS len 48: count 0 with MORE or full fit", rc == 0 && u16(32 + 12) == 0 && (u16(10) & 2), rc, u16(10));
        req(1, 3, (unsigned)idx, 0); rc = call(48 + 79);
        chk("REPORTS len 127 (< one entry) -> count 0 + MORE", rc == 0 && u16(32 + 12) == 0 && (u16(10) & 2) && u32(24) == 48, rc, u16(10));
        req(1, 3, (unsigned)idx, 0); rc = call(48 + 80);
        chk("REPORTS len 128 -> exactly 1 entry + MORE", rc == 0 && u16(32 + 12) == 1 && (u16(10) & 2) && u32(24) == 128 && sentinel_from(128), rc, u16(32 + 12));
        chk("ring overflow: next >= 257 (>= 300 moves sent, 256-entry ring)", next >= 257, next, 0);
        chk("ring overflow: oldest == next-256", oldest == next - 256, oldest, next - 256);
        chk("ring overflow: lost(arg 0) == oldest", lost == oldest, lost, oldest);
        req(1, 3, (unsigned)idx, oldest + 10); rc = call(48 + 80 * 4);
        chk("REPORTS arg oldest+10: lost 0, first seq == arg", rc == 0 && u32(32 + 8) == 0 && u32(48) == oldest + 10 && u16(32 + 12) == 4, rc, u32(48));
        req(1, 3, (unsigned)idx, oldest - 5); rc = call(48 + 80);
        chk("REPORTS arg oldest-5: lost 5, first seq == oldest", rc == 0 && u32(32 + 8) == 5 && u32(48) == oldest, rc, u32(32 + 8));
        req(1, 3, (unsigned)idx, next); rc = call(BUFSZ);
        chk("REPORTS arg==next: count 0, rc 0, no MORE", rc == 0 && u16(32 + 12) == 0 && !(u16(10) & 2), rc, 0);
        /* the whole ring in one go: 256 entries need 48 + 256*80 = 20528 > BUFSZ -> take what fits */
        req(1, 3, (unsigned)idx, oldest); rc = call(BUFSZ);
        cnt = (int)u16(32 + 12);
        chk("REPORTS full ring request into 8192: count == (8192-48)/80, MORE, total == 48+256*80",
            rc == 0 && cnt == (BUFSZ - 48) / 80 && (u16(10) & 2) && u32(20) == 48 + 256 * 80, cnt, u32(20));
        { int ok = 1; for (i = 0; i < cnt; i++) if (u32(48 + i * 80) != oldest + (unsigned)i) ok = 0;
          chk("REPORTS entries consecutive seq from oldest", ok, 0, 0); }
        /* LIST entry ring numbers equal REPORTS header */
        req(1, 1, 0, 0); call(BUFSZ);
        chk("LIST entry ring_oldest/next == REPORTS oldest/next", u32(32 + 16 + idx * 32 + 20) == oldest && u32(32 + 16 + idx * 32 + 24) >= next, u32(32 + 16 + idx * 32 + 20), oldest);
    }

    /* --- MUTE edge cases */
    req(1, 4, 0, 12000); rc = call(BUFSZ);
    chk("MUTE 12000 accepted", rc == 0 && u32(24) == 40, rc, u32(24));
    req(1, 4, 0, 0); rc = call(BUFSZ);
    chk("MUTE 0 -> muted_until 0", rc == 0 && u32(32) == 0, rc, u32(32));
    req(1, 4, 0, 300); rc = call(32);
    chk("MUTE with len 32: header only, TRUNCATED, total 40 (§9: applies anyway)", rc == 0 && (u16(10) & 1) && u32(24) == 32 && u32(20) == 40, rc, u32(20));
    req(1, 4, 0, 0); rc = call(BUFSZ);
    chk("MUTE 0 clears again", rc == 0 && u32(32) == 0, rc, u32(32));
    out("USBPROBE2: done");
    Pterm(0);
    return 0;
}
