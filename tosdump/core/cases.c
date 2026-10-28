/*
 * cases.c -- case table loader (spec section 6).  See cases.h for the format.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cases.h"

/* ------------------------------------------------------------------ */
/* Line iteration -- one rule, used by the parser and by the host tools */
/* ------------------------------------------------------------------ */

void td_line_iter_init(struct td_line_iter *it, const char *buf, size_t len)
{
	it->buf     = buf;
	it->len     = len;
	it->pos     = 0;
	it->line_no = 0;
	it->done    = 0;
}

int td_line_iter_next(struct td_line_iter *it, const char **start,
                      size_t *len, uint16_t *line_no, int *is_case)
{
	size_t begin, n, i;

	if (it->done)
		return 0;

	begin = it->pos;
	while (it->pos < it->len && it->buf[it->pos] != '\n')
		it->pos++;
	n = it->pos - begin;
	if (it->pos < it->len)
		it->pos++;                 /* step over the newline */
	else
		it->done = 1;              /* that was the last line */

	it->line_no++;

	/* a table edited on the Atari carries CRLF */
	if (n > 0 && it->buf[begin + n - 1] == '\r')
		n--;

	*start   = it->buf + begin;
	*len     = n;
	*line_no = it->line_no;

	/* blank, comment and directive lines carry no call, but they do
	 * consume an id: a case id is a physical line number and nothing may
	 * silently renumber the cases.  '!' is here rather than only in the
	 * AES parser so that host/casefile.c and the VDI and raster parsers
	 * agree with it about which line carries which case id. */
	i = 0;
	while (i < n && (it->buf[begin + i] == ' ' || it->buf[begin + i] == '\t'))
		i++;
	*is_case = !(i == n || it->buf[begin + i] == '#' ||
	             it->buf[begin + i] == '!');
	return 1;
}

/* ------------------------------------------------------------------ */
/* Column and value parsing                                            */
/* ------------------------------------------------------------------ */

/*
 * Splits into at most `ncols` columns; the last one keeps its tabs.  The
 * count is per area -- five for the VDI form, seven for the AES form -- and
 * cannot be derived from the line, because the comment column may itself
 * hold tabs.
 */
static int split_columns(const char *line, size_t len,
                         const char **col, size_t *col_len, int ncols)
{
	int n = 0;
	size_t i = 0, start = 0;

	for (;;) {
		if (n == ncols - 1) {
			col[n]     = line + i;
			col_len[n] = len - i;
			return n + 1;
		}
		if (i >= len) {
			col[n]     = line + start;
			col_len[n] = i - start;
			return n + 1;
		}
		if (line[i] == '\t') {
			col[n]     = line + start;
			col_len[n] = i - start;
			n++;
			i++;
			start = i;
			continue;
		}
		i++;
	}
}

static void trim(const char **s, size_t *len)
{
	while (*len > 0 && ((*s)[0] == ' ' || (*s)[0] == '\t')) {
		(*s)++;
		(*len)--;
	}
	while (*len > 0 && ((*s)[*len - 1] == ' ' || (*s)[*len - 1] == '\t'))
		(*len)--;
}

/*
 * Parses one decimal or 0x-hex value.  Returns 0, or an error code.
 * The accepted range is -32768..65535: a case table states bit patterns as
 * often as it states numbers (0xFFFF for an all-ones line-style mask), and
 * both spellings must reach the array as the same 16 bits.
 */
static int parse_value(const char *s, size_t len, int16_t *out)
{
	int negative = 0;
	unsigned long v = 0;
	size_t i = 0;
	int base = 10, digits = 0;

	trim(&s, &len);
	if (len == 0)
		return TD_CASE_ERR_VALUE;

	if (s[0] == '-') {
		negative = 1;
		i = 1;
	} else if (s[0] == '+') {
		i = 1;
	}
	if (len - i > 2 && s[i] == '0' && (s[i + 1] == 'x' || s[i + 1] == 'X')) {
		base = 16;
		i += 2;
	}

	for (; i < len; i++) {
		int d;
		char c = s[i];

		if (c >= '0' && c <= '9')
			d = c - '0';
		else if (base == 16 && c >= 'a' && c <= 'f')
			d = c - 'a' + 10;
		else if (base == 16 && c >= 'A' && c <= 'F')
			d = c - 'A' + 10;
		else
			return TD_CASE_ERR_VALUE;

		v = v * (unsigned long)base + (unsigned long)d;
		digits++;
		if (v > 0x80000000uL)          /* far past anything representable */
			return TD_CASE_ERR_RANGE;
	}
	if (digits == 0)
		return TD_CASE_ERR_VALUE;

	if (negative) {
		if (v > 32768uL)
			return TD_CASE_ERR_RANGE;
		*out = (int16_t)(-(long)v);
	} else {
		if (v > 65535uL)
			return TD_CASE_ERR_RANGE;
		*out = (int16_t)(uint16_t)v;
	}
	return 0;
}

/* "-" means an empty list. */
static int parse_list(const char *s, size_t len, int16_t *dst, uint16_t max,
                      uint16_t *count)
{
	size_t i = 0, start = 0;
	uint16_t n = 0;

	*count = 0;
	trim(&s, &len);
	if (len == 0)
		return TD_CASE_ERR_VALUE;
	if (len == 1 && s[0] == '-')
		return 0;

	for (;;) {
		if (i == len || s[i] == ',') {
			int16_t v;
			int rc;

			if (n >= max)
				return TD_CASE_ERR_TOO_MANY_VALUES;
			rc = parse_value(s + start, i - start, &v);
			if (rc != 0)
				return rc;
			dst[n++] = v;
			if (i == len)
				break;
			i++;
			start = i;
			continue;
		}
		i++;
	}
	*count = n;
	return 0;
}

/* ------------------------------------------------------------------ */
/* Table parsing                                                       */
/* ------------------------------------------------------------------ */

static void fail(struct td_case_error *err, uint16_t line, const char *what)
{
	if (err) {
		err->line = line;
		err->what = what;
	}
}

int td_cases_parse(struct td_case_table *t, const char *buf, size_t len,
                   struct td_case_error *err)
{
	struct td_line_iter it;
	const char *line;
	size_t line_len;
	uint16_t line_no;
	uint16_t pending = 0;      /* preludes seen since the last case */
	uint16_t last_prelude = 0; /* line number of the newest one, for errors */
	int is_case;

	memset(t, 0, sizeof *t);
	t->crc = td_crc32((const uint8_t *)buf, len);
	if (err) {
		err->line = 0;
		err->what = NULL;
	}

	td_line_iter_init(&it, buf, len);
	while (td_line_iter_next(&it, &line, &line_len, &line_no, &is_case)) {
		const char *col[TD_CASE_COLUMNS];
		size_t col_len[TD_CASE_COLUMNS];
		struct td_case *c;
		int16_t v;
		int rc, ncol;

		if (!is_case)
			continue;

		ncol = split_columns(line, line_len, col, col_len,
		                     TD_CASE_COLUMNS);
		if (ncol < TD_CASE_COLUMNS) {
			fail(err, line_no, "needs five tab separated columns");
			return TD_CASE_ERR_COLUMNS;
		}

		/* A '+' opcode marks a prelude: a call issued after the reset and
		 * before the next case's measured call, and not recorded.  It is
		 * what lets a case prove an effect rather than only an echo. */
		{
			const char *o = col[0];
			size_t olen = col_len[0];

			trim(&o, &olen);
			if (td_case_line_is_prelude(line, line_len)) {
				struct td_prelude *p;

				if (pending >= TD_MAX_PRELUDES ||
				    t->prelude_count >= TD_MAX_PRELUDES) {
					fail(err, line_no, "more preludes than TD_MAX_PRELUDES");
					return TD_CASE_ERR_TOO_MANY_PRE;
				}
				p = &t->preludes[t->prelude_count + pending];
				memset(p, 0, sizeof *p);

				rc = parse_value(o + 1, olen - 1, &v);
				if (rc != 0 || v < 0) {
					fail(err, line_no, "prelude opcode is not a number");
					return rc != 0 ? rc : TD_CASE_ERR_RANGE;
				}
				p->opcode = v;

				rc = parse_value(col[1], col_len[1], &v);
				if (rc != 0 || v < 0) {
					fail(err, line_no, "prelude sub-opcode is not a number");
					return rc != 0 ? rc : TD_CASE_ERR_RANGE;
				}
				p->sub = v;

				rc = parse_list(col[2], col_len[2], p->intin,
				                TD_PRELUDE_VALUES, &p->n_intin);
				if (rc != 0) {
					fail(err, line_no, "prelude intin column");
					return rc;
				}
				rc = parse_list(col[3], col_len[3], p->ptsin,
				                TD_PRELUDE_VALUES, &p->n_ptsin);
				if (rc != 0) {
					fail(err, line_no, "prelude ptsin column");
					return rc;
				}
				if (p->n_ptsin & 1) {
					fail(err, line_no, "prelude ptsin holds an odd number of "
					                   "values, so it cannot be a point list");
					return TD_CASE_ERR_ODD_PTSIN;
				}
				pending++;
				last_prelude = line_no;
				continue;
			}
		}

		if (t->count >= TD_MAX_CASES) {
			fail(err, line_no, "more cases than TD_MAX_CASES");
			return TD_CASE_ERR_TOO_MANY_CASES;
		}

		c = &t->cases[t->count];
		memset(c, 0, sizeof *c);
		c->line = line_no;
		c->prelude_first = t->prelude_count;
		c->prelude_count = pending;
		t->prelude_count = (uint16_t)(t->prelude_count + pending);
		pending = 0;

		rc = parse_value(col[0], col_len[0], &v);
		if (rc != 0) {
			fail(err, line_no, "opcode is not a number");
			return rc;
		}
		if (v < 0) {
			fail(err, line_no, "opcode is negative");
			return TD_CASE_ERR_RANGE;
		}
		c->opcode = v;

		rc = parse_value(col[1], col_len[1], &v);
		if (rc != 0) {
			fail(err, line_no, "sub-opcode is not a number");
			return rc;
		}
		if (v < 0) {
			fail(err, line_no, "sub-opcode is negative");
			return TD_CASE_ERR_RANGE;
		}
		c->sub = v;

		rc = parse_list(col[2], col_len[2], c->intin, TD_VDI_INTIN_N,
		                &c->n_intin);
		if (rc != 0) {
			fail(err, line_no, "intin column");
			return rc;
		}

		rc = parse_list(col[3], col_len[3], c->ptsin, TD_VDI_PTSIN_N,
		                &c->n_ptsin);
		if (rc != 0) {
			fail(err, line_no, "ptsin column");
			return rc;
		}
		if (c->n_ptsin & 1) {
			fail(err, line_no, "ptsin holds an odd number of values, so it "
			                   "cannot be a list of x,y points");
			return TD_CASE_ERR_ODD_PTSIN;
		}

		td_case_line_comment(line, line_len, c->comment, TD_COMMENT_LEN);

		t->count++;
	}

	/* A prelude with nothing to precede would silently never run. */
	if (pending > 0) {
		fail(err, last_prelude, "prelude has no case after it");
		return TD_CASE_ERR_DANGLING;
	}

	return 0;
}

/* ------------------------------------------------------------------ */
/* AES table parsing                                                   */
/* ------------------------------------------------------------------ */
/*
 * Seven columns instead of five, and the four count columns are the
 * difference that matters: in the AES every control slot is an input
 * (EmuTOS aes/gemsuper.c:435 copies control[] wholesale onto a local buffer
 * and never writes it back), and the dispatcher's marshalling is driven by
 * those counts alone -- min(IN_LEN, I_SIZE) words in, exactly OUT_LEN words
 * back out (gemsuper.c:436-445).  Stating them separately from the values a
 * case lists is what lets a case measure that rule.
 */

/*
 * ------------------------------------------------------------------
 * The refusal guard
 * ------------------------------------------------------------------
 *
 * A case table is data.  A typo in it must not hang the machine it is
 * carried to, and it must not be mistakable for a real defect once it does.
 * So the loader refuses the whole table -- named reason, non-zero return,
 * the offending physical line -- when a case or a prelude names an opcode
 * from one of three classes:
 *
 *   1. it stops the run finishing.  Most of these wait on the operator; two
 *      (appl_exit, shel_write) end or replace the application instead, which
 *      is the same outcome by a different route.
 *   2. whether the machine has it is UNESTABLISHED and no measurement can
 *      settle it.  That is the four CONF_WITH_PCGEM opcodes: the flag is
 *      absent from EmuTOS' AES_VERSION ladder, so ap_version says nothing
 *      about them, and an opcode the machine lacks reaches crysbind's
 *      default arm, which pops fm_show(ALNOFUNC) -- a modal alert waiting
 *      for a click (gemsuper.c:411-417).
 *   3. it is not an AES opcode at all.  Same default arm, same alert.
 *      Opcode 0 is the single exception: gemsuper.c:414 skips the alert for
 *      it explicitly ("Ignore zero since some PRGs are this call").
 *
 * WHAT IS NO LONGER HERE, ONE.  menu_popup/attach/istart/settings (36-39)
 * and objc_sysvar (48) are compiled out of a smaller build too, but
 * ap_version DOES settle those -- the flags that gate them are the ladder's
 * own (config.h:1345-1354).  They are therefore not a property of the table
 * and not the loader's to refuse: the loader records the capability they
 * need in the table's `features`, and the machine decides, after appl_init,
 * in td_aes_cases_version_refusal below.  Until 2026-09-13 they were refused
 * here on the assumption "TOS 3.06 = AES 3.20", which locked out functions
 * the reference ROM measurably HAS (it reports 0x0340).  See cases.h, the
 * version guard, and TASK/M06.md.
 *
 * WHAT IS NO LONGER HERE, TWO.  evnt_multi (25) and form_error (53) each
 * have ONE arm that returns before the blocking one, and which arm a call
 * takes is decided by its PARAMETERS, not by its opcode:
 *
 *   ev_multi's quick check timer -- MU_TIMER among the flags with a zero
 *      MT_LOCOUNT/MT_HICOUNT -- sets `what` before the `if (what == 0)`
 *      block that arms the events and sleeps (gemevlib.c:216-220 against
 *      :232-262).
 *   fm_error returns FALSE for n > 63 before reaching fm_show at all
 *      (gemfmlib.c:440-441 against :469).
 *
 * A guard that can only see the opcode has to refuse both, and refusing the
 * one non-blocking arm of a function is refusing the only thing about it
 * that is measurable.  They moved to aes_line_parameter_refusal below, which
 * sees the values -- and which, in the same move, took over the opposite
 * duty for four opcodes this list waves through: see its own header.
 *
 * There is no override mechanism, deliberately.  Taking an opcode off this
 * list is a source change with a citation, and that friction is the point:
 * the alternative is a data file hanging a machine with no way to tell that
 * apart from a defect in the thing being measured.
 *
 * The table below is also the complete list of opcodes that HAVE an arm in
 * crysbind (aes/gemsuper.c:87-409, 79 of them); `refusal` is NULL for the
 * ones a table may issue -- subject, for the five in aes_version_gated[], to
 * the machine's answer -- and the reason otherwise.  Anything absent from it
 * is class 3.
 */
struct td_aes_opcode {
	int16_t     opcode;
	const char *refusal;      /* NULL = may be issued */
};

/* class 1, sub-reason a: waits on the operator */
#define BLOCKS "blocks on the operator: "
/* class 1, sub-reason b: the run never gets back here */
#define FATAL  "never returns to the run: "
/* class 2: EmuTOS does not say whether TOS 3.06 has it, and no measurement
 * can ask.  CONF_WITH_PCGEM gates these four (gemsuper.c:115, :315,
 * :398-403) but does NOT appear in the AES_VERSION ladder
 * (config.h:1345-1354), so ap_version -- the one capability the machine
 * reports -- cannot tell us either way.  If TOS 3.06 lacks them the
 * dispatcher pops the modal alert, which on the TT means a run that never
 * finishes and a hang indistinguishable from a real defect.  Refused until a
 * primary source settles it -- Compendium or Profibuch, per CLAUDE.md rule
 * 1's secondary cross-references; EmuTOS alone cannot. */
#define PCGEM  "PC-GEM function whose presence in TOS 3.06 is unestablished "  \
               "(#if CONF_WITH_PCGEM, which is not in the AES_VERSION "        \
               "ladder); if absent, the default arm pops a modal alert: "

static const struct td_aes_opcode aes_opcodes[] = {
	{  10, NULL },                                   /* appl_init      */
	{  11, BLOCKS "appl_read waits on an empty message queue "
	              "(gemsuper.c:100-103, gemaplib.c:88, gemqueue.c:99)" },
	{  12, NULL },                                   /* appl_write     */
	{  13, NULL },                                   /* appl_find      */
	{  14, NULL },                                   /* appl_tplay     */
	{  15, BLOCKS "appl_trecord loops until the operator has filled the "
	              "event buffer (gemaplib.c:216-217)" },
	{  17, PCGEM "appl_yield (gemsuper.c:115-119)" },
	{  19, FATAL  "appl_exit ends the application (gemsuper.c:120-123)" },
	{  20, BLOCKS "evnt_keybd waits for a key (gemsuper.c:127)" },
	{  21, BLOCKS "evnt_button waits for a mouse button (gemevlib.c:90)" },
	{  22, BLOCKS "evnt_mouse waits for the mouse to enter or leave a "
	              "rectangle (gemevlib.c:123-125)" },
	{  23, BLOCKS "evnt_mesag waits for a message (gemevlib.c:100-116, "
	              "gemaplib.c:88)" },
	{  24, NULL },                                   /* evnt_timer     */
	{  25, NULL },                                   /* evnt_multi *   */
	{  26, NULL },                                   /* evnt_dclick    */
	{  30, NULL },                                   /* menu_bar       */
	{  31, NULL },                                   /* menu_icheck    */
	{  32, NULL },                                   /* menu_ienable   */
	{  33, NULL },                                   /* menu_tnormal   */
	{  34, NULL },                                   /* menu_text      */
	{  35, NULL },                                   /* menu_register  */
	{  36, NULL },                                   /* menu_popup     */
	{  37, NULL },                                   /* menu_attach    */
	{  38, NULL },                                   /* menu_istart    */
	{  39, NULL },                                   /* menu_settings  */
	{  40, NULL },                                   /* objc_add       */
	{  41, NULL },                                   /* objc_delete    */
	{  42, NULL },                                   /* objc_draw      */
	{  43, NULL },                                   /* objc_find      */
	{  44, NULL },                                   /* objc_offset    */
	{  45, NULL },                                   /* objc_order     */
	{  46, NULL },                                   /* objc_edit      */
	{  47, NULL },                                   /* objc_change    */
	{  48, NULL },                                   /* objc_sysvar    */
	{  50, BLOCKS "form_do runs a modal ev_multi(MU_KEYBD|MU_BUTTON) loop "
	              "with a zero timer (gemfmlib.c:312)" },
	{  51, NULL },                                   /* form_dial      */
	{  52, BLOCKS "form_alert ends in fm_do (gemfmalt.c:383)" },
	{  53, NULL },                                   /* form_error *   */
	{  54, NULL },                                   /* form_center    */
	{  55, NULL },                                   /* form_keybd     */
	{  56, BLOCKS "form_button waits for the button to come up, through "
	              "gr_watchbox or ev_button (gemfmlib.c:251, :256)" },
	{  70, BLOCKS "graf_rubbox tracks the mouse until the button comes up "
	              "(gemgrlib.c:211-215)" },
	{  71, BLOCKS "graf_dragbox tracks the mouse until the button comes up "
	              "(gemgrlib.c:257-263)" },
	{  72, NULL },                                   /* graf_mbox      */
	{  73, NULL },                                   /* graf_growbox   */
	{  74, NULL },                                   /* graf_shrinkbox */
	{  75, BLOCKS "graf_watchbox loops on gr_stilldn until the button "
	              "comes up (gemgrlib.c:348-352)" },
	{  76, BLOCKS "graf_slidebox is gr_dragbox (gemgrlib.c:388)" },
	{  77, NULL },                                   /* graf_handle    */
	{  78, NULL },                                   /* graf_mouse     */
	{  79, NULL },                                   /* graf_mkstate   */
	{  80, NULL },                                   /* scrp_read      */
	{  81, NULL },                                   /* scrp_write     */
	{  82, PCGEM "scrp_clear (gemsuper.c:315-319)" },
	{  90, BLOCKS "fsel_input runs the modal file selector, fs_input -> "
	              "fm_do (gemfslib.c:718)" },
	{  91, BLOCKS "fsel_exinput runs the modal file selector, fs_input -> "
	              "fm_do (gemfslib.c:718)" },
	{ 100, NULL },                                   /* wind_create    */
	{ 101, NULL },                                   /* wind_open      */
	{ 102, NULL },                                   /* wind_close     */
	{ 103, NULL },                                   /* wind_delete    */
	{ 104, NULL },                                   /* wind_get       */
	{ 105, NULL },                                   /* wind_set       */
	{ 106, NULL },                                   /* wind_find      */
	{ 107, NULL },                                   /* wind_update    */
	{ 108, NULL },                                   /* wind_calc      */
	{ 109, NULL },                                   /* wind_new       */
	{ 110, NULL },                                   /* rsrc_load      */
	{ 111, NULL },                                   /* rsrc_free      */
	{ 112, NULL },                                   /* rsrc_gaddr     */
	{ 113, NULL },                                   /* rsrc_saddr     */
	{ 114, NULL },                                   /* rsrc_obfix     */
	{ 120, NULL },                                   /* shel_read      */
	{ 121, FATAL  "shel_write sets sh_doexec / sh_nextapp, so the AES "
	              "launches something else instead of finishing the run "
	              "(gemshlib.c:157)" },
	{ 122, NULL },                                   /* shel_get       */
	{ 123, NULL },                                   /* shel_put       */
	{ 124, NULL },                                   /* shel_find      */
	{ 125, NULL },                                   /* shel_envrn     */
	{ 126, PCGEM "shel_rdef (gemsuper.c:398-401)" },
	{ 127, PCGEM "shel_wdef (gemsuper.c:398, :402-403)" }
};

#undef BLOCKS
#undef FATAL

#define TD_AES_OPCODE_COUNT \
	((int)(sizeof aes_opcodes / sizeof aes_opcodes[0]))

/*
 * NULL if this opcode may be issued, otherwise the reason it may not.
 * Opcode 0 is allowed by name: it is the one unknown opcode the dispatcher
 * handles silently (gemsuper.c:414).
 */
static const char *aes_opcode_refusal(int16_t opcode)
{
	int i;

	if (opcode == 0)
		return NULL;

	for (i = 0; i < TD_AES_OPCODE_COUNT; i++)
		if (aes_opcodes[i].opcode == opcode)
			return aes_opcodes[i].refusal;

	return "not an AES opcode: crysbind has no arm for it, so it would "
	       "reach the default arm and pop a modal alert "
	       "(gemsuper.c:406-417)";
}

/*
 * ------------------------------------------------------------------
 * The version guard: what the MACHINE reports, not what it is assumed
 * ------------------------------------------------------------------
 *
 * Five of the opcodes above have an arm in crysbind only when a
 * CONF_WITH_* flag is on, and those two flags ARE in EmuTOS' AES_VERSION
 * ladder (include/config.h:1345-1354).  So the presence of these five is
 * readable off a number the machine itself hands out -- ap_version, written
 * by appl_init (gemsuper.c:92) -- and there is no need to assume anything.
 *
 * See cases.h (the version guard) for why the decision is here and not in
 * the loader, and for why membership rather than ">=".
 */
struct td_aes_version_gate {
	int16_t  opcode;
	uint16_t feature;
};

static const struct td_aes_version_gate aes_version_gated[] = {
	{ 36, TD_AES_FEAT_MENU_EXT },     /* menu_popup, gemsuper.c:188-191   */
	{ 37, TD_AES_FEAT_MENU_EXT },     /* menu_attach, gemsuper.c:192-194  */
	{ 38, TD_AES_FEAT_MENU_EXT },     /* menu_istart, gemsuper.c:195-197  */
	{ 39, TD_AES_FEAT_MENU_EXT },     /* menu_settings, gemsuper.c:198-200*/
	{ 48, TD_AES_FEAT_3D_OBJECTS }    /* objc_sysvar, gemsuper.c:232-236  */
};

#define TD_AES_VERSION_GATE_COUNT \
	((int)(sizeof aes_version_gated / sizeof aes_version_gated[0]))

/* The capability this opcode needs, or 0 for the other 74. */
static uint16_t aes_opcode_feature(int16_t opcode)
{
	int i;

	for (i = 0; i < TD_AES_VERSION_GATE_COUNT; i++)
		if (aes_version_gated[i].opcode == opcode)
			return aes_version_gated[i].feature;
	return 0;
}

/*
 * Does an AES reporting `ap_version` have this capability?
 *
 * The answer is read off the ladder's own arms and nothing else.  0x0340 is
 * the only rung that requires CONF_WITH_3D_OBJECTS; 0x0340 and 0x0330 are
 * the two that require CONF_WITH_MENU_EXTENSION (config.h:1346-1349).  A
 * number the ladder never produces -- 0x0331, or anything above 0x0340 --
 * establishes neither: the ladder is the only thing that maps a version onto
 * a feature set, and it does not map those.  Refusing there costs a table
 * that could perhaps have run; issuing there costs a hung machine.
 */
static int feature_established(uint16_t feature, int16_t ap_version)
{
	uint16_t v = (uint16_t)ap_version;

	switch (feature) {
	case TD_AES_FEAT_3D_OBJECTS:
		return v == 0x0340;
	case TD_AES_FEAT_MENU_EXT:
		return v == 0x0340 || v == 0x0330;
	default:
		return 1;
	}
}

/* Why an AES reporting this version may not be handed the opcode. */
static const char *feature_refusal(uint16_t feature)
{
	switch (feature) {
	case TD_AES_FEAT_3D_OBJECTS:
		return "objc_sysvar is #if CONF_WITH_3D_OBJECTS (gemsuper.c:232-236) "
		       "and this AES does not report the one version that "
		       "establishes it, 0x0340 (config.h:1346-1347); the "
		       "dispatcher's default arm would pop a modal alert";
	case TD_AES_FEAT_MENU_EXT:
		return "the menu extension (36-39) is #if CONF_WITH_MENU_EXTENSION "
		       "(gemsuper.c:188-200) and this AES does not report one of the "
		       "two versions that establish it, 0x0330 or 0x0340 "
		       "(config.h:1346-1349); the dispatcher's default arm would pop "
		       "a modal alert";
	default:
		return "an unnamed capability";
	}
}

const char *td_aes_features_text(uint16_t features)
{
	switch (features & (TD_AES_FEAT_3D_OBJECTS | TD_AES_FEAT_MENU_EXT)) {
	case TD_AES_FEAT_3D_OBJECTS:
		return "3D objects (CONF_WITH_3D_OBJECTS, AES 0x0340)";
	case TD_AES_FEAT_MENU_EXT:
		return "the menu extension (CONF_WITH_MENU_EXTENSION, "
		       "AES 0x0330 or 0x0340)";
	case TD_AES_FEAT_3D_OBJECTS | TD_AES_FEAT_MENU_EXT:
		return "3D objects (AES 0x0340) and the menu extension "
		       "(AES 0x0330 or 0x0340)";
	default:
		return "nothing this AES could lack";
	}
}

int td_aes_cases_version_refusal(const struct td_aes_case_table *t,
                                 int16_t ap_version,
                                 struct td_case_error *err)
{
	uint16_t i;

	if (err) {
		err->line = 0;
		err->what = NULL;
	}

	/* Every table written before the version guard needs nothing, so
	 * nothing about the machine can refuse it. */
	if (t->features == 0)
		return 0;

	for (i = 0; i < t->count; i++) {
		const struct td_aes_case *c = &t->cases[i];
		uint16_t k, feature;

		/* the case's preludes first, in the order the runner issues
		 * them; a prelude has no line of its own, so the case's line is
		 * what names it */
		for (k = 0; k < c->prelude_count; k++) {
			feature = aes_opcode_feature(
				t->preludes[c->prelude_first + k].opcode);
			if (feature && !feature_established(feature, ap_version)) {
				fail(err, c->line, feature_refusal(feature));
				return TD_CASE_ERR_AES_VERSION;
			}
		}

		feature = aes_opcode_feature(c->opcode);
		if (feature && !feature_established(feature, ap_version)) {
			fail(err, c->line, feature_refusal(feature));
			return TD_CASE_ERR_AES_VERSION;
		}
	}

	return 0;
}

/* One AES line's payload, shared by the case and the prelude branches so
 * the two cannot drift apart. */
struct td_aes_line {
	int16_t  opcode;
	int16_t  n_intin;
	int16_t  n_intout;
	int16_t  n_addrin;
	int16_t  n_addrout;
	uint16_t n_values;
	uint16_t n_plants;
	int16_t  intin[TD_AES_INTIN_N];
	struct td_aes_plant plant[TD_AES_PLANTS_MAX];
};

/*
 * Does this line point `kind`/`where` at a buffer?  The parameter guard asks
 * it about the places a pointer can arrive, so a refusal can say "not
 * without a buffer" instead of "never".
 */
static int line_plants(const struct td_aes_line *l, int16_t kind,
                       int16_t where)
{
	uint16_t i;

	for (i = 0; i < l->n_plants; i++)
		if (l->plant[i].kind == kind && l->plant[i].where == where)
			return 1;
	return 0;
}

/*
 * Does addr_in[slot] REACH the handler holding one of the dumper's buffers?
 *
 * Two conditions, and both are needed: the addr_in column has to name a
 * buffer for that slot, and the case has to declare AIN_LEN past it, because
 * xif copies min(AIN_LEN, AI_SIZE) slots onto a LOCAL array
 * (gemsuper.c:428-440) and the handler reads the local one.  A plant the
 * case does not declare hands the handler uninitialised stack, which is
 * worse than the poison it replaced: the poison is at least the same number
 * on both machines.
 */
static int line_has_buffer(const struct td_aes_line *l, int16_t slot)
{
	return line_plants(l, TD_AES_PLANT_ADDRIN, slot) && l->n_addrin > slot;
}

/*
 * ------------------------------------------------------------------
 * The parameter guard
 * ------------------------------------------------------------------
 *
 * The opcode guard above answers "may this call be issued at all".  For a
 * dozen opcodes that question has no answer, because the hazard is not in
 * the opcode but in the VALUES: the same call is a poll or a hang, a read of
 * the dumper's own buffer or a write through 0xDEADDEAD, depending on one
 * WORD of int_in or on whether the addr_in column named a buffer.  An
 * opcode-level verdict on those is wrong in both directions at once -- it
 * refuses the harmless arm and it waves the dangerous one through -- and
 * this is where the second question is answered instead.
 *
 * Everything here is decided from ONE LINE, which is why it lives in the
 * loader and not in td_aes_cases_version_refusal: the flags, the field, the
 * error number, the character, the object index and the plants are all
 * columns of the line.  Nothing here needs a measured quantity, so nothing
 * here needs a machine.  (M06 split the refusal at exactly that seam; M07
 * lands on the loader side of it.  TASK/M06.md, TASK/M07.md.)
 *
 * ==================================================================
 * PART ONE: the window manager (round 2)
 * ==================================================================
 *
 * Three window-manager opcodes need the second question because for them the
 * hazard is in the parameters, not in the opcode:
 *
 * 1. FIELDS THAT TAKE A POINTER OR AN UNCHECKED INDEX.  wind_get and
 *    wind_set select a field with int_in[1], and four of those fields are
 *    unusable from a pointer-free table:
 *      WF_NAME (2), WF_INFO (3) -- the classic binding passes a string
 *        pointer in int_in[2..3] (gemlib a_wind_get.c:344-347,
 *        a_wind_set.c:314-322).  EmuTOS' wm_get has no arm for them and
 *        returns FALSE (gemwmlib.c:1592), but NewTOS copies the title out
 *        through that pointer (tos-api/src/aes/window.rs, wf_str_ptr), and
 *        wm_set stores it in gl_aname.te_ptext where the next frame draw
 *        dereferences it (gemwmlib.c:1657-1666).  The pointer would be
 *        0xDEAD-poison -- UNLESS the table's addr_in column plants one of
 *        the dumper's own buffers there (cases.h).  These two are the one
 *        pair of fields the plant re-opens; the two below it does not.
 *      WF_COLOR (18), WF_DCOLOR (19) -- gadget = pinwds[0] indexes
 *        w_tcolor[] / gl_wtcolor[] with NO range check (gemwmlib.c:1567-1576
 *        for the get, :1714-1727 for the set).  Poison is -8531.
 *    and one more only on wind_set:
 *      WF_NEWDESK (14) -- stores a tree pointer in gl_newdesk
 *        (gemwmlib.c:1683), which w_drawdesk then walks
 *        (gemwmlib.c:340-350).  wm_new does NOT clear it; only wm_init does
 *        (:1296-1300), so it also survives the reset.
 *    wind_get(WF_NEWDESK) is a different call: EmuTOS has no arm for it and
 *    returns FALSE without touching a pointer, and round 1 measures exactly
 *    that.
 *
 * 2. THE MESSAGE-QUEUE BOUND -- see TD_AES_MAX_OPEN_WINDOWS in cases.h for
 *    the mechanism.  The guard counts the distinct handles the table names
 *    in int_in[0] of every wind_open, over the whole table.
 *
 * 3. THE MOUSE-CONTROL BALANCE.  wind_update mode 2 (END_MCTRL,
 *    aesdefs.h:559-562) reaches fm_own(0) -- wm_update subtracts 2 and
 *    passes the rest on (gemwmlib.c:1789-1795) -- which decrements ml_ocnt
 *    unconditionally (gemfmlib.c:99).
 *    One without a matching mode 3 (BEG_MCTRL) drives it to -1, and wm_new only drains
 *    it `while (ml_ocnt > 0)` (gemwmlib.c:1880) -- so the reset does not
 *    heal it, and from then on every BEG/END pair is off by one: the BEG
 *    that should save gl_mntree and the mouse owner does not, and the END
 *    that should restore them does, with whatever ml_mnhold and ml_pmown
 *    last held.  The screen-sync half (modes 0 and 1) DOES heal, because
 *    unsync returns early at sy_tas <= 0 (gemflag.c:104-108), so it is not
 *    constrained.
 *
 * And one shape rule that makes the other three total: where the guard has
 * to read a value (the field, the handle, the mode), the case must state
 * it.  A case that leaves it poisoned would hide the very thing being
 * checked.
 *
 * ==================================================================
 * PART TWO: the rest, both directions (M07)
 * ==================================================================
 *
 * THE SHAPE RULE IS THE SAME ONE, and it is worth restating with the reason
 * it is not merely tidiness.  Where the guard reads an int_in WORD, the case
 * must both STATE it and declare IN_LEN past it.  xif's int_in is a LOCAL
 * array and it is copied into only as far as min(IN_LEN, I_SIZE)
 * (gemsuper.c:428-438), so a WORD outside IN_LEN is not "the previous
 * call's value" -- it is uninitialised stack, and the guard would be
 * reasoning about a number that does not exist.
 *
 * TOO COARSE, AND WHAT OPENS.  Two opcodes used to stand in the opcode list
 * above with their only measurable arm locked out with them:
 *
 *   evnt_multi (25) is a hang for every parameter set but one.  The quick
 *      check timer -- MU_TIMER among the flags with MT_LOCOUNT/MT_HICOUNT
 *      zero -- sets `what` at gemevlib.c:216-220, which is BEFORE the
 *      `if (what == 0)` block that arms the events and calls mwait
 *      (:232-262).  That is the one guaranteed return.  Everything else
 *      reaches mwait(flags), and flags == 0 is the sharpest case of it, not
 *      an exemption: mwait tests `!(mask & rlr->p_evflg)`, which is
 *      trivially true for 0, sets WAITIN and calls dsptch()
 *      (gemasync.c:117-125).  No event can ever match an empty mask, so
 *      nothing wakes the process.  The guard therefore has to be able to say
 *      both "yes" and "no" about the same opcode.
 *      MU_MESAG is a separate hazard on the same line: the quick check
 *      message calls ev_mesag(mebuff) as soon as anything is queued
 *      (gemevlib.c:221-229), mebuff is MME_PBUFF = addr_in[0], and it writes
 *      sixteen bytes there.  The dumper never drains the WM_REDRAWs its own
 *      wind_opens generate (TD_AES_MAX_OPEN_WINDOWS, cases.h), so "anything
 *      queued" is the normal state, not the exception.  MU_MESAG therefore
 *      needs a planted buffer even though the timer already guarantees the
 *      return.
 *
 *   form_error (53) returns FALSE for n > 63 before it looks at anything
 *      else (gemfmlib.c:440-441).  Every other n falls through the switch to
 *      fm_show -> fm_alert -> fm_do (:469, :400, gemfmalt.c:383), the modal
 *      alert.  One WORD decides which.
 *
 * TOO LEAKY, AND WHAT CLOSES.  Four opcodes the list waves through do all of
 * their work behind addr_in[0], and until the addr_in column existed (M05)
 * that pointer could only ever be the poison.  Nothing refused them; only
 * care in writing the tables kept them out.  0xDEADDEAD is outside every RAM
 * range of a TT, so each is a bus error or a scribble on a running machine,
 * and the damage would look like a defect in the thing being measured:
 *
 *   scrp_read (80) is strcpy(pscrap, D.g_scrap) (gemsclib.c:40-45) -- a
 *      WRITE of up to LEN_ZPATH = 114 bytes (sysconf.h:51) through the
 *      pointer.  scrp_write (81) is strcpy(D.g_scrap, pscrap) (:57-62) --
 *      an unbounded READ through it, into a 114 byte array.  A planted
 *      buffer satisfies both by construction: it is 256 bytes, and it holds
 *      a NUL at TD_AES_BUF_TEXT_LEN so the read is nine bytes long
 *      (tosdump.h).  With the plant they are ordinary calls; the guard
 *      therefore says "not without a buffer", not "never".
 *
 *   form_keybd (55) needs BOTH halves of the question answered, and it is
 *      the one that shows why a per-opcode verdict cannot work.  Its arm is
 *      chosen by the CHARACTER: fm_keybd leaves direction at -1 for anything
 *      that is not RETURN/ENTER/ARROW_UP/TAB/ARROW_DOWN and returns without
 *      touching the tree at all (gemfmlib.c:167-190).  A navigation key
 *      instead calls find_obj, which walks the tree as an OBJECT array with
 *      no bound but LASTOB in ob_flags (:135-148) -- and one of the dumper's
 *      buffers is a string of letters, not an object tree, so a plant does
 *      NOT make that arm safe.  So: the character must be stated and must
 *      not navigate, AND the tree pointer must be a real buffer.
 *      The character is compared differently by the two AESs -- EmuTOS
 *      switches on the whole key WORD (scancode.h:22-32: 0x0f09, 0x1c0d,
 *      0x720d, 0x4800, 0x5000), NewTOS on the ASCII and scancode bytes
 *      separately (tos-api/src/aes/form.rs, fm_keybd) -- so the guard
 *      refuses the UNION of both readings.  A rule that matched only one of
 *      them would pass a case that is harmless here and fatal there, which
 *      is precisely the asymmetry below.
 *
 *   objc_delete (41) is the asymmetry itself, and it is why this part exists
 *      at all.  ob_delete tests `obj == ROOT` and nothing else
 *      (gemoblib.c:1147-1157); for obj < 0 it reads tree[obj].ob_next BELOW
 *      the tree base and then writes through get_par's result.  NewTOS
 *      answers 0 for obj < 0 (tos-api/src/aes/object.rs) and would come
 *      through clean, so a table author testing on NewTOS gets no warning at
 *      all before the TT run dies.  A planted buffer does not help here
 *      either -- ob_delete WRITES into the tree -- so the guard allows the
 *      one arm that returns before any tree access: OB_DELOB == ROOT.
 *
 * WHAT IS DELIBERATELY NOT HERE.  objc_add (40), objc_order (45),
 * objc_edit (46), menu_register (35), rsrc_gaddr/saddr (112, 113),
 * appl_tplay (14) and shel_get/put (122, 123) reach the same poisoned
 * addr_in[0] and are issued today, in cases/aes-r4-subfunctions.tsv, on
 * exactly the same footing objc_delete(ROOT) is: a stated parameter takes a
 * guard arm that returns first.  Each would need its own arm-by-arm reading
 * of its own handler, and writing those from the opcode's shape rather than
 * from the source is how a wrong guard gets written.  They stay as they are
 * until someone does that reading.
 */

/* wind_get / wind_set field numbers the guard knows by name */
#define WF_NAME     2
#define WF_INFO     3
#define WF_CXYWH    5
#define WF_NEWDESK 14
#define WF_COLOR   18
#define WF_DCOLOR  19

/*
 * evnt_multi: the two flag bits the guard reads (aesdefs.h:260-261) and the
 * int_in WORDs the timer count occupies (aesdefs.h:219-220).
 */
#define MU_MESAG        0x0010
#define MU_TIMER        0x0020
#define MT_LOCOUNT      14
#define MT_HICOUNT      15

/* fm_error's one non-blocking arm: `if (n > 63) return FALSE` */
#define FM_ERRNUM_MAX   63

/* ob_delete's one non-blocking arm: `if (obj == ROOT) return 0` */
#define OB_ROOT         0

/*
 * Would fm_keybd take a direction for this key word, on EITHER machine?
 *
 * EmuTOS compares the whole WORD against TAB 0x0f09, RETURN 0x1c0d, ENTER
 * 0x720d, ARROW_UP 0x4800 and ARROW_DOWN 0x5000 (gemfmlib.c:169-183,
 * scancode.h:22-32).  NewTOS splits the word and compares the ASCII byte
 * against 0x0d / 0x0a / 0x09 and the scancode byte against 0x48 / 0x50
 * (tos-api/src/aes/form.rs, fm_keybd).  NewTOS' reading is the wider one on
 * both halves, so testing it alone covers EmuTOS' five constants as well --
 * 0x0f09 and 0x1c0d and 0x720d all have one of those ASCII bytes, 0x4800 and
 * 0x5000 both have one of those scancodes.  Stated as the union anyway,
 * because "the wider one happens to contain the other" is a property of
 * today's two implementations and not of anything either of them promises.
 */
static int fm_keybd_navigates(int16_t c)
{
	int ascii = c & 0xFF;
	int scan  = (c >> 8) & 0xFF;

	if (ascii == 0x09 || ascii == 0x0A || ascii == 0x0D)
		return 1;                          /* TAB, LF, RETURN/ENTER  */
	if (scan == 0x48 || scan == 0x50)
		return 1;                          /* ARROW_UP, ARROW_DOWN   */
	return (uint16_t)c == 0x0f09u || (uint16_t)c == 0x1c0du ||
	       (uint16_t)c == 0x720du || (uint16_t)c == 0x4800u ||
	       (uint16_t)c == 0x5000u;
}

/*
 * A rectangle a case hands to a call that draws.  wind_open and
 * wind_set(WF_CXYWH) both reach draw_change, which does WORD arithmetic on
 * x+w and y+h throughout (rc_intersect / rc_union / mkpiece,
 * gemwrect.c:63-95) and hands the result to gsx_sclip unsorted
 * (gemwmlib.c:1028).  A negative extent or a sum that wraps is a table typo
 * whose damage on screen would look like a defect in the thing being
 * measured.
 *
 * wind_create is deliberately NOT covered: it only stores the rectangle
 * (gemwmlib.c:1369-1378) and draws nothing, so a marshalling case may state
 * fewer values there and let the rest stay poisoned.
 */
static int rect_is_drawable(const int16_t *r)
{
	int i;

	for (i = 0; i < 4; i++)
		if (r[i] < 0)
			return 0;
	if ((long)r[0] + (long)r[2] > 32767L)
		return 0;
	if ((long)r[1] + (long)r[3] > 32767L)
		return 0;
	return 1;
}

/*
 * The per-line half of the guard: everything decidable from one line on its
 * own.  Applies to preludes exactly as to cases -- a prelude that scribbled
 * through a poisoned pointer would take the machine down just as thoroughly,
 * and it would do it before the case it was meant to set up.
 */
static int aes_line_parameter_refusal(const struct td_aes_line *l,
                                      const char **what)
{
	int16_t field;

	switch (l->opcode) {
	case 104:                                        /* wind_get */
	case 105:                                        /* wind_set */
		if (l->n_values < 2) {
			*what = "wind_get/wind_set select a field with int_in[1], "
			        "which the guard has to read: state it";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		field = l->intin[1];
		/*
		 * WF_NAME / WF_INFO carry a string pointer, and the two AESs do
		 * not read it in the same place.
		 *
		 * wind_SET: EmuTOS' wm_set takes it from the int_in WORDs and
		 * nowhere else -- *(char **)pinwds with pinwds = &int_in[2]
		 * (gemwmlib.c:1661, :1669, gemsuper.c:346) -- and stores it where
		 * the next frame draw dereferences it (:501, :737).  An addr_in
		 * plant would leave that WORD pair poisoned, so the guard demands
		 * the int_in plant, which is a prelude-only form (cases.h).
		 *
		 * wind_GET: EmuTOS has no arm for either field and returns FALSE
		 * without touching a pointer (gemwmlib.c:1591-1592); NewTOS
		 * answers and copies the text out through addr_in[0] when
		 * AIN_LEN >= 1, or through the packed int_in pointer otherwise
		 * (tos-api/src/aes/window.rs, wf_str_ptr).  Either plant is
		 * therefore enough -- and one of them is REQUIRED, because
		 * without one the pointer NewTOS follows would be 0xDEAD poison.
		 */
		if (field == WF_NAME || field == WF_INFO) {
			int packed = line_plants(l, TD_AES_PLANT_INTIN, 2);
			int direct = line_plants(l, TD_AES_PLANT_ADDRIN, 0) &&
			             l->n_addrin >= 1;

			if (l->opcode == 105 && !packed) {
				*what = "wind_set(WF_NAME/WF_INFO) takes the string "
				        "pointer from *(char **)&int_in[2] "
				        "(gemwmlib.c:1661) and stores it where the next "
				        "frame draw follows it: the addr_in column must "
				        "plant a buffer at i2";
				return TD_CASE_ERR_UNSAFE_FIELD;
			}
			if (l->opcode == 104 && !packed && !direct) {
				*what = "wind_get(WF_NAME/WF_INFO) copies the title out "
				        "through the caller's pointer (window.rs, "
				        "wf_str_ptr): the addr_in column must plant a "
				        "buffer at a0 (with n_addrin >= 1) or at i2";
				return TD_CASE_ERR_UNSAFE_FIELD;
			}
		}
		if (field == WF_COLOR || field == WF_DCOLOR) {
			*what = "WF_COLOR/WF_DCOLOR index w_tcolor[]/gl_wtcolor[] "
			        "with int_in[2] and EmuTOS does not range-check it "
			        "(gemwmlib.c:1567-1576, :1714-1727)";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (l->opcode == 105 && field == WF_NEWDESK) {
			*what = "wind_set(WF_NEWDESK) stores a tree pointer in "
			        "gl_newdesk (gemwmlib.c:1683) which w_drawdesk then "
			        "walks, and wm_new does not clear it";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (l->opcode == 105 && l->intin[0] == 0) {
			*what = "wind_set on handle 0 changes the DESKTOP window's "
			        "own fields, and wm_new sweeps handles >= 1 only "
			        "(gemwmlib.c:1886) -- only wm_init restores them "
			        "(:1275-1281), so nothing in the run undoes it";
			return TD_CASE_ERR_UNSAFE_STATE;
		}
		if (l->opcode == 105 && field == WF_CXYWH) {
			if (l->n_values < 6 || !rect_is_drawable(&l->intin[2])) {
				*what = "wind_set(WF_CXYWH) draws int_in[2..5]: all four "
				        "must be stated, non-negative, and x+w / y+h must "
				        "not wrap a WORD";
				return TD_CASE_ERR_GEOMETRY;
			}
		}
		break;
	case 103:                                        /* wind_delete */
		if (l->n_values < 1) {
			*what = "wind_delete names its handle in int_in[0], which the "
			        "guard has to read: state it";
			return TD_CASE_ERR_UNSAFE_STATE;
		}
		if (l->intin[0] == 0) {
			*what = "wind_delete(0) clears VF_INUSE on the DESKTOP window "
			        "(gemwmlib.c:1477-1481); wm_new only sweeps handles "
			        ">= 1 (:1886), so nothing in the run brings it back";
			return TD_CASE_ERR_UNSAFE_STATE;
		}
		break;
	case 101:                                        /* wind_open */
		if (l->n_values < 1) {
			*what = "wind_open names its handle in int_in[0], which the "
			        "message-queue bound has to read: state it";
			return TD_CASE_ERR_UNSAFE_STATE;
		}
		if (l->intin[0] == 0) {
			*what = "wind_open(0) runs w_obadd(W_TREE, ROOT, ROOT) "
			        "(gemwmlib.c:1421, :186-202), which makes the root its "
			        "own child; the next everyobj walk never terminates";
			return TD_CASE_ERR_UNSAFE_STATE;
		}
		if (l->n_values < 5 || !rect_is_drawable(&l->intin[1])) {
			*what = "wind_open draws int_in[1..4]: all four must be "
			        "stated, non-negative, and x+w / y+h must not wrap "
			        "a WORD";
			return TD_CASE_ERR_GEOMETRY;
		}
		break;
	case 107:                                        /* wind_update */
		if (l->n_values < 1) {
			*what = "wind_update names its mode in int_in[0], which the "
			        "mouse-control balance has to read: state it";
			return TD_CASE_ERR_UNSAFE_STATE;
		}
		break;
	case 25:                                         /* evnt_multi */
		/*
		 * The quick check timer is the one arm that returns without
		 * mwait, so the guard has to read the flags AND the timer count.
		 * Both live in int_in, so both must be stated and both must be
		 * inside IN_LEN -- outside it they are xif's uninitialised
		 * stack, not a number the table chose.
		 */
		if (l->n_values <= MT_HICOUNT || l->n_intin <= MT_HICOUNT) {
			*what = "evnt_multi is issuable only as the quick check "
			        "timer, so the guard reads MU_FLAGS and "
			        "MT_LOCOUNT/MT_HICOUNT: state all sixteen int_in "
			        "words and declare IN_LEN 16, or xif's local int_in "
			        "leaves the count uninitialised "
			        "(gemsuper.c:428-438)";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (!(l->intin[0] & MU_TIMER) || l->intin[MT_LOCOUNT] != 0 ||
		    l->intin[MT_HICOUNT] != 0) {
			*what = "evnt_multi returns before mwait only for MU_TIMER "
			        "with a zero MT_LOCOUNT/MT_HICOUNT "
			        "(gemevlib.c:216-220 before :232-262); any other "
			        "flags reach mwait, and a zero mask reaches it too "
			        "-- mwait(0) sets WAITIN and dsptch()es with nothing "
			        "that can wake it (gemasync.c:117-125)";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if ((l->intin[0] & MU_MESAG) && !line_has_buffer(l, 0)) {
			*what = "evnt_multi with MU_MESAG writes sixteen bytes "
			        "through MME_PBUFF = addr_in[0] as soon as anything "
			        "is queued (gemevlib.c:221-229), and the dumper "
			        "never drains the WM_REDRAWs its own wind_opens "
			        "make: the addr_in column must plant a buffer at a0 "
			        "(with n_addrin >= 1)";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		break;
	case 41:                                         /* objc_delete */
		if (l->n_values < 1 || l->n_intin < 1) {
			*what = "objc_delete names its object in int_in[0], which "
			        "the guard has to read: state it and declare "
			        "IN_LEN >= 1";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (l->intin[0] != OB_ROOT) {
			*what = "ob_delete returns before touching the tree only for "
			        "OB_DELOB == ROOT (gemoblib.c:1154-1155); any other "
			        "object reads tree[obj].ob_next and writes through "
			        "get_par (:1157-1188), and a negative one reads "
			        "BELOW the tree base -- which NewTOS answers 0 for "
			        "and the TT does not, so testing on one machine "
			        "proves nothing about the other";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		break;
	case 53:                                         /* form_error */
		if (l->n_values < 1 || l->n_intin < 1) {
			*what = "form_error is issuable only out of range, so the "
			        "guard has to read FM_ERRNUM: state int_in[0] and "
			        "declare IN_LEN >= 1";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (l->intin[0] <= FM_ERRNUM_MAX) {
			*what = "fm_error returns FALSE without drawing only for "
			        "n > 63 (gemfmlib.c:440-441); every other n reaches "
			        "fm_show -> fm_alert -> fm_do (:469, :400, "
			        "gemfmalt.c:383), a modal alert waiting for a click";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		break;
	case 55:                                         /* form_keybd */
		if (l->n_values < 2 || l->n_intin < 2) {
			*what = "form_keybd's arm is chosen by FM_ICHAR = int_in[1], "
			        "which the guard has to read: state it and declare "
			        "IN_LEN >= 2";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (fm_keybd_navigates(l->intin[1])) {
			*what = "form_keybd with RETURN/ENTER/ARROW_UP/TAB/"
			        "ARROW_DOWN calls find_obj, which walks the tree as "
			        "an OBJECT array bounded only by LASTOB "
			        "(gemfmlib.c:185-190, :135-148); one of the dumper's "
			        "buffers is a string of letters and not a tree, so "
			        "no plant makes that arm safe";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (!line_has_buffer(l, 0)) {
			*what = "form_keybd takes its tree from FM_FORM = addr_in[0] "
			        "(gemsuper.c:258): the addr_in column must plant a "
			        "buffer at a0 (with n_addrin >= 1) rather than hand "
			        "the poison to a call that may dereference it";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		break;
	case 80:                                         /* scrp_read */
		if (!line_has_buffer(l, 0)) {
			*what = "scrp_read is strcpy(pscrap, D.g_scrap) "
			        "(gemsclib.c:40-45): it WRITES up to LEN_ZPATH = 114 "
			        "bytes through SC_PATH = addr_in[0].  The addr_in "
			        "column must plant a buffer at a0 (with "
			        "n_addrin >= 1)";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		break;
	case 81:                                         /* scrp_write */
		if (!line_has_buffer(l, 0)) {
			*what = "scrp_write is strcpy(D.g_scrap, pscrap) "
			        "(gemsclib.c:57-62): it READS a NUL terminated "
			        "string through SC_PATH = addr_in[0].  The addr_in "
			        "column must plant a buffer at a0 (with "
			        "n_addrin >= 1)";
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		break;
	default:
		break;
	}

	return 0;
}

/*
 * The distinct wind_open handles a table names, accumulated over the whole
 * file.  Table-wide rather than per case because the message queue is never
 * drained: what bounds it is how many distinct handles the run ever opens,
 * and the reset only guarantees that a case's handles start again at 1.
 */
struct td_aes_open_set {
	int16_t handle[TD_AES_MAX_OPEN_WINDOWS];
	int     count;
};

static int open_set_add(struct td_aes_open_set *s, int16_t handle)
{
	int i;

	for (i = 0; i < s->count; i++)
		if (s->handle[i] == handle)
			return 0;
	if (s->count >= TD_AES_MAX_OPEN_WINDOWS)
		return -1;
	s->handle[s->count++] = handle;
	return 0;
}


/*
 * One count column.  "-" is accepted only where `dash_value` is not
 * negative -- that is the n_intin column, where it means "as many as the
 * value column lists".  In the other three there is no list to derive a
 * count from and "0" is one character away, so guessing there would
 * silently change what the call declares.
 */
static int parse_count(const char *s, size_t len, int dash_value, int16_t *out)
{
	const char *p = s;
	size_t n = len;
	int16_t v;
	int rc;

	trim(&p, &n);
	if (n == 1 && p[0] == '-') {
		if (dash_value < 0)
			return TD_CASE_ERR_VALUE;
		*out = (int16_t)dash_value;
		return 0;
	}
	rc = parse_value(s, len, &v);
	if (rc != 0)
		return rc;
	if (v < 0)
		return TD_CASE_ERR_RANGE;
	*out = v;
	return 0;
}

/*
 * One entry of the addr_in column: "<where>=<buffer>" (cases.h).  Parsed
 * here and validated in check_plants, which is where the line's counts are
 * known.
 */
static int parse_plant(const char *s, size_t len, struct td_aes_plant *out,
                       const char **what)
{
	size_t eq;
	int16_t v;
	int rc;

	trim(&s, &len);
	if (len < 4) {                     /* "aN=B" is the shortest there is */
		*what = "a plant is \"aN=B\" (addr_in slot) or \"iN=B\" "
		        "(a pointer packed at int_in word N)";
		return TD_CASE_ERR_PLANT;
	}
	if (s[0] == 'a')
		out->kind = TD_AES_PLANT_ADDRIN;
	else if (s[0] == 'i')
		out->kind = TD_AES_PLANT_INTIN;
	else {
		*what = "a plant names 'a' for an addr_in slot or 'i' for a "
		        "pointer packed into the int_in WORDs";
		return TD_CASE_ERR_PLANT;
	}

	for (eq = 1; eq < len && s[eq] != '='; eq++)
		;
	if (eq == 1 || eq + 1 >= len) {
		*what = "a plant is \"aN=B\" or \"iN=B\": a position, '=', "
		        "and the buffer number";
		return TD_CASE_ERR_PLANT;
	}

	rc = parse_value(s + 1, eq - 1, &v);
	if (rc != 0 || v < 0) {
		*what = "a plant's position is not a number";
		return rc != 0 ? rc : TD_CASE_ERR_PLANT;
	}
	out->where = v;

	rc = parse_value(s + eq + 1, len - eq - 1, &v);
	if (rc != 0 || v < 0) {
		*what = "a plant's buffer is not a number";
		return rc != 0 ? rc : TD_CASE_ERR_PLANT;
	}
	if (v >= TD_AES_BUF_N) {
		*what = "the dumper owns TD_AES_BUF_N buffers and the column "
		        "names one past the last";
		return TD_CASE_ERR_PLANT;
	}
	out->buffer = v;
	return 0;
}

/* The whole addr_in column.  "-" means no plant at all, which is what every
 * line of a seven column table means. */
static int parse_plant_list(const char *s, size_t len,
                            struct td_aes_line *out, const char **what)
{
	size_t i = 0, start = 0;

	out->n_plants = 0;
	trim(&s, &len);
	if (len == 0) {
		*what = "the addr_in column is empty; \"-\" means no plant";
		return TD_CASE_ERR_PLANT;
	}
	if (len == 1 && s[0] == '-')
		return 0;

	for (;;) {
		if (i == len || s[i] == ',') {
			int rc;

			if (out->n_plants >= TD_AES_PLANTS_MAX) {
				*what = "more plants than TD_AES_PLANTS_MAX";
				return TD_CASE_ERR_PLANT;
			}
			rc = parse_plant(s + start, i - start,
			                 &out->plant[out->n_plants], what);
			if (rc != 0)
				return rc;
			out->n_plants++;
			if (i == len)
				break;
			i++;
			start = i;
			continue;
		}
		i++;
	}
	return 0;
}

/*
 * What a plant needs from the rest of the line, and why each of these is a
 * refusal rather than a silent adjustment:
 *
 *  - an addr_in plant the case does not declare in AIN_LEN never reaches the
 *    AES at all: the dispatcher copies min(AIN_LEN, AI_SIZE) slots onto its
 *    local buffer (gemsuper.c:439) and the handler reads the local one.  The
 *    case would look like it handed over a buffer and would hand over
 *    whatever the dispatcher's stack held.
 *  - an int_in plant is only correct on both targets when IN_LEN covers the
 *    WIDEST pointer footprint: four WORDs on AArch64, two on the m68k.  The
 *    table states one number for both machines, so it has to be the wider.
 *  - an int_in plant on a MEASURED case would put the target's pointer width
 *    into a recorded array (cases.h).  Preludes are not recorded.
 *  - a plant over a WORD the case also lists a value for is two statements
 *    about one WORD.
 */
static int check_plants(const struct td_aes_line *l, int prelude,
                        const char **what)
{
	uint16_t i, j;

	for (i = 0; i < l->n_plants; i++) {
		const struct td_aes_plant *p = &l->plant[i];

		for (j = 0; j < i; j++)
			if (l->plant[j].kind == p->kind &&
			    l->plant[j].where == p->where) {
				*what = "the addr_in column plants the same position "
				        "twice";
				return TD_CASE_ERR_PLANT;
			}

		if (p->kind == TD_AES_PLANT_ADDRIN) {
			if (p->where >= TD_AES_ADDRIN_N) {
				*what = "the AES has TD_AES_ADDRIN_N address slots "
				        "(AI_SIZE, aesdefs.h:124) and the column names "
				        "one past the last";
				return TD_CASE_ERR_PLANT;
			}
			if (p->where >= l->n_addrin) {
				*what = "an addr_in plant past AIN_LEN never reaches the "
				        "handler: xif copies min(AIN_LEN, AI_SIZE) slots "
				        "(gemsuper.c:439)";
				return TD_CASE_ERR_PLANT;
			}
			continue;
		}

		/* TD_AES_PLANT_INTIN */
		if (!prelude) {
			*what = "a pointer packed into int_in occupies two WORDs on "
			        "the m68k and four on AArch64, and int_in is "
			        "recorded: only a prelude, which is not recorded, "
			        "may carry one";
			return TD_CASE_ERR_PLANT;
		}
		if (p->where + TD_AES_PLANT_PTR_WORDS > TD_AES_INTIN_N) {
			*what = "a packed pointer would run past int_in[I_SIZE] on "
			        "the wider target";
			return TD_CASE_ERR_PLANT;
		}
		if (p->where + TD_AES_PLANT_PTR_WORDS > l->n_intin) {
			*what = "IN_LEN must cover the packed pointer on the WIDER "
			        "target -- four WORDs -- or the m68k build and the "
			        "AArch64 build would not be handed the same thing";
			return TD_CASE_ERR_PLANT;
		}
		if (p->where < (int16_t)l->n_values) {
			*what = "the line states a value for the WORD it also plants "
			        "a pointer at";
			return TD_CASE_ERR_PLANT;
		}
		for (j = 0; j < i; j++) {
			const struct td_aes_plant *q = &l->plant[j];
			if (q->kind != TD_AES_PLANT_INTIN)
				continue;
			if (p->where < q->where + TD_AES_PLANT_PTR_WORDS &&
			    q->where < p->where + TD_AES_PLANT_PTR_WORDS) {
				*what = "two packed pointers would overlap on the wider "
				        "target";
				return TD_CASE_ERR_PLANT;
			}
		}
	}
	return 0;
}

/* Columns 1..7 of one AES line.  `opcode_col` already has the '+' of a
 * prelude stripped by the caller; `ncols` is the table's declared column
 * count, so the addr_in column is read only where the table says there is
 * one. */
static int parse_aes_columns(const char *opcode_col, size_t opcode_len,
                             const char **col, const size_t *col_len,
                             int ncols, int prelude,
                             struct td_aes_line *out, const char **what)
{
	int16_t v;
	int rc;

	memset(out, 0, sizeof *out);

	rc = parse_value(opcode_col, opcode_len, &v);
	if (rc != 0) {
		*what = "opcode is not a number";
		return rc;
	}
	if (v < 0) {
		*what = "opcode is negative";
		return TD_CASE_ERR_RANGE;
	}
	out->opcode = v;

	/* The guard, before anything else about the line matters: an opcode
	 * that would hang the machine is refused whether it is a case or a
	 * prelude, and whether or not the rest of the line parses. */
	*what = aes_opcode_refusal(out->opcode);
	if (*what != NULL)
		return TD_CASE_ERR_UNSAFE_OPCODE;

	/* the values first: the n_intin column's "-" is derived from them */
	rc = parse_list(col[5], col_len[5], out->intin, TD_AES_INTIN_N,
	                &out->n_values);
	if (rc != 0) {
		*what = "int_in value column";
		return rc;
	}

	rc = parse_count(col[1], col_len[1], (int)out->n_values, &out->n_intin);
	if (rc != 0) {
		*what = "n_intin column";
		return rc;
	}
	rc = parse_count(col[2], col_len[2], -1, &out->n_intout);
	if (rc != 0) {
		*what = "n_intout column (\"-\" has nothing to be derived from)";
		return rc;
	}
	/*
	 * OUT_LEN is the one count the dispatcher does not clamp:
	 * memcpy(pcrys_blk->intout, int_out, OUT_LEN*sizeof(WORD))
	 * (gemsuper.c:444-445) writes into the CALLER's array, so a larger
	 * value overruns it on the target.  Refuse the table instead of
	 * letting a typo corrupt memory on real hardware.
	 */
	if (out->n_intout > TD_AES_INTOUT_N) {
		*what = "n_intout is larger than the intout array, which the "
		        "dispatcher's copy-back does not clamp";
		return TD_CASE_ERR_OUT_LEN;
	}
	rc = parse_count(col[3], col_len[3], -1, &out->n_addrin);
	if (rc != 0) {
		*what = "n_addrin column (\"-\" has nothing to be derived from)";
		return rc;
	}
	rc = parse_count(col[4], col_len[4], -1, &out->n_addrout);
	if (rc != 0) {
		*what = "n_addrout column (\"-\" has nothing to be derived from)";
		return rc;
	}

	/* the addr_in column, where the table declares one */
	if (ncols >= TD_AES_CASE_COLUMNS_MAX) {
		rc = parse_plant_list(col[6], col_len[6], out, what);
		if (rc != 0)
			return rc;
		rc = check_plants(out, prelude, what);
		if (rc != 0)
			return rc;
	}

	/* the parameter guard needs the values and the plants, so it is last */
	return aes_line_parameter_refusal(out, what);
}

/*
 * The value of a "!columns" line: 7 (today's form) or 8 (with the addr_in
 * column).  One parser, used by the strict path below and by the pre-scan
 * td_aes_table_columns, so the host reader and the loader cannot end up
 * slicing the same file differently.
 */
static int parse_columns_value(const char *s, size_t len, int *out)
{
	trim(&s, &len);
	if (len == 1 && s[0] == '7') {
		*out = TD_AES_CASE_COLUMNS;
		return 0;
	}
	if (len == 1 && s[0] == '8') {
		*out = TD_AES_CASE_COLUMNS_MAX;
		return 0;
	}
	return TD_CASE_ERR_DIRECTIVE;
}

/*
 * One "!name <TAB> value" line.  Refused rather than defaulted on anything
 * it does not recognise: a table that meant to declare a reset and misspelt
 * it would otherwise run round 1's absent one and produce a dump that looks
 * fine and measures the wrong thing.
 */
static int parse_aes_directive(const char *line, size_t len,
                               struct td_aes_case_table *t, int seen_call,
                               const char **what)
{
	const char *col[2];
	size_t col_len[2];
	const char *name;
	size_t name_len;
	int is_reset;

	if (split_columns(line, len, col, col_len, 2) < 2) {
		*what = "a directive is \"!name <TAB> value\"";
		return TD_CASE_ERR_DIRECTIVE;
	}

	name = col[0];
	name_len = col_len[0];
	trim(&name, &name_len);
	name++;                            /* step over the '!' */
	name_len--;
	if (name_len == 5 && memcmp(name, "reset", 5) == 0)
		is_reset = 1;
	else if (name_len == 7 && memcmp(name, "columns", 7) == 0)
		is_reset = 0;
	else {
		*what = "the directives are \"!reset\" and \"!columns\"";
		return TD_CASE_ERR_DIRECTIVE;
	}

	if (seen_call) {
		*what = "a directive must be declared before the first case or "
		        "prelude; the calls before it would have run without it";
		return TD_CASE_ERR_DIRECTIVE;
	}

	name = col[1];
	name_len = col_len[1];
	trim(&name, &name_len);

	if (!is_reset) {
		int n;

		if (t->columns > 0) {
			*what = "the table declares its column count twice";
			return TD_CASE_ERR_DIRECTIVE;
		}
		if (parse_columns_value(name, name_len, &n) != 0) {
			*what = "the column count must be 7 (the form without an "
			        "addr_in column) or 8 (with it)";
			return TD_CASE_ERR_DIRECTIVE;
		}
		t->columns = (int16_t)n;
		return 0;
	}

	if (t->reset >= 0) {
		*what = "the table declares a reset twice";
		return TD_CASE_ERR_DIRECTIVE;
	}
	if (name_len == 4 && memcmp(name, "none", 4) == 0) {
		t->reset = TD_AES_RESET_NONE;
		return 0;
	}
	if (name_len == 8 && memcmp(name, "wind_new", 8) == 0) {
		t->reset = TD_AES_RESET_WIND_NEW;
		return 0;
	}

	*what = "the reset must be \"none\" or \"wind_new\"; guessing here "
	        "would silently run the wrong round";
	return TD_CASE_ERR_DIRECTIVE;
}

/*
 * The column count a table declares, read ahead of everything else -- see
 * cases.h.  A directive this cannot read answers with the default; the
 * strict parse above is the one place that reports it, so a malformed
 * "!columns" is one error and not two.
 */
int td_aes_table_columns(const char *buf, size_t len)
{
	struct td_line_iter it;
	const char *line;
	size_t line_len;
	uint16_t line_no;
	int is_case;

	td_line_iter_init(&it, buf, len);
	while (td_line_iter_next(&it, &line, &line_len, &line_no, &is_case)) {
		const char *col[2], *name;
		size_t col_len[2], name_len;
		int n;

		if (is_case)
			break;                 /* directives come first (cases.h) */
		if (!td_case_line_is_directive(line, line_len))
			continue;
		if (split_columns(line, line_len, col, col_len, 2) < 2)
			continue;
		name = col[0];
		name_len = col_len[0];
		trim(&name, &name_len);
		if (name_len != 8 || memcmp(name, "!columns", 8) != 0)
			continue;
		if (parse_columns_value(col[1], col_len[1], &n) == 0)
			return n;
		break;
	}
	return TD_AES_CASE_COLUMNS;
}

int td_aes_cases_parse(struct td_aes_case_table *t, const char *buf, size_t len,
                       struct td_case_error *err)
{
	struct td_line_iter it;
	const char *line;
	size_t line_len;
	uint16_t line_no;
	uint16_t pending = 0;      /* preludes seen since the last case */
	uint16_t last_prelude = 0; /* line number of the newest one, for errors */
	struct td_aes_open_set opened;
	int is_case;

	memset(t, 0, sizeof *t);
	memset(&opened, 0, sizeof opened);
	/* -1 = not declared yet, so a second "!reset" is an error rather than
	 * an override; it becomes TD_AES_RESET_NONE at the end if absent */
	t->reset = -1;
	/* 0 = not declared yet, for the same reason; it becomes
	 * TD_AES_CASE_COLUMNS at the end if absent */
	t->columns = 0;
	t->crc = td_crc32((const uint8_t *)buf, len);
	if (err) {
		err->line = 0;
		err->what = NULL;
	}

	td_line_iter_init(&it, buf, len);
	while (td_line_iter_next(&it, &line, &line_len, &line_no, &is_case)) {
		const char *col[TD_AES_CASE_COLUMNS_MAX];
		size_t col_len[TD_AES_CASE_COLUMNS_MAX];
		struct td_aes_line parsed;
		const char *what = NULL;
		const char *opcode_col;
		size_t opcode_len;
		int prelude, rc, i, ncols;

		rc = 0;

		if (!is_case) {
			if (!td_case_line_is_directive(line, line_len))
				continue;
			rc = parse_aes_directive(line, line_len, t,
			                         t->count > 0 || pending > 0, &what);
			if (rc != 0) {
				fail(err, line_no, what);
				return rc;
			}
			continue;
		}

		/* The first case fixes the form: a "!columns" after it is refused
		 * above, so from here on the count cannot change under us. */
		if (t->columns <= 0)
			t->columns = TD_AES_CASE_COLUMNS;
		ncols = t->columns;

		if (split_columns(line, line_len, col, col_len, ncols) < ncols) {
			fail(err, line_no,
			     ncols >= TD_AES_CASE_COLUMNS_MAX
			         ? "the table declares \"!columns 8\", so every case "
			           "line needs eight tab separated columns"
			         : "needs seven tab separated columns");
			return TD_CASE_ERR_COLUMNS;
		}

		prelude = td_case_line_is_prelude(line, line_len);
		opcode_col = col[0];
		opcode_len = col_len[0];
		trim(&opcode_col, &opcode_len);
		if (prelude) {                 /* step over the '+' */
			opcode_col++;
			opcode_len--;
		}

		rc = parse_aes_columns(opcode_col, opcode_len, col, col_len,
		                       ncols, prelude, &parsed, &what);
		if (rc != 0) {
			fail(err, line_no, what);
			return rc;
		}

		/* What this line needs of the MACHINE, which the loader cannot
		 * answer and does not try to: it only records it.  Preludes
		 * count exactly as cases do -- a prelude issuing an opcode the
		 * AES lacks meets the same modal alert, and it meets it before
		 * the case it was meant to set up. */
		t->features |= aes_opcode_feature(parsed.opcode);

		/* The message-queue bound, over the whole table: see
		 * TD_AES_MAX_OPEN_WINDOWS.  Preludes count -- a prelude that
		 * opens a window sends the same WM_REDRAW a case does. */
		if (parsed.opcode == 101 &&
		    open_set_add(&opened, parsed.intin[0]) != 0) {
			fail(err, line_no,
			     "more distinct wind_open handles than the AES message "
			     "queue holds; the dumper is the owner of every WM_REDRAW "
			     "and has no non-blocking way to drain its own queue");
			return TD_CASE_ERR_UNSAFE_STATE;
		}

		/*
		 * A '+' opcode marks a prelude: a call issued before the next
		 * case's measured call, and not recorded.  Round 1 needs none,
		 * but the convention is shared with the VDI table and with
		 * host/casefile.c through td_case_line_is_prelude -- a reader
		 * that counted preludes as cases would index every record
		 * wrongly.
		 */
		if (prelude) {
			struct td_aes_prelude *p;

			if (pending >= TD_MAX_PRELUDES ||
			    t->prelude_count >= TD_MAX_PRELUDES) {
				fail(err, line_no, "more preludes than TD_MAX_PRELUDES");
				return TD_CASE_ERR_TOO_MANY_PRE;
			}
			p = &t->preludes[t->prelude_count + pending];
			memset(p, 0, sizeof *p);
			p->opcode    = parsed.opcode;
			p->n_intin   = parsed.n_intin;
			p->n_intout  = parsed.n_intout;
			p->n_addrin  = parsed.n_addrin;
			p->n_addrout = parsed.n_addrout;
			p->n_values  = parsed.n_values;
			p->n_plants  = parsed.n_plants;
			for (i = 0; i < (int)parsed.n_values; i++)
				p->intin[i] = parsed.intin[i];
			for (i = 0; i < (int)parsed.n_plants; i++)
				p->plant[i] = parsed.plant[i];
			pending++;
			last_prelude = line_no;
			continue;
		}

		if (t->count >= TD_MAX_CASES) {
			fail(err, line_no, "more cases than TD_MAX_CASES");
			return TD_CASE_ERR_TOO_MANY_CASES;
		}

		{
			struct td_aes_case *c = &t->cases[t->count];

			memset(c, 0, sizeof *c);
			c->line          = line_no;
			c->prelude_first = t->prelude_count;
			c->prelude_count = pending;
			t->prelude_count = (uint16_t)(t->prelude_count + pending);
			pending = 0;

			c->opcode    = parsed.opcode;
			c->n_intin   = parsed.n_intin;
			c->n_intout  = parsed.n_intout;
			c->n_addrin  = parsed.n_addrin;
			c->n_addrout = parsed.n_addrout;
			c->n_values  = parsed.n_values;
			c->n_plants  = parsed.n_plants;
			for (i = 0; i < (int)parsed.n_values; i++)
				c->intin[i] = parsed.intin[i];
			for (i = 0; i < (int)parsed.n_plants; i++)
				c->plant[i] = parsed.plant[i];

			td_case_line_comment_n(line, line_len, ncols,
			                       c->comment, TD_COMMENT_LEN);

			/*
			 * The mouse-control balance, over this case's calls in the
			 * order the runner issues them: preludes first, then the
			 * measured call.  Per case and not across the table,
			 * because the reset runs in between and drains ml_ocnt --
			 * but only downwards (`while (ml_ocnt > 0)`,
			 * gemwmlib.c:1880), so a count driven negative inside a
			 * case is never healed.  The offending line reported is
			 * the case's: the balance is a property of the case as a
			 * whole, not of any one of its lines.
			 */
			{
				int ocnt = 0;
				uint16_t k;

				for (k = 0; k <= c->prelude_count; k++) {
					int16_t op, mode;

					if (k < c->prelude_count) {
						const struct td_aes_prelude *pp =
							&t->preludes[c->prelude_first + k];
						op   = pp->opcode;
						mode = pp->n_values ? pp->intin[0] : 0;
					} else {
						op   = c->opcode;
						mode = c->n_values ? c->intin[0] : 0;
					}
					if (op == 109) {
						/* wm_new drains ml_ocnt to zero
						 * (gemwmlib.c:1879-1880), so a BEG_MCTRL
						 * before it does not pay for an END_MCTRL
						 * after it */
						ocnt = 0;
						continue;
					}
					if (op != 107)
						continue;
					if (mode == 3)          /* BEG_MCTRL */
						ocnt++;
					else if (mode == 2)     /* END_MCTRL */
						ocnt--;
					if (ocnt < 0)
						break;
				}
				if (ocnt < 0) {
					fail(err, line_no,
					     "wind_update END_MCTRL without a matching "
					     "BEG_MCTRL in the same case: fm_own(0) "
					     "decrements ml_ocnt unconditionally "
					     "(gemfmlib.c:99) and wm_new only drains it "
					     "downwards (gemwmlib.c:1880)");
					return TD_CASE_ERR_UNSAFE_STATE;
				}
			}

			t->count++;
		}
	}

	/* A prelude with nothing to precede would silently never run. */
	if (pending > 0) {
		fail(err, last_prelude, "prelude has no case after it");
		return TD_CASE_ERR_DANGLING;
	}

	/* Absent means round 1's behaviour -- see cases.h for why the default
	 * may not be anything else.  The column count is the same argument:
	 * every table written before the addr_in column has seven. */
	if (t->reset < 0)
		t->reset = TD_AES_RESET_NONE;
	if (t->columns <= 0)
		t->columns = TD_AES_CASE_COLUMNS;

	return 0;
}

int td_case_line_is_prelude(const char *line, size_t len)
{
	size_t i = 0;

	while (i < len && (line[i] == ' ' || line[i] == '\t'))
		i++;
	return i < len && line[i] == '+';
}

int td_case_line_is_directive(const char *line, size_t len)
{
	size_t i = 0;

	while (i < len && (line[i] == ' ' || line[i] == '\t'))
		i++;
	return i < len && line[i] == '!';
}

void td_case_line_comment_n(const char *line, size_t len, int ncols,
                            char *dst, size_t dst_size)
{
	const char *col[TD_AES_CASE_COLUMNS_MAX];
	size_t col_len[TD_AES_CASE_COLUMNS_MAX];
	size_t n;

	if (dst_size == 0)
		return;
	if (ncols > TD_AES_CASE_COLUMNS_MAX ||
	    split_columns(line, len, col, col_len, ncols) < ncols) {
		dst[0] = '\0';
		return;
	}
	n = col_len[ncols - 1];
	if (n >= dst_size)
		n = dst_size - 1;
	memcpy(dst, col[ncols - 1], n);
	dst[n] = '\0';
}

void td_case_line_comment(const char *line, size_t len, char *dst,
                          size_t dst_size)
{
	td_case_line_comment_n(line, len, TD_CASE_COLUMNS, dst, dst_size);
}

/* Slurps a case table.  One reader for both table forms, so a table that
 * loads for one area cannot fail to load for the other. */
static int slurp(const char *path, char **raw_out, size_t *len_out,
                 struct td_case_error *err)
{
	FILE *f;
	long size;
	char *raw;
	size_t got;

	f = fopen(path, "rb");
	if (!f) {
		fail(err, 0, "cannot open the case table");
		return TD_CASE_ERR_IO;
	}
	if (fseek(f, 0, SEEK_END) != 0 || (size = ftell(f)) < 0 ||
	    fseek(f, 0, SEEK_SET) != 0) {
		fclose(f);
		fail(err, 0, "cannot seek the case table");
		return TD_CASE_ERR_IO;
	}
	raw = (char *)malloc((size_t)size + 1);
	if (!raw) {
		fclose(f);
		fail(err, 0, "out of memory reading the case table");
		return TD_CASE_ERR_IO;
	}
	got = fread(raw, 1, (size_t)size, f);
	fclose(f);
	if (got != (size_t)size) {
		free(raw);
		fail(err, 0, "short read on the case table");
		return TD_CASE_ERR_IO;
	}
	raw[got] = '\0';

	*raw_out = raw;
	*len_out = got;
	return 0;
}

int td_cases_load(struct td_case_table *t, const char *path,
                  struct td_case_error *err)
{
	char *raw;
	size_t got;
	int rc;

	rc = slurp(path, &raw, &got, err);
	if (rc != 0)
		return rc;

	rc = td_cases_parse(t, raw, got, err);
	free(raw);
	return rc;
}

int td_aes_cases_load(struct td_aes_case_table *t, const char *path,
                      struct td_case_error *err)
{
	char *raw;
	size_t got;
	int rc;

	rc = slurp(path, &raw, &got, err);
	if (rc != 0)
		return rc;

	rc = td_aes_cases_parse(t, raw, got, err);
	free(raw);
	return rc;
}

/* ------------------------------------------------------------------ */
/* VDI raster table parsing                                            */
/* ------------------------------------------------------------------ */
/*
 * The guard is documented in cases.h.  Its job is not to be clever: it is to
 * make it impossible to state a case that walks off a buffer on a machine
 * whose VDI does no bounds checking of its own.
 */

/* One MFDB column: buffer, fd_w, fd_h, fd_wdwidth, fd_stand, fd_nplanes --
 * or the single letter S for the screen (cases.h, THE SCREEN). */
static int parse_mfdb(const char *s, size_t len, struct td_rast_mfdb *out,
                      const struct td_rast_screen *screen, const char **what)
{
	int16_t v[6];
	uint16_t n = 0;
	int rc;

	trim(&s, &len);
	if (len == 1 && s[0] == 'S') {
		if (!screen->declared) {
			*what = "an MFDB names the screen, but the table has no "
			        "\"!screen w,h,planes\" line before its first case, so "
			        "there is no extent to check its rectangles against";
			return TD_CASE_ERR_GEOMETRY;
		}
		out->buffer  = TD_RAST_BUF_SCREEN;
		out->w       = screen->w;
		out->h       = screen->h;
		out->wdwidth = 0;
		out->stand   = 0;
		out->nplanes = screen->planes;
		return 0;
	}

	rc = parse_list(s, len, v, 6, &n);
	if (rc != 0) {
		*what = "MFDB column is not a list of numbers";
		return rc;
	}
	if (n != 6) {
		*what = "an MFDB column needs six values: buffer, fd_w, fd_h, "
		        "fd_wdwidth, fd_stand, fd_nplanes";
		return TD_CASE_ERR_COLUMNS;
	}

	out->buffer  = v[0];
	out->w       = v[1];
	out->h       = v[2];
	out->wdwidth = v[3];
	out->stand   = v[4];
	out->nplanes = v[5];

	if (out->buffer != TD_RAST_BUF_A && out->buffer != TD_RAST_BUF_B) {
		*what = "the buffer field of an MFDB is 0 (A) or 1 (B); the screen "
		        "is spelt S and needs a \"!screen\" line";
		return TD_CASE_ERR_GEOMETRY;
	}
	if (out->w <= 0 || out->h <= 0 || out->wdwidth <= 0 || out->nplanes <= 0) {
		*what = "fd_w, fd_h, fd_wdwidth and fd_nplanes must all be positive";
		return TD_CASE_ERR_GEOMETRY;
	}
	if (out->stand != 0 && out->stand != 1) {
		*what = "fd_stand is 0 (device form) or 1 (standard form)";
		return TD_CASE_ERR_GEOMETRY;
	}
	if (out->nplanes > 16) {
		*what = "fd_nplanes above 16 is past the plane cap the VDI itself "
		        "applies (vdi_raster.c:963), so the call could only ever be "
		        "a no-op -- and the geometry bound below assumes 16";
		return TD_CASE_ERR_GEOMETRY;
	}
	if (out->w > (int16_t)(out->wdwidth * 16)) {
		*what = "fd_w is wider than fd_wdwidth describes; fd_wdwidth is "
		        "(fd_w + 15) / 16";
		return TD_CASE_ERR_GEOMETRY;
	}
	/* The bound that makes a mistyped digit harmless: sixteen WORDs per
	 * sixteen pixels covers the TT's planar reading (fd_nplanes <= 16
	 * interleaved WORDs) and NewTOS' chunky one (one WORD per pixel)
	 * alike.  32-bit arithmetic so the product cannot wrap. */
	if ((int32_t)out->h * (int32_t)out->wdwidth * 16L >
	    (int32_t)TD_RAST_BUF_WORDS) {
		*what = "the form does not fit the dumper's buffer: "
		        "fd_h * fd_wdwidth * 16 must be at most TD_RAST_BUF_WORDS";
		return TD_CASE_ERR_GEOMETRY;
	}
	return 0;
}

/* arb_corner (vdi_raster.c:116-124): the VDI sorts both rectangles into
 * (min,max) order before it uses them, so the guard must check what the VDI
 * will actually address, not what the table happens to spell. */
static void sort_rect(const int16_t *in, int16_t *x1, int16_t *y1,
                      int16_t *x2, int16_t *y2)
{
	*x1 = in[0] < in[2] ? in[0] : in[2];
	*x2 = in[0] < in[2] ? in[2] : in[0];
	*y1 = in[1] < in[3] ? in[1] : in[3];
	*y2 = in[1] < in[3] ? in[3] : in[1];
}

static int check_rect(const int16_t *pts, const struct td_rast_mfdb *m,
                      int16_t ext_w, int16_t ext_h, const char **what)
{
	int16_t x1, y1, x2, y2;

	sort_rect(pts, &x1, &y1, &x2, &y2);
	if (ext_w > 0) {                   /* destination: extent comes from src */
		x2 = (int16_t)(x1 + ext_w - 1);
		y2 = (int16_t)(y1 + ext_h - 1);
	}
	if (x1 < 0 || y1 < 0 || x2 >= m->w || y2 >= m->h) {
		*what = (m->buffer == TD_RAST_BUF_SCREEN)
		      ? "the rectangle leaves the declared screen; a blit past the "
		        "end of video memory would scribble over the machine"
		      : "the rectangle leaves the form it addresses; EmuTOS does "
		        "not bounds-check a memory MFDB (vdi_raster.c:915-950), so "
		        "the blit would scribble over the machine";
		return TD_CASE_ERR_GEOMETRY;
	}
	return 0;
}

/* The two rectangles of a 109/121 line, case or prelude. */
static int check_copy_rects(int16_t opcode, const int16_t *ptsin,
                            uint16_t n_ptsin, const struct td_rast_mfdb *src,
                            const struct td_rast_mfdb *dst, const char **what)
{
	int16_t x1, y1, x2, y2;
	int rc;

	if (opcode == 110) {
		if (n_ptsin != 0) {
			*what = "vr_trnfm reads no rectangle; leave ptsin empty";
			return TD_CASE_ERR_GEOMETRY;
		}
		if (src->buffer == TD_RAST_BUF_SCREEN ||
		    dst->buffer == TD_RAST_BUF_SCREEN) {
			*what = "vr_trnfm may not name the screen: it dereferences "
			        "fd_addr without a NULL check (vdi_raster.c:295-297)";
			return TD_CASE_ERR_GEOMETRY;
		}
		/*
		 * vr_trnfm walks fd_nplanes planes of fd_h * fd_wdwidth WORDs,
		 * both taken from the SOURCE MFDB (vdi_raster.c:298-300), and
		 * writes as many into the destination buffer.  No extra bound is
		 * needed: fd_nplanes is capped at 16 and parse_mfdb already
		 * required fd_h * fd_wdwidth * 16 to fit, so the transformed
		 * form fits either buffer by construction.
		 */
		return 0;
	}

	if (n_ptsin != 8) {
		*what = "vro_cpyfm and vrt_cpyfm read PTSIN[0..7]; state all eight "
		        "values, or the rest stays poisoned and the blit goes wild";
		return TD_CASE_ERR_GEOMETRY;
	}
	rc = check_rect(ptsin, src, 0, 0, what);
	if (rc != 0)
		return rc;
	/* The destination extent is the SOURCE extent, not PTSIN[6..7]
	 * (dont_clip, vdi_raster.c:882-900) -- so that is what the copy
	 * addresses and what has to fit. */
	sort_rect(ptsin, &x1, &y1, &x2, &y2);
	rc = check_rect(ptsin + 4, dst, (int16_t)(x2 - x1 + 1),
	                (int16_t)(y2 - y1 + 1), what);
	if (rc != 0)
		return rc;
	/* PTSIN[6..7] as WRITTEN has to fit as well, even though the copy
	 * extent does not come from it: the overlap-reversal path starts
	 * from d_xmax/d_ymax, which dont_clip takes literally out of those
	 * two slots (vdi_raster.c:895-898, used at :995-999). */
	return check_rect(ptsin + 4, dst, 0, 0, what);
}

static int parse_rast_line(const char **col, const size_t *col_len,
                           const struct td_rast_screen *screen,
                           struct td_rast_case *c, const char **what)
{
	int16_t fill[6], v;
	uint16_t nfill = 0;
	int rc;

	rc = parse_value(col[0], col_len[0], &v);
	if (rc != 0) {
		*what = "opcode is not a number";
		return rc;
	}
	/* The opcode guard, before anything else about the line matters. */
	if (v != 109 && v != 110 && v != 121) {
		*what = "only the three raster opcodes belong in this table: "
		        "109 vro_cpyfm, 110 vr_trnfm, 121 vrt_cpyfm";
		return TD_CASE_ERR_UNSAFE_OPCODE;
	}
	c->opcode = v;

	rc = parse_list(col[1], col_len[1], c->intin, TD_RAST_INTIN_N,
	                &c->n_intin);
	if (rc != 0) {
		*what = "intin column";
		return rc;
	}
	rc = parse_list(col[2], col_len[2], c->ptsin, TD_RAST_PTSIN_N,
	                &c->n_ptsin);
	if (rc != 0) {
		*what = "ptsin column";
		return rc;
	}
	if (c->n_ptsin & 1) {
		*what = "ptsin holds an odd number of values, so it cannot be a "
		        "list of x,y points";
		return TD_CASE_ERR_ODD_PTSIN;
	}

	rc = parse_mfdb(col[3], col_len[3], &c->src, screen, what);
	if (rc != 0)
		return rc;
	rc = parse_mfdb(col[4], col_len[4], &c->dst, screen, what);
	if (rc != 0)
		return rc;

	rc = parse_list(col[5], col_len[5], fill, 6, &nfill);
	if (rc != 0) {
		*what = "fill column";
		return rc;
	}
	if (nfill != 2 && nfill != 6) {
		*what = "the fill column is two seeds (buffer A, buffer B), or six "
		        "values: the two seeds and a readback rectangle rx,ry,rw,rh";
		return TD_CASE_ERR_COLUMNS;
	}
	c->seed_a = (uint16_t)fill[0];
	c->seed_b = (uint16_t)fill[1];
	c->has_read = 0;
	if (nfill == 6) {
		c->has_read = 1;
		c->rx = fill[2];
		c->ry = fill[3];
		c->rw = fill[4];
		c->rh = fill[5];
		if (!screen->declared) {
			*what = "a readback needs a \"!screen w,h,planes\" line before "
			        "the first case";
			return TD_CASE_ERR_GEOMETRY;
		}
		if (c->src.buffer != TD_RAST_BUF_SCREEN &&
		    c->dst.buffer != TD_RAST_BUF_SCREEN) {
			*what = "a readback of the screen for a case that does not "
			        "name the screen measures nothing";
			return TD_CASE_ERR_GEOMETRY;
		}
		if (c->src.buffer == TD_RAST_BUF_B ||
		    c->dst.buffer == TD_RAST_BUF_B) {
			*what = "the readback lands in buffer B, so a case with a "
			        "readback may not name buffer B";
			return TD_CASE_ERR_GEOMETRY;
		}
		if (c->rw <= 0 || c->rh <= 0 || c->rw > TD_RAST_READ_W ||
		    c->rh > TD_RAST_READ_H) {
			*what = "the readback rectangle is at most 32 x 16: it is "
			        "stored in buffer B as a 32 x 16 device form";
			return TD_CASE_ERR_GEOMETRY;
		}
		if (c->rx < 0 || c->ry < 0 ||
		    (int32_t)c->rx + c->rw > (int32_t)screen->w ||
		    (int32_t)c->ry + c->rh > (int32_t)screen->h) {
			*what = "the readback rectangle leaves the declared screen";
			return TD_CASE_ERR_GEOMETRY;
		}
	}

	/*
	 * The rectangles.  vro_cpyfm and vrt_cpyfm read PTSIN[0..7]; vr_trnfm
	 * reads none.  A short PTSIN would leave the rest at the 0xDEAD poison,
	 * which is -8531 -- a coordinate the VDI would happily blit to.
	 */
	return check_copy_rects(c->opcode, c->ptsin, c->n_ptsin, &c->src,
	                        &c->dst, what);
}

/* A `+` line of the raster table: the whitelist IS the guard. */
static int parse_rast_prelude(const char **col, const size_t *col_len,
                              const struct td_rast_screen *screen,
                              struct td_rast_prelude *p, const char **what)
{
	const char *o = col[0];
	size_t olen = col_len[0];
	struct td_rast_mfdb screen_form;
	int16_t v;
	int rc;

	trim(&o, &olen);
	rc = parse_value(o + 1, olen - 1, &v);     /* step over the '+' */
	if (rc != 0) {
		*what = "prelude opcode is not a number";
		return rc;
	}
	if (v != 109 && v != 121 && v != 25 && v != 114 && v != 129) {
		*what = "a raster prelude is one of 109 vro_cpyfm, 121 vrt_cpyfm, "
		        "25 vsf_color, 114 vr_recfl, 129 vs_clip; nothing else "
		        "may precede a case here";
		return TD_CASE_ERR_UNSAFE_OPCODE;
	}
	p->opcode = v;

	rc = parse_list(col[1], col_len[1], p->intin, TD_PRELUDE_VALUES,
	                &p->n_intin);
	if (rc != 0) {
		*what = "prelude intin column";
		return rc;
	}
	rc = parse_list(col[2], col_len[2], p->ptsin, TD_PRELUDE_VALUES,
	                &p->n_ptsin);
	if (rc != 0) {
		*what = "prelude ptsin column";
		return rc;
	}
	if (p->n_ptsin & 1) {
		*what = "prelude ptsin holds an odd number of values, so it cannot "
		        "be a point list";
		return TD_CASE_ERR_ODD_PTSIN;
	}

	/* The fill column of a prelude is always `-`: the buffers are laid
	 * once per case, from the case's own seeds. */
	{
		const char *f = col[5];
		size_t flen = col_len[5];

		trim(&f, &flen);
		if (flen != 1 || f[0] != '-') {
			*what = "a prelude has no fill column of its own; write `-`";
			return TD_CASE_ERR_COLUMNS;
		}
	}

	if (p->opcode == 109 || p->opcode == 121) {
		rc = parse_mfdb(col[3], col_len[3], &p->src, screen, what);
		if (rc != 0)
			return rc;
		rc = parse_mfdb(col[4], col_len[4], &p->dst, screen, what);
		if (rc != 0)
			return rc;
		return check_copy_rects(p->opcode, p->ptsin, p->n_ptsin, &p->src,
		                        &p->dst, what);
	}

	/* The three attribute / drawing preludes carry no MFDB. */
	{
		const char *a = col[3], *b = col[4];
		size_t alen = col_len[3], blen = col_len[4];

		trim(&a, &alen);
		trim(&b, &blen);
		if (alen != 1 || a[0] != '-' || blen != 1 || b[0] != '-') {
			*what = "only a 109/121 prelude carries MFDBs; write `-` twice";
			return TD_CASE_ERR_COLUMNS;
		}
	}
	memset(&p->src, 0, sizeof p->src);
	memset(&p->dst, 0, sizeof p->dst);

	switch (p->opcode) {
	case 25:                                   /* vsf_color */
		if (p->n_intin != 1 || p->n_ptsin != 0 || p->intin[0] < 0) {
			*what = "vsf_color takes one non-negative intin and no ptsin";
			return TD_CASE_ERR_GEOMETRY;
		}
		return 0;
	case 114:                                  /* vr_recfl */
		if (p->n_intin != 0 || p->n_ptsin != 4) {
			*what = "vr_recfl takes no intin and four ptsin";
			return TD_CASE_ERR_GEOMETRY;
		}
		break;
	case 129:                                  /* vs_clip */
		if (p->n_intin != 1 || p->n_ptsin != 4 ||
		    (p->intin[0] != 0 && p->intin[0] != 1)) {
			*what = "vs_clip takes one intin (0 or 1) and four ptsin";
			return TD_CASE_ERR_GEOMETRY;
		}
		break;
	}
	/* Both draw on / describe the screen: the rectangle must lie inside
	 * the declared screen.  vr_recfl draws where it is told, and a clip
	 * rectangle that leaves the screen would let a later case do so. */
	if (!screen->declared) {
		*what = "vr_recfl and vs_clip preludes need a \"!screen w,h,planes\" "
		        "line before the first case";
		return TD_CASE_ERR_GEOMETRY;
	}
	memset(&screen_form, 0, sizeof screen_form);
	screen_form.buffer = TD_RAST_BUF_SCREEN;
	screen_form.w = screen->w;
	screen_form.h = screen->h;
	return check_rect(p->ptsin, &screen_form, 0, 0, what);
}

/* The one directive of the raster table: `!screen <TAB> w,h,planes`. */
static int parse_rast_directive(const char *line, size_t len,
                                struct td_rast_case_table *t, int late,
                                const char **what)
{
	const char *col[2];
	size_t col_len[2];
	const char *name;
	size_t name_len;
	int16_t v[3];
	uint16_t n = 0;
	int rc;

	if (split_columns(line, len, col, col_len, 2) < 2) {
		*what = "a directive is a name, a tab and its value";
		return TD_CASE_ERR_DIRECTIVE;
	}
	name = col[0];
	name_len = col_len[0];
	trim(&name, &name_len);
	if (name_len != 7 || memcmp(name, "!screen", 7) != 0) {
		*what = "the raster table knows one directive: !screen w,h,planes";
		return TD_CASE_ERR_DIRECTIVE;
	}
	if (late) {
		*what = "!screen must come before the first case or prelude";
		return TD_CASE_ERR_DIRECTIVE;
	}
	if (t->screen.declared) {
		*what = "!screen may appear only once";
		return TD_CASE_ERR_DIRECTIVE;
	}
	rc = parse_list(col[1], col_len[1], v, 3, &n);
	if (rc != 0 || n != 3) {
		*what = "!screen takes three values: w,h,planes";
		return TD_CASE_ERR_DIRECTIVE;
	}
	if (v[0] <= 0 || v[1] <= 0 || v[2] <= 0 || v[2] > 16) {
		*what = "!screen: w and h must be positive, planes 1..16";
		return TD_CASE_ERR_DIRECTIVE;
	}
	t->screen.declared = 1;
	t->screen.w = v[0];
	t->screen.h = v[1];
	t->screen.planes = v[2];
	return 0;
}

int td_rast_cases_parse(struct td_rast_case_table *t, const char *buf,
                        size_t len, struct td_case_error *err)
{
	struct td_line_iter it;
	const char *line;
	size_t line_len;
	uint16_t line_no;
	uint16_t pending = 0;      /* preludes seen since the last case */
	uint16_t last_prelude = 0; /* line number of the newest one, for errors */
	int is_case;

	memset(t, 0, sizeof *t);
	t->crc = td_crc32((const uint8_t *)buf, len);
	if (err) {
		err->line = 0;
		err->what = NULL;
	}

	td_line_iter_init(&it, buf, len);
	while (td_line_iter_next(&it, &line, &line_len, &line_no, &is_case)) {
		const char *col[TD_RAST_CASE_COLUMNS];
		size_t col_len[TD_RAST_CASE_COLUMNS];
		const char *what = NULL;
		struct td_rast_case *c;
		int rc;

		if (!is_case) {
			/* A '!' line carries no call, but the one directive this
			 * table has must be honoured or refused, never skipped. */
			if (td_case_line_is_directive(line, line_len)) {
				rc = parse_rast_directive(line, line_len, t,
				                          t->count > 0 || pending > 0,
				                          &what);
				if (rc != 0) {
					fail(err, line_no, what);
					return rc;
				}
			}
			continue;
		}

		if (split_columns(line, line_len, col, col_len,
		                  TD_RAST_CASE_COLUMNS) < TD_RAST_CASE_COLUMNS) {
			fail(err, line_no, "needs seven tab separated columns");
			return TD_CASE_ERR_COLUMNS;
		}

		/* A '+' line is a prelude (cases.h, THE SCREEN): a call issued
		 * after the reset and before the measured call, never recorded.
		 * host/casefile.c skips it through td_case_line_is_prelude, so
		 * both readers agree on which line carries which case id. */
		if (td_case_line_is_prelude(line, line_len)) {
			struct td_rast_prelude *p;

			if (pending >= TD_MAX_PRELUDES ||
			    t->prelude_count >= TD_MAX_PRELUDES) {
				fail(err, line_no, "more preludes than TD_MAX_PRELUDES");
				return TD_CASE_ERR_TOO_MANY_PRE;
			}
			p = &t->preludes[t->prelude_count + pending];
			memset(p, 0, sizeof *p);
			rc = parse_rast_prelude(col, col_len, &t->screen, p, &what);
			if (rc != 0) {
				fail(err, line_no, what);
				return rc;
			}
			pending++;
			last_prelude = line_no;
			continue;
		}

		if (t->count >= TD_RAST_MAX_CASES) {
			fail(err, line_no, "more cases than TD_RAST_MAX_CASES");
			return TD_CASE_ERR_TOO_MANY_CASES;
		}

		c = &t->cases[t->count];
		memset(c, 0, sizeof *c);
		c->line = line_no;
		c->prelude_first = t->prelude_count;
		c->prelude_count = pending;
		t->prelude_count = (uint16_t)(t->prelude_count + pending);
		pending = 0;

		rc = parse_rast_line(col, col_len, &t->screen, c, &what);
		if (rc != 0) {
			fail(err, line_no, what);
			return rc;
		}

		td_case_line_comment_n(line, line_len, TD_RAST_CASE_COLUMNS,
		                       c->comment, TD_COMMENT_LEN);
		t->count++;
	}

	if (pending > 0) {
		fail(err, last_prelude, "prelude with no case after it");
		return TD_CASE_ERR_DANGLING;
	}

	return 0;
}

int td_rast_cases_load(struct td_rast_case_table *t, const char *path,
                       struct td_case_error *err)
{
	char *raw;
	size_t got;
	int rc;

	rc = slurp(path, &raw, &got, err);
	if (rc != 0)
		return rc;

	rc = td_rast_cases_parse(t, raw, got, err);
	free(raw);
	return rc;
}

/* ------------------------------------------------------------------ */
/* Call tables: BIOS + XBIOS (cases.h, "Call tables")                  */
/* ------------------------------------------------------------------ */

/*
 * One 32-bit value, decimal or 0x hex, -2147483648..4294967295: a LONG is
 * stated as a bit pattern (0xFFFFFFFF) as often as a number (-1), and both
 * spellings must reach the machine as the same 32 bits.
 */
static int parse_value32(const char *s, size_t len, int32_t *out)
{
	int negative = 0, base = 10, digits = 0;
	uint32_t v = 0;
	size_t i = 0;

	trim(&s, &len);
	if (len == 0)
		return TD_CASE_ERR_VALUE;
	if (s[0] == '-') {
		negative = 1;
		i = 1;
	} else if (s[0] == '+') {
		i = 1;
	}
	if (len - i > 2 && s[i] == '0' && (s[i + 1] == 'x' || s[i + 1] == 'X')) {
		base = 16;
		i += 2;
	}
	for (; i < len; i++) {
		uint32_t d;
		char c = s[i];

		if (c >= '0' && c <= '9')
			d = (uint32_t)(c - '0');
		else if (base == 16 && c >= 'a' && c <= 'f')
			d = (uint32_t)(c - 'a' + 10);
		else if (base == 16 && c >= 'A' && c <= 'F')
			d = (uint32_t)(c - 'A' + 10);
		else
			return TD_CASE_ERR_VALUE;
		if (v > (0xFFFFFFFFuL - d) / (uint32_t)base)
			return TD_CASE_ERR_RANGE;
		v = v * (uint32_t)base + d;
		digits++;
	}
	if (digits == 0)
		return TD_CASE_ERR_VALUE;
	if (negative) {
		if (v > 0x80000000uL)
			return TD_CASE_ERR_RANGE;
		*out = (int32_t)(0u - v);
	} else {
		*out = (int32_t)v;
	}
	return 0;
}

/* A small unsigned number, for the trap, the function and the capture. */
static int parse_small(const char *s, size_t len, uint32_t max, uint32_t *out)
{
	int32_t v;
	int rc = parse_value32(s, len, &v);

	if (rc != 0)
		return rc;
	if (v < 0 || (uint32_t)v > max)
		return TD_CASE_ERR_RANGE;
	*out = (uint32_t)v;
	return 0;
}

const char *td_retclass_name(uint8_t retclass)
{
	switch (retclass) {
	case TD_RET_EXACT:   return "EXACT";
	case TD_RET_IGNORE:  return "IGNORE";
	case TD_RET_NONZERO: return "NONZERO";
	case TD_RET_HANDLE:  return "HANDLE";
	case TD_RET_PTR_RAM: return "PTR_RAM";
	default:             return "?";
	}
}

static int span_is(const char *s, size_t len, const char *word)
{
	return strlen(word) == len && memcmp(s, word, len) == 0;
}

static int parse_retclass(const char *s, size_t len, struct td_call_case *c,
                          const char **why)
{
	static const uint8_t classes[] = { TD_RET_EXACT, TD_RET_IGNORE,
	                                   TD_RET_NONZERO, TD_RET_HANDLE,
	                                   TD_RET_PTR_RAM };
	size_t dot, i;

	trim(&s, &len);
	for (dot = 0; dot < len && s[dot] != '.'; dot++)
		;
	c->retclass = 0xFF;
	for (i = 0; i < sizeof classes; i++)
		if (span_is(s, dot, td_retclass_name(classes[i])))
			c->retclass = classes[i];
	if (c->retclass == 0xFF) {
		*why = "retclass is not EXACT, IGNORE, NONZERO, HANDLE or PTR_RAM";
		return TD_CASE_ERR_RETCLASS;
	}
	c->retwidth = TD_RET_WIDTH_LONG;
	if (dot == len)
		return 0;
	/* ".L" is the default and is not spelt: one spelling per meaning, so
	 * that two tables saying the same thing have the same bytes. */
	if (span_is(s + dot, len - dot, ".W"))
		c->retwidth = TD_RET_WIDTH_WORD;
	else if (span_is(s + dot, len - dot, ".B"))
		c->retwidth = TD_RET_WIDTH_BYTE;
	else {
		*why = "retclass width is not .W or .B (LONG has no suffix)";
		return TD_CASE_ERR_RETCLASS;
	}
	if (c->retclass == TD_RET_IGNORE) {
		*why = "IGNORE takes no width";
		return TD_CASE_ERR_RETCLASS;
	}
	if (c->retclass == TD_RET_PTR_RAM) {
		*why = "PTR_RAM takes no width: a pointer is a LONG";
		return TD_CASE_ERR_RETCLASS;
	}
	return 0;
}

/* ------------------------------------------------------------------ */
/* Layouts: structures behind a pointer (cases.h, TASK/TOOL16)         */
/* ------------------------------------------------------------------ */

uint8_t td_layout_width(uint8_t type, int abi)
{
	switch (type) {
	case TD_LT_BYTE:  return 1;
	case TD_LT_WORD:  return 2;
	case TD_LT_LONG:  return 4;
	case TD_LT_CLONG:
	case TD_LT_PTR:   return abi == TD_LAYOUT_ABI_LP64 ? 8 : 4;
	default:          return 0;
	}
}

/* C's alignment of a scalar field: the m68k ABI aligns everything wider
 * than a byte on 2 (m68k-atari-mint-gcc; bios/iorec.h, the 14-byte IOREC),
 * LP64 aligns every type on its own size. */
static uint8_t layout_align(uint8_t type, int abi)
{
	uint8_t w = td_layout_width(type, abi);

	if (abi == TD_LAYOUT_ABI_M68K)
		return w > 1 ? 2 : 1;
	return w;
}

int td_layout_native_abi(void)
{
	return td_native_ptr_width() == 8 ? TD_LAYOUT_ABI_LP64
	                                  : TD_LAYOUT_ABI_M68K;
}

int td_layout_slot_find(const struct td_layout *l, const char *path)
{
	int i;

	for (i = 0; i < (int)l->nslots; i++)
		if (strcmp(l->slot[i].name, path) == 0)
			return i;
	return -1;
}

/* A name: [a-z_][a-z0-9_]*, at most TD_LAYOUT_NAME_LEN - 1 characters. */
static int layout_name_ok(const char *s, size_t len)
{
	size_t i;

	if (len == 0 || len >= TD_LAYOUT_NAME_LEN)
		return 0;
	for (i = 0; i < len; i++) {
		char c = s[i];

		if (c == '_' || (c >= 'a' && c <= 'z') ||
		    (i > 0 && c >= '0' && c <= '9'))
			continue;
		return 0;
	}
	return 1;
}

static int layout_lookup(const struct td_layout *set, uint8_t n,
                         const char *s, size_t len)
{
	int i;

	for (i = 0; i < (int)n; i++)
		if (span_is(s, len, set[i].name))
			return i;
	return -1;
}

/* "<a>/<b>", two unsigned numbers 0..max. */
static int parse_pair(const char *s, size_t len, uint32_t max, uint32_t *a,
                      uint32_t *b)
{
	size_t slash;

	trim(&s, &len);
	for (slash = 0; slash < len && s[slash] != '/'; slash++)
		;
	if (slash == 0 || slash >= len - 1)
		return TD_CASE_ERR_VALUE;
	if (parse_small(s, slash, max, a) != 0 ||
	    parse_small(s + slash + 1, len - slash - 1, max, b) != 0)
		return TD_CASE_ERR_VALUE;
	return 0;
}

/* Append slot `src` of layout `from` under the prefix, as part of `l`.
 * `ptr` is the slot index of the pointer the copied layout hangs from
 * (-1: embedded, at `base_off` in l's own structure). */
static int layout_append(struct td_layout *l, const struct td_layout *from,
                         const char *prefix, size_t plen, int ptr,
                         const uint16_t *base_off, uint8_t base_hops,
                         const char **why)
{
	int first = l->nslots, i;

	for (i = 0; i < (int)from->nslots; i++) {
		const struct td_layout_slot *s = &from->slot[i];
		struct td_layout_slot *d;
		size_t nlen = strlen(s->name);

		if (l->nslots >= TD_LAYOUT_SLOTS) {
			*why = "the layout has more fields than the record's 32 slots";
			return TD_CASE_ERR_TOO_MANY_VALUES;
		}
		if (plen + nlen >= TD_LAYOUT_PATH_LEN) {
			*why = "a field path is longer than TD_LAYOUT_PATH_LEN";
			return TD_CASE_ERR_RANGE;
		}
		d = &l->slot[l->nslots];
		*d = *s;
		memcpy(d->name, prefix, plen);
		memcpy(d->name + plen, s->name, nlen + 1);
		if (s->parent < 0) {
			d->parent = (int8_t)ptr;
			if (ptr < 0) {
				d->off[0] = (uint16_t)(s->off[0] + base_off[0]);
				d->off[1] = (uint16_t)(s->off[1] + base_off[1]);
			}
		} else {
			d->parent = (int8_t)(s->parent + first);
		}
		d->hops = (uint8_t)(s->hops + base_hops);
		if (d->hops > TD_LAYOUT_MAX_HOPS) {
			*why = "the layout follows pointers deeper than "
			       "TD_LAYOUT_MAX_HOPS";
			return TD_CASE_ERR_RANGE;
		}
		l->nslots++;
	}
	return 0;
}

/*
 * One field "<name>:<type>@<m68k>/<lp64>[:<class>]" of layout `l`.
 * `cursor`/`maxal` are the C layout so far, per ABI.
 */
static int layout_field(struct td_layout *l, const struct td_layout *set,
                        uint8_t n, const char *f, size_t flen,
                        uint32_t *cursor, uint8_t *maxal, const char **why)
{
	size_t c1, at, c2, i;
	const char *t;
	size_t tlen;
	uint32_t off[TD_LAYOUT_ABIS], width[TD_LAYOUT_ABIS];
	uint8_t align[TD_LAYOUT_ABIS], type = 0;
	int target = -1, embedded = 0, abi, rc;
	uint8_t cls = 0xFF;

	trim(&f, &flen);
	for (c1 = 0; c1 < flen && f[c1] != ':'; c1++)
		;
	for (at = c1; at < flen && f[at] != '@'; at++)
		;
	for (c2 = at; c2 < flen && f[c2] != ':'; c2++)
		;
	*why = "a layout field is <name>:<type>@<m68k offset>/<LP64 offset>"
	       "[:<class>]";
	if (c1 == 0 || c1 >= flen || at >= flen)
		return TD_CASE_ERR_VALUE;
	if (!layout_name_ok(f, c1)) {
		*why = "a field name is [a-z_][a-z0-9_]*, at most 15 characters";
		return TD_CASE_ERR_VALUE;
	}
	for (i = 0; i < l->nslots; i++) {
		const char *sn = l->slot[i].name;

		/* the field itself, or the head of an embedded or followed
		 * layout's paths ("in.ibuf", "mp_free->md_start") */
		if (strncmp(sn, f, c1) == 0 &&
		    (sn[c1] == '\0' || sn[c1] == '.' ||
		     (sn[c1] == '-' && sn[c1 + 1] == '>'))) {
			*why = "a field name stands twice in one layout";
			return TD_CASE_ERR_VALUE;
		}
	}

	t = f + c1 + 1;
	tlen = at - c1 - 1;
	if (tlen == 1) {
		switch (t[0]) {
		case 'b': type = TD_LT_BYTE;  break;
		case 'w': type = TD_LT_WORD;  break;
		case 'L': type = TD_LT_LONG;  break;
		case 'l': type = TD_LT_CLONG; break;
		case 'p': type = TD_LT_PTR;   break;
		default:  break;
		}
	} else if (tlen > 2 && t[0] == 'p' && t[1] == '>') {
		type = TD_LT_PTR;
		target = layout_lookup(set, n, t + 2, tlen - 2);
		if (target < 0) {
			*why = "p><layout> names no layout defined before this one";
			return TD_CASE_ERR_VALUE;
		}
	} else if (tlen > 2 && t[0] == '[' && t[tlen - 1] == ']') {
		embedded = 1;
		target = layout_lookup(set, n, t + 1, tlen - 2);
		if (target < 0) {
			*why = "[<layout>] names no layout defined before this one";
			return TD_CASE_ERR_VALUE;
		}
	}
	if (!type && !embedded) {
		*why = "a field type is b, w, L, l, p, p><layout> or [<layout>]";
		return TD_CASE_ERR_VALUE;
	}

	rc = parse_pair(f + at + 1, c2 - at - 1, 0xFFFF, &off[0], &off[1]);
	if (rc != 0) {
		*why = "a field offset is <m68k>/<LP64>, two numbers";
		return rc;
	}
	if (c2 < flen) {
		const char *k = f + c2 + 1;
		size_t klen = flen - c2 - 1;
		static const uint8_t classes[] = { TD_RET_EXACT, TD_RET_IGNORE,
		                                   TD_RET_NONZERO, TD_RET_HANDLE,
		                                   TD_RET_PTR_RAM };

		trim(&k, &klen);
		for (i = 0; i < sizeof classes; i++)
			if (span_is(k, klen, td_retclass_name(classes[i])))
				cls = classes[i];
		if (cls == 0xFF) {
			*why = "a field class is EXACT, IGNORE, NONZERO, HANDLE or "
			       "PTR_RAM";
			return TD_CASE_ERR_RETCLASS;
		}
		if (embedded) {
			*why = "an embedded [<layout>] takes no class: its fields "
			       "carry their own";
			return TD_CASE_ERR_RETCLASS;
		}
	} else if (!embedded) {
		cls = type == TD_LT_PTR ? TD_RET_PTR_RAM : TD_RET_EXACT;
	}

	/* the C layout: each offset is the one C gives this field after the
	 * ones before it, on both ABIs */
	for (abi = 0; abi < TD_LAYOUT_ABIS; abi++) {
		uint32_t expect;

		if (embedded) {
			width[abi] = set[target].size[abi];
			align[abi] = set[target].align[abi];
		} else {
			width[abi] = td_layout_width(type, abi);
			align[abi] = layout_align(type, abi);
		}
		expect = (cursor[abi] + align[abi] - 1u) & ~(uint32_t)(align[abi] - 1u);
		if (off[abi] != expect) {
			*why = abi == TD_LAYOUT_ABI_M68K
			       ? "an m68k offset is not where C puts the field "
			         "(everything but a UBYTE WORD aligned, no gaps)"
			       : "an LP64 offset is not where C puts the field "
			         "(natural alignment, no gaps)";
			return TD_CASE_ERR_GEOMETRY;
		}
		cursor[abi] = off[abi] + width[abi];
		if (align[abi] > maxal[abi])
			maxal[abi] = align[abi];
	}

	if (embedded) {
		char prefix[TD_LAYOUT_NAME_LEN + 1];
		uint16_t base[TD_LAYOUT_ABIS];

		memcpy(prefix, f, c1);
		prefix[c1] = '.';
		base[0] = (uint16_t)off[0];
		base[1] = (uint16_t)off[1];
		return layout_append(l, &set[target], prefix, c1 + 1, -1, base, 0,
		                     why);
	}

	if (l->nslots >= TD_LAYOUT_SLOTS) {
		*why = "the layout has more fields than the record's 32 slots";
		return TD_CASE_ERR_TOO_MANY_VALUES;
	}
	{
		struct td_layout_slot *s = &l->slot[l->nslots];
		int self = l->nslots;

		memset(s, 0, sizeof *s);
		memcpy(s->name, f, c1);
		s->name[c1] = '\0';
		s->type   = type;
		s->cls    = cls;
		s->parent = -1;
		s->follow = (int8_t)target;
		s->hops   = 0;
		s->off[0] = (uint16_t)off[0];
		s->off[1] = (uint16_t)off[1];
		l->nslots++;
		if (target >= 0) {
			char prefix[TD_LAYOUT_NAME_LEN + 2];
			static const uint16_t zero[TD_LAYOUT_ABIS] = { 0, 0 };

			memcpy(prefix, f, c1);
			prefix[c1] = '-';
			prefix[c1 + 1] = '>';
			return layout_append(l, &set[target], prefix, c1 + 2, self,
			                     zero, 1, why);
		}
	}
	return 0;
}

int td_layout_parse(struct td_layout *set, uint8_t *n, const char *line,
                    size_t len, const char **why)
{
	const char *col[4];
	size_t col_len[4];
	const char *s;
	size_t slen, i, start;
	struct td_layout *l;
	uint32_t size[TD_LAYOUT_ABIS], cursor[TD_LAYOUT_ABIS] = { 0, 0 };
	uint8_t maxal[TD_LAYOUT_ABIS] = { 1, 1 };
	int abi, rc;

	if (*n >= TD_LAYOUT_MAX) {
		*why = "more than eight layouts in one table";
		return TD_CASE_ERR_TOO_MANY_VALUES;
	}
	if (split_columns(line, len, col, col_len, 4) < 4) {
		*why = "!layout <name> <m68k size>/<LP64 size> <fields>";
		return TD_CASE_ERR_COLUMNS;
	}
	l = &set[*n];
	memset(l, 0, sizeof *l);
	s = col[1];
	slen = col_len[1];
	trim(&s, &slen);
	if (!layout_name_ok(s, slen)) {
		*why = "a layout name is [a-z_][a-z0-9_]*, at most 15 characters";
		return TD_CASE_ERR_DIRECTIVE;
	}
	if (layout_lookup(set, *n, s, slen) >= 0) {
		*why = "a layout name stands twice";
		return TD_CASE_ERR_DIRECTIVE;
	}
	memcpy(l->name, s, slen);
	l->name[slen] = '\0';
	rc = parse_pair(col[2], col_len[2], TD_CALL_BUF_BYTES, &size[0], &size[1]);
	if (rc != 0 || size[0] == 0 || size[1] == 0) {
		*why = "a layout size is <m68k>/<LP64>, 1..1024 bytes each";
		return TD_CASE_ERR_DIRECTIVE;
	}

	s = col[3];
	slen = col_len[3];
	trim(&s, &slen);
	if (slen == 0) {
		*why = "a layout has at least one field";
		return TD_CASE_ERR_DIRECTIVE;
	}
	for (i = 0, start = 0; ; i++) {
		if (i == slen || s[i] == ',') {
			rc = layout_field(l, set, *n, s + start, i - start, cursor,
			                  maxal, why);
			if (rc != 0)
				return rc;
			if (i == slen)
				break;
			start = i + 1;
		}
	}
	for (abi = 0; abi < TD_LAYOUT_ABIS; abi++) {
		uint32_t end = (cursor[abi] + maxal[abi] - 1u) &
		               ~(uint32_t)(maxal[abi] - 1u);

		if (size[abi] != end) {
			*why = abi == TD_LAYOUT_ABI_M68K
			       ? "the m68k size is not the end of the last field "
			         "rounded to the structure's alignment"
			       : "the LP64 size is not the end of the last field "
			         "rounded to the structure's alignment";
			return TD_CASE_ERR_GEOMETRY;
		}
		l->size[abi]  = (uint16_t)size[abi];
		l->align[abi] = maxal[abi];
	}
	(*n)++;
	return 0;
}

static int parse_call_args(const char *s, size_t len, struct td_call_case *c,
                           const char **why)
{
	size_t i = 0, start = 0;

	c->nargs = 0;
	trim(&s, &len);
	if (len == 1 && s[0] == '-')
		return 0;
	if (len == 0) {
		*why = "the args column is empty (\"-\" for none)";
		return TD_CASE_ERR_VALUE;
	}
	for (;;) {
		if (i == len || s[i] == ',') {
			const char *a = s + start;
			size_t alen = i - start;
			struct td_call_arg *arg;
			int32_t v;
			int rc;

			trim(&a, &alen);
			if (c->nargs >= TD_CALL_ARGS_N) {
				*why = "more than eight arguments";
				return TD_CASE_ERR_TOO_MANY_VALUES;
			}
			arg = &c->arg[c->nargs];
			if (alen < 3 || a[1] != ':') {
				*why = "an argument is not w:<v>, l:<v>, b:<n>, f:<n>, "
				       "t:0 or u:0";
				return TD_CASE_ERR_VALUE;
			}
			rc = parse_value32(a + 2, alen - 2, &v);
			if (rc != 0) {
				*why = "an argument value is not a number";
				return rc;
			}
			switch (a[0]) {
			case 'w':
				if (v < -32768 || v > 65535) {
					*why = "a WORD argument does not fit in 16 bits";
					return TD_CASE_ERR_RANGE;
				}
				arg->kind  = TD_CALL_ARG_WORD;
				arg->value = (int32_t)(int16_t)(uint16_t)v;
				break;
			case 'l':
				arg->kind  = TD_CALL_ARG_LONG;
				arg->value = v;
				break;
			case 'b':
				if (v < 0 || v >= TD_CALL_BUF_N) {
					*why = "a buffer argument names no dumper buffer";
					return TD_CASE_ERR_RANGE;
				}
				arg->kind  = TD_CALL_ARG_BUF;
				arg->value = v;
				break;
			case 'f':
				if (v < 0 || v >= TD_CALL_CB_N) {
					*why = "a callback argument names no dumper callback";
					return TD_CASE_ERR_RANGE;
				}
				arg->kind  = TD_CALL_ARG_FUNC;
				arg->value = v;
				break;
			case 't':
				if (v != 0) {
					*why = "the Super token is t:0, there is no other";
					return TD_CASE_ERR_RANGE;
				}
				arg->kind  = TD_CALL_ARG_TOKEN;
				arg->value = v;
				break;
			case 'u':
				/* the test disk's physical unit: the run's profile
				 * names it (td_run_call_set_disk_unit) */
				if (v != 0) {
					*why = "the unit token is u:0, there is no other";
					return TD_CASE_ERR_RANGE;
				}
				arg->kind  = TD_CALL_ARG_UNIT;
				arg->value = v;
				break;
			default:
				*why = "an argument is not w:<v>, l:<v>, b:<n>, f:<n>, "
				       "t:0 or u:0";
				return TD_CASE_ERR_VALUE;
			}
			c->nargs++;
			if (i == len)
				break;
			start = i + 1;
		}
		i++;
	}
	return 0;
}

/*
 * "r:<layout>" / "<argindex>:<layout>" (TOOL16): the structure behind the
 * returned pointer or in the b: buffer, field by field (cases.h, "Layouts").
 */
static int parse_layout_capture(const struct td_call_case_table *t,
                                const char *s, size_t len, size_t colon,
                                struct td_call_case *c, const char **why)
{
	uint32_t idx;
	int li = layout_lookup(t->layouts, t->nlayouts, s + colon + 1,
	                       len - colon - 1);
	const struct td_layout *l;

	if (li < 0) {
		*why = "the capture names no layout the table defines (\"!layout\" "
		       "before the first case)";
		return TD_CASE_ERR_CAPTURE;
	}
	l = &t->layouts[li];
	if (colon == 1 && s[0] == 'r') {
		if (c->retclass != TD_RET_PTR_RAM) {
			*why = "r:<layout> reads through the returned pointer: the "
			       "retclass must be PTR_RAM";
			return TD_CASE_ERR_CAPTURE;
		}
		c->cap_arg = (int8_t)TD_CALL_CAP_RET;
	} else {
		if (parse_small(s, colon, 255, &idx) != 0 || idx >= c->nargs ||
		    c->arg[idx].kind != TD_CALL_ARG_BUF) {
			*why = "capture names an argument that is not a b: buffer";
			return TD_CASE_ERR_CAPTURE;
		}
		if (l->size[0] > TD_CALL_BUF_BYTES || l->size[1] > TD_CALL_BUF_BYTES) {
			*why = "the layout is larger than a dumper buffer";
			return TD_CASE_ERR_CAPTURE;
		}
		c->cap_arg = (int8_t)idx;
	}
	c->cap_layout = (uint8_t)(li + 1);
	c->cap_unit   = 1;
	c->cap_off    = 0;
	c->cap_len    = (uint16_t)(l->nslots * TD_LAYOUT_SLOT_BYTES);
	return 0;
}

static int parse_capture(const struct td_call_case_table *t, const char *s,
                         size_t len, struct td_call_case *c, const char **why)
{
	size_t colon, end, at;
	uint32_t idx, length, off = 0;
	uint8_t unit = 1;
	int rc;

	c->cap_arg = -1;
	c->cap_layout = 0;
	trim(&s, &len);
	if (len == 1 && s[0] == '-')
		return 0;
	*why = "capture is not - or <argindex>:<length>[.B|.W|.L][@<offset>] "
	       "(r:<length>... through the returned pointer; r:<layout> or "
	       "<argindex>:<layout> field by field)";
	for (colon = 0; colon < len && s[colon] != ':'; colon++)
		;
	if (colon == 0 || colon >= len)
		return TD_CASE_ERR_CAPTURE;
	if (colon + 1 < len && s[colon + 1] >= 'a' && s[colon + 1] <= 'z')
		return parse_layout_capture(t, s, len, colon, c, why);
	for (at = colon; at < len && s[at] != '@'; at++)
		;
	for (end = colon; end < at && s[end] != '.'; end++)
		;
	if (colon == 1 && s[0] == 'r')
		idx = 0xFF;             /* through the return: checked below */
	else if (parse_small(s, colon, 255, &idx) != 0)
		return TD_CASE_ERR_CAPTURE;
	rc = parse_small(s + colon + 1, end - colon - 1, 0xFFFF, &length);
	if (rc != 0)
		return TD_CASE_ERR_CAPTURE;
	if (end < at) {
		if (span_is(s + end, at - end, ".B"))
			unit = 1;
		else if (span_is(s + end, at - end, ".W"))
			unit = 2;
		else if (span_is(s + end, at - end, ".L"))
			unit = 4;
		else {
			*why = "capture unit is not .B, .W or .L";
			return TD_CASE_ERR_CAPTURE;
		}
	}
	if (at < len && parse_small(s + at + 1, len - at - 1, 0xFFFF, &off) != 0)
		return TD_CASE_ERR_CAPTURE;

	if (idx == 0xFF) {
		/* r: reads through the address the call returned, which the
		 * runner follows only when it is a positive, even LONG -- and
		 * the comparator checks it as PTR_RAM on each side */
		if (c->retclass != TD_RET_PTR_RAM) {
			*why = "r: reads through the returned pointer: the retclass "
			       "must be PTR_RAM";
			return TD_CASE_ERR_CAPTURE;
		}
	} else if (idx >= c->nargs || c->arg[idx].kind != TD_CALL_ARG_BUF) {
		*why = "capture names an argument that is not a b: buffer";
		return TD_CASE_ERR_CAPTURE;
	}
	if (length == 0 || length > TD_CALL_CAPTURE_LEN) {
		*why = "capture length is not 1..256 bytes";
		return TD_CASE_ERR_CAPTURE;
	}
	if (length % unit != 0 || off % unit != 0) {
		*why = "capture length or offset is not a whole number of units";
		return TD_CASE_ERR_CAPTURE;
	}
	if (off + length > TD_CALL_BUF_BYTES) {
		*why = "capture runs past the end of the buffer";
		return TD_CASE_ERR_CAPTURE;
	}
	c->cap_arg  = idx == 0xFF ? (int8_t)TD_CALL_CAP_RET : (int8_t)idx;
	c->cap_unit = unit;
	c->cap_off  = (uint16_t)off;
	c->cap_len  = (uint16_t)length;
	return 0;
}

/*
 * THE GUARD (cases.h).  The whitelist: every (trap, function) whose query
 * arm is established as safe on both machines in tosdump/cases/
 * COVERAGE-BIOS.md, with the binding's argument widths and the rule its
 * values have to meet.  Citations per entry are in the ledger; the rule
 * comments name the arm that makes the call safe.
 *
 * Signature letters: 'w' a WORD, 'l' a LONG, 'p' a pointer -- a b: buffer
 * or an l: value (NULL, or -1 where the call reads -1 as "unchanged") --,
 * 'f' a dumper callback (f:<n>), 't' the Super token (t:0).
 */
enum {
	CR_NONE,         /* every value is a query                            */
	CR_ARG0_M1,      /* arg 0 is -1: the inquiry arm                      */
	CR_ARG1_M1,      /* arg 1 is -1: Setexc, Setcolor, EsetColor          */
	CR_ALL_M1,       /* every argument is -1: Kbrate, Keytbl              */
	CR_SETSCREEN,    /* logLoc, physLoc, rez all -1                       */
	CR_RSCONF,       /* all -1, or baud -2 (and the rest -1)              */
	CR_BCONIN,       /* only handles that answer at once on both machines */
	CR_BCONOUT,      /* CON / raw CON with a printable byte; or no device */
	CR_CURSCONF,     /* function 5, the blink-rate inquiry                */
	CR_GIACCESS,     /* a register read: no GIACCESS_WRITE bit            */
	CR_FLOPRATE,     /* rate -1 or -4: the two inquiries                  */
	CR_BCONMAP,      /* -1, -2, or a device number that maps nothing      */
	CR_NVM,          /* read or an invalid op; never write or reset       */
	CR_BUF0,         /* arg 0 is a dumper buffer: Protobt, Buffptr        */
	CR_BUF01,        /* args 0 and 1 are dumper buffers: Dsp_Available    */
	CR_EGETPAL,      /* buffer argument, count fits it as WORDs           */
	CR_VGETRGB,      /* buffer argument, count fits it as LONGs           */
	CR_SOUNDCMD,     /* modes 0, 1, 6 with data -1                        */
	CR_SNDSTATUS,    /* reset 0                                           */
	CR_SUPER,        /* l:1 the inquiry, l:0 / t:0 the bracket (MEM12)    */
	CR_GETMPB,       /* a b: buffer, inside the Super bracket, last       */
	                 /* (TOOL16; the bracket and "last" are the parse     */
	                 /* loop's)                                           */
	/* The disk round (TASK/TOOL07): only in a table that says "!disk";
	 * outside one these calls are refused as before (call_refusals). */
	CR_DISK_DRIVE,   /* Getbpb, Mediach: any drive but the floppies A:/B: */
	CR_RWABS         /* reads; the FastCopy NULL arm on A:/B:; writes     */
	                 /* back only, on the test drive (td_call_cases_parse)*/
};

struct call_policy {
	int16_t     trap;
	int16_t     fn_lo, fn_hi;
	const char *sig;
	const char *sig_alt;      /* a second accepted signature, or NULL */
	uint8_t     rule;
};

static const struct call_policy call_policy[] = {
	/* GEMDOS, trap #1: Super only, the processor-state call answered by
	 * the TRAP #1 front end (bdos/rwa.S:159-184, 280-346); MEM12 */
	{  1, 0x20, 0x20, "l",      "t",    CR_SUPER     },  /* Super       */
	/* BIOS, trap #13 (bios/bios.c:1252-1265) */
	{ 13,    0,    0, "p",      NULL,   CR_GETMPB    },  /* Getmpb, TOOL16 */
	{ 13,    1,    1, "w",      NULL,   CR_NONE      },  /* Bconstat    */
	{ 13,    4,    4, "wpwwwl", "wpwwul", CR_RWABS   },  /* Rwabs, !disk only; u:0 physical */
	{ 13,    7,    7, "w",      NULL,   CR_DISK_DRIVE},  /* Getbpb, !disk only */
	{ 13,    9,    9, "w",      NULL,   CR_DISK_DRIVE},  /* Mediach, !disk only */
	{ 13,    2,    2, "w",      NULL,   CR_BCONIN    },  /* Bconin      */
	{ 13,    3,    3, "ww",     NULL,   CR_BCONOUT   },  /* Bconout     */
	{ 13,    5,    5, "wl",     NULL,   CR_ARG1_M1   },  /* Setexc      */
	{ 13,    6,    6, "",       NULL,   CR_NONE      },  /* Tickcal     */
	{ 13,    8,    8, "w",      NULL,   CR_NONE      },  /* Bcostat     */
	{ 13,   10,   10, "",       NULL,   CR_NONE      },  /* Drvmap      */
	{ 13,   11,   11, "w",      NULL,   CR_ARG0_M1   },  /* Kbshift     */
	{ 13,   12,  255, "",       NULL,   CR_NONE      },  /* past table  */
	/* XBIOS, trap #14 (bios/xbios.c:1285-1515) */
	{ 14, 0x02, 0x04, "",       NULL,   CR_NONE      },  /* Physbase, Logbase, Getrez */
	{ 14, 0x05, 0x05, "ppw",    "ppww", CR_SETSCREEN },  /* Setscreen   */
	{ 14, 0x07, 0x07, "ww",     NULL,   CR_ARG1_M1   },  /* Setcolor    */
	{ 14, 0x0E, 0x0E, "w",      NULL,   CR_NONE      },  /* Iorec       */
	{ 14, 0x0F, 0x0F, "wwwwww", NULL,   CR_RSCONF    },  /* Rsconf      */
	{ 14, 0x10, 0x10, "ppp",    NULL,   CR_ALL_M1    },  /* Keytbl      */
	{ 14, 0x11, 0x11, "",       NULL,   CR_NONE      },  /* Random      */
	{ 14, 0x12, 0x12, "plww",   NULL,   CR_BUF0      },  /* Protobt     */
	{ 14, 0x15, 0x15, "ww",     NULL,   CR_CURSCONF  },  /* Cursconf    */
	{ 14, 0x17, 0x17, "",       NULL,   CR_NONE      },  /* Gettime     */
	{ 14, 0x1C, 0x1C, "ww",     NULL,   CR_GIACCESS  },  /* Giaccess    */
	{ 14, 0x20, 0x20, "p",      NULL,   CR_ARG0_M1   },  /* Dosound     */
	{ 14, 0x21, 0x21, "w",      NULL,   CR_ARG0_M1   },  /* Setprt      */
	{ 14, 0x22, 0x22, "",       NULL,   CR_NONE      },  /* Kbdvbase    */
	{ 14, 0x23, 0x23, "ww",     NULL,   CR_ALL_M1    },  /* Kbrate      */
	{ 14, 0x25, 0x25, "",       NULL,   CR_NONE      },  /* Vsync       */
	{ 14, 0x26, 0x26, "f",      NULL,   CR_NONE      },  /* Supexec: the dumper's own callback only (MEM12) */
	{ 14, 0x28, 0x28, "",       NULL,   CR_NONE      },  /* empty slot  */
	{ 14, 0x29, 0x29, "ww",     NULL,   CR_FLOPRATE  },  /* Floprate    */
	{ 14, 0x2C, 0x2C, "w",      NULL,   CR_BCONMAP   },  /* Bconmap     */
	{ 14, 0x2D, 0x2D, "",       NULL,   CR_NONE      },  /* empty slot  */
	{ 14, 0x2E, 0x2E, "wwwp",   NULL,   CR_NVM       },  /* NVMaccess   */
	{ 14, 0x2F, 0x3F, "",       NULL,   CR_NONE      },  /* empty slots */
	{ 14, 0x40, 0x40, "w",      NULL,   CR_ARG0_M1   },  /* Blitmode    */
	{ 14, 0x41, 0x4F, "",       NULL,   CR_NONE      },  /* empty slots */
	{ 14, 0x51, 0x51, "",       NULL,   CR_NONE      },  /* EgetShift   */
	{ 14, 0x52, 0x52, "w",      NULL,   CR_ARG0_M1   },  /* EsetBank    */
	{ 14, 0x53, 0x53, "ww",     NULL,   CR_ARG1_M1   },  /* EsetColor   */
	{ 14, 0x55, 0x55, "wwp",    NULL,   CR_EGETPAL   },  /* EgetPalette */
	{ 14, 0x56, 0x57, "w",      NULL,   CR_ARG0_M1   },  /* EsetGray, EsetSmear */
	{ 14, 0x58, 0x58, "w",      NULL,   CR_ARG0_M1   },  /* VsetMode    */
	{ 14, 0x59, 0x59, "",       NULL,   CR_NONE      },  /* VgetMonitor */
	{ 14, 0x5B, 0x5B, "w",      NULL,   CR_NONE      },  /* VgetSize    */
	{ 14, 0x5C, 0x5C, "",       NULL,   CR_NONE      },  /* empty slot  */
	{ 14, 0x5E, 0x5E, "wwp",    NULL,   CR_VGETRGB   },  /* VgetRGB     */
	{ 14, 0x5F, 0x5F, "w",      NULL,   CR_NONE      },  /* Vfixmode    */
	{ 14, 0x67, 0x67, "",       NULL,   CR_NONE      },  /* Dsp_GetWordSize */
	{ 14, 0x6A, 0x6A, "pp",     NULL,   CR_BUF01     },  /* Dsp_Available   */
	{ 14, 0x72, 0x72, "",       NULL,   CR_NONE      },  /* Dsp_GetProgAbility */
	{ 14, 0x75, 0x75, "w",      NULL,   CR_NONE      },  /* Dsp_InqSubrAbility */
	{ 14, 0x77, 0x78, "w",      NULL,   CR_ARG0_M1   },  /* Dsp_Hf0, Dsp_Hf1 */
	{ 14, 0x79, 0x7A, "",       NULL,   CR_NONE      },  /* Dsp_Hf2, Dsp_Hf3 */
	{ 14, 0x7D, 0x7D, "",       NULL,   CR_NONE      },  /* Dsp_Hstat   */
	{ 14, 0x82, 0x82, "ww",     NULL,   CR_SOUNDCMD  },  /* Soundcmd    */
	{ 14, 0x88, 0x88, "w",      NULL,   CR_ARG0_M1   },  /* Buffoper    */
	{ 14, 0x8C, 0x8C, "w",      NULL,   CR_SNDSTATUS },  /* Sndstatus   */
	{ 14, 0x8D, 0x8D, "p",      NULL,   CR_BUF0      },  /* Buffptr     */
	{ 14, 0x8E, 0x8E, "",       NULL,   CR_NONE      },  /* past table  */
	{  0,    0,    0, NULL,     NULL,   CR_NONE      }
};

/*
 * The calls the ledger EXCLUDES, with the reason a refusal names.  Every
 * other call that is not on the whitelist is refused as "not established".
 */
struct call_refusal {
	int16_t     trap;
	int16_t     fn;
	const char *why;
};

static const struct call_refusal call_refusals[] = {
	{ 13, 0x04, "Rwabs reads and writes media: only in a disk table "
	            "(\"!disk\", the oracle's disk profile and the NewTOS "
	            "test card; COVERAGE-BIOS.md)" },
	{ 13, 0x07, "Getbpb reads a boot sector: only in a disk table "
	            "(\"!disk\"; COVERAGE-BIOS.md)" },
	{ 13, 0x09, "Mediach asks a block device: only in a disk table "
	            "(\"!disk\"; COVERAGE-BIOS.md)" },
	{ 14, 0x00, "Initmous installs a mouse packet handler "
	            "(bios/mouse.c:40)" },
	{ 14, 0x01, "Ssbrk reserves memory at the top of RAM on a real TOS" },
	{ 14, 0x06, "Setpalette reprograms the palette at the next VBL "
	            "(bios/screen.c:1168)" },
	{ 14, 0x09, "Flopwr writes a disk" },
	{ 14, 0x0A, "Flopfmt formats a disk" },
	{ 14, 0x0D, "Mfpint installs an interrupt vector (bios/mfp.c:114)" },
	{ 14, 0x16, "Settime sets the machine clock (bios/clock.c:1218)" },
	{ 14, 0x19, "Ikbdws commands the keyboard processor "
	            "(bios/ikbd.c:948)" },
	{ 14, 0x1A, "Jdisint switches an MFP interrupt (bios/mfp.c:122)" },
	{ 14, 0x1B, "Jenabint switches an MFP interrupt (bios/mfp.c:127)" },
	{ 14, 0x1D, "Offgibit writes PSG port A: drive select, strobe "
	            "(bios/sound.c:130)" },
	{ 14, 0x1E, "Ongibit writes PSG port A (bios/sound.c:115)" },
	{ 14, 0x1F, "Xbtimer programs an MFP timer and its vector "
	            "(bios/mfp.c:163)" },
	{ 14, 0x24, "Prtblk prints a block on a real TOS" },
	{ 14, 0x27, "Puntaes discards the AES and reboots a real TOS" },
	{ 14, 0x2B, "DMAwrite writes a disk (bios/disk.c:1254)" },
	{ 14, 0x50, "EsetShift changes the resolution (bios/screen.c:180)" },
	{ 14, 0x54, "EsetPalette reprograms the palette (bios/screen.c:265)" },
	{ 14, 0x5A, "VsetSync changes the video sync (bios/videl.c:675)" },
	{ 14, 0x5D, "VsetRGB reprograms the palette (bios/videl.c:806)" },
	{  0,    0, NULL }
};

static const struct call_policy *call_policy_for(int16_t trap, int16_t fn)
{
	const struct call_policy *p;

	for (p = call_policy; p->sig; p++)
		if (p->trap == trap && fn >= p->fn_lo && fn <= p->fn_hi)
			return p;
	return NULL;
}

static const char *call_refusal_for(int16_t trap, int16_t fn)
{
	const struct call_refusal *r;

	for (r = call_refusals; r->why; r++)
		if (r->trap == trap && r->fn == fn)
			return r->why;
	if (trap == 1)
		return "of GEMDOS a BIOS table issues Super only; the rest waits "
		       "for DUMPGDOS";
	return "the call is not established as safe on both machines "
	       "(tosdump/cases/COVERAGE-BIOS.md)";
}

static int sig_matches(const char *sig, const struct td_call_case *c)
{
	uint16_t i;

	if (!sig || strlen(sig) != c->nargs)
		return 0;
	for (i = 0; i < c->nargs; i++) {
		uint8_t k = c->arg[i].kind;

		switch (sig[i]) {
		case 'w': if (k != TD_CALL_ARG_WORD) return 0; break;
		case 'l': if (k != TD_CALL_ARG_LONG) return 0; break;
		case 'p': if (k != TD_CALL_ARG_LONG && k != TD_CALL_ARG_BUF)
		              return 0;
		          break;
		case 'f': if (k != TD_CALL_ARG_FUNC) return 0; break;
		case 't': if (k != TD_CALL_ARG_TOKEN) return 0; break;
		case 'u': if (k != TD_CALL_ARG_UNIT) return 0; break;
		default:  return 0;
		}
	}
	return 1;
}

static int is_value(const struct td_call_case *c, int i, int32_t v)
{
	return i < (int)c->nargs &&
	       (c->arg[i].kind == TD_CALL_ARG_WORD ||
	        c->arg[i].kind == TD_CALL_ARG_LONG) &&
	       c->arg[i].value == v;
}

static int is_buf(const struct td_call_case *c, int i)
{
	return i < (int)c->nargs && c->arg[i].kind == TD_CALL_ARG_BUF;
}

/*
 * Rwabs(rw, buf, count, recno, dev, lrecno) in a disk table -- the per-line
 * part of the rule; that a write writes back what its last prelude read is
 * checked in td_call_cases_parse, which sees the preludes.
 *   rw     bits RW_WRITE 1, RW_NOMEDIACH 2, RW_NORETRIES 4, RW_NOBYTESWAP
 *          0x80 only.  RW_NOTRANSLATE (8) only with the unit token "u:0" as
 *          dev: a physical unit number is the machine's (EmuTOS IDE master
 *          18, bios/disk.h:30; the Pi's card 26, docs/api/bios/04-rwabs.md
 *          "Decisions"), so a table names it by the token, which the runner
 *          resolves from the build's profile.  Physical is a read into a b:
 *          buffer, count 0..2, flags RW_NOMEDIACH and RW_NORETRIES besides
 *          (RW_NOBYTESWAP reaches the driver in physical mode: no).
 *   buf    a dumper buffer, count 0..2 (two 512-byte sectors fill one); or
 *          NULL, the FastCopy arm, on A:/B: only, as a read with count 0..2:
 *          it returns before any I/O (bios/blkdev.c:425-428).
 *   dev    not A:/B: with a buffer: a floppy without a disk waits.
 */
static const char *rwabs_value_refusal(const struct td_call_case *c)
{
	int32_t rw = c->arg[0].value, cnt = c->arg[2].value, dev = c->arg[4].value;

	if (c->arg[4].kind == TD_CALL_ARG_UNIT) {
		if (!(rw & 8))
			return "Rwabs: the unit token u:0 is a physical unit: only "
			       "with RW_NOTRANSLATE";
		if (rw & ~0x0E)
			return "Rwabs: a physical access is a read, flags "
			       "RW_NOMEDIACH and RW_NORETRIES besides "
			       "RW_NOTRANSLATE";
		if (c->arg[1].kind != TD_CALL_ARG_BUF)
			return "Rwabs: a physical read reads into a b: buffer";
		if (cnt < 0 || cnt * 512 > TD_CALL_BUF_BYTES)
			return "Rwabs: count 0..2, what one dumper buffer holds";
		return NULL;
	}
	if (rw & 8)
		return "Rwabs: RW_NOTRANSLATE only with the unit token u:0 -- "
		       "the unit number is the machine's (BIOS-STOR-002)";
	if (rw & ~0x87)
		return "Rwabs: only the flags RW_WRITE, RW_NOMEDIACH, "
		       "RW_NORETRIES and RW_NOBYTESWAP";
	if (c->arg[1].kind == TD_CALL_ARG_LONG) {
		if (c->arg[1].value != 0)
			return "Rwabs: the buffer is a b: buffer, or NULL for the "
			       "FastCopy arm";
		if ((dev != 0 && dev != 1) || (rw & 1) || cnt < 0 || cnt > 2)
			return "Rwabs: NULL is the FastCopy arm only -- a read on "
			       "A: or B: with count 0..2";
		return NULL;
	}
	if (cnt < 0 || cnt * 512 > TD_CALL_BUF_BYTES)
		return "Rwabs: count 0..2, what one dumper buffer holds";
	if (dev == 0 || dev == 1)
		return "Rwabs: not on the floppies A: and B: (the NULL arm aside)";
	return NULL;
}

/* The value rule of a whitelisted call: NULL when the values pick its safe
 * arm, else the reason. */
static const char *call_value_refusal(const struct call_policy *p,
                                      const struct td_call_case *c)
{
	int32_t a0 = c->nargs > 0 ? c->arg[0].value : 0;
	int32_t a1 = c->nargs > 1 ? c->arg[1].value : 0;
	uint16_t i;

	switch (p->rule) {
	case CR_NONE:
		return NULL;
	case CR_ARG0_M1:
		return is_value(c, 0, -1) ? NULL
		       : "only -1, the inquiry, is established: any other value sets";
	case CR_ARG1_M1:
		return is_value(c, 1, -1) ? NULL
		       : "only -1 in the second argument, the inquiry, is "
		         "established: any other value sets";
	case CR_ALL_M1:
		for (i = 0; i < c->nargs; i++)
			if (!is_value(c, (int)i, -1))
				return "only all -1, the inquiry, is established";
		return NULL;
	case CR_SETSCREEN:
		/* rez -1 returns before videlmode is read (bios/screen.c:1115) */
		return (is_value(c, 0, -1) && is_value(c, 1, -1) &&
		        is_value(c, 2, -1)) ? NULL
		       : "only Setscreen(-1, -1, -1), which changes nothing, is "
		         "established";
	case CR_RSCONF:
		for (i = 1; i < c->nargs; i++)
			if (!is_value(c, (int)i, -1))
				return "Rsconf: only all -1, or -2 then all -1, inquire";
		return (a0 == -1 || a0 == -2) ? NULL
		       : "Rsconf: a baud rate reprograms the serial port";
	case CR_BCONIN:
		/* 4, 5: char_dummy (bios/chardev.c:34-36); < 0 and >= 10: no
		 * device on either machine.  0..3 wait for input, 6..9 are
		 * Bconmap devices (bios/bios.c:966-972). */
		return (a0 == 4 || a0 == 5 || a0 < 0 || a0 >= 10) ? NULL
		       : "Bconin on this handle waits for input";
	case CR_BCONOUT:
		if (a0 < 0 || a0 >= 10)
			return NULL;
		if ((a0 == 2 || a0 == 5) && a1 >= 0x20 && a1 <= 0x7E)
			return NULL;
		return "Bconout: only a printable byte to CON or raw CON is "
		       "established (PRT waits 30 s, IKBD and MIDI are commands, "
		       "a control byte leaves VT52 state)";
	case CR_CURSCONF:
		return a0 == 5 ? NULL
		       : "Cursconf: only function 5, the blink-rate inquiry";
	case CR_GIACCESS:
		return (a1 >= 0 && a1 <= 15) ? NULL
		       : "Giaccess: only a register read (no write bit 0x80)";
	case CR_FLOPRATE:
		return (a1 == -1 || a1 == -4) ? NULL
		       : "Floprate: only rate -1 or -4, the two inquiries";
	case CR_BCONMAP:
		return (a0 < 6 || a0 >= 10) ? NULL
		       : "Bconmap: a device number 6..9 remaps AUX";
	case CR_NVM:
		if (a0 == 1 || a0 == 2)
			return "NVMaccess: write and reset change the NVRAM";
		if (a0 == 0 && is_buf(c, 3) &&
		    (c->arg[2].value < 0 || c->arg[2].value > TD_CALL_BUF_BYTES))
			return "NVMaccess: the count does not fit the buffer";
		if (a0 == 0 && !is_buf(c, 3) && !is_value(c, 3, 0))
			return "NVMaccess: a read needs a dumper buffer (or NULL)";
		return NULL;
	case CR_BUF0:
		return is_buf(c, 0) ? NULL
		       : "the call writes through its first argument: name a buffer";
	case CR_BUF01:
		return (is_buf(c, 0) && is_buf(c, 1)) ? NULL
		       : "the call writes through both arguments: name two buffers";
	case CR_EGETPAL:
		return (is_buf(c, 2) && a1 >= 0 && a1 * 2 <= TD_CALL_BUF_BYTES)
		       ? NULL
		       : "EgetPalette: name a buffer that holds count WORDs";
	case CR_VGETRGB:
		return (is_buf(c, 2) && a1 >= 0 && a1 * 4 <= TD_CALL_BUF_BYTES)
		       ? NULL
		       : "VgetRGB: name a buffer that holds count LONGs";
	case CR_SOUNDCMD:
		return ((a0 == 0 || a0 == 1 || a0 == 6) && is_value(c, 1, -1))
		       ? NULL
		       : "Soundcmd: only modes 0, 1, 6 with -1, the inquiries";
	case CR_SNDSTATUS:
		return a0 == 0 ? NULL : "Sndstatus: a non-zero argument resets";
	case CR_DISK_DRIVE:
		/* a floppy without a disk: Getbpb ends in the critical-error
		 * handler on a real TOS (bios/blkdev.c:623-629), and the disk
		 * profile has no floppy drives (docs/analysis/ORACLE.md) */
		return (a0 == 0 || a0 == 1)
		       ? "Getbpb/Mediach: not on the floppies A: and B:" : NULL;
	case CR_RWABS:
		return rwabs_value_refusal(c);
	case CR_GETMPB:
		/* it writes mp_mfl and mp_mal through its argument
		 * (bios/biosmem.c:149-150): a dumper buffer only */
		return is_buf(c, 0) ? NULL
		       : "Getmpb writes through its argument: name a b: buffer";
	case CR_SUPER:
		/* the bracket itself (open / close) is the parse loop's */
		if (c->arg[0].kind == TD_CALL_ARG_TOKEN || is_value(c, 0, 1) ||
		    is_value(c, 0, 0))
			return NULL;
		return "Super: only 1L, 0L and the token t:0 -- any other value is "
		       "a stack address EmuTOS would run on (bdos/rwa.S:280-301)";
	default:
		return "unknown rule";
	}
}

/* "!disk <TAB> <drive> [<TAB> destructive]" (docs/TOS-REFERENCE-DUMPER.md
 * section 6, "The disk round"): the table is a disk table, <drive> its test
 * drive, 2..15; "destructive" marks a table that changes the test image and
 * runs on a fresh clone only (the closing tables). */
static int parse_call_directive(const char *line, size_t len,
                                struct td_call_case_table *t, int late,
                                const char **why)
{
	const char *col[3];
	size_t col_len[3];
	const char *name, *val, *mark;
	size_t name_len, val_len, mark_len;
	uint32_t drive;
	int ncol = split_columns(line, len, col, col_len, 3);

	if (ncol < 2) {
		col[1] = line + len;
		col_len[1] = 0;
	}
	if (ncol < 3) {
		col[2] = line + len;
		col_len[2] = 0;
	}
	name = col[0];
	name_len = col_len[0];
	trim(&name, &name_len);
	if (span_is(name, name_len, "!layout")) {
		/* TOOL16: a structure the captures name (cases.h, "Layouts") */
		if (late) {
			*why = "\"!layout\" stands before the first case and prelude";
			return TD_CASE_ERR_DIRECTIVE;
		}
		return td_layout_parse(t->layouts, &t->nlayouts, line, len, why);
	}
	if (span_is(name, name_len, "!write")) {
		/* TOOL16: the table writes through returned pointers ("+put") */
		val = col[1];
		val_len = col_len[1];
		trim(&val, &val_len);
		if (t->writes) {
			*why = "\"!write\" stands once";
			return TD_CASE_ERR_DIRECTIVE;
		}
		if (late) {
			*why = "\"!write\" stands before the first case and prelude";
			return TD_CASE_ERR_DIRECTIVE;
		}
		if (!span_is(val, val_len, "return") || col_len[2] != 0) {
			*why = "\"!write\" takes one word, \"return\": the table "
			       "writes through returned pointers (+put)";
			return TD_CASE_ERR_DIRECTIVE;
		}
		t->writes = 1;
		return 0;
	}
	if (!span_is(name, name_len, "!disk")) {
		*why = "a call table knows the directives \"!disk <drive> "
		       "[destructive]\", \"!layout\" and \"!write return\"";
		return TD_CASE_ERR_DIRECTIVE;
	}
	if (t->disk) {
		*why = "\"!disk\" stands once";
		return TD_CASE_ERR_DIRECTIVE;
	}
	if (late) {
		*why = "\"!disk\" stands before the first case and prelude";
		return TD_CASE_ERR_DIRECTIVE;
	}
	val = col[1];
	val_len = col_len[1];
	trim(&val, &val_len);
	if (val_len == 0 || parse_small(val, val_len, 15, &drive) != 0 ||
	    drive < 2) {
		*why = "\"!disk\" names its test drive, 2 (C:) .. 15 (P:)";
		return TD_CASE_ERR_DIRECTIVE;
	}
	mark = col[2];
	mark_len = col_len[2];
	trim(&mark, &mark_len);
	if (mark_len != 0 && !span_is(mark, mark_len, "destructive")) {
		*why = "after the drive \"!disk\" takes one word, "
		       "\"destructive\"";
		return TD_CASE_ERR_DIRECTIVE;
	}
	t->disk = 1;
	t->disk_drive = (uint8_t)drive;
	t->destructive = mark_len != 0;
	return 0;
}

static int same_value(const struct td_call_arg *a, const struct td_call_arg *b)
{
	return a->kind == b->kind && a->value == b->value;
}

/*
 * A write in a disk table writes back what the LAST prelude before it read:
 * same buffer, count, recno, drive and lrecno, the prelude a logical read
 * (the per-line rules hold for it already).  In a destructive table the last
 * prelude may be a "+set": the read is then the one before it, and what is
 * written back is "read + set".  The runner stops the run when
 * that read does not return 0 (runner_call.c), so the write never carries
 * the poison.  The drive is the table's test drive.
 */
static const char *write_back_refusal(const struct td_call_case_table *t,
                                      const struct td_call_case *c,
                                      uint16_t pending)
{
	const struct td_call_case *p;
	int i;

	if (c->arg[4].value != (int32_t)t->disk_drive)
		return "Rwabs: a write only on the test drive \"!disk\" names";
	if (c->arg[1].kind != TD_CALL_ARG_BUF)
		return "Rwabs: a write writes a dumper buffer";
	if (pending == 0)
		return "Rwabs: a write needs a prelude that read the same "
		       "sectors into the same buffer just before it";
	p = &t->preludes[t->prelude_count - 1];
	if (p->trap == TD_CALL_TRAP_SET)
		p--;            /* parse_set_prelude: a read stands before it */
	for (i = 1; i < 6; i++)
		if (!same_value(&p->arg[i], &c->arg[i]))
			return "Rwabs: a write writes back exactly what its last "
			       "prelude read -- same buffer, count, sector, drive";
	return NULL;
}

/* Is this prelude line a "+set"?  (The opcode column, '+' stripped.) */
static int is_set_line(const char *line, size_t len)
{
	const char *col[2];
	size_t col_len[2];
	const char *o;
	size_t olen;

	split_columns(line, len, col, col_len, 2);
	o = col[0];
	olen = col_len[0];
	trim(&o, &olen);
	return olen >= 1 && o[0] == '+' && span_is(o + 1, olen - 1, "set");
}

/*
 * "+set <TAB> <offset>=<byte>[,<offset>=<byte>...] [<TAB> comment]" -- only
 * in a destructive disk table, IMMEDIATELY after the read prelude whose
 * buffer it changes (docs/TOS-REFERENCE-DUMPER.md section 6, "The closing
 * tables").  At most TD_CALL_SET_PAIRS pairs; an offset inside what the read
 * read (count * 512 bytes), no offset twice, a byte 0..255.  That the case
 * after it is the write-back of "read + set" is checked by the parse loop.
 */
static int parse_set_prelude(const struct td_call_case_table *t,
                             uint16_t pending, const char *line, size_t len,
                             struct td_call_case *c, const char **why)
{
	const char *col[3];
	size_t col_len[3];
	const struct td_call_case *read;
	const char *s;
	size_t slen, i, start;
	int32_t limit;

	if (!t->disk || !t->destructive) {
		*why = "+set changes the test image: only in a destructive disk "
		       "table (\"!disk <drive> destructive\")";
		return TD_CASE_ERR_UNSAFE_STATE;
	}
	if (pending == 0 ||
	    t->preludes[t->prelude_count - 1].trap == TD_CALL_TRAP_SET) {
		*why = "+set follows the read prelude whose buffer it changes";
		return TD_CASE_ERR_UNSAFE_FIELD;
	}
	read = &t->preludes[t->prelude_count - 1];
	limit = read->arg[2].value * 512;
	if (limit <= 0) {
		*why = "+set after a read of count 0: nothing was read";
		return TD_CASE_ERR_UNSAFE_FIELD;
	}
	if (split_columns(line, len, col, col_len, 3) < 2) {
		*why = "+set needs its <offset>=<byte> list in the second column";
		return TD_CASE_ERR_COLUMNS;
	}
	s = col[1];
	slen = col_len[1];
	trim(&s, &slen);
	c->trap = TD_CALL_TRAP_SET;
	c->opcode = 0;
	c->nargs = 0;
	c->cap_arg = -1;
	for (i = 0, start = 0; ; i++) {
		if (i == slen || s[i] == ',') {
			const char *a = s + start;
			size_t alen = i - start, eq, k;
			int32_t off, byte;

			trim(&a, &alen);
			for (eq = 0; eq < alen && a[eq] != '='; eq++)
				;
			if (alen == 0 || eq == 0 || eq >= alen - 1 ||
			    parse_value32(a, eq, &off) != 0 ||
			    parse_value32(a + eq + 1, alen - eq - 1, &byte) != 0) {
				*why = "+set takes <offset>=<byte>[,<offset>=<byte>...], "
				       "decimal or 0x hex";
				return TD_CASE_ERR_VALUE;
			}
			if (byte < 0 || byte > 255) {
				*why = "+set: a byte is 0..255";
				return TD_CASE_ERR_RANGE;
			}
			if (off < 0 || off >= limit) {
				*why = "+set: an offset outside what the read before it "
				       "read (count * 512 bytes)";
				return TD_CASE_ERR_RANGE;
			}
			if (c->nargs >= TD_CALL_SET_PAIRS) {
				*why = "+set: more than eight pairs";
				return TD_CASE_ERR_TOO_MANY_VALUES;
			}
			for (k = 0; k < c->nargs; k++)
				if ((c->arg[k].value >> 8) == off) {
					*why = "+set: an offset named twice";
					return TD_CASE_ERR_VALUE;
				}
			c->arg[c->nargs].kind = TD_CALL_ARG_SET;
			c->arg[c->nargs].value = (off << 8) | byte;
			c->nargs++;
			if (i == slen)
				break;
			start = i + 1;
		}
	}
	td_case_line_comment_n(line, len, 3, c->comment, TD_COMMENT_LEN);
	return 0;
}

/* Is this prelude line a "+put"?  (TOOL16) */
static int is_put_line(const char *line, size_t len)
{
	const char *col[2];
	size_t col_len[2];
	const char *o;
	size_t olen;

	split_columns(line, len, col, col_len, 2);
	o = col[0];
	olen = col_len[0];
	trim(&o, &olen);
	return olen >= 1 && o[0] == '+' && span_is(o + 1, olen - 1, "put");
}

/* A top-level, non-pointer field of layout `l` named by s[0..len). */
static int put_field(const struct td_layout *l, const char *s, size_t len)
{
	char name[TD_LAYOUT_PATH_LEN];
	int k;

	trim(&s, &len);
	if (len == 0 || len >= sizeof name)
		return -1;
	memcpy(name, s, len);
	name[len] = '\0';
	k = td_layout_slot_find(l, name);
	if (k < 0 || l->slot[k].hops != 0 || l->slot[k].type == TD_LT_PTR)
		return -1;
	return k;
}

/*
 * "+put <TAB> <field>=<field> [<TAB> comment]" -- only in a table that says
 * "!write return", IMMEDIATELY after the case whose "r:<layout>" capture it
 * writes through (docs/TOS-REFERENCE-DUMPER.md section 6, "Structures behind
 * a pointer").  One field of that structure gets the value of another of the
 * same type; both are top-level, non-pointer fields (cases.h, "Layouts").
 * That a case follows it, and no other prelude, is the parse loop's.
 */
static int parse_put_prelude(const struct td_call_case_table *t,
                             uint16_t pending, const char *line, size_t len,
                             struct td_call_case *c, const char **why)
{
	const char *col[3];
	size_t col_len[3];
	const struct td_call_case *prev;
	const struct td_layout *l;
	const char *s;
	size_t slen, eq;
	int dst, src;

	if (!t->writes) {
		*why = "+put writes through a returned pointer: only in a table "
		       "that says \"!write return\"";
		return TD_CASE_ERR_UNSAFE_STATE;
	}
	if (pending != 0 || t->count == 0 ||
	    t->cases[t->count - 1].cap_arg != TD_CALL_CAP_RET ||
	    t->cases[t->count - 1].cap_layout == 0) {
		*why = "+put stands right after the case whose returned pointer it "
		       "writes through, a case with an r:<layout> capture";
		return TD_CASE_ERR_UNSAFE_FIELD;
	}
	prev = &t->cases[t->count - 1];
	l = &t->layouts[prev->cap_layout - 1];
	if (split_columns(line, len, col, col_len, 3) < 2) {
		*why = "+put needs its <field>=<field> in the second column";
		return TD_CASE_ERR_COLUMNS;
	}
	s = col[1];
	slen = col_len[1];
	trim(&s, &slen);
	for (eq = 0; eq < slen && s[eq] != '='; eq++)
		;
	if (eq == 0 || eq >= slen - 1) {
		*why = "+put takes <field>=<field>: one field of the structure "
		       "gets the value of another";
		return TD_CASE_ERR_VALUE;
	}
	dst = put_field(l, s, eq);
	src = put_field(l, s + eq + 1, slen - eq - 1);
	if (dst < 0 || src < 0) {
		*why = "+put names a field that is not a top-level, non-pointer "
		       "field of the case's layout";
		return TD_CASE_ERR_UNSAFE_FIELD;
	}
	if (dst == src || l->slot[dst].type != l->slot[src].type) {
		*why = "+put copies between two different fields of the same type";
		return TD_CASE_ERR_UNSAFE_FIELD;
	}
	c->trap       = TD_CALL_TRAP_PUT;
	c->opcode     = 0;
	c->nargs      = 2;
	c->arg[0].kind  = TD_CALL_ARG_PUT;
	c->arg[0].value = dst;
	c->arg[1].kind  = TD_CALL_ARG_PUT;
	c->arg[1].value = src;
	c->cap_arg    = -1;
	c->cap_layout = prev->cap_layout;
	td_case_line_comment_n(line, len, 3, c->comment, TD_COMMENT_LEN);
	return 0;
}

int td_call_cases_parse(struct td_call_case_table *t, uint16_t area,
                        const char *buf, size_t len,
                        struct td_case_error *err)
{
	struct td_line_iter it;
	const char *line;
	size_t line_len;
	uint16_t line_no;
	int is_case;
	uint16_t bracket_open = 0;         /* line of an open Super(0L), or 0 */
	uint16_t pending = 0;              /* preludes since the last case    */
	uint16_t last_prelude = 0;         /* line of the newest one          */
	uint16_t getmpb_line = 0;          /* a Getmpb case stood here (TOOL16) */

	memset(t, 0, sizeof *t);
	t->crc  = td_crc32((const uint8_t *)buf, len);
	t->area = area;
	if (err) {
		err->line = 0;
		err->what = NULL;
	}
	if (area != TD_AREA_BIOS) {
		fail(err, 0, "no call guard exists for this area yet");
		return TD_CASE_ERR_UNSAFE_OPCODE;
	}

	td_line_iter_init(&it, buf, len);
	while (td_line_iter_next(&it, &line, &line_len, &line_no, &is_case)) {
		const char *col[TD_CALL_CASE_COLUMNS];
		size_t col_len[TD_CALL_CASE_COLUMNS];
		const struct call_policy *p;
		struct td_call_case *c;
		const char *why = NULL, *o;
		size_t olen, colon;
		uint32_t trap, fn;
		int rc, prelude;
		uint16_t i;

		if (!is_case) {
			if (td_case_line_is_directive(line, line_len)) {
				rc = parse_call_directive(line, line_len, t,
				                          t->count > 0 || pending > 0,
				                          &why);
				if (rc != 0) {
					fail(err, line_no, why);
					return rc;
				}
			}
			continue;
		}
		prelude = td_case_line_is_prelude(line, line_len);
		if (prelude && pending &&
		    t->preludes[t->prelude_count - 1].trap == TD_CALL_TRAP_PUT) {
			fail(err, line_no, "after a +put comes the case it is for, "
			                   "no other prelude");
			return TD_CASE_ERR_UNSAFE_FIELD;
		}
		if (prelude && is_put_line(line, line_len)) {
			if (t->prelude_count >= TD_MAX_CALL_PRELUDES) {
				fail(err, line_no, "more preludes than "
				                   "TD_MAX_CALL_PRELUDES");
				return TD_CASE_ERR_TOO_MANY_PRE;
			}
			c = &t->preludes[t->prelude_count];
			memset(c, 0, sizeof *c);
			c->line = line_no;
			rc = parse_put_prelude(t, pending, line, line_len, c, &why);
			if (rc != 0) {
				fail(err, line_no, why);
				return rc;
			}
			t->prelude_count++;
			pending++;
			last_prelude = line_no;
			continue;
		}
		if (prelude && is_set_line(line, line_len)) {
			if (t->prelude_count >= TD_MAX_CALL_PRELUDES) {
				fail(err, line_no, "more preludes than "
				                   "TD_MAX_CALL_PRELUDES");
				return TD_CASE_ERR_TOO_MANY_PRE;
			}
			c = &t->preludes[t->prelude_count];
			memset(c, 0, sizeof *c);
			c->line = line_no;
			rc = parse_set_prelude(t, pending, line, line_len, c, &why);
			if (rc != 0) {
				fail(err, line_no, why);
				return rc;
			}
			t->prelude_count++;
			pending++;
			last_prelude = line_no;
			continue;
		}
		if (prelude && !t->disk) {
			fail(err, line_no, "a call table has no preludes: round 1 "
			                   "issues queries only (a disk table, "
			                   "\"!disk\", has Rwabs reads; a \"!write "
			                   "return\" table +put)");
			return TD_CASE_ERR_DIRECTIVE;
		}
		if (split_columns(line, line_len, col, col_len,
		                  TD_CALL_CASE_COLUMNS) < TD_CALL_CASE_COLUMNS) {
			fail(err, line_no, "needs five tab separated columns");
			return TD_CASE_ERR_COLUMNS;
		}
		if (prelude) {
			if (t->prelude_count >= TD_MAX_CALL_PRELUDES) {
				fail(err, line_no, "more preludes than "
				                   "TD_MAX_CALL_PRELUDES");
				return TD_CASE_ERR_TOO_MANY_PRE;
			}
			c = &t->preludes[t->prelude_count];
		} else {
			if (t->count >= TD_MAX_CASES) {
				fail(err, line_no, "more cases than TD_MAX_CASES");
				return TD_CASE_ERR_TOO_MANY_CASES;
			}
			c = &t->cases[t->count];
		}
		memset(c, 0, sizeof *c);
		c->line = line_no;

		/* trap:function */
		o = col[0];
		olen = col_len[0];
		trim(&o, &olen);
		if (prelude) {                 /* step over the '+' */
			o++;
			olen--;
		}
		for (colon = 0; colon < olen && o[colon] != ':'; colon++)
			;
		if (colon == 0 || colon >= olen) {
			fail(err, line_no, "the call column is not <trap>:<function>");
			return TD_CASE_ERR_VALUE;
		}
		rc = parse_small(o, colon, 255, &trap);
		if (rc != 0) {
			fail(err, line_no, "the trap is not a number");
			return rc;
		}
		rc = parse_small(o + colon + 1, olen - colon - 1, 255, &fn);
		if (rc != 0) {
			fail(err, line_no, "the function number is not 0..255");
			return rc;
		}
		if (trap != 13 && trap != 14 && trap != 1) {
			fail(err, line_no, "a BIOS table issues trap 13 and trap 14, "
			                   "and trap 1 for Super only");
			return TD_CASE_ERR_UNSAFE_OPCODE;
		}
		c->trap   = (int16_t)trap;
		c->opcode = (int16_t)fn;

		rc = parse_call_args(col[1], col_len[1], c, &why);
		if (rc == 0 && prelude) {
			/* a prelude is not recorded: no class, no capture */
			const char *r = col[2], *k = col[3];
			size_t rl = col_len[2], kl = col_len[3];

			trim(&r, &rl);
			trim(&k, &kl);
			c->cap_arg = -1;
			if (!span_is(r, rl, "-")) {
				why = "a prelude is not recorded: its retclass is -";
				rc = TD_CASE_ERR_RETCLASS;
			} else if (!span_is(k, kl, "-")) {
				why = "a prelude is not recorded: its capture is -";
				rc = TD_CASE_ERR_CAPTURE;
			}
		} else if (rc == 0) {
			rc = parse_retclass(col[2], col_len[2], c, &why);
			if (rc == 0)
				rc = parse_capture(t, col[3], col_len[3], c, &why);
		}
		if (rc != 0) {
			fail(err, line_no, why);
			return rc;
		}

		/* the guard */
		p = call_policy_for(c->trap, c->opcode);
		if (p && (p->rule == CR_DISK_DRIVE || p->rule == CR_RWABS) &&
		    !t->disk)
			p = NULL;              /* refused outside a disk table */
		if (!p) {
			fail(err, line_no, call_refusal_for(c->trap, c->opcode));
			return TD_CASE_ERR_UNSAFE_OPCODE;
		}
		if (!sig_matches(p->sig, c) && !sig_matches(p->sig_alt, c)) {
			fail(err, line_no, "the argument widths are not the "
			                   "binding's");
			return TD_CASE_ERR_SIGNATURE;
		}
		why = call_value_refusal(p, c);
		if (!why && prelude &&
		    (p->rule != CR_RWABS || (c->arg[0].value & 9) ||
		     c->arg[1].kind != TD_CALL_ARG_BUF))
			why = "a prelude is a logical Rwabs read into a dumper "
			      "buffer, nothing else (a write is a measured case)";
		if (!why && prelude && pending &&
		    t->preludes[t->prelude_count - 1].trap == TD_CALL_TRAP_SET)
			why = "after a +set comes the write that writes its "
			      "buffer back, no other prelude";
		if (!why && !prelude && pending &&
		    t->preludes[t->prelude_count - 1].trap == TD_CALL_TRAP_SET &&
		    (p->rule != CR_RWABS || !(c->arg[0].value & 1)))
			why = "after a +set comes the write that writes its "
			      "buffer back";
		if (!why && !prelude && p->rule == CR_RWABS &&
		    (c->arg[0].value & 1))
			why = write_back_refusal(t, c, pending);
		if (why) {
			fail(err, line_no, why);
			return TD_CASE_ERR_UNSAFE_FIELD;
		}

		/* Getmpb (TOOL16): on EmuTOS it rewrites themd, the descriptor
		 * GEMDOS built its pool on (bios/biosmem.c:143-150), and the MD
		 * lies in protected low memory ($48E, a user-mode read is a bus
		 * error).  As the probe that established it measured it (BCPROB2,
		 * measurements/measurement-2026-10-28-bios-core-audit): inside a
		 * Super bracket, and last -- after it only another Getmpb and the
		 * Super(t:0) that closes the bracket. */
		if (p->rule == CR_GETMPB && !bracket_open) {
			fail(err, line_no, "Getmpb only inside a Super(0L) ... "
			                   "Super(t:0) bracket, as the last call: it "
			                   "rewrites the descriptor GEMDOS built its "
			                   "pool on, in protected memory "
			                   "(bios/biosmem.c:143-150)");
			return TD_CASE_ERR_UNSAFE_OPCODE;
		}
		if (getmpb_line && p->rule != CR_GETMPB &&
		    !(p->rule == CR_SUPER && c->arg[0].kind == TD_CALL_ARG_TOKEN)) {
			fail(err, line_no, "after Getmpb only Getmpb and the Super(t:0) "
			                   "that closes the bracket: Getmpb rewrites "
			                   "the descriptor GEMDOS's pool uses");
			return TD_CASE_ERR_UNSAFE_STATE;
		}
		if (p->rule == CR_GETMPB)
			getmpb_line = line_no;

		/* the Super bracket: l:0 opens, t:0 closes, no nesting */
		if (p->rule == CR_SUPER && is_value(c, 0, 0)) {
			if (bracket_open) {
				fail(err, line_no, "Super(0L) while a bracket is open: "
				                   "brackets do not nest");
				return TD_CASE_ERR_UNSAFE_STATE;
			}
			bracket_open = line_no;
		} else if (p->rule == CR_SUPER &&
		           c->arg[0].kind == TD_CALL_ARG_TOKEN) {
			if (!bracket_open) {
				fail(err, line_no, "Super(token) with no Super(0L) "
				                   "before it");
				return TD_CASE_ERR_UNSAFE_STATE;
			}
			bracket_open = 0;
		}

		for (i = 0; i < c->nargs; i++)
			if (c->arg[i].kind == TD_CALL_ARG_UNIT)
				t->unit_token = 1;
		td_case_line_comment_n(line, line_len, TD_CALL_CASE_COLUMNS,
		                       c->comment, TD_COMMENT_LEN);
		if (prelude) {
			t->prelude_count++;
			pending++;
			last_prelude = line_no;
			continue;
		}
		c->prelude_first = (uint16_t)(t->prelude_count - pending);
		c->prelude_count = pending;
		pending = 0;
		t->count++;
	}
	if (pending) {
		fail(err, last_prelude, "a prelude with no case after it");
		return TD_CASE_ERR_DANGLING;
	}
	if (bracket_open) {
		fail(err, bracket_open, "Super(0L) is never closed: the run would "
		                        "end in supervisor state");
		return TD_CASE_ERR_UNSAFE_STATE;
	}
	return 0;
}

int td_call_cases_load(struct td_call_case_table *t, uint16_t area,
                       const char *path, struct td_case_error *err)
{
	char *raw;
	size_t got;
	int rc;

	rc = slurp(path, &raw, &got, err);
	if (rc != 0)
		return rc;

	rc = td_call_cases_parse(t, area, raw, got, err);
	free(raw);
	return rc;
}

const char *td_case_error_text(int rc)
{
	switch (rc) {
	case 0:                          return "ok";
	case TD_CASE_ERR_COLUMNS:        return "fewer than five columns";
	case TD_CASE_ERR_VALUE:          return "a column is not a number";
	case TD_CASE_ERR_RANGE:          return "a value does not fit in 16 bits";
	case TD_CASE_ERR_ODD_PTSIN:      return "ptsin is not a list of points";
	case TD_CASE_ERR_TOO_MANY_VALUES:return "more values than the array holds";
	case TD_CASE_ERR_TOO_MANY_CASES: return "more cases than TD_MAX_CASES";
	case TD_CASE_ERR_IO:             return "the case table could not be read";
	case TD_CASE_ERR_DANGLING:       return "a prelude has no case after it";
	case TD_CASE_ERR_TOO_MANY_PRE:   return "more preludes than TD_MAX_PRELUDES";
	case TD_CASE_ERR_OUT_LEN:        return "n_intout is larger than the "
	                                        "intout array";
	case TD_CASE_ERR_UNSAFE_OPCODE:  return "the table names an opcode that "
	                                        "may not be issued";
	case TD_CASE_ERR_GEOMETRY:       return "a form or a rectangle would "
	                                        "leave the dumper's buffer";
	case TD_CASE_ERR_DIRECTIVE:      return "a '!' directive line cannot be "
	                                        "honoured";
	case TD_CASE_ERR_UNSAFE_FIELD:   return "the table names a field that "
	                                        "may not be issued";
	case TD_CASE_ERR_UNSAFE_STATE:   return "the table would leave a state "
	                                        "the run cannot come back from";
	case TD_CASE_ERR_PLANT:          return "the addr_in column names a "
	                                        "buffer the call cannot be given";
	case TD_CASE_ERR_AES_VERSION:    return "this AES does not report a "
	                                        "version that establishes an "
	                                        "opcode the table needs";
	case TD_CASE_ERR_RETCLASS:       return "the retclass column is not a "
	                                        "class (and width) the "
	                                        "comparator knows";
	case TD_CASE_ERR_CAPTURE:        return "the capture column does not "
	                                        "name a buffer range the record "
	                                        "can hold";
	case TD_CASE_ERR_SIGNATURE:      return "the argument widths are not "
	                                        "the binding's";
	default:                         return "unknown error";
	}
}
