/*
 * runner_rast.c -- the VDI raster runner (spec sections 4, 5 and 7).
 */
#include <string.h>

#include "abi.h"
#include "runner_rast.h"

/*
 * The MFDB, in the layout the target's own compiler produces -- which IS the
 * ABI both VDIs expect, because both of them read the caller's struct.  On
 * the m68k that is a 4-byte fd_addr followed by eight WORDs at offsets
 * 4..18 (EmuTOS include/gsxdefs.h:65-76); on AArch64 an 8-byte fd_addr
 * followed by the same eight WORDs at 8..22, which is what NewTOS' LP64
 * branch reads (tos-api/src/vdi/raster.rs, Mfdb::from_ptr).  Writing the
 * struct out by hand at explicit offsets -- the rule the RECORD follows --
 * would be wrong here: this is not a file, it is a parameter, and the
 * parameter's layout is the target's.
 */
struct td_mfdb {
	void   *fd_addr;
	int16_t fd_w;
	int16_t fd_h;
	int16_t fd_wdwidth;
	int16_t fd_stand;
	int16_t fd_nplanes;
	int16_t fd_r1;
	int16_t fd_r2;
	int16_t fd_r3;
};

static struct td_raster_arrays rast_arrays;
/* The wide frame, used only by a wide table (cases.h, THE 32-PLANE
 * SETTING): a separate object, so a narrow run lays out nothing new. */
static struct td_raster32_arrays rast32_arrays;
static int16_t                 rast_before[TD_RAST_CONTRL_N];
static struct td_mfdb          rast_src;
static struct td_mfdb          rast_dst;

/*
 * Pack a pointer into the control array the way every GEM binding does:
 * pointer n at contrl[7 + n * N_PTRINTS], N_PTRINTS = sizeof(void *) /
 * sizeof(WORD) (gemlib gem_vdiP.h:177,204).  One expression, correct on
 * both targets because sizeof(void *) is what differs -- so this is core
 * code and not a third ABI backend.  memcpy rather than a cast, because
 * contrl[7] is only WORD-aligned and an 8-byte store through a void ** at
 * that address is undefined C even where the hardware allows it.
 */
static void set_mfdb_ptr(int16_t *contrl, int n, void *p)
{
	const int nptrints = (int)(sizeof(void *) / sizeof(int16_t));

	memcpy(&contrl[7 + n * nptrints], &p, sizeof p);
}

/* The two pixel buffers of the frame a case runs in. */
struct rast_bufs {
	int16_t *a;
	int16_t *b;
};

static void fill_mfdb(struct td_mfdb *m, const struct td_rast_mfdb *spec,
                      const struct rast_bufs *a)
{
	memset(m, 0, sizeof *m);
	/* The screen: fd_addr NULL and every other field 0, the classic
	 * screen MFDB.  EmuTOS' setup_info reads none of the fields then
	 * (vdi_raster.c:915-950); the declared extent the table carries is
	 * the loader's guard, not the VDI's parameter. */
	if (spec->buffer == TD_RAST_BUF_SCREEN)
		return;
	m->fd_addr    = (spec->buffer == TD_RAST_BUF_B) ? (void *)a->b
	                                                : (void *)a->a;
	m->fd_w       = spec->w;
	m->fd_h       = spec->h;
	m->fd_wdwidth = spec->wdwidth;
	m->fd_stand   = spec->stand;
	m->fd_nplanes = spec->nplanes;
	/* fd_r1..fd_r3 stay 0: they are reserved, and a case that found them
	 * changed afterwards would have found a real defect. */
}

/*
 * The preludes' and the readback's own arrays.  Neither may touch the
 * recorded arrays: a prelude is not recorded, and the readback runs AFTER
 * the measured call, whose arrays the record captures as they stand.
 * Fifteen CONTRL words here too, for the raster preludes' MFDB pointers.
 */
static int16_t       pre_contrl[TD_RAST_CONTRL_N];
static int16_t       pre_intin[TD_RAST_INTIN_N];
static int16_t       pre_ptsin[TD_RAST_PTSIN_N];
static int16_t       pre_intout[TD_RAST_INTOUT_N];
static int16_t       pre_ptsout[TD_RAST_PTSOUT_N];
static struct td_mfdb pre_src;
static struct td_mfdb pre_dst;

static void run_prelude(const struct td_rast_prelude *p, const struct td_wk *wk,
                        const struct rast_bufs *a)
{
	int j;

	memset(pre_contrl, 0, sizeof pre_contrl);
	for (j = 0; j < (int)p->n_intin; j++)
		pre_intin[j] = p->intin[j];
	for (j = 0; j < (int)p->n_ptsin; j++)
		pre_ptsin[j] = p->ptsin[j];
	pre_contrl[0] = p->opcode;
	pre_contrl[1] = (int16_t)(p->n_ptsin / 2);
	pre_contrl[3] = (int16_t)p->n_intin;
	pre_contrl[5] = 0;
	pre_contrl[6] = wk->handle;
	if (p->opcode == 109 || p->opcode == 121) {
		fill_mfdb(&pre_src, &p->src, a);
		fill_mfdb(&pre_dst, &p->dst, a);
		set_mfdb_ptr(pre_contrl, 0, &pre_src);
		set_mfdb_ptr(pre_contrl, 1, &pre_dst);
	}
	td_abi_vdi(pre_contrl, pre_intin, pre_ptsin, pre_intout, pre_ptsout);
}

/*
 * The readback (cases.h, THE SCREEN): v_get_pixel over the rectangle, one
 * call per pixel, INTOUT[1] into buffer B at row * 32 + col.  On a
 * truecolor screen INTOUT[1] is the 16-bit screen word itself and
 * INTOUT[0] is 0 (vdi_fill.c:1095-1110); the table's plane guard is what
 * makes that the reading on both machines.
 *
 * In a wide table (cases.h, THE 32-PLANE SETTING) the pixel is two words,
 * INTOUT[0] = 0x00RR and INTOUT[1] = 0xGGBB (ADR-038 point 7), and both
 * are stored, high word first: B[2 * (row * 32 + col)] and [+1].
 */
static void run_readback(const struct td_rast_case *c, const struct td_wk *wk,
                         const struct rast_bufs *a, int wide)
{
	int16_t r, col;

	for (r = 0; r < c->rh; r++) {
		for (col = 0; col < c->rw; col++) {
			memset(pre_contrl, 0, sizeof pre_contrl);
			pre_contrl[0] = 105;                      /* v_get_pixel */
			pre_contrl[1] = 1;
			pre_contrl[3] = 0;
			pre_contrl[6] = wk->handle;
			pre_ptsin[0] = (int16_t)(c->rx + col);
			pre_ptsin[1] = (int16_t)(c->ry + r);
			pre_intout[0] = TD_POISON16;
			pre_intout[1] = TD_POISON16;
			td_abi_vdi(pre_contrl, pre_intin, pre_ptsin, pre_intout,
			           pre_ptsout);
			if (wide) {
				a->b[2 * (r * TD_RAST_READ_W + col)]     = pre_intout[0];
				a->b[2 * (r * TD_RAST_READ_W + col) + 1] = pre_intout[1];
			} else {
				a->b[r * TD_RAST_READ_W + col] = pre_intout[1];
			}
		}
	}
}

int16_t td_rast_planes(const struct td_wk *wk)
{
	memset(pre_contrl, 0, sizeof pre_contrl);
	memset(pre_intout, 0, sizeof pre_intout);
	pre_contrl[0] = 102;                              /* vq_extnd */
	pre_contrl[1] = 0;
	pre_contrl[3] = 1;
	pre_contrl[6] = wk->handle;
	pre_intin[0] = 1;                                 /* INQ_TAB */
	td_abi_vdi(pre_contrl, pre_intin, pre_ptsin, pre_intout, pre_ptsout);
	return pre_intout[4];                             /* planes */
}

/* The eight WORDs after fd_addr, read back out of an MFDB after the call --
 * vr_trnfm overwrites fd_stand (vdi_raster.c:303-314), so the MFDB is an
 * output as well as an input. */
static void mfdb_words(const struct td_mfdb *m, int16_t *out)
{
	out[0] = m->fd_w;
	out[1] = m->fd_h;
	out[2] = m->fd_wdwidth;
	out[3] = m->fd_stand;
	out[4] = m->fd_nplanes;
	out[5] = m->fd_r1;
	out[6] = m->fd_r2;
	out[7] = m->fd_r3;
}

size_t td_rast_record_size(const struct td_rast_case_table *ct)
{
	return ct->screen.wide ? (size_t)TD_RAST32_RECORD_SIZE
	                       : (size_t)TD_RAST_RECORD_SIZE;
}

uint16_t td_rast_area(const struct td_rast_case_table *ct)
{
	return ct->screen.wide ? (uint16_t)TD_AREA_RASTER32
	                       : (uint16_t)TD_AREA_RASTER;
}

int td_run_raster(const struct td_rast_case_table *ct, const struct td_wk *wk,
                  uint8_t *out, size_t out_bytes)
{
	int16_t src_words[TD_RAST_MFDB_N];
	int16_t dst_words[TD_RAST_MFDB_N];
	const int wide = ct->screen.wide != 0;
	const size_t rec_size = td_rast_record_size(ct);
	/* The frame this table runs in: the narrow one, exactly as before,
	 * or the wide one.  Only the parameter arrays and the two buffers
	 * are taken from it; the order of the steps is the same for both. */
	int16_t *contrl = wide ? rast32_arrays.contrl : rast_arrays.contrl;
	int16_t *intin  = wide ? rast32_arrays.intin  : rast_arrays.intin;
	int16_t *ptsin  = wide ? rast32_arrays.ptsin  : rast_arrays.ptsin;
	int16_t *intout = wide ? rast32_arrays.intout : rast_arrays.intout;
	int16_t *ptsout = wide ? rast32_arrays.ptsout : rast_arrays.ptsout;
	const uint16_t buf_words = wide ? (uint16_t)TD_RAST32_BUF_WORDS
	                                : (uint16_t)TD_RAST_BUF_WORDS;
	struct rast_bufs bufs;
	uint16_t i;
	int32_t ret;
	int j;

	bufs.a = wide ? rast32_arrays.buf_a : rast_arrays.buf_a;
	bufs.b = wide ? rast32_arrays.buf_b : rast_arrays.buf_b;

	if (out_bytes < (size_t)ct->count * rec_size)
		return TD_RUN_ERR_BUFFER;

	for (i = 0; i < ct->count; i++) {
		const struct td_rast_case *c = &ct->cases[i];
		uint8_t *rec = out + (size_t)i * rec_size;

		/* 1. reset the device state this case could otherwise inherit */
		td_reset_vdi(wk);

		/* 2. lay the two pixel buffers.  Not poison: see runner_rast.h */
		td_rast_fill(bufs.a, buf_words, c->seed_a);
		td_rast_fill(bufs.b, buf_words, c->seed_b);

		/* 3. the two MFDBs, over the buffers the case named */
		fill_mfdb(&rast_src, &c->src, &bufs);
		fill_mfdb(&rast_dst, &c->dst, &bufs);

		/* 3b. the case's preludes: after the reset and the buffer fill --
		 *     a raster prelude may copy a laid buffer to the screen --
		 *     through their own arrays, and never recorded. */
		for (j = 0; j < (int)c->prelude_count; j++)
			run_prelude(&ct->preludes[c->prelude_first + (uint16_t)j], wk,
			            &bufs);

		/* 4. poison every parameter array (spec section 4) */
		if (wide)
			td_poison_raster32(&rast32_arrays);
		else
			td_poison_raster(&rast_arrays);

		/* 5. lay the case values over the poison */
		for (j = 0; j < (int)c->n_intin; j++)
			intin[j] = c->intin[j];
		for (j = 0; j < (int)c->n_ptsin; j++)
			ptsin[j] = c->ptsin[j];

		/* 6. contrl: only the input slots.  contrl[2] and contrl[4] are
		 *    outputs and stay poisoned, so a VDI that leaves them alone
		 *    stays distinguishable from one that writes 0. */
		contrl[0] = c->opcode;
		contrl[1] = (int16_t)(c->n_ptsin / 2);   /* points */
		contrl[3] = (int16_t)c->n_intin;
		contrl[5] = 0;
		contrl[6] = wk->handle;
		set_mfdb_ptr(contrl, 0, &rast_src);
		set_mfdb_ptr(contrl, 1, &rast_dst);

		/* 7. snapshot contrl as it stands going in */
		for (j = 0; j < TD_RAST_CONTRL_N; j++)
			rast_before[j] = contrl[j];

		/* 8. the call under measurement */
		ret = td_abi_vdi(contrl, intin, ptsin, intout, ptsout);

		/* 8b. the readback, if the case asked for one: the screen
		 *     rectangle into buffer B, through the preludes' arrays. */
		if (c->has_read)
			run_readback(c, wk, &bufs, wide);

		/* 9. the record.  Input arrays and both MFDBs go in as they stand
		 *    AFTER the call: a raster op may modify neither, and dumping
		 *    them afterwards is what makes a violation visible -- except
		 *    for the destination MFDB's fd_stand, which vr_trnfm is
		 *    required to overwrite. */
		mfdb_words(&rast_src, src_words);
		mfdb_words(&rast_dst, dst_words);
		if (wide)
			td_raster32_record_write(rec, c->line, (uint16_t)c->opcode,
			                         ret, &rast32_arrays, rast_before,
			                         src_words, dst_words);
		else
			td_raster_record_write(rec, c->line, (uint16_t)c->opcode,
			                       ret, &rast_arrays, rast_before,
			                       src_words, dst_words);
	}

	return 0;
}
