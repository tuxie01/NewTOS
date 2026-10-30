/*
 * vwprobe.c -- a vector cell written straight into low memory in super state:
 * what Setexc answers afterwards, and what is left after the writer's Pterm
 * (TASK/BIOS16, option B; docs/api/bios/05-setexc.md "Page 0 written
 * directly").  A permanent probe of the tosdump tree like sxprobe.c: DUMPBIOS
 * has no case kind "write memory in super state" (tosdump/core/cases.c), so
 * this writes a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * vwprobe-m68k` (VWPROBE.PRG) and `make -C tosdump vwprobe-newtos`
 * (VWPROBE.NAP, tosdump/build-vwprobe-newtos.sh); run by
 * tosdump/probes/run-vwprobe.sh, the oracle side through
 * tools/hatari-oracle.sh profile tt or falcontc.  Output: VWPROBE.LOG in the
 * current directory, written once at the end.
 *
 * EmuTOS: one vector table; setexc reads and writes the LONG at 4 * num
 * (references/emutos/bios/bios.c:1070-1080), so a direct write IS the vector;
 * xterm calls etv_term through Setexc(0x102, -1)
 * (references/emutos/bdos/proc.c:607-608) and puts no vector back.
 *
 * Part W -- Super(0); the LONG at 4 * n := a marker; Setexc(n, -1) in super
 *   state; the old value written back directly; Super(ssp); Setexc(n, -1) in
 *   user state.  n = 0x02 (bus error, $008, the first cell of the range),
 *   0x20 (trap #0), 0xFF ($3FC), 0x101 (etv_critic, $404, the last).  None of
 *   them is taken while the marker is in place (no fault, no trap #0, no disk
 *   error).
 * Part S -- the write in a Supexec callback, Setexc(0x20, -1) after it.
 * Part E -- the same for etv_term ($408): written directly, queried, written
 *   back before anything could end the program.
 * Part T -- a child (this program, argument "child") writes the marker into
 *   $080 (trap #0) in super state, checks Setexc(0x20, -1), leaves the super
 *   state and ends with Pterm without putting the vector back; the parent then
 *   asks Setexc(0x20, -1): the child's marker (EmuTOS) or the old value.  The
 *   parent puts the old value back itself.
 * Part V -- swv_vec ($46E) against OSHEADER reseth and prt_vec ($50A, EmuTOS'
 *   just_rts, references/emutos/bios/chardev.c:82-83): bios.c:363 sets
 *   just_rts, bios.c:552 then reseth.
 *
 * Markers are even, below 2 GiB and the same on both sides; values that
 * differ by machine (the old vectors) are logged as comparisons only.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#ifdef __NEWTOS__
#define CHILD "C:\\VWPROBE.NAP"
#else
#define CHILD "C:\\VWPROBE.PRG"
#endif

static char out[4096];
static int olen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 160)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

static unsigned long lo32(long r)
{
	return (unsigned long)((long long)r & 0xFFFFFFFFLL);
}

#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))
#define SETEXC(n, v) trap_13_wwl((short)5, (short)(n), (long)(v))
#define MARK(n) (0x00ABC000UL | (4UL * (unsigned long)(n)))

/* ---- part W ------------------------------------------------------------- */

static void part_w(void)
{
	static const int nums[4] = { 0x02, 0x20, 0xFF, 0x101 };
	int i;

	for (i = 0; i < 4; i++) {
		int n = nums[i];
		long old = SETEXC(n, -1L), stack, q, back;

		stack = Super(0L);
		CELL(4UL * n) = (uint32_t)MARK(n);
		q = SETEXC(n, -1L);
		CELL(4UL * n) = (uint32_t)lo32(old);
		Super(stack);
		back = SETEXC(n, -1L);
		emit("W%02d n=0x%03X wrote %08lx: Setexc(n,-1) in super state ret=%08lx is-written=%d"
		     " | written back: is-old=%d\r\n",
		     i + 1, n, MARK(n), lo32(q), lo32(q) == MARK(n), lo32(back) == lo32(old));
	}
}

/* ---- part S ------------------------------------------------------------- */

static volatile uint32_t s_val;

static long wr_cell80(void)
{
	CELL(0x80) = s_val;
	return 0;
}

static void part_s(void)
{
	long old = SETEXC(0x20, -1L), q, back;

	s_val = (uint32_t)(MARK(0x20) | 0x100);
	Supexec(wr_cell80);
	q = SETEXC(0x20, -1L);
	s_val = (uint32_t)lo32(old);
	Supexec(wr_cell80);
	back = SETEXC(0x20, -1L);
	emit("S01 Supexec wrote $080=%08lx: Setexc(0x20,-1) in user state ret=%08lx is-written=%d"
	     " | written back: is-old=%d\r\n",
	     (unsigned long)(MARK(0x20) | 0x100), lo32(q), lo32(q) == (MARK(0x20) | 0x100),
	     lo32(back) == lo32(old));
}

/* ---- part E ------------------------------------------------------------- */

static void part_e(void)
{
	long old = SETEXC(0x102, -1L), stack, q, back;

	stack = Super(0L);
	CELL(0x408) = (uint32_t)MARK(0x102);
	q = SETEXC(0x102, -1L);
	CELL(0x408) = (uint32_t)lo32(old);
	Super(stack);
	back = SETEXC(0x102, -1L);
	emit("E01 n=0x102 wrote %08lx: Setexc(n,-1) in super state is-written=%d is-old=%d"
	     " | written back: is-old=%d\r\n",
	     MARK(0x102), lo32(q) == MARK(0x102), lo32(q) == lo32(old), lo32(back) == lo32(old));
}

/* ---- part T ------------------------------------------------------------- */

static int child(void)
{
	long stack, q;

	stack = Super(0L);
	CELL(0x80) = (uint32_t)(MARK(0x20) | 0x200);
	q = SETEXC(0x20, -1L);
	Super(stack);
	/* Pterm with the vector still set. */
	return lo32(q) == (MARK(0x20) | 0x200) ? 1 : 2;
}

static void part_t(void)
{
	long old = SETEXC(0x20, -1L), r, q;

	r = Pexec(0, CHILD, "\005child", NULL);
	q = SETEXC(0x20, -1L);
	emit("T01 child wrote $080=%08lx, its Setexc(0x20,-1) is-written: Pexec ret=%ld (1 = yes)\r\n",
	     (unsigned long)(MARK(0x20) | 0x200), r);
	emit("T02 after the child's Pterm Setexc(0x20,-1): is-childs=%d is-old=%d\r\n",
	     lo32(q) == (MARK(0x20) | 0x200), lo32(q) == lo32(old));
	SETEXC(0x20, old);
	emit("T03 put back: %d\r\n", lo32(SETEXC(0x20, -1L)) == lo32(old));
}

/* ---- part V ------------------------------------------------------------- */

static volatile uint32_t v_swv, v_reseth, v_prtvec;

static long rd_swv(void)
{
	uint32_t sysbase = CELL(0x4F2);

	v_swv = CELL(0x46E);
	v_reseth = CELL(sysbase + 4);
	v_prtvec = CELL(0x50A);
	return 0;
}

static void part_v(void)
{
	Supexec(rd_swv);
	emit("V01 swv_vec ($46E): is-reseth=%d is-prt_vec(just_rts)=%d is-0=%d\r\n",
	     v_swv == v_reseth, v_swv == v_prtvec, v_swv == 0);
}

int main(int argc, char **argv)
{
	long fh;

	if (argc > 1 && strcmp(argv[1], "child") == 0)
		return child();

	emit("VWPROBE TASK/BIOS16\r\n");
	part_w();
	part_s();
	part_e();
	part_t();
	part_v();
	emit("end\r\n");
	fh = Fcreate("VWPROBE.LOG", 0);
	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}
