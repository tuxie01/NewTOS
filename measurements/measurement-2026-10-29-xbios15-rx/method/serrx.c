/*
 * serrx.c -- XBIOS15 step 1: where does a byte received on the serial line
 * land -- device 0's IOREC input ring (EmuTOS push_serial_iorec,
 * references/emutos/bios/serport.c:331-343) or the keyboard ring of Iorec(1)?
 *
 * NOT tosdump: it calls Bconin(1)/Bconin(2) (refused by the DUMPBIOS loader,
 * they may wait) and reads the ring contents behind the records, which the
 * r:<layout> capture cannot.  Same C source built twice (m68k-atari-mint-gcc +
 * mintlib for EmuTOS on Hatari; clang aarch64 + the SDK's mintlib for NewTOS),
 * calls through mintlib's osbind.h macros, as a ported program does.
 *
 * Sequence:
 *   B  before: Iorec(0) in/out fields, the mapped port's IOREC via Bconmap(-2) (B05), Iorec(1) fields, Bconstat(1), Bconstat(2)
 *   M  ready marker: file SERRX.RDY (Hatari harness polls the host dir) and
 *      "SERRX:WAIT" on the console (NewTOS harness polls the serial line);
 *      the harness injects the bytes only after it saw the marker
 *   W  WAITVBL Vsync() calls -- no console I/O in between (a GEMDOS console
 *      write runs conbrk, which would move keyboard-ring keys into the BDOS
 *      typeahead)
 *   A  after: the same fields plus the ring contents between head and tail
 *   C  Bconin(1) once, only if Bconstat(1) said a byte is there (EmuTOS'
 *      bconin1 waits forever on an empty ring), then Bconstat(1), Iorec(0)
 *   D  Bconin(2) once, only if Bconstat(2) said a key is there, then
 *      Bconstat(2), Iorec(1)
 * Console markers AUDIT:xbios15-serial-rx:BEGIN / :END (the auditor's
 * temp-wiring rule: an image without them is not this probe's).  The
 * BEGIN line is the probe's only console write before its B dump.
 * Output: SERRX.LOG in the current directory (if a drive is there) and the
 * whole log on the console, every line prefixed "SERRX|" (qemu-virt has no
 * drive: the serial line is its only channel).
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#ifndef WAITVBL
#define WAITVBL 400
#endif

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

#define IOREC(d)   ((long)trap_14_ww((short)0x0E, (short)(d)))
#define BCONMAP(d) ((long)trap_14_ww((short)0x2C, (short)(d)))

static int sane(const _IOREC *io)
{
    return io->ibufsiz > 0 && io->ibufsiz <= 4096 && io->ibufhd >= 0 &&
           io->ibufhd < io->ibufsiz && io->ibuftl >= 0 && io->ibuftl < io->ibufsiz;
}

/* One IOREC: fields, and with ring=1/4 the entries between head and tail
 * (byte ring: entry at ++head; IKBD ring: 4-byte entry at head += 4). */
static void dump_rec(const char *id, const char *what, const _IOREC *io, int step)
{
    emit("%s %s ibufsiz=%d hd=%d tl=%d low=%d hi=%d\n", id, what, io->ibufsiz,
         io->ibufhd, io->ibuftl, io->ibuflow, io->ibufhi);
    if (!sane(io)) {
        emit("%s %s ring: indices out of range, not read\n", id, what);
        return;
    }
    {
        int i = io->ibufhd, n = 0;
        emit("%s %s ring:", id, what);
        while (i != io->ibuftl && n < 64) {
            i += step;
            if (i >= io->ibufsiz)
                i = 0;
            if (step == 1)
                emit(" %02x", (unsigned)(unsigned char)io->ibuf[i]);
            else
                emit(" %08lx", (unsigned long)*(const unsigned int *)(io->ibuf + i));
            n++;
        }
        emit(" (%d entries)\n", n);
    }
}

static void dump0(const char *id)
{
    long r = IOREC(0);
    line(id, "Iorec(0)", r);
    if (r == -1 || r == 0x0E)
        return;
    {
        const _IOREC *in = (const _IOREC *)r;
        const unsigned char *x = (const unsigned char *)r + (sizeof(long) > 4 ? 48 : 28);
        dump_rec(id, "dev0.in", in, 1);
        dump_rec(id, "dev0.out", in + 1, 1);
        emit("%s dev0 baudrate=%u flowctrl=%u ucr=%02x datamask=%02x\n", id, x[0], x[1], x[2],
             x[3]);
    }
}

/* The IOREC of the port currently mapped to AUX, reached through the Bconmap
 * table (user addition 2026-10-29): Bconmap(-1) = mapped device, Bconmap(-2) =
 * BCONMAP {maptab, maptabsize, mapped_device} (references/emutos/bios/serport.h:112-116),
 * maptab[mapped_device - 6] is a MAPTAB whose last field is the EXT_IOREC pointer
 * (:103-110).  Followed only if Bconmap(-2) looks like a table address (even,
 * >= 0x400, below 2 GiB) -- NewTOS 2487bbea returns the function number 0x2c. */
static void dumpmap(const char *id)
{
    long d = BCONMAP(-1), p = BCONMAP(-2), io0 = IOREC(0);
    line(id, "Bconmap(-1)", d);
    line(id, "Bconmap(-2)", p);
    if ((p & 1) || (unsigned long)p < 0x400UL || (unsigned long)p >= 0x80000000UL) {
        emit("%s Bconmap(-2) is no table address: mapped port IOREC not reached\n", id);
        return;
    }
    {
        const _BCONMAP *bm = (const _BCONMAP *)p;
        int size = bm->maptabsize, mapped = ((const short *)&bm->maptabsize)[1];
        int idx = mapped - 6;
        emit("%s BCONMAP maptabsize=%d mapped_device=%d (Bconmap(-1) %ld)\n", id, size, mapped, d);
        if (idx < 0 || idx >= size || bm->maptab == 0) {
            emit("%s mapped_device outside maptab: not followed\n", id);
            return;
        }
        {
            const _IOREC *io = bm->maptab[idx].iorec;
            emit("%s maptab[%d].iorec=%08lx Iorec(0)=%08lx same=%d\n", id, idx,
                 lo32((long)io), lo32(io0), (long)io == io0);
            if (io != 0)
                dump_rec(id, "mapped.in", io, 1);
        }
    }
}

static void dump1(const char *id)
{
    long r = IOREC(1);
    line(id, "Iorec(1)", r);
    if (r == -1 || r == 0x0E)
        return;
    dump_rec(id, "ikbd", (const _IOREC *)r, 4);
}

static void mark(void)
{
    long fh = Fcreate("SERRX.RDY", 0);
    if (fh >= 0) {
        Fwrite((short)fh, 6L, "ready\n");
        Fclose((short)fh);
    }
    Cconws("SERRX:WAIT\r\n");
}

static void finish(void)
{
    long fh = Fcreate("SERRX.LOG", 0);
    char ln[200];
    int i, j = 0;
    if (fh >= 0) {
        Fwrite((short)fh, (long)olen, out);
        Fclose((short)fh);
    }
    for (i = 0; i < olen; i++) {
        if (out[i] == '\n' || j >= (int)sizeof(ln) - 4) {
            ln[j] = 0;
            Cconws("SERRX|");
            Cconws(ln);
            Cconws("\r\n");
            j = 0;
            if (out[i] == '\n')
                continue;
        }
        ln[j++] = out[i];
    }
}

int main(void)
{
    long s1, s2, c;
    int i;

    Cconws("AUDIT:xbios15-serial-rx:BEGIN\r\n");
    emit("H00 serrx WAITVBL=%d sizeof(long)=%d sizeof(_IOREC)=%d\n", WAITVBL, (int)sizeof(long),
         (int)sizeof(_IOREC));
    line("H01", "Bconmap(-1)", BCONMAP(-1));
    dump0("B01");
    dumpmap("B05");
    dump1("B02");
    line("B03", "Bconstat(1)", Bconstat(1));
    line("B04", "Bconstat(2)", Bconstat(2));

    mark();
    for (i = 0; i < WAITVBL; i++)
        Vsync();

    dump0("A01");
    dumpmap("A05");
    dump1("A02");
    s1 = Bconstat(1);
    s2 = Bconstat(2);
    line("A03", "Bconstat(1)", s1);
    line("A04", "Bconstat(2)", s2);

    if (s1) {
        c = Bconin(1);
        line("C01", "Bconin(1)", c);
        line("C02", "Bconstat(1)", Bconstat(1));
        dump0("C03");
    } else {
        emit("C01 Bconin(1) not called: Bconstat(1) was 0\n");
    }
    if (s2) {
        c = Bconin(2);
        line("D01", "Bconin(2)", c);
        line("D02", "Bconstat(2)", Bconstat(2));
        dump1("D03");
    } else {
        emit("D01 Bconin(2) not called: Bconstat(2) was 0\n");
    }
    emit("Z99 end\n");
    finish();
    Cconws("SERRX:END\r\nAUDIT:xbios15-serial-rx:END\r\n");
    return 0;
}
