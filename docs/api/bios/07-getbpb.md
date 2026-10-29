# `Getbpb` (BIOS 0x07)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1120` (`getbpb`) and the installed handler
> `bios/blkdev.c:569` (`blkdev_getbpb`).

> **BIOS02 revision (2026-10-28) — approved by the user 2026-10-28.** Changes tagged
> **[BIOS02]**. Source re-read at the pin `1a7484ab`
> (`references/emutos/bios/blkdev.c:569-777`, `bios/blkdev.c:117-163`).
> Cross-check: oracle `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN`
> vs NewTOS, `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`. The
> "Gaps" list of the Pi binding was out of date (the guards exist and agree);
> two real differences were missing (BIOS-STOR-004, -005).

## EmuTOS source

**Primary location:** `bios/bios.c:1120` — `LONG getbpb(WORD drive)`
(body: `return protect_w(hdv_bpb, drive);`).

**Handler behind the vector:** `bios/blkdev.c:569` — `blkdev_getbpb()`,
installed as `hdv_bpb` by `blkdev_init()` (`bios/blkdev.c:101`).

**Helpers called:**
- `util/miscasm.S:225` — `protect_w(func, drive)` — d2/a2-preserving thunk.
- `bios/blkdev.c:625` — `blkdev_rwabs(RW_READ|RW_NOMEDIACH|RW_NOTRANSLATE,…)` —
  physical read of the boot sector into `dskbufp`.
- `bios/disk.c` — `disk_mediach()` (removable non-floppy re-scan).
- `bios/blkdev.c` — `getiword()` (big-endian field decode), `flop_checksum()`.

**Headers / type definitions used:**
- `include/biosdefs.h:67` — `BPB` struct (9 UWORDs = 18 bytes)
- `include/biosdefs.h:84-85` — `B_16`, `B_1FAT` flags
- `bios/blkdev.h:25-26` — `MAX_FAT12_CLUSTERS` (4084), `MAX_FAT16_CLUSTERS`
  (65524)
- `bios/blkdev.h` — `struct bs`, `struct fat16_bs` (boot-sector layout)

**System variables / globals read/written:**
- `hdv_bpb` (TOS var **`0x472`**) — the get-BPB vector (`bios/blkdev.c:101`)
- `blkdev[drive].bpb` — the returned struct (filled in place)
- `blkdev[drive].mediachange = MEDIANOCHANGE`, `.forcechange = FALSE` (reset,
  `bios/blkdev.c:589-590`)
- `dskbufp` — the shared boot-sector read buffer

**Build configuration affecting this function:**
- `CONF_WITH_1FAT_SUPPORT` — enables the single-FAT (`B_1FAT`) path and its
  `fatrec` computation (`bios/blkdev.c:711,753-756`).

---

## Source extraction summary

`Getbpb` returns a pointer to the BIOS Parameter Block for a logical drive,
re-reading and re-parsing the boot sector every call (no caching). The
trap-level `getbpb` forwards to `hdv_bpb`; on EmuTOS that is `blkdev_getbpb`.

`blkdev_getbpb` validates the drive (`0..BLKDEVNUM-1`, `DEVICE_VALID`),
**resets** the drive's `mediachange`/`forcechange` state, primes `bpb.recsiz`
(real size for floppies, 0 = "invalid" for hard units until proven valid),
optionally re-scans a removable unit's partitions, then reads the boot sector
via physical `blkdev_rwabs`. It decodes the (big-endian) boot-sector fields
with `getiword`, validates sector size against `pun_info.max_sect_siz`,
cluster size against `MAX_CLUSTER_SIZE`, and the FAT count, then computes the
9 BPB fields (`recsiz`, `clsiz`, `clsizb`, `rdlen`, `fsiz`, `fatrec`,
`datrec`, `numcl`, `b_flags`). It classifies FAT12 vs FAT16 purely by cluster
count (`numcl > MAX_FAT12_CLUSTERS` ⇒ `B_16`), and rejects FAT32
(`numcl > MAX_FAT16_CLUSTERS` **or** `fsiz == 0`). On success it returns
`(LONG)&blkdev[drive].bpb`; on **any** failure it returns `0L` (NULL).

There is no separate error code — the only failure signal is the NULL return.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x07
- **Stack frame on entry (m68k):**
  ```
  +2: drive (word)
   0: function_id (word) = 0x0007
  ```
- **Total parameter bytes:** 4
- **Native (AArch64) ABI:** ID `0x07` in `x8`; `drive` in `x0`; **the full
  64-bit pointer result (or 0) in `x0`** — this is a `BPB *`, not a
  sign-extended WORD.

  | m68k stack | AArch64 reg | Name    | Type / size |
  |------------|-------------|---------|-------------|
  | +2         | `x0`        | `drive` | `WORD` (16) |

---

## Inputs

| Position | Name    | Type   | Size | Source location     | Notes |
|----------|---------|--------|------|---------------------|-------|
| arg0     | `drive` | `WORD` | 2    | `bios/blkdev.c:569` | logical drive 0=A:..25=Z: **[BIOS02]**; validated against `BLKDEVNUM` (26) |

---

## Outputs

### Return value

- **Register:** D0 / `x0` (full pointer width)
- **Type:** `BPB *` (LONG on m68k)
- **Encoding:** non-NULL pointer to the drive's 18-byte BPB on success; `0L`
  (NULL) on any failure. The pointer is into a BIOS-owned per-drive struct —
  the caller must not free or write it.
- **Source location:** `bios/blkdev.c:776` (success) / `:584,615,634,646,655,
  669,741` (each NULL return).

### Side effects on global state

- `blkdev[drive].bpb` fully rewritten (`bios/blkdev.c:675-762`).
- `blkdev[drive].mediachange = MEDIANOCHANGE`, `.forcechange = FALSE`
  (`bios/blkdev.c:589-590`) — clears any pending media-change.
- `blkdev[drive].geometry`, `.serial`, `.serial2` updated.
- `dskbufp` overwritten with the boot sector.
- For a removable non-floppy unit, `disk_mediach(unit)` may re-scan partitions
  (`bios/blkdev.c:608-609`).

### Side effects on hardware

- Reads the physical boot sector from the drive.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `drive` invalid / not `DEVICE_VALID` | `bios/blkdev.c:581` | `0L` (NULL) |
| Device does not support GetBPB (`!GETBPB_ALLOWED`) | `bios/blkdev.c:612` | `0L` |
| Boot sector unreadable (I/O error) | `bios/blkdev.c:631` | `0L` |
| Logical sector size > `pun_info.max_sect_siz` | `bios/blkdev.c:642` | `0L` |
| Cluster size 0 or > `MAX_CLUSTER_SIZE` | `bios/blkdev.c:651` | `0L` |
| FAT count < `MIN_FATS` or > `MAX_FATS` | `bios/blkdev.c:666` | `0L` |
| FAT32 (`numcl > MAX_FAT16_CLUSTERS` or `fsiz == 0`) | `bios/blkdev.c:736` | `0L` (also sets `recsiz=0`) |

**No error code is ever returned — only `0L`.** To distinguish "drive absent"
from "unreadable/unsupported filesystem" the caller must consult `Mediach`
and/or prior state; `Getbpb` collapses all of it to NULL.

**Error paths not in Compendium:** the `!GETBPB_ALLOWED` and FAT32 branches
are EmuTOS-specific NULL returns.

---

## Verification criteria

- **Nominal (FAT16):** `Getbpb(2)` on a valid FAT16 C: returns a non-NULL
  `BPB *` whose `recsiz == 512`, `b_flags & B_16 != 0`, and `clsizb ==
  recsiz*clsiz`.
- **Nominal (FAT12):** on a valid FAT12 floppy A:, `Getbpb(0)` returns
  non-NULL with `b_flags & B_16 == 0` (`numcl <= 4084`).
- **FAT32/unsupported:** `Getbpb` on a FAT32 partition returns `0L`.
- **Absent drive:** `Getbpb(15)` with no P: returns `0L`.
- **State reset:** after `Getbpb(d)`, `Mediach(d)` no longer reports the
  media change that `Getbpb` consumed (mediachange reset to `MEDIANOCHANGE`).

---

## Edge cases discovered in source

1. **No caching — always re-reads the boot sector.** Every call re-does the
   physical read and rebuild (`bios/blkdev.c:624-776`), so a media change is
   always reflected. Programs that only want to *test* for change must use
   `Mediach`, not `Getbpb`.

2. **`Getbpb` resets the media-change state.** It sets `mediachange =
   MEDIANOCHANGE` and `forcechange = FALSE` up front (`:589-590`) — this is the
   documented "clears the change flag" behaviour, **not** a GEMDOS FAT-cache
   flush. (The prior spec's "flushes any GEMDOS in-memory FAT cache" claim is
   not in the source and has been removed.)

3. **FAT type from cluster count only.** `B_16` is set iff `numcl >
   MAX_FAT12_CLUSTERS` (4084) — the Microsoft rule, cluster-count based, not
   media-size based (`bios/blkdev.c:750-752`).

4. **FAT32 fails softly.** `numcl > MAX_FAT16_CLUSTERS` (65524) or `fsiz == 0`
   ⇒ mark `recsiz = 0` (XHDI "invalid BPB") and return NULL
   (`bios/blkdev.c:736-742`). A subsequent logical `Rwabs` on that drive then
   returns `ESECNF` (see `04-rwabs.md`).

5. **Zero-FAT boot sector accepted as 2 FATs.** For TOS compatibility, `fat ==
   0` is treated as 2 (`bios/blkdev.c:662-665`). `reserved == 0` is treated as
   1 (`:703-704`). NewTOS does both (`tos-api/src/bios/bpb.rs:137@parse`).

8. **`pun_info.max_sect_siz` has two values over time [BIOS02, BIOS-STOR-004].**
   While `pun_info_setup` runs (boot, once per valid hard-disk partition) it
   is `MAX_LOGSEC_SIZE = 32768` (`bios/blkdev.c:122`,
   `bios/blkdev.h:30`). When it finishes it is replaced by
   `max(largest bpb.recsiz of a GETBPB_ALLOWED hard-disk partition, 512)`,
   where 512 = `CONF_LOGSEC_SIZE` (`bios/blkdev.c:134,147-150,159`,
   `include/config.h:1606`). The test at `bios/blkdev.c:642` uses whatever
   value is current. So **after boot** a boot sector with `bps` above that
   maximum is refused (NULL), even though the same sector would have been
   accepted *during* boot. Measured by the audit probe (X01: `bps` 1024 →
   EmuTOS NULL, NewTOS BPB) — **not** by `bios-r2.tsv` (no case yet).

9. **Total sectors: `tot16`, else `tot32`, else 0 clusters [BIOS02,
   BIOS-STOR-005].** For a hard-disk unit (`unit >= NUMFLOPPIES`), if the
   16-bit total is 0 the 32-bit field (`sec2`) is used **whatever its value**
   (`bios/blkdev.c:730-731`); a total smaller than `datrec` gives a cluster
   count of 0 (`:732-733`) — not an error, so `Getbpb` returns a BPB with
   `numcl == 0` and no `B_16` (audit X06: `…,289,0,0`; the drive is then
   unusable but logged in). There is no fallback to the partition length.

6. **Per-drive pointer, stable across drives.** `Getbpb(0)` and `Getbpb(1)`
   return distinct pointers (each into its own `blkdev[].bpb`) that both stay
   valid; a re-call for the *same* drive rewrites that drive's struct in place.

7. **`datrec` computed for all cases as `fatrec + fsiz + rdlen`.** For the
   2-FAT case `fatrec` already includes `+fsiz` (second FAT); for 1-FAT it does
   not (`bios/blkdev.c:705-713`).

---

## Dependencies

### Requires (must be initialized / functional)

- `hdv_bpb` populated (`blkdev_init`).
- `hdv_rw`/`hdv_mediach` functional (used internally).
- `dskbufp` allocated; a live disk/floppy driver.

### Used by

- GEMDOS drive-login / mount logic; drive-enumeration utilities; anything
  needing sector/cluster geometry.

---

## Related data structures

- `docs/api/structures/bpb.md` — the returned 18-byte BPB
- `docs/api/bios/_state.md` — `hdv_bpb` vector + address

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically documents
  NULL-on-failure and pointer-into-BIOS-buffer.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `BLKDEVNUM = 26` (`include/sysconf.h:43`) **[BIOS02, corrected from 16]**. `MAX_FAT12_CLUSTERS = 4084`, `MAX_FAT16_CLUSTERS = 65524`.
- `CONF_WITH_1FAT_SUPPORT` toggles the single-FAT path.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-04: NewTOS returns a `BPB *` only for FAT12/FAT16 logical drives,
matching EmuTOS; FAT32/exFAT partitions return NULL from `Getbpb` and are
reached through a NewTOS-native path instead. Note `Getbpb`'s result is a full
64-bit pointer in `x0` for native callers.

FS09 (2026-10-07): the boot sector is read through the block read cache
(`docs/analysis/fs09-block-cache-2026-10-07.md` §2.7). `Getbpb` itself does not
invalidate the cache — as in FreeMiNT, where invalidation follows a detected
media change (`fatfs_dskchng` → `bio_free_di`) and `Getbpb` is only the re-read
after it. Every write to the card passes the cache, so the cached boot sector
is the card's.

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the Atari FDC / ACSI boot sector to the
> **BCM2711 EMMC2 SD card**. The contract above (NULL-on-failure, the 18-byte
> BPB, media-state reset, FAT type from cluster count) is unchanged; only the
> backing hardware and the concrete NewTOS fields are pinned here. See
> `docs/HARDWARE-PROFILE.md` §3.4.

### Where the BPB comes from

The BPB is **synthesized from the FAT16 boot sector of the mounted partition**,
not from Atari media geometry. `Getbpb` (`tos-api/src/bios/bpb.rs:160-167`):

1. resolves `drive`→partition via the MBR-built drive table
   (`drive::resolve`, see `0a-drvmap.md`);
2. reads **partition-relative sector 0** (the FAT boot sector) from the EMMC SD
   at `part.start_lba` via `BlockDev::read_blocks` (`bpb.rs:139-148`) — which
   forwards to `emmc.rs` (see `04-rwabs.md`);
3. parses the DOS/BPB fields and caches the result in a per-drive static
   (`TABLE`, one slot per drive, `bpb.rs:53-56,150-153`);
4. returns a native 64-bit pointer into that slot (`bpb.rs:166`), or `0`.

Every call re-reads and re-parses (no caching of the sector) — matching the
EmuTOS "always re-reads" contract.

### The exact fields NewTOS fills (`bpb.rs:60-103`)

All fields decoded **little-endian** (no byte-swap on ARM) from the boot-sector
BPB at the standard offsets:

| BPB field | Source (boot-sector offset) | Computation |
|-----------|-----------------------------|-------------|
| `recsiz` | `0x0B..0x0C` bytes/sector | direct |
| `clsiz` | `0x0D` sectors/cluster | direct |
| `clsizb` | — | `recsiz * clsiz` |
| `rdlen` | `0x11..0x12` root entries | `ceil(root_entries*32 / recsiz)` sectors |
| `fsiz` | `0x16..0x17` sectors/FAT (16-bit) | direct |
| `fatrec` | `0x0E..0x0F` reserved (0 → 1), `0x10` num_fats | `reserved`, plus `fsiz` if `num_fats >= 2` (the second FAT; `bios/blkdev.c:703-712`; `MAX_FATS = 2` so this equals `reserved + (num_fats-1)*fsiz`) |
| `datrec` | reserved, num_fats, rdlen | `reserved + num_fats*fsiz + rdlen` |
| `numcl` | total sectors, `datrec`, `clsiz` | `(total - datrec) / clsiz` |
| `b_flags` | `numcl`, num_fats | `B_16` if `numcl >= 4085`; `B_1FAT` if `num_fats == 1` |

- **Total sectors:** `tot16` (`0x13..0x14`) if non-zero, else `tot32`
  (`0x20..0x23`) whatever its value (`tos-api/src/bios/bpb.rs:202@tot32`) —
  **[BIOS01]** as EmuTOS, edge case 9 (BIOS-STOR-005). Before: both 0 fell
  back to the partition length from the MBR.
- **FAT type = cluster count** (`numcl >= 4085` ⇒ `B_16`), the Microsoft rule,
  matching EmuTOS (`bpb.rs:95-97`).

### FAT16-only; FAT32 / exFAT return NULL

`parse` returns `None` (→ `Getbpb` returns `0`) when
`recsiz==0 || clsiz==0 || num_fats==0 || fsiz16==0` (`bpb.rs:71-73`). A FAT32
boot record has its 16-bit sectors-per-FAT field (`0x16`) zeroed, so `fsiz16==0`
⇒ NULL — the same "FAT32 fails softly" outcome as EmuTOS. exFAT (no classic BPB)
likewise fails. FAT16 hard-disk is the scope; FAT12/floppy is out of scope and
exFAT is a Phase-09 goal (`docs/HARDWARE-PROFILE.md` §3.4).

### Gaps vs. the contract (Step-2 candidates)

- **[BIOS01] implemented** for `forcechange`: `log_in` (formerly
  `boot_sector`), shared by the trap's `bpb_for`, the BDOS log-in's `geometry`
  and the boot scan `scan`, clears it after the drive test and before the read
  (`tos-api/src/bios/bpb.rs:274@clear_forcechange`);
  W06/W07, X15/X16 and `bios-r2-x001.tsv` EQUAL with the state armed
  (`measurements/measurement-2026-10-28-bios01-s2/RUNINFO.TXT`). There is still
  no `mediachange` latch to reset. Before:
  **Media-state reset not implemented.** EmuTOS resets
  `mediachange = MEDIANOCHANGE`, `forcechange = FALSE` at the top of `Getbpb`
  (`bios/blkdev.c:589-590`) — right after the drive-valid test and **before**
  any test of the boot sector, so a `Getbpb` that returns NULL clears it too.
  NewTOS keeps **no per-drive media/force flags** (`Mediach`≡0), so there is
  nothing to reset. **[BIOS02]** Once the `forcechange` handshake is built
  (BIOS-STOR-001, `04-rwabs.md`), the reset must be in `getbpb` at the same
  place: after the drive check, before parsing, and for every outcome. Oracle
  W06/W07, X15/X16 show the reset working on EmuTOS; they are EQUAL on NewTOS
  only because the state never arms.
- ~~**No sector-size / cluster-size / FAT-count validation branches.**~~
  **[BIOS02, superseded]** The guards are in `parse`
  (`tos-api/src/bios/bpb.rs:137@parse`: sector size, cluster bytes, FAT count
  1..2, `reserved == 0`, FAT32 test) and the audit probe found them equal to
  EmuTOS (X02-X05, X07 equal on both sides) — except the two differences
  below.
- ~~**Zero-FAT→2 quirk not applied.**~~ **[BIOS02, superseded]** applied
  (`bpb.rs:137@parse`, `num_fats_raw == 0` → 2).
- **[BIOS02] BIOS-STOR-004 — sector-size limit is the boot-time constant.**
  **[BIOS01] fixed** by the boot scan, EmuTOS `pun_info_setup`
  (`tos-api/src/bios/bpb.rs:315@scan`, called by `bios::mount` after the
  partition scan): a `Getbpb` of every drive with the limit at
  `MAX_LOGSEC_SIZE` = 32768, then `MAX_SECT_SIZ` = max(largest `recsiz`
  logged in, `CONF_LOGSEC_SIZE` 512) (`tos-api/src/bios/bpb.rs:78@MAX_SECT_SIZ`);
  `parse` tests against it (`tos-api/src/bios/bpb.rs:150@max_sect_siz`). On a
  card whose partitions all have 512-byte sectors, `bps` 1024 is refused
  (X01 NULL, `bios-r2-x004.tsv`). Not reproduced: the AHDI `PUN_INFO` /
  `pun_ptr` and the `GETBPB_ALLOWED` filter on the maximum (`blkdev.c:148`;
  NewTOS logs in every partition whatever its MBR type). Before: the boot
  constant 32768 held forever (probe X01: EmuTOS NULL, NewTOS BPB).
  Remaining consequence: NewTOS's `Rwabs` does not scale logical to physical
  sectors (`04-rwabs.md` step 7, `bios/blkdev.c:456-458`), so a `recsiz` >
  512 that the boot scan logged in is addressed wrongly (audit, *not
  measured*) — now only on a card that has such a partition at boot.
- **[BIOS01] BIOS-STOR-006 — the invalid-BPB state.** EmuTOS marks the BPB
  invalid (`recsiz = 0`) right after the media-state reset (`blkdev.c:601`,
  hard-disk units) and only a boot sector that passes every test sets it
  again; the FAT32 exit sets it 0 explicitly (`:736-741`). A logical `Rwabs`
  tests it first (`04-rwabs.md` step 4). NewTOS: `log_in` zeroes the slot's
  `recsiz` after the reset (`tos-api/src/bios/bpb.rs:277@recsiz`) and writes
  the whole BPB only on success; `bpb::recsiz` is what `Rwabs` tests. Every
  `Getbpb` keeps this state — the trap's, the BDOS log-in's (`geometry`, which
  before BIOS01 did not write the slot) and the boot scan's. Not reproduced: on
  the FAT32 exit EmuTOS has already overwritten `clsiz`..`datrec` of the slot
  (`blkdev.c:675-713`); NewTOS leaves the eight fields other than `recsiz` as
  the last successful `Getbpb` left them. Visible only through a pointer an
  earlier `Getbpb` returned; not measured.
- **[BIOS02] BIOS-STOR-005 — `tot16 == tot32 == 0` replaced by the partition
  length.** **[BIOS01] fixed:** `parse` takes `tot16`, else `tot32` whatever
  its value (`tos-api/src/bios/bpb.rs:202@tot32`); both 0 give `numcl` 0, no
  `B_16`, and a BPB (edge case 9, `bios-r2-x005.tsv` X06b). Before: the
  partition length (`part_sectors`), probe X06 `…,289,0,0` vs
  `…,289,32672,1`. Intent: the code comment gave none and the introducing
  commit `b247f99f` (first implementation of `Getbpb`) recorded none; no
  later doc or ADR mentioned it. **Decided (user 2026-10-28): follow
  EmuTOS.**
- **`Geometry` extension:** NewTOS additionally exposes `reserved`+`num_fats`
  raw (`bpb.rs:105-134`) for the FAT write path — a NewTOS-native superset, not
  part of the Atari ABI BPB.
- **The returned `BPB *` was a kernel address an EL0 program could not read —
  MEASURED 2026-09-05, CLOSED the same day (ADR-023).** `getbpb` used to
  return `&TABLE[drive]`, a pointer into a kernel static. The kernel is linked
  at `0xFFFF_0000_0000_0000 + physical`
  (`kernel/src/arch/aarch64/link-rpi4.ld:24`) and those pages are EL1-only, so
  an application that followed the pointer took a permission fault instead of
  reading its BPB. Measured on QEMU `raspi4b`: `Getbpb(2)` returned
  `0xFFFF00000026D8F4` (`APITEST` check BB18 FAIL; the field checks
  BB19..BB28 were consequently unrunnable from EL0).

  **Now:** the BPB table lives in a page of its own and is published
  read-only — FreeMiNT `PROT_PR` (`references/freemint/sys/mint/mem.h:133`)
  — via `bdos::process::usermem::expose`; `Getbpb` returns the **user
  address** (ADR-023). Since ARCH03 stage 1 (2026-10-05) that is the table's
  own physical address, published once into the user address space all
  processes share (VA = PA), instead of a per-process window VA. All 16
  entries share one page, so it is one publication for every drive. An in-kernel (EL1) caller — `bdos::fs::fat` via
  `bpb_for`, the desktop, the boot self-tests — still gets the kernel VA,
  which is what *it* can read. If the publication cannot be made,
  the answer is `0` (the contract's "no BPB"), never an address the caller
  would fault on. Measured after the change on QEMU `raspi4b`:
  `Getbpb(2) = 0x00401024`, `APITEST` BB17..BB28 all PASS (every BPB field
  checked against the boot sector actually read from the medium).

### Measured cross-check — `bios-r2.tsv` cases per statement [BIOS02]

Oracle `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN`; NewTOS run
`measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`. Dumper no. = line of
the row in `tosdump/cases/bios-r2.tsv`. Oracle BPB of the test image:
`512,4,2048,32,128,129,289,32672,1`.

| Statement | Cases (label : dumper no.) | EmuTOS | NewTOS | Verdict |
|-----------|----------------------------|--------|--------|---------|
| BPB fields of a FAT16 partition (18 bytes through the returned pointer) | `Getbpb(2)` 42; W06 97; X15 108 | BPB above | same | EQUAL |
| absent / out-of-range drive → NULL | `Getbpb(5/15/16/-1/0x7FFF)` 43-47 | 0 | 0 | EQUAL |
| `Getbpb` after the forcechange state returns the BPB (and clears it) | W06 97, X15 108 | BPB | BPB | EQUAL (state never armed on NewTOS) |
| BIOS-STOR-004 (`bps` 1024), -005 (`tot16 = tot32 = 0`), -006 (`spf` 0, then `Rwabs`) | — | NULL / `…,289,0,0` / `ESECNF` | BPB / `…,289,32672,1` / data | **no `bios-r2` case** — audit probe X01, X06, X07/X08 only (PLAUSIBLE) |

Since measured by the closing tables `tosdump/cases/bios-r2-x004.tsv`,
`-x005.tsv`, `-x006.tsv` (`measurements/measurement-2026-10-28-tool07b/RUNINFO.TXT`);
**after BIOS01** all three 0 cases differ and the end states equal the
oracle's (`measurements/measurement-2026-10-29-bios01-rest/RUNINFO.TXT`).

Not measured at all: FAT12 floppy path (out of scope), removable-unit rescan
(`disk_mediach` at `bios/blkdev.c:608-609`), `!GETBPB_ALLOWED` partition types,
the `Geometry`/serial fields.
