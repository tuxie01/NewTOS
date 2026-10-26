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

	printf("file header (format 3)\n");

	memset(&h, 0, sizeof h);
	h.version      = TD_FORMAT_VERSION;
	h.area         = TD_AREA_VDI;
	h.record_size  = TD_VDI_RECORD_SIZE;
	h.record_count = 7;
	memcpy(h.build_id, "19f7", 4);
	h.build_id[4]  = '\0';
	h.case_crc     = 0xCAFEBABEuL;
	h.ptr_width    = 4;
	h.word_order   = TD_WORD_ORDER_MSW_FIRST;

	memset(hdr, 0xFF, sizeof hdr);
	td_header_write(hdr, &h);

	TD_CHECK_MEM(hdr + TD_HDR_OFF_MAGIC, "TDMP", 4);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_VERSION], 0x00);
	TD_CHECK_EQ(hdr[TD_HDR_OFF_VERSION + 1], 0x03);
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
	/* format 3: the pointer width and the word order, big-endian WORDs at
	 * 20 and 22, where format 2 had the first four reserved bytes */
	TD_CHECK_EQ(TD_HDR_OFF_PTRWIDTH, 20);
	TD_CHECK_EQ(TD_HDR_OFF_WORDORDER, 22);
	TD_CHECK_MEM(hdr + 20, "\x00\x04", 2);
	TD_CHECK_MEM(hdr + 22, "\x00\x01", 2);
	TD_CHECK_EQ(TD_HDR_OFF_RESERVED, 24);
	TD_CHECK_EQ(TD_HDR_RESERVED_LEN, 8);
	TD_CHECK_EQ(TD_HDR_OFF_RESERVED + TD_HDR_RESERVED_LEN, TD_HEADER_SIZE);
	{
		uint8_t zero[TD_HDR_RESERVED_LEN];
		memset(zero, 0, sizeof zero);
		TD_CHECK_MEM(hdr + TD_HDR_OFF_RESERVED, zero, TD_HDR_RESERVED_LEN);
	}

	/* the other two values: an AArch64 dumper */
	{
		struct td_header s = h;
		uint8_t shdr[TD_HEADER_SIZE];
		s.ptr_width  = 8;
		s.word_order = TD_WORD_ORDER_LSW_FIRST;
		td_header_write(shdr, &s);
		TD_CHECK_MEM(shdr + 20, "\x00\x08\x00\x02", 4);
		TD_CHECK_EQ(td_header_parse(shdr, sizeof shdr, &out), 0);
		TD_CHECK_EQ(out.ptr_width, 8);
		TD_CHECK_EQ(out.word_order, TD_WORD_ORDER_LSW_FIRST);
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
	TD_CHECK_EQ(out.version, 3);
	TD_CHECK_EQ(out.area, TD_AREA_VDI);
	TD_CHECK_EQ(out.record_size, TD_VDI_RECORD_SIZE);
	TD_CHECK_EQ(out.record_count, 7);
	TD_CHECK_EQ(out.case_crc, 0xCAFEBABEuL);
	TD_CHECK(strcmp(out.build_id, "19f7") == 0);
	TD_CHECK_EQ(out.ptr_width, 4);
	TD_CHECK_EQ(out.word_order, TD_WORD_ORDER_MSW_FIRST);

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
		bad[TD_HDR_OFF_VERSION + 1] = 4;   /* a format nobody wrote yet */
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_VERSION);

		memcpy(bad, hdr, sizeof bad);
		bad[TD_HDR_OFF_RESERVED + 5] = 1;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_RESERVED);

		/* a pointer is four or eight bytes wide; anything else, and a
		 * zero in particular, is a header that does not say */
		memcpy(bad, hdr, sizeof bad);
		bad[TD_HDR_OFF_PTRWIDTH + 1] = 0;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_PTRINFO);
		bad[TD_HDR_OFF_PTRWIDTH + 1] = 6;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_PTRINFO);
		bad[TD_HDR_OFF_PTRWIDTH]     = 1;
		bad[TD_HDR_OFF_PTRWIDTH + 1] = 4;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_PTRINFO);

		memcpy(bad, hdr, sizeof bad);
		bad[TD_HDR_OFF_WORDORDER + 1] = 0;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_PTRINFO);
		bad[TD_HDR_OFF_WORDORDER + 1] = 3;
		TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_PTRINFO);

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

/*
 * A format 2 header stays readable.  Written out byte for byte -- the header
 * of tosdump/reference/EMUTOS-1a7484ab-AES.BIN as it was recorded -- so the
 * test cannot agree with a writer that no longer produces format 2.
 */
static void test_header_v2(void)
{
	static const uint8_t v2[TD_HEADER_SIZE] = {
		'T', 'D', 'M', 'P', 0x00, 0x02, 0x00, 0x05,
		0x00, 0x78, 0x00, 0x5F, 'b', '5', '9', 'e',
		0x33, 0xC1, 0x3C, 0x8F, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0
	};
	uint8_t bad[TD_HEADER_SIZE];
	struct td_header out;

	printf("file header (format 2 stays readable)\n");

	memset(&out, 0xFF, sizeof out);
	TD_CHECK_EQ(td_header_parse(v2, sizeof v2, &out), 0);
	TD_CHECK_EQ(out.version, 2);
	TD_CHECK_EQ(out.area, TD_AREA_AES);
	TD_CHECK_EQ(out.record_size, TD_AES_RECORD_SIZE);
	TD_CHECK_EQ(out.record_count, 95);
	TD_CHECK(strcmp(out.build_id, "b59e") == 0);
	TD_CHECK_EQ(out.case_crc, 0x33C13C8FuL);
	/* format 2 does not say: zero, never a guess */
	TD_CHECK_EQ(out.ptr_width, 0);
	TD_CHECK_EQ(out.word_order, TD_WORD_ORDER_UNKNOWN);
	TD_CHECK_EQ(td_header_states_ptr(&out), 0);

	/* in format 2 all twelve bytes from 20 on are reserved, so a v2
	 * header that carries a pointer width is malformed, not a v3 */
	memcpy(bad, v2, sizeof bad);
	bad[TD_HDR_OFF_PTRWIDTH + 1] = 4;
	TD_CHECK_EQ(td_header_parse(bad, sizeof bad, &out), TD_ERR_RESERVED);

	/* the writer still produces format 2 when asked to (tests forge
	 * one), and then writes no pointer information */
	{
		struct td_header h;
		uint8_t hdr[TD_HEADER_SIZE];

		memset(&h, 0, sizeof h);
		h.version      = 2;
		h.area         = TD_AREA_AES;
		h.record_size  = TD_AES_RECORD_SIZE;
		h.record_count = 95;
		memcpy(h.build_id, "b59e", 4);
		h.case_crc     = 0x33C13C8FuL;
		h.ptr_width    = 8;
		h.word_order   = TD_WORD_ORDER_LSW_FIRST;
		memset(hdr, 0xFF, sizeof hdr);
		td_header_write(hdr, &h);
		TD_CHECK_MEM(hdr, v2, TD_HEADER_SIZE);
	}

	{
		struct td_header h3;
		memset(&h3, 0, sizeof h3);
		h3.version    = 3;
		h3.ptr_width  = 4;
		h3.word_order = TD_WORD_ORDER_MSW_FIRST;
		TD_CHECK_EQ(td_header_states_ptr(&h3), 1);
	}
}

/*
 * What the dumper puts into the header is measured on the machine that runs
 * it, not configured: the pointer width is sizeof(void *) and the word order
 * is where the high WORD of a LONG lands in memory.  Checked here against the
 * compiler's own predefined byte order, an independent statement of the same
 * fact.
 */
static void test_native_ptr_info(void)
{
	printf("native pointer width and word order\n");

	TD_CHECK_EQ(td_native_ptr_width(), sizeof(void *));
#if defined(__BYTE_ORDER__) && defined(__ORDER_LITTLE_ENDIAN__) && \
    (__BYTE_ORDER__ == __ORDER_LITTLE_ENDIAN__)
	TD_CHECK_EQ(td_native_word_order(), TD_WORD_ORDER_LSW_FIRST);
#elif defined(__BYTE_ORDER__) && defined(__ORDER_BIG_ENDIAN__) && \
    (__BYTE_ORDER__ == __ORDER_BIG_ENDIAN__)
	TD_CHECK_EQ(td_native_word_order(), TD_WORD_ORDER_MSW_FIRST);
#else
	TD_CHECK(!"host byte order unknown to the compiler");
#endif
}

/* Lay `n` WORDs into a record at a field's offset, big-endian as the record
 * format stores every WORD. */
static void put_words(uint8_t *rec, const struct td_field *f, uint16_t at,
                      const uint16_t *w, int n)
{
	int i;
	for (i = 0; i < n; i++)
		td_store_be16(rec + f->off + (at + i) * 2, w[i]);
}

static struct td_header hdr_of(uint16_t version, uint16_t ptr_width,
                               uint16_t word_order)
{
	struct td_header h;
	memset(&h, 0, sizeof h);
	h.version    = version;
	h.ptr_width  = ptr_width;
	h.word_order = word_order;
	return h;
}

/*
 * The address field class: cells, their value, and the relational property
 * every cell must hold -- a TOS address is a signed 32-bit LONG, so one at or
 * above 0x80000000 reads as an error code (TASK/MEM11.md).
 */
static void test_addr_cells(void)
{
	const struct td_field *ap3 = &td_aes_fields[3];      /* global[11..12] */
	const struct td_field *mfdb = &td_raster_fields[2];  /* contrl[7..14]  */
	struct td_header m68k = hdr_of(3, 4, TD_WORD_ORDER_MSW_FIRST);
	struct td_header a64  = hdr_of(3, 8, TD_WORD_ORDER_LSW_FIRST);
	struct td_header v2   = hdr_of(2, 0, TD_WORD_ORDER_UNKNOWN);
	uint8_t arec[TD_AES_RECORD_SIZE];
	uint8_t rrec[TD_RAST_RECORD_SIZE];

	printf("address field class: cells and the < 0x80000000 property\n");

	TD_CHECK_EQ(TD_ADDR_LIMIT, 0x80000000uL);
	TD_CHECK(td_addr_ok(0));
	TD_CHECK(td_addr_ok(0x7FFFFFFFuL));
	TD_CHECK(!td_addr_ok(0x80000000uL));
	TD_CHECK(!td_addr_ok(0xDEADDEADuL));
	TD_CHECK(!td_addr_ok((uint64_t)1 << 32));

	/* ap_3resv: a LONG by the AESGLOBAL layout, one cell of two WORDs on
	 * every target -- also on AArch64, where NewTOS writes four bytes */
	TD_CHECK(strcmp(ap3->name, "global") == 0 && ap3->first == 11);
	TD_CHECK_EQ(ap3->addr_cell, TD_ADDR_CELL_LONG);
	TD_CHECK_EQ(td_addr_cell_count(ap3, &m68k), 1);
	TD_CHECK_EQ(td_addr_cell_words(ap3, &m68k), 2);
	TD_CHECK_EQ(td_addr_cell_count(ap3, &a64), 1);
	TD_CHECK_EQ(td_addr_cell_words(ap3, &a64), 2);
	/* format 2: not checkable, no cell is read */
	TD_CHECK_EQ(td_addr_cell_count(ap3, &v2), 0);

	memset(arec, 0, sizeof arec);
	{
		/* most significant WORD first (m68k): EmuTOS' &D as recorded
		 * in tosdump/reference/EMUTOS-1a7484ab-AES.BIN */
		static const uint16_t emutos[2] = { 0x0000, 0xBD60 };
		static const uint16_t max_m[2]  = { 0x7FFF, 0xFFFF };
		static const uint16_t lim_m[2]  = { 0x8000, 0x0000 };
		/* least significant WORD first (AArch64): NewTOS' truncated
		 * pointer as recorded in tosdump/reference/NEWTOS-AES.BIN */
		static const uint16_t newtos[2] = { 0x80B0, 0x0011 };
		static const uint16_t max_l[2]  = { 0xFFFF, 0x7FFF };
		static const uint16_t lim_l[2]  = { 0x0000, 0x8000 };

		put_words(arec, ap3, 0, emutos, 2);
		TD_CHECK_EQ(td_addr_cell_value(arec, ap3, &m68k, 0), 0x0000BD60uL);
		TD_CHECK(td_addr_ok(td_addr_cell_value(arec, ap3, &m68k, 0)));
		put_words(arec, ap3, 0, max_m, 2);
		TD_CHECK_EQ(td_addr_cell_value(arec, ap3, &m68k, 0), 0x7FFFFFFFuL);
		TD_CHECK(td_addr_ok(td_addr_cell_value(arec, ap3, &m68k, 0)));
		put_words(arec, ap3, 0, lim_m, 2);
		TD_CHECK_EQ(td_addr_cell_value(arec, ap3, &m68k, 0), 0x80000000uL);
		TD_CHECK(!td_addr_ok(td_addr_cell_value(arec, ap3, &m68k, 0)));

		put_words(arec, ap3, 0, newtos, 2);
		TD_CHECK_EQ(td_addr_cell_value(arec, ap3, &a64, 0), 0x001180B0uL);
		TD_CHECK(td_addr_ok(td_addr_cell_value(arec, ap3, &a64, 0)));
		put_words(arec, ap3, 0, max_l, 2);
		TD_CHECK_EQ(td_addr_cell_value(arec, ap3, &a64, 0), 0x7FFFFFFFuL);
		TD_CHECK(td_addr_ok(td_addr_cell_value(arec, ap3, &a64, 0)));
		put_words(arec, ap3, 0, lim_l, 2);
		TD_CHECK_EQ(td_addr_cell_value(arec, ap3, &a64, 0), 0x80000000uL);
		TD_CHECK(!td_addr_ok(td_addr_cell_value(arec, ap3, &a64, 0)));
	}

	/* the MFDB pointer slots: two cells of the TARGET's pointer width,
	 * packed from contrl[7] as gemlib packs them (gem_vdiP.h:177,204);
	 * the field is sized for the widest pointer, eight bytes */
	TD_CHECK_EQ(mfdb->addr_cell, TD_ADDR_CELL_PTR);
	TD_CHECK_EQ(mfdb->count, 2 * TD_ADDR_PTR_WORDS_MAX);
	TD_CHECK_EQ(td_addr_cell_count(mfdb, &m68k), 2);
	TD_CHECK_EQ(td_addr_cell_words(mfdb, &m68k), 2);
	TD_CHECK_EQ(td_addr_cell_count(mfdb, &a64), 2);
	TD_CHECK_EQ(td_addr_cell_words(mfdb, &a64), 4);
	TD_CHECK_EQ(td_addr_cell_count(mfdb, &v2), 0);
	TD_CHECK_EQ(td_addr_cell_count(&td_raster_fields[4], &a64), 2);

	memset(rrec, 0, sizeof rrec);
	{
		/* m68k: contrl[7..8] and [9..10]; [11..14] keep the poison and
		 * are no cell at this width (EMUTOS-1a7484ab-RASTER.BIN) */
		static const uint16_t m[8] = { 0x0103, 0x6690, 0x0103, 0x667C,
		                               0xDEAD, 0xDEAD, 0xDEAD, 0xDEAD };
		/* AArch64: contrl[7..10] and [11..14] (NEWTOS-RASTER.BIN), the
		 * second one moved above 4 GiB */
		static const uint16_t a[8] = { 0xA6B0, 0x0006, 0x0000, 0x0000,
		                               0x0000, 0x0000, 0x0001, 0x0000 };
		static const uint16_t a_max[4] = { 0xFFFF, 0x7FFF, 0x0000, 0x0000 };
		static const uint16_t a_lim[4] = { 0x0000, 0x8000, 0x0000, 0x0000 };

		put_words(rrec, mfdb, 0, m, 8);
		TD_CHECK_EQ(td_addr_cell_value(rrec, mfdb, &m68k, 0), 0x01036690uL);
		TD_CHECK_EQ(td_addr_cell_value(rrec, mfdb, &m68k, 1), 0x0103667CuL);
		TD_CHECK(td_addr_ok(td_addr_cell_value(rrec, mfdb, &m68k, 1)));

		put_words(rrec, mfdb, 0, a, 8);
		TD_CHECK_EQ(td_addr_cell_value(rrec, mfdb, &a64, 0), 0x0006A6B0uL);
		TD_CHECK(td_addr_ok(td_addr_cell_value(rrec, mfdb, &a64, 0)));
		TD_CHECK(td_addr_cell_value(rrec, mfdb, &a64, 1) ==
		         ((uint64_t)1 << 32));
		TD_CHECK(!td_addr_ok(td_addr_cell_value(rrec, mfdb, &a64, 1)));

		put_words(rrec, mfdb, 0, a_max, 4);
		TD_CHECK_EQ(td_addr_cell_value(rrec, mfdb, &a64, 0), 0x7FFFFFFFuL);
		TD_CHECK(td_addr_ok(td_addr_cell_value(rrec, mfdb, &a64, 0)));
		put_words(rrec, mfdb, 0, a_lim, 4);
		TD_CHECK_EQ(td_addr_cell_value(rrec, mfdb, &a64, 0), 0x80000000uL);
		TD_CHECK(!td_addr_ok(td_addr_cell_value(rrec, mfdb, &a64, 0)));
	}

	/* a value field has no address cell, whatever the header says */
	TD_CHECK_EQ(td_addr_cell_count(&td_aes_fields[2], &m68k), 0);
	TD_CHECK_EQ(td_addr_cell_count(&td_raster_fields[1], &a64), 0);
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
	TD_CHECK(f && f[3].addr_cell == TD_ADDR_CELL_LONG);
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
		TD_CHECK_EQ(f[n].addr_cell,
		            is_ptr ? TD_ADDR_CELL_PTR : TD_ADDR_CELL_NONE);
	}
	TD_CHECK_EQ(n, 13);
	TD_CHECK(f && strcmp(f[0].name, "d0") == 0);
	TD_CHECK(f && f[1].count == 7);      /* contrl[0..6] only */
	TD_CHECK(f && f[2].off == TD_RAST_OFF_CONTRL + 14);
	TD_CHECK(f && f[2].count == TD_RAST_CONTRL_PTR_N);

	/* every other field is compared by value; the AES has exactly one
	 * address field (ap_3resv, above) */
	for (n = 0; td_vdi_fields[n].name; n++) {
		TD_CHECK_EQ(td_vdi_fields[n].compare, TD_FIELD_VALUE);
		TD_CHECK_EQ(td_vdi_fields[n].addr_cell, TD_ADDR_CELL_NONE);
	}
	for (n = 0; td_aes_fields[n].name; n++) {
		TD_CHECK_EQ(td_aes_fields[n].compare,
		            n == 3 ? TD_FIELD_ADDRESS : TD_FIELD_VALUE);
		TD_CHECK_EQ(td_aes_fields[n].addr_cell,
		            n == 3 ? TD_ADDR_CELL_LONG : TD_ADDR_CELL_NONE);
	}
	for (n = 0; td_call_fields[n].name; n++) {
		TD_CHECK_EQ(td_call_fields[n].compare, TD_FIELD_VALUE);
		TD_CHECK_EQ(td_call_fields[n].addr_cell, TD_ADDR_CELL_NONE);
	}
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
	test_header_v2();
	test_native_ptr_info();
	test_poison();
	test_vdi_record();
	test_aes_record();
	test_call_record();
	test_crc32();
	test_raster_record();
	test_field_tables();
	test_addr_cells();
	return TD_TEST_RESULT();
}
