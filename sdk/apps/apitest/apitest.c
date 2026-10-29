/*
 * apitest.c -- NewTOS console API diagnostic (native EL0, `.NCP`).
 *
 * Runs from C:\AUTO\ (console, BEFORE the AES/desktop). It walks the
 * GEMDOS / BIOS + low-memory-sysvar surface step by step and writes a
 * progress trace to the screen AND a log file, so a real-Pi4 crash/hang
 * localises exactly which svc or memory access broke.
 *
 * Output policy (robustness principle):
 *   - PRIMARY  = the framebuffer console (HDMI). `Cconws` (GEMDOS 0x09) is
 *     wired to `host.console_write_byte`, which renders to BOTH the PL011
 *     serial AND the VT52 framebuffer console (`kernel/src/host_env.rs:17`,
 *     `fbcon::write_byte`). So every trace line appears on the display with
 *     no serial cable.
 *   - SECONDARY = C:\APITEST.LOG, written with a COMMIT-PER-LINE pattern:
 *     Fopen(rw) -> Fseek(SEEK_END) -> Fwrite -> Fclose for EVERY line. On
 *     GEMDOS/FAT the directory entry (file size) is committed on Fclose, NOT
 *     on Fwrite, so closing after each line means a crash after line N leaves
 *     a valid N-line file on the SD card (readable on the Mac). The obvious
 *     "keep one handle open, Fclose at the end" would lose everything on a
 *     mid-run fault -- which is exactly the case this tool exists to catch.
 *   - `emit()` tees to BOTH; `log_commit()` is the durable file half.
 *
 * Each probe emits a BEFORE line, performs the access, then emits an AFTER
 * line with the result. If a probe faults/hangs on real HW, the AFTER line
 * never prints, so the last unpaired BEFORE line on screen (and the last
 * line in the committed log) = the exact failing svc/access. No single
 * failing probe aborts the run: each is independent and unguarded probes
 * only READ (a stale/zero read is the headline result, not a fault).
 *
 * The run ends with a COMPACT SUMMARY: pass/fail/skip per group, then the
 * names of every failed and skipped check, paged (20 lines, "press a key")
 * so it never scrolls away. After it, the program blocks on Cconin()
 * ("press a key") so the verdict stays readable before C:\AUTO returns and
 * the desktop clears the screen. That summary is the deliverable on a real
 * Pi-4, where there is no serial line and the screen is the only output.
 *
 * Groups: GEMDOS (file / DTA round-trips), BIOS-CHR (Bconstat / Bconin /
 * Bconout / Bcostat), BIOS-BLK (Drvmap / Mediach / Rwabs / Getbpb),
 * BIOS-SYS (Tickcal / Getmpb / Setexc / Kbshift), AES. Every BIOS function
 * id the trap-13 dispatcher wires (tos-api/src/bios/mod.rs, 0x00..0x0B) is
 * exercised against docs/api/bios/<id>-<name>.md -- both the documented
 * return value for documented input and the documented error code for a
 * documented error case.
 *
 * THE BIOS GROUP IS READ-ONLY. Rwabs can write; this program never sets the
 * write bit, on any drive, in any mode. It runs from the user's real SD
 * card. Nor does it install anything into a live exception vector: Setexc
 * only QUERIES the etv_* hooks and set/restores a shadow-only slot.
 *
 * Deliberately LOW dependency: it uses only the <mint/osbind.h> trap macros
 * (header-only inline SVC) plus hand-rolled string/format helpers -- NO
 * printf, NO mintlib malloc, NO stdio. The only libc pull is crt0's
 * __libc_init_heap. This keeps the diagnostic itself from depending on the
 * high-surface machinery it is meant to diagnose.
 */

#include <mint/osbind.h>
#include <mint/basepage.h>   /* BASEPAGE (32-bit fields, NewTOS overlay) + _base */

/* Linker symbol (sdk/newtos.ld): first free TPA byte above the zeroed BSS.
   T32 checks it against the basepage's p_bbase + p_blen. */
extern char _end[];

/* ------------------------------------------------------------------ *
 *  raw volatile reads (the classic direct-pointer TOS idiom)          *
 * ------------------------------------------------------------------ */
#define RD_UL(a)  (*(volatile unsigned long  *)(unsigned long)(a)) /* 8-byte */
#define RD_U32(a) (*(volatile unsigned int   *)(unsigned long)(a)) /* 4-byte */
#define RD_U16(a) (*(volatile unsigned short *)(unsigned long)(a)) /* 2-byte */
/* A LONG of our own BasePage. Since ARCH03 stage 1 it sits at the start of
 * our TPA block (the address crt0 hands to __libc_init_heap as `_base`), no
 * longer at VA 0 -- page 0 holds only the system variables now. */
#define BP_U32(off) RD_U32((unsigned long)_base + (off))

static const char LOGPATH[] = "C:\\APITEST.LOG";
static const char CFGPATH[] = "C:\\APITEST.CFG";

/* 1 once C:\APITEST.LOG has been created; enables the commit-per-line tee. */
static int g_log_ok = 0;

/*
 * Batch (non-interactive) mode -- 1 = never block on Cconin().
 *
 * APITEST normally pauses on Cconin() at every summary page break and once
 * more at the very end, so the verdict stays readable on a real Pi-4: there
 * is no serial line there, and the desktop clears the screen the moment
 * C:\AUTO returns. A HEADLESS run (QEMU raspi4b, -display none, and on that
 * machine no keyboard at all -- "[pcie] RC not present ... USB disabled") has
 * nobody to press that key and would hang forever at the first page break. A
 * headless run could then never produce a log, which is exactly what such a
 * run exists for.
 *
 * The switch cannot be a command-line argument: TOS gives a program started
 * from C:\AUTO no command line (its BASEPAGE p_cmdlin is empty), and the
 * NEWDESK.INF `#Z` autorun path does not fill one either. So it has to come
 * from the medium. Batch mode is selected by an optional file
 *
 *     C:\APITEST.CFG   containing the token  BATCH
 *
 * Absent file -- the real-hardware case, and every card built so far --
 * leaves today's interactive behaviour bit for bit unchanged.
 */
static int g_batch = 0;

/* ------------------------------------------------------------------ *
 *  tiny freestanding string/format helpers (no libc)                  *
 * ------------------------------------------------------------------ */
static unsigned slen(const char *s)
{
	unsigned n = 0;
	while (s[n]) n++;
	return n;
}

static int mcmp(const char *a, const char *b, unsigned n)
{
	unsigned i;
	for (i = 0; i < n; i++)
		if (a[i] != b[i]) return 1;
	return 0;
}

/* Durable file tee: append one string to C:\APITEST.LOG and commit the
 * directory entry (Fclose) before returning, so a later crash leaves a
 * valid file. Any GEMDOS error at any step -> silently skip the file half
 * for this line (the screen tee still shows it); logging never aborts the
 * run. GEMDOS has no append mode, so we Fopen read/write then Fseek to the
 * end. */
static void log_commit(const char *s)
{
	long fd, pos;
	if (!g_log_ok) return;
	fd = Fopen(LOGPATH, 2);                 /* S_READWRITE */
	if (fd < 0) return;
	pos = Fseek(0, (short)fd, 2);           /* SEEK_END */
	if (pos < 0) { (void)Fclose((short)fd); return; }
	(void)Fwrite((short)fd, (long)slen(s), (long)s);
	(void)Fclose((short)fd);                /* commits the FAT dir entry */
}

/* Emit a NUL-terminated string to the screen (fbcon+serial via Cconws)
 * AND durably to the log file (commit-per-line). */
static void emit(const char *s)
{
	Cconws(s);                              /* -> host.console_write_byte */
	log_commit(s);
}

/*
 * Read C:\APITEST.CFG and report whether it selects batch mode (see g_batch).
 * The file is optional; any read failure means "interactive", the safe
 * default. The token is searched for anywhere in the first 64 bytes, so the
 * file may carry a comment line or CR/LF in any combination.
 */
static int batch_requested(void)
{
	static char cfg[64];
	long fd, n;
	unsigned i;

	fd = Fopen(CFGPATH, 0);                 /* S_READ */
	if (fd < 0)
		return 0;
	n = Fread((short)fd, (long)sizeof(cfg), (long)cfg);
	(void)Fclose((short)fd);
	if (n < 5)
		return 0;
	for (i = 0; i + 5 <= (unsigned)n; i++)
		if (mcmp(cfg + i, "BATCH", 5) == 0)
			return 1;
	return 0;
}

/* Line builder: build a line with formatted values, then flush via emit(). */
static char LB[512];
static int  LP;

static void b_reset(void) { LP = 0; LB[0] = 0; }

static void b_ch(char c)
{
	if (LP < (int)sizeof(LB) - 1) {
		LB[LP++] = c;
		LB[LP] = 0;
	}
}

static void b_str(const char *s) { while (*s) b_ch(*s++); }

static void b_hex(unsigned long v, int mindigits)
{
	static const char H[] = "0123456789ABCDEF";
	char t[20];
	int n = 0;
	if (v == 0) t[n++] = '0';
	while (v) { t[n++] = H[v & 0xFUL]; v >>= 4; }
	while (n < mindigits) t[n++] = '0';
	b_str("0x");
	while (n) b_ch(t[--n]);
}

static void b_dec(long v)
{
	char t[24];
	int n = 0;
	unsigned long u;
	if (v < 0) { b_ch('-'); u = (unsigned long)(-(v)); }
	else       { u = (unsigned long)v; }
	if (u == 0) t[n++] = '0';
	while (u) { t[n++] = (char)('0' + (int)(u % 10UL)); u /= 10UL; }
	while (n) b_ch(t[--n]);
}

static void b_nl(void)   { b_ch('\r'); b_ch('\n'); }
static void b_emit(void) { emit(LB); }

/* Two raw hex digits (no "0x" prefix), for byte dumps. */
static void b_hh(unsigned char v)
{
	static const char H[] = "0123456789ABCDEF";
	b_ch(H[(v >> 4) & 0xF]);
	b_ch(H[v & 0xF]);
}

/* Append a success verdict: rc >= 0 -> [OK]; rc < 0 -> [FAIL rc=..]. Works
 * for Fopen/Fread (handle/count >= 0) and Dsetpath/Dgetpath (E_OK == 0). */
static void b_verdict(long rc)
{
	if (rc >= 0) {
		b_str("  [OK]");
	} else {
		b_str("  [FAIL rc=");
		b_dec(rc);
		b_str("]");
	}
}

/* ------------------------------------------------------------------ *
 *  check registry -- the compact end-of-run verdict                   *
 * ------------------------------------------------------------------ *
 *
 * Every probe with a documented expected value records its verdict here.
 * The per-check trace line is still emitted (and committed to the log) as
 * the check runs; the registry exists so the run can END with a one-screen
 * summary, because on real hardware there is no serial line and the trace
 * has long scrolled off the framebuffer by the time the last check runs.
 *
 * A check is PASS, FAIL, or SKIP. SKIP is used ONLY where the precondition
 * for a documented case is genuinely absent on this machine (no drive
 * mounted, no key pressed within the window, a pointer that cannot be
 * dereferenced from EL0) -- never to hide a wrong result.
 */
#define MAX_CHECKS 200
#define V_PASS 0
#define V_FAIL 1
#define V_SKIP 2

#define G_GEMDOS 0
#define G_BCHR   1
#define G_BBLK   2
#define G_BSYS   3
#define G_AES    4
#define G_SUPER  5
#define N_GROUPS 6

static const char *const GROUP_NAME[N_GROUPS] = {
	"GEMDOS  ", "BIOS-CHR", "BIOS-BLK", "BIOS-SYS", "AES     ", "SUPER   "
};

static const char   *chk_name[MAX_CHECKS];
static const char   *chk_why[MAX_CHECKS];
static unsigned char chk_group[MAX_CHECKS];
static unsigned char chk_state[MAX_CHECKS];
static int           chk_count;
static int           chk_lost;      /* checks that did not fit the registry */

static void chk_add(int grp, const char *name, int state, const char *why)
{
	if (chk_count >= MAX_CHECKS) { chk_lost++; return; }
	chk_name[chk_count]  = name;
	chk_why[chk_count]   = why;
	chk_group[chk_count] = (unsigned char)grp;
	chk_state[chk_count] = (unsigned char)state;
	chk_count++;
}

/* Trace tail shared by the ck_* helpers. */
static void ck_tail(int ok)
{
	b_str(ok ? "  [OK]" : "  [FAIL]");
	b_nl();
	b_emit();
}

/* Check "got must equal want", decimal trace (return/error codes). */
static void ck_eq(int grp, const char *name, long got, long want)
{
	int ok = (got == want);
	chk_add(grp, name, ok ? V_PASS : V_FAIL, 0);
	b_reset();
	b_str(name);
	b_str("  got=");
	b_dec(got);
	b_str(" want=");
	b_dec(want);
	ck_tail(ok);
}

/* Check "got must equal want", hex trace (bitmaps, pointers, packed words). */
static void ck_hex(int grp, const char *name, unsigned long got, unsigned long want)
{
	int ok = (got == want);
	chk_add(grp, name, ok ? V_PASS : V_FAIL, 0);
	b_reset();
	b_str(name);
	b_str("  got=");
	b_hex(got, 2);
	b_str(" want=");
	b_hex(want, 2);
	ck_tail(ok);
}

/* Check a condition the name states in full (no value pair to print). */
static void ck_bool(int grp, const char *name, int ok)
{
	chk_add(grp, name, ok ? V_PASS : V_FAIL, 0);
	b_reset();
	b_str(name);
	ck_tail(ok);
}

/* Record a check that could not be performed, with the reason. */
static void ck_skip(int grp, const char *name, const char *why)
{
	chk_add(grp, name, V_SKIP, why);
	b_reset();
	b_str(name);
	b_str("  [SKIP: ");
	b_str(why);
	b_str("]");
	b_nl();
	b_emit();
}

/* BEFORE-line for a probe (constant text, ends with " ...\r\n"). */
static void before(const char *s)
{
	b_reset();
	b_str(s);
	b_str(" ...");
	b_nl();
	b_emit();
}


/* ------------------------------------------------------------------ *
 *  S1 stage probes -- helpers                                         *
 * ------------------------------------------------------------------ */

/*
 * Minimal AES binding. The GEM trap is `svc #2` with x0 = selector (200 =
 * AES) and x1 = &AESPB -- see sdk/gemlib/overlay/_gc_aes.c, which documents
 * the same ABI. Hand-rolled here so this diagnostic keeps its "osbind.h and
 * nothing else" dependency profile.
 */
typedef struct {
	short *control;
	short *global;
	short *intin;
	short *intout;
	long  *addrin;
	long  *addrout;
} AESPB;

static short aes_control[6];
static short aes_global[16];
static short aes_intin[16];
static short aes_intout[16];
static long  aes_addrin[4];
static long  aes_addrout[4];

static void aes_trap(void *pb)
{
	register long x0 __asm__("x0") = 200;
	register long x1 __asm__("x1") = (long)pb;
	__asm__ __volatile__("svc\t#2" : "+r"(x0) : "r"(x1) : "memory", "cc");
	(void)x0;
}

/* Issue one AES call; returns int_out[0] (the AES return code). */
static short aes_call(short opcode, short nin, short nout, short naddrin)
{
	AESPB pb;
	aes_control[0] = opcode;
	aes_control[1] = nin;
	aes_control[2] = nout;
	aes_control[3] = naddrin;
	aes_control[4] = 0;
	aes_intout[0] = 0;
	pb.control = aes_control;
	pb.global  = aes_global;
	pb.intin   = aes_intin;
	pb.intout  = aes_intout;
	pb.addrin  = aes_addrin;
	pb.addrout = aes_addrout;
	aes_trap(&pb);
	return aes_intout[0];
}

/*
 * ap_private out of the AESGLOBAL: the m68k LONG, FOUR bytes at byte offset
 * 6 (aes/gemrslib.h:15-27). The AESGLOBAL keeps the TOS field widths on
 * every ABI -- it is addressed by WORD index as well as by byte offset, and
 * only the m68k widths satisfy both (docs/api/aes/application.md
 * #AESGLOBAL, decision of 2026-09-13). NewTOS's gemlib overlay reads
 * `_AESappglobal` the same way; the pristine header's 8-byte read would
 * drag in ap_ptree's four bytes. Read byte-wise because byte 6 is only
 * WORD-aligned.
 */
static unsigned long ap_private_of_global(void)
{
	const unsigned char *g = (const unsigned char *)aes_global;
	unsigned long v = 0;
	int i;
	for (i = 3; i >= 0; i--)
		v = (v << 8) | (unsigned long)g[6 + i];
	return v;
}

/* The 32-byte native (LP64) GEM OBJECT: ob_spec is 8 bytes at offset 16. */
typedef struct {
	short          ob_next, ob_head, ob_tail;
	unsigned short ob_type, ob_flags, ob_state;
	unsigned long  ob_spec;
	short          ob_x, ob_y, ob_width, ob_height;
} OBJ;

/*
 * A two-object menu bar: ROOT (G_IBOX) + THEBAR (G_BOX, LASTOB). menu_bar
 * resizes THEBAR to the screen width itself and always draws the black
 * separator line under the bar, so a successful redraw is visible as a
 * 1-pixel black rule across the top of the screen.
 */
static OBJ mbar_tree[2] = {
	{ -1,  1,  1, 24 /* G_IBOX */, 0x0000, 0, 0UL,      0, 0, 640, 19 },
	{ -1, -1, -1, 20 /* G_BOX  */, 0x0020 /* LASTOB */, 0, 0x70UL, 0, 0, 640, 19 }
};

/* struct keytbl as XBIOS Keytbl returns it (EmuTOS bios/ikbd.h:57-86). */
typedef struct {
	const unsigned char *norm;
	const unsigned char *shft;
	const unsigned char *caps;
	const unsigned char *altnorm;
	const unsigned char *altshft;
	const unsigned char *altcaps;
	const unsigned char *const *dead;
	short features;
} KEYTBL;

/* ------------------------------------------------------------------ *
 *  BIOS (trap 13) probe support                                       *
 * ------------------------------------------------------------------ */

/*
 * The 18-byte BIOS Parameter Block `Getbpb` points at -- nine UWORDs, the
 * fixed Atari ABI layout (docs/api/structures/bpb.md, tos-api/src/bios/bpb.rs).
 * Unlike most TOS structs this one has NO pointers, so it is byte-identical
 * on m68k and on LP64 AArch64.
 */
typedef struct {
	unsigned short recsiz;   /* bytes per sector                */
	unsigned short clsiz;    /* sectors per cluster             */
	unsigned short clsizb;   /* bytes per cluster               */
	unsigned short rdlen;    /* root directory length, sectors  */
	unsigned short fsiz;     /* FAT size, sectors               */
	unsigned short fatrec;   /* first sector of the default FAT */
	unsigned short datrec;   /* first data sector               */
	unsigned short numcl;    /* number of data clusters         */
	unsigned short b_flags;  /* B_16 = 0x01, B_1FAT = 0x02      */
} BPB18;

#define B_16   0x01
#define B_1FAT 0x02

/*
 * MD / MPB as the NewTOS kernel writes them. The Atari field ORDER is kept,
 * but the sizes are AArch64-natural: 8-byte pointers, 4-byte LONGs
 * (tos-types/src/memory.rs -- MD is 24 bytes, MPB is 3 pointers = 24). A C
 * `long` is 8 bytes here, so m_start / m_length must be `unsigned int`.
 */
typedef struct MDX {
	struct MDX   *m_link;    /* +0,  8 */
	unsigned int  m_start;   /* +8,  4 */
	unsigned int  m_length;  /* +12, 4 */
	void         *m_own;     /* +16, 8 */
} MDX;

typedef struct {
	MDX *mp_mfl;             /* +0,  free list head      */
	MDX *mp_mal;             /* +8,  allocated list head */
	MDX *mp_rover;           /* +16, roving pointer      */
} MPBX;

/* Layout drift here would silently invalidate every Getmpb check below. */
typedef char apitest_assert_mdx[(sizeof(MDX)  == 24) ? 1 : -1];
typedef char apitest_assert_mpb[(sizeof(MPBX) == 24) ? 1 : -1];

/* TOS error codes the BIOS contract names (tos-types::error). */
#define E_OK    0L
#define ESECNF  (-8L)
#define EUNDEV  (-15L)

/* Rwabs r_w flag bits (docs/api/bios/04-rwabs.md, EmuTOS bios/disk.h:53-61). */
#define RW_READ        0x00
#define RW_NOTRANSLATE 0x08
/* With RW_NOTRANSLATE the drive argument is a physical unit: the SD card is
 * unit 26, EmuTOS' SD/MMC numbering 2 + 8*bus + device (BIOS-STOR-002,
 * docs/api/bios/04-rwabs.md "Decisions"). */
#define PHYS_SD_UNIT   26

/* Low-memory system variables this group cross-checks against. */
#define SV_MEMBOT   0x432UL   /* LONG, bottom of the TPA / ST-RAM pool */
#define SV_MEMTOP   0x436UL   /* LONG, top of same                     */
#define SV_HZ200    0x4BAUL   /* LONG, 200 Hz tick counter             */
#define SV_DRVBITS  0x4C2UL   /* LONG, physical drive bitmap           */
#define SV_PCOOKIES 0x5A0UL   /* LONG, cookie-jar pointer              */

/*
 * `Setexc` as a plain LONG-returning call. mintlib's Setexc() macro casts its
 * result to `void (*)(void)`; the NewTOS vector shadow holds 64-bit values and
 * a check wants the number, so go through the header's own trap helper. Same
 * trap, same function id, same argument registers -- only the result cast
 * differs.
 */
#define SETEXC(num, vec) \
	((long)trap_13_wwl((short)(0x05), (short)(num), (long)(vec)))

/*
 * A pure vector-shadow slot for the Setexc set/restore probe: m68k vector
 * 0x20 is TRAP #0, which NewTOS shadows but never dispatches
 * (tos-api/src/bios/vectors.rs) -- so installing a value there cannot make
 * the kernel jump anywhere. The live etv_* hooks (0x100..0x102) are only ever
 * QUERIED here (vector == -1), never written: etv_term is delivered as an EL0
 * upcall by Pterm, so writing a bogus handler would crash this program on exit.
 */
#define SHADOW_VEC   0x20
#define SHADOW_MAGIC 0x12345678L

/* Kbshift `shifty` bits used below (docs/api/bios/0b-kbshift.md). */
#define MODE_CTRL 0x04

/* Little-endian field readers for a raw 512-byte sector. */
static unsigned int le16(const unsigned char *p)
{
	return (unsigned int)p[0] | ((unsigned int)p[1] << 8);
}

static unsigned long le32(const unsigned char *p)
{
	return (unsigned long)p[0] | ((unsigned long)p[1] << 8)
	     | ((unsigned long)p[2] << 16) | ((unsigned long)p[3] << 24);
}

/* MBR partition types that are containers, not mountable volumes
 * (tos-api/src/bios/drive.rs `is_extended`). */
static int is_extended_type(unsigned int t)
{
	return t == 0x05 || t == 0x0F || t == 0x85;
}

/*
 * Exclusive top of the user addresses NewTOS hands out. Since ARCH03 stage 1
 * every user address is its own physical address in the one address space
 * all processes share, and everything a call returns -- Malloc/Mxalloc
 * blocks, the TPA, the BIOS structures Getbpb/Getmpb point at (ADR-023), the
 * screen -- lies below 2 GiB (plan invariant I2: positive 32-bit LONGs).
 */
#define NEWTOS_USER_VA_TOP 0x80000000UL

/*
 * Can `p` be dereferenced from EL0?
 *
 * The kernel's own statics are linked at 0xFFFF_0000_0000_0000 + physical
 * (kernel/src/arch/aarch64/link-rpi4.ld:24) and are EL1-only. A call that
 * hands back a pointer into kernel memory therefore gives an application an
 * address it CANNOT read -- and dereferencing it here would kill the program
 * instead of reporting the defect. So every returned pointer is range-checked
 * before it is followed: a user address is below NEWTOS_USER_VA_TOP; the
 * first 0x100 bytes of page 0 (the m68k vector area) hold nothing.
 */
static int el0_readable(unsigned long p)
{
	if (p < 0x100UL) return 0;
	return p < NEWTOS_USER_VA_TOP;
}

/*
 * Bounded wait for console input. Returns 1 as soon as Bconstat(2) reports a
 * byte ready, 0 on timeout. The window is measured on the 200 Hz `_hz_200`
 * sysvar; a spin bound is the backstop, so the loop terminates even if the
 * system tick is dead. An unattended run must never leave the machine hanging.
 */
static int wait_input(unsigned int ticks)
{
	unsigned int t0 = RD_U32(SV_HZ200);
	unsigned long spins;
	for (spins = 0; ; spins++) {
		unsigned int now;
		if (Bconstat(2)) return 1;
		now = RD_U32(SV_HZ200);
		if ((unsigned int)(now - t0) >= ticks) return 0;
		/* Backstop: if the counter has not moved at all after this many
		 * polls it is not running, and the tick-based timeout would never
		 * fire. Give up instead of spinning forever. While the counter
		 * IS moving the loop is bounded by `ticks` by construction. */
		if (spins >= 200000UL && now == t0) return 0;
	}
}

/* ------------------------------------------------------------------ *
 *  Super / Supexec probe support  (TASK/X01)                          *
 * ------------------------------------------------------------------ */

/*
 * The two GEMDOS/XBIOS calls every ported TOS program reaches for when it
 * wants a low-memory system variable: Super (GEMDOS 0x20) and Supexec
 * (XBIOS 0x26). On m68k they are the only legal way to touch $400..$5FF;
 * on NewTOS those cells live in the process's own page 0, readable always
 * and writable only in the SUPER STATE these two calls switch on
 * (docs/api/_super-state.md, TASK/MEM12). The callers bracket their accesses
 * with them (TeraDesk library/utility/gettos.c:40-42, main.c:2090,
 * file.c:945/969, video.c:289/318).
 *
 * The probes below drive exactly those shapes. They are written so they are
 * SAFE in both states of the world: before the binding was retargeted,
 * Super returned EINVFN (-32) and Supexec returned its own function number
 * (38), and neither value is ever dereferenced without el0_readable().
 */

/* An arbitrary, unmistakable LONG the Supexec callee hands back. Positive
 * and > 0xFFFF, so it can never be confused with a TOS error code or with
 * XBIOS 0x26's own "unimplemented" answer (38). */
#define SX_MAGIC 0x5EC0FFEEL

/* Cookie tag 'NTOS' (tos-api/src/cookie.rs:25) and the version the kernel
 * seeds it with (kernel/src/lib.rs:285). */
#define TAG_NTOS 0x4E544F53UL
#define NTOS_VERSION 0x00000100UL

/* Ssystem (GEMDOS 0x154) as a plain LONG-returning call, so the cookie the
 * kernel really owns can be cross-checked against the per-process $5A0
 * mirror. Same trap helper mint/mintbind.h uses for Ssystem(); apitest does
 * not include mintbind.h (it deliberately depends on osbind.h only). */
#define SSYSTEM(mode, a1, a2) \
	((long)trap_1_wwll((short)(0x154), (short)(mode), (long)(a1), (long)(a2)))
#define S_GETCOOKIE 8
#define S_SETCOOKIE 9
#define S_DELCOOKIE 26

/* Scratch cookie SX11..SX15 add and remove again, so the run leaves the
 * kernel jar exactly as it found it. 'APIT' has no zero byte, so it passes
 * the FreeMiNT tag rule (tos-api/src/cookie.rs `valid_tag`). */
#define TAG_APIT 0x41504954UL
#define APIT_VALUE 0x13572468UL

/* Cookie tag '_MCH' -- the second cookie the kernel seeds
 * (tos-api/src/cookie.rs `seed_defaults`), used to measure the jar's slot
 * pitch on the wire. */
#define TAG_MCH 0x5F4D4348UL

/* TeraDesk's own jar element (library/xdialog/library.h:201-205), verbatim.
 * On m68k this is 8 bytes and matches the wire format; on LP64 `long` is 8
 * bytes, so sizeof(COOKIE) is 16 and a `cookie++` walk steps TWO slots at a
 * time. SX16/SX17 measure that instead of arguing about it. */
typedef struct {
	long name;
	long value;
} COOKIE_AS_PORTED;

/* Did the Supexec callee actually run? A no-op Supexec leaves this at 0. */
static long sx_calls;

/* The Supexec callee: has a side effect AND a return value, so both halves
 * of the contract ("the function runs", "its LONG result comes back") are
 * separately observable. */
static long sx_probe(void)
{
	sx_calls++;
	return SX_MAGIC;
}

/*
 * cflib's get_cookie_adr (sdk/cflib/cflib-21/cookie.c:60-63) in NewTOS form:
 * fetch the cookie-jar pointer out of _p_cookies. The width is deliberately
 * FOUR bytes, not the eight a C `long` has here: a TOS system variable is a
 * 32-bit LONG on the wire, and `ramtop` sits immediately above at $5A4, so
 * an 8-byte read returns (ramtop << 32) | jar.
 */
static long sx_cookie_adr(void)
{
	return (long)RD_U32(SV_PCOOKIES);
}

/* Scan a classic {tag, value} jar for `tag`; 1 + *out on a hit, 0 on a miss.
 * Terminator is tag == 0 (docs/api/bios/_cookie-jar.md). Bounded by the
 * jar's 16 slots so a garbage pointer cannot run away. */
static int sx_jar_find(unsigned long jar, unsigned long tag, unsigned long *out)
{
	int i;
	for (i = 0; i < 16; i++) {
		unsigned long t = RD_U32(jar + (unsigned long)i * 8);
		if (t == 0)
			return 0;
		if (t == tag) {
			if (out)
				*out = RD_U32(jar + (unsigned long)i * 8 + 4);
			return 1;
		}
	}
	return 0;
}

/* The `value` word of the {0, capacity} terminator a jar scan stops at, or 0
 * if there is none inside the 16-slot window -- which would send the loop
 * above off the end of the jar. */
static unsigned long sx_jar_term(unsigned long jar)
{
	int i;
	for (i = 0; i < 16; i++) {
		if (RD_U32(jar + (unsigned long)i * 8) == 0)
			return RD_U32(jar + (unsigned long)i * 8 + 4);
	}
	return 0;
}

/* ------------------------------------------------------------------ *
 *  paged summary output                                               *
 * ------------------------------------------------------------------ */

/* Lines per page. The framebuffer console is far taller than this on both
 * boards, but a 25-line screen must stay readable too. */
#define PAGE_LINES 20

static int page_used;

/* Flush the line buffer as one summary line; hold for a key every page. */
static void page_emit(void)
{
	b_emit();
	if (++page_used >= PAGE_LINES) {
		page_used = 0;
		if (!g_batch) {
			emit("-- more -- press any key --\r\n");
			(void)Cconin();
		}
	}
}

/* Right-pad a decimal to `w` columns so the summary table lines up. */
static void b_dec_w(long v, int w)
{
	int before_len = LP;
	b_dec(v);
	while (LP - before_len < w) b_ch(' ');
}

/* The end-of-run verdict: one line per group, then the names of every
 * failed and skipped check. Paged, so it never scrolls away unread. */
static void summary(void)
{
	int pass[N_GROUPS], fail[N_GROUPS], skip[N_GROUPS];
	int tp = 0, tf = 0, ts = 0;
	int g, i, n;

	for (g = 0; g < N_GROUPS; g++) { pass[g] = fail[g] = skip[g] = 0; }
	for (i = 0; i < chk_count; i++) {
		g = chk_group[i];
		if (g >= N_GROUPS) continue;
		if      (chk_state[i] == V_PASS) { pass[g]++; tp++; }
		else if (chk_state[i] == V_FAIL) { fail[g]++; tf++; }
		else                             { skip[g]++; ts++; }
	}

	page_used = 0;
	b_reset(); b_str("=== APITEST SUMMARY ==="); b_nl(); page_emit();
	b_reset(); b_str("GROUP      PASS  FAIL  SKIP"); b_nl(); page_emit();
	for (g = 0; g < N_GROUPS; g++) {
		b_reset();
		b_str(GROUP_NAME[g]);
		b_str("   ");
		b_dec_w(pass[g], 6);
		b_dec_w(fail[g], 6);
		b_dec_w(skip[g], 6);
		b_nl();
		page_emit();
	}
	b_reset();
	b_str("TOTAL      ");
	b_dec_w(tp, 6);
	b_dec_w(tf, 6);
	b_dec_w(ts, 6);
	b_nl();
	page_emit();

	if (chk_lost) {
		b_reset();
		b_str("WARNING: ");
		b_dec(chk_lost);
		b_str(" checks did not fit the registry (raise MAX_CHECKS)");
		b_nl();
		page_emit();
	}

	b_reset(); b_str("-- FAILED --"); b_nl(); page_emit();
	n = 0;
	for (i = 0; i < chk_count; i++) {
		if (chk_state[i] != V_FAIL) continue;
		n++;
		b_reset();
		b_str("  ");
		b_str(chk_name[i]);
		b_nl();
		page_emit();
	}
	if (n == 0) { b_reset(); b_str("  (none)"); b_nl(); page_emit(); }

	b_reset(); b_str("-- SKIPPED --"); b_nl(); page_emit();
	n = 0;
	for (i = 0; i < chk_count; i++) {
		if (chk_state[i] != V_SKIP) continue;
		n++;
		b_reset();
		b_str("  ");
		b_str(chk_name[i]);
		if (chk_why[i]) { b_str(" : "); b_str(chk_why[i]); }
		b_nl();
		page_emit();
	}
	if (n == 0) { b_reset(); b_str("  (none)"); b_nl(); page_emit(); }
}

/* ------------------------------------------------------------------ */

int main(void)
{
	/* Pre-log screen banner (log not created yet -> emit() is screen-only
	 * because g_log_ok == 0). Proves EL0 + Cconws work even if Fcreate
	 * itself faults. */
	emit("\r\n=== APITEST START (pre-log) ===\r\n");

	/* ---- T01: create/truncate the log, then switch to commit-per-line ---- */
	{
		long fd = Fcreate(LOGPATH, 0);
		if (fd >= 0) {
			(void)Fclose((short)fd);   /* created + truncated + committed */
			g_log_ok = 1;
		}
	}

	/* Batch-mode probe, before anything can page: a headless run must never
	 * reach a Cconin(). Deliberately AFTER T01 so the answer is logged. */
	g_batch = batch_requested();

	/* First COMMITTED line, before any probe: if T02+ faults, this proves on
	 * the Mac that the log path itself works on real HW. */
	emit("=== APITEST v1 START ===\r\n");
	emit("NewTOS EL0 console API diagnostic (C:\\AUTO\\APITEST.NCP)\r\n");
	emit("trace -> HDMI fbcon (Cconws) + C:\\APITEST.LOG (commit-per-line)\r\n");
	b_reset();
	if (g_log_ok)
		b_str("T01   C:\\APITEST.LOG created; commit-per-line ACTIVE");
	else
		b_str("T01   Fcreate C:\\APITEST.LOG FAILED -> SCREEN-ONLY");
	b_nl();
	b_emit();
	b_reset();
	b_str("T01b  C:\\APITEST.CFG -> mode = ");
	b_str(g_batch ? "BATCH (never waits for a key)"
	              : "INTERACTIVE (pauses on Cconin)");
	b_nl();
	b_emit();

	/* ---- T02: Sversion -- basic EL0 GEMDOS round-trip ---- */
	before("T02 Sversion() (GEMDOS 0x30)");
	{
		unsigned int sv = (unsigned int)(Sversion()) & 0xFFFFu;
		b_reset();
		b_str("T02   Sversion = ");
		b_hex(sv, 4);
		b_str("  [expect 0x3000]");
		b_nl();
		b_emit();
	}

	/* ---- T03..T07: low-memory sysvars (the #1 suspect: B1 per-process
	 *      page-0 populate visible on real HW?) ---- */

	/* T03: _sysbase @0x4F2 (LP64 8-byte read, the TeraDesk `ldr x8,[x8]`
	 *      idiom) -> OSHEADER VA. */
	before("T03 read _sysbase @0x4F2 (8-byte)");
	{
		unsigned long sb = RD_UL(0x4F2UL);
		b_reset();
		b_str("T03   _sysbase = ");
		b_hex(sb, 8);
		b_str("  [expect 0x000005C0, NOT 0]");
		b_nl();
		b_emit();

		/* T04: OSHEADER.os_version @ sysbase+2 -- only if sysbase looks
		 *      sane (inside the reserved low sysvar region). */
		before("T04 read OSHEADER.os_version @_sysbase+2");
		b_reset();
		if (sb >= 0x400UL && sb < 0x1000UL) {
			unsigned short ov = RD_U16(sb + 2);
			b_str("T04   os_version = ");
			b_hex(ov, 4);
			b_str("  [expect 0x0404]");
		} else {
			b_str("T04   SKIPPED (sysbase out of [0x400,0x1000) -> not populated)");
		}
		b_nl();
		b_emit();
	}

	/* T05: _p_cookies @0x5A0 -> cookie jar VA. */
	before("T05 read _p_cookies @0x5A0 (8-byte)");
	{
		unsigned long pc = RD_UL(0x5A0UL);
		b_reset();
		b_str("T05   _p_cookies = ");
		b_hex(pc, 8);
		b_str("  [expect 0x000005F0]");
		b_nl();
		b_emit();
	}

	/* T06: memvalid @0x420 -- RAM-config validity magic (4-byte). */
	before("T06 read memvalid @0x420 (4-byte)");
	{
		unsigned int mv = RD_U32(0x420UL);
		b_reset();
		b_str("T06   memvalid = ");
		b_hex(mv, 8);
		b_str("  [expect 0x752019F3]");
		b_nl();
		b_emit();
	}

	/* T07: drvbits @0x4C2 -- physical drive bitmap sysvar (4-byte). */
	before("T07 read drvbits @0x4C2 (4-byte)");
	{
		unsigned int db = RD_U32(0x4C2UL);
		b_reset();
		b_str("T07   drvbits = ");
		b_hex(db, 8);
		b_str("  [nonzero: C:=bit2=0x4, ...]");
		b_nl();
		b_emit();
	}

	/* ---- T08: our BasePage (the loader populates it at the start of our
	 *      TPA block -- ARCH03 stage 1; the image follows at +0x1000).
	 *      p_tbase@8, p_env@0x2C, p_hitpa@4 ---- */
	before("T08 read BasePage at _base (p_lowtpa/p_hitpa/p_tbase/p_env)");
	{
		unsigned int lowtpa = BP_U32(0x00UL);
		unsigned int hitpa  = BP_U32(0x04UL);
		unsigned int tbase  = BP_U32(0x08UL);
		unsigned int penv   = BP_U32(0x2CUL);
		b_reset();
		b_str("T08   p_lowtpa=");
		b_hex(lowtpa, 8);
		b_str(" p_hitpa=");
		b_hex(hitpa, 8);
		b_nl();
		b_emit();
		b_reset();
		b_str("T08   p_tbase=");
		b_hex(tbase, 8);
		b_str(" [expect p_lowtpa+0x1000]  p_env=");
		b_hex(penv, 8);
		b_str(" [0=none]");
		b_nl();
		b_emit();
	}

	/* ---- T09: Dgetdrv -- current drive ---- */
	before("T09 Dgetdrv() (GEMDOS 0x19)");
	{
		short drv = (short)Dgetdrv();
		b_reset();
		b_str("T09   current drive = ");
		b_dec(drv);
		b_str("  (");
		b_ch((char)('A' + (drv & 0x1F)));
		b_str(":)");
		b_nl();
		b_emit();
	}

	/* ---- T10: Drvmap -- BIOS drive bitmap ---- */
	before("T10 Drvmap() (BIOS 0x0A)");
	{
		long dm = Drvmap();
		b_reset();
		b_str("T10   Drvmap = ");
		b_hex((unsigned long)dm, 8);
		b_str("  (should match drvbits sysvar T07)");
		b_nl();
		b_emit();
	}

	/* ---- T11: Dgetpath -- current path on the default drive ---- */
	before("T11 Dgetpath(buf,0) (GEMDOS 0x47)");
	{
		static char path[128];
		long r;
		unsigned i;
		for (i = 0; i < sizeof(path); i++) path[i] = 0;
		r = Dgetpath(path, 0);
		b_reset();
		b_str("T11   rc=");
		b_dec(r);
		b_str("  path=\"");
		b_str(path);
		b_str("\"");
		b_nl();
		b_emit();
	}

	/* ---- T12: Dfree(0) -- free/total clusters on the current drive.
	 *      The kernel writes 4 x 32-bit LONGs (drive.rs:68), so use a
	 *      4 x u32 buffer, NOT mintlib's _DISKINFO (4 x LP64 long). ---- */
	before("T12 Dfree(&buf,0) (GEMDOS 0x36)");
	{
		static unsigned int di[4];
		long r;
		di[0] = di[1] = di[2] = di[3] = 0;
		r = Dfree(di, 0);
		b_reset();
		b_str("T12   rc=");
		b_dec(r);
		b_str(" free_cl=");
		b_dec((long)di[0]);
		b_str(" tot_cl=");
		b_dec((long)di[1]);
		b_str(" bps=");
		b_dec((long)di[2]);
		b_str(" spc=");
		b_dec((long)di[3]);
		b_nl();
		b_emit();
	}

	/* ---- T13/T14: directory enumeration (the DTA probe). Names must be
	 *      correct byte strings; a 4-byte shift = the old DTA bug.
	 *
	 *      DTA-SAFETY: do NOT open the log (commit-per-line -> Fopen) between
	 *      Fsfirst and the final Fsnext -- a filesystem op mid-search could
	 *      disturb the active DTA search state. So collect the names into
	 *      local arrays during the loop (no fs op / no emit inside it) and
	 *      emit+commit them AFTER the enumeration finishes. ---- */
	before("T13 Fsetdta+Fsfirst(\"*.*\",0x17)+Fsnext loop (GEMDOS 0x1A/0x4E/0x4F)");
	{
		static _DTA dta;
		static char          names[30][16];
		static unsigned char attrs[30];
		static unsigned int  sizes[30];
		long fsfirst_rc, last_rc;
		int count = 0;
		int i;

		Fsetdta(&dta);
		fsfirst_rc = Fsfirst("*.*", 0x17);
		last_rc = fsfirst_rc;
		while (last_rc == 0 && count < 30) {
			/* copy the DTA fields out -- NO filesystem op inside the search */
			for (i = 0; i < 14; i++) names[count][i] = dta.dta_name[i];
			names[count][14] = 0;
			names[count][15] = 0;
			attrs[count] = (unsigned char)dta.dta_attribute;
			sizes[count] = (unsigned int)dta.dta_size;
			count++;
			last_rc = Fsnext();
		}

		/* search finished -> now it is safe to emit+commit (opens the log) */
		b_reset();
		b_str("T13   Fsfirst rc=");
		b_dec(fsfirst_rc);
		if (fsfirst_rc != 0) b_str("  [no match / empty dir?]");
		b_nl();
		b_emit();

		for (i = 0; i < count; i++) {
			b_reset();
			b_str("T14   [");
			b_dec(i);
			b_str("] name=\"");
			b_str(names[i]);
			b_str("\" attr=");
			b_hex((unsigned long)attrs[i], 2);
			b_str(" size=");
			b_dec((long)sizes[i]);
			b_nl();
			b_emit();
		}
		b_reset();
		b_str("T14   enumerated ");
		b_dec(count);
		b_str(" entr");
		b_str(count == 1 ? "y" : "ies");
		b_str(", last Fsnext rc=");
		b_dec(last_rc);
		b_nl();
		b_emit();
	}

	/* ---- T15/T16/T17: file round-trip ---- */
	{
		static const char TPATH[] = "C:\\APITST.TMP";
		static const char PAYLOAD[] = "APITEST round-trip 0123456789 ABCDEF";
		static char rbuf[64];
		long wh, rh, wr, rd;
		unsigned plen = slen(PAYLOAD);
		unsigned i;

		before("T15 Fcreate C:\\APITST.TMP + Fwrite payload");
		wh = Fcreate(TPATH, 0);
		b_reset();
		if (wh >= 0) {
			wr = Fwrite((short)wh, (long)plen, (long)PAYLOAD);
			(void)Fclose((short)wh);
			b_str("T15   handle=");
			b_dec(wh);
			b_str(" Fwrite=");
			b_dec(wr);
			b_str(" (want ");
			b_dec((long)plen);
			b_str(") + Fclose");
		} else {
			b_str("T15   Fcreate FAILED err=");
			b_dec(wh);
		}
		b_nl();
		b_emit();

		before("T16 Fopen C:\\APITST.TMP + Fread + compare");
		for (i = 0; i < sizeof(rbuf); i++) rbuf[i] = 0;
		rh = Fopen(TPATH, 0);
		b_reset();
		if (rh >= 0) {
			rd = Fread((short)rh, (long)sizeof(rbuf), (long)rbuf);
			(void)Fclose((short)rh);
			b_str("T16   handle=");
			b_dec(rh);
			b_str(" Fread=");
			b_dec(rd);
			b_str("  compare=");
			if (rd == (long)plen && mcmp(rbuf, PAYLOAD, plen) == 0)
				b_str("MATCH");
			else
				b_str("MISMATCH");
		} else {
			b_str("T16   Fopen FAILED err=");
			b_dec(rh);
		}
		b_nl();
		b_emit();
		ck_bool(G_GEMDOS, "T16 file round-trip payload matches",
			rh >= 0 && rd == (long)plen && mcmp(rbuf, PAYLOAD, plen) == 0);

		before("T17 Fdelete C:\\APITST.TMP");
		{
			long dr = Fdelete(TPATH);
			b_reset();
			b_str("T17   Fdelete rc=");
			b_dec(dr);
			b_nl();
			b_emit();
		}
	}

	/* ---- T18/T19/T20: memory (Malloc EL0 VA -> write/read -> Mfree) ---- */
	before("T18 Malloc(1024) (GEMDOS 0x48)");
	{
		long va = Malloc(1024);
		b_reset();
		b_str("T18   Malloc = ");
		b_hex((unsigned long)va, 8);
		b_str("  [expect a mapped EL0 VA > 0x1000]");
		b_nl();
		b_emit();

		if (va > 0) {
			before("T19 write+read a byte at the Malloc VA");
			{
				volatile unsigned char *p = (volatile unsigned char *)(unsigned long)va;
				unsigned char got;
				*p = 0x5A;
				got = *p;
				b_reset();
				b_str("T19   wrote 0x5A, read ");
				b_hex((unsigned long)got, 2);
				b_str(got == 0x5A ? "  MATCH" : "  MISMATCH");
				b_nl();
				b_emit();
			}
			before("T20 Mfree the block (GEMDOS 0x49)");
			{
				long mr = Mfree(va);
				b_reset();
				b_str("T20   Mfree rc=");
				b_dec(mr);
				b_nl();
				b_emit();
			}
		} else {
			before("T19/T20 SKIPPED (Malloc returned no VA)");
			emit("T19/T20   skipped\r\n");
		}
	}

	/* ---- T21/T22: time (raw packed DOS words) ---- */
	before("T21 Tgettime() (GEMDOS 0x2C)");
	{
		unsigned int tt = (unsigned int)(Tgettime()) & 0xFFFFu;
		b_reset();
		b_str("T21   Tgettime raw = ");
		b_hex(tt, 4);
		b_str("  (packed hhhhh mmmmmm sssss)");
		b_nl();
		b_emit();
	}
	before("T22 Tgetdate() (GEMDOS 0x2A)");
	{
		unsigned int td = (unsigned int)(Tgetdate()) & 0xFFFFu;
		b_reset();
		b_str("T22   Tgetdate raw = ");
		b_hex(td, 4);
		b_str("  (packed yyyyyyy mmmm ddddd, y+1980)");
		b_nl();
		b_emit();
	}

	/* ================================================================
	 * T23..T27: QED rsrc_load reproduction (console, no AES). Replicates
	 * exactly the Fopen that QED's rsrc_load performs, so we can reproduce
	 * "RSC not found" on real HW from the AUTO console app, with the same
	 * durable commit-per-line log. Card layout for reference:
	 *   C:\qed.rsc            (root, 41166 bytes)
	 *   C:\APPS\QED\qed.rsc   (subdir copy)
	 *   C:\APPS\QED\QED.NAP   (the app)
	 * ================================================================ */
	{
		static char          savebuf[128];
		static char          nowbuf[128];
		static unsigned char hbuf[16];
		long rc, rc2;
		unsigned i;

		/* T23 -- absolute drive path (the one the stale trace shows
		 *        failing): the exact string rsrc_load("C:\qed.rsc") used. */
		before("T23 Fopen(\"C:\\qed.rsc\",0) -- absolute rsrc_load path");
		rc = Fopen("C:\\qed.rsc", 0);
		b_reset();
		b_str("T23   Fopen rc/handle=");
		b_dec(rc);
		b_verdict(rc);
		b_nl();
		b_emit();
		if (rc >= 0) (void)Fclose((short)rc);

		/* T24 -- bare name from CWD=root (C:\qed.rsc exists -> resolves). */
		before("T24 Fopen(\"qed.rsc\",0) -- bare name from CWD (root)");
		rc = Fopen("qed.rsc", 0);
		b_reset();
		b_str("T24   Fopen rc/handle=");
		b_dec(rc);
		b_verdict(rc);
		b_nl();
		b_emit();
		if (rc >= 0) (void)Fclose((short)rc);

		/* T25 -- relative from the QED subdir (how the desktop launches
		 *        QED: chdir into C:\APPS\QED, then open by bare name). */
		before("T25 chdir C:\\APPS\\QED + relative Fopen (qed.rsc, QED.NAP)");
		/* 25.1 save the current dir first (restored in T27) */
		for (i = 0; i < sizeof(savebuf); i++) savebuf[i] = 0;
		rc = Dgetpath(savebuf, 0);
		b_reset();
		b_str("T25.1 Dgetpath(save,0) rc=");
		b_dec(rc);
		b_str(" cwd=\"");
		b_str(savebuf);
		b_str("\"");
		b_verdict(rc);
		b_nl();
		b_emit();
		/* 25.2 change into the QED subdir */
		rc = Dsetpath("C:\\APPS\\QED");
		b_reset();
		b_str("T25.2 Dsetpath(\"C:\\APPS\\QED\") rc=");
		b_dec(rc);
		b_verdict(rc);
		b_nl();
		b_emit();
		/* 25.3 confirm the CWD actually changed */
		for (i = 0; i < sizeof(nowbuf); i++) nowbuf[i] = 0;
		rc = Dgetpath(nowbuf, 0);
		b_reset();
		b_str("T25.3 Dgetpath(now,0) rc=");
		b_dec(rc);
		b_str(" cwd=\"");
		b_str(nowbuf);
		b_str("\"");
		b_verdict(rc);
		b_nl();
		b_emit();
		/* 25.4 relative RSC open from the subdir */
		rc = Fopen("qed.rsc", 0);
		b_reset();
		b_str("T25.4 Fopen(\"qed.rsc\",0) rel rc/handle=");
		b_dec(rc);
		b_verdict(rc);
		b_nl();
		b_emit();
		if (rc >= 0) (void)Fclose((short)rc);
		/* 25.5 relative NAP open */
		rc2 = Fopen("QED.NAP", 0);
		b_reset();
		b_str("T25.5 Fopen(\"QED.NAP\",0) rel rc/handle=");
		b_dec(rc2);
		b_verdict(rc2);
		b_nl();
		b_emit();
		if (rc2 >= 0) (void)Fclose((short)rc2);

		/* T26 -- actually READ the RSC header (16 bytes) to confirm we can
		 *        read content, not just open. rsh_vrsn is the first WORD. */
		before("T26 Fopen C:\\qed.rsc + Fread 16 bytes (RSC header)");
		rc = Fopen("C:\\qed.rsc", 0);
		b_reset();
		if (rc >= 0) {
			long got;
			for (i = 0; i < sizeof(hbuf); i++) hbuf[i] = 0;
			got = Fread((short)rc, 16, (long)hbuf);
			(void)Fclose((short)rc);
			b_str("T26   Fread=");
			b_dec(got);
			b_str(" hdr=");
			for (i = 0; i < 16; i++) { b_hh(hbuf[i]); b_ch(' '); }
			b_verdict(got);
		} else {
			b_str("T26   Fopen rc=");
			b_dec(rc);
			b_verdict(rc);
		}
		b_nl();
		b_emit();

		/* T27 -- RESTORE the CWD (critical). The GEMDOS current directory is
		 *        GLOBAL in single-tasking TOS, so leaving it at C:\APPS\QED
		 *        would leak into the next AUTO program AND the desktop. If the
		 *        saved path was empty/root, restore to "\". */
		before("T27 restore CWD via Dsetpath(saved)");
		if (savebuf[0] == 0) { savebuf[0] = '\\'; savebuf[1] = 0; }
		rc = Dsetpath(savebuf);
		b_reset();
		b_str("T27   Dsetpath(\"");
		b_str(savebuf);
		b_str("\") rc=");
		b_dec(rc);
		b_verdict(rc);
		b_nl();
		b_emit();
	}

	/* ================================================================ *
	 *  S1 stage: the four AES/BIOS server corrections                  *
	 * ================================================================ */

	/* ---- T28 (S1b): appl_init -> ap_private == rs_trees[DESKTOP][ROOT]
	 *      .ob_spec. 0x1143 (4419) with no #Q line in C:\NEWDESK.INF;
	 *      0x1100 | patcol when the #Q line supplies one (only the LOW
	 *      byte is replaced -- EmuTOS geminit.c:635-641). A real TT
	 *      running EmuTOS measures 0x1143. ---- */
	before("T28 appl_init() -> ap_private (AES opcode 10)");
	{
		short apid;
		unsigned long apriv;
		int i;
		for (i = 0; i < 16; i++) aes_global[i] = 0;
		apid  = aes_call(10, 0, 1, 0);
		apriv = ap_private_of_global();
		b_reset();
		b_str("T28   ap_id = ");
		b_dec(apid);
		b_str("  ap_private = ");
		b_hex(apriv, 4);
		b_str("  [expect 0x1143 without #Q, 0x11xx with]");
		if ((apriv & 0xFFFFFF00UL) == 0x1100UL)
			b_str("  [OK]");
		else
			b_str("  [FAIL: high 24 bits are not 0x1100]");
		b_nl();
		b_emit();
		ck_bool(G_AES, "T28 appl_init ap_private high 24 bits == 0x1100",
			(apriv & 0xFFFFFF00UL) == 0x1100UL);
	}

	/* ---- T29 (S1c): the DTA lives in the running process's basepage
	 *      (p_xdta), so a Pexec'd child cannot move the parent's.
	 *      C:\DTACHILD.NCP does Fsetdta(0x2000) and exits. ---- */
	before("T29 Fsetdta(buf); Pexec(C:\\DTACHILD.NCP); Fgetdta()==buf");
	{
		static char dtabuf[44];
		unsigned long want = (unsigned long)(void *)dtabuf;
		unsigned long got0, got1;
		long prc;

		Fsetdta(dtabuf);
		got0 = (unsigned long)Fgetdta();
		b_reset();
		b_str("T29.1 Fsetdta/Fgetdta round-trip: set=");
		b_hex(want, 4);
		b_str(" got=");
		b_hex(got0, 4);
		if (got0 == want) b_str("  [OK]"); else b_str("  [FAIL]");
		b_nl();
		b_emit();
		ck_bool(G_GEMDOS, "T29.1 Fsetdta/Fgetdta round-trip", got0 == want);

		prc = Pexec(0, "C:\\DTACHILD.NCP", "", 0L);
		b_reset();
		b_str("T29.2 Pexec(0,C:\\DTACHILD.NCP) rc=");
		b_dec(prc);
		b_nl();
		b_emit();

		got1 = (unsigned long)Fgetdta();
		b_reset();
		b_str("T29.3 parent Fgetdta after child = ");
		b_hex(got1, 4);
		if (prc < 0) {
			b_str("  [child did not run -- T29.3 inconclusive]");
		} else if (got1 == want) {
			b_str("  [OK: child did not disturb the parent DTA]");
		} else {
			b_str("  [FAIL: expected ");
			b_hex(want, 4);
			b_str("]");
		}
		b_nl();
		b_emit();
		if (prc < 0)
			ck_skip(G_GEMDOS, "T29.3 Pexec child cannot move the parent DTA",
				"child did not run");
		else
			ck_bool(G_GEMDOS, "T29.3 Pexec child cannot move the parent DTA",
				got1 == want);
	}

	/* ---- T30 (S1d): Keytbl(-1,-1,-1) returns a struct keytbl the caller
	 *      can DEREFERENCE from EL0 (EmuTOS's desktop does exactly that,
	 *      desk/deskmain.c:1144-1145). shft[0x1E] is the shifted form of
	 *      the 'a' key = 'A' on the US layout. ---- */
	before("T30 k = Keytbl(-1,-1,-1); read k->shft[0x1E] from EL0");
	{
		KEYTBL *k = (KEYTBL *)Keytbl(-1L, -1L, -1L);
		b_reset();
		b_str("T30.1 Keytbl returned ");
		b_hex((unsigned long)(void *)k, 4);
		b_nl();
		b_emit();
		before("T30.2 dereference k->shft / k->norm");
		b_reset();
		if (k == 0) {
			b_str("T30.2 NULL -- no keyboard translator on this board");
		} else {
			unsigned char sh = k->shft[0x1E];
			unsigned char no = k->norm[0x1E];
			b_str("T30.2 norm[0x1E]='");
			b_ch((char)(no ? no : '?'));
			b_str("' shft[0x1E]='");
			b_ch((char)(sh ? sh : '?'));
			b_str("' (");
			b_hex(sh, 2);
			b_str(") features=");
			b_dec(k->features);
			if (sh == 'A') b_str("  [OK]"); else b_str("  [FAIL: expected 'A']");
		}
		b_nl();
		b_emit();
		if (k == 0)
			ck_skip(G_AES, "T30.2 Keytbl shft[0x1E] == 'A'",
				"no keyboard translator on this board");
		else
			ck_bool(G_AES, "T30.2 Keytbl shft[0x1E] == 'A'",
				k->shft[0x1E] == 'A');
	}

	/* ---- T31 (S1a): menu_bar(NULL, 0) must be a legal call -- EmuTOS's
	 *      mn_bar never touches `tree` on the hide path (gemmnlib.c:
	 *      724-728). Afterwards menu_bar(tree, 1) must draw again; the
	 *      visible proof is the black separator rule at y = 18 across the
	 *      full screen width. ---- */
	before("T31 menu_bar(NULL,0) then menu_bar(tree,1)");
	{
		short r0, r1;
		aes_addrin[0] = 0L;               /* NULL tree */
		aes_intin[0]  = 0;                /* hide */
		r0 = aes_call(30, 1, 1, 1);
		b_reset();
		b_str("T31.1 menu_bar(NULL,0) returned ");
		b_dec(r0);
		if (r0 == 1) b_str("  [OK: survived, reports 1]");
		else b_str("  [FAIL: expected 1]");
		b_nl();
		b_emit();
		ck_bool(G_AES, "T31.1 menu_bar(NULL,0) returns 1", r0 == 1);

		aes_addrin[0] = (long)(void *)mbar_tree;
		aes_intin[0]  = 1;                /* show */
		r1 = aes_call(30, 1, 1, 1);
		b_reset();
		b_str("T31.2 menu_bar(tree,1) returned ");
		b_dec(r1);
		b_str("  (look for the black rule at y=18)");
		if (r1 == 1) b_str("  [OK]"); else b_str("  [FAIL: expected 1]");
		b_nl();
		b_emit();
		ck_bool(G_AES, "T31.2 menu_bar(tree,1) returns 1", r1 == 1);
	}

	/* ---- T32: the six basepage segment fields (TASK/T05). EmuTOS fills
	 *      them from the PRG header in bdos/kpgmld.c:123-145: p_tbase = the
	 *      first byte after the PD, p_dbase = p_tbase + p_tlen (:132),
	 *      p_bbase = p_tbase + tlen + dlen (:131), p_blen = the BSS size;
	 *      the free TPA starts at p_bbase + p_blen (crtinit.c's Mshrink
	 *      idiom sums exactly these). On NewTOS the loader takes them from
	 *      the two PT_LOAD segments sdk/newtos.ld emits (text R-X, data RW-
	 *      with .bss as its memsz-only tail), so p_bbase + p_blen must be
	 *      the linker's _end. `_base` is what mintlib's readers dereference
	 *      (getppid.c, gmon.c, crtinit.c); sdk/libc/newtos-arch/heap_init.c
	 *      sets it from the basepage crt0 hands it. ---- */
	before("T32 BasePage p_tbase/p_tlen/p_dbase/p_dlen/p_bbase/p_blen vs _end");
	{
		unsigned long tbase = BP_U32(0x08UL);
		unsigned long tlen  = BP_U32(0x0CUL);
		unsigned long dbase = BP_U32(0x10UL);
		unsigned long dlen  = BP_U32(0x14UL);
		unsigned long bbase = BP_U32(0x18UL);
		unsigned long blen  = BP_U32(0x1CUL);
		unsigned long end   = (unsigned long) _end;

		b_reset();
		b_str("T32   p_tbase=");
		b_hex(tbase, 8);
		b_str(" p_tlen=");
		b_hex(tlen, 8);
		b_str(" p_dbase=");
		b_hex(dbase, 8);
		b_str(" p_dlen=");
		b_hex(dlen, 8);
		b_nl();
		b_emit();
		b_reset();
		b_str("T32   p_bbase=");
		b_hex(bbase, 8);
		b_str(" p_blen=");
		b_hex(blen, 8);
		b_str(" _end=");
		b_hex(end, 8);
		b_str(" _base=");
		b_hex((unsigned long) _base, 8);
		b_nl();
		b_emit();

		ck_bool(G_GEMDOS, "T32.1 p_tbase < p_dbase", tbase < dbase);
		ck_bool(G_GEMDOS, "T32.2 p_dbase <= p_bbase", dbase <= bbase);
		ck_hex(G_GEMDOS, "T32.3 p_bbase + p_blen == _end", bbase + blen, end);
		ck_hex(G_GEMDOS, "T32.4 p_dbase == p_tbase + p_tlen (kpgmld.c:132)",
		       dbase, tbase + tlen);
		ck_hex(G_GEMDOS, "T32.5 p_bbase == p_dbase + p_dlen (kpgmld.c:131)",
		       bbase, dbase + dlen);
		ck_bool(G_GEMDOS, "T32.6 p_tlen, p_dlen, p_blen all nonzero",
			tlen != 0 && dlen != 0 && blen != 0);
		ck_hex(G_GEMDOS, "T32.7 _base == p_lowtpa (mintlib BASEPAGE *)",
		       (unsigned long) _base, (unsigned long) BP_U32(0x00UL));
	}

	/* ---- T33 (PROC06): Pexec returns the child's exit code as a UWORD
	 *      zero-extended into the LONG -- EmuTOS xterm(UWORD rc) stores
	 *      run->p_dreg[0] = rc (bdos/proc.c:601,616), so the parent sees
	 *      rc & 0xFFFF, never a negative value
	 *      (docs/api/gemdos/process/4c-pterm.md, "Exit-code matrix").
	 *      C:\PTERMCH.NCP (sdk/examples/ptermch.c) ends with the code its
	 *      one-character command tail selects. Only "child not on the
	 *      card" (EFILNF/EPTHNF) is a skip; any other negative result is
	 *      the defect this case pins. ---- */
	before("T33 Pexec(0,C:\\PTERMCH.NCP,m|h|z): child exit code zero-extended");
	{
		static const struct { const char *tail; const char *name; long want; } t33[] = {
			{ "\001m", "T33.1 child Pterm(-1) -> Pexec == 65535", 65535L },
			{ "\001h", "T33.2 child Pterm(0x8000) -> Pexec == 32768", 32768L },
			{ "\001z", "T33.3 child Pterm0() -> Pexec == 0", 0L },
		};
		unsigned int i;

		for (i = 0; i < sizeof t33 / sizeof t33[0]; i++) {
			long prc = Pexec(0, "C:\\PTERMCH.NCP", t33[i].tail, 0L);
			if (prc == -33L || prc == -34L)
				ck_skip(G_GEMDOS, t33[i].name, "C:\\PTERMCH.NCP not on the card");
			else
				ck_eq(G_GEMDOS, t33[i].name, prc, t33[i].want);
		}
	}

	/* ================================================================ *
	 *  SUPER group -- Super (GEMDOS 0x20) / Supexec (XBIOS 0x26)       *
	 *                                                                  *
	 *  Contract source: EmuTOS bdos/rwa.S -- the TRAP #1 front end     *
	 *  `_enter` (:159-182) intercepts function 0x20 BEFORE the BDOS     *
	 *  table, and x20_usr (:277-296) / x20_sup (:300-330) / x20_inq     *
	 *  (:334-339) do the work; bios/vectors.S `_supexec` (:1376) is     *
	 *  `movea.l 4(a7),a0 / jmp (a0)`.                                   *
	 *                                                                  *
	 *  On NewTOS an application runs at EL0 and can never be raised to  *
	 *  EL1, but it does not need to be: the $400 system-variable area   *
	 *  is materialised in its OWN address space. So the observable      *
	 *  contract is "the inquiry tells the truth (user mode), the        *
	 *  bracket is a no-op that round-trips, and Supexec really calls    *
	 *  the function and returns its LONG".                              *
	 * ================================================================ */
	before("SX Super/Supexec (GEMDOS 0x20 / XBIOS 0x26)");
	{
		long inq, tok, inq_in, sxr, jaradr;
		unsigned long sysbase, ver, ss_val;
		unsigned long mirror_val = 0, jar_val = 0;
		long ss_rc;

		/* SX01: Super(1L) is the inquiry. EmuTOS x20_inq returns 0L for a
		 * user-mode caller; NewTOS answers it from the process's super
		 * state (docs/api/_super-state.md S3, TASK/MEM12), off at load. */
		inq = Super(1L);
		ck_eq(G_SUPER, "SX01 Super(1L) inquiry reports user mode (0)", inq, 0L);

		/* SX02: Super(0L) must hand back a usable token. Every caller feeds
		 * it straight back into Super(token) to close the bracket, so the
		 * only hard requirement is "neither 0 nor 1" -- 0 would re-open the
		 * bracket, 1 would be read as the inquiry. An error code (-32) is
		 * not a token. */
		tok = Super(0L);
		b_reset();
		b_str("SX    Super(0L) token = ");
		b_hex((unsigned long)tok, 8);
		b_nl();
		b_emit();
		ck_bool(G_SUPER, "SX02 Super(0L) returns a token, not an error",
			tok > 1L);

		/* SX03: inside the bracket the inquiry says super state, -1 --
		 * EmuTOS x20_inq in supervisor mode (bdos/rwa.S:340-346). Until
		 * TASK/MEM12 NewTOS had no such state and this expected 0. */
		inq_in = Super(1L);
		ck_eq(G_SUPER, "SX03 Super(1L) inside the bracket says -1 (super state)",
		      inq_in, -1L);

		/* SX04: the TeraDesk get_tosversion() shape, verbatim
		 * (library/utility/gettos.c:38-44): Super(NULL), read
		 * _sysbase->os_version, Super(stack). Must yield 0x0404. */
		sysbase = RD_U32(0x4F2UL);
		ver = el0_readable(sysbase) ? RD_U16(sysbase + 2) : 0;
		Super((void *)tok);              /* close the bracket */
		b_reset();
		b_str("SX    _sysbase=");
		b_hex(sysbase, 8);
		b_str(" os_version=");
		b_hex(ver, 4);
		b_nl();
		b_emit();
		ck_hex(G_SUPER, "SX04 gettos.c shape: os_version via Super bracket",
		       ver, 0x0404UL);

		/* SuperToUser() -- mintlib's stack-safe way to close the bracket --
		 * is retargeted in the same header, but it is deliberately NOT
		 * probed here: its m68k form does not COMPILE for AArch64, so a
		 * probe would make this program unbuildable in the before-state and
		 * destroy the controlled before/after comparison. Its proof is the
		 * libc build instead: sdk/libc/build/failed.txt loses
		 * mintlib/setsysvar.c and unix/sysinfo.c, which failed on nothing
		 * but that macro ("unknown register name 'a0' in asm").
		 *
		 * SX05/SX06: Supexec must CALL the function and return its LONG
		 * (EmuTOS `jmp (a0)`). Before the binding was retargeted XBIOS 0x26
		 * answered with its own function number, 38, and the callee never
		 * ran -- both halves are checked separately so the log says which. */
		sx_calls = 0;
		sxr = Supexec(sx_probe);
		b_reset();
		b_str("SX    Supexec(sx_probe) = ");
		b_hex((unsigned long)sxr, 8);
		b_str("  calls=");
		b_dec(sx_calls);
		b_nl();
		b_emit();
		ck_hex(G_SUPER, "SX05 Supexec returns the callee's LONG",
		       (unsigned long)sxr, (unsigned long)SX_MAGIC);
		ck_eq(G_SUPER, "SX06 Supexec really ran the callee", sx_calls, 1L);

		/* SX07: the cflib getcookie() shape (cflib-21/cookie.c:76):
		 * Supexec(get_cookie_adr) must deliver the _p_cookies value. */
		jaradr = Supexec(sx_cookie_adr);
		b_reset();
		b_str("SX    Supexec(get_cookie_adr) = ");
		b_hex((unsigned long)jaradr, 8);
		b_str("  [$5A0 holds ");
		b_hex((unsigned long)RD_U32(SV_PCOOKIES), 8);
		b_str("]");
		b_nl();
		b_emit();
		ck_hex(G_SUPER, "SX07 Supexec(get_cookie_adr) == _p_cookies",
		       (unsigned long)jaradr, (unsigned long)RD_U32(SV_PCOOKIES));

		/* SX08: and the jar it points at must carry the NewTOS identity
		 * cookie. Guarded: before the retarget `jaradr` was 38, which
		 * el0_readable() rejects, so this reports SKIP instead of faulting. */
		if (!el0_readable((unsigned long)jaradr)) {
			ck_skip(G_SUPER, "SX08 jar via Supexec carries NTOS=0x100",
				"Supexec did not return a readable pointer");
		} else if (!sx_jar_find((unsigned long)jaradr, TAG_NTOS, &jar_val)) {
			ck_bool(G_SUPER, "SX08 jar via Supexec carries NTOS=0x100", 0);
		} else {
			ck_hex(G_SUPER, "SX08 jar via Supexec carries NTOS=0x100",
			       jar_val, NTOS_VERSION);
		}

		/* SX09/SX10: does the per-process $5A0 MIRROR agree with the jar the
		 * kernel itself owns? The mirror is written once, when the program is
		 * loaded (loader.rs `populate_sysvars`); Ssystem(S_GETCOOKIE) reads
		 * the live kernel jar. Equal values mean the snapshot is still
		 * current for this process -- the question TASK/X01 asks. */
		(void)sx_jar_find((unsigned long)RD_U32(SV_PCOOKIES), TAG_NTOS,
				  &mirror_val);
		ss_val = 0;
		ss_rc = SSYSTEM(S_GETCOOKIE, (long)TAG_NTOS, (long)&ss_val);
		b_reset();
		b_str("SX    NTOS: $5A0 mirror=");
		b_hex(mirror_val, 8);
		b_str("  Ssystem rc=");
		b_dec(ss_rc);
		b_str(" val=");
		b_hex(ss_val, 8);
		b_nl();
		b_emit();
		ck_eq(G_SUPER, "SX09 Ssystem(S_GETCOOKIE,NTOS) succeeds", ss_rc, 0L);
		ck_hex(G_SUPER, "SX10 $5A0 mirror agrees with the kernel jar",
		       mirror_val, ss_val);
	}

	/* SX11..SX15: is the mirror a LIVE view or a load-time snapshot?
	 * SX09/SX10 above only compare cookies that existed before this program
	 * was loaded, so they pass either way. These five set a cookie now,
	 * while the program is already running, and look for it at $5A0 -- the
	 * question docs/api/bios/_cookie-jar.md leaves open under "OFFEN". */
	{
		unsigned long jar, v = 0;
		long set_rc, del_rc;
		int found_after_set, found_after_del;

		jar = RD_U32(SV_PCOOKIES);

		set_rc = SSYSTEM(S_SETCOOKIE, (long)TAG_APIT, (long)APIT_VALUE);
		found_after_set = sx_jar_find(jar, TAG_APIT, &v);
		b_reset();
		b_str("SX    Setcookie('APIT') rc=");
		b_dec(set_rc);
		b_str("  mirror ");
		b_str(found_after_set ? "carries " : "MISSES it, val=");
		b_hex(v, 8);
		b_nl();
		b_emit();
		ck_eq(G_SUPER, "SX11 Ssystem(S_SETCOOKIE,'APIT') succeeds", set_rc, 0L);
		ck_bool(G_SUPER, "SX12 $5A0 mirror sees a cookie set after load",
			found_after_set);
		ck_hex(G_SUPER, "SX13 that mirrored cookie carries the right value",
		       v, APIT_VALUE);

		del_rc = SSYSTEM(S_DELCOOKIE, (long)TAG_APIT, 0L);
		found_after_del = sx_jar_find(jar, TAG_APIT, (unsigned long *)0);
		ck_bool(G_SUPER, "SX14 Ssystem(S_DELCOOKIE) removes it from the mirror",
			del_rc == 0L && !found_after_del);

		/* Whatever the refresh does, the array must still end in the classic
		 * {0, capacity} pair -- otherwise the scan above walks off the jar. */
		ck_hex(G_SUPER, "SX15 mirror still ends in the {0,capacity} terminator",
		       sx_jar_term(jar), 16UL);
	}

	/* SX16/SX17: the jar's WIRE format against a 1:1-ported reader's idea of
	 * it. A TOS cookie slot is a pair of 32-bit LONGs -- 8 bytes -- and that
	 * is what the kernel writes. A ported program that declares
	 * `struct { long name; long value; }` gets 16 bytes per element on LP64
	 * and therefore reads every SECOND slot, with the tag and the previous
	 * value fused into one 8-byte word. This is the same ABI property as the
	 * 8-byte read of $5A0 itself (T05); it is measured here, not fixed --
	 * see docs/api/bios/_cookie-jar.md. */
	{
		unsigned long jar = RD_U32(SV_PCOOKIES);
		COOKIE_AS_PORTED *c = (COOKIE_AS_PORTED *)jar;
		int steps = 0, found = 0;

		b_reset();
		b_str("SX    slot pitch: wire=8  sizeof(ported COOKIE)=");
		b_dec((long)sizeof(COOKIE_AS_PORTED));
		b_str("  jar[0] as 8 bytes=");
		b_hex((unsigned long)c->name, 16);
		b_nl();
		b_emit();

		/* TeraDesk's find_cookie loop (library/utility/cookie.c:58-62),
		 * verbatim but bounded so a runaway scan cannot fault the probe. */
		while (c->name != 0 && c->name != (long)TAG_NTOS && steps < 16) {
			c++;
			steps++;
		}
		found = (c->name == (long)TAG_NTOS);

		b_reset();
		b_str("SX    ported-stride scan: steps=");
		b_dec(steps);
		b_str("  stopped at 0x");
		b_hex((unsigned long)(void *)c, 8);
		b_str("  found=");
		b_dec(found);
		b_nl();
		b_emit();

		/* The wire format itself: slot 1 begins 8 bytes after slot 0. */
		ck_hex(G_SUPER, "SX16 jar slot pitch is 8 bytes (slot 1 tag == _MCH)",
		       RD_U32(jar + 8), TAG_MCH);
		/* And the documented consequence for a 1:1-ported LP64 reader. */
		ck_bool(G_SUPER,
			"SX17 LP64 struct-stride scan misses the jar (ABI note, not a defect)",
			!found);
	}

	/* ================================================================ *
	 *  BIOS group -- trap 13, function ids 0x00..0x0B.                 *
	 *                                                                  *
	 *  Contract source: docs/api/bios/ (one .md per function),         *
	 *  section "Verification criteria" + "Error conditions", with the  *
	 *  NewTOS device/drive map from each file's "Pi / BCM2711 binding" *
	 *  section. Every function id the dispatcher wires                 *
	 *  (tos-api/src/bios/mod.rs) is exercised.                         *
	 *                                                                  *
	 *  READ-ONLY BY DESIGN: not one probe below writes to a disk. The  *
	 *  Rwabs write path (r_w bit 0) is deliberately NOT exercised --   *
	 *  this program runs from the user's real SD card.                 *
	 * ================================================================ */

	/* ---- B1: character I/O -- Bconstat / Bconin / Bconout / Bcostat ----
	 *
	 * Handle map per docs/api/bios/{01,02,03,08}-*.md "Pi / BCM2711
	 * binding": CON (2) is the only input device; CON (2) and raw-CON (5)
	 * are the output devices; PRT (0), MIDI (3), IKBD-TX (4) and 6/7 are
	 * absent and follow EmuTOS' char_dummy / charout_dummy slots. AUX (1)
	 * is "port present, no cable" (docs/api/gemdos/file/3f-fread.md item
	 * 1a): Bconout(1) accepts+discards -> 1, Bcostat(1) -> -1 (EmuTOS
	 * bconout1/bcostat1 with an MFP port), Bconstat(1) -> 0.
	 * Bcostat is deliberately NOT symmetric with Bconout: bcostat_init[5]
	 * is char_dummy (references/emutos/bios/chardev.c:38-41), so handle 5
	 * writes (returns 1) but reports busy (returns 0).
	 *
	 * Bconin is only ever called on handles the source proves cannot
	 * block (the char_dummy slots), plus handle 2 guarded by Bconstat.
	 * Handle 1 (AUX) is left out on purpose: EmuTOS' bconin1 blocks on the
	 * RS-232 receiver, and probing a handle whose EmuTOS-side routine
	 * blocks is not worth the risk of hanging the machine. */
	before("B1 BIOS character I/O (Bconstat/Bconin/Bconout/Bcostat)");
	{
		int i;
		long k;

		/* Drain type-ahead so "no data queued" is a defined state. Bconin
		 * runs only after Bconstat has just reported a byte, so it cannot
		 * block; the loop is bounded regardless. */
		for (i = 0; i < 256 && Bconstat(2); i++)
			(void)Bconin(2);

		ck_eq(G_BCHR, "BC01 Bconstat(2) CON drained ->0",  Bconstat(2), 0);
		ck_eq(G_BCHR, "BC02 Bconstat(0) PRT ->0",          Bconstat(0), 0);
		ck_eq(G_BCHR, "BC03 Bconstat(1) AUX ->0",          Bconstat(1), 0);
		ck_eq(G_BCHR, "BC04 Bconstat(3) MIDI ->0",         Bconstat(3), 0);
		ck_eq(G_BCHR, "BC05 Bconstat(4) dummy ->0",        Bconstat(4), 0);
		ck_eq(G_BCHR, "BC06 Bconstat(9) invalid ->0",      Bconstat(9), 0);

		ck_eq(G_BCHR, "BC07 Bconin(0) PRT ->0",            Bconin(0), 0);
		ck_eq(G_BCHR, "BC08 Bconin(3) MIDI ->0",           Bconin(3), 0);
		ck_eq(G_BCHR, "BC09 Bconin(4) dummy ->0",          Bconin(4), 0);
		ck_eq(G_BCHR, "BC10 Bconin(9) invalid ->0",        Bconin(9), 0);

		/* The console-output handles really put their character on the
		 * screen. All eight calls run back to back, so the marker line
		 * ends up showing exactly the bytes that reach the screen: "CR" from
		 * handles 2 and 5, nothing from the five absent devices and nothing
		 * from AUX (accepts and discards). */
		b_reset();
		b_str("B1    Bconout(0..7) write here -> ");
		b_emit();
		{
			long o2 = Bconout(2, 'C');
			long o5 = Bconout(5, 'R');
			long o0 = Bconout(0, '*');
			long o1 = Bconout(1, '*');
			long o3 = Bconout(3, '*');
			long o4 = Bconout(4, '*');
			long o6 = Bconout(6, '*');
			long o7 = Bconout(7, '*');
			emit("\r\n");
			ck_eq(G_BCHR, "BC11 Bconout(2,'C') CON ->1",   o2, 1);
			ck_eq(G_BCHR, "BC12 Bconout(5,'R') raw ->1",   o5, 1);
			ck_eq(G_BCHR, "BC13 Bconout(0,'*') PRT ->0",   o0, 0);
			ck_eq(G_BCHR, "BC14 Bconout(1,'*') AUX ->1",   o1, 1);
			ck_eq(G_BCHR, "BC15 Bconout(3,'*') MIDI ->0",  o3, 0);
			ck_eq(G_BCHR, "BC16 Bconout(4,'*') IKBD ->0",  o4, 0);
			ck_eq(G_BCHR, "BC17 Bconout(6,'*') dummy ->0", o6, 0);
			ck_eq(G_BCHR, "BC18 Bconout(7,'*') dummy ->0", o7, 0);
		}

		ck_eq(G_BCHR, "BC19 Bcostat(2) CON ->-1",      Bcostat(2), -1);
		ck_eq(G_BCHR, "BC20 Bcostat(5) dummy ->0",     Bcostat(5), 0);
		ck_eq(G_BCHR, "BC21 Bcostat(0) PRT ->0",       Bcostat(0), 0);
		ck_eq(G_BCHR, "BC22 Bcostat(1) AUX ->-1",      Bcostat(1), -1);
		ck_eq(G_BCHR, "BC23 Bcostat(3) IKBD-TX ->0",   Bcostat(3), 0);
		ck_eq(G_BCHR, "BC24 Bcostat(4) MIDI-TX ->0",   Bcostat(4), 0);
		ck_eq(G_BCHR, "BC25 Bcostat(9) invalid ->0",   Bcostat(9), 0);

		/* Bconin(2): the packed-LONG contract (docs/api/bios/02-bconin.md
		 * "Return value") can only be checked against a real key. Bounded
		 * wait -> no key means SKIP, never a hang. Tap ONE key: holding it
		 * lets the auto-repeat queue more and BC30 then legitimately fails. */
		emit("B1    tap one key within 10 s to test Bconin(2) (or wait) ...\r\n");
		if (wait_input(10 * 200)) {
			/* Captured back to back: no console or file I/O may run
			 * between them, or a second keystroke would decide BC30. */
			long s1 = Bconstat(2);
			long s2 = Bconstat(2);
			long s3;
			k  = Bconin(2);
			s3 = Bconstat(2);

			b_reset();
			b_str("B1    Bconin(2) = ");
			b_hex((unsigned long)k, 8);
			b_str("  (scancode ");
			b_hex((k >> 16) & 0xFF, 2);
			b_str(", ascii ");
			b_hex(k & 0xFF, 2);
			b_str(")");
			b_nl();
			b_emit();

			ck_bool(G_BCHR, "BC26 Bconstat(2) ready, non-destructive",
				s1 == -1 && s2 == -1);
			ck_eq(G_BCHR, "BC27 Bconin(2) bits 8-15 zero",  (k >> 8) & 0xFF, 0);
			ck_eq(G_BCHR, "BC28 Bconin(2) bits 24-31 zero", (k >> 24) & 0xFF, 0);
			ck_bool(G_BCHR, "BC29 Bconin(2) ascii or scancode set",
				(k & 0xFF) != 0 || ((k >> 16) & 0xFF) != 0);
			ck_eq(G_BCHR, "BC30 Bconstat(2)==0 after Bconin", s3, 0);
		} else {
			ck_skip(G_BCHR, "BC26 Bconstat(2) ready, non-destructive", "no key pressed");
			ck_skip(G_BCHR, "BC27 Bconin(2) bits 8-15 zero",  "no key pressed");
			ck_skip(G_BCHR, "BC28 Bconin(2) bits 24-31 zero", "no key pressed");
			ck_skip(G_BCHR, "BC29 Bconin(2) ascii or scancode set", "no key pressed");
			ck_skip(G_BCHR, "BC30 Bconstat(2)==0 after Bconin", "no key pressed");
		}
	}

	/* ---- B2: block devices -- Drvmap / Mediach / Rwabs / Getbpb ----
	 *
	 * docs/api/bios/{0a,09,04,07}-*.md. Reads only. */
	before("B2 BIOS block devices (Drvmap/Mediach/Rwabs/Getbpb)");
	{
		static unsigned char boot[512];    /* logical sector 0 of the drive */
		static unsigned char mbr[512];     /* physical sector 0             */
		static unsigned char again[512];   /* physical sector at part start */
		static unsigned char guard[512];   /* numb==0 no-transfer sentinel  */
		unsigned long dm = (unsigned long)Drvmap();
		int bdrv = -1, udrv = -1;
		int d, i, ok;
		long r;

		for (d = 0; d < 16; d++) {
			if ((dm >> d) & 1UL) { if (bdrv < 0) bdrv = d; }
			else                 { if (udrv < 0) udrv = d; }
		}

		b_reset();
		b_str("B2    Drvmap=");
		b_hex(dm, 8);
		b_str(" first mounted drive=");
		b_dec(bdrv);
		b_nl();
		b_emit();

		ck_bool(G_BBLK, "BB01 Drvmap() has at least one drive", dm != 0);
		ck_hex(G_BBLK, "BB02 Drvmap() bits 16-31 zero", dm >> 16, 0);
		ck_hex(G_BBLK, "BB03 Drvmap() == drvbits $4C2",
		       dm, (unsigned long)RD_U32(SV_DRVBITS));

		/* Mediach: 0 for every mounted drive (the SD is non-removable),
		 * EUNDEV for every unmounted one -- and stable across calls, since
		 * nothing here arms a media change. */
		{
			int okm = 1, okv = 1, okl = 1;
			for (d = 0; d < 16; d++) {
				long m1 = Mediach(d);
				long m2 = Mediach(d);
				if (m1 != m2) okl = 0;
				if ((dm >> d) & 1UL) { if (m1 != 0)      okm = 0; }
				else                 { if (m1 != EUNDEV) okv = 0; }
			}
			ck_bool(G_BBLK, "BB04 Mediach(mounted)==0 for all", okm);
			ck_bool(G_BBLK, "BB05 Mediach(absent)==EUNDEV for all", okv);
			ck_bool(G_BBLK, "BB06 Mediach latched (2 calls equal)", okl);
		}

		if (bdrv < 0) {
			ck_skip(G_BBLK, "BB07..BB29 Rwabs/Getbpb", "no drive mounted");
		} else {
			/* Logical read of sector 0 -- the drive's FAT boot sector. */
			for (i = 0; i < 512; i++) boot[i] = 0;
			r = Rwabs(RW_READ, boot, 1, 0, bdrv);
			ck_eq(G_BBLK, "BB07 Rwabs read logical sector 0", r, E_OK);
			ck_bool(G_BBLK, "BB08 logical sector 0 has 0x55AA",
				boot[510] == 0x55 && boot[511] == 0xAA);

			/* numb == 0: no transfer, E_OK -- the source has no EBADRQ
			 * branch (docs/api/bios/04-rwabs.md "Error conditions"). */
			for (i = 0; i < 512; i++) guard[i] = 0xA5;
			r = Rwabs(RW_READ, guard, 0, 0, bdrv);
			ck_eq(G_BBLK, "BB09 Rwabs numb=0 -> E_OK", r, E_OK);
			ok = 1;
			for (i = 0; i < 512; i++) if (guard[i] != 0xA5) ok = 0;
			ck_bool(G_BBLK, "BB10 Rwabs numb=0 transferred nothing", ok);

			/* Documented error cases. */
			if (udrv < 0) {
				ck_skip(G_BBLK, "BB11 Rwabs absent drive -> EUNDEV",
					"all 16 drives mounted");
			} else {
				r = Rwabs(RW_READ, guard, 1, 0, udrv);
				ck_eq(G_BBLK, "BB11 Rwabs absent drive -> EUNDEV", r, EUNDEV);
			}
			/* Beyond the partition: the 16-bit `first` cannot reach past a
			 * multi-megabyte partition, so use the AHDI long-sector form. */
			r = Lrwabs(RW_READ, guard, 1, 0x7FFFFFFFL, bdrv);
			ck_eq(G_BBLK, "BB12 Rwabs sector past partition -> ESECNF", r, ESECNF);

			/* Physical mode: the drive argument is the physical unit
			 * (PHYS_SD_UNIT), `first` an absolute device sector, no
			 * partition translation, range-checked against the device. */
			for (i = 0; i < 512; i++) mbr[i] = 0;
			r = Rwabs(RW_NOTRANSLATE, mbr, 1, 0, PHYS_SD_UNIT);
			ck_eq(G_BBLK, "BB13 Rwabs physical sector 0 -> E_OK", r, E_OK);
			ck_bool(G_BBLK, "BB14 physical sector 0 is the MBR (0x55AA)",
				mbr[510] == 0x55 && mbr[511] == 0xAA);
			r = Lrwabs(RW_NOTRANSLATE, guard, 1, 0x7FFFFFFFL, PHYS_SD_UNIT);
			ck_eq(G_BBLK, "BB15 Rwabs physical past device -> ESECNF", r, ESECNF);

			/* Translation proof: find the MBR entry the kernel's drive
			 * table assigns to `bdrv` (primaries in MBR order from C:=2,
			 * skipping empty and container entries -- drive.rs `mount`),
			 * then read that absolute sector physically. It must be the
			 * same 512 bytes the logical read of sector 0 returned. */
			{
				int letter = 2, found = -1;
				unsigned long pstart = 0;
				for (i = 0; i < 4; i++) {
					unsigned int  kind  = mbr[0x1BE + i * 16 + 4];
					unsigned long start = le32(&mbr[0x1BE + i * 16 + 8]);
					unsigned long count = le32(&mbr[0x1BE + i * 16 + 12]);
					if (kind == 0 || count == 0) continue;
					if (is_extended_type(kind)) continue;
					if (letter == bdrv) { found = i; pstart = start; break; }
					letter++;
				}
				if (found < 0 || pstart == 0) {
					ck_skip(G_BBLK, "BB16 physical(part start) == logical sector 0",
						"drive is not an MBR primary partition");
				} else {
					for (i = 0; i < 512; i++) again[i] = 0;
					r = Lrwabs(RW_NOTRANSLATE, again, 1, (long)pstart, PHYS_SD_UNIT);
					ck_bool(G_BBLK, "BB16 physical(part start) == logical sector 0",
						r == E_OK
						&& mcmp((const char *)again, (const char *)boot, 512) == 0);
				}
			}

			/* Getbpb: the returned pointer, then every field against the
			 * boot sector we just read (EmuTOS blkdev_getbpb arithmetic). */
			{
				BPB18 *bp = (BPB18 *)Getbpb(bdrv);
				unsigned int  raw_recsiz = le16(&boot[0x0B]);
				unsigned int  raw_clsiz  = boot[0x0D];
				unsigned int  raw_res    = le16(&boot[0x0E]);
				unsigned int  raw_nfat   = boot[0x10];
				unsigned int  raw_rootent= le16(&boot[0x11]);
				unsigned int  raw_fsiz   = le16(&boot[0x16]);
				unsigned int  nfat = (raw_nfat == 0) ? 2 : raw_nfat;
				unsigned int  res  = (raw_res  == 0) ? 1 : raw_res;
				unsigned int  rdlen, fatrec, datrec;

				b_reset();
				b_str("B2    Getbpb(");
				b_dec(bdrv);
				b_str(") = ");
				b_hex((unsigned long)(void *)bp, 8);
				b_nl();
				b_emit();

				ck_bool(G_BBLK, "BB17 Getbpb(mounted) != NULL", bp != 0);
				/* The contract is "pointer to the drive's BPB" -- i.e. the
				 * CALLER reads it. A pointer an EL0 program cannot
				 * dereference does not satisfy that. */
				ck_bool(G_BBLK, "BB18 Getbpb pointer is EL0-readable",
					bp != 0 && el0_readable((unsigned long)(void *)bp));

				if (bp == 0 || !el0_readable((unsigned long)(void *)bp)) {
					ck_skip(G_BBLK, "BB19..BB28 BPB fields",
						"BPB pointer not readable from EL0");
				} else {
					rdlen  = (raw_recsiz == 0) ? 0
					       : (raw_rootent * 32u + raw_recsiz - 1u) / raw_recsiz;
					fatrec = res + ((nfat >= 2) ? raw_fsiz : 0u);
					datrec = fatrec + raw_fsiz + rdlen;

					ck_eq(G_BBLK, "BB19 BPB recsiz == 512", bp->recsiz, 512);
					ck_eq(G_BBLK, "BB20 BPB recsiz == bootsec[0x0B]",
					      bp->recsiz, (long)raw_recsiz);
					ck_eq(G_BBLK, "BB21 BPB clsiz == bootsec[0x0D]",
					      bp->clsiz, (long)raw_clsiz);
					ck_eq(G_BBLK, "BB22 BPB clsizb == recsiz*clsiz",
					      bp->clsizb, (long)bp->recsiz * (long)bp->clsiz);
					ck_eq(G_BBLK, "BB23 BPB fsiz == bootsec[0x16]",
					      bp->fsiz, (long)raw_fsiz);
					ck_eq(G_BBLK, "BB24 BPB rdlen == ceil(rootent*32/recsiz)",
					      bp->rdlen, (long)rdlen);
					ck_eq(G_BBLK, "BB25 BPB fatrec == res+(nfat>1?fsiz:0)",
					      bp->fatrec, (long)(unsigned short)fatrec);
					ck_eq(G_BBLK, "BB26 BPB datrec == fatrec+fsiz+rdlen",
					      bp->datrec, (long)(unsigned short)datrec);
					ck_bool(G_BBLK, "BB27 BPB B_16 set iff numcl > 4084",
						((bp->b_flags & B_16) != 0) == (bp->numcl > 4084));
					ck_bool(G_BBLK, "BB28 BPB B_1FAT set iff one FAT",
						((bp->b_flags & B_1FAT) != 0) == (nfat < 2));
				}
			}

			if (udrv < 0) {
				ck_skip(G_BBLK, "BB29 Getbpb(absent) == NULL",
					"all 16 drives mounted");
			} else {
				ck_bool(G_BBLK, "BB29 Getbpb(absent) == NULL",
					Getbpb(udrv) == 0);
			}
		}
	}

	/* ---- B3: system calls -- Tickcal / Getmpb / Setexc / Kbshift ----
	 *
	 * docs/api/bios/{06,00,05,0b}-*.md. */
	before("B3 BIOS system calls (Tickcal/Getmpb/Setexc/Kbshift)");
	{
		static MPBX mpb;
		MDX *md;
		unsigned int t0, t1;
		unsigned long spins;
		long q1, q2, old, prev, back;

		/* Tickcal is a pure constant 20 (ms per system tick). */
		ck_eq(G_BSYS, "BS01 Tickcal() == 20", Tickcal(), 20);

		/* Its counterpart, the 200 Hz counter the contract's tick derives
		 * from -- also the clock the Bconin wait above is bounded on. */
		t0 = RD_U32(SV_HZ200);
		t1 = t0;
		for (spins = 0; spins < 100000000UL && t1 == t0; spins++)
			t1 = RD_U32(SV_HZ200);
		ck_bool(G_BSYS, "BS02 _hz_200 $4BA advances", t1 != t0);

		/* Getmpb: mp_mfl / mp_mal written, mp_rover left alone. */
		mpb.mp_mfl   = (MDX *)0x1111UL;
		mpb.mp_mal   = (MDX *)0x2222UL;
		mpb.mp_rover = (MDX *)0xDEADUL;
		Getmpb(&mpb);
		md = mpb.mp_mfl;
		b_reset();
		b_str("B3    Getmpb: mp_mfl=");
		b_hex((unsigned long)(void *)md, 8);
		b_str(" mp_mal=");
		b_hex((unsigned long)(void *)mpb.mp_mal, 2);
		b_nl();
		b_emit();
		ck_bool(G_BSYS, "BS03 Getmpb leaves mp_rover untouched",
			mpb.mp_rover == (MDX *)0xDEADUL);
		ck_bool(G_BSYS, "BS04 Getmpb mp_mal == NULL", mpb.mp_mal == 0);
		ck_bool(G_BSYS, "BS05 Getmpb mp_mfl != NULL", md != 0);
		/* Same reasoning as Getbpb: the contract is that the caller walks
		 * the free list, which requires a readable pointer. */
		ck_bool(G_BSYS, "BS06 Getmpb mp_mfl is EL0-readable",
			md != 0 && el0_readable((unsigned long)(void *)md));
		if (md == 0 || !el0_readable((unsigned long)(void *)md)) {
			ck_skip(G_BSYS, "BS07..BS10 MD fields",
				"MD pointer not readable from EL0");
		} else {
			unsigned int membot = RD_U32(SV_MEMBOT);
			unsigned int memtop = RD_U32(SV_MEMTOP);
			ck_bool(G_BSYS, "BS07 MD m_link == NULL", md->m_link == 0);
			ck_bool(G_BSYS, "BS08 MD m_own == NULL",  md->m_own  == 0);
			ck_hex(G_BSYS, "BS09 MD m_start == membot $432",
			       (unsigned long)md->m_start, (unsigned long)membot);
			ck_hex(G_BSYS, "BS10 MD m_length == memtop-membot",
			       (unsigned long)md->m_length,
			       (unsigned long)(memtop - membot));
		}

		/* Setexc, query form: the etv_* hooks carry a non-NULL boot default
		 * so the `old = Setexc(n,-1)` chain idiom always has something to
		 * chain to, and a query must not change the value. Read-only: the
		 * live hooks are never written here. */
		q1 = SETEXC(0x102, -1L);      /* etv_term */
		q2 = SETEXC(0x102, -1L);
		b_reset();
		b_str("B3    Setexc(etv_term,-1) = ");
		b_hex((unsigned long)q1, 8);
		b_nl();
		b_emit();
		ck_bool(G_BSYS, "BS11 Setexc(etv_term,-1) != 0", q1 != 0);
		ck_bool(G_BSYS, "BS12 Setexc query does not change it", q1 == q2);

		/* Setexc, set form, on a shadow-only vector (see SHADOW_VEC). */
		old  = SETEXC(SHADOW_VEC, -1L);
		prev = SETEXC(SHADOW_VEC, SHADOW_MAGIC);
		ck_bool(G_BSYS, "BS13 Setexc(set) returns the previous value",
			prev == old);
		ck_hex(G_BSYS, "BS14 Setexc(query) reads the new value",
		       (unsigned long)SETEXC(SHADOW_VEC, -1L),
		       (unsigned long)SHADOW_MAGIC);
		(void)SETEXC(SHADOW_VEC, old);
		back = SETEXC(SHADOW_VEC, -1L);
		ck_bool(G_BSYS, "BS15 Setexc restored the old value", back == old);

		/* The cookie-jar idiom: Setexc(0x5A0/4, -1) peeks the sysvar. */
		ck_hex(G_BSYS, "BS16 Setexc($5A0/4,-1) == *(long*)$5A0",
		       (unsigned long)SETEXC(SV_PCOOKIES / 4, -1L),
		       (unsigned long)RD_U32(SV_PCOOKIES));

		/* Kbshift: query is side-effect free, set returns the previous
		 * value, only the low byte of the argument is stored. The whole
		 * sequence runs back to back -- `shifty` is live keyboard state,
		 * so any console or file I/O in between could let a real modifier
		 * key change it under the probe. The original value is put back
		 * before anything else runs. */
		{
			long k0 = Kbshift(-1);
			long k1 = Kbshift(-1);
			long kprev = Kbshift(0x0100 | MODE_CTRL);
			long kset  = Kbshift(-1);
			long kres  = Kbshift((short)k0);
			long kback = Kbshift(-1);

			ck_bool(G_BSYS, "BS17 Kbshift(-1) query is stable", k0 == k1);
			ck_hex(G_BSYS, "BS18 Kbshift(-1) bits 8-31 zero",
			       ((unsigned long)k0) >> 8, 0);
			ck_bool(G_BSYS, "BS19 Kbshift(set) returns the previous value",
				kprev == k1);
			ck_hex(G_BSYS, "BS20 Kbshift stores only the low byte",
			       (unsigned long)kset, (unsigned long)MODE_CTRL);
			ck_bool(G_BSYS, "BS21 Kbshift restored the old state",
				kres == MODE_CTRL && kback == k0);
		}
	}

	/* ---- done: the DONE marker is committed per-line like every other line
	 *      (no trailing Fclose to lose). Then pause so the screen stays
	 *      readable before C:\AUTO returns and the desktop clears it. ---- */
	emit("\r\n=== APITEST DONE ===\r\n");

	/* Push the trace off the screen so the verdict stands alone. The
	 * framebuffer console has no ESC-sequence parser (kernel/src/fbcon.rs),
	 * so a screenful of blank lines is the portable "clear"; 30 lines is the
	 * height of the 640x480 8x16 console. Screen only -- Cconws directly,
	 * not emit(), so the blanks never reach the log file. */
	{
		int blank;
		for (blank = 0; blank < 30; blank++)
			Cconws("\r\n");
	}

	/* The compact verdict, printed LAST and paged. On the real machine this
	 * is the only output the user gets to read: there is no serial line, the
	 * trace above has scrolled off, and the desktop clears the screen the
	 * moment C:\AUTO returns. It is still teed to C:\APITEST.LOG. */
	summary();

	if (!g_batch) {
		g_log_ok = 0;        /* stop teeing to file: prompt is screen-only */
		emit("\r\nPress any key to continue (then the desktop starts)...\r\n");
		(void)Cconin();      /* blocks; USB-HID/IKBD from AUTO */
	} else {
		emit("\r\n=== APITEST BATCH: no key wait, continuing ===\r\n");
	}

	Pterm(0);                    /* GEMDOS 0x4C -- does not return */
	return 0;                    /* unreachable; crt0 would Pterm(0) anyway */
}
