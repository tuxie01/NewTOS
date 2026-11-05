/*
 * dbprobe.c -- graf_dragbox with a bounding rectangle larger than the screen: the XOR frame (dotted, one
 * graf_dragbox per scene) runs past the top and the bottom screen edge, so the vertical segments of the
 * frame are clipped by the screen (TASK/V20 acceptance; the desktop's own drags are bounded by the desktop
 * and never reach the edge).  Built and run by tosdump/probes/run-dbprobe.sh (test code only).
 *
 * Each scene: wait for a button press (evnt_button), then graf_dragbox(60, 40, 300, 200, bounds) with the
 * bounds BX,BY,BW,BH = 0,-100,640,700.  The scripted pointer presses at (330,220) and moves up / down to the
 * scene's target y: the frame follows the pointer delta, so its top is at (target y - 20): scene T1 target 3 =
 * top at -17 (17 rows above the screen), T2 target 2 = -18, B1 target 478 = bottom row 498, B2 target 477, S0
 * target 100 = inside the screen (control).  The shots are taken while the button is held.
 *
 * INPUT (as slprobe.c): dbprobe-mouse.txt-style `<ticks> 0xF8|buttons dx dy` and `<ticks> <name>` shots.
 *   NewTOS  tools/inject-arm.py launch:DBPROBE feeds the packets; QMP screendumps by ticks after "armed".
 *   EmuTOS  the probe feeds them from etv_timer (Kbdvbase()->mousevec), DBPROBE.INJ / DBPROBE.SHT, and
 *           increments dbp_shot_n at each shot tick; a Hatari tracking breakpoint on that long takes the shot.
 * Output: DBPROBE.LOG (Hxx handle, Sxx script sizes, Dnn per graf_dragbox: result and final dx dy, Z99 end).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>

#include "abi.h"

#define WF_VSLSIZ    16

#define KIND 0x0FEF		/* all but INFO */
#define WX 80
#define WY 60
#define WW 400
#define WH 320

static char out[262144];
static int olen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 200)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

static void flush_log(void)
{
	long fh = Fcreate("DBPROBE.LOG", 0);

	if (fh < 0)
		return;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
}

static int16_t control[5];
static int16_t global[15];
static int16_t int_in[16];
static int16_t int_out[8];
static void *addr_in[2];
static void *addr_out[2];

static int16_t aes(int16_t op, int16_t nin, int16_t nout, int16_t nain)
{
	int i;

	control[0] = op;
	control[1] = nin;
	control[2] = nout;
	control[3] = nain;
	control[4] = 0;
	for (i = 0; i < 8; i++)
		int_out[i] = 0;
	td_abi_aes(control, global, int_in, int_out, addr_in, addr_out);
	return int_out[0];
}

static long hz200(void)
{
	return (long)*(volatile uint32_t *)(uintptr_t)0x4BA;
}

static uint32_t ticks(void)
{
	return (uint32_t)Supexec(hz200);
}


/* ---- EmuTOS: the in-probe IKBD mouse feeder (mgprobe.c's) + shot counter ---- */

#ifndef __NEWTOS__
#define NSTEP 4096
#define NSHOT 256
static uint32_t st_tick[NSTEP];
static unsigned char st_pkt[NSTEP][3];
static int nstep;
static uint32_t sh_tick[NSHOT];
static int nshot;

volatile uint32_t inj_t0 __asm__("dbp_inj_t0");
volatile int inj_next __asm__("dbp_inj_next");
volatile int inj_armed __asm__("dbp_inj_armed");
void *volatile inj_old __asm__("dbp_inj_old");
volatile int shot_next __asm__("dbp_shot_next");
/* the Hatari breakpoint's target: changes exactly once per shot tick */
volatile uint32_t shot_n __asm__("dbp_shot_n");
static long *kvecs;			/* Kbdvbase(): ... mousevec at [4] */
static uint32_t inj_at[NSTEP];
static uint32_t shot_at[NSHOT];
static unsigned char inj_buf[3];

void inj_timer(void) __asm__("dbp_inj_timer");
void inj_mousevec(void *fn, unsigned char *pkt) __asm__("dbp_inj_mousevec");
void inj_step(void) __asm__("dbp_inj_step");

__asm__(
"	.text\n"
"	.even\n"
"dbp_inj_timer:\n"
"	movem.l	d0-d7/a0-a6,-(sp)\n"
"	jsr	dbp_inj_step\n"
"	movem.l	(sp)+,d0-d7/a0-a6\n"
"	move.l	dbp_inj_old,-(sp)\n"
"	rts\n"
"dbp_inj_mousevec:\n"
"	move.l	4(sp),a1\n"
"	move.l	8(sp),a0\n"
"	movem.l	d2-d7/a2-a6,-(sp)\n"
"	jsr	(a1)\n"
"	movem.l	(sp)+,d2-d7/a2-a6\n"
"	rts\n"
);

void inj_step(void)
{
	uint32_t now;

	if (!inj_armed)
		return;
	now = *(volatile uint32_t *)0x4BAUL;
	while (inj_next < nstep && now - inj_t0 >= st_tick[inj_next]) {
		inj_buf[0] = st_pkt[inj_next][0];
		inj_buf[1] = st_pkt[inj_next][1];
		inj_buf[2] = st_pkt[inj_next][2];
		inj_at[inj_next] = now;
		inj_mousevec((void *)kvecs[4], inj_buf);
		inj_next++;
	}
	if (shot_next < nshot && now - inj_t0 >= sh_tick[shot_next]) {
		shot_at[shot_next] = now;
		shot_next++;
		shot_n = (uint32_t)shot_next;	/* one change per timer call at most */
	}
}

static long arm(void)
{
	inj_next = 0;
	shot_next = 0;
	inj_t0 = *(volatile uint32_t *)0x4BAUL;
	inj_armed = 1;
	return (long)inj_t0;
}

static long disarm(void)
{
	inj_armed = 0;
	return 0;
}

static int num(const char **p, long *v)
{
	const char *s = *p;
	long n = 0;
	int base = 10, any = 0, neg = 0;

	while (*s == ' ' || *s == '\t')
		s++;
	if (*s == '-') {
		neg = 1;
		s++;
	}
	if (s[0] == '0' && (s[1] == 'x' || s[1] == 'X')) {
		base = 16;
		s += 2;
	}
	for (;; s++) {
		int d;

		if (*s >= '0' && *s <= '9')
			d = *s - '0';
		else if (base == 16 && *s >= 'a' && *s <= 'f')
			d = *s - 'a' + 10;
		else if (base == 16 && *s >= 'A' && *s <= 'F')
			d = *s - 'A' + 10;
		else
			break;
		n = n * base + d;
		any = 1;
	}
	*p = s;
	*v = neg ? -n : n;
	return any;
}

static char script[131072];

/* `want` numbers per line (4: packets, 1: shot ticks, the rest is the name);
 * '#' comments.  Returns the line count or < 0. */
static int load(const char *name, int want)
{
	long fh = Fopen(name, 0), n;
	const char *p, *end;
	int cnt = 0;

	if (fh < 0)
		return -1;
	n = Fread((short)fh, (long)sizeof(script) - 1, script);
	Fclose((short)fh);
	if (n <= 0 || n >= (long)sizeof(script) - 1)
		return -2;
	script[n] = 0;
	p = script;
	end = script + n;
	while (p < end) {
		const char *eol = p;
		long v[4];
		int k = 0;

		while (eol < end && *eol != '\n')
			eol++;
		while (k < want && p < eol && *p != '#' && *p != '\r' && num(&p, &v[k]))
			k++;
		if (k == want) {
			if (want == 4 && nstep < NSTEP) {
				st_tick[nstep] = (uint32_t)v[0];
				st_pkt[nstep][0] = (unsigned char)v[1];
				st_pkt[nstep][1] = (unsigned char)v[2];
				st_pkt[nstep][2] = (unsigned char)v[3];
				nstep++;
			} else if (want == 1 && nshot < NSHOT) {
				sh_tick[nshot++] = (uint32_t)v[0];
			} else {
				return -4;
			}
			cnt++;
		} else if (k != 0) {
			return -3;
		}
		p = eol + 1;
	}
	return cnt;
}

static int file_exists(const char *name)
{
	long fh = Fopen(name, 0);

	if (fh < 0)
		return 0;
	Fclose((short)fh);
	return 1;
}
#endif

typedef struct { int16_t w, h, sx, sy, bx, by, bw, bh; const char *name; } SCENE;
static const SCENE scene[] = {
	{ 60, 40, 300, 200, 0, -100, 640, 700, "S0 inside" },
	{ 60, 40, 300, 200, 0, -100, 640, 700, "T1 top -17" },
	{ 60, 40, 300, 200, 0, -100, 640, 700, "T2 top -18" },
	{ 60, 40, 300, 200, 0, -100, 640, 700, "B1 bottom 498" },
	{ 60, 40, 300, 200, 0, -100, 640, 700, "B2 bottom 497" },
};

int main(void)
{
	int16_t ap_id = aes(10, 0, 1, 0);	/* appl_init */
	unsigned k;
	uint32_t t;

	if (ap_id < 0) {
		emit("A99 appl_init=%d\n", ap_id);
		flush_log();
		return 1;
	}
	aes(77, 0, 5, 0);			/* graf_handle */
	emit("H00 graf_handle %d %d %d %d\n", int_out[1], int_out[2], int_out[3], int_out[4]);
	int_in[0] = 0;
	addr_in[0] = 0;
	aes(78, 1, 1, 1);			/* graf_mouse(ARROW) */
#ifndef __NEWTOS__
	{
		int r = load("DBPROBE.INJ", 4), s = load("DBPROBE.SHT", 1);
		long fh;

		emit("S00 script steps %d shots %d\n", r, s);
		if (r < 0 || s < 0) {
			flush_log();
			aes(19, 0, 1, 0);
			return 1;
		}
		emit("A00 shot counter at %08lx\n", (unsigned long)(uintptr_t)&shot_n);
		flush_log();
		fh = Fcreate("DBPROBE.ADR", 0);
		if (fh >= 0) {
			char a[16];
			int n = snprintf(a, sizeof(a), "%08lx\n", (unsigned long)(uintptr_t)&shot_n);

			Fwrite((short)fh, (long)n, a);
			Fclose((short)fh);
		}
		for (k = 0; k < 600 && !file_exists("DBPROBE.GO"); k++) {
			int_in[0] = 100;
			int_in[1] = 0;
			aes(24, 2, 1, 0);	/* evnt_timer(100 ms) */
		}
		emit("G02 go after %u polls\n", k);
		kvecs = (long *)Kbdvbase();
		inj_old = (void *)Setexc(0x100, -1L);
		Setexc(0x100, (long)inj_timer);
		t = (uint32_t)Supexec(arm);
	}
#else
	t = ticks();
#endif
	emit("T00 t=%lu\n", (unsigned long)t);
	flush_log();
	for (k = 0; k < sizeof(scene) / sizeof(scene[0]); k++) {
		const SCENE *sc = &scene[k];
		int16_t ret, dx, dy;

		int_in[0] = 1;			/* evnt_button: wait for the left press */
		int_in[1] = 1;
		int_in[2] = 1;
		aes(21, 3, 5, 0);
		emit("E%02u press at %d,%d t=%lu\n", k, int_out[1], int_out[2], (unsigned long)ticks());
		int_in[0] = sc->w;
		int_in[1] = sc->h;
		int_in[2] = sc->sx;
		int_in[3] = sc->sy;
		int_in[4] = sc->bx;
		int_in[5] = sc->by;
		int_in[6] = sc->bw;
		int_in[7] = sc->bh;
		aes(71, 8, 3, 0);		/* graf_dragbox */
		ret = int_out[0];
		dx = int_out[1];
		dy = int_out[2];
		emit("D%02u %s graf_dragbox=%d dx %d dy %d t=%lu\n", k, sc->name, ret, dx, dy, (unsigned long)ticks());
		flush_log();
	}
#ifndef __NEWTOS__
	Supexec(disarm);
	if ((void *)Setexc(0x100, -1L) == (void *)inj_timer)
		Setexc(0x100, (long)inj_old);
#endif
	emit("Z99 end\n");
	flush_log();
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
