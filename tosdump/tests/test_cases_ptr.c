/*
 * test_cases_ptr.c -- structures behind a pointer (TASK/TOOL16): the
 * "!layout" directive and its C-layout check, the layout capture
 * ("r:<layout>", "<n>:<layout>"), the write permission "!write return" and
 * the "+put" prelude, and the Getmpb guard.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "Structures behind a
 * pointer".  Every expectation is written out here, not read from core/.
 */
#include "tdtest.h"
#include "cases.h"

static struct td_call_case_table t;

/* The four layouts of docs/api/structures/iorec.md and md.md. */
#define L_IOREC  "!layout\tiorec\t14/24\tibuf:p@0/0,ibufsiz:w@4/8," \
                 "ibufhd:w@6/10,ibuftl:w@8/12,ibuflow:w@10/14,ibufhi:w@12/16\n"
#define L_EXT    "!layout\textiorec\t34/56\tin:[iorec]@0/0,out:[iorec]@14/24," \
                 "baudrate:b@28/48,flowctrl:b@29/49,ucr:b@30/50," \
                 "datamask:b@31/51,wr5:b@32/52:IGNORE\n"
#define L_MD     "!layout\tmd\t16/32\tmd_next:p@0/0:EXACT," \
                 "md_start:l@4/8:PTR_RAM,md_length:l@8/16:NONZERO," \
                 "md_owner:p@12/24:EXACT\n"
#define L_MPB    "!layout\tmpb\t12/24\tmp_free:p>md@0/0,mp_used:p@4/8:EXACT," \
                 "mp_rover:p@8/16:EXACT\n"
#define IOREC1   "14:0x0E\tw:1\tPTR_RAM\tr:iorec\tIorec(1)\n"
#define BCONSTAT "13:1\tw:2\tEXACT\t-\tBconstat(2)\n"
#define SUPER0   "1:0x20\tl:0\tNONZERO\t-\tSuper(0L)\n"
#define SUPERT   "1:0x20\tt:0\tIGNORE\t-\tSuper(token)\n"
#define GETMPB   "13:0\tb:0\tIGNORE\t0:mpb\tGetmpb\n"

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

static const struct td_layout_slot *slot(const struct td_layout *l,
                                         const char *name)
{
	int k = td_layout_slot_find(l, name);

	if (k < 0) {
		printf("  FAIL layout %s has no field %s\n", l->name, name);
		td_test_failures++;
		return &l->slot[0];
	}
	return &l->slot[k];
}

static void test_layouts(void)
{
	const struct td_layout *l;
	const struct td_layout_slot *s;

	printf("!layout: the four spec layouts, flattened into slots\n");
	expect_ok(L_IOREC L_EXT L_MD L_MPB IOREC1, 1, "the spec layouts");
	TD_CHECK_EQ(t.nlayouts, 4);

	l = &t.layouts[0];
	TD_CHECK(strcmp(l->name, "iorec") == 0);
	TD_CHECK_EQ(l->size[TD_LAYOUT_ABI_M68K], 14);
	TD_CHECK_EQ(l->size[TD_LAYOUT_ABI_LP64], 24);
	TD_CHECK_EQ(l->nslots, 6);
	s = slot(l, "ibuf");
	TD_CHECK_EQ(s->type, TD_LT_PTR);
	TD_CHECK_EQ(s->cls, TD_RET_PTR_RAM);          /* p: PTR_RAM by default */
	s = slot(l, "ibufhd");
	TD_CHECK_EQ(s->cls, TD_RET_EXACT);            /* the rest: EXACT */
	TD_CHECK_EQ(s->off[TD_LAYOUT_ABI_M68K], 6);
	TD_CHECK_EQ(s->off[TD_LAYOUT_ABI_LP64], 10);

	/* the embedded IOREC: its fields under "in." / "out.", offsets added */
	l = &t.layouts[1];
	TD_CHECK_EQ(l->nslots, 17);
	s = slot(l, "out.ibuftl");
	TD_CHECK_EQ(s->parent, -1);
	TD_CHECK_EQ(s->off[TD_LAYOUT_ABI_M68K], 14 + 8);
	TD_CHECK_EQ(s->off[TD_LAYOUT_ABI_LP64], 24 + 12);
	s = slot(l, "wr5");
	TD_CHECK_EQ(s->cls, TD_RET_IGNORE);
	TD_CHECK_EQ(s->type, TD_LT_BYTE);

	/* the followed pointer: the MD's fields hang from mp_free */
	l = &t.layouts[3];
	TD_CHECK_EQ(l->nslots, 3 + 4);
	TD_CHECK_EQ(td_layout_slot_find(l, "mp_free"), 0);
	TD_CHECK_EQ(l->slot[0].follow, 2);
	s = slot(l, "mp_free->md_length");
	TD_CHECK_EQ(s->parent, 0);
	TD_CHECK_EQ(s->hops, 1);
	TD_CHECK_EQ(s->type, TD_LT_CLONG);
	TD_CHECK_EQ(s->cls, TD_RET_NONZERO);
	TD_CHECK_EQ(s->off[TD_LAYOUT_ABI_M68K], 8);
	TD_CHECK_EQ(s->off[TD_LAYOUT_ABI_LP64], 16);
	/* order: the pointer, then what it points at, then the next field */
	TD_CHECK_EQ(td_layout_slot_find(l, "mp_free->md_next"), 1);
	TD_CHECK_EQ(td_layout_slot_find(l, "mp_used"), 5);

	TD_CHECK_EQ(td_layout_width(TD_LT_CLONG, TD_LAYOUT_ABI_M68K), 4);
	TD_CHECK_EQ(td_layout_width(TD_LT_CLONG, TD_LAYOUT_ABI_LP64), 8);
	TD_CHECK_EQ(td_layout_width(TD_LT_LONG, TD_LAYOUT_ABI_LP64), 4);
	TD_CHECK_EQ(td_layout_native_abi(),
	            sizeof(void *) == 8 ? TD_LAYOUT_ABI_LP64 : TD_LAYOUT_ABI_M68K);
}

static void test_layout_refusals(void)
{
	printf("!layout: the numbers are C's, on both ABIs\n");
	/* an LP64 pointer at 4: C puts it at 8 */
	expect_rc("!layout\tx\t6/16\ta:w@0/0,b:p@2/4\n" IOREC1,
	          TD_CASE_ERR_GEOMETRY, 1, "LP64 offset not natural");
	/* m68k: a WORD after a byte is WORD aligned */
	expect_rc("!layout\tx\t4/4\ta:b@0/0,b:w@1/2\n" IOREC1,
	          TD_CASE_ERR_GEOMETRY, 1, "m68k offset not WORD aligned");
	/* a gap is not C */
	expect_rc("!layout\tx\t6/6\ta:w@0/0,b:w@4/4\n" IOREC1,
	          TD_CASE_ERR_GEOMETRY, 1, "a gap");
	/* the size: the end rounded to the alignment (IOREC LP64 is 24, not 18) */
	expect_rc("!layout\tiorec\t14/18\tibuf:p@0/0,ibufsiz:w@4/8,"
	          "ibufhd:w@6/10,ibuftl:w@8/12,ibuflow:w@10/14,ibufhi:w@12/16\n"
	          IOREC1, TD_CASE_ERR_GEOMETRY, 1, "LP64 size not rounded");
	expect_rc("!layout\tx\t3/3\ta:w@0/0,b:b@2/2\n" IOREC1,
	          TD_CASE_ERR_GEOMETRY, 1, "m68k size not rounded to 2");
	expect_rc("!layout\tx\t2/2\ta:q@0/0\n" IOREC1, TD_CASE_ERR_VALUE, 1,
	          "an unknown type");
	expect_rc("!layout\tx\t4/8\ta:p>y@0/0\n" IOREC1, TD_CASE_ERR_VALUE, 1,
	          "a pointer to a layout not defined before");
	expect_rc("!layout\tx\t4/8\ta:p>x@0/0\n" IOREC1, TD_CASE_ERR_VALUE, 1,
	          "a pointer to itself: no loops");
	expect_rc("!layout\tx\t2/2\ta:w@0/0,a:w@2/2\n" IOREC1, TD_CASE_ERR_VALUE, 1,
	          "a field twice");
	expect_rc(L_IOREC "!layout\tx\t14/24\ta:[iorec]@0/0:EXACT\n" IOREC1,
	          TD_CASE_ERR_RETCLASS, 2, "an embedded layout takes no class");
	expect_rc("!layout\tx\t2/2\ta:w@0/0:SOMETIMES\n" IOREC1,
	          TD_CASE_ERR_RETCLASS, 1, "an unknown class");
	expect_rc(L_IOREC L_IOREC IOREC1, TD_CASE_ERR_DIRECTIVE, 2,
	          "a layout name twice");
	expect_rc("!layout\tX\t2/2\ta:w@0/0\n" IOREC1, TD_CASE_ERR_DIRECTIVE, 1,
	          "a layout name is lower case");
	expect_rc(IOREC1 L_IOREC, TD_CASE_ERR_CAPTURE, 1,
	          "a layout must stand before the case that names it");
	expect_rc(L_IOREC BCONSTAT L_MD, TD_CASE_ERR_DIRECTIVE, 3,
	          "!layout after the first case");
	/* 33 slots: the record has 32 */
	expect_rc("!layout\tw8\t16/16\ta:w@0/0,b:w@2/2,c:w@4/4,d:w@6/6,e:w@8/8,"
	          "f:w@10/10,g:w@12/12,h:w@14/14\n"
	          "!layout\tw32\t64/64\ta:[w8]@0/0,b:[w8]@16/16,c:[w8]@32/32,"
	          "d:[w8]@48/48\n"
	          "!layout\tw33\t66/66\ta:[w32]@0/0,b:w@64/64\n" IOREC1,
	          TD_CASE_ERR_TOO_MANY_VALUES, 3, "more fields than 32 slots");
	/* three hops: the bound is two */
	expect_rc("!layout\ta1\t4/8\tx:p@0/0\n"
	          "!layout\ta2\t4/8\tx:p>a1@0/0\n"
	          "!layout\ta3\t4/8\tx:p>a2@0/0\n"
	          "!layout\ta4\t4/8\tx:p>a3@0/0\n" IOREC1,
	          TD_CASE_ERR_RANGE, 4, "three pointer hops");
}

static void test_captures(void)
{
	printf("capture: r:<layout> and <n>:<layout>\n");
	expect_ok(L_IOREC IOREC1, 1, "r:iorec");
	TD_CHECK_EQ(t.cases[0].cap_arg, TD_CALL_CAP_RET);
	TD_CHECK_EQ(t.cases[0].cap_layout, 1);
	TD_CHECK_EQ(t.cases[0].cap_len, 6 * 8);
	expect_rc(L_IOREC "14:0x0E\tw:1\tEXACT\tr:iorec\tIorec\n",
	          TD_CASE_ERR_CAPTURE, 2, "r:<layout> needs PTR_RAM");
	expect_rc(L_IOREC "14:0x0E\tw:1\tPTR_RAM\tr:nosuch\tIorec\n",
	          TD_CASE_ERR_CAPTURE, 2, "an unknown layout");
	expect_rc(L_MD L_MPB "14:0x0E\tw:1\tPTR_RAM\t0:mpb\tIorec\n",
	          TD_CASE_ERR_CAPTURE, 3, "<n>:<layout> needs a b: argument");
	/* the old forms are unchanged */
	expect_ok(L_IOREC "14:0x0E\tw:1\tPTR_RAM\t-\tIorec\n", 1, "no capture");
	TD_CHECK_EQ(t.cases[0].cap_layout, 0);
}

static void test_put(void)
{
	printf("+put: only with !write return, right after its r:<layout> case\n");
	expect_ok(L_IOREC "!write\treturn\n" IOREC1
	          "+put\tibufhd=ibuftl\tthe flush\n" BCONSTAT IOREC1, 3,
	          "the flush");
	TD_CHECK_EQ(t.writes, 1);
	TD_CHECK_EQ(t.prelude_count, 1);
	TD_CHECK_EQ(t.preludes[0].trap, TD_CALL_TRAP_PUT);
	TD_CHECK_EQ(t.preludes[0].cap_layout, 1);
	TD_CHECK_EQ(t.preludes[0].arg[0].value, 2);   /* ibufhd */
	TD_CHECK_EQ(t.preludes[0].arg[1].value, 3);   /* ibuftl */
	TD_CHECK_EQ(t.cases[1].prelude_count, 1);     /* it is Bconstat's */
	TD_CHECK_EQ(t.cases[1].prelude_first, 0);

	/* the write permission */
	expect_rc(L_IOREC IOREC1 "+put\tibufhd=ibuftl\n" BCONSTAT,
	          TD_CASE_ERR_UNSAFE_STATE, 3, "+put without !write return");
	expect_rc(L_IOREC "!write\tall\n" IOREC1, TD_CASE_ERR_DIRECTIVE, 2,
	          "!write takes return only");
	expect_rc(L_IOREC "!write\treturn\n!write\treturn\n" IOREC1,
	          TD_CASE_ERR_DIRECTIVE, 3, "!write twice");
	expect_rc(L_IOREC IOREC1 "!write\treturn\n" BCONSTAT,
	          TD_CASE_ERR_DIRECTIVE, 3, "!write after the first case");
	/* where it stands */
	expect_rc(L_IOREC "!write\treturn\n" BCONSTAT "+put\tibufhd=ibuftl\n"
	          BCONSTAT, TD_CASE_ERR_UNSAFE_FIELD, 4,
	          "+put after a case without r:<layout>");
	expect_rc(L_IOREC "!write\treturn\n+put\tibufhd=ibuftl\n" BCONSTAT,
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "+put before any case");
	expect_rc(L_IOREC "!write\treturn\n" IOREC1 "+put\tibufhd=ibuftl\n"
	          "+put\tibuftl=ibufhd\n" BCONSTAT, TD_CASE_ERR_UNSAFE_FIELD, 5,
	          "a second prelude after +put");
	expect_rc(L_IOREC "!write\treturn\n" IOREC1 "+put\tibufhd=ibuftl\n",
	          TD_CASE_ERR_DANGLING, 4, "+put with no case after it");
	/* which fields */
	expect_rc(L_IOREC "!write\treturn\n" IOREC1 "+put\tibufhd=nosuch\n"
	          BCONSTAT, TD_CASE_ERR_UNSAFE_FIELD, 4, "an unknown field");
	expect_rc(L_IOREC "!write\treturn\n" IOREC1 "+put\tibuf=ibuf\n" BCONSTAT,
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "a pointer field");
	expect_rc(L_IOREC "!write\treturn\n" IOREC1 "+put\tibufhd=ibufhd\n"
	          BCONSTAT, TD_CASE_ERR_UNSAFE_FIELD, 4, "a field onto itself");
	expect_rc(L_IOREC L_EXT "!write\treturn\n"
	          "14:0x0E\tw:0\tPTR_RAM\tr:extiorec\tIorec(0)\n"
	          "+put\tbaudrate=in.ibufhd\n" BCONSTAT, TD_CASE_ERR_UNSAFE_FIELD, 5,
	          "two fields of different types");
	expect_ok(L_IOREC L_EXT "!write\treturn\n"
	          "14:0x0E\tw:0\tPTR_RAM\tr:extiorec\tIorec(0)\n"
	          "+put\tin.ibufhd=in.ibuftl\n" BCONSTAT, 2,
	          "an embedded field is top-level");
	expect_rc(L_MD L_MPB "!write\treturn\n"
	          "14:0x0E\tw:1\tPTR_RAM\tr:mpb\tnot really an MPB\n"
	          "+put\tmp_free->md_length=mp_free->md_start\n" BCONSTAT,
	          TD_CASE_ERR_UNSAFE_FIELD, 5, "a field behind a pointer");
	expect_rc(L_IOREC "!write\treturn\n" IOREC1 "+put\tibufhd\n" BCONSTAT,
	          TD_CASE_ERR_VALUE, 4, "+put without =");
}

static void test_getmpb(void)
{
	printf("Getmpb: a b: buffer, inside the Super bracket, last\n");
	expect_ok(L_MD L_MPB SUPER0 GETMPB SUPERT, 3, "Getmpb in the bracket");
	TD_CHECK_EQ(t.cases[1].cap_arg, 0);
	TD_CHECK_EQ(t.cases[1].cap_layout, 2);
	expect_ok(L_MD L_MPB SUPER0 GETMPB GETMPB SUPERT, 4, "Getmpb twice");
	expect_rc(L_MD L_MPB GETMPB, TD_CASE_ERR_UNSAFE_OPCODE, 3,
	          "Getmpb outside a bracket");
	expect_rc(L_MD L_MPB SUPER0 "13:0\tl:0\tIGNORE\t-\tGetmpb(NULL)\n" SUPERT,
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "Getmpb without a buffer");
	expect_rc(L_MD L_MPB SUPER0 GETMPB BCONSTAT SUPERT,
	          TD_CASE_ERR_UNSAFE_STATE, 5, "a call after Getmpb");
	expect_rc(L_MD L_MPB SUPER0 GETMPB SUPERT BCONSTAT,
	          TD_CASE_ERR_UNSAFE_STATE, 6, "a call after the bracket closes");
	/* the buffer capture through the layout: the MPB in buffer 0 */
	expect_ok(L_MD L_MPB SUPER0 "13:0\tb:0\tIGNORE\t0:mpb\tGetmpb\n" SUPERT,
	          3, "0:mpb");
	TD_CHECK_EQ(t.cases[1].cap_len, 7 * 8);
}

int main(void)
{
	test_layouts();
	test_layout_refusals();
	test_captures();
	test_put();
	test_getmpb();
	return TD_TEST_RESULT();
}
