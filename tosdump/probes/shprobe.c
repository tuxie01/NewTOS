/*
 * shprobe.c -- what a window's outer frame, its drop shadow and the 3D bevels
 * of its gadgets look like at rest, topped and untopped, overlapping and not
 * (TASK/A131, GitHub #7 "Shadow of windows wrong").  A sibling of slprobe.c
 * (same tosdump ABI backends, same shot mechanics, no mouse input); the
 * evidence is the screen, captured from outside, so the probe only provides
 * the windows, a fixed schedule and the log.  Built and run by
 * tosdump/probes/run-shprobe.sh.  Output: SHPROBE.LOG in the current
 * directory.
 *
 * WINDOWS: two, kind 0x0FEF (all gadgets but INFO, no WF_NAME string -- a
 * string pointer in int_in is not portable across the two ABIs), WF_VSLSIZ =
 * WF_HSLSIZ = 300 and WF_VSLIDE = WF_HSLIDE = 500 so the elevators and the
 * tracks are both visible.  WM_REDRAW: the rectangle list clipped to the
 * damage, objc_draw of a white G_BOX over the work area (slprobe.c's).
 *   W1 border 20,40,300,250    W2 border 340,120,280,300 (apart),
 *   W2 later moved to 200,150,300,240 (overlapping W1's lower right corner).
 *
 * SCHEDULE (200 Hz ticks from T0; STATE ticks per state, the state's change is
 * made at its start; shots at +SHOT_A and +SHOT_B into each state, so two shots
 * of each static state -- they must be equal):
 *   state 0  W1 open alone (topped)
 *   state 1  W2 opened apart (W2 topped, W1 untopped)
 *   state 2  wind_set(W1, WF_TOP) (W1 topped, W2 untopped)
 *   state 3  wind_set(W2, WF_CURRXYWH, overlap) (W1 stays on top; W1's shadow
 *            falls on W2)
 *   state 4  wind_set(W2, WF_TOP) (W2's shadow falls on W1)
 *   state 5  wind_set(Wn, WF_COLOR, W_BOX, t, b) on both windows with the
 *            border colour nibble (bits 15..12) of W_BOX's current words set
 *            to 2 (RED), the rest kept: does the shadow follow W_BOX's word?
 *   then: Z99, both windows closed and deleted, appl_exit.
 *
 *   EmuTOS  T0 = the tick after C:\SHPROBE.GO appeared; the probe increments
 *           shp_shot_n at each shot tick, a Hatari tracking breakpoint on that
 *           long takes the screenshot (run-shprobe.sh).
 *   NewTOS  T0 = the probe's first tick; the host takes QMP screendumps at
 *           the same offsets after the injector's "launch:SHPROBE armed" line
 *           (tools/inject-arm.py with an empty script: only the clock), so the
 *           host's shot lies (T0 - armed) ticks early -- logged as T00/T01.
 *
 * Lines: H00 graf_handle, C<n> wind_create, O<n> wind_open, G<n> rects,
 * W<n> wind_get(WF_COLOR / WF_DCOLOR) of every gadget, A00 shot counter
 * address (EmuTOS), T00 start tick, Y<k> state k applied at tick, K<n> shot
 * n at tick, D<n> redraw rectangles, Z99 end.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>

#include "abi.h"

typedef struct {
	int16_t ob_next, ob_head, ob_tail;
	uint16_t ob_type, ob_flags, ob_state;
	long ob_spec;
	int16_t ob_x, ob_y, ob_width, ob_height;
} OBJ;

#define G_BOX     20
#define LASTOB    0x0020

#define MU_MESAG  0x0010
#define MU_TIMER  0x0020

#define WM_REDRAW  20

#define WF_WXYWH     4
#define WF_CXYWH     5
#define WF_HSLIDE    8
#define WF_VSLIDE    9
#define WF_TOP       10
#define WF_FIRSTXYWH 11
#define WF_NEXTXYWH  12
#define WF_HSLSIZ    15
#define WF_VSLSIZ    16
#define WF_COLOR     18
#define WF_DCOLOR    19

#define KIND 0x0FEF		/* all but INFO */
#define NGADGET 19		/* W_BOX .. W_HELEV, gemlib.h */

#define STATE  1600		/* 8 s */
#define SHOT_A 700
#define SHOT_B 1400
#define NSTATE 6

static const int16_t geo[3][4] = {
	{ 20, 40, 300, 250 },	/* W1 */
	{ 340, 120, 280, 300 },	/* W2 apart */
	{ 200, 150, 300, 240 },	/* W2 overlapping W1 */
};

static char out[65536];
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
	long fh = Fcreate("SHPROBE.LOG", 0);

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

static int16_t wh[2];
static OBJ work[2];

static int16_t wset(int16_t h, int16_t field, int16_t a, int16_t b, int16_t c, int16_t d)
{
	int_in[0] = h;
	int_in[1] = field;
	int_in[2] = a;
	int_in[3] = b;
	int_in[4] = c;
	int_in[5] = d;
	return aes(105, 6, 1, 0);		/* wind_set */
}

static void wget(int16_t h, int16_t field, int16_t sub)
{
	int_in[0] = h;
	int_in[1] = field;
	int_in[2] = sub;
	aes(104, 3, 5, 0);			/* wind_get, int_in[2] = gadget for WF_COLOR */
}

static void wupdate(int16_t mode)
{
	int_in[0] = mode;
	aes(107, 1, 1, 0);			/* wind_update */
}

static void set_work(int i)
{
	wget(wh[i], WF_WXYWH, 0);
	work[i].ob_next = -1;
	work[i].ob_head = -1;
	work[i].ob_tail = -1;
	work[i].ob_type = G_BOX;
	work[i].ob_flags = LASTOB;
	work[i].ob_state = 0;
	work[i].ob_spec = 0x00000070L;	/* no border, solid white */
	work[i].ob_x = int_out[1];
	work[i].ob_y = int_out[2];
	work[i].ob_width = int_out[3];
	work[i].ob_height = int_out[4];
	emit("G%d.%d work %d,%d,%d,%d", i, 1, int_out[1], int_out[2], int_out[3], int_out[4]);
	wget(wh[i], WF_CXYWH, 0);
	emit(" curr %d,%d,%d,%d\n", int_out[1], int_out[2], int_out[3], int_out[4]);
}

static void redraw(int n, const int16_t *m)
{
	int16_t dx = m[4], dy = m[5], dw = m[6], dh = m[7];
	int i = (m[3] == wh[0]) ? 0 : 1;
	int k = 0;

	wupdate(1);
	wget(wh[i], WF_FIRSTXYWH, 0);
	for (;;) {
		int16_t r[4], x0, y0, x1, y1;

		r[0] = int_out[1];
		r[1] = int_out[2];
		r[2] = int_out[3];
		r[3] = int_out[4];
		if (r[2] <= 0 || r[3] <= 0)
			break;
		x0 = r[0] > dx ? r[0] : dx;
		y0 = r[1] > dy ? r[1] : dy;
		x1 = (r[0] + r[2] < dx + dw) ? r[0] + r[2] : dx + dw;
		y1 = (r[1] + r[3] < dy + dh) ? r[1] + r[3] : dy + dh;
		emit("D%03d.%d w%d damage %d,%d,%d,%d list %d,%d,%d,%d\n", n, k, i + 1, dx, dy, dw, dh,
		     r[0], r[1], r[2], r[3]);
		if (x1 > x0 && y1 > y0) {
			int_in[0] = 0;
			int_in[1] = 1;
			int_in[2] = x0;
			int_in[3] = y0;
			int_in[4] = (int16_t)(x1 - x0);
			int_in[5] = (int16_t)(y1 - y0);
			addr_in[0] = &work[i];
			aes(42, 6, 1, 1);	/* objc_draw */
		}
		if (++k > 64)
			break;
		wget(wh[i], WF_NEXTXYWH, 0);
	}
	wupdate(0);
}

static int open_win(int i, int g)
{
	int_in[0] = KIND;
	int_in[1] = 0;
	int_in[2] = 0;
	int_in[3] = 640;
	int_in[4] = 480;
	wh[i] = aes(100, 5, 1, 0);		/* wind_create */
	emit("C%d wind_create(%04x)=%d\n", i, KIND, wh[i]);
	if (wh[i] <= 0)
		return -1;
	wset(wh[i], WF_VSLSIZ, 300, 0, 0, 0);
	wset(wh[i], WF_HSLSIZ, 300, 0, 0, 0);
	wset(wh[i], WF_VSLIDE, 500, 0, 0, 0);
	wset(wh[i], WF_HSLIDE, 500, 0, 0, 0);
	int_in[0] = wh[i];
	int_in[1] = geo[g][0];
	int_in[2] = geo[g][1];
	int_in[3] = geo[g][2];
	int_in[4] = geo[g][3];
	aes(101, 5, 1, 0);			/* wind_open */
	emit("O%d wind_open(%d,%d,%d,%d)=%d\n", i, geo[g][0], geo[g][1], geo[g][2], geo[g][3], int_out[0]);
	set_work(i);
	return 0;
}

static void log_colors(int i)
{
	int g;

	for (g = 0; g < NGADGET; g++) {
		int16_t t, b, dt, db, r1, r2;

		/* int_out[2] = topped word, int_out[3] = untopped word (gemwmlib.c:1494-1498,
		 * :1567-1576: poutwds = &int_out[1], poutwds[1..2]) */
		r1 = (wget(wh[i], WF_COLOR, g), int_out[0]);
		t = int_out[2];
		b = int_out[3];
		r2 = (wget(wh[i], WF_DCOLOR, g), int_out[0]);
		dt = int_out[2];
		db = int_out[3];
		emit("W%d.%02d WF_COLOR=%d t=%04x b=%04x WF_DCOLOR=%d t=%04x b=%04x\n", i + 1, g, r1, (uint16_t)t,
		     (uint16_t)b, r2, (uint16_t)dt, (uint16_t)db);
	}
}

/* ---- EmuTOS: the shot counter (a Hatari breakpoint watches it) ---------- */

#ifndef __NEWTOS__
volatile uint32_t shot_n __asm__("shp_shot_n");

static int file_exists(const char *name)
{
	long fh = Fopen(name, 0);

	if (fh < 0)
		return 0;
	Fclose((short)fh);
	return 1;
}
#endif

static int16_t msg[8];
static int nmsg;

/* wait until tick `until`, answering every message meanwhile */
static void wait_until(uint32_t until)
{
	int i;

	for (;;) {
		uint32_t now = ticks();
		int32_t left = (int32_t)(until - now);
		long ms;

		if (left <= 0)
			return;
		ms = (long)left * 5;
		if (ms > 100)
			ms = 100;
		for (i = 0; i < 16; i++)
			int_in[i] = 0;
		int_in[0] = MU_MESAG | MU_TIMER;
		int_in[14] = (int16_t)ms;
		int_in[15] = 0;
		for (i = 0; i < 8; i++)
			msg[i] = 0;
		addr_in[0] = msg;
		aes(25, 16, 7, 1);		/* evnt_multi */
		if (int_out[0] & MU_MESAG) {
			int16_t m[8];

			nmsg++;
			for (i = 0; i < 8; i++)
				m[i] = msg[i];
			emit("R%03d t=%lu msg=%d,%d,%d,%d,%d,%d,%d,%d\n", nmsg, (unsigned long)ticks(), m[0],
			     m[1], m[2], m[3], m[4], m[5], m[6], m[7]);
			if (m[0] == WM_REDRAW && (m[3] == wh[0] || m[3] == wh[1]))
				redraw(nmsg, m);
		}
	}
}

int main(void)
{
	int16_t ap_id = aes(10, 0, 1, 0);	/* appl_init */
	uint32_t t0;
	int k, n = 0;

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
	int_in[0] = 256;
	aes(78, 1, 1, 1);			/* graf_mouse(M_OFF): no pointer in the shots */

#ifndef __NEWTOS__
	{
		long fh;
		int i;

		emit("A00 shot counter at %08lx\n", (unsigned long)(uintptr_t)&shot_n);
		flush_log();
		fh = Fcreate("SHPROBE.ADR", 0);
		if (fh >= 0) {
			char a[16];
			int len = snprintf(a, sizeof(a), "%08lx\n", (unsigned long)(uintptr_t)&shot_n);

			Fwrite((short)fh, (long)len, a);
			Fclose((short)fh);
		}
		for (i = 0; i < 600 && !file_exists("SHPROBE.GO"); i++) {
			int_in[0] = 100;
			int_in[1] = 0;
			aes(24, 2, 1, 0);	/* evnt_timer(100 ms) */
		}
		emit("G99 go after %d polls\n", i);
	}
#endif
	t0 = ticks();
	emit("T00 t0=%lu\n", (unsigned long)t0);
	flush_log();

	for (k = 0; k < NSTATE; k++) {
		uint32_t ts = t0 + (uint32_t)k * STATE;

		wait_until(ts);
		emit("Y%d state %d at t=%lu (+%lu)\n", k, k, (unsigned long)ticks(), (unsigned long)(ticks() - t0));
		switch (k) {
		case 0:
			if (open_win(0, 0) < 0)
				goto out;
			log_colors(0);
			break;
		case 1:
			if (open_win(1, 1) < 0)
				goto out;
			log_colors(1);
			break;
		case 2:
			emit("Y2 wind_set(W1,WF_TOP)=%d\n", wset(wh[0], WF_TOP, 0, 0, 0, 0));
			break;
		case 3:
			emit("Y3 wind_set(W2,WF_CURRXYWH)=%d\n",
			     wset(wh[1], WF_CXYWH, geo[2][0], geo[2][1], geo[2][2], geo[2][3]));
			set_work(1);
			break;
		case 4:
			emit("Y4 wind_set(W2,WF_TOP)=%d\n", wset(wh[1], WF_TOP, 0, 0, 0, 0));
			break;
		case 5: {
			int i;

			for (i = 0; i < 2; i++) {
				int16_t t, b, r;

				wget(wh[i], WF_COLOR, 0);
				t = (int16_t)((int_out[2] & 0x0FFF) | 0x2000);
				b = (int16_t)((int_out[3] & 0x0FFF) | 0x2000);
				int_in[0] = wh[i];
				int_in[1] = WF_COLOR;
				int_in[2] = 0;		/* W_BOX */
				int_in[3] = t;
				int_in[4] = b;
				int_in[5] = 0;
				r = aes(105, 6, 1, 0);
				wget(wh[i], WF_COLOR, 0);
				emit("Y5 wind_set(W%d,WF_COLOR,W_BOX,%04x,%04x)=%d readback %04x %04x\n", i + 1,
				     (uint16_t)t, (uint16_t)b, r, (uint16_t)int_out[2], (uint16_t)int_out[3]);
			}
			break;
		}
		}
		wget(0, WF_TOP, 0);
		emit("Y%d top=%d\n", k, int_out[1]);
		flush_log();
		wait_until(ts + SHOT_A);
		emit("K%02d t=%lu (+%lu)\n", n, (unsigned long)ticks(), (unsigned long)(ticks() - t0));
		n++;
#ifndef __NEWTOS__
		shot_n = (uint32_t)n;
#endif
		wait_until(ts + SHOT_B);
		emit("K%02d t=%lu (+%lu)\n", n, (unsigned long)ticks(), (unsigned long)(ticks() - t0));
		n++;
#ifndef __NEWTOS__
		shot_n = (uint32_t)n;
#endif
		flush_log();
	}
	wait_until(t0 + NSTATE * STATE);
out:
	emit("Z99 end shots=%d\n", n);
	flush_log();
	for (k = 1; k >= 0; k--) {
		if (wh[k] > 0) {
			int_in[0] = wh[k];
			aes(102, 1, 1, 0);	/* wind_close */
			int_in[0] = wh[k];
			aes(103, 1, 1, 0);	/* wind_delete */
		}
	}
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
