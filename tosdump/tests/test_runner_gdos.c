/*
 * test_runner_gdos.c -- the call runner on a GEMDOS media-change table
 * (TASK/TOOL20) against the stub backend: strings reach the call as the
 * table's text and the record as their token; the handle and the drive come
 * back through t:1 and t:2 as WORDs; the DTA is poisoned by Fsetdta and
 * Fsfirst only, so Fsnext finds the search state Fsfirst left; "d:<layout>"
 * captures the DTA field by field; "x:" gets the profile's extension and a
 * table that names one is not run without it.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "GEMDOS after a media
 * change".  The stub's GEMDOS arms are documented in stub_abi.h.
 */
#include "tdtest.h"
#include "cases.h"
#include "runner_call.h"
#include "abi.h"
#include "stub_abi.h"

static struct td_call_case_table table;
static uint8_t records[16 * TD_CALL_RECORD_SIZE];

static const char *text =
	"!layout\tdta\t44/44:packed\td_reserved:x21@0/0,d_attrib:b@21/21,"
	"d_time:w@22/22,d_date:w@24/24,d_length:L@26/26,d_fname:b14@30/30\n"
	"!disk\t2\tdestructive\n"
	"1:0x1A\tb:1\tIGNORE\t-\tFsetdta\n"                         /* 3  */
	"1:0x4E\ts:C:\\*.*,w:0x10\tEXACT\td:dta\tFsfirst\n"           /* 4  */
	"13:9\tw:2\tEXACT\t-\tMediach between\n"                     /* 5  */
	"1:0x4F\t-\tEXACT\td:dta\tFsnext\n"                         /* 6  */
	"1:0x4F\t-\tEXACT\td:dta\tFsnext\n"                         /* 7  */
	"1:0x4F\t-\tEXACT\td:dta\tFsnext: ENMFIL\n"                 /* 8  */
	"1:0x3C\ts:C:\\X.DAT,w:0\tHANDLE\t-\tFcreate\n"              /* 9  */
	"1:0x40\tt:1,l:512,b:0\tEXACT\t-\tFwrite\n"                 /* 10 */
	"1:0x3E\tt:1\tEXACT\t-\tFclose\n"                           /* 11 */
	"1:0x19\t-\tEXACT\t-\tDgetdrv\n"                            /* 12 */
	"1:0x0E\tw:2\tEXACT\t-\tDsetdrv(C:)\n"                      /* 13 */
	"1:0x0E\tt:2\tEXACT\t-\tDsetdrv back\n"                     /* 14 */
	"1:0x4B\tw:0,x:C:\\PXPROBE,c:child,l:0\tEXACT\td:dta\tPexec\n" /* 15 */
	"1:0x47\tb:0,w:3\tEXACT\t0:8\tDgetpath\n";                  /* 16 */

static uint16_t be16(const uint8_t *p)
{
	return (uint16_t)(((unsigned)p[0] << 8) | p[1]);
}

static uint32_t be32(const uint8_t *p)
{
	return ((uint32_t)be16(p) << 16) | be16(p + 2);
}

static uint64_t slot(const uint8_t *rec, int i)
{
	const uint8_t *p = rec + 40 + 8 * i;

	return ((uint64_t)be32(p) << 32) | be32(p + 4);
}

static const uint8_t *record(int k)
{
	return records + (size_t)k * TD_CALL_RECORD_SIZE;
}

static void load(void)
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

static void test_no_ext(void)
{
	int rc;

	printf("a table that names x: is not run without the profile's "
	       "extension\n");
	TD_CHECK_EQ(td_run_call_set_prog_ext(""), -1);
	TD_CHECK_EQ(td_run_call_set_prog_ext("PRGX"), -1);
	TD_CHECK_EQ(td_run_call_set_prog_ext("prg"), -1);
	td_stub_reset();
	rc = td_run_call(&table, records, sizeof records);
	TD_CHECK_EQ(rc, TD_RUN_CALL_ERR_PROGEXT);
	TD_CHECK_EQ(td_stub_ncalls, 0);
}

static void test_run(void)
{
	const struct td_stub_call *c;
	const uint8_t *rec;
	int rc, k;

	printf("strings, tokens, the DTA and the profile's extension\n");
	TD_CHECK_EQ(td_run_call_set_prog_ext("NAP"), 0);
	td_stub_reset();
	rc = td_run_call(&table, records, sizeof records);
	TD_CHECK_EQ(rc, 0);
	TD_CHECK_EQ(td_stub_ncalls, 14);

	/* Fsetdta got buffer 1 */
	c = &td_stub_calls[0];
	TD_CHECK(c->call_value[0] == (long)td_call_buffer(1));
	/* Fsfirst: the table's string, as a pointer; the token in the record */
	c = &td_stub_calls[1];
	TD_CHECK(strcmp(c->call_str, "C:\\*.*") == 0);
	TD_CHECK_EQ(c->call_kind[1], TD_ABI_ARG_WORD);
	TD_CHECK(c->call_value[1] == 0x10L);
	rec = record(1);
	TD_CHECK_EQ(be32(rec + 4), TD_CALL_STR_TOKEN(0));
	/* its DTA, slot by slot: entry 0 */
	TD_CHECK(slot(rec, 0) == 0x20);
	TD_CHECK(slot(rec, 1) == TD_STUB_DTA_TIME);
	TD_CHECK(slot(rec, 2) == TD_STUB_DTA_DATE);
	TD_CHECK(slot(rec, 3) == TD_STUB_DTA_LEN);
	TD_CHECK(slot(rec, 4) == 'F');
	TD_CHECK(slot(rec, 5) == '0');
	TD_CHECK(slot(rec, 10) == 0);                /* the NUL */
	/* past the NUL: what the runner laid before Fsfirst, the UBYTE
	 * poison of the layout -- the stub writes up to the NUL, as packit */
	TD_CHECK(slot(rec, 11) == 0xAD);
	TD_CHECK(slot(rec, 17) == 0xAD);
	/* the layout's 18 slots, then the record's own poison */
	TD_CHECK_EQ(be16(rec + 40 + 18 * 8), 0xDEAD);

	/* Fsnext after a Mediach: the DTA was not poisoned between them,
	 * so the search goes on -- entries 1 and 2, then ENMFIL */
	rec = record(3);
	TD_CHECK(slot(rec, 1) == TD_STUB_DTA_TIME + 1);
	TD_CHECK(slot(rec, 5) == '1');
	rec = record(4);
	TD_CHECK(slot(rec, 5) == '2');
	rec = record(5);
	TD_CHECK_EQ(be32(rec + 36), (uint32_t)-49);
	TD_CHECK(slot(rec, 5) == '2');               /* the last entry stays */

	/* Fcreate's handle comes back through t:1, as a WORD */
	c = &td_stub_calls[6];                       /* Fcreate */
	TD_CHECK(strcmp(c->call_str, "C:\\X.DAT") == 0);
	c = &td_stub_calls[7];                       /* Fwrite */
	TD_CHECK_EQ(c->opcode, 0x40);
	TD_CHECK_EQ(c->call_kind[0], TD_ABI_ARG_WORD);
	TD_CHECK(c->call_value[0] == TD_STUB_HANDLE);
	TD_CHECK(c->call_value[2] == (long)td_call_buffer(0));
	rec = record(7);
	TD_CHECK_EQ(be32(rec + 4), TD_CALL_SUPER_TOKEN(TD_CALL_TOKEN_HANDLE));
	c = &td_stub_calls[8];                       /* Fclose */
	TD_CHECK(c->call_value[0] == TD_STUB_HANDLE);

	/* Dgetdrv's drive comes back through t:2 */
	c = &td_stub_calls[11];                      /* Dsetdrv(t:2) */
	TD_CHECK_EQ(c->opcode, 0x0E);
	TD_CHECK_EQ(c->call_kind[0], TD_ABI_ARG_WORD);
	TD_CHECK(c->call_value[0] == TD_STUB_DRIVE);

	/* Pexec: the program path with the profile's extension, the tail
	 * with its length byte; both tokens in the record */
	c = &td_stub_calls[12];
	TD_CHECK(strcmp(c->call_str, "C:\\PXPROBE.NAP") == 0);
	TD_CHECK_EQ(c->call_str2[0], 5);
	TD_CHECK(strcmp(c->call_str2 + 1, "child") == 0);
	rec = record(12);
	TD_CHECK_EQ(be32(rec + 36), TD_STUB_PEXEC_RET);
	TD_CHECK_EQ(be32(rec + 8), TD_CALL_STR_TOKEN(2));
	TD_CHECK_EQ(be32(rec + 12), TD_CALL_STR_TOKEN(3));
	/* the DTA after it: Pexec does not poison it, the last Fsnext's
	 * entry is still there */
	TD_CHECK(slot(rec, 5) == '2');

	/* Dgetpath into buffer 0: the string, bytes as they are */
	rec = record(13);
	TD_CHECK_MEM(rec + 40, "\\SUB", 5);
	for (k = 0; k < 13; k++)
		TD_CHECK_EQ(be16(record(k) + 300), k == 2 ? 13 : 1);
}

int main(void)
{
	load();
	test_no_ext();
	test_run();
	return TD_TEST_RESULT();
}
