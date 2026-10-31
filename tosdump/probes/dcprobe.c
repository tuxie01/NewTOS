/*
 * dcprobe.c -- what one Dcreate costs (TASK/FS26: "New folder" on the Pi takes
 * about three seconds).  A permanent probe of the tosdump tree like gcprobe.c:
 * it makes GEMDOS and BIOS calls and writes a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * dcprobe-m68k` (DCPROBE.PRG) and `make -C tosdump dcprobe-newtos`
 * (DCPROBE.NAP, tosdump/build-dcprobe-newtos.sh); run by
 * tosdump/probes/run-dcprobe.sh.  The NewTOS build runs unchanged as a GEM
 * program (C:\DCPROBE.NAP, the desktop's #Z autorun) and, copied as
 * C:\AUTO\DCPROBE.NCP, as an AUTO program before the AES -- the extension
 * alone decides (docs/sdk/06-image-and-testing.md).
 *
 * Layers, bottom up (the project's practice: one layer at a time):
 *   Part R  BIOS Rwabs on C: itself: the last record of the partition read
 *           once, then written back unchanged 8 times (one sector each) --
 *           the cost of one single-sector write, the unit a Dcreate is made
 *           of.  The bytes on the card do not change.
 *   Part D  Dcreate C:\FS26D1 .. C:\FS26D5 in the root, one at a time
 *   Part N  Dcreate C:\FS26D1\SUB (parent not the root)
 *   Part X  Ddelete of all six again (the run leaves C: as it found it)
 * Each call is timed with _hz_200 (5 ms; read in a Supexec callback on both
 * sides).  On EmuTOS the probe also counts the sector accesses of each call:
 * it chains hdv_rw ($476), which every BDOS Rwabs reaches (Rwabs -> trap #13
 * -> lrwabs -> protect_wlwwwl(hdv_rw, ...), bios/bios.c:1045-1048,
 * util/miscasm.S:252-270), for exactly that call, and lists each access
 * (R/W, record, count).  NewTOS calls no hdv_rw (ADR-031); there the kernel
 * feature `fs-io-trace` (kernel/src/dbg_fsio.rs) lists the device requests of
 * every call on the serial line, and the probe says so.
 *
 * Output: DCPROBE.LOG in the current directory, written once at the end
 * (EmuTOS: the GEMDOS drive D:, so C: sees only the measured calls; NewTOS:
 * C:\ for the NAP and, measured, for the NCP too).
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static char out[24000];
static int olen;
static unsigned char sec[512] __attribute__((aligned(2)));

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 200)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))

static long hz200(void)
{
	return (long)CELL(0x4BA);
}

/* ---- the hdv_rw counter (EmuTOS only) ----------------------------------- */

#ifndef __NEWTOS__
#define MAXACC 160
struct acc {
	short rw, cnt, rec, dev;
	long lrec;
};
struct acc dcprobe_acc[MAXACC];
volatile long dcprobe_nacc;
volatile unsigned long dcprobe_rw_old;
extern void dcprobe_rw_hook(void);

/*
 * The arguments as protect_wlwwwl lays them (util/miscasm.S:252-270): WORD
 * rw, LONG buf, WORD cnt, WORD recnr, WORD dev, LONG lrecnr, packed (EmuTOS
 * is built with -mshort; this probe is not, so it reads the bytes itself).
 */
__attribute__((used)) void dcprobe_rw_record(const unsigned char *a)
{
	long n = dcprobe_nacc;

	if (n < MAXACC) {
		dcprobe_acc[n].rw = *(const short *)(a + 0);
		dcprobe_acc[n].cnt = *(const short *)(a + 6);
		dcprobe_acc[n].rec = *(const short *)(a + 8);
		dcprobe_acc[n].dev = *(const short *)(a + 10);
		dcprobe_acc[n].lrec = *(const long *)(a + 12);
	}
	dcprobe_nacc = n + 1;
}

#define S2(x) #x
#define S1(x) S2(x)
#define SYM(n) S1(__USER_LABEL_PREFIX__) #n
/* Record, then tail-jump to the previous handler with the stack unchanged. */
__asm__(
	"	.text\n"
	"	.even\n"
	"	.globl	" SYM(dcprobe_rw_hook) "\n"
	SYM(dcprobe_rw_hook) ":\n"
	"	pea	4(sp)\n"
	"	jsr	" SYM(dcprobe_rw_record) "\n"
	"	addq.l	#4,sp\n"
	"	move.l	" SYM(dcprobe_rw_old) ",a0\n"
	"	jmp	(a0)\n");
#endif

/* ---- one measured call --------------------------------------------------- */

enum { OP_DCREATE, OP_DDELETE, OP_RWABS };

static long call(int op, const char *path, int rwflag, long lrec)
{
	switch (op) {
	case OP_DCREATE:
		return Dcreate(path);
	case OP_DDELETE:
		return Ddelete(path);
	default:
		return trap_13_wwlwwwl((short)4, (short)rwflag, (long)sec, (short)1,
		                       (short)-1, (short)2, lrec);
	}
}

/* Run one call, timed, and on EmuTOS with its sector accesses; log it. */
static long measure(const char *id, int op, const char *path, int rwflag, long lrec)
{
	long r, t0, t1;
	const char *what = op == OP_DCREATE ? "Dcreate" : op == OP_DDELETE ? "Ddelete" : "Rwabs";
#ifdef __NEWTOS__
	t0 = Supexec(hz200);
	r = call(op, path, rwflag, lrec);
	t1 = Supexec(hz200);
#else
	long stack = Super(0L);

	dcprobe_nacc = 0;
	dcprobe_rw_old = CELL(0x476);
	t0 = hz200();
	CELL(0x476) = (uint32_t)(uintptr_t)dcprobe_rw_hook;
	r = call(op, path, rwflag, lrec);
	CELL(0x476) = (uint32_t)dcprobe_rw_old;
	t1 = hz200();
	Super(stack);
#endif
	if (op == OP_RWABS)
		emit("%s %s(%d,1,lrec=%ld) ret=%ld ticks=%ld", id, what, rwflag, lrec, r, t1 - t0);
	else
		emit("%s %s(%s) ret=%ld ticks=%ld", id, what, path, r, t1 - t0);
#ifdef __NEWTOS__
	emit(" acc=n/a (kernel fs-io-trace)\r\n");
#else
	{
		long i, n = dcprobe_nacc, nr = 0, nw = 0, sr = 0, sw = 0;

		for (i = 0; i < n && i < MAXACC; i++) {
			if (dcprobe_acc[i].rw & 1) {
				nw++;
				sw += dcprobe_acc[i].cnt;
			} else {
				nr++;
				sr += dcprobe_acc[i].cnt;
			}
		}
		emit(" acc=%ld r=%ld/%ld w=%ld/%ld\r\n", n, nr, sr, nw, sw);
		for (i = 0; i < n && i < MAXACC; i++)
			emit("  %c rec=%ld n=%d dev=%d rwflag=%d\r\n",
			     (dcprobe_acc[i].rw & 1) ? 'W' : 'R',
			     dcprobe_acc[i].rec == -1 ? dcprobe_acc[i].lrec : (long)(unsigned short)dcprobe_acc[i].rec,
			     dcprobe_acc[i].cnt, dcprobe_acc[i].dev, dcprobe_acc[i].rw);
		if (n > MAXACC)
			emit("  +%ld more\r\n", n - MAXACC);
	}
#endif
	return r;
}

int main(void)
{
	static const char *const dirs[] = {
		"C:\\FS26D1", "C:\\FS26D2", "C:\\FS26D3", "C:\\FS26D4", "C:\\FS26D5",
	};
	const unsigned short *b;
	long fh, r, last, t0, t1;
	char id[8];
	int i;
	_DTA dta;

#ifdef __NEWTOS__
	emit("DCPROBE TASK/FS26 side=newtos\r\n");
#else
	emit("DCPROBE TASK/FS26 side=emutos\r\n");
#endif
	/* The geometry: Getbpb(2) = recsiz clsiz clsizb rdlen fsiz fatrec datrec numcl bflags. */
	b = (const unsigned short *)Getbpb(2);
	if (!b) {
		emit("B01 Getbpb(2) = NULL -- no C:, stop\r\n");
		goto done;
	}
	emit("B01 bpb recsiz=%u clsiz=%u clsizb=%u rdlen=%u fsiz=%u fatrec=%u datrec=%u numcl=%u bflags=%u\r\n",
	     b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8]);
	/* The last record of the partition (logical, partition-relative). */
	last = (long)b[6] + (long)b[7] * (long)b[1] - 1;
	/* Log C: in for GEMDOS (an Fsfirst on the root, like any first access). */
	Fsetdta(&dta);
	r = Fsfirst("C:\\*.*", 0x17);
	emit("L01 Fsfirst(C:\\*.*) ret=%ld\r\n", r);

	/* Part R: read the last record once, write it back unchanged 8 times. */
	r = measure("R01", OP_RWABS, "", 0, last);
	if (r == 0) {
		t0 = Supexec(hz200);
		for (i = 0; i < 8; i++) {
			sprintf(id, "R%02d", i + 2);
			if (measure(id, OP_RWABS, "", 1, last) != 0)
				break;
		}
		t1 = Supexec(hz200);
		emit("R10 8 writes ticks=%ld (with logging)\r\n", t1 - t0);
	} else {
		emit("R02 read failed, no write\r\n");
	}

	/* Part D, N, X. */
	t0 = Supexec(hz200);
	for (i = 0; i < 5; i++) {
		sprintf(id, "D%02d", i + 1);
		measure(id, OP_DCREATE, dirs[i], 0, 0);
	}
	t1 = Supexec(hz200);
	emit("D10 5 Dcreate ticks=%ld (with logging)\r\n", t1 - t0);
	measure("N01", OP_DCREATE, "C:\\FS26D1\\SUB", 0, 0);
	measure("X01", OP_DDELETE, "C:\\FS26D1\\SUB", 0, 0);
	for (i = 4; i >= 0; i--) {
		sprintf(id, "X%02d", 6 - i);
		measure(id, OP_DDELETE, dirs[i], 0, 0);
	}
done:
	emit("end\r\n");
	fh = Fcreate("DCPROBE.LOG", 0);
	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}
