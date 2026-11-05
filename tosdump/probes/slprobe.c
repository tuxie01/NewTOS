/*
 * slprobe.c -- what the screen shows while the user presses and drags in a
 * window's scroll bars (track and elevator), and after the release, and
 * which messages the application gets (TASK/A130, GitHub #2).  A sibling of
 * mgprobe.c (same etv_timer mouse feeder on EmuTOS, same tools/inject-arm.py
 * arming on NewTOS, same ABI backends); the evidence is the screen, captured
 * from outside, so the probe only provides the window, its slider settings,
 * the replies to the window messages, the message log and the input clock.
 * Built and run by tosdump/probes/run-slprobe.sh.  Output: SLPROBE.LOG in the
 * current directory, rewritten after every evnt_multi return.
 *
 * WINDOW: wind_create(NAME|CLOSER|FULLER|MOVER|SIZER|UPARROW|DNARROW|VSLIDE|
 * LFARROW|RTARROW|HSLIDE), wind_open at the border rect WX,WY,WW,WH (no
 * WF_NAME string).  WM_REDRAW: the rectangle list (WF_FIRSTXYWH/NEXTXYWH)
 * clipped to the damage, objc_draw of a white G_BOX over the work area,
 * inside wind_update(BEG/END_UPDATE).
 *
 * WAIT: evnt_multi(MU_BUTTON|MU_MESAG, 2, 1, 1) (the desktop's, as mgprobe.c).
 * CONFIGURATIONS: a left-button press in the WORK area selects one by the
 * column it falls in (the work area split into 4 equal columns):
 *   col 0  WF_VSLSIZ = WF_HSLSIZ = 1000, WF_VSLIDE = WF_HSLIDE = 500
 *   col 1  size 300, position 500
 *   col 2  size -1 (the minimum, "use default size"), position 500
 *   col 3  quit (wind_close, wind_delete, appl_exit)
 * The press re-applies its configuration in full every time, so a press in
 * the same column resets the elevator positions.
 * REPLIES: WM_VSLID/WM_HSLID -> wind_set(WF_VSLIDE/WF_HSLIDE, msg[4]);
 * WM_ARROWED -> the matching position moves by 250 (page) or 50 (line),
 * clamped to 1..1000 (wind_set's own range, gemwmlib.c:1650-1655), and is
 * set with wind_set.  Every message is logged with all eight words.
 *
 * INPUT.  tosdump/probes/slprobe-mouse.txt (`<ticks> 0xF8|buttons dx dy`) and
 * slprobe-shots.txt (`<ticks> <name>`), exactly as mgprobe.c:
 *   NewTOS  tools/inject-arm.py launch:SLPROBE feeds the packets;
 *           run-slprobe.sh takes QMP screendumps after the "armed" line.
 *   EmuTOS  the probe feeds them from etv_timer (Kbdvbase()->mousevec) and
 *           increments slp_shot_n at each shot tick; a Hatari tracking
 *           breakpoint on that long takes the screenshot.
 *
 * Lines: H00 graf_handle, C00 wind_create, O00 wind_open, G<n> wind_get of
 * the rects, P<k> configuration k applied (+ readback), S00 script/shot
 * counts, A00 the shot counter's address (EmuTOS), T00 input clock start,
 * R<n> per evnt_multi (tick, which, msg[0..7] or mouse/button), X<n> the count
 * of further WM_ARROWED equal to the one before (not logged one by one), D<n> the
 * redraw rectangles, J<n>/K<n> (EmuTOS) feed and shot ticks, Z99 end.
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

#define MU_BUTTON 0x0002
#define MU_MESAG  0x0010

#define WM_REDRAW  20
#define WM_ARROWED 24
#define WM_HSLID   25
#define WM_VSLID   26

#define WF_WXYWH     4
#define WF_CXYWH     5
#define WF_HSLIDE    8
#define WF_VSLIDE    9
#define WF_FIRSTXYWH 11
#define WF_NEXTXYWH  12
#define WF_HSLSIZ    15
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
	long fh = Fcreate("SLPROBE.LOG", 0);

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

static int16_t wh;
static OBJ work[1];

static int16_t wset(int16_t field, int16_t v)
{
	int i;

	for (i = 0; i < 6; i++)
		int_in[i] = 0;
	int_in[0] = wh;
	int_in[1] = field;
	int_in[2] = v;
	return aes(105, 6, 1, 0);		/* wind_set */
}

static int16_t wget(int16_t field)
{
	int_in[0] = wh;
	int_in[1] = field;
	aes(104, 2, 5, 0);			/* wind_get */
	return int_out[1];
}

static void wupdate(int16_t mode)
{
	int_in[0] = mode;
	aes(107, 1, 1, 0);			/* wind_update */
}

static void redraw(int n, const int16_t *m)
{
	int16_t dx = m[4], dy = m[5], dw = m[6], dh = m[7];
	int16_t r[4];
	int k = 0;

	wupdate(1);
	int_in[0] = wh;
	int_in[1] = WF_FIRSTXYWH;
	aes(104, 2, 5, 0);
	for (;;) {
		int16_t x0, y0, x1, y1;

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
		emit("D%03d.%d list %d,%d,%d,%d draw %d,%d,%d,%d\n", n, k, r[0], r[1], r[2], r[3],
		     x0, y0, x1 - x0, y1 - y0);
		if (x1 > x0 && y1 > y0) {
			int_in[0] = 0;
			int_in[1] = 1;
			int_in[2] = x0;
			int_in[3] = y0;
			int_in[4] = (int16_t)(x1 - x0);
			int_in[5] = (int16_t)(y1 - y0);
			addr_in[0] = work;
			aes(42, 6, 1, 1);	/* objc_draw */
		}
		if (++k > 64)
			break;
		int_in[0] = wh;
		int_in[1] = WF_NEXTXYWH;
		aes(104, 2, 5, 0);
	}
	wupdate(0);
}

static int16_t vpos, hpos;

static void config(int k)
{
	static const int16_t size[3] = { 1000, 300, -1 };

	vpos = hpos = 500;
	wset(WF_VSLSIZ, size[k]);
	wset(WF_HSLSIZ, size[k]);
	wset(WF_VSLIDE, vpos);
	wset(WF_HSLIDE, hpos);
	emit("P%d size %d pos %d readback vslsiz %d hslsiz %d vslide %d hslide %d\n", k, size[k], vpos,
	     wget(WF_VSLSIZ), wget(WF_HSLSIZ), wget(WF_VSLIDE), wget(WF_HSLIDE));
}

static int16_t clamp_pos(int v)
{
	return (int16_t)(v < 1 ? 1 : v > 1000 ? 1000 : v);
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

volatile uint32_t inj_t0 __asm__("slp_inj_t0");
volatile int inj_next __asm__("slp_inj_next");
volatile int inj_armed __asm__("slp_inj_armed");
void *volatile inj_old __asm__("slp_inj_old");
volatile int shot_next __asm__("slp_shot_next");
/* the Hatari breakpoint's target: changes exactly once per shot tick */
volatile uint32_t shot_n __asm__("slp_shot_n");
static long *kvecs;			/* Kbdvbase(): ... mousevec at [4] */
static uint32_t inj_at[NSTEP];
static uint32_t shot_at[NSHOT];
static unsigned char inj_buf[3];

void inj_timer(void) __asm__("slp_inj_timer");
void inj_mousevec(void *fn, unsigned char *pkt) __asm__("slp_inj_mousevec");
void inj_step(void) __asm__("slp_inj_step");

__asm__(
"	.text\n"
"	.even\n"
"slp_inj_timer:\n"
"	movem.l	d0-d7/a0-a6,-(sp)\n"
"	jsr	slp_inj_step\n"
"	movem.l	(sp)+,d0-d7/a0-a6\n"
"	move.l	slp_inj_old,-(sp)\n"
"	rts\n"
"slp_inj_mousevec:\n"
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

/* ---- the wait ------------------------------------------------------------- */

static int16_t msg[8];
static int16_t last_arrow[8];
static int nrep, last_rep_m1;
static uint32_t last_rep_t;

int main(void)
{
	int16_t ap_id = aes(10, 0, 1, 0);	/* appl_init */
	int k, i, quit = 0;
	int16_t wx, wy, ww, whh;
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

	int_in[0] = KIND;
	int_in[1] = 0;
	int_in[2] = 0;
	int_in[3] = 640;
	int_in[4] = 480;
	wh = aes(100, 5, 1, 0);			/* wind_create */
	emit("C00 wind_create(%04x)=%d\n", KIND, wh);
	if (wh <= 0) {
		flush_log();
		aes(19, 0, 1, 0);
		return 1;
	}
	int_in[0] = wh;
	int_in[1] = WX;
	int_in[2] = WY;
	int_in[3] = WW;
	int_in[4] = WH;
	aes(101, 5, 1, 0);			/* wind_open */
	emit("O00 wind_open=%d\n", int_out[0]);
	int_in[0] = wh;
	int_in[1] = WF_CXYWH;
	aes(104, 2, 5, 0);
	emit("G00 curr %d,%d,%d,%d\n", int_out[1], int_out[2], int_out[3], int_out[4]);
	int_in[0] = wh;
	int_in[1] = WF_WXYWH;
	aes(104, 2, 5, 0);
	wx = int_out[1];
	wy = int_out[2];
	ww = int_out[3];
	whh = int_out[4];
	emit("G01 work %d,%d,%d,%d\n", wx, wy, ww, whh);
	work[0].ob_next = -1;
	work[0].ob_head = -1;
	work[0].ob_tail = -1;
	work[0].ob_type = G_BOX;
	work[0].ob_flags = LASTOB;
	work[0].ob_state = 0;
	work[0].ob_spec = 0x00000070L;	/* no border, solid white */
	work[0].ob_x = wx;
	work[0].ob_y = wy;
	work[0].ob_width = ww;
	work[0].ob_height = whh;
	config(0);

#ifndef __NEWTOS__
	{
		int r = load("SLPROBE.INJ", 4), s = load("SLPROBE.SHT", 1);
		long fh;

		emit("S00 script steps %d shots %d\n", r, s);
		if (r < 0 || s < 0) {
			flush_log();
			aes(19, 0, 1, 0);
			return 1;
		}
		emit("A00 shot counter at %08lx\n", (unsigned long)(uintptr_t)&shot_n);
		flush_log();
		fh = Fcreate("SLPROBE.ADR", 0);
		if (fh >= 0) {
			char a[16];
			int n = snprintf(a, sizeof(a), "%08lx\n", (unsigned long)(uintptr_t)&shot_n);

			Fwrite((short)fh, (long)n, a);
			Fclose((short)fh);
		}
		/* wait (at most 60 s) until the host has set the breakpoint; the
		 * window's first WM_REDRAW waits in the queue meanwhile */
		for (i = 0; i < 600 && !file_exists("SLPROBE.GO"); i++) {
			int_in[0] = 100;
			int_in[1] = 0;
			aes(24, 2, 1, 0);	/* evnt_timer(100 ms) */
		}
		emit("G02 go after %d polls\n", i);
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

	for (k = 1; !quit && k < 30000; k++) {
		uint32_t t1;
		int16_t which;
		int rep;

		for (i = 0; i < 16; i++)
			int_in[i] = 0;
		int_in[0] = MU_BUTTON | MU_MESAG;
		int_in[1] = 2;			/* the desktop's (2, 1, 1), as mgprobe.c */
		int_in[2] = 1;
		int_in[3] = 1;
		for (i = 0; i < 8; i++)
			msg[i] = 0;
		addr_in[0] = msg;
		aes(25, 16, 7, 1);		/* evnt_multi */
		t1 = ticks();
		which = int_out[0];
		/* a WM_ARROWED equal to the previous one (msg[0], [3], [4]; EmuTOS
		 * repeats it while the button is held, gemctrl.c:111-139) is
		 * counted, not logged: the run ends in one X line */
		rep = which == MU_MESAG && msg[0] == WM_ARROWED && last_arrow[0] == WM_ARROWED &&
		      msg[3] == last_arrow[3] && msg[4] == last_arrow[4];
		if (rep) {
			nrep++;
			last_rep_t = t1;
			last_rep_m1 = msg[1];
		} else {
			if (nrep) {
				emit("X%03d %d more equal WM_ARROWED, last t=%lu msg[1]=%d, position now v %d h %d\n",
				     k, nrep, (unsigned long)last_rep_t, last_rep_m1, vpos, hpos);
				nrep = 0;
			}
			emit("R%03d t=%lu which=%04x mx=%d my=%d bs=%d br=%d", k, (unsigned long)t1,
			     (uint16_t)which, int_out[1], int_out[2], int_out[3], int_out[6]);
			if (which & MU_MESAG)
				emit(" msg=%d,%d,%d,%d,%d,%d,%d,%d", msg[0], msg[1], msg[2], msg[3], msg[4],
				     msg[5], msg[6], msg[7]);
			emit("\n");
		}
		for (i = 0; i < 8; i++)
			last_arrow[i] = (which & MU_MESAG) ? msg[i] : 0;
		if (which & MU_MESAG) {
			int16_t m[8];

			for (i = 0; i < 8; i++)
				m[i] = msg[i];
			switch (m[0]) {
			case WM_REDRAW:
				if (m[3] == wh)
					redraw(k, m);
				break;
			case WM_VSLID:
				vpos = m[4];
				emit("V%03d wind_set(VSLIDE,%d)=%d\n", k, vpos, wset(WF_VSLIDE, vpos));
				break;
			case WM_HSLID:
				hpos = m[4];
				emit("V%03d wind_set(HSLIDE,%d)=%d\n", k, hpos, wset(WF_HSLIDE, hpos));
				break;
			case WM_ARROWED: {
				static const int16_t dv[8] = { -250, 250, -50, 50, 0, 0, 0, 0 };
				static const int16_t dh[8] = { 0, 0, 0, 0, -250, 250, -50, 50 };
				int a = m[4] & 7;

				int16_t r;

				if (dv[a]) {
					vpos = clamp_pos(vpos + dv[a]);
					r = wset(WF_VSLIDE, vpos);
					if (!rep)
						emit("V%03d wind_set(VSLIDE,%d)=%d\n", k, vpos, r);
				} else {
					hpos = clamp_pos(hpos + dh[a]);
					r = wset(WF_HSLIDE, hpos);
					if (!rep)
						emit("V%03d wind_set(HSLIDE,%d)=%d\n", k, hpos, r);
				}
				break;
			}
			default:
				break;
			}
		}
		if ((which & MU_BUTTON) && !(which & MU_MESAG)) {
			int16_t mx = int_out[1], my = int_out[2];

			if (mx >= wx && mx < wx + ww && my >= wy && my < wy + whh) {
				int col = (mx - wx) * 4 / ww;

				if (col >= 3)
					quit = 1;
				else
					config(col);
			}
		}
		if (!rep)
			flush_log();
	}
	if (nrep)
		emit("X%03d %d more equal WM_ARROWED, last t=%lu msg[1]=%d\n", k, nrep, (unsigned long)last_rep_t,
		     last_rep_m1);

#ifndef __NEWTOS__
	Supexec(disarm);
	if ((void *)Setexc(0x100, -1L) == (void *)inj_timer)
		Setexc(0x100, (long)inj_old);
	for (i = 0; i < inj_next; i++)
		emit("J%03d t=%lu +%lu pkt=%02x %02x %02x\n", i, (unsigned long)inj_at[i],
		     (unsigned long)(inj_at[i] - inj_t0), st_pkt[i][0], st_pkt[i][1], st_pkt[i][2]);
	for (i = 0; i < shot_next; i++)
		emit("K%03d t=%lu +%lu (sched +%lu)\n", i, (unsigned long)shot_at[i],
		     (unsigned long)(shot_at[i] - inj_t0), (unsigned long)sh_tick[i]);
#endif
	emit("Z99 end quit=%d\n", quit);
	flush_log();
	int_in[0] = wh;
	aes(102, 1, 1, 0);			/* wind_close */
	int_in[0] = wh;
	aes(103, 1, 1, 0);			/* wind_delete */
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
