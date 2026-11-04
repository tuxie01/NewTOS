/*
 * sbprobe.c -- graf_slidebox (AES 76) without a drag (TASK/A128): the value
 * gr_slidebox returns when the button is already up, for 3D and non-3D
 * children and parents, vertical and horizontal, and the object left where it
 * was.  A permanent probe of the tosdump tree like akprobe.c: the call needs an
 * OBJECT tree, which no case table can give (tosdump/core/cases.c lists 76 as
 * BLOCKS); it writes a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS): the AES
 * calls go through the tosdump ABI backends (abi/m68k_trap.c,
 * abi/newtos_svc.c, td_abi_aes), the GEMDOS calls through mintlib's osbind.h.
 * The OBJECT below is the native one of each side: `long ob_spec` makes it the
 * 24-byte m68k record and the 32-byte LP64 record.  Built by `make -C tosdump
 * sbprobe-m68k` (SBPROBE.PRG) and `make -C tosdump sbprobe-newtos`
 * (SBPROBE.NAP, tosdump/build-sbprobe-newtos.sh); run by
 * tosdump/probes/run-sbprobe.sh.  Output: SBPROBE.LOG in the current directory.
 *
 * EmuTOS (references/emutos/aes/gemgrlib.c:358-413): c = ob_actxywh(parent)
 * (3D-grown, aes/gemoblib.c:313-343), t = ob_relxywh(obj); a FL3DOBJ child
 * grows by 2*ADJ3DSTD and, in a non-3D parent, moves up/left by ADJ3DSTD
 * (xy_adjust); gr_dragbox (:243-267) keeps the grab offset
 * max(mouse - start, 0) and returns at once with the button up
 * (gr_stilldn -> ev_multi MU_BUTTON state 0, abutton/downorup,
 * aes/geminput.c:348-357,545-551); divnd/divis, +ADJ3DSTD for a non-zero
 * divnd under xy_adjust, capped at divis, mul_div_round(divnd, 1000, divis).
 *
 * Each case builds ROOT (IBOX) -> PARENT -> CHILD, reads the pointer with
 * graf_mkstate and places ROOT so that the pointer sits at a fixed offset from
 * the CHILD's object rect: anchor "in" = 5 px right and below the child's
 * top-left (inside the child, right/below the outline start in both the 3D and
 * the plain case, so the outline stays at its start), anchor "above" = 5 px
 * right and 10 px above (resp. left of, for a horizontal slider) the child,
 * inside the parent (offset 0: the outline jumps to the pointer).
 *
 * Lines: Cnn ret=<int_out[0]> out1=<int_out[1]> obj=<ob_x>,<ob_y> after
 * the call (compared); I-lines (the pointer) are side-specific.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>

#include "abi.h"

typedef struct {
	int16_t ob_next, ob_head, ob_tail;
	uint16_t ob_type, ob_flags, ob_state;
	long ob_spec;
	int16_t ob_x, ob_y, ob_width, ob_height;
} OBJ;

#define G_BOX     20
#define G_IBOX    25
#define LASTOB    0x0020
#define FL3DIND   0x0200
#define FL3DACT   0x0600
#define OUTLINED  0x0010
#define SHADOWED  0x0020

static char out[4096];
static int olen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 160)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

static void flush_log(void)
{
	long fh = Fcreate("SBPROBE.LOG", 0);

	if (fh < 0)
		return;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
}

static int16_t control[5];
static int16_t global[15];
static int16_t int_in[16];
static int16_t int_out[8];
static void *addr_in[2];
static void *addr_out[2];

static int16_t aes(int16_t op, int16_t nin, int16_t nout, int16_t nain)
{
	int i;

	control[0] = op;
	control[1] = nin;
	control[2] = nout;
	control[3] = nain;
	control[4] = 0;
	for (i = 0; i < 8; i++)
		int_out[i] = 0;
	td_abi_aes(control, global, int_in, int_out, addr_in, addr_out);
	return int_out[0];
}

struct sbcase {
	const char *id;
	uint16_t pflags, pstate;
	long pspec;
	int16_t px, py, pw, ph;		/* parent, relative to ROOT */
	uint16_t cflags;
	int16_t cx, cy, cw, ch;		/* child, relative to the parent */
	int16_t isvert;
	int16_t above;			/* anchor: 0 = in, 1 = above / left of */
	const char *what;
};

/* parent spec: G_BOX, border thickness -1 (0xFF), colour word 0x1141 */
#define PSPEC 0x00FF1141L
/* child spec: G_BOX, thickness -1, colour word 0x1101 */
#define CSPEC 0x00FF1101L

static const struct sbcase cases[] = {
	{ "C01", 0, 0, PSPEC, 10, 10, 24, 150, 0,       2, 20, 20, 30, 1, 0, "plain child, plain parent" },
	{ "C02", 0, 0, PSPEC, 10, 10, 24, 150, FL3DACT, 2, 20, 20, 30, 1, 0, "3D child, plain parent (xy_adjust)" },
	{ "C03", FL3DIND, 0, PSPEC, 10, 10, 24, 150, FL3DACT, 2, 20, 20, 30, 1, 0, "3D child, 3D parent" },
	{ "C04", 0, 0, PSPEC, 10, 10, 24, 150, FL3DACT, 2, 0, 20, 30, 1, 0, "3D child at the top (divnd 0)" },
	{ "C05", 0, 0, PSPEC, 10, 10, 24, 150, FL3DACT, 2, 120, 20, 30, 1, 0, "3D child at the bottom (capped)" },
	{ "C06", 0, 0, PSPEC, 10, 10, 200, 20, 0,       40, 2, 40, 16, 0, 0, "horizontal, plain" },
	{ "C07", 0, 0, PSPEC, 10, 10, 200, 20, FL3DACT, 40, 2, 40, 16, 0, 0, "horizontal, 3D child" },
	{ "C08", 0, 0, PSPEC, 10, 10, 24, 150, 0,       2, 40, 20, 30, 1, 1, "plain child, pointer above it" },
	{ "C09", 0, 0, PSPEC, 10, 10, 24, 150, FL3DACT, 2, 40, 20, 30, 1, 1, "3D child, pointer above it" },
	{ "C10", FL3DIND, OUTLINED, PSPEC, 10, 10, 24, 150, 0, 2, 20, 20, 30, 1, 0, "plain child, 3D OUTLINED parent" },
	{ "C11", FL3DIND, SHADOWED, PSPEC, 10, 10, 24, 150, 0, 2, 20, 20, 30, 1, 0, "plain child, 3D SHADOWED parent" },
	{ "C12", 0, 0, PSPEC, 10, 10, 24, 150, 0,       2, 0, 20, 150, 1, 0, "child as high as the parent (divis 0)" },
	{ "C13", 0, 0, PSPEC, 10, 10, 24, 132, FL3DACT, 2, 46, 20, 36, 1, 0, "file-selector thumb, 30 entries, top 10" },
};

static OBJ tree[3];

int main(void)
{
	unsigned i;
	int16_t mx, my;

	int16_t ap_id = aes(10, 0, 1, 0);	/* appl_init */
	if (ap_id < 0) {
		emit("A99 appl_init=%d\n", ap_id);
		flush_log();
		return 1;
	}
	for (i = 0; i < sizeof(cases) / sizeof(cases[0]); i++) {
		const struct sbcase *k = &cases[i];
		int16_t ax, ay;

		aes(79, 0, 5, 0);		/* graf_mkstate */
		mx = int_out[1];
		my = int_out[2];
		/* the pointer relative to the child's top-left */
		ax = 5;
		ay = 5;
		if (k->above) {
			if (k->isvert)
				ay = -10;
			else
				ax = -10;
		}
		tree[0].ob_next = -1; tree[0].ob_head = 1; tree[0].ob_tail = 1;
		tree[0].ob_type = G_IBOX; tree[0].ob_flags = 0; tree[0].ob_state = 0;
		tree[0].ob_spec = 0;
		tree[0].ob_x = (int16_t)(mx - ax - k->px - k->cx);
		tree[0].ob_y = (int16_t)(my - ay - k->py - k->cy);
		tree[0].ob_width = 300; tree[0].ob_height = 300;
		tree[1].ob_next = 0; tree[1].ob_head = 2; tree[1].ob_tail = 2;
		tree[1].ob_type = G_BOX; tree[1].ob_flags = k->pflags; tree[1].ob_state = k->pstate;
		tree[1].ob_spec = k->pspec;
		tree[1].ob_x = k->px; tree[1].ob_y = k->py; tree[1].ob_width = k->pw; tree[1].ob_height = k->ph;
		tree[2].ob_next = 1; tree[2].ob_head = -1; tree[2].ob_tail = -1;
		tree[2].ob_type = G_BOX; tree[2].ob_flags = (uint16_t)(k->cflags | LASTOB); tree[2].ob_state = 0;
		tree[2].ob_spec = CSPEC;
		tree[2].ob_x = k->cx; tree[2].ob_y = k->cy; tree[2].ob_width = k->cw; tree[2].ob_height = k->ch;

		int_in[0] = 1;			/* parent */
		int_in[1] = 2;			/* obj */
		int_in[2] = k->isvert;
		addr_in[0] = tree;
		aes(76, 3, 1, 1);		/* graf_slidebox */
		emit("%s ret=%d out1=%d obj=%d,%d  (%s)\n", k->id, int_out[0], int_out[1],
		     tree[2].ob_x, tree[2].ob_y, k->what);
		emit("I%s pointer=%d,%d\n", k->id + 1, mx, my);
	}
	aes(19, 0, 1, 0);			/* appl_exit */
	flush_log();
	return 0;
}
