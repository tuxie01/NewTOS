/*
 * mgprobe.c -- what a drop-down menu shows while the pointer is in a border or
 * a gap of the open box, and after the pointer left the box and came back
 * (TASK/A127 D2/D3, GitHub #19).  A sibling of mnprobe.c (same feeder, same
 * ABI backends, same tree style); the evidence is the screen, captured from
 * outside after every pointer step, so the probe itself only provides the
 * menu bar, the wait and the input clock.  Built and run by
 * tosdump/probes/run-mgprobe.sh.  Output: MGPROBE.LOG in the current
 * directory, rewritten after every evnt_multi return.
 *
 * MENU (mgprobe.c build_tree; pixels, the usual RCS menu layout):
 *   titles " Desk " x 16..63, " File " x 64..111, " Gaps " x 112..159
 *   File box  x 64..159, y 19..98: Item A, Item B, a DISABLED separator,
 *             Item C, Quit -- 16 px each, contiguous (no gap)
 *   Gaps box  x 112..223, y 19..114: G1 (y 19..34), G2 (35..50), an inner gap
 *             (51..58), a DISABLED separator (59..74), G4 (75..90), G5 (91..106,
 *             x 112..207: a right gap x 208..223), a bottom gap (107..114)
 *   The gaps are points inside the box that no child covers: EmuTOS
 *   aes/gemmnlib.c:422 (ob_find(tree, cur_menu, 1, x, y)) answers the box
 *   itself there.
 *
 * INPUT.  tosdump/probes/mgprobe-mouse.txt, lines `<ticks> 0xF8|buttons dx dy`
 * (IKBD relative mouse packets, bit 1 = left), and mgprobe-shots.txt, lines
 * `<ticks> <name>` (the screenshot schedule):
 *   NewTOS  tools/inject-arm.py launch:MGPROBE feeds the packets to the IKBD
 *           decoder (kernel/src/ikbd.rs feed) at <ticks> after the launch;
 *           run-mgprobe.sh takes QMP screendumps at <shot ticks> after the
 *           "[inj] launch:MGPROBE armed" line (wall clock = QEMU's clock).
 *   EmuTOS  the probe feeds the packets itself (mnprobe.c's etv_timer feeder:
 *           Kbdvbase()->mousevec with a0 = packet, as call_mousevec,
 *           bios/aciavecs.S:538-553), C:\MGPROBE.INJ = mgprobe-mouse.txt.  At
 *           each shot tick of C:\MGPROBE.SHT the feeder increments the long
 *           mgp_shot_n; run-mgprobe.sh has set a Hatari tracking breakpoint
 *           on that long (address from C:\MGPROBE.ADR) whose command file takes
 *           the screenshot -- so the oracle's shots fall on the exact emulated
 *           tick.  The probe arms the feeder only after C:\MGPROBE.GO exists
 *           (the breakpoint is set).
 *
 * WAIT: the desktop's, evnt_multi(MU_KEYBD|MU_BUTTON|MU_MESAG, 2, 1, 1, ...)
 * (references/emutos/desk/deskmain.c:2078,2088; mnprobe.c "D2").  After each
 * MN_SELECTED: menu_tnormal(title, 1); MN_SELECTED for Quit ends the probe.
 *
 * Lines: H00 graf_handle + evnt_dclick, M00 menu_bar, O<n> objc_offset,
 * S00 script/shot counts, A00 the shot counter's address (EmuTOS), T00 input
 * clock start, W/R per evnt_multi (R: which, msg[0] msg[3] msg[4], mouse,
 * button), J<n> (EmuTOS) feed of packet n, K<n> (EmuTOS) shot n, Z99 end.
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
	long fh = Fcreate("MGPROBE.LOG", 0);

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
static char s_gaps[] = " Gaps ";
static char s_about[] = "  About MGPROBE ";
static char s_sep[] = "----------------";
static char s_acc[6][20] = { "  Desk Accessory ", "  Desk Accessory ", "  Desk Accessory ",
			     "  Desk Accessory ", "  Desk Accessory ", "  Desk Accessory " };
static char s_ia[] = "  Item A    ";
static char s_ib[] = "  Item B    ";
static char s_fsep[] = "------------";
static char s_ic[] = "  Item C    ";
static char s_quit[] = "  Quit      ";
static char s_g1[] = "  Gap item 1  ";
static char s_g2[] = "  Gap item 2  ";
static char s_gsep[] = "--------------";
static char s_g4[] = "  Gap item 4  ";
static char s_g5[] = "  Gap it 5  ";

#define NOBJ 28
#define T_FILE 4
#define O_QUIT 21
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
 *   0 ROOT IBOX, 1 THEBAR, 2 THEACTIVE (x 16..159), 3 Desk, 4 File, 5 Gaps,
 *   6 the screen IBOX at y 19, 7 the desk box (About, separator, 6 accessory
 *   slots, 8..15), 16 the File box (17 Item A, 18 Item B, 19 separator,
 *   20 Item C, 21 Quit), 22 the Gaps box (23 G1, 24 G2, 25 separator, 26 G4,
 *   27 G5).
 */
static void build_tree(void)
{
	int i;

	obj(0, -1, 1, 6, G_IBOX, 0, 0, 0L, 0, 0, 640, 480);
	obj(1, 6, 2, 2, G_BOX, 0, 0, 0x00001100L, 0, 0, 640, 19);
	obj(2, 1, 3, 5, G_IBOX, 0, 0, 0L, 16, 0, 144, 19);
	obj(3, 4, -1, -1, G_TITLE, 0, 0, (long)s_desk, 0, 0, 48, 19);
	obj(4, 5, -1, -1, G_TITLE, 0, 0, (long)s_file, 48, 0, 48, 19);
	obj(5, 2, -1, -1, G_TITLE, 0, 0, (long)s_gaps, 96, 0, 48, 19);
	obj(6, 0, 7, 22, G_IBOX, 0, 0, 0L, 0, 19, 640, 461);
	obj(7, 16, 8, 15, G_BOX, 0, 0, 0x00FF1100L, 16, 0, 160, 128);
	obj(8, 9, -1, -1, G_STRING, 0, 0, (long)s_about, 0, 0, 160, 16);
	obj(9, 10, -1, -1, G_STRING, 0, DISABLED, (long)s_sep, 0, 16, 160, 16);
	for (i = 0; i < 6; i++)
		obj(10 + i, i < 5 ? 11 + i : 7, -1, -1, G_STRING, 0, 0, (long)s_acc[i], 0, 32 + 16 * i, 160, 16);
	obj(16, 22, 17, 21, G_BOX, 0, 0, 0x00FF1100L, 64, 0, 96, 80);
	obj(17, 18, -1, -1, G_STRING, 0, 0, (long)s_ia, 0, 0, 96, 16);
	obj(18, 19, -1, -1, G_STRING, 0, 0, (long)s_ib, 0, 16, 96, 16);
	obj(19, 20, -1, -1, G_STRING, 0, DISABLED, (long)s_fsep, 0, 32, 96, 16);
	obj(20, 21, -1, -1, G_STRING, 0, 0, (long)s_ic, 0, 48, 96, 16);
	obj(21, 16, -1, -1, G_STRING, 0, 0, (long)s_quit, 0, 64, 96, 16);
	obj(22, 6, 23, 27, G_BOX, 0, 0, 0x00FF1100L, 112, 0, 112, 96);
	obj(23, 24, -1, -1, G_STRING, 0, 0, (long)s_g1, 0, 0, 112, 16);
	obj(24, 25, -1, -1, G_STRING, 0, 0, (long)s_g2, 0, 16, 112, 16);
	obj(25, 26, -1, -1, G_STRING, 0, DISABLED, (long)s_gsep, 0, 40, 112, 16);
	obj(26, 27, -1, -1, G_STRING, 0, 0, (long)s_g4, 0, 56, 112, 16);
	obj(27, 22, -1, -1, G_STRING, LASTOB, 0, (long)s_g5, 0, 72, 96, 16);
}

/* ---- EmuTOS: the in-probe IKBD mouse feeder (mnprobe.c's) + shot counter ---- */

#ifndef __NEWTOS__
#define NSTEP 1024
#define NSHOT 128
static uint32_t st_tick[NSTEP];
static unsigned char st_pkt[NSTEP][3];
static int nstep;
static uint32_t sh_tick[NSHOT];
static int nshot;

volatile uint32_t inj_t0 __asm__("mgp_inj_t0");
volatile int inj_next __asm__("mgp_inj_next");
volatile int inj_armed __asm__("mgp_inj_armed");
void *volatile inj_old __asm__("mgp_inj_old");
volatile int shot_next __asm__("mgp_shot_next");
/* the Hatari breakpoint's target: changes exactly once per shot tick */
volatile uint32_t shot_n __asm__("mgp_shot_n");
static long *kvecs;			/* Kbdvbase(): ... mousevec at [4] */
static uint32_t inj_at[NSTEP];
static uint32_t shot_at[NSHOT];
static unsigned char inj_buf[3];

void inj_timer(void) __asm__("mgp_inj_timer");
void inj_mousevec(void *fn, unsigned char *pkt) __asm__("mgp_inj_mousevec");
void inj_step(void) __asm__("mgp_inj_step");

__asm__(
"	.text\n"
"	.even\n"
"mgp_inj_timer:\n"
"	movem.l	d0-d7/a0-a6,-(sp)\n"
"	jsr	mgp_inj_step\n"
"	movem.l	(sp)+,d0-d7/a0-a6\n"
"	move.l	mgp_inj_old,-(sp)\n"
"	rts\n"
"mgp_inj_mousevec:\n"
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

static char script[32768];

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

int main(void)
{
	int16_t ap_id = aes(10, 0, 1, 0);	/* appl_init */
	int k, i, quit = 0;
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

	build_tree();
	int_in[0] = 1;
	addr_in[0] = tree;
	aes(30, 1, 1, 1);			/* menu_bar(tree, 1) */
	emit("M00 menu_bar=%d desk box h=%d\n", int_out[0], tree[7].ob_height);
	for (i = 3; i < NOBJ; i++) {
		if (i >= 6 && i < 16)
			continue;
		int_in[0] = (int16_t)i;
		addr_in[0] = tree;
		aes(44, 1, 3, 1);		/* objc_offset */
		emit("O%02d %d,%d %dx%d state %04x\n", i, int_out[1], int_out[2], tree[i].ob_width,
		     tree[i].ob_height, tree[i].ob_state);
	}

#ifndef __NEWTOS__
	{
		int r = load("MGPROBE.INJ", 4), s = load("MGPROBE.SHT", 1);
		long fh;

		emit("S00 script steps %d shots %d\n", r, s);
		if (r < 0 || s < 0) {
			flush_log();
			aes(19, 0, 1, 0);
			return 1;
		}
		emit("A00 shot counter at %08lx\n", (unsigned long)(uintptr_t)&shot_n);
		flush_log();
		fh = Fcreate("MGPROBE.ADR", 0);
		if (fh >= 0) {
			char a[16];
			int n = snprintf(a, sizeof(a), "%08lx\n", (unsigned long)(uintptr_t)&shot_n);

			Fwrite((short)fh, (long)n, a);
			Fclose((short)fh);
		}
		/* wait (at most 60 s) until the host has set the breakpoint */
		for (i = 0; i < 600 && !file_exists("MGPROBE.GO"); i++) {
			int_in[0] = 100;
			int_in[1] = 0;
			aes(24, 2, 1, 0);	/* evnt_timer(100 ms) */
		}
		emit("G00 go after %d polls\n", i);
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

	for (k = 1; !quit && k < 400; k++) {
		uint32_t t0, t1;

		for (i = 0; i < 16; i++)
			int_in[i] = 0;
		int_in[0] = MU_KEYBD | MU_BUTTON | MU_MESAG;
		int_in[1] = 2;
		int_in[2] = 1;
		int_in[3] = 1;
		addr_in[0] = msg;
		t0 = ticks();
		aes(25, 16, 7, 1);		/* evnt_multi */
		t1 = ticks();
		emit("W%03d t=%lu\n", k, (unsigned long)t0);
		emit("R%03d t=%lu which=%04x msg=%d,%d,%d mx=%d my=%d bs=%d br=%d\n", k,
		     (unsigned long)t1, (uint16_t)int_out[0],
		     (int_out[0] & MU_MESAG) ? msg[0] : -1, (int_out[0] & MU_MESAG) ? msg[3] : -1,
		     (int_out[0] & MU_MESAG) ? msg[4] : -1, int_out[1], int_out[2], int_out[3], int_out[6]);
		if ((int_out[0] & MU_MESAG) && msg[0] == MN_SELECTED) {
			int16_t title = msg[3];

			if (title == T_FILE && msg[4] == O_QUIT)
				quit = 1;
			int_in[0] = title;
			int_in[1] = 1;
			addr_in[0] = tree;
			aes(33, 2, 1, 1);	/* menu_tnormal(title, 1) */
		}
		flush_log();
	}

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
	int_in[0] = 0;
	addr_in[0] = tree;
	aes(30, 1, 1, 1);			/* menu_bar(tree, 0) */
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
