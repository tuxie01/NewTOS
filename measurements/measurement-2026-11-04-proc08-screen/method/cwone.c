/*
 * cwone.c -- parent for the screen check of the crash-walk block
 * (TASK/PROC08, kernel debug feature `crash-walk`). Reads the case letters
 * from C:\CWCASE.TXT (written into each run's card clone) and starts
 * C:\CWKID.NAP (measurements/measurement-2026-11-02-proc08-diag/method/cwkid.c)
 * once per letter with it as the command tail, like cwpar.c of that run.
 * Without crash-nowait the first crashing case stops at the report's key
 * prompt, so one run = one crash screen.
 *
 * Build: sdk/build.sh <this file> <out>/CWONE.NAP
 */
#include <newtos.h>

typedef unsigned long UL;

static NLONG gemdos3(NLONG fn, NLONG a, NLONG b, NLONG c)
{
    register NLONG x8 __asm__("x8") = fn;
    register NLONG r0 __asm__("x0") = a;
    register NLONG r1 __asm__("x1") = b;
    register NLONG r2 __asm__("x2") = c;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2) : "memory");
    return r0;
}

static NLONG Pexec(short mode, const char *path, const char *tail, const char *env)
{
    register NLONG x8 __asm__("x8") = 0x4B;
    register NLONG r0 __asm__("x0") = mode;
    register NLONG r1 __asm__("x1") = (NLONG)path;
    register NLONG r2 __asm__("x2") = (NLONG)tail;
    register NLONG r3 __asm__("x3") = (NLONG)env;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2), "r"(r3) : "memory");
    return r0;
}

int main(void)
{
    char cases[64];
    char tail[3] = { 1, 0, 0 };
    char line[] = "CWONE: case ? rc ";
    NLONG fh, n, i;

    fh = gemdos3(0x3D, (NLONG)"C:\\CWCASE.TXT", 0, 0);   /* Fopen read */
    if (fh < 0) {
        Cconws("CWONE: no C:\\CWCASE.TXT\r\n");
        return 2;
    }
    n = gemdos3(0x3F, fh, 63, (NLONG)cases);              /* Fread */
    (void)gemdos3(0x3E, fh, 0, 0);                        /* Fclose */
    if (n <= 0) {
        Cconws("CWONE: empty C:\\CWCASE.TXT\r\n");
        return 3;
    }
    cases[n] = 0;
    Cconws("CWONE: start\r\n");
    for (i = 0; i < n; i++) {
        NLONG rc;
        if (cases[i] < 'a' || cases[i] > 'z')
            continue;
        tail[1] = cases[i];
        rc = Pexec(0, "C:\\CWKID.NAP", tail, 0);
        line[12] = cases[i];
        Cconws(line);
        Cconws(rc == 0 ? "0\r\n" : "nonzero\r\n");
    }
    Cconws("CWONE: done\r\n");
    return 0;
}
