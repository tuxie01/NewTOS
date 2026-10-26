# `BPB` — BIOS Parameter Block

> Phase 00 spec D6. Source: EmuTOS @ `1a7484ab` —
> `include/biosdefs.h:67-86`. Returned by `Getbpb` (BIOS 0x07).

## Purpose

Geometry descriptor for one mounted block device. Exposes enough FAT
metadata for GEMDOS to compute sector/cluster offsets without
re-parsing the boot sector. **Pointer-returning API** — `Getbpb`
returns a `BPB *` to the kernel-owned BPB for the requested drive
(or `NULL` on error). Callers must not mutate it.

## Binary layout (18 bytes, all UWORD)

```c
struct _bpb {
/* 0x00 */ UWORD recsiz;    /* sector size in bytes      */
/* 0x02 */ UWORD clsiz;     /* cluster size in sectors   */
/* 0x04 */ UWORD clsizb;    /* cluster size in bytes     */
/* 0x06 */ UWORD rdlen;     /* root dir length in records*/
/* 0x08 */ UWORD fsiz;      /* FAT size in records       */
/* 0x0A */ UWORD fatrec;    /* first FAT record (last)   */
/* 0x0C */ UWORD datrec;    /* first data record         */
/* 0x0E */ UWORD numcl;     /* number of data clusters   */
/* 0x10 */ UWORD b_flags;   /* see B_* below             */
};
```

Layout is fixed by Atari ABI. **`UWORD` (16-bit) on every field** —
even on disks where the value would fit only into a 32-bit count
(see "Limits" below).

## Field semantics

### `recsiz` — sector size in bytes

Always 512 in practice (Atari floppy and most hard drives). Values
of 256, 1024, 2048 are theoretically supported and the FAT layer
honours them; the BIOS sector cache assumes powers of two.

### `clsiz` — cluster size in sectors

Power of 2 between 1 and 64. Floppies typically use 2; hard-disk
partitions 4–32.

### `clsizb` = `recsiz * clsiz`

Pre-computed; clients use this rather than multiplying.

### `rdlen` — root directory length in records (sectors)

Number of sectors reserved for the FAT12/16 root directory.
Doesn't apply to FAT32 (not supported by classic TOS).
112-entry floppy root → 7 sectors (`112*32/512`).

### `fsiz` — FAT size in records

Sectors per single FAT. **Not** the total of both FATs.

### `fatrec` — first record of the **last** FAT

Sector number of the *second* FAT's first sector (assuming standard
2-FAT layout). The choice of "last FAT" is so DOS/TOS can read FAT
data preferentially from the second FAT (acting as backup-as-primary
when the first one is corrupt). On `B_1FAT` media (one FAT only),
`fatrec` points at that single FAT.

### `datrec` — first data record

Sector number of the first data sector (cluster 2). Computed as
`fatrec + fsiz + rdlen` for 2-FAT, `fatrec + rdlen` for 1-FAT.

### `numcl` — number of data clusters

Total clusters available for files/directories. **Determines FAT
type** under the standard FAT spec:
- `numcl <  4085`  → FAT12
- `numcl >= 4085`  → FAT16 (and `B_16` flag must be set)
- FAT32 is not supported.

Because `numcl` is a UWORD, the maximum addressable cluster count is
65535; the maximum partition size is `65535 × clsizb`. For 32 KiB
clusters that is 2 GiB — the classic Atari TOS partition limit.

### `b_flags`

```c
#define B_16    0x01  /* media uses 16-bit FAT entries */
#define B_1FAT  0x02  /* media has only one FAT        */
```

Other bits are reserved (must be zero on read; ignored on
historical media). EmuTOS does not define additional flags.

## Lifetime and ownership

- The BPB is **kernel-owned**. `Getbpb(drv)` returns a pointer into
  EmuTOS's per-drive BPB array (`bios/blkdev.c:units[drv].bpb`).
- The BPB is updated when:
  - Media change is detected (`Mediach` returns 2).
  - The drive is first accessed.
- `Getbpb` returning `NULL` means the drive is unmountable
  (unrecognised filesystem, hardware error during boot sector read,
  or drive does not exist).

GEMDOS keeps a separate, more detailed `DMD` structure
(see `dnd-dmd.md`) per mounted drive; the BPB is the BIOS-level
slice of that information.

## Limits / known incompatibilities

- 16-bit `numcl` caps partition size at 2 GiB.
- Sector size > 4096 not tested in practice.
- `b_flags` has no bit for FAT32, exFAT, or sector size > 65535
  bytes.

## Cross-references

- **Compendium:** "BPB" — agrees on layout and semantics.
- **Profibuch:** "BIOS Parameter Block" — agrees.
- **`docs/api/bios/07-getbpb.md`** (TBD) — returns `BPB *`.
- **`docs/api/structures/dnd.md`** — DMD subsumes BPB and adds
  precomputed log/mask fields.
- **EmuTOS source:** `bios/blkdev.c` (BPB construction from boot
  sector), `bdos/fsbuf.c` (cache parameterisation).

## NewTOS implementation notes

(Empty during Phase 00. Filled in Phase 04 — block storage.)
