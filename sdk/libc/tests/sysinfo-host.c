/*
 * sysinfo-host.c -- host harness for mintlib's sysinfo(SI_PLATFORM) and
 * sysinfo(SI_HW_PROVIDER) with the NewTOS overlay (TASK/SDK03). Built and run
 * by sysinfo-host.sh: the overlay sdk/libc/overlay/unix/sysinfo.c (under
 * AddressSanitizer) and sdk/libc/newtos-arch/getcookie.c, both with
 * sysinfo-host-mintbind.h forced in, so Getcookie() takes the target's path:
 * Ssystem(S_GETCOOKIE, tag, &v) into a `long v = 0', four bytes written.
 *
 * This file plays the kernel's cookie jar. Each case runs in its own forked
 * child, because si_platform() caches its answer in file statics. A case
 * passes when both strings and both return values (strlen + 1) match;
 * a crash (the pristine strlen (NULL)) or an ASan report (the pristine
 * platforms[9] read past the table) fails it.
 *
 * Expected names: the families 0-5, Hades and ColdFire are mintlib's own
 * table (sysinfo.c si_platform); 0xFFFFFFFF without ColdFire is "unknown",
 * EmuTOS's machine name for MCH_NOHARD (bios/machine.c:807-821); a family
 * above 5 is mintlib's default arm, "atariclone".
 *
 * Exit status: the number of failed cases; 77 = host not LP64 little-endian.
 */
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

int sh_sysinfo (int command, char *buf, long bufsize);	/* mintlib __sysinfo */

#define SI_PLATFORM	8
#define SI_HW_PROVIDER	9
#define S_GETCOOKIE	8

#define C_hade 0x68616465L	/* mint/cookie.h */
#define C__CF_ 0x5F43465FL
#define C__MCH 0x5F4D4348L

struct cookie { uint32_t tag, value; };

static const struct cookie *jar;	/* the current case's jar, 0-terminated */

/* mintlib/lib.h: the MiNT version from the `MiNT' cookie, which NewTOS does
 * not set (mintlib/main.c), so 0; read only by the SI_SYSNAME/SI_RELEASE
 * arms, not by SI_PLATFORM. */
int __mint = 0;

/* S_GETCOOKIE as tos-api/src/gemdos/ssystem.rs s_getcookie answers a named
 * tag: hit + arg2 -> four bytes (u32) to *arg2, return 0 (E_OK); hit without
 * arg2 -> the value as a signed 32-bit long; miss -> -1 (ERROR). */
long
sh_Ssystem (int mode, long arg1, long arg2)
{
	const struct cookie *c;

	if (mode != S_GETCOOKIE) {
		fprintf (stderr, "unexpected Ssystem(%d)\n", mode);
		_exit (3);
	}
	for (c = jar; c->tag; c++) {
		if (c->tag != (uint32_t) arg1)
			continue;
		if (arg2) {
			memcpy ((void *) arg2, &c->value, 4);
			return 0;
		}
		return (long) (int32_t) c->value;
	}
	return -1;
}

long sh_Super (long s) { (void) s; fprintf (stderr, "unexpected Super\n"); _exit (3); }
long sh_SuperToUser (void *s) { (void) s; fprintf (stderr, "unexpected SuperToUser\n"); _exit (3); }
long sh_Sversion (void) { fprintf (stderr, "unexpected Sversion\n"); _exit (3); }

struct tcase {
	const char *name;
	struct cookie jar[4];
	const char *platform, *provider;
};

static const struct tcase cases[] = {
	/* the known values */
	{ "no _MCH cookie",       { { 0, 0 } },                                   "atarist",    "atari" },
	{ "_MCH ST 0x00000000",   { { C__MCH, 0x00000000 }, { 0, 0 } },           "atarist",    "atari" },
	{ "_MCH STe 0x00010000",  { { C__MCH, 0x00010000 }, { 0, 0 } },           "atariste",   "atari" },
	{ "_MCH MSTe 0x00010010", { { C__MCH, 0x00010010 }, { 0, 0 } },           "atariste",   "atari" },
	{ "_MCH TT 0x00020000",   { { C__MCH, 0x00020000 }, { 0, 0 } },           "ataritt",    "atari" },
	{ "_MCH Falcon 0x00030000", { { C__MCH, 0x00030000 }, { 0, 0 } },         "falcon",     "atari" },
	{ "_MCH 0x00030000 + _CF_ (FireBee)",
	  { { C__CF_, 0 }, { C__MCH, 0x00030000 }, { 0, 0 } },                     "firebee",    "hades" },
	{ "_MCH Milan 0x00040000", { { C__MCH, 0x00040000 }, { 0, 0 } },          "milan",      "milan" },
	{ "_MCH ARAnyM 0x00050000", { { C__MCH, 0x00050000 }, { 0, 0 } },         "aranym",     "aranym" },
	{ "hade cookie (Hades)",  { { C_hade, 0 }, { C__MCH, 0x00030000 }, { 0, 0 } }, "hades", "hades" },
	{ "_MCH 0xFFFFFFFF + _CF_ (ColdFire EVB)",
	  { { C__CF_, 0 }, { C__MCH, 0xFFFFFFFF }, { 0, 0 } },                     "cfevb",      "freescale" },
	/* NewTOS's _MCH: MCH_NOHARD without ColdFire (pristine: strlen (NULL)) */
	{ "_MCH 0xFFFFFFFF (MCH_NOHARD, NewTOS)",
	  { { C__MCH, 0xFFFFFFFF }, { 0, 0 } },                                    "unknown",    "unknown" },
	/* families above 5 (pristine: platforms[9] past the table) */
	{ "_MCH family 6 0x00060000", { { C__MCH, 0x00060000 }, { 0, 0 } },       "atariclone", "unknown" },
	{ "_MCH family 0x7FFF",   { { C__MCH, 0x7FFF0000 }, { 0, 0 } },           "atariclone", "unknown" },
	{ "_MCH family 0x8000",   { { C__MCH, 0x80000000 }, { 0, 0 } },           "atariclone", "unknown" },
};

static int
check (const char *what, int cmd, const char *want)
{
	char buf[64];
	int r;

	memset (buf, 0x55, sizeof buf);
	r = sh_sysinfo (cmd, buf, sizeof buf);
	if (r != (int) strlen (want) + 1 || strcmp (buf, want) != 0) {
		printf ("      %s: got \"%.*s\" (return %d), want \"%s\" (return %d)\n",
			what, (int) sizeof buf, buf, r, want, (int) strlen (want) + 1);
		return 1;
	}
	return 0;
}

static int
run_case (const struct tcase *t)
{
	jar = t->jar;
	/* SI_PLATFORM first: it fills the cache SI_HW_PROVIDER reads. */
	return check ("SI_PLATFORM", SI_PLATFORM, t->platform)
	     | check ("SI_HW_PROVIDER", SI_HW_PROVIDER, t->provider);
}

int
main (void)
{
	unsigned i, fail = 0;
	const uint32_t one = 1;

	if (sizeof (long) != 8 || sizeof (void *) != 8 || *(const uint8_t *) &one != 1) {
		printf ("sysinfo-host: skipped -- host is not LP64 little-endian\n");
		return 77;
	}
	for (i = 0; i < sizeof cases / sizeof cases[0]; i++) {
		int st;
		pid_t pid;

		fflush (stdout);
		pid = fork ();
		if (pid < 0) {
			perror ("fork");
			return 1;
		}
		if (pid == 0) {
			int r = run_case (&cases[i]);
			fflush (stdout);
			_exit (r);
		}
		if (waitpid (pid, &st, 0) < 0) {
			perror ("waitpid");
			return 1;
		}
		if (WIFEXITED (st) && WEXITSTATUS (st) == 0) {
			printf ("ok    %s\n", cases[i].name);
		} else {
			if (WIFSIGNALED (st))
				printf ("FAIL  %s -- killed by signal %d (%s)\n", cases[i].name,
					WTERMSIG (st), strsignal (WTERMSIG (st)));
			else
				printf ("FAIL  %s -- exit %d\n", cases[i].name, WEXITSTATUS (st));
			fail++;
		}
	}
	printf ("sysinfo-host: %u of %u cases failed\n", fail,
		(unsigned) (sizeof cases / sizeof cases[0]));
	return (int) fail;
}
