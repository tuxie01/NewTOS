/*
 * bstprobe.c -- ad-hoc BIOS storage probe (Getbpb, Rwabs, Mediach, Drvmap)
 * for the audit docs/audits/2026-10-28-bios-storage.md.
 *
 * NOT tosdump: the DUMPBIOS loader refuses Rwabs/Getbpb/Mediach
 * (tosdump/core/cases.c call_policy) and the oracle has no BIOS block drive.
 * This probe is the same C source built twice -- m68k (EmuTOS on Hatari,
 * m68k-atari-mint-gcc + mintlib) and AArch64 (NewTOS, clang + sdk mintlib) --
 * and calls the traps through mintlib's trap_13_* helpers, as a ported TOS
 * program would.  Its output is evidence for PLAUSIBLE findings and for
 * designing tosdump cases; it can never establish EQUAL (no tosdiff).
 *
 * Output: BSTPROBE.LOG (and BSTPROB2.LOG for the floppy cases) in the
 * current directory, one line per case:
 *   <id> <call> ret=<low 32 bits hex> sx=<1 if the native register is the
 *   sign-extended 32-bit value> [ck=<FNV-1a of the buffer>] [b=<first 16
 *   bytes>] [bpb=<9 words>]
 * The log is written after all BIOS calls (no GEMDOS file access in
 * between).
 *
 * Compile-time: PSIZE = sectors of the C: partition (MBR length),
 *               DSIZE = sectors of the whole device,
 *               IDEUNIT = EmuTOS physical unit of IDE master (2 + 8*2 = 18).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#ifndef PSIZE
#error PSIZE
#endif
#ifndef DSIZE
#error DSIZE
#endif
#ifndef IDEUNIT
#define IDEUNIT 18
#endif
/* The physical unit that holds C:, for the boot-sector patch steps:
 * EmuTOS IDE master = GET_UNITNUM(IDE_BUS, 0) = 2 + 8*2 (bios/disk.h:30,
 * bios/machine.h:41); NewTOS resolves a physical "unit" through the drive
 * letter table, so C: = 2 (measured: P01/P02 of this probe). */
#ifdef __NEWTOS__
#define PHYSUNIT 2
#else
#define PHYSUNIT IDEUNIT
#endif
#ifndef PSTART
#error PSTART
#endif

static char out[24000];
static int olen;
static unsigned char buf[4 * 512] __attribute__((aligned(2)));
static unsigned char sec0[512] __attribute__((aligned(2)));
static unsigned char seclast[512] __attribute__((aligned(2)));

static void emit(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    if (olen < (int)sizeof(out) - 256)
        olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
    va_end(ap);
}

static unsigned long fnv(const unsigned char *p, int n)
{
    unsigned long h = 2166136261UL;
    int i;
    for (i = 0; i < n; i++) {
        h ^= p[i];
        h = (h * 16777619UL) & 0xFFFFFFFFUL;
    }
    return h;
}

static void retline(const char *id, const char *call, long r)
{
    emit("%s %s ret=%08lx sx=%d", id, call, (unsigned long)r & 0xFFFFFFFFUL,
         (int)(r == (long)(int)r));
}

static void bufinfo(int nsec)
{
    int i;
    emit(" ck=%08lx b=", fnv(buf, nsec * 512));
    for (i = 0; i < 16; i++)
        emit("%02x", buf[i]);
}

static void poison(void)
{
    memset(buf, 0xA5, sizeof(buf));
}

/* Rwabs with the long sector (AHDI 3.1 form; first==-1 selects lfirst). */
static long rw(int flag, void *b, int n, int first, int drv, long lfirst)
{
    return trap_13_wwlwwwl((short)4, (short)flag, (long)b, (short)n,
                           (short)first, (short)drv, (long)lfirst);
}

static void rcase(const char *id, int flag, int n, int first, int drv,
                  long lfirst, int show)
{
    char call[96];
    long r;
    poison();
    r = rw(flag, buf, n, first, drv, lfirst);
    snprintf(call, sizeof(call), "Rwabs(%d,buf,%d,%d,%d,%ld)", flag, n, first,
             drv, lfirst);
    retline(id, call, r);
    if (show)
        bufinfo(n > 0 ? n : 1);
    emit("\n");
}

static void gcase(const char *id, int d)
{
    char call[32];
    long r = trap_13_ww((short)7, (short)d);
    snprintf(call, sizeof(call), "Getbpb(%d)", d);
    emit("%s %s nz=%d sx=%d", id, call, r != 0, (int)(r == (long)(int)r));
    if (r != 0) {
        const unsigned short *w = (const unsigned short *)r;
        int i;
        long r2 = trap_13_ww((short)7, (short)d);
        emit(" bpb=");
        for (i = 0; i < 9; i++)
            emit("%u%s", w[i], i < 8 ? "," : "");
        emit(" same=%d", r2 == r);
    }
    emit("\n");
}

/* Patch the pristine boot sector at off1 (and off2 if off2 >= 0, and the
 * high byte tot32[2..3] too when zero32), write it physically, Getbpb(2). */
static void xcase(const char *id, int off1, int v1, int off2, int v2,
                  int zero32)
{
    char call[64];
    long w;
    memcpy(buf, sec0, 512);
    buf[off1] = (unsigned char)v1;
    if (off2 >= 0)
        buf[off2] = (unsigned char)v2;
    if (zero32) {
        buf[0x13] = buf[0x14] = 0;
        buf[0x22] = buf[0x23] = 0;
    }
    w = rw(11, buf, 1, -1, PHYSUNIT, PSTART);
    snprintf(call, sizeof(call), "patch[%02x]=%02x", off1, v1 & 0xFF);
    retline(id, call, w);
    emit("\n");
    gcase(id, 2);
}

static void mcase(const char *id, int d)
{
    char call[32];
    long r = trap_13_ww((short)9, (short)d);
    snprintf(call, sizeof(call), "Mediach(%d)", d);
    retline(id, call, r);
    emit("\n");
}

int main(void)
{
    long r;
    int fh;

    emit("BSTPROBE PSIZE=%ld DSIZE=%ld IDEUNIT=%d\n", (long)PSIZE, (long)DSIZE,
         IDEUNIT);

    /* --- Drvmap ------------------------------------------------------- */
    r = trap_13_w((short)10);
    retline("D01", "Drvmap()", r);
    emit("\n");

    /* --- Getbpb: present, absent, out of range ------------------------- */
    gcase("G01", 2);
    gcase("G02", 3);
    gcase("G03", 5);
    gcase("G04", 15);
    gcase("G05", 16);
    gcase("G06", -1);
    gcase("G07", 0x7FFF);

    /* --- Mediach: present, absent, out of range ------------------------ */
    mcase("M01", 2);
    mcase("M02", 3);
    mcase("M03", 5);
    mcase("M04", 15);
    mcase("M05", 16);
    mcase("M06", -1);
    mcase("M07", 2);

    /* --- Rwabs, logical reads ------------------------------------------ */
    rcase("R01", 0, 1, 0, 2, 0, 1);              /* boot sector */
    memcpy(sec0, buf, 512);
    rcase("R02", 0, 2, 0, 2, 0, 1);              /* two sectors */
    rcase("R03", 0, 1, -1, 2, 1, 1);             /* first=-1 -> lfirst */
    rcase("R04", 0, 1, 1, 2, 0, 1);              /* same sector via first */
    rcase("R05", 0, 1, -1, 2, PSIZE - 1, 1);     /* last sector */
    memcpy(seclast, buf, 512);
    rcase("R06", 0, 1, -1, 2, PSIZE, 1);         /* one past the end */
    rcase("R07", 0, 2, -1, 2, PSIZE - 1, 1);     /* straddles the end */
    rcase("R08", 0, 1, -1, 2, -5, 1);            /* negative lfirst */
    rcase("R09", 0, 0, 0, 2, 0, 1);              /* count 0 */
    rcase("R10", 0, 0, -1, 2, PSIZE + 10, 1);    /* count 0, past end */
    rcase("R11", 0, 1, 0, 15, 0, 0);             /* P: absent */
    rcase("R12", 0, 1, 0, 5, 0, 0);              /* F: absent */
    rcase("R13", 0, 1, 0, -1, 0, 0);             /* drive -1 */
    rcase("R14", 0, 1, 0, 16, 0, 0);             /* drive 16 */
    rcase("R15", 2, 1, 0, 2, 0, 1);              /* RW_NOMEDIACH */
    rcase("R16", 4, 1, 0, 2, 0, 1);              /* RW_NORETRIES */
    rcase("R17", 0x80, 1, 0, 2, 0, 1);           /* RW_NOBYTESWAP, logical */
#if PSIZE > 32769
    rcase("R18", 0, 1, (short)0x8000, 2, 0, 1);  /* first unsigned 32768 */
    rcase("R19", 0, 1, -1, 2, 32768L, 1);        /* same via lfirst */
#endif
    rcase("R20", 0, 1, -2, 2, 0, 1);             /* first=-2 = 65534 */

    /* --- Rwabs, physical (RW_NOTRANSLATE) ------------------------------ */
    rcase("P01", 8, 1, 0, 2, 0, 1);              /* EmuTOS: ACSI unit 0 */
    rcase("P02", 8, 1, 0, IDEUNIT, 0, 1);        /* EmuTOS: IDE master, MBR */
    rcase("P03", 8, 1, -1, IDEUNIT, DSIZE - 1, 1);   /* last device sector */
    rcase("P04", 8, 1, -1, IDEUNIT, DSIZE - 2, 1);
    rcase("P05", 8, 1, 0, 40, 0, 0);             /* past UNITSNUM */

    /* --- write sector 0 back unchanged: forcechange handshake ---------- */
    {
        long w;
        memcpy(buf, sec0, 512);
        w = rw(1, buf, 1, 0, 2, 0);
        retline("W01", "Rwabs(1,sec0,1,0,2,0)", w);
        emit("\n");
    }
    mcase("W02", 2);
    rcase("W03", 0, 1, 1, 2, 0, 1);              /* read after the write */
    rcase("W04", 2, 1, 1, 2, 0, 1);              /* same with RW_NOMEDIACH */
    mcase("W05", 2);
    gcase("W06", 2);                             /* Getbpb resets */
    mcase("W07", 2);
    rcase("W08", 0, 1, 1, 2, 0, 1);
    {
        long w;
        memcpy(buf, seclast, 512);
        w = rw(1, buf, 1, -1, 2, PSIZE - 1);     /* write last sector back */
        retline("W09", "Rwabs(1,last,1,-1,2,PSIZE-1)", w);
        emit("\n");
    }
    mcase("W10", 2);
    rcase("W11", 0, 1, 1, 2, 0, 1);

    /* --- Getbpb validation: patched boot sectors ----------------------
     * Each step writes a patched copy of the pristine boot sector through
     * the PHYSICAL path (RW_WRITE|RW_NOMEDIACH|RW_NOTRANSLATE = 11, absolute
     * sector PSTART of PHYSUNIT -- the forcechange/recsiz state of the
     * logical path is not involved), then calls Getbpb(2), which re-reads
     * the boot sector on both systems.  The last step restores sec0. */
    xcase("X01", 0x0C, 0x04, -1, 0, 0);           /* bps 1024, spc 4 */
    xcase("X02", 0x10, 0x00, -1, 0, 0);           /* FAT count 0 */
    xcase("X03", 0x10, 0x03, -1, 0, 0);           /* FAT count 3 */
    xcase("X04", 0x0D, 0x80, -1, 0, 0);           /* spc 128: 64 KiB clusters */
    xcase("X05", 0x0E, 0x00, 0x0F, 0x00, 0);      /* reserved 0 */
    xcase("X06", 0x20, 0x00, 0x21, 0x00, 1);      /* tot16 = tot32 = 0 */
    xcase("X07", 0x16, 0x00, 0x17, 0x00, 0);      /* sectors per FAT 0 */
    rcase("X08", 0, 1, 1, 2, 0, 1);              /* logical read after X07 */
    {
        long w;
        memcpy(buf, sec0, 512);
        w = rw(11, buf, 1, -1, PHYSUNIT, PSTART);
        retline("X09", "Rwabs(11,sec0,1,-1,PHYSUNIT,PSTART) restore", w);
        emit("\n");
    }
    gcase("X10", 2);
    rcase("X11", 0, 1, 1, 2, 0, 1);
    {
        long w;
        memcpy(buf, sec0, 512);
        w = rw(3, buf, 1, 0, 2, 0);              /* logical, RW_NOMEDIACH */
        retline("X12", "Rwabs(3,sec0,1,0,2,0)", w);
        emit("\n");
    }
    mcase("X13", 2);
    rcase("X14", 2, 1, 1, 2, 0, 1);
    gcase("X15", 2);
    mcase("X16", 2);

    /* main log first: a floppy access below may end in a critical-error
     * alert on EmuTOS (no disk in A:), which would stop the program */
    emit("END\n");
    fh = (int)Fcreate("BSTPROBE.LOG", 0);
    if (fh >= 0) {
        Fwrite(fh, olen, out);
        Fclose(fh);
    }
    olen = 0;

    /* --- floppy-only paths (A:/B:) ------------------------------------ */
    mcase("F01", 0);
    gcase("F02", 0);
    r = rw(0, NULL, 2, 0, 0, 0);                 /* NULL buffer feature */
    retline("F03", "Rwabs(0,NULL,2,0,0,0)", r);
    emit("\n");
    mcase("F04", 0);
    gcase("F05", 1);
    mcase("F06", 1);

    emit("END\n");

    fh = (int)Fcreate("BSTPROB2.LOG", 0);
    if (fh >= 0) {
        Fwrite(fh, olen, out);
        Fclose(fh);
    }
    Cconws("BSTPROBE done\r\n");
    return 0;
}
