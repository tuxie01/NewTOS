/*
 * test_cases_gdos.c -- GEMDOS after a media change (TASK/TOOL20): the
 * layout grammar it needs (":packed", the opaque "x<n>", the array
 * "<type><n>"), the string arguments "s:", "c:", "x:", the tokens t:1 and
 * t:2, the DTA capture "d:<layout>", and the guard that admits the GEMDOS
 * calls in a destructive disk table only.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "GEMDOS after a media
 * change".  Every expectation is written out here, not read from core/.
 */
#include "tdtest.h"
#include "cases.h"

static struct td_call_case_table t;

#define DTA_LAYOUT "!layout\tdta\t44/44:packed\td_reserved:x21@0/0," \
	"d_attrib:b@21/21,d_time:w@22/22,d_date:w@24/24,d_length:L@26/26," \
	"d_fname:b14@30/30\n"
#define DESTR  "!disk\t2\tdestructive\n"
#define SETDTA "1:0x1A\tb:1\tIGNORE\t-\tFsetdta\n"
#define PRE_READ0 "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t-\tsector 0\n"
#define WRITE0    "13:4\tw:3,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\twrite back\n"

static int parse(const char *text, struct td_case_error *err)
{
	memset(err, 0, sizeof *err);
	return td_call_cases_parse(&t, TD_AREA_BIOS, text, strlen(text), err);
}

static void expect_ok(const char *text, uint16_t cases, const char *why)
{
	struct td_case_error err;
	int rc = parse(text, &err);

	if (rc != 0 || t.count != cases) {
		printf("  FAIL %s: refused or miscounted (%d, %u cases, line %u: "
		       "%s)\n", why, rc, t.count, err.line, err.what ? err.what : "");
		td_test_failures++;
	}
}

static void expect_rc(const char *text, int want_rc, uint16_t want_line,
                      const char *why)
{
	struct td_case_error err;
	int rc = parse(text, &err);

	if (rc != want_rc) {
		printf("  FAIL %s: gave %d, want %d (line %u: %s)\n", why, rc,
		       want_rc, err.line, err.what ? err.what : "");
		td_test_failures++;
		return;
	}
	if (err.line != want_line) {
		printf("  FAIL %s: refusal names line %u, want %u\n", why, err.line,
		       want_line);
		td_test_failures++;
	}
	if (!err.what || !*err.what) {
		printf("  FAIL %s: refusal carries no reason\n", why);
		td_test_failures++;
	}
}

static void expect_layout(const char *line, int want_rc, const char *why)
{
	struct td_layout set[TD_LAYOUT_MAX];
	uint8_t n = 0;
	const char *reason = NULL;
	int rc = td_layout_parse(set, &n, line, strlen(line), &reason);

	if (rc != want_rc) {
		printf("  FAIL %s: gave %d, want %d (%s)\n", why, rc, want_rc,
		       reason ? reason : "");
		td_test_failures++;
	}
}

static void test_layout_grammar(void)
{
	struct td_layout set[TD_LAYOUT_MAX];
	uint8_t n = 0;
	const char *why = NULL;
	const char *dta = DTA_LAYOUT;
	const struct td_layout *l;
	int rc, k;

	printf("layouts: :packed, opaque x<n>, arrays <type><n> (the DTA)\n");
	rc = td_layout_parse(set, &n, dta, strlen(dta) - 1, &why);
	TD_CHECK_EQ(rc, 0);
	if (rc != 0) {
		printf("    %s\n", why ? why : "");
		return;
	}
	l = &set[0];
	TD_CHECK_EQ(l->size[0], 44);
	TD_CHECK_EQ(l->size[1], 44);
	TD_CHECK_EQ(l->align[1], 1);
	/* d_reserved has no slot: attr, time, date, length, 14 name bytes */
	TD_CHECK_EQ(l->nslots, 18);
	TD_CHECK(strcmp(l->slot[0].name, "d_attrib") == 0);
	TD_CHECK_EQ(l->slot[0].off[1], 21);
	TD_CHECK_EQ(l->slot[3].type, TD_LT_LONG);
	TD_CHECK_EQ(l->slot[3].off[0], 26);
	TD_CHECK_EQ(l->slot[3].off[1], 26);
	for (k = 0; k < 14; k++) {
		char want[16];

		snprintf(want, sizeof want, "d_fname[%d]", k);
		TD_CHECK(strcmp(l->slot[4 + k].name, want) == 0);
		TD_CHECK_EQ(l->slot[4 + k].off[0], 30 + k);
		TD_CHECK_EQ(l->slot[4 + k].type, TD_LT_BYTE);
		TD_CHECK_EQ(l->slot[4 + k].cls, TD_RET_EXACT);
	}
	TD_CHECK_EQ(td_layout_slot_find(l, "d_reserved"), -1);

	/* without :packed the LONG at 26 is not where LP64 C puts it */
	expect_layout("!layout\tdta\t44/44\td_reserved:x21@0/0,d_attrib:b@21/21,"
	              "d_time:w@22/22,d_date:w@24/24,d_length:L@26/26,"
	              "d_fname:b14@30/30", TD_CASE_ERR_GEOMETRY,
	              "the DTA without :packed");
	/* a packed field has one offset on both ABIs */
	expect_layout("!layout\tp\t6/6:packed\ta:w@0/0,b:L@2/4",
	              TD_CASE_ERR_GEOMETRY, "packed, two offsets");
	/* packed: no padding before a LONG */
	expect_layout("!layout\tp\t5/5:packed\ta:b@0/0,b:L@1/1", 0,
	              "packed LONG at 1");
	expect_layout("!layout\tp\t5/5:packet\ta:b@0/0,b:L@1/1",
	              TD_CASE_ERR_DIRECTIVE, "a word other than :packed");
	/* arrays: aligned like their element, n 1..32 */
	expect_layout("!layout\tw\t8/8\ta:w4@0/0", 0, "w4");
	expect_layout("!layout\tw\t3/4\ta:b@0/0,b:w1@1/1", TD_CASE_ERR_GEOMETRY,
	              "a WORD array at an odd offset");
	expect_layout("!layout\tw\t33/33\ta:b33@0/0", TD_CASE_ERR_VALUE,
	              "an array of 33");
	expect_layout("!layout\tw\t32/32\ta:b16@0/0,b:b16@16/16",
	              0, "32 slots");
	expect_layout("!layout\tw\t33/33\ta:b16@0/0,b:b17@16/16",
	              TD_CASE_ERR_TOO_MANY_VALUES, "an array past the 32 slots");
	expect_layout("!layout\tw\t8/16\ta:p2@0/0", TD_CASE_ERR_VALUE,
	              "no pointer arrays");
	/* opaque: no class, a name stands once */
	expect_layout("!layout\to\t4/4\ta:x3@0/0,b:b@3/3", 0, "x3");
	expect_layout("!layout\to\t4/4\ta:x3@0/0:IGNORE,b:b@3/3",
	              TD_CASE_ERR_RETCLASS, "an opaque field with a class");
	expect_layout("!layout\to\t4/4\ta:x3@0/0,a:b@3/3", TD_CASE_ERR_VALUE,
	              "an opaque name twice");
	expect_layout("!layout\to\t4/4\ta:x@0/0", TD_CASE_ERR_VALUE, "x without n");
	expect_layout("!layout\to\t4/4\ta:b2@0/0,a:w@2/2", TD_CASE_ERR_VALUE,
	              "an array name twice");
}

static void test_strings_and_tokens(void)
{
	printf("string arguments s: c: x:, tokens t:1 t:2\n");
	expect_ok(DESTR "1:0x3C\ts:C:\\X.DAT,w:0\tHANDLE\t-\tFcreate\n"
	          "1:0x3E\tt:1\tEXACT\t-\tFclose\n", 2, "s: and t:1");
	TD_CHECK_EQ(t.nstrings, 1);
	TD_CHECK_EQ(t.cases[0].arg[0].kind, TD_CALL_ARG_STR);
	TD_CHECK(strcmp(t.str_pool + t.str_off[0], "C:\\X.DAT") == 0);
	TD_CHECK_EQ(t.cases[0].keep, 1 + TD_CALL_TOKEN_HANDLE);
	TD_CHECK_EQ(t.cases[1].arg[0].kind, TD_CALL_ARG_TOKEN);
	TD_CHECK_EQ(t.cases[1].arg[0].value, TD_CALL_TOKEN_HANDLE);

	expect_ok(DESTR "1:0x4B\tw:0,x:C:\\PXPROBE,c:child,l:0\tEXACT\t-\tPexec\n",
	          1, "x: and c:");
	TD_CHECK_EQ(t.prog_ext, 1);
	TD_CHECK_EQ(t.cases[0].arg[1].kind, TD_CALL_ARG_PROG);
	TD_CHECK_EQ(t.cases[0].arg[2].kind, TD_CALL_ARG_TAIL);
	/* the tail is stored as TOS passes one: its length first */
	TD_CHECK_EQ(t.str_pool[t.str_off[1]], 5);
	TD_CHECK(strcmp(t.str_pool + t.str_off[1] + 1, "child") == 0);

	expect_rc(DESTR "1:0x41\ts:C:\\A B\tEXACT\t-\tFdelete\n",
	          TD_CASE_ERR_VALUE, 2, "a blank in a string");
	expect_rc(DESTR "1:0x41\ts:C:\\0123456789012345678901234567890123456789"
	          "0123456789012345678901\tEXACT\t-\tFdelete\n",
	          TD_CASE_ERR_RANGE, 2, "a string past 63 characters");
	expect_rc(DESTR "1:0x0E\tt:3\tEXACT\t-\tDsetdrv\n", TD_CASE_ERR_RANGE, 2,
	          "t:3");
}

static void test_guard(void)
{
	printf("the guard: destructive disk tables only, the test drive\n");
	/* outside a destructive disk table GEMDOS stays refused */
	expect_rc("1:0x19\t-\tEXACT\t-\tDgetdrv\n", TD_CASE_ERR_UNSAFE_OPCODE, 1,
	          "Dgetdrv in a round-1 table");
	expect_rc("!disk\t2\n1:0x19\t-\tEXACT\t-\tDgetdrv\n",
	          TD_CASE_ERR_UNSAFE_OPCODE, 2, "Dgetdrv in a disk table");
	expect_rc(DESTR "1:0x3D\ts:C:\\X,w:0\tHANDLE\t-\tFopen\n",
	          TD_CASE_ERR_UNSAFE_OPCODE, 2, "Fopen is not TOOL20's");
	expect_ok(DESTR "1:0x19\t-\tEXACT\t-\tDgetdrv\n", 1, "Dgetdrv");
	TD_CHECK_EQ(t.cases[0].keep, 1 + TD_CALL_TOKEN_DRIVE);

	/* paths on the test drive only */
	expect_rc(DESTR "1:0x41\ts:D:\\BIOS.BIN\tEXACT\t-\tFdelete\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Fdelete on D:");
	expect_rc(DESTR "1:0x3B\ts:\\SUB\tEXACT\t-\tDsetpath\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Dsetpath without a drive");
	expect_rc(DESTR "1:0x3C\ts:C:\\X,w:1\tHANDLE\t-\tFcreate\n"
	          "1:0x3E\tt:1\tEXACT\t-\tFclose\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Fcreate with an attribute");
	expect_rc(DESTR "1:0x4B\tw:0,x:D:\\X,c:x,l:0\tEXACT\t-\tPexec\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Pexec from D:");
	expect_rc(DESTR "1:0x4B\tw:3,x:C:\\X,c:x,l:0\tEXACT\t-\tPexec\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Pexec mode 3");
	expect_rc(DESTR "1:0x4B\tw:0,s:C:\\X,c:x,l:0\tEXACT\t-\tPexec\n",
	          TD_CASE_ERR_SIGNATURE, 2, "Pexec with s: for the program");
	expect_rc(DESTR "1:0x36\tb:0,w:4\tEXACT\t0:16.L\tDfree\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Dfree of D:");
	expect_rc(DESTR "1:0x47\tl:0,w:3\tEXACT\t-\tDgetpath\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Dgetpath without a buffer");

	/* the file bracket */
	expect_rc(DESTR "1:0x3C\ts:C:\\X,w:0\tHANDLE\t-\tFcreate\n",
	          TD_CASE_ERR_UNSAFE_STATE, 2, "a file left open");
	expect_rc(DESTR "1:0x3E\tt:1\tEXACT\t-\tFclose\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Fclose with nothing open");
	expect_rc(DESTR "1:0x3C\ts:C:\\X,w:0\tHANDLE\t-\tFcreate\n"
	          "1:0x3C\ts:C:\\Y,w:0\tHANDLE\t-\tFcreate\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "two files open");
	expect_rc(DESTR "1:0x3C\ts:C:\\X,w:0\tHANDLE\t-\tFcreate\n"
	          "1:0x41\ts:C:\\X\tEXACT\t-\tFdelete\n"
	          "1:0x3E\tt:1\tEXACT\t-\tFclose\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "Fdelete with a file open");
	expect_rc(DESTR "1:0x3C\ts:C:\\X,w:0\tHANDLE\t-\tFcreate\n"
	          "1:0x40\tt:1,l:1025,b:0\tEXACT\t-\tFwrite\n"
	          "1:0x3E\tt:1\tEXACT\t-\tFclose\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "Fwrite past the buffer");
	expect_rc(DESTR "1:0x3C\ts:C:\\X,w:0\tHANDLE\t-\tFcreate\n"
	          "1:0x40\tt:2,l:4,b:0\tEXACT\t-\tFwrite\n"
	          "1:0x3E\tt:1\tEXACT\t-\tFclose\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "Fwrite with the drive token");
	expect_ok(DESTR "1:0x3C\ts:C:\\X,w:0\tHANDLE\t-\tFcreate\n"
	          "1:0x40\tt:1,l:512,b:0\tEXACT\t-\tFwrite\n"
	          "1:0x3E\tt:1\tEXACT\t-\tFclose\n", 3, "the file bracket");

	/* the drive bracket */
	expect_rc(DESTR "1:0x0E\tw:2\tEXACT\t-\tDsetdrv\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Dsetdrv without a Dgetdrv");
	expect_rc(DESTR "1:0x19\t-\tEXACT\t-\tDgetdrv\n"
	          "1:0x0E\tw:2\tEXACT\t-\tDsetdrv\n",
	          TD_CASE_ERR_UNSAFE_STATE, 3, "the drive never restored");
	expect_rc(DESTR "1:0x19\t-\tEXACT\t-\tDgetdrv\n"
	          "1:0x0E\tw:3\tEXACT\t-\tDsetdrv\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "Dsetdrv to another drive");
	expect_rc(DESTR "1:0x0E\tt:2\tEXACT\t-\tDsetdrv\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Dsetdrv(t:2) with nothing open");
	expect_ok(DESTR "1:0x19\t-\tEXACT\t-\tDgetdrv\n"
	          "1:0x0E\tw:2\tEXACT\t-\tDsetdrv\n"
	          "1:0x19\t-\tEXACT\t-\tDgetdrv inside\n"
	          "1:0x0E\tt:2\tEXACT\t-\tDsetdrv back\n", 4, "the drive bracket");
	/* a Dgetdrv inside the bracket does not replace the kept drive */
	TD_CHECK_EQ(t.cases[2].keep, 0);

	/* no GEMDOS file call in supervisor state */
	expect_rc(DESTR "1:0x20\tl:0\tNONZERO\t-\tSuper(0L)\n"
	          "1:0x19\t-\tEXACT\t-\tDgetdrv\n"
	          "1:0x20\tt:0\tIGNORE\t-\tSuper(t:0)\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "Dgetdrv inside a Super bracket");
	/* a GEMDOS call is never a prelude */
	expect_rc(DESTR "+1:0x19\t-\t-\t-\tDgetdrv\n13:9\tw:2\tEXACT\t-\tMediach\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "Dgetdrv as a prelude");
}

static void test_dta(void)
{
	printf("the DTA: Fsetdta(b:n), d:<layout>, nothing else names b:n\n");
	expect_ok(DTA_LAYOUT DESTR SETDTA
	          "1:0x4E\ts:C:\\*.*,w:0x10\tEXACT\td:dta\tFsfirst\n"
	          "13:9\tw:2\tEXACT\t-\tMediach\n"
	          "1:0x4F\t-\tEXACT\td:dta\tFsnext\n", 4, "a search");
	TD_CHECK_EQ(t.cases[0].dta_buf, -1);          /* Fsetdta itself */
	TD_CHECK_EQ(t.cases[0].dta_keep, 0);
	TD_CHECK_EQ(t.cases[1].dta_buf, 1);
	TD_CHECK_EQ(t.cases[1].dta_keep, 0);          /* Fsfirst poisons it */
	TD_CHECK_EQ(t.cases[1].cap_arg, TD_CALL_CAP_DTA);
	TD_CHECK_EQ(t.cases[1].cap_len, 18 * TD_LAYOUT_SLOT_BYTES);
	TD_CHECK_EQ(t.cases[2].dta_keep, 1);          /* Mediach does not */
	TD_CHECK_EQ(t.cases[3].dta_keep, 1);          /* nor does Fsnext */

	expect_rc(DTA_LAYOUT DESTR "1:0x4E\ts:C:\\*.*,w:0\tEXACT\t-\tFsfirst\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "Fsfirst with no DTA set");
	expect_rc(DTA_LAYOUT DESTR "1:0x4F\t-\tEXACT\td:dta\tFsnext\n",
	          TD_CASE_ERR_CAPTURE, 3, "d: with no DTA set");
	expect_rc(DTA_LAYOUT DESTR SETDTA
	          "+13:4\tw:2,b:1,w:1,w:0,w:2,l:0\t-\t-\tsector 0 into the DTA\n"
	          "13:9\tw:2\tEXACT\t-\tMediach\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "an Rwabs into the DTA buffer");
	expect_rc(DTA_LAYOUT DESTR SETDTA
	          "1:0x36\tb:1,w:3\tEXACT\t0:16.L\tDfree into the DTA\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "Dfree into the DTA buffer");
	expect_rc(DTA_LAYOUT DESTR "1:0x1A\tl:0\tIGNORE\t-\tFsetdta(NULL)\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "Fsetdta(NULL)");
	/* the arm and the search around it: what the tables do */
	expect_ok(DTA_LAYOUT DESTR SETDTA PRE_READ0 WRITE0
	          "1:0x4E\ts:C:\\*.*,w:0x10\tEXACT\td:dta\tFsfirst\n", 3,
	          "arm, then search");
}

int main(void)
{
	test_layout_grammar();
	test_strings_and_tokens();
	test_guard();
	test_dta();
	return TD_TEST_RESULT();
}
