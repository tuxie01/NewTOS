/*
 * cfprobe.c -- V38 acceptance probe (Tester copy, lives only under this run's method/).
 * The smallest never-ending v_contourfill found by the V38 developer: a 5x3 clip, black dots at
 * (3,1) and (1,2), seed (0,2), fill red up to black (boundary colour 1).  Placed at (10,20) on the
 * screen: clip (10,20)-(14,22), dots (13,21) and (11,22), seed (10,22).
 *
 * Built as VDPROBE.PRG / VDPROBE.NAP so tools/hatari-oracle.sh and tosdump/probes/run-vdprobe.sh
 * style runs accept it (their AREA is VDPROBE: they wait for VDPROBE.LOG).  -DCASE=0: the fill,
 * 1: no fill (baseline for the setup time), 2: control, the same fill on the same clip and seed
 * WITHOUT the dots (the fill ends on its own: proves the probe and the oracle return from a fill).
 *
 * Output VDPROBE.LOG: first "running 1 cases" (written BEFORE the fill, so a hang leaves it as the
 * last line, the hatari-oracle.sh convention), after the fill the pixel words of the 5x3 region and
 * "END".
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

#ifndef CASE
#define CASE 0
#endif

static int16_t contrl[16];
static int16_t intin[300];
static int16_t ptsin[2 * 1100];
static int16_t intout[128];
static int16_t ptsout[256];
static int16_t handle;

static int16_t aes_control[5];
static int16_t aes_global[15];
static int16_t aes_int_in[16];
static int16_t aes_int_out[8];
static void *aes_addr_in[2];
static void *aes_addr_out[2];

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

static char logbuf[4096];
static int loglen;

static void emit(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	loglen += vsnprintf(logbuf + loglen, sizeof(logbuf) - loglen, fmt, ap);
	va_end(ap);
}

static void writelog(void)
{
	long fh = Fcreate("VDPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
}

static int16_t getpix(int x, int y)
{
	intout[0] = intout[1] = 0x7777;
	ptsin[0] = (int16_t)x; ptsin[1] = (int16_t)y;
	vdi(105, 0, 1, 0);
	return intout[1];
}

static void dump(const char *what)
{
	int x, y;
	emit("%s\n", what);
	for (y = 20; y <= 22; y++) {
		for (x = 10; x <= 14; x++)
			emit(" %04x", (unsigned)(uint16_t)getpix(x, y));
		emit("\n");
	}
}

int main(void)
{
	int16_t ap_id;
	int i;

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
	emit("cfprobe CASE=%d handle %d xres %d yres %d\n", CASE, handle, intout[0], intout[1]);

	vdi(3, 0, 0, 0);			/* v_clrwk */
	i1(32, 1);				/* replace */
	i1(15, 1);				/* vsl_type solid */
	i1(17, 1);				/* vsl_color 1 (black) */
	i1(23, 1);				/* vsf_interior solid */
	i1(24, 1);
	i1(25, 2);				/* vsf_color 2 (red) */
	i1(104, 1);
#if CASE != 2
	/* the two dots: one-point lines in colour 1 */
	ptsin[0] = 13; ptsin[1] = 21; ptsin[2] = 13; ptsin[3] = 21; vdi(6, 0, 2, 0);
	ptsin[0] = 11; ptsin[1] = 22; ptsin[2] = 11; ptsin[3] = 22; vdi(6, 0, 2, 0);
#endif
	/* clip (10,20)-(14,22) */
	intin[0] = 1; ptsin[0] = 10; ptsin[1] = 20; ptsin[2] = 14; ptsin[3] = 22; vdi(129, 0, 2, 1);
	dump("before");
	emit("running 1 cases\n");
	writelog();				/* a hang leaves this as the last line */
#if CASE == 0 || CASE == 2
	intin[0] = 1; ptsin[0] = 10; ptsin[1] = 22; vdi(103, 0, 1, 1);	/* v_contourfill up to colour 1 */
#endif
	loglen = 0;
	emit("cfprobe CASE=%d: contourfill returned\n", CASE);
	dump("after");
	i1(129, 0);
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 101; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	aes(19, 0, 1, 0);
	emit("END\n");
	writelog();
	return 0;
}
