/*
 * mchprobe.c -- TASK/ARCH06 on-target probe (run from C:\AUTO before the AES).
 *
 * 1. Reads the _MCH cookie the kernel seeds (tos-api/src/cookie.rs
 *    seed_defaults, MCH_NOHARD = 0xFFFFFFFF after ARCH06) the ways an EL0
 *    program can:
 *      V  Ssystem(S_GETCOOKIE, tag, 0)      value form, the whole x0 printed
 *      P  Ssystem(S_GETCOOKIE, tag, &v)     pointer form, v preset to a sentinel
 *      G  mintlib Getcookie(tag, &v)         sdk/libc/newtos-arch/getcookie.c
 *      J  the per-process $5A0 mirror        Setexc(0x5A0/4, -1), 32-bit slots
 *    each for _MCH and for an absent tag (ZZZZ), so the value form's -1 can be
 *    compared with its "not found" -1.
 * 2. Reads what mintlib makes of _MCH: sysinfo(SI_PLATFORM / SI_HW_PROVIDER)
 *    and uname().machine (sdk/libc/overlay/unix/sysinfo.c, SDK03).
 * 3. Built with -DSET_VDO=<value> (the _VDO injection side of the run only):
 *    Ssystem(S_SETCOOKIE, '_VDO', SET_VDO) and the cookie read back, so the
 *    TeraDesk started after the AUTO folder finds _VDO in its $5A0 mirror.
 *    Without SET_VDO nothing is written to the jar.
 *
 * Output: the console (Cconws) and C:\MCHPROBE.LOG; last line "MCHPROBE: done".
 * SCRATCH measurement program of measurements/measurement-2026-10-29-arch06,
 * not part of the SDK.
 */
#include <stdint.h>
#include <stdio.h>
#include <string.h>
#include <mint/osbind.h>
#include <mint/mintbind.h>
#include <mint/cookie.h>
#include <mint/ssystem.h>
#include <sys/systeminfo.h>
#include <sys/utsname.h>

#define T_MCH  0x5F4D4348L
#define T_VDO  0x5F56444FL
#define T_ZZZZ 0x5A5A5A5AL
#define SENTINEL ((long)0x5A5A5A5A5A5A5A5AL)
#define JAR_MAX_SLOTS 16

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

static void read_tag(const char *name, long tag)
{
	char line[200];
	long x0, rc, v;

	x0 = Ssystem(S_GETCOOKIE, tag, 0L);
	snprintf(line, sizeof(line), "V %s Ssystem(S_GETCOOKIE, 0x%08lX, 0) x0 = 0x%016lX (%ld)\r\n",
		 name, (unsigned long)tag, (unsigned long)x0, x0);
	out(line);

	v = SENTINEL;
	rc = Ssystem(S_GETCOOKIE, tag, (long)&v);
	snprintf(line, sizeof(line), "P %s Ssystem(S_GETCOOKIE, 0x%08lX, &v) rc = %ld, v = 0x%016lX\r\n",
		 name, (unsigned long)tag, rc, (unsigned long)v);
	out(line);

	v = SENTINEL;
	rc = Getcookie(tag, &v);
	snprintf(line, sizeof(line), "G %s Getcookie(0x%08lX, &v) rc = %ld (%s), v = 0x%016lX\r\n",
		 name, (unsigned long)tag, rc, rc == C_FOUND ? "C_FOUND" : "C_NOTFOUND",
		 (unsigned long)v);
	out(line);
}

static void dump_mirror(const char *when)
{
	const volatile uint32_t *j =
		(const volatile uint32_t *)(uintptr_t)Setexc(0x5A0 / 4, (void (*)(void))-1);
	char line[160];
	int s;

	snprintf(line, sizeof(line), "J %s: $5A0 mirror at 0x%lX\r\n", when,
		 (unsigned long)(uintptr_t)j);
	out(line);
	if (j == NULL)
		return;
	for (s = 0; s < JAR_MAX_SLOTS; s++) {
		uint32_t t = j[2 * s];
		char ts[5];
		int i;

		for (i = 0; i < 4; i++) {
			unsigned char c = (unsigned char)(t >> (24 - 8 * i));
			ts[i] = (c >= 0x20 && c < 0x7F) ? (char)c : '.';
		}
		ts[4] = '\0';
		snprintf(line, sizeof(line), "J   slot %2d tag 0x%08lX %s value 0x%08lX\r\n", s,
			 (unsigned long)t, ts, (unsigned long)j[2 * s + 1]);
		out(line);
		if (t == 0)
			break;
	}
}

int main(void)
{
	char line[200], buf[64];
	struct utsname u;
	long fh, rc;

	out("MCHPROBE ARCH06 _MCH read-back\r\n");
	read_tag("_MCH", T_MCH);
	read_tag("ZZZZ", T_ZZZZ);
	read_tag("_VDO", T_VDO);

	memset(buf, 0, sizeof(buf));
	rc = sysinfo(SI_PLATFORM, buf, sizeof(buf));
	snprintf(line, sizeof(line), "S sysinfo(SI_PLATFORM) = %ld \"%s\"\r\n", rc, buf);
	out(line);
	memset(buf, 0, sizeof(buf));
	rc = sysinfo(SI_HW_PROVIDER, buf, sizeof(buf));
	snprintf(line, sizeof(line), "S sysinfo(SI_HW_PROVIDER) = %ld \"%s\"\r\n", rc, buf);
	out(line);
	memset(&u, 0, sizeof(u));
	rc = uname(&u);
	snprintf(line, sizeof(line), "U uname() = %ld machine \"%s\" sysname \"%s\"\r\n", rc,
		 u.machine, u.sysname);
	out(line);

#ifdef SET_VDO
	rc = Ssystem(S_SETCOOKIE, T_VDO, (long)(SET_VDO));
	snprintf(line, sizeof(line), "W Ssystem(S_SETCOOKIE, _VDO, 0x%08lX) = %ld\r\n",
		 (unsigned long)(SET_VDO), rc);
	out(line);
	read_tag("_VDO", T_VDO);
	dump_mirror("after S_SETCOOKIE");
#else
	dump_mirror("seeded jar");
#endif

	out("MCHPROBE: done\r\n");
	fh = Fcreate("C:\\MCHPROBE.LOG", 0);
	if (fh >= 0) {
		Fwrite((short)fh, (long)loglen, logbuf);
		Fclose((short)fh);
	}
	return 0;
}
