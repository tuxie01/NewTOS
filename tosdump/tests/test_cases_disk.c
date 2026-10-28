/*
 * test_cases_disk.c -- the disk round of the call tables (TASK/TOOL07): the
 * "!disk" directive, its preludes, the guard for Rwabs / Getbpb / Mediach
 * and the capture through the returned pointer ("r:").
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "The disk round".  Every
 * expectation is written out here, not read from core/.  The round-1 rules
 * (no directive, no prelude, no Rwabs outside a disk table) are asserted by
 * test_cases_call.c and stay as they are.
 */
#include "tdtest.h"
#include "cases.h"

static struct td_call_case_table t;

#define DISK "!disk\t2\n"
/* A read of C: sector 0 into buffer 0 as a prelude, and the matching write. */
#define PRE_READ0  "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t-\tsector 0 into buffer 0\n"
#define WRITE0     "13:4\tw:1,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\twrite it back\n"

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

static void test_directive(void)
{
	printf("the !disk directive\n");
	expect_ok(DISK "13:7\tw:2\tPTR_RAM\t-\tGetbpb C:\n", 1, "!disk 2");
	TD_CHECK_EQ(t.disk, 1);
	TD_CHECK_EQ(t.disk_drive, 2);
	TD_CHECK_EQ(t.cases[0].line, 2);
	expect_ok("# no directive\n13:10\t-\tEXACT\t-\tDrvmap\n", 1, "plain");
	TD_CHECK_EQ(t.disk, 0);

	expect_rc(DISK DISK "13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_DIRECTIVE,
	          2, "twice");
	expect_rc("13:10\t-\tEXACT\t-\tDrvmap\n" DISK, TD_CASE_ERR_DIRECTIVE, 2,
	          "after a case");
	expect_rc("!disk\t1\n13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_DIRECTIVE,
	          1, "a floppy is no test drive");
	expect_rc("!disk\t16\n13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_DIRECTIVE,
	          1, "past P:");
	expect_rc("!disk\tC\n13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_DIRECTIVE,
	          1, "not a number");
	expect_rc("!disk\n13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_DIRECTIVE,
	          1, "no value");
	expect_rc("!reset\tnone\n13:10\t-\tEXACT\t-\tDrvmap\n",
	          TD_CASE_ERR_DIRECTIVE, 1, "an AES directive");
}

static void test_outside_a_disk_table(void)
{
	printf("Rwabs, Getbpb, Mediach and preludes stay refused without !disk\n");
	expect_rc("13:7\tw:2\tPTR_RAM\t-\tGetbpb\n", TD_CASE_ERR_UNSAFE_OPCODE, 1,
	          "Getbpb");
	expect_rc("13:9\tw:2\tEXACT\t-\tMediach\n", TD_CASE_ERR_UNSAFE_OPCODE, 1,
	          "Mediach");
	expect_rc("13:4\tw:0,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\tRwabs read\n",
	          TD_CASE_ERR_UNSAFE_OPCODE, 1, "Rwabs read");
	expect_rc(PRE_READ0 WRITE0, TD_CASE_ERR_DIRECTIVE, 1, "a prelude");
}

static void test_getbpb_mediach(void)
{
	printf("Getbpb and Mediach in a disk table\n");
	expect_ok(DISK "13:7\tw:15\tEXACT\t-\tabsent\n"
	               "13:7\tw:-1\tEXACT\t-\tnegative\n"
	               "13:9\tw:2\tEXACT\t-\tC:\n"
	               "13:9\tw:16\tEXACT\t-\tpast\n", 4, "drives 2.., -1");
	expect_rc(DISK "13:7\tw:0\tEXACT\t-\tA:\n", TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "Getbpb A: waits for a floppy");
	expect_rc(DISK "13:7\tw:1\tEXACT\t-\tB:\n", TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "Getbpb B:");
	expect_rc(DISK "13:9\tw:0\tEXACT\t-\tA:\n", TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "Mediach A:");
	expect_rc(DISK "13:7\tl:2\tEXACT\t-\tGetbpb\n", TD_CASE_ERR_SIGNATURE, 2,
	          "Getbpb takes a WORD");
}

static void test_rwabs_reads(void)
{
	printf("Rwabs reads in a disk table\n");
	expect_ok(DISK
	    "13:4\tw:0,b:0,w:1,w:0,w:2,l:0\tEXACT\t1:256\tsector 0\n"
	    "13:4\tw:0,b:1,w:2,w:-1,w:2,l:130975\tEXACT\t1:16@512\ttwo\n"
	    "13:4\tw:2,b:0,w:0,w:-1,w:2,l:130987\tEXACT\t-\tcount 0\n"
	    "13:4\tw:0x86,b:0,w:1,w:0,w:15,l:0\tEXACT\t-\tabsent, flags\n"
	    "13:4\tw:0,l:0,w:2,w:0,w:0,l:0\tEXACT\t-\tFastCopy A:\n"
	    "13:4\tw:0,l:0,w:0,w:0,w:1,l:0\tEXACT\t-\tFastCopy B: 0\n",
	    6, "reads, absent drive, the NULL arm");
	expect_rc(DISK "13:4\tw:0,b:0,w:3,w:0,w:2,l:0\tEXACT\t-\tthree\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "three sectors overrun a buffer");
	expect_rc(DISK "13:4\tw:0,b:0,w:-1,w:0,w:2,l:0\tEXACT\t-\tnegative\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "a negative count");
	expect_rc(DISK "13:4\tw:8,b:0,w:1,w:0,w:18,l:0\tEXACT\t-\tphysical\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "RW_NOTRANSLATE");
	expect_rc(DISK "13:4\tw:0x10,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\tbit 4\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "an unknown flag");
	expect_rc(DISK "13:4\tw:0,b:0,w:1,w:0,w:0,l:0\tEXACT\t-\tA:\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "a floppy read");
	expect_rc(DISK "13:4\tw:0,l:0x1000,w:1,w:0,w:2,l:0\tEXACT\t-\taddr\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "a raw address");
	expect_rc(DISK "13:4\tw:0,l:0,w:1,w:0,w:2,l:0\tEXACT\t-\tNULL C:\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "NULL is the floppy arm only");
	expect_rc(DISK "13:4\tw:0,l:0,w:3,w:0,w:0,l:0\tEXACT\t-\tNULL 3\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "the NULL arm takes 0..2");
	expect_rc(DISK "13:4\tw:1,l:0,w:2,w:0,w:0,l:0\tEXACT\t-\tNULL write\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "the NULL arm is a read");
	expect_rc(DISK "13:4\tw:0,b:0,w:1,w:0,w:2\tEXACT\t-\tshort form\n",
	          TD_CASE_ERR_SIGNATURE, 2, "only the long form");
}

static void test_writes(void)
{
	printf("Rwabs writes: write-back only, on the test drive\n");
	expect_ok(DISK PRE_READ0 WRITE0, 1, "read then write back");
	TD_CHECK_EQ(t.prelude_count, 1);
	TD_CHECK_EQ(t.cases[0].line, 3);
	TD_CHECK_EQ(t.cases[0].prelude_first, 0);
	TD_CHECK_EQ(t.cases[0].prelude_count, 1);
	TD_CHECK_EQ(t.preludes[0].line, 2);
	TD_CHECK_EQ(t.preludes[0].opcode, 4);
	TD_CHECK_EQ(t.preludes[0].arg[0].value, 2);

	expect_ok(DISK
	    "+13:4\tw:0,b:1,w:2,w:-1,w:2,l:130975\t-\t-\tlast two\n"
	    "13:4\tw:3,b:1,w:2,w:-1,w:2,l:130975\tEXACT\t-\tback, NOMEDIACH\n",
	    1, "two sectors, buffer 1, RW_NOMEDIACH");

	expect_rc(DISK WRITE0, TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "a write with no read before it");
	expect_rc(DISK
	    "+13:4\tw:2,b:0,w:1,w:1,w:2,l:0\t-\t-\tsector 1\n" WRITE0,
	    TD_CASE_ERR_UNSAFE_FIELD, 3, "a write of another sector");
	expect_rc(DISK
	    "+13:4\tw:2,b:1,w:1,w:0,w:2,l:0\t-\t-\tbuffer 1\n" WRITE0,
	    TD_CASE_ERR_UNSAFE_FIELD, 3, "a write from another buffer");
	expect_rc(DISK
	    "+13:4\tw:2,b:0,w:2,w:0,w:2,l:0\t-\t-\ttwo sectors\n" WRITE0,
	    TD_CASE_ERR_UNSAFE_FIELD, 3, "a write of another count");
	expect_rc("!disk\t3\n" PRE_READ0 WRITE0, TD_CASE_ERR_UNSAFE_FIELD, 3,
	          "a write to a drive that is not the test drive");
	expect_rc(DISK PRE_READ0
	    "+13:4\tw:2,b:1,w:1,w:0,w:2,l:0\t-\t-\tanother read last\n" WRITE0,
	    TD_CASE_ERR_UNSAFE_FIELD, 4, "the read must be the LAST prelude");
	expect_rc(DISK "+13:4\tw:1,b:0,w:1,w:0,w:2,l:0\t-\t-\twrite\n"
	          "13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "a write is never a prelude");
	expect_rc(DISK
	    "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t-\tread\n"
	    "13:4\tw:9,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\tphysical write\n",
	    TD_CASE_ERR_UNSAFE_FIELD, 3, "no physical write");
}

static void test_preludes(void)
{
	printf("preludes in a disk table\n");
	expect_rc(DISK PRE_READ0, TD_CASE_ERR_DANGLING, 2, "a prelude at the end");
	expect_rc(DISK "+13:7\tw:2\t-\t-\tGetbpb\n13:10\t-\tEXACT\t-\tDrvmap\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "a prelude is an Rwabs read into a buffer, nothing else");
	expect_rc(DISK "+13:4\tw:0,l:0,w:2,w:0,w:0,l:0\t-\t-\tNULL arm\n"
	          "13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "the NULL arm is no prelude");
	expect_rc(DISK "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\tread\n"
	          "13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_RETCLASS, 2,
	          "a prelude has no retclass");
	expect_rc(DISK "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t1:16\t-\tread\n"
	          "13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_CAPTURE, 2,
	          "a prelude has no capture");
	expect_ok(DISK PRE_READ0 PRE_READ0
	          "13:4\tw:0,b:0,w:1,w:1,w:2,l:0\tEXACT\t1:16\tread\n", 1,
	          "preludes stack before a read case too");
	TD_CHECK_EQ(t.cases[0].prelude_count, 2);
}

static void test_capture_ret(void)
{
	printf("capture through the returned pointer (r:)\n");
	expect_ok(DISK "13:7\tw:2\tPTR_RAM\tr:18.W\tGetbpb BPB\n", 1, "r:18.W");
	TD_CHECK_EQ(t.cases[0].cap_arg, TD_CALL_CAP_RET);
	TD_CHECK_EQ(t.cases[0].cap_len, 18);
	TD_CHECK_EQ(t.cases[0].cap_unit, 2);
	TD_CHECK_EQ(t.cases[0].cap_off, 0);
	expect_ok(DISK "13:7\tw:2\tPTR_RAM\tr:4.L@4\tGetbpb\n", 1, "r:4.L@4");
	TD_CHECK_EQ(t.cases[0].cap_off, 4);
	expect_rc(DISK "13:7\tw:2\tNONZERO\tr:18.W\tGetbpb\n", TD_CASE_ERR_CAPTURE,
	          2, "only a PTR_RAM return is read through");
	expect_rc(DISK "13:7\tw:2\tPTR_RAM\tr:0\tGetbpb\n", TD_CASE_ERR_CAPTURE, 2,
	          "length 0");
	expect_rc(DISK "13:7\tw:2\tPTR_RAM\tr:257\tGetbpb\n", TD_CASE_ERR_CAPTURE,
	          2, "longer than the record");
	expect_rc(DISK "13:7\tw:2\tPTR_RAM\tr:3.W\tGetbpb\n", TD_CASE_ERR_CAPTURE,
	          2, "not whole WORDs");
}

/* The closing tables (TASK/TOOL07, user decision 2026-10-28): the
 * "destructive" mark, the "+set" prelude and the unit token "u:0". */
#define DESTR "!disk\t2\tdestructive\n"
#define WRITE0_NM "13:4\tw:3,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\twrite it back\n"

static void test_destructive_mark(void)
{
	printf("the destructive mark\n");
	expect_ok(DESTR "13:10\t-\tEXACT\t-\tDrvmap\n", 1, "!disk 2 destructive");
	TD_CHECK_EQ(t.disk, 1);
	TD_CHECK_EQ(t.disk_drive, 2);
	TD_CHECK_EQ(t.destructive, 1);
	expect_ok(DISK "13:10\t-\tEXACT\t-\tDrvmap\n", 1, "!disk 2");
	TD_CHECK_EQ(t.destructive, 0);
	expect_rc("!disk\t2\tdestroy\n13:10\t-\tEXACT\t-\tDrvmap\n",
	          TD_CASE_ERR_DIRECTIVE, 1, "another word than destructive");
	expect_rc("!disk\tdestructive\n13:10\t-\tEXACT\t-\tDrvmap\n",
	          TD_CASE_ERR_DIRECTIVE, 1, "the mark in place of the drive");
	expect_rc("!disk\t2\tdestructive\tx\n13:10\t-\tEXACT\t-\tDrvmap\n",
	          TD_CASE_ERR_DIRECTIVE, 1, "one word after the drive, no more");
}

static void test_set(void)
{
	printf("+set: only in a destructive table, right after its read\n");
	expect_ok(DESTR PRE_READ0 "+set\t0x0C=0x04,11=0\n" WRITE0_NM, 1,
	          "read, set, write back");
	TD_CHECK_EQ(t.prelude_count, 2);
	TD_CHECK_EQ(t.cases[0].prelude_count, 2);
	TD_CHECK_EQ(t.preludes[1].line, 3);
	TD_CHECK_EQ(t.preludes[1].trap, TD_CALL_TRAP_SET);
	TD_CHECK_EQ(t.preludes[1].nargs, 2);
	TD_CHECK_EQ(t.preludes[1].arg[0].kind, TD_CALL_ARG_SET);
	TD_CHECK_EQ(t.preludes[1].arg[0].value, (0x0C << 8) | 0x04);
	TD_CHECK_EQ(t.preludes[1].arg[1].value, (11 << 8) | 0);
	expect_ok(DESTR PRE_READ0 "+set\t0x20=0\tthe comment\n" WRITE0, 1,
	          "a comment column, the write without RW_NOMEDIACH");
	TD_CHECK(strcmp(t.preludes[1].comment, "the comment") == 0);
	expect_ok(DESTR
	    "+13:4\tw:2,b:1,w:2,w:-1,w:2,l:130975\t-\t-\ttwo\n"
	    "+set\t1023=0xFF,0=1,2=3,4=5,6=7,8=9,10=11,12=13\n"
	    "13:4\tw:1,b:1,w:2,w:-1,w:2,l:130975\tEXACT\t-\tback\n", 1,
	    "eight pairs, the last byte of a two-sector read");
	TD_CHECK_EQ(t.preludes[1].nargs, 8);
	expect_ok(DESTR PRE_READ0 WRITE0, 1,
	          "a plain write-back in a destructive table");

	expect_rc(DISK PRE_READ0 "+set\t0x0C=4\n" WRITE0,
	          TD_CASE_ERR_UNSAFE_STATE, 3, "not in a table that is not "
	          "destructive");
	expect_rc("+set\t0x0C=4\n13:10\t-\tEXACT\t-\tDrvmap\n",
	          TD_CASE_ERR_UNSAFE_STATE, 1, "not without !disk");
	expect_rc(DESTR "+set\t1=2\n" WRITE0, TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "no read before it");
	expect_rc(DESTR "13:10\t-\tEXACT\t-\tDrvmap\n+set\t1=2\n" WRITE0,
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "after a case, not a read");
	expect_rc(DESTR PRE_READ0 "+set\t1=2\n+set\t3=4\n" WRITE0,
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "after another +set");
	expect_rc(DESTR "+13:4\tw:2,b:0,w:0,w:0,w:2,l:0\t-\t-\tcount 0\n"
	          "+set\t1=2\n" WRITE0, TD_CASE_ERR_UNSAFE_FIELD, 3,
	          "after a read of nothing");
	expect_rc(DESTR PRE_READ0 "+set\t512=1\n" WRITE0, TD_CASE_ERR_RANGE, 3,
	          "an offset past what the read read");
	expect_rc(DESTR PRE_READ0 "+set\t-1=1\n" WRITE0, TD_CASE_ERR_RANGE, 3,
	          "a negative offset");
	expect_rc(DESTR PRE_READ0 "+set\t1=256\n" WRITE0, TD_CASE_ERR_RANGE, 3,
	          "a byte past 255");
	expect_rc(DESTR PRE_READ0 "+set\t1=1,2=2,3=3,4=4,5=5,6=6,7=7,8=8,9=9\n"
	          WRITE0, TD_CASE_ERR_TOO_MANY_VALUES, 3, "nine pairs");
	expect_rc(DESTR PRE_READ0 "+set\t1=1,0x01=2\n" WRITE0,
	          TD_CASE_ERR_VALUE, 3, "an offset twice");
	expect_rc(DESTR PRE_READ0 "+set\t0x0C\n" WRITE0, TD_CASE_ERR_VALUE, 3,
	          "no byte");
	expect_rc(DESTR PRE_READ0 "+set\t=4\n" WRITE0, TD_CASE_ERR_VALUE, 3,
	          "no offset");
	expect_rc(DESTR PRE_READ0 "+set\t12=\n" WRITE0, TD_CASE_ERR_VALUE, 3,
	          "an empty byte");
	expect_rc(DESTR PRE_READ0 "+set\t12=4,\n" WRITE0, TD_CASE_ERR_VALUE, 3,
	          "an empty pair");
	expect_rc(DESTR PRE_READ0 "+set\n" WRITE0, TD_CASE_ERR_COLUMNS, 3,
	          "no list");
	expect_rc(DESTR PRE_READ0 "+set\t1=2\n"
	          "13:4\tw:0,b:0,w:1,w:1,w:2,l:0\tEXACT\t1:16\tread\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "a read case after it");
	expect_rc(DESTR PRE_READ0 "+set\t1=2\n13:10\t-\tEXACT\t-\tDrvmap\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "a case that is no Rwabs");
	expect_rc(DESTR PRE_READ0 "+set\t1=2\n"
	          "+13:4\tw:2,b:1,w:1,w:1,w:2,l:0\t-\t-\tread\n" WRITE0,
	          TD_CASE_ERR_UNSAFE_FIELD, 4, "another prelude after it");
	expect_rc(DESTR PRE_READ0 "+set\t1=2\n"
	          "13:4\tw:1,b:0,w:1,w:1,w:2,l:0\tEXACT\t-\tsector 1\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 4,
	          "the write-back names the read before the +set");
	expect_rc(DESTR PRE_READ0 "+set\t1=2\n", TD_CASE_ERR_DANGLING, 3,
	          "a +set at the end");

	/* +set is a prelude: no case id, so every reader skips it */
	TD_CHECK_EQ(td_case_line_is_prelude("+set\t1=2", 9), 1);
}

static void test_unit_token(void)
{
	printf("u:0, the test disk's physical unit, with RW_NOTRANSLATE only\n");
	expect_ok(DISK
	    "13:4\tw:8,b:0,w:1,w:0,u:0,l:0\tEXACT\t1:16@446\tMBR\n"
	    "13:4\tw:10,b:1,w:2,w:-1,u:0,l:63\tEXACT\t1:16\tNOMEDIACH\n"
	    "13:4\tw:12,b:0,w:0,w:0,u:0,l:0\tEXACT\t-\tNORETRIES, count 0\n",
	    3, "physical reads");
	TD_CHECK_EQ(t.unit_token, 1);
	TD_CHECK_EQ(t.cases[0].arg[4].kind, TD_CALL_ARG_UNIT);
	TD_CHECK_EQ(t.cases[0].arg[4].value, 0);
	expect_ok(DISK "13:10\t-\tEXACT\t-\tDrvmap\n", 1, "no token");
	TD_CHECK_EQ(t.unit_token, 0);

	expect_rc(DISK "13:4\tw:8,b:0,w:1,w:0,w:18,l:0\tEXACT\t-\tunit 18\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "a unit number, not the token");
	expect_rc(DISK "13:4\tw:0,b:0,w:1,w:0,u:0,l:0\tEXACT\t-\tlogical\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "the token without RW_NOTRANSLATE");
	expect_rc(DISK "13:4\tw:8,b:0,w:1,w:0,u:1,l:0\tEXACT\t-\tu:1\n",
	          TD_CASE_ERR_RANGE, 2, "u:1");
	expect_rc(DISK "+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t-\tread\n"
	          "13:4\tw:9,b:0,w:1,w:0,u:0,l:0\tEXACT\t-\tphysical write\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 3, "no physical write");
	expect_rc(DESTR "13:4\tw:11,b:0,w:1,w:-1,u:0,l:63\tEXACT\t-\twrite\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "no physical write in a destructive table either");
	expect_rc(DISK "13:4\tw:0x88,b:0,w:1,w:0,u:0,l:0\tEXACT\t-\tswap\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "no RW_NOBYTESWAP, physical");
	expect_rc(DISK "13:4\tw:8,l:0,w:1,w:0,u:0,l:0\tEXACT\t-\tNULL\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "a physical read needs a buffer");
	expect_rc(DISK "13:4\tw:8,b:0,w:3,w:0,u:0,l:0\tEXACT\t-\tthree\n",
	          TD_CASE_ERR_UNSAFE_FIELD, 2, "three sectors overrun a buffer");
	expect_rc(DISK "+13:4\tw:8,b:0,w:1,w:0,u:0,l:0\t-\t-\tphysical\n"
	          "13:10\t-\tEXACT\t-\tDrvmap\n", TD_CASE_ERR_UNSAFE_FIELD, 2,
	          "a prelude is a logical read");
	expect_rc(DISK "13:7\tu:0\tEXACT\t-\tGetbpb\n", TD_CASE_ERR_SIGNATURE,
	          2, "Getbpb takes a drive");
	expect_rc(DISK "13:4\tw:8,b:0,u:0,w:0,w:2,l:0\tEXACT\t-\tcount\n",
	          TD_CASE_ERR_SIGNATURE, 2, "the token is the dev argument only");
	expect_rc("13:4\tw:8,b:0,w:1,w:0,u:0,l:0\tEXACT\t-\tno !disk\n",
	          TD_CASE_ERR_UNSAFE_OPCODE, 1, "Rwabs outside a disk table");
	expect_rc("14:0x2C\tu:0\tEXACT\t-\tBconmap\n", TD_CASE_ERR_SIGNATURE,
	          1, "nowhere else");
}

int main(void)
{
	test_destructive_mark();
	test_set();
	test_unit_token();
	test_directive();
	test_outside_a_disk_table();
	test_getbpb_mediach();
	test_rwabs_reads();
	test_writes();
	test_preludes();
	test_capture_ret();
	return TD_TEST_RESULT();
}
