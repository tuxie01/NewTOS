/*
 * iorec-layout.c -- compile-time check of the SDK's _IOREC (and of the
 * device-0 record a program declares from it) against the offsets the
 * kernel lays the Iorec records out at (TASK/XBIOS01, user decision
 * 2026-10-28: "offsetof/sizeof-Pruefung zur Uebersetzungszeit").
 *
 * The contract is docs/api/xbios/0e-iorec.md F1 (LP64 layout) and the
 * tosdump-layout block of docs/api/structures/iorec.md (LP64 column); the
 * kernel side is tos-api/src/xbios/iorec.rs (OFF_* constants). Compiled by
 * iorec-layout.sh for the TARGET (clang -target aarch64-unknown-none-elf,
 * the include path of build-libc.sh), so the types are the ones a NewTOS
 * program gets. Every check is a _Static_assert: the file compiles iff the
 * header matches.
 *
 * The SDK has no type for EmuTOS' EXT_IOREC (bios/serport.h:57-65); a
 * program reaches device 0's out record and line bytes through
 * `(_IOREC *)Iorec(0) + 1` (the bios-core audit probe did exactly that,
 * measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c),
 * so the check builds the record the same way: two _IOREC, five bytes.
 */
#include <stddef.h>
#include <mint/ostruct.h>

/* _IOREC: 24 bytes, docs/api/xbios/0e-iorec.md F1. */
_Static_assert(sizeof(_IOREC) == 24, "sizeof(_IOREC)");
_Static_assert(offsetof(_IOREC, ibuf) == 0, "ibuf");
_Static_assert(sizeof(((_IOREC *)0)->ibuf) == 8, "ibuf is a 64-bit pointer");
_Static_assert(offsetof(_IOREC, ibufsiz) == 8, "ibufsiz");
_Static_assert(offsetof(_IOREC, ibufhd) == 10, "ibufhd");
_Static_assert(offsetof(_IOREC, ibuftl) == 12, "ibuftl");
_Static_assert(offsetof(_IOREC, ibuflow) == 14, "ibuflow");
_Static_assert(offsetof(_IOREC, ibufhi) == 16, "ibufhi");
_Static_assert(sizeof(((_IOREC *)0)->ibufhd) == 2, "ibufhd is a WORD");

/* Device 0 (EXT_IOREC), F1: in @0, out @24, baudrate @48 .. wr5 @52, 56. */
typedef struct {
    _IOREC in;
    _IOREC out;
    unsigned char baudrate, flowctrl, ucr, datamask, wr5;
} ext_iorec;
_Static_assert(offsetof(ext_iorec, out) == 24, "out");
_Static_assert(offsetof(ext_iorec, baudrate) == 48, "baudrate");
_Static_assert(offsetof(ext_iorec, flowctrl) == 49, "flowctrl");
_Static_assert(offsetof(ext_iorec, ucr) == 50, "ucr");
_Static_assert(offsetof(ext_iorec, datamask) == 51, "datamask");
_Static_assert(offsetof(ext_iorec, wr5) == 52, "wr5");
_Static_assert(sizeof(ext_iorec) == 56, "sizeof(EXT_IOREC)");

/* The IKBD ring entry: shifty << 24 | scancode << 16 | ascii, 4 bytes
 * (bios/ikbd.c:427-435). A program must read it as a 32-bit int, not as a
 * `long` (8 bytes under LP64, F1 "LP64-Fallstrick"). */
_Static_assert(sizeof(int) == 4 && sizeof(long) == 8, "entry width vs long");
