/*
 * ksprobe.c -- Kbshift: the whole shift byte, set by a program and by the
 * keyboard (TASK/BIOS10, finding BIOS-CORE-009 of
 * docs/audits/2026-10-28-bios-core.md; spec docs/api/bios/0b-kbshift.md).
 * A permanent probe of the tosdump tree like sxprobe.c: it uses the set arm of
 * Kbshift, which DUMPBIOS refuses (tosdump/core/cases.c call policy: Kbshift
 * queries only), and real key strokes, which no case table can give; it writes
 * a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * ksprobe-m68k` (KSPROBE.PRG) and `make -C tosdump ksprobe-newtos`
 * (KSPROBE.NAP, tosdump/build-ksprobe-newtos.sh); run by
 * tosdump/probes/run-ksprobe.sh, which also injects the key strokes of
 * tosdump/probes/ksprobe-keys.txt.  Output: KSPROBE.LOG in the current
 * directory; part K is written before part H starts, part H at the end.
 *
 * EmuTOS: kbshift stores the WORD flag into the UBYTE shifty when flag >= 0 and
 * returns the old value (references/emutos/bios/ikbd.c:145-153); kbd_int sets
 * MODE_HOME (0x20) / MODE_INSERT (0x40) on a Clr/Home / Insert press while Alt
 * is down (:876-883) and clears them on that key's release (:807-814); a
 * modifier key changes only its own bit (:778-801, :825-853).
 *
 * Part K -- the set arm, the lines K01-K13 of the audit probe bcprobe.c
 *   (measurements/measurement-2026-10-28-bios-core-audit/method/bcprobe.c),
 *   same calls, same format.
 * Part H -- the key path.  conterm ($484, super state) gets bit 3 (Bconin
 *   returns the shift byte of the key press in bits 24-31) and loses bit 1 (no
 *   key repeat).  The probe creates KSPROBE.RDY and then polls: about every
 *   5 ms (a busy loop calibrated against _hz_200, so the trap count stays
 *   small; I00 prints the calibration, side-specific, not compared)
 *   Bconstat(2), a Bconin(2) when it reports a key, and Kbshift(-1); every
 *   change of the shift byte is one line
 *     Tnn now=<Kbshift(-1)>
 *   No time is printed: the two sides differ in timing, not in order.  The
 *   keys (ksprobe-keys.txt, 100 ms apart, each key held less than the repeat
 *   delay; EmuTOS expectation in brackets):
 *     A Alt+Home:            Alt, Home, Home up, Alt up    [08 28 08 00]
 *     B Alt+Insert:          Alt, Ins, Ins up, Alt up      [08 48 08 00]
 *     C Home without Alt:    Home, Home up                 [no change]
 *     D Alt released first:  Alt, Home, Alt up, Home up    [08 28 20 00]
 *     E both:                Alt, Ins, Home, Home up, Ins up, Alt up
 *                                                          [08 48 68 48 08 00]
 *   A pause of 5 s follows.  After 1 s without a change the probe calls
 *   Kbshift(0x80) (H01, read back H02), then
 *     F a program-set bit 7: LShift, a, a up, LShift up, Alt, Home, Home up,
 *       Alt up                                             [82 80 88 a8 88 80]
 *   After the next second without a change Kbshift(old) restores the byte
 *   (H03), and the probe prints every key it read, in order:
 *     Qnn sh=<entry bits 24-31> sc=<bits 16-23>   and   Ann asc=<bits 0-7>
 *   (the ASCII byte on its own line: it depends on the keyboard table, not on
 *   the shift byte).  Reading the keys as they come keeps the queue empty: on
 *   NewTOS Bconstat(2) pumps the input only then (kernel/src/host_env.rs
 *   fill_from_device), and nothing else in the loop does.  EmuTOS queues
 *   Home of C and Shift+a of F only: Alt+Home and Alt+Insert are its keyboard
 *   mouse emulation (right / left button, bios/ikbd.c:93-99, 295-387,
 *   904-908), which queues no key.  conterm is put back at the end.
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>
 * (m68k: the sign extension of d0, what (long long)(long) gives).
 */
#include <osbind.h>
#include <stdarg.h>
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

static void flush_log(int append)
{
	long fh;

	if (append) {
		fh = Fopen("KSPROBE.LOG", 1);
		if (fh >= 0)
			Fseek(0L, (short)fh, 2);
	} else {
		fh = Fcreate("KSPROBE.LOG", 0);
	}
	if (fh < 0)
		return;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	olen = 0;
}

static unsigned long lo32(long r)
{
	return (unsigned long)r & 0xFFFFFFFFUL;
}

static unsigned long hi32(long r)
{
	if (sizeof(long) > 4)
		return ((unsigned long)r >> 16 >> 16) & 0xFFFFFFFFUL;
	return r < 0 ? 0xFFFFFFFFUL : 0UL;
}

static void line(const char *id, const char *call, long r)
{
	emit("%s %s ret=%08lx hi=%08lx\n", id, call, lo32(r), hi32(r));
}

#define KBSHIFT(f)   trap_13_ww((short)11, (short)(f))
#define BCONSTAT(d)  trap_13_ww((short)1, (short)(d))
#define BCONIN(d)    trap_13_ww((short)2, (short)(d))

/* ---- Supexec callbacks: system variables in super state ---------------- */
static volatile long sv_hz;
static volatile int sv_conterm;

static long rd_hz(void)
{
	sv_hz = *(volatile long *)0x4BA;	/* _hz_200 */
	return 0;
}

static long ct_save(void)
{
	sv_conterm = *(volatile unsigned char *)0x484;
	return 0;
}

static long ct_keys(void)
{
	/* bit 3: shift byte in Bconin; bit 1 off: no key repeat */
	*(volatile unsigned char *)0x484 = (unsigned char)((sv_conterm | 8) & ~2);
	return 0;
}

static long ct_restore(void)
{
	*(volatile unsigned char *)0x484 = (unsigned char)sv_conterm;
	return 0;
}

static long hz200(void)
{
	Supexec(rd_hz);
	return sv_hz;
}

static void part_k(void)
{
	long s0 = KBSHIFT(-1);

	line("K01", "Kbshift(-1)", s0);
	line("K02", "Kbshift(0x10)", KBSHIFT(0x10));
	line("K03", "Kbshift(-1)", KBSHIFT(-1));
	line("K04", "Kbshift(0x1FF)", KBSHIFT(0x1FF));
	line("K05", "Kbshift(-1)", KBSHIFT(-1));
	line("K06", "Kbshift(-2)", KBSHIFT(-2));
	line("K07", "Kbshift(-1)", KBSHIFT(-1));
	line("K08", "Kbshift(0x7F81)", KBSHIFT(0x7F81));
	line("K09", "Kbshift(-1)", KBSHIFT(-1));
	line("K10", "Kbshift(-32768)", KBSHIFT(-32768));
	line("K11", "Kbshift(-1)", KBSHIFT(-1));
	line("K12", "Kbshift(s0) restore", KBSHIFT(s0 & 0xFFFF));
	line("K13", "Kbshift(-1)", KBSHIFT(-1));
}

static volatile long spin_sink;

static void spin(long n)
{
	long i;

	for (i = 0; i < n; i++)
		spin_sink++;
}

/*
 * Busy-loop iterations per 5 ms tick, measured against _hz_200: the largest
 * of three measurements of at least 10 ticks (a tick that jumps -- an emulator
 * catching up -- only makes one measurement too small).  It paces the polls
 * and so bounds the trap count; the probe's timing decisions read _hz_200.
 */
static long calibrate(void)
{
	long n = 1000, t0, dt, best = 1, k;
	int i;

	for (i = 0; i < 3; i++) {
		do {
			n *= 2;
			t0 = hz200();
			spin(n);
			dt = hz200() - t0;
		} while (dt < 10 && n < 0x10000000L);
		k = dt > 0 ? n / dt : n;
		if (k > best)
			best = k;
		n /= 4;
	}
	return best;
}

static int nt;
static unsigned long keys[40];
static int nkeys;

/* Read one queued key into keys[] (printed as Q/A lines at the end). */
static void take_key(void)
{
	unsigned long e = lo32(BCONIN(2));

	if (nkeys < (int)(sizeof(keys) / sizeof(keys[0])))
		keys[nkeys++] = e;
}

/*
 * Poll until the shift byte has stayed the same for `quiet` ticks after at
 * least one change, or for `first` ticks before the first; every change is a
 * T line.  Returns the number of changes.
 */
static int follow(long per_tick, long *prev, long quiet, long first)
{
	long v, now, t_last = hz200();
	int changes = 0;

	for (;;) {
		spin(per_tick);
		if (BCONSTAT(2) != 0)
			take_key();
		v = lo32(KBSHIFT(-1));
		now = hz200();
		if (v != *prev) {
			emit("T%02d now=%02lx\n", ++nt, v);
			*prev = v;
			changes++;
			t_last = now;
		} else if (now - t_last >= (changes ? quiet : first)) {
			return changes;
		}
	}
}

static void part_h(void)
{
	long fh, s0, prev, per_tick;
	int n;

	Supexec(ct_save);
	Supexec(ct_keys);
	s0 = KBSHIFT(-1);
	per_tick = calibrate();
	emit("H00 conterm=%02x Kbshift(-1)=%02lx\n", sv_conterm, lo32(s0));
	emit("I00 per_tick=%ld\n", per_tick);
	prev = lo32(s0);
	fh = Fcreate("KSPROBE.RDY", 0);
	if (fh >= 0)
		Fclose((short)fh);
	if (follow(per_tick, &prev, 200, 3000) == 0) {
		emit("H99 no change within 15 s\n");
		Supexec(ct_restore);
		return;
	}
	line("H01", "Kbshift(0x80)", KBSHIFT(0x80));
	prev = KBSHIFT(-1);
	line("H02", "Kbshift(-1)", prev);
	prev = lo32(prev);
	if (follow(per_tick, &prev, 200, 3000) == 0)
		emit("H98 no change within 15 s after H01\n");
	line("H03", "Kbshift(s0) restore", KBSHIFT(s0 & 0xFFFF));
	while (BCONSTAT(2) != 0 && nkeys < 40)
		take_key();
	for (n = 0; n < nkeys; n++) {
		emit("Q%02d sh=%02lx sc=%02lx\n", n + 1, (keys[n] >> 24) & 0xFF, (keys[n] >> 16) & 0xFF);
		emit("A%02d asc=%02lx\n", n + 1, keys[n] & 0xFF);
	}
	emit("H04 queued keys=%d\n", nkeys);
	Supexec(ct_restore);
}

int main(void)
{
	part_k();
	flush_log(0);
	part_h();
	emit("Z99 end\n");
	flush_log(1);
	Cconws("KSPROBE done\r\n");
	return 0;
}
