/*
 * cookie-host.c -- host harness for TeraDesk's find_cookie() jar walk
 * (TASK/SDK04, finding C6; patch sdk/overlays/teradesk/library/utility/cookie.c,
 * patch site SDK04-C6). Built and run by cookie-host.sh beside this file.
 *
 * This file is the translation unit: it includes TeraDesk's library.h the way
 * the -D_TOS_ONLY build does, replaces mintlib's Setexc macro (an svc trap)
 * with host_setexc() below, and then #includes the cookie.c under test
 * (COOKIE_C, a quoted path) unchanged -- cookie.c's own includes are then
 * no-ops behind their guards. find_cookie() takes its Setexc branch
 * (_MINT_ == 0) and walks the jar host_setexc() hands out.
 *
 * The jar is laid out in the wire format the kernel writes into the $5A0
 * mirror (tos-api/src/bdos/process/loader.rs write_cookie_jar): slots of two
 * 32-bit LONGs, tag then value, ended by {0, capacity}. Its content is the
 * jar of the on-target probe (cookie-probe.c): the five seeded cookies plus a
 * value-0 and a non-zero cookie in an odd and an even slot each, then one
 * cookie with bit 31 set in tag and value, the terminator, and behind it a
 * "poison" cookie that a walk which stops at the terminator never reaches.
 *
 * Checks (the m68k contract of find_cookie, read from cookie.c):
 *   - every cookie in the jar: TRUE, *value = its 32-bit value zero-extended
 *     (as Getcookie returns it, sdk/libc/newtos-arch/getcookie.c);
 *   - value == NULL: TRUE, nothing written;
 *   - a tag whose 32-bit pattern is handed over sign-extended (as a 32-bit
 *     LONG constant would be on m68k) is found too;
 *   - absent tag and the poison tag behind the terminator: FALSE, *value
 *     untouched;
 *   - empty jar (terminator in slot 0) and a NULL jar: FALSE;
 *   - Setexc was asked for vector 0x5A0/4 with -1 (read, not write).
 *
 * The host must be LP64 little-endian like the target (macOS arm64 is);
 * otherwise the harness refuses (exit 77). Exit status: number of failed
 * checks.
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
#error "COOKIE_C (quoted path of the cookie.c under test) not defined"
#endif
#include COOKIE_C

#define SLOTS 64                      /* buffer; the jar itself uses the first 16 */
#define SENTINEL ((long)0x5A5A5A5A5A5A5A5AL)

static uint32_t jar_buf[2 * SLOTS] __attribute__((aligned(8)));
static uint32_t *jar_ptr;             /* what Setexc hands out (NULL = no jar) */
static int setexc_calls, setexc_bad_args;

static long host_setexc(short vnum, long vptr)
{
	setexc_calls++;
	if (vnum != 0x5A0 / 4 || vptr != -1)
		setexc_bad_args++;
	return (long)(uintptr_t)jar_ptr;
}

struct ck { const char *name; uint32_t tag, value; };

static const struct ck in_jar[] = {
	{ "NTOS", 0x4E544F53u, 0x00000100u },   /* slot 0 even */
	{ "_MCH", 0x5F4D4348u, 0x00050000u },   /* slot 1 odd  */
	{ "_IDT", 0x5F494454u, 0x0000112Eu },   /* slot 2 even */
	{ "_AKP", 0x5F414B50u, 0x00000000u },   /* slot 3 odd, value 0 */
	{ "EdDI", 0x45644449u, 0x00000001u },   /* slot 4 even */
	{ "Z0Z0", 0x5A305A30u, 0x00000000u },   /* slot 5 odd, value 0 */
	{ "Z1Z1", 0x5A315A31u, 0x00000000u },   /* slot 6 even, value 0 */
	{ "Z2Z2", 0x5A325A32u, 0x12345678u },   /* slot 7 odd */
	{ "Z3Z3", 0x5A335A33u, 0x87654321u },   /* slot 8 even, value bit 31 */
	{ "high", 0xC1C2C3C4u, 0xFFFFFFFFu },   /* slot 9 odd, tag and value bit 31 */
};
#define N_IN (sizeof(in_jar) / sizeof(in_jar[0]))
#define CAPACITY 16u
#define POISON_TAG 0x5053544Eu            /* "PSTN", behind the terminator */

static int failed, checks;

static void check(int ok, const char *what, const char *name, long got, long want)
{
	checks++;
	if (!ok) {
		failed++;
		printf("FAIL %s %s: got 0x%016lX want 0x%016lX\n", what, name,
		       (unsigned long)got, (unsigned long)want);
	}
}

static void fill_jar(void)
{
	unsigned i;

	memset(jar_buf, 0, sizeof(jar_buf));
	for (i = 0; i < N_IN; i++) {
		jar_buf[2 * i] = in_jar[i].tag;
		jar_buf[2 * i + 1] = in_jar[i].value;
	}
	jar_buf[2 * N_IN] = 0;                    /* terminator {0, capacity} in slot 10 */
	jar_buf[2 * N_IN + 1] = CAPACITY;
	jar_buf[2 * N_IN + 2] = POISON_TAG;       /* slot 11: never reached */
	jar_buf[2 * N_IN + 3] = 0xDEADBEEFu;
	jar_buf[2 * N_IN + 4] = POISON_TAG;       /* slot 12: never reached */
	jar_buf[2 * N_IN + 5] = 0xDEADBEEFu;
	jar_ptr = jar_buf;
}

int main(void)
{
	unsigned i;
	long v;
	int r;

	if (sizeof(long) != 8 || sizeof(void *) != 8) {
		printf("cookie-host: skipped -- host is not LP64\n");
		return 77;
	}
	{
		const uint32_t one = 1;
		if (*(const unsigned char *)&one != 1) {
			printf("cookie-host: skipped -- host is not little-endian\n");
			return 77;
		}
	}

	fill_jar();
	for (i = 0; i < N_IN; i++) {
		v = SENTINEL;
		r = find_cookie((long)in_jar[i].tag, &v);
		check(r == TRUE, "found", in_jar[i].name, r, TRUE);
		check(v == (long)(unsigned long)in_jar[i].value, "value", in_jar[i].name,
		      v, (long)(unsigned long)in_jar[i].value);

		r = find_cookie((long)in_jar[i].tag, NULL);
		check(r == TRUE, "found (value NULL)", in_jar[i].name, r, TRUE);
	}

	/* the 32-bit tag pattern sign-extended, as an m68k LONG constant reads */
	v = SENTINEL;
	r = find_cookie((long)(int32_t)0xC1C2C3C4u, &v);
	check(r == TRUE, "found (sign-extended tag)", "high", r, TRUE);
	check(v == 0xFFFFFFFFL, "value (sign-extended tag)", "high", v, 0xFFFFFFFFL);

	v = SENTINEL;
	r = find_cookie(0x58585858L, &v);                 /* XXXX, absent */
	check(r == FALSE, "found", "XXXX", r, FALSE);
	check(v == SENTINEL, "value untouched", "XXXX", v, SENTINEL);

	v = SENTINEL;
	r = find_cookie((long)POISON_TAG, &v);            /* behind the terminator */
	check(r == FALSE, "found", "PSTN (behind terminator)", r, FALSE);
	check(v == SENTINEL, "value untouched", "PSTN", v, SENTINEL);

	/* empty jar: the terminator in slot 0 */
	memset(jar_buf, 0, sizeof(jar_buf));
	jar_buf[1] = CAPACITY;
	jar_buf[2] = in_jar[0].tag;                       /* behind the terminator */
	jar_buf[3] = in_jar[0].value;
	v = SENTINEL;
	r = find_cookie((long)in_jar[0].tag, &v);
	check(r == FALSE, "found (empty jar)", "NTOS", r, FALSE);
	check(v == SENTINEL, "value untouched (empty jar)", "NTOS", v, SENTINEL);

	/* no jar at all */
	jar_ptr = NULL;
	v = SENTINEL;
	r = find_cookie((long)in_jar[0].tag, &v);
	check(r == FALSE, "found (NULL jar)", "NTOS", r, FALSE);
	check(v == SENTINEL, "value untouched (NULL jar)", "NTOS", v, SENTINEL);

	check(setexc_calls > 0 && setexc_bad_args == 0, "Setexc(0x5A0/4, -1) calls (bad args)",
	      "", setexc_bad_args, 0);

	printf("cookie-host: %d of %d checks failed\n", failed, checks);
	return failed;
}
