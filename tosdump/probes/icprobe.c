/*
 * icprobe.c -- G_CICON from rsrc_load to the screen (TASK/A141): what a colour
 * icon of a real resource file becomes.  A permanent probe of the tosdump tree
 * like vdprobe.c: no case table can give a loaded resource, an OBJECT tree and
 * the device form behind it.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari profile falcontc = 640x480
 * 16-bit truecolor, AArch64 NewTOS = the Falcon truecolor workstation of
 * ADR-024); all VDI and AES calls go through the tosdump ABI backends
 * (abi/m68k_trap.c, abi/newtos_svc.c), the structs below are the native ones of
 * each side (`void *` makes OBJECT 24/32, ICONBLK 34/48 bytes and so on).
 * Built by `make -C tosdump icprobe-m68k` (ICPROBE.PRG) and `make -C tosdump
 * icprobe-newtos` (ICPROBE.NAP, build-icprobe-newtos.sh); run by
 * tosdump/probes/run-icprobe.sh <rsc-classic> <rsc-native>; compared by
 * tosdump/probes/icprobe-compare.py.
 *
 * Input, in the current directory:
 *   ICPROBE.RSC  the resource: the classic file on EmuTOS, its tools/mkrsc.py
 *                conversion on NewTOS
 *   ICPROBE.TXT  the number of trees, in decimal
 *
 * Output, in the current directory:
 *   ICPROBE.BIN  "ICPR" + version WORD, then per case: 'C', case number (WORD),
 *                8-byte name, x, y, w, h (WORDs), then w*h words, row by row.
 *                Every WORD big-endian on both sides (vdprobe.c's records).
 *                PENS       vq_color(pen, 0) and (pen, 1): 256 rows of six
 *                           WORDs, requested then realized R, G, B
 *                I<t>.<o>   device form of tree t's object o, as rsrc_load left
 *                           col_data: x = t, y = o, w = ib_wicon, h = ib_hicon
 *                S<t>.<o>   the same of sel_data (only when there is one)
 *                M<t>.<o>   the file's col_mask, one row of (w/16)*h words
 *                T<t>.<p>   tree t drawn by objc_draw at (16,16) on a cleared
 *                           screen, read back with vro_cpyfm; pass p: 0 the
 *                           tree as loaded, 1 every G_CICON SELECTED, 2 every
 *                           G_CICON DISABLED, 3 the same with the text box of
 *                           every G_CICON zeroed (HypView's Init, sdk/apps/
 *                           hypview/init.c, "Entferne jeglichen Icon-Text")
 *   ICPROBE.LOG  one line per case; last line "END".
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

#define VERSION 1

typedef struct cicon {
	int16_t num_planes;
	int16_t *col_data, *col_mask, *sel_data, *sel_mask;
	struct cicon *next_res;
} CICON;

typedef struct {
	int16_t *ib_pmask, *ib_pdata;
	char *ib_ptext;
	int16_t ib_char, ib_xchar, ib_ychar, ib_xicon, ib_yicon, ib_wicon, ib_hicon;
	int16_t ib_xtext, ib_ytext, ib_wtext, ib_htext;
} ICONBLK;

typedef struct {
	ICONBLK monoblk;
	CICON *mainlist;
} CICONBLK;

typedef struct {
	int16_t ob_next, ob_head, ob_tail;
	uint16_t ob_type, ob_flags, ob_state;
	void *ob_spec;
	int16_t ob_x, ob_y, ob_width, ob_height;
} OBJ;

#define G_CICON   33
#define LASTOB    0x0020
#define SELECTED  0x0001
#define DISABLED  0x0008

static int16_t contrl[16];
static int16_t intin[32];
static int16_t ptsin[16];
static int16_t intout[64];
static int16_t ptsout[32];
static int16_t handle;

static int16_t aes_control[5];
static int16_t aes_global[15];
static int16_t aes_int_in[16];
static int16_t aes_int_out[8];
static void *aes_addr_in[2];
static void *aes_addr_out[2];

static int16_t aes(int16_t op, int16_t nin, int16_t nout, int16_t nain, int16_t naout)
{
	aes_control[0] = op;
	aes_control[1] = nin;
	aes_control[2] = nout;
	aes_control[3] = nain;
	aes_control[4] = naout;
	td_abi_aes(aes_control, aes_global, aes_int_in, aes_int_out,
	           aes_addr_in, aes_addr_out);
	return aes_int_out[0];
}

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

static int caseno;
static uint8_t wbuf[2048];

static void rec_head(const char *name, int x, int y, int w, int h)
{
	uint8_t hd[19];

	hd[0] = 'C';
	put16(hd + 1, caseno++);
	memset(hd + 3, 0, 8);
	memcpy(hd + 3, name, strlen(name) < 8 ? strlen(name) : 8);
	put16(hd + 11, x); put16(hd + 13, y); put16(hd + 15, w); put16(hd + 17, h);
	if (binfh >= 0)
		Fwrite((short)binfh, 19L, hd);
}

/* w*h native words from an arbitrary (word-aligned on m68k) address */
static void rec_words(const int16_t *src, int w, int h)
{
	int r, c;

	for (r = 0; r < h; r++) {
		for (c = 0; c < w; c++) {
			int16_t v;

			memcpy(&v, src + (long)r * w + c, 2);
			put16(wbuf + 2 * c, v);
		}
		if (binfh >= 0)
			Fwrite((short)binfh, (long)(2 * w), wbuf);
	}
}

typedef struct {
	void *fd_addr;
	int16_t fd_w, fd_h, fd_wdwidth, fd_stand, fd_nplanes, fd_r1, fd_r2, fd_r3;
} MFDB;

#define RB_MAXW 640
#define RB_MAXH 480
static uint16_t rbuf[RB_MAXW * RB_MAXH];
static MFDB scr_mfdb, mem_mfdb;

static void set_mfdb_ptr(int n, void *p)
{
	const int nptrints = (int)(sizeof(void *) / sizeof(int16_t));

	memcpy(&contrl[7 + n * nptrints], &p, sizeof p);
}

static void grab(const char *name, int x, int y, int w, int h)
{
	memset(&scr_mfdb, 0, sizeof scr_mfdb);
	memset(&mem_mfdb, 0, sizeof mem_mfdb);
	mem_mfdb.fd_addr = rbuf;
	mem_mfdb.fd_w = (int16_t)w;
	mem_mfdb.fd_h = (int16_t)h;
	mem_mfdb.fd_wdwidth = (int16_t)((w + 15) / 16);
	mem_mfdb.fd_stand = 0;
	mem_mfdb.fd_nplanes = 16;
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

	rec_head(name, x, y, w, h);
	{
		int r;
		for (r = 0; r < h; r++)
			rec_words((const int16_t *)(rbuf + (long)r * mem_mfdb.fd_wdwidth * 16), w, 1);
	}
	emit("%s %d,%d %dx%d\n", name, x, y, w, h);
}

static void pens(void)
{
	static int16_t row[6];
	int pen, f;

	rec_head("PENS", 0, 0, 6, 256);
	for (pen = 0; pen < 256; pen++) {
		for (f = 0; f < 2; f++) {
			memset(contrl, 0, sizeof contrl);
			contrl[0] = 26;
			contrl[3] = 2;
			contrl[6] = handle;
			intin[0] = (int16_t)pen;
			intin[1] = (int16_t)f;
			td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
			row[f * 3 + 0] = intout[1];
			row[f * 3 + 1] = intout[2];
			row[f * 3 + 2] = intout[3];
		}
		rec_words(row, 6, 1);
	}
	emit("PENS\n");
}

/* rsrc_gaddr(R_TREE, t); on both sides the address arrives in addr_out[0] */
static OBJ *tree_addr(int t)
{
	aes_int_in[0] = 0;
	aes_int_in[1] = (int16_t)t;
	aes_addr_out[0] = 0;
	aes(112, 2, 1, 0, 1);
	return (OBJ *)aes_addr_out[0];
}

static void icons(int t, OBJ *tr)
{
	int o;

	for (o = 0;; o++) {
		if ((tr[o].ob_type & 0xFF) == G_CICON) {
			CICONBLK *cb = (CICONBLK *)tr[o].ob_spec;
			CICON *c = cb->mainlist;
			int w = cb->monoblk.ib_wicon, h = cb->monoblk.ib_hicon;
			char n[32];

			emit("O %d.%d char %04x icon %d,%d %dx%d text %d,%d %dx%d planes %d sel %d state %04x flags %04x\n",
			     t, o, (unsigned)cb->monoblk.ib_char & 0xFFFF,
			     cb->monoblk.ib_xicon, cb->monoblk.ib_yicon, w, h,
			     cb->monoblk.ib_xtext, cb->monoblk.ib_ytext,
			     cb->monoblk.ib_wtext, cb->monoblk.ib_htext,
			     c ? c->num_planes : -1, c && c->sel_data != 0,
			     tr[o].ob_state, tr[o].ob_flags);
			if (c) {
				snprintf(n, sizeof n, "I%d.%d", t, o);
				rec_head(n, t, o, w, h);
				rec_words(c->col_data, w, h);
				if (c->sel_data) {
					snprintf(n, sizeof n, "S%d.%d", t, o);
					rec_head(n, t, o, w, h);
					rec_words(c->sel_data, w, h);
				}
				snprintf(n, sizeof n, "M%d.%d", t, o);
				rec_head(n, t, o, (w / 16) * h, 1);
				rec_words(c->col_mask, (w / 16) * h, 1);
			}
		}
		if (tr[o].ob_flags & LASTOB)
			break;
	}
}

static void draw(int t, OBJ *tr, int pass)
{
	char n[32];
	int o, w = tr[0].ob_width + 32, h = tr[0].ob_height + 32;

	if (w > RB_MAXW)
		w = RB_MAXW;
	if (h > RB_MAXH)
		h = RB_MAXH;
	for (o = 0;; o++) {
		if ((tr[o].ob_type & 0xFF) == G_CICON) {
			if (pass == 1)
				tr[o].ob_state |= SELECTED;
			else if (pass == 2) {
				tr[o].ob_state &= (uint16_t)~SELECTED;
				tr[o].ob_state |= DISABLED;
			} else if (pass == 3) {
				((CICONBLK *)tr[o].ob_spec)->monoblk.ib_wtext = 0;
				((CICONBLK *)tr[o].ob_spec)->monoblk.ib_htext = 0;
			}
		}
		if (tr[o].ob_flags & LASTOB)
			break;
	}
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 3;
	contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);	/* v_clrwk */
	tr[0].ob_x = 16;
	tr[0].ob_y = 16;
	aes_addr_in[0] = tr;
	aes_int_in[0] = 0;
	aes_int_in[1] = 8;
	aes_int_in[2] = 0;
	aes_int_in[3] = 0;
	aes_int_in[4] = 640;
	aes_int_in[5] = 480;
	aes(42, 6, 1, 1, 0);			/* objc_draw */
	snprintf(n, sizeof n, "T%d.%d", t, pass);
	grab(n, 0, 0, w, h);
}

int main(void)
{
	int16_t ap_id;
	int i, t, ntree = 0;
	uint8_t hd[6];
	char txt[8];
	long fh;

	ap_id = aes(10, 0, 1, 0, 0);		/* appl_init */
	if (ap_id < 0)
		return 1;
	aes(77, 0, 5, 0, 0);			/* graf_handle */
	handle = aes_int_out[0];
	for (i = 0; i < 10; i++)
		intin[i] = 1;
	intin[10] = 2;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 100; contrl[3] = 11; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);	/* v_opnvwk */
	handle = contrl[6];
	emit("H handle %d xres %d yres %d colours %d\n", handle, intout[0], intout[1], intout[13]);

	aes_int_in[0] = 256;			/* graf_mouse M_OFF */
	aes_addr_in[0] = 0;
	aes(78, 1, 1, 1, 0);

	binfh = Fcreate("ICPROBE.BIN", 0);
	memcpy(hd, "ICPR", 4);
	put16(hd + 4, VERSION);
	if (binfh >= 0)
		Fwrite((short)binfh, 6L, hd);

	pens();

	memset(txt, 0, sizeof txt);
	fh = Fopen("ICPROBE.TXT", 0);
	if (fh >= 0) {
		Fread((short)fh, (long)sizeof txt - 1, txt);
		Fclose((short)fh);
	}
	for (i = 0; txt[i] >= '0' && txt[i] <= '9'; i++)
		ntree = ntree * 10 + (txt[i] - '0');

	aes_addr_in[0] = (void *)"ICPROBE.RSC";
	i = aes(110, 0, 1, 1, 0);		/* rsrc_load */
	emit("R rsrc_load %d ntree %d\n", i, ntree);
	if (i) {
		for (t = 0; t < ntree; t++) {
			OBJ *tr = tree_addr(t);

			if (!tr) {
				emit("t %d no tree\n", t);
				continue;
			}
			icons(t, tr);
			for (i = 0; i < 4; i++)
				draw(t, tr, i);
		}
		aes(111, 0, 1, 0, 0);		/* rsrc_free */
	}
	aes_int_in[0] = 257;			/* graf_mouse M_ON */
	aes_addr_in[0] = 0;
	aes(78, 1, 1, 1, 0);
	if (binfh >= 0)
		Fclose((short)binfh);
	emit("END\n");
	fh = Fcreate("ICPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
	aes(19, 0, 1, 0, 0);			/* appl_exit */
	return 0;
}
