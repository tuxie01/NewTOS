/*
 * uakid.c -- hands the kernel ONE bad user pointer on purpose (ARCH03 3a).
 *
 * The first character of the command tail selects the system call and the
 * argument that carries the bad address 0xDEAD0000 (unmapped in every
 * process: not page 0, not a pool, not a publication). Every case expects
 * the EmuTOS answer to a bus error inside a system call
 * (docs/api/bios/_exception-crash.md §5): the crash report, a key wait, then
 * Pterm(-1) of THIS program -- the parent's Pexec returns 0x0000FFFF -- and
 * the machine keeps running. The line "UAKID: survived" must never appear.
 *
 *   GEMDOS (sub-step 2)                          BIOS/XBIOS (sub-step 3)
 *   c  Cconws(BAD)  (stage 0c pilot)             k  Cconrs(BAD)
 *   o  Fopen(BAD)
 *   n  Frename(0, "C:\UAKID.NAP", BAD)           s  Ssystem(S_GETCOOKIE, 0, BAD)
 *   w  Fwrite(1 = console, 5, BAD)               m  Getmpb(BAD)
 *   g  Dgetpath(BAD, 0)                          P  Setpalette(BAD)
 *   r  Fread(file, 16, BAD)            [disk]    T  Protobt(BAD, 0x01000000, 2, 0)
 *   W  Fwrite(new file, 16, BAD)       [disk]    K  Keytbl(BAD, -1, -1)
 *   d  Fsetdta(BAD) + Fsfirst("C:\*.*") [disk]   b  Rwabs(0, BAD, 1, 0, C:, 0) [disk]
 *   D  Fsetdta(BAD) + Fgetdta + Fsfirst("C:\*.*") [disk] (TASK/MEM11)
 *   f  Dfree(BAD, 0)                   [disk]
 *   t  Fdatime(BAD, file, 0)           [disk]
 *
 *   AES / VDI (sub-step 4/5)
 *   A  AES trap, parameter block pointer BAD
 *   I  AES graf_mouse(ARROW) with intin = BAD
 *   G  appl_init, then AES call with global = BAD
 *   F  appl_init, then appl_find(BAD name)
 *   V  VDI trap, parameter block pointer BAD
 *   C  VDI vq_extnd with contrl = BAD
 *   M  appl_init/graf_handle/v_opnvwk, vro_cpyfm screen -> MFDB at BAD
 *   E  appl_init, objc_edit(ED_CHAR) on a tree whose te_ptext is BAD
 *   X  appl_init, menu_text(own tree, 1, BAD)
 *
 *   o-k  clean exit (case '.')
 *
 * Build: sdk/build.sh sdk/examples/uakid.c <out>/UAKID.NAP
 */
#include <newtos.h>

#define BAD 0xDEAD0000UL
#define P_CMDLIN 0x80

typedef unsigned long UL;

static UL sys(int trap, UL fn, UL a0, UL a1, UL a2, UL a3, UL a4, UL a5)
{
    register UL x8 __asm__("x8") = fn;
    register UL r0 __asm__("x0") = a0;
    register UL r1 __asm__("x1") = a1;
    register UL r2 __asm__("x2") = a2;
    register UL r3 __asm__("x3") = a3;
    register UL r4 __asm__("x4") = a4;
    register UL r5 __asm__("x5") = a5;
    switch (trap) {
    case 1:
        __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5) : "memory");
        break;
    case 2:
        __asm__ volatile("svc #2" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5) : "memory");
        break;
    case 13:
        __asm__ volatile("svc #13" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5) : "memory");
        break;
    default:
        __asm__ volatile("svc #14" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2), "r"(r3), "r"(r4), "r"(r5) : "memory");
        break;
    }
    return r0;
}
#define G(fn, a, b, c) sys(1, (fn), (UL)(a), (UL)(b), (UL)(c), 0, 0, 0)
#define B(fn, a, b, c, d, e, f) sys(13, (fn), (UL)(a), (UL)(b), (UL)(c), (UL)(d), (UL)(e), (UL)(f))
#define X(fn, a, b, c, d) sys(14, (fn), (UL)(a), (UL)(b), (UL)(c), (UL)(d), 0, 0)

static const unsigned char *own_basepage(void)
{
    UL sysbase = *(volatile unsigned int *)0x4F2UL;
    UL run_cell = *(volatile unsigned int *)(sysbase + 0x28);
    return (const unsigned char *)(UL)*(volatile unsigned int *)run_cell;
}

/* ---- AES / VDI (LP64: 8-byte pointers, 8-byte addrin slots) ------------- */
static short control[16], global[16], intin[16], intout[16];
static UL addrin[8], addrout[8];
static struct { void *c, *g, *ii, *io, *ai, *ao; } aespb = {
    control, global, intin, intout, addrin, addrout };
static short contrl[16], vintin[32], ptsin[32], vintout[64], ptsout[32];
static struct { void *c, *ii, *pi, *io, *po; } vdipb = {
    contrl, vintin, ptsin, vintout, ptsout };

static void aes(int op, int nin, int nout, int nain, int naout)
{
    control[0] = op; control[1] = nin; control[2] = nout;
    control[3] = nain; control[4] = naout;
    sys(2, 0, 200, (UL)&aespb, 0, 0, 0, 0);
}
static void vdi(int op, int npts, int nint, int handle)
{
    contrl[0] = op; contrl[1] = npts; contrl[3] = nint; contrl[6] = handle;
    sys(2, 0, 115, (UL)&vdipb, 0, 0, 0, 0);
}
static void appl_init(void) { aes(10, 0, 1, 0, 0); }

/* OBJECT (LP64, 32 bytes) and TEDINFO (LP64) for case 'E'. */
struct obj { short next, head, tail, type, flags, state; int pad; UL spec; short x, y, w, h; };
struct ted { UL ptext, ptmplt, pvalid; short font, fontid, just, color, fontsize, thick, txtlen, tmplen; };
static struct ted ted = { BAD, (UL)"____", (UL)"XXXX", 3, 0, 0, 0x1180, 0, 0, 5, 5 };
static struct obj tree[2] = {
    { -1, 1, 1, 20 /* G_BOX */, 0, 0, 0, 0x1100, 0, 0, 100, 40 },
    { 0, -1, -1, 29 /* G_FTEXT */, 0x20 /* LASTOB */ | 0x08 /* EDITABLE */, 0, 0, 0, 8, 8, 64, 16 },
};
static struct obj menu[3] = {
    { -1, 1, 2, 25 /* G_IBOX */, 0, 0, 0, 0, 0, 0, 100, 40 },
    { 2, -1, -1, 28 /* G_STRING */, 0, 0, 0, 0, 0, 0, 80, 16 },
    { 0, -1, -1, 28 /* G_STRING */, 0x20, 0, 0, 0, 0, 16, 80, 16 },
};

int main(void)
{
    const unsigned char *tail = own_basepage() + P_CMDLIN;
    char c = tail[0] ? (char)tail[1] : '.';
    char line[] = "UAKID: case ? starting\r\n";
    long h;

    line[12] = c;
    Cconws(line);

    switch (c) {
    /* ---- GEMDOS ---------------------------------------------------- */
    case 'c': G(0x09, BAD, 0, 0); break; /* Cconws, the 0c pilot */
    case 'o': G(0x3D, BAD, 0, 0); break;
    case 'n': G(0x56, 0, "C:\\UAKID.NAP", BAD); break;
    case 'w': G(0x40, 1, 5, BAD); break;
    case 'g': G(0x47, BAD, 0, 0); break;
    case 'r':
        h = (long)G(0x3D, "C:\\UAKID.NAP", 0, 0);
        if (h < 0) { Cconws("UAKID: no file\r\n"); return 3; }
        G(0x3F, h, 16, BAD);
        break;
    case 'W':
        h = (long)G(0x3C, "C:\\UAKID.TMP", 0, 0);
        if (h < 0) { Cconws("UAKID: no create\r\n"); return 3; }
        G(0x40, h, 16, BAD);
        break;
    case 'd':
        G(0x1A, BAD, 0, 0);
        G(0x4E, "C:\\*.*", 0x37, 0);
        break;
    case 'D':
        /* TASK/MEM11: BAD is >= 0x80000000. Fgetdta must hand the program's
         * own DTA back -- the same 32 bits, as EmuTOS `return run->p_xdta`
         * (bdos/fsmain.c:396) -- and a debug kernel must not trip its
         * pointer check on it (user input, not a kernel invariant). Only
         * Fsfirst's write to the DTA ends the program (bus error). */
        G(0x1A, BAD, 0, 0);
        if ((unsigned int)G(0x2F, 0, 0, 0) != (unsigned int)BAD) {
            Cconws("UAKID: Fgetdta did not return the DTA set -- FAIL\r\n");
            return 4;
        }
        Cconws("UAKID: Fgetdta returned the DTA set\r\n");
        G(0x4E, "C:\\*.*", 0x37, 0);
        break;
    case 'f': G(0x36, BAD, 0, 0); break;
    case 't':
        h = (long)G(0x3D, "C:\\UAKID.NAP", 0, 0);
        if (h < 0) { Cconws("UAKID: no file\r\n"); return 3; }
        G(0x57, BAD, h, 0);
        break;
    /* ---- BIOS / XBIOS / Ssystem / Cconrs ---------------------------- */
    case 'k': G(0x0A, BAD, 0, 0); break;
    case 's': G(0x154, 8, 0, BAD); break; /* tag 0: the jar capacity, always found */
    case 'm': B(0x00, BAD, 0, 0, 0, 0, 0); break;
    case 'b': B(0x04, 0, BAD, 1, 0, 2, 0); break;
    case 'P': X(0x06, BAD, 0, 0, 0); break;
    case 'T': X(0x12, BAD, 0x01000000UL, 2, 0); break;
    case 'K': X(0x10, BAD, -1L, -1L, 0); break;
    /* ---- AES / VDI --------------------------------------------------- */
    case 'A': sys(2, 0, 200, BAD, 0, 0, 0, 0); break;
    case 'I':
        aespb.ii = (void *)BAD;
        intin[0] = 0;
        aes(78, 1, 1, 1, 0); /* graf_mouse(ARROW) */
        break;
    case 'G':
        appl_init();
        aespb.g = (void *)BAD;
        aes(10, 0, 1, 0, 0); /* appl_init writes global[] */
        break;
    case 'F':
        appl_init();
        addrin[0] = BAD;
        aes(13, 0, 1, 1, 0); /* appl_find(name) */
        break;
    case 'V': sys(2, 0, 115, BAD, 0, 0, 0, 0); break;
    case 'C':
        vdipb.c = (void *)BAD;
        sys(2, 0, 115, (UL)&vdipb, 0, 0, 0, 0);
        break;
    case 'M': {
        static struct { UL addr; short w, h, wdw, std, planes, r1, r2, r3; } src, dst;
        short handle;
        int i;
        appl_init();
        aes(77, 0, 5, 0, 0); /* graf_handle */
        handle = intout[0];
        for (i = 0; i < 10; i++) vintin[i] = 1;
        vintin[10] = 2;
        vdi(100, 0, 11, handle); /* v_opnvwk */
        handle = contrl[6];
        src.addr = 0;
        /* 16 x 16 pixels, one WORD of width per plane row, device planes */
        dst.addr = BAD; dst.w = 16; dst.h = 16; dst.wdw = 1; dst.std = 0;
        vintin[0] = 1;
        vdi(102, 0, 1, handle); /* vq_extnd(1): intout[4] = planes */
        dst.planes = vintout[4];
        vintin[0] = 3; /* S_ONLY */
        ptsin[0] = 0; ptsin[1] = 0; ptsin[2] = 15; ptsin[3] = 15;
        ptsin[4] = 0; ptsin[5] = 0; ptsin[6] = 15; ptsin[7] = 15;
        *(UL *)&contrl[7] = (UL)&src;   /* N_PTRINTS = 4 (LP64) */
        *(UL *)&contrl[11] = (UL)&dst;
        vdi(109, 4, 1, handle); /* vro_cpyfm */
        break;
    }
    case 'E':
        appl_init();
        tree[1].spec = (UL)&ted;
        addrin[0] = (UL)tree;
        intin[0] = 1; intin[1] = 'A'; intin[2] = 0; intin[3] = 2; /* ED_CHAR */
        aes(46, 4, 2, 1, 0); /* objc_edit */
        break;
    case 'X':
        appl_init();
        menu[1].spec = (UL)" Item ";
        menu[2].spec = (UL)" Two ";
        addrin[0] = (UL)menu;
        addrin[1] = BAD;
        intin[0] = 1;
        aes(34, 1, 1, 2, 0); /* menu_text */
        break;
    case '.':
        Cconws("UAKID: clean exit\r\n");
        return 0;
    default:
        Cconws("UAKID: unknown case\r\n");
        return 2;
    }
    Cconws("UAKID: survived -- FAIL\r\n");
    return 1;
}
