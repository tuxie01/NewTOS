/* vbench.c -- V35 timing probe: the cost of GSX_ENTRY's local PTSIN copy, which tos-api/src/vdi/mod.rs
 * dispatch_in now makes on every VDI call (CONTRL[1] pairs). Time base _hz_200 (0x4BA) via Supexec, as
 * measurements/measurement-2026-11-06-v44/method/txbench/txbench.c. Result -> C:\VBENCH.LOG. Not product code.
 *   A vsf_color        0 pairs  (no copy: the control)
 *   B vsl_width        1 pair
 *   C v_pline          2 pairs  (a short line)
 *   D v_pline       1024 pairs  (the most one call copies)
 *   E v_gtext          1 pair, 9 chars (txbench case E) */
#include <gem.h>
#include <osbind.h>
#include <stdio.h>
#include <string.h>

static long hz200(void) { return (long)*(volatile unsigned int *)(unsigned long)0x4BA; }
static long ticks(void) { return Supexec(hz200); }

static char out[2048];
static int olen;
static short big[2 * 1024];

static void emit(const char *name, long t0, long t1, long n)
{
	olen += snprintf(out + olen, sizeof(out) - olen, "%s calls=%ld ticks200=%ld\n", name, n, t1 - t0);
}

int main(void)
{
	short work_in[11], work_out[57], h, i, ch, cw, bh, bw;
	short pxy[4];
	long t0, t1, k;
	appl_init();
	for (i = 0; i < 10; i++) work_in[i] = 1;
	work_in[10] = 2;
	h = graf_handle(&cw, &ch, &bw, &bh);
	v_opnvwk(work_in, &h, work_out);
	pxy[0] = 0; pxy[1] = 0; pxy[2] = work_out[0]; pxy[3] = work_out[1];
	vs_clip(h, 1, pxy);
	for (i = 0; i < 1024; i++) { big[2 * i] = (short)(20 + (i % 2) * 4); big[2 * i + 1] = (short)(40 + i / 8); }
	t0 = ticks();
	for (k = 0; k < 100000; k++) vsf_color(h, 1);
	t1 = ticks();
	emit("A_vsf_color_0pairs", t0, t1, 100000);
	t0 = ticks();
	for (k = 0; k < 100000; k++) vsl_width(h, 1);
	t1 = ticks();
	emit("B_vsl_width_1pair", t0, t1, 100000);
	t0 = ticks();
	for (k = 0; k < 20000; k++) {
		pxy[0] = 100; pxy[1] = (short)(40 + k % 200); pxy[2] = 300; pxy[3] = pxy[1];
		v_pline(h, 2, pxy);
	}
	t1 = ticks();
	emit("C_v_pline_2pairs", t0, t1, 20000);
	t0 = ticks();
	for (k = 0; k < 500; k++) v_pline(h, 1024, big);
	t1 = ticks();
	emit("D_v_pline_1024pairs", t0, t1, 500);
	t0 = ticks();
	for (k = 0; k < 10000; k++) v_gtext(h, 8 + (k % 50) * 8, 300 + (k % 10) * 16, "File Edit");
	t1 = ticks();
	emit("E_v_gtext_9chars", t0, t1, 10000);
	v_clsvwk(h);
	{
		long fh = Fcreate("VBENCH.LOG", 0);
		if (fh >= 0) { Fwrite((short)fh, olen, out); Fclose((short)fh); }
	}
	appl_exit();
	return 0;
}
