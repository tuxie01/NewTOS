/*
 * test_peek.c -- peek cases (TASK/BIOS21): the loader's form and its guard,
 * and the call runner against the stub's TOS memory window.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "System variables: the peek
 * case"; the record: core/tosdump.h, "A PEEK CASE".  The stub's window and
 * its Vsync: stub_abi.h, "System variables".  The expected values are
 * written out here, not read from core/: a parser that agreed with itself
 * would prove nothing.
 *
 * Writes the stub dump and its table to argv[1] / argv[2] for the
 * comparator tests (test_tosdiff.sh, "peek cases").
 */
#include "tdtest.h"
#include "cases.h"
#include "runner_call.h"
#include "dump.h"
#include "abi.h"
#include "stub_abi.h"

static struct td_call_case_table t;

static int parse(const char *text, struct td_case_error *err)
{
	memset(err, 0, sizeof *err);
	return td_call_cases_parse(&t, TD_AREA_BIOS, text, strlen(text), err);
}

/* A table; the refusal must name `want_line`. */
static void expect_refused(const char *text, int want_rc, uint16_t want_line,
                           const char *why)
{
	struct td_case_error err;
	int rc = parse(text, &err);

	if (rc != want_rc) {
		printf("  FAIL %s: gave %d, want %d (%s)\n", why, rc, want_rc,
		       err.what ? err.what : "");
		td_test_failures++;
		return;
	}
	if (err.line != want_line) {
		printf("  FAIL %s: refusal names line %u, want %u\n", why,
		       err.line, want_line);
		td_test_failures++;
	}
	if (!err.what || !*err.what) {
		printf("  FAIL %s: refusal carries no reason\n", why);
		td_test_failures++;
	}
}

static void test_form(void)
{
	struct td_case_error err;
	int rc;

	printf("peek:<addr>:<width>: trap 257, the width as opcode, the "
	       "address as LONG argument 0\n");
	rc = parse("# peek\n"
	           "peek:0x442:2\t-\tEXACT\t-\t_timer_ms\n"
	           "peek:0x484:1\t-\tEXACT\t-\tconterm\n"
	           "peek:0x4BA:4\t-\tMONOTONE\t-\t_hz_200\n"
	           "peek:0x4F2:4\t-\tPTR_RAM\t-\t_sysbase\n"
	           "peek:0x4C2:4\t-\tNONZERO\t-\t_drvbits\n"
	           "peek:1184:4\t-\tIGNORE\t-\tdecimal\n"
	           "peek:0x380:4\t-\tEXACT\t-\tthe lowest cell\n"
	           "peek:0x7FF:1\t-\tEXACT\t-\tthe highest byte\n"
	           "peek:0x7FC:4\t-\tEXACT\t-\tthe highest LONG\n", &err);
	TD_CHECK_EQ(rc, 0);
	TD_CHECK_EQ(t.count, 9);
	TD_CHECK_EQ(t.cases[0].line, 2);
	TD_CHECK_EQ(t.cases[0].trap, 257);
	TD_CHECK_EQ(t.cases[0].opcode, 2);
	TD_CHECK_EQ(t.cases[0].nargs, 1);
	TD_CHECK_EQ(t.cases[0].arg[0].kind, TD_CALL_ARG_LONG);
	TD_CHECK_EQ(t.cases[0].arg[0].value, 0x442);
	TD_CHECK_EQ(t.cases[0].retclass, TD_RET_EXACT);
	TD_CHECK_EQ(t.cases[0].retwidth, TD_RET_WIDTH_LONG);
	TD_CHECK_EQ(t.cases[0].cap_arg, -1);
	TD_CHECK_EQ(t.cases[0].cap_layout, 0);
	TD_CHECK_EQ(t.cases[0].prelude_count, 0);
	TD_CHECK(strcmp(t.cases[0].comment, "_timer_ms") == 0);
	TD_CHECK_EQ(t.cases[1].opcode, 1);
	TD_CHECK_EQ(t.cases[2].opcode, 4);
	TD_CHECK_EQ(t.cases[2].retclass, TD_RET_MONOTONE);
	TD_CHECK_EQ(t.cases[3].retclass, TD_RET_PTR_RAM);
	TD_CHECK_EQ(t.cases[4].retclass, TD_RET_NONZERO);
	TD_CHECK_EQ(t.cases[5].retclass, TD_RET_IGNORE);
	TD_CHECK_EQ(t.cases[5].arg[0].value, 0x4A0);
	TD_CHECK_EQ(t.cases[6].arg[0].value, 0x380);
	TD_CHECK_EQ(t.cases[7].arg[0].value, 0x7FF);
	TD_CHECK(strcmp(td_retclass_name(TD_RET_MONOTONE), "MONOTONE") == 0);

	printf("peek cases mix with trap and jar cases, which keep their guard\n");
	rc = parse("13:1\tw:2\tEXACT\t-\tBconstat(2)\n"
	           "peek:0x420:4\t-\tEXACT\t-\tmemvalid\n"
	           "jar:_MCH\t-\tEXACT\t-\t_MCH\n"
	           "14:0x04\t-\tEXACT.W\t-\tGetrez\n", &err);
	TD_CHECK_EQ(rc, 0);
	TD_CHECK_EQ(t.count, 4);
	TD_CHECK_EQ(t.cases[1].line, 2);
	TD_CHECK_EQ(t.cases[1].trap, 257);
	TD_CHECK_EQ(t.cases[2].trap, 256);
	TD_CHECK_EQ(t.cases[3].trap, 14);
}

static void test_refusals(void)
{
	printf("the address: inside $380..$7FF, a WORD or LONG even\n");
	expect_refused("peek:0x37F:1\t-\tEXACT\t-\tx\n", TD_CASE_ERR_UNSAFE_FIELD,
	               1, "below the area");
	expect_refused("peek:0x37E:4\t-\tEXACT\t-\tx\n", TD_CASE_ERR_UNSAFE_FIELD,
	               1, "a LONG that starts below the area");
	expect_refused("peek:0x800:1\t-\tEXACT\t-\tx\n", TD_CASE_ERR_UNSAFE_FIELD,
	               1, "at the area's end");
	expect_refused("peek:0x7FE:4\t-\tEXACT\t-\tx\n", TD_CASE_ERR_UNSAFE_FIELD,
	               1, "a LONG that runs past the area");
	expect_refused("peek:0:4\t-\tEXACT\t-\tx\n", TD_CASE_ERR_UNSAFE_FIELD,
	               1, "address 0");
	expect_refused("peek:0xFFFF8260:2\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE,
	               1, "a hardware register");
	expect_refused("peek:0x443:2\t-\tEXACT\t-\tx\n", TD_CASE_ERR_UNSAFE_FIELD,
	               1, "an odd WORD");
	expect_refused("peek:0x4BB:4\t-\tEXACT\t-\tx\n", TD_CASE_ERR_UNSAFE_FIELD,
	               1, "an odd LONG");
	expect_refused("peek:-2:4\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "a negative address");

	printf("the width: 1, 2 or 4\n");
	expect_refused("peek:0x442:3\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "width 3");
	expect_refused("peek:0x442:0\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "width 0");
	expect_refused("peek:0x442:8\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "width 8");
	expect_refused("peek:0x442\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "no width");
	expect_refused("peek::2\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "no address");
	expect_refused("peek:0x442:\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "an empty width");

	printf("read-only: no argument, no capture, no write form\n");
	expect_refused("peek:0x4BA:4\tl:1\tEXACT\t-\tx\n", TD_CASE_ERR_SIGNATURE,
	               1, "a value to write");
	expect_refused("peek:0x4BA:4\tb:0\tEXACT\t-\tx\n", TD_CASE_ERR_SIGNATURE,
	               1, "a buffer");
	expect_refused("peek:0x4BA:4\t-\tEXACT\t0:4\tx\n", TD_CASE_ERR_CAPTURE,
	               1, "a capture");
	expect_refused("poke:0x4BA:4\tl:1\tEXACT\t-\tx\n",
	               TD_CASE_ERR_UNSAFE_OPCODE, 1, "poke");
	expect_refused("write:0x452:2\tw:0\tEXACT\t-\tx\n",
	               TD_CASE_ERR_UNSAFE_OPCODE, 1, "write");

	printf("the classes: EXACT IGNORE NONZERO; PTR_RAM, MONOTONE at "
	       "width 4; no width suffix\n");
	expect_refused("peek:0x442:2\t-\tPTR_RAM\t-\tx\n", TD_CASE_ERR_RETCLASS,
	               1, "PTR_RAM on a WORD");
	expect_refused("peek:0x484:1\t-\tMONOTONE\t-\tx\n", TD_CASE_ERR_RETCLASS,
	               1, "MONOTONE on a BYTE");
	expect_refused("peek:0x4BA:4\t-\tHANDLE\t-\tx\n", TD_CASE_ERR_RETCLASS,
	               1, "HANDLE");
	expect_refused("peek:0x4BA:4\t-\tABSENT\t-\tx\n", TD_CASE_ERR_RETCLASS,
	               1, "ABSENT");
	expect_refused("peek:0x442:2\t-\tEXACT.W\t-\tx\n", TD_CASE_ERR_RETCLASS,
	               1, "a width suffix");
	expect_refused("13:1\tw:2\tMONOTONE\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "MONOTONE on a trap case");
	expect_refused("jar:_MCH\t-\tMONOTONE\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "MONOTONE on a jar case");

	printf("no prelude, not inside a Super bracket, not after Getmpb\n");
	expect_refused("+peek:0x4BA:4\t-\tEXACT\t-\tx\n"
	               "peek:0x4BA:4\t-\tEXACT\t-\tx\n",
	               TD_CASE_ERR_DIRECTIVE, 1, "a peek prelude");
	expect_refused("!disk\t2\n"
	               "+peek:0x4BA:4\t-\t-\t-\tx\n"
	               "13:4\tw:0,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\tsector 0\n",
	               TD_CASE_ERR_DIRECTIVE, 2, "a peek prelude in a disk table");
	expect_refused("!disk\t2\n"
	               "+poke:0x4BA:4\t-\t-\t-\tx\n"
	               "13:4\tw:0,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\tsector 0\n",
	               TD_CASE_ERR_UNSAFE_OPCODE, 2, "a poke prelude");
	expect_refused("!disk\t2\n"
	               "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t-\tsector 0\n"
	               "peek:0x4BA:4\t-\tEXACT\t-\tx\n",
	               TD_CASE_ERR_DANGLING, 3, "after a prelude");
	expect_refused("1:0x20\tl:0\tNONZERO\t-\tSuper(0L)\n"
	               "peek:0x4BA:4\t-\tEXACT\t-\tx\n"
	               "1:0x20\tt:0\tIGNORE\t-\tSuper(t:0)\n",
	               TD_CASE_ERR_UNSAFE_STATE, 2, "inside a Super bracket");
	expect_refused("1:0x20\tl:0\tNONZERO\t-\tSuper(0L)\n"
	               "13:0\tb:0\tIGNORE\t-\tGetmpb\n"
	               "1:0x20\tt:0\tIGNORE\t-\tSuper(t:0)\n"
	               "peek:0x4BA:4\t-\tEXACT\t-\tx\n",
	               TD_CASE_ERR_UNSAFE_STATE, 4, "after Getmpb");
}

/* ------------------------------------------------------------------ */
/* The runner                                                          */
/* ------------------------------------------------------------------ */

static const char *table_text =
	"# stub table of test_peek.c (BIOS21)\n"                   /* 1 */
	"peek:0x442:2\t-\tEXACT\t-\ta WORD, its neighbour set\n"   /* 2  c0 */
	"peek:0x484:1\t-\tEXACT\t-\ta BYTE\n"                      /* 3  c1 */
	"peek:0x4F2:4\t-\tPTR_RAM\t-\ta pointer\n"                 /* 4  c2 */
	"peek:0x4EE:2\t-\tEXACT\t-\t-1 as a WORD\n"                /* 5  c3 */
	"peek:0x5FC:4\t-\tMONOTONE\t-\tthe stub's frame counter\n" /* 6  c4 */
	"peek:0x4BA:4\t-\tMONOTONE\t-\ta counter that stands\n"    /* 7  c5 */
	"peek:0x45E:4\t-\tNONZERO\t-\tzero: refused\n"             /* 8  c6 */
	"13:1\tw:2\tEXACT\t-\ta trap case among them\n"            /* 9  c7 */
	"peek:0x5A8:4\t-\tEXACT\t-\ta LONG with its top bit\n"     /* 10 c8 */
	"peek:0x420:4\t-\tIGNORE\t-\tnot judged\n";                /* 11 c9 */

#define NCASES 10

static struct td_call_case_table table;
static uint8_t records[NCASES * TD_CALL_RECORD_SIZE];

static const uint8_t *rec(int i)
{
	return records + (size_t)i * TD_CALL_RECORD_SIZE;
}

static uint32_t be32(const uint8_t *p)
{
	return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
	       ((uint32_t)p[2] << 8) | (uint32_t)p[3];
}

static uint16_t be16(const uint8_t *p)
{
	return (uint16_t)((p[0] << 8) | p[1]);
}

#define RET(i)     be32(rec(i) + TD_CALL_OFF_RET)
#define RET_HI(i)  be32(rec(i) + TD_CALL_OFF_RET_HI)
#define ARG(i, k)  be32(rec(i) + TD_CALL_OFF_ARGS + 4 * (k))
#define CAP(i, k)  be32(rec(i) + TD_CALL_OFF_CAPTURE + 4 * (k))

/* A cell of the stub's window, in machine order, as the machine keeps it. */
static void lay(long addr, const void *v, size_t n)
{
	memcpy((void *)td_abi_addr(addr), v, n);
}

static void lay_cells(void)
{
	uint16_t w;
	uint8_t b;
	uint32_t l;

	w = 20;          lay(0x442, &w, 2);
	w = 0x1234;      lay(0x444, &w, 2);    /* the neighbour: not read */
	b = 7;           lay(0x484, &b, 1);
	b = 0x55;        lay(0x485, &b, 1);    /* the neighbour: not read */
	l = 0x00E00030uL; lay(0x4F2, &l, 4);
	w = 0xFFFF;      lay(0x4EE, &w, 2);
	l = 1000;        lay(TD_STUB_VSYNC_CELL, &l, 4);
	l = 0x00012345uL; lay(0x4BA, &l, 4);
	l = 0;           lay(0x45E, &l, 4);
	l = 0x9555AAAAuL; lay(0x5A8, &l, 4);
	l = 0x752019F3uL; lay(0x420, &l, 4);
}

static int run(void)
{
	int rc = td_run_call(&table, records, sizeof records);

	TD_CHECK_EQ(rc, 0);
	return rc;
}

static void test_records(void)
{
	int k, i, supexecs = 0, vsyncs = 0;

	printf("a peek: trap 257, the width, the address in argument 0, "
	       "the other slots poisoned\n");
	TD_CHECK_EQ(be16(rec(0) + TD_CALL_OFF_TRAP), 257);
	TD_CHECK_EQ(be16(rec(0) + TD_CALL_OFF_OPCODE), 2);
	TD_CHECK_EQ(be16(rec(0) + TD_CALL_OFF_CASEID), 2);
	TD_CHECK_EQ(ARG(0, 0), 0x442);
	for (k = 1; k < TD_CALL_ARGS_N; k++)
		TD_CHECK_EQ(ARG(0, k), 0xDEADDEADuL);

	printf("the cell at its own width, zero-extended; the neighbour is "
	       "not read\n");
	TD_CHECK_EQ(RET(0), 20);
	TD_CHECK_EQ(RET_HI(0), 0);
	TD_CHECK_EQ(RET(1), 7);
	TD_CHECK_EQ(be16(rec(1) + TD_CALL_OFF_OPCODE), 1);
	TD_CHECK_EQ(RET(2), 0x00E00030uL);
	TD_CHECK_EQ(RET(3), 0x0000FFFFuL);
	TD_CHECK_EQ(RET_HI(3), 0);
	TD_CHECK_EQ(RET(8), 0x9555AAAAuL);
	TD_CHECK_EQ(RET_HI(8), 0);
	TD_CHECK_EQ(RET(9), 0x752019F3uL);

	printf("a peek writes no capture; MONOTONE its second read at "
	       "capture[0..3]\n");
	TD_CHECK_MEM(rec(0) + TD_CALL_OFF_CAPTURE, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_MEM(rec(2) + TD_CALL_OFF_CAPTURE, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_EQ(RET(4), 1000);
	TD_CHECK_EQ(CAP(4, 0), 1000 + TD_PEEK_MONO_VSYNCS);
	TD_CHECK_MEM(rec(4) + TD_CALL_OFF_CAPTURE + 4, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_EQ(RET(5), 0x00012345uL);
	TD_CHECK_EQ(CAP(5, 0), 0x00012345uL);   /* stands: tosdiff's to judge */

	printf("every peek is a Supexec; MONOTONE two, with the Vsyncs "
	       "between\n");
	for (i = 0; i < td_stub_ncalls && i < TD_STUB_MAX_CALLS; i++) {
		if (td_stub_calls[i].call_trap == 14 &&
		    td_stub_calls[i].opcode == 0x26)
			supexecs++;
		if (td_stub_calls[i].call_trap == 14 &&
		    td_stub_calls[i].opcode == 0x25)
			vsyncs++;
	}
	TD_CHECK_EQ(supexecs, 7 + 2 * 2);
	TD_CHECK_EQ(vsyncs, 2 * TD_PEEK_MONO_VSYNCS);
	TD_CHECK_EQ(td_stub_vsyncs, 2 * TD_PEEK_MONO_VSYNCS);
	TD_CHECK_EQ(td_stub_ncalls, 7 + 2 * (2 + TD_PEEK_MONO_VSYNCS) + 1);
	/* the MONOTONE case c4: Supexec, Vsync, Vsync, Supexec */
	TD_CHECK_EQ(td_stub_calls[4].opcode, 0x26);
	TD_CHECK_EQ(td_stub_calls[5].opcode, 0x25);
	TD_CHECK_EQ(td_stub_calls[6].opcode, 0x25);
	TD_CHECK_EQ(td_stub_calls[7].opcode, 0x26);

	printf("a trap case between peek cases is issued as before\n");
	TD_CHECK_EQ(be16(rec(7) + TD_CALL_OFF_TRAP), 13);
}

static void test_reread(void)
{
	static uint8_t keep[NCASES * TD_CALL_RECORD_SIZE];
	uint16_t w = 21;

	printf("a peek reads the cell afresh in every run\n");
	memcpy(keep, records, sizeof keep);
	td_stub_reset();
	lay_cells();
	lay(0x442, &w, 2);
	run();
	TD_CHECK_EQ(RET(0), 21);
	memcpy(records, keep, sizeof keep);
}

int main(int argc, char **argv)
{
	struct td_case_error err;
	int rc;
	FILE *f;

	if (argc != 3) {
		fprintf(stderr, "usage: test_peek OUT.BIN OUT.TSV\n");
		return 2;
	}
	test_form();
	test_refusals();

	memset(&err, 0, sizeof err);
	rc = td_call_cases_parse(&table, TD_AREA_BIOS, table_text,
	                         strlen(table_text), &err);
	TD_CHECK_EQ(rc, 0);
	if (rc != 0) {
		printf("    line %u: %s\n", err.line, err.what ? err.what : "");
		return TD_TEST_RESULT();
	}
	TD_CHECK_EQ(table.count, NCASES);
	td_stub_reset();
	lay_cells();
	run();
	test_records();
	test_reread();

	/* the stub dump, for test_tosdiff.sh */
	rc = td_dump_write(argv[1], TD_AREA_BIOS, table.count, "test",
	                   table.crc, records, sizeof records);
	TD_CHECK_EQ(rc, 0);
	f = fopen(argv[2], "wb");
	TD_CHECK(f != NULL);
	if (f) {
		TD_CHECK_EQ(fwrite(table_text, 1, strlen(table_text), f),
		            strlen(table_text));
		fclose(f);
	}
	return TD_TEST_RESULT();
}
