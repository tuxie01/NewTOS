/*
 * stub_abi.h -- the host-side stub ABI backend (spec section 9, T2).
 *
 * T2's acceptance condition is a host-side test that drives the runner against
 * a stub backend.  The stub logs every call it is handed and answers with a
 * fixed, documented pattern, so the test can assert what the runner did
 * without a TT or a Pi in the room.
 *
 * It deliberately overwrites intin[0] and ptsin[0] on every ordinary VDI call.
 * Nothing in a real VDI is allowed to do that -- which is exactly why the
 * record dumps the input arrays after the call, and the stub proves the runner
 * really captures them afterwards rather than before.
 *
 * The AES side mirrors the dispatcher's marshalling rule rather than reading
 * and writing the caller's arrays directly (EmuTOS aes/gemsuper.c:428-448):
 * int_in is copied onto a local buffer, min(IN_LEN, I_SIZE) words of it; the
 * answer is built in a local int_out and exactly OUT_LEN words are copied
 * back; control[], int_in[], addr_in[] and addr_out[] are never written.
 * That is what lets a test assert that a case declaring fewer output words
 * than the call produces leaves the surplus slots poisoned.
 *
 * Two documented departures, both because a test double may not be
 * undefined:
 *   - xif's local int_in / int_out are UNINITIALISED stack; the stub fills
 *     them with TD_STUB_AES_UNWRITTEN instead, so "the handler never wrote
 *     this slot" is observable rather than being whatever the stack held.
 *     On a real target those slots are genuinely undefined, which is why a
 *     case table may not declare more output words than the call produces.
 *   - the copy-back is clamped to TD_AES_INTOUT_N.  The real one is not
 *     (gemsuper.c:444-445), so a larger OUT_LEN overruns the caller's array
 *     on the target; the case loader refuses such a table, and the stub
 *     refuses to reproduce a heap smash inside the test suite.
 *
 * The AES arm also writes a mark into global[] on every call.  A real AES
 * writes global only at appl_init (gemsuper.c:92-98), but global IS an array
 * the AES may write, it is recorded after every call, and it is the one field
 * the runner must NOT poison -- so a per-call mark is what proves the record
 * captured the live array after the call rather than a poisoned copy.
 */
#ifndef TOSDUMP_STUB_ABI_H
#define TOSDUMP_STUB_ABI_H

#include "record.h"

#define TD_STUB_KIND_VDI  0
#define TD_STUB_KIND_AES  1
#define TD_STUB_MAX_CALLS 4096

/* what the stub answers, so the test and the stub agree in one place */
#define TD_STUB_AP_ID        1
#define TD_STUB_PHYS_HANDLE  7
#define TD_STUB_VWK_HANDLE  42
#define TD_STUB_MIN_CHHT     7    /* work_out pts[1]  */
#define TD_STUB_MIN_LWID     1    /* work_out pts[4]  */
#define TD_STUB_MIN_MKHT    11    /* work_out pts[9]  */
#define TD_STUB_INTIN_MARK  0x5A5A
#define TD_STUB_PTSIN_MARK  0x5A5B
#define TD_STUB_PTSOUT_0    0x0111
#define TD_STUB_PTSOUT_1    0x0222
#define TD_STUB_N_PTSOUT     1
#define TD_STUB_N_INTOUT     2
/* what the stub leaves in the trap's return register, per call kind */
#define TD_STUB_VDI_D0       1    /* flip_y */
#define TD_STUB_AES_D0       0    /* super() returns a hard 0, gemsuper.c:465 */

/* AES side -------------------------------------------------------------- */
#define TD_STUB_AES_UNWRITTEN 0x5AE5  /* stand-in for xif's uninitialised local */
#define TD_STUB_GLOBAL_MARK   0x6E11  /* written into global[] on every call    */
#define TD_STUB_GLOBAL_MARK_AT (TD_AES_GLOBAL_N - 1)
#define TD_STUB_AP_VERSION   0x0340   /* what the stub's appl_init leaves in    */
#define TD_STUB_AP_COUNT          1   /* global[0..2] and global[10], modelled  */
#define TD_STUB_AP_PLANES         4   /* on gemsuper.c:92-98                    */
#define TD_STUB_WCHAR             8   /* graf_handle int_out[1..4]              */
#define TD_STUB_HCHAR            16
#define TD_STUB_WBOX              8
#define TD_STUB_HBOX             16
/* wind_calc: the stub answers int_out[1..4] = int_in[2..5] + these, so a
 * test can tell an output slot apart from the input it came from. */
#define TD_STUB_CALC_DX           1
#define TD_STUB_CALC_DY           2
#define TD_STUB_CALC_DW           3
#define TD_STUB_CALC_DH           4
/* wind_get: int_out[1..4] = handle, field, and these two */
#define TD_STUB_WGET_2       0x0303
#define TD_STUB_WGET_3       0x0404
/* Raster side ------------------------------------------------------------
 * The three raster opcodes get their own arm, because the whole point of
 * the raster runner is WHERE it puts the two MFDB pointers: at
 * contrl[7 + n * N_PTRINTS] (gemlib gem_vdiP.h:177,204).  The stub reads
 * them back out of exactly those slots, stamps the destination MFDB's
 * fd_stand and the first word of the destination buffer, and leaves the
 * source alone -- so a test can prove the runner packed the pointers where
 * a real VDI looks for them and that the record captured both after the
 * call.  A stub that only logged the opcode would prove neither. */
#define TD_STUB_RAST_STAND   0x0707  /* stamped into dst->fd_stand      */
#define TD_STUB_RAST_PIXEL   0x0BB0  /* stamped into dst buffer word 0  */

/* the unsupported-opcode sentinel every AES returns (gemsuper.c:411-417) */
#define TD_STUB_AES_UNSUPPORTED (-1)

/* ------------------------------------------------------------------
 * The two arms that FOLLOW A POINTER
 * ------------------------------------------------------------------
 * Everything above answers out of int_in alone.  A case that points a call
 * at one of the dumper's buffers (core/cases.h, the addr_in column) needs
 * the double to actually dereference it, or a test could not tell a planted
 * pointer from a poisoned one.  Two arms do, each reading the pointer where
 * the real AES reads it:
 *
 *   appl_find (13) takes it from addr_in[0] -- AP_PNAME, aesdefs.h:165,
 *     gemsuper.c:107.  The stub copies AP_NAMELEN bytes from there into the
 *     call log and answers -1, which is what ap_find answers for a name no
 *     process has (gemaplib.c:95-108).
 *   wind_set (105) with WF_NAME takes it from *(char **)&int_in[2] --
 *     gemwmlib.c:1661 through gemsuper.c:346 -- which is the classic GEM
 *     binding and the one place an addr_in plant cannot reach.
 *
 * Both copy through memcpy: &int_in[2] is only WORD aligned.
 */
#define TD_STUB_NAME_LEN      8   /* AP_NAMELEN, aes/struct.h:111 */
#define TD_STUB_APPL_FIND_RET (-1)

/* Call side (BIOS / XBIOS, td_abi_trap) ---------------------------------
 * The stub logs the trap, the function number and the eight marshalled
 * argument slots exactly as the backend received them, so a test can prove
 * that a WORD arrives sign-extended, a buffer argument as the runner's own
 * buffer address, and an unused slot as poison.
 *
 * What it answers, so the record writer and the comparator get something to
 * chew on:
 *   - by default TD_STUB_TRAP_RET(trap, fn): positive, high half zero;
 *   - Bconstat (13:1) answers -1 -- the high half of the return register is
 *     then all ones, which is what a sign-extended LONG must look like;
 *   - Physbase (14:2) answers TD_STUB_PHYSBASE, a pointer that passes
 *     PTR_RAM (non-zero, LONG aligned, below 0x80000000);
 *   - every LONG argument that IS one of the runner's buffers
 *     (td_call_buffer) gets TD_STUB_BUF_TEXT at bytes 0..3, the buffer
 *     number and the low byte of the function number at 4..5, the LONG
 *     TD_STUB_BUF_LONG in machine order at 8..11 and the bytes 0x55 0xAA at
 *     510..511 -- except for EgetPalette (14:0x55), which the stub leaves
 *     alone so that a test can see the poison the runner laid before the
 *     case.
 */
#define TD_STUB_KIND_CALL     2
#define TD_STUB_TRAP_RET(trap, fn)  ((long)(trap) * 0x10000L + (long)(fn))
#define TD_STUB_PHYSBASE      0x00ABCD00L
#define TD_STUB_BUF_TEXT      "STUB"
#define TD_STUB_BUF_LONG      0x0A0B0C0DuL
#define TD_STUB_NOWRITE_FN    0x55

/* Super and Supexec (MEM12) -- the stub models EmuTOS's processor state:
 *   - Super (1:0x20): 1L answers 0 in user state and -1 in supervisor state
 *     (x20_inq, bdos/rwa.S:340-346); any other value in user state enters
 *     supervisor state and answers TD_STUB_SUPER_SSP, the "old SSP"
 *     (x20_usr); in supervisor state it leaves and answers
 *     TD_STUB_SUPER_LEAVE_D0 (x20_sup writes no d0 -- the stub may not be
 *     undefined, so it answers a mark);
 *   - Supexec (14:0x26): calls value[0] as `long (*)(void)` in supervisor
 *     state, restores the state it found, answers the function's value
 *     (bios/vectors.S:1376-1378). */
#define TD_STUB_SUPER_SSP       0x00E0F000L
#define TD_STUB_SUPER_LEAVE_D0  0x00005AE5L

/* The disk round (TASK/TOOL07) -- the stub models what the runner needs to
 * be seen doing with Rwabs and Getbpb:
 *   - Rwabs (13:4) READ (bit 0 of the flag clear) fills the buffer like any
 *     other call (above) and answers 0 -- except at the long sector
 *     TD_STUB_RWABS_FAIL, where it answers TD_STUB_RWABS_FAIL_RET (E_CHNG)
 *     and leaves the buffer alone;
 *   - Rwabs WRITE does not fill: it copies the first TD_STUB_WRITTEN_N bytes
 *     of the buffer it was handed into the call log (call_written) and
 *     answers 0, so a test sees what a write would have put on the disk;
 *   - Getbpb (13:7) answers, for drive 2 only, the address of a static
 *     9-WORD BPB whose WORD i is TD_STUB_BPB_WORD(i) (machine order, as a
 *     real BPB is), and 0 for every other drive. */
#define TD_STUB_RWABS_FAIL      0x7777L
#define TD_STUB_RWABS_FAIL_STR  "0x7777"
#define TD_STUB_RWABS_FAIL_RET  (-14L)
#define TD_STUB_WRITTEN_N       8
#define TD_STUB_BPB_WORD(i)     (0x0B00 + (i))

struct td_stub_call {
	int16_t kind;
	int16_t opcode;
	int16_t contrl1;   /* VDI: point count in ptsin        */
	int16_t contrl3;   /* VDI: value count in intin        */
	int16_t sub;       /* VDI: contrl[5]                   */
	int16_t handle;    /* VDI: contrl[6]                   */
	int16_t intin[2];  /* as handed in, before any mutation */
	int16_t ptsin[2];
	/* AES only: the whole control array and the whole int_in array as they
	 * stood when the call arrived.  Kept separate from the VDI fields above
	 * so neither side has to guess which shape a logged call has. */
	int16_t aes_control[TD_AES_CONTRL_N];
	int16_t aes_int_in[TD_AES_INTIN_N];
	/* AES only: the address array as it arrived, and -- for the two arms
	 * above -- the first bytes of whatever the pointer they follow points
	 * at.  aes_name_seen says the arm ran and dereferenced something. */
	void   *aes_addr_in[TD_AES_ADDRIN_N];
	char    aes_name[TD_STUB_NAME_LEN + 1];
	int     aes_name_seen;
	/* Call side only: what td_abi_trap was handed. */
	int16_t call_trap;
	uint16_t call_nargs;
	uint8_t call_kind[TD_CALL_ARGS_N];
	long    call_value[TD_CALL_ARGS_N];
	/* Call side, Rwabs writes only: the buffer's first bytes as handed in. */
	uint8_t call_written[TD_STUB_WRITTEN_N];
};

extern struct td_stub_call td_stub_calls[TD_STUB_MAX_CALLS];
extern int                 td_stub_ncalls;

void td_stub_reset(void);

#endif /* TOSDUMP_STUB_ABI_H */
