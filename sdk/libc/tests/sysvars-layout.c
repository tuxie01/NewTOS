/*
 * sysvars-layout.c -- compile-time check of the SDK's <mint/sysvars.h>
 * (the overlay sdk/libc/overlay/include/mint/sysvars.h, TASK/SDK04 point 3)
 * against the wire layout of the TOS system variables.
 *
 * Contract: docs/api/bios/_boot-init.md section 7 (the $380-$5B4 table from
 * EmuTOS tosvars.ld, with the width of every cell), docs/api/structures/
 * osheader.md "Binary layout" (48 bytes) and docs/api/bios/_cookie-jar.md
 * "Structure" ({ULONG tag; ULONG value}, 8 bytes). The NewTOS writer is
 * populate_sysvars / write_cookie_jar in tos-api/src/bdos/process/loader.rs
 * (one 4-byte wr_long per LONG cell, OSHEADER at $5C0, the jar mirror at
 * $5F0 in 8-byte slots).
 *
 * Compiled by sysvars-layout.sh for the TARGET (clang -target
 * aarch64-unknown-none-elf, the include path of build-libc.sh with the
 * overlay include directory first), so the types are the ones a NewTOS
 * program gets. Every check is a _Static_assert (the header adds its own
 * offsetof/sizeof checks for OSHEADER, NEWTOS_COOKIE and the post-mortem
 * dump); the file compiles iff the header matches. Against mintlib's
 * pristine sysvars.h the LONG-width checks fail (its cells are `long *`,
 * `void **`, 8 bytes on LP64) and NEWTOS_COOKIE does not exist.
 */
#include <stddef.h>
#include <mint/sysvars.h>

#define ADDR(p) ((__UINTPTR_TYPE__)(p))

/* A LONG or pointer cell: right address, 4 bytes wide, unsigned. */
#define LONG_CELL(p, a) \
	_Static_assert(ADDR(p) == (a), #p " address"); \
	_Static_assert(sizeof(*(p)) == 4, #p " is a 4-byte LONG"); \
	_Static_assert((__typeof__(*(p) + 0))-1 > 0, #p " is unsigned")
/* A WORD cell: right address, 2 bytes wide. */
#define WORD_CELL(p, a) \
	_Static_assert(ADDR(p) == (a), #p " address"); \
	_Static_assert(sizeof(*(p)) == 2, #p " is a 2-byte WORD")

/* section 7a, crash dump */
LONG_CELL(proc_lives, 0x380);
_Static_assert(ADDR(proc_post_mortem_dump_p) == 0x384, "proc_dregs@$384");
_Static_assert(ADDR(proc_aregs) == 0x3A4, "proc_aregs@$3A4");
_Static_assert(ADDR(&proc_pc) == 0x3C4, "proc_enum@$3C4");
_Static_assert(ADDR(&proc_usp) == 0x3C8, "proc_usp@$3C8");
_Static_assert(ADDR(proc_utk) == 0x3CC, "proc_stk@$3CC");
_Static_assert(sizeof(proc_dregs[0]) == 4 && sizeof(proc_aregs[0]) == 4, "registers are LONGs");

/* section 7b, the etv_* vectors */
LONG_CELL(etv_timer, 0x400);
LONG_CELL(etv_critic, 0x404);
LONG_CELL(etv_term, 0x408);

/* section 7c */
LONG_CELL(memvalid, 0x420);
LONG_CELL(resvalid, 0x426);
LONG_CELL(resvector, 0x42A);
LONG_CELL(phystop, 0x42E);
LONG_CELL(_membot, 0x432);
LONG_CELL(_memtop, 0x436);
LONG_CELL(memval2, 0x43A);
WORD_CELL(flock, 0x43E);
WORD_CELL(seekrate, 0x440);
WORD_CELL(_timr_ms, 0x442);
WORD_CELL(_fverify, 0x444);
WORD_CELL(_bootdev, 0x446);
WORD_CELL(palmode, 0x448);
WORD_CELL(sshiftmd, 0x44C);
LONG_CELL(_v_bas_ad, 0x44E);
WORD_CELL(vblsem, 0x452);
WORD_CELL(nvbls, 0x454);
LONG_CELL(_vblqueue, 0x456);
LONG_CELL(colorptr, 0x45A);
LONG_CELL(screenptr, 0x45E);
LONG_CELL(_vbclock, 0x462);
LONG_CELL(_frclock, 0x466);
_Static_assert(ADDR(conterm) == 0x484 && sizeof(*conterm) == 1, "conterm BYTE@$484");
LONG_CELL(savptr, 0x4A2);
WORD_CELL(_nflops, 0x4A6);
LONG_CELL(_hz_200, 0x4BA);
LONG_CELL(_sysbase, 0x4F2);
LONG_CELL(_shell_p, 0x4F6);
LONG_CELL(pun_ptr, 0x516);
LONG_CELL(_p_cookies, 0x5A0);
LONG_CELL(ramtop, 0x5A4);

/* The cookie slot: stride 8, tag@0, value@4, both unsigned 32-bit. */
_Static_assert(sizeof(NEWTOS_COOKIE) == 8, "jar stride 8");
_Static_assert(offsetof(NEWTOS_COOKIE, value) == 4, "value@4");
_Static_assert(sizeof(((NEWTOS_COOKIE *)0)->tag) == 4, "tag is a LONG");
_Static_assert(sizeof(NEWTOS_COOKIE[16]) == 16 * 8, "16 NewTOS mirror slots = 128 bytes");

/* OSHEADER: the header pins every offset; here only what a reader needs. */
_Static_assert(sizeof(OSHEADER) == 48, "OSHEADER 48 bytes");
_Static_assert(offsetof(OSHEADER, os_beg) == 0x08 && offsetof(OSHEADER, p_run) == 0x28,
	       "os_beg@0x08, os_run@0x28");
_Static_assert(sizeof(((OSHEADER *)0)->os_beg) == 4, "pointer members are 32-bit cells");

/* The two readers yield typed pointers from the 4-byte cells. */
_Static_assert(__builtin_types_compatible_p(__typeof__(NEWTOS_COOKIE_JAR),
					    const volatile NEWTOS_COOKIE *), "NEWTOS_COOKIE_JAR type");
_Static_assert(__builtin_types_compatible_p(__typeof__(NEWTOS_SYSBASE),
					    const volatile OSHEADER *), "NEWTOS_SYSBASE type");
