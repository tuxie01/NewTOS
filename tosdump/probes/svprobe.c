/*
 * svprobe.c -- the system variables TASK/BIOS20 serves as EmuTOS does
 * (docs/api/bios/_sysvars-counters-contract.md, section 17.6, cases C01-C14).
 * A permanent probe of the tosdump tree like hkprobe.c: it calls through
 * page-0 vectors and writes page-0 cells, which DUMPBIOS does not do, and
 * writes a text log, no record file.
 *
 * Same C source on both sides (m68k EmuTOS on Hatari, AArch64 NewTOS), mintlib
 * bindings as a ported TOS program calls them.  Built by `make -C tosdump
 * svprobe-m68k` (SVPROBE.PRG) and `make -C tosdump svprobe-newtos`
 * (SVPROBE.NAP, tosdump/build-svprobe-newtos.sh); run by
 * tosdump/probes/run-svprobe.sh.  Output: SVPROBE.LOG in the current
 * directory, written once at the end.
 *
 * Every cell lies at an address 2 mod 4, which Setexc cannot read; all reads
 * and writes of page 0 run in a Supexec callback (on EmuTOS a user-state read
 * below $800 is a bus error).  Address cells are printed as classes
 * (nonzero, equal to another cell), never as numbers: the stub, ROM and buffer
 * addresses differ by design between the sides.
 *
 *   C01 _timer_ms ($442) WORD                    (EmuTOS 20)
 *   C02 _v_bas_ad ($44E) after Logbase(): nonzero, == Logbase()
 *   C03 Setscreen(L,-1,-1): cell == L, Logbase() == L, Physbase() unchanged,
 *       then restored.  L: NewTOS Logbase() + VgetSize(-1) (its second screen
 *       page; a one-page display refuses it, moved=0); EmuTOS a Malloc'ed
 *       256-aligned 640-KiB buffer (any address is stored there).
 *   C04 _vblsem ($452) WORD 1                    C05 _nvbls ($454) WORD 8
 *   C06 _vblqueue ($456) and its eight slots: the cell == $4CE, slot 0 and the
 *       count of non-NULL slots (EmuTOS: vb_draw in slot 0 after the VDI)
 *   C07 a = _frclock; Vsync(); b = _frclock: b - a >= 1; the same for _vbclock;
 *       and _frclock - _vbclock after boot
 *   C08 _vblsem = 0; Vsync() x3; deltas of _vbclock and _frclock; _vblsem = 1
 *   C09 _frclock in the parent; a child (this program, Pexec 0 with the tail
 *       "CHILD") runs Vsync() x3; the parent reads again: strictly larger
 *   C09b h0 = _hz_200, f0 = _frclock; 30 x Vsync(); 100 * (h1-h0)/(f1-f0)
 *       (200 Hz over the frame rate: ~333 at 60 Hz, ~400 at 50 Hz)
 *   C10 hdv_init ($46A), dump_vec ($502), aux_stat ($50E), aux_vec ($512)
 *       against prt_stat ($506)/prt_vec ($50A); a call through each returns.
 *       hdv_init is not called on EmuTOS (flop_hdv_init re-detects floppies).
 *   C11 hdv_boot ($47A) nonzero; NewTOS calls it (returns 4); EmuTOS not (it
 *       would try to boot)
 *   C12 _themd ($48E) 4 LONGs as classes: m_link zero, m_start == membot,
 *       m_length == memtop - membot, m_own zero
 *   C13 _dskbufp ($4C6): nonzero, 4-aligned; 1024 bytes read, written with a
 *       pattern, read back and restored
 *   C14 _pun_ptr ($516) zero or not (NewTOS 0, TASK/BIOS18.md)
 */
#include <osbind.h>
#include <stdarg.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static char out[4096];
static int olen;

static void emit(const char *fmt, ...)
{
	va_list ap;

	va_start(ap, fmt);
	if (olen < (int)sizeof(out) - 160)
		olen += vsnprintf(out + olen, sizeof(out) - olen, fmt, ap);
	va_end(ap);
}

#define CELL(a) (*(volatile uint32_t *)(uintptr_t)(a))
#define CELLW(a) (*(volatile int16_t *)(uintptr_t)(a))

#ifdef __NEWTOS__
#define SELF "C:\\SVPROBE.NAP"
#else
#define SELF "C:\\SVPROBE.PRG"
#endif

/* ---- Supexec callbacks ---------------------------------------------------- */

static volatile uint32_t v[16];
static volatile int16_t w[4];

static long rd_frames(void)
{
	v[0] = CELL(0x466);
	v[1] = CELL(0x462);
	v[2] = CELL(0x4BA);
	return 0;
}

static long rd_static(void)
{
	w[0] = CELLW(0x442);
	w[1] = CELLW(0x452);
	w[2] = CELLW(0x454);
	v[0] = CELL(0x456);
	v[1] = CELL(0x46A);
	v[2] = CELL(0x47A);
	v[3] = CELL(0x502);
	v[4] = CELL(0x506);
	v[5] = CELL(0x50A);
	v[6] = CELL(0x50E);
	v[7] = CELL(0x512);
	v[8] = CELL(0x4C6);
	v[9] = CELL(0x516);
	v[10] = CELL(0x432); /* membot */
	v[11] = CELL(0x436); /* memtop */
	return 0;
}

static uint32_t themd[4];

static long rd_themd(void)
{
	int i;

	for (i = 0; i < 4; i++)
		themd[i] = CELL(0x48E + 4 * i);
	return 0;
}

static uint32_t vbl_slots[8];

static long rd_vblq(void)
{
	uint32_t q = CELL(0x456);
	int16_t n = CELLW(0x454);
	int i;

	for (i = 0; i < 8; i++)
		vbl_slots[i] = (q != 0 && i < n) ? CELL(q + 4 * i) : 0xFFFFFFFFUL;
	return 0;
}

static long rd_vbas(void)
{
	return (long)CELL(0x44E);
}

static long sem_down(void)
{
	CELLW(0x452) = 0;
	return 0;
}

static long sem_up(void)
{
	CELLW(0x452) = 1;
	return 0;
}

static long call_ret_cells(void)
{
	((void (*)(void))(uintptr_t)CELL(0x502))();
	((void (*)(void))(uintptr_t)CELL(0x50E))();
	((void (*)(void))(uintptr_t)CELL(0x512))();
#ifdef __NEWTOS__
	((void (*)(void))(uintptr_t)CELL(0x46A))();
#endif
	return 0x12345678L;
}

#ifdef __NEWTOS__
static long call_hdv_boot(void)
{
	return ((long (*)(void))(uintptr_t)CELL(0x47A))();
}
#endif

static int dsk_read_ok, dsk_write_ok;
static unsigned char dsk_save[1024];

static long dskbuf_rw(void)
{
	volatile unsigned char *p = (volatile unsigned char *)(uintptr_t)CELL(0x4C6);
	int i, ok = 1;

	if (p == 0)
		return -1;
	for (i = 0; i < 1024; i++)
		dsk_save[i] = p[i];
	dsk_read_ok = 1;
	for (i = 0; i < 1024; i++)
		p[i] = (unsigned char)(i * 7 + 3);
	for (i = 0; i < 1024; i++)
		if (p[i] != (unsigned char)(i * 7 + 3))
			ok = 0;
	for (i = 0; i < 1024; i++)
		p[i] = dsk_save[i];
	dsk_write_ok = ok;
	return 0;
}

/* ---- the cases ------------------------------------------------------------ */

static void frames_now(uint32_t *f, uint32_t *vb, uint32_t *hz)
{
	Supexec(rd_frames);
	*f = v[0];
	*vb = v[1];
	if (hz)
		*hz = v[2];
}

static void cases_static(void)
{
	int i, nonnull = 0;

	Supexec(rd_static);
	emit("C01 timer_ms=%d\r\n", (int)w[0]);
	emit("C04 vblsem=%d\r\n", (int)w[1]);
	emit("C05 nvbls=%d\r\n", (int)w[2]);
	Supexec(rd_vblq);
	for (i = 0; i < 8; i++)
		if (vbl_slots[i] != 0)
			nonnull++;
	emit("C06 vblqueue=%s slot0=%s nonnull=%d\r\n",
	     v[0] == 0x4CEUL ? "4ce" : (v[0] == 0 ? "zero" : "other"),
	     vbl_slots[0] == 0 ? "null" : "set", nonnull);
	emit("C10 hdv_init nonzero=%d eq_prt_stat=%d dump_vec eq=%d aux_stat eq=%d aux_vec eq=%d"
	     " prt_stat nonzero=%d eq_prt_vec=%d\r\n",
	     v[1] != 0, v[1] == v[4], v[3] == v[4], v[6] == v[4], v[7] == v[4],
	     v[4] != 0, v[4] == v[5]);
	emit("C11 hdv_boot nonzero=%d eq_prt_stat=%d\r\n", v[2] != 0, v[2] == v[4]);
	emit("C13 dskbufp nonzero=%d aligned4=%d\r\n", v[8] != 0, (v[8] & 3) == 0);
	emit("C14 pun_ptr nonzero=%d\r\n", v[9] != 0);
	Supexec(rd_themd);
	emit("C12 themd link_zero=%d start_eq_membot=%d len_eq_memtop_minus_membot=%d own_zero=%d\r\n",
	     themd[0] == 0, themd[1] == v[10], themd[2] == v[11] - v[10], themd[3] == 0);
}

static void cases_calls(void)
{
	uint32_t c502, c50e, c512;

	Supexec(rd_static);
	c502 = v[3];
	c50e = v[6];
	c512 = v[7];
	if (c502 != 0 && c50e != 0 && c512 != 0)
		emit("C10 calls ret=%08lx\r\n", (unsigned long)Supexec(call_ret_cells));
	else
		emit("C10 calls not made (a cell is 0)\r\n");
#ifdef __NEWTOS__
	if (v[2] != 0)
		emit("C11 call hdv_boot ret=%ld\r\n", (long)Supexec(call_hdv_boot));
	else
		emit("C11 call hdv_boot not made (cell 0)\r\n");
#else
	emit("C11 call hdv_boot not made (EmuTOS would try to boot)\r\n");
#endif
	if (Supexec(dskbuf_rw) == 0)
		emit("C13 read1024=%d write_restore1024=%d\r\n", dsk_read_ok, dsk_write_ok);
	else
		emit("C13 not accessed (cell 0)\r\n");
}

static void cases_screen(void)
{
	long lb, pb, cell, l, lb2, pb2;
	int moved;
#ifndef __NEWTOS__
	long buf;
#endif

	lb = (long)Logbase();
	pb = (long)Physbase();
	cell = Supexec(rd_vbas);
	emit("C02 v_bas_ad nonzero=%d eq_logbase=%d\r\n", cell != 0, cell == lb);
#ifdef __NEWTOS__
	/* VgetSize(VsetMode(-1)): the size of one screen page. */
	l = lb + trap_14_ww((short)0x5B, (short)trap_14_ww((short)0x58, (short)-1));
#else
	/* Large enough for the oracle's largest screen (640 x 480 x 16 bit): the
	 * VBL and the mouse draw through v_bas_ad while it points here. */
	buf = (long)Malloc(655360L);
	if (buf <= 0) {
		emit("C03 not run (no buffer)\r\n");
		return;
	}
	l = (buf + 255L) & ~255L;
#endif
	Setscreen(l, -1L, -1);
	cell = Supexec(rd_vbas);
	lb2 = (long)Logbase();
	pb2 = (long)Physbase();
	moved = lb2 == l;
	Setscreen(lb, -1L, -1);
	emit("C03 moved=%d cell_eq_L=%d logbase_eq_L=%d cell_eq_logbase=%d physbase_unchanged=%d"
	     " restored=%d\r\n",
	     moved, cell == l, lb2 == l, cell == lb2, pb2 == pb,
	     (long)Logbase() == lb && Supexec(rd_vbas) == lb);
#ifndef __NEWTOS__
	Mfree((void *)buf);
#endif
}

static void cases_frames(void)
{
	uint32_t f0, v0, f1, v1, h0, h1;
	long rc;
	int i;

	frames_now(&f0, &v0, 0);
	emit("C07 boot frclock_minus_vbclock_zero=%d\r\n", f0 == v0);
	Vsync();
	frames_now(&f1, &v1, 0);
	emit("C07 vsync d466_ge1=%d d462_ge1=%d\r\n", f1 - f0 >= 1, v1 - v0 >= 1);

	Supexec(sem_down);
	frames_now(&f0, &v0, 0);
	for (i = 0; i < 3; i++)
		Vsync();
	frames_now(&f1, &v1, 0);
	Supexec(sem_up);
	emit("C08 vblsem0 d466_ge3=%d d462=%lu\r\n", f1 - f0 >= 3, (unsigned long)(v1 - v0));

	frames_now(&f0, &v0, 0);
	rc = Pexec(0, SELF, "\005CHILD", 0L);
	frames_now(&f1, &v1, 0);
	emit("C09 child rc=%ld d466_gt0=%d d466_ge3=%d\r\n", rc, f1 - f0 > 0, f1 - f0 >= 3);

	/* One Vsync first: after the C09 child the parent's own _hz_200 copy is
	 * stale until the next 200 Hz tick over it (NewTOS, per process). */
	Vsync();
	frames_now(&f0, &v0, &h0);
	for (i = 0; i < 30; i++)
		Vsync();
	frames_now(&f1, &v1, &h1);
	emit("C09b d466=%lu d4ba=%lu ratio100=%lu\r\n", (unsigned long)(f1 - f0),
	     (unsigned long)(h1 - h0),
	     f1 != f0 ? (unsigned long)(100UL * (h1 - h0) / (f1 - f0)) : 0UL);
}

int main(int argc, char **argv)
{
	long fh;
	int i;

	if (argc > 1 && strcmp(argv[1], "CHILD") == 0) {
		for (i = 0; i < 3; i++)
			Vsync();
		return 0;
	}
	emit("SVPROBE TASK/BIOS20\r\n");
	cases_static();
	cases_calls();
	cases_screen();
	cases_frames();
	emit("end\r\n");
	fh = Fcreate("SVPROBE.LOG", 0);
	if (fh < 0)
		return 1;
	Fwrite((short)fh, (long)olen, out);
	Fclose((short)fh);
	return 0;
}
