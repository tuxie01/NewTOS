/*
 * test_runner_ptr.c -- the call runner on structures behind a pointer
 * (TASK/TOOL16) against the stub backend: a layout capture writes one
 * 64-bit slot per field, follows a pointer only where the follow rule
 * admits it (and records why not, without reading), lays typed poison in a
 * buffer it reads through a layout, and "+put" writes through the previous
 * case's returned pointer only when that pointer passes the rule.  The m68k
 * column of a layout is read here too, through td_layout_capture.
 *
 * Writes the stub dump and its table to argv[1] / argv[2] for the
 * comparator tests (test_tosdiff.sh).
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md section 6, "Structures behind a
 * pointer".  The stub's structures: stub_abi.h, "Structures behind a
 * pointer".
 */
#include "tdtest.h"
#include "cases.h"
#include "runner_call.h"
#include "dump.h"
#include "abi.h"
#include "stub_abi.h"

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
#define L_PTRS   "!layout\tptrs\t16/32\ta:p>iorec@0/0,b:p>iorec@4/8," \
                 "c:p>iorec@8/16,d:p>iorec@12/24\n"

static const char *table_text =
	"# stub table of test_runner_ptr.c (TOOL16)\n"            /* 1 */
	L_IOREC L_EXT L_MD L_MPB L_PTRS                            /* 2-6 */
	"!write\treturn\n"                                          /* 7 */
	"14:0x0E\tw:0\tPTR_RAM\tr:extiorec\tIorec(0)\n"             /* 8  c0 */
	"14:0x0E\tw:1\tPTR_RAM\tr:iorec\tIorec(1)\n"                /* 9  c1 */
	"+put\tibufhd=ibuftl\tthe flush\n"                          /* 10 */
	"13:1\tw:2\tEXACT\t-\tBconstat(2) after the flush\n"        /* 11 c2 */
	"14:0x0E\tw:1\tPTR_RAM\tr:iorec\tIorec(1) read back\n"      /* 12 c3 */
	"14:0x0E\tw:5\tPTR_RAM\tr:iorec\tIorec answers 0x0E\n"      /* 13 c4 */
	"+put\tibufhd=ibuftl\tnot written: 0x0E\n"                  /* 14 */
	"13:1\tw:2\tEXACT\t-\tBconstat(2)\n"                        /* 15 c5 */
	"14:0x0E\tw:6\tPTR_RAM\tr:iorec\truns past 0x80000000\n"    /* 16 c6 */
	"14:0x0E\tw:7\tPTR_RAM\tr:ptrs\tfour pointers followed\n"   /* 17 c7 */
	"1:0x20\tl:0\tNONZERO\t-\tSuper(0L)\n"                      /* 18 c8 */
	"13:0\tb:0\tIGNORE\t0:mpb\tGetmpb into buffer 0\n"          /* 19 c9 */
	"1:0x20\tt:0\tIGNORE\t-\tSuper(token)\n";                   /* 20 c10 */

#define NCASES 11

static struct td_call_case_table table;
static uint8_t records[NCASES * TD_CALL_RECORD_SIZE];

static const uint8_t *rec(int i)
{
	return records + (size_t)i * TD_CALL_RECORD_SIZE;
}

static uint64_t slot_at(const uint8_t *capture, int i)
{
	const uint8_t *p = capture + i * TD_LAYOUT_SLOT_BYTES;
	uint64_t v = 0;
	int k;

	for (k = 0; k < 8; k++)
		v = (v << 8) | p[k];
	return v;
}

static uint64_t slot(int c, int i)
{
	return slot_at(rec(c) + TD_CALL_OFF_CAPTURE, i);
}

#define TD_CHECK_SLOT(c, i, want) \
	do { \
		uint64_t g_ = slot((c), (i)), w_ = (uint64_t)(want); \
		if (g_ != w_) { \
			printf("  FAIL %s:%d: case %d slot %d: got 0x%016llX want " \
			       "0x%016llX\n", __FILE__, __LINE__, (c), (i), \
			       (unsigned long long)g_, (unsigned long long)w_); \
			td_test_failures++; \
		} \
	} while (0)

#define POISON_PTR 0xFFFFFFFFDEADDEADuLL

static void test_iorec(void)
{
	int i;

	printf("r:<layout>: one 64-bit slot per field, in layout order\n");
	/* Iorec(1): buf, size, head, tail, low, high */
	TD_CHECK_SLOT(1, 0, TD_STUB_RING_ADDR);
	TD_CHECK_SLOT(1, 1, 256);
	TD_CHECK_SLOT(1, 2, TD_STUB_IKBD_HEAD);
	TD_CHECK_SLOT(1, 3, TD_STUB_IKBD_TAIL);
	TD_CHECK_SLOT(1, 4, 64);
	TD_CHECK_SLOT(1, 5, 192);
	/* after the six slots the capture keeps the poison */
	TD_CHECK_MEM(rec(1) + TD_CALL_OFF_CAPTURE + 6 * 8, "\xDE\xAD\xDE\xAD", 4);

	/* the EXT_IOREC: in.*, out.*, then five UBYTEs, zero-extended */
	TD_CHECK_SLOT(0, 1, 256);
	TD_CHECK_SLOT(0, 6, TD_STUB_RING_ADDR);       /* out.ibuf */
	TD_CHECK_SLOT(0, 8, 2);                       /* out.ibufhd */
	TD_CHECK_SLOT(0, 12, 1);                      /* baudrate */
	TD_CHECK_SLOT(0, 14, 0x88);                   /* ucr */
	TD_CHECK_SLOT(0, 15, 0xFF);                   /* datamask, not -1 */
	TD_CHECK_SLOT(0, 16, 0xEA);                   /* wr5 */

	printf("the follow rule: nothing is read, every slot says why\n");
	for (i = 0; i < 6; i++) {
		TD_CHECK_SLOT(4, i, TD_LAYOUT_NOREAD(TD_NOREAD_LOW));   /* 0x0E */
		TD_CHECK_SLOT(6, i, TD_LAYOUT_NOREAD(TD_NOREAD_END));   /* 0x7FFFFFF0 */
	}
	/* four pointers: the IKBD IOREC, 0x0E, 0x401, NULL -- each followed
	 * only when it passes; the slots are pointer, then its six fields */
	TD_CHECK_SLOT(7, 0, TD_STUB_IKBD_ADDR);
	TD_CHECK_SLOT(7, 2, 256);                     /* a->ibufsiz: read */
	TD_CHECK_SLOT(7, 7, 0x0E);
	TD_CHECK_SLOT(7, 8, TD_LAYOUT_NOREAD(TD_NOREAD_LOW));
	TD_CHECK_SLOT(7, 14, 0x401);
	TD_CHECK_SLOT(7, 15, TD_LAYOUT_NOREAD(TD_NOREAD_ODD));
	TD_CHECK_SLOT(7, 21, 0);
	TD_CHECK_SLOT(7, 27, TD_LAYOUT_NOREAD(TD_NOREAD_NULL));
}

static void test_put(void)
{
	printf("+put: written through the previous case's pointer, or not at all\n");
	/* before the flush head 4, tail 8; after it the stub's Bconstat(2)
	 * finds head == tail and answers 0; the read-back shows it */
	TD_CHECK_SLOT(1, 2, TD_STUB_IKBD_HEAD);
	TD_CHECK_MEM(rec(2) + TD_CALL_OFF_RET, "\x00\x00\x00\x00", 4);
	TD_CHECK_SLOT(3, 2, TD_STUB_IKBD_TAIL);
	TD_CHECK_SLOT(3, 3, TD_STUB_IKBD_TAIL);
	TD_CHECK_EQ(td_run_call_put_result(0), TD_PUT_WRITTEN);
	/* through 0x0E: refused, nothing written (a write there would fault
	 * the host test, which has no memory below 4 GiB) */
	TD_CHECK_EQ(td_run_call_put_result(1), TD_NOREAD_LOW);
	TD_CHECK_EQ(td_run_call_put_result(2), 0);
}

static void test_getmpb(void)
{
	printf("<n>:<layout>: the buffer in typed poison, its pointer followed\n");
	TD_CHECK_SLOT(9, 0, TD_STUB_MD_ADDR);                 /* mp_free */
	TD_CHECK_SLOT(9, 1, 0);                               /* ->md_next */
	TD_CHECK_SLOT(9, 2, TD_STUB_MD_START);
	TD_CHECK_SLOT(9, 3, TD_STUB_MD_LENGTH);
	TD_CHECK_SLOT(9, 4, 0);                               /* ->md_owner */
	TD_CHECK_SLOT(9, 5, 0);                               /* mp_used */
	/* mp_rover, which Getmpb leaves alone: the pointer poison, the same
	 * 64-bit value an m68k buffer reads as (below) */
	TD_CHECK_SLOT(9, 6, POISON_PTR);
	/* the argument is the buffer's token, as for any b: argument */
	TD_CHECK_MEM(rec(9) + TD_CALL_OFF_ARGS, "\xB0\xFF\x00\x00", 4);
}

/*
 * The m68k column: a structure laid at the m68k offsets with 4-byte
 * pointers and longs, read through td_layout_capture with the m68k ABI.
 * Byte order is the host's own, as it is the target's own on the m68k.
 */
static void test_m68k_column(void)
{
	uint8_t mpb[12], cap[TD_CALL_CAPTURE_LEN];
	uint8_t *md = (uint8_t *)(uintptr_t)td_abi_addr(TD_STUB_MD_ADDR + 32);
	int32_t v;
	const struct td_layout *l = &table.layouts[3];

	printf("the m68k column: 4-byte pointers and longs, sign-extended\n");
	memset(cap, 0xEE, sizeof cap);
	v = (int32_t)(TD_STUB_MD_ADDR + 32);
	memcpy(mpb, &v, 4);                            /* mp_free */
	v = 0;
	memcpy(mpb + 4, &v, 4);                        /* mp_used */
	v = (int32_t)0xDEADDEADuL;
	memcpy(mpb + 8, &v, 4);                        /* mp_rover, poison */
	memset(md, 0, 16);
	v = 0x00023BA0;
	memcpy(md + 4, &v, 4);                         /* md_start */
	v = -2;
	memcpy(md + 8, &v, 4);                         /* md_length < 0 */
	td_layout_capture(cap, table.layouts, l, mpb, 0, TD_LAYOUT_ABI_M68K);
	TD_CHECK(slot_at(cap, 0) == (uint64_t)(TD_STUB_MD_ADDR + 32));
	TD_CHECK(slot_at(cap, 2) == 0x00023BA0uLL);
	TD_CHECK(slot_at(cap, 3) == 0xFFFFFFFFFFFFFFFEuLL);
	TD_CHECK(slot_at(cap, 6) == POISON_PTR);       /* = the LP64 poison */
	TD_CHECK(cap[7 * 8] == 0xEE);                  /* nothing after */

	/* a refused base: no read at all, every slot the reason */
	memset(cap, 0xEE, sizeof cap);
	td_layout_capture(cap, table.layouts, l, NULL, TD_NOREAD_HIGH,
	                  TD_LAYOUT_ABI_M68K);
	TD_CHECK(slot_at(cap, 0) == TD_LAYOUT_NOREAD(TD_NOREAD_HIGH));
	TD_CHECK(slot_at(cap, 6) == TD_LAYOUT_NOREAD(TD_NOREAD_HIGH));
}

static void test_follow_rule(void)
{
	printf("td_layout_follow_refusal: PTR_RAM, then the end of the structure\n");
	TD_CHECK_EQ(td_layout_follow_refusal(0, 24), TD_NOREAD_NULL);
	TD_CHECK_EQ(td_layout_follow_refusal(0x80000000uLL, 24), TD_NOREAD_HIGH);
	TD_CHECK_EQ(td_layout_follow_refusal(0x4E75000000000000uLL, 24),
	            TD_NOREAD_HIGH);
	TD_CHECK_EQ(td_layout_follow_refusal(0x3FE, 24), TD_NOREAD_LOW);
	TD_CHECK_EQ(td_layout_follow_refusal(0x401, 24), TD_NOREAD_ODD);
	TD_CHECK_EQ(td_layout_follow_refusal(0x7FFFFFF0uLL, 24), TD_NOREAD_END);
	TD_CHECK_EQ(td_layout_follow_refusal(0x7FFFFFE8uLL, 24), 0);
	TD_CHECK_EQ(td_layout_follow_refusal(0x400, 24), 0);
}

int main(int argc, char **argv)
{
	struct td_case_error err;
	int rc;
	FILE *f;

	if (argc != 3) {
		fprintf(stderr, "usage: test_runner_ptr OUT.BIN OUT.TSV\n");
		return 2;
	}
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
	rc = td_run_call(&table, records, sizeof records);
	TD_CHECK_EQ(rc, 0);

	test_iorec();
	test_put();
	test_getmpb();
	test_m68k_column();
	test_follow_rule();

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
