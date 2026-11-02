/*
 * cwpar.c -- parent driver for the crash-walk verification (TASK/PROC08).
 * Starts C:\CWKID.NAP once per case of its command tail (default "jdgwx.")
 * with Pexec(PE_LOADGO) and prints the exit code: 0x0000FFFF for a fault
 * case (crash report, Pterm(-1)), 0 for '.'. Meant for a kernel with
 * `crash-walk,crash-nowait` (no key wait); on a kernel without
 * `crash-nowait` the first fault case waits at the prompt.
 *
 * Build: sdk/build.sh <this file> <out>/CWPAR.NAP
 */
#include <newtos.h>

typedef unsigned long UL;

#define SYSBASE 0x4F2UL
#define P_CMDLIN 0x80

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

static const unsigned char *own_basepage(void)
{
    UL sysbase = *(volatile unsigned int *)SYSBASE;
    UL run_cell = *(volatile unsigned int *)(sysbase + 0x28);
    return (const unsigned char *)(UL)*(volatile unsigned int *)run_cell;
}

static __attribute__((noinline)) void put_hex(UL v)
{
    volatile char buf[11];
    int i;
    buf[0] = '0';
    buf[1] = 'x';
    for (i = 0; i < 8; i++) {
        unsigned d = (unsigned)(v >> (28 - 4 * i)) & 0xF;
        buf[2 + i] = (char)(d < 10 ? '0' + d : 'A' + d - 10);
    }
    buf[10] = '\0';
    Cconws((const char *)buf);
}

int main(void)
{
    const unsigned char *t = own_basepage() + P_CMDLIN;
    char cases[64] = "jdgwx.";
    char tail[3] = { 1, 0, 0 };
    char line[] = "CWPAR: case ? rc ";
    int i, n = t[0];

    if (n > 0 && n < 63) {
        for (i = 0; i < n; i++)
            cases[i] = (char)t[1 + i];
        cases[n] = 0;
    }
    Cconws("CWPAR: start\r\n");
    for (i = 0; cases[i]; i++) {
        NLONG rc;
        tail[1] = cases[i];
        rc = Pexec(0, "C:\\CWKID.NAP", tail, 0);
        line[12] = cases[i];
        Cconws(line);
        put_hex((UL)rc);
        Cconws("\r\n");
    }
    Cconws("CWPAR: done\r\n");
    return 0;
}
