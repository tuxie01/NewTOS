/* definitions for various GEMDOS system variables in low memory */
/* WARNING: this file is not compatible with the old one */

/*
 * Convention:
 * 	the address for each variable is defined.
 */

/*
 * NewTOS overlay (TASK/SDK04 point 3): the system variables are 32-bit on
 * the wire, so every LONG and every pointer cell is a FIXED 32-bit type.
 *
 * The page-0 cells are TOS-canonical: a LONG or a pointer is 4 bytes, a WORD
 * 2 bytes, at the classic address (docs/api/bios/_boot-init.md section 7, the
 * full $380-$5B4 table from EmuTOS tosvars.ld; the NewTOS writer is
 * populate_sysvars in tos-api/src/bdos/process/loader.rs, one 4-byte
 * wr_long per LONG cell). Pristine mintlib declares the LONG/pointer cells
 * as `long *`, `unsigned long *`, `void **` or `void (**)()`: 8 bytes on
 * LP64 (AArch64), so `*_sysbase` fused _sysbase with _shell_p@$4F6,
 * `*_p_cookies` fused the jar pointer with ramtop@$5A4 (measured
 * 0x3BE00000000005F0, docs/api/bios/_cookie-jar.md "OFFEN"), and a write
 * through such a macro hit the neighbouring cell. Here those cells are
 * `volatile __UINT32_TYPE__ *`: exactly 32 bits on every ABI. A cell that
 * holds an address is read as a number and widened by the reader:
 *
 *     OSHEADER *os = (OSHEADER *)(__UINTPTR_TYPE__)*_sysbase;
 *
 * (NewTOS user addresses are below 2 GiB, docs/sdk/05-abi-pitfalls.md A2;
 * zero extension keeps them valid.) NEWTOS_SYSBASE and NEWTOS_COOKIE_JAR
 * below do exactly that. Assigning a pointer to such a cell no longer
 * compiles silently -- that is the point: the old 8-byte store was the
 * defect. The etv_* hooks are not read from these cells on NewTOS: they
 * live in Setexc's vector shadow (setexc in tos-api/src/bios/vectors.rs),
 * so install one with Setexc(0x100..0x102, ...); NewTOS calls etv_term
 * from there, etv_timer/etv_critic not (ADR-031).
 *
 * Also fixed-width here, each with _Static_assert on every offset and on
 * the size: OSHEADER (48 bytes, docs/api/structures/osheader.md "Binary
 * layout"; mintlib's pointer members made it 96 on LP64), the post-mortem
 * dump at $384 (docs/api/bios/_boot-init.md section 7a), and the new
 * NEWTOS_COOKIE slot (8 bytes, docs/api/bios/_cookie-jar.md "Structure").
 * Field names stay mintlib's, so existing readers (os_version, ...) compile
 * unchanged.
 *
 * Unchanged: the BYTE and WORD cells (already the wire width), the
 * function declarations, and HDINFO behind pun_ptr -- docs/api has no
 * layout for EmuTOS's pun_info yet, so its `long pstart[16]` is NOT pinned
 * (8 bytes per entry on LP64; nothing in the SDK reads it).
 *
 * A direct read of page 0 is a plain load on NewTOS (the page is the
 * program's own, readable at EL0: sdk/libc/newtos-arch/getsysvar.c); on an
 * Atari it needs Super()/Supexec(). get_sysvar() does the 32-bit read on
 * both. Host check: sdk/libc/tests/sysvars-layout.sh.
 */
#ifndef _SYSVARS_H
# define _SYSVARS_H 1

#ifndef	_FEATURES_H
# include <features.h>
#endif

__BEGIN_DECLS


/* Processor state and post mortem dump area */

#define PROC_LIVES_MAGIC 0x12345678L	/* proc_lives if dump is valid */
#define proc_lives	((volatile __UINT32_TYPE__ *) 0x380L)
/* $384-$3EB, docs/api/bios/_boot-init.md section 7a: proc_dregs@$384,
 * proc_aregs@$3A4, proc_enum@$3C4, proc_usp@$3C8, proc_stk@$3CC (16 WORDs).
 * Every register is a 32-bit LONG on the wire. */
struct __post_mortem_dump {
    __UINT32_TYPE__	d0, d1, d2, d3, d4, d5, d6, d7;
    __UINT32_TYPE__	a0, a1, a2, a3, a4, a5, a6, a7; /* a7 == ssp */
    __UINT32_TYPE__	pc;	  /* first byte is exception #   */
    __UINT32_TYPE__	usp;
    unsigned short	stk[16];  /* sixteen word of super stack */
} __attribute__((packed));
_Static_assert(__builtin_offsetof(struct __post_mortem_dump, a0)  == 0x3A4 - 0x384, "proc_aregs@$3A4");
_Static_assert(__builtin_offsetof(struct __post_mortem_dump, pc)  == 0x3C4 - 0x384, "proc_enum@$3C4");
_Static_assert(__builtin_offsetof(struct __post_mortem_dump, usp) == 0x3C8 - 0x384, "proc_usp@$3C8");
_Static_assert(__builtin_offsetof(struct __post_mortem_dump, stk) == 0x3CC - 0x384, "proc_stk@$3CC");
_Static_assert(sizeof(struct __post_mortem_dump) == 0x3EC - 0x384, "post-mortem dump $384-$3EB");
#define proc_post_mortem_dump_p	((volatile struct __post_mortem_dump *) 0x384L)
#define proc_dregs	(&proc_post_mortem_dump_p->d0)
#define proc_aregs	(&proc_post_mortem_dump_p->a0)
#define proc_pc		(proc_post_mortem_dump_p->pc)
#define proc_usp	(proc_post_mortem_dump_p->usp)
#define proc_utk	(proc_post_mortem_dump_p->stk)


/* timer, crit error and process termination handoff vectors (LONG cells;
 * install a hook with Setexc(0x100..0x102, ...), see the header note) */
#define etv_timer	((volatile __UINT32_TYPE__ *) 0x400L)
#define etv_critic	((volatile __UINT32_TYPE__ *) 0x404L)
#define etv_term	((volatile __UINT32_TYPE__ *) 0x408L)

/* memory controller */
#define MEMVALID_MAGIC	0x752019F3L	/* once memory is sized */
#define memvalid	((volatile __UINT32_TYPE__ *) 0x420L)
#define mencntlr	((unsigned char *) 0x424L)
 			/* 0 = 128K, 4 = 512K 0 = 256k(2banks) 5 = 1M */

/* reset vector, jump through resvector if resvalid on reset */
#define RESVALID_MAGIC	0x31415926L
#define resvalid	((volatile __UINT32_TYPE__ *) 0x426L)
#define resvector	((volatile __UINT32_TYPE__ *) 0x42aL)
			/* do a jmp 0x24L(a6) at end to go to system reset */

/* mem */
#define phystop		((volatile __UINT32_TYPE__ *) 0x42eL) /* physical top of st ram */
#define _membot		((volatile __UINT32_TYPE__ *) 0x432L) /* bottom of avail     */
#define _memtop		((volatile __UINT32_TYPE__ *) 0x436L) /* top    of avail     */
#define MEMVAL2_MAGIC	0x237698AAL /* after suc. coldstart && memvalid     */
#define memval2		((volatile __UINT32_TYPE__ *) 0x43aL)
#define ramtop		((volatile __UINT32_TYPE__ *) 0x5a4L) /* physical top of tt ram */

/* floppy */
#define flock		((volatile short *) 0x43eL) /* lock usage of DMA   chip */
#define seekrate	((short *) 0x440L) /* 0=6ms 1=12ms 2=2ms 3=3ms */
#define _timr_ms	((short *) 0x442L) /* timer calib == 20ms      */
#define _fverify	((short *) 0x444L) /* write verify flag        */
#define _bootdev	((short *) 0x446L)

/* video */
#define palmode		((short *) 0x448L) /* PAL video mode flag         */
#define defshiftmd	((unsigned char *) 0x44aL) /* default video rez   */
#define sshiftmd	((short *) 0x44cL) /* shadow of hdwr. shiftmd reg */
 					   /* 0=Lo 1=med 2=Hi rez         */
#define _v_bas_ad	((volatile __UINT32_TYPE__ *) 0x44eL)  /* screen mem base             */
#define vblsem		((volatile short *) 0x452L) /* vbl semaphore               */
#define nvbls		((short *) 0x454L) /* # of vbl entries def. == 8  */
#define _vblqueue	((volatile __UINT32_TYPE__ *) 0x456L) /* vbl queue pointer      */
#define colorptr	((volatile __UINT32_TYPE__ *) 0x45aL) /* pal. on next vblank if!NULL */
#define screenptr	((volatile __UINT32_TYPE__ *) 0x45eL) /* screen mem on next vblank if !NULL */
#define _vbclock	((volatile __UINT32_TYPE__ *) 0x462L) /* vbi counter         */
#define _frclock	((volatile __UINT32_TYPE__ *) 0x466L) /* #vbi not vblsem'ed  */

#define _hz_200		((volatile __UINT32_TYPE__ *) 0x4baL)

#define conterm		((char *) 0x484L)
#define savptr		((volatile __UINT32_TYPE__ *) 0x4A2L)
#define _nflops		((short *) 0x4A6L)
#define _sysbase	((volatile __UINT32_TYPE__ *) 0x4F2L)
#define _shell_p	((volatile __UINT32_TYPE__ *) 0x4F6L)

typedef struct {
  short	puns;
  char	v_p_un[16];
  long	pstart[16];		/* NOT pinned on LP64, see the header note */
  short	bpbs[1];		/* really 16 BPB's (bios parameter block) */
} HDINFO;

#define pun_ptr		((volatile __UINT32_TYPE__ *) 0x516L)
#define _p_cookies	((volatile __UINT32_TYPE__ *) 0x5A0L)

/* Every LONG/pointer cell above is 4 bytes, every WORD cell 2
 * (docs/api/bios/_boot-init.md section 7c). */
_Static_assert(sizeof(*_hz_200) == 4 && sizeof(*_sysbase) == 4 &&
	       sizeof(*_p_cookies) == 4 && sizeof(*ramtop) == 4 &&
	       sizeof(*etv_term) == 4 && sizeof(*_v_bas_ad) == 4 &&
	       sizeof(*proc_lives) == 4, "LONG cells are 32-bit");
_Static_assert(sizeof(*_bootdev) == 2 && sizeof(*flock) == 2, "WORD cells are 16-bit");

/* One cookie-jar slot: two 32-bit LONGs, 8 bytes, the jar a contiguous
 * array of them ended by {0, capacity} (docs/api/bios/_cookie-jar.md
 * "Structure"; EmuTOS include/cookie.h:16; NewTOS writes the mirror at
 * $5F0, write_cookie_jar in tos-api/src/bdos/process/loader.rs). NewTOS
 * name: mintlib has no slot type, and TeraDesk's own COOKIE is the 16-byte
 * `long` pair that missed every second slot (TASK/SDK04 C6). Prefer
 * Getcookie() / Ssystem(S_GETCOOKIE); the mirror is a read-only view. */
typedef struct {
    __UINT32_TYPE__ tag;	/* 0 ends the jar */
    __UINT32_TYPE__ value;	/* in the {0, n} terminator: capacity n */
} __attribute__((packed)) NEWTOS_COOKIE;

_Static_assert(__builtin_offsetof(NEWTOS_COOKIE, tag)   == 0, "tag@0");
_Static_assert(__builtin_offsetof(NEWTOS_COOKIE, value) == 4, "value@4");
_Static_assert(sizeof(NEWTOS_COOKIE) == 8, "a jar slot is 8 bytes (stride 8)");

/* The jar, read the only correct way: the 4-byte $5A0 cell, zero-extended. */
#define NEWTOS_COOKIE_JAR \
	((const volatile NEWTOS_COOKIE *) (__UINTPTR_TYPE__) *_p_cookies)

/* os header: 48 bytes on the wire (docs/api/structures/osheader.md "Binary
 * layout", EmuTOS include/biosdefs.h:44-63; NewTOS instance at $5C0,
 * OSHEADER_VA in tos-api/src/bdos/process/loader.rs). The pointer members
 * of the pristine struct are 32-bit cells here, as on the wire. */
typedef struct _osheader
{
    unsigned short	os_entry;   	 /* 0x00  BRA to reset handler	*/
    unsigned short	os_version; 	 /* 0x02  TOS version		*/
    __UINT32_TYPE__	reseth;		 /* 0x04 -> reset handler	*/
    __UINT32_TYPE__	os_beg;		 /* 0x08 -> base of OS		*/
    __UINT32_TYPE__	os_end;		 /* 0x0c -> end of OS ram usage */
    __UINT32_TYPE__	os_rsv1;	 /* 0x10 reserved		*/
    __UINT32_TYPE__	os_magic;	 /* 0x14 GEM memory usage param */
    __UINT32_TYPE__	os_date;	 /* 0x18 Build date 0xMMDDYYYY	*/
    unsigned short	os_conf;	 /* 0x1c OS conf bits		*/
    unsigned short	os_dosdate;	 /* 0x1e DOS format build date  */
    /* the following available on TOS version >= 1.2 */
    __UINT32_TYPE__	p_root;		 /* 0x20 -> base of OS pool	*/
    __UINT32_TYPE__	pkbshift;	 /* 0x24 -> kbd shift state var */
    __UINT32_TYPE__	p_run;		 /* 0x28 -> PID of current proc */
    __UINT32_TYPE__	p_rsv2;		 /* 0x2c reserved ('ETOS'/'NTOS') */
} __attribute__((packed)) OSHEADER;

_Static_assert(__builtin_offsetof(OSHEADER, os_entry)   == 0x00, "os_entry@0x00");
_Static_assert(__builtin_offsetof(OSHEADER, os_version) == 0x02, "os_version@0x02");
_Static_assert(__builtin_offsetof(OSHEADER, reseth)     == 0x04, "reseth@0x04");
_Static_assert(__builtin_offsetof(OSHEADER, os_beg)     == 0x08, "os_beg@0x08");
_Static_assert(__builtin_offsetof(OSHEADER, os_end)     == 0x0C, "os_end@0x0C");
_Static_assert(__builtin_offsetof(OSHEADER, os_rsv1)    == 0x10, "os_rsvl@0x10");
_Static_assert(__builtin_offsetof(OSHEADER, os_magic)   == 0x14, "os_magic@0x14");
_Static_assert(__builtin_offsetof(OSHEADER, os_date)    == 0x18, "os_date@0x18");
_Static_assert(__builtin_offsetof(OSHEADER, os_conf)    == 0x1C, "os_conf@0x1C");
_Static_assert(__builtin_offsetof(OSHEADER, os_dosdate) == 0x1E, "os_dosdate@0x1E");
_Static_assert(__builtin_offsetof(OSHEADER, p_root)     == 0x20, "os_root@0x20");
_Static_assert(__builtin_offsetof(OSHEADER, pkbshift)   == 0x24, "os_kbshift@0x24");
_Static_assert(__builtin_offsetof(OSHEADER, p_run)      == 0x28, "os_run@0x28");
_Static_assert(__builtin_offsetof(OSHEADER, p_rsv2)     == 0x2C, "os_dummy@0x2C");
_Static_assert(sizeof(OSHEADER) == 0x30, "sizeof(OSHEADER) == 48");

/* The OS header _sysbase points at, through the 4-byte cell. */
#define NEWTOS_SYSBASE \
	((const volatile OSHEADER *) (__UINTPTR_TYPE__) *_sysbase)

/* zzzz to-do more */

extern long get_sysvar (const volatile void *var) __THROW;
extern void set_sysvar_to_short (volatile void *var, short val) __THROW;
extern void set_sysvar_to_long (volatile void *var, long val) __THROW;


__END_DECLS

#endif /* _SYSVARS_H */
