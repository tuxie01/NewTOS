# `Dfree` (GEMDOS 0x36)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsfat.c:337` (`xgetfree`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `+1` drive convention, the 4-LONG output-buffer
> layout, the single `ERR` (-1) error return. Reference: the DMD geometry fields
> the values are copied from, `ckdrv`, `countfree16`/`getrealcl`.

## EmuTOS source

**Primary location:** `bdos/fsfat.c:337` — `long xgetfree(long *buf, int drv)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x36` =
`xgetfree`, `wparms = 3` (`bdos/bdosmain.c:248`).

**Helpers called:**
- `bdos/fsdrive.c:77` — `ckdrv(drv, TRUE)` — validate + lazily log in the drive;
  **`TRUE` requests a media-change check** on removable media.
- `bdos/fsfat.c:298` — `countfree16(dm)` — fast free-cluster count for 16-bit
  FATs (scans `CLNO`-sized FAT slots directly).
- `bdos/fsfat.c` — `getrealcl(cl, dm)` — read one cluster's FAT entry (the slow
  12-bit path, one cluster at a time).

**Headers / type definitions used:**
- `bdos/fs.h:240-266` — `DMD` (Drive Media Block); fields `m_numcl` (`CLNO`),
  `m_recsiz` (`UWORD`), `m_clsiz` (`WORD`), `m_16` flag.
- `bdos/fs.h:89` — `CLNO` = `UWORD`.
- `include/gemerror.h:20` — `ERR = -1`.

**System variables / globals read:** `run->p_curdrv` (when `drv == 0`);
`drvtbl[n]` (the DMD for the resolved drive, `bdos/fsglob.c:28`);
`drvsel`/`drvrem` (indirectly, inside `ckdrv`).
**System variables / globals written:** the 4-LONG buffer at `buf`; `ckdrv` may
lazily populate `drvtbl[]`/`dirtbl[]` and, on a detected media change, `longjmp`
out to the trap dispatcher (which re-logs the drive).

**Build configuration affecting this function:** none (the 12-bit vs 16-bit FAT
paths are runtime-selected via `dm->m_16`, not a build flag).

---

## Source extraction summary

`Dfree` reports free-space geometry for a drive into a caller-supplied 16-byte
buffer (four LONGs). `xgetfree` first maps the drive argument: `drv == 0` means
"current drive" (`run->p_curdrv`), otherwise `drv-1` is the 0-based drive (so
`1 = A:`, `2 = B:`, …) — the **`+1` convention**, the same as `Dgetpath` and the
opposite of `Dsetdrv`/`Dgetdrv`.

It then calls `ckdrv(drv, TRUE)`. `ckdrv` validates the drive, logs it in if it
has not been accessed yet, and — because the second argument is `TRUE` — runs a
`Mediach` check on removable media, which can trigger a media-change `longjmp`
(re-login) before the count proceeds. If `ckdrv` returns a negative value (any of
`EDRIVE`, `EPTHNF`, or `ENSMEM`), `xgetfree` collapses it to the generic
`ERR = -1` and returns (the specific `ckdrv` code is discarded).

On success it counts free clusters. For a 16-bit FAT (`dm->m_16` set) it uses the
fast `countfree16` scan; for a 12-bit FAT it loops every cluster from 2 upward
calling `getrealcl`, counting entries whose value is 0 (free). It then writes four
LONGs: free clusters, total clusters (`m_numcl`), bytes per logical record/sector
(`m_recsiz`), and records per cluster (`m_clsiz`), and returns `E_OK`.

Note cluster numbering starts at 2 (clusters 0 and 1 are the FAT's reserved
media-descriptor / end-of-chain slots), which is why the slow loop reads
`getrealcl(i+2, dm)`.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x36
- **Stack frame on entry (m68k):** `Dfree(buf, drive)` = `trap1(0x36, buf, drive)`:
  ```
  +6: drive (word)
  +2: buf   (LONG, pointer)
   0: function_id (word) = 0x0036
  ```
- **Total parameter bytes:** 6 (LONG + WORD) → `wparms = 3`.
- **Native (AArch64) ABI:** ID `0x36` in `x8`; args in `x0..x1`; LONG result
  (`E_OK`/`ERR`) sign-extended into `x0`. The geometry is delivered via the
  **output buffer** at `buf`, not the register.

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `buf` | `long *` (ptr) — 16-byte / 4-LONG buffer |
  | +6         | `x1`        | `drv` | `WORD` (16) — `0 = current`, `1 = A:`, … |

---

## Inputs

| Position | Name  | Type    | Size | Source location     | Notes |
|----------|-------|---------|------|---------------------|-------|
| arg0     | `buf` | `long *`| 4    | `bdos/fsfat.c:337`  | pointer to a 16-byte (4-LONG) output buffer; **not validated** |
| arg1     | `drv` | `int`   | 2    | `bdos/fsfat.c:343`  | `0 = current drive`, else `drv-1` = 0-based drive (`1 = A:`) |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `E_OK` (0) on success; `ERR` (-1) on any `ckdrv` failure.
- **Source location:** `bdos/fsfat.c:346` (`return ERR`), `:366` (`return E_OK`).

### Output buffer (`buf`, 16 bytes = 4 LONGs)

Written at `bdos/fsfat.c:361-364`:

| Index | Field    | Source (DMD)  | Meaning |
|------:|----------|---------------|---------|
| `buf[0]` | free clusters | computed | number of free (unallocated) clusters |
| `buf[1]` | total clusters | `m_numcl` | total data clusters on the volume |
| `buf[2]` | bytes/sector | `m_recsiz` | bytes per logical record (sector) |
| `buf[3]` | sectors/cluster | `m_clsiz` | logical records per cluster |

Derived values a caller computes: bytes-per-cluster = `buf[2] * buf[3]`;
free bytes = `buf[0] * buf[2] * buf[3]`; total bytes = `buf[1] * buf[2] * buf[3]`.

### Side effects on global state

- `ckdrv` may log in a not-yet-accessed drive (populates `drvtbl[]`, a `dirtbl[]`
  root slot, sets the bit in `drvsel`).
- On a detected media change (`TRUE` flag), `ckdrv` `longjmp`s to the trap
  dispatcher, which frees the old DMD/DND tree, invalidates the drive's buffer
  cache, re-reads the BPB, and re-runs the call (`bdos/bdosmain.c:449-487`).

### Side effects on hardware

- Reads FAT sectors through the buffer cache to count free clusters. The
  `Mediach` probe (removable media) may touch the block device.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `ckdrv` failed — invalid drive, unmounted removable drive, or out-of-memory logging in | `bdos/fsfat.c:345-346` | `ERR` (-1) |

`ckdrv` internally distinguishes `EDRIVE` (-46), `EPTHNF` (-34), and `ENSMEM`
(-39) (`bdos/fsdrive.c:89-102`), but `xgetfree` discards the specific code and
returns the generic **`ERR` (-1)** for all of them.

**Error paths not in Compendium:** Compendium documents `EDRIVE` here; EmuTOS
(and real Atari TOS) actually return `ERR = -1`. See Discrepancy log D-026.

---

## Verification criteria

- **Nominal:** on a mounted FAT16 C: with known geometry, `Dfree(buf, 3)`
  returns `E_OK` and fills `buf[1] = m_numcl`, `buf[2] = 512` (typical),
  `buf[3] = sectors/cluster`, with `buf[0] <= buf[1]`.
- **Current drive:** `Dfree(buf, 0)` reports the geometry of `Dgetdrv()`'s drive.
- **Error — bad drive:** `Dfree(buf, 20)` (drive T:) on a system without that
  drive returns `ERR` (-1), **not** `EDRIVE`.
- **Free ≤ total invariant:** `buf[0] <= buf[1]` always holds on a consistent
  volume.

---

## Edge cases discovered in source

1. **`+1` drive convention.** `drv = 0` is "current"; `drv = 1` is A:. Same as
   `Dgetpath`; opposite of `Dsetdrv`/`Dgetdrv` (0-based). (`bdos/fsfat.c:343`)

2. **Any `ckdrv` failure collapses to `ERR` (-1).** The distinct `EDRIVE`/
   `EPTHNF`/`ENSMEM` codes are discarded (`bdos/fsfat.c:345-346`). This is a
   Compendium discrepancy (D-026).

3. **`ckdrv(drv, TRUE)` triggers a media-change check.** On removable media the
   `TRUE` flag runs `Mediach`; a swap causes a re-login via `longjmp`. Programs
   use `Dfree(buf, d)` as a cheap "is the disk still there / has it changed?"
   probe. (`bdos/fsdrive.c:106-115`)

4. **Cluster numbers start at 2.** The slow 12-bit loop reads `getrealcl(i+2)`
   for `i = 0..m_numcl-1`; clusters 0/1 are FAT-reserved. (`bdos/fsfat.c:356-357`)

5. **12-bit vs 16-bit FAT selected at runtime.** `dm->m_16` picks the fast
   `countfree16` path; a 12-bit FAT falls back to the per-cluster loop
   (`getrealcl` crosses byte boundaries, so no bulk scan). This is a runtime
   choice, not a build flag. (`bdos/fsfat.c:349-359`)

6. **No buffer validation.** `xgetfree` never checks that `buf` is a valid,
   16-byte-writable pointer; a bad pointer faults at the store. NewTOS should
   validate the user pointer at trap entry. (`bdos/fsfat.c:361-364`)

7. **O(N) on large volumes.** Counting free clusters is linear in the cluster
   count; on a multi-GB volume this is slow. AES caches the result; tight
   `Dfree` loops noticeably slow down on large drives.

---

## Dependencies

### Requires (must be initialized / functional)

- `ckdrv`, `drvtbl[]`, a logged-in DMD for the drive.
- The FAT buffer cache (`getrec`) for the free-cluster scan.
- `run->p_curdrv` (for `drv == 0`).

### Used by

- `df`-style tools, file managers showing "X bytes free", the GEM disk-info
  dialog.

---

## Related data structures

- `docs/api/gemdos/_state.md` §4 — drive descriptors (`drvtbl[]`, DMD) and the
  current-drive model; §11 — the `Dfree` output-buffer layout.
- `docs/api/structures/bpb.md` — the BPB the DMD geometry derives from.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the 4-LONG
  buffer layout and — incorrectly — `EDRIVE` on error; EmuTOS returns `ERR`. See
  D-026.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None; the 12/16-bit FAT split is runtime, not a build option.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: backed by the FAT driver. The precise (slow) free-cluster scan is kept
for ABI fidelity; a cached "fast Dfree" is a possible free-implementation
extension that must return the same buffer values. Return `ERR` (-1), not
`EDRIVE`, on bad drive for compatibility.

TASK/HW08 (2026-11-06): a FAT sector that cannot be had ends the call with the
device's code and nothing stored, as `countfree16`'s `getrec` long-jumps
(`bdos/fsfat.c:312`) before `xgetfree` stores anything (`:361-364`). Oracle,
case `r1`: `Dfree` -11, buffer untouched, the next `Dfree` right. Before, NewTOS
stopped counting at the first failed sector and answered 0 with a wrong number.
A FAT sector still dirty counts as it stands in the cache on both sides.
