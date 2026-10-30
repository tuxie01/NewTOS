/*
 * akprobe.c -- the keyboard state the AES reports (TASK/A126): only bits 0-3
 * of the BIOS shift byte, with CapsLock and Alt held and with bits 4-7 set by
 * the program.  A permanent probe of the tosdump tree like ksprobe.c: it needs
 * real key strokes and a mouse button, which no case table can give, and the
 * set arm of Kbshift, which DUMPBIOS refuses; it writes a text log, no record
 * file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS): the AES
 * calls go through the tosdump ABI backends (abi/m68k_trap.c, abi/newtos_svc.c,
 * td_abi_aes), the BIOS / GEMDOS calls through mintlib's osbind.h.  Built by
 * `make -C tosdump akprobe-m68k` (AKPROBE.PRG) and `make -C tosdump
 * akprobe-newtos` (AKPROBE.NAP, tosdump/build-akprobe-newtos.sh); run by
 * tosdump/probes/run-akprobe.sh, which injects the input of
 * tosdump/probes/akprobe-keys.txt.  Output: AKPROBE.LOG in the current
 * directory.
 *
 * EmuTOS: the AES takes the keyboard state from gsx_kstate
 * (references/emutos/aes/gemgsxif.c:432-436) = the VDI's vq_key_s
 * (vdi/vdi_input.c:110-113) = gshift_s() = Kbshift(-1) & 0x000f (:183-187);
 * chkkbd keeps it in kstate (aes/gemdisp.c:165,175; aes/geminput.c:288-293),
 * which ev_rets (aes/gemevlib.c:56: evnt_button, evnt_mouse, evnt_multi
 * EV_KS) and gr_mkstate (aes/gemgrlib.c:463: graf_mkstate) report.  EmuTOS
 * refreshes kstate in the dispatcher that runs after every AES call
 * (aes/gemsuper.c:459-463), so the probe reads the state after an evnt_timer.
 *
 * Lines (compared, except I-lines, which are side-specific):
 *   G00  the state at the start: Kbshift(-1) and graf_mkstate's kstate.
 *   Part K -- keys (akprobe-keys.txt; the first input 3 s after the launch on
 *   NewTOS, 0.3 s after AKPROBE.RDY on the oracle):
 *     CapsLock make+break (toggles MODE_CAPS on), Alt make (held).  The probe
 *     polls Kbshift(-1) behind evnt_timer(20) until it reads 0x18 for 200 ms:
 *   S01  Kbshift(-1) and graf_mkstate's kstate                 [18 / 0008]
 *     right button down, right button up (Alt still held):
 *   E01  evnt_multi(MU_BUTTON|MU_TIMER, 1 click, mask 2, state 2, 15 s):
 *        the events and EV_KS                                    [0002 0008]
 *   B01  evnt_button(1, 2, 0) (wait for right up): ret and EV_KS [0001 0008]
 *     'a' make+break (Alt still held):
 *   K01  evnt_multi(MU_KEYBD|MU_TIMER, 15 s): events, EV_KS and the scancode
 *        byte of EV_KRET                                      [0001 0008 1e]
 *   I01  EV_KRET's ASCII byte (keyboard table, not compared)
 *     Alt break, CapsLock make+break (MODE_CAPS off); the probe waits for
 *     Kbshift(-1) 0x00 for 200 ms:
 *   S02  Kbshift(-1) and graf_mkstate's kstate                 [00 / 0000]
 *   Part P -- the program sets the byte (Kbshift(flag), bios/ikbd.c:145-153):
 *   P01  Kbshift(0xf0): CapsLock, Clr/Home, Insert, bit 7, no modifier;
 *        after evnt_timer(50): graf_mkstate kstate, evnt_multi(MU_TIMER, 10 ms)
 *        EV_KS, evnt_button(1, 2, 0) EV_KS                   [0000 0000 0000]
 *   P02  Kbshift(0xff): all eight bits, the same three reads  [000f 000f 000f]
 *   P03  Kbshift(old) restores the byte.
 *   A part that times out says so (S98/S99) and the probe goes on.
 *
 * ret / ks values are the low 16 bits of the AES int_out word.
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>

#include "abi.h"

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
	long fh = Fcreate("AKPROBE.LOG", 0);

	if (fh < 0)
		return;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
}

#define KBSHIFT(f)   trap_13_ww((short)11, (short)(f))

/* ---- the AES ------------------------------------------------------------ */
static int16_t control[5];
static int16_t global[15];
static int16_t int_in[16];
static int16_t int_out[8];
static void *addr_in[2];
static void *addr_out[2];
static int16_t msgbuf[8];

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

static void evnt_timer(long ms)
{
	int_in[0] = (int16_t)(ms & 0xFFFF);
	int_in[1] = (int16_t)((ms >> 16) & 0xFFFF);
	aes(24, 2, 1, 0);
}

/* graf_mkstate: int_out[4] = kstate */
static unsigned mkstate(void)
{
	aes(79, 0, 5, 0);
	return (uint16_t)int_out[4];
}

/* evnt_button(clicks, mask, state): RET_CODE, int_out[4] = EV_KS */
static unsigned evnt_button(int16_t clicks, int16_t mask, int16_t state)
{
	int_in[0] = clicks;
	int_in[1] = mask;
	int_in[2] = state;
	return (uint16_t)aes(21, 3, 5, 0);
}

/* evnt_multi with the button arguments and a timer: RET_CODE = events */
static unsigned evnt_multi(int16_t flags, int16_t clicks, int16_t mask,
                           int16_t state, long ms)
{
	int i;

	for (i = 0; i < 16; i++)
		int_in[i] = 0;
	int_in[0] = flags;
	int_in[1] = clicks;
	int_in[2] = mask;
	int_in[3] = state;
	int_in[14] = (int16_t)(ms & 0xFFFF);
	int_in[15] = (int16_t)((ms >> 16) & 0xFFFF);
	addr_in[0] = msgbuf;
	return (uint16_t)aes(25, 16, 7, 1);
}

#define MU_KEYBD   0x0001
#define MU_BUTTON  0x0002
#define MU_TIMER   0x0020

static unsigned long kb(void)
{
	return (unsigned long)KBSHIFT(-1) & 0xFFUL;
}

/*
 * Poll Kbshift(-1) behind evnt_timer(20) until it reads `want` for 200 ms
 * (10 polls in a row); give up after 15 s.  Returns 1 when reached.
 */
static int await(unsigned long want)
{
	int polls, same = 0;

	for (polls = 0; polls < 750; polls++) {
		evnt_timer(20);
		if (kb() == want) {
			if (++same >= 10)
				return 1;
		} else {
			same = 0;
		}
	}
	return 0;
}

static void state_line(const char *id, unsigned long want)
{
	if (!await(want)) {
		emit("%s timeout: Kbshift(-1)=%02lx, waited for %02lx\n", id, kb(), want);
		return;
	}
	evnt_timer(20);
	emit("%s Kbshift(-1)=%02lx graf_mkstate ks=%04x\n", id, kb(), mkstate());
}

static void part_k(void)
{
	unsigned ev, ret;
	long fh;

	fh = Fcreate("AKPROBE.RDY", 0);
	if (fh >= 0)
		Fclose((short)fh);

	state_line("S01", 0x18);

	ev = evnt_multi(MU_BUTTON | MU_TIMER, 1, 2, 2, 15000L);
	emit("E01 evnt_multi(MU_BUTTON|MU_TIMER) ev=%04x ks=%04x\n", ev, (uint16_t)int_out[4]);
	emit("I01 E01 mb=%04x bret=%04x\n", (uint16_t)int_out[3], (uint16_t)int_out[6]);

	ret = evnt_button(1, 2, 0);
	emit("B01 evnt_button(1,2,0) ret=%04x ks=%04x\n", ret, (uint16_t)int_out[4]);

	ev = evnt_multi(MU_KEYBD | MU_TIMER, 0, 0, 0, 15000L);
	emit("K01 evnt_multi(MU_KEYBD|MU_TIMER) ev=%04x ks=%04x sc=%02x\n", ev,
	     (uint16_t)int_out[4], ((uint16_t)int_out[5] >> 8) & 0xFF);
	emit("I02 K01 asc=%02x\n", (uint16_t)int_out[5] & 0xFF);

	state_line("S02", 0x00);
}

static void p_line(const char *id, int16_t flag)
{
	unsigned mk, ev, ret;
	unsigned ks_multi;

	KBSHIFT(flag);
	evnt_timer(50);
	mk = mkstate();
	ev = evnt_multi(MU_TIMER, 0, 0, 0, 10L);
	ks_multi = (uint16_t)int_out[4];
	ret = evnt_button(1, 2, 0);
	emit("%s Kbshift(%02x) now=%02lx graf_mkstate ks=%04x evnt_multi ev=%04x ks=%04x "
	     "evnt_button ret=%04x ks=%04x\n", id, (unsigned)flag & 0xFF, kb(), mk, ev,
	     ks_multi, ret, (uint16_t)int_out[4]);
}

static void part_p(void)
{
	unsigned long s0 = kb();

	p_line("P01", 0xf0);
	p_line("P02", 0xff);
	KBSHIFT((short)s0);
	evnt_timer(50);
	emit("P03 Kbshift(%02lx) restore now=%02lx graf_mkstate ks=%04x\n", s0, kb(), mkstate());
}

int main(void)
{
	int16_t ap_id = aes(10, 0, 1, 0);	/* appl_init */

	if (ap_id < 0) {
		emit("A99 appl_init=%d\n", ap_id);
		flush_log();
		return 1;
	}
	evnt_timer(20);
	emit("G00 Kbshift(-1)=%02lx graf_mkstate ks=%04x\n", kb(), mkstate());
	part_k();
	part_p();
	emit("Z99 end\n");
	flush_log();
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
