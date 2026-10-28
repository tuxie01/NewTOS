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
	/* Format 3 only (tosdump.h).  Parsing a format 2 header leaves both
	 * at 0 / TD_WORD_ORDER_UNKNOWN: not stated, never guessed. */
	uint16_t ptr_width;    /* bytes: 4 or 8                          */
	uint16_t word_order;   /* TD_WORD_ORDER_*                        */
};

/*
 * Writes `h->version`'s layout: from format 3 on with the pointer width and
 * word order at 20 / 22; a format 2 header (only tests forge one) without
 * them, the twelve bytes from 20 on all zero.
 */
void td_header_write(uint8_t *hdr, const struct td_header *h);

/* Does this header state the pointer width and the word order?  0 for a
 * format 2 dump, whose address cells are therefore not checkable. */
int td_header_states_ptr(const struct td_header *h);

/*
 * The two facts format 3 records, measured on the machine that runs the
 * code: sizeof(void *), and where the high WORD of a LONG lands in memory
 * (memcpy of a LONG into two WORDs -- the same path set_mfdb_ptr and the
 * AES's own stores take).  The dumper writes these into every header
 * (dump.c).
 */
uint16_t td_native_ptr_width(void);
uint16_t td_native_word_order(void);

/* Returns 0 on success, negative on a malformed header. */
#define TD_ERR_MAGIC    (-1)
#define TD_ERR_VERSION  (-2)
#define TD_ERR_RESERVED (-3)
#define TD_ERR_RECSIZE  (-4)
#define TD_ERR_AREA     (-5)
#define TD_ERR_SIZE     (-6)
#define TD_ERR_PTRINFO  (-7)   /* format 3: pointer width not 4 or 8, or
                                  word order not MSW_FIRST / LSW_FIRST    */
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
	int32_t  ret;                      /* return register bits 31..0      */
	uint32_t ret_hi;                   /* return register bits 63..32     */
	int16_t  trap;                     /* 13, 14 (1)                      */
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

/*
 * Copy `len` bytes of a call buffer into a frame's capture field, as
 * elements of `unit` bytes (1, 2 or 4) read in the machine's own order and
 * stored through td_store_be16 / td_store_be32 -- the raster record's rule
 * for its pixel words.  A LONG a call wrote is four bytes in opposite order
 * on the two machines; only the element value means the same on both.
 * `src` must be aligned for `unit`; `len` a multiple of it and at most
 * TD_CALL_CAPTURE_LEN.  What `len` does not cover keeps the poison.
 */
void td_call_capture(uint8_t *dst, const void *src, uint16_t len, uint8_t unit);
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
 * TD_FIELD_ADDRESS  an address the target chose itself.  Never compared by
 *                   value: it CANNOT be equal across two machines or two
 *                   builds by construction, so its inequality measures the
 *                   linker, not the API.  Today: the MFDB pointer slots
 *                   inside a raster CONTRL (the dumper's own buffers) and
 *                   the AES's ap_3resv (global[11..12], EmuTOS `&D`,
 *                   aes/gemsuper.c:98).
 *
 * Instead, every address CELL is checked against a relational property on
 * each side separately: below TD_ADDR_LIMIT.  A TOS address is a signed
 * 32-bit LONG, and one at or above 0x80000000 reads as a negative error
 * code to a caller testing `< 0` (TASK/MEM11.md).  A violation is its own
 * verdict in a report, never a value difference.  The check needs the
 * dump's pointer width and word order, which only a format 3 header states;
 * on a format 2 dump it is reported as not checkable.
 *
 * This is NOT a per-case exception mechanism (spec section 6 forbids one):
 * it is a property of the field, the same for every case, and the data
 * still lands in the dump.
 */
#define TD_FIELD_ADDRESS  0
#define TD_FIELD_VALUE    1

/*
 * What one address cell of a TD_FIELD_ADDRESS field is, the `addr_cell`
 * member.  TD_ADDR_CELL_NONE for every value field.
 *
 * TD_ADDR_CELL_LONG  a LONG by the structure's own layout: two WORDs on
 *                    every target.  ap_3resv -- the AESGLOBAL is 15 WORDs
 *                    for an LP64 caller too, and NewTOS writes four bytes
 *                    (tos-api/src/aes/appl.rs, AesGlobal::write).
 * TD_ADDR_CELL_PTR   a pointer at the TARGET's own width, packed WORD by
 *                    WORD from the field's start: pointer n at word
 *                    n * width / 2 (gemlib gem_vdiP.h:177,204, the raster
 *                    runner's set_mfdb_ptr).  The field is sized for the
 *                    widest pointer, TD_ADDR_PTR_WORDS_MAX WORDs per cell;
 *                    on a target with four-byte pointers the WORDs behind
 *                    the last cell are no address and are not checked
 *                    (contrl[11..14] keep the poison on the m68k).
 */
#define TD_ADDR_CELL_NONE  0
#define TD_ADDR_CELL_LONG  1
#define TD_ADDR_CELL_PTR   2
#define TD_ADDR_PTR_WORDS_MAX  4

#define TD_ADDR_LIMIT  0x80000000uL

/*
 * The lower bound of retclass PTR_RAM (TASK/TOOL11.md): EmuTOS's
 * sysvars_start (references/emutos/tosvars.ld:31).  Below it lie the 68000
 * exception vectors and the processor-state save area, no structure a
 * BIOS / XBIOS call returns a pointer to; above it start the system
 * variables, and NewTOS keeps answers in its page 0 there (Setexc(0x168)
 * 0x5F0, Keytbl 0x768).  What it catches: a function number or a small
 * constant returned where a pointer belongs (Iorec's 0x0E).  It cannot say
 * that a structure lies behind an address above it -- only a capture
 * through the pointer can (docs/analysis/ptr-ram-rule-2026-10-28.md).
 */
#define TD_PTR_LOW     0x400uL

/*
 * The per-side property of retclass PTR_RAM, for a returned register value
 * `v` (all 64 bits): NULL when it holds, else the reason -- "is null",
 * "not < 0x80000000", "below 0x400", "is odd".  Even, not 4-aligned: m68k
 * structures are WORD aligned, and the oracle returns such pointers
 * (Iorec, Keytbl, Bconmap).  One rule for both users: the comparator
 * judges a return by it, and the call runner reads through a returned
 * pointer ("r:") only when it holds, so a value the comparator flags is
 * never dereferenced by the dumper.
 */
const char *td_ptr_ram_violation(uint64_t v);

/*
 * The follow rule of a layout capture (TASK/TOOL16): may the runner read a
 * structure of `size` bytes at the address `v` -- a returned pointer, or a
 * pointer field of a structure it already read?  0 when it may, else the
 * reason, TD_NOREAD_* (tosdump.h): the PTR_RAM rule above, in its order
 * (null, upper bound, lower bound, odd), and then the end of the structure,
 * which must lie at or below TD_ADDR_LIMIT too.  The runner reads nothing
 * when it is refused and records TD_LAYOUT_NOREAD(reason) instead; the
 * same function writes through a returned pointer ("+put") only when it
 * holds.  td_layout_noread_text names a reason for a report.
 */
uint8_t     td_layout_follow_refusal(uint64_t v, uint32_t size);
const char *td_layout_noread_text(uint8_t reason);

struct td_field {
	const char *name;
	uint16_t    off;    /* byte offset inside the record          */
	uint16_t    count;  /* element count                          */
	uint8_t     width;  /* element width in bytes: 2 or 4         */
	uint8_t     is_signed;
	uint8_t     compare;  /* field class, TD_FIELD_* above          */
	uint8_t     addr_cell; /* TD_ADDR_CELL_*, address class only    */
	/* Index of the first element in the array the field is part of, for
	 * a field that is one slice of a longer array (global[11..12]): a
	 * report then says global[11], not global[0]. */
	uint16_t    first;
};

/*
 * The address cells of field `f` in a dump with header `h`.
 *
 * td_addr_cell_count  how many cells; 0 for a value field, and 0 when the
 *                     header does not state pointer width and word order
 *                     (format 2) -- the caller reports that as not
 *                     checkable, it never reads a cell it cannot decode.
 * td_addr_cell_words  WORDs per cell: 2 for a LONG, ptr_width / 2 for a
 *                     pointer.
 * td_addr_cell_value  cell n of the record `rec`, put together from its
 *                     WORDs in the header's word order.
 * td_addr_ok          the relational property: below TD_ADDR_LIMIT.
 */
uint16_t td_addr_cell_count(const struct td_field *f, const struct td_header *h);
uint16_t td_addr_cell_words(const struct td_field *f, const struct td_header *h);
uint64_t td_addr_cell_value(const uint8_t *rec, const struct td_field *f,
                            const struct td_header *h, uint16_t n);
int      td_addr_ok(uint64_t v);

/* NULL-name-terminated tables. */
extern const struct td_field td_vdi_fields[];
extern const struct td_field td_aes_fields[];
extern const struct td_field td_call_fields[];
extern const struct td_field td_raster_fields[];

const struct td_field *td_area_fields(uint16_t area);

#endif /* TOSDUMP_RECORD_H */
