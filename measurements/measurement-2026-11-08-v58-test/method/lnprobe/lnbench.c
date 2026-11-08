/*
 * lnbench.c -- V58 tester: instruction-count timing of v_pline through the real kernel, read from the
 * _hz_200 under QEMU -icount shift=0 (virtual time = executed instructions: 1 ns each, 1 tick = 5e6 instr).
 * Scenarios: 1000 calls each of a 2-point line, modes Replace/Xor, solid/dashed, 400 px long:
 * horizontal, vertical, diagonal, shallow slope; the per-call counter delta is logged.
 * The VDI trap path per call is the same on both kernels, so the difference between the two kernels is
 * the line path (plus the cache clean).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

static int16_t contrl[16], intin[300], ptsin[32], intout[128], ptsout[256], handle;
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
static char logbuf[8192];
static int loglen;
static void emit(const char *fmt, ...)
{
	va_list ap;
	va_start(ap, fmt);
	if (loglen < (int)sizeof(logbuf) - 200)
		loglen += vsnprintf(logbuf + loglen, sizeof(logbuf) - loglen, fmt, ap);
	va_end(ap);
}
static inline uint64_t cnt(void) { return *(volatile uint32_t *)0x4BA; }	/* _hz_200, own page 0 readable in user state (hzprobe.c) */

#define N 20000
static void bench(const char *name, int mode, int style, int x0, int y0, int x1, int y1)
{
	int i;
	uint64_t t0, t1, e0, e1;
	i1(32, (int16_t)mode); i1(15, (int16_t)style); i1(17, 1);
	/* warm the path once */
	ptsin[0] = (int16_t)x0; ptsin[1] = (int16_t)y0; ptsin[2] = (int16_t)x1; ptsin[3] = (int16_t)y1;
	vdi(6, 0, 2, 0);
	t0 = cnt();
	for (i = 0; i < N; i++) {
		ptsin[0] = (int16_t)x0; ptsin[1] = (int16_t)y0; ptsin[2] = (int16_t)x1; ptsin[3] = (int16_t)y1;
		vdi(6, 0, 2, 0);
	}
	t1 = cnt();
	/* the same calls with a zero-length line (trap + setup, no pixels) */
	e0 = cnt();
	for (i = 0; i < N; i++) {
		ptsin[0] = (int16_t)x0; ptsin[1] = (int16_t)y0; ptsin[2] = (int16_t)x0; ptsin[3] = (int16_t)y0;
		vdi(6, 0, 2, 0);
	}
	e1 = cnt();
	emit("%-22s mode %d style %d  %5lu ticks of 5 ms for %d calls; zero-length calls %5lu ticks\n", name, mode, style,
	     (unsigned long)(t1 - t0), N, (unsigned long)(e1 - e0));
}

int main(void)
{
	int16_t ap_id;
	int i, m, st;
	long fh;
	static const int styles[2] = { 1, 3 };

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
	aes_int_in[0] = 256; aes_addr_in[0] = 0; aes(78, 1, 1, 1);
	for (m = 1; m <= 3; m += 2)		/* 1 Replace, 3 Xor */
		for (st = 0; st < 2; st++) {
			bench("horizontal 400", m, styles[st], 100, 100, 499, 100);
			bench("vertical 400", m, styles[st], 100, 20, 100, 419);
			bench("vertical 400 up", m, styles[st], 100, 419, 100, 20);
			bench("diagonal 400", m, styles[st], 100, 20, 499, 419);
			bench("shallow 400x100", m, styles[st], 100, 20, 499, 119);
		}
	emit("END\n");
	fh = Fcreate("LNPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
	aes(19, 0, 1, 0);
	return 0;
}
