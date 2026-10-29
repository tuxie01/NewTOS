/*
 * cookie-host.c -- host harness for cflib's getcookie() tag construction
 * (TASK/SDK05; driver: cookie-host.sh, which compiles the cookie.c under test
 * beside this file).
 *
 * This file plays mintlib's Getcookie(long cookie, long *val): it records the
 * WHOLE long it was handed and answers from a jar holding the tags NewTOS
 * seeds (tos-api/src/cookie.rs `seed_defaults`, values as the kernel sets
 * them), matching the full 64-bit long against the 32-bit tag -- so a tag in
 * the wrong byte order and a tag with undefined upper bytes both miss, as
 * they would in the kernel's lookup if it compared all of x1.
 *
 * Before each call the harness dirties a stack area with 0xA5 bytes, so a
 * construction that leaves part of the long unwritten is less likely to find
 * the zeros a fresh stack page would hide (cookie.c is compiled at -O0 for the
 * same reason: its locals live on that stack). Which stale bytes it meets
 * depends on the host compiler's frame layout; any non-zero byte fails.
 *
 * Cases: the five jar names (getcookie() true, value from the jar, the tag
 * exact); a name not in the jar (false, *p_value 0); p_value NULL; a name
 * with bytes >= 0x80 (tag exact: no sign extension into bits 63..32).
 * Exit status = number of failed checks.
 */
#include <stdio.h>
#include <string.h>

int getcookie(const char *cookie, long *p_value);   /* the cookie.c under test */

#define C_FOUND    0
#define C_NOTFOUND (-1)

struct jar_entry { unsigned long tag; long value; };

static const struct jar_entry jar[] = {
	{ 0x4E544F53UL, 0x00000100L },  /* NTOS */
	{ 0x5F4D4348UL, 0x00050000L },  /* _MCH */
	{ 0x5F494454UL, 0x0000112FL },  /* _IDT */
	{ 0x5F414B50UL, 0x00000000L },  /* _AKP */
	{ 0x45644449UL, 0x00000001L },  /* EdDI */
};

static unsigned long last_tag;
static int calls;

int Getcookie(long cookie, long *val)
{
	unsigned i;

	last_tag = (unsigned long)cookie;
	calls++;
	for (i = 0; i < sizeof(jar) / sizeof(jar[0]); i++) {
		if ((unsigned long)cookie == jar[i].tag) {
			if (val)
				*val = jar[i].value;
			return C_FOUND;
		}
	}
	if (val)
		*val = 0;
	return C_NOTFOUND;
}

static int failed, checks;

static void check(int ok, const char *what, const char *name)
{
	checks++;
	if (!ok) {
		failed++;
		printf("FAIL  %-28s %s\n", name, what);
	}
}

/* Fill the stack below the caller with 0xA5, then call getcookie(). */
static __attribute__((noinline)) void dirty_stack(void)
{
	volatile unsigned char pad[4096];

	memset((void *)pad, 0xA5, sizeof(pad));
	__asm__ volatile("" : : "r"(pad) : "memory");
}

static __attribute__((noinline)) int call_getcookie(const char *name, long *v)
{
	dirty_stack();
	return getcookie(name, v);
}

static void case_found(const char *name, unsigned long tag, long value)
{
	long v = 0x7777L;
	int r;

	calls = 0;
	r = call_getcookie(name, &v);
	check(calls == 1, "Getcookie called once", name);
	check(last_tag == tag, "tag handed to Getcookie", name);
	if (last_tag != tag)
		printf("      got 0x%016lX want 0x%016lX\n", last_tag, tag);
	check(r == 1, "getcookie() returns TRUE", name);
	check(v == value, "*p_value = jar value", name);
}

int main(void)
{
	long v;
	int r;

	if (sizeof(long) != 8 || sizeof(void *) != 8) {
		printf("cookie-host: skipped -- host is not LP64\n");
		return 77;
	}
	{
		unsigned int one = 1;
		if (*(unsigned char *)&one != 1) {
			printf("cookie-host: skipped -- host is not little-endian\n");
			return 77;
		}
	}

	case_found("NTOS", 0x4E544F53UL, 0x00000100L);
	case_found("_MCH", 0x5F4D4348UL, 0x00050000L);
	case_found("_IDT", 0x5F494454UL, 0x0000112FL);
	case_found("_AKP", 0x5F414B50UL, 0x00000000L);
	case_found("EdDI", 0x45644449UL, 0x00000001L);

	/* not in the jar: FALSE, *p_value 0 (Getcookie's miss value) */
	v = 0x7777L;
	r = call_getcookie("XXXX", &v);
	check(last_tag == 0x58585858UL, "tag handed to Getcookie", "XXXX (absent)");
	check(r == 0, "getcookie() returns FALSE", "XXXX (absent)");
	check(v == 0, "*p_value = 0", "XXXX (absent)");

	/* p_value NULL: presence test only (cflib appinit.c: getcookie("MgPC", NULL)) */
	r = call_getcookie("_IDT", NULL);
	check(r == 1, "getcookie(name, NULL) returns TRUE", "_IDT (NULL)");

	/* bytes >= 0x80 stay unsigned: no sign extension into bits 63..32 */
	r = call_getcookie("\xC4\x80" "AB", &v);
	check(last_tag == 0xC4804142UL, "tag handed to Getcookie", "C4 80 41 42");
	if (last_tag != 0xC4804142UL)
		printf("      got 0x%016lX want 0x%016lX\n", last_tag, 0xC4804142UL);
	check(r == 0, "getcookie() returns FALSE", "C4 80 41 42");

	printf("cookie-host: %d of %d checks failed\n", failed, checks);
	return failed;
}
