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

#define TD_RUN_CALL_ERR_BUFFER (-1)   /* the output cannot hold every record */

int td_run_call(const struct td_call_case_table *ct, uint8_t *out,
                size_t out_bytes);

/* The runner's own buffer n (0..TD_CALL_BUF_N-1), for the host tests'
 * stub backend, which has to recognise a buffer address to write into it
 * the way a real call would. */
void *td_call_buffer(int n);

#endif /* TOSDUMP_RUNNER_CALL_H */
