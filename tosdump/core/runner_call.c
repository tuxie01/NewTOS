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

/*
 * The physical unit of the test disk, what a "u:0" argument passes (the
 * disk round, docs/TOS-REFERENCE-DUMPER.md section 6).  It is the machine's,
 * so the run's PROFILE names it -- the oracle's profile disk 18 (IDE master),
 * NewTOS 26 (the SD card), docs/api/bios/04-rwabs.md "Decisions" -- and the
 * dumper hands it in (td_run_call_set_disk_unit) before the run; no search,
 * and nothing compiled in, so the two builds stay the same but for the byte
 * order (section 2.1).  -1: not given.
 */
static int32_t disk_unit = -1;

void td_run_call_set_disk_unit(int32_t unit)
{
	disk_unit = unit;
}

/*
 * The callbacks a Supexec case names with "f:<n>" (MEM12; docs/TOS-REFERENCE-
 * DUMPER.md section 6, "Super and Supexec").  Portable C, the same lines on
 * both machines; EmuTOS jumps to them in supervisor mode, NewTOS calls them
 * at EL0 in super state.
 *   0  a positive LONG                    -- the value comes back
 *   1  a negative LONG                    -- sign-extended in the register
 *   2  what Super(1L) answers in here     -- the callback runs in super
 *      state.  EmuTOS forbids GEMDOS calls from Supexec (bios/xbios.c:770);
 *      Super(1L) is answered by the TRAP #1 front end before the BDOS
 *      (bdos/rwa.S:159-184, 340-346), so nothing the rule protects is used.
 */
static long td_cb_positive(void)
{
	return 0x12345678L;
}

static long td_cb_negative(void)
{
	return -2L;
}

static long td_cb_super_inquiry(void)
{
	uint8_t kind[TD_ABI_ARGS_MAX];
	long value[TD_ABI_ARGS_MAX];
	int j;

	for (j = 0; j < TD_ABI_ARGS_MAX; j++) {
		kind[j]  = 0;
		value[j] = (long)(int32_t)0xDEADDEADuL;
	}
	kind[0]  = TD_ABI_ARG_LONG;
	value[0] = 1L;
	return td_abi_trap(1, 0x20, 1, kind, value);
}

typedef long (*td_call_cb)(void);

static const td_call_cb call_cb[TD_CALL_CB_N] = {
	td_cb_positive, td_cb_negative, td_cb_super_inquiry
};

void *td_call_callback(int n)
{
	if (n < 0 || n >= TD_CALL_CB_N)
		return NULL;
	return (void *)call_cb[n];
}

/* What the last Super(0L) case returned: the argument of the "t:0" case
 * that closes the bracket.  The two calls come from this one call site with
 * the same frame, so on the m68k EmuTOS's x20_sup finds the USP where
 * x20_usr left it (bdos/rwa.S:305-336, "already at old USP"). */
static long super_token;

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

static uint16_t stop_line;
static long     stop_ret;

uint16_t td_run_call_stop_line(void)
{
	return stop_line;
}

long td_run_call_stop_ret(void)
{
	return stop_ret;
}

/*
 * Marshal one line's arguments and issue its trap.  What the backend is
 * handed, and -- when `f` is not NULL, for a measured case -- what the record
 * says was passed.  An unused register slot carries poison.  Preludes go
 * through here too (f NULL): the same marshalling, nothing recorded.
 */
static long issue(const struct td_call_case *c, struct td_call_frame *f)
{
	uint8_t kind[TD_ABI_ARGS_MAX];
	long value[TD_ABI_ARGS_MAX];
	int j;

	for (j = 0; j < TD_ABI_ARGS_MAX; j++) {
		kind[j]  = 0;
		value[j] = (long)(int32_t)0xDEADDEADuL;
	}
	for (j = 0; j < (int)c->nargs; j++) {
		const struct td_call_arg *a = &c->arg[j];
		uint32_t rec;

		switch (a->kind) {
		case TD_CALL_ARG_WORD:
			kind[j]  = TD_ABI_ARG_WORD;
			value[j] = (long)(int16_t)a->value;
			rec = (uint32_t)(int32_t)(int16_t)a->value;
			break;
		case TD_CALL_ARG_LONG:
			kind[j]  = TD_ABI_ARG_LONG;
			value[j] = (long)a->value;
			rec = (uint32_t)a->value;
			break;
		case TD_CALL_ARG_FUNC:   /* the callback, and its token */
			kind[j]  = TD_ABI_ARG_LONG;
			value[j] = (long)td_call_callback((int)a->value);
			rec = TD_CALL_CB_TOKEN(a->value);
			break;
		case TD_CALL_ARG_TOKEN:  /* what Super(0L) returned */
			kind[j]  = TD_ABI_ARG_LONG;
			value[j] = super_token;
			rec = TD_CALL_SUPER_TOKEN(a->value);
			break;
		case TD_CALL_ARG_UNIT:   /* the test disk's physical unit */
			kind[j]  = TD_ABI_ARG_WORD;
			value[j] = (long)(int16_t)disk_unit;
			rec = TD_CALL_UNIT_TOKEN(a->value);
			break;
		default:   /* TD_CALL_ARG_BUF: its address, and its token */
			kind[j]  = TD_ABI_ARG_LONG;
			value[j] = (long)td_call_buffer((int)a->value);
			rec = TD_CALL_BUF_TOKEN(a->value);
			break;
		}
		if (f)
			f->args[j] = rec;
	}
	if (f)
		f->trap = c->trap;
	return td_abi_trap(c->trap, c->opcode, c->nargs, kind, value);
}

int td_run_call(const struct td_call_case_table *ct, uint8_t *out,
                size_t out_bytes)
{
	uint16_t i, k;

	stop_line = 0;
	stop_ret  = 0;
	if (out_bytes < (size_t)ct->count * TD_CALL_RECORD_SIZE)
		return TD_RUN_CALL_ERR_BUFFER;
	/* a "u:0" with no unit from the profile: nothing is issued */
	if (ct->unit_token && (disk_unit < 0 || disk_unit > 0x7FFF))
		return TD_RUN_CALL_ERR_UNIT;

	for (i = 0; i < ct->count; i++) {
		const struct td_call_case *c = &ct->cases[i];
		uint8_t *rec = out + (size_t)i * TD_CALL_RECORD_SIZE;
		int64_t r;
		long ret;

		/* 1. poison the frame and both buffers (spec section 4) -- once,
		 *    before the preludes, so a prelude's buffer reaches its case */
		td_poison_call(&run_frame);
		poison_buffers(c);

		/* 2. the preludes of a disk table: Rwabs reads into a buffer the
		 *    case then writes back or reads again.  One that does not
		 *    return 0 left the poison where its data should be, so the run
		 *    stops here: no case after it is issued, no dump is written
		 *    (docs/TOS-REFERENCE-DUMPER.md section 6, "The disk round"). */
		for (k = 0; k < c->prelude_count; k++) {
			const struct td_call_case *p =
				&ct->preludes[c->prelude_first + k];
			long pr;

			/* "+set" (destructive tables): change the buffer the
			 * read before it filled -- the loader put that read
			 * right before it, in this case's preludes */
			if (p->trap == TD_CALL_TRAP_SET) {
				uint8_t *b = (uint8_t *)td_call_buffer(
					(int)p[-1].arg[1].value);
				uint16_t j;

				for (j = 0; j < p->nargs; j++)
					b[p->arg[j].value >> 8] =
						(uint8_t)(p->arg[j].value & 0xFF);
				continue;
			}
			pr = issue(p, NULL);

			if (pr != 0) {
				stop_line = p->line;
				stop_ret  = pr;
				return TD_RUN_CALL_ERR_PRELUDE;
			}
		}

		/* 3. the call under measurement */
		ret = issue(c, &run_frame);

		/* Super(0L) opened the bracket: keep the token for its t:0 */
		if (c->trap == 1 && c->opcode == 0x20 && c->nargs == 1 &&
		    c->arg[0].kind == TD_CALL_ARG_LONG && c->arg[0].value == 0)
			super_token = ret;

		/* 4. the return register at its native width, as one 64-bit
		 *    value: on the m68k the high half is the sign extension of
		 *    d0, on AArch64 it is x0's own (section 3) */
		r = (int64_t)ret;
		run_frame.ret    = (int32_t)(uint32_t)((uint64_t)r & 0xFFFFFFFFuL);
		run_frame.ret_hi = (uint32_t)((uint64_t)r >> 32);

		/* 5. the capture, element by element in record order: from a
		 *    buffer argument, or (r:) through the returned pointer -- only
		 *    when it passes PTR_RAM, the rule the comparator judges it by
		 *    (td_ptr_ram_violation, record.h): an error code, a NULL, a
		 *    function number or a 64-bit placeholder is never read
		 *    through.  A capture not taken keeps the poison. */
		if (c->cap_arg == TD_CALL_CAP_RET) {
			if (td_ptr_ram_violation((uint64_t)r) == NULL)
				td_call_capture(run_frame.capture,
				                (const uint8_t *)td_abi_addr(ret) +
				                c->cap_off,
				                c->cap_len, c->cap_unit);
		} else if (c->cap_arg >= 0) {
			const uint8_t *b =
				(const uint8_t *)td_call_buffer((int)c->arg[c->cap_arg].value);

			td_call_capture(run_frame.capture, b + c->cap_off, c->cap_len,
			                c->cap_unit);
		}

		td_call_record_write(rec, c->line, (uint16_t)c->opcode, &run_frame);
	}
	return 0;
}
