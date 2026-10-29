/*
 * test_jar.c -- jar cases (TASK/TOOL18): the loader's form and its
 * refusals, and the call runner against the stub's cookie jar.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "The cookie jar"; the
 * record: core/tosdump.h, "A JAR CASE".  The stub's jar: stub_abi.h, "The
 * cookie jar".  The expected values are written out here, not read from
 * core/: a parser that agreed with itself would prove nothing.
 *
 * Writes the stub dump and its table to argv[1] / argv[2] for the
 * comparator tests (test_tosdiff.sh, "jar cases").
 */
#include "tdtest.h"
#include "cases.h"
#include "runner_call.h"
#include "dump.h"
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

	printf("jar:<TAG> and jar:*: trap 256, the tag as LONG argument 0\n");
	rc = parse("# jar\n"
	           "jar:_MCH\t-\tEXACT\t-\t_MCH\n"
	           "jar:*\t-\tEXACT\t-\tthe whole jar\n"
	           "jar:XHDI\t-\tNONZERO\t-\tXHDI\n"
	           "jar:_FRB\t-\tPTR_RAM\t-\t_FRB\n"
	           "jar:NTOS\t-\tABSENT\t-\tNTOS\n"
	           "jar:__NF\t-\tEXACT\t-\ttwo underscores\n", &err);
	TD_CHECK_EQ(rc, 0);
	TD_CHECK_EQ(t.count, 6);
	TD_CHECK_EQ(t.cases[0].line, 2);
	TD_CHECK_EQ(t.cases[0].trap, 256);
	TD_CHECK_EQ(t.cases[0].opcode, 0);
	TD_CHECK_EQ(t.cases[0].nargs, 1);
	TD_CHECK_EQ(t.cases[0].arg[0].kind, TD_CALL_ARG_LONG);
	TD_CHECK_EQ((uint32_t)t.cases[0].arg[0].value, 0x5F4D4348uL);
	TD_CHECK_EQ(t.cases[0].retclass, TD_RET_EXACT);
	TD_CHECK_EQ(t.cases[0].retwidth, TD_RET_WIDTH_LONG);
	TD_CHECK_EQ(t.cases[0].cap_arg, -1);
	TD_CHECK_EQ(t.cases[0].cap_layout, 0);
	TD_CHECK_EQ(t.cases[0].prelude_count, 0);
	TD_CHECK(strcmp(t.cases[0].comment, "_MCH") == 0);
	TD_CHECK_EQ(t.cases[1].trap, 256);
	TD_CHECK_EQ(t.cases[1].opcode, 1);
	TD_CHECK_EQ(t.cases[1].nargs, 0);
	TD_CHECK_EQ(t.cases[2].retclass, TD_RET_NONZERO);
	TD_CHECK_EQ(t.cases[3].retclass, TD_RET_PTR_RAM);
	TD_CHECK_EQ(t.cases[4].retclass, TD_RET_ABSENT);
	TD_CHECK_EQ((uint32_t)t.cases[5].arg[0].value, 0x5F5F4E46uL);
	TD_CHECK(strcmp(td_retclass_name(TD_RET_ABSENT), "ABSENT") == 0);

	printf("jar cases mix with trap cases, which keep their guard\n");
	rc = parse("13:1\tw:2\tEXACT\t-\tBconstat(2)\n"
	           "jar:_MCH\t-\tEXACT\t-\t_MCH\n"
	           "14:0x04\t-\tEXACT.W\t-\tGetrez\n", &err);
	TD_CHECK_EQ(rc, 0);
	TD_CHECK_EQ(t.count, 3);
	TD_CHECK_EQ(t.cases[1].line, 2);
	TD_CHECK_EQ(t.cases[2].trap, 14);
}

static void test_refusals(void)
{
	printf("the tag: four characters 0x21..0x7E, or *\n");
	expect_refused("jar:_MC\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "three characters");
	expect_refused("jar:_MCHX\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "five characters");
	expect_refused("jar:_M H\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "a blank inside the tag");
	expect_refused("jar:\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "no tag");
	expect_refused("jar:**\t-\tEXACT\t-\tx\n", TD_CASE_ERR_VALUE, 1,
	               "two stars");

	printf("args and capture are -\n");
	expect_refused("jar:_MCH\tl:1\tEXACT\t-\tx\n", TD_CASE_ERR_SIGNATURE, 1,
	               "an argument");
	expect_refused("jar:_MCH\t-\tEXACT\t0:4\tx\n", TD_CASE_ERR_CAPTURE, 1,
	               "a capture");
	expect_refused("jar:*\t-\tEXACT\tr:4\tx\n", TD_CASE_ERR_CAPTURE, 1,
	               "a capture on jar:*");

	printf("the classes: EXACT NONZERO PTR_RAM ABSENT; jar:* EXACT only\n");
	expect_refused("jar:_MCH\t-\tHANDLE\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "HANDLE");
	expect_refused("jar:_MCH\t-\tIGNORE\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "IGNORE");
	expect_refused("jar:_MCH\t-\tEXACT.W\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "a width");
	expect_refused("jar:*\t-\tABSENT\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "jar:* ABSENT");
	expect_refused("jar:*\t-\tNONZERO\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "jar:* NONZERO");
	expect_refused("13:1\tw:2\tABSENT\t-\tx\n", TD_CASE_ERR_RETCLASS, 1,
	               "ABSENT on a trap case");

	printf("no prelude, not inside a Super bracket, not after Getmpb\n");
	expect_refused("+jar:_MCH\t-\tEXACT\t-\tx\njar:_MCH\t-\tEXACT\t-\tx\n",
	               TD_CASE_ERR_DIRECTIVE, 1, "a jar prelude");
	expect_refused("!disk\t2\n"
	               "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t-\tsector 0\n"
	               "jar:_MCH\t-\tEXACT\t-\tx\n",
	               TD_CASE_ERR_DANGLING, 3, "after a prelude");
	expect_refused("1:0x20\tl:0\tNONZERO\t-\tSuper(0L)\n"
	               "jar:_MCH\t-\tEXACT\t-\tx\n"
	               "1:0x20\tt:0\tIGNORE\t-\tSuper(t:0)\n",
	               TD_CASE_ERR_UNSAFE_STATE, 2, "inside a Super bracket");
	expect_refused("1:0x20\tl:0\tNONZERO\t-\tSuper(0L)\n"
	               "13:0\tb:0\tIGNORE\t-\tGetmpb\n"
	               "1:0x20\tt:0\tIGNORE\t-\tSuper(t:0)\n"
	               "jar:_MCH\t-\tEXACT\t-\tx\n",
	               TD_CASE_ERR_UNSAFE_STATE, 4, "after Getmpb");
}

/* ------------------------------------------------------------------ */
/* The runner                                                          */
/* ------------------------------------------------------------------ */

static const char *table_text =
	"# stub table of test_jar.c (TOOL18)\n"                    /* 1 */
	"jar:*\t-\tEXACT\t-\tthe whole jar\n"                       /* 2  c0 */
	"jar:_CPU\t-\tEXACT\t-\t_CPU\n"                             /* 3  c1 */
	"jar:_MCH\t-\tEXACT\t-\t_MCH\n"                             /* 4  c2 */
	"jar:XHDI\t-\tPTR_RAM\t-\tXHDI, a pointer\n"                /* 5  c3 */
	"jar:_FRB\t-\tPTR_RAM\t-\t_FRB, odd: refused\n"             /* 6  c4 */
	"jar:_FLK\t-\tNONZERO\t-\t_FLK, zero: refused\n"            /* 7  c5 */
	"jar:NTOS\t-\tABSENT\t-\tNTOS, absent\n"                    /* 8  c6 */
	"jar:_SWI\t-\tABSENT\t-\t_SWI, present: refused\n"          /* 9  c7 */
	"jar:EdDI\t-\tNONZERO\t-\tEdDI, absent: refused\n"          /* 10 c8 */
	"13:1\tw:2\tEXACT\t-\ta trap case among them\n"             /* 11 c9 */
	"jar:_MCH\t-\tEXACT\t-\t_MCH again\n";                      /* 12 c10 */

#define NCASES 11

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
#define TAG(i, k)  be32(rec(i) + TD_CALL_OFF_CAPTURE + 4 * (k))

static int run(void)
{
	int rc = td_run_call(&table, records, sizeof records);

	TD_CHECK_EQ(rc, 0);
	return rc;
}

static void test_records(void)
{
	int k;

	printf("jar:*: the count, the tags in jar order, poison after them\n");
	TD_CHECK_EQ(be16(rec(0) + TD_CALL_OFF_TRAP), 256);
	TD_CHECK_EQ(be16(rec(0) + TD_CALL_OFF_OPCODE), 1);
	TD_CHECK_EQ(be16(rec(0) + TD_CALL_OFF_CASEID), 2);
	TD_CHECK_EQ(RET(0), 6);
	TD_CHECK_EQ(RET_HI(0), 0);
	TD_CHECK_MEM(rec(0) + TD_CALL_OFF_CAPTURE,
	             "_CPU_MCH_SWIXHDI_FRB_FLK", 24);
	TD_CHECK_MEM(rec(0) + TD_CALL_OFF_CAPTURE + 24, "\xDE\xAD\xDE\xAD", 4);
	for (k = 0; k < TD_CALL_ARGS_N; k++)
		TD_CHECK_EQ(ARG(0, k), 0xDEADDEADuL);

	printf("jar:<TAG>: the tag in argument 0, the value zero-extended\n");
	TD_CHECK_EQ(be16(rec(1) + TD_CALL_OFF_TRAP), 256);
	TD_CHECK_EQ(be16(rec(1) + TD_CALL_OFF_OPCODE), 0);
	TD_CHECK_EQ(ARG(1, 0), 0x5F435055uL);
	TD_CHECK_EQ(ARG(1, 1), 0xDEADDEADuL);
	TD_CHECK_EQ(RET(1), 30);
	TD_CHECK_EQ(RET_HI(1), 0);
	TD_CHECK_MEM(rec(1) + TD_CALL_OFF_CAPTURE, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_EQ(RET(2), 0x00020000uL);
	TD_CHECK_EQ(RET(3), TD_STUB_JAR_XHDI);
	TD_CHECK_EQ(RET_HI(3), 0);
	TD_CHECK_EQ(RET(5), 0);                    /* _FLK present, zero */
	TD_CHECK_EQ(RET_HI(5), 0);

	printf("an absent tag: ret_hi:ret all ones\n");
	TD_CHECK_EQ(RET(6), 0xFFFFFFFFuL);
	TD_CHECK_EQ(RET_HI(6), 0xFFFFFFFFuL);
	TD_CHECK_EQ(ARG(6, 0), 0x4E544F53uL);      /* NTOS */

	printf("a trap case between jar cases is issued as before\n");
	TD_CHECK_EQ(be16(rec(9) + TD_CALL_OFF_TRAP), 13);
	TD_CHECK_EQ(td_stub_ncalls, 1);
	TD_CHECK_EQ(td_stub_jar_reads, 10);        /* one read per jar case */
	TD_CHECK_EQ(RET(10), 0x00020000uL);
}

static void test_value_ffffffff(void)
{
	static uint8_t keep[NCASES * TD_CALL_RECORD_SIZE];

	printf("a present 0xFFFFFFFF is not absent: the high half is 0\n");
	memcpy(keep, records, sizeof keep);
	td_stub_reset();
	td_stub_jar.e[1].value = 0xFFFFFFFFuL;     /* _MCH = MCH_NOHARD */
	run();
	TD_CHECK_EQ(RET(2), 0xFFFFFFFFuL);
	TD_CHECK_EQ(RET_HI(2), 0);
	memcpy(records, keep, sizeof keep);
}

static void test_no_jar(void)
{
	static uint8_t keep[NCASES * TD_CALL_RECORD_SIZE];

	printf("_p_cookies 0: every tag absent, jar:* absent, no tag\n");
	memcpy(keep, records, sizeof keep);
	td_stub_reset();
	td_stub_jar.jar_addr = 0;
	run();
	TD_CHECK_EQ(RET(0), 0xFFFFFFFFuL);
	TD_CHECK_EQ(RET_HI(0), 0xFFFFFFFFuL);
	TD_CHECK_MEM(rec(0) + TD_CALL_OFF_CAPTURE, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_EQ(RET_HI(1), 0xFFFFFFFFuL);
	TD_CHECK_EQ(RET_HI(2), 0xFFFFFFFFuL);
	memcpy(records, keep, sizeof keep);
}

static void test_long_jar(void)
{
	static uint8_t keep[NCASES * TD_CALL_RECORD_SIZE];
	uint16_t i;

	printf("a jar of 70 entries: count 70, the first 64 tags, a tag "
	       "past them still found\n");
	memcpy(keep, records, sizeof keep);
	td_stub_reset();
	for (i = 0; i < 70; i++) {
		td_stub_jar.e[i].tag   = 0x54300000uL + i;   /* "T0.." */
		td_stub_jar.e[i].value = i;
	}
	td_stub_jar.e[69].tag   = 0x5F4D4348uL;          /* _MCH last */
	td_stub_jar.e[69].value = 0x00030000uL;
	td_stub_jar.count = 70;
	run();
	TD_CHECK_EQ(RET(0), 70);
	TD_CHECK_EQ(TAG(0, 0), 0x54300000uL);
	TD_CHECK_EQ(TAG(0, 63), 0x5430003FuL);
	TD_CHECK_EQ(RET(2), 0x00030000uL);
	memcpy(records, keep, sizeof keep);
}

static void test_first_entry(void)
{
	static uint8_t keep[NCASES * TD_CALL_RECORD_SIZE];

	printf("a tag twice: the first entry, as Getcookie finds it\n");
	memcpy(keep, records, sizeof keep);
	td_stub_reset();
	td_stub_jar.e[0].tag = 0x5F4D4348uL;               /* _MCH first */
	td_stub_jar.e[0].value = 0x00010000uL;
	run();
	TD_CHECK_EQ(RET(2), 0x00010000uL);
	TD_CHECK_EQ(TAG(0, 0), 0x5F4D4348uL);
	TD_CHECK_EQ(TAG(0, 1), 0x5F4D4348uL);
	memcpy(records, keep, sizeof keep);
}

int main(int argc, char **argv)
{
	struct td_case_error err;
	int rc;
	FILE *f;

	if (argc != 3) {
		fprintf(stderr, "usage: test_jar OUT.BIN OUT.TSV\n");
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
	run();
	test_records();
	test_value_ffffffff();
	test_no_jar();
	test_long_jar();
	test_first_entry();

	/* the default-jar dump, for test_tosdiff.sh */
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
