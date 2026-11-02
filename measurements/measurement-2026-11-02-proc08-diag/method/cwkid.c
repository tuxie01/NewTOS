/*
 * cwkid.c -- one EL0 fault per run for the crash-walk verification
 * (TASK/PROC08, kernel debug feature `crash-walk`). Started by cwpar.c with
 * the case in the command tail; every case except '.' must end this program
 * with the crash report and Pterm(-1).
 *
 *   d  read a pool page this program just freed (Malloc + Mfree): not mapped
 *   j  jump to that freed page: instruction abort, not mapped
 *   g  read 0x90000000 (GiB 2, nothing user there on raspi4b): not mapped
 *   w  write conterm ($484, page 0, read-only in user state): mapped
 *   x  call code at $700 (page 0, never executable): mapped
 *   .  clean exit
 *
 * Before the fault it prints the address it uses, so the [crashwalk] lines
 * can be checked against it.
 *
 * Build: sdk/build.sh <this file> <out>/CWKID.NAP
 */
#include <newtos.h>

typedef unsigned long UL;

#define SYSBASE 0x4F2UL
#define P_CMDLIN 0x80

static NLONG Malloc(NLONG n)
{
    register NLONG x8 __asm__("x8") = 0x48;
    register NLONG r0 __asm__("x0") = n;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8) : "memory");
    return r0;
}

static NLONG Mfree(NLONG p)
{
    register NLONG x8 __asm__("x8") = 0x49;
    register NLONG r0 __asm__("x0") = p;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8) : "memory");
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
    volatile char buf[19];
    int i;
    buf[0] = '0';
    buf[1] = 'x';
    for (i = 0; i < 16; i++) {
        unsigned d = (unsigned)(v >> (60 - 4 * i)) & 0xF;
        buf[2 + i] = (char)(d < 10 ? '0' + d : 'A' + d - 10);
    }
    buf[18] = '\0';
    Cconws((const char *)buf);
}

static UL freed_page(void)
{
    NLONG p = Malloc(0x10000);
    if (p <= 0) {
        Cconws("CWKID: Malloc failed\r\n");
        Pterm(3);
    }
    if (Mfree(p) != 0) {
        Cconws("CWKID: Mfree failed\r\n");
        Pterm(4);
    }
    return (UL)p;
}

int main(void)
{
    const unsigned char *tail = own_basepage() + P_CMDLIN;
    char c = tail[0] ? (char)tail[1] : '.';
    char line[] = "CWKID: case ? addr ";
    UL a;

    switch (c) {
    case 'd': case 'j': a = freed_page(); break;
    case 'g': a = 0x90000000UL; break;
    case 'w': a = 0x484UL; break;
    case 'x': a = 0x700UL; break;
    default: a = 0; break;
    }
    line[12] = c;
    Cconws(line);
    put_hex(a);
    Cconws("\r\n");

    switch (c) {
    case 'd': (void)*(volatile unsigned int *)a; break;
    case 'j': ((void (*)(void))a)(); break;
    case 'g': (void)*(volatile unsigned int *)a; break;
    case 'w': *(volatile unsigned char *)a = 7; break;
    case 'x': ((void (*)(void))a)(); break;
    case '.': return 0;
    default:
        Cconws("CWKID: unknown case\r\n");
        return 9;
    }
    Cconws("CWKID: survived\r\n");
    return 1;
}
