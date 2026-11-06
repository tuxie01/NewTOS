/*
 * vdprobe.c -- VDI drawing compared pixel for pixel (TASK/V18 part 1: the
 * TOOL32 candidates K1-K8, K10, K11).  A permanent probe of the tosdump tree
 * like sbprobe.c: the drawing primitives leave the parameter arrays empty, so
 * DUMPVDI cannot see what they draw; this probe draws each case on a cleared
 * screen and reads the affected rectangle back with vro_cpyfm (opcode 109,
 * S_ONLY, screen -> a 16-plane device-form memory MFDB), the copy AES bb_save
 * makes.  The instrument itself is measured by tosdump/cases/
 * vdi-raster-screen.tsv against EMUTOS-1a7484ab-RASTER-screen-falcontc.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari profile falcontc =
 * 640x480 16-bit truecolor, AArch64 NewTOS = the Falcon truecolor workstation
 * of ADR-024).  All VDI and AES calls go through the tosdump ABI backends
 * (abi/m68k_trap.c, abi/newtos_svc.c); no gemlib.  The one width-dependent
 * thing, the MFDB pointer in CONTRL[7..], is packed as tosdump/core/
 * runner_rast.c packs it (pointer n at CONTRL[7 + n * sizeof(void *) / 2]),
 * and the MFDB is the native struct of each side (pointer, then five WORDs),
 * which is what tos-api/src/vdi/raster.rs Mfdb::read reads for an LP64 caller.
 * Built by `make -C tosdump vdprobe-m68k` (VDPROBE.PRG) and
 * `make -C tosdump vdprobe-newtos` (VDPROBE.NAP, build-vdprobe-newtos.sh);
 * run by tosdump/probes/run-vdprobe.sh; compared by
 * tosdump/probes/vdprobe-compare.py.
 *
 * Output, in the current directory:
 *   VDPROBE.BIN  "VDPR" + version WORD, then per case: 'C', case number
 *                (WORD), 8-byte name, x, y, w, h (WORDs), then w*h screen
 *                words, row by row.  Every WORD big-endian on both sides.
 *   VDPROBE.LOG  one line per case (name, what it draws) plus the WORD
 *                results of v_get_pixel cases; last line "END".
 *
 * Every case starts from the same state: v_clrwk, the attribute defaults the
 * dumper's reset uses (tosdump/core/runner.c td_reset_vdi), clipping off.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

#define VERSION 1

/* ---- parameter arrays ------------------------------------------------ */
static int16_t contrl[16];
static int16_t intin[300];
static int16_t ptsin[2 * 1100];
static int16_t intout[128];
static int16_t ptsout[256];
static int16_t handle;
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
static void p1(int16_t op, int16_t x, int16_t y) { ptsin[0] = x; ptsin[1] = y; vdi(op, 0, 1, 0); }

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

/* ---- the read-back ----------------------------------------------------- */
typedef struct {
	void *fd_addr;
	int16_t fd_w, fd_h, fd_wdwidth, fd_stand, fd_nplanes, fd_r1, fd_r2, fd_r3;
} MFDB;

#define RB_MAXW 640
#define RB_MAXH 480
static uint16_t rbuf[RB_MAXW * RB_MAXH];
static uint8_t wbuf[RB_MAXW * 2];
static MFDB scr_mfdb, mem_mfdb;

static void set_mfdb_ptr(int n, void *p)
{
	const int nptrints = (int)(sizeof(void *) / sizeof(int16_t));

	memcpy(&contrl[7 + n * nptrints], &p, sizeof p);
}

static int caseno;

/* read the rectangle (x,y,w,h) and append it to VDPROBE.BIN */
static void grab(const char *name, int x, int y, int w, int h)
{
	uint8_t hd[19];
	int r, c;

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
	set_mfdb_ptr(0, &scr_mfdb);
	set_mfdb_ptr(1, &mem_mfdb);
	intin[0] = 3;				/* S_ONLY */
	ptsin[0] = (int16_t)x; ptsin[1] = (int16_t)y;
	ptsin[2] = (int16_t)(x + w - 1); ptsin[3] = (int16_t)(y + h - 1);
	ptsin[4] = 0; ptsin[5] = 0;
	ptsin[6] = (int16_t)(w - 1); ptsin[7] = (int16_t)(h - 1);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);

	hd[0] = 'C';
	put16(hd + 1, caseno);
	memset(hd + 3, 0, 8);
	memcpy(hd + 3, name, strlen(name) < 8 ? strlen(name) : 8);
	put16(hd + 11, x); put16(hd + 13, y); put16(hd + 15, w); put16(hd + 17, h);
	if (binfh >= 0)
		Fwrite((short)binfh, 19L, hd);
	for (r = 0; r < h; r++) {
		const uint16_t *row = rbuf + (long)r * mem_mfdb.fd_wdwidth * 16;
		for (c = 0; c < w; c++)
			put16(wbuf + 2 * c, row[c]);
		if (binfh >= 0)
			Fwrite((short)binfh, (long)(2 * w), wbuf);
	}
}

/* ---- per-case state -------------------------------------------------- */
static void reset(void)
{
	vdi(3, 0, 0, 0);			/* v_clrwk */
	i1(32, 1);				/* vswr_mode replace */
	i1(15, 1);				/* vsl_type solid */
	p1(16, work_pts[4], 0);			/* vsl_width default */
	i1(17, 1);				/* vsl_color 1 */
	i2(108, 0, 0);				/* vsl_ends squared */
	i1(113, (int16_t)0xFFFF);		/* vsl_udsty */
	i1(18, 1);				/* vsm_type 1 */
	p1(19, 0, work_pts[9]);			/* vsm_height default */
	i1(20, 1);				/* vsm_color 1 */
	i1(23, 1);				/* vsf_interior solid */
	i1(24, 1);				/* vsf_style 1 */
	i1(25, 1);				/* vsf_color 1 */
	i1(104, 1);				/* vsf_perimeter on */
	i1(129, 0);				/* vs_clip off */
}

static void begin(const char *name, const char *what)
{
	caseno++;
	reset();
	emit("%02d %-8s %s\n", caseno, name, what);
}

static void clip(int on, int x1, int y1, int x2, int y2)
{
	intin[0] = (int16_t)on;
	ptsin[0] = (int16_t)x1; ptsin[1] = (int16_t)y1;
	ptsin[2] = (int16_t)x2; ptsin[3] = (int16_t)y2;
	vdi(129, 0, 2, 1);
}

static void line2(int x1, int y1, int x2, int y2)
{
	ptsin[0] = (int16_t)x1; ptsin[1] = (int16_t)y1;
	ptsin[2] = (int16_t)x2; ptsin[3] = (int16_t)y2;
	vdi(6, 0, 2, 0);
}

static void pline(int n, const int16_t *xy)
{
	int i;

	for (i = 0; i < 2 * n; i++)
		ptsin[i] = xy[i];
	vdi(6, 0, (int16_t)n, 0);
}

static void fillarea(int n, const int16_t *xy)
{
	int i;

	for (i = 0; i < 2 * n; i++)
		ptsin[i] = xy[i];
	vdi(9, 0, (int16_t)n, 0);
}

static void recfl(int x1, int y1, int x2, int y2)
{
	ptsin[0] = (int16_t)x1; ptsin[1] = (int16_t)y1;
	ptsin[2] = (int16_t)x2; ptsin[3] = (int16_t)y2;
	vdi(114, 0, 2, 0);			/* vr_recfl */
}

static void bar(int x1, int y1, int x2, int y2)
{
	ptsin[0] = (int16_t)x1; ptsin[1] = (int16_t)y1;
	ptsin[2] = (int16_t)x2; ptsin[3] = (int16_t)y2;
	vdi(11, 1, 2, 0);			/* v_bar */
}

static void contour(int x, int y, int col)
{
	intin[0] = (int16_t)col;
	ptsin[0] = (int16_t)x; ptsin[1] = (int16_t)y;
	vdi(103, 0, 1, 1);
}

static void udpat(int n, const uint16_t *w)
{
	int i;

	for (i = 0; i < n; i++)
		intin[i] = (int16_t)w[i];
	vdi(112, 0, 0, (int16_t)n);
}

static int16_t getpix(int x, int y)
{
	intout[0] = intout[1] = 0x7777;
	ptsin[0] = (int16_t)x; ptsin[1] = (int16_t)y;
	vdi(105, 0, 1, 0);
	return intout[1];
}

/* ---- K3: the line fan ------------------------------------------------ */
static const int8_t fanbase[][2] = {
	{22, 1}, {22, 3}, {22, 7}, {22, 11}, {21, 8}, {20, 13}, {19, 5}, {17, 16},
	{15, 4}, {13, 6}, {9, 2}, {22, 21}, {7, 3}, {5, 2}, {3, 1}, {2, 1},
};

/* 128 lines, one per 48x48 cell, 13 cells a row: cell k at
 * (48*(k%13), 48*(k/13)), line from the cell centre (24,24) to centre +
 * (dx,dy); rev = draw from the far end */
static void fan(int rev)
{
	int k = 0, b, s;

	for (b = 0; b < 16; b++)
		for (s = 0; s < 8; s++) {
			int a = fanbase[b][0], c = fanbase[b][1];
			int dx = (s & 4) ? c : a, dy = (s & 4) ? a : c;
			int cx = 48 * (k % 13) + 24, cy = 48 * (k / 13) + 24;

			if (s & 1) dx = -dx;
			if (s & 2) dy = -dy;
			if (rev)
				line2(cx + dx, cy + dy, cx, cy);
			else
				line2(cx, cy, cx + dx, cy + dy);
			k++;
		}
}

/* ---- the cases --------------------------------------------------------- */
static int16_t pts[2 * 1100];

static void k_calibration(void)
{
	int i;

	begin("CAL", "pens 0..15, 8x8 each, vr_recfl solid; x 128..135 untouched");
	for (i = 0; i < 16; i++) {
		i1(25, (int16_t)i);
		recfl(8 * i, 0, 8 * i + 7, 7);
	}
	grab("CAL", 0, 0, 136, 8);
}

static void k1(void)
{
	static const int16_t a[] = { 20, 20, 20, 40, 40, 40 };
	static const int16_t b[] = { 20, 20, 40, 20, 40, 40, 60, 40 };
	static const int16_t c[] = { 20, 40, 20, 20, 40, 20 };
	static const int16_t d[] = { 20, 20, 40, 20, 40, 40, 20, 40, 20, 20 };
	static const int16_t e[] = { 20, 20, 40, 30, 60, 20 };
	static const int16_t f[] = { 20, 20, 20, 40 };

	begin("K1A", "XOR polyline: vertical segment first (20,20)-(20,40)-(40,40)");
	i1(32, 3); pline(3, a); grab("K1A", 10, 10, 60, 40);
	begin("K1B", "XOR polyline: vertical in the middle (20,20)-(40,20)-(40,40)-(60,40)");
	i1(32, 3); pline(4, b); grab("K1B", 10, 10, 60, 40);
	begin("K1C", "XOR polyline: vertical upward first (20,40)-(20,20)-(40,20)");
	i1(32, 3); pline(3, c); grab("K1C", 10, 10, 60, 40);
	begin("K1D", "XOR closed square, 5 points");
	i1(32, 3); pline(5, d); grab("K1D", 10, 10, 60, 40);
	begin("K1E", "XOR polyline, two diagonals (control)");
	i1(32, 3); pline(3, e); grab("K1E", 10, 10, 60, 40);
	begin("K1F", "XOR single vertical line (last segment, control)");
	i1(32, 3); pline(2, f); grab("K1F", 10, 10, 60, 40);
	begin("K1G", "XOR vertical non-last, dashed (vsl_type 3)");
	i1(32, 3); i1(15, 3); pline(3, a); grab("K1G", 10, 10, 60, 40);
}

static void k2(void)
{
	static const int16_t ends[][4] = {
		{ 0, 5, 79, 47 }, { 5, 0, 47, 79 }, { 0, 60, 120, 40 }, { 10, 127, 70, 0 },
		{ 0, 0, 127, 127 }, { 0, 31, 127, 33 }, { 127, 90, 3, 100 }, { 100, 127, 120, 0 },
		{ 1, 50, 25, 61 }, { 33, 2, 59, 25 },
	};
	int i;

	begin("K2A", "clip on (30,30)-(90,90), 10 lines crossing it, solid");
	clip(1, 30, 30, 90, 90);
	for (i = 0; i < 10; i++) line2(ends[i][0], ends[i][1], ends[i][2], ends[i][3]);
	grab("K2A", 0, 0, 128, 128);
	begin("K2B", "the same lines, clip on, dash-dot (vsl_type 4)");
	clip(1, 30, 30, 90, 90); i1(15, 4);
	for (i = 0; i < 10; i++) line2(ends[i][0], ends[i][1], ends[i][2], ends[i][3]);
	grab("K2B", 0, 0, 128, 128);
	begin("K2C", "the same lines, clip on, user style 0xF39C (vsl_type 7)");
	clip(1, 30, 30, 90, 90); i1(15, 7); i1(113, (int16_t)0xF39C);
	for (i = 0; i < 10; i++) line2(ends[i][0], ends[i][1], ends[i][2], ends[i][3]);
	grab("K2C", 0, 0, 128, 128);
	begin("K2D", "clip on (30,30)-(90,90), vertical dashed (60,5)-(60,120) and (70,120)-(70,5), horizontal dashed (5,50)-(120,50)");
	clip(1, 30, 30, 90, 90); i1(15, 4);
	line2(60, 5, 60, 120); line2(70, 120, 70, 5); line2(5, 50, 120, 50);
	grab("K2D", 0, 0, 128, 128);
	begin("K2E", "clip OFF: diagonal (600,100)-(680,104) past the right edge");
	line2(600, 100, 680, 104);
	grab("K2E", 0, 96, 640, 14);
	begin("K2F", "clip OFF: horizontal (600,120)-(700,120) past the right edge");
	line2(600, 120, 700, 120);
	grab("K2F", 0, 118, 640, 5);
	begin("K2G", "clip OFF: diagonal (-20,140)-(40,150) past the left edge");
	line2(-20, 140, 40, 150);
	grab("K2G", 0, 136, 640, 18);
	begin("K2H", "clip OFF: horizontal (-30,170)-(20,170) past the left edge");
	line2(-30, 170, 20, 170);
	grab("K2H", 0, 168, 640, 4);
	begin("K2I", "clip ON (0,0)-(639,479): diagonal (600,200)-(680,204) and horizontal (600,210)-(700,210)");
	clip(1, 0, 0, 639, 479);
	line2(600, 200, 680, 204); line2(600, 210, 700, 210);
	grab("K2I", 0, 196, 640, 18);
}

static void k3(void)
{
	begin("K3A", "128-line fan, solid, from the cell centre");
	fan(0); grab("K3A", 0, 0, 624, 480);
	begin("K3B", "128-line fan, solid, from the far end");
	fan(1); grab("K3B", 0, 0, 624, 480);
	begin("K3C", "128-line fan, dash-dot-dot (vsl_type 6), from the centre");
	i1(15, 6); fan(0); grab("K3C", 0, 0, 624, 480);
	begin("K3D", "128-line fan, user style 0xF39C, from the far end");
	i1(15, 7); i1(113, (int16_t)0xF39C); fan(1); grab("K3D", 0, 0, 624, 480);
}

static void k4(void)
{
	static const int16_t one[] = { 50, 50 };
	static const int16_t two[] = { 50, 50, 90, 70 };
	static const int16_t twov[] = { 50, 50, 50, 80 };
	static const int16_t twoh[] = { 40, 60, 90, 60 };
	static const int16_t three[] = { 50, 50, 90, 70, 60, 90 };

	begin("K4A", "v_fillarea 1 point (50,50), perimeter on");
	fillarea(1, one); grab("K4A", 30, 30, 80, 80);
	begin("K4B", "v_fillarea 2 points (50,50)-(90,70), perimeter on");
	fillarea(2, two); grab("K4B", 30, 30, 80, 80);
	begin("K4C", "v_fillarea 2 points (50,50)-(90,70), perimeter off");
	i1(104, 0); fillarea(2, two); grab("K4C", 30, 30, 80, 80);
	begin("K4D", "v_fillarea 2 points vertical (50,50)-(50,80), perimeter off");
	i1(104, 0); fillarea(2, twov); grab("K4D", 30, 30, 80, 80);
	begin("K4E", "v_fillarea 2 points (50,50)-(90,70), XOR, perimeter on");
	i1(32, 3); fillarea(2, two); grab("K4E", 30, 30, 80, 80);
	begin("K4F", "v_fillarea 1 point, perimeter off");
	i1(104, 0); fillarea(1, one); grab("K4F", 30, 30, 80, 80);
	begin("K4G", "v_fillarea 2 points horizontal (40,60)-(90,60), perimeter on");
	fillarea(2, twoh); grab("K4G", 30, 30, 80, 80);
	begin("K4H", "v_fillarea 3 points (control), perimeter on, colour 2");
	i1(25, 2); fillarea(3, three); grab("K4H", 30, 30, 80, 80);
}

static void k5(void)
{
	int i;

	begin("K5A", "v_pmarker 1030 points: 0..1023 at (20,20), 1024..1029 at (40+10k,20); plus, height default");
	i1(18, 2);
	for (i = 0; i < 1024; i++) { ptsin[2 * i] = 20; ptsin[2 * i + 1] = 20; }
	for (i = 0; i < 6; i++) { ptsin[2 * (1024 + i)] = (int16_t)(40 + 10 * i); ptsin[2 * (1024 + i) + 1] = 20; }
	vdi(7, 0, 1030, 0);
	emit("   K5A contrl[1] after the call %d\n", contrl[1]);
	grab("K5A", 0, 0, 120, 40);
	begin("K5B", "v_pmarker 1025 points: 0..1023 at (20,20), 1024 at (60,20)");
	i1(18, 2);
	for (i = 0; i < 1024; i++) { ptsin[2 * i] = 20; ptsin[2 * i + 1] = 20; }
	ptsin[2048] = 60; ptsin[2049] = 20;
	vdi(7, 0, 1025, 0);
	grab("K5B", 0, 0, 120, 40);
	begin("K5C", "v_pline 1030 points: 0..1022 at (20,60), 1023 (40,60), 1024.. (40,80),(80,80),...");
	for (i = 0; i < 1023; i++) { ptsin[2 * i] = 20; ptsin[2 * i + 1] = 60; }
	ptsin[2046] = 40; ptsin[2047] = 60;
	for (i = 0; i < 6; i++) { ptsin[2 * (1024 + i)] = (int16_t)(40 + 20 * i); ptsin[2 * (1024 + i) + 1] = 80; }
	vdi(6, 0, 1030, 0);
	emit("   K5C contrl[1] after the call %d\n", contrl[1]);
	grab("K5C", 0, 50, 160, 40);
	begin("K5D", "v_fillarea 1030 points: 0..1021 (100,100), 1022 (140,100), 1023 (120,130), 1024.. (200+10k,200)");
	for (i = 0; i < 1022; i++) { ptsin[2 * i] = 100; ptsin[2 * i + 1] = 100; }
	ptsin[2044] = 140; ptsin[2045] = 100; ptsin[2046] = 120; ptsin[2047] = 130;
	for (i = 0; i < 6; i++) { ptsin[2 * (1024 + i)] = (int16_t)(200 + 10 * i); ptsin[2 * (1024 + i) + 1] = (int16_t)(200 + 7 * i); }
	vdi(9, 0, 1030, 0);
	grab("K5D", 90, 90, 180, 170);
}

static void k6(void)
{
	static uint16_t a[16], big[256];
	int i, p;

	for (i = 0; i < 16; i++)
		a[i] = (uint16_t)((0x8000u >> i) | (1u << i) | (i & 1 ? 0x0F00 : 0x0000));
	begin("K6A", "vsf_udpat 16 words (plane 0 = A), FIS_USER, v_bar 20..83 x 20..51, colour 2");
	udpat(16, a); i1(23, 4); i1(25, 2); bar(20, 20, 83, 51); grab("K6A", 16, 16, 72, 40);
	begin("K6B", "vsf_udpat 256 words: plane 0 = A, planes 1..15 = 0xFFFF");
	for (p = 0; p < 16; p++) for (i = 0; i < 16; i++) big[16 * p + i] = p ? 0xFFFF : a[i];
	udpat(256, big); i1(23, 4); i1(25, 2); bar(20, 20, 83, 51); grab("K6B", 16, 16, 72, 40);
	begin("K6C", "vsf_udpat 256 words: plane 0 = 0, planes 1..15 = A");
	for (p = 0; p < 16; p++) for (i = 0; i < 16; i++) big[16 * p + i] = p ? a[i] : 0;
	udpat(256, big); i1(23, 4); i1(25, 2); bar(20, 20, 83, 51); grab("K6C", 16, 16, 72, 40);
	begin("K6D", "vsf_udpat 16 words A, then 64 words (4 planes, invalid here) of 0xFFFF");
	udpat(16, a); for (i = 0; i < 64; i++) big[i] = 0xFFFF; udpat(64, big);
	i1(23, 4); i1(25, 2); bar(20, 20, 83, 51); grab("K6D", 16, 16, 72, 40);
	begin("K6E", "vsf_udpat 256 words (plane 0 = A, rest 0xFFFF), transparent mode, colour 3, v_fillarea triangle");
	for (p = 0; p < 16; p++) for (i = 0; i < 16; i++) big[16 * p + i] = p ? 0xFFFF : a[i];
	udpat(256, big); i1(23, 4); i1(25, 3); i1(32, 2);
	{ static const int16_t t[] = { 20, 20, 83, 30, 40, 51 }; fillarea(3, t); }
	grab("K6E", 16, 16, 72, 40);
	begin("K6F", "vsf_udpat 256 words (plane 0 = A, rest 0xFFFF), then style back to solid and to user again");
	udpat(256, big); i1(23, 1); i1(23, 4); i1(25, 2); bar(20, 20, 83, 51); grab("K6F", 16, 16, 72, 40);
}

static void k8(void)
{
	static const int16_t box[] = { 20, 20, 120, 20, 120, 100, 20, 100, 20, 20 };
	int i, n;

	begin("K8A", "contourfill inside a black box, boundary colour 1, solid colour 2");
	pline(5, box); i1(25, 2); contour(70, 60, 1); grab("K8A", 10, 10, 120, 100);
	begin("K8B", "the same, INTIN[0] = -1 (seed colour)");
	pline(5, box); i1(25, 2); contour(70, 60, -1); grab("K8B", 10, 10, 120, 100);
	begin("K8C", "comb: box with 40 teeth from the top, pattern fill (style 2 index 5), boundary 1");
	pline(5, box);
	for (i = 0; i < 40; i++) line2(22 + 2 * i + (i > 20), 20, 22 + 2 * i + (i > 20), 80);
	i1(23, 2); i1(24, 5); i1(25, 2); contour(70, 90, 1); grab("K8C", 10, 10, 120, 100);
	begin("K8D", "spiral of lines, solid colour 3, seed -1");
	{
		static const int16_t sp[] = { 30, 30, 110, 30, 110, 95, 35, 95, 35, 40, 100, 40,
		                              100, 85, 45, 85, 45, 50, 90, 50, 90, 75, 55, 75, 55, 60, 80, 60 };
		pline(14, sp);
	}
	i1(25, 3); contour(25, 25, -1); grab("K8D", 10, 10, 120, 100);
	begin("K8E", "dot lattice 600x120 (dots every 3 px), contourfill from below, solid colour 2, boundary 1");
	n = 0;
	for (i = 0; i < 200 * 40; i++) {
		pts[2 * n] = (int16_t)(20 + 3 * (i % 200));
		pts[2 * n + 1] = (int16_t)(200 + 3 * (i / 200));
		if (++n == 1000) { int j; for (j = 0; j < 2 * n; j++) ptsin[j] = pts[j]; vdi(7, 0, (int16_t)n, 0); n = 0; }
	}
	if (n) { int j; for (j = 0; j < 2 * n; j++) ptsin[j] = pts[j]; vdi(7, 0, (int16_t)n, 0); }
	i1(25, 2); contour(320, 340, 1); grab("K8E", 0, 190, 640, 160);
	begin("K8F", "contourfill inside the box with clip (40,40)-(90,80)");
	pline(5, box); i1(25, 2); clip(1, 40, 40, 90, 80); contour(70, 60, 1); grab("K8F", 10, 10, 120, 100);
	begin("K8G", "contourfill with the seed on the boundary (20,60), boundary 1");
	pline(5, box); i1(25, 2); contour(20, 60, 1); grab("K8G", 10, 10, 120, 100);
	begin("K8H", "contourfill XOR mode, hatch (style 3 index 2), boundary 1");
	pline(5, box); i1(23, 3); i1(24, 2); i1(25, 2); i1(32, 3); contour(70, 60, 1); grab("K8H", 10, 10, 120, 100);
	begin("K8I", "contourfill pattern (style 2 index 2) with seed colour -1");
	pline(5, box); i1(23, 2); i1(24, 2); i1(25, 2); contour(70, 60, -1); grab("K8I", 10, 10, 120, 100);
	begin("K8J", "contourfill boundary colour 16 (> 15, 256 pens), solid colour 2");
	pline(5, box); i1(25, 2); contour(70, 60, 16); grab("K8J", 10, 10, 120, 100);
}

static void k10(void)
{
	begin("K10", "v_get_pixel inside and outside, after painting (0,11) red, (639,10) green, (60,11) blue");
	i1(25, 2); recfl(0, 11, 0, 11);
	i1(25, 3); recfl(639, 10, 639, 10);
	i1(25, 4); recfl(60, 11, 60, 11);
	emit("   K10 (0,11)=%04x (639,10)=%04x (60,11)=%04x (5,5)=%04x\n",
	     (uint16_t)getpix(0, 11), (uint16_t)getpix(639, 10), (uint16_t)getpix(60, 11), (uint16_t)getpix(5, 5));
	emit("   K10 (640,10)=%04x (-1,11)=%04x (700,10)=%04x (-580,12)=%04x\n",
	     (uint16_t)getpix(640, 10), (uint16_t)getpix(-1, 11), (uint16_t)getpix(700, 10), (uint16_t)getpix(-580, 12));
	grab("K10", 0, 9, 640, 4);
}

static void k11(void)
{
	begin("K11A", "v_arc centre (100,100) r 20, 0..3600, line width 9, clip (121,0)-(300,200)");
	p1(16, 9, 0); clip(1, 121, 0, 300, 200);
	intin[0] = 0; intin[1] = 3600;
	ptsin[0] = 100; ptsin[1] = 100; ptsin[2] = ptsin[3] = ptsin[4] = ptsin[5] = 0; ptsin[6] = 20; ptsin[7] = 0;
	vdi(11, 2, 4, 2);
	grab("K11A", 60, 60, 80, 80);
	begin("K11B", "v_ellarc centre (100,100) 30x20, 0..1800, width 11, clip (0,121)-(300,200)");
	p1(16, 11, 0); clip(1, 0, 121, 300, 200);
	intin[0] = 0; intin[1] = 1800;
	ptsin[0] = 100; ptsin[1] = 100; ptsin[2] = 30; ptsin[3] = 20;
	vdi(11, 6, 2, 2);
	grab("K11B", 50, 60, 100, 80);
	begin("K11C", "v_arc as K11A but clip (115,0)-(300,200) (cuts the curve, control)");
	p1(16, 9, 0); clip(1, 115, 0, 300, 200);
	intin[0] = 0; intin[1] = 3600;
	ptsin[0] = 100; ptsin[1] = 100; ptsin[2] = ptsin[3] = ptsin[4] = ptsin[5] = 0; ptsin[6] = 20; ptsin[7] = 0;
	vdi(11, 2, 4, 2);
	grab("K11C", 60, 60, 80, 80);
	begin("K11D", "v_arc r 20 at (100,100), width 1, arrowed ends, 0..900, clip (121,0)-(300,200)");
	i2(108, 1, 1); clip(1, 121, 0, 300, 200);
	intin[0] = 0; intin[1] = 900;
	ptsin[0] = 100; ptsin[1] = 100; ptsin[2] = ptsin[3] = ptsin[4] = ptsin[5] = 0; ptsin[6] = 20; ptsin[7] = 0;
	vdi(11, 2, 4, 2);
	grab("K11D", 60, 60, 80, 80);
}

/* ---- V19: memory -> memory below nine planes, plain op vs op | 0x10 ------
 * (TASK/V19 acceptance, appended after K11 so cases 1..54 stay byte-identical.)
 * The raster table (vdi-v18-raster.tsv) cannot set a fill pattern, so this
 * probe sets a user-defined, non-solid one (vsf_udpat, interior 4) and then
 * copies a rectangle between two memory forms of 1/2/4/8 planes with vro_cpyfm,
 * once with the plain op and once with op | 0x10 (PAT_FLAG).  The destination
 * form plus 32 guard words on both sides is written to VDPROBE.BIN as one
 * record per copy (x = y = 0, w = number of words, h = 1; the words are the
 * WORD values of the form, written big-endian).  EmuTOS strips PAT_FLAG for
 * memory forms (vdi_raster.c setup_pattern, p_addr = NULL), so within a side the
 * two records must be equal; across the sides every record must be equal. */
#define PF_GUARD 32
#define PF_WORDS (PF_GUARD + 256 + PF_GUARD)
static uint16_t pf_src[PF_WORDS], pf_dst[PF_WORDS];

static void pf_fill(uint16_t *buf, uint32_t seed)
{
	int i;

	/* WORD values, not bytes: a planar form is a sequence of WORDs, in the
	 * native byte order of each side (m68k big-endian, NewTOS aarch64
	 * little-endian), pixel 0 = the WORD's most significant bit. */
	for (i = 0; i < PF_WORDS; i++) {
		seed = seed * 1103515245UL + 12345UL;
		buf[i] = (uint16_t)(seed >> 16);
	}
	for (i = 0; i < PF_GUARD; i++)
		buf[i] = buf[PF_WORDS - PF_GUARD + i] = 0xA5A5;
}

static void pf_record(const char *name, const uint16_t *buf)
{
	uint8_t hd[19];
	int i;

	memset(hd, 0, sizeof hd);
	hd[0] = 'C';
	put16(hd + 1, caseno);
	memcpy(hd + 3, name, strlen(name) < 8 ? strlen(name) : 8);
	put16(hd + 15, PF_WORDS); put16(hd + 17, 1);
	if (binfh < 0)
		return;
	Fwrite((short)binfh, 19L, hd);
	for (i = 0; i < PF_WORDS; i++) {
		uint8_t w2[2];

		put16(w2, buf[i]);		/* the WORD value, big-endian in the file */
		Fwrite((short)binfh, 2L, w2);
	}
}

static void pf_copy(const char *name, int planes, int geo, int op, int flag)
{
	MFDB src, dst;
	int sx1, sy1, sx2, sy2, dx, dy;

	begin(name, geo ? "pattern set, mem->mem op, shifted left" : "pattern set, mem->mem op, shifted right");
	{
		static const uint16_t pat[16] = {
			0x8001, 0x4002, 0x2004, 0x1008, 0x0810, 0x0420, 0x0240, 0x0180,
			0xF00F, 0x0FF0, 0xAAAA, 0x5555, 0xCCCC, 0x3333, 0xFF00, 0x00FF };
		udpat(16, pat);
		i1(23, 4);			/* vsf_interior user-defined */
		i1(25, 2);
	}
	memset(&src, 0, sizeof src);
	memset(&dst, 0, sizeof dst);
	pf_fill(pf_src, 0x1234ABCDUL + (uint32_t)planes);
	pf_fill(pf_dst, 0x9876FEDCUL + (uint32_t)planes);
	src.fd_addr = pf_src + PF_GUARD;
	dst.fd_addr = pf_dst + PF_GUARD;
	src.fd_w = dst.fd_w = 64;
	src.fd_h = dst.fd_h = 8;
	src.fd_wdwidth = dst.fd_wdwidth = 4;
	src.fd_stand = dst.fd_stand = 0;
	src.fd_nplanes = dst.fd_nplanes = (int16_t)planes;
	if (geo == 0) { sx1 = 3; sy1 = 1; sx2 = 50; sy2 = 6; dx = 11; dy = 2; }
	else          { sx1 = 20; sy1 = 0; sx2 = 60; sy2 = 7; dx = 5; dy = 0; }
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 109;
	contrl[1] = 4;
	contrl[3] = 1;
	contrl[6] = handle;
	set_mfdb_ptr(0, &src);
	set_mfdb_ptr(1, &dst);
	intin[0] = (int16_t)(op | (flag ? 0x10 : 0));
	ptsin[0] = (int16_t)sx1; ptsin[1] = (int16_t)sy1;
	ptsin[2] = (int16_t)sx2; ptsin[3] = (int16_t)sy2;
	ptsin[4] = (int16_t)dx; ptsin[5] = (int16_t)dy;
	ptsin[6] = (int16_t)(dx + sx2 - sx1); ptsin[7] = (int16_t)(dy + sy2 - sy1);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	pf_record(name, pf_dst);
}

static void k12(void)
{
	static const uint8_t ops[] = { 0, 1, 3, 4, 6, 7, 10, 12, 15 };
	static const uint8_t pl[] = { 1, 2, 4, 8 };
	char nm[9];
	int p, g, o, f;

	for (p = 0; p < 4; p++)
		for (g = 0; g < 2; g++)
			for (o = 0; o < (int)sizeof ops; o++)
				for (f = 0; f < 2; f++) {
					nm[0] = 'P'; nm[1] = (char)('0' + pl[p]); nm[2] = (char)('a' + g);
					nm[3] = "0123456789ABCDEF"[ops[o] >> 4]; nm[4] = "0123456789ABCDEF"[ops[o] & 15];
					nm[5] = f ? 'f' : 'p'; nm[6] = 0;
					pf_copy(nm, pl[p], g, ops[o], f);
				}
}

/* ---- k13: memory forms whose fd_w does not match the row (TASK/V26) ------
 * EmuTOS addresses a memory form by fd_wdwidth alone (vdi_raster.c setup_info,
 * vro_cpyfm16 / vrt_cpyfm16 / the planar copy); fd_w and fd_h are never read, so a
 * block wider than the form's row runs on into the next row.  Every block here
 * stays inside the form's fd_h rows (EmuTOS must write nothing foreign); both
 * forms are recorded whole with WK_GUARD guard words before and behind
 * (0xA5A5), the destination as "<name>d", the source as "<name>s". */
#define WK_GUARD 32
#define WK_MAX 320
static uint16_t wk_src[WK_GUARD + WK_MAX + WK_GUARD], wk_dst[WK_GUARD + WK_MAX + WK_GUARD];

struct wk {
	int opcode;			/* 109 vro_cpyfm, 121 vrt_cpyfm */
	int mode;			/* write mode / logic op */
	int sp, sw, sh, swd;		/* source planes, fd_w, fd_h, fd_wdwidth */
	int dp, dw, dh, dwd;		/* destination */
	int sx, sy, bw, bh, dx, dy;	/* block size, source and destination origin */
};

static void wk_fill(uint16_t *buf, int n, uint32_t seed)
{
	int i;

	for (i = 0; i < WK_GUARD + n + WK_GUARD; i++) {
		seed = seed * 1103515245UL + 12345UL;
		buf[i] = (uint16_t)(seed >> 16);
	}
	for (i = 0; i < WK_GUARD; i++)
		buf[i] = buf[WK_GUARD + n + i] = 0xA5A5;
}

static void wk_record(const char *name, char tag, const uint16_t *buf, int n)
{
	uint8_t hd[19];
	int i, total = WK_GUARD + n + WK_GUARD;

	caseno++;
	memset(hd, 0, sizeof hd);
	hd[0] = 'C';
	put16(hd + 1, caseno);
	memcpy(hd + 3, name, strlen(name) < 7 ? strlen(name) : 7);
	hd[3 + (strlen(name) < 7 ? strlen(name) : 7)] = (uint8_t)tag;
	put16(hd + 15, total); put16(hd + 17, 1);
	if (binfh < 0)
		return;
	Fwrite((short)binfh, 19L, hd);
	for (i = 0; i < total; i++) {
		uint8_t w2[2];

		put16(w2, buf[i]);
		Fwrite((short)binfh, 2L, w2);
	}
}

static void wk_run(const char *name, const char *what, const struct wk *c)
{
	MFDB src, dst;
	int ns = c->swd * c->sp * c->sh, nd = c->dwd * c->dp * c->dh;

	begin(name, what);
	wk_fill(wk_src, ns, 0x31415926UL + (uint32_t)c->sp * 7UL + (uint32_t)c->swd);
	wk_fill(wk_dst, nd, 0x27182818UL + (uint32_t)c->dp * 13UL + (uint32_t)c->dwd);
	memset(&src, 0, sizeof src);
	memset(&dst, 0, sizeof dst);
	src.fd_addr = wk_src + WK_GUARD;
	src.fd_w = (int16_t)c->sw; src.fd_h = (int16_t)c->sh; src.fd_wdwidth = (int16_t)c->swd;
	src.fd_nplanes = (int16_t)c->sp;
	dst.fd_addr = wk_dst + WK_GUARD;
	dst.fd_w = (int16_t)c->dw; dst.fd_h = (int16_t)c->dh; dst.fd_wdwidth = (int16_t)c->dwd;
	dst.fd_nplanes = (int16_t)c->dp;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = (int16_t)c->opcode;
	contrl[1] = 4;
	contrl[3] = (int16_t)(c->opcode == 121 ? 3 : 1);
	contrl[6] = handle;
	set_mfdb_ptr(0, &src);
	set_mfdb_ptr(1, &dst);
	intin[0] = (int16_t)c->mode;
	intin[1] = 1;			/* vrt_cpyfm colours (ignored by vro_cpyfm) */
	intin[2] = 0;
	ptsin[0] = (int16_t)c->sx; ptsin[1] = (int16_t)c->sy;
	ptsin[2] = (int16_t)(c->sx + c->bw - 1); ptsin[3] = (int16_t)(c->sy + c->bh - 1);
	ptsin[4] = (int16_t)c->dx; ptsin[5] = (int16_t)c->dy;
	ptsin[6] = (int16_t)(c->dx + c->bw - 1); ptsin[7] = (int16_t)(c->dy + c->bh - 1);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	wk_record(name, 'd', wk_dst, nd);
	wk_record(name, 's', wk_src, ns);
}

static void k13(void)
{
	static const uint8_t ops[2] = { 3, 6 };
	static const uint8_t modes[2] = { 1, 3 };
	static const int8_t vdx[2] = { 0, -4 }, vdy[2] = { 1, 2 };
	char nm[9];
	struct wk c;
	int v, o, p;

	/* a: 16 planes mem->mem, destination row 16 px, source 48 px wide; the block runs on
	 * into the following rows; variant 1 starts left of the form (fd_h 5 keeps the end inside) */
	for (v = 0; v < 2; v++)
		for (o = 0; o < 2; o++) {
			memset(&c, 0, sizeof c);
			c.opcode = 109; c.mode = ops[o];
			c.sp = 16; c.sw = 48; c.sh = 4; c.swd = 3;
			c.dp = 16; c.dw = 40; c.dh = v ? 5 : 4; c.dwd = 1;
			c.bw = 40; c.bh = 1; c.dx = vdx[v]; c.dy = vdy[v];
			nm[0] = 'W'; nm[1] = 'a'; nm[2] = (char)('0' + v); nm[3] = (char)('0' + ops[o]); nm[4] = 0;
			wk_run(nm, "16 planes mem->mem, dest fd_w 40 fd_wdwidth 1, block 40x1 wraps", &c);
		}
	/* b: source fd_w 40, fd_wdwidth 1, block 40x1 from (0,1) into a 48x1 destination */
	for (o = 0; o < 2; o++) {
		memset(&c, 0, sizeof c);
		c.opcode = 109; c.mode = ops[o];
		c.sp = 16; c.sw = 40; c.sh = 4; c.swd = 1;
		c.dp = 16; c.dw = 48; c.dh = 1; c.dwd = 3;
		c.sy = 1; c.bw = 40; c.bh = 1;
		nm[0] = 'W'; nm[1] = 'b'; nm[2] = (char)('0' + ops[o]); nm[3] = 0;
		wk_run(nm, "16 planes mem->mem, source fd_w 40 fd_wdwidth 1, block 40x1 from (0,1)", &c);
	}
	/* c: destination fd_w 10, fd_wdwidth 1, block 16x1 (padding columns 10..15) */
	for (o = 0; o < 2; o++) {
		memset(&c, 0, sizeof c);
		c.opcode = 109; c.mode = ops[o];
		c.sp = 16; c.sw = 16; c.sh = 2; c.swd = 1;
		c.dp = 16; c.dw = 10; c.dh = 2; c.dwd = 1;
		c.bw = 16; c.bh = 1;
		nm[0] = 'W'; nm[1] = 'c'; nm[2] = (char)('0' + ops[o]); nm[3] = 0;
		wk_run(nm, "16 planes mem->mem, dest fd_w 10 fd_wdwidth 1, block 16x1", &c);
	}
	/* d: vrt_cpyfm 1 plane -> 16 plane memory as in a */
	for (v = 0; v < 2; v++)
		for (o = 0; o < 2; o++) {
			memset(&c, 0, sizeof c);
			c.opcode = 121; c.mode = modes[o];
			c.sp = 1; c.sw = 48; c.sh = 4; c.swd = 3;
			c.dp = 16; c.dw = 40; c.dh = v ? 5 : 4; c.dwd = 1;
			c.bw = 40; c.bh = 1; c.dx = vdx[v]; c.dy = vdy[v];
			nm[0] = 'W'; nm[1] = 'd'; nm[2] = (char)('0' + v); nm[3] = (char)('0' + modes[o]); nm[4] = 0;
			wk_run(nm, "vrt_cpyfm 1 plane -> 16 plane memory, dest fd_w 40 fd_wdwidth 1, block 40x1 wraps", &c);
		}
	/* e: planar 1 and 4 planes mem->mem, destination fd_w 40, fd_wdwidth 2, fd_h 6 */
	for (p = 0; p < 2; p++)
		for (v = 0; v < 2; v++)
			for (o = 0; o < 2; o++) {
				int pl = p ? 4 : 1;

				memset(&c, 0, sizeof c);
				c.opcode = 109; c.mode = ops[o];
				c.sp = pl; c.sw = 48; c.sh = 4; c.swd = 3;
				c.dp = pl; c.dw = 40; c.dh = 6; c.dwd = 2;
				c.bw = 40; c.bh = 1; c.dx = v ? 5 : 0; c.dy = v ? 2 : 1;
				nm[0] = 'W'; nm[1] = 'e'; nm[2] = (char)('0' + pl); nm[3] = (char)('0' + v);
				nm[4] = (char)('0' + ops[o]); nm[5] = 0;
				wk_run(nm, "planar mem->mem, dest fd_w 40 fd_wdwidth 2 fd_h 6, block 40x1 wraps", &c);
			}
}

/* K14 (TASK/V21 acceptance): the seed queue of v_contourfill overflows.  EmuTOS ends the whole fill when its
 * 1024-entry queue is full (vdi/vdi_fill.c get_seed), leaving a partial fill.  Dots come from a fixed LCG so
 * both sides draw the same field. */
static uint32_t k14_seed;

static int k14_rand(void)
{
	k14_seed = k14_seed * 1103515245UL + 12345UL;
	return (int)((k14_seed >> 16) & 0x7fff);
}

/* dots on rows y0..y1 over the full width; pct = density in percent, or (step > 0) every step-th column (x % step == 1) */
static void k14_dots(int y0, int y1, int pct, int step)
{
	int16_t pts[2000];
	int n = 0, y, x, j;

	k14_seed = 12345UL;
	for (y = y0; y <= y1; y++)
		for (x = 0; x < 640; x++) {
			int dot = step ? (x % step == 1) : (k14_rand() % 100 < pct);
			if (!dot)
				continue;
			pts[2 * n] = (int16_t)x;
			pts[2 * n + 1] = (int16_t)y;
			if (++n == 1000) {
				for (j = 0; j < 2 * n; j++) ptsin[j] = pts[j];
				vdi(7, 0, (int16_t)n, 0);
				n = 0;
			}
		}
	if (n) {
		for (j = 0; j < 2 * n; j++) ptsin[j] = pts[j];
		vdi(7, 0, (int16_t)n, 0);
	}
}

static void k14(void)
{
	begin("K14A", "queue overflow: 25% random dots on rows 100..104, contourfill from (0,50), solid colour 2, boundary 1");
	k14_dots(100, 104, 25, 0);
	i1(25, 2); contour(0, 50, 1); grab("K14A", 0, 40, 640, 80);
	begin("K14B", "queue overflow: 10% random dots on rows 100..139, contourfill from below (0,300), solid colour 2, boundary 1");
	k14_dots(100, 139, 10, 0);
	i1(25, 2); contour(0, 300, 1); grab("K14B", 0, 0, 640, 320);
	begin("K14C", "control, no overflow: dot on every 2nd column of rows 100..299 (queue 321), contourfill from (0,50)");
	k14_dots(100, 299, 0, 2);
	i1(25, 2); contour(0, 50, 1); grab("K14C", 0, 40, 640, 280);
}

/* ---- k15: closed GDP curves that degenerate to two points (TASK/V22 acceptance) -----------------------------
 * clc_arc drops duplicate points, so a tiny radius leaves 2 (or 1+1) points for polygon(); V22 removed the
 * "at least 3 points" return, so these draw like EmuTOS.  Solid fill, colour 2; the window is (30,30)-(110,110). */
static void gdp15(int sub, int npts, int nint, int xc, int yc, int a2, int a3, int a6, int ba, int ea)
{
	int i;

	for (i = 0; i < 8; i++)
		ptsin[i] = 0;
	ptsin[0] = (int16_t)xc; ptsin[1] = (int16_t)yc;
	ptsin[2] = (int16_t)a2; ptsin[3] = (int16_t)a3;
	ptsin[4] = (int16_t)a2; ptsin[6] = (int16_t)a6;
	intin[0] = (int16_t)ba; intin[1] = (int16_t)ea;
	vdi(11, (int16_t)sub, (int16_t)npts, (int16_t)nint);
}

static void k15_solid(int perim, int mode)
{
	i1(23, 1);
	i1(25, 2);
	i1(104, perim);
	if (mode)
		i1(32, mode);
}

static void k15(void)
{
	/* v_circle: radius in ptsin[4] */
	begin("K15A", "v_circle r=0 at (60,60), solid colour 2, perimeter on");
	k15_solid(1, 0); gdp15(4, 3, 0, 60, 60, 0, 0, 0, 0, 0); grab("K15A", 30, 30, 80, 80);
	begin("K15B", "v_circle r=1 at (60,60), solid colour 2, perimeter on");
	k15_solid(1, 0); gdp15(4, 3, 0, 60, 60, 1, 0, 0, 0, 0); grab("K15B", 30, 30, 80, 80);
	begin("K15C", "v_circle r=2 at (60,60), solid colour 2, perimeter on");
	k15_solid(1, 0); gdp15(4, 3, 0, 60, 60, 2, 0, 0, 0, 0); grab("K15C", 30, 30, 80, 80);
	begin("K15D", "v_circle r=0 at (60,60), solid colour 2, perimeter off");
	k15_solid(0, 0); gdp15(4, 3, 0, 60, 60, 0, 0, 0, 0, 0); grab("K15D", 30, 30, 80, 80);
	begin("K15E", "v_circle r=1 at (60,60), solid colour 2, perimeter off");
	k15_solid(0, 0); gdp15(4, 3, 0, 60, 60, 1, 0, 0, 0, 0); grab("K15E", 30, 30, 80, 80);
	/* v_ellipse: xr in ptsin[2], yr in ptsin[3] */
	begin("K15F", "v_ellipse xr=0 yr=5 at (60,60), perimeter on");
	k15_solid(1, 0); gdp15(5, 2, 0, 60, 60, 0, 5, 0, 0, 0); grab("K15F", 30, 30, 80, 80);
	begin("K15G", "v_ellipse xr=5 yr=0 at (60,60), perimeter on");
	k15_solid(1, 0); gdp15(5, 2, 0, 60, 60, 5, 0, 0, 0, 0); grab("K15G", 30, 30, 80, 80);
	begin("K15H", "v_ellipse xr=0 yr=5 at (60,60), perimeter off");
	k15_solid(0, 0); gdp15(5, 2, 0, 60, 60, 0, 5, 0, 0, 0); grab("K15H", 30, 30, 80, 80);
	begin("K15I", "v_ellipse xr=0 yr=0 at (60,60), perimeter on, XOR");
	k15_solid(1, 3); gdp15(5, 2, 0, 60, 60, 0, 0, 0, 0, 0); grab("K15I", 30, 30, 80, 80);
	begin("K15J", "v_ellipse xr=1 yr=6 at (60,60), perimeter off");
	k15_solid(0, 0); gdp15(5, 2, 0, 60, 60, 1, 6, 0, 0, 0); grab("K15J", 30, 30, 80, 80);
	/* v_pieslice: radius in ptsin[6], angles in intin (tenths of a degree) */
	begin("K15K", "v_pieslice r=0 at (60,60), 0..900, perimeter on");
	k15_solid(1, 0); gdp15(3, 4, 2, 60, 60, 0, 0, 0, 0, 900); grab("K15K", 30, 30, 80, 80);
	begin("K15L", "v_pieslice r=1 at (60,60), 0..1800, perimeter on");
	k15_solid(1, 0); gdp15(3, 4, 2, 60, 60, 0, 0, 1, 0, 1800); grab("K15L", 30, 30, 80, 80);
	begin("K15M", "v_pieslice r=2 at (60,60), 0..2700, perimeter off");
	k15_solid(0, 0); gdp15(3, 4, 2, 60, 60, 0, 0, 2, 0, 2700); grab("K15M", 30, 30, 80, 80);
	begin("K15N", "v_pieslice r=0 at (60,60), 0..0, perimeter on");
	k15_solid(1, 0); gdp15(3, 4, 2, 60, 60, 0, 0, 0, 0, 0); grab("K15N", 30, 30, 80, 80);
	/* v_ellpie: xr, yr in ptsin[2], [3] */
	begin("K15O", "v_ellpie xr=0 yr=5 at (60,60), 0..900, perimeter on");
	k15_solid(1, 0); gdp15(7, 2, 2, 60, 60, 0, 5, 0, 0, 900); grab("K15O", 30, 30, 80, 80);
	begin("K15P", "v_ellpie xr=1 yr=0 at (60,60), 0..1800, perimeter off");
	k15_solid(0, 0); gdp15(7, 2, 2, 60, 60, 1, 0, 0, 0, 1800); grab("K15P", 30, 30, 80, 80);
	/* control: normal circle r=12 */
	begin("K15Q", "v_circle r=12 at (60,60), control");
	k15_solid(1, 0); gdp15(4, 3, 0, 60, 60, 12, 0, 0, 0, 0); grab("K15Q", 30, 30, 80, 80);
}

/* K16 (TASK/V24 acceptance): v_get_pixel at the wrap points of the frame buffer, more than the three K10 reads. */
static void k16(void)
{
	begin("K16A", "v_get_pixel x outside 0..639 but inside the frame buffer (wraps to the next/previous row)");
	i1(25, 2); recfl(0, 1, 0, 1);
	i1(25, 3); recfl(639, 0, 639, 0);
	i1(25, 4); recfl(639, 1, 639, 1);
	i1(25, 5); recfl(0, 2, 0, 2);
	i1(25, 6); recfl(0, 4, 0, 4);
	i1(25, 7); recfl(1, 101, 1, 101);
	i1(25, 8); recfl(0, 479, 0, 479);
	emit("   K16A (640,0)=%04x (-1,1)=%04x (1279,0)=%04x (1280,0)=%04x\n",
	     (uint16_t)getpix(640, 0), (uint16_t)getpix(-1, 1), (uint16_t)getpix(1279, 0), (uint16_t)getpix(1280, 0));
	emit("   K16A (-640,5)=%04x (641,100)=%04x (640,478)=%04x (0,1)=%04x\n",
	     (uint16_t)getpix(-640, 5), (uint16_t)getpix(641, 100), (uint16_t)getpix(640, 478), (uint16_t)getpix(0, 1));
	grab("K16A", 0, 0, 16, 8);
	begin("K16B", "v_get_pixel beyond the frame buffer (decision B: 0 in NewTOS; EmuTOS reads whatever lies there)");
	emit("   K16B (640,479)=%04x (0,-1)=%04x (-1,0)=%04x (0,480)=%04x\n",
	     (uint16_t)getpix(640, 479), (uint16_t)getpix(0, -1), (uint16_t)getpix(-1, 0), (uint16_t)getpix(0, 480));
	grab("K16B", 0, 0, 16, 8);
}

int main(void)
{
	int16_t ap_id;
	int i;
	uint8_t hd[6];

	ap_id = aes(10, 0, 1, 0);		/* appl_init */
	if (ap_id < 0)
		return 1;
	aes(77, 0, 5, 0);			/* graf_handle */
	handle = aes_int_out[0];
	for (i = 0; i < 10; i++)
		intin[i] = 1;
	intin[10] = 2;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 100; contrl[3] = 11; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);	/* v_opnvwk */
	handle = contrl[6];
	for (i = 0; i < 12; i++)
		work_pts[i] = ptsout[i];
	emit("H00 handle %d xres %d yres %d planes? colours %d\n", handle, intout[0], intout[1], intout[13]);

	aes_int_in[0] = 256;			/* graf_mouse M_OFF */
	aes_addr_in[0] = 0;
	aes(78, 1, 1, 1);

	binfh = Fcreate("VDPROBE.BIN", 0);
	memcpy(hd, "VDPR", 4);
	put16(hd + 4, VERSION);
	if (binfh >= 0)
		Fwrite((short)binfh, 6L, hd);

	k_calibration();
	k1();
	k2();
	k3();
	k4();
	k5();
	k6();
	k8();
	k10();
	k11();
	k12();
	k13();
	k14();
	k15();
	k16();

	if (binfh >= 0)
		Fclose((short)binfh);
	reset();
	aes_int_in[0] = 257;			/* graf_mouse M_ON */
	aes(78, 1, 1, 1);
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 101; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);	/* v_clsvwk */
	aes(19, 0, 1, 0);			/* appl_exit */
	emit("END\n");
	{
		long fh = Fcreate("VDPROBE.LOG", 0);
		if (fh >= 0) {
			Fwrite((short)fh, (long)loglen, logbuf);
			Fclose((short)fh);
		}
	}
	return 0;
}
