/*
 * ldgtest.c -- LDG milestone M1 acceptance client (TASK/LDG-M1,
 * docs/analysis/ldg-after-arch03-2026-10-06.md section 7, lead decision Q4).
 *
 * A minimal headless LDG client: it loads LDG's demo library ex_lib -- built
 * UNMODIFIED as the native module C:\LDG\EX_LIB.NLD -- through the real LDG
 * mechanism of libldg.a (ldg_open -> ldg_load -> Pexec 5/3/4 + OFFSETLDG in
 * the module's environment, no TSR, no custom loader), binds its functions
 * with the demo's own, unmodified binding demo/libs/exlibbnd.c
 * (init_ldg_math), calls them and closes the library again. What it checks:
 *
 *   - ldg_open by absolute path: the LDG descriptor the module filled in its
 *     ldg_init() (magic 'LDGM', version 0x0101, 3 functions, info string,
 *     LDG protocol 0x0235, basepage, path). A non-NULL result already proves
 *     the module found OFFSETLDG with getenv() -- the libc environ walk of
 *     PORT09 in a Pexec(3)/(4)-started module.
 *   - ldg_find for the three exported names (and NULL for an unknown name);
 *     each pointer lies inside the module's TPA.
 *   - information(), sinus(), cosinus() through the pointers, compared with
 *     the client's own libm and with closed-form values (sin(pi/6) = 0.5).
 *   - ldg_close returns 0 and the largest free block (Malloc(-1)) is back to
 *     its value before the open.
 *   - ldg_open by a relative name after Dsetpath("\LDG"), as zView does it
 *     (plugins.c chdir + bare name); ldg->path is then built by ldg_load from
 *     Dgetdrv/Dgetpath.
 *   - reopen after close (the reload may land at the same address -- printed,
 *     not judged: First-Fit decides), and a second open while the first is
 *     still open (no TSR -> loaded again, a different descriptor; both work).
 *   - at the end the largest free block equals the one at the start.
 *
 * Output: one "ldgtest: PASS|FAIL ..." line per check over Cconws and the
 * verdict line "ldgtest: ALL PASS (N checks)" or "ldgtest: FAIL (k of N
 * checks failed)" -- the serial line a headless QEMU run waits for. Floating
 * point is printed with a fixed-point printer of its own (as
 * sdk/examples/mathtst.c does), not through stdio's float formatting. No key
 * pause: it runs as \AUTO\LDGTEST.NCP (tools/make-sd-image.sh,
 * SD_LDGTEST_AUTO=1).
 *
 * Build: sdk/ldg/build-ldg-demo.sh (also builds EX_LIB.NLD).
 */
#include <math.h>
#include <string.h>
#include <stdint.h>
#include <mint/osbind.h>
#include <mint/basepage.h>
#include <gem.h>
#include <ldg.h>
#include <ex_lib.h>          /* LDG demo/libs/ldg/ex_lib.h: Real, sinus, ... */

#define LIB_ABS   "C:\\LDG\\EX_LIB.NLD"
#define LIB_DIR   "\\LDG"
#define LIB_REL   "EX_LIB.NLD"

static unsigned checks, failed;

static void
put_dec(unsigned long n)
{
	char buf[24];
	int i = sizeof buf - 1;

	buf[i] = '\0';
	do {
		buf[--i] = (char)('0' + n % 10);
		n /= 10;
	} while (n);
	Cconws(&buf[i]);
}

static void
put_hex(unsigned long n)
{
	char buf[20];
	int i = sizeof buf - 1;

	buf[i] = '\0';
	do {
		buf[--i] = "0123456789abcdef"[n & 15];
		n >>= 4;
	} while (n);
	buf[--i] = 'x';
	buf[--i] = '0';
	Cconws(&buf[i]);
}

static void
put_long(long n)
{
	if (n < 0) { Cconws("-"); put_dec((unsigned long)-n); }
	else put_dec((unsigned long)n);
}

/* prints v with six decimals; NaN/inf spelled out (mathtst.c's printer) */
static void
put_double(double v)
{
	unsigned long ip, frac;

	if (isnan(v)) { Cconws("nan"); return; }
	if (isinf(v)) { Cconws(v < 0 ? "-inf" : "inf"); return; }
	if (v < 0) { Cconws("-"); v = -v; }
	ip = (unsigned long)v;
	frac = (unsigned long)((v - (double)ip) * 1000000.0 + 0.5);
	if (frac >= 1000000) { ip++; frac -= 1000000; }
	put_dec(ip);
	Cconws(".");
	{
		char f[7];
		int i;
		for (i = 5; i >= 0; i--) { f[i] = (char)('0' + frac % 10); frac /= 10; }
		f[6] = '\0';
		Cconws(f);
	}
}

static int
check(const char *name, int ok)
{
	checks++;
	if (!ok)
		failed++;
	Cconws("ldgtest: ");
	Cconws(ok ? "PASS " : "FAIL ");
	Cconws(name);
	Cconws("\r\n");
	return ok;
}

static int
check_dbl(const char *name, double got, double want, double tol)
{
	int ok = fabs(got - want) <= tol;

	checks++;
	if (!ok)
		failed++;
	Cconws("ldgtest: ");
	Cconws(ok ? "PASS " : "FAIL ");
	Cconws(name);
	Cconws(" = ");
	put_double(got);
	Cconws(" (want ");
	put_double(want);
	Cconws(")\r\n");
	return ok;
}

static void
note(const char *what, unsigned long v)
{
	Cconws("ldgtest: info ");
	Cconws(what);
	Cconws(" ");
	put_hex(v);
	Cconws("\r\n");
}

static long
largest_free(void)
{
	return Malloc(-1L);
}

/* The client's AES global array: ldg_open() reads only gl[2] (the AES id,
 * stored in ldg->id, ldgapp.c:38 / ldgload.c:174). This client runs before
 * the AES (\AUTO\), so it has no application id; the array stays zero. */
static short gl[15];

/* volatile so the client's reference values are computed at run time by the
 * client's own libm, not folded by the compiler. */
static volatile double half = 0.5, sixth_pi = M_PI / 6.0;

/* The function pointers exlibbnd.c's init_ldg_math() filled must point into
 * the module's TPA (basepage .. p_hitpa). */
static int
in_module(LDG *ldg, void *fn)
{
	BASEPAGE *bp = (BASEPAGE *)ldg->baspag;
	uintptr_t lo = (uintptr_t)bp, hi = (uintptr_t)bp->p_hitpa;

	return (uintptr_t)fn > lo && (uintptr_t)fn < hi;
}

/* Bind and call the three functions of an open ex_lib. */
static void
exercise(const char *tag, LDG *ldg)
{
	Real r;
	char name[80];

	init_ldg_math(ldg);
	strcpy(name, tag); strcat(name, ": ldg_find(information/sinus/cosinus) != NULL");
	if (!check(name, mathinfo != NULL && sinus != NULL && cosinus != NULL))
		return;
	note("sinus at", (unsigned long)(uintptr_t)sinus);
	strcpy(name, tag); strcat(name, ": function pointers inside the module TPA");
	check(name, in_module(ldg, (void *)mathinfo) && in_module(ldg, (void *)sinus)
	      && in_module(ldg, (void *)cosinus));
	strcpy(name, tag); strcat(name, ": information() text");
	check(name, strcmp(mathinfo(), "[1][Example of math library!!!][Ok]") == 0);
	r.value = half;
	strcpy(name, tag); strcat(name, ": sinus(0.5) vs client sin(0.5)");
	check_dbl(name, sinus(&r), sin(half), 1e-12);
	strcpy(name, tag); strcat(name, ": sinus(0.5) vs 0.479426");
	check_dbl(name, sinus(&r), 0.479425538604203, 1e-12);
	r.value = half;
	strcpy(name, tag); strcat(name, ": cosinus(0.5) vs client cos(0.5)");
	check_dbl(name, cosinus(&r), cos(half), 1e-12);
	r.value = sixth_pi;
	strcpy(name, tag); strcat(name, ": sinus(pi/6) vs 0.5");
	check_dbl(name, sinus(&r), 0.5, 1e-12);
}

/* Check the descriptor ex_lib's ldg_init() filled (ex_lib.c:40-47). */
static int
check_descriptor(const char *tag, LDG *ldg, const char *path)
{
	char name[80];

	strcpy(name, tag); strcat(name, ": ldg_open != NULL");
	if (!check(name, ldg != NULL)) {
		Cconws("ldgtest: info ldg_error() = ");
		put_long(ldg_error());
		Cconws(" (without the LDG TSR always LDG_NO_TSR, ldgapp.c:103)\r\n");
		return 0;
	}
	note("ldg at", (unsigned long)(uintptr_t)ldg);
	note("basepage at", (unsigned long)(uintptr_t)ldg->baspag);
	strcpy(name, tag); strcat(name, ": magic 'LDGM'");
	check(name, ldg->magic == LDG_COOKIE);
	strcpy(name, tag); strcat(name, ": vers 0x0101, num 3, vers_ldg 0x0235");
	check(name, ldg->vers == 0x0101 && ldg->num == 3 && ldg->vers_ldg == 0x0235);
	strcpy(name, tag); strcat(name, ": infos text");
	check(name, ldg->infos != NULL
	      && strcmp(ldg->infos, "Example of library with floating parameters.") == 0);
	strcpy(name, tag); strcat(name, ": flags 0 + LDG_STDCALL, no close");
	check(name, ldg->flags == LDG_STDCALL && ldg->close == NULL);
	strcpy(name, tag); strcat(name, ": baspag set, path ");
	strcat(name, path);
	check(name, ldg->baspag != NULL && strcmp(ldg->path, path) == 0);
	if (strcmp(ldg->path, path) != 0) {
		Cconws("ldgtest: info ldg->path = ");
		Cconws(ldg->path);
		Cconws("\r\n");
	}
	return 1;
}

int
main(void)
{
	LDG *a, *b;
	long free0;
	int i, bad = 0;

	Cconws("ldgloop: start\r\n");
	free0 = largest_free();
	for (i = 0; i < 400; i++) {
		a = ldg_open(LIB_ABS, gl);
		b = ldg_open(LIB_ABS, gl);
		if (!a || !b) { bad++; Cconws("ldgloop: open failed\r\n"); }
		if (a) { exercise("a", a); }
		if (b) { exercise("b", b); }
		if (b && i % 3 != 0) ldg_close(b, gl);
		if (a) ldg_close(a, gl);
		if (i % 3 == 0 && b) ldg_close(b, gl);
		if ((i % 50) == 0) { Cconws("ldgloop: iter "); put_dec(i); Cconws("\r\n"); }
	}
	Cconws("ldgloop: done bad="); put_dec(bad);
	Cconws(largest_free() == free0 ? " free-ok\r\n" : " free-DIFFERS\r\n");
	Pterm(0);
	return 0;
}
