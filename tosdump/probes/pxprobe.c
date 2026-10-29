/*
 * pxprobe.c -- Pexec and Fsfirst meet a media change (TASK/FS22).  A
 * permanent probe of the tosdump tree like gcprobe.c (TASK/FS20): it makes
 * GEMDOS calls, which DUMPBIOS admits none of (docs/TOS-REFERENCE-DUMPER.md
 * section 6), and writes a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * pxprobe-m68k` (PXPROBE.PRG) and `make -C tosdump pxprobe-newtos`
 * (PXPROBE.NAP, tosdump/build-pxprobe-newtos.sh); run by
 * tosdump/probes/run-pxprobe.sh, the oracle side through
 * tools/hatari-oracle.sh profile disk.
 *
 * The program is its own child: started with the command tail "child" it
 * returns 42 and does nothing else.  Both builds lie on the test drive C:
 * (C:\PXPROBE.PRG and C:\PXPROBE.NAP, both on both sides, so the two test
 * images are the same bytes); each side starts its own.  The probe itself
 * runs from D:.
 *
 * EmuTOS, read from the source (pin 1a7484ab; docs/api/gemdos/_state.md
 * section 4a): once a logical write of sector 0 has armed forcechange, the
 * next GEMDOS access to the drive meets it and osif logs the drive in again
 * and restarts the call (bdos/bdosmain.c:449-487).  Pexec is such a call
 * (xexec under osif, bdos/bdosmain.c:271): its ixsfirst (bdos/proc.c:258)
 * meets the change, the restarted xexec loads the program.  ixsfirst is
 * called with a NULL DTA (bdos/proc.c:258, bdos/fsdir.c:452-477): Pexec
 * leaves the caller's DTA alone.  Before TASK/FS22 NewTOS's Pexec loaded
 * outside the dispatcher's restart and answered EFILNF.
 *
 * Disk: C: = the test image of tools/mktestdisk.sh (FAT16, LBA 63, 130977
 * sectors) with C:\PXPROBE.PRG and C:\PXPROBE.NAP, on a fresh clone; D: = the
 * program's drive.  Output: D:\PXPROBE.LOG, written once at the end.  The
 * boot sector is written back unchanged, so the test partition must come
 * out as it went in.
 *
 * Part P (Pexec):
 *   P01 Fsetdta(&dta), Fsfirst(C:\PXPROBE.*, 0)   the DTA before the Pexec
 *   P02 Rwabs(2,..,0,2,0)  read sector 0 (RW_NOMEDIACH)
 *   P03 Rwabs(3,..,0,2,0)  write it back unchanged, RW_NOMEDIACH: arms
 *   P04 Mediach(2)         2
 *   P05 Pexec(0, C:\PXPROBE.<own>, "child", NULL)   EmuTOS: 42
 *   P06 Mediach(2)         EmuTOS: 0 (the restart's Getbpb cleared it)
 *   P07 Fgetdta() and the DTA after the Pexec: unchanged (all 44 bytes)
 * Part F (Fsfirst):
 *   F01 Rwabs read, F02 Rwabs write (arms), F03 Mediach(2)  2
 *   F04 Fsfirst(C:\PXPROBE.*, 0)  the DTA after the restart
 *   F05 Mediach(2)         0
 *   F06 Fsnext, F07 Fsnext the search the restarted Fsfirst set up
 * The DTA's public part is logged by field (attribute, time, date, length,
 * name; native byte order on each side), the reserved part (bytes 0-20) is
 * not: its layout is each system's own (docs/api/structures/dta.md).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#ifdef __NEWTOS__
#define CHILD "C:\\PXPROBE.NAP"
#else
#define CHILD "C:\\PXPROBE.PRG"
#endif

static char out[8000];
static int olen;
static unsigned char buf[512] __attribute__((aligned(2)));
static _DTA dta __attribute__((aligned(2)));

static void emit(const char *fmt, ...)
{
    va_list ap;
    va_start(ap, fmt);
    if (olen < (int)sizeof(out) - 256)
        olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
    va_end(ap);
}

static long rw(int flag, void *b, int n, int first, int drv, long lfirst)
{
    return trap_13_wwlwwwl((short)4, (short)flag, (long)b, (short)n,
                           (short)first, (short)drv, (long)lfirst);
}

static void mcase(const char *id)
{
    long r = trap_13_ww((short)9, (short)2);
    emit("%s Mediach(2) ret=%ld\n", id, r);
}

static void rcase(const char *id, int flag)
{
    long r = rw(flag, buf, 1, 0, 2, 0);
    emit("%s Rwabs(%d,buf,1,0,2,0) ret=%ld\n", id, flag, r);
}

/* the public part of the DTA, field by field */
static void dta_fields(void)
{
    emit(" attr=%02x time=%04x date=%04x len=%ld name=%.14s",
         (unsigned)(unsigned char)dta.dta_attribute,
         (unsigned)(unsigned short)dta.dta_time,
         (unsigned)(unsigned short)dta.dta_date,
         (long)dta.dta_size, dta.dta_name);
}

static void scase(const char *id, int first)
{
    long r = first ? Fsfirst("C:\\PXPROBE.*", 0) : Fsnext();
    emit("%s %s ret=%ld", id, first ? "Fsfirst(C:\\PXPROBE.*,0)" : "Fsnext", r);
    if (r == 0)
        dta_fields();
    emit("\n");
}

int main(int argc, char **argv)
{
    unsigned char before[44];
    long fh, r;

    if (argc > 1 && strcmp(argv[1], "child") == 0)
        return 42;

    /* ---- part P: Pexec meets the change ---- */
    memset(&dta, 0, sizeof(dta));
    Fsetdta(&dta);
    scase("P01", 1);
    memcpy(before, &dta, sizeof(before));
    rcase("P02", 2);
    rcase("P03", 3);
    mcase("P04");
    r = Pexec(0, CHILD, "\005child", NULL);
    emit("P05 Pexec(0,C:\\PXPROBE.<own>,child) ret=%ld\n", r);
    mcase("P06");
    emit("P07 Fgetdta same=%d dta-unchanged=%d",
         (void *)Fgetdta() == (void *)&dta,
         memcmp(before, &dta, sizeof(before)) == 0);
    dta_fields();
    emit("\n");

    /* ---- part F: Fsfirst meets the change ---- */
    rcase("F01", 2);
    rcase("F02", 3);
    mcase("F03");
    memset(&dta, 0, sizeof(dta));
    scase("F04", 1);
    mcase("F05");
    scase("F06", 0);
    scase("F07", 0);
    emit("END\n");

    fh = Fcreate("D:\\PXPROBE.LOG", 0);
    if (fh >= 0) {
        Fwrite((short)fh, olen, out);
        Fclose((short)fh);
    }
    Cconws("PXPROBE done\r\n");
    return 0;
}
