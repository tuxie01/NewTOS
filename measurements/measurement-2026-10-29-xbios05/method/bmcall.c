/*
 * bmcall.c -- XBIOS05: a program calls through a Bconmap MAPTAB function
 * pointer (user decision 2026-10-28: "aus EL0 nicht aufrufbar, laut
 * fehlschlagend"). NewTOS only: on EmuTOS the pointer is a ROM routine and
 * the call works. Reads Bconmap(-2)'s BCONMAP and MAPTAB[0] through the SDK's
 * _BCONMAP/_MAPTAB, writes them to BMCALL.LOG, closes the log, then calls
 * maptab[0].Bconstat. Expected: the kernel's crash report (bus error,
 * "Bconmap MAPTAB function (not callable)"), the program ends with
 * Pterm(-1); the line "returned" is never appended.
 */
#include <osbind.h>
#include <mint/ostruct.h>
#include <stdio.h>
#include <string.h>

static void put(const char *s)
{
    long fh = Fopen("BMCALL.LOG", 1);
    if (fh < 0)
        fh = Fcreate("BMCALL.LOG", 0);
    if (fh < 0)
        return;
    Fseek(0L, (short)fh, 2);
    Fwrite((short)fh, (long)strlen(s), s);
    Fclose((short)fh);
}

int main(void)
{
    char b[256];
    _BCONMAP *bm = (_BCONMAP *)Bconmap(-2);
    _MAPTAB *m = bm->maptab;
    snprintf(b, sizeof b, "C01 Bconmap(-1)=%ld Bconmap(-2)=%lx maptab=%lx maptabsize=%d mapped_device=%d\n",
             (long)Bconmap(-1), (unsigned long)bm, (unsigned long)m, bm->maptabsize,
             ((short *)&bm->maptabsize)[1]);
    put(b);
    snprintf(b, sizeof b, "C02 maptab[0] Bconstat=%lx Bconin=%lx Bcostat=%lx Bconout=%lx Rsconf=%lx iorec=%lx Iorec(0)=%lx\n",
             (unsigned long)m->Bconstat, (unsigned long)m->Bconin, (unsigned long)m->Bcostat,
             (unsigned long)m->Bconout, (unsigned long)m->Rsconf, (unsigned long)m->iorec,
             (unsigned long)Iorec(0));
    put(b);
    put("C03 calling maptab[0].Bconstat(1)\n");
    {
        short r = (m->Bconstat)(1); /* parenthesised: osbind.h defines Bconstat() */
        snprintf(b, sizeof b, "C04 returned %d\n", r);
        put(b);
    }
    return 0;
}
