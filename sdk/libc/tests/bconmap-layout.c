/*
 * bconmap-layout.c -- compile-time check of the SDK's _BCONMAP and _MAPTAB
 * against the offsets the kernel lays the Bconmap table out at (TASK/XBIOS05;
 * the _IOREC check iorec-layout.c of XBIOS01 is the model).
 *
 * The contract is the LP64 column of docs/api/structures/bconmap.md (and its
 * tosdump-layout block); the kernel side is tos-api/src/xbios/bconmap.rs
 * (OFF_* constants). Compiled by bconmap-layout.sh for the TARGET (clang
 * -target aarch64-unknown-none-elf, the overlay include directory first),
 * so the types are the ones a NewTOS program gets. Every check is a
 * _Static_assert: the file compiles iff the header matches.
 *
 * The SDK's _BCONMAP has two fields (mintlib, as the Compendium); EmuTOS'
 * BCONMAP has a third, mapped_device (bios/serport.h:115-119). Under LP64 it
 * falls into the padding of the SDK struct, at 10; a program reads it as
 * ((short *)&bm->maptabsize)[1] (the audit probe xcprobe.c, group B). The
 * check builds EmuTOS' three-field struct from the SDK's types the same way.
 */
#include <stddef.h>
#include <mint/ostruct.h>

_Static_assert(sizeof(void *) == 8, "the contract below is the LP64 one");

/* _MAPTAB: six pointers, 48 bytes (bios/serport.h:106-113 order). */
_Static_assert(sizeof(_MAPTAB) == 48, "sizeof(_MAPTAB)");
_Static_assert(offsetof(_MAPTAB, Bconstat) == 0, "Bconstat");
_Static_assert(offsetof(_MAPTAB, Bconin) == 8, "Bconin");
_Static_assert(offsetof(_MAPTAB, Bcostat) == 16, "Bcostat");
_Static_assert(offsetof(_MAPTAB, Bconout) == 24, "Bconout");
_Static_assert(offsetof(_MAPTAB, Rsconf) == 32, "Rsconf");
_Static_assert(offsetof(_MAPTAB, iorec) == 40, "iorec");

/* _BCONMAP: maptab @0, maptabsize @8, 16 bytes. */
_Static_assert(sizeof(_BCONMAP) == 16, "sizeof(_BCONMAP)");
_Static_assert(offsetof(_BCONMAP, maptab) == 0, "maptab");
_Static_assert(offsetof(_BCONMAP, maptabsize) == 8, "maptabsize");
_Static_assert(sizeof(((_BCONMAP *)0)->maptabsize) == 2, "maptabsize is a WORD");

/* EmuTOS' three fields: mapped_device right after maptabsize, @10, inside
 * the SDK struct's 16 bytes. */
typedef struct {
    _MAPTAB *maptab;
    short maptabsize;
    short mapped_device;
} emutos_bconmap;
_Static_assert(offsetof(emutos_bconmap, mapped_device) == 10, "mapped_device");
_Static_assert(sizeof(emutos_bconmap) == sizeof(_BCONMAP), "same size as the SDK's");
_Static_assert(offsetof(emutos_bconmap, mapped_device) ==
               offsetof(_BCONMAP, maptabsize) + sizeof(short),
               "((short *)&bm->maptabsize)[1] is mapped_device");
