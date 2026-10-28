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

int main(void)
{
	test_directive();
	test_outside_a_disk_table();
	test_getbpb_mediach();
	test_rwabs_reads();
	test_writes();
	test_preludes();
	test_capture_ret();
	return TD_TEST_RESULT();
}
