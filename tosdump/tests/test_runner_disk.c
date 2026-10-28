/*
 * test_runner_disk.c -- the call runner on a disk table (TASK/TOOL07)
 * against the stub backend: preludes are issued after the poison and before
 * their case, their buffer reaches the case unpoisoned, a failed Rwabs
 * prelude stops the run, and "r:" captures through the returned pointer.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "The disk round".  The
 * stub's Rwabs and Getbpb arms are documented in stub_abi.h.
 */
#include "tdtest.h"
#include "cases.h"
#include "runner_call.h"
#include "abi.h"
#include "stub_abi.h"

static struct td_call_case_table table;
static uint8_t records[8 * TD_CALL_RECORD_SIZE];

static const char *disk_text =
	"!disk\t2\n"
	"+13:4\tw:2,b:0,w:1,w:0,w:2,l:0\t-\t-\tsector 0 into buffer 0\n"
	"13:4\tw:1,b:0,w:1,w:0,w:2,l:0\tEXACT\t-\twrite it back\n"
	"13:7\tw:2\tPTR_RAM\tr:18.W\tGetbpb, the BPB\n"
	"13:7\tw:15\tEXACT\t-\tGetbpb absent: no read through 0\n"
	"13:7\tw:2\tPTR_RAM\tr:4.W@2\tGetbpb, an offset\n";

static uint16_t be16(const uint8_t *p)
{
	return (uint16_t)(((unsigned)p[0] << 8) | p[1]);
}

static void load(const char *text)
{
	struct td_case_error err;
	int rc;

	memset(&err, 0, sizeof err);
	rc = td_call_cases_parse(&table, TD_AREA_BIOS, text, strlen(text), &err);
	if (rc != 0) {
		printf("  FAIL table refused: %d line %u %s\n", rc, err.line,
		       err.what ? err.what : "");
		exit(1);
	}
}

static void test_preludes_and_write(void)
{
	const struct td_stub_call *c;
	const uint8_t *rec;
	int rc, i;

	printf("a prelude runs after the poison, its buffer reaches the case\n");
	load(disk_text);
	td_stub_reset();
	rc = td_run_call(&table, records, sizeof records);
	TD_CHECK_EQ(rc, 0);
	/* prelude, write, three Getbpb: five trap calls, four records */
	TD_CHECK_EQ(td_stub_ncalls, 5);

	c = &td_stub_calls[0];                       /* the prelude: a read */
	TD_CHECK_EQ(c->call_trap, 13);
	TD_CHECK_EQ(c->opcode, 4);
	TD_CHECK_EQ(c->call_nargs, 6);
	TD_CHECK(c->call_value[0] == 2L);
	TD_CHECK(c->call_value[1] == (long)td_call_buffer(0));
	TD_CHECK(c->call_value[4] == 2L);

	c = &td_stub_calls[1];                       /* the measured write */
	TD_CHECK(c->call_value[0] == 1L);
	TD_CHECK(c->call_value[1] == (long)td_call_buffer(0));
	/* what the write was handed is what the read put there -- the stub's
	 * fill of a read, not the 0xDEAD poison: nothing poisoned the buffer
	 * between the prelude and its case */
	TD_CHECK_MEM(c->call_written, TD_STUB_BUF_TEXT, 4);
	TD_CHECK_EQ(c->call_written[4], 0);          /* buffer 0 */
	TD_CHECK_EQ(c->call_written[5], 4);          /* filled by function 4 */

	/* record 0 is the write, case id 3; the prelude left no record */
	rec = records;
	TD_CHECK_EQ(be16(rec + 0), 3);
	TD_CHECK_EQ(be16(rec + 2), 4);

	printf("r: captures through the returned pointer, in units\n");
	rec = records + 1 * TD_CALL_RECORD_SIZE;     /* Getbpb(2) r:18.W */
	TD_CHECK_EQ(be16(rec + 0), 4);
	for (i = 0; i < 9; i++)
		TD_CHECK_EQ(be16(rec + 40 + 2 * i), TD_STUB_BPB_WORD(i));
	TD_CHECK_EQ(be16(rec + 40 + 18), 0xDEAD);    /* past the capture */

	rec = records + 2 * TD_CALL_RECORD_SIZE;     /* Getbpb(15) -> 0 */
	TD_CHECK_EQ(be16(rec + 40), 0xDEAD);

	rec = records + 3 * TD_CALL_RECORD_SIZE;     /* r:4.W@2 */
	TD_CHECK_EQ(be16(rec + 40), TD_STUB_BPB_WORD(1));
	TD_CHECK_EQ(be16(rec + 42), TD_STUB_BPB_WORD(2));
	TD_CHECK_EQ(be16(rec + 44), 0xDEAD);
}

static void test_failed_prelude_stops(void)
{
	static const char *text =
		"!disk\t2\n"
		"13:10\t-\tEXACT\t-\tDrvmap\n"
		"+13:4\tw:2,b:0,w:1,w:-1,w:2,l:" TD_STUB_RWABS_FAIL_STR "\t-\t-\tfails\n"
		"13:4\tw:1,b:0,w:1,w:-1,w:2,l:" TD_STUB_RWABS_FAIL_STR "\tEXACT\t-\tback\n"
		"13:10\t-\tEXACT\t-\tDrvmap\n";
	int rc;

	printf("a failed Rwabs prelude stops the run before its case\n");
	load(text);
	td_stub_reset();
	rc = td_run_call(&table, records, sizeof records);
	TD_CHECK_EQ(rc, TD_RUN_CALL_ERR_PRELUDE);
	TD_CHECK_EQ(td_run_call_stop_line(), 3);
	TD_CHECK(td_run_call_stop_ret() == TD_STUB_RWABS_FAIL_RET);
	/* Drvmap and the failed prelude, nothing after it: the write never
	 * went out with a poisoned buffer */
	TD_CHECK_EQ(td_stub_ncalls, 2);
	TD_CHECK_EQ(td_stub_calls[1].opcode, 4);
	TD_CHECK(td_stub_calls[1].call_value[0] == 2L);
}

int main(void)
{
	test_preludes_and_write();
	test_failed_prelude_stops();
	return TD_TEST_RESULT();
}
