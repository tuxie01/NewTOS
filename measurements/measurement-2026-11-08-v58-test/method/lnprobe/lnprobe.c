/*
 * lnprobe.c -- V58 tester probe (new file, not a copy of a tosdump source except the helper idiom of
 * tosdump/probes/vdprobe.c).  Many random lines through v_pline: all octants and axis-parallel/diagonal
 * specials, lengths 0..300, line styles 1..7 (7 = user style, random udsty), all 4 writing modes,
 * clipping off (endpoints up to 150 px off every screen side) / on (inner rectangle) / on (whole screen),
 * 2-point lines and 3-point polylines (LSTLIN false for XOR).  After every case the whole 640x480 screen
 * is read back (vro_cpyfm, the vdprobe way) and hashed (FNV-1a 64 over the words); LNPROBE.LOG holds one
 * line per case.  Same seed -> same drawing on base and change; the logs are compared byte for byte.
 * Build: method/lnprobe/build-lnprobe.sh;  run: method/lnprobe/run-lnprobe.sh.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

static int16_t contrl[16];
static int16_t intin[300];
static int16_t ptsin[2 * 16];
static int16_t intout[128];
static int16_t ptsout[256];
static int16_t handle;
static int16_t aes_control[5], aes_global[15], aes_int_in[16], aes_int_out[8];
static void *aes_addr_in[2], *aes_addr_out[2];

static int16_t aes(int16_t op, int16_t nin, int16_t nout, int16_t nain)
{
	aes_control[0] = op; aes_control[1] = nin; aes_control[2] = nout; aes_control[3] = nain; aes_control[4] = 0;
	td_abi_aes(aes_control, aes_global, aes_int_in, aes_int_out, aes_addr_in, aes_addr_out);
	return aes_int_out[0];
}

static void vdi(int16_t op, int16_t sub, int16_t npts, int16_t nint)
{
	memset(contrl, 0, sizeof contrl);
	contrl[0] = op; contrl[1] = npts; contrl[3] = nint; contrl[5] = sub; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
}
static void i1(int16_t op, int16_t a) { intin[0] = a; vdi(op, 0, 0, 1); }

static char logbuf[16384];
static int loglen;
static void emit(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	if (loglen < (int)sizeof(logbuf) - 200)
		loglen += vsnprintf(logbuf + loglen, sizeof(logbuf) - loglen, fmt, ap);
	va_end(ap);
}

typedef struct { void *fd_addr; int16_t fd_w, fd_h, fd_wdwidth, fd_stand, fd_nplanes, fd_r1, fd_r2, fd_r3; } MFDB;
static uint16_t rbuf[640 * 480];
static MFDB scr_mfdb, mem_mfdb;

static uint64_t screen_hash(void)
{
	uint64_t h = 1469598103934665603ULL;
	long k;
	memset(&scr_mfdb, 0, sizeof scr_mfdb);
	memset(&mem_mfdb, 0, sizeof mem_mfdb);
	mem_mfdb.fd_addr = rbuf; mem_mfdb.fd_w = 640; mem_mfdb.fd_h = 480; mem_mfdb.fd_wdwidth = 40;
	mem_mfdb.fd_nplanes = 16;
	memset(rbuf, 0x5A, sizeof rbuf);
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 109; contrl[1] = 4; contrl[3] = 1; contrl[6] = handle;
	{ void *p = &scr_mfdb; memcpy(&contrl[7], &p, sizeof p); }
	{ void *p = &mem_mfdb; memcpy(&contrl[7 + sizeof(void *) / 2], &p, sizeof p); }
	intin[0] = 3;
	ptsin[0] = 0; ptsin[1] = 0; ptsin[2] = 639; ptsin[3] = 479; ptsin[4] = 0; ptsin[5] = 0; ptsin[6] = 639; ptsin[7] = 479;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	for (k = 0; k < 640L * 480; k++) {
		h ^= rbuf[k] & 0xFF; h *= 1099511628211ULL;
		h ^= rbuf[k] >> 8; h *= 1099511628211ULL;
	}
	return h;
}

static uint64_t rs;
static uint32_t rnd(void)
{
	rs ^= rs << 13; rs ^= rs >> 7; rs ^= rs << 17;
	return (uint32_t)(rs >> 16);
}
static int rr(int lo, int hi) { return lo + (int)(rnd() % (uint32_t)(hi - lo + 1)); }

static void clipset(int on, int x1, int y1, int x2, int y2)
{
	intin[0] = (int16_t)on;
	ptsin[0] = (int16_t)x1; ptsin[1] = (int16_t)y1; ptsin[2] = (int16_t)x2; ptsin[3] = (int16_t)y2;
	vdi(129, 0, 2, 1);
}

/* one random end point from a start: octant, axis/diagonal specials, length 0..300 */
static void endpoint(int sx, int sy, int lo_x, int hi_x, int lo_y, int hi_y, int *ex, int *ey)
{
	int len = rr(0, 300), kind = rr(0, 9), dx, dy;
	switch (kind) {
	case 0: dx = (rnd() & 1) ? len : -len; dy = 0; break;		/* horizontal */
	case 1: dx = 0; dy = (rnd() & 1) ? len : -len; break;		/* vertical */
	case 2: dx = (rnd() & 1) ? len : -len; dy = (rnd() & 1) ? dx : -dx; break;	/* diagonal */
	default: dx = rr(-len, len); dy = rr(-len, len); break;	/* any octant (and any length) */
	}
	*ex = sx + dx; *ey = sy + dy;
	if (*ex < lo_x) *ex = lo_x; if (*ex > hi_x) *ex = hi_x;
	if (*ey < lo_y) *ey = lo_y; if (*ey > hi_y) *ey = hi_y;
}

static int caseno;
/* mode 1..4; clip 0 off, 1 inner rectangle, 2 whole screen; poly 0 two-point lines, 1 three-point polylines */
static void run_case(int mode, int clip, int poly, int nlines)
{
	int i, k, lx, hx, ly, hy;
	uint64_t h;

	caseno++;
	rs = 0x9E3779B97F4A7C15ULL ^ ((uint64_t)caseno * 0xD1B54A32D192ED03ULL);
	vdi(3, 0, 0, 0);			/* v_clrwk */
	i1(32, 1); i1(15, 1); i1(17, 1);
	i1(23, 1); i1(24, 1); i1(104, 0);
	for (i = 0; i < 60; i++) {		/* a coloured background so Transparent/Xor/RevTransp show */
		i1(25, (int16_t)rr(0, 15));
		ptsin[0] = (int16_t)rr(0, 600); ptsin[1] = (int16_t)rr(0, 440);
		ptsin[2] = (int16_t)(ptsin[0] + rr(1, 200)); ptsin[3] = (int16_t)(ptsin[1] + rr(1, 150));
		vdi(114, 0, 2, 0);
	}
	if (clip == 0) { clipset(0, 0, 0, 0, 0); lx = -150; hx = 789; ly = -150; hy = 629; }
	else if (clip == 1) { clipset(1, 50, 40, 590, 440); lx = -150; hx = 789; ly = -150; hy = 629; }
	else { clipset(1, 0, 0, 639, 479); lx = -150; hx = 789; ly = -150; hy = 629; }
	i1(32, (int16_t)mode);
	for (i = 0; i < nlines; i++) {
		int sx = rr(lx, hx), sy = rr(ly, hy), ex, ey, ex2, ey2, st = rr(1, 7);
		i1(15, (int16_t)st);
		if (st == 7) i1(113, (int16_t)rr(0, 0xFFFF));
		i1(17, (int16_t)rr(1, 15));
		endpoint(sx, sy, lx, hx, ly, hy, &ex, &ey);
		ptsin[0] = (int16_t)sx; ptsin[1] = (int16_t)sy; ptsin[2] = (int16_t)ex; ptsin[3] = (int16_t)ey;
		k = 2;
		if (poly) {
			endpoint(ex, ey, lx, hx, ly, hy, &ex2, &ey2);
			ptsin[4] = (int16_t)ex2; ptsin[5] = (int16_t)ey2; k = 3;
		}
		vdi(6, 0, (int16_t)k, 0);
	}
	h = screen_hash();
	emit("%02d mode %d clip %d poly %d lines %d hash %08lx%08lx\n", caseno, mode, clip, poly, nlines,
	     (unsigned long)(h >> 32), (unsigned long)(h & 0xFFFFFFFFUL));
}

int main(void)
{
	int16_t ap_id;
	int i, m, c, p;
	long fh;

	ap_id = aes(10, 0, 1, 0);
	if (ap_id < 0)
		return 1;
	aes(77, 0, 5, 0);
	handle = aes_int_out[0];
	for (i = 0; i < 10; i++)
		intin[i] = 1;
	intin[10] = 2;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 100; contrl[3] = 11; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	handle = contrl[6];
	emit("H00 handle %d xres %d yres %d\n", handle, intout[0], intout[1]);
	aes_int_in[0] = 256; aes_addr_in[0] = 0; aes(78, 1, 1, 1);	/* mouse off */
	for (p = 0; p < 2; p++)
		for (c = 0; c < 3; c++)
			for (m = 1; m <= 4; m++)
				run_case(m, c, p, 300);
	emit("END\n");
	fh = Fcreate("LNPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
	aes(19, 0, 1, 0);
	return 0;
}
