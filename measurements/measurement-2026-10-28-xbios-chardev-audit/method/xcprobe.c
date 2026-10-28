/*
 * xcprobe.c -- ad-hoc probe for the audit docs/audits/2026-10-28-xbios-chardev.md
 * (XBIOS character devices and keyboard: Midiws 0x0C, Iorec 0x0E, Rsconf 0x0F,
 * Keytbl 0x10, Bioskeys 0x18, Ikbdws 0x19, Kbdvbase 0x22, Kbrate 0x23,
 * Bconmap 0x2C).
 *
 * NOT tosdump: it measures what the DUMPBIOS guard refuses
 * (tosdump/core/cases.c call_policy) -- the set arms of Rsconf, Keytbl, Kbrate,
 * Bconmap, the calls Midiws/Bioskeys/Ikbdws -- and what a pointer result
 * points at (Iorec, Keytbl, Kbdvbase, Bconmap(-2)), which PTR_RAM cannot see.
 * Same C source built twice (m68k-atari-mint-gcc + mintlib for EmuTOS on
 * Hatari; clang aarch64 + the SDK's mintlib for NewTOS), calls through
 * mintlib's osbind.h macros and trap_14_* helpers, as a ported program does;
 * the structures are mintlib's (_IOREC, _KEYTAB, _KBDVECS, _BCONMAP, _MAPTAB).
 *
 * Output (current directory):
 *   XCPROBE.LOG   groups R (Kbrate), S (Rsconf), B (Bconmap), I (Iorec),
 *                 K (Keytbl/Bioskeys), V (Kbdvbase), W (Ikbdws -> kbdvecs
 *                 hooks), M (Midiws)
 * Variants (-DVARIANT=n), each a separate program, run last-thing:
 *   1  XCIOFL.LOG  the keyboard-flush idiom  io = Iorec(1); io->ibufhd =
 *                  io->ibuftl;  in user mode ("before" line flushed, then the
 *                  write, then "after")
 *   2  XCKTNL.LOG  Keytbl(NULL,-1,-1) then Bioskeys() ("before"/"after")
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>
 * (m68k: the sign extension of d0, the value (long long)(long) gives).
 */
#include <osbind.h>
#include <stdarg.h>
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

static void flush_to(const char *name, int append)
{
    long fh;
    if (append) {
        fh = Fopen(name, 1);
        if (fh >= 0)
            Fseek(0L, (short)fh, 2);
    } else {
        fh = Fcreate(name, 0);
    }
    if (fh < 0)
        return;
    Fwrite((short)fh, (long)olen, out);
    Fclose((short)fh);
    olen = 0;
}

static unsigned long lo32(long r) { return (unsigned long)r & 0xFFFFFFFFUL; }
static unsigned long hi32(long r)
{
    if (sizeof(long) > 4)
        return ((unsigned long)r >> 16 >> 16) & 0xFFFFFFFFUL;
    return r < 0 ? 0xFFFFFFFFUL : 0UL;
}

U static void line(const char *id, const char *call, long r)
{
    emit("%s %s ret=%08lx hi=%08lx\n", id, call, lo32(r), hi32(r));
}

/* A byte address (keyboard tables may be odd): non-null, below 2 GiB. */
U static int tosbyte(long v)
{
    return v != 0 && (unsigned long)v < 0x80000000UL;
}

/* An address as a TOS LONG: non-null, even, below 2 GiB. */
U static int tosaddr(long v)
{
    return v != 0 && (v & 1) == 0 && (unsigned long)v < 0x80000000UL;
}

U static unsigned long crc32(const unsigned char *p, int n)
{
    unsigned long c = 0xFFFFFFFFUL;
    int i, k;
    for (i = 0; i < n; i++) {
        c ^= p[i];
        for (k = 0; k < 8; k++)
            c = (c >> 1) ^ (0xEDB88320UL & (0UL - (c & 1)));
    }
    return (c ^ 0xFFFFFFFFUL) & 0xFFFFFFFFUL;
}

/* ---- Supexec: _hz_200 ($4BA, protected on TOS) -------------------------- */
static volatile long sv_c;
U static long hz200(void)
{
    sv_c = *(volatile long *)0x4BA;
    return sv_c;
}
U static void wait_ticks(long n)
{
    long t0 = Supexec(hz200), t1 = t0, i;
    for (i = 0; i < 50000000L && t1 - t0 < n; i++)
        t1 = Supexec(hz200);
}

#define KBRATE(a, b) ((long)trap_14_www((short)0x23, (short)(a), (short)(b)))
#define RSCONF(a, b, c, d, e, f) \
    ((long)trap_14_wwwwwww((short)0x0F, (short)(a), (short)(b), (short)(c), \
                           (short)(d), (short)(e), (short)(f)))
#define BCONMAP(d)  ((long)trap_14_ww((short)0x2C, (short)(d)))
#define IOREC(d)    ((long)trap_14_ww((short)0x0E, (short)(d)))
#define KEYTBL(a, b, c) ((long)trap_14_wlll((short)0x10, (long)(a), (long)(b), (long)(c)))
#define BIOSKEYS()  ((long)trap_14_w((short)0x18))
#define KBDVBASE()  ((long)trap_14_w((short)0x22))
#define IKBDWS(n, p) ((long)trap_14_wwl((short)0x19, (short)(n), (long)(p)))
#define MIDIWS(n, p) ((long)trap_14_wwl((short)0x0C, (short)(n), (long)(p)))
#define BCOSTAT(d)  ((long)trap_13_ww((short)8, (short)(d)))

#ifndef VARIANT
/* ---- R: Kbrate ----------------------------------------------------------- */
static void group_r(void)
{
    long r0 = KBRATE(-1, -1);
    line("R01", "Kbrate(-1,-1)", r0);
    line("R02", "Kbrate(10,5)", KBRATE(10, 5));
    line("R03", "Kbrate(-1,-1)", KBRATE(-1, -1));
    line("R04", "Kbrate(0x1FF,0x1FF)", KBRATE(0x1FF, 0x1FF));
    line("R05", "Kbrate(-1,-1)", KBRATE(-1, -1));
    line("R06", "Kbrate(-2,3)", KBRATE(-2, 3));
    line("R07", "Kbrate(-1,-1)", KBRATE(-1, -1));
    line("R08", "Kbrate(0x100,-32768)", KBRATE(0x100, -32768));
    line("R09", "Kbrate(-1,-1)", KBRATE(-1, -1));
    line("R10", "Kbrate(0,0)", KBRATE(0, 0));
    line("R11", "Kbrate(-1,-1)", KBRATE(-1, -1));
    line("R12", "Kbrate(r0) restore", KBRATE((r0 >> 8) & 0xFF, r0 & 0xFF));
    line("R13", "Kbrate(-1,-1)", KBRATE(-1, -1));
}

/* ---- S: Rsconf ----------------------------------------------------------- */
static void group_s(void)
{
    long b0 = RSCONF(-2, -1, -1, -1, -1, -1);
    long g0 = RSCONF(-1, -1, -1, -1, -1, -1);
    line("S01", "Rsconf(-2,..) baud", b0);
    line("S02", "Rsconf(-1 x6) regs", g0);
    line("S03", "Rsconf(-2,2,..) baud, ctrl ignored", RSCONF(-2, 2, -1, -1, -1, -1));
    line("S04", "Rsconf(4,-1..) set 2400", RSCONF(4, -1, -1, -1, -1, -1));
    line("S05", "Rsconf(-2,..)", RSCONF(-2, -1, -1, -1, -1, -1));
    line("S06", "Rsconf(16,-1..) out of range", RSCONF(16, -1, -1, -1, -1, -1));
    line("S07", "Rsconf(-2,..)", RSCONF(-2, -1, -1, -1, -1, -1));
    line("S08", "Rsconf(-3,-1..)", RSCONF(-3, -1, -1, -1, -1, -1));
    line("S09", "Rsconf(-2,..)", RSCONF(-2, -1, -1, -1, -1, -1));
    line("S10", "Rsconf(-1,-1,0x98,..) ucr", RSCONF(-1, -1, 0x98, -1, -1, -1));
    line("S11", "Rsconf(-1 x6)", RSCONF(-1, -1, -1, -1, -1, -1));
    line("S12", "Rsconf(-1,-1,-1,0,-1,-1) rsr=0", RSCONF(-1, -1, -1, 0, -1, -1));
    line("S13", "Rsconf(-1 x6)", RSCONF(-1, -1, -1, -1, -1, -1));
    line("S14", "Rsconf(-1,-1,0x188,-1..) ucr high byte", RSCONF(-1, -1, 0x188, -1, -1, -1));
    line("S15", "Rsconf(-1 x6)", RSCONF(-1, -1, -1, -1, -1, -1));
    /* restore: ucr/rsr/tsr from g0, baud from b0 */
    line("S16", "Rsconf(b0,-1,ucr0,rsr0,tsr0,-1) restore",
         RSCONF(b0 & 0xFFFF, -1, (g0 >> 24) & 0xFF, (g0 >> 16) & 0xFF, (g0 >> 8) & 0xFF, -1));
    line("S17", "Rsconf(-1 x6)", RSCONF(-1, -1, -1, -1, -1, -1));
    line("S18", "Rsconf(-2,..)", RSCONF(-2, -1, -1, -1, -1, -1));
}

/* ---- B: Bconmap ---------------------------------------------------------- */
static void group_b(void)
{
    long d0 = BCONMAP(-1);
    long p = BCONMAP(-2);
    line("B01", "Bconmap(-1)", d0);
    line("B02", "Bconmap(-2)", p);
    emit("B03 Bconmap(-2) tosaddr=%d\n", tosaddr(p));
    if (tosaddr(p)) {
        _BCONMAP *bm = (_BCONMAP *)p;
        long mt = (long)bm->maptab;
        emit("B04 BCONMAP maptab_tosaddr=%d maptabsize=%d mapped_device=%d sizeof(_BCONMAP)=%d\n",
             tosaddr(mt), bm->maptabsize, ((short *)&bm->maptabsize)[1], (int)sizeof(_BCONMAP));
        if (tosaddr(mt)) {
            int i;
            for (i = 0; i < bm->maptabsize && i < 4; i++) {
                _MAPTAB *m = &bm->maptab[i];
                emit("B05 maptab[%d] fn_tosaddr=%d%d%d%d%d iorec_tosaddr=%d\n", i,
                     tosaddr((long)m->Bconstat), tosaddr((long)m->Bconin),
                     tosaddr((long)m->Bcostat), tosaddr((long)m->Bconout),
                     tosaddr((long)m->Rsconf), tosaddr((long)m->iorec));
            }
        }
    }
    line("B06", "Bconmap(0)", BCONMAP(0));
    line("B07", "Bconmap(1)", BCONMAP(1));
    line("B08", "Bconmap(5)", BCONMAP(5));
    line("B09", "Bconmap(10)", BCONMAP(10));
    line("B10", "Bconmap(-3)", BCONMAP(-3));
    line("B11", "Bconmap(0x7FFF)", BCONMAP(0x7FFF));
    line("B12", "Bconmap(-32768)", BCONMAP(-32768));
    {
        long io6 = IOREC(0), b6 = RSCONF(-2, -1, -1, -1, -1, -1);
        line("B13", "Bconmap(7)", BCONMAP(7));
        line("B14", "Bconmap(-1)", BCONMAP(-1));
        emit("B15 after Bconmap(7): Iorec(0)_changed=%d\n", IOREC(0) != io6);
        line("B16", "Rsconf(-2,..) after Bconmap(7)", RSCONF(-2, -1, -1, -1, -1, -1));
        line("B17", "Bconmap(6)", BCONMAP(6));
        line("B18", "Bconmap(-1)", BCONMAP(-1));
        emit("B19 after Bconmap(6): Iorec(0)_is_first=%d Rsconf(-2)_is_first=%d\n",
             IOREC(0) == io6, RSCONF(-2, -1, -1, -1, -1, -1) == b6);
        line("B20", "Bconmap(8)", BCONMAP(8));
        line("B21", "Bconmap(9)", BCONMAP(9));
        line("B22", "Bconmap(-1)", BCONMAP(-1));
        line("B23", "Bconmap(d0) restore", BCONMAP(d0 & 0xFFFF));
        line("B24", "Bconmap(-1)", BCONMAP(-1));
    }
}

/* ---- I: Iorec ------------------------------------------------------------ */
static void iorec_show(const char *id, int dev)
{
    long r = IOREC(dev);
    char c[32];
    snprintf(c, sizeof c, "Iorec(%d)", dev);
    line(id, c, r);
    emit("%sa %s tosaddr=%d\n", id, c, tosaddr(r));
    if (tosaddr(r)) {
        _IOREC *io = (_IOREC *)r;
        emit("%sb IOREC ibuf_tosaddr=%d ibufsiz=%d hd=%d tl=%d low=%d hi=%d sizeof=%d\n", id,
             tosaddr((long)io->ibuf), io->ibufsiz, io->ibufhd, io->ibuftl,
             io->ibuflow, io->ibufhi, (int)sizeof(_IOREC));
        if (dev == 0) {
            _IOREC *o = io + 1;
            unsigned char *x = (unsigned char *)(o + 1);
            emit("%sc out-IOREC obuf_tosaddr=%d obufsiz=%d hd=%d tl=%d low=%d hi=%d"
                 " ext baud=%d flow=%d ucr=%02x datamask=%02x\n", id,
                 tosaddr((long)o->ibuf), o->ibufsiz, o->ibufhd, o->ibuftl,
                 o->ibuflow, o->ibufhi, x[0], x[1], x[2], x[3]);
        }
    }
}

static void group_i(void)
{
    iorec_show("I01", 0);
    iorec_show("I02", 1);
    iorec_show("I03", 2);
    line("I04", "Iorec(3)", IOREC(3));
    line("I05", "Iorec(-1)", IOREC(-1));
    line("I06", "Iorec(0x7FFF)", IOREC(0x7FFF));
    line("I07", "Iorec(-32768)", IOREC(-32768));
    emit("I08 Iorec(1) stable=%d\n", IOREC(1) == IOREC(1));
}

/* ---- K: Keytbl / Bioskeys ------------------------------------------------ */
static unsigned char my_n[128], my_s[128], my_c[128];

static int pairlen(const unsigned char *p)
{
    int n = 0;
    while (n < 200 && p[n] != 0)
        n += 2;
    return n;
}

static void keytab_show(const char *id, long r)
{
    _KEYTAB *k = (_KEYTAB *)r;
    long *raw = (long *)r;
    emit("%s keytab ptrs_tosbyte=%d%d%d%d%d%d f7=%08lx%08lx w_after_f7=%04x sizeof(ptr)=%d\n", id,
         tosbyte((long)k->unshift), tosbyte((long)k->shift), tosbyte((long)k->caps),
         tosbyte((long)k->alt), tosbyte((long)k->altshift), tosbyte((long)k->altcaps),
         hi32(raw[6]), lo32(raw[6]), *(unsigned short *)&raw[7], (int)sizeof(void *));
    if (tosbyte((long)k->unshift) && tosbyte((long)k->shift) && tosbyte((long)k->caps))
        emit("%s tables crc norm=%08lx shft=%08lx caps=%08lx norm[0x10..0x13]=%02x%02x%02x%02x"
             " shft[0x1E]=%02x caps[0x2C]=%02x\n", id,
             crc32((unsigned char *)k->unshift, 128), crc32((unsigned char *)k->shift, 128),
             crc32((unsigned char *)k->caps, 128),
             (unsigned char)k->unshift[0x10], (unsigned char)k->unshift[0x11],
             (unsigned char)k->unshift[0x12], (unsigned char)k->unshift[0x13],
             (unsigned char)k->shift[0x1E], (unsigned char)k->caps[0x2C]);
    if (tosbyte((long)k->alt) && tosbyte((long)k->altshift) && tosbyte((long)k->altcaps)) {
        int a = pairlen((unsigned char *)k->alt), b = pairlen((unsigned char *)k->altshift),
            c = pairlen((unsigned char *)k->altcaps);
        emit("%s alt pairs len=%d,%d,%d crc=%08lx,%08lx,%08lx\n", id, a, b, c,
             crc32((unsigned char *)k->alt, a), crc32((unsigned char *)k->altshift, b),
             crc32((unsigned char *)k->altcaps, c));
    }
}

static void group_k(void)
{
    long p = KEYTBL(-1, -1, -1), p2, q;
    _KEYTAB *k;
    char *n0, *s0, *c0;
    int i;
    line("K01", "Keytbl(-1,-1,-1)", p);
    emit("K01a tosaddr=%d\n", tosaddr(p));
    if (!tosaddr(p))
        return;
    k = (_KEYTAB *)p;
    keytab_show("K02", p);
    n0 = k->unshift; s0 = k->shift; c0 = k->caps;
    p2 = KEYTBL(-1, -1, -1);
    emit("K03 second Keytbl(-1,-1,-1) same=%d\n", p2 == p);
    for (i = 0; i < 128; i++) {
        my_n[i] = (unsigned char)(i ^ 0x55);
        my_s[i] = (unsigned char)(i ^ 0x2A);
        my_c[i] = (unsigned char)(i ^ 0x7F);
    }
    q = KEYTBL(my_n, -1, -1);
    k = (_KEYTAB *)q;
    emit("K04 Keytbl(my_n,-1,-1) same_ptr=%d unshift_is_my=%d shift_is_s0=%d caps_is_c0=%d\n",
         q == p, k->unshift == (char *)my_n, k->shift == s0, k->caps == c0);
    q = KEYTBL(-1, -1, -1);
    k = (_KEYTAB *)q;
    emit("K05 Keytbl(-1,-1,-1) unshift_is_my=%d shift_is_s0=%d caps_is_c0=%d\n",
         k->unshift == (char *)my_n, k->shift == s0, k->caps == c0);
    q = KEYTBL(-1, my_s, my_c);
    k = (_KEYTAB *)q;
    emit("K06 Keytbl(-1,my_s,my_c) unshift_is_my=%d shift_is_my=%d caps_is_my=%d alt_tosbyte=%d\n",
         k->unshift == (char *)my_n, k->shift == (char *)my_s, k->caps == (char *)my_c,
         tosbyte((long)k->alt));
    line("K07", "Bioskeys()", BIOSKEYS());
    q = KEYTBL(-1, -1, -1);
    k = (_KEYTAB *)q;
    emit("K08 after Bioskeys: same_ptr=%d unshift_is_n0=%d shift_is_s0=%d caps_is_c0=%d"
         " unshift_is_my=%d\n", q == p, k->unshift == n0, k->shift == s0, k->caps == c0,
         k->unshift == (char *)my_n);
    keytab_show("K09", q);
    /* make sure the defaults are back whatever Bioskeys did */
    q = KEYTBL(n0, s0, c0);
    k = (_KEYTAB *)q;
    emit("K10 Keytbl(n0,s0,c0) restore unshift_is_n0=%d shift_is_s0=%d caps_is_c0=%d\n",
         k->unshift == n0, k->shift == s0, k->caps == c0);
    line("K11", "Bioskeys() final", BIOSKEYS());
}

/* ---- V/W: Kbdvbase, Ikbdws ------------------------------------------------ */
static volatile long n_stat, n_clock, n_joy;
static volatile unsigned char clk[6], st[7];
U static void h_stat(void *p)
{
    int i;
    for (i = 0; i < 7; i++)
        st[i] = ((unsigned char *)p)[i];
    n_stat++;
}
U static void h_clock(void *p)
{
    int i;
    for (i = 0; i < 6; i++)
        clk[i] = ((unsigned char *)p)[i];
    n_clock++;
}
U static void h_joy(void *p) { (void)p; n_joy++; }

static void group_v(void)
{
    long p = KBDVBASE();
    _KBDVECS *kv;
    long *slots;
    int i, ok = 1, distinct = 0;
    line("V01", "Kbdvbase()", p);
    emit("V01a tosaddr=%d second_same=%d\n", tosaddr(p), KBDVBASE() == p);
    if (!tosaddr(p))
        return;
    kv = (_KBDVECS *)p;
    slots = (long *)p;
    for (i = 0; i < 9; i++) {
        int j, seen = 0;
        ok &= tosaddr(slots[i]);
        for (j = 0; j < i; j++)
            seen |= slots[j] == slots[i];
        distinct += !seen;
    }
    emit("V02 slots_all_tosaddr=%d distinct_values=%d kbstate=%d kbdvec(p[-1])_tosaddr=%d"
         " midivec==vkbderr=%d\n",
         ok, distinct, kv->kbstate, tosaddr(slots[-1]), slots[0] == slots[1]);
    {
        /* W: IKBD commands whose answer is a packet the IKBD vectors get. */
        void (*o_stat)(void *) = kv->statvec;
        void (*o_clock)(void *) = kv->clockvec;
        void (*o_joy)(void *) = kv->joyvec;
        static const unsigned char c_clock[1] = { 0x1C };  /* interrogate clock */
        static const unsigned char c_stat[1] = { 0x88 };   /* inquire mouse mode */
        static const unsigned char c_joy[1] = { 0x16 };    /* joystick interrogate */
        n_stat = n_clock = n_joy = 0;
        kv->statvec = h_stat;
        kv->clockvec = h_clock;
        kv->joyvec = h_joy;
        line("W01", "Ikbdws(0,{0x1C})", IKBDWS(0, c_clock));
        wait_ticks(100);
        line("W02", "Ikbdws(0,{0x88})", IKBDWS(0, c_stat));
        wait_ticks(100);
        line("W03", "Ikbdws(0,{0x16})", IKBDWS(0, c_joy));
        wait_ticks(100);
        kv->statvec = o_stat;
        kv->clockvec = o_clock;
        kv->joyvec = o_joy;
        emit("W04 hooks after 3x100 ticks: clockvec=%ld statvec=%ld joyvec=%ld"
             " st=%02x%02x%02x%02x%02x%02x%02x\n",
             (long)n_clock, (long)n_stat, (long)n_joy,
             st[0], st[1], st[2], st[3], st[4], st[5], st[6]);
        emit("V03 vectors restored=%d\n",
             kv->statvec == o_stat && kv->clockvec == o_clock && kv->joyvec == o_joy);
    }
}

/* ---- M: Midiws ----------------------------------------------------------- */
static void group_m(void)
{
    static unsigned char buf[256];
    long t0, t1;
    int i;
    for (i = 0; i < 256; i++)
        buf[i] = 0xFE;      /* MIDI active sensing */
    line("M01", "Bcostat(3) before", BCOSTAT(3));
    line("M02", "Midiws(0,{0xFE})", MIDIWS(0, buf));
    t0 = Supexec(hz200);
    line("M03", "Midiws(255,256x0xFE)", MIDIWS(255, buf));
    t1 = Supexec(hz200);
    emit("M04 Midiws(255) took_ticks_ge_10=%d\n", t1 - t0 >= 10);
    line("M05", "Bcostat(3) after", BCOSTAT(3));
}

int main(void)
{
    group_r();
    group_s();
    group_b();
    group_i();
    group_k();
    group_v();
    group_m();
    emit("XCPROBE done\n");
    flush_to("XCPROBE.LOG", 0);
    return 0;
}

#elif VARIANT == 1
int main(void)
{
    long r = IOREC(1);
    _IOREC *io = (_IOREC *)r;
    emit("F01 Iorec(1) ret=%08lx hi=%08lx tosaddr=%d -- before user-mode write\n",
         lo32(r), hi32(r), tosaddr(r));
    flush_to("XCIOFL.LOG", 0);
    io->ibufhd = io->ibuftl;        /* the keyboard-flush idiom */
    emit("F02 after write hd=%d tl=%d\n", io->ibufhd, io->ibuftl);
    flush_to("XCIOFL.LOG", 1);
    return 0;
}

#elif VARIANT == 2
int main(void)
{
    long r;
    emit("N01 before Keytbl(NULL,-1,-1)\n");
    flush_to("XCKTNL.LOG", 0);
    r = KEYTBL(0L, -1, -1);
    emit("N02 Keytbl(NULL,-1,-1) returned ret=%08lx hi=%08lx\n", lo32(r), hi32(r));
    flush_to("XCKTNL.LOG", 1);
    BIOSKEYS();
    r = KEYTBL(-1, -1, -1);
    emit("N03 after Bioskeys: unshift_tosbyte=%d\n", tosbyte((long)((_KEYTAB *)r)->unshift));
    flush_to("XCKTNL.LOG", 1);
    return 0;
}
#endif
