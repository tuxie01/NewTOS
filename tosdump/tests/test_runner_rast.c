/*
 * test_runner_rast.c -- drives the raster runner against the stub backend.
 *
 * What it has to prove, and none of it can be read out of the runner:
 *
 *   - the two MFDB pointers land at contrl[7 + n * N_PTRINTS], which is
 *     where every GEM binding puts them (gemlib gem_vdiP.h:177,204) and
 *     where both VDIs look.  The stub reads them back out of exactly those
 *     slots and stamps what it finds; if the runner packed them anywhere
 *     else the stamp never appears.
 *   - the MFDB fields the case named reach the struct, and the MFDB is
 *     recorded AFTER the call, because vr_trnfm overwrites fd_stand.
 *   - the pixel buffers are laid with the case's seeds, not poisoned, and
 *     are recorded after the call.
 *   - the parameter arrays ARE poisoned, and the input arrays are recorded
 *     after the call (the stub mutates intin[0]/ptsin[0], which no real VDI
 *     may do -- that is the point).
 *   - contrl[2] and contrl[4] are outputs and go in poisoned.
 */
#include "tdtest.h"
#include "cases.h"
#include "gem.h"
#include "dump.h"
#include "runner_rast.h"
#include "stub_abi.h"

/* 32 x 16, fd_wdwidth 2: 16 * 2 * 16 = 512 words = TD_RAST_BUF_WORDS. */
static const char *table_text =
	"# a comment consumes a line number\n"
	"110\t-\t-\t0,32,16,2,1,4\t1,32,16,2,0,4\t11,22\tvr_trnfm out of place\n"
	"109\t3\t0,0,15,7,0,0,15,7\t0,32,16,2,0,4\t1,32,16,2,0,4\t33,44\t"
		"vro_cpyfm replace\n";

static struct td_rast_case_table table;
static uint8_t records[(size_t)TD_RAST_MAX_CASES * TD_RAST_RECORD_SIZE];

static int16_t rec_word(const uint8_t *rec, uint16_t off, int i)
{
	return (int16_t)td_load_be16(rec + off + (size_t)i * 2);
}

/* Where a GEM binding packs pointer n -- stated here, not read from the
 * runner, so the test asserts the contract rather than the code. */
static int ptr_slot(int n)
{
	return 7 + n * (int)(sizeof(void *) / sizeof(int16_t));
}

int main(int argc, char **argv)
{
	const char *out = (argc > 1) ? argv[1] : "stub-raster.bin";
	const char *tsv = (argc > 2) ? argv[2] : "stub-raster.tsv";
	struct td_case_error err;
	struct td_wk wk;
	const uint8_t *r0, *r1;
	int16_t src_ptr[4], dst_ptr[4];
	int i, rc;

	printf("raster case table\n");
	memset(&err, 0, sizeof err);
	rc = td_rast_cases_parse(&table, table_text, strlen(table_text), &err);
	TD_CHECK_EQ(rc, 0);
	TD_CHECK_EQ(table.count, 2);

	printf("workstation open\n");
	td_stub_reset();
	TD_CHECK_EQ(td_gem_open(&wk), 0);
	TD_CHECK_EQ(wk.handle, TD_STUB_VWK_HANDLE);

	printf("the run\n");
	td_stub_reset();
	TD_CHECK_EQ(td_run_raster(&table, &wk, records, sizeof records), 0);

	/* The buffer must be refused rather than overrun. */
	TD_CHECK_EQ(td_run_raster(&table, &wk, records, TD_RAST_RECORD_SIZE),
	            TD_RUN_ERR_BUFFER);

	r0 = records;
	r1 = records + TD_RAST_RECORD_SIZE;

	/* case ids are the physical line numbers */
	TD_CHECK_EQ(td_load_be16(r0 + TD_RAST_OFF_CASEID), 2);
	TD_CHECK_EQ(td_load_be16(r1 + TD_RAST_OFF_CASEID), 3);
	TD_CHECK_EQ(td_load_be16(r0 + TD_RAST_OFF_OPCODE), 110);
	TD_CHECK_EQ(td_load_be16(r1 + TD_RAST_OFF_OPCODE), 109);
	TD_CHECK_EQ((int32_t)td_load_be32(r0 + TD_RAST_OFF_RET), TD_STUB_VDI_D0);

	/* --- contrl ------------------------------------------------------ */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_CONTRL, 0), 110);
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_CONTRL, 1), 0);   /* no points  */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_CONTRL, 3), 0);   /* no intin   */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_CONTRL, 5), 0);   /* no sub-op  */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_CONTRL, 6), TD_STUB_VWK_HANDLE);
	TD_CHECK_EQ(rec_word(r1, TD_RAST_OFF_CONTRL, 1), 4);   /* 8 values   */
	TD_CHECK_EQ(rec_word(r1, TD_RAST_OFF_CONTRL, 3), 1);

	/* contrl[2] and contrl[4] are outputs: the runner must hand them over
	 * poisoned, so a VDI that leaves them alone stays distinguishable from
	 * one that writes 0.  The stub writes 0 here, which is what proves the
	 * before-snapshot carries the poison and the after-copy does not. */
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_CONTRL_BEFORE, 2),
	            (uint16_t)TD_POISON16);
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_CONTRL_BEFORE, 4),
	            (uint16_t)TD_POISON16);
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_CONTRL, 2), 0);
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_CONTRL, 4), 0);

	/* --- the MFDB pointer slots -------------------------------------- */
	/* Recorded, and identical before and after: the VDI may not write its
	 * own input pointers.  The stub does not, so these must agree. */
	for (i = 0; i < (int)(sizeof(void *) / sizeof(int16_t)); i++) {
		src_ptr[i] = rec_word(r0, TD_RAST_OFF_CONTRL, ptr_slot(0) + i);
		dst_ptr[i] = rec_word(r0, TD_RAST_OFF_CONTRL, ptr_slot(1) + i);
		TD_CHECK_EQ(src_ptr[i],
		            rec_word(r0, TD_RAST_OFF_CONTRL_BEFORE, ptr_slot(0) + i));
		TD_CHECK_EQ(dst_ptr[i],
		            rec_word(r0, TD_RAST_OFF_CONTRL_BEFORE, ptr_slot(1) + i));
	}
	/* Two different MFDBs, so the two slots cannot hold the same address. */
	TD_CHECK(memcmp(src_ptr, dst_ptr,
	                sizeof(void *)) != 0);

	/* --- the MFDB fields --------------------------------------------- */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 0), 32);  /* fd_w      */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 1), 16);  /* fd_h      */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 2), 2);   /* fd_wdwidth*/
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 3), 1);   /* fd_stand  */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 4), 4);   /* fd_nplanes*/
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 5), 0);   /* fd_r1     */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 7), 0);   /* fd_r3     */
	/* The destination MFDB is recorded AFTER the call: the stub stamps
	 * fd_stand, standing in for the toggle vr_trnfm performs.  A record
	 * taken before the call would still say 0. */
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_DST_MFDB, 3),
	            (uint16_t)TD_STUB_RAST_STAND);
	/* ... and the pointer really was the destination MFDB, not the
	 * source: the source's fd_stand is untouched. */
	TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, 3), 1);

	/* --- the pixel buffers ------------------------------------------- */
	/* Laid with the case's seeds, not poisoned. */
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_A, 1),
	            td_rast_fill_word(11, 1));
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_B, 1),
	            td_rast_fill_word(22, 1));
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_A,
	                               TD_RAST_BUF_WORDS - 1),
	            td_rast_fill_word(11, TD_RAST_BUF_WORDS - 1));
	/* Recorded after the call: the stub stamps word 0 of the DESTINATION
	 * buffer, which case 1 put in B. */
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_B, 0),
	            (uint16_t)TD_STUB_RAST_PIXEL);
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_A, 0),
	            td_rast_fill_word(11, 0));
	/* Different seeds must actually give different buffers, or a logic op
	 * could not tell source from destination. */
	TD_CHECK(td_rast_fill_word(11, 7) != td_rast_fill_word(22, 7));
	TD_CHECK(td_rast_fill_word(11, 7) != td_rast_fill_word(11, 8));

	/* Case 2 puts the SOURCE in A and the destination in B again, with new
	 * seeds -- so the fill is re-laid per case, not once for the run. */
	TD_CHECK_EQ((uint16_t)rec_word(r1, TD_RAST_OFF_BUF_A, 3),
	            td_rast_fill_word(33, 3));
	TD_CHECK_EQ((uint16_t)rec_word(r1, TD_RAST_OFF_BUF_B, 3),
	            td_rast_fill_word(44, 3));

	/* --- the parameter arrays ---------------------------------------- */
	/* intin/ptsin are recorded AFTER the call; the stub mutates slot 0,
	 * which no real VDI may do, and that is what makes the violation
	 * visible in a plain byte comparison. */
	TD_CHECK_EQ((uint16_t)rec_word(r1, TD_RAST_OFF_INTIN, 0),
	            (uint16_t)TD_STUB_INTIN_MARK);
	TD_CHECK_EQ((uint16_t)rec_word(r1, TD_RAST_OFF_PTSIN, 0),
	            (uint16_t)TD_STUB_PTSIN_MARK);
	/* everything the case did not fill stays poisoned */
	TD_CHECK_EQ((uint16_t)rec_word(r1, TD_RAST_OFF_INTIN, 1),
	            (uint16_t)TD_POISON16);
	TD_CHECK_EQ((uint16_t)rec_word(r1, TD_RAST_OFF_PTSIN, 8),
	            (uint16_t)TD_POISON16);
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_INTOUT, 0),
	            (uint16_t)TD_POISON16);
	TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_PTSOUT, 0),
	            (uint16_t)TD_POISON16);
	/* the values the case did state come back unchanged */
	TD_CHECK_EQ(rec_word(r1, TD_RAST_OFF_PTSIN, 2), 15);
	TD_CHECK_EQ(rec_word(r1, TD_RAST_OFF_PTSIN, 7), 7);

	/* --- the reset ran before each case ------------------------------ */
	/* v_clrwk opens the reset sequence (spec section 5); two cases means
	 * two of them, before any raster call. */
	{
		int clrwk = 0, raster = 0;

		for (i = 0; i < td_stub_ncalls; i++) {
			if (td_stub_calls[i].opcode == 3)
				clrwk++;
			if (td_stub_calls[i].opcode == 109 ||
			    td_stub_calls[i].opcode == 110 ||
			    td_stub_calls[i].opcode == 121)
				raster++;
		}
		TD_CHECK_EQ(clrwk, 2);
		TD_CHECK_EQ(raster, 2);
	}

	/* --- the screen: preludes and the readback (cases.h, THE SCREEN) -- */
	/* A table that lays a pattern on the screen through two preludes,
	 * copies screen to screen, and reads a 4 x 3 rectangle back into
	 * buffer B.  The stub answers v_get_pixel with intout[1] = the handle
	 * (its default arm), and logs every call with its ptsin, so the test
	 * can see the readback's coordinates and where its words landed. */
	printf("the screen: preludes run before the call, the readback after\n");
	{
		static const char *screen_text =
			"!screen\t640,480,16\n"
			"+25\t2\t-\t-\t-\t-\tpen 2\n"
			"+109\t3\t0,0,31,15,100,60,131,75\t0,32,16,2,0,16\tS\t-\tpattern\n"
			"109\t6\t100,60,115,67,108,64,123,71\tS\tS\t7,8,100,60,4,3\ts to s\n";
		static struct td_rast_case_table st;
		static uint8_t srec[TD_RAST_RECORD_SIZE];
		int first_raster = -1, measured = -1, n105 = 0, pen = -1, k;
		int16_t planes;

		memset(&err, 0, sizeof err);
		rc = td_rast_cases_parse(&st, screen_text, strlen(screen_text), &err);
		TD_CHECK_EQ(rc, 0);
		TD_CHECK_EQ(st.count, 1);
		TD_CHECK_EQ(st.cases[0].prelude_count, 2);

		td_stub_reset();
		planes = td_rast_planes(&wk);
		TD_CHECK_EQ(td_stub_ncalls, 1);
		TD_CHECK_EQ(td_stub_calls[0].opcode, 102);       /* vq_extnd */
		TD_CHECK_EQ(td_stub_calls[0].intin[0], 1);
		TD_CHECK_EQ(planes, 0);        /* the stub leaves INQ_TAB[4] at 0 */

		td_stub_reset();
		TD_CHECK_EQ(td_run_raster(&st, &wk, srec, sizeof srec), 0);
		r0 = srec;
		TD_CHECK_EQ(td_load_be16(r0 + TD_RAST_OFF_CASEID), 4);
		TD_CHECK_EQ(td_load_be16(r0 + TD_RAST_OFF_OPCODE), 109);

		/* order: reset (opens with v_clrwk, ends with vs_clip 129), the
		 * two preludes, the measured call, then 12 v_get_pixel calls */
		for (k = 0; k < td_stub_ncalls; k++) {
			int16_t op = td_stub_calls[k].opcode;

			/* the reset issues its own vsf_color (pen 1); the prelude's
			 * is the last 25 before the first raster call */
			if (op == 25 && first_raster < 0)
				pen = k;
			if (op == 109 && first_raster < 0)
				first_raster = k;
			else if (op == 109)
				measured = k;
			if (op == 105)
				n105++;
		}
		TD_CHECK_EQ(td_stub_calls[0].opcode, 3);          /* v_clrwk */
		TD_CHECK(pen > 0);
		TD_CHECK(first_raster > pen);
		TD_CHECK(measured > first_raster);
		TD_CHECK_EQ(td_stub_calls[pen].intin[0], 2);      /* vsf_color 2 */
		TD_CHECK_EQ(td_stub_calls[first_raster].intin[0], 3);
		TD_CHECK_EQ(td_stub_calls[measured].intin[0], 6);
		TD_CHECK_EQ(n105, 4 * 3);
		TD_CHECK_EQ(td_stub_calls[measured + 1].opcode, 105);
		TD_CHECK_EQ(td_stub_calls[measured + 1].ptsin[0], 100);
		TD_CHECK_EQ(td_stub_calls[measured + 1].ptsin[1], 60);
		TD_CHECK_EQ(td_stub_calls[measured + 4].ptsin[0], 103);
		TD_CHECK_EQ(td_stub_calls[measured + 5].ptsin[0], 100);
		TD_CHECK_EQ(td_stub_calls[measured + 5].ptsin[1], 61);
		TD_CHECK_EQ(td_stub_calls[measured + 12].ptsin[0], 103);
		TD_CHECK_EQ(td_stub_calls[measured + 12].ptsin[1], 62);
		TD_CHECK_EQ(td_stub_ncalls, measured + 13);

		/* the readback landed in buffer B as a 32-wide form: the 4 x 3
		 * words hold what v_get_pixel answered, the rest keeps seed 8 */
		TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_BUF_B, 0), TD_STUB_VWK_HANDLE);
		TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_BUF_B, 3), TD_STUB_VWK_HANDLE);
		TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_BUF_B, 2 * 32 + 3),
		            TD_STUB_VWK_HANDLE);
		TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_B, 4),
		            td_rast_fill_word(8, 4));
		TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_B, 3 * 32),
		            td_rast_fill_word(8, 3 * 32));
		/* buffer A was laid with seed 7 and is untouched */
		TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_BUF_A, 0),
		            td_rast_fill_word(7, 0));

		/* the recorded arrays are the measured call's, not the readback's:
		 * intout stays poisoned, ptsin[2..7] are the case's corners */
		TD_CHECK_EQ((uint16_t)rec_word(r0, TD_RAST_OFF_INTOUT, 1),
		            (uint16_t)TD_POISON16);
		TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_PTSIN, 4), 108);
		TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_PTSIN, 7), 71);

		/* a screen MFDB is handed over as all zeros and recorded so */
		for (k = 0; k < TD_RAST_MFDB_N; k++) {
			TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_SRC_MFDB, k), 0);
			TD_CHECK_EQ(rec_word(r0, TD_RAST_OFF_DST_MFDB, k), 0);
		}
		r0 = records;
	}

	/* --- a wide table (cases.h, THE 32-PLANE SETTING; TASK/V69) ------- */
	/* The wide record: buffers of 1024 words laid with the case's seeds,
	 * the readback storing BOTH words of v_get_pixel per pixel, INTOUT[0]
	 * then INTOUT[1], at 2 * (row * 32 + col).  The stub's default arm
	 * answers intout[0] = the opcode (105) and intout[1] = the handle, so
	 * the two words are told apart and their order is visible. */
	printf("a wide table: the wide record, both readback words in order\n");
	{
		static const char *wide_text =
			"!screen\t640,480,32\n"
			"109\t3\t0,0,31,15,100,60,131,75\t0,32,16,2,0,32\tS\t7,8,100,60,4,3\tm to s\n"
			"109\t3\t0,0,31,15,0,0,31,15\t0,32,16,2,0,32\t1,32,16,2,0,32\t9,10\tm to m\n";
		static struct td_rast_case_table wt;
		static uint8_t wrec[2 * TD_RAST32_RECORD_SIZE];
		const uint8_t *w0, *w1;
		int k;

		memset(&err, 0, sizeof err);
		rc = td_rast_cases_parse(&wt, wide_text, strlen(wide_text), &err);
		TD_CHECK_EQ(rc, 0);
		TD_CHECK_EQ(wt.count, 2);
		TD_CHECK_EQ(td_rast_record_size(&wt), TD_RAST32_RECORD_SIZE);
		TD_CHECK_EQ(td_rast_area(&wt), TD_AREA_RASTER32);
		/* a narrow table keeps the narrow record and area */
		TD_CHECK_EQ(td_rast_record_size(&table), TD_RAST_RECORD_SIZE);
		TD_CHECK_EQ(td_rast_area(&table), TD_AREA_RASTER);

		/* refused rather than overrun: the narrow size is too small */
		TD_CHECK_EQ(td_run_raster(&wt, &wk, wrec, 2 * TD_RAST_RECORD_SIZE),
		            TD_RUN_ERR_BUFFER);
		td_stub_reset();
		TD_CHECK_EQ(td_run_raster(&wt, &wk, wrec, sizeof wrec), 0);
		w0 = wrec;
		w1 = wrec + TD_RAST32_RECORD_SIZE;
		TD_CHECK_EQ(td_load_be16(w0 + TD_RAST_OFF_CASEID), 2);
		TD_CHECK_EQ(td_load_be16(w1 + TD_RAST_OFF_CASEID), 3);
		TD_CHECK_EQ(rec_word(w0, TD_RAST_OFF_SRC_MFDB, 4), 32); /* planes */

		/* the readback: 4 x 3 pixels, two words each, high word first */
		for (k = 0; k < 3; k++) {
			int c;

			for (c = 0; c < 4; c++) {
				TD_CHECK_EQ(rec_word(w0, TD_RAST32_OFF_BUF_B,
				                     2 * (k * 32 + c)), 105);
				TD_CHECK_EQ(rec_word(w0, TD_RAST32_OFF_BUF_B,
				                     2 * (k * 32 + c) + 1),
				            TD_STUB_VWK_HANDLE);
			}
		}
		/* past the rectangle B keeps its fill, to the last of 1024 words */
		TD_CHECK_EQ((uint16_t)rec_word(w0, TD_RAST32_OFF_BUF_B, 8),
		            td_rast_fill_word(8, 8));
		TD_CHECK_EQ((uint16_t)rec_word(w0, TD_RAST32_OFF_BUF_B, 1023),
		            td_rast_fill_word(8, 1023));
		TD_CHECK_EQ((uint16_t)rec_word(w0, TD_RAST32_OFF_BUF_A, 1023),
		            td_rast_fill_word(7, 1023));
		/* the second case: both buffers laid whole with its seeds */
		TD_CHECK_EQ((uint16_t)rec_word(w1, TD_RAST32_OFF_BUF_A, 1000),
		            td_rast_fill_word(9, 1000));
		TD_CHECK_EQ((uint16_t)rec_word(w1, TD_RAST32_OFF_BUF_B, 1000),
		            td_rast_fill_word(10, 1000));
		/* the parameter arrays are poisoned and recorded as in a narrow
		 * record, at the narrow offsets */
		TD_CHECK_EQ((uint16_t)rec_word(w1, TD_RAST_OFF_INTOUT, 0),
		            (uint16_t)TD_POISON16);
		TD_CHECK_EQ(rec_word(w1, TD_RAST_OFF_CONTRL, 0), 109);
		TD_CHECK_EQ((uint16_t)rec_word(w1, TD_RAST_OFF_CONTRL_BEFORE, 2),
		            (uint16_t)TD_POISON16);
		for (k = 0; k < TD_RAST32_RESERVED_LEN; k++)
			TD_CHECK_EQ(w1[TD_RAST32_OFF_RESERVED + k], 0);

		/* the dump writer takes the wide area with the wide size, and
		 * refuses the narrow size for it */
		TD_CHECK_EQ(td_dump_write(out, TD_AREA_RASTER32, wt.count, "test",
		                          wt.crc, wrec,
		                          (size_t)wt.count * TD_RAST_RECORD_SIZE),
		            TD_DUMP_ERR_SIZE);
		/* the wide dump and its table, for test_tosdiff.sh */
		if (argc > 4) {
			FILE *f;

			TD_CHECK_EQ(td_dump_write(argv[3], TD_AREA_RASTER32, wt.count,
			                          "test", wt.crc, wrec,
			                          (size_t)wt.count *
			                          TD_RAST32_RECORD_SIZE), 0);
			f = fopen(argv[4], "wb");
			TD_CHECK(f != NULL);
			if (f) {
				TD_CHECK_EQ(fwrite(wide_text, 1, strlen(wide_text), f),
				            strlen(wide_text));
				fclose(f);
			}
		}
	}

	/* --- a well formed dump file ------------------------------------- */
	rc = td_dump_write(out, TD_AREA_RASTER, table.count,
	                   "test", table.crc, records,
	                   (size_t)table.count * TD_RAST_RECORD_SIZE);
	TD_CHECK_EQ(rc, 0);

	/* The table this dump was taken with, written out beside it: the
	 * comparator refuses a dump whose case-table CRC it cannot match, and
	 * a table kept only as a C string could not be handed to it. */
	{
		FILE *f = fopen(tsv, "wb");

		TD_CHECK(f != NULL);
		if (f) {
			TD_CHECK_EQ(fwrite(table_text, 1, strlen(table_text), f),
			            strlen(table_text));
			fclose(f);
		}
	}

	return TD_TEST_RESULT();
}
