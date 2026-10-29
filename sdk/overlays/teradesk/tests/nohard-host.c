/*
 * nohard-host.c -- host harness for TeraDesk's overscan gate (TASK/SDK07,
 * patch site SDK07: sdk/overlays/teradesk/nohard.h, used by the video.c
 * overlay). Built and run by nohard-host.sh beside this file.
 *
 * Same construction as cookie-host.c: this file includes TeraDesk's library.h
 * the way the -D_TOS_ONLY build does, replaces mintlib's Setexc macro with
 * host_setexc() (a fake jar in the kernel's $5A0 wire format: 8-byte slots of
 * two 32-bit LONGs, {0, capacity} terminator), #includes the overlay
 * cookie.c (find_cookie, patch site SDK04-C6) and then nohard.h unchanged.
 *
 * Checks: newtos_mch_nohard() is TRUE exactly when the jar holds
 * _MCH = 0xFFFFFFFF (MCH_NOHARD, the value NewTOS publishes; ADR-012
 * amendment 2026-10-28), in an odd and in an even slot; FALSE for the Atari
 * _MCH values 0x00000000 (ST), 0x00010000 (STe), 0x00020000 (TT),
 * 0x00030000 (Falcon), 0x00050000 (ARAnyM), for a jar without _MCH, for an
 * empty jar and for no jar. A cookie whose value is 0xFFFFFFFF under another
 * tag does not count.
 *
 * Host must be LP64 little-endian (exit 77 otherwise). Exit status: number
 * of failed checks.
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include <library.h>
#include <mint/ssystem.h>

static long host_setexc(short vnum, long vptr);

#undef Setexc
#define Setexc(vnum, vptr) ((void (*)(void))host_setexc((short)(vnum), (long)(vptr)))

#ifndef COOKIE_C
#error "COOKIE_C (quoted path of cookie.c) not defined"
#endif
#ifndef NOHARD_H
#error "NOHARD_H (quoted path of nohard.h under test) not defined"
#endif
#include COOKIE_C
#include NOHARD_H

static uint32_t jar_buf[2 * 16] __attribute__((aligned(8)));
static uint32_t *jar_ptr;

static long host_setexc(short vnum, long vptr)
{
	(void)vnum;
	(void)vptr;
	return (long)(uintptr_t)jar_ptr;
}

#define TAG_NTOS 0x4E544F53u
#define TAG_MCH  0x5F4D4348u
#define TAG_IDT  0x5F494454u

/* jar: NTOS, [pad,] _MCH = mch (or not at all), _IDT = 0xFFFFFFFF, terminator */
static void fill(int with_mch, uint32_t mch, int odd_slot)
{
	unsigned n = 0;

	memset(jar_buf, 0, sizeof(jar_buf));
	jar_buf[2 * n] = TAG_NTOS; jar_buf[2 * n + 1] = 0x100; n++;
	if (!odd_slot) {
		jar_buf[2 * n] = 0x5A305A30u; jar_buf[2 * n + 1] = 0; n++;
	}
	if (with_mch) {
		jar_buf[2 * n] = TAG_MCH; jar_buf[2 * n + 1] = mch; n++;
	}
	jar_buf[2 * n] = TAG_IDT; jar_buf[2 * n + 1] = 0xFFFFFFFFu; n++;
	jar_buf[2 * n] = 0; jar_buf[2 * n + 1] = 16;
	jar_ptr = jar_buf;
}

static int failed, checks;

static void check(int got, int want, const char *what)
{
	checks++;
	if (got != want) {
		failed++;
		printf("FAIL %s: got %d want %d\n", what, got, want);
	}
}

int main(void)
{
	static const struct { uint32_t mch; const char *name; } atari[] = {
		{ 0x00000000u, "_MCH ST" },
		{ 0x00010000u, "_MCH STe" },
		{ 0x00020000u, "_MCH TT" },
		{ 0x00030000u, "_MCH Falcon" },
		{ 0x00050000u, "_MCH ARAnyM" },
	};
	unsigned i;

	if (sizeof(long) != 8 || sizeof(void *) != 8) {
		printf("nohard-host: skipped -- host is not LP64\n");
		return 77;
	}
	{
		const uint32_t one = 1;
		if (*(const unsigned char *)&one != 1) {
			printf("nohard-host: skipped -- host is not little-endian\n");
			return 77;
		}
	}

	fill(1, 0xFFFFFFFFu, 1);
	check(newtos_mch_nohard(), TRUE, "_MCH NOHARD (odd slot)");
	fill(1, 0xFFFFFFFFu, 0);
	check(newtos_mch_nohard(), TRUE, "_MCH NOHARD (even slot)");

	for (i = 0; i < sizeof(atari) / sizeof(atari[0]); i++) {
		fill(1, atari[i].mch, 1);
		check(newtos_mch_nohard(), FALSE, atari[i].name);
		fill(1, atari[i].mch, 0);
		check(newtos_mch_nohard(), FALSE, atari[i].name);
	}

	fill(0, 0, 1);
	check(newtos_mch_nohard(), FALSE, "no _MCH (_IDT = 0xFFFFFFFF present)");

	memset(jar_buf, 0, sizeof(jar_buf));
	jar_buf[1] = 16;
	jar_buf[2] = TAG_MCH; jar_buf[3] = 0xFFFFFFFFu;     /* behind the terminator */
	jar_ptr = jar_buf;
	check(newtos_mch_nohard(), FALSE, "empty jar");

	jar_ptr = NULL;
	check(newtos_mch_nohard(), FALSE, "no jar");

	printf("nohard-host: %d of %d checks failed\n", failed, checks);
	return failed;
}
