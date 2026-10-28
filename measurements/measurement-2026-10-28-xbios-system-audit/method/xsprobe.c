/*
 * xsprobe.c -- ad-hoc probe for the audit docs/audits/2026-10-28-xbios-system.md
 * (XBIOS system calls without disk access: Random 0x11, Protobt 0x12,
 * Cursconf 0x15, Settime 0x16, Gettime 0x17, Setprt 0x21).
 *
 * NOT tosdump: it measures what the DUMPBIOS guard refuses
 * (tosdump/core/cases.c call_policy) -- the set arms of Cursconf, Setprt,
 * Settime -- and what needs state a call table cannot set up (Protobt on an
 * executable sector, Random's sequence, the random Protobt serial).  Every
 * setting it changes is put back before it ends (Cursconf blink rate and
 * flash bit, Setprt word, the clock: start value plus the elapsed _hz_200
 * time).  Same C source built twice (m68k-atari-mint-gcc + mintlib for EmuTOS
 * on Hatari; clang aarch64 + the SDK's mintlib for NewTOS), calls through
 * mintlib's osbind.h macros as a ported program does.
 *
 * Output (current directory): XSPROBE.LOG, groups
 *   P  Protobt state paths (exec keep/clear/set, odd buffer, random serial
 *      chained against Random)
 *   R  Random: range of 1000 values, LCG consistency of a 10-value run
 *   C  Cursconf return values and the 4/5 round trip
 *   S  Setprt set/query round trip
 *   T  Settime/Gettime and the GEMDOS clock (Tgetdate/Tgettime/Tsetdate/
 *      Tsettime) -- relational: a value set, then read back
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>
 * (m68k: the sign extension of d0, the value (long long)(long) gives).
 * Lines marked "info" carry machine values (boot clock, elapsed ticks) that
 * are not compared.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#define U __attribute__((unused))
static char out[24000];
static int olen;

static void emit(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    if (olen < (int)sizeof(out) - 256)
        olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
    va_end(ap);
}

static void flush_to(const char *name)
{
    long fh = Fcreate(name, 0);
    if (fh < 0)
        return;
    Fwrite((short)fh, (long)olen, out);
    Fclose((short)fh);
}

static unsigned long lo32(long r) { return (unsigned long)r & 0xFFFFFFFFUL; }
static unsigned long hi32(long r)
{
    if (sizeof(long) > 4)
        return ((unsigned long)r >> 16 >> 16) & 0xFFFFFFFFUL;
    return r < 0 ? 0xFFFFFFFFUL : 0UL;
}

static void line(const char *id, const char *call, long r)
{
    emit("%s %s ret=%08lx hi=%08lx\n", id, call, lo32(r), hi32(r));
}

/* WORD-returning calls: only the low 16 bits are defined on the m68k. */
static void linew(const char *id, const char *call, long r)
{
    emit("%s %s ret.w=%04lx\n", id, call, lo32(r) & 0xFFFFUL);
}

/* ---- Supexec: _hz_200 ($4BA, protected on TOS) -------------------------- */
static volatile long sv_c;
static long hz200(void)
{
    sv_c = *(volatile long *)0x4BA;
    return sv_c;
}
static long now_ticks(void) { return Supexec(hz200); }

/* ---- Protobt helpers ---------------------------------------------------- */
static unsigned char sec[512 + 8];

static unsigned sum_be(const unsigned char *b)
{
    unsigned s = 0;
    int i;
    for (i = 0; i < 512; i += 2)
        s = (s + ((unsigned)b[i] << 8 | b[i + 1])) & 0xFFFFU;
    return s;
}

static void hexbytes(const char *id, const unsigned char *b, int n)
{
    int i;
    emit("%s bytes", id);
    for (i = 0; i < n; i++)
        emit(" %02x", b[i]);
    emit("\n");
}

/* ---- Random LCG --------------------------------------------------------- */
#define LCG_A 3141592621UL
static uint32_t lcg(uint32_t s) { return (uint32_t)(s * (uint32_t)LCG_A + 1U); }
static uint32_t lcg_out(uint32_t s) { return (s >> 8) & 0xFFFFFFU; }

/* Count the seeds s (bits 8..31 of s fixed by v[0] up to bits 24..31, and
 * bits 0..7 free) that reproduce v[1..n-1] as the following outputs.  The
 * seed before v[0] is not needed: v[0] = out(s0), v[k] = out(s_k). */
static int lcg_fit(const uint32_t *v, int n)
{
    int hi, lo, k, fits = 0;
    for (hi = 0; hi < 256; hi++)
        for (lo = 0; lo < 256; lo++) {
            uint32_t s = ((uint32_t)hi << 24) | (v[0] << 8) | (uint32_t)lo;
            for (k = 1; k < n; k++) {
                s = lcg(s);
                if (lcg_out(s) != v[k])
                    break;
            }
            if (k == n)
                fits++;
        }
    return fits;
}

/* ---- clock helpers ------------------------------------------------------ */
static unsigned long pack(int y, int mo, int d, int h, int mi, int s2)
{
    unsigned long date = ((unsigned long)(y - 1980) << 9) | ((unsigned long)mo << 5) | (unsigned long)d;
    unsigned long time = ((unsigned long)h << 11) | ((unsigned long)mi << 5) | (unsigned long)s2;
    return ((date & 0xFFFFUL) << 16) | (time & 0xFFFFUL);
}

static void dec(const char *id, const char *what, unsigned long v)
{
    unsigned d = (unsigned)(v >> 16) & 0xFFFFU, t = (unsigned)v & 0xFFFFU;
    emit("%s %s=%08lx %04u-%02u-%02u %02u:%02u:%02u\n", id, what, v & 0xFFFFFFFFUL,
         1980 + (d >> 9), (d >> 5) & 15, d & 31, t >> 11, (t >> 5) & 63, (t & 31) * 2);
}

static int mdays(int y, int m)
{
    static const int md[12] = { 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31 };
    return (m == 2 && (y % 4) == 0) ? 29 : md[m - 1];
}

/* packed (valid) + n seconds, for the final restore */
static unsigned long add_secs(unsigned long v, long n)
{
    unsigned d = (unsigned)(v >> 16) & 0xFFFFU, t = (unsigned)v & 0xFFFFU;
    int y = 1980 + (d >> 9), mo = (d >> 5) & 15, dd = d & 31;
    long s = (long)(t >> 11) * 3600 + (long)((t >> 5) & 63) * 60 + (t & 31) * 2 + n;
    if (mo < 1 || mo > 12 || dd < 1)
        return v;
    while (s >= 86400) {
        s -= 86400;
        if (++dd > mdays(y, mo)) {
            dd = 1;
            if (++mo > 12) {
                mo = 1;
                y++;
            }
        }
    }
    return pack(y, mo, dd, (int)(s / 3600), (int)(s / 60 % 60), (int)(s % 60 / 2));
}

static long gt(void) { return (long)Gettime(); }

/* Date and time in one consistent pair: a clock tick between the two calls
 * would pair an old date with a new time; re-read until the date is the same
 * before and after the time (one tick cannot change the date twice). */
static unsigned long tget(void)
{
    long d1, t, d2;
    do {
        d1 = (long)Tgetdate();
        t = (long)Tgettime();
        d2 = (long)Tgetdate();
    } while (lo32(d1) != lo32(d2));
    return ((lo32(d1) & 0xFFFFUL) << 16) | (lo32(t) & 0xFFFFUL);
}

/* Settime(v), then the readers at once. */
static void set_and_read(const char *id, unsigned long v)
{
    long g;
    unsigned long tg;
    Settime((long)v);
    tg = tget();
    g = gt();
    dec(id, "set", v);
    dec(id, "Tget", tg);
    dec(id, "Gettime", lo32(g));
    emit("%s Gettime.hi=%08lx\n", id, hi32(g));
}

/* Settime(v), then poll the GEMDOS clock (Tgetdate/Tgettime) until it moves
 * (at most `limit` ticks): on EmuTOS the cache Settime wrote, advanced by
 * tikfrk (bdos/time.c:180-230); Gettime reads the RTC instead, which on the
 * oracle is Hatari's (see the audit, section 6). */
static void set_and_roll(const char *id, unsigned long v, long limit)
{
    long t0, g;
    unsigned long tg;
    Settime((long)v);
    t0 = now_ticks();
    do {
        tg = tget();
    } while (tg == (v & 0xFFFFFFFFUL) && now_ticks() - t0 < limit);
    g = gt();
    dec(id, "set", v);
    dec(id, "Tget-after-change", tg);
    dec(id, "Gettime-after-change", lo32(g));
    emit("%s info ticks=%ld\n", id, now_ticks() - t0);
}

int main(void)
{
    char b[96];
    long r, r1, r3, p0, s0, t_start;
    unsigned long g0;
    uint32_t v[10];
    int i, bad, bad_hi;
    unsigned char *odd;

    /* ---- T00: the clock before anything, and its start in ticks -------- */
    t_start = now_ticks();
    g0 = lo32(gt());
    dec("T00 info", "Gettime-at-start", g0);
    {
        unsigned long tg = tget();
        dec("T00 info", "Tget-at-start", tg);
        /* the relation, not the values: Gettime and Tget agree (+-2 s) */
        emit("T00 Gettime-vs-Tget %s\n",
             (tg == g0 || add_secs(g0, 2) == tg || add_secs(tg, 2) == g0) ? "agree" : "differ");
    }

    /* ---- P: Protobt ------------------------------------------------------ */
    memset(sec, 0, sizeof sec);
    Protobt(sec, -1L, -1, 1);
    emit("P01 zero sector exec=1 sum=%04x cksum=%02x%02x\n", sum_be(sec), sec[510], sec[511]);
    Protobt(sec, -1L, -1, -1);
    emit("P02 executable exec=-1 sum=%04x\n", sum_be(sec));
    Protobt(sec, -1L, -1, 0);
    emit("P03 exec=0 sum=%04x cksum=%02x%02x\n", sum_be(sec), sec[510], sec[511]);
    Protobt(sec, -1L, -1, -1);
    emit("P04 non-executable exec=-1 sum=%04x\n", sum_be(sec));
    Protobt(sec, -1L, -1, 1);
    Protobt(sec, 0x123456L, 3, -1);
    emit("P05 executable then (0x123456,3,-1) sum=%04x\n", sum_be(sec));
    hexbytes("P05", sec, 32);
    Protobt(sec, -1L, -1, -32768);
    emit("P06 executable exec=-32768 sum=%04x\n", sum_be(sec));
    /* an odd buffer address */
    memset(sec, 0, sizeof sec);
    odd = sec + 1;
    Protobt(odd, 0x123456L, 2, 1);
    emit("P07 odd buffer sum=%04x\n", sum_be(odd));
    hexbytes("P07", odd, 32);
    emit("P07 byte-before=%02x byte-after=%02x\n", sec[0], odd[512]);
    /* the random serial is the next Random value: r1, serial, r3 chain */
    memset(sec, 0, sizeof sec);
    r1 = (long)Random();
    Protobt(sec, 0x01000000L, -1, -1);
    r3 = (long)Random();
    v[0] = (uint32_t)lo32(r1);
    v[1] = (uint32_t)sec[8] | ((uint32_t)sec[9] << 8) | ((uint32_t)sec[10] << 16);
    v[2] = (uint32_t)lo32(r3);
    i = lcg_fit(v, 3);
    emit("P08 Random,Protobt(random serial),Random one LCG chain: %s\n", i > 0 ? "yes" : "no");
    emit("P08 info seeds=%d\n", i);
    emit("P08 bytes-0..7-untouched=%s sum=%04x\n",
         (sec[0] | sec[1] | sec[2] | sec[3] | sec[4] | sec[5] | sec[6] | sec[7]) ? "no" : "yes",
         sum_be(sec));

    /* ---- R: Random -------------------------------------------------------- */
    bad = bad_hi = 0;
    for (i = 0; i < 1000; i++) {
        r = (long)Random();
        if (lo32(r) & 0xFF000000UL)
            bad++;
        if (hi32(r) != 0)
            bad_hi++;
    }
    emit("R01 1000 values, bits 24..31 set in %d, high half set in %d\n", bad, bad_hi);
    for (i = 0; i < 10; i++)
        v[i] = (uint32_t)lo32((long)Random());
    i = lcg_fit(v, 10);
    emit("R02 10 consecutive values from one LCG seed: %s\n", i > 0 ? "yes" : "no");
    emit("R02 info seeds=%d\n", i);
    emit("R03 two consecutive values differ: %s\n", v[0] != v[1] ? "yes" : "no");

    /* ---- C: Cursconf ------------------------------------------------------ */
    p0 = (long)Cursconf(5, 0);
    emit("C00 info blink-rate-at-start=%ld\n", lo32(p0) & 0xFFFFUL);
    linew("C01", "Cursconf(4,7)", (long)Cursconf(4, 7));
    linew("C02", "Cursconf(5,0) after 7", (long)Cursconf(5, 0));
    linew("C03", "Cursconf(4,0x1FF)", (long)Cursconf(4, 0x1FF));
    linew("C04", "Cursconf(5,0) after 0x1FF", (long)Cursconf(5, 0));
    Cursconf(4, 0x80);
    linew("C05", "Cursconf(5,0) after 0x80", (long)Cursconf(5, 0));
    Cursconf(4, -1);
    linew("C06", "Cursconf(5,0) after -1", (long)Cursconf(5, 0));
    Cursconf(4, 0);
    linew("C07", "Cursconf(5,0) after 0", (long)Cursconf(5, 0));
    linew("C08", "Cursconf(2,0) flash off", (long)Cursconf(2, 0));
    linew("C09", "Cursconf(3,0) flash on", (long)Cursconf(3, 0));
    linew("C10", "Cursconf(1,0) show", (long)Cursconf(1, 0));
    linew("C11", "Cursconf(0,0) hide", (long)Cursconf(0, 0));
    linew("C12", "Cursconf(6,5)", (long)Cursconf(6, 5));
    linew("C13", "Cursconf(-1,5)", (long)Cursconf(-1, 5));
    linew("C14", "Cursconf(0x7FFF,5)", (long)Cursconf(0x7FFF, 5));
    linew("C15", "Cursconf(-32768,5)", (long)Cursconf(-32768, 5));
    linew("C16", "Cursconf(5,0) unchanged by 6/-1/..", (long)Cursconf(5, 0));
    Cursconf(4, (short)(lo32(p0) & 0xFF));
    emit("C17 restored=%s\n", ((lo32((long)Cursconf(5, 0)) ^ lo32(p0)) & 0xFFFFUL) ? "no" : "yes");

    /* ---- S: Setprt -------------------------------------------------------- */
    s0 = (long)Setprt(-1);
    emit("S00 info word-at-start=%04lx\n", lo32(s0) & 0xFFFFUL);
    linew("S01", "Setprt(0x1234)", (long)Setprt(0x1234));
    linew("S02", "Setprt(-1) after 0x1234", (long)Setprt(-1));
    linew("S03", "Setprt(-2)", (long)Setprt(-2));
    line("S04", "Setprt(-1) after -2 (LONG view)", (long)Setprt(-1));
    linew("S05", "Setprt(-32768)", (long)Setprt(-32768));
    linew("S06", "Setprt(-1) after -32768", (long)Setprt(-1));
    linew("S07", "Setprt(0)", (long)Setprt(0));
    linew("S08", "Setprt(-1) after 0", (long)Setprt(-1));
    Setprt((short)(lo32(s0) & 0xFFFF));
    emit("S09 restored=%s\n", ((lo32((long)Setprt(-1)) ^ lo32(s0)) & 0xFFFFUL) ? "no" : "yes");

    /* ---- T: Settime / Gettime -------------------------------------------- */
    set_and_read("T01", pack(2001, 2, 3, 4, 5, 3));          /* valid: 04:05:06 */
    set_and_roll("T02", pack(2001, 2, 3, 4, 5, 3), 6000);   /* advances by 2 s */
    set_and_roll("T03", pack(2001, 2, 3, 23, 59, 29), 6000); /* day roll */
    set_and_roll("T04", pack(2000, 2, 28, 23, 59, 29), 6000);/* leap 2000 */
    set_and_roll("T05", pack(2001, 2, 28, 23, 59, 29), 6000);/* no leap */
    set_and_roll("T06", pack(2001, 12, 31, 23, 59, 29), 6000);/* year roll */
    set_and_roll("T07", pack(2099, 12, 31, 23, 59, 29), 6000);/* 2100 */
    set_and_roll("T08", pack(2107, 12, 31, 23, 59, 29), 6000);/* past the field */
    /* no validation: impossible fields */
    set_and_read("T10", pack(2001, 2, 3, 25, 0, 0));         /* hour 25 */
    set_and_read("T11", pack(2001, 2, 3, 31, 63, 31));       /* all max */
    set_and_read("T12", pack(2001, 2, 3, 4, 61, 0));         /* minute 61 */
    set_and_read("T13", pack(2001, 2, 3, 4, 5, 30));         /* second 60 */
    set_and_read("T14", pack(2001, 2, 3, 4, 5, 31));         /* second 62 */
    set_and_read("T15", pack(2001, 13, 3, 4, 5, 3));         /* month 13 */
    set_and_read("T16", pack(2001, 15, 3, 4, 5, 3));         /* month 15 */
    set_and_read("T17", pack(2001, 0, 0, 4, 5, 3));          /* month 0 day 0 */
    set_and_read("T18", pack(2001, 2, 30, 4, 5, 3));         /* Feb 30 */
    set_and_read("T19", pack(2001, 2, 29, 4, 5, 3));         /* Feb 29, no leap */
    set_and_read("T20", pack(2001, 4, 31, 4, 5, 3));         /* Apr 31 */
    set_and_read("T21", 0UL);                                /* all zero */
    set_and_read("T22", 0xFFFFFFFFUL);                       /* all ones */
    set_and_roll("T23", pack(2001, 2, 3, 23, 59, 31), 6000); /* 23:59:62 rolls */
    set_and_roll("T24", pack(2001, 2, 3, 4, 5, 31), 6000);   /* 04:05:62 rolls */
    set_and_roll("T25", pack(2001, 2, 3, 25, 0, 0), 6000);   /* hour 25 rolls */
    set_and_roll("T26", pack(2001, 2, 3, 4, 61, 29), 6000);  /* 04:61:58 rolls */
    set_and_roll("T27", pack(2001, 2, 30, 23, 59, 29), 6000);/* Feb 30 day roll */
    set_and_roll("T28", pack(2001, 0, 0, 4, 5, 3), 6000);    /* month 0 day 0 */
    /* GEMDOS setters commit through the clock Gettime reads */
    Settime((long)pack(2001, 2, 3, 4, 5, 3));
    line("T30", "Tsettime(10:20:30)", (long)Tsettime((10 << 11) | (20 << 5) | 15));
    dec("T30", "Gettime", lo32(gt()));
    line("T31", "Tsetdate(2002-03-04)", (long)Tsetdate(((2002 - 1980) << 9) | (3 << 5) | 4));
    dec("T31", "Gettime", lo32(gt()));
    line("T32", "Tsettime(24:00:00) invalid", (long)Tsettime(24 << 11));
    dec("T32", "Gettime", lo32(gt()));
    line("T33", "Tsetdate(2001-02-29) invalid", (long)Tsetdate(((2001 - 1980) << 9) | (2 << 5) | 29));
    dec("T33", "Gettime", lo32(gt()));

    /* restore: the start value plus the elapsed time */
    {
        unsigned long back = add_secs(g0, (now_ticks() - t_start) / 200);
        Settime((long)back);
        dec("T99 info", "restored-to", back);
        dec("T99 info", "Gettime-after-restore", lo32(gt()));
    }

    sprintf(b, "END\n");
    emit("%s", b);
    flush_to("XSPROBE.LOG");
    return 0;
}
