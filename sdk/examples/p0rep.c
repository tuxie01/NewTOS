/*
 * p0rep.c -- P0PAR repeated: the MEM12 page-0 probe as an unattended
 * endurance run (TASK/PITEST01 Part 1, step A1 of the real-Pi test scenario).
 * Same child (C:\P0KID.NAP, sdk/examples/p0kid.c), same cases, same expected
 * exit codes as sdk/examples/p0par.c: 0x0000FFFF for the fault cases
 * w x r W T (crash report, Pterm(-1)), 0 for R S E K L . and, after every L,
 * the parent's own state must still be the user state. Each case runs once
 * per round; a round is the whole case string once.
 *
 * ONLY FOR THE crash-nowait KERNEL (kernel feature `crash-nowait`, the phase-B
 * test kernel): there a crash report is logged and the child terminated
 * without a key. On a normal kernel every fault case stops at "Press any key".
 * Detection, round 1 only: each case is timed with _hz_200 (readable in user
 * state on NewTOS); before round 1 one uncounted run of case '.' gives the
 * baseline B (load + start + exit, no report). A fault case of round 1
 * taking longer than 4*B + 200 ticks (one second over four baselines) took a
 * key wait or a stall; the second such case of round 1 is taken as "the
 * report waits for a key": P0PAR-REP prints "P0PAR-REP: STOP ..." and ends
 * with rc 2 (on the release kernel after two answered prompts; one slow case
 * alone, a stall, only gets a NOTE line). A tester who answers the prompts of
 * round 1 faster than that is not caught -- the run sheet allows P0REP in
 * phase B only.
 *
 * Timing: NewTOS mirrors the 200-Hz counter into the page 0 of the RUNNING
 * process only, on each tick (tos-api/src/bios/tick.rs, publish). A parent
 * that has just got control back from its child still reads the value of its
 * own last tick, from before the child ran, until the next tick (at most
 * 5 ms). The first P0REP read $4BA directly around Pexec and saw no time
 * pass for a 7.5-s key wait (measurements/measurement-2026-10-31-pitest01-
 * part1/RUNINFO.TXT, run v1). So every time stamp is taken
 * at the next tick (hz200_next_tick), never from a value the cell may still
 * hold from before.
 *
 * Command tail: the number of rounds, decimal, 1..100000; default 1000.
 *
 * Output:
 *   screen   one line per WRONG (case, round, rc), a progress line every 100
 *            rounds, and the summary
 *              P0PAR-REP: PASS <ok>/<total> (<n> rounds)
 *              P0PAR-REP: FAIL <ok>/<total> wrong=<w> (<n> rounds)
 *            total = rounds * 12 checks (11 case exit codes + the own-state
 *            check after L).
 *   C:\P0PAR.LOG  per case: expected rc, ok, wrong, the first wrong round
 *            and its rc, the largest and the mean time of one run (Pexec to
 *            return) in 200-Hz ticks; the baseline B and the count of
 *            next-tick waits that timed out (0 expected); the L-state
 *            check; the summary line. Rewritten after round 1, every 100
 *            rounds and at the end (a hang leaves the last checkpoint).
 *   rc       0 PASS, 1 FAIL, 2 STOP (not the crash-nowait kernel),
 *            3 bad command tail
 *
 * Build: sdk/build.sh sdk/examples/p0rep.c <out>/P0REP.NAP
 */
#include <newtos.h>

static const char cases[] = "RwSxErWTKL.";
static const char faults[] = "wxrWT";
#define NCASES 11

#define HZ_200   0x4BAUL
#define SYSBASE  0x4F2UL
#define P_CMDLIN 0x80
#define MAX_ROUNDS 100000UL
#define DEFAULT_ROUNDS 1000UL

struct cstat {
    unsigned long ok, wrong, first_round, first_rc, max_ticks, sum_ticks;
};

static struct cstat st[NCASES];
static unsigned long lstate_ok, lstate_wrong, lstate_first, log_base;
static char logbuf[4096];
static int loglen;

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

static NLONG Fwrite(short handle, long count, const void *buf)
{
    register NLONG x8 __asm__("x8") = 0x40;
    register NLONG r0 __asm__("x0") = handle;
    register NLONG r1 __asm__("x1") = count;
    register NLONG r2 __asm__("x2") = (NLONG)buf;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8), "r"(r1), "r"(r2) : "memory");
    return r0;
}

static long Super(long stack)
{
    register long x8 __asm__("x8") = 0x20;
    register long r0 __asm__("x0") = stack;
    __asm__ volatile("svc #1" : "+r"(r0) : "r"(x8) : "memory");
    return r0;
}

static unsigned long hz200(void)
{
    return *(volatile unsigned int *)HZ_200;
}

/*
 * The counter at the next tick: wait until the cell changes; the new value
 * was written by a tick while this process ran, so it is current (see
 * "Timing"). Bounded: should the cell never change, the old value is returned
 * and the call counted in tick_timeouts.
 */
static unsigned long tick_timeouts;

static unsigned long hz200_next_tick(void)
{
    unsigned long v = hz200(), n;
    unsigned long spins;

    for (spins = 0; spins < (1UL << 28); spins++) {
        n = hz200();
        if (n != v)
            return n;
    }
    tick_timeouts++;
    return v;
}

static const unsigned char *own_basepage(void)
{
    unsigned long sysbase = *(volatile unsigned int *)SYSBASE;
    unsigned long run_cell = *(volatile unsigned int *)(sysbase + 0x28);
    return (const unsigned char *)(unsigned long)*(volatile unsigned int *)run_cell;
}

static int is_fault(char c)
{
    const char *p;
    for (p = faults; *p; p++)
        if (*p == c)
            return 1;
    return 0;
}

/* ---- text output (no libc) ------------------------------------------------ */

static void lput(const char *s)
{
    while (*s && loglen < (int)sizeof(logbuf) - 1)
        logbuf[loglen++] = *s++;
    logbuf[loglen] = '\0';
}

/* Decimal into buf (at least 21 bytes); returns buf. */
static char *dec(char *buf, unsigned long v)
{
    char tmp[21];
    int n = 0, i = 0;
    do {
        tmp[n++] = (char)('0' + v % 10);
        v /= 10;
    } while (v);
    while (n)
        buf[i++] = tmp[--n];
    buf[i] = '\0';
    return buf;
}

/* "0x" + 8 hex digits into buf (at least 11 bytes); returns buf. */
static char *hex32(char *buf, unsigned long v)
{
    int i;
    buf[0] = '0';
    buf[1] = 'x';
    for (i = 0; i < 8; i++) {
        unsigned d = (unsigned)(v >> (28 - 4 * i)) & 0xF;
        buf[2 + i] = (char)(d < 10 ? '0' + d : 'A' + d - 10);
    }
    buf[10] = '\0';
    return buf;
}

/* Parse the command tail: a decimal round count, or empty. -1 = invalid. */
static long parse_rounds(const unsigned char *cmd)
{
    unsigned len = cmd[0];
    unsigned long v = 0;
    unsigned i;
    int digits = 0;

    for (i = 1; i <= len && i < 128; i++) {
        unsigned char c = cmd[i];
        if (c == ' ' && !digits)
            continue;
        if (c == ' ' || c == '\r' || c == '\0')
            break;
        if (c < '0' || c > '9')
            return -1;
        v = v * 10 + (c - '0');
        digits++;
        if (v > MAX_ROUNDS)
            return -1;
    }
    if (!digits)
        return (long)DEFAULT_ROUNDS;
    if (v == 0)
        return -1;
    return (long)v;
}

static unsigned long total_ok(void)
{
    unsigned long s = lstate_ok;
    int i;
    for (i = 0; i < NCASES; i++)
        s += st[i].ok;
    return s;
}

static unsigned long total_wrong(void)
{
    unsigned long s = lstate_wrong;
    int i;
    for (i = 0; i < NCASES; i++)
        s += st[i].wrong;
    return s;
}

static int cat(char *buf, int l, const char *s)
{
    while (*s)
        buf[l++] = *s++;
    buf[l] = '\0';
    return l;
}

/* The summary line (no CR/LF) into buf (at least 96 bytes). */
static void summary(char *buf, unsigned long rounds_done)
{
    char n[21];
    unsigned long ok = total_ok(), wrong = total_wrong();
    int l = cat(buf, 0, wrong ? "P0PAR-REP: FAIL " : "P0PAR-REP: PASS ");

    l = cat(buf, l, dec(n, ok));
    l = cat(buf, l, "/");
    l = cat(buf, l, dec(n, ok + wrong));
    if (wrong) {
        l = cat(buf, l, " wrong=");
        l = cat(buf, l, dec(n, wrong));
    }
    l = cat(buf, l, " (");
    l = cat(buf, l, dec(n, rounds_done));
    cat(buf, l, " rounds)");
}

static void write_log(unsigned long rounds, unsigned long done, int final)
{
    char a[21], h[11], sum[96];
    NLONG fh;
    int i;

    loglen = 0;
    logbuf[0] = '\0';
    lput("P0PAR-REP rounds=");
    lput(dec(a, rounds));
    lput(" done=");
    lput(dec(a, done));
    lput(final ? " final\r\n" : " checkpoint\r\n");
    lput("case want ok wrong first-wrong-round first-wrong-rc max-ticks mean-ticks\r\n");
    for (i = 0; i < NCASES; i++) {
        char c[2] = { cases[i], '\0' };
        lput(c);
        lput("    ");
        lput(hex32(h, is_fault(cases[i]) ? 0xFFFFUL : 0UL));
        lput(" ");
        lput(dec(a, st[i].ok));
        lput(" ");
        lput(dec(a, st[i].wrong));
        lput(" ");
        lput(st[i].wrong ? dec(a, st[i].first_round) : "-");
        lput(" ");
        lput(st[i].wrong ? hex32(h, st[i].first_rc) : "-");
        lput(" ");
        lput(dec(a, st[i].max_ticks));
        lput(" ");
        lput(dec(a, (st[i].ok + st[i].wrong) ? st[i].sum_ticks / (st[i].ok + st[i].wrong) : 0));
        lput("\r\n");
    }
    lput("baseline-ticks=");
    lput(dec(a, log_base));
    lput(" next-tick-timeouts=");
    lput(dec(a, tick_timeouts));
    lput("\r\n");
    lput("L-own-state ok=");
    lput(dec(a, lstate_ok));
    lput(" wrong=");
    lput(dec(a, lstate_wrong));
    if (lstate_wrong) {
        lput(" first-wrong-round=");
        lput(dec(a, lstate_first));
    }
    lput("\r\n");
    summary(sum, done);
    lput(sum);
    lput("\r\n");

    fh = gemdos2(0x3C, (NLONG) "C:\\P0PAR.LOG", 0); /* Fcreate */
    if (fh < 0) {
        Cconws("P0PAR-REP: cannot create C:\\P0PAR.LOG\r\n");
        return;
    }
    if (Fwrite((short)fh, loglen, logbuf) != loglen)
        Cconws("P0PAR-REP: short write to C:\\P0PAR.LOG\r\n");
    gemdos1(0x3E, fh); /* Fclose */
}

int main(void)
{
    char tail[3];
    char a[21], h[11], sum[96];
    long rounds = parse_rounds(own_basepage() + P_CMDLIN);
    unsigned long r, base = 0, slow1 = 0;
    int i;

    if (rounds < 0) {
        Cconws("P0PAR-REP: command tail must be a round count 1..100000\r\n");
        return 3;
    }
    Cconws("P0PAR-REP: ");
    Cconws(dec(a, (unsigned long)rounds));
    Cconws(" rounds of RwSxErWTKL. -- needs the crash-nowait kernel\r\n");

    /* The baseline for the key-wait detection: one uncounted clean run. */
    tail[0] = 1;
    tail[1] = '.';
    tail[2] = '\0';
    {
        unsigned long t0 = hz200_next_tick();
        NLONG rc = Pexec(0, "C:\\P0KID.NAP", tail, 0);
        base = (hz200_next_tick() - t0) & 0xFFFFFFFFUL;
        log_base = base;
        if (rc != 0) {
            Cconws("P0PAR-REP: baseline run of case . failed, rc=");
            Cconws(hex32(h, (unsigned long)rc & 0xFFFFFFFFUL));
            Cconws("\r\n");
            return 1;
        }
    }

    for (r = 1; r <= (unsigned long)rounds; r++) {
        for (i = 0; i < NCASES; i++) {
            char c = cases[i];
            unsigned long want = is_fault(c) ? 0xFFFF : 0;
            unsigned long t0, dt;
            NLONG rc;

            tail[0] = 1;
            tail[1] = c;
            tail[2] = '\0';
            t0 = hz200_next_tick();
            rc = Pexec(0, "C:\\P0KID.NAP", tail, 0);
            dt = (hz200_next_tick() - t0) & 0xFFFFFFFFUL;
            if (dt > st[i].max_ticks)
                st[i].max_ticks = dt;
            st[i].sum_ticks += dt;
            if (r == 1 && is_fault(c) && dt > 4 * base + 200) {
                char cs[2] = { c, '\0' };
                Cconws(++slow1 < 2 ? "P0PAR-REP: NOTE -- case " : "P0PAR-REP: STOP -- case ");
                Cconws(cs);
                Cconws(" took ");
                Cconws(dec(a, dt));
                Cconws(" ticks, baseline ");
                Cconws(dec(a, base));
                if (slow1 < 2) {
                    Cconws("\r\n");
                } else {
                    Cconws(": the crash report waits for a key -- not the crash-nowait kernel\r\n");
                    return 2;
                }
            }
            if ((unsigned long)rc == want) {
                st[i].ok++;
            } else {
                char cs[2] = { c, '\0' };
                if (!st[i].wrong) {
                    st[i].first_round = r;
                    st[i].first_rc = (unsigned long)rc & 0xFFFFFFFFUL;
                }
                st[i].wrong++;
                Cconws("P0PAR-REP: WRONG case ");
                Cconws(cs);
                Cconws(" round ");
                Cconws(dec(a, r));
                Cconws(" rc=");
                Cconws(hex32(h, (unsigned long)rc & 0xFFFFFFFFUL));
                Cconws("\r\n");
            }
            if (c == 'L') {
                /* S7: a child's super state ends with it; ours is untouched. */
                if (Super(1L) == 0) {
                    lstate_ok++;
                } else {
                    if (!lstate_wrong)
                        lstate_first = r;
                    lstate_wrong++;
                    Cconws("P0PAR-REP: WRONG own state after L, round ");
                    Cconws(dec(a, r));
                    Cconws("\r\n");
                }
            }
        }
        if (r == 1 || r % 100 == 0) {
            if (r % 100 == 0) {
                Cconws("P0PAR-REP: round ");
                Cconws(dec(a, r));
                Cconws(" wrong ");
                Cconws(dec(a, total_wrong()));
                Cconws("\r\n");
            }
            if (r != (unsigned long)rounds)
                write_log((unsigned long)rounds, r, 0);
        }
    }
    write_log((unsigned long)rounds, (unsigned long)rounds, 1);
    summary(sum, (unsigned long)rounds);
    Cconws(sum);
    Cconws("\r\n");
    return total_wrong() ? 1 : 0;
}
