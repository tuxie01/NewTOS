/*
 * tosdump.h -- record format for the TOS reference dumper.
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md, section 3 (record format) and 4 (poison).
 *
 * This header is compiled by three toolchains: the host cc, m68k-atari-mint-gcc
 * (TOS 3.06 target) and clang/aarch64 (NewTOS target).  Per spec section 2.1 it
 * therefore uses fixed-width types only -- never int, short or long -- and the
 * record is written field by field at explicit offsets, never by dumping a
 * struct, so that padding and alignment rules cannot differ between targets.
 */
#ifndef TOSDUMP_H
#define TOSDUMP_H

#include <stdint.h>
#include <stddef.h>

/* ------------------------------------------------------------------ */
/* File header -- 32 bytes (spec section 3)                            */
/* ------------------------------------------------------------------ */

#define TD_MAGIC            "TDMP"
#define TD_MAGIC_LEN        4
#define TD_FORMAT_VERSION   3   /* what the dumpers write                */
#define TD_FORMAT_VERSION_2 2   /* still read: no pointer information    */
#define TD_HEADER_SIZE      32

#define TD_HDR_OFF_MAGIC        0   /* 4  bytes, "TDMP"                   */
#define TD_HDR_OFF_VERSION      4   /* u16, format version                */
#define TD_HDR_OFF_AREA         6   /* u16, TD_AREA_*                     */
#define TD_HDR_OFF_RECSIZE      8   /* u16, record size in bytes          */
#define TD_HDR_OFF_RECCOUNT    10   /* u16, record count                  */
#define TD_HDR_OFF_BUILDID     12   /* 4  bytes, git short hash ASCII     */
#define TD_HDR_OFF_CASECRC     16   /* u32, CRC32 of the case table       */
#define TD_HDR_OFF_PTRWIDTH    20   /* u16, format 3: sizeof(void *)      */
#define TD_HDR_OFF_WORDORDER   22   /* u16, format 3: TD_WORD_ORDER_*     */
#define TD_HDR_OFF_RESERVED    24   /* 8  bytes, must be zero             */
#define TD_HDR_BUILDID_LEN      4
#define TD_HDR_RESERVED_LEN     8
/* Format 2 had no pointer information: all twelve bytes from 20 on were
 * reserved, and a format 2 header is still checked that way. */
#define TD_HDR_V2_OFF_RESERVED 20
#define TD_HDR_V2_RESERVED_LEN 12

/*
 * Format 3 -- the pointer width and the word order of the machine that
 * wrote the dump.  The ONLY change against format 2; every record is byte
 * for byte what format 2 wrote.
 *
 * Why the comparator needs them: a few recorded WORDs are an address the
 * target laid into a WORD array in its own memory order -- ap_3resv in the
 * AESGLOBAL (global[11..12]), the MFDB pointers packed into a raster
 * CONTRL (contrl[7..14]).  The m68k stores the most significant WORD first
 * (EmuTOS' &D reads 0x0000 0xBD60), AArch64 the least significant first
 * (NewTOS' ap_3resv reads 0x80B0 0x0011 = 0x001180B0), and an AArch64
 * pointer is four WORDs, an m68k one two.  Without both facts the WORDs
 * cannot be put back together into the address they are, so a format 2
 * dump's address cells are "not checkable" (host/tosdiff.c).
 *
 * Both are measured on the machine by the dumper itself
 * (td_native_ptr_width / td_native_word_order, record.c), not configured:
 * the same expression on both targets, so the backends stay symmetric
 * (spec section 8, rule 5).
 */
#define TD_WORD_ORDER_UNKNOWN    0   /* format 2: not stated              */
#define TD_WORD_ORDER_MSW_FIRST  1   /* most significant WORD first, m68k */
#define TD_WORD_ORDER_LSW_FIRST  2   /* least significant WORD first,
                                        AArch64 little-endian             */

/* area codes */
#define TD_AREA_BIOS        1
#define TD_AREA_XBIOS       2
#define TD_AREA_GEMDOS      3
#define TD_AREA_VDI         4
#define TD_AREA_AES         5
#define TD_AREA_RASTER      6

/* ------------------------------------------------------------------ */
/* Case tables (spec section 6)                                        */
/* ------------------------------------------------------------------ */

/* Upper bound on cases per area.  Fixed so that the target program needs no
 * allocator: it runs on a TT with TOS 3.06 and on NewTOS alike. */
#define TD_MAX_CASES        256
#define TD_COMMENT_LEN       64

/* retclass column (spec section 6), BIOS / XBIOS / GEMDOS only */
#define TD_RET_EXACT        0
#define TD_RET_IGNORE       1
#define TD_RET_NONZERO      2
#define TD_RET_HANDLE       3
#define TD_RET_PTR_RAM      4

/*
 * The trap's own return value.
 *
 * The spec's record layout (section 3) leaves four bytes reserved after the
 * opcode and has no field for what the trap returns.  The VDI trap does return
 * something: d0 carries flip_y (docs/api/vdi/_dispatcher.md, "Return d0"), and
 * docs/api/vdi/_working-list.md D-080 records that vsm_height, vst_height,
 * vst_point and vst_font set it as a side effect.  That is an observable
 * output of a call, so it is measured like every other output, in the space
 * the layout already reserved.  Format version 2; format 3 (pointer width
 * and word order in the header, above) left every record as it was.
 */

/* ------------------------------------------------------------------ */
/* Poison (spec section 4)                                             */
/* ------------------------------------------------------------------ */

#define TD_POISON16         ((int16_t)0xDEAD)
#define TD_POISON_HI        ((uint8_t)0xDE)
#define TD_POISON_LO        ((uint8_t)0xAD)

/* ------------------------------------------------------------------ */
/* VDI record -- 1592 bytes                                            */
/* ------------------------------------------------------------------ */

#define TD_VDI_CONTRL_N     12
#define TD_VDI_INTIN_N     128
#define TD_VDI_PTSIN_N     256
#define TD_VDI_INTOUT_N    128
#define TD_VDI_PTSOUT_N    256

#define TD_VDI_RECORD_SIZE      1592
#define TD_VDI_OFF_CASEID          0   /* u16                             */
#define TD_VDI_OFF_OPCODE          2   /* u16                             */
#define TD_VDI_OFF_RET             4   /* i32, the trap's d0 return       */
#define TD_VDI_OFF_CONTRL          8   /* contrl[12] after the call       */
#define TD_VDI_OFF_INTIN          32   /* intin[128]  after the call      */
#define TD_VDI_OFF_PTSIN         288   /* ptsin[256]  after the call      */
#define TD_VDI_OFF_INTOUT        800   /* intout[128] after the call      */
#define TD_VDI_OFF_PTSOUT       1056   /* ptsout[256] after the call      */
#define TD_VDI_OFF_CONTRL_BEFORE 1568  /* contrl[12] before the call      */

/* ------------------------------------------------------------------ */
/* AES record -- 120 bytes                                             */
/* ------------------------------------------------------------------ */

#define TD_AES_CONTRL_N      5
#define TD_AES_GLOBAL_N     15
#define TD_AES_INTIN_N      16
#define TD_AES_INTOUT_N      7
#define TD_AES_ADDRIN_N      3
#define TD_AES_ADDROUT_N     1

#define TD_AES_RECORD_SIZE   120
#define TD_AES_OFF_CASEID      0   /* u16                                 */
#define TD_AES_OFF_OPCODE      2   /* u16                                 */
#define TD_AES_OFF_CONTRL      4   /* contrl[5]   after                   */
#define TD_AES_OFF_GLOBAL     14   /* global[15]  after                   */
#define TD_AES_OFF_INTIN      44   /* int_in[16]  after                   */
#define TD_AES_OFF_INTOUT     76   /* int_out[7]  after                   */
#define TD_AES_OFF_ADDRIN     90   /* addr_in[3]  after, 32-bit each      */
#define TD_AES_OFF_ADDROUT   102   /* addr_out[1] after, 32-bit           */
#define TD_AES_OFF_RET       106   /* i32, the trap's d0 return           */
#define TD_AES_OFF_RESERVED  110   /* 10 bytes, zero                      */
#define TD_AES_RESERVED_LEN   10

/* ------------------------------------------------------------------ */
/* The AES pointer buffers (the "addr_in" column, cases.h)             */
/* ------------------------------------------------------------------ */
/*
 * A dozen AES opcodes do all of their work behind a pointer -- the object
 * manager, the menu manager, the scrap manager, appl_find, rsrc_load, the
 * shell's path calls, wind_get/wind_set WF_NAME and WF_INFO.  A case table
 * cannot name an address: it is one file carried to two machines, and an
 * address is different on each.  So the table names a BUFFER BY NUMBER and
 * the dumper puts its own address in, exactly as cases/vdi-raster.tsv names
 * its two pixel buffers instead of their addresses.
 *
 * The buffers are STATICS of the runner: fixed count, fixed size, laid fresh
 * before every call it issues.  Same size and same bytes on both targets --
 * a buffer that differed would make the case measure the buffer instead of
 * the function.
 *
 * WHAT IS IN ONE, and why it is not the raster fill.  Half of the calls this
 * unlocks take a C STRING through the pointer (appl_find's AP_PNAME,
 * rsrc_load's filename, menu_text, shel_find, the scrap manager's strcpy),
 * and neither AES bounds-checks one.  A pseudo-random word fill would send
 * strlen off the end of the buffer.  So every buffer is a bounded string by
 * construction: TD_AES_BUF_TEXT_LEN printable characters, a NUL, and the
 * same printable pattern to the end as a canary.  Eight characters because
 * that is the AES' own name length (EmuTOS aes/struct.h:111, AP_NAMELEN),
 * which is what fpdnm's strncmp compares (gempd.c:48).
 */
#define TD_AES_BUF_N          4    /* buffers a table may name, 0..3      */
#define TD_AES_BUF_BYTES    256    /* each, on both targets               */
#define TD_AES_BUF_TEXT_LEN   8    /* printable characters before the NUL */

/*
 * What the RECORD carries for an addr_in slot a case pointed at a buffer.
 *
 * The record's address slots are compared byte for byte between the two
 * machines (record.c, td_aes_fields).  A buffer's address is different on a
 * TT and on a Pi by construction, so recording it would fabricate a
 * difference in every case that uses one -- the same trap the poison pattern
 * was built to avoid (runner_aes.c, poison_ptrs).  A planted slot therefore
 * records this TOKEN, which says what the table said: "slot j points at
 * buffer n".  A slot the call MODIFIED records the modified value instead,
 * low 32 bits, which is exactly the difference it is.
 *
 * A slot no case planted is untouched by all of this and still records the
 * poison, so a table without an addr_in column produces the same bytes it
 * always did.
 */
#define TD_AES_ADDR_TOKEN(n)  ((uint32_t)0xB0FF0000uL + (uint32_t)(n))

/* ------------------------------------------------------------------ */
/* VDI raster record -- 3696 bytes                                     */
/* ------------------------------------------------------------------ */
/*
 * The raster opcodes (109 vro_cpyfm, 110 vr_trnfm, 121 vrt_cpyfm) do not fit
 * the VDI record, and not because they are bigger: their result is MEMORY,
 * not a number in INTOUT.  Three things follow, and each of them is a field
 * the VDI record has no room for.
 *
 *   1. The forms are described by two MFDBs whose pointers live in CONTRL,
 *      packed at 7 + n * (sizeof(void*) / sizeof(WORD)) (gemlib
 *      gem_vdiP.h:177,204).  On the m68k that is CONTRL[7..8] and [9..10];
 *      on AArch64 it is CONTRL[7..10] and [11..14].  A twelve WORD CONTRL
 *      cannot hold the AArch64 form at all, so this record carries fifteen
 *      (gemlib's own VDI_CNTRLMAX, mt_gem.h:3508).
 *   2. The MFDB itself is an output: vr_trnfm OVERWRITES the destination's
 *      fd_stand (vdi_raster.c:303-314).  The eight WORD fields of both MFDBs
 *      are therefore recorded after the call.
 *   3. The pixel data is the result.  Two fixed buffers are recorded whole,
 *      as WORDS through td_store_be16, so that the comparison is word for
 *      word and not byte for byte -- the two machines differ in byte order
 *      and only the word values mean the same thing on both.
 *
 * WHAT IS AND IS NOT COMPARABLE, because it decides what a case may do:
 *
 *   Comparable.  vr_trnfm in either direction with fd_nplanes <= 8: both
 *   machines run the same plane-permutation (EmuTOS vdi_vr_trnfm
 *   vdi_raster.c:295-350; NewTOS tos-api/src/vdi/raster.rs vr_trnfm), and
 *   the standard form is machine-independent BY DEFINITION -- it is what
 *   fd_stand = 1 means.  Also comparable for every opcode: the rules and the
 *   coordinates (which calls are silent no-ops, what CONTRL echoes, that
 *   INTIN/PTSIN come back unmodified, that INTOUT/PTSOUT stay poisoned) and
 *   the untouched parts of a destination buffer.
 *
 *   NOT comparable.  Pixels in DEVICE form below nine planes.  The TT stores
 *   a 4-plane form word-interleaved (s_nxwd = fd_nplanes * 2,
 *   vdi_raster.c:922); NewTOS stores every memory MFDB as one 16-bit RGB565
 *   word per pixel (raster.rs, Mfdb::mem_stride_px).  The same bytes mean
 *   different pixels, so a copy of them is not a measurement of anything.
 *   A 16-plane form is the one exception -- EmuTOS' own vro_cpyfm16
 *   (vdi_raster.c:975-1221) is the identical chunky UWORD loop -- but only
 *   where CONF_WITH_VDI_16BIT is on in the ROM being measured, which the
 *   table cannot know and therefore measures instead.
 */

#define TD_RAST_CONTRL_N    15   /* gemlib VDI_CNTRLMAX, mt_gem.h:3508    */
#define TD_RAST_INTIN_N    128
#define TD_RAST_PTSIN_N    256
#define TD_RAST_INTOUT_N   128
#define TD_RAST_PTSOUT_N   256

/* The eight WORD fields of an MFDB after fd_addr: fd_w, fd_h, fd_wdwidth,
 * fd_stand, fd_nplanes, fd_r1, fd_r2, fd_r3 (EmuTOS include/gsxdefs.h:65-76).
 * fd_addr itself is a pointer and is not recorded: it is the address of one
 * of the two buffers below, chosen by the linker, and carries nothing about
 * the API. */
#define TD_RAST_MFDB_N       8

/*
 * The two pixel buffers, A and B, in WORDS.  Every access either machine can
 * make is bounded by the loader's geometry gate (cases.c): a form of fd_h
 * rows and fd_wdwidth words is at most fd_h * fd_wdwidth * 16 words under
 * EITHER reading -- planar with fd_nplanes <= 16, or chunky with one word per
 * pixel -- so requiring that product to fit here bounds both.  512 words
 * leaves room for a 32x16 form (fd_wdwidth 2) and for 16 planes of it.
 */
#define TD_RAST_BUF_WORDS  512

/* Fewer than the other areas on purpose: a record is 3696 bytes and the
 * program holds every record in RAM until the end (spec section 7). */
#define TD_RAST_MAX_CASES   64

#define TD_RAST_RECORD_SIZE      3696
#define TD_RAST_OFF_CASEID          0   /* u16                            */
#define TD_RAST_OFF_OPCODE          2   /* u16                            */
#define TD_RAST_OFF_RET             4   /* i32, the trap's d0 return      */
#define TD_RAST_OFF_CONTRL          8   /* contrl[15] after the call      */
#define TD_RAST_OFF_CONTRL_BEFORE  38   /* contrl[15] before the call     */
#define TD_RAST_OFF_INTIN          68   /* intin[128]  after the call     */
#define TD_RAST_OFF_PTSIN         324   /* ptsin[256]  after the call     */
#define TD_RAST_OFF_INTOUT        836   /* intout[128] after the call     */
#define TD_RAST_OFF_PTSOUT       1092   /* ptsout[256] after the call     */
#define TD_RAST_OFF_SRC_MFDB     1604   /* source MFDB WORDs after        */
#define TD_RAST_OFF_DST_MFDB     1620   /* dest MFDB WORDs after          */
#define TD_RAST_OFF_BUF_A        1636   /* buffer A[512] after            */
#define TD_RAST_OFF_BUF_B        2660   /* buffer B[512] after            */
#define TD_RAST_OFF_RESERVED     3684   /* 12 bytes, zero                 */
#define TD_RAST_RESERVED_LEN       12

/* The two pointer slots inside CONTRL: contrl[7..14].  Recorded, never
 * compared by value -- they hold the address of one of the two buffers, at
 * the target's own pointer width, so they cannot be equal across the two
 * machines and a difference there measures the linker, not the VDI.  Field
 * class address (record.h): each pointer is checked against
 * TD_ADDR_LIMIT on its own side, read with the header's pointer width and
 * word order (format 3). */
#define TD_RAST_CONTRL_PTR_FIRST    7
#define TD_RAST_CONTRL_PTR_N        8

/* ------------------------------------------------------------------ */
/* BIOS / XBIOS / GEMDOS record -- 320 bytes                           */
/* ------------------------------------------------------------------ */

#define TD_CALL_ARGS_N         8
#define TD_CALL_CAPTURE_LEN  256

#define TD_CALL_RECORD_SIZE  320
#define TD_CALL_OFF_CASEID     0   /* u16                                 */
#define TD_CALL_OFF_OPCODE     2   /* u16, the function number            */
#define TD_CALL_OFF_ARGS       4   /* 8 longs as passed                   */
#define TD_CALL_OFF_RET       36   /* u32, return register bits 31..0     */
#define TD_CALL_OFF_CAPTURE   40   /* capture buffer                      */
#define TD_CALL_OFF_RET_HI   296   /* u32, return register bits 63..32    */
#define TD_CALL_OFF_TRAP     300   /* u16, 13 BIOS / 14 XBIOS / 1 GEMDOS  */
#define TD_CALL_OFF_RESERVED 302   /* 18 bytes, zero                      */
#define TD_CALL_RESERVED_LEN  18

/*
 * The two fields at 296 and 300 were added on 2026-10-27, before the first
 * dump of this record was taken (docs/TOS-REFERENCE-DUMPER.md section 3,
 * "Extended 2026-10-27").
 *
 * RET_HI.  NewTOS returns a TOS LONG in the 64-bit x0, and an LP64 caller
 * compares the whole register (`long r = Bconstat(2); if (r < 0)`).  A
 * negative error code returned zero-extended, or a pointer above 4 GiB, is
 * invisible in the low 32 bits.  The m68k's d0 has no high half; its RET_HI
 * is the sign extension of the low half -- what (int64_t)(long) gives -- so
 * the comparator reads the return as the 64-bit ret_hi:ret on both sides.
 *
 * TRAP.  DUMPBIOS measures trap #13 and trap #14 from one table into one
 * dump (header area TD_AREA_BIOS); the same function number is a different
 * call on each, so every record says which trap it was.
 */

/*
 * The call buffers.  A case table cannot name an address (one file, two
 * machines), so -- exactly as the AES addr_in column and the raster MFDBs
 * do -- it names one of the dumper's own buffers by number, "b:<n>", and the
 * runner passes the buffer's address.  Poisoned with 0xDEAD before every
 * case.  1024 bytes because the largest round-1 output is Protobt's 512-byte
 * boot sector (bios/floppy.c:690).
 */
#define TD_CALL_BUF_N           2
#define TD_CALL_BUF_BYTES    1024

/* What the record's argument slot carries for a buffer argument: a token
 * that says "buffer n", never the address, which differs between the two
 * machines by construction.  Same value as the AES's TD_AES_ADDR_TOKEN. */
#define TD_CALL_BUF_TOKEN(n)  ((uint32_t)0xB0FF0000uL + (uint32_t)(n))

/*
 * Super and Supexec (MEM12; docs/TOS-REFERENCE-DUMPER.md section 6, "Super
 * and Supexec in a BIOS-area table").  "f:<n>" names the dumper's own
 * callback n (runner_call.c), passed as a pointer; "t:0" the token the last
 * Super(0L) case returned.  The record carries these tokens, never the
 * address or the token's value, which differ per machine by construction.
 */
#define TD_CALL_CB_N            3
#define TD_CALL_CB_TOKEN(n)    ((uint32_t)0xB0FC0000uL + (uint32_t)(n))
#define TD_CALL_SUPER_TOKEN(n) ((uint32_t)0xB0F50000uL + (uint32_t)(n))

/* The physical unit of the test disk ("u:0", the disk round's closing tables,
 * docs/TOS-REFERENCE-DUMPER.md section 6): the record carries this token,
 * never the unit number, which is the machine's (oracle 18, NewTOS 26). */
#define TD_CALL_UNIT_TOKEN(n)  ((uint32_t)0xB0F40000uL + (uint32_t)(n))

/* The width of the return value a retclass judges, in bytes: the return
 * type of the EmuTOS handler (LONG, WORD, UBYTE).  d0 above that type is
 * not defined by the m68k calling convention. */
#define TD_RET_WIDTH_LONG     4
#define TD_RET_WIDTH_WORD     2
#define TD_RET_WIDTH_BYTE     1

#endif /* TOSDUMP_H */
