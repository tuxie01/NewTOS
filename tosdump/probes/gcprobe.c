/*
 * gcprobe.c -- GEMDOS meets a media change (TASK/FS20; first used for BIOS01
 * work item 4).  A permanent probe of the tosdump tree, not a dumper: it makes
 * GEMDOS calls, which DUMPBIOS admits none of (docs/TOS-REFERENCE-DUMPER.md
 * section 6, "The call table"), and writes a text log, no record file.  The
 * case moves to DUMPGDOS once that dumper exists (TASK/FS20 work item 4).
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * gcprobe-m68k` (GCPROBE.PRG) and `make -C tosdump gcprobe-newtos`
 * (GCPROBE.NAP, tosdump/build-gcprobe-newtos.sh); run by
 * tosdump/probes/run-gcprobe.sh, the oracle side through
 * tools/hatari-oracle.sh profile disk.  Code byte for byte that of the first
 * copy, measurements/measurement-2026-10-28-bios01-s2/method/gcprobe.c.
 *
 * EmuTOS, read from the source (pin 1a7484ab; docs/api/gemdos/_state.md
 * section 4a): once a logical write of sector 0 has armed forcechange, the
 * next GEMDOS access to the drive meets it -- getbcb's Mediach on a buffer
 * hit (bdos/fsbuf.c:179-190) or E_CHNG from the Rwabs of a miss
 * (bdos/fs.h:60-68) -- and osif re-logs the drive in: frees its open files,
 * directories and buffers, Getbpb, log_media with the NEW boot sector,
 * restarts the call (bdos/bdosmain.c:449-487).  The Getbpb clears
 * forcechange.
 *
 * Disk: C: = the test image of tools/mktestdisk.sh (FAT16, LBA 63, 130977
 * sectors, BPB 512,4,2048,32,128,129,289,32672,1), on a fresh clone (the
 * probe changes its boot sector), D: = the program's drive.
 * Output: D:\GCPROBE.LOG, written once at the end (no GEMDOS access between
 * the BIOS calls other than the measured ones).
 *
 * Part A (re-login, the boot sector unchanged):
 *   A01 Dfree(C:)            logs C: in, baseline geometry
 *   A02 Mediach(2)           0
 *   A03 Rwabs(2,..,0,2,0)    read sector 0 (RW_NOMEDIACH)
 *   A04 Rwabs(3,..,0,2,0)    write it back unchanged, RW_NOMEDIACH: 0, arms
 *   A05 Mediach(2)           2
 *   A06 Dfree(C:)            the GEMDOS access
 *   A07 Mediach(2)           EmuTOS: 0 (re-logged in, Getbpb cleared it)
 *   A08 Getbpb(2), A09 Mediach(2)
 * Part B (the boot sector changed):
 *   B01 Rwabs read sector 0; root entries 512 -> 256 (bytes 0x11-0x12):
 *       rdlen 32 -> 16 sectors, datrec 289 -> 273, numcl 32672 -> 32676
 *   B02 Rwabs(3) write it, RW_NOMEDIACH: 0, arms
 *   B03 Mediach(2)           2
 *   B04 Dfree(C:)            total clusters: the geometry GEMDOS uses now
 *   B05 Mediach(2)
 *   B06 Fcreate C:\GCPROBE.DAT, Fwrite 512 bytes of a marker, Fclose:
 *       where the marker lands in the image shows the geometry of the write
 *       (EmuTOS: cluster 3, partition sector 277 = datrec 273 + 4)
 *   B07 Getbpb(2), B08 Mediach(2)
 * The image is left with the changed boot sector; the run compares the test
 * partitions of both sides (tosdump/probes/gcprobe-compare.py).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

static char out[8000];
static int olen;
static unsigned char buf[512] __attribute__((aligned(2)));
static unsigned char dat[512] __attribute__((aligned(2)));

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

static void gcase(const char *id)
{
    long r = trap_13_ww((short)7, (short)2);
    emit("%s Getbpb(2) nz=%d", id, r != 0);
    if (r != 0) {
        const unsigned short *w = (const unsigned short *)r;
        int i;
        emit(" bpb=");
        for (i = 0; i < 9; i++)
            emit("%u%s", w[i], i < 8 ? "," : "");
    }
    emit("\n");
}

static void dcase(const char *id)
{
    _DISKINFO di;
    long r;
    memset(&di, 0, sizeof(di));
    r = Dfree(&di, 3);                       /* 3 = C: */
    emit("%s Dfree(C:) ret=%ld free=%lu total=%lu secsiz=%lu clsiz=%lu\n", id, r,
         (unsigned long)di.b_free, (unsigned long)di.b_total,
         (unsigned long)di.b_secsiz, (unsigned long)di.b_clsiz);
}

static void rcase(const char *id, int flag)
{
    long r = rw(flag, buf, 1, 0, 2, 0);
    emit("%s Rwabs(%d,buf,1,0,2,0) ret=%ld b11=%02x%02x\n", id, flag, r,
         buf[0x11], buf[0x12]);
}

int main(void)
{
    long fh, r;
    int i;

    /* ---- part A: re-login after the handshake ---- */
    dcase("A01");
    mcase("A02");
    rcase("A03", 2);
    rcase("A04", 3);
    mcase("A05");
    dcase("A06");
    mcase("A07");
    gcase("A08");
    mcase("A09");

    /* ---- part B: a changed boot sector ---- */
    rcase("B01", 2);
    buf[0x11] = 0x00;                        /* root entries 512 (0x0200) -> 256 */
    buf[0x12] = 0x01;
    rcase("B02", 3);
    mcase("B03");
    dcase("B04");
    mcase("B05");
    for (i = 0; i < 512; i++)
        dat[i] = (unsigned char)"GCPROBE-STALE-GEOMETRY-MARKER-01"[i % 32];
    fh = Fcreate("C:\\GCPROBE.DAT", 0);
    emit("B06 Fcreate(C:\\GCPROBE.DAT) ret=%ld", fh);
    if (fh >= 0) {
        r = Fwrite((short)fh, 512L, dat);
        emit(" Fwrite=%ld", r);
        r = Fclose((short)fh);
        emit(" Fclose=%ld", r);
    }
    emit("\n");
    gcase("B07");
    mcase("B08");
    emit("END\n");

    fh = Fcreate("D:\\GCPROBE.LOG", 0);
    if (fh >= 0) {
        Fwrite((short)fh, olen, out);
        Fclose((short)fh);
    }
    Cconws("GCPROBE done\r\n");
    return 0;
}
