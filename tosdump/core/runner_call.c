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

/* The values a "t:<n>" argument passes: what an earlier case returned, the
 * case the loader marked (td_call_case.keep).
 *   t:0  the last Super(0L): the argument of the case that closes the
 *        bracket.  The two calls come from this one call site with the same
 *        frame, so on the m68k EmuTOS's x20_sup finds the USP where x20_usr
 *        left it (bdos/rwa.S:305-336, "already at old USP").  A LONG.
 *   t:1  the open file's Fcreate: the handle Fwrite and Fclose take (TOOL20).
 *        A WORD, as the binding's handle argument.
 *   t:2  a Dgetdrv outside the Dsetdrv bracket: the drive Dsetdrv(t:2)
 *        restores (TOOL20).  A WORD, as Dsetdrv's argument. */
static long call_token[TD_CALL_TOKENS];

/*
 * The extension of a program path ("x:", TOOL20): the program format is the
 * machine's -- the oracle's PRG, NewTOS's NAP -- so the run's PROFILE names
 * it (PROGEXT.TXT; tools/hatari-oracle.sh profile disk, cargo xtask tosdump
 * --disk), as it names the disk unit.  Empty: not given.
 */
static char prog_ext[TD_CALL_EXT_MAX + 1];

int td_run_call_set_prog_ext(const char *ext)
{
	size_t n = strlen(ext), i;

	if (n == 0 || n > TD_CALL_EXT_MAX)
		return -1;
	for (i = 0; i < n; i++)
		if (!((ext[i] >= 'A' && ext[i] <= 'Z') ||
		      (ext[i] >= '0' && ext[i] <= '9')))
			return -1;
	memcpy(prog_ext, ext, n + 1);
	return 0;
}

/* "x:<path>" + "." + the profile's extension; one per call (Pexec names
 * one program). */
static char prog_path[TD_CALL_STR_MAX + 2 + TD_CALL_EXT_MAX];

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

/*
 * A buffer a layout capture reads (TOOL16): every field gets the poison OF
 * ITS TYPE at the machine's own width -- a UBYTE 0xAD, a WORD 0xDEAD, a LONG
 * 0xDEADDEAD, a C long and a pointer the LONG 0xDEADDEAD sign-extended to
 * the machine's width (the unused-register rule of issue()).  Read back as
 * a slot (tosdump.h) a field the call left alone then holds the same 64-bit
 * value on both machines, although its bytes and its offset differ.  The
 * bytes between the fields keep the byte poison laid first.
 */
static void poison_layout(uint8_t *b, const struct td_layout *l)
{
	int abi = td_layout_native_abi();
	uint8_t i;

	for (i = 0; i < l->nslots; i++) {
		const struct td_layout_slot *s = &l->slot[i];
		uint8_t *d = b + s->off[abi];
		uint8_t w = td_layout_width(s->type, abi);

		if (s->parent >= 0)
			continue;           /* behind a pointer: not in the buffer */
		if (w == 1) {
			*d = TD_POISON_LO;
		} else if (w == 2) {
			uint16_t v = (uint16_t)TD_POISON16;
			memcpy(d, &v, 2);
		} else if (w == 4) {
			uint32_t v = 0xDEADDEADuL;
			memcpy(d, &v, 4);
		} else {
			int64_t v = (int64_t)(int32_t)0xDEADDEADuL;
			memcpy(d, &v, 8);
		}
	}
}

static void poison_buffers(const struct td_call_case_table *ct,
                           const struct td_call_case *c)
{
	int n;

	for (n = 0; n < TD_CALL_BUF_N; n++) {
		uint8_t unit = 1;
		int read_by_layout = 0;

		/* TOOL20: the DTA holds the search state of the last Fsfirst;
		 * only Fsetdta and Fsfirst lay the poison on it */
		if (n == c->dta_buf && c->dta_keep)
			continue;
		if (c->cap_arg >= 0 && c->arg[c->cap_arg].value == n) {
			unit = c->cap_unit;
			read_by_layout = c->cap_layout != 0;
		}
		if (c->cap_arg == TD_CALL_CAP_DTA && n == c->dta_buf)
			read_by_layout = 1;
		poison_buffer(n, unit);
		if (read_by_layout)
			poison_layout((uint8_t *)call_buf[n],
			              &ct->layouts[c->cap_layout - 1]);
	}
}

/* ------------------------------------------------------------------ */
/* Layout captures (TOOL16; cases.h, "Layouts")                         */
/* ------------------------------------------------------------------ */

static void store_slot(uint8_t *dst, uint64_t v)
{
	td_store_be32(dst, (uint32_t)(v >> 32));
	td_store_be32(dst + 4, (uint32_t)(v & 0xFFFFFFFFuL));
}

/* One field at `p`, at its width on `abi`, in the machine's own order,
 * extended to 64 bits: a UBYTE zero-, everything else sign-extended. */
static uint64_t read_field(const uint8_t *p, uint8_t type, int abi)
{
	switch (td_layout_width(type, abi)) {
	case 1: {
		uint8_t v = *p;
		return (uint64_t)v;
	}
	case 2: {
		int16_t v;
		memcpy(&v, p, 2);
		return (uint64_t)(int64_t)v;
	}
	case 4: {
		int32_t v;
		memcpy(&v, p, 4);
		return (uint64_t)(int64_t)v;
	}
	default: {
		int64_t v;
		memcpy(&v, p, 8);
		return (uint64_t)v;
	}
	}
}

void td_layout_capture(uint8_t *dst, const struct td_layout *set,
                       const struct td_layout *l, const void *base,
                       uint8_t base_refusal, int abi)
{
	const uint8_t *addr[TD_LAYOUT_SLOTS];
	uint8_t why[TD_LAYOUT_SLOTS];
	uint8_t i;

	for (i = 0; i < l->nslots; i++) {
		const struct td_layout_slot *s = &l->slot[i];
		const uint8_t *b;
		uint8_t r;
		uint64_t v;

		if (s->parent < 0) {
			b = (const uint8_t *)base;
			r = base_refusal;
		} else {
			b = addr[s->parent];
			r = why[s->parent];
		}
		addr[i] = NULL;
		why[i]  = 0;
		if (r) {
			/* not read: the pointer it hangs from fails the rule;
			 * whatever hangs from THIS field is not read either */
			store_slot(dst + (size_t)i * TD_LAYOUT_SLOT_BYTES,
			           TD_LAYOUT_NOREAD(r));
			why[i] = r;
			continue;
		}
		v = read_field(b + s->off[abi], s->type, abi);
		store_slot(dst + (size_t)i * TD_LAYOUT_SLOT_BYTES, v);
		if (s->follow >= 0) {
			r = td_layout_follow_refusal(v, set[s->follow].size[abi]);
			if (r)
				why[i] = r;
			else
				addr[i] = (const uint8_t *)td_abi_addr((long)v);
		}
	}
}

/*
 * "+put" (TOOL16): copy one field of the structure behind the previous
 * case's returned pointer onto another, when that pointer passes the follow
 * rule -- the same rule the read takes.  A WORD is stored as one WORD (an
 * IOREC index is read by an interrupt on a real TOS, bios/ikbd.c:157-173);
 * the other types through memcpy.  The outcome is kept per prelude and
 * logged by the program after the run (td_run_call_put_result): no console
 * output between two measurements.
 */
static uint8_t put_result[TD_MAX_CALL_PRELUDES];

uint8_t td_run_call_put_result(uint16_t prelude)
{
	return prelude < TD_MAX_CALL_PRELUDES ? put_result[prelude] : 0;
}

static void put_field(const struct td_call_case_table *ct,
                      const struct td_call_case *p, long prev_ret)
{
	const struct td_layout *l = &ct->layouts[p->cap_layout - 1];
	const struct td_layout_slot *d = &l->slot[p->arg[0].value];
	const struct td_layout_slot *s = &l->slot[p->arg[1].value];
	int abi = td_layout_native_abi();
	uint16_t k = (uint16_t)(p - ct->preludes);
	uint8_t r = td_layout_follow_refusal((uint64_t)(int64_t)prev_ret,
	                                     l->size[abi]);
	uint8_t *b;

	if (r) {
		put_result[k] = r;
		return;
	}
	b = (uint8_t *)td_abi_addr(prev_ret);
	if (d->type == TD_LT_WORD) {
		int16_t v;

		memcpy(&v, b + s->off[abi], 2);
		*(volatile int16_t *)(void *)(b + d->off[abi]) = v;
	} else {
		memmove(b + d->off[abi], b + s->off[abi],
		        td_layout_width(d->type, abi));
	}
	put_result[k] = TD_PUT_WRITTEN;
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
static long issue(const struct td_call_case_table *ct,
                  const struct td_call_case *c, struct td_call_frame *f)
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
		case TD_CALL_ARG_TOKEN:  /* what an earlier case returned */
			if (a->value == TD_CALL_TOKEN_SUPER) {
				kind[j]  = TD_ABI_ARG_LONG;
				value[j] = call_token[TD_CALL_TOKEN_SUPER];
			} else {         /* the handle, the drive: WORDs */
				kind[j]  = TD_ABI_ARG_WORD;
				value[j] = (long)(int16_t)call_token[a->value];
			}
			rec = TD_CALL_SUPER_TOKEN(a->value);
			break;
		case TD_CALL_ARG_STR:    /* the table's string, and its token */
		case TD_CALL_ARG_TAIL:
			kind[j]  = TD_ABI_ARG_LONG;
			value[j] = (long)(ct->str_pool + ct->str_off[a->value]);
			rec = TD_CALL_STR_TOKEN(a->value);
			break;
		case TD_CALL_ARG_PROG:   /* the path + the profile's extension */
			strcpy(prog_path, ct->str_pool + ct->str_off[a->value]);
			strcat(prog_path, ".");
			strcat(prog_path, prog_ext);
			kind[j]  = TD_ABI_ARG_LONG;
			value[j] = (long)prog_path;
			rec = TD_CALL_STR_TOKEN(a->value);
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
	long prev_ret = 0;   /* the return of the case before (+put)      */

	stop_line = 0;
	stop_ret  = 0;
	memset(put_result, 0, sizeof put_result);
	if (out_bytes < (size_t)ct->count * TD_CALL_RECORD_SIZE)
		return TD_RUN_CALL_ERR_BUFFER;
	/* a "u:0" with no unit from the profile: nothing is issued */
	if (ct->unit_token && (disk_unit < 0 || disk_unit > 0x7FFF))
		return TD_RUN_CALL_ERR_UNIT;
	/* an "x:" with no program extension from the profile: the same */
	if (ct->prog_ext && prog_ext[0] == '\0')
		return TD_RUN_CALL_ERR_PROGEXT;

	for (i = 0; i < ct->count; i++) {
		const struct td_call_case *c = &ct->cases[i];
		uint8_t *rec = out + (size_t)i * TD_CALL_RECORD_SIZE;
		int64_t r;
		long ret;

		/* 1. poison the frame and both buffers (spec section 4) -- once,
		 *    before the preludes, so a prelude's buffer reaches its case */
		td_poison_call(&run_frame);
		poison_buffers(ct, c);

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
			/* "+put" (TOOL16): through the pointer the case before
			 * returned -- the loader put that case right before it */
			if (p->trap == TD_CALL_TRAP_PUT) {
				put_field(ct, p, prev_ret);
				continue;
			}
			pr = issue(ct, p, NULL);

			if (pr != 0) {
				stop_line = p->line;
				stop_ret  = pr;
				return TD_RUN_CALL_ERR_PRELUDE;
			}
		}

		/* 3. the call under measurement */
		ret = issue(ct, c, &run_frame);
		prev_ret = ret;

		/* a return a later "t:<n>" passes: Super(0L)'s token, the
		 * handle of Fcreate, the drive of Dgetdrv (the loader marks
		 * the case) */
		if (c->keep)
			call_token[c->keep - 1] = ret;

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
		if (c->cap_layout) {
			/* field by field (TOOL16): through the return, or the
			 * buffer argument; a return that fails the follow rule
			 * is never read through, every slot says why */
			const struct td_layout *l = &ct->layouts[c->cap_layout - 1];
			int abi = td_layout_native_abi();

			if (c->cap_arg == TD_CALL_CAP_RET) {
				uint8_t why = td_layout_follow_refusal((uint64_t)r,
				                                       l->size[abi]);

				td_layout_capture(run_frame.capture, ct->layouts, l,
				                  why ? NULL : td_abi_addr(ret), why,
				                  abi);
			} else if (c->cap_arg == TD_CALL_CAP_DTA) {
				/* TOOL20: the DTA buffer */
				td_layout_capture(run_frame.capture, ct->layouts, l,
				                  td_call_buffer((int)c->dta_buf),
				                  0, abi);
			} else {
				td_layout_capture(run_frame.capture, ct->layouts, l,
				                  td_call_buffer((int)
				                      c->arg[c->cap_arg].value),
				                  0, abi);
			}
		} else if (c->cap_arg == TD_CALL_CAP_RET) {
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
