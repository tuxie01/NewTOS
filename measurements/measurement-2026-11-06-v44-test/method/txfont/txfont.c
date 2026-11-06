/* txfont.c -- scratch timing probe (V44 acceptance): v_gtext with a LOADED font (vst_load_fonts chain as
 * vdprobe T13), narrow cell (8x8, fits 64x64) and wide cell (72x16, does not fit). _hz_200 via Supexec.
 * Result -> C:\TXFONT.LOG. Not product code. */
#include <gem.h>
#include <osbind.h>
#include <stdio.h>
#include <stdint.h>
#include <string.h>

static long hz200(void) { return (long)*(volatile unsigned int *)(unsigned long)0x4BA; }
static long ticks(void) { return Supexec(hz200); }

struct fh {
	int16_t font_id, point;
	char name[32];
	uint16_t first_ade, last_ade, top, ascent, half, descent, bottom, max_char_width, max_cell_width;
	uint16_t left_offset, right_offset, thicken, ul_size, lighten, skew, flags;
	const uint8_t *hor_table;
	const uint16_t *off_table;
	const uint16_t *dat_table;
	uint16_t form_width, form_height;
	struct fh *next_font;
	uint16_t reserved;
};
typedef struct { int16_t *contrl, *intin, *ptsin, *intout, *ptsout; } PB;
static int16_t contrl[16], intin[300], ptsin[64], intout[128], ptsout[64];
static int16_t handle;
static struct fh nf, wf;
static uint16_t noff[70], woff[8];
static uint8_t ndat[8 * 64 + 8];
static uint8_t wdat[16 * 36 + 8];
static uint16_t scratch[2048];
static char out[2048];
static int olen;

static void vdicall(void)
{
	PB pb = { contrl, intin, ptsin, intout, ptsout };
	register long x0 __asm__("x0") = 115;
	register long x1 __asm__("x1") = (long)&pb;
	__asm__ __volatile__("svc #2" : "+r"(x0) : "r"(x1) : "memory", "cc");
}
static void emit(const char *name, long t0, long t1, long n, long chars)
{
	olen += snprintf(out + olen, sizeof(out) - olen, "%s calls=%ld chars_per_call=%ld ticks200=%ld\n", name, n, chars, t1 - t0);
}
static void head(struct fh *f, int id, const char *name, int top, int cell, int first, int last, uint16_t *off, void *dat, int fw, int fh_)
{
	memset(f, 0, sizeof *f);
	f->font_id = id; f->point = 12; memcpy(f->name, name, strlen(name));
	f->first_ade = first; f->last_ade = last;
	f->top = top; f->ascent = top; f->half = top * 2 / 3; f->descent = 1; f->bottom = 1;
	f->max_char_width = cell; f->max_cell_width = cell;
	f->left_offset = 1; f->right_offset = 1; f->thicken = 1; f->ul_size = 1; f->lighten = 0x5555; f->skew = 0x5555;
	f->flags = 4 | 8;
	f->off_table = off; f->dat_table = (const uint16_t *)dat; f->form_width = fw; f->form_height = fh_;
}
static void load(struct fh *chain)
{
	void *scr = scratch, *fp = chain;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 119; contrl[3] = 1; contrl[6] = handle;
	intin[0] = 0;
	memcpy(&contrl[7], &scr, sizeof scr); contrl[11] = 2048; memcpy(&contrl[12], &fp, sizeof fp);
	vdicall();
}
static void unload(void) { memset(contrl, 0, sizeof contrl); contrl[0] = 120; contrl[3] = 1; contrl[6] = handle; intin[0] = 0; vdicall(); }

static char line[256];
static long run(const char *name, int fid, int eff, long n, int chars)
{
	long t0, t1, k;
	vst_font(handle, fid);
	vst_effects(handle, eff);
	t0 = ticks();
	for (k = 0; k < n; k++) v_gtext(handle, 8, 40 + (k % 20) * 16, line);
	t1 = ticks();
	emit(name, t0, t1, n, chars);
	return t1 - t0;
}
int main(void)
{
	short work_in[11], work_out[57], i, ch, cw, bh, bw, ig, pxy[4];
	int r, c;
	appl_init();
	for (i = 0; i < 10; i++) work_in[i] = 1;
	work_in[10] = 2;
	handle = graf_handle(&cw, &ch, &bw, &bh);
	v_opnvwk(work_in, &handle, work_out);
	pxy[0] = 0; pxy[1] = 0; pxy[2] = work_out[0]; pxy[3] = work_out[1];
	vs_clip(handle, 1, pxy);
	vst_color(handle, 1);
	for (c = 0; c <= 64; c++) noff[c] = 8 * c;
	for (r = 0; r < 8; r++) for (c = 0; c < 64; c++) ndat[r * 64 + c] = (uint8_t)(((c * 29 + r * 53) ^ 0xA5) | 0x81);
	head(&nf, 7, "NARROW SEVEN", 6, 8, 32, 95, noff, ndat, 64, 8);
	for (c = 0; c <= 4; c++) woff[c] = 72 * c;
	for (r = 0; r < 16; r++) for (c = 0; c < 36; c++) wdat[r * 36 + c] = (uint8_t)(((c * 29 + r * 53) ^ 0xA5) | 0x81);
	head(&wf, 8, "WIDE EIGHT", 12, 72, 65, 68, woff, wdat, 36, 16);
	memset(line, 'M', 76); line[76] = 0;
	for (i = 0; i < 76; i += 3) line[i] = "AbgjQ"[(i / 3) % 5];
	/* system font reference, same shape as txbench A/C */
	run("S_sys_plain", 1, 0, 2000, 76);
	run("S_sys_bold", 1, 1, 2000, 76);
	load(&nf);
	run("N_narrow_plain", 7, 0, 2000, 76);
	run("N_narrow_bold", 7, 1, 2000, 76);
	run("N_narrow_skew", 7, 4 | 8, 1000, 76);
	unload();
	for (i = 0; i < 8; i++) line[i] = "ABCD"[i % 4];
	line[8] = 0;
	load(&wf);
	run("W_wide_plain", 8, 0, 1000, 8);
	run("W_wide_bold", 8, 1, 1000, 8);
	unload();
	v_clsvwk(handle);
	{
		long fh_ = Fcreate("TXFONT.LOG", 0);
		if (fh_ >= 0) { Fwrite((short)fh_, olen, out); Fclose((short)fh_); }
	}
	appl_exit();
	return 0;
}
