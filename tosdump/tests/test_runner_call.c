/*
 * test_runner_call.c -- drives the call runner against the stub backend (T7).
 *
 * States the contract of docs/TOS-REFERENCE-DUMPER.md sections 3, 4 and 6
 * ("The call table, as built for DUMPBIOS") and checks the runner against
 * it: the marshalled arguments, the poison, the record bytes.  Offsets and
 * byte strings are written out here, not read back from tosdump.h.
 *
 * Writes the stub dump and its table to argv[1] / argv[2]; the comparator
 * tests (test_tosdiff.sh) take them from there.
 *
 * usage: test_runner_call OUT.BIN OUT.TSV
 */
#include "tdtest.h"
#include "cases.h"
#include "dump.h"
#include "runner_call.h"
#include "abi.h"
#include "stub_abi.h"

/* Every line here passes the guard: the runner is under test, not the
 * loader.  The retclass column is what test_tosdiff.sh exercises later. */
static const char *table_text =
	"# stub call table\n"
	"13:1\tw:2\tEXACT\t-\tBconstat CON\n"
	"14:0x02\t-\tPTR_RAM\t-\tPhysbase\n"
	"14:0x12\tb:0,l:0xFFFFFFFF,w:-1,w:-1\tIGNORE\t0:6\tProtobt bytes\n"
	"14:0x6A\tb:0,b:1\tIGNORE\t1:4.L@8\tDsp_Available a LONG\n"
	"14:0x2E\tw:0,w:0,w:50,b:1\tEXACT.W\t3:2@510\tNVMaccess tail\n"
	"14:0x55\tw:0,w:16,b:1\tIGNORE\t2:8.W\tEgetPalette poison\n"
	"14:0x04\t-\tEXACT.W\t-\tGetrez\n"
	"13:10\t-\tNONZERO\t-\tDrvmap\n"
	"14:0x59\t-\tHANDLE.W\t-\tVgetMonitor\n";

#define NCASES 9

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

static const long poison_long = (long)(int32_t)0xDEADDEADuL;

static void test_marshalling(void)
{
	const struct td_stub_call *c;
	int i;

	printf("the backend is handed the binding's arguments\n");
	TD_CHECK_EQ(td_stub_ncalls, NCASES);

	c = &td_stub_calls[0];                              /* Bconstat(2) */
	TD_CHECK_EQ(c->kind, TD_STUB_KIND_CALL);
	TD_CHECK_EQ(c->call_trap, 13);
	TD_CHECK_EQ(c->opcode, 1);
	TD_CHECK_EQ(c->call_nargs, 1);
	TD_CHECK_EQ(c->call_kind[0], TD_ABI_ARG_WORD);
	TD_CHECK(c->call_value[0] == 2L);
	for (i = 1; i < TD_CALL_ARGS_N; i++)
		TD_CHECK(c->call_value[i] == poison_long);

	c = &td_stub_calls[1];                              /* Physbase() */
	TD_CHECK_EQ(c->call_trap, 14);
	TD_CHECK_EQ(c->opcode, 2);
	TD_CHECK_EQ(c->call_nargs, 0);
	for (i = 0; i < TD_CALL_ARGS_N; i++)
		TD_CHECK(c->call_value[i] == poison_long);

	c = &td_stub_calls[2];                              /* Protobt */
	TD_CHECK_EQ(c->call_nargs, 4);
	TD_CHECK_EQ(c->call_kind[0], TD_ABI_ARG_LONG);
	TD_CHECK(c->call_value[0] == (long)td_call_buffer(0));
	TD_CHECK_EQ(c->call_kind[1], TD_ABI_ARG_LONG);
	TD_CHECK(c->call_value[1] == -1L);                  /* sign-extended */
	TD_CHECK_EQ(c->call_kind[2], TD_ABI_ARG_WORD);
	TD_CHECK(c->call_value[2] == -1L);                  /* sign-narrowed */
	TD_CHECK_EQ(c->call_kind[3], TD_ABI_ARG_WORD);

	c = &td_stub_calls[3];                              /* Dsp_Available */
	TD_CHECK(c->call_value[0] == (long)td_call_buffer(0));
	TD_CHECK(c->call_value[1] == (long)td_call_buffer(1));
	TD_CHECK(td_call_buffer(0) != td_call_buffer(1));
	TD_CHECK(td_call_buffer(0) != NULL);
	TD_CHECK(td_call_buffer(TD_CALL_BUF_N) == NULL);
}

static void test_records(void)
{
	uint8_t zero[18];
	const uint8_t *r;
	int i;

	printf("the records\n");
	memset(zero, 0, sizeof zero);

	r = rec(0);                                          /* Bconstat(2) */
	TD_CHECK_MEM(r + 0, "\x00\x02", 2);                  /* case id = line */
	TD_CHECK_MEM(r + 2, "\x00\x01", 2);                  /* function      */
	TD_CHECK_MEM(r + 4, "\x00\x00\x00\x02", 4);          /* WORD 2        */
	for (i = 1; i < 8; i++)
		TD_CHECK_MEM(r + 4 + i * 4, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_MEM(r + 36, "\xFF\xFF\xFF\xFF", 4);         /* -1, low half  */
	TD_CHECK_MEM(r + 296, "\xFF\xFF\xFF\xFF", 4);        /* and high half */
	TD_CHECK_MEM(r + 300, "\x00\x0D", 2);                /* trap 13       */
	TD_CHECK_MEM(r + 40, "\xDE\xAD\xDE\xAD", 4);         /* no capture    */
	TD_CHECK_MEM(r + 294, "\xDE\xAD", 2);
	TD_CHECK_MEM(r + 302, zero, 18);

	r = rec(1);                                          /* Physbase */
	TD_CHECK_MEM(r + 0, "\x00\x03", 2);
	TD_CHECK_MEM(r + 300, "\x00\x0E", 2);                /* trap 14 */
	TD_CHECK_EQ(be32(r + 36), (uint32_t)TD_STUB_PHYSBASE);
	TD_CHECK_MEM(r + 296, "\x00\x00\x00\x00", 4);

	r = rec(2);                                          /* Protobt */
	TD_CHECK_MEM(r + 4, "\xB0\xFF\x00\x00", 4);          /* buffer 0 token */
	TD_CHECK_MEM(r + 8, "\xFF\xFF\xFF\xFF", 4);          /* LONG -1        */
	TD_CHECK_MEM(r + 12, "\xFF\xFF\xFF\xFF", 4);         /* WORD -1        */
	TD_CHECK_MEM(r + 16, "\xFF\xFF\xFF\xFF", 4);
	TD_CHECK_MEM(r + 20, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_EQ(be32(r + 36), (uint32_t)TD_STUB_TRAP_RET(14, 0x12));
	TD_CHECK_MEM(r + 296, "\x00\x00\x00\x00", 4);
	TD_CHECK_MEM(r + 40, "STUB\x00\x12", 6);             /* 0:6 bytes */
	TD_CHECK_MEM(r + 46, "\xDE\xAD", 2);                 /* the rest poison */

	r = rec(3);                                          /* Dsp_Available */
	TD_CHECK_MEM(r + 4, "\xB0\xFF\x00\x00", 4);
	TD_CHECK_MEM(r + 8, "\xB0\xFF\x00\x01", 4);          /* buffer 1 token */
	/* the LONG the stub wrote in machine order, stored big-endian */
	TD_CHECK_MEM(r + 40, "\x0A\x0B\x0C\x0D", 4);
	TD_CHECK_MEM(r + 44, "\xDE\xAD", 2);

	r = rec(4);                                          /* NVMaccess */
	TD_CHECK_MEM(r + 40, "\x55\xAA", 2);                 /* 3:2@510 */

	/* The stub leaves EgetPalette's buffer alone, and buffer 1 was written
	 * by the two cases before: what the record shows is the poison the
	 * runner laid before THIS case, as WORDs. */
	r = rec(5);
	TD_CHECK_MEM(r + 12, "\xB0\xFF\x00\x01", 4);
	TD_CHECK_MEM(r + 40, "\xDE\xAD\xDE\xAD\xDE\xAD\xDE\xAD", 8);

	r = rec(6);                                          /* Getrez */
	TD_CHECK_MEM(r + 2, "\x00\x04", 2);
	TD_CHECK_EQ(be32(r + 36), (uint32_t)TD_STUB_TRAP_RET(14, 4));
}

/*
 * Super and Supexec (MEM12, section 6 "Super and Supexec in a BIOS-area
 * table"): the token Super(0L) returned travels into the closing case, the
 * callbacks are the runner's own, and the record carries tokens, never the
 * addresses.  The stub models EmuTOS: a processor state, x20_inq, and
 * Supexec calling the function in supervisor state (stub_abi.h).
 */
static const char *super_text =
	"# stub super table\n"
	"1:0x20\tl:1\tEXACT\t-\tSuper(1L) user\n"
	"1:0x20\tl:0\tNONZERO\t-\tSuper(0L) opens\n"
	"1:0x20\tl:1\tEXACT\t-\tSuper(1L) super\n"
	"1:0x20\tt:0\tIGNORE\t-\tSuperToUser\n"
	"1:0x20\tl:1\tEXACT\t-\tSuper(1L) user again\n"
	"14:0x26\tf:0\tEXACT\t-\tSupexec(cb0)\n"
	"14:0x26\tf:1\tEXACT\t-\tSupexec(cb1)\n"
	"14:0x26\tf:2\tEXACT\t-\tSupexec(cb2)\n";

#define NSUPER 8

static struct td_call_case_table super_table;
static uint8_t super_records[NSUPER * TD_CALL_RECORD_SIZE];

static const uint8_t *srec(int i)
{
	return super_records + (size_t)i * TD_CALL_RECORD_SIZE;
}

static void test_super_supexec(void)
{
	struct td_case_error err;
	const struct td_stub_call *c;
	int rc;

	printf("Super and Supexec\n");
	memset(&err, 0, sizeof err);
	rc = td_call_cases_parse(&super_table, TD_AREA_BIOS, super_text,
	                         strlen(super_text), &err);
	TD_CHECK_EQ(rc, 0);
	if (rc != 0) {
		printf("    line %u: %s\n", err.line, err.what ? err.what : "");
		return;
	}
	td_stub_reset();
	rc = td_run_call(&super_table, super_records, sizeof super_records);
	TD_CHECK_EQ(rc, 0);

	/* eight cases, plus the Super(1L) callback 2 issues from inside */
	TD_CHECK_EQ(td_stub_ncalls, NSUPER + 1);

	c = &td_stub_calls[0];                       /* Super(1L) */
	TD_CHECK_EQ(c->call_trap, 1);
	TD_CHECK_EQ(c->opcode, 0x20);
	TD_CHECK_EQ(c->call_nargs, 1);
	TD_CHECK_EQ(c->call_kind[0], TD_ABI_ARG_LONG);
	TD_CHECK(c->call_value[0] == 1L);
	c = &td_stub_calls[1];                       /* Super(0L) */
	TD_CHECK(c->call_value[0] == 0L);
	c = &td_stub_calls[3];                       /* Super(token) */
	TD_CHECK_EQ(c->call_kind[0], TD_ABI_ARG_LONG);
	TD_CHECK(c->call_value[0] == TD_STUB_SUPER_SSP);
	c = &td_stub_calls[5];                       /* Supexec(cb0) */
	TD_CHECK_EQ(c->call_trap, 14);
	TD_CHECK_EQ(c->opcode, 0x26);
	TD_CHECK_EQ(c->call_kind[0], TD_ABI_ARG_LONG);
	TD_CHECK(c->call_value[0] == (long)td_call_callback(0));
	TD_CHECK(td_call_callback(0) != NULL);
	TD_CHECK(td_call_callback(0) != td_call_callback(1));
	TD_CHECK(td_call_callback(3) == NULL);
	c = &td_stub_calls[8];                       /* cb2's own Super(1L) */
	TD_CHECK_EQ(c->call_trap, 1);
	TD_CHECK_EQ(c->opcode, 0x20);
	TD_CHECK(c->call_value[0] == 1L);

	/* the records: tokens, returns */
	TD_CHECK_MEM(srec(0) + 300, "\x00\x01", 2);           /* trap 1 */
	TD_CHECK_MEM(srec(0) + 2, "\x00\x20", 2);
	TD_CHECK_MEM(srec(0) + 4, "\x00\x00\x00\x01", 4);
	TD_CHECK_MEM(srec(0) + 36, "\x00\x00\x00\x00", 4);   /* user: 0 */
	TD_CHECK_EQ(be32(srec(1) + 36), (uint32_t)TD_STUB_SUPER_SSP);
	TD_CHECK_MEM(srec(2) + 36, "\xFF\xFF\xFF\xFF", 4);   /* super: -1 */
	TD_CHECK_MEM(srec(2) + 296, "\xFF\xFF\xFF\xFF", 4);
	TD_CHECK_MEM(srec(3) + 4, "\xB0\xF5\x00\x00", 4);    /* the token */
	TD_CHECK_MEM(srec(3) + 8, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_MEM(srec(4) + 36, "\x00\x00\x00\x00", 4);   /* user again */
	TD_CHECK_MEM(srec(5) + 4, "\xB0\xFC\x00\x00", 4);    /* callback 0 */
	TD_CHECK_MEM(srec(5) + 36, "\x12\x34\x56\x78", 4);
	TD_CHECK_MEM(srec(5) + 296, "\x00\x00\x00\x00", 4);
	TD_CHECK_MEM(srec(6) + 4, "\xB0\xFC\x00\x01", 4);
	TD_CHECK_MEM(srec(6) + 36, "\xFF\xFF\xFF\xFE", 4);   /* -2 ... */
	TD_CHECK_MEM(srec(6) + 296, "\xFF\xFF\xFF\xFF", 4);  /* ... 64 bit */
	TD_CHECK_MEM(srec(7) + 4, "\xB0\xFC\x00\x02", 4);
	TD_CHECK_MEM(srec(7) + 36, "\xFF\xFF\xFF\xFF", 4);   /* super in cb */
}

int main(int argc, char **argv)
{
	struct td_case_error err;
	int rc;
	FILE *f;

	if (argc != 3) {
		fprintf(stderr, "usage: test_runner_call OUT.BIN OUT.TSV\n");
		return 2;
	}

	printf("call case table\n");
	memset(&err, 0, sizeof err);
	rc = td_call_cases_parse(&table, TD_AREA_BIOS, table_text,
	                         strlen(table_text), &err);
	TD_CHECK_EQ(rc, 0);
	if (rc != 0) {
		printf("    line %u: %s\n", err.line, err.what ? err.what : "");
		return TD_TEST_RESULT();
	}
	TD_CHECK_EQ(table.count, NCASES);

	printf("the run\n");
	td_stub_reset();
	TD_CHECK_EQ(td_run_call(&table, records, sizeof records - 1),
	            TD_RUN_CALL_ERR_BUFFER);
	TD_CHECK_EQ(td_stub_ncalls, 0);
	rc = td_run_call(&table, records, sizeof records);
	TD_CHECK_EQ(rc, 0);

	test_marshalling();
	test_records();
	test_super_supexec();

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
