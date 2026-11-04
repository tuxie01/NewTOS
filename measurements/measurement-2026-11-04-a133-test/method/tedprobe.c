/*
 * tedprobe.c -- A133 measurement helper (not a permanent test): a GEM program
 * that draws one fixed OBJECT tree of TEDINFO texts with objc_draw and holds
 * it on the screen for two minutes, for a screendump on NewTOS and a Hatari
 * screenshot on the pinned EmuTOS oracle. Same C source on both sides; the
 * AES calls go through the tosdump ABI backend (td_abi_aes), the OBJECT and
 * TEDINFO are the native records of each side (`long ob_spec`, pointers), as
 * in tosdump/probes/sbprobe.c.
 *
 * The tree sits at the absolute position (16,48), so both screens are
 * compared at the same coordinates. ROOT is a G_BOX with the 50 % checker
 * (colour word 0x1141, 1-px inside border); its children, 128x20 each:
 *   column A (x 16)  MD_REPLACE (bit 7 set)   G_TEXT, G_BOXTEXT, G_FTEXT,
 *   column B (x 168) MD_TRANS                 G_FBOXTEXT; boxed kinds filled
 *                                             with pattern 2 in black (0x11A1 /
 *                                             0x1121), te_thickness 1
 *   column C (x 320, 104 wide) 3D: FL3DBAK hollow-white MD_REPLACE G_TEXT and
 *     G_BOXTEXT (0x1180, the 3D switch to MD_TRANS), FL3DBAK hollow-white
 *     MD_TRANS G_FBOXTEXT (0x1100), the file selector's FTITLE look
 *     (G_BOXTEXT FL3DBAK 0x11C1, te_thickness -1, TE_CNTR "*.*")
 *   row 4: G_BOXTEXT FL3DIND SELECTED 0x11A1 (A), G_TEXT SELECTED 0x11A1 (B)
 *   row 5: G_BOXTEXT in the SMALL font, 0x11A1 (A) and 0x1121 (B)
 */
#include <stdint.h>

#include "abi.h"

typedef struct {
	int16_t ob_next, ob_head, ob_tail;
	uint16_t ob_type, ob_flags, ob_state;
	long ob_spec;
	int16_t ob_x, ob_y, ob_width, ob_height;
} OBJ;

typedef struct {
	char *te_ptext, *te_ptmplt, *te_pvalid;
	int16_t te_font, te_junk1, te_just, te_color, te_junk2, te_thickness, te_txtlen, te_tmplen;
} TED;

#define G_BOX      20
#define G_TEXT     21
#define G_BOXTEXT  22
#define G_FTEXT    29
#define G_FBOXTEXT 30
#define LASTOB     0x0020
#define FL3DIND    0x0200
#define FL3DBAK    0x0400
#define SELECTED   0x0001
#define IBM        3
#define SMALL      5
#define TE_LEFT    0
#define TE_CNTR    2

#define RX 16
#define RY 48
#define RW 440
#define RH 176
#define NOBJ 17

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

static char t_text[] = "Text Ab_1";
static char t_box[] = "BoxText Ab";
static char t_fraw[] = "abc";
static char t_ftmpl[] = "F:______";
static char t_star[] = "*.*";
static char t_small[] = "Small text Ab";
static char t_empty[] = "";

static TED ted[16];
static OBJ tree[NOBJ];
static int nted, nobj;

static long mkted(char *text, char *tmpl, int16_t font, int16_t just, uint16_t color, int16_t th)
{
	TED *t = &ted[nted++];

	t->te_ptext = text;
	t->te_ptmplt = tmpl;
	t->te_pvalid = t_empty;
	t->te_font = font;
	t->te_junk1 = 0;
	t->te_just = just;
	t->te_color = (int16_t)color;
	t->te_junk2 = 0;
	t->te_thickness = th;
	t->te_txtlen = 16;
	t->te_tmplen = 16;
	return (long)t;
}

static void child(uint16_t type, uint16_t flags, uint16_t state, long spec, int16_t x, int16_t y, int16_t w)
{
	OBJ *o = &tree[nobj];

	o->ob_next = (int16_t)(nobj + 1);
	o->ob_head = -1;
	o->ob_tail = -1;
	o->ob_type = type;
	o->ob_flags = flags;
	o->ob_state = state;
	o->ob_spec = spec;
	o->ob_x = x;
	o->ob_y = y;
	o->ob_width = w;
	o->ob_height = 20;
	nobj++;
}

static void build(void)
{
	static const uint16_t kinds[4] = { G_TEXT, G_BOXTEXT, G_FTEXT, G_FBOXTEXT };
	int i;

	tree[0].ob_next = -1;
	tree[0].ob_head = 1;
	tree[0].ob_type = G_BOX;
	tree[0].ob_spec = 0x00011141L;
	tree[0].ob_x = RX;
	tree[0].ob_y = RY;
	tree[0].ob_width = RW;
	tree[0].ob_height = RH;
	nobj = 1;
	for (i = 0; i < 4; i++) {
		int f = kinds[i] == G_FTEXT || kinds[i] == G_FBOXTEXT;
		int b = kinds[i] == G_BOXTEXT || kinds[i] == G_FBOXTEXT;
		char *txt = f ? t_fraw : (b ? t_box : t_text);
		char *tm = f ? t_ftmpl : t_empty;

		child(kinds[i], 0, 0, mkted(txt, tm, IBM, TE_LEFT, 0x11A1, 1), 16, (int16_t)(8 + 24 * i), 128);
		child(kinds[i], 0, 0, mkted(txt, tm, IBM, TE_LEFT, 0x1121, 1), 168, (int16_t)(8 + 24 * i), 128);
	}
	child(G_TEXT, FL3DBAK, 0, mkted(t_text, t_empty, IBM, TE_LEFT, 0x1180, 0), 320, 8, 104);
	child(G_BOXTEXT, FL3DBAK, 0, mkted(t_box, t_empty, IBM, TE_LEFT, 0x1180, -1), 320, 32, 104);
	child(G_FBOXTEXT, FL3DBAK, 0, mkted(t_fraw, t_ftmpl, IBM, TE_LEFT, 0x1100, 1), 320, 56, 104);
	child(G_BOXTEXT, FL3DBAK, 0, mkted(t_star, t_empty, IBM, TE_CNTR, 0x11C1, -1), 320, 80, 104);
	child(G_BOXTEXT, FL3DIND, SELECTED, mkted(t_box, t_empty, IBM, TE_LEFT, 0x11A1, 1), 16, 104, 128);
	child(G_TEXT, 0, SELECTED, mkted(t_text, t_empty, IBM, TE_LEFT, 0x11A1, 0), 168, 104, 128);
	child(G_BOXTEXT, 0, 0, mkted(t_small, t_empty, SMALL, TE_LEFT, 0x11A1, 1), 16, 128, 128);
	child(G_BOXTEXT, 0, 0, mkted(t_small, t_empty, SMALL, TE_LEFT, 0x1121, 1), 168, 128, 128);
	tree[nobj - 1].ob_next = 0;
	tree[nobj - 1].ob_flags |= LASTOB;
	tree[0].ob_tail = (int16_t)(nobj - 1);
}

int main(void)
{
	aes(10, 0, 1, 0);			/* appl_init */
	build();
	int_in[0] = 1;				/* wind_update(BEG_UPDATE) */
	aes(107, 1, 1, 0);
	int_in[0] = 256;			/* graf_mouse(M_OFF) */
	addr_in[0] = 0;
	aes(78, 1, 1, 1);
	int_in[0] = 0;				/* objc_draw(tree, ROOT, 8, clip) */
	int_in[1] = 8;
	int_in[2] = RX - 4;
	int_in[3] = RY - 4;
	int_in[4] = RW + 8;
	int_in[5] = RH + 8;
	addr_in[0] = tree;
	aes(42, 6, 1, 1);
	int_in[0] = (int16_t)(120000L & 0xffff);	/* evnt_timer(120 s) */
	int_in[1] = (int16_t)(120000L >> 16);
	aes(24, 2, 1, 0);
	int_in[0] = 257;			/* graf_mouse(M_ON) */
	addr_in[0] = 0;
	aes(78, 1, 1, 1);
	int_in[0] = 0;				/* wind_update(END_UPDATE) */
	aes(107, 1, 1, 0);
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
