# `BCONMAP` / `MAPTAB` — the `Bconmap` mapping table

> Spec D6 form, added by XBIOS05 (2026-10-29). Source: EmuTOS @ `1a7484ab` —
> `bios/serport.h:102-121` (`MAPTAB`, `BCONMAP`, `BCONMAP_START_HANDLE`),
> `bios/serport.c:92-117` (the static entries), `:956-1004` (`init_bconmap`),
> `:1082-1126` (`bconmap`). Returned by XBIOS `Bconmap(-2)` (function 0x2C,
> `docs/api/xbios/2c-bconmap.md`).

## Purpose

`Bconmap(-2)` returns a pointer to the `BCONMAP`; its `maptab` points at an
array of `maptabsize` `MAPTAB` entries, one per serial port that can be
mapped to BIOS device 1 (AUX). Device numbers start at
`BCONMAP_START_HANDLE` = 6: entry `i` is device `6 + i`. `mapped_device` is
the device currently mapped.

## Binary layout — `MAPTAB` (24 bytes on m68k)

```c
typedef struct {        /* one per mappable device */
/* 0x00 */ LONG (*Bconstat)(void);
/* 0x04 */ LONG (*Bconin)(void);
/* 0x08 */ LONG (*Bcostat)(void);
/* 0x0C */ LONG (*Bconout)(WORD,WORD);
/* 0x10 */ ULONG (*Rsconf)(WORD,WORD,WORD,WORD,WORD,WORD);
/* 0x14 */ EXT_IOREC *Iorec;   /* points to IOREC and extended IOREC */
/* 0x18 — total 24 bytes (bios/serport.h:106-113) */
} MAPTAB;
```

## Binary layout — `BCONMAP` (8 bytes on m68k)

```c
typedef struct {
/* 0x00 */ MAPTAB *maptab;
/* 0x04 */ WORD maptabsize;
/* 0x06 */ WORD mapped_device; /* device currently mapped to device 1 */
/* 0x08 — total 8 bytes (bios/serport.h:115-119) */
} BCONMAP;
```

## EmuTOS instances

`init_bconmap` (`bios/serport.c:956-1004`) fills `maptable[4]` with
`maptable_dummy`, sets `maptab = maptable`, `maptabsize = 1`,
`mapped_device = 7` on Falcon/ARAnyM, else 6 (`:962-966`), copies
`maptable_mfp` = `{bconstat1, bconin1, bcostat1, bconout1, rsconf1, &iorec1}`
(`:98`) into entry 0 (`:972`), then by machine: Falcon/MSTe/ARAnyM
`maptabsize = 3` (SCC port B, port A in entries 1, 2), TT `maptabsize = 4`
(SCC B, TT-MFP, SCC A in entries 1, 2, 3); any other machine keeps one
entry. Measured on the pinned oracle: TT `maptabsize` 4, `mapped_device` 6;
Falcon 3 and 7 (`measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`,
probe B04; `tosdump/cases/xbios-bconmap-r1.tsv`).

## NewTOS layout (LP64) [XBIOS05]

A NewTOS program reads the table through the SDK's `_BCONMAP`/`_MAPTAB`
(`sdk/libc/overlay/include/mint/ostruct.h:718-733@_MAPTAB`): every pointer
8 bytes, natural alignment.

| Structure | Field | m68k offset | LP64 offset |
|---|---|---:|---:|
| `MAPTAB` | `Bconstat` | 0 | 0 |
| | `Bconin` | 4 | 8 |
| | `Bcostat` | 8 | 16 |
| | `Bconout` | 12 | 24 |
| | `Rsconf` | 16 | 32 |
| | `Iorec` | 20 | 40 |
| | size | 24 | 48 |
| `BCONMAP` | `maptab` | 0 | 0 |
| | `maptabsize` | 4 | 8 |
| | `mapped_device` | 6 | 10 |
| | size | 8 | 16 |

The SDK's `_BCONMAP` has only `maptab` and `maptabsize` (mintlib, as the
Compendium); its LP64 size is 16 all the same, and `mapped_device` lies in
its padding at 10, where EmuTOS' third field falls under LP64 rules. A program
reads it as `((short *)&bm->maptabsize)[1]` (the audit probe did,
`measurements/measurement-2026-10-29-xbios05/method/xcprobe.c`). The offsets
are checked at compile time against the SDK header by
`sdk/libc/tests/bconmap-layout.c`.

The NewTOS table, its place and the five non-callable function pointers:
`docs/api/xbios/2c-bconmap.md`, "NewTOS implementation".

## Dumper layout (tosdump) [XBIOS05]

The layout the reference dumper reads behind `Bconmap(-2)`'s pointer
(`docs/TOS-REFERENCE-DUMPER.md` section 6, "Structures behind a pointer"):
the `BCONMAP` and, through `maptab`, the first `MAPTAB`. This block is the
source; the case table carries a byte-identical copy (`layoutcheck`).

- `maptab` -- m68k 24 bytes, LP64 48. Every slot is an address on both
  machines (EmuTOS: ROM routines and `&iorec1`; NewTOS: the non-callable
  targets and `Iorec(0)`'s record): PTR_RAM, the default for `p`.
- `bconmap` -- m68k 8 bytes, LP64 16; `maptab` is followed into `maptab`.
  `maptabsize` and `mapped_device` are EXACT: the Falcon/TT values differ
  from NewTOS' one-entry table by the machine (ACCEPTED per profile,
  `docs/COMPATIBILITY.md`, "`Bconmap`").

```tosdump-layout
!layout	maptab	24/48	bconstat:p@0/0,bconin:p@4/8,bcostat:p@8/16,bconout:p@12/24,rsconf:p@16/32,iorec:p@20/40
!layout	bconmap	8/16	maptab:p>maptab@0/0,maptabsize:w@4/8,mapped_device:w@6/10
```
