/*
 * wkprobe.c -- the VDI workstation and the mouse pointer, compared pixel for
 * pixel and value for value (TASK/V18 part 3: the TOOL32 candidates K9/K24,
 * K21, K22).  A permanent probe of the tosdump tree like vdprobe.c, whose
 * read-back it copies: each case draws on a cleared screen and reads the
 * affected rectangle back with vro_cpyfm (opcode 109, S_ONLY, screen -> a
 * 16-plane device-form memory MFDB).  DUMPVDI cannot express these cases: they
 * need the pointer on the screen, mouse packets between two VDI calls, and
 * v_opnwk / v_clswk, after which EmuTOS has closed the dumper's own workstation.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari profile falcontc =
 * 640x480 16-bit truecolor, AArch64 NewTOS = the Falcon truecolor workstation
 * of ADR-024).  VDI and AES calls through the tosdump ABI backends
 * (abi/m68k_trap.c, abi/newtos_svc.c); pointers in CONTRL[7..] (the MFDBs, the
 * vex_* vectors) packed as tosdump/core/runner_rast.c packs them: pointer n at
 * CONTRL[7 + n * sizeof(void *) / 2].  Built and run by
 * tosdump/probes/run-wkprobe.sh; compared by tosdump/probes/wkprobe-compare.py.
 *
 * MOUSE PACKETS.  Six relative IKBD packets, in this order (wkprobe-mouse.txt):
 *   M1 0xF8 +20 +8   M2 0xF8 +30 0   M3 0xF8 +10 +10   W1 0xFA 0 0 (left down)
 *   W1 0xF8 0 0 (left up)            C1 0xF8 +40 +20
 *   EmuTOS  the probe feeds each packet itself, in a Supexec callback at IPL 7,
 *           to Kbdvbase()->mousevec with a0 = packet, as EmuTOS' call_mousevec
 *           does (bios/aciavecs.S:538-553; the shim is mnprobe.c's); in the
 *           same callback, still at IPL 7, it reads vq_mouse and the screen
 *           ("immediate": before the next VBL can run vb_draw,
 *           vdi/vdi_mouse.c:668-686).
 *   NewTOS  a kernel armed by `tools/inject-arm.py <tree>
 *           launch:WKPROBE=wkprobe-mouse.txt` feeds the packets to the IKBD
 *           decoder from the trap-side input pump; the probe pumps with
 *           Bconstat(2) and polls vq_mouse until the position or the buttons
 *           change, then reads the screen ("immediate").  On a kernel without
 *           the injector the wait times out (logged "TIMEOUT").
 *   Both    then Vsync() three times and read again ("settled").
 *
 * Output, in the current directory:
 *   WKPROBE.BIN  "VDPR" + version WORD, then per record: 'C', record number
 *                (WORD), 8-byte name, x, y, w, h (WORDs), then w*h screen
 *                words, row by row; every WORD big-endian (vdprobe.c's format).
 *   WKPROBE.LOG  one line per case and per measured value; last line "END".
 *
 * Cases (record names; the letters after the number are the steps):
 *   CAL      pens 0..15, as vdprobe.c (the compare script's pen map)
 *   P0       the form the AES left installed, shown at (60,60)
 *   P1       the AES arrow via vsc_form at (100,100), right after v_show_c;
 *            v_get_pixel on a mask and a data pixel
 *   P2       block form, hotspot 0x17,0x17 (& 0x0f = 7), pens 2/3, on a pattern
 *   P3       block form with out-of-range pens (300, -2 -> pen 1)
 *   P4       arrow at (632,472): clipped right and bottom
 *   P5       hotspot 15,15 at (3,3): clipped left and top
 *   P6a/b    pattern before v_show_c and after v_hide_c (save-under)
 *   P7a-c    hide count 3, then v_show_c(1) three times
 *   F1a-d    K24: vsc_form while the pointer is shown (a: before, b: right
 *            after vsc_form, c: after v_hide_c, d: after v_show_c(0))
 *   F2b/c    K24: only the hotspot changes while shown (0,0 -> 8,8)
 *   G1b-d    (version 2, TASK/V54) F1 through the AES: graf_mouse(USER_DEF,
 *            block, pens 0/1) while shown (b: right after it, c: after
 *            v_hide_c, d: after v_show_c(0)) -- gsx_mfset's gsx_moff/gsx_mon
 *            bracket, aes/gemgsxif.c:407-420
 *   G2b/c    (version 2) F2 through the AES: graf_mouse(USER_DEF, arrow,
 *            hotspot 8,8) while shown
 *   D1a-c    v_bar over the shown pointer, then v_hide_c, then v_show_c(0)
 *   M1a/b    K9/K24: packet M1 while shown (immediate / settled)
 *   M2a/b    K9/K24: v_bar over the shown pointer, then packet M2
 *   M3a/b    packet M3 while hidden (settled), then v_show_c(0)
 *   W1a-c    K21: pointer state, vex_* vectors, virtual workstations around
 *            v_clswk + v_opnwk (a: screen after it, b: after v_show_c(1), c:
 *            after v_show_c(0)); log lines W1*
 *   W2a-c    K21: v_clswk + v_opnwk while the pointer is shown (a: after it,
 *            b: after v_hide_c, c: after v_show_c(0))
 *   C1a-d    K22: v_clswk while the pointer is shown (a: after it, then
 *            packet C1 b: immediate, c: settled, d: after v_hide_c); log C1*
 *   Z1-Z8    K23 (log only): v_opnwk with work_in[0] = 1, 2, 3, 4, 5 + INTOUT[45]
 *            0x1C, 5 + 0x1B, 1, 5 + 0x1C, each after a v_clswk
 *   W9       (log only, last) v_opnwk while the workstation is open
 * The LOG is rewritten at every case, so a hang leaves the lines up to it.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

#define VERSION 2

/* ---- parameter arrays ------------------------------------------------ */
static int16_t contrl[16];
static int16_t intin[128];
static int16_t ptsin[64];
static int16_t intout[128];
static int16_t ptsout[64];
static int16_t handle;		/* the workstation the drawing and the grabs use */
static int16_t ghandle;		/* graf_handle: the AES' physical workstation */
static int16_t work_pts[12];

static int16_t aes_control[5];
static int16_t aes_global[15];
static int16_t aes_int_in[16];
static int16_t aes_int_out[8];
static void *aes_addr_in[2];
static void *aes_addr_out[2];

static int16_t aes(int16_t op, int16_t nin, int16_t nout, int16_t nain)
{
	aes_control[0] = op;
	aes_control[1] = nin;
	aes_control[2] = nout;
	aes_control[3] = nain;
	aes_control[4] = 0;
	td_abi_aes(aes_control, aes_global, aes_int_in, aes_int_out,
	           aes_addr_in, aes_addr_out);
	return aes_int_out[0];
}

static void vdi(int16_t op, int16_t sub, int16_t npts, int16_t nint)
{
	memset(contrl, 0, sizeof contrl);
	contrl[0] = op;
	contrl[1] = npts;
	contrl[3] = nint;
	contrl[5] = sub;
	contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
}

static void i1(int16_t op, int16_t a) { intin[0] = a; vdi(op, 0, 0, 1); }
static void i2(int16_t op, int16_t a, int16_t b) { intin[0] = a; intin[1] = b; vdi(op, 0, 0, 2); }

static void set_ptr(int n, void *p)
{
	const int nptrints = (int)(sizeof(void *) / sizeof(int16_t));

	memcpy(&contrl[7 + n * nptrints], &p, sizeof p);
}

static void *get_ptr(int n)
{
	const int nptrints = (int)(sizeof(void *) / sizeof(int16_t));
	void *p;

	memcpy(&p, &contrl[7 + n * nptrints], sizeof p);
	return p;
}

/* ---- output ------------------------------------------------------------ */
static char logbuf[32768];
static int loglen;
static long binfh = -1;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (loglen < (int)sizeof(logbuf) - 200)
		loglen += vsnprintf(logbuf + loglen, sizeof(logbuf) - loglen, fmt, ap);
	va_end(ap);
}

static void put16(uint8_t *p, int v)
{
	p[0] = (uint8_t)((unsigned)v >> 8);
	p[1] = (uint8_t)v;
}

/* ---- the read-back (vdprobe.c grab, split into read and write) --------- */
typedef struct {
	void *fd_addr;
	int16_t fd_w, fd_h, fd_wdwidth, fd_stand, fd_nplanes, fd_r1, fd_r2, fd_r3;
} MFDB;

#define RB_WORDS (256 * 256)
static uint16_t rbuf[RB_WORDS];
static uint8_t wbuf[512];
static MFDB scr_mfdb, mem_mfdb;
static int recno;

static void grab_mem(int x, int y, int w, int h)
{
	memset(&scr_mfdb, 0, sizeof scr_mfdb);
	memset(&mem_mfdb, 0, sizeof mem_mfdb);
	mem_mfdb.fd_addr = rbuf;
	mem_mfdb.fd_w = (int16_t)w;
	mem_mfdb.fd_h = (int16_t)h;
	mem_mfdb.fd_wdwidth = (int16_t)((w + 15) / 16);
	mem_mfdb.fd_stand = 0;
	mem_mfdb.fd_nplanes = 16;
	memset(rbuf, 0x5A, sizeof rbuf);	/* a word no case draws */
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 109;
	contrl[1] = 4;
	contrl[3] = 1;
	contrl[6] = handle;
	set_ptr(0, &scr_mfdb);
	set_ptr(1, &mem_mfdb);
	intin[0] = 3;				/* S_ONLY */
	ptsin[0] = (int16_t)x; ptsin[1] = (int16_t)y;
	ptsin[2] = (int16_t)(x + w - 1); ptsin[3] = (int16_t)(y + h - 1);
	ptsin[4] = 0; ptsin[5] = 0;
	ptsin[6] = (int16_t)(w - 1); ptsin[7] = (int16_t)(h - 1);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
}

static void write_rec(const char *name, int x, int y, int w, int h)
{
	uint8_t hd[19];
	int r, c;

	recno++;
	hd[0] = 'C';
	put16(hd + 1, recno);
	memset(hd + 3, 0, 8);
	memcpy(hd + 3, name, strlen(name) < 8 ? strlen(name) : 8);
	put16(hd + 11, x); put16(hd + 13, y); put16(hd + 15, w); put16(hd + 17, h);
	if (binfh >= 0)
		Fwrite((short)binfh, 19L, hd);
	for (r = 0; r < h; r++) {
		const uint16_t *row = rbuf + (long)r * ((w + 15) / 16) * 16;
		for (c = 0; c < w; c++)
			put16(wbuf + 2 * c, row[c]);
		if (binfh >= 0)
			Fwrite((short)binfh, (long)(2 * w), wbuf);
	}
	emit("  rec %02d %s (%d,%d %dx%d)\n", recno, name, x, y, w, h);
}

static void grab(const char *name, int x, int y, int w, int h)
{
	grab_mem(x, y, w, h);
	write_rec(name, x, y, w, h);
}

/* ---- VDI helpers ------------------------------------------------------- */
static void clear(void) { vdi(3, 0, 0, 0); }	/* v_clrwk */

static void attrs(void)
{
	i1(32, 1);				/* vswr_mode replace */
	i1(23, 1);				/* vsf_interior solid */
	i1(25, 1);				/* vsf_color 1 */
	i1(104, 0);				/* vsf_perimeter off */
	i1(17, 1);				/* vsl_color 1 */
	i1(129, 0);				/* vs_clip off */
}

static void recfl(int pen, int x1, int y1, int x2, int y2)
{
	i1(25, (int16_t)pen);
	ptsin[0] = (int16_t)x1; ptsin[1] = (int16_t)y1;
	ptsin[2] = (int16_t)x2; ptsin[3] = (int16_t)y2;
	vdi(114, 0, 2, 0);			/* vr_recfl */
}

static void bar(int pen, int x1, int y1, int x2, int y2)
{
	i1(25, (int16_t)pen);
	ptsin[0] = (int16_t)x1; ptsin[1] = (int16_t)y1;
	ptsin[2] = (int16_t)x2; ptsin[3] = (int16_t)y2;
	vdi(11, 1, 2, 0);			/* v_bar */
}

/* a pattern with structure: pen 6 ground, pen 2 and pen 4 stripes every 4 px */
static void pattern(int x1, int y1, int x2, int y2)
{
	int x;

	recfl(6, x1, y1, x2, y2);
	for (x = x1; x + 1 <= x2; x += 4)
		recfl((x & 4) ? 2 : 4, x, y1, x + 1, y2);
}

static int16_t getpix(int x, int y)
{
	intout[0] = intout[1] = 0x7777;
	ptsin[0] = (int16_t)x; ptsin[1] = (int16_t)y;
	vdi(105, 0, 1, 0);
	return intout[1];
}

/* the pointer */
static void show(int flag) { i1(122, (int16_t)flag); }	/* v_show_c */
static void hide(void) { vdi(123, 0, 0, 0); }		/* v_hide_c */

static void setpos(int x, int y)
{
	i2(33, 1, 2);				/* vsin_mode: locator, sample */
	ptsin[0] = (int16_t)x; ptsin[1] = (int16_t)y;
	vdi(28, 0, 1, 0);			/* v_locator: GCURX/GCURY := (x,y) */
}

static void mouse(int16_t *b, int16_t *x, int16_t *y)
{
	intout[0] = 0x7777; ptsout[0] = ptsout[1] = 0x7777;
	vdi(124, 0, 0, 0);			/* vq_mouse */
	*b = intout[0]; *x = ptsout[0]; *y = ptsout[1];
}

/* the AES arrow (aes/mform.rsc; NewTOS PRESET_FORMS[0]) */
static const uint16_t arrow_mask[16] = {
	0xC000, 0xE000, 0xF000, 0xF800, 0xFC00, 0xFE00, 0xFF00, 0xFF80,
	0xFFC0, 0xFFE0, 0xFE00, 0xEF00, 0xCF00, 0x8780, 0x0780, 0x0380 };
static const uint16_t arrow_data[16] = {
	0x0000, 0x4000, 0x6000, 0x7000, 0x7800, 0x7C00, 0x7E00, 0x7F00,
	0x7F80, 0x7C00, 0x6C00, 0x4600, 0x0600, 0x0300, 0x0300, 0x0000 };
/* a block: full mask, checker data */
static uint16_t block_mask[16], block_data[16];
/* a cross: diagonal data, widened mask */
static uint16_t cross_mask[16], cross_data[16];

static void build_forms(void)
{
	int i;

	for (i = 0; i < 16; i++) {
		block_mask[i] = 0xFFFF;
		block_data[i] = (i & 1) ? 0x5555 : 0xAAAA;
		cross_data[i] = (uint16_t)((0x8000u >> i) | (1u << i));
		cross_mask[i] = (uint16_t)(cross_data[i] | (cross_data[i] << 1) | (cross_data[i] >> 1));
	}
}

static void form(int hx, int hy, int bg, int fg, const uint16_t *m, const uint16_t *d)
{
	int i;

	intin[0] = (int16_t)hx; intin[1] = (int16_t)hy; intin[2] = 1;
	intin[3] = (int16_t)bg; intin[4] = (int16_t)fg;
	for (i = 0; i < 16; i++) {
		intin[5 + i] = (int16_t)m[i];
		intin[21 + i] = (int16_t)d[i];
	}
	vdi(111, 0, 0, 37);			/* vsc_form */
}

static void arrow(void) { form(0, 0, 0, 1, arrow_mask, arrow_data); }

/* hide count := 1 (hidden), the screen cleared, attributes set */
static int caseno;

static void norm(void)
{
	show(0);
	hide();
	clear();
	attrs();
}

static void flush_log(void);

static void begin(const char *name, const char *what)
{
	flush_log();
	caseno++;
	norm();
	emit("%02d %-6s %s\n", caseno, name, what);
}

/* ---- vex_* ------------------------------------------------------------- */
static void xnop(void) { }

/* exchange vector `op` (118 timv, 125 butv, 126 motv, 127 curv, 134 wheelv) */
static void *vex(int16_t op, void *newv)
{
	memset(contrl, 0, sizeof contrl);
	contrl[0] = op;
	contrl[6] = handle;
	set_ptr(0, newv);
	set_ptr(1, (void *)0);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	return get_ptr(1);
}

static const int16_t vex_ops[5] = { 125, 126, 127, 118, 134 };
static const char *const vex_names[5] = { "butv", "motv", "curv", "timv", "wheelv" };

/* ---- mouse packets ----------------------------------------------------- */
static int16_t mb0, mx0, my0, mb1, mx1, my1;
static int g_x, g_y, g_w, g_h;		/* the immediate grab, w = 0: none */

#ifndef __NEWTOS__
static long *kvecs;
static unsigned char pkt[4];
void inj_mousevec(void *fn, unsigned char *p) __asm__("wkp_inj_mousevec");

/* call_mousevec (bios/aciavecs.S:538-553) with the vector passed in (mnprobe.c) */
__asm__(
"	.text\n"
"	.even\n"
"wkp_inj_mousevec:\n"
"	move.l	4(sp),a1\n"
"	move.l	8(sp),a0\n"
"	movem.l	d2-d7/a2-a6,-(sp)\n"
"	jsr	(a1)\n"
"	movem.l	(sp)+,d2-d7/a2-a6\n"
"	rts\n"
);

static long sv_feed(void)
{
	uint16_t sr;

	__asm__ volatile("move.w %%sr,%0\n\tori.w #0x0700,%%sr" : "=d"(sr) : : "memory");
	inj_mousevec((void *)kvecs[4], pkt);
	mouse(&mb1, &mx1, &my1);
	if (g_w)
		grab_mem(g_x, g_y, g_w, g_h);
	__asm__ volatile("move.w %0,%%sr" : : "d"(sr) : "memory");
	return 0;
}

static long hz200(void) { return (long)*(volatile uint32_t *)(uintptr_t)0x4BA; }
static uint32_t ticks(void) { return (uint32_t)Supexec(hz200); }
#else
static uint32_t ticks(void) { return *(volatile uint32_t *)(uintptr_t)0x4BA; }
#endif

/*
 * Feed one packet (EmuTOS) or wait for it (NewTOS); read vq_mouse and, if
 * name != NULL, the rectangle right after it ("immediate", record <name>).
 */
static void packet(int hdr, int dx, int dy, const char *name, int x, int y, int w, int h)
{
	g_x = x; g_y = y; g_w = name ? w : 0; g_h = h;
	mouse(&mb0, &mx0, &my0);
#ifndef __NEWTOS__
	pkt[0] = (unsigned char)hdr; pkt[1] = (unsigned char)(int8_t)dx; pkt[2] = (unsigned char)(int8_t)dy;
	Supexec(sv_feed);
#else
	{
		/* NewTOS-only lines start with "N " (the compare skips them) */
		uint32_t t0 = ticks();
		long n;

		for (n = 0; n < 2000000L; n++) {
			(void)Bconstat(2);	/* pumps the input: the injector feeds here */
			mouse(&mb1, &mx1, &my1);
			if (mb1 != mb0 || mx1 != mx0 || my1 != my0)
				break;
			if (ticks() - t0 > 6000)	/* 30 s */
				break;
		}
		if (g_w)
			grab_mem(g_x, g_y, g_w, g_h);
		emit("N   waited %lu ticks, %ld polls\n", (unsigned long)(ticks() - t0), n);
	}
#endif
	emit("  packet %02x %d %d: vq_mouse %d %d %d -> %d %d %d (%s)\n", hdr, dx, dy,
	     mb0, mx0, my0, mb1, mx1, my1,
	     (mb1 != mb0 || mx1 != mx0 || my1 != my0) ? "changed" : "unchanged");
	if (name)
		write_rec(name, x, y, w, h);
}

static void settle(void)
{
	Vsync(); Vsync(); Vsync();
}

/* ---- the cases --------------------------------------------------------- */
static void k_calibration(void)
{
	int i;

	begin("CAL", "pens 0..15, 8x8 each, vr_recfl solid; x 128..135 untouched");
	for (i = 0; i < 16; i++)
		recfl(i, 8 * i, 0, 8 * i + 7, 7);
	grab("CAL", 0, 0, 136, 8);
}

static void k_pointer(void)
{
	begin("P0", "the form left installed by the AES, v_show_c(0) at (60,60)");
	setpos(60, 60); show(0); grab("P0", 52, 52, 32, 32); hide();

	begin("P1", "AES arrow via vsc_form at (100,100) on the cleared screen, read right after v_show_c(0)");
	arrow(); setpos(100, 100); show(0); grab("P1", 96, 96, 32, 32);
	emit("  v_get_pixel (100,100) %04x (mask)  (101,101) %04x (data)  (99,99) %04x (outside)\n",
	     (uint16_t)getpix(100, 100), (uint16_t)getpix(101, 101), (uint16_t)getpix(99, 99));
	hide();

	begin("P2", "block form, hotspot 0x17,0x17, mask pen 2, data pen 3, at (200,150) on a pattern");
	pattern(176, 126, 239, 189);
	form(0x17, 0x17, 2, 3, block_mask, block_data); setpos(200, 150); show(0);
	grab("P2", 176, 126, 64, 64); hide();

	begin("P3", "block form, mask pen 300, data pen -2 (both out of range), hotspot 0,0 at (300,150)");
	form(0, 0, 300, -2, block_mask, block_data); setpos(300, 150); show(0);
	grab("P3", 292, 142, 32, 32); hide();

	begin("P4", "arrow at (632,472): clipped right and bottom");
	pattern(600, 440, 639, 479);
	arrow(); setpos(632, 472); show(0); grab("P4", 600, 440, 40, 40); hide();

	begin("P5", "cross, hotspot 15,15, at (3,3): clipped left and top");
	pattern(0, 0, 39, 39);
	form(15, 15, 4, 1, cross_mask, cross_data); setpos(3, 3); show(0); grab("P5", 0, 0, 40, 40); hide();

	begin("P6", "save-under: pattern, block form shown at (400,150), hidden again");
	pattern(380, 130, 443, 193);
	grab("P6a", 380, 130, 64, 64);
	form(4, 4, 2, 3, block_mask, block_data); setpos(400, 150); show(0); hide();
	grab("P6b", 380, 130, 64, 64);

	begin("P7", "hide count 3 (two v_hide_c more), then v_show_c(1) three times at (500,150)");
	arrow(); setpos(500, 150); hide(); hide();
	show(1); grab("P7a", 492, 142, 32, 32);
	show(1); grab("P7b", 492, 142, 32, 32);
	show(1); grab("P7c", 492, 142, 32, 32);
	hide();
}

static void k_form(void)
{
	begin("F1", "K24: vsc_form while the pointer is shown (arrow -> block) at (110,260) on a pattern");
	pattern(90, 240, 153, 303);
	arrow(); setpos(110, 260); show(0);
	grab("F1a", 90, 240, 64, 64);
	form(0, 0, 2, 3, block_mask, block_data);
	grab("F1b", 90, 240, 64, 64);
	hide();
	grab("F1c", 90, 240, 64, 64);
	show(0);
	grab("F1d", 90, 240, 64, 64);
	hide();

	begin("F2", "K24: only the hotspot changes while shown (arrow 0,0 -> 8,8) at (300,260) on a pattern");
	pattern(280, 240, 343, 303);
	arrow(); setpos(300, 260); show(0);
	form(8, 8, 0, 1, arrow_mask, arrow_data);
	grab("F2b", 280, 240, 64, 64);
	hide();
	grab("F2c", 280, 240, 64, 64);
}

/*
 * The AES's form install (TASK/V54): gr_mouse(USER_DEF) -> gsx_mfset
 * (aes/gemgrlib.c:419-455, aes/gemgsxif.c:407-420), which brackets the same
 * vsc_form with gsx_moff/gsx_mon.  Pens 0/1 and hotspots inside 0..15, so the
 * cases see only the moment of drawing.
 */
static int16_t mform[37];

static void graf_mouse_user(int hx, int hy, const uint16_t *m, const uint16_t *d)
{
	int i;

	mform[0] = (int16_t)hx; mform[1] = (int16_t)hy; mform[2] = 1;
	mform[3] = 0; mform[4] = 1;
	for (i = 0; i < 16; i++) {
		mform[5 + i] = (int16_t)m[i];
		mform[21 + i] = (int16_t)d[i];
	}
	aes_int_in[0] = 255;			/* USER_DEF */
	aes_addr_in[0] = mform;
	aes(78, 1, 1, 1);			/* graf_mouse */
}

static void k_aesform(void)
{
	begin("G1", "V54: graf_mouse(USER_DEF, block, pens 0/1) while the pointer is shown at (110,360) on a pattern");
	pattern(90, 340, 153, 403);
	arrow(); setpos(110, 360); show(0);
	graf_mouse_user(0, 0, block_mask, block_data);
	grab("G1b", 90, 340, 64, 64);
	hide();
	grab("G1c", 90, 340, 64, 64);
	show(0);
	grab("G1d", 90, 340, 64, 64);
	hide();

	begin("G2", "V54: graf_mouse(USER_DEF, arrow, hotspot 8,8) while shown at (300,360) on a pattern");
	pattern(280, 340, 343, 403);
	arrow(); setpos(300, 360); show(0);
	graf_mouse_user(8, 8, arrow_mask, arrow_data);
	grab("G2b", 280, 340, 64, 64);
	hide();
	grab("G2c", 280, 340, 64, 64);
	aes_int_in[0] = 0;			/* graf_mouse(ARROW) while hidden */
	aes(78, 1, 1, 1);
}

static void k_drop(void)
{
	begin("D1", "v_bar pen 2 over the shown arrow at (450,260), then v_hide_c, then v_show_c(0)");
	arrow(); setpos(450, 260); show(0);
	bar(2, 440, 250, 479, 289);
	grab("D1a", 432, 244, 56, 56);
	hide();
	grab("D1b", 432, 244, 56, 56);
	show(0);
	grab("D1c", 432, 244, 56, 56);
	hide();
}

static void k_motion(void)
{
	begin("M1", "K9/K24: arrow shown at (200,350), packet +20,+8 (immediate, settled)");
	arrow(); setpos(200, 350); show(0);
	packet(0xF8, 20, 8, "M1a", 192, 344, 64, 40);
	settle();
	grab("M1b", 192, 344, 64, 40);
	hide();

	begin("M2", "K9/K24: arrow shown at (350,350), v_bar pen 2 over it, packet +30,0");
	arrow(); setpos(350, 350); show(0);
	bar(2, 340, 340, 389, 389);
	packet(0xF8, 30, 0, "M2a", 336, 336, 96, 56);
	settle();
	grab("M2b", 336, 336, 96, 56);
	hide();

	begin("M3", "arrow hidden at (500,350), packet +10,+10, settled; then v_show_c(0)");
	arrow(); setpos(500, 350);
	packet(0xF8, 10, 10, NULL, 0, 0, 0, 0);
	settle();
	grab("M3a", 492, 344, 40, 40);
	show(0);
	grab("M3b", 492, 344, 40, 40);
	hide();
}

static void *vec_a[5];		/* the vectors found before W1 (the AES' own) */

static int16_t open_vwk(int16_t on)
{
	int i;

	for (i = 0; i < 10; i++)
		intin[i] = 1;
	intin[10] = 2;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 100; contrl[3] = 11; contrl[6] = on;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	return contrl[6];
}

/* v_opnwk with work_in[0] = rez (1 = no mode request); intout45 = the Videl
 * mode EmuTOS reads from INTOUT[45] for rez 5 (vdi_control.c:517-525) */
static void opnwk(const char *tag, int16_t rez, int16_t intout45)
{
	int i;

	intin[0] = rez;
	for (i = 1; i < 10; i++)
		intin[i] = 1;
	intin[10] = 2;
	memset(intout, 0, sizeof intout);
	intout[45] = intout45;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 1; contrl[3] = 11; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	emit("  %s v_opnwk(%d): handle %d nintout %d nptsout %d work_out[0] %d [1] %d [13] %d [35] %d [39] %d ptsout[1] %d\n",
	     tag, rez, contrl[6], contrl[4], contrl[2], intout[0], intout[1], intout[13], intout[35],
	     intout[39], ptsout[1]);
}

/* v_clswk on the AES' physical handle; the drawing workstation is gone after it */
static void clswk(const char *tag)
{
	handle = ghandle;
	vdi(2, 0, 0, 0);
	emit("  %s v_clswk on the graf_handle handle\n", tag);
}

static void flush_log(void)
{
	long fh = Fcreate("WKPROBE.LOG", 0);

	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
}

/*
 * K21.  EmuTOS' own sequence is v_clswk ... v_opnwk (the AES' session end and
 * start, aes/geminit.c:675,749): a v_opnwk while the physical workstation is
 * open runs timer_init again, whose Setexc(0x100, tick_int) then returns
 * tick_int itself as the chain (vdi_misc.c:126-127, :88) -- see W9.  So the
 * state is set, v_clswk and v_opnwk run as a pair, and what v_opnwk reset is
 * read afterwards.
 */
static void k_opnwk(void)
{
	int16_t old, nh, b, x, y;
	void *c;
	int i;

	begin("W1", "K21: cross at (100,100), hide count 3, own vex vectors, left button down, v_clswk + v_opnwk");
	form(5, 5, 4, 1, cross_mask, cross_data);
	setpos(100, 100);
	hide(); hide();				/* hide count 3 */
	for (i = 0; i < 5; i++)
		vec_a[i] = vex(vex_ops[i], (void *)xnop);
	packet(0xFA, 0, 0, NULL, 0, 0, 0, 0);	/* left button down */
	mouse(&b, &x, &y);
	emit("  W1 before: vq_mouse %d %d %d\n", b, x, y);
	old = handle;
	flush_log();
	clswk("W1");
	opnwk("W1", 1, 0);
	flush_log();
	/* the probe's previous workstation: still open? (vq_extnd mode 0, asked
	 * before a new v_opnvwk could reuse the handle) */
	handle = old;
	intin[0] = 0;
	intout[0] = 0x7777;
	vdi(102, 0, 0, 1);
	emit("  W1 old handle after: vq_extnd nintout %d nptsout %d intout[0] %d\n",
	     contrl[4], contrl[2], intout[0]);
	nh = open_vwk(ghandle);
	handle = nh;
	emit("  W1 new handle == old handle %d\n", nh == old);
	attrs();
	mouse(&b, &x, &y);
	emit("  W1 after: vq_mouse %d %d %d\n", b, x, y);
	intin[0] = 1;
	vdi(115, 0, 0, 1);			/* vqin_mode locator */
	emit("  W1 after: vqin_mode(1) %d\n", intout[0]);
	for (i = 0; i < 5; i++) {
		c = vex(vex_ops[i], vec_a[i]);	/* put the AES' vector back */
		emit("  W1 vex_%s after: old == own %d, old == vector before W1 %d%s\n",
		     vex_names[i], c == (void *)xnop, c == vec_a[i],
		     vex_ops[i] == 118 ? (intout[0] == 20 ? " (ms 20)" : " (ms not 20)") : "");
	}
	grab("W1a", 80, 80, 256, 176);
	show(1);
	grab("W1b", 80, 80, 256, 176);
	show(0);
	grab("W1c", 80, 80, 256, 176);
	hide();
	packet(0xF8, 0, 0, NULL, 0, 0, 0, 0);	/* left button up */

	begin("W2", "K21: cross shown at (100,100) on a pattern, v_clswk + v_opnwk while shown");
	pattern(90, 90, 149, 149);
	form(5, 5, 4, 1, cross_mask, cross_data);
	setpos(100, 100); show(0);
	flush_log();
	clswk("W2");
	opnwk("W2", 1, 0);
	handle = open_vwk(ghandle);
	attrs();
	for (i = 0; i < 5; i++)
		(void)vex(vex_ops[i], vec_a[i]);	/* the AES' vectors back */
	grab("W2a", 80, 80, 256, 176);
	hide();
	grab("W2b", 80, 80, 256, 176);
	show(0);
	grab("W2c", 80, 80, 256, 176);
	hide();
	mouse(&b, &x, &y);
	emit("  W2 after: vq_mouse %d %d %d\n", b, x, y);
}

static void k_clswk(void)
{
	void *cur0, *c, *a2[5];
	int16_t b, x, y;
	int i;

	begin("C1", "K22: arrow shown at (150,150) on a pattern, own butv/motv/timv/wheelv, then v_clswk");
	pattern(130, 130, 193, 193);
	arrow(); setpos(150, 150); show(0);
	cur0 = vex(127, (void *)xnop);		/* the cursor vector in place */
	(void)vex(127, cur0);
	for (i = 0; i < 5; i++)
		if (vex_ops[i] != 127)
			a2[i] = vex(vex_ops[i], (void *)xnop);
	flush_log();
	clswk("C1");
	grab("C1a", 120, 120, 128, 128);
	for (i = 0; i < 5; i++) {
		if (vex_ops[i] == 127) {
			c = vex(127, cur0);
			emit("  C1 vex_curv after v_clswk: old == cursor vector before %d, == own %d\n",
			     c == cur0, c == (void *)xnop);
		} else {
			c = vex(vex_ops[i], a2[i]);
			emit("  C1 vex_%s after v_clswk: old == own %d, old == vector before C1 %d\n",
			     vex_names[i], c == (void *)xnop, c == a2[i]);
		}
	}
	mouse(&b, &x, &y);
	emit("  C1 after v_clswk: vq_mouse %d %d %d\n", b, x, y);
	packet(0xF8, 40, 20, "C1b", 120, 120, 128, 128);
	settle();
	grab("C1c", 120, 120, 128, 128);
	hide();
	grab("C1d", 120, 120, 128, 128);
}

/* the geometry the physical workstation reports now */
static void geometry(const char *tag)
{
	intin[0] = 1;
	vdi(102, 0, 0, 1);			/* vq_extnd(1): INQ_TAB */
	emit("  %s vq_extnd(1): planes %d lut %d; Getrez %d\n", tag, intout[4], intout[5], (int)Getrez());
}

/*
 * K23: the mode request in work_in[0] (vdi_control.c:503-526), each v_opnwk
 * after a v_clswk.  Runs after C1 (the workstation is closed there).  The
 * last request puts the Falcon truecolor mode back (VGA|COL80|BPS16 = 0x1C).
 */
static void k_rez(void)
{
	static const int16_t rez[] = { 1, 2, 3, 4, 5, 5, 1, 5 };
	static const int16_t vid[] = { 0, 0, 0, 0, 0x1C, 0x1B, 0, 0x1C };
	char tag[8];
	int i;

	caseno++;
	emit("%02d %-6s %s\n", caseno, "Z", "K23: v_opnwk work_in[0] = 1, 2, 3, 4, 5 (0x1C), 5 (0x1B), 1, 5 (0x1C)");
	for (i = 0; i < 8; i++) {
		snprintf(tag, sizeof tag, "Z%d", i + 1);
		flush_log();
		if (i > 0)
			clswk(tag);
		handle = ghandle;
		opnwk(tag, rez[i], vid[i]);
		geometry(tag);
	}
}

/*
 * W9: v_opnwk while the physical workstation is open (no v_clswk before it),
 * the last case: on EmuTOS the second timer_init chains tick_int to itself.
 */
static void k_unpaired(void)
{
	caseno++;
	emit("%02d %-6s %s\n", caseno, "W9", "v_opnwk while the physical workstation is open (no v_clswk)");
	handle = ghandle;
	emit("  W9 before\n");
	flush_log();
	opnwk("W9", 1, 0);
	emit("  W9 v_opnwk returned\n");
	flush_log();
	{
		uint32_t t0 = ticks();

		Vsync(); Vsync(); Vsync();
		emit("  W9 three Vsync returned, hz_200 advanced %d\n", ticks() != t0);
	}
	flush_log();
}

int main(void)
{
	int16_t ap_id;
	int i;
	uint8_t hd[6];
	build_forms();
	ap_id = aes(10, 0, 1, 0);		/* appl_init */
	if (ap_id < 0)
		return 1;
	aes(77, 0, 5, 0);			/* graf_handle */
	ghandle = aes_int_out[0];
	handle = open_vwk(ghandle);
	for (i = 0; i < 12; i++)
		work_pts[i] = ptsout[i];
	emit("H00 graf_handle %d handle %d xres %d yres %d colours %d\n", ghandle, handle,
	     intout[0], intout[1], intout[13]);
#ifndef __NEWTOS__
	kvecs = (long *)Kbdvbase();
#endif
	emit("T01 start hz_200 %lu\n", (unsigned long)ticks());

	binfh = Fcreate("WKPROBE.BIN", 0);
	memcpy(hd, "VDPR", 4);
	put16(hd + 4, VERSION);
	if (binfh >= 0)
		Fwrite((short)binfh, 6L, hd);

	k_calibration();
	k_pointer();
	k_form();
	k_aesform();
	k_drop();
	emit("T02 before the packets hz_200 %lu\n", (unsigned long)ticks());
	k_motion();
	k_opnwk();
	k_clswk();
	if (binfh >= 0)
		Fclose((short)binfh);
	binfh = -1;
	k_rez();
	k_unpaired();
	emit("T03 end hz_200 %lu\n", (unsigned long)ticks());
	emit("END\n");
	flush_log();
	return 0;
}
