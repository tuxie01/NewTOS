/*
 * xdprobe.c -- files on two drives whose directory entries have the same
 * numbers (TASK/FS31: Fdelete on D: hit the open C:\FSWBENCH.LOG on the Pi,
 * "log last Fwrite=-36 Fclose=-65").  A permanent probe of the tosdump tree
 * like dcprobe.c: GEMDOS and BIOS calls, a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * xdprobe-m68k` (XDPROBE.PRG) and `make -C tosdump xdprobe-newtos`
 * (XDPROBE.NAP, tosdump/build-xdprobe-newtos.sh); run by
 * tosdump/probes/run-xdprobe.sh.
 *
 * The card is the same bytes on both sides: C: and D: are two FAT16
 * partitions of the same geometry and the same root (label, NEWDESK.INF), E:
 * holds the NewTOS program and its log.  EmuTOS' dumper drive is F:.  Every
 * collision case logs the slots (read raw with Rwabs) and, in folders, the
 * start clusters of both entries, so the log itself shows that the numbers
 * were equal: EmuTOS' d_files is per directory node, a node belongs to a
 * drive (bdos/fs.h:219,224), so ixdel (bdos/fsopnclo.c:660-669) cannot see the
 * other drive's file.  This probe measures that, it does not assume it.
 *
 * Cases:
 *   A  Fdelete on D:, own open file on C:, Fwrite and Fclose after it
 *   B  the same, Fclose at once
 *   C  the file on C: only read, Fread after it
 *   D  Fcreate over a file on D:, source open on C:, then the copy
 *   E  Ddelete on D:, own open file on C:
 *   F  folders with the same start cluster on both drives
 *   G  the child deletes on D:, the parent has the file open on C:
 *   H  control: the child deletes the parent's open file on the same drive
 *   I  control: own open file, Fdelete on the same drive, Fclose
 *   J  same-slot files open on both drives, Fdelete on one
 *   K  Frename across folders on D:, same-numbered file open on C:
 *   L  Frename from C: to D:
 *   M  Fdatime set on D:, the same-slot entry on C:
 *   O  two Fsfirst/Fsnext searches, one per drive, alternating
 *   P  Dsetpath on both drives, the same path; relative paths
 *
 * NewTOS' sanctioned deviations stay out of the log's way: no Fwrite or Fread
 * through a handle whose file was deleted (docs/COMPATIBILITY.md, TASK/PROC04),
 * no write through a handle moved by Frename (TASK/FS18).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#ifdef __NEWTOS__
#define SELF "E:\\XDPROBE.NAP"
#define LOG "E:\\XDPROBE.LOG"
#else
#define SELF "F:\\XDPROBE.PRG"
#define LOG "F:\\XDPROBE.LOG"
#endif

#define C 2
#define D 3

static char out[24000];
static int olen;
static unsigned char sec[512] __attribute__((aligned(2)));
static char buf[256];

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 200)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

/* ---- the entries as they are on the medium ------------------------------- */

/* Getbpb: recsiz clsiz clsizb rdlen fsiz fatrec datrec numcl bflags. */
static const unsigned short *bpb(int drv)
{
	return (const unsigned short *)Getbpb(drv);
}

static long rawread(int drv, long rec)
{
	return trap_13_wwlwwwl((short)4, (short)2, (long)sec, (short)1, (short)-1, (short)drv, rec);
}

/*
 * The slot of the 11-byte name in the sector at `rec`, -1 if it is not there;
 * *clust = its start cluster.
 */
static int slot_in(int drv, long rec, const char *name11, int *clust)
{
	int i;

	if (rawread(drv, rec) != 0)
		return -1;
	for (i = 0; i < 16; i++) {
		const unsigned char *e = sec + i * 32;

		if (memcmp(e, name11, 11) == 0) {
			*clust = e[0x1A] | (e[0x1B] << 8);
			return i;
		}
	}
	return -1;
}

static long root_rec(int drv)
{
	const unsigned short *b = bpb(drv);

	return b ? (long)b[6] - (long)b[3] : -1;
}

static long cluster_rec(int drv, int cl)
{
	const unsigned short *b = bpb(drv);

	return b ? (long)b[6] + (long)(cl - 2) * (long)b[1] : -1;
}

/* The precondition of a collision case, logged. */
static void root_slots(const char *id, const char *on_c, const char *on_d)
{
	int a, b, ca = 0, cb = 0;

	a = slot_in(C, root_rec(C), on_c, &ca);
	b = slot_in(D, root_rec(D), on_d, &cb);
	emit("%s slots c=%d d=%d same=%d\r\n", id, a, b, a == b && a >= 0);
}

/* ---- calls --------------------------------------------------------------- */

static long mk(const char *name, const char *data)
{
	long h = Fcreate(name, 0);

	if (h < 0)
		return h;
	Fwrite((short)h, (long)strlen(data), data);
	return Fclose((short)h);
}

static void show(const char *id, const char *name)
{
	long h = Fopen(name, 0);
	long n;

	if (h < 0) {
		emit("%s %s Fopen=%ld\r\n", id, name, h);
		return;
	}
	n = Fread((short)h, (long)sizeof(buf) - 1, buf);
	if (n >= 0)
		buf[n] = 0;
	else
		buf[0] = 0;
	emit("%s %s len=%ld \"%s\"", id, name, n, buf);
	emit(" Fclose=%ld\r\n", Fclose((short)h));
}

/* A handle's number is the machine's business: log only whether it is one. */
static void handle(const char *id, const char *what, long h)
{
	if (h >= 6)
		emit("%s %s handle\r\n", id, what);
	else
		emit("%s %s ret=%ld\r\n", id, what, h);
}

static void ret(const char *id, const char *what, long r)
{
	emit("%s %s ret=%ld\r\n", id, what, r);
}

static long child(const char *cmd)
{
	char tail[16];

	tail[0] = (char)strlen(cmd);
	strcpy(tail + 1, cmd);
	return Pexec(0, SELF, tail, NULL);
}

/* The child's job: its Fdelete's code as a small positive exit code. */
static int child_main(const char *what)
{
	long r;

	if (strcmp(what, "xa") == 0)
		r = Fdelete("D:\\FS31V.DAT");
	else
		r = Fdelete("C:\\FS31A.LOG");
	return r == 0 ? 100 : (int)-r;
}

int main(int argc, char **argv)
{
	long h, h2, r;
	const unsigned short *b;
	unsigned int freed[4], freea[4];
	_DOSTIME td, td2;
	_DTA dc, dd;
	int s1, s2, ign, c1 = 0, c2 = 0;

	if (argc > 1 && argv[1][0] == 'x')
		return child_main(argv[1]);

	r = Drvmap();
	emit("Z01 C: D: E: present=%d\r\n", (r & 0x1C) == 0x1C);
	b = bpb(C);
	emit("Z02 C: Getbpb nz=%d\r\n", b != 0);
	if (b) {
		emit("Z03 C: recsiz=%u clsiz=%u rdlen=%u fsiz=%u fatrec=%u datrec=%u numcl=%u\r\n",
		     b[0], b[1], b[3], b[4], b[5], b[6], b[7]);
	}
	b = bpb(D);
	emit("Z04 D: Getbpb nz=%d\r\n", b != 0);
	if (b) {
		emit("Z05 D: recsiz=%u clsiz=%u rdlen=%u fsiz=%u fatrec=%u datrec=%u numcl=%u\r\n",
		     b[0], b[1], b[3], b[4], b[5], b[6], b[7]);
	}

	/* A: the Pi's line, Fwrite and Fclose after the delete on the other drive. */
	h = Fcreate("C:\\FS31A.LOG", 0);
	handle("A01", "Fcreate C:\\FS31A.LOG", h);
	ret("A02", "Fwrite 6", Fwrite((short)h, 6L, "first "));
	ret("A03", "make D:\\FS31V.DAT", mk("D:\\FS31V.DAT", "victim"));
	root_slots("A04", "FS31A   LOG", "FS31V   DAT");
	ret("A05", "Fdelete D:\\FS31V.DAT", Fdelete("D:\\FS31V.DAT"));
	ret("A06", "Fwrite 6", Fwrite((short)h, 6L, "second"));
	ret("A07", "Fclose", Fclose((short)h));
	show("A08", "C:\\FS31A.LOG");
	ret("A09", "Fopen D:\\FS31V.DAT", Fopen("D:\\FS31V.DAT", 0));
	Fdelete("C:\\FS31A.LOG");

	/* B: Fclose at once. */
	h = Fcreate("C:\\FS31A.LOG", 0);
	ret("B01", "Fwrite 6", Fwrite((short)h, 6L, "first "));
	mk("D:\\FS31V.DAT", "victim");
	root_slots("B02", "FS31A   LOG", "FS31V   DAT");
	ret("B03", "Fdelete D:\\FS31V.DAT", Fdelete("D:\\FS31V.DAT"));
	ret("B04", "Fclose", Fclose((short)h));
	show("B05", "C:\\FS31A.LOG");
	Fdelete("C:\\FS31A.LOG");

	/* C: a reader on C:. */
	mk("C:\\FS31S.DAT", "0123456789");
	mk("D:\\FS31V.DAT", "victim");
	root_slots("C01", "FS31S   DAT", "FS31V   DAT");
	h = Fopen("C:\\FS31S.DAT", 0);
	ret("C02", "Fread 4", Fread((short)h, 4L, buf));
	ret("C03", "Fdelete D:\\FS31V.DAT", Fdelete("D:\\FS31V.DAT"));
	r = Fread((short)h, 16L, buf);
	buf[r > 0 ? r : 0] = 0;
	emit("C04 Fread 16 ret=%ld \"%s\"\r\n", r, buf);
	ret("C05", "Fclose", Fclose((short)h));
	Fdelete("C:\\FS31S.DAT");

	/* D: a copy from C: to D: over an existing file. */
	mk("C:\\FS31S.DAT", "source bytes");
	mk("D:\\FS31T.DAT", "old target");
	root_slots("D01", "FS31S   DAT", "FS31T   DAT");
	h = Fopen("C:\\FS31S.DAT", 0);
	h2 = Fcreate("D:\\FS31T.DAT", 0);
	handle("D02", "Fcreate over D:\\FS31T.DAT", h2);
	r = Fread((short)h, 32L, buf);
	ret("D03", "Fread 32", r);
	ret("D04", "Fwrite", Fwrite((short)h2, r > 0 ? r : 0L, buf));
	ret("D05", "Fclose source", Fclose((short)h));
	ret("D06", "Fclose target", Fclose((short)h2));
	show("D07", "C:\\FS31S.DAT");
	show("D08", "D:\\FS31T.DAT");
	Fdelete("C:\\FS31S.DAT");
	Fdelete("D:\\FS31T.DAT");

	/* E: Ddelete. */
	h = Fcreate("C:\\FS31A.LOG", 0);
	Fwrite((short)h, 6L, "first ");
	ret("E01", "Dcreate D:\\FS31E", Dcreate("D:\\FS31E"));
	root_slots("E02", "FS31A   LOG", "FS31E      ");
	ret("E03", "Ddelete D:\\FS31E", Ddelete("D:\\FS31E"));
	ret("E04", "Fwrite 6", Fwrite((short)h, 6L, "second"));
	ret("E05", "Fclose", Fclose((short)h));
	show("E06", "C:\\FS31A.LOG");
	Fdelete("C:\\FS31A.LOG");

	/* F: folders with the same start cluster. */
	ret("F01", "Dcreate C:\\FS31F", Dcreate("C:\\FS31F"));
	ret("F02", "Dcreate D:\\FS31F", Dcreate("D:\\FS31F"));
	slot_in(C, root_rec(C), "FS31F      ", &c1);
	slot_in(D, root_rec(D), "FS31F      ", &c2);
	emit("F03 start clusters same=%d\r\n", c1 == c2 && c1 >= 2);
	h = Fcreate("C:\\FS31F\\A.LOG", 0);
	Fwrite((short)h, 6L, "first ");
	mk("D:\\FS31F\\V.DAT", "victim");
	s1 = slot_in(C, cluster_rec(C, c1), "A       LOG", &ign);
	s2 = slot_in(D, cluster_rec(D, c2), "V       DAT", &ign);
	emit("F04 slots c=%d d=%d same=%d\r\n", s1, s2, s1 == s2 && s1 >= 0);
	ret("F05", "Fdelete D:\\FS31F\\V.DAT", Fdelete("D:\\FS31F\\V.DAT"));
	ret("F06", "Fwrite 6", Fwrite((short)h, 6L, "second"));
	ret("F07", "Fclose", Fclose((short)h));
	show("F08", "C:\\FS31F\\A.LOG");
	Fdelete("C:\\FS31F\\A.LOG");

	/* G: the child deletes on D:, the parent's file is on C:. */
	h = Fcreate("C:\\FS31A.LOG", 0);
	Fwrite((short)h, 6L, "first ");
	mk("D:\\FS31V.DAT", "victim");
	root_slots("G01", "FS31A   LOG", "FS31V   DAT");
	ret("G02", "Pexec child Fdelete D:\\FS31V.DAT (100 = E_OK, else -code)", child("xa"));
	ret("G03", "Fopen D:\\FS31V.DAT", Fopen("D:\\FS31V.DAT", 0));
	ret("G04", "Fwrite 6", Fwrite((short)h, 6L, "second"));
	ret("G05", "Fclose", Fclose((short)h));
	show("G06", "C:\\FS31A.LOG");
	Fdelete("C:\\FS31A.LOG");
	Fdelete("D:\\FS31V.DAT");

	/* H: control, the same drive: EACCDN. */
	h = Fcreate("C:\\FS31A.LOG", 0);
	Fwrite((short)h, 5L, "bytes");
	ret("H01", "Pexec child Fdelete C:\\FS31A.LOG (100 = E_OK, else -code)", child("xb"));
	ret("H02", "Fclose", Fclose((short)h));
	show("H03", "C:\\FS31A.LOG");
	Fdelete("C:\\FS31A.LOG");

	/* I: control, own file on the same drive. */
	Dfree((void *)freea, D + 1);
	h = Fcreate("D:\\FS31A.LOG", 0);
	Fwrite((short)h, 5L, "bytes");
	ret("I01", "Fdelete D:\\FS31A.LOG", Fdelete("D:\\FS31A.LOG"));
	ret("I02", "Fclose", Fclose((short)h));
	ret("I03", "Fopen D:\\FS31A.LOG", Fopen("D:\\FS31A.LOG", 0));
	Dfree((void *)freed, D + 1);
	emit("I04 free clusters on D: as before=%d\r\n", freea[0] == freed[0]);

	/* J: same slot open on both, delete on one. */
	h = Fcreate("C:\\FS31G.LOG", 0);
	Fwrite((short)h, 4L, "goes");
	h2 = Fcreate("D:\\FS31K.LOG", 0);
	Fwrite((short)h2, 6L, "first ");
	root_slots("J01", "FS31G   LOG", "FS31K   LOG");
	ret("J02", "Fdelete C:\\FS31G.LOG", Fdelete("C:\\FS31G.LOG"));
	ret("J03", "Fclose C:", Fclose((short)h));
	ret("J04", "Fwrite 6 D:", Fwrite((short)h2, 6L, "second"));
	ret("J05", "Fclose D:", Fclose((short)h2));
	show("J06", "D:\\FS31K.LOG");
	Fdelete("D:\\FS31K.LOG");

	/* K: Frename across folders on D:. */
	h = Fcreate("C:\\FS31F\\A.LOG", 0);
	Fwrite((short)h, 5L, "on c ");
	mk("D:\\FS31F\\M.DAT", "moved");
	s1 = slot_in(C, cluster_rec(C, c1), "A       LOG", &ign);
	s2 = slot_in(D, cluster_rec(D, c2), "M       DAT", &ign);
	emit("K01 slots c=%d d=%d same=%d\r\n", s1, s2, s1 == s2 && s1 >= 0);
	ret("K02", "Frename D:\\FS31F\\M.DAT D:\\FS31M.DAT", Frename(0, "D:\\FS31F\\M.DAT", "D:\\FS31M.DAT"));
	ret("K03", "Fwrite 4", Fwrite((short)h, 4L, "more"));
	ret("K04", "Fclose", Fclose((short)h));
	show("K05", "C:\\FS31F\\A.LOG");
	show("K06", "D:\\FS31M.DAT");
	Fdelete("C:\\FS31F\\A.LOG");
	Fdelete("D:\\FS31M.DAT");

	/* L: Frename across drives. */
	mk("C:\\FS31A.DAT", "a");
	ret("L01", "Frename C:\\FS31A.DAT D:\\FS31B.DAT", Frename(0, "C:\\FS31A.DAT", "D:\\FS31B.DAT"));
	show("L02", "C:\\FS31A.DAT");
	ret("L03", "Fopen D:\\FS31B.DAT", Fopen("D:\\FS31B.DAT", 0));
	Fdelete("C:\\FS31A.DAT");

	/* M: Fdatime. */
	mk("C:\\FS31A.DAT", "a");
	mk("D:\\FS31B.DAT", "b");
	root_slots("M01", "FS31A   DAT", "FS31B   DAT");
	h = Fopen("C:\\FS31A.DAT", 0);
	Fdatime(&td, (short)h, 0);
	Fclose((short)h);
	h = Fopen("D:\\FS31B.DAT", 2);
	td2.time = 0x1234;
	td2.date = 0x5678;
	ret("M02", "Fdatime set D:", Fdatime(&td2, (short)h, 1));
	ret("M03", "Fclose", Fclose((short)h));
	h = Fopen("D:\\FS31B.DAT", 0);
	Fdatime(&td2, (short)h, 0);
	Fclose((short)h);
	emit("M04 D: time=%04x date=%04x\r\n", (unsigned)(unsigned short)td2.time, (unsigned)(unsigned short)td2.date);
	h = Fopen("C:\\FS31A.DAT", 0);
	Fdatime(&td2, (short)h, 0);
	Fclose((short)h);
	emit("M05 C: unchanged=%d\r\n", td.time == td2.time && td.date == td2.date);
	Fdelete("C:\\FS31A.DAT");
	Fdelete("D:\\FS31B.DAT");

	/* O: searches. */
	mk("C:\\FS31C1.DAT", "x");
	mk("C:\\FS31C2.DAT", "x");
	mk("D:\\FS31D1.DAT", "x");
	mk("D:\\FS31D2.DAT", "x");
	Fsetdta(&dc);
	r = Fsfirst("C:\\FS31C*.*", 0);
	emit("O01 Fsfirst C: ret=%ld %s\r\n", r, dc.dta_name);
	Fsetdta(&dd);
	r = Fsfirst("D:\\FS31D*.*", 0);
	emit("O02 Fsfirst D: ret=%ld %s\r\n", r, dd.dta_name);
	Fsetdta(&dc);
	r = Fsnext();
	emit("O03 Fsnext C: ret=%ld %s\r\n", r, dc.dta_name);
	Fsetdta(&dd);
	r = Fsnext();
	emit("O04 Fsnext D: ret=%ld %s\r\n", r, dd.dta_name);
	Fdelete("C:\\FS31C1.DAT");
	Fdelete("C:\\FS31C2.DAT");
	Fdelete("D:\\FS31D1.DAT");
	Fdelete("D:\\FS31D2.DAT");

	/* P: current directories. */
	ret("P01", "Dsetpath C:\\FS31F", Dsetpath("C:\\FS31F"));
	Dgetpath(buf, C + 1);
	emit("P02 Dgetpath C: \"%s\"\r\n", buf);
	Dgetpath(buf, D + 1);
	emit("P03 Dgetpath D: \"%s\"\r\n", buf);
	ret("P04", "make D:FS31R.DAT", mk("D:FS31R.DAT", "root of d"));
	show("P05", "D:\\FS31R.DAT");
	ret("P06", "Dsetpath D:\\FS31F", Dsetpath("D:\\FS31F"));
	ret("P07", "make D:FS31I.DAT", mk("D:FS31I.DAT", "folder of d"));
	show("P08", "D:\\FS31F\\FS31I.DAT");
	ret("P09", "Fopen C:\\FS31F\\FS31I.DAT", Fopen("C:\\FS31F\\FS31I.DAT", 0));
	Dgetpath(buf, C + 1);
	emit("P10 Dgetpath C: \"%s\"\r\n", buf);
	Dgetpath(buf, D + 1);
	emit("P11 Dgetpath D: \"%s\"\r\n", buf);
	Dsetpath("C:\\");
	Dsetpath("D:\\");
	Fdelete("D:\\FS31R.DAT");
	Fdelete("D:\\FS31F\\FS31I.DAT");
	ret("P12", "Ddelete C:\\FS31F", Ddelete("C:\\FS31F"));
	ret("P13", "Ddelete D:\\FS31F", Ddelete("D:\\FS31F"));

	h = Fcreate(LOG, 0);
	if (h >= 0) {
		Fwrite((short)h, (long)olen, out);
		Fclose((short)h);
	}
	return 0;
}
