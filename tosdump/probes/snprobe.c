/*
 * snprobe.c -- the XBIOS DMA sound group 0x80-0x8D on the oracle (TASK/SND04,
 * cases O1-O5 of TASK/SND01.md "Spec review", semantics S1-S7 of
 * docs/api/xbios/_dmasound-group.md section 9.1.2).  A permanent probe of the
 * tosdump tree like hkprobe.c: DUMPBIOS refuses every set arm of the group
 * (tosdump/core/cases.c, call_policy: only the Soundcmd/Buffoper/Sndstatus/
 * Buffptr inquiries), and O2/O3/O5 are timing (end of frame, pointer
 * progress, interrupts), which no case table expresses.  It writes a text
 * log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), the
 * mintlib bindings (<mint/falcon.h>, <osbind.h>) as a ported program calls
 * them.  Built by `make -C tosdump snprobe-m68k` (SNPROBE.PRG) and
 * `make -C tosdump snprobe-newtos` (SNPROBE.NAP,
 * tosdump/build-snprobe-newtos.sh); run by tosdump/probes/run-snprobe.sh.
 * Output: SNPROBE.LOG in the current directory, written once at the end.
 *
 * THE ORACLE IS HATARI'S DMA SOUND MODEL, NOT SILICON.  What the timing parts
 * measure is EmuTOS' code driving Hatari 2.6.1's crossbar / STE DMA sound
 * emulation; a real Falcon or TT may differ.
 *
 * Gate: Locksnd() answering its function number 0x80 means "no DMA sound"
 * (EmuTOS SOUND_IS_AVAILABLE, references/emutos/bios/dmasound.c:122-126).
 * Then only part A runs -- every call of the group once, recording what it
 * answers -- and B-F say "skipped".  NewTOS today is that case.
 *
 * Part A (O1) -- returns and the boot state through the inquiry forms.
 * Part B (O2, O3; S3, S4) -- one single-shot frame: Buffoper(-1) bit 0 and
 *   Buffptr play over time; when the bit drops, the pointer's range,
 *   granularity and value after the end; res1/res2/record untouched.
 * Part C (O3; S4) -- loop mode over about 3.5 frame times: wraps, then stop.
 * Part D (S5) -- Setbuffer while a loop frame plays: when the pointer moves.
 * Part E (O5; S2, S6) -- Setinterrupt(0,1) + Xbtimer(0,8,1,h) + Jenabint(13)
 *   with a counting m68k handler: single shot, loop, and Setinterrupt(0,0) as
 *   control; then Setinterrupt(1,1) with counting handlers on MFP channels 15
 *   (GPIP7, vector 0x13C) and 7 (GPIP5, vector 0x11C).
 * Part F (O4) -- Sndstatus(1) while a loop frame plays, after setting the
 *   attenuation (EmuTOS zeroes it itself, bios/dmasound.c:761) and ADDERIN
 *   (zeroed only by the codec toggle the source comment describes,
 *   :735-750): Buffoper(-1), the interrupt/control bytes $FFFF8900/01 (read
 *   in a Supexec callback, oracle only), the pointer, Soundcmd inquiries;
 *   then EmuTOS' init sequence
 *   (bios/dmasound.c:182-198) is replayed to leave the machine as found.
 *
 * Times are 200 Hz ticks (_hz_200, $4BA, read in a Supexec callback) since
 * the call that started the frame.  ret=<low 32 bits of the return>.
 */
#include <osbind.h>
#include <mint/falcon.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static char out[16384];
static int olen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 200)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

static void ret(const char *id, const char *what, long r)
{
	emit("%s %s ret=%08lx (%ld)\r\n", id, what,
	     (unsigned long)((unsigned long long)(long long)r & 0xFFFFFFFFULL),
	     (long)(int32_t)(uint32_t)((unsigned long long)(long long)r & 0xFFFFFFFFULL));
}

#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))

/* The SndBufPtr as the caller's ABI lays it out: four longs (16 bytes on
 * m68k, 32 on LP64).  Two spare longs behind it catch an overlong write. */
static long sbp[6];
#define SENT0 0x11111111L
#define SENT1 0x22222222L
#define SENT2 0x33333333L
#define SENT3 0x44444444L

static void sbp_fill(void)
{
	sbp[0] = SENT0; sbp[1] = SENT1; sbp[2] = SENT2; sbp[3] = SENT3;
	sbp[4] = 0x55555555L; sbp[5] = 0x66666666L;
}

static unsigned long lo32(long v)
{
	return (unsigned long)((unsigned long long)(long long)v & 0xFFFFFFFFULL);
}

static long hz200(void)
{
	return (long)CELL(0x4BA);
}

static long now(void)
{
	return Supexec(hz200);
}

/* ---- oracle-only: MFP and sound registers, the counting handlers -------- */

#ifndef __NEWTOS__
static volatile long rd_val;

static long read_snd_regs(void)
{
	rd_val = (long)(*(volatile uint8_t *)0xFFFF8900UL) << 8 |
	         *(volatile uint8_t *)0xFFFF8901UL;
	return 0;
}

static long snd_regs(void)
{
	Supexec(read_snd_regs);
	return rd_val;
}

/* IERA, IERB, IMRA, IMRB of the (main) MFP */
static volatile uint8_t mfp_save[4];

static long save_mfp(void)
{
	mfp_save[0] = *(volatile uint8_t *)0xFFFFFA07UL;
	mfp_save[1] = *(volatile uint8_t *)0xFFFFFA09UL;
	mfp_save[2] = *(volatile uint8_t *)0xFFFFFA13UL;
	mfp_save[3] = *(volatile uint8_t *)0xFFFFFA15UL;
	return 0;
}

static long restore_mfp(void)
{
	*(volatile uint8_t *)0xFFFFFA07UL = mfp_save[0];
	*(volatile uint8_t *)0xFFFFFA09UL = mfp_save[1];
	*(volatile uint8_t *)0xFFFFFA13UL = mfp_save[2];
	*(volatile uint8_t *)0xFFFFFA15UL = mfp_save[3];
	return 0;
}

/*
 * Three counting MFP interrupt handlers (vr = 0x48: software end of
 * interrupt, references/emutos/bios/mfp.c:108): count, keep the first eight
 * _hz_200 stamps, clear the in-service bit, rte.  0 = timer A (channel 13,
 * ISRA bit 5), 1 = GPIP7 (channel 15, ISRA bit 7), 2 = GPIP5 (channel 7,
 * ISRB bit 7).
 */
volatile long snd_cnt[3] __asm__("snd_cnt");
volatile unsigned long snd_ts[3][8] __asm__("snd_ts");
extern void snd_isr0(void) __asm__("snd_isr0");
extern void snd_isr1(void) __asm__("snd_isr1");
extern void snd_isr2(void) __asm__("snd_isr2");

#define SND_ISR(n, isrreg, bit)						\
	"	.text\n"						\
	"	.even\n"						\
	"snd_isr" #n ":\n"						\
	"	move.l	%d0,-(%sp)\n"					\
	"	move.l	%a0,-(%sp)\n"					\
	"	move.l	snd_cnt+" #n "*4,%d0\n"				\
	"	cmp.l	#8,%d0\n"					\
	"	bcc.s	1f\n"						\
	"	lsl.l	#2,%d0\n"					\
	"	lea	snd_ts+" #n "*32,%a0\n"				\
	"	move.l	0x4ba,0(%a0,%d0.l)\n"				\
	"1:	addq.l	#1,snd_cnt+" #n "*4\n"				\
	"	move.l	(%sp)+,%a0\n"					\
	"	move.l	(%sp)+,%d0\n"					\
	"	bclr	#" #bit "," #isrreg "\n"			\
	"	rte\n"

__asm__(SND_ISR(0, 0xfffffa0f, 5)
        SND_ISR(1, 0xfffffa0f, 7)
        SND_ISR(2, 0xfffffa11, 7));

static void cnt_emit(const char *id, int n, long t0)
{
	int i, k = snd_cnt[n] < 8 ? (int)snd_cnt[n] : 8;

	emit("%s count=%ld ticks-t0=", id, (long)snd_cnt[n]);
	for (i = 0; i < k; i++)
		emit("%s%ld", i ? "," : "", (long)(snd_ts[n][i] - (unsigned long)t0));
	emit("\r\n");
}
#endif

/* ---- helpers for the timing parts -------------------------------------- */

static unsigned long ugcd(unsigned long a, unsigned long b)
{
	while (b) {
		unsigned long t = a % b;
		a = b;
		b = t;
	}
	return a;
}

static long play_ptr(void)
{
	sbp_fill();
	Buffptr(sbp);
	return sbp[0];
}

#define NA 8192L       /* buffer A: 4096 stereo 8-bit frames */
#define NB 4096L       /* buffer B */

static char *bufa, *bufb;

/*
 * Poll Buffoper(-1) and Buffptr until `limit` ticks after t0, or (stop_on_off)
 * until bit 0 has been 0 for 10 ticks.  Reports the statistics of the pointer
 * relative to `base`.
 */
struct pollres {
	long samples, distinct;
	long first_rel, min_rel, max_rel, last_rel;
	long min_step, max_step;
	unsigned long gcd;
	long wraps;            /* the pointer went down */
	long out_of_range;     /* outside [lo, hi] relative to base */
	long off_tick;         /* first tick bit 0 read 0, or -1 */
	long ptr_at_off;       /* the pointer read with that 0 */
	long ptr_before_off;   /* the last pointer read while bit 0 was 1 */
	long last_bo_on_tick;
	long bo_values;        /* bitmask of the Buffoper(-1) values seen (0..15) */
	long end_tick;
	int ndown;             /* the first four times the pointer went down */
	long down_t[4], down_from[4], down_to[4];
};

static void poll(long t0, long limit, int stop_on_off, char *base, long lo, long hi,
                 struct pollres *r, const char *trace_id, int trace_max)
{
	long prev = 0x7FFFFFFFL, prevbo = -1;
	int traced = 0;

	memset(r, 0, sizeof(*r));
	r->off_tick = -1;
	r->min_rel = 0x7FFFFFFFL;
	r->max_rel = -0x7FFFFFFFL;
	r->min_step = 0x7FFFFFFFL;
	r->last_bo_on_tick = -1;
	for (;;) {
		long t = now() - t0;
		long bo = Buffoper(-1);
		long p = play_ptr() - (long)(uintptr_t)base;

		r->samples++;
		if (bo >= 0 && bo < 16)
			r->bo_values |= 1L << bo;
		if (r->samples == 1)
			r->first_rel = p;
		if (p < r->min_rel)
			r->min_rel = p;
		if (p > r->max_rel)
			r->max_rel = p;
		if (p < lo || p > hi)
			r->out_of_range++;
		if (p != prev) {
			if (prev != 0x7FFFFFFFL) {
				long d = p - prev;
				if (d < 0) {
					if (r->ndown < 4) {
						r->down_t[r->ndown] = t;
						r->down_from[r->ndown] = prev;
						r->down_to[r->ndown] = p;
						r->ndown++;
					}
					r->wraps++;
				}
				else {
					if (d < r->min_step)
						r->min_step = d;
					if (d > r->max_step)
						r->max_step = d;
				}
			}
			r->distinct++;
			r->gcd = ugcd(r->gcd, (unsigned long)(p < 0 ? -p : p));
		}
		if (trace_id && traced < trace_max && (p != prev || bo != prevbo)) {
			emit("%s t=%ld bo=%ld ptr=%ld\r\n", trace_id, t, bo, p);
			traced++;
		}
		if (bo & 1) {
			r->last_bo_on_tick = t;
			r->ptr_before_off = p;
		} else if (r->off_tick < 0) {
			r->off_tick = t;
			r->ptr_at_off = p;
		}
		prev = p;
		prevbo = bo;
		r->last_rel = p;
		r->end_tick = t;
		if (t >= limit)
			break;
		if (stop_on_off && r->off_tick >= 0 && t >= r->off_tick + 10)
			break;
	}
}

static void poll_emit(const char *id, const struct pollres *r)
{
	emit("%s samples>0=%d distinct=%ld first=%ld min=%ld max=%ld last=%ld "
	     "minstep=%ld maxstep=%ld gcd=%lu wraps=%ld outofrange=%ld "
	     "bo_seen=%lx off_tick=%ld ptr_at_off=%ld ptr_before_off=%ld "
	     "last_on_tick=%ld end_tick=%ld\r\n",
	     id, r->samples > 0, r->distinct, r->first_rel, r->min_rel, r->max_rel,
	     r->last_rel, r->min_step == 0x7FFFFFFFL ? -1 : r->min_step, r->max_step,
	     r->gcd, r->wraps, r->out_of_range, (unsigned long)r->bo_values,
	     r->off_tick, r->ptr_at_off, r->ptr_before_off, r->last_bo_on_tick,
	     r->end_tick);
	{
		int i;
		for (i = 0; i < r->ndown; i++)
			emit("%s down t=%ld %ld->%ld\r\n", id, r->down_t[i],
			     r->down_from[i], r->down_to[i]);
	}
}

static void sbp_emit(const char *id, long r)
{
	emit("%s Buffptr ret=%08lx play=%08lx record=%08lx res1=%08lx res2=%08lx "
	     "spare=%08lx,%08lx\r\n", id, lo32(r), lo32(sbp[0]), lo32(sbp[1]),
	     lo32(sbp[2]), lo32(sbp[3]), lo32(sbp[4]), lo32(sbp[5]));
}

/* ---- part A: O1 -------------------------------------------------------- */

static void part_a(void)
{
	int m;
	char id[8];
	long r;

	ret("A01", "Locksnd()", Locksnd());
	ret("A02", "Locksnd() again", Locksnd());
	ret("A03", "Unlocksnd()", Unlocksnd());
	ret("A04", "Unlocksnd() again", Unlocksnd());
	for (m = 0; m <= 7; m++) {
		char what[24];
		snprintf(id, sizeof(id), "A%02d", 5 + m);
		snprintf(what, sizeof(what), "Soundcmd(%d,-1)", m);
		ret(id, what, Soundcmd(m, -1));
	}
	ret("A13", "Buffoper(-1)", Buffoper(-1));
	ret("A14", "Buffoper(-2)", Buffoper(-2));
	ret("A15", "Sndstatus(0)", Sndstatus(0));
	sbp_fill();
	r = Buffptr(sbp);
	sbp_emit("A16", r);
	ret("A17", "Setmode(3)", Setmode(3));
	ret("A18", "Setmode(0)", Setmode(0));
	ret("A19", "Settracks(4,0)", Settracks(4, 0));
	ret("A20", "Settracks(0,0)", Settracks(0, 0));
	ret("A21", "Setmontracks(4)", Setmontracks(4));
	ret("A22", "Setmontracks(0)", Setmontracks(0));
	ret("A23", "Setinterrupt(2,0)", Setinterrupt(2, 0));
	ret("A24", "Setinterrupt(0,0)", Setinterrupt(0, 0));
	ret("A25", "Setinterrupt(1,0)", Setinterrupt(1, 0));
	ret("A26", "Setinterrupt(0,1)", Setinterrupt(0, 1));
	ret("A27", "Setinterrupt(0,0)", Setinterrupt(0, 0));
	ret("A28", "Dsptristate(0,0)", Dsptristate(0, 0));
	ret("A29", "Gpio(1,0)", Gpio(1, 0));
	ret("A30", "Gpio(3,0)", Gpio(3, 0));
	ret("A31", "Devconnect(4,8,0,0,1)", Devconnect(4, 8, 0, 0, 1));
	ret("A32", "Devconnect(0,8,0,0,1)", Devconnect(0, 8, 0, 0, 1));
	ret("A33", "Setbuffer(2,0,0)", Setbuffer(2, 0L, 0L));
	ret("A34", "Soundcmd(2,0x0080)", Soundcmd(2, 0x0080));
	ret("A35", "Soundcmd(0,0x0010)", Soundcmd(0, 0x0010));
	ret("A36", "Soundcmd(0,-1)", Soundcmd(0, -1));
	ret("A37", "Soundcmd(0,0)", Soundcmd(0, 0));
	ret("A38", "Soundcmd(6,4)", Soundcmd(6, 4));
	ret("A39", "Soundcmd(6,3)", Soundcmd(6, 3));
	ret("A40", "Buffoper(0)", Buffoper(0));
}

/* ---- parts B-F --------------------------------------------------------- */

static void part_b(void)
{
	struct pollres r;
	long t0, rr;

	ret("B01", "Setmode(0)", Setmode(0));
	ret("B02", "Soundcmd(6,3)", Soundcmd(6, 3));
	ret("B03", "Setbuffer(0,A,A+8192)", Setbuffer(0, (long)(uintptr_t)bufa,
	                                             (long)(uintptr_t)(bufa + NA)));
	sbp_fill();
	rr = Buffptr(sbp);
	emit("B04 before start: play-A=%ld\r\n", sbp[0] - (long)(uintptr_t)bufa);
	sbp_emit("B04", rr);
	t0 = now();
	ret("B05", "Buffoper(1)", Buffoper(1));
	poll(t0, 200, 1, bufa, 0, NA, &r, "B06", 24);
	poll_emit("B07", &r);
	sbp_fill();
	rr = Buffptr(sbp);
	emit("B08 after end: play-A=%ld\r\n", sbp[0] - (long)(uintptr_t)bufa);
	sbp_emit("B08", rr);
	ret("B09", "Buffoper(-1) after end", Buffoper(-1));
}

static void part_c(void)
{
	struct pollres r;
	long t0, p1, p2;

	ret("C01", "Setbuffer(0,A,A+8192)", Setbuffer(0, (long)(uintptr_t)bufa,
	                                             (long)(uintptr_t)(bufa + NA)));
	t0 = now();
	ret("C02", "Buffoper(3)", Buffoper(3));
	poll(t0, 58, 0, bufa, 0, NA, &r, NULL, 0);
	poll_emit("C03", &r);
	ret("C04", "Buffoper(-1) while looping", Buffoper(-1));
	ret("C05", "Buffoper(0)", Buffoper(0));
	p1 = play_ptr() - (long)(uintptr_t)bufa;
	t0 = now();
	while (now() - t0 < 4)
		;
	p2 = play_ptr() - (long)(uintptr_t)bufa;
	emit("C06 after stop: play-A=%ld, 4 ticks later %ld\r\n", p1, p2);
	ret("C07", "Buffoper(-1) after stop", Buffoper(-1));
}

static void part_d(void)
{
	struct pollres r;
	long t0, p, tset = -1;
	long first_b_tick = -1, first_b_ptr = 0, last_a_ptr = -1;

	ret("D01", "Setbuffer(0,A,A+8192)", Setbuffer(0, (long)(uintptr_t)bufa,
	                                             (long)(uintptr_t)(bufa + NA)));
	t0 = now();
	ret("D02", "Buffoper(3)", Buffoper(3));
	/* wait until the pointer is past the middle of A */
	for (;;) {
		long t = now() - t0;
		p = play_ptr() - (long)(uintptr_t)bufa;
		if (p > NA / 2 || t > 100)
			break;
	}
	tset = now() - t0;
	ret("D03", "Setbuffer(0,B,B+4096) while A loops", Setbuffer(0,
	    (long)(uintptr_t)bufb, (long)(uintptr_t)(bufb + NB)));
	emit("D04 set at t=%ld with play-A=%ld\r\n", tset, p);
	for (;;) {
		long t = now() - t0;
		long q = play_ptr();
		if (q >= (long)(uintptr_t)bufa && q <= (long)(uintptr_t)(bufa + NA))
			last_a_ptr = q - (long)(uintptr_t)bufa;
		if (q >= (long)(uintptr_t)bufb && q <= (long)(uintptr_t)(bufb + NB)) {
			first_b_tick = t;
			first_b_ptr = q - (long)(uintptr_t)bufb;
			break;
		}
		if (t > tset + 100)
			break;
	}
	emit("D05 last play-A=%ld; first in B at t=%ld play-B=%ld\r\n",
	     last_a_ptr, first_b_tick, first_b_ptr);
	t0 = now();
	poll(t0, 30, 0, bufb, 0, NB, &r, NULL, 0);
	poll_emit("D06", &r);
	ret("D07", "Buffoper(0)", Buffoper(0));
}

#ifndef __NEWTOS__
static void wait_ticks(long n)
{
	long t0 = now();
	while (now() - t0 < n)
		;
}

static void part_e(void)
{
	long oldA = (long)Setexc(0x4D, -1L);
	long old15 = (long)Setexc(0x4F, -1L);
	long old7 = (long)Setexc(0x47, -1L);
	struct pollres r;
	long t0, tstop;

	Supexec(save_mfp);
	emit("E01 IERA=%02x IERB=%02x IMRA=%02x IMRB=%02x\r\n", mfp_save[0],
	     mfp_save[1], mfp_save[2], mfp_save[3]);
	memset((void *)snd_cnt, 0, sizeof(snd_cnt));
	memset((void *)snd_ts, 0, sizeof(snd_ts));
	Xbtimer(0, 8, 1, snd_isr0);
	Jenabint(13);
	ret("E02", "Setinterrupt(0,1)", Setinterrupt(0, 1));
	emit("E03 regs 8900/01=%04lx\r\n", snd_regs());
	Setbuffer(0, (long)(uintptr_t)bufa, (long)(uintptr_t)(bufa + NA));
	t0 = now();
	emit("E04 t0=%lu\r\n", (unsigned long)t0);
	Buffoper(1);
	poll(t0, 200, 1, bufa, 0, NA, &r, NULL, 0);
	poll_emit("E05", &r);
	cnt_emit("E06 single-shot timerA", 0, t0);
	snd_cnt[0] = 0;
	memset((void *)snd_ts, 0, sizeof(snd_ts));
	t0 = now();
	emit("E07 t0=%lu\r\n", (unsigned long)t0);
	Buffoper(3);
	poll(t0, 58, 0, bufa, 0, NA, &r, NULL, 0);
	tstop = now() - t0;
	Buffoper(0);
	wait_ticks(5);
	poll_emit("E08", &r);
	emit("E08 Buffoper(0) at t=%ld\r\n", tstop);
	cnt_emit("E09 loop timerA", 0, t0);
	snd_cnt[0] = 0;
	ret("E10", "Setinterrupt(0,0)", Setinterrupt(0, 0));
	Setbuffer(0, (long)(uintptr_t)bufa, (long)(uintptr_t)(bufa + NA));
	t0 = now();
	Buffoper(1);
	poll(t0, 200, 1, bufa, 0, NA, &r, NULL, 0);
	cnt_emit("E11 single-shot timerA, Setinterrupt(0,0)", 0, t0);
	Jdisint(13);
	Setexc(0x4D, oldA);

	/* mode 1, "MFP-i7": count on GPIP7 (channel 15) and GPIP5 (channel 7) */
	memset((void *)snd_cnt, 0, sizeof(snd_cnt));
	memset((void *)snd_ts, 0, sizeof(snd_ts));
	Mfpint(15, snd_isr1);
	Mfpint(7, snd_isr2);
	ret("E12", "Setinterrupt(1,1)", Setinterrupt(1, 1));
	emit("E13 regs 8900/01=%04lx\r\n", snd_regs());
	Setbuffer(0, (long)(uintptr_t)bufa, (long)(uintptr_t)(bufa + NA));
	t0 = now();
	emit("E14 t0=%lu\r\n", (unsigned long)t0);
	Buffoper(1);
	poll(t0, 200, 1, bufa, 0, NA, &r, NULL, 0);
	poll_emit("E15", &r);
	ret("E16", "Setinterrupt(1,0)", Setinterrupt(1, 0));
	wait_ticks(2);
	Supexec(restore_mfp);
	Setexc(0x4F, old15);
	Setexc(0x47, old7);
	cnt_emit("E17 single-shot GPIP7 (ch15)", 1, t0);
	cnt_emit("E18 single-shot GPIP5 (ch7)", 2, t0);
	Supexec(save_mfp);
	emit("E19 restored IERA=%02x IERB=%02x IMRA=%02x IMRB=%02x "
	     "vectors 0x134 %d 0x13C %d 0x11C %d\r\n", mfp_save[0], mfp_save[1],
	     mfp_save[2], mfp_save[3], (long)Setexc(0x4D, -1L) == oldA,
	     (long)Setexc(0x4F, -1L) == old15, (long)Setexc(0x47, -1L) == old7);
}

static void part_f(void)
{
	long p1, p2, t0;
	int m;

	Setbuffer(0, (long)(uintptr_t)bufa, (long)(uintptr_t)(bufa + NA));
	ret("F01", "Setinterrupt(0,1)", Setinterrupt(0, 1));
	ret("F02", "Buffoper(3)", Buffoper(3));
	wait_ticks(3);
	emit("F03 before: regs 8900/01=%04lx Buffoper(-1)=%ld\r\n", snd_regs(),
	     Buffoper(-1));
	ret("F03a", "Soundcmd(0,0x0050) (attenuation, zeroed by the reset)",
	    Soundcmd(0, 0x0050));
	ret("F03b", "Soundcmd(4,0x0001)", Soundcmd(4, 0x0001));
	ret("F04", "Sndstatus(1)", Sndstatus(1));
	emit("F05 after: regs 8900/01=%04lx\r\n", snd_regs());
	ret("F06", "Buffoper(-1) after Sndstatus(1)", Buffoper(-1));
	p1 = play_ptr() - (long)(uintptr_t)bufa;
	t0 = now();
	while (now() - t0 < 3)
		;
	p2 = play_ptr() - (long)(uintptr_t)bufa;
	emit("F07 play-A=%ld, 3 ticks later %ld\r\n", p1, p2);
	for (m = 0; m <= 6; m++) {
		char id[8], what[40];
		snprintf(id, sizeof(id), "F%02d", 8 + m);
		snprintf(what, sizeof(what), "Soundcmd(%d,-1) after reset", m);
		ret(id, what, Soundcmd(m, -1));
	}
	ret("F15", "Sndstatus(0) after reset", Sndstatus(0));
	ret("F16", "Buffoper(0)", Buffoper(0));
	ret("F17", "Setinterrupt(0,0)", Setinterrupt(0, 0));
	/* EmuTOS' falcon_dmasound_init sequence, bios/dmasound.c:182-198 */
	ret("F18", "Devconnect(0,8,0,0,1)", Devconnect(0, 8, 0, 0, 1));
	ret("F19", "Setmode(0)", Setmode(0));
	Soundcmd(2, 0x0080);
	Soundcmd(3, 0x0080);
	Soundcmd(4, 0x0003);
	Soundcmd(5, 0x0003);
	Soundcmd(6, 0x0003);
	ret("F20", "Sndstatus(0) after re-init", Sndstatus(0));
}
#endif

int main(void)
{
	long fh, avail;

	emit("SNPROBE TASK/SND04\r\n");
	part_a();
	avail = Locksnd();
	if (avail == 1)
		Unlocksnd();
	if (avail == 0x80) {
		emit("B-F skipped: Locksnd answers its function number (no DMA sound)\r\n");
	} else {
		long a = (long)Mxalloc(NA + NB + 64, 0);
		if (a == -32 || a == 0)
			a = (long)Malloc(NA + NB + 64);
		if (a <= 0) {
			emit("B-F skipped: no memory (%ld)\r\n", a);
		} else {
			bufa = (char *)(uintptr_t)((a + 15) & ~15L);
			bufb = bufa + NA + 16;
			memset(bufa, 0, NA + NB + 16);
			emit("M01 buffers in ST-RAM below 16 MB: %d\r\n",
			     (unsigned long)(uintptr_t)bufa < 0x01000000UL);
			part_b();
			part_c();
			part_d();
#ifdef __NEWTOS__
			emit("E-F skipped: no m68k handler on NewTOS\r\n");
#else
			part_e();
			part_f();
#endif
			Mfree((void *)(uintptr_t)a);
		}
	}
	emit("end\r\n");
	fh = Fcreate("SNPROBE.LOG", 0);
	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}
