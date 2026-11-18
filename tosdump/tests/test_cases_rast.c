/*
 * test_cases_rast.c -- the VDI raster case table loader and its guard.
 *
 * The guard is the reason this file is long.  EmuTOS does not bounds-check a
 * memory MFDB (setup_info, vdi_raster.c:915-950; vdi_vr_trnfm does not even
 * check fd_addr for NULL, :295-297), so a mistyped digit in a data file would
 * scribble over a running TT and the damage would be indistinguishable from a
 * defect in the thing being measured.  Every rule the loader enforces is
 * therefore asserted here with a table that breaks exactly that rule.
 */
#include "cases.h"
#include "tdtest.h"

static struct td_rast_case_table t;

static int parse(const char *text, struct td_case_error *err)
{
	memset(err, 0, sizeof *err);
	return td_rast_cases_parse(&t, text, strlen(text), err);
}

/* A form that fits: 32 x 16 pixels, fd_wdwidth 2, so 16 * 2 * 16 = 512
 * words, exactly TD_RAST_BUF_WORDS. */
#define FORM_A "0,32,16,2,1,4"
#define FORM_B "1,32,16,2,0,4"

static void test_good_table(void)
{
	struct td_case_error err;
	const char *text =
		"# a comment consumes a line number\n"
		"\n"
		"110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\tvr_trnfm standard to device\n"
		"109\t3\t0,0,15,7,0,8,15,15\t" FORM_A "\t" FORM_B "\t3,4\t"
			"vro_cpyfm replace\n"
		"121\t1,2,3\t0,0,15,7,0,0,15,7\t" FORM_A "\t" FORM_B "\t5,6\t"
			"vrt_cpyfm replace\n";

	printf("a well formed raster table\n");
	TD_CHECK_EQ(parse(text, &err), 0);
	TD_CHECK_EQ(t.count, 3);
	TD_CHECK(t.crc != 0);

	/* Case ids are PHYSICAL line numbers: the comment and the blank line
	 * each consume one. */
	TD_CHECK_EQ(t.cases[0].line, 3);
	TD_CHECK_EQ(t.cases[1].line, 4);
	TD_CHECK_EQ(t.cases[2].line, 5);

	TD_CHECK_EQ(t.cases[0].opcode, 110);
	TD_CHECK_EQ(t.cases[0].n_intin, 0);
	TD_CHECK_EQ(t.cases[0].n_ptsin, 0);
	TD_CHECK_EQ(t.cases[0].src.buffer, TD_RAST_BUF_A);
	TD_CHECK_EQ(t.cases[0].src.w, 32);
	TD_CHECK_EQ(t.cases[0].src.h, 16);
	TD_CHECK_EQ(t.cases[0].src.wdwidth, 2);
	TD_CHECK_EQ(t.cases[0].src.stand, 1);
	TD_CHECK_EQ(t.cases[0].src.nplanes, 4);
	TD_CHECK_EQ(t.cases[0].dst.buffer, TD_RAST_BUF_B);
	TD_CHECK_EQ(t.cases[0].dst.stand, 0);
	TD_CHECK_EQ(t.cases[0].seed_a, 1);
	TD_CHECK_EQ(t.cases[0].seed_b, 2);
	TD_CHECK(strcmp(t.cases[0].comment,
	                "vr_trnfm standard to device") == 0);

	TD_CHECK_EQ(t.cases[1].opcode, 109);
	TD_CHECK_EQ(t.cases[1].n_intin, 1);
	TD_CHECK_EQ(t.cases[1].intin[0], 3);
	TD_CHECK_EQ(t.cases[1].n_ptsin, 8);
	TD_CHECK_EQ(t.cases[1].ptsin[3], 7);
	TD_CHECK_EQ(t.cases[1].seed_a, 3);

	TD_CHECK_EQ(t.cases[2].opcode, 121);
	TD_CHECK_EQ(t.cases[2].n_intin, 3);
	TD_CHECK_EQ(t.cases[2].intin[2], 3);
}

/* One line, built from the pieces a test wants to break. */
static void one(char *dst, size_t n, const char *op, const char *intin,
                const char *ptsin, const char *src, const char *dst_mfdb,
                const char *fill)
{
	snprintf(dst, n, "%s\t%s\t%s\t%s\t%s\t%s\tcase\n", op, intin, ptsin,
	         src, dst_mfdb, fill);
}

#define REJECT(want, op, intin, ptsin, src, dstm, fill)                       \
	do {                                                                      \
		char line_[512];                                                      \
		struct td_case_error e_;                                              \
		one(line_, sizeof line_, op, intin, ptsin, src, dstm, fill);          \
		TD_CHECK_EQ(parse(line_, &e_), want);                                 \
		TD_CHECK_EQ(e_.line, 1);                                              \
		TD_CHECK(e_.what != NULL);                                            \
	} while (0)

#define ACCEPT(op, intin, ptsin, src, dstm, fill)                             \
	do {                                                                      \
		char line_[512];                                                      \
		struct td_case_error e_;                                              \
		one(line_, sizeof line_, op, intin, ptsin, src, dstm, fill);          \
		TD_CHECK_EQ(parse(line_, &e_), 0);                                    \
		TD_CHECK_EQ(t.count, 1);                                              \
	} while (0)

static void test_opcode_guard(void)
{
	printf("only the three raster opcodes may be issued\n");
	/* v_pline is a perfectly good VDI opcode and has no business in a
	 * table whose record carries two MFDBs. */
	REJECT(TD_CASE_ERR_UNSAFE_OPCODE, "6", "-", "-", FORM_A, FORM_B, "1,2");
	REJECT(TD_CASE_ERR_UNSAFE_OPCODE, "111", "-", "-", FORM_A, FORM_B, "1,2");
	REJECT(TD_CASE_ERR_UNSAFE_OPCODE, "0", "-", "-", FORM_A, FORM_B, "1,2");
}

static void test_mfdb_guard(void)
{
	printf("the MFDB columns\n");

	/* five values instead of six */
	REJECT(TD_CASE_ERR_COLUMNS, "110", "-", "-", "0,32,16,2,1", FORM_B, "1,2");
	/* there is no buffer C, and no screen MFDB */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "2,32,16,2,1,4", FORM_B,
	       "1,2");
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "-1,32,16,2,1,4", FORM_B,
	       "1,2");
	/* fd_stand is a flag */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "0,32,16,2,2,4", FORM_B,
	       "1,2");
	/* zero and negative geometry */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "0,32,0,2,1,4", FORM_B,
	       "1,2");
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "0,32,16,2,1,0", FORM_B,
	       "1,2");
	/* fd_w wider than fd_wdwidth describes */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "0,33,16,2,1,4", FORM_B,
	       "1,2");
	/* past the VDI's own plane cap */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "0,32,16,2,1,17", FORM_B,
	       "1,2");
	/* the bound that matters: fd_h * fd_wdwidth * 16 must fit the buffer.
	 * 17 rows of 2 words is 544 > 512, one row too many. */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", "0,32,17,2,1,4", FORM_B,
	       "1,2");
	/* the destination is bounded by the same rule */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "-", FORM_A, "1,32,17,2,0,4",
	       "1,2");
	/* 16 planes is allowed: it is the CONF_WITH_VDI_16BIT cap, and whether
	 * the ROM under measurement honours it is exactly what a case asks */
	ACCEPT("110", "-", "-", "0,32,16,2,1,16", "1,32,16,2,0,16", "1,2");
}

static void test_rect_guard(void)
{
	printf("the rectangles\n");

	/* vro_cpyfm and vrt_cpyfm read PTSIN[0..7]; six values would leave
	 * PTSIN[6..7] at the 0xDEAD poison, which is -8531 */
	REJECT(TD_CASE_ERR_GEOMETRY, "109", "3", "0,0,15,7,0,0", FORM_A, FORM_B,
	       "1,2");
	REJECT(TD_CASE_ERR_ODD_PTSIN, "109", "3", "0,0,15,7,0,0,15", FORM_A,
	       FORM_B, "1,2");
	/* vr_trnfm reads no rectangle */
	REJECT(TD_CASE_ERR_GEOMETRY, "110", "-", "0,0,1,1", FORM_A, FORM_B,
	       "1,2");

	/* the source rect leaves the source form (fd_w is 32, fd_h is 16) */
	REJECT(TD_CASE_ERR_GEOMETRY, "109", "3", "0,0,32,7,0,0,15,7", FORM_A,
	       FORM_B, "1,2");
	REJECT(TD_CASE_ERR_GEOMETRY, "109", "3", "0,0,15,16,0,0,15,7", FORM_A,
	       FORM_B, "1,2");
	REJECT(TD_CASE_ERR_GEOMETRY, "109", "3", "-1,0,15,7,0,0,15,7", FORM_A,
	       FORM_B, "1,2");
	/* the destination extent is the SOURCE extent, not PTSIN[6..7]
	 * (dont_clip, vdi_raster.c:882-900): a 16x8 source landing at (24,0)
	 * runs off a 32 pixel wide destination even though PTSIN[6..7] would
	 * look harmless */
	REJECT(TD_CASE_ERR_GEOMETRY, "109", "3", "0,0,15,7,24,0,31,7", FORM_A,
	       FORM_B, "1,2");
	/* reversed corners are legal: the VDI sorts them itself (arb_corner,
	 * vdi_raster.c:116-124), and a degenerate rectangle is a case worth
	 * measuring */
	ACCEPT("109", "3", "15,7,0,0,0,0,15,7", FORM_A, FORM_B, "1,2");
	ACCEPT("109", "3", "5,5,5,5,0,0,0,0", FORM_A, FORM_B, "1,2");
}

static void test_misc_guard(void)
{
	struct td_case_error err;

	printf("columns, fill and preludes\n");

	/* six columns */
	TD_CHECK_EQ(parse("110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\n", &err),
	            TD_CASE_ERR_COLUMNS);
	/* the fill column is two seeds -- or two seeds and a readback
	 * rectangle, six values; anything else is refused */
	REJECT(TD_CASE_ERR_COLUMNS, "110", "-", "-", FORM_A, FORM_B, "1");
	REJECT(TD_CASE_ERR_COLUMNS, "110", "-", "-", FORM_A, FORM_B, "1,2,3");
	REJECT(TD_CASE_ERR_TOO_MANY_VALUES, "110", "-", "-", FORM_A, FORM_B,
	       "1,2,3,4,5,6,7");
	/* "-" is an empty list, which is not two seeds */
	REJECT(TD_CASE_ERR_COLUMNS, "110", "-", "-", FORM_A, FORM_B, "-");

	/* a vr_trnfm prelude is refused: only the whitelist of cases.h (THE
	 * SCREEN) may precede a case, and 110 is not on it */
	TD_CHECK_EQ(parse("+110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\tp\n"
	                  "110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\tc\n", &err),
	            TD_CASE_ERR_UNSAFE_OPCODE);
	TD_CHECK_EQ(err.line, 1);
	/* and the screen, S, cannot be named without a !screen line */
	TD_CHECK_EQ(parse("109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                  &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(err.line, 1);

	/* hex spellings reach the arrays as the same bits */
	ACCEPT("109", "0x13", "0,0,15,7,0,0,15,7", FORM_A, FORM_B, "0xDEAD,0");
	TD_CHECK_EQ(t.cases[0].intin[0], 0x13);
	TD_CHECK_EQ(t.cases[0].seed_a, 0xDEAD);

	/* both MFDBs may name the same buffer -- that is how the in-place
	 * vr_trnfm cases are stated */
	ACCEPT("110", "-", "-", FORM_A, "0,32,16,2,0,4", "1,2");
	TD_CHECK_EQ(t.cases[0].src.buffer, t.cases[0].dst.buffer);
}

/*
 * The screen (cases.h, THE SCREEN): the !screen directive, S as an MFDB,
 * the prelude whitelist and the readback -- and the guard on each.
 */
#define SCREEN "!screen\t640,480,16\n"
#define MEM16  "0,32,16,2,0,16"
#define MONO   "0,32,16,2,0,1"

static int parse2(const char *head, const char *line, struct td_case_error *e)
{
	char buf[1024];

	snprintf(buf, sizeof buf, "%s%s", head, line);
	return parse(buf, e);
}

static void test_screen_rules(void)
{
	struct td_case_error err;

	printf("the screen: directive, S, preludes, readback\n");

	/* a well formed screen table: screen -> memory, memory -> screen with
	 * a readback, a pattern prelude and a screen -> screen copy */
	TD_CHECK_EQ(parse2(SCREEN,
		"109\t3\t0,0,31,15,0,0,31,15\tS\t1,32,16,2,0,16\t1,2\ts to m\n"
		"109\t3\t0,0,31,15,100,60,131,75\t" MEM16 "\tS\t3,4,100,60,32,16\tm to s\n"
		"+25\t2\t-\t-\t-\t-\tpen 2\n"
		"+114\t-\t100,60,131,75\t-\t-\t-\tfill\n"
		"+129\t1\t0,0,639,479\t-\t-\t-\tclip\n"
		"+109\t3\t0,0,31,15,100,60,131,75\t" MEM16 "\tS\t-\tpattern\n"
		"+121\t1,1,0\t0,0,31,15,100,60,131,75\t" MONO "\tS\t-\tmono\n"
		"109\t3\t100,60,115,67,108,64,123,71\tS\tS\t5,6,100,60,32,16\ts to s\n",
		&err), 0);
	TD_CHECK_EQ(t.count, 3);
	TD_CHECK_EQ(t.screen.declared, 1);
	TD_CHECK_EQ(t.screen.w, 640);
	TD_CHECK_EQ(t.screen.h, 480);
	TD_CHECK_EQ(t.screen.planes, 16);
	/* case ids: the directive consumes line 1 */
	TD_CHECK_EQ(t.cases[0].line, 2);
	TD_CHECK_EQ(t.cases[1].line, 3);
	TD_CHECK_EQ(t.cases[2].line, 9);
	TD_CHECK_EQ(t.cases[0].src.buffer, TD_RAST_BUF_SCREEN);
	TD_CHECK_EQ(t.cases[0].src.w, 640);
	TD_CHECK_EQ(t.cases[0].src.h, 480);
	TD_CHECK_EQ(t.cases[0].src.nplanes, 16);
	TD_CHECK_EQ(t.cases[0].has_read, 0);
	TD_CHECK_EQ(t.cases[1].has_read, 1);
	TD_CHECK_EQ(t.cases[1].rx, 100);
	TD_CHECK_EQ(t.cases[1].ry, 60);
	TD_CHECK_EQ(t.cases[1].rw, 32);
	TD_CHECK_EQ(t.cases[1].rh, 16);
	TD_CHECK_EQ(t.cases[1].seed_a, 3);
	TD_CHECK_EQ(t.cases[1].seed_b, 4);
	TD_CHECK_EQ(t.cases[1].prelude_count, 0);
	TD_CHECK_EQ(t.cases[2].prelude_first, 0);
	TD_CHECK_EQ(t.cases[2].prelude_count, 5);
	TD_CHECK_EQ(t.prelude_count, 5);
	TD_CHECK_EQ(t.preludes[0].opcode, 25);
	TD_CHECK_EQ(t.preludes[0].intin[0], 2);
	TD_CHECK_EQ(t.preludes[1].opcode, 114);
	TD_CHECK_EQ(t.preludes[1].n_ptsin, 4);
	TD_CHECK_EQ(t.preludes[2].opcode, 129);
	TD_CHECK_EQ(t.preludes[3].opcode, 109);
	TD_CHECK_EQ(t.preludes[3].src.buffer, TD_RAST_BUF_A);
	TD_CHECK_EQ(t.preludes[3].dst.buffer, TD_RAST_BUF_SCREEN);
	TD_CHECK_EQ(t.preludes[4].opcode, 121);
	TD_CHECK_EQ(t.preludes[4].n_intin, 3);
	TD_CHECK(strcmp(t.cases[2].comment, "s to s") == 0);

	/* the directive: once, before the first case, three sane values */
	TD_CHECK_EQ(parse(SCREEN SCREEN, &err), TD_CASE_ERR_DIRECTIVE);
	TD_CHECK_EQ(err.line, 2);
	TD_CHECK_EQ(parse2("110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\tc\n",
	                   SCREEN, &err), TD_CASE_ERR_DIRECTIVE);
	TD_CHECK_EQ(err.line, 2);
	TD_CHECK_EQ(parse("!screen\t640,480\n", &err), TD_CASE_ERR_DIRECTIVE);
	TD_CHECK_EQ(parse("!screen\t640,480,17\n", &err), TD_CASE_ERR_DIRECTIVE);
	TD_CHECK_EQ(parse("!screen\t0,480,16\n", &err), TD_CASE_ERR_DIRECTIVE);
	TD_CHECK_EQ(parse("!other\t1\n", &err), TD_CASE_ERR_DIRECTIVE);
	/* the old table's contract: no directive, no screen, nothing changes */
	TD_CHECK_EQ(parse("110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\tc\n", &err), 0);
	TD_CHECK_EQ(t.screen.declared, 0);

	/* S: the rectangle guard uses the declared extent */
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,31,15,620,470,651,485\t" MEM16
	                   "\tS\t1,2\tc\n", &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,31,15,608,464,639,479\t" MEM16
	                   "\tS\t1,2\tc\n", &err), 0);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t620,470,651,485,0,0,31,15\tS\t"
	                   "1,32,16,2,0,16\t1,2\tc\n", &err), TD_CASE_ERR_GEOMETRY);
	/* vr_trnfm may not name the screen */
	TD_CHECK_EQ(parse2(SCREEN, "110\t-\t-\tS\t" FORM_B "\t1,2\tc\n", &err),
	            TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "110\t-\t-\t" FORM_A "\tS\t1,2\tc\n", &err),
	            TD_CASE_ERR_GEOMETRY);
	/* the buffer field 2 is not a spelling of the screen */
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\t2,640,480,40,0,16\t"
	                   FORM_B "\t1,2\tc\n", &err), TD_CASE_ERR_GEOMETRY);

	/* preludes: the whitelist, the shapes, the rectangles */
	TD_CHECK_EQ(parse2(SCREEN, "+3\t-\t-\t-\t-\t-\tclrwk\n"
	                   "109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                   &err), TD_CASE_ERR_UNSAFE_OPCODE);
	TD_CHECK_EQ(err.line, 2);
	TD_CHECK_EQ(parse2(SCREEN, "+25\t2\t-\t-\t-\t1,2\tfill given\n"
	                   "109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                   &err), TD_CASE_ERR_COLUMNS);
	TD_CHECK_EQ(parse2(SCREEN, "+25\t2\t-\t" MEM16 "\t-\t-\tmfdb given\n"
	                   "109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                   &err), TD_CASE_ERR_COLUMNS);
	TD_CHECK_EQ(parse2(SCREEN, "+25\t2,3\t-\t-\t-\t-\ttwo intin\n"
	                   "109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                   &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "+114\t-\t600,400,700,500\t-\t-\t-\toff\n"
	                   "109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                   &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "+129\t2\t0,0,10,10\t-\t-\t-\tflag 2\n"
	                   "109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                   &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "+109\t3\t0,0,31,15,620,470,651,485\t" MEM16
	                   "\tS\t-\toff screen\n"
	                   "109\t3\t0,0,7,7,0,0,7,7\tS\t" FORM_B "\t1,2\tc\n",
	                   &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "+114\t-\t0,0,10,10\t-\t-\t-\tdangling\n",
	                   &err), TD_CASE_ERR_DANGLING);
	TD_CHECK_EQ(err.line, 2);
	/* without !screen, vr_recfl has no extent to be checked against */
	TD_CHECK_EQ(parse("+114\t-\t0,0,10,10\t-\t-\t-\tno screen\n"
	                  "110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\tc\n", &err),
	            TD_CASE_ERR_GEOMETRY);

	/* the readback: needs the screen in the case, keeps B free, fits */
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\t" FORM_A "\t"
	                   FORM_B "\t1,2,0,0,8,8\tno screen operand\n", &err),
	            TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\tS\t"
	                   FORM_B "\t1,2,0,0,8,8\tB named\n", &err),
	            TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\t" FORM_A "\tS\t"
	                   "1,2,0,0,33,8\ttoo wide\n", &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\t" FORM_A "\tS\t"
	                   "1,2,0,0,8,17\ttoo high\n", &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\t" FORM_A "\tS\t"
	                   "1,2,620,470,32,16\toff screen\n", &err),
	            TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\t" FORM_A "\tS\t"
	                   "1,2,0,0,0,8\tempty\n", &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN, "109\t3\t0,0,7,7,0,0,7,7\t" FORM_A "\tS\t"
	                   "1,2,608,464,32,16\tlast pixel\n", &err), 0);
	TD_CHECK_EQ(t.cases[0].has_read, 1);
	/* a readback without a directive has no screen to read */
	TD_CHECK_EQ(parse("109\t3\t0,0,7,7,0,0,7,7\t" FORM_A "\t" FORM_B
	                  "\t1,2,0,0,8,8\tno directive\n", &err),
	            TD_CASE_ERR_GEOMETRY);
}

/*
 * The 32-plane setting (cases.h, THE 32-PLANE SETTING; TASK/V69): a table
 * is wide iff it declares `!screen w,h,32`.  Wide: fd_nplanes 1..32 and the
 * bound fd_h * fd_wdwidth * 32 <= TD_RAST32_BUF_WORDS.  Not wide: every
 * rule as before -- the 17-plane refusals above still hold.
 */
#define SCREEN32 "!screen\t640,480,32\n"
#define MEM32    "0,32,16,2,0,32"

static void test_wide_rules(void)
{
	struct td_case_error err;

	printf("the 32-plane setting: a wide table\n");

	/* 32 is accepted, and only 32 above 16 */
	TD_CHECK_EQ(parse(SCREEN32, &err), 0);
	TD_CHECK_EQ(t.screen.declared, 1);
	TD_CHECK_EQ(t.screen.planes, 32);
	TD_CHECK_EQ(t.screen.wide, 1);
	TD_CHECK_EQ(parse("!screen\t640,480,33\n", &err), TD_CASE_ERR_DIRECTIVE);
	TD_CHECK_EQ(parse("!screen\t640,480,24\n", &err), TD_CASE_ERR_DIRECTIVE);
	TD_CHECK_EQ(parse("!screen\t640,480,31\n", &err), TD_CASE_ERR_DIRECTIVE);
	/* a 16-plane table, and a table without the directive, are not wide */
	TD_CHECK_EQ(parse(SCREEN, &err), 0);
	TD_CHECK_EQ(t.screen.wide, 0);
	TD_CHECK_EQ(parse("110\t-\t-\t" FORM_A "\t" FORM_B "\t1,2\tc\n", &err), 0);
	TD_CHECK_EQ(t.screen.wide, 0);

	/* a 32 x 16 form at 32 planes: 16 * 2 * 32 = 1024 words, the whole
	 * wide buffer; the screen at 32 planes; a readback */
	TD_CHECK_EQ(parse2(SCREEN32,
		"109\t3\t0,0,31,15,0,0,31,15\tS\t1,32,16,2,0,32\t1,2\ts to m\n"
		"109\t3\t0,0,31,15,100,60,131,75\t" MEM32 "\tS\t3,4,100,60,32,16\tm to s\n"
		"110\t-\t-\t0,32,16,2,1,32\t1,32,16,2,0,32\t5,6\ttrnfm\n",
		&err), 0);
	TD_CHECK_EQ(t.count, 3);
	TD_CHECK_EQ(t.cases[0].src.nplanes, 32);
	TD_CHECK_EQ(t.cases[0].dst.nplanes, 32);
	TD_CHECK_EQ(t.cases[1].has_read, 1);
	/* one row more is past the wide buffer: 17 * 2 * 32 = 1088 */
	TD_CHECK_EQ(parse2(SCREEN32, "110\t-\t-\t0,32,17,2,1,32\t" MEM32
	                   "\t1,2\tc\n", &err), TD_CASE_ERR_GEOMETRY);
	/* the wide bound holds for every plane count: a 1-plane 32 x 17 form
	 * fits the narrow rule (544 > 512 refused there) but not this one */
	TD_CHECK_EQ(parse2(SCREEN32, "109\t3\t0,0,7,7,0,0,7,7\t0,32,17,2,0,1\t"
	                   "1,32,16,2,0,1\t1,2\tc\n", &err), TD_CASE_ERR_GEOMETRY);
	/* 9..16-plane forms are writable in a wide table: refusal cases */
	TD_CHECK_EQ(parse2(SCREEN32, "109\t3\t0,0,15,15,0,0,15,15\t"
	                   "0,32,16,2,0,16\t1,32,16,2,0,16\t1,2\tc\n", &err), 0);
	TD_CHECK_EQ(parse2(SCREEN32, "109\t3\t0,0,15,15,0,0,15,15\t"
	                   "0,32,16,2,0,12\t1,32,16,2,0,12\t1,2\tc\n", &err), 0);
	/* above 32 planes is still refused */
	TD_CHECK_EQ(parse2(SCREEN32, "110\t-\t-\t0,16,16,1,1,33\t1,16,16,1,0,33"
	                   "\t1,2\tc\n", &err), TD_CASE_ERR_GEOMETRY);
	/* and outside a wide table 32 planes stays refused */
	TD_CHECK_EQ(parse2(SCREEN, "110\t-\t-\t0,16,16,1,1,32\t1,16,16,1,0,32"
	                   "\t1,2\tc\n", &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse("110\t-\t-\t0,16,16,1,1,32\t1,16,16,1,0,32\t1,2\tc\n",
	                  &err), TD_CASE_ERR_GEOMETRY);
	/* the readback rectangle keeps its limits: 32 x 16 pixels */
	TD_CHECK_EQ(parse2(SCREEN32, "109\t3\t0,0,7,7,0,0,7,7\t" MEM32 "\tS\t"
	                   "1,2,0,0,33,8\ttoo wide\n", &err), TD_CASE_ERR_GEOMETRY);
	TD_CHECK_EQ(parse2(SCREEN32, "109\t3\t0,0,7,7,0,0,7,7\t" MEM32 "\tS\t"
	                   "1,2,0,0,32,17\ttoo high\n", &err), TD_CASE_ERR_GEOMETRY);
}

/*
 * The shipped table.  It is carried to a machine that has no debugger, so
 * "the guard accepts it" is not something to find out there.
 */
static void test_shipped_table(void)
{
	struct td_case_error err;
	int rc;

	printf("the shipped raster table passes the guard\n");

	memset(&err, 0, sizeof err);
	rc = td_rast_cases_load(&t, "../cases/vdi-raster.tsv", &err);
	if (rc != 0) {
		printf("  FAIL cases/vdi-raster.tsv line %u: %s (%s)\n", err.line,
		       err.what ? err.what : "", td_case_error_text(rc));
		td_test_failures++;
		return;
	}
	TD_CHECK_EQ(t.count, 59);
	TD_CHECK(t.count <= TD_RAST_MAX_CASES);
	/* Only the three raster opcodes, and all three of them present. */
	{
		int n109 = 0, n110 = 0, n121 = 0;
		uint16_t i;

		for (i = 0; i < t.count; i++) {
			switch (t.cases[i].opcode) {
			case 109: n109++; break;
			case 110: n110++; break;
			case 121: n121++; break;
			default:  TD_CHECK(0); break;
			}
		}
		TD_CHECK(n109 > 0);
		TD_CHECK(n110 > 0);
		TD_CHECK(n121 > 0);
	}
	TD_CHECK_EQ(t.screen.wide, 0);

	/* the two 32-plane tables (TASK/V69): wide, and within the guard */
	{
		static const char *wide_tables[] = {
			"../cases/vdi-raster32.tsv", "../cases/vdi-raster-screen32.tsv"
		};
		int k;

		for (k = 0; k < 2; k++) {
			memset(&err, 0, sizeof err);
			rc = td_rast_cases_load(&t, wide_tables[k], &err);
			if (rc != 0) {
				printf("  FAIL %s line %u: %s (%s)\n", wide_tables[k],
				       err.line, err.what ? err.what : "",
				       td_case_error_text(rc));
				td_test_failures++;
				continue;
			}
			TD_CHECK_EQ(t.screen.wide, 1);
			TD_CHECK_EQ(t.screen.planes, 32);
			TD_CHECK_EQ(t.count, TD_RAST_MAX_CASES);
		}
	}
	/* the shipped 16-plane screen table stays narrow */
	memset(&err, 0, sizeof err);
	TD_CHECK_EQ(td_rast_cases_load(&t, "../cases/vdi-raster-screen.tsv", &err), 0);
	TD_CHECK_EQ(t.screen.wide, 0);
	TD_CHECK_EQ(t.screen.planes, 16);
}

int main(void)
{
	test_good_table();
	test_opcode_guard();
	test_mfdb_guard();
	test_rect_guard();
	test_misc_guard();
	test_screen_rules();
	test_wide_rules();
	test_shipped_table();
	return TD_TEST_RESULT();
}
