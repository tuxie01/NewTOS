/*
 * mxprobe.c -- MEM02 probe: which pool answers each Mxalloc mode, where the
 * block lies relative to the top of ST-RAM, and whether the calling EL0
 * process can write and read it back.
 *
 * Reference: EmuTOS xmxalloc, references/emutos/bdos/umem.c:234-330
 * (mode &= MX_MODEMASK; -1 = largest block of the selected pool(s), PREF*
 * = the larger of both; MX_STRAM -> pmd only, MX_TTRAM -> pmdalt only,
 * MX_PREFSTRAM -> pmd then pmdalt, MX_PREFTTRAM -> pmdalt then pmd).
 *
 * phystop ($42E) is the top of ST-RAM, ramtop ($5A4) the top of alternate
 * RAM (EmuTOS bios/biosmem.c); the classification below is the one the Ball
 * demo prints (address < phystop -> ST, else alt).
 *
 * TASK/MEM02 acceptance probe (docs/analysis/mem02-mxalloc-pools-2026-10-17.md,
 * first run measurements/measurement-2026-10-17-mem02/). A console program for
 * C:\AUTO\ (runs before the AES); prints one line per check over Cconws and
 * ends in "MXPROBE: PASS" / "MXPROBE: FAIL". Needs alternate RAM (TT pool);
 * the expected answers do not depend on memprot (on/off/strict).
 *
 * TASK/MEM11: tools/make-sd-image.sh puts it into \AUTO\ by default, and it
 * also calls Malloc with a real size and Fgetdta, so every boot of that image
 * with a debug kernel reaches the user-pointer assertions of Malloc, Mxalloc
 * and Fgetdta.
 *
 * Build: sdk/libc/build-libc.sh sdk/libc/tests/mxprobe.c sdk/libc/build/MXPROBE.NCP
 */
#include <mint/basepage.h>
#include <mint/osbind.h>

static void puthex(unsigned long v)
{
	static const char d[] = "0123456789abcdef";
	char b[11];
	int i;

	b[0] = '0';
	b[1] = 'x';
	for (i = 0; i < 8; i++)
		b[2 + i] = d[(v >> ((7 - i) * 4)) & 0xf];
	b[10] = '\0';
	Cconws(b);
}

static int fails;

static void check(const char *what, int ok)
{
	Cconws(ok ? "  ok   " : "  FAIL ");
	Cconws(what);
	Cconws("\r\n");
	if (!ok)
		fails++;
}

static void checkhex(const char *what, unsigned long got, unsigned long want)
{
	Cconws(got == want ? "  ok   " : "  FAIL ");
	Cconws(what);
	Cconws(" got=");
	puthex(got);
	Cconws(" want=");
	puthex(want);
	Cconws("\r\n");
	if (got != want)
		fails++;
}

#define MX_STRAM     0
#define MX_TTRAM     1
#define MX_PREFSTRAM 2
#define MX_PREFTTRAM 3

static unsigned long phystop_v, ramtop_v;

static void kv(const char *k, unsigned long v)
{
	Cconws(" ");
	Cconws(k);
	Cconws("=");
	puthex(v);
}

static const char *pool_of(unsigned long a)
{
	if (!a)
		return "-";
	return a < phystop_v ? "ST" : "alt";
}

/* Write a pattern at the first and last byte and at every 4 KiB step, read it back. */
static int rw_ok(unsigned char *p, long n)
{
	long i;
	for (i = 0; i < n; i += 4096)
		p[i] = (unsigned char)(0x5a ^ (i >> 12));
	p[n - 1] = 0xa5;
	for (i = 0; i < n; i += 4096)
		if (p[i] != (unsigned char)(0x5a ^ (i >> 12)))
			return 0;
	return p[n - 1] == 0xa5;
}

static unsigned long one(const char *name, long amount, int mode, int want_alt, int want_null)
{
	unsigned long p = (unsigned long)Mxalloc(amount, mode);
	int rw = 0;

	Cconws("[MXPROBE] Mxalloc(");
	puthex((unsigned long)amount);
	Cconws(",");
	puthex((unsigned long)(unsigned short)mode);
	Cconws(") ");
	Cconws(name);
	kv("p", p);
	Cconws(" pool=");
	Cconws(pool_of(p));
	if (p) {
		rw = rw_ok((unsigned char *)p, amount);
		Cconws(rw ? " el0-rw=ok" : " el0-rw=MISMATCH");
	}
	Cconws("\r\n");
	if (want_null) {
		check("  -> refused (0)", p == 0);
	} else {
		check("  -> a block", p != 0);
		if (p) {
			check(want_alt ? "  -> from alternate RAM (>= phystop)" : "  -> from ST-RAM (< phystop)",
			      want_alt ? p >= phystop_v : p < phystop_v);
			check("  -> EL0 write/read-back", rw);
		}
	}
	return p;
}

int main(void)
{
	long q[4], q0[4], any;
	unsigned long b[8];
	int i, n = 0;

	phystop_v = *(volatile unsigned int *)0x42EUL;
	ramtop_v = *(volatile unsigned int *)0x5A4UL;
	for (i = 0; i < 4; i++)
		q[i] = (long)Mxalloc(-1L, i);
	any = (long)Malloc(-1L);
	for (i = 0; i < 4; i++)
		q0[i] = q[i];
	Cconws("[MXPROBE]");
	kv("phystop", phystop_v);
	kv("ramtop", ramtop_v);
	Cconws("\r\n[MXPROBE]");
	kv("Mxalloc(-1,ST)", (unsigned long)q[0]);
	kv("Mxalloc(-1,TT)", (unsigned long)q[1]);
	kv("Mxalloc(-1,PREFST)", (unsigned long)q[2]);
	kv("Mxalloc(-1,PREFTT)", (unsigned long)q[3]);
	kv("Malloc(-1)", (unsigned long)any);
	kv("p_flags(+0x28)", *(volatile unsigned int *)((char *)_base + 0x28));
	Cconws("\r\n");
	check("ramtop above phystop (alternate RAM present)", ramtop_v > phystop_v);
	check("Mxalloc(-1,PREFST) == max(ST,TT)", q[2] == (q[0] > q[1] ? q[0] : q[1]));
	check("Mxalloc(-1,PREFTT) == max(ST,TT)", q[3] == (q[0] > q[1] ? q[0] : q[1]));
	checkhex("Mxalloc(-1,4) (unknown mode -> masked to 0)", (unsigned long)Mxalloc(-1L, 4), (unsigned long)q[0]);

	b[n++] = one("MX_STRAM", 0x10000, MX_STRAM, 0, 0);
	b[n++] = one("MX_TTRAM", 0x10000, MX_TTRAM, 1, 0);
	b[n++] = one("MX_PREFSTRAM", 0x10000, MX_PREFSTRAM, 0, 0);
	b[n++] = one("MX_PREFTTRAM", 0x10000, MX_PREFTTRAM, 1, 0);
	b[n++] = one("MX_PREFTTRAM|F_PROT_G", 0x10000, MX_PREFTTRAM | 0x10, 1, 0);
	/* the Ball demo's largest buffer at 1920x1080 (idx/bg: 2025 KiB) */
	b[n++] = one("MX_PREFTTRAM 2025K", 2025L * 1024, MX_PREFTTRAM, 1, 0);
	/* forcing modes refuse what their own pool cannot give */
	q[0] = (long)Mxalloc(-1L, MX_STRAM);
	q[1] = (long)Mxalloc(-1L, MX_TTRAM);
	one("MX_STRAM > ST largest", q[0] + 0x10000, MX_STRAM, 0, 1);
	one("MX_TTRAM > TT largest", q[1] + 0x10000, MX_TTRAM, 0, 1);
	/* the preference falls back to the other pool */
	if (q[1] > q[0] + 0x20000)
		b[n++] = one("MX_PREFSTRAM > ST largest (falls back)", q[0] + 0x10000, MX_PREFSTRAM, 1, 0);

	/* TASK/MEM11: the two other kernel pointers that leave as a LONG, so a
	   boot from \AUTO\ runs their debug assertions too. Malloc with a real
	   size (Malloc(-1) above is a size query, exempt from the check), and
	   Fgetdta before any Fsetdta: the default DTA = &p_cmdlin = basepage +
	   0x80 (EmuTOS bdos/proc.c:375). */
	{
		unsigned long m = (unsigned long)Malloc(0x1000L);

		Cconws("[MXPROBE]");
		kv("Malloc(0x1000)", m);
		Cconws("\r\n");
		check("Malloc(0x1000) -> a block", m != 0);
		if (m)
			Mfree((void *)m);
		checkhex("Fgetdta() == default DTA (basepage + 0x80)",
			 (unsigned long)Fgetdta(), (unsigned long)((char *)_base + 0x80));
	}

	for (i = 0; i < n; i++)
		if (b[i])
			Mfree((void *)b[i]);
	checkhex("all back: Mxalloc(-1,ST) as at start", (unsigned long)Mxalloc(-1L, MX_STRAM), (unsigned long)q0[0]);
	checkhex("all back: Mxalloc(-1,TT) as at start", (unsigned long)Mxalloc(-1L, MX_TTRAM), (unsigned long)q0[1]);
	Cconws(fails ? "[MXPROBE] MXPROBE: FAIL\r\n" : "[MXPROBE] MXPROBE: PASS\r\n");
	return fails;
}
