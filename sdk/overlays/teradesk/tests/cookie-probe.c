/*
 * cookie-probe.c -- on-target probe for TeraDesk's find_cookie()
 * (TASK/SDK04, finding C6 of docs/analysis/lp64-tos-struct-survey-2026-10-28.md).
 *
 * A console program (.NCP, run from C:\AUTO before the AES), built by
 *   sdk/overlays/teradesk/tests/build-cookie-probe.sh <out>/TDCOOKIE.NCP [cookie.c]
 * which compiles TeraDesk's library/utility/cookie.c -- the overlay if there is
 * one, else the vendored file, or the file named -- unchanged, with the flags
 * of sdk/apps/build-teradesk.sh (-D_TOS_ONLY, so find_cookie takes its Setexc
 * branch and walks the $5A0 jar itself), and links it with this file. The
 * find_cookie() called here is the one DESKTOP.NAP is built from, not a copy.
 *
 * Steps:
 *  1. Ssystem(S_SETCOOKIE) adds four cookies behind the five the kernel seeds
 *     (tos-api/src/cookie.rs seed_defaults: NTOS, _MCH, _IDT, _AKP, EdDI in
 *     slots 0..4), so the jar has a cookie with value 0 and one with a
 *     non-zero value in an odd and in an even slot each:
 *       Z0Z0 = 0 (slot 5), Z1Z1 = 0 (slot 6),
 *       Z2Z2 = 0x12345678 (slot 7), Z3Z3 = 0x87654321 (slot 8, bit 31 set).
 *  2. The J lines list the jar as the wire format has it: pairs of 32-bit
 *     LONGs from the address Setexc(0x5A0/4, -1) returns, up to the
 *     {0, capacity} terminator (at most 16 slots, the mirror's size).
 *  3. For every seeded and added tag, and for three absent ones (XXXX and
 *     _VDO, CT60 -- two of the tags TeraDesk itself asks for), the P lines
 *     call find_cookie(tag, &v) and, as the reference, mintlib's
 *     Getcookie(tag, &v) (Ssystem S_GETCOOKIE, sdk/libc/newtos-arch/getcookie.c).
 *     Both must agree on found / not found and, when found, on the whole
 *     8-byte value (Getcookie zero-extends the 32-bit LONG). The value
 *     variables start at 0x5A5A5A5A5A5A5A5A so an unwritten value shows.
 *  4. The four added cookies are removed again (S_DELCOOKIE), so the AES and
 *     the desktop started after AUTO see the kernel's seeded jar.
 *
 * Output: the console (Cconws) and C:\TDCOOKIE.LOG. The last line is
 * "TDCOOKIE: <n>/<m> agree".
 */
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <mint/osbind.h>
#include <mint/mintbind.h>
#include <mint/cookie.h>
#include <mint/ssystem.h>

/* TeraDesk's prototype, library/xdialog/library.h (find_cookie). */
bool find_cookie(long name, long *value);

#define JAR_MAX_SLOTS 16   /* the $5A0 mirror window, tos-api loader.rs COOKIE_JAR_SLOTS */
#define SENTINEL ((long)0x5A5A5A5A5A5A5A5AL)

struct probe_case {
	const char *name;
	uint32_t tag;
	int add;               /* 1 = set by this probe (and removed at the end) */
	uint32_t value;        /* value to set when add */
};

static const struct probe_case cases[] = {
	{ "NTOS", 0x4E544F53u, 0, 0 },
	{ "_MCH", 0x5F4D4348u, 0, 0 },
	{ "_IDT", 0x5F494454u, 0, 0 },
	{ "_AKP", 0x5F414B50u, 0, 0 },
	{ "EdDI", 0x45644449u, 0, 0 },
	{ "Z0Z0", 0x5A305A30u, 1, 0x00000000u },
	{ "Z1Z1", 0x5A315A31u, 1, 0x00000000u },
	{ "Z2Z2", 0x5A325A32u, 1, 0x12345678u },
	{ "Z3Z3", 0x5A335A33u, 1, 0x87654321u },
	{ "XXXX", 0x58585858u, 0, 0 },     /* not in the jar */
	{ "_VDO", 0x5F56444Fu, 0, 0 },     /* not in the jar; TeraDesk video.c asks */
	{ "CT60", 0x43543630u, 0, 0 },     /* not in the jar; TeraDesk main.c asks */
};
#define N_CASES (sizeof(cases) / sizeof(cases[0]))

static char logbuf[4096];
static size_t loglen;

static void out(const char *s)
{
	size_t n = strlen(s);

	Cconws(s);
	if (loglen + n < sizeof(logbuf)) {
		memcpy(logbuf + loglen, s, n);
		loglen += n;
	}
}

static void tagstr(uint32_t tag, char *s)
{
	int i;

	for (i = 0; i < 4; i++) {
		unsigned char c = (unsigned char)(tag >> (24 - 8 * i));
		s[i] = (c >= 0x20 && c < 0x7F) ? (char)c : '.';
	}
	s[4] = '\0';
}

/* The jar as the wire format has it: 32-bit tag, 32-bit value per slot. */
static const volatile uint32_t *jar_base(void)
{
	return (const volatile uint32_t *)(uintptr_t)Setexc(0x5A0 / 4, (void (*)(void))-1);
}

static int slot_of(uint32_t tag)
{
	const volatile uint32_t *j = jar_base();
	int i;

	if (j == NULL)
		return -1;
	for (i = 0; i < JAR_MAX_SLOTS && j[2 * i] != 0; i++)
		if (j[2 * i] == tag)
			return i;
	return -1;
}

int main(void)
{
	char line[256], ts[5];
	const volatile uint32_t *j;
	unsigned i, agree = 0;
	long fh, rc;
	int s;

	out("TDCOOKIE SDK04 C6 TeraDesk find_cookie vs Getcookie\r\n");

	for (i = 0; i < N_CASES; i++) {
		if (!cases[i].add)
			continue;
		rc = Ssystem(S_SETCOOKIE, (long)cases[i].tag, (long)cases[i].value);
		snprintf(line, sizeof(line), "S %s Ssystem(S_SETCOOKIE, 0x%08lX, 0x%08lX) = %ld\r\n",
			 cases[i].name, (unsigned long)cases[i].tag,
			 (unsigned long)cases[i].value, rc);
		out(line);
	}

	j = jar_base();
	snprintf(line, sizeof(line), "J jar at 0x%lX (Setexc(0x5A0/4, -1))\r\n",
		 (unsigned long)(uintptr_t)j);
	out(line);
	if (j != NULL) {
		for (s = 0; s < JAR_MAX_SLOTS; s++) {
			tagstr(j[2 * s], ts);
			snprintf(line, sizeof(line), "J slot %2d (%s) tag 0x%08lX %s value 0x%08lX\r\n",
				 s, (s & 1) ? "odd " : "even", (unsigned long)j[2 * s], ts,
				 (unsigned long)j[2 * s + 1]);
			out(line);
			if (j[2 * s] == 0)
				break;
		}
	}

	for (i = 0; i < N_CASES; i++) {
		long v_td = SENTINEL, v_ref = SENTINEL;
		int found_td = find_cookie((long)cases[i].tag, &v_td) ? 1 : 0;
		int found_ref = (Getcookie((long)cases[i].tag, &v_ref) == C_FOUND);
		int ok = (found_td == found_ref) && (!found_ref || v_td == v_ref);
		int sl = slot_of(cases[i].tag);

		agree += ok;
		snprintf(line, sizeof(line),
			 "P%-2u %s slot %2d find_cookie found=%d value=0x%016lX | Getcookie found=%d value=0x%016lX -> %s\r\n",
			 i + 1, cases[i].name, sl, found_td, (unsigned long)v_td,
			 found_ref, (unsigned long)v_ref, ok ? "AGREE" : "DIFFER");
		out(line);
	}

	for (i = 0; i < N_CASES; i++) {
		if (!cases[i].add)
			continue;
		rc = Ssystem(S_DELCOOKIE, (long)cases[i].tag, 0L);
		snprintf(line, sizeof(line), "D %s Ssystem(S_DELCOOKIE) = %ld\r\n", cases[i].name, rc);
		out(line);
	}

	snprintf(line, sizeof(line), "TDCOOKIE: %u/%u agree\r\n", agree, (unsigned)N_CASES);
	out(line);

	fh = Fcreate("C:\\TDCOOKIE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
	return 0;
}
