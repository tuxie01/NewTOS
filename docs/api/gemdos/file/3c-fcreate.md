# `Fcreate` (GEMDOS 0x3C)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsopnclo.c:81` (`xcreat`) and the work helper
> `bdos/fsopnclo.c:99` (`ixcreat`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: the trap ID/ABI, the `attr` bit meanings, the returned handle
> being open **read-write** regardless of `FA_RO`, and the error codes.
> Reference (free to reimplement): the FCB pool, `scan`/`findit`/`opnfil`
> internals, directory-grow strategy.

## EmuTOS source

**Primary location:** `bdos/fsopnclo.c:81` — `long xcreat(char *name, UBYTE attr)`.
It strips `FA_SUBDIR` (`attr & ~FA_SUBDIR`) and forwards to `ixcreat`.

**Installed handler / vector:** (none) — dispatched directly from the
GEMDOS `funcs[]` table, entry `0x3C` = `xcreat`, `wparms=3`
(`bdos/bdosmain.c:256`). No indirect vector.

**Helpers called:**
- `bdos/fsopnclo.c:99` — `ixcreat()` — the actual create logic.
- `bdos/fsdir.c:1407` — `findit()` — resolve the path to a parent `DND`.
- `bdos/fsdir.c:1525` — `scan()` — search the directory for a name / free slot.
- `bdos/fsopnclo.c:652` — `ixdel()` — delete a pre-existing entry of the same name.
- `bdos/fsopnclo.c:335` — `opnfil()` — allocate the SFT slot + build the OFD.
- `bdos/fsfat.c` — `nextcl()` — grow a non-root directory when full.
- `bdos/fsdir.c:1257` — `dirinit()` — re-init the directory after growth.
- `bdos/fsopnclo.c:708` — `contains_illegal_characters()`.
- `bdos/fsdir.c` — `builds()` (build the 11-byte FCB name), `ixgetfcb()`,
  `ixlseek()`, `ixwrite()`, `ixclose()`.

**Headers / type definitions used:**
- `include/bdosdefs.h:36-41` — `FA_RO`/`FA_HIDDEN`/`FA_SYSTEM`/`FA_VOL`/
  `FA_SUBDIR`/`FA_ARCHIVE`.
- `bdos/fs.h:170-173` — `RO_MODE`/`WO_MODE`/`RW_MODE`.
- `bdos/fs.h:183-193` — `FCB` (32-byte on-disk directory entry), `ERASE_MARKER`.
- `bdos/fs.h:32` — `FNAMELEN = 11`.
- `bdos/fsopnclo.c:64` — `ILLEGAL_FNAME_CHARACTERS = " *,:;<=>?[]|"`.

**System variables / globals read:**
- `run` (current PD) — for the drive/path context via `findit`.
- `current_time`, `current_date` (`bdos/time.c:53`) — timestamp stamped into
  the new FCB (**byte-swapped** to little-endian on disk).

**System variables / globals written:**
- `sft[]` (`opnfil` claims a free entry; `bdos/fsopnclo.c:341-350`).
- On-disk directory sector (via the buffer cache) + FAT cache buffers.

**Build configuration affecting this function:** none — always present.

---

## Source extraction summary

`Fcreate` creates (or truncates) a file and returns an open **read-write**
handle to it. `xcreat` first masks off `FA_SUBDIR` so `Fcreate` can never make
a directory (use `Dcreate`, 0x39, for that). `ixcreat` resolves the parent
directory with `findit`; a missing path, an empty filename, or a name that is
`.`/`..` yields `EPTHNF`. The final name component is checked against the
illegal-character set `" *,:;<=>?[]|"` — note this is the *create/rename*
character set, **not** the same as the wildcard set, and it does **not**
include `/`, `\` or `"`.

Volume-label handling is special: if `FA_VOL` is set, no other attribute bit
may be set, and a label may only be created in the root directory (otherwise
`EACCDN`). When creating a label, *any* existing label matches and is deleted
first; when creating a normal file, the name must match and the existing entry
is deleted unless it is read-only, a folder, or the caller asked to create a
folder (all `EACCDN`). Deleting the pre-existing entry goes through `ixdel`,
which returns `< 0` (surfaced as `EACCDN`) if the file is open by *another*
process.

It then scans for a free directory slot, growing a non-root directory by one
cluster via `nextcl` when full (the root directory cannot grow → `EACCDN`).
The new FCB is written with the current time/date (byte-swapped to disk
little-endian), the requested attribute, a zero start-cluster and zero length,
and flushed. Finally `opnfil` allocates an SFT slot and builds the OFD in
`RO_MODE` if `FA_RO` was requested, else `RW_MODE` — **but the returned handle
can be written to regardless**, because EmuTOS does not enforce the access
code (see `Fwrite`). The DFD is marked `O_DIRTY` so `Fclose` writes the final
size/time back.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x3C
- **Stack frame on entry (m68k):** `Fcreate(fname, attr)` =
  `trap1(0x3c, fname, attr)` (`include/bdosbind.h:37`):
  ```
  +6: attr  (word)
  +2: fname (LONG, pointer)
   0: function_id (word) = 0x003C
  ```
- **Total parameter bytes:** 6 (LONG + WORD) → `wparms = 3` WORD slots.
- **Native (AArch64) ABI:** ID `0x3C` in `x8`; args in `x0..x1` in declaration
  order; LONG result (file handle or negative error) **sign-extended** into `x0`.

  | m68k stack | AArch64 reg | Name    | Type / size |
  |------------|-------------|---------|-------------|
  | +2         | `x0`        | `name`  | `char *` (ptr) |
  | +6         | `x1`        | `attr`  | `WORD` (16) — low 8 bits used |

---

## Inputs

| Position | Name   | Type     | Size | Source location        | Notes |
|----------|--------|----------|------|------------------------|-------|
| arg0     | `name` | `char *` | 4    | `bdos/fsopnclo.c:99`   | path of the file to create; last component is the filename |
| arg1     | `attr` | `UBYTE`  | (2→1)| `bdos/fsopnclo.c:81`   | attribute byte; `FA_SUBDIR` bit is stripped before use |

### `attr` bits (`include/bdosdefs.h:36-41`)

| Bit | Mask | Constant     | Effect in `Fcreate` |
|----:|-----:|--------------|---------------------|
| 0   | 0x01 | `FA_RO`      | read-only entry (handle still opens RW) |
| 1   | 0x02 | `FA_HIDDEN`  | hidden |
| 2   | 0x04 | `FA_SYSTEM`  | system |
| 3   | 0x08 | `FA_VOL`     | volume label — must be the *only* bit set, root only |
| 4   | 0x10 | `FA_SUBDIR`  | **stripped** by `xcreat` (`& ~FA_SUBDIR`) — never honoured |
| 5   | 0x20 | `FA_ARCHIVE` | archive |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** on success, a file handle `>= NUMSTD` (i.e. `>= 6`); on failure
  a negative TOS error code.
- **Source location:** `bdos/fsopnclo.c:201-208` (handle from `opnfil`).

### Side effects on global state

- Claims one `sft[]` entry (`f_own = run`, `f_use = 1`), builds an OFD/DFD.
- Writes a new directory entry (FCB) to disk via the buffer cache; may free a
  pre-existing same-name entry (delete + FAT-chain free) and may grow a
  non-root directory by one cluster.
- Marks the new DFD `O_DIRTY` (`bdos/fsopnclo.c:206`).

### Side effects on hardware

- Sector reads/writes to the underlying block device (through `Rwabs`).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Path (directory portion) not found | `bdos/fsopnclo.c:114-115` | `EPTHNF` (-34) |
| Empty filename, or name is `.`/`..` | `bdos/fsopnclo.c:117-118` | `EPTHNF` (-34) |
| Filename contains an illegal char (`" *,:;<=>?[]|"`) | `bdos/fsopnclo.c:121-122` | `EACCDN` (-36) |
| `FA_VOL` set together with any other bit | `bdos/fsopnclo.c:127-128` | `EACCDN` (-36) |
| `FA_VOL` requested outside the root dir | `bdos/fsopnclo.c:133-134` | `EACCDN` (-36) |
| Existing entry is a folder or read-only (non-label create) | `bdos/fsopnclo.c:159-160` | `EACCDN` (-36) |
| Existing entry open by **another** process (`ixdel` < 0) | `bdos/fsopnclo.c:163-164` | `EACCDN` (-36) |
| Root directory full and cannot grow | `bdos/fsopnclo.c:175-176` | `EACCDN` (-36) |
| Cannot allocate a new cluster to grow directory | `bdos/fsopnclo.c:178-179` | `EACCDN` (-36) |
| SFT exhausted | `bdos/fsopnclo.c:345-346` (`opnfil`) | `ENHNDL` (-35) |

**Error paths not in Compendium:** the exact illegal-character set is
`" *,:;<=>?[]|"` (`bdos/fsopnclo.c:64`) — narrower than the MS-DOS set and it
allows `+`; `/`, `\`, `"` are **not** rejected here (they are path separators or
handled elsewhere).

---

## Verification criteria

- **Nominal:** `Fcreate("C:\\NEW.TXT", 0)` on an existing, writable directory
  returns a handle `>= 6`; a following `Fwrite` on that handle succeeds even
  though the entry is a normal file.
- **Truncate-existing:** `Fcreate("C:\\EXIST.TXT", 0)` when `EXIST.TXT` is a
  normal, non-RO, unopened file deletes it and returns a fresh empty handle.
- **RO entry, RW handle:** `Fcreate("C:\\RO.TXT", FA_RO)` returns a handle that
  is still writable; the `FA_RO` flag only takes effect after `Fclose` +
  re-`Fopen`.
- **Error — bad path:** `Fcreate("C:\\NODIR\\F.TXT", 0)` where `NODIR` does not
  exist returns `EPTHNF` (-34).
- **Error — illegal char:** `Fcreate("C:\\A?B.TXT", 0)` returns `EACCDN` (-36).
- **Error — label misuse:** `Fcreate("C:\\X", FA_VOL|FA_RO)` returns `EACCDN`.

---

## Edge cases discovered in source

1. **`FA_SUBDIR` is silently stripped.** `xcreat` masks it off
   (`bdos/fsopnclo.c:85`); passing `0x10` yields a *regular* file, not a folder.

2. **Returned handle is always writable.** The OFD is opened `RO_MODE` when
   `FA_RO` is set (`bdos/fsopnclo.c:201`), but EmuTOS does not enforce the
   access code on `Fwrite`, so the caller can write immediately. The read-only
   *attribute* only bites on a later `Fopen` for write.

3. **Volume-label create deletes any existing label.** For `attr == FA_VOL`,
   the scan uses `"*.*"` with `FA_VOL` (`bdos/fsopnclo.c:150-151`) so *any*
   existing label matches and is removed first — regardless of its name.

4. **Existing-file overwrite is silent.** A matching non-RO, non-folder file is
   deleted (`ixdel`) and re-created empty (`bdos/fsopnclo.c:154-165`). There is
   no "already exists" error; "create or fail" callers must probe with
   `Fsfirst` first.

5. **Illegal-char set is create-specific and narrow.** `" *,:;<=>?[]|"` —
   allows `+`, does **not** list `/`, `\`, `"`. (`bdos/fsopnclo.c:64`.)

6. **Root directory cannot grow.** On a FAT12/16 volume the root has a fixed
   entry count; when full, create fails with `EACCDN`
   (`bdos/fsopnclo.c:175-176`). Sub-directories grow a cluster at a time.

7. **Start cluster and length are zero.** `f_clust = 0`, `f_fileln = 0`
   (`bdos/fsopnclo.c:194-195`); the first `Fwrite` triggers allocation.

8. **Timestamp is stamped at create time, byte-swapped to disk.**
   `f_td = current_time/current_date` then `swpw` to little-endian
   (`bdos/fsopnclo.c:190-193`).

---

## Dependencies

### Requires (must be initialized / functional)

- FAT subsystem logged in for the target drive (`findit`/`scan`, DMD/DND).
- `sft[]` with a free slot; the buffer cache and `Rwabs` path.
- `current_time`/`current_date` maintained by the BIOS clock.

### Used by

- `Frename` cross-directory path (`xrename` calls `ixcreat`,
  `bdos/fsdir.c:1014`).
- Every program that writes a new file; installers, shells, AES.

---

## Related data structures

- `docs/api/structures/ofd.md` — OFD/DFD/FCB layout, `o_mod` access codes.
- `docs/api/structures/dta.md` — attribute-bit definitions.
- `docs/api/gemdos/_state.md` §2 (SFT) — handle allocation model.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically documents
  the `(name, attr)` ABI and the "returns handle open for read/write" note;
  the exact illegal-character set is EmuTOS's.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `OPNFILES = 75`, `NUMSTD = 6`, `NUMHANDLES = 81` (`bdos/fs.h:43-45`,
  `include/bdosdefs.h:61`).
- Behaviour is TOS 1-3 compatible; no folder creation (that is `Dcreate`).

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: backed by the FAT16/32 driver on Pi SD/USB storage. The
"RO-attribute but RW-handle" and "silent overwrite" behaviours are contract;
directory-grow strategy is a free NewTOS choice.

TASK/FS21 (2026-10-31): the new OFD starts dirty, as EmuTOS marks the new
DFD `O_DIRTY` (`bdos/fsopnclo.c:206`), so its `Fclose` sets `FA_ARCHIVE` in
the entry even when nothing was written (`3e-fclose.md` edge case 8;
`tos-api/src/bdos/fs/file.rs:341@fcreate`).
