/*
 * wvprobe.c -- the vex_* default vectors as an application sees them, and the
 * save / restore of the AES's wheel vector (TASK/V56).
 *
 * A well-behaved GEM program that wants the mouse wheel itself does
 *
 *     vex_wheelv(handle, own, &old);    ... its own handler runs ...
 *     vex_wheelv(handle, old, &dummy);  ... and gives the AES's back
 *
 * (gemlib vex_wheelv.c; EmuTOS vdi_vex_wheelv, vdi/vdi_mouse.c:424-442).  The
 * vector is only an address: EmuTOS stores it and calls it from wheel_int.
 * NewTOS has to know whose code the address is (the AES's EL1 handler or an
 * EL0 routine), so vex_wheelv recognises the AES's address when it comes back
 * -- which needs the AES's address to differ from the VDI's no-op default
 * just_rts.  Before V56 the release kernel folded the three empty marker
 * functions just_rts, mov_cur and the AES wheel token into one address, the
 * restore was taken for "no handler", and the desktop's wheel stayed dead.
 *
 * Same C source on both sides (tosdump/core/abi.h backends).  Steps:
 *   1. appl_init, graf_handle, v_opnvwk.
 *   2. vex_butv, vex_curv, vex_wheelv each: install an own routine, take the
 *      old vector, put the old one straight back (the round trip a program
 *      makes).  The own routine is an empty function; between install and
 *      restore nothing is fed, so it is not expected to run.
 *   3. Log, per line, a 0/1 the two sides must agree on:
 *        E1  wheelv's old == butv's old    EmuTOS: aes_wheel vs far_bcha;
 *        E2  curv's old   == butv's old    mov_cur vs far_bcha;
 *        E3  wheelv's old == curv's old    aes_wheel vs mov_cur.
 *      NewTOS' AES leaves butv at just_rts (TASK/V53 side finding 2), so its
 *      E1/E2 compare against just_rts -- still distinct addresses.
 *        R1  restoring wheelv hands back the own routine (the swap works)
 *        R2  a second swap with the saved old hands back the saved old (it
 *            was stored as given)
 *   4. v_clsvwk, appl_exit.
 * Lines starting "A " carry the raw addresses (differ by side, not compared).
 *
 * The observable damage is on NewTOS after the probe has ended: the desktop
 * the #Z autorun returns to must still scroll a window with the wheel.
 * tosdump/probes/run-wvprobe.sh drives that scene (injector wheel detents,
 * QMP screendumps before / after).
 *
 * Output: WVPROBE.LOG in the current directory, last line "END".
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

static int16_t contrl[16];
static int16_t intin[16];
static int16_t ptsin[16];
static int16_t intout[64];
static int16_t ptsout[16];
static int16_t handle;

static int16_t aes_control[5];
static int16_t aes_global[15];
static int16_t aes_int_in[16];
static int16_t aes_int_out[8];
static void *aes_addr_in[2];
static void *aes_addr_out[2];

static char logbuf[4096];
static int loglen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (loglen < (int)sizeof(logbuf) - 200)
		loglen += vsnprintf(logbuf + loglen, sizeof(logbuf) - loglen, fmt, ap);
	va_end(ap);
}

static int16_t aes(int16_t op, int16_t nin, int16_t nout, int16_t nain)
{
	aes_control[0] = op;
	aes_control[1] = nin;
	aes_control[2] = nout;
	aes_control[3] = nain;
	aes_control[4] = 0;
	td_abi_aes(aes_control, aes_global, aes_int_in, aes_int_out,
	           aes_addr_in, aes_addr_out);
	return aes_int_out[0];
}

/* pointer n at CONTRL[7 + n * sizeof(void *) / 2], as gemlib packs it */
static void set_ptr(int n, void *p)
{
	memcpy(&contrl[7 + n * (int)(sizeof(void *) / sizeof(int16_t))], &p, sizeof p);
}

static void *get_ptr(int n)
{
	void *p;

	memcpy(&p, &contrl[7 + n * (int)(sizeof(void *) / sizeof(int16_t))], sizeof p);
	return p;
}

/* vex_butv 125, vex_curv 127, vex_wheelv 134: install newv, answer the old */
static void *vex(int16_t op, void *newv)
{
	memset(contrl, 0, sizeof contrl);
	contrl[0] = op;
	contrl[6] = handle;
	set_ptr(0, newv);
	set_ptr(1, (void *)0);
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);
	return get_ptr(1);
}

/* three own routines; distinct bodies, so no compiler folds them either */
static volatile int own_hits[3];
static void own_but(void) { own_hits[0]++; }
static void own_cur(void) { own_hits[1] += 2; }
static void own_wheel(void) { own_hits[2] += 3; }

int main(void)
{
	void *but_old, *cur_old, *wh_old, *back, *again;
	long fh;
	int i;

	if (aes(10, 0, 1, 0) < 0)		/* appl_init */
		return 1;
	aes(77, 0, 5, 0);			/* graf_handle */
	for (i = 0; i < 10; i++)
		intin[i] = 1;
	intin[10] = 2;
	memset(contrl, 0, sizeof contrl);
	contrl[0] = 100; contrl[3] = 11; contrl[6] = aes_int_out[0];
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);	/* v_opnvwk */
	handle = contrl[6];
	emit("H graf_handle %d handle %d\n", aes_int_out[0], handle);

	but_old = vex(125, (void *)own_but);
	(void)vex(125, but_old);
	cur_old = vex(127, (void *)own_cur);
	(void)vex(127, cur_old);
	wh_old = vex(134, (void *)own_wheel);
	back = vex(134, wh_old);
	again = vex(134, wh_old);

	emit("A butv old %p curv old %p wheelv old %p\n", but_old, cur_old, wh_old);
	emit("E1 wheelv old == butv old: %d\n", wh_old == but_old);
	emit("E2 curv old == butv old: %d\n", cur_old == but_old);
	emit("E3 wheelv old == curv old: %d\n", wh_old == cur_old);
	emit("R1 restoring wheelv returns the own routine: %d\n", back == (void *)own_wheel);
	emit("R2 the restored wheelv is the saved old: %d\n", again == wh_old);
	emit("A own routine ran %d %d %d\n", own_hits[0], own_hits[1], own_hits[2]);

	memset(contrl, 0, sizeof contrl);
	contrl[0] = 101; contrl[6] = handle;
	td_abi_vdi(contrl, intin, ptsin, intout, ptsout);	/* v_clsvwk */
	aes(19, 0, 1, 0);			/* appl_exit */
	emit("END\n");

	fh = Fcreate("WVPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
	return 0;
}
