/*
 * uacc3a.c -- parent driver for the ARCH03 stage-3a bad-pointer probe.
 *
 * Starts C:\UAKID.NAP once per case with Pexec(PE_LOADGO) (an EL0 parent, so
 * every child's end resumes an EL0 program) and checks the exit code:
 * 0x0000FFFF for every bad-pointer case (EmuTOS: bus error in the trap ->
 * dopanic -> Pterm(-1) of the caller), 0 for the closing clean case '.'.
 * Cases marked [disk] in uakid.c need a C: drive; on a machine without one
 * (QEMU virt) they are skipped. One key per crash case answers the crash
 * prompt. Prints one line per case and "UACC3A: PASS"/"UACC3A: FAIL".
 *
 * The case list is UACC3A_CASES (compile-time), default: all of uakid.c.
 *
 * Build: sdk/build.sh sdk/examples/uacc3a.c <out>/UACC3A.NAP
 */
#include <newtos.h>

#ifndef UACC3A_CASES
#define UACC3A_CASES "congwrWdDftksmbPTKAIGFVCMEX."
#endif
static const char disk_cases[] = "rWdDftb";

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

static __attribute__((noinline)) void put_hex(unsigned long v)
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

static int is_disk_case(char c)
{
    const char *p;
    for (p = disk_cases; *p; p++)
        if (*p == c)
            return 1;
    return 0;
}

int main(void)
{
    static const char cases[] = UACC3A_CASES;
    static char dta[44];
    char tail[3];
    char line[] = "UACC3A: case ? rc=";
    int i, fails = 0, has_disk;

    gemdos1(0x1A, (NLONG)dta);
    has_disk = gemdos2(0x4E, (NLONG)"C:\\UAKID.NAP", 0x37) == 0;
    Cconws(has_disk ? "UACC3A: start (C: present)\r\n" : "UACC3A: start (no C:)\r\n");
    for (i = 0; cases[i]; i++) {
        unsigned long want = cases[i] == '.' ? 0 : 0xFFFF;
        NLONG rc;

        line[13] = cases[i];
        if (!has_disk && is_disk_case(cases[i])) {
            Cconws(line);
            Cconws("skipped (no disk)\r\n");
            continue;
        }
        tail[0] = 1;
        tail[1] = cases[i];
        tail[2] = '\0';
        rc = Pexec(0, "C:\\UAKID.NAP", tail, 0);
        Cconws(line);
        put_hex((unsigned long)rc);
        if ((unsigned long)rc == want) {
            Cconws(" OK\r\n");
        } else {
            Cconws(" WRONG\r\n");
            fails++;
        }
    }
    Cconws(fails ? "UACC3A: FAIL\r\n" : "UACC3A: PASS\r\n");
    return 0;
}
