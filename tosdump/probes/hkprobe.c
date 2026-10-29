/*
 * hkprobe.c -- OS hooks installed by programs (TASK/BIOS15, ADR-031 in
 * docs/MEMORY.md).  A permanent probe of the tosdump tree like gcprobe.c and
 * pxprobe.c: it installs hooks and calls Mfpint/Xbtimer, which DUMPBIOS
 * refuses (tosdump/core/cases.c call_refusals), and writes a text log, no
 * record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * hkprobe-m68k` (HKPROBE.PRG) and `make -C tosdump hkprobe-newtos`
 * (HKPROBE.NAP, tosdump/build-hkprobe-newtos.sh); run by
 * tosdump/probes/run-hkprobe.sh, the oracle side through tools/hatari-oracle.sh
 * profile tt or falcontc.  Output: HKPROBE.LOG in the current directory,
 * written once at the end.
 *
 * Nothing it changes outlives it: every hook is put back, every MFP vector is
 * reinstalled with the value it already holds, timer C is reprogrammed with
 * EmuTOS' own setting (xbtimer(2, 0x50, 192, int_timerc), bios/mfp.c:208).
 *
 * Part M -- the return value of Mfpint and Xbtimer (both void in EmuTOS,
 * bios/mfp.c:114-120, :163-169: d0 is what the code leaves there; the trap
 * glue clears d0 and loads the function number first, bios/vectors.S:1323-1324).
 *   M01 the MFP interrupt mask registers IMRA ($FFFFFA13) and IMRB
 *       ($FFFFFA15), read in a Supexec callback (EmuTOS only: NewTOS has no
 *       MFP, the line says so)
 *   M02 Mfpint(5, timer C vector)       M03 Mfpint(0x15, the same) (num & 15)
 *   M04 Mfpint(12, RS-232 receive vector)
 *   M05 Xbtimer(2, 0x50, 192, timer C vector)
 *   M06 Xbtimer(4, 0x1234, 0, 0)        M07 Xbtimer(-1, 0x8001, 0, 0)
 *       (no timer: bios/mfp.c:165-166 returns at once)
 *   The vectors come from Setexc(0x40 + n, -1): on NewTOS the shadow, 0.
 * Part P -- prt_stat/prt_vec ($506/$50A), EmuTOS just_rts (bios/chardev.c:82-83):
 *   P01 both non-zero and equal          P02 a call through each returns
 * Part H -- hooks NewTOS does not call (the NewTOS log must show one warning
 * each, ADR-031; EmuTOS calls them):
 *   H01 hdv_mediach ($47E) hooked in super state around an Fopen, then put
 *       back: the number of calls
 *   H02 etv_timer hooked with Setexc(0x100) for 40 ticks of _hz_200, then put
 *       back: the number of calls
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

static void ret(const char *id, const char *what, long r)
{
	long long v = (long long)r;

	emit("%s %s ret=%08lx hi=%08lx\r\n", id, what,
	     (unsigned long)(v & 0xFFFFFFFFLL),
	     (unsigned long)((unsigned long long)v >> 32));
}

#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))

/* ---- callbacks ---------------------------------------------------------- */

#ifndef __NEWTOS__
static long imr_word(void)
{
	return (long)(*(volatile uint8_t *)0xFFFFFA13UL) << 8 |
	       *(volatile uint8_t *)0xFFFFFA15UL;
}
#endif

static long hz200(void)
{
	return (long)CELL(0x4BA);
}

static uint32_t prt_stat_v, prt_vec_v;

static long read_prt(void)
{
	prt_stat_v = CELL(0x506);
	prt_vec_v = CELL(0x50A);
	return 0;
}

static long call_prt(void)
{
	((void (*)(void))(uintptr_t)CELL(0x506))();
	((void (*)(void))(uintptr_t)CELL(0x50A))();
	return 0x12345678L;
}

static volatile long mediach_calls, timer_calls;

/* hdv_mediach: "not changed" for every drive (bios/blkdev.c:103's contract). */
static long hk_mediach(void)
{
	mediach_calls++;
	return 0;
}

/* etv_timer: called with the tick length on the stack; nothing to return. */
static long hk_timer(void)
{
	timer_calls++;
	return 0;
}

/* ---- the parts ---------------------------------------------------------- */

static void part_m(void)
{
	long vc = (long)Setexc(0x45, -1L);
	long vr = (long)Setexc(0x4C, -1L);

#ifdef __NEWTOS__
	emit("M01 imra/imrb n/a (no MFP)\r\n");
#else
	{
		long w = Supexec(imr_word);
		emit("M01 imra=%02lx imrb=%02lx\r\n", (w >> 8) & 0xFF, w & 0xFF);
	}
#endif
	ret("M02", "Mfpint(5,timerC)", trap_14_wwl((short)0x0D, (short)5, vc));
	ret("M03", "Mfpint(0x15,timerC)", trap_14_wwl((short)0x0D, (short)0x15, vc));
	ret("M04", "Mfpint(12,rs232rx)", trap_14_wwl((short)0x0D, (short)12, vr));
	ret("M05", "Xbtimer(2,0x50,192,timerC)",
	    trap_14_wwwwl((short)0x1F, (short)2, (short)0x50, (short)192, vc));
	ret("M06", "Xbtimer(4,0x1234,0,0)",
	    trap_14_wwwwl((short)0x1F, (short)4, (short)0x1234, (short)0, 0L));
	ret("M07", "Xbtimer(-1,0x8001,0,0)",
	    trap_14_wwwwl((short)0x1F, (short)-1, (short)0x8001, (short)0, 0L));
}

static void part_p(void)
{
	Supexec(read_prt);
	emit("P01 prt_stat nonzero=%d equal=%d\r\n",
	     prt_stat_v != 0, prt_stat_v == prt_vec_v);
	if (prt_stat_v != 0 && prt_vec_v != 0)
		ret("P02", "call prt_stat+prt_vec", Supexec(call_prt));
	else
		emit("P02 not called (a cell is 0)\r\n");
}

static void part_h(void)
{
	long stack, fh, old, t0, n;
	uint32_t saved;

	stack = Super(0L);
	saved = CELL(0x47E);
	CELL(0x47E) = (uint32_t)(uintptr_t)hk_mediach;
	fh = Fopen("C:\\HKPROBE.NOT", 0);
	if (fh >= 0)
		Fclose((short)fh);
	CELL(0x47E) = saved;
	Super(stack);
	emit("H01 hdv_mediach calls=%ld (Fopen %ld)\r\n", (long)mediach_calls, fh);

	old = (long)Setexc(0x100, (long)hk_timer);
	t0 = Supexec(hz200);
	for (n = 0; n < 2000000L && (unsigned long)(Supexec(hz200) - t0) < 40UL; n++)
		;
	Setexc(0x100, old);
	emit("H02 etv_timer calls=%ld over %lu ticks\r\n", (long)timer_calls,
	     (unsigned long)(Supexec(hz200) - t0));
}

int main(void)
{
	long fh;

	emit("HKPROBE TASK/BIOS15\r\n");
	part_m();
	part_p();
	part_h();
	emit("end\r\n");
	fh = Fcreate("HKPROBE.LOG", 0);
	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}
