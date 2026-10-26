/*
 * test_record.c -- record format tests (spec section 3 + 4).
 *
 * Everything here is checked against hand-written big-endian byte literals,
 * never against the writer's own arithmetic, so the test cannot agree with a
 * broken writer.
 */
#include "tdtest.h"
#include "record.h"

static void test_store_load(void)
{
	uint8_t b[4];

	printf("store/load big-endian primitives\n");

	memset(b, 0, sizeof b);
	td_store_be16(b, 0x1234u);
	TD_CHECK_EQ(b[0], 0x12);
	TD_CHECK_EQ(b[1], 0x34);

	memset(b, 0, sizeof b);
	td_store_be16(b, 0xDEADu);
	TD_CHECK_EQ(b[0], 0xDE);
	TD_CHECK_EQ(b[1], 0xAD);

	memset(b, 0, sizeof b);
	td_store_be32(b, 0xDEADBEEFuL);
	TD_CHECK_EQ(b[0], 0xDE);
	TD_CHECK_EQ(b[1], 0xAD);
	TD_CHECK_EQ(b[2], 0xBE);
	TD_CHECK_EQ(b[3], 0xEF);

	TD_CHECK_EQ(td_load_be16(b), 0xDEADu);
	TD_CHECK_EQ(td_load_be32(b), 0xDEADBEEFuL);

	/* negative i32 must survive the round trip bit for bit */
	td_store_be32(b, (uint32_t)(int32_t)-33);
	TD_CHECK_EQ(b[0], 0xFF);
	TD_CHECK_EQ(b[3], 0xDF);
	TD_CHECK_EQ((int32_t)td_load_be32(b), -33);
}

static void test_header(void)
{
	uint8_t hdr[TD_HEADER_SIZE];
	struct td_header h, out;
	int rc;

	printf("file header\n");

	memset(&h, 0, sizeof h);
	h.version      = TD_FORMAT_VERSION;
	h.area         = TD_AREA_VDI;
	h.record_size  = TD_VDI_RECORD_SIZE;
	h.record_count = 7;
	memcpy(h.build_id, "19f7", 4);
	h.build_id[4]  = '\0';
	h.case_crc     = 0xCAFEBABEuL;

	memset(hdr, 0xFF, sizeof hdr);
	td_header_write(hdr, &h);

	TD_CHECK_MEM(hdr + TD_HDR_OFF_MAGIC, "TDMP", 4);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_VERSION], 0x00);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_VERSION + 1], 0x02);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_AREA], 0x00);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_AREA + 1], TD_AREA_VDI);
	/* 1592 == 0x0638 */
	TD_CHECK_EQ(hdr[TD_HDR_OFF_RECSIZE], 0x06);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_RECSIZE + 1], 0x38);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_RECCOUNT], 0x00);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_RECCOUNT + 1], 0x07);
	TD_CHECK_MEM(hdr + TD_HDR_OFF_BUILDID, "19f7", 4);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_CASECRC], 0xCA);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_CASECRC + 1], 0xFE);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_CASECRC + 2], 0xBA);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_CASECRC + 3], 0xBE);
	{
		uint8_t zero[TD_HDR_RESERVED_LEN];
		memset(zero, 0, sizeof zero);
		TD_CHECK_MEM(hdr + TD_HDR_OFF_RESERVED, zero, TD_HDR_RESERVED_LEN);
	}

	/* a short build id is zero padded, not left uninitialised */
	{
		struct td_header s = h;
		uint8_t shdr[TD_HEADER_SIZE];
		memset(s.build_id, 0, sizeof s.build_id);
		memcpy(s.build_id, "ab", 2);
		memset(shdr, 0xFF, sizeof shdr);
		td_header_write(shdr, &s);
		TD_CHECK_MEM(shdr + TD_HDR_OFF_BUILDID, "ab\0\0", 4);
	}

	memset(&out, 0, sizeof out);
	rc = td_header_parse(hdr, sizeof hdr, &out);
	TD_CHECK_EQ(rc, 0);
	TD_CHECK_EQ(out.version, TD_FORMAT_VERSION);
	TD_CHECK_EQ(out.area, TD_AREA_VDI);
	TD_CHECK_EQ(out.record_size, TD_VDI_RECORD_SIZE);
	TD_CHECK_EQ(out.record_count, 7);
	TD_CHECK_EQ(out.case_crc, 0xCAFEBABEuL);
	TD_CHECK(strcmp(out.build_id, "19f7") == 0);

	/* rejections */
	{
		uint8_t bad[TD_HEADER_SIZE];

		memcpy(bad, hdr, sizeof bad);
		bad[0] = 'X';
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_MAGIC);

		memcpy(bad, hdr, sizeof bad);
		bad[TD_HDR_OFF_VERSION + 1] = 1;   /* format 1 had no d0 field */
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_VERSION);

		memcpy(bad, hdr, sizeof bad);
		bad[TD_HDR_OFF_RESERVED + 5] = 1;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_RESERVED);

		memcpy(bad, hdr, sizeof bad);
		bad[TD_HDR_OFF_AREA + 1] = 9;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_AREA);

		/* record size must match what the area mandates */
		memcpy(bad, hdr, sizeof bad);
		bad[TD_HDR_OFF_RECSIZE + 1] = 0x39;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_RECSIZE);

		TD_CHECK_EQ(td_header_parse(hdr, TD_HEADER_SIZE - 1, &out), TD_ERR_SIZE);
	}

	TD_CHECK_EQ(td_area_record_size(TD_AREA_VDI), TD_VDI_RECORD_SIZE);
	TD_CHECK_EQ(td_area_record_size(TD_AREA_AES), TD_AES_RECORD_SIZE);
	TD_CHECK_EQ(td_area_record_size(TD_AREA_BIOS), TD_CALL_RECORD_SIZE);
	TD_CHECK_EQ(td_area_record_size(TD_AREA_XBIOS), TD_CALL_RECORD_SIZE);
	TD_CHECK_EQ(td_area_record_size(TD_AREA_GEMDOS), TD_CALL_RECORD_SIZE);
	TD_CHECK_EQ(td_area_record_size(TD_AREA_RASTER), TD_RAST_RECORD_SIZE);
	TD_CHECK_EQ(td_area_record_size(0), 0);
	TD_CHECK_EQ(td_area_record_size(7), 0);
}

static void test_poison(void)
{
	struct td_vdi_arrays v;
	struct td_aes_arrays a;
	struct td_call_frame c;
	int i;

	printf("poison pattern (spec section 4)\n");

	memset(&v, 0, sizeof v);
	td_poison_vdi(&v);
	for (i = 0; i < TD_VDI_CONTRL_N; i++)
		TD_CHECK_EQ((uint16_t)v.contrl[i], 0xDEADu);
	TD_CHECK_EQ((uint16_t)v.intin[TD_VDI_INTIN_N - 1], 0xDEADu);
	TD_CHECK_EQ((uint16_t)v.ptsin[TD_VDI_PTSIN_N - 1], 0xDEADu);
	TD_CHECK_EQ((uint16_t)v.intout[0], 0xDEADu);
	TD_CHECK_EQ((uint16_t)v.ptsout[TD_VDI_PTSOUT_N - 1], 0xDEADu);

	memset(&a, 0, sizeof a);
	td_poison_aes(&a);
	TD_CHECK_EQ((uint16_t)a.contrl[TD_AES_CONTRL_N - 1], 0xDEADu);
	TD_CHECK_EQ((uint16_t)a.global[TD_AES_GLOBAL_N - 1], 0xDEADu);
	TD_CHECK_EQ((uint16_t)a.int_in[TD_AES_INTIN_N - 1], 0xDEADu);
	TD_CHECK_EQ((uint16_t)a.int_out[TD_AES_INTOUT_N - 1], 0xDEADu);
	TD_CHECK_EQ(a.addr_in[TD_AES_ADDRIN_N - 1], 0xDEADDEADuL);
	TD_CHECK_EQ(a.addr_out[0], 0xDEADDEADuL);

	memset(&c, 0, sizeof c);
	td_poison_call(&c);
	TD_CHECK_EQ(c.args[TD_CALL_ARGS_N - 1], 0xDEADDEADuL);
	TD_CHECK_EQ((uint32_t)c.ret, 0xDEADDEADuL);
	TD_CHECK_EQ(c.capture[0], 0xDE);
	TD_CHECK_EQ(c.capture[1], 0xAD);
	TD_CHECK_EQ(c.capture[TD_CALL_CAPTURE_LEN - 2], 0xDE);
	TD_CHECK_EQ(c.capture[TD_CALL_CAPTURE_LEN - 1], 0xAD);
}

static void test_vdi_record(void)
{
	struct td_vdi_arrays a;
	int16_t before[TD_VDI_CONTRL_N];
	uint8_t rec[TD_VDI_RECORD_SIZE];
	int i;

	printf("VDI record layout\n");

	td_poison_vdi(&a);
	for (i = 0; i < TD_VDI_CONTRL_N; i++)
		before[i] = (int16_t)(0x0100 + i);

	a.contrl[0] = 6;
	a.contrl[1] = 2;
	a.intin[0]  = 0x0102;
	a.ptsin[0]  = 0x0304;
	a.intout[3] = 0x1234;
	a.ptsout[1] = -2;

	memset(rec, 0xFF, sizeof rec);
	td_vdi_record_write(rec, 42, 6, 1, &a, before);

	/* 42 == 0x002A */
	TD_CHECK_EQ(rec[TD_VDI_OFF_CASEID], 0x00);
	TD_CHECK_EQ(rec[TD_VDI_OFF_CASEID + 1], 0x2A);
	TD_CHECK_EQ(rec[TD_VDI_OFF_OPCODE], 0x00);
	TD_CHECK_EQ(rec[TD_VDI_OFF_OPCODE + 1], 0x06);

	/* the trap's d0 return, here flip_y = 1 */
	TD_CHECK_MEM(rec + TD_VDI_OFF_RET, "\x00\x00\x00\x01", 4);

	TD_CHECK_MEM(rec + TD_VDI_OFF_CONTRL, "\x00\x06\x00\x02", 4);
	TD_CHECK_MEM(rec + TD_VDI_OFF_INTIN, "\x01\x02", 2);
	TD_CHECK_MEM(rec + TD_VDI_OFF_INTIN + 2, "\xDE\xAD", 2);
	TD_CHECK_MEM(rec + TD_VDI_OFF_PTSIN, "\x03\x04", 2);
	TD_CHECK_MEM(rec + TD_VDI_OFF_INTOUT + 3 * 2, "\x12\x34", 2);
	TD_CHECK_MEM(rec + TD_VDI_OFF_PTSOUT + 1 * 2, "\xFF\xFE", 2);
	/* contrl before the call: 0x0100, 0x0101, ... */
	TD_CHECK_MEM(rec + TD_VDI_OFF_CONTRL_BEFORE, "\x01\x00\x01\x01", 4);
	TD_CHECK_MEM(rec + TD_VDI_OFF_CONTRL_BEFORE + (TD_VDI_CONTRL_N - 1) * 2,
	             "\x01\x0B", 2);

	/* last byte of the record is the last byte of contrl-before */
	TD_CHECK_EQ(TD_VDI_OFF_CONTRL_BEFORE + TD_VDI_CONTRL_N * 2,
	            TD_VDI_RECORD_SIZE);
}

static void test_aes_record(void)
{
	struct td_aes_arrays a;
	uint8_t rec[TD_AES_RECORD_SIZE];
	uint8_t zero[TD_AES_RESERVED_LEN];

	printf("AES record layout\n");

	td_poison_aes(&a);
	a.contrl[0]   = 10;      /* opcode */
	a.global[0]   = 0x0140;
	a.int_in[0]   = 1;
	a.int_out[0]  = 1;
	a.addr_in[0]  = 0x00012345uL;
	a.addr_out[0] = 0x0009ABCDuL;

	memset(rec, 0xFF, sizeof rec);
	td_aes_record_write(rec, 3, 10, -1, &a);

	TD_CHECK_MEM(rec + TD_AES_OFF_CASEID, "\x00\x03", 2);
	TD_CHECK_MEM(rec + TD_AES_OFF_OPCODE, "\x00\x0A", 2);
	TD_CHECK_MEM(rec + TD_AES_OFF_CONTRL, "\x00\x0A", 2);
	TD_CHECK_MEM(rec + TD_AES_OFF_GLOBAL, "\x01\x40", 2);
	TD_CHECK_MEM(rec + TD_AES_OFF_INTIN, "\x00\x01", 2);
	TD_CHECK_MEM(rec + TD_AES_OFF_INTOUT, "\x00\x01", 2);
	TD_CHECK_MEM(rec + TD_AES_OFF_ADDRIN, "\x00\x01\x23\x45", 4);
	TD_CHECK_MEM(rec + TD_AES_OFF_ADDRIN + 4, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_MEM(rec + TD_AES_OFF_ADDROUT, "\x00\x09\xAB\xCD", 4);

	TD_CHECK_MEM(rec + TD_AES_OFF_RET, "\xFF\xFF\xFF\xFF", 4);
	memset(zero, 0, sizeof zero);
	TD_CHECK_MEM(rec + TD_AES_OFF_RESERVED, zero, TD_AES_RESERVED_LEN);
	TD_CHECK_EQ(TD_AES_OFF_RESERVED + TD_AES_RESERVED_LEN, TD_AES_RECORD_SIZE);
}

static void test_call_record(void)
{
	struct td_call_frame f;
	uint8_t rec[TD_CALL_RECORD_SIZE];
	uint8_t zero[TD_CALL_RESERVED_LEN];

	printf("BIOS/XBIOS/GEMDOS record layout\n");

	td_poison_call(&f);
	f.args[0] = 0x00000002uL;
	f.args[1] = 0x000A0B0CuL;
	f.ret     = -33;                    /* EFILNF */
	memcpy(f.capture, "TOSDUMP", 7);

	memset(rec, 0xFF, sizeof rec);
	td_call_record_write(rec, 5, 0x3D, &f);

	TD_CHECK_MEM(rec + TD_CALL_OFF_CASEID, "\x00\x05", 2);
	TD_CHECK_MEM(rec + TD_CALL_OFF_OPCODE, "\x00\x3D", 2);
	TD_CHECK_MEM(rec + TD_CALL_OFF_ARGS, "\x00\x00\x00\x02", 4);
	TD_CHECK_MEM(rec + TD_CALL_OFF_ARGS + 4, "\x00\x0A\x0B\x0C", 4);
	TD_CHECK_MEM(rec + TD_CALL_OFF_ARGS + 8, "\xDE\xAD\xDE\xAD", 4);
	TD_CHECK_MEM(rec + TD_CALL_OFF_RET, "\xFF\xFF\xFF\xDF", 4);
	TD_CHECK_MEM(rec + TD_CALL_OFF_CAPTURE, "TOSDUMP", 7);
	TD_CHECK_MEM(rec + TD_CALL_OFF_CAPTURE + 8, "\xDE\xAD", 2);

	memset(zero, 0, sizeof zero);
	TD_CHECK_MEM(rec + TD_CALL_OFF_RESERVED, zero, TD_CALL_RESERVED_LEN);
	TD_CHECK_EQ(TD_CALL_OFF_RESERVED + TD_CALL_RESERVED_LEN, TD_CALL_RECORD_SIZE);
}

static void test_crc32(void)
{
	printf("CRC32 (case table identity)\n");

	/* IEEE 802.3 CRC-32, the value every crc32 tool prints for "123456789" */
	TD_CHECK_EQ(td_crc32((const uint8_t *)"123456789", 9), 0xCBF43926uL);
	TD_CHECK_EQ(td_crc32((const uint8_t *)"", 0), 0x00000000uL);
	TD_CHECK_EQ(td_crc32((const uint8_t *)"a", 1), 0xE8B7BE43uL);
}

static void test_field_tables(void)
{
	const struct td_field *f;
	int n;

	printf("field descriptor tables\n");

	f = td_area_fields(TD_AREA_VDI);
	TD_CHECK(f != NULL);
	for (n = 0; f && f[n].name; n++)
		;
	/* d0, contrl, intin, ptsin, intout, ptsout, contrl_before.
	 *
	 * d0 is compared as a WORD, not a LONG. The VDI returns flip_y in the
	 * low half (EmuTOS vdi_asm.S:124-125 does `moveq #0,d0` and only then
	 * `move.w _flip_y,d0`); TOS 3.06 does not clear the high half, so it
	 * carries whatever the call path left there -- measured as 0x0111 for
	 * the inquiries and setters and 0xFFFF after v_pline/v_fillarea. The
	 * whole long stays in the record as evidence; only the half the
	 * contract defines is compared. The field descriptor was wrong, not
	 * the case. */
	TD_CHECK_EQ(n, 7);
	TD_CHECK(f && strcmp(f[0].name, "d0") == 0);
	TD_CHECK(f && f[0].off == TD_VDI_OFF_RET + 2);
	TD_CHECK(f && f[0].width == 2);
	TD_CHECK(f && f[0].count == 1);
	TD_CHECK(f && strcmp(f[1].name, "contrl") == 0);
	TD_CHECK(f && f[1].off == TD_VDI_OFF_CONTRL);
	TD_CHECK(f && strcmp(f[4].name, "intout") == 0);
	TD_CHECK(f && f[4].off == TD_VDI_OFF_INTOUT);
	TD_CHECK(f && f[4].count == TD_VDI_INTOUT_N);
	TD_CHECK(f && f[4].width == 2);

	f = td_area_fields(TD_AREA_AES);
	TD_CHECK(f != NULL);
	for (n = 0; f && f[n].name; n++)
		;
	/* d0, contrl, global[0..10], global[11..12], global[13..14], int_in,
	 * int_out, addr_in, addr_out.
	 *
	 * global[11..12] is ap_3resv, which EmuTOS fills with the address of
	 * its own AES global area (aes/gemsuper.c:98): field class address.
	 * Two builds of the same EmuTOS source with different toolchains put
	 * it at 0xBE04 and 0xBD5C
	 * (measurements/measurement-2026-10-26-oracle/RUNINFO.TXT). It is
	 * split off so the rest of global keeps being compared, and keeps its
	 * own index in a report. */
	TD_CHECK_EQ(n, 9);
	TD_CHECK(f && strcmp(f[0].name, "d0") == 0);
	TD_CHECK(f && f[0].off == TD_AES_OFF_RET + 2);
	TD_CHECK(f && f[0].width == 2);
	TD_CHECK(f && strcmp(f[2].name, "global") == 0);
	TD_CHECK(f && f[2].off == TD_AES_OFF_GLOBAL);
	TD_CHECK(f && f[2].count == 11 && f[2].first == 0);
	TD_CHECK(f && f[2].compare == TD_FIELD_VALUE);
	TD_CHECK(f && strcmp(f[3].name, "global") == 0);
	TD_CHECK(f && f[3].off == TD_AES_OFF_GLOBAL + 22);
	TD_CHECK(f && f[3].count == 2 && f[3].first == 11);
	TD_CHECK(f && f[3].compare == TD_FIELD_ADDRESS);
	TD_CHECK(f && strcmp(f[4].name, "global") == 0);
	TD_CHECK(f && f[4].off == TD_AES_OFF_GLOBAL + 26);
	TD_CHECK(f && f[4].count == 2 && f[4].first == 13);
	TD_CHECK(f && f[4].compare == TD_FIELD_VALUE);
	/* the three pieces cover global[0..14] exactly */
	TD_CHECK(f && f[2].count + f[3].count + f[4].count == TD_AES_GLOBAL_N);

	f = td_area_fields(TD_AREA_GEMDOS);
	TD_CHECK(f != NULL);
	TD_CHECK(td_area_fields(TD_AREA_BIOS) == f);
	TD_CHECK(td_area_fields(TD_AREA_XBIOS) == f);
	TD_CHECK(td_area_fields(0) == NULL);

	/*
	 * The raster area.  Two of its fields are recorded and never compared:
	 * contrl[7..14] hold the two MFDB pointers, at the target's own pointer
	 * width and pointing at the dumper's own buffers, so they cannot be
	 * equal across the two machines and a difference there would measure
	 * the linker.  Every other field is compared like any other.
	 */
	f = td_area_fields(TD_AREA_RASTER);
	TD_CHECK(f != NULL);
	for (n = 0; f && f[n].name; n++) {
		int is_ptr = strstr(f[n].name, "mfdb") != NULL &&
		             strncmp(f[n].name, "contrl", 6) == 0;
		TD_CHECK_EQ(f[n].compare,
		            is_ptr ? TD_FIELD_ADDRESS : TD_FIELD_VALUE);
	}
	TD_CHECK_EQ(n, 13);
	TD_CHECK(f && strcmp(f[0].name, "d0") == 0);
	TD_CHECK(f && f[1].count == 7);      /* contrl[0..6] only */
	TD_CHECK(f && f[2].off == TD_RAST_OFF_CONTRL + 14);
	TD_CHECK(f && f[2].count == TD_RAST_CONTRL_PTR_N);

	/* every other field is compared by value; the AES has exactly one
	 * address field (ap_3resv, above) */
	for (n = 0; td_vdi_fields[n].name; n++)
		TD_CHECK_EQ(td_vdi_fields[n].compare, TD_FIELD_VALUE);
	for (n = 0; td_aes_fields[n].name; n++)
		TD_CHECK_EQ(td_aes_fields[n].compare,
		            n == 3 ? TD_FIELD_ADDRESS : TD_FIELD_VALUE);
	for (n = 0; td_call_fields[n].name; n++)
		TD_CHECK_EQ(td_call_fields[n].compare, TD_FIELD_VALUE);
}

/*
 * The raster record.  Its offsets are stated here, not read from tosdump.h:
 * a record layout that agreed with itself would prove nothing, and the two
 * machines only agree because both compile the same numbers.
 */
static void test_raster_record(void)
{
	struct td_raster_arrays a;
	int16_t before[TD_RAST_CONTRL_N];
	int16_t src[TD_RAST_MFDB_N], dst[TD_RAST_MFDB_N];
	uint8_t rec[TD_RAST_RECORD_SIZE];
	int i;

	printf("VDI raster record layout\n");

	TD_CHECK_EQ(TD_RAST_RECORD_SIZE, 3696);
	TD_CHECK_EQ(TD_RAST_OFF_CONTRL, 8);
	TD_CHECK_EQ(TD_RAST_OFF_CONTRL_BEFORE, 8 + TD_RAST_CONTRL_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_INTIN, 38 + TD_RAST_CONTRL_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_PTSIN, TD_RAST_OFF_INTIN + TD_RAST_INTIN_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_INTOUT, TD_RAST_OFF_PTSIN + TD_RAST_PTSIN_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_PTSOUT, TD_RAST_OFF_INTOUT + TD_RAST_INTOUT_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_SRC_MFDB,
	            TD_RAST_OFF_PTSOUT + TD_RAST_PTSOUT_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_DST_MFDB,
	            TD_RAST_OFF_SRC_MFDB + TD_RAST_MFDB_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_BUF_A,
	            TD_RAST_OFF_DST_MFDB + TD_RAST_MFDB_N * 2);
	TD_CHECK_EQ(TD_RAST_OFF_BUF_B,
	            TD_RAST_OFF_BUF_A + TD_RAST_BUF_WORDS * 2);
	TD_CHECK_EQ(TD_RAST_OFF_RESERVED,
	            TD_RAST_OFF_BUF_B + TD_RAST_BUF_WORDS * 2);
	TD_CHECK_EQ(TD_RAST_OFF_RESERVED + TD_RAST_RESERVED_LEN,
	            TD_RAST_RECORD_SIZE);
	/* an AArch64 caller packs the second MFDB pointer at contrl[11..14] */
	TD_CHECK(TD_RAST_CONTRL_N >= 7 + 2 * 4);

	td_poison_raster(&a);
	/* the parameter arrays are poisoned ... */
	TD_CHECK_EQ((uint16_t)a.contrl[TD_RAST_CONTRL_N - 1],
	            (uint16_t)TD_POISON16);
	TD_CHECK_EQ((uint16_t)a.intout[0], (uint16_t)TD_POISON16);
	TD_CHECK_EQ((uint16_t)a.ptsout[TD_RAST_PTSOUT_N - 1],
	            (uint16_t)TD_POISON16);

	/* ... the pixel buffers are not: they are laid with a per-seed
	 * pattern, because a copy out of an all-0xDEAD source proves nothing
	 * about where the bits went */
	td_rast_fill(a.buf_a, TD_RAST_BUF_WORDS, 1);
	td_rast_fill(a.buf_b, TD_RAST_BUF_WORDS, 2);
	TD_CHECK(a.buf_a[0] != a.buf_a[1]);
	TD_CHECK(a.buf_a[0] != a.buf_b[0]);
	TD_CHECK_EQ((uint16_t)a.buf_a[5], td_rast_fill_word(1, 5));

	for (i = 0; i < TD_RAST_CONTRL_N; i++) {
		a.contrl[i] = (int16_t)(0x100 + i);
		before[i]   = (int16_t)(0x200 + i);
	}
	for (i = 0; i < TD_RAST_MFDB_N; i++) {
		src[i] = (int16_t)(0x300 + i);
		dst[i] = (int16_t)(0x400 + i);
	}
	td_raster_record_write(rec, 42, 109, -1, &a, before, src, dst);

	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_CASEID), 42);
	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_OPCODE), 109);
	TD_CHECK_EQ(td_load_be32(rec + TD_RAST_OFF_RET), 0xFFFFFFFFuL);
	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_CONTRL + 28), 0x10E);
	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_CONTRL_BEFORE), 0x200);
	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_SRC_MFDB + 6), 0x303);
	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_DST_MFDB + 6), 0x403);
	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_BUF_A + 10),
	            td_rast_fill_word(1, 5));
	TD_CHECK_EQ(td_load_be16(rec + TD_RAST_OFF_BUF_B + 10),
	            td_rast_fill_word(2, 5));
	for (i = 0; i < TD_RAST_RESERVED_LEN; i++)
		TD_CHECK_EQ(rec[TD_RAST_OFF_RESERVED + i], 0);
}

int main(void)
{
	test_store_load();
	test_header();
	test_poison();
	test_vdi_record();
	test_aes_record();
	test_call_record();
	test_crc32();
	test_raster_record();
	test_field_tables();
	return TD_TEST_RESULT();
}
