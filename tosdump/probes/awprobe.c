/*
 * awprobe.c -- Bconin(1) waits on real Pi 4, a kbdvec hook keeps firing
 * (TASK/BIOS09).  A permanent probe of the tosdump tree for the real-Pi step
 * of the MEM12 check list: run it on the Pi with NO serial adapter attached.
 *
 * The question: since BIOS09 AUX (BIOS device 1) on board-rpi4 is the
 * kernel's serial driver over device 0's input ring, as on qemu-virt since
 * XBIOS15.  Bconin(1) must wait for a byte like EmuTOS' bconin1
 * (bios/serport.c:255-263) -- with no cable, for good -- and while it waits
 * no other process runs, only the waiting process's own kbdvec hook
 * (XBIOS18; EmuTOS calls it from the IKBD interrupt during the wait,
 * bios/aciavecs.S:407-414).  Ctrl+Alt+Del must stay effective: it is decoded
 * behind kbdvec, so the hook chains every byte to the previous vector.
 *
 * NewTOS (AArch64) only.  The oracle's answer for the hook-during-Bconin(1)
 * question is tosdump/probes/axprobe.c (measured equal on tt and falcontc,
 * measurements/measurement-2026-10-29-xbios18/RUNINFO.TXT); this probe is the
 * visible check on the board where nobody can inject a serial byte.  Its hook
 * writes to the screen with a BIOS call, which a NewTOS hook may do (it runs
 * as an EL0 upcall) but an EmuTOS hook, running inside the ACIA interrupt,
 * should not.
 *
 * What it does:
 *   1. hooks kbdvec (Supexec; the slot is Kbdvbase()[-1]) and prints
 *      "AWPROBE:WAIT" once;
 *   2. loops for ever: prints "W<n> Bconstat(1)=.. waiting in Bconin(1)",
 *      calls Bconin(1), prints "R<n> Bconin(1) ret=<low 32 bits> hi=.. after
 *      <t> ticks, hook calls during the wait: <k> (...), total <m>" and
 *      rewrites AWPROBE.LOG in the current directory (if there is a drive).
 *   Every line also goes to the console prefixed "AWPROBE|" (the serial line
 *   with serial=on).
 *   The hook prints " <byte>" (IKBD scancode in hex) for every byte it gets,
 *   then chains to the previous kbdvec with the byte unchanged.
 *
 * Expected on the real Pi without a serial adapter:
 *   - "W1 ..." and then nothing from the main program: Bconin(1) waits.  If
 *     the floating RXD0 line clocked in its spurious 0x00 at boot
 *     (TASK/BIOS09.md, "Empfangspfad auf rpi4"), "W1" says Bconstat(1)=-1,
 *     "R1 Bconin(1) ret=00000000 ..." follows at once and "W2" waits.
 *   - every key press and release adds " 1e 9e" etc. on the screen: the hook
 *     runs while Bconin(1) waits;
 *   - Ctrl+Alt+Del restarts the machine (the hook shows " 1d 38 53" first).
 * With a serial adapter each received byte ends one wait ("R<n> ...").
 *
 * Build: make -C tosdump awprobe-newtos (AWPROBE.NAP,
 * tosdump/build-awprobe-newtos.sh).  QEMU raspi4b run (serial byte injected,
 * keys from a scratch injector kernel): tosdump/probes/run-awprobe.sh.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#if !defined(__aarch64__)
#error "awprobe.c: NewTOS (AArch64) only -- the oracle side is axprobe.c"
#endif

/* Written by the hook (assembly): call count, last byte, _hz_200 of the
 * last call. */
volatile unsigned int aw_n;
volatile unsigned char aw_last;
volatile unsigned int aw_tick;
/* The previous kbdvec, the hook's chain target. */
void *volatile aw_old;

void aw_hook(void);

/*
 * An AAPCS64 function, x0 = the IKBD byte, x1 = the IKBD IOREC
 * (docs/api/xbios/22-kbdvbase.md B2/D2a), run as an EL0 upcall.  Integer
 * registers only (the upcall does not save FP/SIMD).  Prints " hh" through
 * Bconout(2, c) -- BIOS trap: svc #13, function 3 in x8, x0 = device,
 * x1 = character; the svc keeps x1..x29 -- then chains with x0/x1 as they
 * came in.
 */
__asm__(
"	.text\n"
"	.p2align 2\n"
"	.globl	aw_hook\n"
"	.type	aw_hook,%function\n"
"aw_hook:\n"
"	stp	x29, x30, [sp, #-32]!\n"
"	stp	x0, x1, [sp, #16]\n"
"	adrp	x9, aw_n\n"
"	add	x9, x9, :lo12:aw_n\n"
"	ldr	w10, [x9]\n"
"	add	w10, w10, #1\n"
"	str	w10, [x9]\n"
"	adrp	x9, aw_last\n"
"	add	x9, x9, :lo12:aw_last\n"
"	strb	w0, [x9]\n"
"	mov	x12, #0x4ba\n"
"	ldr	w13, [x12]\n"
"	adrp	x9, aw_tick\n"
"	add	x9, x9, :lo12:aw_tick\n"
"	str	w13, [x9]\n"
"	mov	x1, #0x20\n"			/* ' ' */
"	bl	aw_putc\n"
"	ldr	x2, [sp, #16]\n"
"	ubfx	x1, x2, #4, #4\n"
"	bl	aw_puthex\n"
"	ldr	x2, [sp, #16]\n"
"	and	x1, x2, #0xf\n"
"	bl	aw_puthex\n"
"	ldp	x0, x1, [sp, #16]\n"
"	ldp	x29, x30, [sp], #32\n"
"	adrp	x9, aw_old\n"
"	ldr	x9, [x9, :lo12:aw_old]\n"
"	br	x9\n"
"aw_puthex:\n"					/* x1 = nibble */
"	add	x2, x1, #0x30\n"		/* '0' + n */
"	add	x3, x1, #0x57\n"		/* 'a' - 10 + n */
"	cmp	x1, #10\n"
"	csel	x1, x2, x3, lo\n"
"aw_putc:\n"					/* x1 = character */
"	mov	x0, #2\n"
"	mov	x8, #3\n"
"	svc	#13\n"
"	ret\n"
"	.size	aw_hook, .-aw_hook\n"
);

static char out[8192];
static int olen;

static void say(const char *fmt, ...)
{
	char line[200];
	va_list ap;
	int n;

	va_start(ap, fmt);
	n = vsnprintf(line, sizeof(line), fmt, ap);
	va_end(ap);
	if (n < 0)
		return;
	if (n >= (int)sizeof(line))
		n = sizeof(line) - 1;
	Cconws("AWPROBE|");
	Cconws(line);
	Cconws("\r\n");
	if (olen < (int)sizeof(out) - n - 2) {
		memcpy(out + olen, line, n);
		olen += n;
		out[olen++] = '\n';
	} else {
		/* the log is full: start it over (the screen has it all) */
		olen = 0;
	}
}

static void save_log(void)
{
	long fh = Fcreate("AWPROBE.LOG", 0);

	if (fh >= 0) {
		Fwrite((short)fh, (long)olen, out);
		Fclose((short)fh);
	}
}

/* ---- super-state callbacks ---------------------------------------------- */

static void **slot;		/* the kbdvec slot: Kbdvbase()[-1] */

static long hz200(void)
{
	return (long)*(volatile unsigned int *)0x4BAUL;
}

static long install(void)
{
	aw_old = *slot;
	*slot = (void *)aw_hook;
	return 0;
}

static unsigned int ticks(void)
{
	return (unsigned int)Supexec(hz200);
}

int main(void)
{
	unsigned int n, t0, t1, before, after, k;
	long r;

	say("AWPROBE TASK/BIOS09 -- Bconin(1) waits; keys: the kbdvec hook "
	    "prints each byte; Ctrl+Alt+Del ends");
	slot = (void **)Kbdvbase() - 1;
	Supexec(install);
	say("H01 kbdvec held a vector before the hook: %d", aw_old != 0);
	save_log();
	Cconws("AWPROBE:WAIT\r\n");

	for (n = 1;; n++) {
		say("W%u Bconstat(1)=%ld, waiting in Bconin(1) (hook calls so "
		    "far %u)", n, Bconstat(1), aw_n);
		before = aw_n;
		t0 = ticks();
		r = Bconin(1);
		t1 = ticks();
		after = aw_n;
		k = after - before;
		Cconws("\r\n");
		say("R%u Bconin(1) ret=%08lx hi=%08lx after %u ticks, hook calls "
		    "during the wait: %u (the last %d ticks before the return), "
		    "total %u",
		    n, (unsigned long)((unsigned long long)r & 0xFFFFFFFFULL),
		    (unsigned long)((unsigned long long)r >> 32), t1 - t0, k,
		    k ? (int)(t1 - aw_tick) : -1, after);
		save_log();
	}
	return 0;
}
