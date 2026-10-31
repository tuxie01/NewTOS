/*
 * hzprobe.c -- does _hz_200 ($4BA) keep counting for a program in user state,
 * also while it switches between user and super state all the time?
 * (TASK/PITEST01 Part 1; step A2 of the real-Pi test scenario,
 * docs/analysis/tester-scenario-draft-2026-10-31.md; DOOM01 candidate K6.)
 *
 * NewTOS (AArch64) only.  On TOS page 0 is supervisor-only, so a user-state
 * read of $4BA is a bus error there; on NewTOS a program reads its own page 0
 * in user state (MEM12, docs/api/_super-state.md; P0KID case R).  The probe
 * checks NewTOS' own mirror: the kernel credits the 200 Hz ticks of the ARM
 * generic timer and copies the counter into the running process's page 0 on
 * every tick (tos-api/src/bios/tick.rs).
 *
 * Reference clock: Tgettime.  On NewTOS it is the generic timer's counter
 * itself (CNTPCT_EL0 / CNTFRQ_EL0, kernel host monotonic_seconds), not the
 * tick interrupt -- independent of interrupt delivery and of the page-0
 * mirror, with 2-s resolution.  The probe measures from one Tgettime edge
 * (the 2-s field changes) to the first edge at least <secs> seconds later, so
 * the reference interval is exact to one loop pass.
 *
 * Loop, until that end edge:
 *   64 reads of $4BA in user state; Super(0L); 64 reads in super state;
 *   Super(token); Tgettime.
 * Every read is compared with the previous one (whatever the state): the
 * step.  A step of 1 is a normal tick; a step of k > 1 means the counter
 * stood still for k-1 ticks longer than the clock ran.
 *
 * PASS rule (all must hold):
 *   P1  no step > 2 in either state: the visible counter never lags the
 *       kernel clock by more than one tick ("kein Stillstand laenger als ein
 *       Tick gegenueber der Kern-Uhr");
 *   P2  total ticks between the two edges within 1 % of 200 per second of
 *       Tgettime;
 *   P3  no step backwards;
 *   P4  the counter advanced in user state and in super state (at least one
 *       step observed by a read in each state);
 *   P5  every Super(0L) returned a token (not 0, 1 or negative) and every
 *       Super(token) returned to user state (Super(1L) == 0 afterwards, on the
 *       first and every 1024th pass).
 * Informational: the step histogram, the ticks of every 2-s interval (min,
 * max; nominal 400), passes and Super toggles.
 *
 * Not covered: the counter right after a process switch. The kernel writes
 * it into the running process's page 0 only, so a parent that has just got
 * control back from a child reads its stale value until the next tick
 * (sdk/examples/p0rep.c, "Timing"). This probe starts no child.
 *
 * Command tail: the measuring time in seconds, 4..3600, default 30 (rounded
 * up to the next even number: Tgettime counts 2-s units).
 *
 * Output: C:\HZPROBE.LOG (written once at the end), and on screen the start
 * line and the summary
 *   HZPROBE: PASS ticks=<n>/<expected> maxstep=<k>
 *   HZPROBE: FAIL ticks=<n>/<expected> maxstep=<k> <failed rules>
 * Exit code 0 PASS, 1 FAIL, 3 bad command tail.
 *
 * The pure part (Tgettime arithmetic, step bookkeeping, the PASS rule) is in
 * hzprobe-rules.h, host-tested by tosdump/tests/test_hzprobe.c.
 *
 * Built by `make -C tosdump hzprobe-newtos` (tosdump/build-hzprobe-newtos.sh,
 * HZPROBE.NAP); run in QEMU by tosdump/probes/run-hzprobe.sh.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "hzprobe-rules.h"

#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))
#define HZ_200 0x4BA
#define READS 64

static char out[4096];
static int olen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 200)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

static struct hz_side user, sup;
static struct hz_track trk;

int main(int argc, char **argv)
{
	unsigned long secs = 30, t, e0_sec, e_sec, hz_e0, hz_prev_edge, hz_end;
	unsigned long passes = 0, toggles = 0, bad_token = 0, bad_inquiry = 0;
	unsigned long edges = 0, imin = ~0UL, imax = 0, total, expected, tol, maxstep;
	unsigned long guard;
	unsigned verdict;
	char fails[64];
	long fh;
	int i;

	if (argc > 1) {
		char *end;
		long v = strtol(argv[1], &end, 10);
		if (*end != '\0' || v < 4 || v > 3600) {
			(void)Cconws("HZPROBE: command tail must be 4..3600 seconds\r\n");
			return 3;
		}
		secs = (unsigned long)v;
	}
	secs = (secs + 1) & ~1UL;
	printf("HZPROBE: %lu s, _hz_200 in user and super state against Tgettime\r\n", secs);
	fflush(stdout);

	/* The start edge: wait for the 2-s field to change (bounded by 3 s of
	 * _hz_200, should that one run and Tgettime not). */
	t = (unsigned long)Tgettime() & 0xFFFF;
	guard = CELL(HZ_200);
	for (;;) {
		unsigned long n = (unsigned long)Tgettime() & 0xFFFF;
		if (n != t) {
			t = n;
			break;
		}
		if (((CELL(HZ_200) - guard) & 0xFFFFFFFFUL) > 600) {
			(void)Cconws("HZPROBE: FAIL Tgettime does not advance\r\n");
			return 1;
		}
	}
	hz_e0 = hz_prev_edge = trk.last = CELL(HZ_200);
	e0_sec = e_sec = hz_secs_of(t);
	hz_end = hz_e0;

	for (;;) {
		long tok;
		unsigned long n;

		for (i = 0; i < READS; i++)
			hz_observe(&trk, &user, CELL(HZ_200));
		tok = Super(0L);
		if (tok == 0 || tok == 1 || tok < 0)
			bad_token++;
		for (i = 0; i < READS; i++)
			hz_observe(&trk, &sup, CELL(HZ_200));
		if (tok != 0 && tok != 1 && tok >= 0)
			SuperToUser((void *)tok);
		toggles++;
		if ((passes & 1023) == 0 && Super(1L) != 0)
			bad_inquiry++;
		passes++;

		n = (unsigned long)Tgettime() & 0xFFFF;
		if (n != t) {
			unsigned long hz = CELL(HZ_200);
			unsigned long iv = (hz - hz_prev_edge) & 0xFFFFFFFFUL;
			unsigned long s = hz_secs_of(n);
			/* the ticks of one 2-s interval (only when exactly 2 s apart) */
			if (hz_since(e_sec, s) == 2) {
				if (iv < imin)
					imin = iv;
				if (iv > imax)
					imax = iv;
			}
			edges++;
			t = n;
			e_sec = s;
			hz_prev_edge = hz;
			if (hz_since(e0_sec, s) >= secs) {
				hz_end = hz;
				break;
			}
		}
		/* Bound by _hz_200 too, should Tgettime stop. */
		if (((CELL(HZ_200) - hz_e0) & 0xFFFFFFFFUL) > secs * 400UL) {
			hz_end = CELL(HZ_200);
			break;
		}
	}

	total = (hz_end - hz_e0) & 0xFFFFFFFFUL;
	expected = 200UL * hz_since(e0_sec, e_sec);
	tol = expected / 100;
	maxstep = user.maxstep > sup.maxstep ? user.maxstep : sup.maxstep;
	verdict = hz_verdict(&user, &sup, &trk, total, expected, bad_token + bad_inquiry);

	fails[0] = '\0';
	if (verdict & HZ_FAIL_P1)
		strcat(fails, " P1");
	if (verdict & HZ_FAIL_P2)
		strcat(fails, " P2");
	if (verdict & HZ_FAIL_P3)
		strcat(fails, " P3");
	if (verdict & HZ_FAIL_P4)
		strcat(fails, " P4");
	if (verdict & HZ_FAIL_P5)
		strcat(fails, " P5");

	emit("HZPROBE (TASK/PITEST01) %lu s requested\r\n", secs);
	emit("reference  Tgettime %02lu:%02lu:%02lu -> %02lu:%02lu:%02lu = %lu s, edges %lu\r\n",
	     e0_sec / 3600, e0_sec / 60 % 60, e0_sec % 60, e_sec / 3600, e_sec / 60 % 60,
	     e_sec % 60, hz_since(e0_sec, e_sec), edges);
	emit("ticks      %lu of %lu expected (200/s), diff %ld, tolerance %lu (1 %%)\r\n", total,
	     expected, (long)total - (long)expected, tol);
	emit("interval   ticks per 2-s interval min %lu max %lu (nominal 400)\r\n",
	     imin == ~0UL ? 0UL : imin, imax);
	emit("passes     %lu, Super toggles %lu, bad tokens %lu, bad Super(1L) %lu\r\n", passes,
	     toggles, bad_token, bad_inquiry);
	emit("user       reads %lu steps %lu maxstep %lu hist 1:%lu 2:%lu 3-10:%lu >10:%lu\r\n",
	     user.reads, user.steps, user.maxstep, user.hist[0], user.hist[1], user.hist[2],
	     user.hist[3]);
	emit("super      reads %lu steps %lu maxstep %lu hist 1:%lu 2:%lu 3-10:%lu >10:%lu\r\n",
	     sup.reads, sup.steps, sup.maxstep, sup.hist[0], sup.hist[1], sup.hist[2],
	     sup.hist[3]);
	emit("backwards  %lu\r\n", trk.backwards);
	emit("rules      P1 maxstep<=2  P2 ticks within 1 %%  P3 no step back  P4 counts in "
	     "both states  P5 Super tokens\r\n");
	emit("HZPROBE: %s ticks=%lu/%lu maxstep=%lu%s\r\n", fails[0] ? "FAIL" : "PASS", total,
	     expected, maxstep, fails);

	fh = Fcreate("C:\\HZPROBE.LOG", 0);
	if (fh >= 0) {
		if (Fwrite((short)fh, olen, out) != olen)
			(void)Cconws("HZPROBE: short write to C:\\HZPROBE.LOG\r\n");
		Fclose((short)fh);
	} else {
		(void)Cconws("HZPROBE: cannot create C:\\HZPROBE.LOG\r\n");
	}
	printf("HZPROBE: %s ticks=%lu/%lu maxstep=%lu%s\r\n", fails[0] ? "FAIL" : "PASS", total,
	       expected, maxstep, fails);
	fflush(stdout);
	return fails[0] ? 1 : 0;
}
