/*
 * record.h -- big-endian record writer / reader for the TOS reference dumper.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md sections 2.1 and 3.
 *
 * Every field of every record goes through td_store_be16 / td_store_be32.
 * Reserved fields and the header are routed through them too, exactly as the
 * spec demands, so that a byte-order bug cannot hide in a field nobody looks at.
 */
#ifndef TOSDUMP_RECORD_H
#define TOSDUMP_RECORD_H

#include "tosdump.h"

/* ---- byte order (spec section 2.1) ------------------------------------ */

void     td_store_be16(uint8_t *p, uint16_t v);
void     td_store_be32(uint8_t *p, uint32_t v);
uint16_t td_load_be16(const uint8_t *p);
uint32_t td_load_be32(const uint8_t *p);

/* ---- CRC32 (case table identity) -------------------------------------- */

uint32_t td_crc32(const uint8_t *data, size_t len);

/* ---- file header ------------------------------------------------------ */

struct td_header {
	uint16_t version;
	uint16_t area;
	uint16_t record_size;
	uint16_t record_count;
	char     build_id[TD_HDR_BUILDID_LEN + 1];  /* NUL-terminated copy */
	uint32_t case_crc;
};

void td_header_write(uint8_t *hdr, const struct td_header *h);

/* Returns 0 on success, negative on a malformed header. */
#define TD_ERR_MAGIC    (-1)
#define TD_ERR_VERSION  (-2)
#define TD_ERR_RESERVED (-3)
#define TD_ERR_RECSIZE  (-4)
#define TD_ERR_AREA     (-5)
#define TD_ERR_SIZE     (-6)
int td_header_parse(const uint8_t *hdr, size_t len, struct td_header *out);

/* Record size mandated for an area, or 0 if the area code is unknown. */
uint16_t td_area_record_size(uint16_t area);
const char *td_area_name(uint16_t area);

/* ---- per-area frames --------------------------------------------------- */

struct td_vdi_arrays {
	int16_t contrl[TD_VDI_CONTRL_N];
	int16_t intin[TD_VDI_INTIN_N];
	int16_t ptsin[TD_VDI_PTSIN_N];
	int16_t intout[TD_VDI_INTOUT_N];
	int16_t ptsout[TD_VDI_PTSOUT_N];
};

struct td_aes_arrays {
	int16_t  contrl[TD_AES_CONTRL_N];
	int16_t  global[TD_AES_GLOBAL_N];
	int16_t  int_in[TD_AES_INTIN_N];
	int16_t  int_out[TD_AES_INTOUT_N];
	uint32_t addr_in[TD_AES_ADDRIN_N];
	uint32_t addr_out[TD_AES_ADDROUT_N];
};

struct td_call_frame {
	uint32_t args[TD_CALL_ARGS_N];
	int32_t  ret;
	uint8_t  capture[TD_CALL_CAPTURE_LEN];
};

/*
 * The raster frame.  Fifteen CONTRL words rather than twelve, because an
 * AArch64 caller packs the destination MFDB pointer at CONTRL[11..14]
 * (gemlib gem_vdiP.h:177,204; VDI_CNTRLMAX = 15, mt_gem.h:3508), and two
 * pixel buffers, which are what a raster call actually produces.
 */
struct td_raster_arrays {
	int16_t contrl[TD_RAST_CONTRL_N];
	int16_t intin[TD_RAST_INTIN_N];
	int16_t ptsin[TD_RAST_PTSIN_N];
	int16_t intout[TD_RAST_INTOUT_N];
	int16_t ptsout[TD_RAST_PTSOUT_N];
	int16_t buf_a[TD_RAST_BUF_WORDS];
	int16_t buf_b[TD_RAST_BUF_WORDS];
};

/* Fill every array and the capture buffer with the 0xDEAD pattern. */
void td_poison_vdi(struct td_vdi_arrays *a);
void td_poison_aes(struct td_aes_arrays *a);
void td_poison_call(struct td_call_frame *f);

/*
 * Poison the parameter arrays of a raster frame -- but NOT the two pixel
 * buffers.  Those are the operand of the call, and a copy out of an all-
 * 0xDEAD source proves nothing about where the bits went; they are laid
 * with td_rast_fill instead, which gives every word a different value so
 * that a permutation, an off-by-one row or an untouched region is visible.
 */
void td_poison_raster(struct td_raster_arrays *a);

/*
 * The buffer pattern.  A pure function of (seed, index) in fixed-width
 * arithmetic, so both builds lay down the identical words without the case
 * table having to spell 512 of them out.  Two different seeds give a source
 * and a destination that no logic op can confuse with one another.
 */
uint16_t td_rast_fill_word(uint16_t seed, uint16_t index);
void     td_rast_fill(int16_t *buf, uint16_t words, uint16_t seed);

/*
 * Lay one AES pointer buffer (tosdump.h, TD_AES_BUF_N).  A pure function of
 * the buffer's number, so both builds put the identical bytes there without
 * the case table having to spell 256 of them out -- and so a case that hands
 * the AES two different buffers hands it two distinguishable strings.
 *
 * The result is a bounded C string followed by a canary: printable ASCII
 * only, a NUL at TD_AES_BUF_TEXT_LEN, the same printable pattern behind it.
 * See tosdump.h for why a string and not the raster's word pattern.
 */
void td_aes_buf_fill(char *buf, size_t bytes, int index);

/* Writers.  `rec` must be at least the area's record size. */
void td_vdi_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                         int32_t ret, const struct td_vdi_arrays *after,
                         const int16_t *contrl_before);
void td_aes_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                         int32_t ret, const struct td_aes_arrays *after);
void td_call_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                          const struct td_call_frame *f);
/* `src_mfdb` / `dst_mfdb` are the TD_RAST_MFDB_N WORD fields that follow
 * fd_addr, read back out of the two MFDBs after the call. */
void td_raster_record_write(uint8_t *rec, uint16_t case_id, uint16_t opcode,
                            int32_t ret, const struct td_raster_arrays *after,
                            const int16_t *contrl_before,
                            const int16_t *src_mfdb, const int16_t *dst_mfdb);

/* ---- field descriptors (used by the comparator to name a difference) --- */

/*
 * Field classes, the `compare` member of a descriptor.
 *
 * TD_FIELD_VALUE    compared by value.
 * TD_FIELD_ADDRESS  an address the target chose itself: recorded, never
 *                   compared.  Reserved for a field that CANNOT be equal
 *                   across two machines or two builds by construction, so
 *                   its inequality measures the linker, not the API.  Today:
 *                   the MFDB pointer slots inside a raster CONTRL (the
 *                   dumper's own buffers) and the AES's ap_3resv
 *                   (global[11..12], EmuTOS `&D`, aes/gemsuper.c:98).
 *
 * This is NOT a per-case exception mechanism (spec section 6 forbids one):
 * it is a property of the field, the same for every case, and the data
 * still lands in the dump.  Phase 1 replaces "never compared" with a
 * relational check (null/non-null, alignment) -- never by value, never
 * ignored; tos-api's aes_reference harness already checks ap_3resv that way.
 */
#define TD_FIELD_ADDRESS  0
#define TD_FIELD_VALUE    1

struct td_field {
	const char *name;
	uint16_t    off;    /* byte offset inside the record          */
	uint16_t    count;  /* element count                          */
	uint8_t     width;  /* element width in bytes: 2 or 4         */
	uint8_t     is_signed;
	uint8_t     compare;  /* field class, TD_FIELD_* above          */
	/* Index of the first element in the array the field is part of, for
	 * a field that is one slice of a longer array (global[11..12]): a
	 * report then says global[11], not global[0]. */
	uint16_t    first;
};

/* NULL-name-terminated tables. */
extern const struct td_field td_vdi_fields[];
extern const struct td_field td_aes_fields[];
extern const struct td_field td_call_fields[];
extern const struct td_field td_raster_fields[];

const struct td_field *td_area_fields(uint16_t area);

#endif /* TOSDUMP_RECORD_H */
