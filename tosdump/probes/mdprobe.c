/*
 * mdprobe.c -- BIOS devices 3 (MIDI) and 4 (IKBD) (TASK/BIOS04, audit
 * finding BIOS-CORE-001, docs/audits/2026-10-28-bios-core.md).  A permanent
 * probe of the tosdump tree like hkprobe.c: DUMPBIOS refuses Bconout to
 * MIDI/IKBD and every Bconin that may wait (tosdump/core/cases.c CR_BCONOUT,
 * CR_BCONIN), so these calls have no table case yet (TASK/BIOS13).
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * mdprobe-m68k` (MDPROBE.PRG) and `make -C tosdump mdprobe-newtos`
 * (MDPROBE.NAP, tosdump/build-mdprobe-newtos.sh); run by
 * tosdump/probes/run-mdprobe.sh.  Output: MDPROBE.LOG in the current
 * directory.
 *
 * Part D -- status, output and the IKBD input slot (bios/chardev.c:30-45;
 * Bcostat 3/4 swapped):
 *   D01 Bconstat(3)  D02 Bconstat(4)  D03 Bconin(4) (char_dummy)
 *   D04 Bcostat(3)   D05 Bcostat(4)   (before any output)
 *   D06 Bconout(3,0xFE)  D07 Bconout(4,0x11)  -- the audit probe's B02/B03
 *   D08 Bcostat(3)   D09 Bcostat(4)   (after the output) -- its B11/B12
 *   D10 Bconstat(3)  (the output did not feed the MIDI input)
 * Part R -- the MIDI ring Bconin(3) reads (bios/midi.c:22-62): three bytes
 * 0xFE 0x90 0x3C put into the ring through the record Iorec(2) returns
 * (ibuftl advanced, the byte stored at ibuf[ibuftl], in a Supexec callback),
 * as the MIDI receive interrupt does:
 *   R01 ring size, head == tail before   R02 Bconstat(3) with bytes waiting
 *   R03..R05 Bconin(3) three times (zero-extended: 0xFE is 0x000000FE)
 *   R06 Bconstat(3) after               R07 head == tail after
 * Part W -- Bconin(3) on the empty ring (bios/midi.c:37-38, while
 * (!bconstat3())): the log is written up to the line "W01 before", the
 * console gets "MDPROBE:WAIT", then Bconin(3) is called.  If it returns, the
 * log is written again with "W01 after ret=..." and "Z99 end".  The runner
 * stops the machine a fixed time after "W01 before" is in the log: no "W01
 * after" line = the call was still waiting.
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>
 * (m68k: the sign extension of d0, what (long long)(long) gives).
 */
#include <osbind.h>
#include <mint/ostruct.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>

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

static int write_log(void)
{
	long fh = Fcreate("MDPROBE.LOG", 0);

	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}

/* ---- the MIDI ring, in super state -------------------------------------- */

static _IOREC *midi;
static const unsigned char feed[3] = { 0xFE, 0x90, 0x3C };
static int ring_size, ring_empty;

static long ring_state(void)
{
	ring_size = midi->ibufsiz;
	ring_empty = midi->ibufhd == midi->ibuftl;
	return 0;
}

static long ring_feed(void)
{
	unsigned int i;

	for (i = 0; i < sizeof(feed); i++) {
		short tail = (short)(midi->ibuftl + 1);

		if (tail >= midi->ibufsiz)
			tail = 0;
		midi->ibuf[tail] = (char)feed[i];
		midi->ibuftl = tail;
	}
	return 0;
}

/* ---- the parts ---------------------------------------------------------- */

static void part_d(void)
{
	ret("D01", "Bconstat(3) MIDI", Bconstat(3));
	ret("D02", "Bconstat(4) IKBD", Bconstat(4));
	ret("D03", "Bconin(4) IKBD", Bconin(4));
	ret("D04", "Bcostat(3) before output", Bcostat(3));
	ret("D05", "Bcostat(4) before output", Bcostat(4));
	ret("D06", "Bconout(3,0xFE) MIDI", Bconout(3, 0xFE));
	ret("D07", "Bconout(4,0x11) IKBD resume", Bconout(4, 0x11));
	ret("D08", "Bcostat(3) after Bconout(4)", Bcostat(3));
	ret("D09", "Bcostat(4) after Bconout(3)", Bcostat(4));
	ret("D10", "Bconstat(3) after the output", Bconstat(3));
}

static void part_r(void)
{
	midi = (_IOREC *)Iorec(2);
	Supexec(ring_state);
	emit("R01 Iorec(2) ibufsiz=%d head_eq_tail=%d\r\n", ring_size, ring_empty);
	Supexec(ring_feed);
	ret("R02", "Bconstat(3) 3 bytes in the ring", Bconstat(3));
	ret("R03", "Bconin(3) first", Bconin(3));
	ret("R04", "Bconin(3) second", Bconin(3));
	ret("R05", "Bconin(3) third", Bconin(3));
	ret("R06", "Bconstat(3) ring read", Bconstat(3));
	Supexec(ring_state);
	emit("R07 head_eq_tail=%d\r\n", ring_empty);
}

int main(void)
{
	long r;

	emit("MDPROBE TASK/BIOS04\r\n");
	part_d();
	part_r();
	emit("W01 before Bconin(3) on the empty ring\r\n");
	if (write_log())
		return 1;
	Cconws("MDPROBE:WAIT\r\n");
	r = Bconin(3);
	ret("W01", "after Bconin(3)", r);
	emit("Z99 end\r\n");
	return write_log();
}
