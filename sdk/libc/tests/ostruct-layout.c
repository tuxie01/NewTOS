/*
 * ostruct-layout.c -- compile-time check of <mint/ostruct.h>'s _MD/_MPB
 * (docs/api/bios/00-getmpb.md V1/V4, decision O1 = A: 32/24 bytes on LP64)
 * and _KBDVECS (docs/api/xbios/22-kbdvbase.md B1: 8-byte slots, midivec at
 * the address Kbdvbase() returns, kbstate = drvstat at 0x48), TASK/SDK04
 * point 3. The checks are the _Static_assert blocks in the overlay header
 * itself; this file makes sysvars-layout.sh compile them for the target.
 * _IOREC has its own check (iorec-layout.c, TASK/XBIOS01) and _DTA,
 * _DISKINFO and BASEPAGE theirs in the headers.
 */
#include <stddef.h>
#include <mint/ostruct.h>

_Static_assert(sizeof(void *) == 8, "the contracts above are the LP64 ones");
