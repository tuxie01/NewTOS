/*
 * runner_call.c -- poison, call, record for the call tables.  See
 * runner_call.h and docs/TOS-REFERENCE-DUMPER.md sections 3, 4 and 6.
 *
 * Portable: the same lines run on the TT and on the Pi.  The one thing that
 * differs between the two machines -- how an argument travels -- is the ABI
 * backend's (td_abi_trap); this file only says WHAT is passed.
 */
#include <string.h>

#include "abi.h"
#include "runner_call.h"

/* The buffers a case names with "b:<n>".  uint32_t so that a LONG capture
 * reads aligned on both machines. */
static uint32_t call_buf[TD_CALL_BUF_N][TD_CALL_BUF_BYTES / 4];

static struct td_call_frame run_frame;

void *td_call_buffer(int n)
{
	if (n < 0 || n >= TD_CALL_BUF_N)
		return NULL;
	return call_buf[n];
}

/*
 * Spec section 4: the 0xDEAD pattern -- laid in the UNIT the case's capture
 * reads the buffer in.  The two machines differ in byte order, so one fill
 * cannot read the same in every unit: the bytes DE AD are the WORD 0xADDE
 * on AArch64, and the WORD 0xDEAD is the bytes AD DE there.  An element the
 * call left alone must read the same on both, or the poison itself becomes
 * a difference:
 *
 *   unit B (and every buffer no capture reads): the bytes DE AD, the same
 *          memory on both -- which is also what a call that reads the
 *          buffer as bytes (Protobt's checksum, a disk format) must see;
 *   unit W: native WORDs 0xDEAD;   unit L: native LONGs 0xDEADDEAD.
 */
static void poison_buffer(int n, uint8_t unit)
{
	uint8_t *b = (uint8_t *)call_buf[n];
	uint32_t *l = call_buf[n];
	uint16_t *w = (uint16_t *)call_buf[n];
	int i;

	if (unit == 4) {
		for (i = 0; i < TD_CALL_BUF_BYTES / 4; i++)
			l[i] = 0xDEADDEADuL;
	} else if (unit == 2) {
		for (i = 0; i < TD_CALL_BUF_BYTES / 2; i++)
			w[i] = (uint16_t)TD_POISON16;
	} else {
		for (i = 0; i < TD_CALL_BUF_BYTES; i += 2) {
			b[i]     = TD_POISON_HI;
			b[i + 1] = TD_POISON_LO;
		}
	}
}

static void poison_buffers(const struct td_call_case *c)
{
	int n;

	for (n = 0; n < TD_CALL_BUF_N; n++) {
		uint8_t unit = 1;

		if (c->cap_arg >= 0 && c->arg[c->cap_arg].value == n)
			unit = c->cap_unit;
		poison_buffer(n, unit);
	}
}

int td_run_call(const struct td_call_case_table *ct, uint8_t *out,
                size_t out_bytes)
{
	uint8_t kind[TD_ABI_ARGS_MAX];
	long value[TD_ABI_ARGS_MAX];
	uint16_t i;
	int j;

	if (out_bytes < (size_t)ct->count * TD_CALL_RECORD_SIZE)
		return TD_RUN_CALL_ERR_BUFFER;

	for (i = 0; i < ct->count; i++) {
		const struct td_call_case *c = &ct->cases[i];
		uint8_t *rec = out + (size_t)i * TD_CALL_RECORD_SIZE;
		int64_t r;
		long ret;

		/* 1. poison the frame and both buffers (spec section 4) */
		td_poison_call(&run_frame);
		poison_buffers(c);

		/* 2. the arguments: what the backend passes, and what the record
		 *    says was passed.  An unused register slot carries poison. */
		for (j = 0; j < TD_ABI_ARGS_MAX; j++) {
			kind[j]  = 0;
			value[j] = (long)(int32_t)0xDEADDEADuL;
		}
		for (j = 0; j < (int)c->nargs; j++) {
			const struct td_call_arg *a = &c->arg[j];

			switch (a->kind) {
			case TD_CALL_ARG_WORD:
				kind[j]  = TD_ABI_ARG_WORD;
				value[j] = (long)(int16_t)a->value;
				run_frame.args[j] = (uint32_t)(int32_t)(int16_t)a->value;
				break;
			case TD_CALL_ARG_LONG:
				kind[j]  = TD_ABI_ARG_LONG;
				value[j] = (long)a->value;
				run_frame.args[j] = (uint32_t)a->value;
				break;
			default:   /* TD_CALL_ARG_BUF: its address, and its token */
				kind[j]  = TD_ABI_ARG_LONG;
				value[j] = (long)td_call_buffer((int)a->value);
				run_frame.args[j] = TD_CALL_BUF_TOKEN(a->value);
				break;
			}
		}
		run_frame.trap = c->trap;

		/* 3. the call under measurement */
		ret = td_abi_trap(c->trap, c->opcode, c->nargs, kind, value);

		/* 4. the return register at its native width, as one 64-bit
		 *    value: on the m68k the high half is the sign extension of
		 *    d0, on AArch64 it is x0's own (section 3) */
		r = (int64_t)ret;
		run_frame.ret    = (int32_t)(uint32_t)((uint64_t)r & 0xFFFFFFFFuL);
		run_frame.ret_hi = (uint32_t)((uint64_t)r >> 32);

		/* 5. the capture, element by element in record order */
		if (c->cap_arg >= 0) {
			const uint8_t *b =
				(const uint8_t *)td_call_buffer((int)c->arg[c->cap_arg].value);

			td_call_capture(run_frame.capture, b + c->cap_off, c->cap_len,
			                c->cap_unit);
		}

		td_call_record_write(rec, c->line, (uint16_t)c->opcode, &run_frame);
	}
	return 0;
}
