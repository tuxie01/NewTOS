/*
 * gnprobe.c -- TASK/BIOS06 probe: Getmpb through the SDK/mintlib _MD/_MPB.
 *
 *   group G  G00-G06, the Getmpb group of the bios-core audit probe, copied
 *            verbatim (measurement-2026-10-28-bios-core-audit/method/bcprobe.c,
 *            group_g with rd_mem/tosaddr/lo32/hi32/emit/flush_to) -- the O4
 *            acceptance probe of BIOS06.  One change: rd_mem reads the
 *            sysvars as 32-bit int, not long (see there).
 *   group N  Getmpb(NULL) -- user decision O3 (TASK/BIOS06, 2026-10-28):
 *            "measure EmuTOS at mpb == NULL first".  EmuTOS getmpb does not
 *            test the pointer (bios/biosmem.c:134-151) and stores mp_mfl to
 *            address 0, mp_mal to address 4.  The probe records the longs at
 *            0 and 4 (read in super state, Supexec) before and after, and a
 *            "before" line is on disk before the call, so a call that never
 *            returns (bus error, bombs, panic) leaves N01 without N02.
 *
 * Same C source built twice (m68k-atari-mint-gcc + mintlib for EmuTOS on
 * Hatari; clang aarch64 + the SDK's mintlib for NewTOS), method/build.sh.
 * Output (current directory):
 *   GNPROBE.LOG  group G, written after it
 *   GNPROB2.LOG  group N, N00/N01 flushed before the call, N02/N03 after
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define U __attribute__((unused))
static char out[4000];
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

/* An address as a TOS LONG: non-null, even, below 2 GiB. */
U static int tosaddr(long v)
{
    return v != 0 && (v & 1) == 0 && (unsigned long)v < 0x80000000UL;
}

/* ---- Supexec callbacks: read memory in supervisor/super state ----------- */
static volatile long sv_a, sv_b;
U static long rd_mem(void)
{
    /* A TOS LONG is 4 bytes: read it as int (32 bits under both ABIs).
     * bcprobe.c read `long`, 8 bytes under LP64 -- _membot plus _memtop's
     * bytes -- which made its G04 start_is_membot/end_is_memtop 0 on
     * NewTOS whatever the descriptor held (first NewTOS run of this probe,
     * a development run; G00 prints only the low 32 bits). */
    sv_a = *(volatile int *)0x432;      /* _membot */
    sv_b = *(volatile int *)0x436;      /* _memtop */
    return 0;
}

/* group N: the TOS LONGs at addresses 0 and 4 (where EmuTOS' unguarded
 * getmpb would store mp_mfl and mp_mal for mpb == NULL). */
static volatile unsigned long z0, z4;
/* The address comes from a volatile object: a constant 0 would be a null
 * dereference the compiler turns into a trap (gcc -O2 replaced it with
 * TRAP #7 in a development run: "Panic: Exception number 39"). */
static volatile unsigned long zaddr = 0;
U static long rd_zero(void)
{
    unsigned long a = zaddr;
    z0 = *(volatile unsigned int *)a;
    z4 = *(volatile unsigned int *)(a + 4);
    return 0;
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

static void group_n(void)
{
    long r;

    Supexec(rd_zero);
    emit("N00 before: long@0=%08lx long@4=%08lx\n", z0, z4);
    emit("N01 calling Getmpb(NULL)\n");
    flush_to("GNPROB2.LOG", 0);
    r = trap_13_wl((short)0, 0L);
    emit("N02 Getmpb(NULL) returned ret=%08lx hi=%08lx\n", lo32(r), hi32(r));
    flush_to("GNPROB2.LOG", 1);
    Supexec(rd_zero);
    emit("N03 after: long@0=%08lx long@4=%08lx\n", z0, z4);
    flush_to("GNPROB2.LOG", 1);
}

int main(void)
{
    group_g();
    flush_to("GNPROBE.LOG", 0);
    group_n();
    Cconws("GNPROBE done\r\n");
    return 0;
}
