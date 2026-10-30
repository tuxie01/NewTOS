/*
 * bcprobe.c -- ad-hoc probe for the audit docs/audits/2026-10-28-bios-core.md
 * (BIOS trap #13 without disk: Getmpb, Bconstat, Bconin, Bconout, Setexc,
 * Tickcal, Bcostat, Kbshift).
 *
 * NOT tosdump: it measures the arms the DUMPBIOS guard refuses
 * (tosdump/core/cases.c call_policy) -- Getmpb, the set arms of Setexc and
 * Kbshift, Bconout to PRT/AUX/MIDI/IKBD/Bconmap, Bconin on handles that may
 * block -- and the dispatch of an installed etv_timer hook.  Same C source
 * built twice (m68k-atari-mint-gcc + mintlib for EmuTOS on Hatari; clang
 * aarch64 + the SDK's mintlib for NewTOS), calling the traps through
 * mintlib's trap_13_* / trap_14_* helpers as a ported program does.
 * Evidence class MEASURED at best (no tosdiff); every finding names the
 * tosdump case that would confirm it.
 *
 * Output (current directory), one line per case:
 *   BCPROBE.LOG  groups K (Kbshift), S (Setexc), T (etv_timer dispatch),
 *                B (character devices) -- written after all their calls
 *   BCPROB2.LOG  group G (Getmpb) -- run last: on EmuTOS Getmpb rewrites the
 *                live free-list descriptor themd (bios/biosmem.c:143-150)
 * Build with -DHANGLIST=a,b,... for the blocking-Bconin variant instead:
 *   BCHANG.LOG   per handle "H<n> before" (file closed), then Bconin(n), then
 *                "H<n> after ret=..." -- a missing "after" line = the call
 *                did not return before the harness killed the machine.
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>
 * (m68k: the sign extension of d0, the value (long long)(long) gives).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define U __attribute__((unused))
static char out[16000];
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

/* An address as a TOS LONG: non-null, even, below 2 GiB. */
U static int tosaddr(long v)
{
    return v != 0 && (v & 1) == 0 && (unsigned long)v < 0x80000000UL;
}

/* ---- Supexec callbacks: read system variables in supervisor/super state */
static volatile long sv_a, sv_b, sv_c;
U static long rd_mem(void)
{
    sv_a = *(volatile long *)0x432;     /* _membot */
    sv_b = *(volatile long *)0x436;     /* _memtop */
    sv_c = *(volatile long *)0x4BA;     /* _hz_200 */
    return 0;
}
U static long hz200(void)
{
    rd_mem();
    return sv_c;
}

/* ---- etv_timer / dummy handlers ------------------------------------------ */
static volatile long ticks;
U static void tick_hook(void) { ticks++; }
U static void dummy_a(void) { ticks += 1000; }
U static void dummy_b(void) { ticks += 2000; }

#define SETEXC(n, v) trap_13_wwl((short)5, (short)(n), (long)(v))
#define KBSHIFT(f)   trap_13_ww((short)11, (short)(f))

#ifndef HANGLIST
static void group_k(void)
{
    long s0 = KBSHIFT(-1);
    line("K01", "Kbshift(-1)", s0);
    line("K02", "Kbshift(0x10)", KBSHIFT(0x10));
    line("K03", "Kbshift(-1)", KBSHIFT(-1));
    line("K04", "Kbshift(0x1FF)", KBSHIFT(0x1FF));
    line("K05", "Kbshift(-1)", KBSHIFT(-1));
    line("K06", "Kbshift(-2)", KBSHIFT(-2));
    line("K07", "Kbshift(-1)", KBSHIFT(-1));
    line("K08", "Kbshift(0x7F81)", KBSHIFT(0x7F81));
    line("K09", "Kbshift(-1)", KBSHIFT(-1));
    line("K10", "Kbshift(-32768)", KBSHIFT(-32768));
    line("K11", "Kbshift(-1)", KBSHIFT(-1));
    line("K12", "Kbshift(s0) restore", KBSHIFT(s0 & 0xFFFF));
    line("K13", "Kbshift(-1)", KBSHIFT(-1));
}

static void setcycle(const char *id, int n, long nv)
{
    char c[64];
    long old = SETEXC(n, nv);
    long q1 = SETEXC(n, -1);
    long back = SETEXC(n, old);
    long q2 = SETEXC(n, -1);
    snprintf(c, sizeof c, "Setexc(0x%X,new)", n);
    emit("%s %s old_ok=%d old_hi=%08lx q1_is_new=%d back_is_new=%d q2_is_old=%d\n",
         id, c, tosaddr(old), hi32(old), q1 == nv, back == nv, q2 == old);
}

static void group_s(void)
{
    setcycle("S01", 0x102, (long)dummy_a);  /* etv_term   */
    setcycle("S02", 0x101, (long)dummy_b);  /* etv_critic */
    setcycle("S03", 0x100, (long)dummy_a);  /* etv_timer  */
    setcycle("S04", 0x2, (long)dummy_b);    /* bus error  */
    setcycle("S05", 0x4, (long)dummy_a);    /* illegal    */
    setcycle("S06", 0x16A, 0x11112222L);    /* ramvalid $5A8 (read at reset only) */
    setcycle("S07", 0x108, 0x33334444L);    /* memvalid $420 (read at reset only) */
    /* The low-memory cell itself, read in super state, after S07's restore. */
    {
        long q = SETEXC(0x108, -1);
        line("S08", "Setexc(0x108,-1) after restore", q);
    }
}

static void group_t(void)
{
    long old, t0, t1, i;
    ticks = 0;
    t0 = Supexec(hz200);
    old = SETEXC(0x100, (long)tick_hook);
    for (i = 0, t1 = t0; i < 20000000L && t1 - t0 < 400; i++)
        t1 = Supexec(hz200);
    SETEXC(0x100, old);
    emit("T01 etv_timer hook over %ld hz_200 ticks: calls=%ld (calls>0: %d) loops=%ld\n",
         t1 - t0, (long)ticks, ticks > 0, i);
}

static void group_b(void)
{
    line("B01", "Bconout(1,'A') AUX", trap_13_www((short)3, (short)1, (short)'A'));
    line("B02", "Bconout(3,0xFE) MIDI", trap_13_www((short)3, (short)3, (short)0xFE));
    line("B03", "Bconout(4,0x11) IKBD resume", trap_13_www((short)3, (short)4, (short)0x11));
    line("B04", "Bconout(6,'A') Bconmap 6", trap_13_www((short)3, (short)6, (short)'A'));
    line("B05", "Bconout(7,'A') Bconmap 7", trap_13_www((short)3, (short)7, (short)'A'));
    line("B06", "Bconout(2,7) BEL", trap_13_www((short)3, (short)2, (short)7));
    line("B07", "Bconout(2,0x141) high byte", trap_13_www((short)3, (short)2, (short)0x141));
    line("B08", "Bconout(5,0x0A) raw LF", trap_13_www((short)3, (short)5, (short)0x0A));
    line("B09", "Bconin(0) PRT", trap_13_ww((short)2, (short)0));
    line("B10", "Bconstat(1) AUX", trap_13_ww((short)1, (short)1));
    line("B11", "Bcostat(3) after Bconout(4)", trap_13_ww((short)8, (short)3));
    line("B12", "Bcostat(4) after Bconout(3)", trap_13_ww((short)8, (short)4));
    line("B13", "Tickcal()", trap_13_w((short)6));
    line("B14", "Tickcal() again", trap_13_w((short)6));
    /* last: EmuTOS waits up to 30 s for a printer (bios/parport.c:150-168) */
    line("B15", "Bconout(0,'A') PRT", trap_13_www((short)3, (short)0, (short)'A'));
    line("B16", "Bcostat(0) PRT", trap_13_ww((short)8, (short)0));
}

typedef struct {
    _MD *mp_free, *mp_used, *mp_rover;
} XMPB;

/* The descriptor Getmpb hands out lies in protected low memory on TOS
 * (EmuTOS themd = $48E, a user-mode read is a bus error -- measured, run
 * 1): copy it in supervisor/super state, as a TOS program has to. */
static _MD *g_md;
static _MD g_copy;
U static long cp_md(void)
{
    memcpy(&g_copy, g_md, sizeof g_copy);
    return 0;
}

static void group_g(void)
{
    static XMPB mpb;
    static _MD sentinel;
    long r;

    memset(&mpb, 0xA5, sizeof mpb);
    mpb.mp_rover = &sentinel;
    Supexec(rd_mem);
    emit("G00 sizeof(_MD)=%d sizeof(XMPB)=%d membot=%08lx memtop=%08lx\n",
         (int)sizeof(_MD), (int)sizeof(XMPB), lo32(sv_a), lo32(sv_b));
    r = trap_13_wl((short)0, (long)&mpb);
    emit("G01 Getmpb(&mpb) ret=%08lx hi=%08lx (void: recorded, not compared)\n",
         lo32(r), hi32(r));
    emit("G02 mp_free_ok=%d mp_free_hi=%08lx mp_used_null=%d mp_rover_kept=%d\n",
         tosaddr((long)mpb.mp_free), hi32((long)mpb.mp_free), mpb.mp_used == 0,
         mpb.mp_rover == &sentinel);
    if (tosaddr((long)mpb.mp_free)) {
        const unsigned char *b = (const unsigned char *)&g_copy;
        int i;
        g_md = mpb.mp_free;
        Supexec(cp_md);
        emit("G03 md_next_null=%d md_owner_null=%d md_start=%08lx/%08lx md_length=%08lx/%08lx\n",
             g_copy.md_next == 0, g_copy.md_owner == 0, hi32(g_copy.md_start),
             lo32(g_copy.md_start), hi32(g_copy.md_length), lo32(g_copy.md_length));
        emit("G04 start_ok=%d length_pos=%d end_le_2G=%d start_is_membot=%d end_is_memtop=%d\n",
             tosaddr(g_copy.md_start), g_copy.md_length > 0,
             (unsigned long)g_copy.md_start + (unsigned long)g_copy.md_length <= 0x80000000UL,
             g_copy.md_start == sv_a, g_copy.md_start + g_copy.md_length == sv_b);
        emit("G05 raw=");
        for (i = 0; i < (int)sizeof(_MD); i++)
            emit("%02x", b[i]);
        emit("\n");
        {
            XMPB m2;
            long st = g_copy.md_start;
            memset(&m2, 0, sizeof m2);
            trap_13_wl((short)0, (long)&m2);
            if (tosaddr((long)m2.mp_free)) {
                g_md = m2.mp_free;
                Supexec(cp_md);
            }
            emit("G06 second Getmpb: same_free=%d same_start=%d\n",
                 m2.mp_free == mpb.mp_free, g_copy.md_start == st);
        }
    }
}
#else
static const int hl[] = { HANGLIST };

static void group_h(void)
{
    unsigned i;
    char id[16];
    for (i = 0; i < sizeof hl / sizeof hl[0]; i++) {
        long r;
        snprintf(id, sizeof id, "H%d", hl[i]);
        emit("%s before Bconin(%d)\n", id, hl[i]);
        flush_to("BCHANG.LOG", i != 0);
        r = trap_13_ww((short)2, (short)hl[i]);
        emit("%s after ret=%08lx hi=%08lx\n", id, lo32(r), hi32(r));
        flush_to("BCHANG.LOG", 1);
    }
}
#endif

int main(void)
{
#ifndef HANGLIST
    group_k();
    group_s();
    group_t();
    group_b();
    flush_to("BCPROBE.LOG", 0);
    group_g();
    flush_to("BCPROB2.LOG", 0);
    Cconws("BCPROBE done\r\n");
#else
    group_h();
    Cconws("BCPROBE hang variant done\r\n");
#endif
    return 0;
}
