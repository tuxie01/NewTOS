/*
 * cookie-probe.c -- on-target probe for cflib's getcookie() (TASK/SDK05).
 *
 * A console program (.NCP, run from C:\AUTO before the AES) linked like every
 * cflib client: crt0 + this file + cflib.a + gemlib.a + libc.a, built by
 *   sdk/cflib/build-cflib.sh sdk/cflib/tests/cookie-probe.c <out>/CKPROBE.NCP
 * so the getcookie() it calls is the archive member cookie.o that QED and
 * every other cflib program links -- not a copy.
 *
 * For each name it asks cflib's getcookie(name, &v) and, as the reference,
 * mintlib's Getcookie(tag, &v) with the tag as a numeric constant (the jar
 * tags of tos-api/src/cookie.rs, `u32::from_be_bytes`; NewTOS seeds NTOS,
 * _MCH, _IDT, _AKP, EdDI). The two must agree on found/not-found and on the
 * value. XXXX is a name that is not in the jar: both must say "not found".
 *
 * The U lines rebuild the tag with the union of upstream cookie.c
 * (sdk/cflib/cflib-21-ce57817.tar.xz, cflib-21/cookie.c, getcookie,
 * __GNUC__ branch) word for word and print the whole 8-byte long the union
 * hands to Getcookie: on little-endian AArch64 the four characters land in the
 * low four bytes in reverse order and the upper four bytes are whatever the
 * union held. Informational: they show the input of the unpatched library,
 * not of the archive this program is linked with.
 *
 * Output: the framebuffer console (Cconws) and C:\CKPROBE.LOG. The last line
 * is "CKPROBE: <n>/<m> agree".
 */
#include <stdio.h>
#include <string.h>
#include <mint/osbind.h>
#include <mint/cookie.h>
#include "cflib.h"

struct probe_case {
	const char *name;    /* the 4-character literal handed to getcookie() */
	long tag;            /* the same name as a numeric jar tag */
};

static const struct probe_case cases[] = {
	{ "NTOS", 0x4E544F53L },
	{ "_IDT", 0x5F494454L },
	{ "_MCH", 0x5F4D4348L },
	{ "_AKP", 0x5F414B50L },
	{ "EdDI", 0x45644449L },
	{ "XXXX", 0x58585858L },     /* not in the jar */
};

static char logbuf[2048];
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

/* Upstream cookie.c's tag construction, word for word (see the header). */
static long upstream_union_tag(const char *cookie)
{
	union
	{
		long as_long;
		char as_char[4];
	} cast;

	cast.as_char[0] = cookie[0];
	cast.as_char[1] = cookie[1];
	cast.as_char[2] = cookie[2];
	cast.as_char[3] = cookie[3];

	return cast.as_long;
}

int main(void)
{
	char line[200];
	unsigned i, agree = 0, n = sizeof(cases) / sizeof(cases[0]);
	long fh;

	out("CKPROBE SDK05 cflib getcookie vs Getcookie\r\n");
	for (i = 0; i < n; i++) {
		long v_cf = 0x5A5A5A5AL, v_ref = 0x5A5A5A5AL;
		int found_cf = getcookie(cases[i].name, &v_cf);
		int rc_ref = Getcookie(cases[i].tag, &v_ref);
		int found_ref = (rc_ref == C_FOUND);
		int ok = (found_cf == found_ref) && (!found_ref || v_cf == v_ref);

		agree += ok;
		snprintf(line, sizeof(line),
			 "P%u %s getcookie found=%d value=0x%08lX | Getcookie(0x%08lX) found=%d value=0x%08lX -> %s\r\n",
			 i + 1, cases[i].name, found_cf, (unsigned long)v_cf,
			 (unsigned long)cases[i].tag, found_ref, (unsigned long)v_ref,
			 ok ? "AGREE" : "DIFFER");
		out(line);
	}
	for (i = 0; i < n; i++) {
		snprintf(line, sizeof(line),
			 "U%u %s upstream union long=0x%016lX (jar tag 0x%08lX)\r\n",
			 i + 1, cases[i].name,
			 (unsigned long)upstream_union_tag(cases[i].name),
			 (unsigned long)cases[i].tag);
		out(line);
	}
	snprintf(line, sizeof(line), "CKPROBE: %u/%u agree\r\n", agree, n);
	out(line);

	fh = Fcreate("C:\\CKPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
	return 0;
}
