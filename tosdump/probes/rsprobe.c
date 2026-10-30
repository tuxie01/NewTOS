/*
 * rsprobe.c -- XBIOS Rsconf (0x0F) and the device-0 record Iorec(0)
 * (TASK/XBIOS06, audit finding XBIOS-CHARDEV-006,
 * docs/audits/2026-10-28-xbios-chardev.md; case design P00-P17, S19-S21 in
 * docs/analysis/xbios06-rsconf-profile-2026-10-30.md section 3.2).  A
 * permanent probe of the tosdump tree like mdprobe.c: DUMPBIOS refuses every
 * Rsconf with a baud rate set (tosdump/core/cases.c CR_RSCONF), so the set
 * arms have no table case yet (TASK/TOOL27).
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * rsprobe-m68k` (RSPROBE.PRG) and `make -C tosdump rsprobe-newtos`
 * (RSPROBE.NAP, tosdump/build-rsprobe-newtos.sh); run by
 * tosdump/probes/run-rsprobe.sh.  Output: RSPROBE.LOG in the current
 * directory, written once at the end; the probe terminates.
 *
 * Every step line (P.., S..) is
 *   <id> <call> ret=<lo> hi=<hi> q=<lo> qhi=<hi> g=<lo> ghi=<hi>
 *        ib=.. if=.. iu=.. idm=.. iw=..
 *   ret/hi   the step's own call (64-bit native return register split in two
 *            32-bit halves; m68k: the sign extension of d0, what
 *            (long long)(long) gives)
 *   q/qhi    Rsconf(-2,-1,-1,-1,-1,-1) right after (the baud query)
 *   g/ghi    Rsconf(-1,-1,-1,-1,-1,-1) after that (the register read-back;
 *            changes nothing on either side)
 *   ib..iw   the five UBYTEs behind the two IORECs of Iorec(0)'s record
 *            (EXT_IOREC: baudrate, flowctrl, ucr, datamask, wr5), read in
 *            user mode at the offsets of the tosdump-layout block in
 *            docs/api/structures/iorec.md (m68k 28..32, LP64 48..52; wr5 is
 *            class IGNORE there, printed anyway)
 *
 * Order (S before P01 so the TSR lines start from the power-up registers):
 *   L00      layout self-check: the fixed offset base against 2*sizeof(_IOREC)
 *   P00      initial state, the step call is Rsconf(-1 x6)
 *   S19-S21  tsr = 0x00 / 0x08 / 0xFF, each followed by S..r restoring the
 *            initial tsr (the TSR byte of P00's return)
 *   P01-P04  baud 0, 1, 4, 15          P05-P06 baud 16, -3 (out of range)
 *   P07-P10  flow 0..3                 P11a/b  flow 4, -2 (out of range)
 *   P12      baud 4, flow 2, ucr 0x98 in one call
 *   P13-P15  ucr 0x28 / 0x48 / 0x68 (MFP word-length bits 6-5 = 01/10/11,
 *            the SCC handler's 7/6/5 bits, bios/serport.c:813-829)
 *   P16      the record's baudrate set to 7 by the program (Supexec), then the
 *            step call Rsconf(-2,..): does the answer come from the record?
 *   P17      restore: Rsconf(baud0, flow0, ucr0, rsr0, tsr0, -1) with baud0
 *            from P00's query, flow0 from P00's record, the register bytes
 *            from P00's return
 *   P18      the record's baudrate/flowctrl after P17 compared with P00's;
 *            where they differ the program writes P00's bytes back
 *            (Supexec) -- needed=1 says it had to
 *   Z01      final state equal to P00 (query, read-back, record bytes)
 */
#include <osbind.h>
#include <mint/ostruct.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>

static char out[8192];
static int olen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 256)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

static unsigned long lo32(long r)
{
	return (unsigned long)((unsigned long long)(long long)r & 0xFFFFFFFFULL);
}

static unsigned long hi32(long r)
{
	return (unsigned long)((unsigned long long)(long long)r >> 32);
}

static int write_log(void)
{
	long fh = Fcreate("RSPROBE.LOG", 0);

	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}

/* ---- device 0's record --------------------------------------------------- */

/* Offset of EXT_IOREC.baudrate: docs/api/structures/iorec.md, the
 * tosdump-layout line "extiorec" (baudrate:b@28/48). */
#define REC_BASE ((int)(sizeof(void *) > 4 ? 48 : 28))

static volatile unsigned char *rec;
static unsigned char rec0[5];		/* P00's five bytes */
static volatile int poke_off;
static volatile unsigned char poke_val;

static long rec_poke(void)
{
	rec[REC_BASE + poke_off] = poke_val;
	return 0;
}

static void poke(int off, unsigned char v)
{
	poke_off = off;
	poke_val = v;
	Supexec(rec_poke);
}

static void rec_read(unsigned char b[5])
{
	int i;

	for (i = 0; i < 5; i++)
		b[i] = rec[REC_BASE + i];
}

#define RS(a, b, c, d, e) Rsconf((a), (b), (c), (d), (e), -1)

static long last_q, last_g;
static unsigned char last_rec[5];

/* One step: the call's return, then the baud query, the register read-back
 * and the record bytes. */
static void step(const char *id, const char *call, long r)
{
	last_q = RS(-2, -1, -1, -1, -1);
	last_g = RS(-1, -1, -1, -1, -1);
	rec_read(last_rec);
	emit("%s %s ret=%08lx hi=%08lx q=%08lx qhi=%08lx g=%08lx ghi=%08lx"
	     " ib=%02x if=%02x iu=%02x idm=%02x iw=%02x\r\n",
	     id, call, lo32(r), hi32(r), lo32(last_q), hi32(last_q),
	     lo32(last_g), hi32(last_g), last_rec[0], last_rec[1],
	     last_rec[2], last_rec[3], last_rec[4]);
}

int main(void)
{
	long g0, b0;
	short ucr0, rsr0, tsr0, flow0;
	int needed, i, same;

	emit("RSPROBE TASK/XBIOS06\r\n");
	emit("L00 sizeof(long)=%d sizeof(_IOREC)=%d base=%d two_iorec=%d ok=%d\r\n",
	     (int)sizeof(long), (int)sizeof(_IOREC), REC_BASE,
	     (int)(2 * sizeof(_IOREC)), REC_BASE == (int)(2 * sizeof(_IOREC)));

	rec = (volatile unsigned char *)Iorec(0);

	g0 = RS(-1, -1, -1, -1, -1);
	step("P00", "Rsconf(-1,-1,-1,-1,-1,-1) initial", g0);
	b0 = last_q;
	for (i = 0; i < 5; i++)
		rec0[i] = last_rec[i];
	ucr0 = (short)((g0 >> 24) & 0xFF);
	rsr0 = (short)((g0 >> 16) & 0xFF);
	tsr0 = (short)((g0 >> 8) & 0xFF);
	flow0 = (short)rec0[1];

	/* S: the TSR byte written and read back */
	step("S19", "Rsconf(-1,-1,-1,-1,0x00,-1) tsr", RS(-1, -1, -1, -1, 0x00));
	step("S19r", "Rsconf(-1,-1,-1,-1,tsr0,-1) restore", RS(-1, -1, -1, -1, tsr0));
	step("S20", "Rsconf(-1,-1,-1,-1,0x08,-1) tsr", RS(-1, -1, -1, -1, 0x08));
	step("S20r", "Rsconf(-1,-1,-1,-1,tsr0,-1) restore", RS(-1, -1, -1, -1, tsr0));
	step("S21", "Rsconf(-1,-1,-1,-1,0xFF,-1) tsr", RS(-1, -1, -1, -1, 0xFF));
	step("S21r", "Rsconf(-1,-1,-1,-1,tsr0,-1) restore", RS(-1, -1, -1, -1, tsr0));

	/* P: baud */
	step("P01", "Rsconf(0,-1,-1,-1,-1,-1) baud", RS(0, -1, -1, -1, -1));
	step("P02", "Rsconf(1,-1,-1,-1,-1,-1) baud", RS(1, -1, -1, -1, -1));
	step("P03", "Rsconf(4,-1,-1,-1,-1,-1) baud", RS(4, -1, -1, -1, -1));
	step("P04", "Rsconf(15,-1,-1,-1,-1,-1) baud", RS(15, -1, -1, -1, -1));
	step("P05", "Rsconf(16,-1,-1,-1,-1,-1) baud out", RS(16, -1, -1, -1, -1));
	step("P06", "Rsconf(-3,-1,-1,-1,-1,-1) baud out", RS(-3, -1, -1, -1, -1));
	/* P: flow */
	step("P07", "Rsconf(-1,0,-1,-1,-1,-1) flow", RS(-1, 0, -1, -1, -1));
	step("P08", "Rsconf(-1,1,-1,-1,-1,-1) flow", RS(-1, 1, -1, -1, -1));
	step("P09", "Rsconf(-1,2,-1,-1,-1,-1) flow", RS(-1, 2, -1, -1, -1));
	step("P10", "Rsconf(-1,3,-1,-1,-1,-1) flow", RS(-1, 3, -1, -1, -1));
	step("P11a", "Rsconf(-1,4,-1,-1,-1,-1) flow out", RS(-1, 4, -1, -1, -1));
	step("P11b", "Rsconf(-1,-2,-1,-1,-1,-1) flow out", RS(-1, -2, -1, -1, -1));
	/* P: both in one call, then ucr */
	step("P12", "Rsconf(4,2,0x98,-1,-1,-1)", RS(4, 2, 0x98, -1, -1));
	step("P13", "Rsconf(-1,-1,0x28,-1,-1,-1) ucr", RS(-1, -1, 0x28, -1, -1));
	step("P14", "Rsconf(-1,-1,0x48,-1,-1,-1) ucr", RS(-1, -1, 0x48, -1, -1));
	step("P15", "Rsconf(-1,-1,0x68,-1,-1,-1) ucr", RS(-1, -1, 0x68, -1, -1));
	/* P16: the record's baudrate written by the program, then the query */
	poke(0, 7);
	step("P16", "record baudrate:=7; Rsconf(-2,-1,-1,-1,-1,-1)",
	     RS(-2, -1, -1, -1, -1));
	/* P17: restore */
	step("P17", "Rsconf(baud0,flow0,ucr0,rsr0,tsr0,-1) restore",
	     RS((short)b0, flow0, ucr0, rsr0, tsr0));
	emit("P17 args baud0=%d flow0=%d ucr0=%02x rsr0=%02x tsr0=%02x\r\n",
	     (int)(short)b0, (int)flow0, (unsigned)ucr0, (unsigned)rsr0,
	     (unsigned)tsr0);
	/* P18: the record bytes Rsconf owns back to P00's */
	needed = 0;
	for (i = 0; i < 2; i++) {
		if (last_rec[i] != rec0[i]) {
			poke(i, rec0[i]);
			needed = 1;
		}
	}
	emit("P18 record restore needed=%d\r\n", needed);
	step("Z01", "Rsconf(-1,-1,-1,-1,-1,-1) final", RS(-1, -1, -1, -1, -1));
	same = last_q == b0 && last_g == g0;
	for (i = 0; i < 5; i++)
		same = same && last_rec[i] == rec0[i];
	emit("Z02 final equals initial=%d\r\n", same);
	emit("Z99 end\r\n");
	return write_log();
}
