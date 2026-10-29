/*
 * runner_call.h -- poison, call, record for the call tables (BIOS + XBIOS).
 *
 * Spec: docs/TOS-REFERENCE-DUMPER.md sections 3, 4 and 6 ("The call table,
 * as built for DUMPBIOS").
 *
 * One case is: poison the frame and both buffers, marshal the arguments
 * (a buffer argument becomes the buffer's address, and its TOKEN in the
 * record), issue the trap through the ABI backend, copy the capture, write
 * the record.  No reset: a round-1 call table holds queries only.
 */
#ifndef TOSDUMP_RUNNER_CALL_H
#define TOSDUMP_RUNNER_CALL_H

#include "cases.h"

#define TD_RUN_CALL_ERR_BUFFER  (-1)  /* the output cannot hold every record */
#define TD_RUN_CALL_ERR_PRELUDE (-2)  /* an Rwabs prelude did not return 0:
                                       * the run stopped before its case    */
#define TD_RUN_CALL_ERR_UNIT    (-3)  /* the table names "u:0" and no unit
                                       * was set: nothing was issued        */
#define TD_RUN_CALL_ERR_PROGEXT (-4)  /* the table names "x:" and no program
                                       * extension was set: nothing issued
                                       * (TOOL20)                           */

int td_run_call(const struct td_call_case_table *ct, uint8_t *out,
                size_t out_bytes);

/* The runner's own buffer n (0..TD_CALL_BUF_N-1), for the host tests'
 * stub backend, which has to recognise a buffer address to write into it
 * the way a real call would. */
void *td_call_buffer(int n);

/* The runner's own callback n (0..TD_CALL_CB_N-1) for a Supexec case
 * ("f:<n>"), or NULL; for the host tests' stub, which calls it. */
void *td_call_callback(int n);

/* The physical unit of the test disk a "u:0" argument passes, 0..0x7FFF,
 * named by the run's profile (DISKUNIT.TXT, read by dumpbios.c;
 * docs/TOS-REFERENCE-DUMPER.md section 6).  Until it is set, a table that
 * names "u:0" is not run (TD_RUN_CALL_ERR_UNIT). */
void td_run_call_set_disk_unit(int32_t unit);

/* The program extension an "x:<path>" argument gets, 1..3 characters A-Z
 * and 0-9, named by the run's profile (PROGEXT.TXT, read by dumpbios.c;
 * TOOL20): the oracle's PRG, NewTOS's NAP.  0, or -1 for a malformed one.
 * Until it is set, a table that names "x:" is not run
 * (TD_RUN_CALL_ERR_PROGEXT). */
int td_run_call_set_prog_ext(const char *ext);

/*
 * A layout capture (TOOL16; cases.h, "Layouts"): write one 64-bit slot per
 * field of `l` into `dst` (tosdump.h), reading the structure at `base` with
 * the offsets and widths of `abi` and following its p> fields only where
 * td_layout_follow_refusal (record.h) admits them.  `base_refusal` non-zero
 * means the base pointer itself was refused: nothing is read, every slot is
 * TD_LAYOUT_NOREAD(base_refusal).  `set` holds the layouts a p> field names.
 * The runner passes the machine's own ABI; the host tests call it with the
 * other one too.
 */
void td_layout_capture(uint8_t *dst, const struct td_layout *set,
                       const struct td_layout *l, const void *base,
                       uint8_t base_refusal, int abi);

/*
 * What "+put" prelude number `prelude` (an index into the table's
 * preludes) did in the last run: 0 not reached, TD_PUT_WRITTEN written,
 * else the follow rule's refusal (TD_NOREAD_*, tosdump.h) -- nothing was
 * written.  DUMPBIOS logs it after the run (TOOL16).
 */
#define TD_PUT_WRITTEN 0xFF
uint8_t td_run_call_put_result(uint16_t prelude);

/* After TD_RUN_CALL_ERR_PRELUDE: the line of the prelude that stopped the
 * run, and what it returned.  0 / 0 after a run that did not stop. */
uint16_t td_run_call_stop_line(void);
long     td_run_call_stop_ret(void);

#endif /* TOSDUMP_RUNNER_CALL_H */
