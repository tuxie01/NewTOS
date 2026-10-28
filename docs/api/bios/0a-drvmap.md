# `Drvmap` (BIOS 0x0A)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1206` (`drvmap`) and `bios/blkdev.c:833`
> (`blkdev_drvmap`).

> **BIOS02 revision (2026-10-28) — for user approval.** Changes tagged
> **[BIOS02]**. Source re-read at the pin `1a7484ab`. Corrected: the drive
> limit is 26, not 16; the "single floppy sets A: and B:" convention is not in
> the code; measured value added. Cross-check: oracle
> `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN` vs NewTOS,
> `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`.

## EmuTOS source

**Primary location:** `bios/bios.c:1206` — `LONG drvmap(void)`
(body: `return blkdev_drvmap();`).

**Handler:** `bios/blkdev.c:833` — `LONG blkdev_drvmap(void)`
(body: `return drvbits;`). It returns the global `drvbits` variable directly;
it does **not** recompute a bitmap from per-drive flags.

**Helpers called:** none (a plain read of `drvbits`; no `protect_*` wrapper —
`drvmap` calls `blkdev_drvmap` directly, not through `hdv_*`).

**Headers / type definitions used:**
- `include/tosvars.h:57` — `extern LONG drvbits;`

**System variables / globals read:**
- `drvbits` (TOS var **`0x4c2`**) — bit `n` set ⇒ logical drive `n` present.

**System variables / globals written:** none.

**Build configuration affecting this function:** none directly; which bits are
set in `drvbits` is decided during `blkdev_init()` / partition scan.

---

## Source extraction summary

`Drvmap` returns the 32-bit logical-drive bitmap: bit 0 = A:, bit 1 = B:, …,
bit 15 = P:. It is a one-line read of the global `drvbits`, which the block-
device layer builds during boot device discovery (each valid logical drive
sets its bit). `blkdev_avail(dev)` (`bios/blkdev.c:846`) is the matching
predicate — `(1L << dev) & drvbits`.

The bitmap is essentially static after boot: EmuTOS does not update `drvbits`
on hot-plug (Atari hardware had no such expectation). **[BIOS02, corrected]**
EmuTOS has `BLKDEVNUM = 26` logical drives, A: to Z. (`include/sysconf.h:43`);
bits are set by `add_partition` (`bios/blkdev.c:392`) for every found partition
up to the 26th (`next_logical`, `bios/blkdev.c:315-324`; `disk_init_all`,
`bios/disk.c:320`) and by the floppy detection for A:/B: (`bios/floppy.c:387`);
only bits 26..31 are never set. (Earlier text: "16 drives, bits 16..31 unused"
— wrong; `PUN_MAXUNITS` = 16, `include/ahdi.h:19`, limits only the AHDI `pun_info` copy.) Steem's GEMDOS-HD emulation is known to
poke `drvbits` from a cartridge hook, which is why `biosmain` calls `Drvmap()`
once at boot (`bios/bios.c:791`) to let it take effect.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x0A
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x000A
  ```
- **Total parameter bytes:** 2 (just the ID)
- **Native (AArch64) ABI:** ID `0x0A` in `x8`; no arguments; the drive bitmap
  in `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (drive bitmap)
- **Encoding:** bit `n` set ⇒ drive index `n` (0=A:) is present. **[BIOS02]**
  Bits 26..31 are 0; bits 16..25 (Q:..Z:) can be set (`BLKDEVNUM = 26`).
- **Source location:** `bios/blkdev.c:835` (`return drvbits;`).

| Bit | Drive | … | Bit | Drive |
|----:|-------|---|----:|-------|
| 0   | A:    |   | 8   | I:    |
| 1   | B:    |   | …   | …     |
| 2   | C:    |   | 15  | P:    |
| …   | …     |   | 25  | Z:    |

### Side effects on global state

None.

### Side effects on hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none — cannot fail) | `bios/blkdev.c:833` | the bitmap (possibly 0) |

---

## Verification criteria

- **Nominal:** on a system with C:..F: mounted, `Drvmap()` has bits 2..5 set
  and returns the same `LONG` as the `drvbits` variable.
- **Floppy convention:** on a system with a single floppy, bits 0 **and** 1
  are set (A: and B: on one physical drive).
- **`blkdev_avail` consistency:** for every bit `n` set in `Drvmap()`,
  `blkdev_avail(n)` is non-zero, and vice-versa.

---

## Edge cases discovered in source

1. **It returns the raw `drvbits` global, not a recomputed map.** `Drvmap`
   never scans `blkdev[]` at call time (`bios/blkdev.c:833-836`). (Prior spec
   said the bitmap is "built from the per-drive `blkdev[i].valid` flags" *on
   each call* — that is wrong; it is precomputed into `drvbits` and simply
   read back.)

2. **A:+B: single-floppy convention — [BIOS02] not implemented by the code.**
   The comment above `blkdev_drvmap` (`bios/blkdev.c:824-830`) states the
   convention ("should return both bits set if a floppy drive is present"),
   but `flop_hdv_init` detects drive 0 and drive 1 independently
   (`bios/floppy.c:329-330`) and `flop_add_drive` sets only that drive's own
   bit (`bios/floppy.c:387`). Nothing duplicates A: into B:. The comment is
   documentation of the Atari convention; the code is what a program sees.

3. **Static after boot.** `drvbits` is not updated on hot-plug in EmuTOS. A
   card inserted after boot does not flip a bit.

4. **P: (bit 15) may be selectable even if clear.** A BetaDOS quirk lets
   `Dsetdrv(15)` succeed regardless; `Drvmap` remains the truthful presence
   map, and clients must consult it before assuming a drive is usable.

5. **Bits 26..31 are always 0** in EmuTOS (26-drive limit, `BLKDEVNUM`
   `include/sysconf.h:43`) **[BIOS02, corrected from "16..31 / 16-drive
   limit"]**; do not assume they carry information. Q:..Z: (bits 16..25) are
   reachable when more than 16 logical drives are found; NewTOS stops at 16
   (ticket FS12, `tos-api/src/bios/drive.rs:11@MAX_DRIVES`).

---

## Dependencies

### Requires (must be initialized / functional)

- `blkdev_init()` and the boot partition scan (populate `drvbits`).

### Used by

- GEMDOS `Dsetdrv`/`Dgetdrv` (returned drive map); AES/desktop drive
  enumeration; shell drive listing; `biosmain` (called once at boot,
  `bios/bios.c:791`, to let Steem's cartridge hook take effect).

---

## Related data structures

- `docs/api/bios/_state.md` — `drvbits` variable + address
- `docs/api/gemdos/drive/0e-dsetdrv.md` — consumes the map

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically documents
  the A:+B: convention verbatim — EmuTOS mirrors that wording,
  `bios/blkdev.c:826-830`.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `BLKDEVNUM = 26` logical drives, A:..Z. **[BIOS02, corrected from 16]**
  (`include/sysconf.h:43`).

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-04: NewTOS may make the map dynamic (SD/USB hot-plug flips bits).
Programs must treat `Drvmap()` as a snapshot. `drvbits` is a kernel-owned
variable (no `0x4c2` physical address for native ports).

---

## Pi / BCM2711 binding

> Re-points the drive-bitmap source from Atari floppy/ACSI enumeration to the
> **MBR partition table on the BCM2711 EMMC2 SD card**. The bitmap contract
> (bit `n` = drive `n`, 0=A:) is unchanged; only how the bits are populated is
> pinned here. See `docs/HARDWARE-PROFILE.md` §3.4.

### `drvbits` is built from the SD MBR partitions

`Drvmap` (`tos-api/src/bios/mod.rs:247-249`) returns `drive::drvbits()`
(`tos-api/src/bios/drive.rs:143-153`), which OR's in bit `n` for every mounted
partition slot. The table is filled once at boot by `drive::mount()`
(`tos-api/src/bios/drive.rs:55-131`), which:

1. reads **sector 0** (the MBR) from the EMMC SD via `BlockDev::read_blocks`
   (`tos-api/src/bios/drive.rs:60-63`);
2. checks the `0x55AA` signature at offset 510;
3. walks the 4 primary partition entries at `0x1BE` (16 bytes each,
   `tos-api/src/bios/drive.rs:42-48,76-88`), assigning each non-empty, non-extended filesystem
   partition to the next drive letter from **C:** upward
   (`FIRST_HD = 2`, `tos-api/src/bios/drive.rs:12-13,69`);
4. walks any extended partition's EBR chain, appending each logical volume
   (`tos-api/src/bios/drive.rs:93-120`);
5. if there is no valid MBR / no partitions, mounts the whole device as a single
   **C:** volume (super-floppy fallback, `tos-api/src/bios/drive.rs:127-130`).

### Concrete drive-letter mapping

| Config | Partitions → drives | `Drvmap()` bits |
|--------|---------------------|-----------------|
| **Real Pi-4** | MBR + 4 × FAT16 primary → **C: D: E: F:** (verified on HW) | bits 2,3,4,5 set = `0x3C` |
| **QEMU `virt:`** | MBR + 1 × FAT16 primary (type `0x06`, built by `tools/make-sd-image.sh`) → **C:** | bit 2 set = `0x04` |

- **A:/B: (bits 0,1) are never set** — there is no floppy on the Pi, so the
  Atari A:+B: single-floppy convention does not apply. Hard-disk drives start at
  C: (bit 2), matching the Atari hard-disk convention.
- MBR partition type is recorded (`Partition.kind`, `tos-api/src/bios/drive.rs:17-24,42-48`) but
  a slot is mounted regardless of FAT16 vs FAT32 type; whether it is *usable* is
  decided later by `Getbpb` (FAT32/exFAT ⇒ NULL, see `07-getbpb.md`). So a bit
  can be set for a partition that `Getbpb` will reject.

### Contract behaviours vs. the current code

- **Precomputed, read back** — `drvbits()` reads the mount-time table, matching
  the EmuTOS "returns the raw `drvbits` global, not a recomputed map" contract
  (though NewTOS recomputes from the `Option<Partition>` slots on each call
  rather than caching a `LONG`; the result is identical and static after boot).
- **Static after boot** — no SD/USB hot-plug re-scan today; a card swapped after
  boot does not flip a bit (matches EmuTOS; hot-plug is deferred, §PHASE-04
  above).
- **`drvbits` is kernel-owned** — no `0x4c2` low-memory address; native callers
  reach it only through `Drvmap`.
- **Bits 16..31 always 0** — `MAX_DRIVES = 16` (`tos-api/src/bios/drive.rs:11@MAX_DRIVES`).
  **[BIOS02]** This is a difference from EmuTOS (26 drives), tracked by FS12;
  not observable with one SD card and four partitions, not measured
  (`Getbpb(16)`, `Mediach(16)`, `Rwabs(…,16,…)` are EQUAL only because both
  sides have no drive 16, bios-r2 cases `Getbpb(16)` 45, `Mediach(16)` 53,
  R14 72).

### Gaps vs. the contract (Step-2 candidates)

- **No `blkdev_avail` predicate exposed** — clients cannot query a single
  drive's presence except by testing a bit of `Drvmap()`. (Cosmetic; the bit
  test is equivalent.)
- **Extended/logical partitions map past F:** — with 4 primaries plus an
  extended chain, the EBR walk can assign G:+; documented here so the C:-F:
  "verified" figure is understood as the 4-primary real-HW layout, not a hard
  cap.

### Measured cross-check — `bios-r2.tsv` cases per statement [BIOS02]

| Statement | Case (dumper no.) | EmuTOS | NewTOS | Verdict |
|-----------|-------------------|--------|--------|---------|
| `Drvmap` with C: (test partition) and D: (program drive), A:/B: off | `Drvmap()` 39 (EXACT) | `0x0000000C` | `0x0000000C` | EQUAL |
| `Drvmap` value is a device parameter outside C:+D: | `bios-r1.tsv` 74 (class NONZERO, weak) | `0x7` (Hatari with floppies) | `0x4` | EQUAL by class; A:/B: = device parameter, audit §8 |

Layout of the two sides is arranged to give the same map
(`tosdump/cases/bios-r2.tsv` header), so case 39 confirms the *value format*
and bit order, not the partition-scan rules. Not measured: more than 16
drives, floppy bits, bits changing after boot.
