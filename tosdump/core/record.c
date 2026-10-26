/*
 * record.c -- big-endian record writer / reader for the TOS reference dumper.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md sections 2.1, 3 and 4.
 *
 * Compiled by all three toolchains (host cc, m68k-atari-mint-gcc,
 * clang/aarch64).  No struct is ever written to the file: every field goes to
 * an explicit offset through td_store_be*, so padding and alignment rules
 * cannot differ between targets.
 */
#include <string.h>

#include "record.h"

/* ------------------------------------------------------------------ */
/* Byte order (spec section 2.1)                                       */
/* ------------------------------------------------------------------ */
/*
 * Section 2.1 frames this as "the AArch64 build byte-swaps on write, the m68k
 * build does not" -- a compile-time selection around a store of the machine's
 * own native word.  The same section also requires a host-side test proving
 * that both configurations emit byte-identical records, which a native-word
 * store cannot satisfy: on any one host only one of the two configurations
 * would be correct.
 *
 * The two requirements together settle the implementation.  Both branches
 * below are written with explicit byte indices, so each is correct on any
 * machine, and the little-endian branch spells out exactly what it models --
 * swap the word, then store it low byte first, which is what a native store on
 * a little-endian machine does.  TD_BIG_ENDIAN_TARGET is still honoured, so
 * the two configurations are really built and really compared; it selects
 * between two spellings that must agree byte for byte.
 */
#ifndef TD_BIG_ENDIAN_TARGET
#  if defined(__BYTE_ORDER__) && defined(__ORDER_BIG_ENDIAN__) && \
      (__BYTE_ORDER__ == __ORDER_BIG_ENDIAN__)
#    define TD_BIG_ENDIAN_TARGET 1
#  else
#    define TD_BIG_ENDIAN_TARGET 0
#  endif
#endif

void td_store_be16(uint8_t *p, uint16_t v)
{
#if TD_BIG_ENDIAN_TARGET
	/* m68k: the record order is already the machine order, no swap. */
	p[0] = (uint8_t)(v >> 8);
	p[1] = (uint8_t)(v & 0xFFu);
#else
	/* AArch64: swap on write, then store low byte first. */
	uint16_t s = (uint16_t)(((v & 0x00FFu) << 8) | ((v & 0xFF00u) >> 8));
	p[0] = (uint8_t)(s & 0xFFu);
	p[1] = (uint8_t)((s >> 8) & 0xFFu);
#endif
}

void td_store_be32(uint8_t *p, uint32_t v)
{
#if TD_BIG_ENDIAN_TARGET
	p[0] = (uint8_t)((v >> 24) & 0xFFuL);
	p[1] = (uint8_t)((v >> 16) & 0xFFuL);
	p[2] = (uint8_t)((v >>  8) & 0xFFuL);
	p[3] = (uint8_t)( v        & 0xFFuL);
#else
	uint32_t s = ((v & 0x000000FFuL) << 24) |
	             ((v & 0x0000FF00uL) <<  8) |
	             ((v & 0x00FF0000uL) >>  8) |
	             ((v & 0xFF000000uL) >> 24);
	p[0] = (uint8_t)( s        & 0xFFuL);
	p[1] = (uint8_t)((s >>  8) & 0xFFuL);
	p[2] = (uint8_t)((s >> 16) & 0xFFuL);
	p[3] = (uint8_t)((s >> 24) & 0xFFuL);
#endif
}

uint16_t td_load_be16(const uint8_t *p)
{
	return (uint16_t)(((uint16_t)p[0] << 8) | (uint16_t)p[1]);
}

uint32_t td_load_be32(const uint8_t *p)
{
	return ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
	       ((uint32_t)p[2] <<  8) |  (uint32_t)p[3];
}

/* ------------------------------------------------------------------ */
/* CRC32 -- IEEE 802.3, reflected, the value crc32(1) prints            */
/* ------------------------------------------------------------------ */

uint32_t td_crc32(const uint8_t *data, size_t len)
{
	uint32_t crc = 0xFFFFFFFFuL;
	size_t i;
	int b;

	for (i = 0; i < len; i++) {
		crc ^= (uint32_t)data[i];
		for (b = 0; b < 8; b++)
			crc = (crc >> 1) ^ (0xEDB88320uL & (uint32_t)(-(int32_t)(crc & 1uL)));
	}
	return crc ^ 0xFFFFFFFFuL;
}

/* ------------------------------------------------------------------ */
/* File header                                                         */
/* ------------------------------------------------------------------ */

uint16_t td_area_record_size(uint16_t area)
{
	switch (area) {
	case TD_AREA_BIOS:
	case TD_AREA_XBIOS:
	case TD_AREA_GEMDOS: return TD_CALL_RECORD_SIZE;
	case TD_AREA_VDI:    return TD_VDI_RECORD_SIZE;
	case TD_AREA_AES:    return TD_AES_RECORD_SIZE;
	case TD_AREA_RASTER: return TD_RAST_RECORD_SIZE;
	default:             return 0;
	}
}

const char *td_area_name(uint16_t area)
{
	switch (area) {
	case TD_AREA_BIOS:   return "BIOS";
	case TD_AREA_XBIOS:  return "XBIOS";
	case TD_AREA_GEMDOS: return "GEMDOS";
	case TD_AREA_VDI:    return "VDI";
	case TD_AREA_AES:    return "AES";
	case TD_AREA_RASTER: return "VDI-RASTER";
	default:             return "?";
	}
}

void td_header_write(uint8_t *hdr, const struct td_header *h)
{
	int i;

	memset(hdr, 0, TD_HEADER_SIZE);
	memcpy(hdr + TD_HDR_OFF_MAGIC, TD_MAGIC, TD_MAGIC_LEN);
	td_store_be16(hdr + TD_HDR_OFF_VERSION,  h->version);
	td_store_be16(hdr + TD_HDR_OFF_AREA,     h->area);
	td_store_be16(hdr + TD_HDR_OFF_RECSIZE,  h->record_size);
	td_store_be16(hdr + TD_HDR_OFF_RECCOUNT, h->record_count);

	/* build id: ASCII, zero padded -- never left uninitialised */
	for (i = 0; i < TD_HDR_BUILDID_LEN; i++) {
		char c = h->build_id[i];
		hdr[TD_HDR_OFF_BUILDID + i] = (uint8_t)c;
		if (c == '\0')
			break;
	}

	td_store_be32(hdr + TD_HDR_OFF_CASECRC, h->case_crc);

	/* Format 3 states the pointer width and the word order; in format 2
	 * the same bytes are reserved and stay zero. */
	if (h->version >= TD_FORMAT_VERSION) {
		td_store_be16(hdr + TD_HDR_OFF_PTRWIDTH,  h->ptr_width);
		td_store_be16(hdr + TD_HDR_OFF_WORDORDER, h->word_order);
	}
	/* reserved stays zero from the memset above */
}

int td_header_parse(const uint8_t *hdr, size_t len, struct td_header *out)
{
	uint16_t mandated;
	int i, res_off, res_len;

	if (len < TD_HEADER_SIZE)
		return TD_ERR_SIZE;
	if (memcmp(hdr + TD_HDR_OFF_MAGIC, TD_MAGIC, TD_MAGIC_LEN) != 0)
		return TD_ERR_MAGIC;

	/* Format 2 stays readable: its records are byte for byte format 3's,
	 * only the header says less. */
	out->version = td_load_be16(hdr + TD_HDR_OFF_VERSION);
	if (out->version == TD_FORMAT_VERSION) {
		res_off = TD_HDR_OFF_RESERVED;
		res_len = TD_HDR_RESERVED_LEN;
	} else if (out->version == TD_FORMAT_VERSION_2) {
		res_off = TD_HDR_V2_OFF_RESERVED;
		res_len = TD_HDR_V2_RESERVED_LEN;
	} else {
		return TD_ERR_VERSION;
	}

	for (i = 0; i < res_len; i++)
		if (hdr[res_off + i] != 0)
			return TD_ERR_RESERVED;

	if (out->version == TD_FORMAT_VERSION) {
		out->ptr_width  = td_load_be16(hdr + TD_HDR_OFF_PTRWIDTH);
		out->word_order = td_load_be16(hdr + TD_HDR_OFF_WORDORDER);
		if (out->ptr_width != 4 && out->ptr_width != 8)
			return TD_ERR_PTRINFO;
		if (out->word_order != TD_WORD_ORDER_MSW_FIRST &&
		    out->word_order != TD_WORD_ORDER_LSW_FIRST)
			return TD_ERR_PTRINFO;
	} else {
		out->ptr_width  = 0;
		out->word_order = TD_WORD_ORDER_UNKNOWN;
	}

	out->area = td_load_be16(hdr + TD_HDR_OFF_AREA);
	mandated = td_area_record_size(out->area);
	if (mandated == 0)
		return TD_ERR_AREA;

	out->record_size = td_load_be16(hdr + TD_HDR_OFF_RECSIZE);
	if (out->record_size != mandated)
		return TD_ERR_RECSIZE;

	out->record_count = td_load_be16(hdr + TD_HDR_OFF_RECCOUNT);
	memcpy(out->build_id, hdr + TD_HDR_OFF_BUILDID, TD_HDR_BUILDID_LEN);
	out->build_id[TD_HDR_BUILDID_LEN] = '\0';
	out->case_crc = td_load_be32(hdr + TD_HDR_OFF_CASECRC);
	return 0;
}

int td_header_states_ptr(const struct td_header *h)
{
	return h->version >= TD_FORMAT_VERSION &&
	       (h->ptr_width == 4 || h->ptr_width == 8) &&
	       (h->word_order == TD_WORD_ORDER_MSW_FIRST ||
	        h->word_order == TD_WORD_ORDER_LSW_FIRST);
}

uint16_t td_native_ptr_width(void)
{
	return (uint16_t)sizeof(void *);
}

/*
 * Copy a LONG whose two halves differ into two WORDs, the way a target
 * lays an address into a WORD array, and look where the high half went.
 * Fixed-width types only (spec section 2.1); memcpy, not a cast, so the
 * answer is the machine's memory order and not a compiler's guess at it.
 */
uint16_t td_native_word_order(void)
{
	const uint32_t probe = 0x00010002uL;
	uint16_t w[2];

	memcpy(w, &probe, sizeof w);
	if (w[0] == 0x0001u && w[1] == 0x0002u)
		return TD_WORD_ORDER_MSW_FIRST;
	if (w[0] == 0x0002u && w[1] == 0x0001u)
		return TD_WORD_ORDER_LSW_FIRST;
	return TD_WORD_ORDER_UNKNOWN;
}

/* ------------------------------------------------------------------ */
/* Poison (spec section 4)                                             */
/* ------------------------------------------------------------------ */

static void poison16(int16_t *p, int n)
{
	int i;
	for (i = 0; i < n; i++)
		p[i] = TD_POISON16;
}

static void poison32(uint32_t *p, int n)
{
	int i;
	for (i = 0; i < n; i++)
		p[i] = 0xDEADDEADuL;
}

void td_poison_vdi(struct td_vdi_arrays *a)
{
	poison16(a->contrl, TD_VDI_CONTRL_N);
	poison16(a->intin,  TD_VDI_INTIN_N);
	poison16(a->ptsin,  TD_VDI_PTSIN_N);
	poison16(a->intout, TD_VDI_INTOUT_N);
	poison16(a->ptsout, TD_VDI_PTSOUT_N);
}

void td_poison_aes(struct td_aes_arrays *a)
{
	poison16(a->contrl,  TD_AES_CONTRL_N);
	poison16(a->global,  TD_AES_GLOBAL_N);
	poison16(a->int_in,  TD_AES_INTIN_N);
	poison16(a->int_out, TD_AES_INTOUT_N);
	poison32(a->addr_in,  TD_AES_ADDRIN_N);
	poison32(a->addr_out, TD_AES_ADDROUT_N);
}

void td_poison_call(struct td_call_frame *f)
{
	int i;

	poison32(f->args, TD_CALL_ARGS_N);
	f->ret = (int32_t)0xDEADDEADuL;
	for (i = 0; i < TD_CALL_CAPTURE_LEN; i += 2) {
		f->capture[i]     = TD_POISON_HI;
		f->capture[i + 1] = TD_POISON_LO;
	}
}

void td_poison_raster(struct td_raster_arrays *a)
{
	poison16(a->contrl, TD_RAST_CONTRL_N);
	poison16(a->intin,  TD_RAST_INTIN_N);
	poison16(a->ptsin,  TD_RAST_PTSIN_N);
	poison16(a->intout, TD_RAST_INTOUT_N);
	poison16(a->ptsout, TD_RAST_PTSOUT_N);
	/* buf_a / buf_b deliberately untouched -- see record.h */
}

/*
 * The buffer pattern.
 *
 * Everything here is uint32_t and uint16_t, so the two toolchains cannot
 * disagree about a width, and every operation is defined for unsigned
 * wrap-around.  The requirement is not randomness; it is that neighbouring
 * words differ, that neighbouring seeds differ, and that the sequence is the
 * same on a 68030 and on an AArch64.
 */
uint16_t td_rast_fill_word(uint16_t seed, uint16_t index)
{
	uint32_t x = (uint32_t)index * 1103515245uL + 12345uL;

	x ^= (uint32_t)seed * 2654435761uL;
	x ^= x >> 15;
	x = x * 2246822519uL;
	x ^= x >> 13;
	return (uint16_t)(x & 0xFFFFuL);
}

void td_rast_fill(int16_t *buf, uint16_t words, uint16_t seed)
{
	uint16_t i;

	for (i = 0; i < words; i++)
		buf[i] = (int16_t)td_rast_fill_word(seed, i);
}

/*
 * One AES pointer buffer -- see record.h and tosdump.h.
 *
 * Deliberately trivial arithmetic on uint32_t: the two toolchains cannot
 * disagree about it, every character it produces is a printable ASCII letter
 * on both (the m68k build's char is signed, the AArch64 build's is unsigned,
 * and 'A'..'Z' is the same number either way), and the NUL lands at a fixed
 * offset so that strlen of a buffer is TD_AES_BUF_TEXT_LEN on both machines.
 *
 * The seven-letter stride between buffers makes the four of them start with
 * four visibly different strings (ABCDEFGH, HIJKLMNO, OPQRSTUV, VWXYZABC),
 * none of which is the name of any AES process -- so appl_find against one
 * answers -1 on both machines rather than a machine-dependent ap_id.
 */
void td_aes_buf_fill(char *buf, size_t bytes, int index)
{
	size_t i;

	for (i = 0; i < bytes; i++) {
		uint32_t n = (uint32_t)index * 7uL + (uint32_t)i;
		buf[i] = (char)('A' + (int)(n % 26uL));
	}
	if (bytes > (size_t)TD_AES_BUF_TEXT_LEN)
		buf[TD_AES_BUF_TEXT_LEN] = '\0';
	else if (bytes > 0)
		buf[bytes - 1] = '\0';
}

/* ------------------------------------------------------------------ */
/* Record writers                                                      */
/* ------------------------------------------------------------------ */

static void store16_array(uint8_t *dst, const int16_t *src, int n)
{
	int i;
	for (i = 0; i < n; i++)
		td_store_be16(dst + i * 2, (uint16_t)src[i]);
}

static void store32_array(uint8_t *dst, const uint32_t *src, int n)
{
	int i;
	for (i = 0; i < n; i++)
		td_store_be32(dst + i * 4, src[i]);
}

void td_vdi_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                         int32_t ret, const struct td_vdi_arrays *after,
                         const int16_t *contrl_before)
{
	memset(rec, 0, TD_VDI_RECORD_SIZE);
	td_store_be16(rec + TD_VDI_OFF_CASEID, case_id);
	td_store_be16(rec + TD_VDI_OFF_OPCODE, opcode);
	td_store_be32(rec + TD_VDI_OFF_RET, (uint32_t)ret);
	store16_array(rec + TD_VDI_OFF_CONTRL, after->contrl, TD_VDI_CONTRL_N);
	store16_array(rec + TD_VDI_OFF_INTIN,  after->intin,  TD_VDI_INTIN_N);
	store16_array(rec + TD_VDI_OFF_PTSIN,  after->ptsin,  TD_VDI_PTSIN_N);
	store16_array(rec + TD_VDI_OFF_INTOUT, after->intout, TD_VDI_INTOUT_N);
	store16_array(rec + TD_VDI_OFF_PTSOUT, after->ptsout, TD_VDI_PTSOUT_N);
	store16_array(rec + TD_VDI_OFF_CONTRL_BEFORE, contrl_before,
	              TD_VDI_CONTRL_N);
}

void td_aes_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                         int32_t ret, const struct td_aes_arrays *after)
{
	memset(rec, 0, TD_AES_RECORD_SIZE);
	td_store_be16(rec + TD_AES_OFF_CASEID, case_id);
	td_store_be16(rec + TD_AES_OFF_OPCODE, opcode);
	store16_array(rec + TD_AES_OFF_CONTRL,  after->contrl,  TD_AES_CONTRL_N);
	store16_array(rec + TD_AES_OFF_GLOBAL,  after->global,  TD_AES_GLOBAL_N);
	store16_array(rec + TD_AES_OFF_INTIN,   after->int_in,  TD_AES_INTIN_N);
	store16_array(rec + TD_AES_OFF_INTOUT,  after->int_out, TD_AES_INTOUT_N);
	store32_array(rec + TD_AES_OFF_ADDRIN,  after->addr_in,  TD_AES_ADDRIN_N);
	store32_array(rec + TD_AES_OFF_ADDROUT, after->addr_out, TD_AES_ADDROUT_N);
	td_store_be32(rec + TD_AES_OFF_RET, (uint32_t)ret);
	/* reserved stays zero from the memset above */
}

void td_call_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                          const struct td_call_frame *f)
{
	memset(rec, 0, TD_CALL_RECORD_SIZE);
	td_store_be16(rec + TD_CALL_OFF_CASEID, case_id);
	td_store_be16(rec + TD_CALL_OFF_OPCODE, opcode);
	store32_array(rec + TD_CALL_OFF_ARGS, f->args, TD_CALL_ARGS_N);
	td_store_be32(rec + TD_CALL_OFF_RET, (uint32_t)f->ret);
	memcpy(rec + TD_CALL_OFF_CAPTURE, f->capture, TD_CALL_CAPTURE_LEN);
	/* reserved stays zero from the memset above */
}

void td_raster_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                            int32_t ret, const struct td_raster_arrays *after,
                            const int16_t *contrl_before,
                            const int16_t *src_mfdb, const int16_t *dst_mfdb)
{
	memset(rec, 0, TD_RAST_RECORD_SIZE);
	td_store_be16(rec + TD_RAST_OFF_CASEID, case_id);
	td_store_be16(rec + TD_RAST_OFF_OPCODE, opcode);
	td_store_be32(rec + TD_RAST_OFF_RET, (uint32_t)ret);
	store16_array(rec + TD_RAST_OFF_CONTRL, after->contrl, TD_RAST_CONTRL_N);
	store16_array(rec + TD_RAST_OFF_CONTRL_BEFORE, contrl_before,
	              TD_RAST_CONTRL_N);
	store16_array(rec + TD_RAST_OFF_INTIN,  after->intin,  TD_RAST_INTIN_N);
	store16_array(rec + TD_RAST_OFF_PTSIN,  after->ptsin,  TD_RAST_PTSIN_N);
	store16_array(rec + TD_RAST_OFF_INTOUT, after->intout, TD_RAST_INTOUT_N);
	store16_array(rec + TD_RAST_OFF_PTSOUT, after->ptsout, TD_RAST_PTSOUT_N);
	store16_array(rec + TD_RAST_OFF_SRC_MFDB, src_mfdb, TD_RAST_MFDB_N);
	store16_array(rec + TD_RAST_OFF_DST_MFDB, dst_mfdb, TD_RAST_MFDB_N);
	store16_array(rec + TD_RAST_OFF_BUF_A, after->buf_a, TD_RAST_BUF_WORDS);
	store16_array(rec + TD_RAST_OFF_BUF_B, after->buf_b, TD_RAST_BUF_WORDS);
	/* reserved stays zero from the memset above */
}

/* ------------------------------------------------------------------ */
/* Field descriptors -- so a difference is reported by name             */
/* ------------------------------------------------------------------ */

const struct td_field td_vdi_fields[] = {
	/* The VDI returns flip_y as a WORD: vdi_asm.S:124-125 clears d0 with
	 * `moveq #0,d0` before `move.w _flip_y,d0`.  TOS 3.06 does not clear the
	 * high half -- measured 0x0111 after the inquiries and setters, 0xFFFF
	 * after v_pline and v_fillarea -- so comparing the whole long would make
	 * every case differ on a register leftover that is not part of the
	 * contract.  The long stays in the record as evidence; the compared
	 * field is the half the contract defines. */
	{ "d0",            TD_VDI_OFF_RET + 2,       1,               2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "contrl",        TD_VDI_OFF_CONTRL,        TD_VDI_CONTRL_N, 2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "intin",         TD_VDI_OFF_INTIN,         TD_VDI_INTIN_N,  2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "ptsin",         TD_VDI_OFF_PTSIN,         TD_VDI_PTSIN_N,  2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "intout",        TD_VDI_OFF_INTOUT,        TD_VDI_INTOUT_N, 2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "ptsout",        TD_VDI_OFF_PTSOUT,        TD_VDI_PTSOUT_N, 2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "contrl_before", TD_VDI_OFF_CONTRL_BEFORE, TD_VDI_CONTRL_N, 2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ NULL, 0, 0, 0, 0, 0, 0, 0 }
};

/*
 * global is split in three because two of its words are an address:
 * global[11..12] is ap_3resv, which EmuTOS sets to the address of its own
 * AES global area, `pglobal->ap_3resv = (LONG)&D` (aes/gemsuper.c:98,
 * aes/gemrslib.h:25).  Field class address.  Measured: the same EmuTOS 1.4
 * source built by two toolchains puts it at 0xBE04 and 0xBD5C, every other
 * word of every AES case identical
 * (measurements/measurement-2026-10-26-oracle/RUNINFO.TXT).  One cell, a
 * LONG on every target: the AESGLOBAL keeps its 15 WORDs for an LP64
 * caller (tos-api/src/aes/appl.rs, AesGlobal::write).
 */
const struct td_field td_aes_fields[] = {
	{ "d0",       TD_AES_OFF_RET + 2,     1,                2, 1, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 0 },
	{ "contrl",   TD_AES_OFF_CONTRL,      TD_AES_CONTRL_N,  2, 1, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 0 },
	{ "global",   TD_AES_OFF_GLOBAL,      11,               2, 1, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 0 },
	{ "global",   TD_AES_OFF_GLOBAL + 22, 2,                2, 1, TD_FIELD_ADDRESS,  TD_ADDR_CELL_LONG, 11 },
	{ "global",   TD_AES_OFF_GLOBAL + 26, 2,                2, 1, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 13 },
	{ "int_in",   TD_AES_OFF_INTIN,       TD_AES_INTIN_N,   2, 1, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 0 },
	{ "int_out",  TD_AES_OFF_INTOUT,      TD_AES_INTOUT_N,  2, 1, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 0 },
	{ "addr_in",  TD_AES_OFF_ADDRIN,      TD_AES_ADDRIN_N,  4, 0, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 0 },
	{ "addr_out", TD_AES_OFF_ADDROUT,     TD_AES_ADDROUT_N, 4, 0, TD_FIELD_VALUE,    TD_ADDR_CELL_NONE, 0 },
	{ NULL, 0, 0, 0, 0, 0, 0, 0 }
};

const struct td_field td_call_fields[] = {
	{ "args",    TD_CALL_OFF_ARGS,    TD_CALL_ARGS_N,      4, 0, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "ret",     TD_CALL_OFF_RET,     1,                   4, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "capture", TD_CALL_OFF_CAPTURE, TD_CALL_CAPTURE_LEN, 1, 0, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ NULL, 0, 0, 0, 0, 0, 0, 0 }
};

/*
 * The raster area.  CONTRL is split in three because its middle is not
 * comparable: contrl[7..14] hold the two MFDB pointers, at the target's own
 * pointer width and pointing at the dumper's own buffers, so they are
 * recorded (field class address, two pointer cells) and never diffed by
 * value.  Everything on either side of them is ordinary VDI CONTRL and is
 * compared as usual.
 */
const struct td_field td_raster_fields[] = {
	{ "d0",            TD_RAST_OFF_RET + 2,      1,                 2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "contrl",        TD_RAST_OFF_CONTRL,       7,                 2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "contrl_mfdb",   TD_RAST_OFF_CONTRL + 14,  TD_RAST_CONTRL_PTR_N,
	                                                                2, 0, TD_FIELD_ADDRESS, TD_ADDR_CELL_PTR, 0 },
	{ "contrl_before", TD_RAST_OFF_CONTRL_BEFORE, 7,                2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "contrl_before_mfdb",
	                   TD_RAST_OFF_CONTRL_BEFORE + 14,
	                                             TD_RAST_CONTRL_PTR_N,
	                                                                2, 0, TD_FIELD_ADDRESS, TD_ADDR_CELL_PTR, 0 },
	{ "intin",         TD_RAST_OFF_INTIN,        TD_RAST_INTIN_N,   2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "ptsin",         TD_RAST_OFF_PTSIN,        TD_RAST_PTSIN_N,   2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "intout",        TD_RAST_OFF_INTOUT,       TD_RAST_INTOUT_N,  2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "ptsout",        TD_RAST_OFF_PTSOUT,       TD_RAST_PTSOUT_N,  2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "src_mfdb",      TD_RAST_OFF_SRC_MFDB,     TD_RAST_MFDB_N,    2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "dst_mfdb",      TD_RAST_OFF_DST_MFDB,     TD_RAST_MFDB_N,    2, 1, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "buf_a",         TD_RAST_OFF_BUF_A,        TD_RAST_BUF_WORDS, 2, 0, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ "buf_b",         TD_RAST_OFF_BUF_B,        TD_RAST_BUF_WORDS, 2, 0, TD_FIELD_VALUE, TD_ADDR_CELL_NONE, 0 },
	{ NULL, 0, 0, 0, 0, 0, 0, 0 }
};

const struct td_field *td_area_fields(uint16_t area)
{
	switch (area) {
	case TD_AREA_BIOS:
	case TD_AREA_XBIOS:
	case TD_AREA_GEMDOS: return td_call_fields;
	case TD_AREA_VDI:    return td_vdi_fields;
	case TD_AREA_AES:    return td_aes_fields;
	case TD_AREA_RASTER: return td_raster_fields;
	default:             return NULL;
	}
}

/* ------------------------------------------------------------------ */
/* Address cells (field class address, record.h)                       */
/* ------------------------------------------------------------------ */

uint16_t td_addr_cell_words(const struct td_field *f, const struct td_header *h)
{
	switch (f->addr_cell) {
	case TD_ADDR_CELL_LONG: return 2;
	case TD_ADDR_CELL_PTR:  return (uint16_t)(h->ptr_width / 2);
	default:                return 0;
	}
}

uint16_t td_addr_cell_count(const struct td_field *f, const struct td_header *h)
{
	if (f->compare != TD_FIELD_ADDRESS || !td_header_states_ptr(h))
		return 0;
	switch (f->addr_cell) {
	case TD_ADDR_CELL_LONG: return (uint16_t)(f->count / 2);
	case TD_ADDR_CELL_PTR:  return (uint16_t)(f->count / TD_ADDR_PTR_WORDS_MAX);
	default:                return 0;
	}
}

uint64_t td_addr_cell_value(const uint8_t *rec, const struct td_field *f,
                            const struct td_header *h, uint16_t n)
{
	uint16_t words = td_addr_cell_words(f, h);
	const uint8_t *p = rec + f->off + (size_t)n * words * 2;
	uint64_t v = 0;
	uint16_t i;

	for (i = 0; i < words; i++) {
		uint16_t at = (h->word_order == TD_WORD_ORDER_MSW_FIRST)
		              ? i : (uint16_t)(words - 1 - i);
		v = (v << 16) | (uint64_t)td_load_be16(p + at * 2);
	}
	return v;
}

int td_addr_ok(uint64_t v)
{
	return v < (uint64_t)TD_ADDR_LIMIT;
}
