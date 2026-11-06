/*
 * fswbench.c -- FSWBENCH.NCP, the layer-by-layer SD WRITE bench of TASK/HW08,
 * the sibling of the read bench fsbench.c (TASK/FS08). A measuring tool, not
 * product code. It decides HW05 and HW06 with the same run: where the time of
 * a write goes, and whether a directory sector costs more than a data sector.
 *
 * Runs from the desktop on a TEST card. It writes only on the drive it is
 * given (default D:, or the first key of its menu), inside its own files
 * FSWB*.DAT in that drive's root, which it deletes at the end, and it rewrites
 * directory and FAT sectors of that drive with the bytes they already hold.
 *
 *   H  sector cost, HW06: N single-sector Rwabs of one sector, unchanged bytes,
 *      one call per sample, the whole distribution (min, median, p90, max):
 *      a data sector of the test file, the root directory sector with its
 *      entry, the FAT sector of its first cluster. Twice: right away ("cold"
 *      only for the first sample) and after a pause of 1 s between samples.
 *   R  pattern cost, the same patterns as the user's measurement of this card
 *      in a USB reader on the Mac (TASK/HW08, 2026-11-06), Rwabs into a
 *      contiguous 64-MiB test file, 4 KiB per sample: random over 64 MiB,
 *      random within 256 KiB, the same offset, sequential; and new small files
 *      (Fcreate, 4-KiB Fwrite, Fclose). p50, p90, max, failures with the code.
 *      Against WRITE_BUSY_MS = 1000 ms: this is what decides the timeout
 *      hypothesis, and it is the Pi path, not the reader.
 *   L1 BIOS   Rwabs(write) into the clusters of a test file made first with
 *             Fwrite: no GEMDOS, no FAT, no BDOS buffers. 1 and 128 sectors per
 *             call. Skipped for a size whose clusters are not contiguous.
 *   L2 GEMDOS Fcreate, Fwrite loop (512 B, 4 KiB, 32 KiB, whole file), Fclose,
 *             each timed, with a read-back compare after it.
 *   L3 stdio  fopen("wb"), fwrite in 1 KiB, fclose: what a ported program does
 *             (mintlib; the stream buffer is st_blksize, 32 KiB).
 *   L4 copy   Fopen, Fcreate, Fread + Fwrite of 32 KiB, Fclose twice: a file
 *             copy as the desktop does one, with a read-back compare.
 *
 * Sizes as in fsbench.c: 10 KiB, 100 KiB, 1 MiB, 10 MiB. Nothing is printed
 * while a region is timed. On an sd-diag kernel the driver's own report
 * (Ssystem 2090) closes the log; on other kernels that call answers EINVFN.
 *
 * Clock: as fsbench.c: _hz_200 (5 ms; ticks lost in a long SVC are credited by
 * the kernel), refined by a spin count to the next edge.
 *
 * Output: screen pages with a key wait between them (no key wait with
 * -DFSWB_NOKEY for QEMU runs), and the same text in C:\FSWBENCH.LOG, created
 * before the first measurement and written page by page: on the drive under
 * test a failing card took the log with it (TASK/HW08, Pi run 2026-11-06).
 *
 * Build: sdk/examples/fsbench/build-fswbench.sh.
 */
#include <mint/osbind.h>
#include <mint/mintbind.h>
#include <mint/sysvars.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#define TICK_NS    5000000UL
#define SPIN_LIMIT (1UL << 34)
#define SECT       512
#define MAXSIZE    (10L * 1024 * 1024)
#define SAMPLES    41
#define S_NEWTOS_SDDIAG 2090

/* ------------------------------------------------------------------ clock */

static unsigned long spinL;
static unsigned int t0tick;
static int clock_dead;

static unsigned int hz(void)
{
	return (unsigned int)get_sysvar(_hz_200);
}

static unsigned long spin_edge(unsigned int t, unsigned int *now)
{
	unsigned long n = 0;
	unsigned int v;

	while ((v = hz()) == t) {
		if (++n > SPIN_LIMIT) {
			clock_dead = 1;
			break;
		}
	}
	*now = v;
	return n;
}

static void tstart(void)
{
	unsigned int v;

	spin_edge(hz(), &v);
	t0tick = v;
}

/* Elapsed microseconds since the matching tstart(). */
static unsigned long tstop(void)
{
	unsigned int t1 = hz(), v;
	unsigned long n = spin_edge(t1, &v), rem;

	if (n > spinL)
		n = spinL;
	rem = spinL ? n * TICK_NS / spinL : 0;
	return ((unsigned long)(t1 - t0tick + 1) * TICK_NS - rem) / 1000;
}

static void calibrate(void)
{
	unsigned long c[9], t;
	unsigned int v;
	int i, j;

	spin_edge(hz(), &v);
	for (i = 0; i < 9 && !clock_dead; i++)
		c[i] = spin_edge(v, &v);
	for (i = 1; i < 9; i++)
		for (j = i; j > 0 && c[j - 1] > c[j]; j--) {
			t = c[j];
			c[j] = c[j - 1];
			c[j - 1] = t;
		}
	spinL = c[4];
}

static void wait_ticks(unsigned int n)
{
	unsigned int t = hz();

	while (hz() - t < n && !clock_dead)
		;
}

/* ----------------------------------------------------------------- output */

static char out[64000];
static long olen;
static long logh = -1;
static long logres;
static long pagestart;

static void pr(const char *fmt, ...)
{
	va_list ap;
	int n;

	va_start(ap, fmt);
	n = vsnprintf(out + olen, sizeof out - olen, fmt, ap);
	va_end(ap);
	if (n > 0 && olen + n < (long)sizeof out)
		olen += n;
}

/* Show what was added since the last page, wait for a key. */
static void page(void)
{
	long i;

	for (i = pagestart; i < olen; i++) {
		if (out[i] == '\n')
			Cconout('\r');
		Cconout(out[i]);
	}
	if (logh >= 0)
		logres = Fwrite((short)logh, olen - pagestart, out + pagestart);
	pagestart = olen;
#ifndef FSWB_NOKEY
	Cconws("--- key ---\r\n");
	Cconin();
#endif
}

/* ------------------------------------------------------------------- data */

static unsigned char data[MAXSIZE + SECT] __attribute__((aligned(16)));
static unsigned char back[MAXSIZE + SECT] __attribute__((aligned(16)));
static unsigned char sec[SECT * 128] __attribute__((aligned(16)));

static void fill(void)
{
	long i;
	unsigned int x = 0x12345678u;

	for (i = 0; i < MAXSIZE + SECT; i++) {
		x = x * 1103515245u + 12345u;
		data[i] = (unsigned char)(x >> 16);
	}
}

static char drive = 'D';
static char path[32];

static const char *name(const char *n)
{
	sprintf(path, "%c:\\%s", drive, n);
	return path;
}

static int verify(const char *n, long size)
{
	long fh = Fopen(name(n), 0), r;

	if (fh < 0)
		return 0;
	r = Fread((short)fh, size, back);
	Fclose((short)fh);
	return r == size && memcmp(back, data, size) == 0;
}

/* ------------------------------------------------------------ BIOS layer */

static unsigned short le16(const unsigned char *p)
{
	return (unsigned short)(p[0] | (p[1] << 8));
}

static long rwabs(int rw, void *b, int n, long rec)
{
	int d = drive - 'A';

	if (rec > 32767)
		return trap_13_wwlwwwl((short)4, (short)rw, (long)b, (short)n, (short)-1, (short)d, rec);
	return trap_13_wwlwwwl((short)4, (short)rw, (long)b, (short)n, (short)rec, (short)d, 0L);
}

struct geo {
	long clsiz, datrec, fatrec, rootrec, rootsecs, sectors_per_fat;
};
static struct geo g;

static int get_geo(void)
{
	const short *b = (const short *)Getbpb(drive - 'A');

	if (!b) {
		pr("Getbpb(%c:) = 0\n", drive);
		return 0;
	}
	if (b[0] != SECT) {
		pr("recsiz %d, this tool writes 512-byte sectors only\n", b[0]);
		return 0;
	}
	g.clsiz = b[1];
	g.rootsecs = b[3];
	g.sectors_per_fat = b[4];
	g.fatrec = b[5];           /* the second FAT, EmuTOS BPB: fatrec */
	g.rootrec = b[5] + b[4];
	g.datrec = b[6];
	pr("%c: BPB clsiz %ld rootsecs %ld fat %ld at %ld root at %ld data at %ld\n", drive, g.clsiz,
	   g.rootsecs, g.sectors_per_fat, g.fatrec, g.rootrec, g.datrec);
	return 1;
}

/* The root directory sector with the entry of `n11` (11 bytes, FCB form), the
 * file's first cluster. */
static long find_entry(const char *n11, unsigned int *cluster)
{
	long s;
	int i;

	for (s = 0; s < g.rootsecs; s++) {
		if (rwabs(0, sec, 1, g.rootrec + s) != 0)
			return -1;
		for (i = 0; i < SECT; i += 32)
			if (!memcmp(sec + i, n11, 11)) {
				*cluster = le16(sec + i + 26);
				return g.rootrec + s;
			}
	}
	return -1;
}

static unsigned char fsec[SECT];
static long fsec_have = -1;

static unsigned short fat_entry(unsigned int cl)
{
	long s = g.fatrec + cl / (SECT / 2);

	if (s != fsec_have) {
		if (rwabs(0, fsec, 1, s) != 0)
			return 0;
		fsec_have = s;
	}
	return le16(fsec + (cl % (SECT / 2)) * 2);
}

/* Contiguous clusters from `cl` on, at most `want`. */
static long contiguous(unsigned int cl, long want)
{
	long n = 1;

	fsec_have = -1;
	while (n < want) {
		unsigned short e = fat_entry(cl);

		if (e != cl + 1)
			break;
		cl++;
		n++;
	}
	return n;
}

/* Longest contiguous stretch of the chain from *cl, up to `want` clusters of
 * chain; *cl becomes its first cluster. A file on an otherwise empty drive can
 * still start in a small hole left by the Mac's metadata (Pi run 2026-11-06). */
static long longest_run(unsigned int *cl, long want)
{
	unsigned int cur = *cl, start = cur, best_start = cur;
	long run = 1, best = 1, i;

	fsec_have = -1;
	for (i = 1; i < want; i++) {
		unsigned short e = fat_entry(cur);

		if (e < 2 || e >= 0xFFF0)
			break;
		if (e == cur + 1) {
			run++;
		} else {
			start = e;
			run = 1;
		}
		cur = e;
		if (run > best) {
			best = run;
			best_start = start;
		}
	}
	*cl = best_start;
	return best;
}

static long clrec(unsigned int cl)
{
	return g.datrec + (long)(cl - 2) * g.clsiz;
}

static int cmp_ul(const void *a, const void *b)
{
	unsigned long x = *(const unsigned long *)a, y = *(const unsigned long *)b;

	return x < y ? -1 : x > y;
}

static void sort_ul(unsigned long *v, int n)
{
	int i, j;

	for (i = 1; i < n; i++)
		for (j = i; j > 0 && cmp_ul(&v[j - 1], &v[j]) > 0; j--) {
			unsigned long t = v[j];

			v[j] = v[j - 1];
			v[j - 1] = t;
		}
}

/* HW06: `SAMPLES` rewrites of one sector, unchanged bytes. */
static void sector_cost(const char *label, long rec, unsigned int pause_ticks)
{
	static unsigned long s[SAMPLES];
	static unsigned char one[SECT] __attribute__((aligned(16)));
	unsigned long first;
	int i, bad = 0;

	if (rwabs(0, one, 1, rec) != 0) {
		pr("  %-12s read failed\n", label);
		return;
	}
	for (i = 0; i < SAMPLES; i++) {
		if (pause_ticks)
			wait_ticks(pause_ticks);
		tstart();
		if (rwabs(1, one, 1, rec) != 0)
			bad++;
		s[i] = tstop();
	}
	first = s[0];
	sort_ul(s, SAMPLES);
	pr("  %-12s pause %3u first %7lu us  min %7lu med %7lu p90 %7lu max %7lu  failed %d\n", label,
	   pause_ticks, first, s[0], s[SAMPLES / 2], s[SAMPLES * 9 / 10], s[SAMPLES - 1], bad);
}

static void layer_h(void)
{
	unsigned int cl = 0;
	long dirrec, fh;

	pr("H  sector cost (HW06), %d single-sector Rwabs per line, unchanged bytes\n", SAMPLES);
	fh = Fcreate(name("FSWBH.DAT"), 0);
	if (fh < 0 || Fwrite((short)fh, 65536L, data) != 65536L) {
		pr("  test file failed: %ld\n", fh);
		return;
	}
	Fclose((short)fh);
	dirrec = find_entry("FSWBH   DAT", &cl);
	if (dirrec < 0 || cl < 2) {
		pr("  entry not found\n");
		return;
	}
	pr("  sectors: data %ld dir %ld fat %ld\n", clrec(cl), dirrec, g.fatrec + cl / (SECT / 2));
	sector_cost("data", clrec(cl), 0);
	sector_cost("directory", dirrec, 0);
	sector_cost("fat", g.fatrec + cl / (SECT / 2), 0);
	sector_cost("data", clrec(cl), 200);
	sector_cost("directory", dirrec, 200);
	sector_cost("fat", g.fatrec + cl / (SECT / 2), 200);
	Fdelete(name("FSWBH.DAT"));
}

/* ---------------------------------------------- patterns, R (TASK/HW08) */

#define R_SECS   8                   /* 4 KiB */
#define R_MIB64  (64L * 1024 * 1024)
#define R_N      64

static unsigned int rnd_x = 0x2545F491u;

static unsigned int rnd(void)
{
	rnd_x ^= rnd_x << 13;
	rnd_x ^= rnd_x >> 17;
	rnd_x ^= rnd_x << 5;
	return rnd_x;
}

static void stats(const char *label, unsigned long *v, int n, int bad, long lastcode)
{
	unsigned long over = 0;
	int i;

	for (i = 0; i < n; i++)
		over += v[i] >= 1000000UL;
	sort_ul(v, n);
	pr("  %-22s n %3d p50 %8lu p90 %8lu max %8lu us  >=1s %lu  failed %d last %ld\n", label, n, v[n / 2],
	   v[n * 9 / 10], v[n - 1], over, bad, lastcode);
}

/* `span_secs` = sectors the offsets range over; `seq`: consecutive offsets. */
static void pattern(const char *label, long base, long span_secs, int seq)
{
	static unsigned long s[R_N];
	long code = 0;
	int i, bad = 0, run = 0;

	for (i = 0; i < R_N; i++) {
		long off = seq ? (long)i * R_SECS : (long)(rnd() % (unsigned long)(span_secs / R_SECS)) * R_SECS;
		long r;

		tstart();
		r = rwabs(1, data + (long)i * SECT, R_SECS, base + off);
		s[i] = tstop();
		if (r != 0) {
			bad++;
			code = r;
			if (++run >= 5) {
				/* The driver no longer answers: no point in 64 seconds more. */
				i++;
				break;
			}
		} else {
			run = 0;
		}
	}
	stats(label, s, i, bad, code);
}

static void layer_r(void)
{
	static unsigned long s[R_N];
	unsigned int cl = 0;
	long fh, at = 0, dirrec, have, base, code = 0;
	int i, bad = 0;

	pr("R  patterns against the Mac numbers, 4 KiB per write, Rwabs (WRITE_BUSY_MS = 1000 ms)\n");
	fh = Fcreate(name("FSWBR.DAT"), 0);
	while (fh >= 0 && at < R_MIB64) {
		long r = Fwrite((short)fh, 1048576L, data);

		if (r != 1048576L) {
			pr("  test file: Fwrite = %ld at %ld\n", r, at);
			break;
		}
		at += r;
	}
	if (fh >= 0)
		Fclose((short)fh);
	dirrec = find_entry("FSWBR   DAT", &cl);
	have = (dirrec >= 0 && cl >= 2) ? longest_run(&cl, R_MIB64 / SECT / g.clsiz) * g.clsiz : 0;
	pr("  test file: longest contiguous run %ld of %ld sectors, at cluster %u; patterns on it\n", have,
	   (long)(R_MIB64 / SECT), cl);
	if (have >= 512) {
		base = clrec(cl);
		pattern("random over 64 MiB", base, have, 0);
		pattern("random within 256 KiB", base, 512, 0);
		pattern("same offset", base, R_SECS, 0);
		pattern("sequential", base, have, 1);
	}
	Fdelete(name("FSWBR.DAT"));
	for (i = 0; i < R_N; i++) {
		char n[16];
		long r = 0, c = 0;

		sprintf(n, "FSWBN%02d.DAT", i);
		name(n);
		tstart();
		fh = Fcreate(path, 0);
		if (fh >= 0) {
			r = Fwrite((short)fh, 4096L, data);
			c = Fclose((short)fh);
		}
		s[i] = tstop();
		if (fh < 0 || r != 4096L || c != 0) {
			bad++;
			code = fh < 0 ? fh : (r != 4096L ? r : c);
		}
	}
	stats("new small files", s, R_N, bad, code);
	for (i = 0; i < R_N; i++) {
		char n[16];

		sprintf(n, "FSWBN%02d.DAT", i);
		Fdelete(name(n));
	}
}

/* ------------------------------------------------------- the three layers */

static const long sizes[] = { 10240, 102400, 1048576, 10485760 };
#define NSIZES 4

static void rate(const char *label, long size, unsigned long us)
{
	pr("  %-14s %9ld B %10lu us %7lu KB/s\n", label, size, us,
	   us ? (unsigned long)(size / 1024) * 1000000UL / us : 0);
}

static void layer_1(long size)
{
	unsigned int cl = 0;
	long fh, secs = size / SECT, dirrec, have, i;
	unsigned long us;

	fh = Fcreate(name("FSWBL1.DAT"), 0);
	if (fh < 0 || Fwrite((short)fh, size, data) != size) {
		pr("  L1 test file failed: %ld\n", fh);
		return;
	}
	Fclose((short)fh);
	dirrec = find_entry("FSWBL1  DAT", &cl);
	if (dirrec < 0 || cl < 2) {
		pr("  L1 entry not found\n");
		Fdelete(name("FSWBL1.DAT"));
		return;
	}
	have = contiguous(cl, (secs + g.clsiz - 1) / g.clsiz) * g.clsiz;
	if (have < secs) {
		pr("  L1 not contiguous (%ld of %ld sectors), skipped\n", have, secs);
		Fdelete(name("FSWBL1.DAT"));
		return;
	}
	tstart();
	for (i = 0; i < secs; i++)
		rwabs(1, data + i * SECT, 1, clrec(cl) + i);
	us = tstop();
	rate("L1 Rwabs 1 sec", size, us);
	tstart();
	for (i = 0; i < secs; i += 128)
		rwabs(1, data + i * SECT, secs - i < 128 ? (int)(secs - i) : 128, clrec(cl) + i);
	us = tstop();
	rate("L1 Rwabs 128", size, us);
	Fdelete(name("FSWBL1.DAT"));
}

static void layer_2(long size, long chunk)
{
	unsigned long tc, tw, tx;
	long fh, at = 0;
	char label[32];
	int ok;

	tstart();
	fh = Fcreate(name("FSWBL2.DAT"), 0);
	tc = tstop();
	if (fh < 0) {
		pr("  L2 Fcreate = %ld\n", fh);
		return;
	}
	tstart();
	while (at < size) {
		long n = size - at < chunk ? size - at : chunk;
		long r = Fwrite((short)fh, n, data + at);

		if (r != n) {
			pr("  L2 Fwrite(%ld) = %ld at %ld\n", n, r, at);
			break;
		}
		at += n;
	}
	tw = tstop();
	tstart();
	Fclose((short)fh);
	tx = tstop();
	ok = verify("FSWBL2.DAT", size);
	sprintf(label, "L2 Fwrite %ld", chunk);
	rate(label, size, tw);
	pr("     Fcreate %lu us  Fclose %lu us  read-back %s\n", tc, tx, ok ? "ok" : "DIFFERS");
	Fdelete(name("FSWBL2.DAT"));
}

static void layer_3(long size)
{
	unsigned long us;
	FILE *f;
	long at = 0;
	int ok;

	tstart();
	f = fopen(name("FSWBL3.DAT"), "wb");
	if (f) {
		while (at < size) {
			size_t n = size - at < 1024 ? (size_t)(size - at) : 1024;

			if (fwrite(data + at, 1, n, f) != n)
				break;
			at += n;
		}
		fclose(f);
	}
	us = tstop();
	ok = verify("FSWBL3.DAT", size);
	rate("L3 fwrite 1K", size, us);
	pr("     read-back %s\n", ok ? "ok" : "DIFFERS");
	Fdelete(name("FSWBL3.DAT"));
}

static void layer_4(long size)
{
	static unsigned char cb[32768] __attribute__((aligned(16)));
	unsigned long us;
	long in, o, fh, r = 0;
	int ok;

	fh = Fcreate(name("FSWBSRC.DAT"), 0);
	if (fh < 0 || Fwrite((short)fh, size, data) != size) {
		pr("  L4 source failed: %ld\n", fh);
		return;
	}
	Fclose((short)fh);
	tstart();
	in = Fopen(name("FSWBSRC.DAT"), 0);
	o = Fcreate(name("FSWBDST.DAT"), 0);
	if (in >= 0 && o >= 0) {
		while ((r = Fread((short)in, sizeof cb, cb)) > 0)
			if (Fwrite((short)o, r, cb) != r)
				break;
		Fclose((short)o);
		Fclose((short)in);
	}
	us = tstop();
	ok = verify("FSWBDST.DAT", size);
	rate("L4 copy 32K", size, us);
	pr("     read-back %s\n", ok ? "ok" : "DIFFERS");
	Fdelete(name("FSWBSRC.DAT"));
	Fdelete(name("FSWBDST.DAT"));
}

static void close_log(void)
{
	static char diag[4096];
	long n = trap_1_wwll((short)0x154, (short)S_NEWTOS_SDDIAG, (long)diag, (long)sizeof diag);

	if (n > 0)
		pr("--- driver report (Ssystem %d)\n%.*s", S_NEWTOS_SDDIAG, (int)n, diag);
	else
		pr("--- no driver report: Ssystem %d = %ld (no sd-diag kernel)\n", S_NEWTOS_SDDIAG, n);
	page();
	if (logh >= 0) {
		long c = Fclose((short)logh);

		logh = -1;
		pr("log last Fwrite=%ld Fclose=%ld\n", logres, c);
		page();
	}
}

int main(void)
{
	int s;

#ifndef FSWB_NOKEY
	Cconws("FSWBENCH (TASK/HW08) writes on one drive of a TEST card.\r\nDrive letter (other key: end): ");
	{
		int k = (int)(Cconin() & 0xFF);

		if (k >= 'a' && k <= 'z')
			k -= 32;
		if (k < 'C' || k > 'Z') {
			Cconws("\r\nend\r\n");
			return 0;
		}
		drive = (char)k;
	}
	Cconws("\r\n");
#endif
	calibrate();
	if (clock_dead) {
		Cconws("_hz_200 does not run\r\n");
		return 1;
	}
	fill();
	logh = Fcreate("C:\\FSWBENCH.LOG", 0);
	pr("==== FSWBENCH (TASK/HW08), write path layer by layer, drive %c:, log handle %ld\n", drive, logh);
	pr("clock: _hz_200 + spin, %lu spins/tick\n", spinL);
	if (get_geo()) {
		layer_h();
		layer_r();
	}
	page();
	for (s = 0; s < NSIZES; s++) {
		long size = sizes[s];

		pr("size %ld\n", size);
		if (g.clsiz)
			layer_1(size);
		if (size <= 102400)
			layer_2(size, 512);
		layer_2(size, 4096);
		layer_2(size, 32768);
		layer_2(size, size);
		layer_3(size);
		layer_4(size);
		page();
	}
	close_log();
	Cconws("FSWBENCH done\r\n");
	return 0;
}
