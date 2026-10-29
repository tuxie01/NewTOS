/*
 * axprobe.c -- a kbdvec hook while the program waits in Bconin(1) (TASK/XBIOS18).
 * A permanent probe of the tosdump tree like hkprobe.c: it waits in
 * Bconin(1) (DUMPBIOS refuses blocking reads), so it is a program of its own.
 *
 * The question: a program has hooked the IKBD keyboard vector kbdvec
 * (docs/api/xbios/22-kbdvbase.md) and waits in Bconin(1) for a byte on the
 * serial line. A key is pressed meanwhile. Is the hook called while the
 * program waits, or only after the serial byte has ended the wait?
 * EmuTOS: bconin1 spins in bconin_iorec (bios/serport.c:202-210, :260-263)
 * with interrupts open; the IKBD interrupt jumps through kbdvec at once
 * (ikbdraw, bios/aciavecs.S:407-414).
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * axprobe-m68k` (AXPROBE.PRG) and `make -C tosdump axprobe-newtos`
 * (AXPROBE.NAP, tosdump/build-axprobe-newtos.sh); run by
 * tosdump/probes/run-axprobe.sh, which injects the input:
 *   1. the probe prints "AXPROBE:WAIT" (and creates C:\AXPROBE.RDY where it
 *      has a drive), then calls Bconin(1);
 *   2. 0.3 s later the harness presses and releases the key 'a' (ST scancode
 *      0x1E: make 1E, break 9E);
 *   3. 2 s after the key the harness sends the byte 'Q' (0x51) on the serial
 *      line, which ends the wait.
 *
 * The hook (assembly, below) records each byte and _hz_200 at the moment it
 * runs, then chains to the previous kbdvec with the byte unchanged, so the key
 * also reaches the BIOS key queue (X06).  It is installed in super state
 * (Supexec) and put back before the probe ends.
 *
 * Output lines (compared between the sides):
 *   H01 the kbdvec slot held a vector (non-zero) before the hook went in
 *   B01 Bconstat(1), Bconstat(2) before the wait (nothing queued)
 *   X01 Bconin(1)'s return value
 *   X02 hook calls in total, until 1 s after Bconin(1) returned
 *   X03 hook calls DURING the wait: _hz_200 at the call is at least 20 ticks
 *       (0.1 s) before Bconin(1) returned
 *   X04 the bytes the hook saw, in order
 *   X05 whether Bconin(1) waited at least 1 s (it must: the serial byte comes
 *       2.3 s after the marker)
 *   X06 Bconstat(2), then Bconin(2) if a key is queued (the chained key)
 * Lines starting with "I" carry the raw tick distances, informational only
 * (emulator timing, not compared).
 *
 * Output: AXPROBE.LOG in the current directory (if there is a drive) and the
 * whole log on the console, every line prefixed "AXPROBE|" (qemu-virt has no
 * drive: its serial line is the only channel), between the markers
 * "AXPROBE:BEGIN" and "AXPROBE:END".
 *
 * ret=<low 32 bits> hi=<high 32 bits of the native return register>.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define AX_MAX 16

/* Written by the hook (assembly): call count, and per call the byte and
 * _hz_200.  32-bit and byte cells on both CPUs. */
volatile unsigned int ax_n;
volatile unsigned char ax_byte[AX_MAX];
volatile unsigned int ax_tick[AX_MAX];
/* The previous kbdvec, the hook's chain target. */
void *volatile ax_old;

void ax_hook(void);

#if defined(__aarch64__)
/*
 * NewTOS: an AAPCS64 function, x0 = the IKBD byte, x1 = the IKBD IOREC
 * (22-kbdvbase.md B2/D2a), run as an EL0 upcall.  It may change any register
 * (the kernel saves the interrupted context) but no FP/SIMD register (the
 * upcall does not save those): integer registers only.  _hz_200 is a plain
 * 32-bit load of the per-process sysvar page (sdk/libc/newtos-arch/
 * getsysvar.c).  Chaining is a tail call with x0/x1 unchanged.
 */
__asm__(
"	.text\n"
"	.p2align 2\n"
"	.globl	ax_hook\n"
"	.type	ax_hook,%function\n"
"ax_hook:\n"
"	adrp	x9, ax_n\n"
"	add	x9, x9, :lo12:ax_n\n"
"	ldr	w10, [x9]\n"
"	cmp	w10, #16\n"			/* AX_MAX */
"	b.hs	1f\n"
"	adrp	x11, ax_byte\n"
"	add	x11, x11, :lo12:ax_byte\n"
"	strb	w0, [x11, x10]\n"
"	mov	x12, #0x4ba\n"
"	ldr	w13, [x12]\n"
"	adrp	x11, ax_tick\n"
"	add	x11, x11, :lo12:ax_tick\n"
"	str	w13, [x11, x10, lsl #2]\n"
"1:	add	w10, w10, #1\n"
"	str	w10, [x9]\n"
"	adrp	x9, ax_old\n"
"	ldr	x9, [x9, :lo12:ax_old]\n"
"	br	x9\n"
"	.size	ax_hook, .-ax_hook\n"
);
#elif defined(__m68k__)
/*
 * EmuTOS: ikbdraw jumps here from the ACIA interrupt with d0.b = the byte and
 * a0 = the IKBD IOREC (aciavecs.S:407-414).  Every register preserved; chain
 * by pushing the old vector and returning into it, d0/a0 untouched (the
 * chaining of sdk/apps/doom/doomgeneric_gem_kbdvec.c).  a.out symbols carry
 * the `_` prefix.
 */
__asm__(
"	.text\n"
"	.even\n"
"	.globl	_ax_hook\n"
"_ax_hook:\n"
"	movem.l	%d1/%a1,-(%sp)\n"
"	move.l	_ax_n,%d1\n"
"	cmp.l	#16,%d1\n"			/* AX_MAX */
"	bcc.s	1f\n"
"	lea	_ax_byte,%a1\n"
"	move.b	%d0,0(%a1,%d1.w)\n"
"	lea	_ax_tick,%a1\n"
"	add.w	%d1,%d1\n"
"	add.w	%d1,%d1\n"
"	move.l	0x4ba,0(%a1,%d1.w)\n"
"	move.l	_ax_n,%d1\n"
"1:	addq.l	#1,%d1\n"
"	move.l	%d1,_ax_n\n"
"	movem.l	(%sp)+,%d1/%a1\n"
"	move.l	_ax_old,-(%sp)\n"
"	rts\n"
);
#else
#error "axprobe.c: no kbdvec hook for this CPU"
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

static void ret(const char *id, const char *what, long r)
{
	long long v = (long long)r;

	emit("%s %s ret=%08lx hi=%08lx\n", id, what,
	     (unsigned long)(v & 0xFFFFFFFFLL),
	     (unsigned long)((unsigned long long)v >> 32));
}

/* ---- super-state callbacks ---------------------------------------------- */

static void **slot;		/* the kbdvec slot: Kbdvbase()[-1] */
static int restored;

static long hz200(void)
{
	return (long)*(volatile unsigned int *)0x4BAUL;
}

static long install(void)
{
	ax_old = *slot;
	*slot = (void *)ax_hook;
	return 0;
}

static long uninstall(void)
{
	restored = *slot == (void *)ax_hook;
	if (restored)
		*slot = ax_old;
	return 0;
}

static unsigned int ticks(void)
{
	return (unsigned int)Supexec(hz200);
}

/* ---- the probe ---------------------------------------------------------- */

static void print_log(void)
{
	char lbuf[200];
	int i = 0;

	Cconws("AXPROBE:BEGIN\r\n");
	while (i < olen) {
		int j = 0;

		while (i < olen && out[i] != '\n' && j < (int)sizeof(lbuf) - 4)
			lbuf[j++] = out[i++];
		if (i < olen && out[i] == '\n')
			i++;
		lbuf[j] = 0;
		Cconws("AXPROBE|");
		Cconws(lbuf);
		Cconws("\r\n");
	}
	Cconws("AXPROBE:END\r\n");
}

int main(void)
{
	long kv, fh, r, st;
	unsigned int t0, t1, n, k, i;

	emit("AXPROBE TASK/XBIOS18\n");
	kv = (long)Kbdvbase();
	slot = (void **)kv - 1;
	Supexec(install);
	emit("H01 kbdvec held a vector before the hook: %d\n", ax_old != 0);
	ret("B01", "Bconstat(1)", Bconstat(1));
	ret("B02", "Bconstat(2)", Bconstat(2));

	/* the ready marker: the harness injects only after it has seen it */
	fh = Fcreate("AXPROBE.RDY", 0);
	if (fh >= 0)
		Fclose((short)fh);
	Cconws("AXPROBE:WAIT\r\n");

	t0 = ticks();
	r = Bconin(1);
	t1 = ticks();
	/* one more second: hook calls that come only after the wait */
	while (ticks() - t1 < 200U)
		Vsync();
	Supexec(uninstall);

	n = ax_n;
	k = 0;
	for (i = 0; i < n && i < AX_MAX; i++)
		if ((int)(t1 - ax_tick[i]) >= 20)
			k++;
	ret("X01", "Bconin(1)", r);
	emit("X02 hook calls in total: %u\n", n);
	emit("X03 hook calls during the wait (>= 20 ticks before Bconin(1) returned): %u\n", k);
	emit("X04 hook bytes:");
	for (i = 0; i < n && i < AX_MAX; i++)
		emit(" %02x", (unsigned)ax_byte[i]);
	emit("\n");
	emit("X05 Bconin(1) waited >= 200 ticks: %d\n", t1 - t0 >= 200U);
	st = Bconstat(2);
	ret("X06", "Bconstat(2)", st);
	if (st != 0)
		ret("X07", "Bconin(2)", Bconin(2));
	else
		emit("X07 Bconin(2) not called (nothing queued)\n");
	emit("X08 hook put back: %d\n", restored);
	emit("I01 Bconin(1) waited %u ticks\n", t1 - t0);
	for (i = 0; i < n && i < AX_MAX; i++)
		emit("I02 hook call %u byte %02x at Bconin(1) return - %d ticks\n", i,
		     (unsigned)ax_byte[i], (int)(t1 - ax_tick[i]));
	emit("Z99 end\n");

	fh = Fcreate("AXPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)olen, out);
		Fclose((short)fh);
	}
	print_log();
	return 0;
}
