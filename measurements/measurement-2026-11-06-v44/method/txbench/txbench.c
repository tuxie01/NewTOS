/* txbench.c -- scratch timing probe (V33 acceptance, item 8): v_gtext with the system font,
 * time base = _hz_200 (0x4BA) read via Supexec. Result -> C:\TXBENCH.LOG. Not product code. */
#include <gem.h>
#include <osbind.h>
#include <stdio.h>
#include <string.h>

static long hz200(void) { return (long)*(volatile unsigned int *)(unsigned long)0x4BA; }
static long ticks(void) { return Supexec(hz200); }

static char line[256];
static char out[2048];
static int olen;

static void emit(const char *name, long t0, long t1, long n, long chars)
{
	olen += snprintf(out + olen, sizeof(out) - olen, "%s calls=%ld chars_per_call=%ld ticks200=%ld\n",
	                 name, n, chars, t1 - t0);
}

int main(void)
{
	short work_in[11], work_out[57], h, i, ch, cw, bh, bw, ig;
	short pxy[4];
	long t0, t1, n, k;
	appl_init();
	for (i = 0; i < 10; i++) work_in[i] = 1;
	work_in[10] = 2;
	h = graf_handle(&cw, &ch, &bw, &bh);
	v_opnvwk(work_in, &h, work_out);
	pxy[0] = 0; pxy[1] = 0; pxy[2] = work_out[0]; pxy[3] = work_out[1];
	memset(line, 'M', 76); line[76] = 0;   /* 76 glyphs */
	for (i = 0; i < 76; i += 3) line[i] = "AbgjQ"[(i / 3) % 5];
	vst_color(h, 1);
	vst_font(h, 1);
	vst_point(h, 10, &ig, &ig, &ig, &ig);
	n = 2000;
	/* A: clip off, no effects */
	vs_clip(h, 0, pxy);
	vst_effects(h, 0);
	t0 = ticks();
	for (k = 0; k < n; k++) v_gtext(h, 8, 40 + (k % 20) * 16, line);
	t1 = ticks();
	emit("A_clipoff_plain", t0, t1, n, 76);
	/* B: clip on (full screen) */
	vs_clip(h, 1, pxy);
	t0 = ticks();
	for (k = 0; k < n; k++) v_gtext(h, 8, 40 + (k % 20) * 16, line);
	t1 = ticks();
	emit("B_clipon_plain", t0, t1, n, 76);
	/* C: bold */
	vst_effects(h, 1);
	t0 = ticks();
	for (k = 0; k < n; k++) v_gtext(h, 8, 40 + (k % 20) * 16, line);
	t1 = ticks();
	emit("C_clipon_bold", t0, t1, n, 76);
	/* D: skew + underline */
	vst_effects(h, 4 | 8);
	t0 = ticks();
	for (k = 0; k < n / 2; k++) v_gtext(h, 8, 40 + (k % 20) * 16, line);
	t1 = ticks();
	emit("D_clipon_light_skew", t0, t1, n / 2, 76);
	vst_effects(h, 0);
	/* E: short strings (menu-like, 8 glyphs) */
	t0 = ticks();
	for (k = 0; k < n * 5; k++) v_gtext(h, 8 + (k % 50) * 8, 40 + (k % 20) * 16, "File Edit");
	t1 = ticks();
	emit("E_clipon_9chars", t0, t1, n * 5, 9);
	v_clsvwk(h);
	{
		long fh = Fcreate("TXBENCH.LOG", 0);
		if (fh >= 0) { Fwrite((short)fh, olen, out); Fclose((short)fh); }
	}
	appl_exit();
	return 0;
}
