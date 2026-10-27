/*
 * p0kid.c -- one page-0 / super-state case per run (TASK/MEM12,
 * docs/api/_super-state.md). Started by p0par.c with the case in the command
 * tail. The four fault cases must end THIS program with the crash report and
 * Pterm(-1) -- the parent sees 0x0000FFFF -- and the machine keeps running;
 * the line "P0KID: survived" must never appear for them. Every other case
 * ends with 0 when what it checked holds, else with a small code that names
 * the failed check.
 *
 *   faults (user state)                      allowed
 *   w  write conterm ($484)                  R  read _hz_200, _sysbase -> os_version (0x0404)
 *   x  call code at $700 (page 0, no exec)   S  Super(0L): Super(1L) = -1, write + read back
 *   r  read the stub page ($7E000)              conterm, Super(token), Super(1L) = 0
 *   W  write the stub page                   E  Supexec(callback writing conterm): its value back,
 *   T  Super(0L), write conterm, Super(token),
 *      then write conterm again (the stale-TLB check of the narrowing)
 *                                               Super(1L) = -1 inside, 0 after
 *                                            K  Kbdvbase()[-1] = itself and back (private page)
 *                                            L  Super(0L) and end without leaving: the parent
 *                                               checks that its own state is untouched
 *                                            .  clean exit
 *
 * S reads page 0 before Super(0L) and T writes it after leaving: on real
 * hardware a missing TLB invalidation on the RO <-> RW switch would show here
 * (QEMU often hides it; user decision 2026-10-27, TASK/MEM12).
 *
 * Build: sdk/build.sh sdk/examples/p0kid.c <out>/P0KID.NAP
 */
#include <newtos.h>

typedef unsigned long UL;

#define CONTERM   0x484UL
#define HZ_200    0x4BAUL
#define SYSBASE   0x4F2UL
#define STUB_PAGE 0x7E000UL
#define P_CMDLIN  0x80

static long Super(long stack)
{
    register long x8 __asm__("x8") = 0x20;
    register long r0 __asm__("x0") = stack;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8) : "memory");
    return r0;
}

static long Supexec(long (*f)(void))
{
    register long x8 __asm__("x8") = 0x26;
    register long r0 __asm__("x0") = (long)f;
    __asm__ volatile("svc #14" : "+r"(r0) : "r"(x8)
                     : "memory", "v0", "v1", "v2", "v3", "v4", "v5", "v6", "v7", "v16", "v17",
                       "v18", "v19", "v20", "v21", "v22", "v23", "v24", "v25", "v26", "v27",
                       "v28", "v29", "v30", "v31");
    return r0;
}

static long Kbdvbase(void)
{
    register long x8 __asm__("x8") = 0x22;
    register long r0 __asm__("x0");
    __asm__ volatile("svc #14" : "=r"(r0) : "r"(x8) : "memory");
    return r0;
}

static const unsigned char *own_basepage(void)
{
    UL sysbase = *(volatile unsigned int *)SYSBASE;
    UL run_cell = *(volatile unsigned int *)(sysbase + 0x28);
    return (const unsigned char *)(UL)*(volatile unsigned int *)run_cell;
}

static volatile long in_callback;

static long cb_write_conterm(void)
{
    volatile unsigned char *c = (volatile unsigned char *)CONTERM;
    unsigned char v = *c;
    in_callback = Super(1L);    /* -1: the callback runs in super state */
    *c = (unsigned char)(v ^ 0x01);
    *c = v;
    return 0x12345678L;
}

int main(void)
{
    const unsigned char *tail = own_basepage() + P_CMDLIN;
    char c = tail[0] ? (char)tail[1] : '.';
    char line[] = "P0KID: case ? starting\r\n";
    volatile unsigned char *conterm = (volatile unsigned char *)CONTERM;

    line[12] = c;
    Cconws(line);

    switch (c) {
    /* ---- faults --------------------------------------------------------- */
    case 'w': *conterm = 7; break;
    case 'x': ((void (*)(void))0x700UL)(); break;
    case 'r': (void)*(volatile unsigned int *)STUB_PAGE; break;
    case 'W': *(volatile unsigned int *)STUB_PAGE = 0; break;
    case 'T': {
        long tok = Super(0L);
        unsigned char v = *conterm;
        *conterm = v;           /* allowed: super state, RW entry in the TLB */
        Super(tok);
        *conterm = v;           /* user state again: must fault */
        break;
    }

    /* ---- allowed -------------------------------------------------------- */
    case 'R': {
        UL hz = *(volatile unsigned int *)HZ_200;
        UL sb = *(volatile unsigned int *)SYSBASE;
        if (hz == 0) return 2;
        if (*(volatile unsigned short *)(sb + 2) != 0x0404) return 3;
        return 0;
    }
    case 'S': {
        long tok;
        unsigned char v;
        if (Super(1L) != 0) return 2;
        /* a read first, so the read-only entry is in the TLB when the
         * state widens it */
        if (*(volatile unsigned int *)HZ_200 == 0) return 7;
        tok = Super(0L);
        if (tok == 0 || tok == 1) return 3;
        if (Super(1L) != -1) return 4;
        v = *conterm;
        *conterm = (unsigned char)(v ^ 0x02);
        if (*conterm != (unsigned char)(v ^ 0x02)) return 5;
        *conterm = v;
        Super(tok);
        if (Super(1L) != 0) return 6;
        return 0;
    }
    case 'E': {
        long r;
        in_callback = 0;
        r = Supexec(cb_write_conterm);
        if (r != 0x12345678L) return 2;
        if (in_callback != -1) return 3;
        if (Super(1L) != 0) return 4;
        return 0;
    }
    case 'K': {
        volatile UL *slot = (volatile UL *)(Kbdvbase() - 8);
        UL old = *slot;
        if (old == 0) return 2;
        *slot = old;            /* the private page: writable in user state */
        if (*slot != old) return 3;
        return 0;
    }
    case 'L':
        Super(0L);
        return 0;               /* ends in super state; the parent checks its own */
    case '.':
        return 0;
    default:
        Cconws("P0KID: unknown case\r\n");
        return 9;
    }
    Cconws("P0KID: survived\r\n");
    return 1;
}
