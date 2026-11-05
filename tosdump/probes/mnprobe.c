/*
 * mnprobe.c -- how long a drop-down menu selection takes from the left-button
 * edge until the application's evnt_multi returns MN_SELECTED (TASK/A127 D1).
 * A permanent probe of the tosdump tree like akprobe.c / sbprobe.c: it needs
 * an own menu bar and timed mouse input while the program waits, which no
 * case table can give; it writes a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS): the AES
 * calls go through the tosdump ABI backends (abi/m68k_trap.c,
 * abi/newtos_svc.c, td_abi_aes), BIOS / XBIOS / GEMDOS through mintlib's
 * osbind.h.  The OBJECT below is the native one of each side (`long ob_spec`:
 * 24-byte m68k record, 32-byte LP64 record), as in sbprobe.c.  Built and run
 * by tosdump/probes/run-mnprobe.sh.  Output: MNPROBE.LOG in the current
 * directory, rewritten after every evnt_multi return.
 *
 * INPUT.  One script, tosdump/probes/mnprobe-mouse.txt, lines
 * `<ticks> 0xF8|buttons dx dy` (IKBD relative mouse packets, bit 1 = left):
 *   NewTOS  tools/inject-arm.py launch:MNPROBE feeds the bytes to the IKBD
 *           decoder (kernel/src/ikbd.rs feed) at <ticks> after the launch;
 *           the kernel prints "[inj] ... feed 0x.. at hz_200 <n>".
 *   EmuTOS  Hatari 2.6.1 cannot inject a left button or a mouse motion through
 *           --cmd-fifo (control.c of 2.6.1: doubleclick, rightdown, rightup,
 *           keypress/keydown/keyup only).  The probe therefore feeds the same
 *           packets itself: an etv_timer hook (Setexc 0x100, chained, every
 *           4th Timer-C tick = 20 ms) passes each due packet to the IKBD
 *           mouse vector Kbdvbase()->mousevec with a0 = packet, exactly as
 *           EmuTOS's own call_mousevec does (bios/aciavecs.S:538-553; EmuTOS
 *           itself calls it from Timer C for the Alt-arrow mouse emulation's
 *           repeat, bios/ikbd.c:518-523).  It records _hz_200 at each feed
 *           (J-lines).  The script file is C:\MNPROBE.INJ (same bytes as
 *           mnprobe-mouse.txt); <ticks> count from the moment the hook is
 *           armed, right before the first wait.
 *
 * WAITS (W/R lines; the mode changes after every 5th MN_SELECTED):
 *   seg 0  mode D2: evnt_multi(MU_KEYBD|MU_BUTTON|MU_MESAG, 2, 1, 1, ...) --
 *          the desktop's blocking wait on both sides (EmuTOS
 *          desk/deskmain.c:2078,2088; programs/desktop/src/lib.rs
 *          evnt_multi_msg, main.rs event_loop); script: click-select Item A
 *   seg 1  mode M:  evnt_multi(MU_MESAG, ...); click-select Item B
 *   seg 2  mode D2; drag-select Item C (press on the title, release on it)
 *   seg 3  mode M;  drag-select Item D
 *   seg 4  mode D1: evnt_multi(MU_KEYBD|MU_BUTTON|MU_MESAG, 1, 1, 1, ...);
 *          click-select Item A
 * With C:\MNPROBE.MFS present the order is M, D2, M, D2, D1 (same script):
 * the message-only wait measured before the program has ever waited for a
 * double click.  EmuTOS raises gl_bpend for a wait with bclicks > 1 and
 * lowers it only when that wait is satisfied by a button (aes/geminput.c:
 * 477-479, 559-561), so after a D2 wait that ended with a message every later
 * press opens the double-click window, whatever the program waits for.
 * After each MN_SELECTED: menu_tnormal(title, 1).  After 25: Z99 end.
 *
 * Lines:
 *   H00  graf_handle (wchar hchar wbox hbox), evnt_dclick(0, 0)
 *   O<n> objc_offset of the File title and items (geometry check)
 *   T00  _hz_200 when the input clock starts (EmuTOS: the hook armed)
 *   W<k> mode, _hz_200 before evnt_multi k
 *   R<k> mode, _hz_200 after it returned, which, msg[0] msg[3] msg[4],
 *        mx my button bret
 *   J<n> (EmuTOS only) _hz_200 at the feed of script packet n, its bytes
 *   V<n> (EmuTOS only) every packet the mouse vector got, _hz_200 and bytes: a
 *        passive observer in front of Kbdvbase()->mousevec.  It also sees
 *        packets that did not come from the feeder -- the --cmd-fifo
 *        cross-check of run-mnprobe.sh (MNPROBE_FIFO) drives the pointer and
 *        the left button through EmuTOS's Alt-arrow / Alt-Insert mouse
 *        emulation (bios/ikbd.c:285-389), i.e. through the IKBD interrupt.
 *   On EmuTOS the probe creates MNPROBE.RDY when the input clock starts.
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
#define G_IBOX    25
#define G_STRING  28
#define G_TITLE   32
#define LASTOB    0x0020
#define DISABLED  0x0008

#define MU_KEYBD  0x0001
#define MU_BUTTON 0x0002
#define MU_MESAG  0x0010
#define MN_SELECTED 10

static char out[131072];
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
	long fh = Fcreate("MNPROBE.LOG", 0);

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

/* ---- the menu tree -------------------------------------------------------- */

static char s_desk[] = " Desk ";
static char s_file[] = " File ";
static char s_about[] = "  About MNPROBE ";
static char s_sep[] = "----------------";
static char s_acc[6][20] = { "  Desk Accessory ", "  Desk Accessory ", "  Desk Accessory ",
			     "  Desk Accessory ", "  Desk Accessory ", "  Desk Accessory " };
static char s_ia[] = "  Item A  ";
static char s_ib[] = "  Item B  ";
static char s_ic[] = "  Item C  ";
static char s_id[] = "  Item D  ";

#define NOBJ 20
static OBJ tree[NOBJ];

static void obj(int i, int next, int head, int tail, int type, int flags, int state,
		long spec, int x, int y, int w, int h)
{
	tree[i].ob_next = (int16_t)next;
	tree[i].ob_head = (int16_t)head;
	tree[i].ob_tail = (int16_t)tail;
	tree[i].ob_type = (uint16_t)type;
	tree[i].ob_flags = (uint16_t)flags;
	tree[i].ob_state = (uint16_t)state;
	tree[i].ob_spec = spec;
	tree[i].ob_x = (int16_t)x;
	tree[i].ob_y = (int16_t)y;
	tree[i].ob_width = (int16_t)w;
	tree[i].ob_height = (int16_t)h;
}

/*
 * The usual RCS menu layout in pixels (the pointer script uses these):
 *   0 ROOT IBOX, 1 THEBAR, 2 THEACTIVE (x 16..111), 3 " Desk " (16..63),
 *   4 " File " (64..111, y 0..18), 5 the screen IBOX at y 19, 6 the desk
 *   box (About, separator, 6 accessory slots), 15 the File box at x 64,
 *   16..19 Item A..D, 16 px each: y 19..34, 35..50, 51..66, 67..82.
 */
static void build_tree(void)
{
	int i;

	obj(0, -1, 1, 5, G_IBOX, 0, 0, 0L, 0, 0, 640, 480);
	obj(1, 5, 2, 2, G_BOX, 0, 0, 0x00001100L, 0, 0, 640, 19);
	obj(2, 1, 3, 4, G_IBOX, 0, 0, 0L, 16, 0, 96, 19);
	obj(3, 4, -1, -1, G_TITLE, 0, 0, (long)s_desk, 0, 0, 48, 19);
	obj(4, 2, -1, -1, G_TITLE, 0, 0, (long)s_file, 48, 0, 48, 19);
	obj(5, 0, 6, 15, G_IBOX, 0, 0, 0L, 0, 19, 640, 461);
	obj(6, 15, 7, 14, G_BOX, 0, 0, 0x00FF1100L, 16, 0, 160, 128);
	obj(7, 8, -1, -1, G_STRING, 0, 0, (long)s_about, 0, 0, 160, 16);
	obj(8, 9, -1, -1, G_STRING, 0, DISABLED, (long)s_sep, 0, 16, 160, 16);
	for (i = 0; i < 6; i++)
		obj(9 + i, i < 5 ? 10 + i : 6, -1, -1, G_STRING, 0, 0, (long)s_acc[i], 0, 32 + 16 * i, 160, 16);
	obj(15, 5, 16, 19, G_BOX, 0, 0, 0x00FF1100L, 64, 0, 80, 64);
	obj(16, 17, -1, -1, G_STRING, 0, 0, (long)s_ia, 0, 0, 80, 16);
	obj(17, 18, -1, -1, G_STRING, 0, 0, (long)s_ib, 0, 16, 80, 16);
	obj(18, 19, -1, -1, G_STRING, 0, 0, (long)s_ic, 0, 32, 80, 16);
	obj(19, 15, -1, -1, G_STRING, LASTOB, 0, (long)s_id, 0, 48, 80, 16);
}

/* ---- EmuTOS: the in-probe IKBD mouse feeder -------------------------------- */

#ifndef __NEWTOS__
#define NSTEP 256
static uint32_t st_tick[NSTEP];
static unsigned char st_pkt[NSTEP][3];
static int nstep;

volatile uint32_t inj_t0 __asm__("mnp_inj_t0");
volatile int inj_next __asm__("mnp_inj_next");
volatile int inj_armed __asm__("mnp_inj_armed");
void *volatile inj_old __asm__("mnp_inj_old");
static long *kvecs;			/* Kbdvbase(): midivec vkbderr vmiderr statvec mousevec ... */
static uint32_t inj_at[NSTEP];
static unsigned char inj_buf[3];

/* The passive mousevec observer: every packet the mouse vector gets (from
 * the feeder above or from the IKBD path), _hz_200 and its 3 bytes. */
#define NOBS 2048
volatile int obs_n __asm__("mnp_obs_n");
volatile uint32_t obs_t[NOBS] __asm__("mnp_obs_t");
volatile unsigned char obs_p[NOBS][4] __asm__("mnp_obs_p");
void *volatile obs_old __asm__("mnp_obs_old");
void obs_hook(void) __asm__("mnp_obs_hook");

void inj_timer(void) __asm__("mnp_inj_timer");
void inj_mousevec(void *fn, unsigned char *pkt) __asm__("mnp_inj_mousevec");
void inj_step(void) __asm__("mnp_inj_step");

/*
 * mnp_inj_timer: the etv_timer hook, called with jsr and the timer_ms word on
 * the stack (bios/vectors.S:540-544); runs mnp_inj_step and continues in the
 * previous handler with the stack unchanged.  mnp_inj_mousevec is
 * call_mousevec (bios/aciavecs.S:538-553) with the vector passed in.
 */
__asm__(
"	.text\n"
"	.even\n"
"mnp_inj_timer:\n"
"	movem.l	d0-d7/a0-a6,-(sp)\n"
"	jsr	mnp_inj_step\n"
"	movem.l	(sp)+,d0-d7/a0-a6\n"
"	move.l	mnp_inj_old,-(sp)\n"
"	rts\n"
"mnp_obs_hook:\n"			/* a0 = packet; record, then the previous vector */
"	movem.l	d0-d1/a1,-(sp)\n"
"	move.l	mnp_obs_n,d0\n"
"	cmp.l	#2048,d0\n"
"	bge.s	1f\n"
"	move.l	d0,d1\n"
"	lsl.l	#2,d1\n"
"	lea	mnp_obs_t,a1\n"
"	move.l	0x4ba.w,0(a1,d1.l)\n"
"	lea	mnp_obs_p,a1\n"
"	add.l	d1,a1\n"
"	move.b	(a0),(a1)+\n"
"	move.b	1(a0),(a1)+\n"
"	move.b	2(a0),(a1)\n"
"	addq.l	#1,d0\n"
"	move.l	d0,mnp_obs_n\n"
"1:	movem.l	(sp)+,d0-d1/a1\n"
"	move.l	mnp_obs_old,-(sp)\n"
"	rts\n"
"mnp_inj_mousevec:\n"
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
}

static long arm(void)
{
	inj_next = 0;
	inj_t0 = *(volatile uint32_t *)0x4BAUL;
	inj_armed = 1;
	return (long)inj_t0;
}

static long disarm(void)
{
	inj_armed = 0;
	return 0;
}

static int vdone;

/* the observer's records not logged yet */
static void drain_obs(void)
{
	while (vdone < obs_n && vdone < NOBS) {
		emit("V%03d t=%lu pkt=%02x %02x %02x\n", vdone, (unsigned long)obs_t[vdone], obs_p[vdone][0],
		     obs_p[vdone][1], obs_p[vdone][2]);
		vdone++;
	}
}

static long obs_install(void)
{
	obs_n = 0;
	obs_old = (void *)kvecs[4];
	kvecs[4] = (long)obs_hook;
	return 0;
}

static long obs_remove(void)
{
	if (kvecs[4] == (long)obs_hook)
		kvecs[4] = (long)obs_old;
	return 0;
}

static int hexnum(const char **p, long *v)
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

static char script[16384];

/* C:\MNPROBE.INJ, the lines `<ticks> <b0> <b1> <b2>`; # comments. */
static int load_script(void)
{
	long fh = Fopen("MNPROBE.INJ", 0), n;
	const char *p, *end;

	if (fh < 0)
		return -1;
	n = Fread((short)fh, (long)sizeof(script) - 1, script);
	Fclose((short)fh);
	if (n <= 0)
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
		while (k < 4 && p < eol && *p != '#' && *p != '\r' && hexnum(&p, &v[k]))
			k++;
		if (k == 4 && nstep < NSTEP) {
			st_tick[nstep] = (uint32_t)v[0];
			st_pkt[nstep][0] = (unsigned char)v[1];
			st_pkt[nstep][1] = (unsigned char)v[2];
			st_pkt[nstep][2] = (unsigned char)v[3];
			nstep++;
		} else if (k != 0) {
			return -3;
		}
		p = eol + 1;
	}
	return nstep;
}
#endif

/* ---- the waits -------------------------------------------------------------- */

static int16_t msg[8];

static const char *mode_name(int m)
{
	return m == 0 ? "D2" : m == 1 ? "M " : "D1";
}

/* seg -> mode: D2, M, D2, M, D1; with C:\MNPROBE.MFS present M, D2, M, D2, D1
 * (the message-only wait before the program ever asked for a double click) */
static int seg_mode[5] = { 0, 1, 0, 1, 2 };

static void set_wait(int m)
{
	int i;

	for (i = 0; i < 16; i++)
		int_in[i] = 0;
	if (m == 1) {
		int_in[0] = MU_MESAG;
		int_in[1] = 2;		/* unused without MU_BUTTON; the desktop's values */
		int_in[2] = 1;
		int_in[3] = 1;
	} else {
		int_in[0] = MU_KEYBD | MU_BUTTON | MU_MESAG;
		int_in[1] = m == 0 ? 2 : 1;
		int_in[2] = 1;
		int_in[3] = 1;
	}
	addr_in[0] = msg;
}

int main(void)
{
	int16_t ap_id = aes(10, 0, 1, 0);	/* appl_init */
	int k, nsel = 0, i;
	uint32_t t;

	if (ap_id < 0) {
		emit("A99 appl_init=%d\n", ap_id);
		flush_log();
		return 1;
	}
	aes(77, 0, 5, 0);			/* graf_handle */
	emit("H00 graf_handle %d %d %d %d", int_out[1], int_out[2], int_out[3], int_out[4]);
	int_in[0] = 0;
	int_in[1] = 0;
	aes(26, 2, 1, 0);			/* evnt_dclick(0, 0): get */
	emit(" evnt_dclick %d\n", int_out[0]);
	int_in[0] = 0;
	addr_in[0] = 0;
	aes(78, 1, 1, 1);			/* graf_mouse(ARROW) */

	{
		long fh = Fopen("MNPROBE.MFS", 0);

		if (fh >= 0) {
			Fclose((short)fh);
			seg_mode[0] = 1;
			seg_mode[1] = 0;
			seg_mode[2] = 1;
			seg_mode[3] = 0;
		}
		emit("M01 order %s\n", seg_mode[0] == 1 ? "M-first" : "D2-first");
	}
	build_tree();
	int_in[0] = 1;
	addr_in[0] = tree;
	aes(30, 1, 1, 1);			/* menu_bar(tree, 1) */
	emit("M00 menu_bar=%d desk box h=%d\n", int_out[0], tree[6].ob_height);
	for (i = 4; i < NOBJ; i++) {
		if (i >= 5 && i < 16)
			continue;
		int_in[0] = (int16_t)i;
		addr_in[0] = tree;
		aes(44, 1, 3, 1);		/* objc_offset */
		emit("O%02d %d,%d %dx%d\n", i, int_out[1], int_out[2], tree[i].ob_width, tree[i].ob_height);
	}

#ifndef __NEWTOS__
	{
		int r = load_script();

		emit("S00 script steps %d\n", r);
		if (r < 0) {
			flush_log();
			aes(19, 0, 1, 0);
			return 1;
		}
		kvecs = (long *)Kbdvbase();
		Supexec(obs_install);
		inj_old = (void *)Setexc(0x100, -1L);
		Setexc(0x100, (long)inj_timer);
		t = (uint32_t)Supexec(arm);
	}
#else
	t = ticks();
#endif
	emit("T00 t=%lu\n", (unsigned long)t);
	flush_log();
#ifndef __NEWTOS__
	{
		long fh = Fcreate("MNPROBE.RDY", 0);	/* for the --cmd-fifo cross-check */

		if (fh >= 0)
			Fclose((short)fh);
	}
#endif

	for (k = 1; nsel < 25 && k < 400; k++) {
		int m = seg_mode[nsel / 5];
		uint32_t t0, t1;

		set_wait(m);
		t0 = ticks();
		aes(25, 16, 7, 1);		/* evnt_multi */
		t1 = ticks();
		emit("W%03d %s t=%lu\n", k, mode_name(m), (unsigned long)t0);
		emit("R%03d %s t=%lu which=%04x msg=%d,%d,%d mx=%d my=%d bs=%d br=%d\n", k, mode_name(m),
		     (unsigned long)t1, (uint16_t)int_out[0],
		     (int_out[0] & MU_MESAG) ? msg[0] : -1, (int_out[0] & MU_MESAG) ? msg[3] : -1,
		     (int_out[0] & MU_MESAG) ? msg[4] : -1, int_out[1], int_out[2], int_out[3], int_out[6]);
		if ((int_out[0] & MU_MESAG) && msg[0] == MN_SELECTED) {
			int16_t title = msg[3];

			nsel++;
			int_in[0] = title;
			int_in[1] = 1;
			addr_in[0] = tree;
			aes(33, 2, 1, 1);	/* menu_tnormal(title, 1) */
		}
#ifndef __NEWTOS__
		drain_obs();
#endif
		flush_log();
	}

#ifndef __NEWTOS__
	Supexec(disarm);
	if ((void *)Setexc(0x100, -1L) == (void *)inj_timer)
		Setexc(0x100, (long)inj_old);
	Supexec(obs_remove);
	drain_obs();
	for (i = 0; i < inj_next; i++)
		emit("J%03d t=%lu +%lu pkt=%02x %02x %02x\n", i, (unsigned long)inj_at[i],
		     (unsigned long)(inj_at[i] - inj_t0), st_pkt[i][0], st_pkt[i][1], st_pkt[i][2]);
#endif
	emit("Z99 end sel=%d\n", nsel);
	flush_log();
	int_in[0] = 0;
	addr_in[0] = tree;
	aes(30, 1, 1, 1);			/* menu_bar(tree, 0) */
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
