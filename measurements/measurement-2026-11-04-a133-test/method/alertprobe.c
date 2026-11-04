/*
 * alertprobe.c (A133 tester, derived from A129 fsprobe.c) -- A128 measurement helper (not a permanent test): a GEM program
 * that opens the file selector on C:\LIST\*.* (30 files on the run's card)
 * and logs what fsel_exinput returned. The scripted IKBD injector
 * (tools/inject-arm.py, armed launch:FSPROBE) clicks the slider track, drags
 * the thumb and turns the wheel while the dialog is up; the screendumps are
 * the evidence. AES calls through the tosdump ABI backend (td_abi_aes,
 * tosdump/abi/newtos_svc.c), GEMDOS through mintlib's osbind.h, like
 * tosdump/probes/akprobe.c.
 */
#include <osbind.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "abi.h"

static int16_t control[5];
static int16_t global[15];
static int16_t int_in[16];
static int16_t int_out[8];
static void *addr_in[3];
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

static char text[] = "[1][Hello world, 123 456|NewTOS form_alert probe|third line][OK|Cancel]";

int main(void)
{
	aes(10, 0, 1, 0);			/* appl_init */
	int_in[0] = 1;
	addr_in[0] = text;
	aes(52, 1, 1, 1);			/* form_alert */
	aes(19, 0, 1, 0);			/* appl_exit */
	return 0;
}
