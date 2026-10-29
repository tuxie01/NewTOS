/*
 * sxprobe.c -- Setexc: is the page-0 cell what Setexc answers, and which bits
 * of the vector argument mean "query only" (TASK/BIOS07, decision D3 and
 * sub-point P3 of docs/api/bios/05-setexc.md "Decisions"; the case proposed in
 * docs/analysis/abi-assumptions-check-2026-10-28.md section 1.4).  A permanent
 * probe of the tosdump tree like hkprobe.c: it sets a vector, which DUMPBIOS
 * refuses (tosdump/core/cases.c call policy: Setexc queries only), and writes
 * a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * sxprobe-m68k` (SXPROBE.PRG) and `make -C tosdump sxprobe-newtos`
 * (SXPROBE.NAP, tosdump/build-sxprobe-newtos.sh); run by
 * tosdump/probes/run-sxprobe.sh, the oracle side through
 * tools/hatari-oracle.sh profile tt or falcontc.  Output: SXPROBE.LOG in the
 * current directory, written once at the end.
 *
 * EmuTOS: setexc reads and writes the LONG at 4 * num and tests the LONG
 * argument against -1 (references/emutos/bios/bios.c:1070-1080).
 *
 * Part C -- the cell behind the answer: Setexc(n, -1) against the LONG at
 *   4 * n, read in a Supexec callback, for one vector of each class:
 *   C01 n = 0x02 (bus error)   C02 0x20 (trap #0)   C03 0x100 (etv_timer)
 *   C04 0x101 (etv_critic)     C05 0x102 (etv_term) C06 0x103 (etv_xtra)
 *   C07 0x108 (memvalid)       C08 0x168 (_p_cookies)
 * Part Q -- which bits make a query (P3), on vector 0x20:
 *   Q01 Setexc(0x20, 0x80000001): the old value is a TOS address
 *   Q02 Setexc(0x20, -1L)                 the value just set comes back
 *   Q03 Setexc(0x20, 0xFFFFFFFF as long)  m68k: the same LONG as -1; AArch64:
 *       a long with only the low 32 bits set
 *   Q04 Setexc(0x20, -1L)                 Q03 was a query or a set?
 *   Q05 the LONG at $80, read in super state
 *   Q06 Setexc(0x20, old) puts it back; Setexc(0x20, -1) is old again
 * Nothing it changes outlives it: vector 0x20 is put back.  No trap #0 runs
 * in between (the m68k mintlib calls GEMDOS/BIOS/XBIOS only).
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>
 * (m68k: the sign extension of d0, what (long long)(long) gives).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

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

static unsigned long hi32(long r)
{
	return (unsigned long)((unsigned long long)(long long)r >> 32);
}

static void ret(const char *id, const char *what, long r)
{
	emit("%s %s ret=%08lx hi=%08lx\r\n", id, what, lo32(r), hi32(r));
}

/* An address as a TOS LONG: non-null, even, below 2 GiB. */
static int tosaddr(long v)
{
	return v != 0 && (v & 1) == 0 && lo32(v) < 0x80000000UL && hi32(v) == 0;
}

#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))
#define SETEXC(n, v) trap_13_wwl((short)5, (short)(n), (long)(v))

static volatile uint32_t cell_addr, cell_val;

static long rd_cell(void)
{
	cell_val = CELL(cell_addr);
	return 0;
}

static uint32_t cell_of(int n)
{
	cell_addr = 4UL * (uint32_t)n;
	Supexec(rd_cell);
	return cell_val;
}

/* ---- the parts ---------------------------------------------------------- */

static void part_c(void)
{
	static const int nums[8] = { 0x02, 0x20, 0x100, 0x101, 0x102, 0x103, 0x108, 0x168 };
	int i;

	for (i = 0; i < 8; i++) {
		long q = SETEXC(nums[i], -1L);
		uint32_t c = cell_of(nums[i]);

		emit("C%02d Setexc(0x%03X,-1) is the cell: %d (hi=%08lx)\r\n", i + 1, nums[i],
		     lo32(q) == (unsigned long)c, hi32(q));
	}
}

static void part_q(void)
{
	long old, q;

	old = SETEXC(0x20, (long)0x80000001UL);
	emit("Q01 Setexc(0x20,0x80000001) old is a TOS address: %d\r\n", tosaddr(old));
	ret("Q02", "Setexc(0x20,-1L)", SETEXC(0x20, -1L));
	ret("Q03", "Setexc(0x20,(long)0xFFFFFFFFUL)", SETEXC(0x20, (long)0xFFFFFFFFUL));
	ret("Q04", "Setexc(0x20,-1L)", SETEXC(0x20, -1L));
	emit("Q05 cell $80 in super state ret=%08lx\r\n", (unsigned long)cell_of(0x20));
	SETEXC(0x20, old);
	q = SETEXC(0x20, -1L);
	emit("Q06 put back: %d\r\n", q == old);
}

int main(void)
{
	long fh;

	emit("SXPROBE TASK/BIOS07\r\n");
	part_c();
	part_q();
	emit("end\r\n");
	fh = Fcreate("SXPROBE.LOG", 0);
	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}
