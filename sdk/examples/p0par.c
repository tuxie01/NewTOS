/*
 * p0par.c -- parent driver for the MEM12 page-0 probe (TASK/MEM12,
 * docs/api/_super-state.md S2, S4-S7, S9). Starts C:\P0KID.NAP once per case
 * with Pexec(PE_LOADGO) and checks the exit code: 0x0000FFFF for the fault
 * cases (a write to page 0 in user state, an instruction fetch from page 0, a
 * read and a write of the execute-only stub page, a write right after
 * leaving the super state: crash report, Pterm(-1),
 * the machine keeps running), 0 for the others. After case 'L' (the child ended
 * in super state) it checks that its own state is still the user state. One
 * key per fault case answers the crash prompt. Prints one line per case and
 * "P0PAR: PASS"/"P0PAR: FAIL".
 *
 * Build: sdk/build.sh sdk/examples/p0par.c <out>/P0PAR.NAP
 */
#include <newtos.h>

static const char cases[] = "RwSxErWTKL.";
static const char faults[] = "wxrWT";

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

static long Super(long stack)
{
    register long x8 __asm__("x8") = 0x20;
    register long r0 __asm__("x0") = stack;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8) : "memory");
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

static int is_fault(char c)
{
    const char *p;
    for (p = faults; *p; p++)
        if (*p == c)
            return 1;
    return 0;
}

int main(void)
{
    char tail[3];
    char line[] = "P0PAR: case ? rc=";
    int i, fails = 0;

    Cconws("P0PAR: start\r\n");
    for (i = 0; cases[i]; i++) {
        unsigned long want = is_fault(cases[i]) ? 0xFFFF : 0;
        NLONG rc;

        line[12] = cases[i];
        tail[0] = 1;
        tail[1] = cases[i];
        tail[2] = '\0';
        rc = Pexec(0, "C:\\P0KID.NAP", tail, 0);
        Cconws(line);
        put_hex((unsigned long)rc);
        if ((unsigned long)rc == want) {
            Cconws(" OK\r\n");
        } else {
            Cconws(" WRONG\r\n");
            fails++;
        }
        if (cases[i] == 'L') {
            /* S7: a child's super state ends with it; ours is untouched. */
            long s = Super(1L);
            Cconws(s == 0 ? "P0PAR: own state after L: user OK\r\n"
                          : "P0PAR: own state after L: super WRONG\r\n");
            if (s != 0)
                fails++;
        }
    }
    Cconws(fails ? "P0PAR: FAIL\r\n" : "P0PAR: PASS\r\n");
    return 0;
}
