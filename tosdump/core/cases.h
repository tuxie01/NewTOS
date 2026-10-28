/*
 * cases.h -- case table loader (spec section 6).
 *
 * Format, for cases/vdi.tsv and cases/aes.tsv:
 *
 *   opcode <TAB> sub <TAB> intin values <TAB> ptsin values <TAB> comment
 *
 * Values are comma separated, decimal or 0x-hex, and "-" means none.  A line
 * whose first non-blank character is '#' is a comment, and so is an empty
 * line; both still consume a line number, because **a case id is the physical
 * 1-based line number** and nothing may silently renumber the cases.  The
 * comment column is everything from the fifth column on, tabs included.
 *
 * A line whose opcode field starts with '+' is a PRELUDE, not a case: a call
 * issued after the reset and before the next case's measured call, and not
 * recorded. That is what makes an effect measurable rather than only an echo
 * -- a setter's echo proves the echo, and only a read-back after a
 * non-default set proves the effect. Preludes stack; several may precede one
 * case. A prelude consumes a line number like any other line, and a prelude
 * with no case after it is an error.
 *
 * A malformed line is an error, never a skipped case: a table that half-loads
 * would produce a dump that compares against the wrong cases.
 *
 * The table is loaded at runtime from the same directory as the executable and
 * is never compiled in -- the table must be provably identical for both runs,
 * and recompiling between runs makes that unprovable.
 */
#ifndef TOSDUMP_CASES_H
#define TOSDUMP_CASES_H

#include "record.h"

/*
 * Tab separated columns of one case line, per table form.  The comment is
 * everything from the last column on, tabs included, so the count is what
 * tells a reader where the comment starts -- and it cannot be derived from
 * a line, because a comment may itself hold tabs.
 */
#define TD_CASE_COLUMNS      5   /* VDI form, and the BIOS/GEMDOS form   */

/* Values a prelude may carry.  Small on purpose: a prelude sets one piece of
 * state, and the target holds the whole table in static memory. */
#define TD_PRELUDE_VALUES  16
#define TD_MAX_PRELUDES   256

struct td_prelude {
	int16_t  opcode;
	int16_t  sub;
	uint16_t n_intin;
	uint16_t n_ptsin;
	int16_t  intin[TD_PRELUDE_VALUES];
	int16_t  ptsin[TD_PRELUDE_VALUES];
};

struct td_case {
	uint16_t line;                     /* physical line number = case id  */
	uint16_t prelude_first;            /* index into the table's pool     */
	uint16_t prelude_count;            /* calls issued before this case   */
	int16_t  opcode;                   /* contrl[0]                       */
	int16_t  sub;                      /* contrl[5]                       */
	uint16_t n_intin;                  /* values, becomes contrl[3]       */
	uint16_t n_ptsin;                  /* values; contrl[1] = n_ptsin / 2 */
	int16_t  intin[TD_VDI_INTIN_N];
	int16_t  ptsin[TD_VDI_PTSIN_N];
	char     comment[TD_COMMENT_LEN];
};

struct td_case_table {
	uint32_t          crc;             /* CRC32 over the raw file bytes   */
	uint16_t          count;
	uint16_t          prelude_count;
	struct td_case    cases[TD_MAX_CASES];
	struct td_prelude preludes[TD_MAX_PRELUDES];
};

#define TD_CASE_ERR_COLUMNS         (-1)   /* fewer than five columns      */
#define TD_CASE_ERR_VALUE           (-2)   /* a column is not a number     */
#define TD_CASE_ERR_RANGE           (-3)   /* a value does not fit         */
#define TD_CASE_ERR_ODD_PTSIN       (-4)   /* ptsin is not a point list    */
#define TD_CASE_ERR_TOO_MANY_VALUES (-5)   /* more values than the array   */
#define TD_CASE_ERR_TOO_MANY_CASES  (-6)   /* more than TD_MAX_CASES       */
#define TD_CASE_ERR_IO              (-7)   /* file could not be read       */
#define TD_CASE_ERR_DANGLING        (-8)   /* prelude with no case after   */
#define TD_CASE_ERR_TOO_MANY_PRE    (-9)   /* more than TD_MAX_PRELUDES    */
#define TD_CASE_ERR_OUT_LEN        (-10)  /* n_intout > the intout array  */
#define TD_CASE_ERR_UNSAFE_OPCODE  (-11)  /* opcode may not be issued     */
#define TD_CASE_ERR_GEOMETRY       (-12)  /* a form or rect leaves the buf*/
#define TD_CASE_ERR_DIRECTIVE      (-13)  /* a '!' line cannot be honoured*/
#define TD_CASE_ERR_UNSAFE_FIELD   (-14)  /* a field may not be issued    */
#define TD_CASE_ERR_UNSAFE_STATE   (-15)  /* the run could not finish     */
#define TD_CASE_ERR_PLANT          (-16)  /* the addr_in column is wrong  */
#define TD_CASE_ERR_AES_VERSION    (-17)  /* this AES does not have it    */
#define TD_CASE_ERR_RETCLASS       (-18)  /* retclass column is wrong     */
#define TD_CASE_ERR_CAPTURE        (-19)  /* capture column is wrong      */
#define TD_CASE_ERR_SIGNATURE      (-20)  /* argument widths != binding   */

struct td_case_error {
	uint16_t    line;                  /* physical line, 0 if not a line  */
	const char *what;                  /* static description              */
};

int td_cases_parse(struct td_case_table *t, const char *buf, size_t len,
                   struct td_case_error *err);
int td_cases_load(struct td_case_table *t, const char *path,
                  struct td_case_error *err);

const char *td_case_error_text(int rc);

/* Copies the comment column -- everything from column five on, tabs included
 * -- of one case line.  Shared so the runner and the host tools name a case
 * the same way.  This is the FIVE column (VDI) rule; see
 * td_case_line_comment_n for tables with a different column count. */
void td_case_line_comment(const char *line, size_t len, char *dst,
                          size_t dst_size);

/*
 * The same rule for a table with `ncols` columns: the comment is everything
 * from column `ncols` on, tabs included.  The column count is per area (five
 * for the VDI form, seven for the AES form), which is why it cannot be
 * derived from the line: a VDI comment may itself hold tabs, so counting the
 * tabs on a line does not tell you which form it is.
 */
void td_case_line_comment_n(const char *line, size_t len, int ncols,
                            char *dst, size_t dst_size);

/* Is this line a prelude rather than a case?  Shared so every reader agrees
 * on what carries a case id -- a comparator that counted preludes as cases
 * would index every record wrongly. */
int td_case_line_is_prelude(const char *line, size_t len);

/* Is this line a '!' directive?  A directive carries no call at all, so the
 * shared line iterator already reports it as not-a-case; this is how the
 * parser that owns the directive picks it back out. */
int td_case_line_is_directive(const char *line, size_t len);

/* ------------------------------------------------------------------ */
/* AES case tables (cases/aes-*.tsv)                                   */
/* ------------------------------------------------------------------ */
/*
 * Format, seven tab separated columns:
 *
 *   opcode <TAB> n_intin <TAB> n_intout <TAB> n_addrin <TAB> n_addrout
 *          <TAB> intin values <TAB> comment
 *
 * The four count columns become control[1..4] and are stated EXPLICITLY
 * rather than derived from the value list.  That is the whole point of the
 * form: in the AES all five control slots are inputs, and IN_LEN / OUT_LEN
 * being settable independently of the values actually listed is what lets a
 * case measure the dispatcher's marshalling rule -- xif copies
 * min(IN_LEN, I_SIZE) words in and exactly OUT_LEN words back out
 * (EmuTOS aes/gemsuper.c:428-448, include/aesdefs.h:121-134).  A case that
 * declares fewer output words than the call produces leaves the surplus
 * slots poisoned, and that absence is the measurement.
 *
 * A "-" in the n_intin column means "as many as the value column lists".
 * In the other three count columns "-" is an ERROR: there is no value list
 * to derive them from, and 0 is one character away.  Guessing there would
 * silently change what the call declares.
 *
 * n_intout is refused above TD_AES_INTOUT_N.  xif's copy-back is
 * memcpy(pcrys_blk->intout, int_out, OUT_LEN*sizeof(WORD)) with NO clamp
 * (gemsuper.c:444-445), so a larger OUT_LEN overruns the caller's own
 * intout array on the target.  The other three counts are clamped by the
 * dispatcher (min(IN_LEN,I_SIZE), min(AIN_LEN,AI_SIZE)) or never read at
 * all (control[4]), so they are only refused when negative.
 *
 * Everything else -- physical line number as the case id, "#" comments,
 * blank lines, the "+" prelude convention, malformed line is an error -- is
 * identical to the VDI form above, and goes through the same line iterator
 * and the same value parser so the two cannot drift apart.
 *
 * ------------------------------------------------------------------
 * THE EIGHTH COLUMN: addr_in, and why it is optional
 * ------------------------------------------------------------------
 *
 * A table may declare an EIGHTH column with the "!columns" directive below:
 *
 *   opcode <TAB> n_intin <TAB> n_intout <TAB> n_addrin <TAB> n_addrout
 *          <TAB> intin values <TAB> addr_in <TAB> comment
 *
 * It is optional and it is declared per table rather than sniffed per line,
 * for the same reason the comment column is an argument to
 * td_case_line_comment_n: a comment may itself hold tabs, so counting the
 * tabs on a line does not tell you which form the line is in.  A table that
 * does not declare it is read exactly as before, its lines mean exactly what
 * they meant, and the records it produces are the same bytes -- which
 * matters, because every dump already taken is bound to its table by CRC and
 * a re-run of an old table has to reproduce its dump.
 *
 * The column is a comma separated list of PLANTS, or "-" for none.  Each
 * plant is <where>=<buffer>, where the buffer is a number 0..TD_AES_BUF_N-1
 * naming one of the dumper's own buffers (tosdump.h) and <where> is one of:
 *
 *   aN   addr_in[N], N < TD_AES_ADDRIN_N.  The address array proper.  The
 *        dispatcher copies min(AIN_LEN, AI_SIZE) of them onto its local
 *        buffer (gemsuper.c:439), so the plant is only reachable when the
 *        case declares n_addrin > N -- which the loader requires rather than
 *        letting the call read a slot that was never handed over.
 *
 *   iN   a pointer packed into the int_in WORDS at word N, at the TARGET's
 *        own pointer width -- four bytes on the m68k, eight on AArch64 --
 *        which is the classic GEM binding (gemlib a_wind_set_str.c:25,
 *        a_wind_get.c:344).  It is how EmuTOS' wm_set reads WF_NAME:
 *        *(char **)pinwds with pinwds = &int_in[2] (gemwmlib.c:1661), which
 *        no addr_in plant can reach.
 *
 *        ONLY ON A PRELUDE.  int_in is RECORDED, and a planted pointer
 *        occupies two WORDs of it on one machine and four on the other, so
 *        on a measured case the record would differ by the target's pointer
 *        width and not by anything the AES did.  A prelude is not recorded,
 *        so it can carry one; that is what lets a case be measured against a
 *        window that really has a title.
 *
 * A plant may not land on a WORD the case already lists a value for -- the
 * two would contradict each other -- and an "iN" plant needs the case to
 * declare enough IN_LEN for the widest of the two targets to copy it whole.
 */
#define TD_AES_CASE_COLUMNS     7
#define TD_AES_CASE_COLUMNS_MAX 8

/* Where one plant puts a buffer's address. */
#define TD_AES_PLANT_ADDRIN  0     /* addr_in[where]                      */
#define TD_AES_PLANT_INTIN   1     /* a target-width pointer at int_in[w] */

/* Plants one line may carry: the three addr_in slots plus one packed
 * pointer is every shape the AES has. */
#define TD_AES_PLANTS_MAX    4

/* The WORDs a planted int_in pointer occupies on the widest target (8-byte
 * pointer / 2-byte WORD).  The loader requires IN_LEN to cover it so that
 * one table line is correct on both. */
#define TD_AES_PLANT_PTR_WORDS 4

struct td_aes_plant {
	int16_t kind;                      /* TD_AES_PLANT_*                  */
	int16_t where;                     /* slot index / int_in word index  */
	int16_t buffer;                    /* 0..TD_AES_BUF_N-1               */
};

/* ------------------------------------------------------------------ */
/* The directives (spec section 5, for the AES)                        */
/* ------------------------------------------------------------------ */
/*
 * A line whose first non-blank character is '!' is a DIRECTIVE: it carries
 * no case and no call, but it does consume a line number like every other
 * line, so case ids stay the physical line numbers.  Two directives exist:
 *
 *   !reset   <TAB> none | wind_new
 *   !columns <TAB> 7 | 8
 *
 * Each must appear before the first case or prelude, at most once, and its
 * value must be one of the names below -- an unknown name is refused rather
 * than defaulting, because defaulting would silently run a round-2 table
 * with round 1's absent reset.
 *
 * `!columns 8` adds the addr_in column described above; absent means 7, the
 * form every table written before it uses.  It is stated before the first
 * case for the same reason `!reset` is: the parser has to know how to slice
 * a line before it slices one, and a file that is read differently than it
 * was written produces a dump that looks fine and measures something else.
 *
 * WHY THE TABLE DECLARES IT AND NOT THE PROGRAM.  Spec section 5 puts the
 * reset sequence in core/ so both targets run the identical one, and it is
 * in core/ here too -- td_reset_aes(), one fixed call.  What the table
 * chooses is only WHICH named reset runs, and that has to be the table's:
 * the dumper is double-clicked on the TT with no arguments and always reads
 * AES.TSV, so the file is the only thing that can tell two rounds apart, and
 * the file's CRC is what the dump header carries.  A reset selected by a
 * command line or a rebuild would not be provable from the dump.
 *
 * ABSENT MEANS `none`, and that is deliberate rather than lazy: round 1's
 * table (cases/aes-r1.tsv, CRC 33C13C8F) is frozen and its dumps are taken.
 * Any other default would silently change what re-running it measures, and
 * the comparator would then compare a reset run against an unreset reference
 * and call the difference a defect.
 *
 * `wind_new` issues opcode 109 with control[1..4] = 0, exactly as gemlib
 * lays it (a_wind_new.c:25).  It is the AES' OWN reset: wm_new closes and
 * deletes every window with a handle >= 1 and drains the wind_update mouse
 * and screen locks (gemwmlib.c:1875-1893).  EmuTOS issues it itself after
 * every graphics-mode program the shell runs (gemshlib.c:676-677), so a run
 * that issues it leaves nothing behind that the shell would not have swept
 * anyway.
 */
#define TD_AES_RESET_NONE      0
#define TD_AES_RESET_WIND_NEW  1

/* ------------------------------------------------------------------ */
/* The parameter guard: what the VALUES decide, not the opcode         */
/* ------------------------------------------------------------------ */
/*
 * The opcode guard (cases.c, the refusal guard) answers "may this call be
 * issued at all".  For a dozen opcodes there is no such answer, because the
 * same opcode is a poll or a hang, a read of the dumper's own buffer or a
 * write through 0xDEADDEAD, depending on ONE WORD of int_in or on whether
 * the addr_in column named a buffer.  An opcode-level verdict on those is
 * wrong in both directions at once -- it refuses the harmless arm and it
 * waves the dangerous one through.
 *
 * So a second question, "with THESE values?", is answered per line in
 * cases.c (aes_line_parameter_refusal), for cases and preludes alike.  It
 * covers, with the EmuTOS reading that makes each arm what it is:
 *
 *   wind_get/wind_set (104, 105) -- the field, the handle and the geometry
 *      (round 2, and WF_NAME/WF_INFO re-opened by a plant in M05)
 *   wind_open/wind_delete/wind_update (101, 103, 107) -- the handle, the
 *      message-queue bound and the mouse-control balance
 *   evnt_multi (25) -- MU_TIMER with a zero count is the only arm that
 *      returns before mwait; MU_MESAG needs a planted buffer
 *   form_error (53) -- n > 63 is the only arm that returns before fm_show
 *   form_keybd (55) -- the tree must be planted AND the character must not
 *      send find_obj walking it
 *   objc_delete (41) -- ROOT is the only arm that returns before the tree
 *      is read
 *   scrp_read/scrp_write (80, 81) -- a strcpy each way through addr_in[0],
 *      so the slot must be planted
 *
 * ALL OF IT IS DECIDED FROM ONE LINE, which is why it is in the LOADER and
 * not in td_aes_cases_version_refusal below: the flags, the field, the error
 * number, the character, the object index and the plants are all columns of
 * the line.  Nothing here needs a measured quantity, so nothing here needs a
 * machine, and a table can still be checked on the host before it is carried
 * anywhere.  That is the same seam M06 drew, taken from the other side.
 *
 * THE SHAPE RULE, and why it is not tidiness.  Where the guard reads an
 * int_in WORD, the case must both STATE it and declare IN_LEN past it.
 * xif's int_in is a LOCAL array copied into only as far as
 * min(IN_LEN, I_SIZE) (gemsuper.c:428-438), so a WORD outside IN_LEN is not
 * the previous call's value -- it is uninitialised stack, and a guard
 * reasoning about it would be reasoning about a number that does not exist.
 *
 * THE ASYMMETRY IT EXISTS FOR.  A table is written on a host, tried on
 * NewTOS, and then carried to a TT that has no memory protection.  Where the
 * two AESs disagree about whether a parameter is fatal, trying it on one
 * proves nothing about the other: objc_delete(-1) answers 0 on NewTOS
 * (tos-api/src/aes/object.rs) and reads tree[-1].ob_next through the poison
 * on the TT (gemoblib.c:1154-1157).  The guard therefore refuses the UNION
 * of what either machine would find fatal, never the intersection.
 * TASK/M07.md.
 */

/* ------------------------------------------------------------------ */
/* The version guard: decided on the machine, not on the host          */
/* ------------------------------------------------------------------ */
/*
 * Two of the AES opcodes with an arm in crysbind are compiled out of a
 * smaller build, and issuing one on a machine that lacks it reaches the
 * dispatcher's default arm -- fm_show(ALNOFUNC), a modal alert waiting for a
 * click (gemsuper.c:406-417).  On an unattended run that is a hang, which is
 * exactly what the loader's refusal guard exists to prevent.
 *
 * WHERE THE DECISION CAN BE MADE.  Whether a machine HAS such an opcode is a
 * property of the machine, and the only non-destructive way to ask is
 * `ap_version` in the AESGLOBAL -- which appl_init writes (gemsuper.c:92) and
 * which therefore does not exist until the dumper has an application context.
 * So the question splits in two, and each half is answered where its answer
 * exists:
 *
 *   the LOADER (host and machine alike) answers "is this opcode issuable at
 *      all" -- blocking, fatal, unestablished or not an opcode.  Those are
 *      properties of the table, so a table can be checked without a machine.
 *      It also records WHICH capabilities the table needs, in `features`.
 *
 *   the MACHINE answers "does this AES have them", once, after appl_init and
 *      before the first case or prelude runs, through
 *      td_aes_cases_version_refusal().  A machine that does not have them
 *      refuses the WHOLE table -- named reason, the offending line -- so the
 *      dump's shape stays a property of the table alone: every dump has
 *      exactly `count` records, or there is no dump.
 *
 * Until 2026-09-13 the loader refused these opcodes outright, on the stated
 * assumption "TOS 3.06 = AES 3.20".  The assumption is not wrong about TOS
 * 3.06; it was wrong to be the decider, because the reference EmuTOS the
 * dumps are taken against reports 0x0340 and HAS them.  Measured beats
 * assumed: TASK/M06.md.
 *
 * WHAT ap_version ESTABLISHES.  EmuTOS derives AES_VERSION from the
 * CONF_WITH_* flags that gate the arms (include/config.h:1345-1354):
 *
 *      0x0340  <=  3D_OBJECTS && MENU_EXTENSION && WINDOW_COLOURS
 *                  && GRAF_MOUSE_EXTENSION
 *      0x0330  <=  MENU_EXTENSION && WINDOW_COLOURS && GRAF_MOUSE_EXTENSION
 *      0x0320  <=  GRAF_MOUSE_EXTENSION
 *      0x0140  otherwise
 *
 * The implication runs ONE WAY: a rung of that ladder tells you which flags
 * were on, not the other way round (a build with 3D objects but no menu
 * extension reports 0x0320).  So a version establishes a feature only where
 * the ladder REQUIRES it, and the guard tests exactly that -- membership in
 * the set of rungs that require the flag, not ">= some minimum".  A machine
 * reporting a number the ladder does not produce establishes nothing, and
 * the table is refused rather than issued on a guess.
 *
 * Only these two flags remove an OPCODE.  WINDOW_COLOURS and
 * GRAF_MOUSE_EXTENSION gate fields and modes inside opcodes that exist
 * either way (gemwmlib.c:1566, :1718; gemgrlib.c:439), which is a
 * per-parameter question -- but not one the PARAMETER guard below can take
 * either, because "does this build have window colours" is still a measured
 * property of the machine.  Whoever wants those measured needs a third
 * shape: a per-case, per-feature refusal on the machine side.  Not built.
 *
 * CONF_WITH_PCGEM (appl_yield 17, scrp_clear 82, shel_rdef 126, shel_wdef
 * 127) is NOT in the ladder, so ap_version cannot settle it either way and
 * those four stay refused by the loader.  Measuring them needs a primary
 * source saying whether TOS 3.06 has them, not a different guard.
 */
#define TD_AES_FEAT_3D_OBJECTS  0x0001   /* CONF_WITH_3D_OBJECTS           */
#define TD_AES_FEAT_MENU_EXT    0x0002   /* CONF_WITH_MENU_EXTENSION       */

/*
 * How many DISTINCT window handles one table may open.  This is a hang
 * bound, not a style limit.
 *
 * Every wind_open sends the window's owner one WM_REDRAW (w_redraw,
 * gemwmlib.c:887-908, reached through draw_change -> w_update), and the
 * dumper is that owner.  It never reads its own message queue: every AES
 * call that would drain one blocks on an empty queue, so there is no
 * non-blocking drain to put in the reset.  doq unions successive WM_REDRAWs
 * for the same handle (gemqueue.c:45-56), so what accumulates is one message
 * per distinct handle; p_queue holds QUEUE_SIZE = 128 bytes = eight 16-byte
 * messages (struct.h:56, :152), and aqueue BLOCKS the sender when the next
 * message does not fit (gemqueue.c:96, :124).  Here the sender and the
 * receiver are the same process, so that block never ends.
 *
 * The reset frees every handle, so wm_create hands out handle 1 again
 * (gemwmlib.c:1369-1378) and the distinct-handle count over a whole run is
 * the largest number any single case opens.  Four leaves half the queue
 * free for the control manager's own messages, which an operator touching
 * the mouse during the run would generate.
 */
#define TD_AES_MAX_OPEN_WINDOWS 4

struct td_aes_prelude {
	int16_t  opcode;
	int16_t  n_intin;                  /* control[1], IN_LEN              */
	int16_t  n_intout;                 /* control[2], OUT_LEN             */
	int16_t  n_addrin;                 /* control[3], AIN_LEN             */
	int16_t  n_addrout;                /* control[4]                      */
	uint16_t n_values;                 /* int_in values actually listed   */
	uint16_t n_plants;                 /* buffers the addr_in column names*/
	int16_t  intin[TD_AES_INTIN_N];
	struct td_aes_plant plant[TD_AES_PLANTS_MAX];
};

struct td_aes_case {
	uint16_t line;                     /* physical line number = case id  */
	uint16_t prelude_first;            /* index into the table's pool     */
	uint16_t prelude_count;            /* calls issued before this case   */
	int16_t  opcode;                   /* control[0]                      */
	int16_t  n_intin;                  /* control[1], IN_LEN              */
	int16_t  n_intout;                 /* control[2], OUT_LEN             */
	int16_t  n_addrin;                 /* control[3], AIN_LEN             */
	int16_t  n_addrout;                /* control[4]                      */
	uint16_t n_values;                 /* int_in values actually listed   */
	uint16_t n_plants;                 /* buffers the addr_in column names*/
	int16_t  intin[TD_AES_INTIN_N];
	struct td_aes_plant plant[TD_AES_PLANTS_MAX];
	char     comment[TD_COMMENT_LEN];
};

struct td_aes_case_table {
	uint32_t              crc;         /* CRC32 over the raw file bytes   */
	int16_t               reset;       /* TD_AES_RESET_*, from "!reset"   */
	int16_t               columns;     /* 7 or 8, from "!columns"         */
	uint16_t              features;    /* TD_AES_FEAT_* this table needs  */
	uint16_t              count;
	uint16_t              prelude_count;
	struct td_aes_case    cases[TD_MAX_CASES];
	struct td_aes_prelude preludes[TD_MAX_PRELUDES];
};

int td_aes_cases_parse(struct td_aes_case_table *t, const char *buf,
                       size_t len, struct td_case_error *err);
int td_aes_cases_load(struct td_aes_case_table *t, const char *path,
                      struct td_case_error *err);

/*
 * May this table be issued on an AES that reports `ap_version`?
 *
 * Returns 0, or TD_CASE_ERR_AES_VERSION with `err` naming the capability and
 * the physical line of the case that needs it (for a prelude, the line of
 * the case it belongs to -- a prelude carries no line of its own, and the
 * balance it is part of is the case's).
 *
 * `ap_version` is the MEASURED global[0] the machine's own appl_init wrote,
 * never a constant: see the version guard above.  A table that needs nothing
 * (`features == 0`, which is every table written before 2026-09-13) is
 * accepted for any value, including a nonsense one -- there is nothing for
 * the version to decide.
 */
int td_aes_cases_version_refusal(const struct td_aes_case_table *t,
                                 int16_t ap_version,
                                 struct td_case_error *err);

/* What a table's `features` mask needs, in words, for a log or a report.
 * A fixed string per combination rather than a formatted one: the dumper
 * runs on a 4 MB TT with no allocator and this has to be printable there. */
const char *td_aes_features_text(uint16_t features);

/*
 * How many columns an AES table's lines have: what its "!columns" directive
 * says, or TD_AES_CASE_COLUMNS when it has none.  Exported because the
 * host-side reader has to take the comment from the same column the parser
 * does, and a reader that guessed would name every case after its addr_in
 * column.  A malformed directive answers with the default here and is
 * refused by td_aes_cases_parse, which is the one place that reports it.
 */
int td_aes_table_columns(const char *buf, size_t len);

/* ------------------------------------------------------------------ */
/* VDI raster case tables (cases/vdi-raster.tsv)                       */
/* ------------------------------------------------------------------ */
/*
 * Format, seven tab separated columns:
 *
 *   opcode <TAB> intin <TAB> ptsin <TAB> src <TAB> dst <TAB> fill <TAB> comment
 *
 * `src` and `dst` each describe one MFDB, as six comma separated values:
 *
 *   buffer , fd_w , fd_h , fd_wdwidth , fd_stand , fd_nplanes
 *
 * `buffer` is 0 for buffer A and 1 for buffer B -- the two fixed pixel
 * buffers the dumper owns.  fd_addr is not in the table because it cannot
 * be: it is an address, and the whole point of the table is that it is the
 * same file on two machines.  Naming a buffer instead of an address is also
 * what lets both MFDBs name the SAME buffer, which is how the in-place
 * vr_trnfm cases are stated.
 *
 * `fill` is two values, the seeds the two buffers are laid with before the
 * call (td_rast_fill).  Two different seeds give a source and a destination
 * that no logic op can confuse.
 *
 * There is no sub-opcode column: none of the three raster opcodes is an
 * escape.  There are no preludes: the raster opcodes read no attribute the
 * reset does not already set, and a '+' line is refused rather than silently
 * treated as a case.
 *
 * ------------------------------------------------------------------
 * THE GUARD
 * ------------------------------------------------------------------
 *
 * The AES loader refuses opcodes that would hang the machine (see above).
 * The raster loader has the same duty for a different hazard: EmuTOS does
 * NOT bounds-check a memory MFDB.  setup_info takes fd_addr, fd_wdwidth and
 * fd_nplanes at face value (vdi_raster.c:915-950) and bit_blt then walks
 * whatever they describe; vdi_vr_trnfm does not even check fd_addr for NULL
 * (vdi_raster.c:295-297).  A mistyped digit in this file would therefore
 * scribble over a running TT, and the damage would look like a defect in
 * the thing being measured.
 *
 * So the loader refuses the whole table -- named reason, non-zero return,
 * the offending line -- unless every case satisfies all of:
 *
 *   1. the opcode is one of 109, 110, 121.  Nothing else belongs in a
 *      table whose record carries two MFDBs and two pixel buffers.
 *   2. both MFDBs name a real buffer.  A NULL MFDB pointer, or an MFDB with
 *      fd_addr == 0 (the screen), is not expressible here at all: EmuTOS
 *      dereferences the MFDB pointer unconditionally, and the screen is a
 *      different size and depth on the two machines, so nothing measured
 *      against it could be compared anyway.
 *   3. the form fits the buffer under EITHER machine's reading of it:
 *      fd_h * fd_wdwidth * 16 <= TD_RAST_BUF_WORDS.  Sixteen, not
 *      fd_nplanes, because NewTOS reads a memory MFDB as one WORD per pixel
 *      (raster.rs) and the TT reads it as fd_nplanes interleaved planes
 *      (s_nxwd = fd_nplanes * 2, vdi_raster.c:922); sixteen words per
 *      sixteen pixels bounds both, and fd_nplanes is separately capped at
 *      16 (the CONF_WITH_VDI_16BIT plane limit, vdi_raster.c:963).
 *   4. fd_w <= fd_wdwidth * 16, fd_stand is 0 or 1, and all of fd_w, fd_h,
 *      fd_wdwidth, fd_nplanes are positive.
 *   5. for 109 and 121, PTSIN holds exactly eight values.  Fewer would
 *      leave the rest of the rectangle poisoned -- 0xDEAD is -8531, and a
 *      blit to it is exactly the scribble this guard exists to prevent.
 *      Both rectangles, after the arb_corner sort the VDI itself applies
 *      (vdi_raster.c:1336-1337), must lie inside the form they address.
 *   6. for 110, PTSIN is empty: vr_trnfm reads no rectangle.
 *
 * As in the AES table there is no override.  A case that cannot pass is a
 * case that is wrong.
 *
 * ------------------------------------------------------------------
 * THE SCREEN (cases/vdi-raster-screen.tsv, 2026-09-21)
 * ------------------------------------------------------------------
 *
 * Everything above was written when the two machines' screens differed in
 * size and depth.  Since the Falcon030 truecolor reference (Hatari, VIDEL
 * 640x480 at 16 planes) that is no longer so: EmuTOS there and NewTOS in
 * QEMU raspi4b run the same device model, and a screen operand is
 * comparable.  A table may therefore, IN ADDITION to the rules above:
 *
 *   * declare the screen it is written for, once, before the first case:
 *
 *       !screen <TAB> w,h,planes
 *
 *     The runner refuses the whole run unless the workstation it opened
 *     reports at least w x h (DEV_TAB[0..1], vdi_control.c:203-204) and
 *     exactly `planes` (INQ_TAB[4], vdi_control.c:210).  Without the
 *     directive no line may name the screen.
 *   * name the screen in an MFDB column with the single letter `S`.  The
 *     runner hands the VDI an MFDB whose fd_addr is NULL and whose other
 *     fields are 0 -- the classic screen MFDB -- and EmuTOS' setup_info
 *     then reads v_bas_ad / v_planes / v_lin_wr instead of the struct
 *     (vdi_raster.c:915-950).  The rectangle guard checks a screen rect
 *     against the declared w x h.  vr_trnfm (110) may not name the screen:
 *     it dereferences fd_addr without a NULL check (vdi_raster.c:295-297).
 *   * carry PRELUDES, `+` lines in the seven column form, issued after the
 *     reset and before the measured call and never recorded -- exactly the
 *     VDI table's rule.  They are what puts a known picture on the screen.
 *     Allowed: 109 and 121 (the same MFDB and rectangle guard as a case;
 *     the fill column must be `-`, the buffers are laid once per case from
 *     the case's own seeds), 25 vsf_color (one intin), 114 vr_recfl (four
 *     ptsin inside the screen) and 129 vs_clip (one intin, four ptsin
 *     inside the screen).  Nothing else: the whitelist is the guard.
 *   * ask for a READBACK: a fill column of six values,
 *
 *       seed_a,seed_b,rx,ry,rw,rh
 *
 *     After the measured call the runner reads the screen rectangle
 *     (rx,ry)-(rx+rw-1,ry+rh-1) pixel by pixel with v_get_pixel (through
 *     its own arrays, so the record's are untouched) and stores INTOUT[1]
 *     -- on a truecolor screen the 16-bit word itself, vdi_fill.c:1105-
 *     1110 -- into buffer B at row * 32 + col, the layout of a 32x16
 *     device form.  A readback is therefore compared exactly like a memory
 *     destination, and lands in the record where buffer B always is.  rw
 *     is at most 32 and rh at most 16, the rectangle must lie inside the
 *     declared screen, the case must name the screen, and it may not name
 *     buffer B (the readback would overwrite an operand or the result).
 *
 * A table without a `!screen` line, without `S`, without `+` lines and
 * with two-value fill columns -- cases/vdi-raster.tsv -- parses and runs
 * exactly as before; its CRC-bound dumps stay comparable.
 */
#define TD_RAST_CASE_COLUMNS 7

/* Which of the two buffers an MFDB addresses -- or the screen. */
#define TD_RAST_BUF_A      0
#define TD_RAST_BUF_B      1
#define TD_RAST_BUF_SCREEN 2

/* The readback lands in buffer B laid out as a 32 x 16 device form. */
#define TD_RAST_READ_W  32
#define TD_RAST_READ_H  16

struct td_rast_mfdb {
	int16_t buffer;                    /* TD_RAST_BUF_A / _B / _SCREEN    */
	int16_t w;                         /* fd_w   (screen: declared w)     */
	int16_t h;                         /* fd_h   (screen: declared h)     */
	int16_t wdwidth;                   /* fd_wdwidth (screen: 0)          */
	int16_t stand;                     /* fd_stand   (screen: 0)          */
	int16_t nplanes;                   /* fd_nplanes (screen: declared)   */
};

/* The screen a table declares with `!screen`. */
struct td_rast_screen {
	int16_t declared;                  /* 1 once the directive was seen   */
	int16_t w;
	int16_t h;
	int16_t planes;
};

/* A prelude of the raster table: one call, issued before the measured
 * one, never recorded.  Raster preludes (109/121) carry two MFDBs. */
struct td_rast_prelude {
	int16_t  opcode;
	uint16_t n_intin;
	uint16_t n_ptsin;
	int16_t  intin[TD_PRELUDE_VALUES];
	int16_t  ptsin[TD_PRELUDE_VALUES];
	struct td_rast_mfdb src;
	struct td_rast_mfdb dst;
};

struct td_rast_case {
	uint16_t line;                     /* physical line number = case id  */
	uint16_t prelude_first;            /* index into the table's pool     */
	uint16_t prelude_count;            /* calls issued before this case   */
	int16_t  opcode;                   /* contrl[0]: 109, 110 or 121      */
	uint16_t n_intin;                  /* values, becomes contrl[3]       */
	uint16_t n_ptsin;                  /* values; contrl[1] = n_ptsin / 2 */
	int16_t  intin[TD_RAST_INTIN_N];
	int16_t  ptsin[TD_RAST_PTSIN_N];
	struct td_rast_mfdb src;
	struct td_rast_mfdb dst;
	uint16_t seed_a;                   /* buffer A fill seed              */
	uint16_t seed_b;                   /* buffer B fill seed              */
	int16_t  has_read;                 /* six-value fill column           */
	int16_t  rx, ry, rw, rh;           /* the readback rectangle          */
	char     comment[TD_COMMENT_LEN];
};

struct td_rast_case_table {
	uint32_t               crc;        /* CRC32 over the raw file bytes   */
	uint16_t               count;
	uint16_t               prelude_count;
	struct td_rast_screen  screen;
	struct td_rast_case    cases[TD_RAST_MAX_CASES];
	struct td_rast_prelude preludes[TD_MAX_PRELUDES];
};

int td_rast_cases_parse(struct td_rast_case_table *t, const char *buf,
                        size_t len, struct td_case_error *err);
int td_rast_cases_load(struct td_rast_case_table *t, const char *path,
                       struct td_case_error *err);

/* ------------------------------------------------------------------ */
/* Call tables: BIOS + XBIOS (cases/bios-*.tsv), later GEMDOS           */
/* ------------------------------------------------------------------ */
/*
 * Format, five tab separated columns (docs/TOS-REFERENCE-DUMPER.md
 * section 6, "The call table, as built for DUMPBIOS"):
 *
 *   trap:function <TAB> args <TAB> retclass <TAB> capture <TAB> comment
 *
 *   trap:function  13 or 14 in a BIOS-area table -- and 1 for function 0x20
 *                  (Super) only -- then the function number, decimal or 0x
 *                  hex, 0..255.  "14:0x04" is Getrez.
 *   args           "-" or up to eight comma separated arguments in binding
 *                  order, each with its width: "w:<v>" a WORD, "l:<v>" a
 *                  LONG, "b:<n>" the dumper's buffer n (0..TD_CALL_BUF_N-1),
 *                  "f:<n>" the dumper's callback n (0..TD_CALL_CB_N-1, for
 *                  Supexec), "t:0" the token the last Super(0L) returned.
 *   retclass       EXACT | IGNORE | NONZERO | HANDLE | PTR_RAM, optionally
 *                  ".W" or ".B" (not on IGNORE or PTR_RAM): the width of the
 *                  EmuTOS handler's return type.  No suffix = LONG.
 *   capture        "-" or <argindex>:<length>[.B|.W|.L][@<offset>] -- copy
 *                  `length` bytes from `offset` of the buffer argument
 *                  `argindex` (0-based), as elements of the unit.
 *
 * Physical line number = case id, '#' comments, a malformed line refuses
 * the whole table: as for every other form.  There are NO preludes and NO
 * directives: a '+' or '!' line is refused.  Round 1 is queries only, so
 * there is no state to reset or to establish.
 *
 * The disk round (TASK/TOOL07; docs/TOS-REFERENCE-DUMPER.md section 6, "The
 * disk round") is the one exception: a table that starts with
 *   !disk <TAB> <drive>          its test drive, 2 (C:) .. 15 (P:)
 * may name Rwabs, Getbpb and Mediach, and may hold '+' preludes -- Rwabs
 * reads into a dumper buffer, retclass and capture "-", issued after the
 * case's poison and before its call.  An Rwabs write is a measured case that
 * writes back exactly what its last prelude read (write-back only), on the
 * test drive.  The capture column also takes "r:<length>..." -- through the
 * address the call returned, retclass PTR_RAM only.
 *
 * Two additions of the closing tables (TASK/TOOL07, user decision
 * 2026-10-28, same section of the spec):
 *   !disk <TAB> <drive> <TAB> destructive
 *                     the table changes the test image: it runs on a fresh
 *                     clone of the test image only (the runners enforce it,
 *                     tools/hatari-oracle.sh and cargo xtask tosdump), and
 *                     only such a table may hold
 *   +set <TAB> <offset>=<byte>[,<offset>=<byte>...]
 *                     a prelude that changes the buffer of the read prelude
 *                     IMMEDIATELY before it -- at most eight pairs, offsets
 *                     inside what that read read, no offset twice; the case
 *                     after it is the write that writes "read + set" back
 *                     (the write-back rule then names the read before the
 *                     +set).  Offset and byte decimal or 0x hex.
 *   u:0               an argument: the physical unit of the test disk, for
 *                     Rwabs with RW_NOTRANSLATE -- the only way that flag is
 *                     admitted (reads, b: buffer, count 0..2).  The runner
 *                     passes the unit the run's profile names
 *                     (td_run_call_set_disk_unit; DUMPBIOS reads it from
 *                     DISKUNIT.TXT, which the profile's runner writes: the
 *                     oracle's profile disk IDE master 18, NewTOS the SD
 *                     card 26 -- docs/api/bios/04-rwabs.md, "Decisions");
 *                     the record carries TD_CALL_UNIT_TOKEN, never the
 *                     number.
 *
 * ------------------------------------------------------------------
 * THE GUARD -- a whitelist
 * ------------------------------------------------------------------
 *
 * A BIOS typo can write a sector (Rwabs), install a vector (Setexc) or wait
 * forever for a key (Bconin).  The loader therefore refuses the whole table
 * unless EVERY line is a call whose safety on both machines -- EmuTOS on the
 * TT and NewTOS on the Pi -- is established in tosdump/cases/
 * COVERAGE-BIOS.md: the trap and function are on the list in cases.c
 * (call_policy), the argument widths are the binding's, and the values pick
 * the query arm.  Anything else is "not established", and a later round
 * that establishes it extends the list.  As in the AES and raster guards
 * there is no override.
 *
 * Super is admitted as l:1 (the inquiry) and as a BRACKET: l:0 opens it,
 * t:0 closes it, no nesting, and a table that ends with it open is refused
 * (TD_CASE_ERR_UNSAFE_STATE) -- MEM12, docs/TOS-REFERENCE-DUMPER.md section
 * 6, "Super and Supexec in a BIOS-area table".
 */
#define TD_CALL_CASE_COLUMNS 5

/* Argument kinds of a call-table line. */
#define TD_CALL_ARG_WORD     1     /* "w:"                                */
#define TD_CALL_ARG_LONG     2     /* "l:"                                */
#define TD_CALL_ARG_BUF      3     /* "b:", value = buffer number         */
#define TD_CALL_ARG_TOKEN    4     /* "t:", value = 0: the Super token    */
#define TD_CALL_ARG_FUNC     5     /* "f:", value = callback number       */
#define TD_CALL_ARG_UNIT     6     /* "u:0", the test disk's physical unit */
#define TD_CALL_ARG_SET      7     /* a +set pair: (offset << 8) | byte    */
#define TD_CALL_ARG_PUT      8     /* a +put field: its slot in the layout */

/* The trap field of a "+set" prelude: it issues no trap. */
#define TD_CALL_TRAP_SET     0
#define TD_CALL_SET_PAIRS    TD_CALL_ARGS_N
/* The trap field of a "+put" prelude (TOOL16): it issues no trap either. */
#define TD_CALL_TRAP_PUT     (-1)

/* ------------------------------------------------------------------ */
/* Layouts: structures behind a pointer (TASK/TOOL16)                   */
/* ------------------------------------------------------------------ */
/*
 * docs/TOS-REFERENCE-DUMPER.md section 6, "Structures behind a pointer".
 * A call table may describe a structure and capture it field by field:
 *
 *   !layout <TAB> <name> <TAB> <size m68k>/<size LP64> <TAB> <field>,...
 *
 *   field   <fname>:<type>@<offset m68k>/<offset LP64>[:<class>]
 *   type    b  UBYTE                1 / 1 bytes, zero-extended
 *           w  WORD                 2 / 2, sign-extended
 *           L  LONG (int32)         4 / 4, sign-extended
 *           l  C long               4 / 8, sign-extended
 *           p  a pointer            4 / 8, sign-extended (the ret rule)
 *           p><layout>  a pointer the runner FOLLOWS into <layout>
 *           [<layout>]  <layout> embedded at this offset
 *   class   EXACT, IGNORE, NONZERO, HANDLE, PTR_RAM -- how tosdiff compares
 *           the field (the retclass rules, on the 64-bit slot).  Default:
 *           PTR_RAM for p, EXACT for everything else; none on [..].
 *
 * The layout itself is not the dumper's: it is copied from the structure's
 * spec in docs/api/ (a "tosdump-layout" block), and the host test
 * `layoutcheck` (tosdump/host/layoutcheck.c) refuses a table whose
 * "!layout" line is not, byte for byte, the spec's.  The loader checks the
 * numbers against C: every offset is the one C gives the fields in this
 * order on that ABI (m68k: everything but a UBYTE WORD aligned; LP64:
 * natural alignment), every size the end rounded to the structure's
 * alignment.  A layout names only layouts defined before it, so following
 * a pointer cannot loop; TD_LAYOUT_MAX_HOPS bounds the depth anyway.
 *
 * Capture:  r:<layout>   through the returned pointer (retclass PTR_RAM)
 *           <n>:<layout> the b: buffer argument n
 * The record's capture holds one 64-bit slot per field (tosdump.h,
 * TD_LAYOUT_SLOT_BYTES).  The runner follows a pointer -- the returned one,
 * or a p> field -- only when td_layout_follow_refusal (record.h) admits it;
 * otherwise every slot behind it is TD_LAYOUT_NOREAD(reason) and nothing
 * is read.
 *
 * Writing through a returned pointer, only in a table that says
 *   !write <TAB> return
 * is one prelude form:
 *   +put <TAB> <field>=<field> [<TAB> comment]
 * immediately after the case whose "r:<layout>" capture it writes through:
 * copy one field of that structure onto another of the same type -- the
 * flush "ibufhd=ibuftl".  Only top-level, non-pointer fields; issued after
 * the next case's poison, before its call, and only when the returned
 * pointer passes the follow rule.
 */
#define TD_LAYOUT_ABI_M68K    0
#define TD_LAYOUT_ABI_LP64    1
#define TD_LAYOUT_ABIS        2

#define TD_LT_BYTE            1    /* b */
#define TD_LT_WORD            2    /* w */
#define TD_LT_LONG            3    /* L */
#define TD_LT_CLONG           4    /* l */
#define TD_LT_PTR             5    /* p */

#define TD_LAYOUT_MAX         8    /* layouts per table               */
#define TD_LAYOUT_NAME_LEN   16    /* a name, NUL included            */
#define TD_LAYOUT_PATH_LEN   48    /* "mp_free->md_start", NUL incl.  */
#define TD_LAYOUT_MAX_HOPS    2    /* pointers followed from the base */

struct td_layout_slot {
	char     name[TD_LAYOUT_PATH_LEN];  /* the field's path            */
	uint8_t  type;                      /* TD_LT_*                     */
	uint8_t  cls;                       /* TD_RET_*                    */
	int8_t   parent;     /* the pointer slot this field hangs from, -1:
	                      * the base (the returned pointer / the buffer) */
	int8_t   follow;     /* a followed pointer: the layout it points at */
	uint8_t  hops;       /* pointers followed to reach this field       */
	uint16_t off[TD_LAYOUT_ABIS];       /* in the parent's structure   */
};

struct td_layout {
	char     name[TD_LAYOUT_NAME_LEN];
	uint16_t size[TD_LAYOUT_ABIS];
	uint8_t  align[TD_LAYOUT_ABIS];
	uint8_t  nslots;
	struct td_layout_slot slot[TD_LAYOUT_SLOTS];
};

/* Bytes of a field type on an ABI. */
uint8_t td_layout_width(uint8_t type, int abi);
/* The ABI of the machine running this code: LP64 when a pointer has 8
 * bytes, m68k when 4 -- measured, td_native_ptr_width (record.h). */
int     td_layout_native_abi(void);
/* The slot index of a field path, or -1. */
int     td_layout_slot_find(const struct td_layout *l, const char *path);

/*
 * Parse one "!layout" line into set[*n] (layouts before it in set[0..*n-1]
 * may be named).  0 or a TD_CASE_ERR_*, *why the reason.  Exposed for the
 * host test that binds the tables' layouts to docs/api/ (layoutcheck).
 */
int td_layout_parse(struct td_layout *set, uint8_t *n, const char *line,
                    size_t len, const char **why);

struct td_call_arg {
	uint8_t kind;                      /* TD_CALL_ARG_*                   */
	int32_t value;                     /* WORD / LONG value, buffer no.   */
};

/* The capture reads through the address the call RETURNED ("r:" in the
 * capture column, disk tables), not through a buffer argument. */
#define TD_CALL_CAP_RET     (-2)

/* Preludes of a disk table ("+" lines, "!disk" only): Rwabs reads into a
 * dumper buffer, issued after the case's poison and before its call; in a
 * destructive table also "+set" (trap TD_CALL_TRAP_SET, nargs pairs of kind
 * TD_CALL_ARG_SET), which changes the buffer of the read before it. */
#define TD_MAX_CALL_PRELUDES 32

struct td_call_case {
	uint16_t line;                     /* physical line number = case id  */
	uint16_t prelude_first;            /* index into the table's preludes */
	uint16_t prelude_count;            /* Rwabs reads issued before it    */
	int16_t  trap;                     /* 13 / 14, 1 for Super            */
	int16_t  opcode;                   /* function number                 */
	uint16_t nargs;
	struct td_call_arg arg[TD_CALL_ARGS_N];
	uint8_t  retclass;                 /* TD_RET_*                        */
	uint8_t  retwidth;                 /* TD_RET_WIDTH_*                  */
	int8_t   cap_arg;                  /* -1: none, TD_CALL_CAP_RET: r:   */
	uint8_t  cap_layout;               /* 0: none, else 1 + the capture's
	                                    * layout index (TOOL16)           */
	uint8_t  cap_unit;                 /* 1, 2, 4                         */
	uint16_t cap_off;                  /* bytes into the buffer           */
	uint16_t cap_len;                  /* bytes, <= TD_CALL_CAPTURE_LEN   */
	char     comment[TD_COMMENT_LEN];
};

struct td_call_case_table {
	uint32_t            crc;           /* CRC32 over the raw file bytes   */
	uint16_t            area;          /* TD_AREA_BIOS                    */
	uint16_t            count;
	uint8_t             disk;          /* 1: the table says "!disk"       */
	uint8_t             disk_drive;    /* its test drive (2 = C:)         */
	uint8_t             destructive;   /* 1: "!disk <d> destructive"      */
	uint8_t             unit_token;    /* 1: some case names "u:0"        */
	uint8_t             writes;        /* 1: "!write return" (TOOL16)     */
	uint8_t             nlayouts;      /* "!layout" lines (TOOL16)        */
	struct td_layout    layouts[TD_LAYOUT_MAX];
	uint16_t            prelude_count;
	struct td_call_case cases[TD_MAX_CASES];
	struct td_call_case preludes[TD_MAX_CALL_PRELUDES];
};

/* `area` is the dump's area: TD_AREA_BIOS (traps 13 and 14).  No other
 * area has a guard yet, so any other is refused. */
int td_call_cases_parse(struct td_call_case_table *t, uint16_t area,
                        const char *buf, size_t len,
                        struct td_case_error *err);
int td_call_cases_load(struct td_call_case_table *t, uint16_t area,
                       const char *path, struct td_case_error *err);

/* "EXACT", "IGNORE", ... -- for a report. */
const char *td_retclass_name(uint8_t retclass);

/*
 * Shared line iterator.  One rule for what a line is, used by the parser and
 * by the host-side reader, so the two cannot drift apart.  Returns 0 when the
 * buffer is exhausted; *line_no counts every physical line including comments.
 */
struct td_line_iter {
	const char *buf;
	size_t      len;
	size_t      pos;
	uint16_t    line_no;
	int         done;
};

void td_line_iter_init(struct td_line_iter *it, const char *buf, size_t len);
/* On return, start and len describe the line with any CR stripped, line_no is
 * its physical number, and is_case says whether it carries a case. */
int  td_line_iter_next(struct td_line_iter *it, const char **start,
                       size_t *len, uint16_t *line_no, int *is_case);

#endif /* TOSDUMP_CASES_H */
