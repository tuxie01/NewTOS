# `Dsetpath` (GEMDOS 0x3B)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsdir.c:1113` (`xchdir`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the per-drive current-directory semantics (drive prefix
> sets *that drive's* curdir, not the current drive), the error codes. Reference:
> the `dirtbl[]` reference-counted DND cache, `findit`/`incr_curdir_usage`.

## EmuTOS source

**Primary location:** `bdos/fsdir.c:1113` — `long xchdir(char *p)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x3B` =
`xchdir`, `wparms = 2` (`bdos/bdosmain.c:255`).

**Helpers called:**
- `bdos/fsdir.c:505` — `contains_wildcard_characters(p)` — reject `*`/`?`.
- `util/miscutil.c:34` — `extract_drive_number(p)` — parse a leading `X:` prefix
  (returns 0..25, or -1 if absent).
- `bdos/fsdir.c:1407` — `findit(p, &s, 1)` — resolve the path; **mode 1** = must
  be a directory (returns its DND, else NULL).
- `bdos/fsdir.c:1164` — `incr_curdir_usage(dnd)` — find/allocate a `dirtbl[]`
  slot for the DND and bump its use count (returns the slot index, or `EINTRN`
  if `dirtbl[]` is full).
- `bdos/fsdir.c:1194` — `decr_curdir_usage(olddir)` — drop the old slot's use
  count.

**Headers / type definitions used:**
- `bdos/fs.h:342-345` — `DIRTBL_ENTRY { DND *dnd; WORD use; }`.
- `bdos/fs.h:44` — `NCURDIR = 40` (size of `dirtbl[]`).
- `include/bdosdefs.h:89` — `PD.p_curdir[NUMCURDIR]` (`UBYTE[26]`, basepage
  offset **0x40**) — per-drive `dirtbl[]` slot index.
- `include/sysconf.h:43,62` — `BLKDEVNUM = 26`, `NUMCURDIR = BLKDEVNUM`.

**System variables / globals read:** `run->p_curdrv` (when the path has no drive
prefix); `run->p_curdir[dlog]` (old slot); `dirtbl[]`.
**System variables / globals written:** `run->p_curdir[dlog]` (new slot index);
`dirtbl[].use` counts.

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Dsetpath` sets the calling process's **current directory** for a drive.
`xchdir` first rejects any wildcard in the path (`*`/`?` → `EPTHNF`). It then
extracts the target drive from a leading `X:` prefix via `extract_drive_number`
(0..25); if there is no prefix it uses the current drive (`run->p_curdrv`). The
resulting `dlog` selects **which drive's** per-process current-directory slot is
updated. Crucially, a drive prefix changes only *that drive's* curdir — it does
**not** change the current drive itself; to change both, call `Dsetdrv` then
`Dsetpath`.

It resolves the path with `findit(p, &s, 1)` (mode 1 = must be a directory);
failure returns `EPTHNF`. (Because `findit`→`dcrack`→`ckdrv(TRUE)` runs, an
invalid or unmounted drive prefix is caught here and also surfaces as `EPTHNF`.
`ckdrv` asks `Mediach` only for drives in `drvrem` (removable units,
`bdos/fsdrive.c:106-114`, `bios/disk.c:346-347`); for a fixed disk a pending
media change is met only when `findit` has to read a directory
(`dirscan`→`scan`→`getrec`→`getbcb`, `bdos/fsdir.c:1462,1482,1550`,
`bdos/fsbuf.c:177-188`). A path whose every element is already a cached DND
child (`bdos/fsdir.c:1455-1473`) touches no disk: `Dsetpath` then succeeds
without meeting the change, `Mediach` stays 2 — measured, `gemdos-mc-paths`
case S04, TASK/FS24. Otherwise `findit` reads the directory and meets the
change, the call restarts on the logged-in drive and `Mediach` reads 0: right
after a re-login (no child logged), for a name that is not a logged child
(`EPTHNF` for a name that does not exist), and for a folder an earlier `scan`
did not reach — `scan` logs the folders it reads up to and including the
entry it stops at (`bdos/fsdir.c:1557-1574`), so an `Fsfirst` for a file that
stands before the folder logs none. A logged folder stays logged across
buffer traffic: after 9316 bytes written elsewhere, `Dsetpath` still does not
meet the change — measured, `gemdos-mc-fs24` cases 85-86, 101-102, 116-117,
148-149, TASK/FS24.) On success it registers the resolved DND in the global
`dirtbl[]` via `incr_curdir_usage`, which either finds an existing slot pointing
at the same DND (and bumps its use count) or allocates a free slot. If
`dirtbl[]` is full, `incr_curdir_usage` returns `EINTRN` and `xchdir` translates
it to `EPTHNF`. It stores the new slot index into `run->p_curdir[dlog]`, then
decrements the old slot (if any). The current directory is thus stored
per-process, per-drive, as an index into the shared reference-counted `dirtbl[]`.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x3B
- **Stack frame on entry (m68k):** `Dsetpath(path)` = `trap1(0x3b, path)`:
  ```
  +2: path (LONG, pointer)
   0: function_id (word) = 0x003B
  ```
- **Total parameter bytes:** 4 (LONG) → `wparms = 2`.
- **Native (AArch64) ABI:** ID `0x3B` in `x8`; arg in `x0`; LONG result
  (`E_OK` or negative error) sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | +2         | `x0`        | `p`  | `char *` (ptr) — path to a directory (absolute or relative) |

---

## Inputs

| Position | Name | Type    | Size | Source location     | Notes |
|----------|------|---------|------|---------------------|-------|
| arg0     | `p`  | `char *`| 4    | `bdos/fsdir.c:1113` | directory path; optional `X:` prefix selects the drive whose curdir changes; wildcards rejected |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `E_OK` (0) on success; `EPTHNF` (-34) on any failure.
- **Source location:** `bdos/fsdir.c:1153` (`return E_OK`); error returns at
  `:1120`, `:1136`, `:1144`.

### Side effects on global state

- `run->p_curdir[dlog]` is set to the new `dirtbl[]` slot index
  (`bdos/fsdir.c:1145`).
- `dirtbl[]` use counts change: the new slot's count is incremented (or a slot
  allocated), the old slot's count decremented (`bdos/fsdir.c:1142,1150-1151`).

### Side effects on hardware

- Reads directory sectors during `findit` path resolution only for path
  elements not yet cached as DND children (`bdos/fsdir.c:1455-1485`); a media
  change is met there (fixed disk) or in `ckdrv` (removable drive,
  `bdos/fsdrive.c:106-114`) and triggers the re-login and a restart of the call
  (`bdos/bdosmain.c:449-487`).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Path contains a wildcard (`*` or `?`) | `bdos/fsdir.c:1119-1120` | `EPTHNF` (-34) |
| Path does not resolve to a directory (`findit` mode 1 fails; includes invalid/unmounted drive prefix) | `bdos/fsdir.c:1135-1136` | `EPTHNF` (-34) |
| `dirtbl[]` full (`incr_curdir_usage` returns `EINTRN`) | `bdos/fsdir.c:1143-1144` | `EPTHNF` (-34) |

**Error paths not in Compendium:** the `dirtbl[]`-full case is internally
`EINTRN` but is surfaced to the caller as `EPTHNF`; a path resolving to a *file*
returns `EPTHNF` (not `EFILNF`). Compendium documents only `EPTHNF` generically.

---

## Verification criteria

- **Nominal:** with `C:\AUTO` existing, `Dsetpath("C:\\AUTO")` returns `E_OK` and
  a following `Dgetpath(buf, 3)` yields `\AUTO`.
- **Relative:** after `Dsetpath("C:\\AUTO")`, `Dsetpath("..")` (no prefix,
  current drive C:) moves the C: curdir back to the root; `Dgetpath(buf, 3)`
  yields `""`.
- **Prefix does not change current drive:** with current drive A:,
  `Dsetpath("C:\\AUTO")` sets C:'s curdir but `Dgetdrv()` still returns 0 (A:).
- **Error — wildcard:** `Dsetpath("C:\\*.X")` returns `EPTHNF` (-34).
- **Error — not a folder:** `Dsetpath("C:\\FILE.TXT")` (a file) returns `EPTHNF`
  (-34).
- **Error — missing:** `Dsetpath("C:\\NODIR")` returns `EPTHNF` (-34).

---

## Edge cases discovered in source

1. **Wildcards rejected up front.** `*`/`?` → `EPTHNF` before any resolution.
   (`bdos/fsdir.c:1119`)

2. **Drive prefix changes that drive's curdir, not the current drive.**
   `Dsetpath("C:\\foo")` sets `p_curdir[2]` (C:) but leaves `p_curdrv`
   unchanged. Use `Dsetdrv` + `Dsetpath` to change both. (`bdos/fsdir.c:1122-1145`)

3. **Per-process, per-drive current directory.** `p_curdir[NUMCURDIR]` has one
   slot per drive A:..Z: (`NUMCURDIR = BLKDEVNUM = 26`), each an index into the
   global `dirtbl[]`. Each process has its own `p_curdir[]`.
   (`include/bdosdefs.h:89`) (Earlier drafts said `p_curdir[16]` — wrong; it is
   26. See D-025.)

4. **`dirtbl[]` is reference-counted and shared.** `NCURDIR = 40` slots; slot 0
   is reserved (`p_curdir[i] == 0` means "not set yet, use root"). Multiple
   processes/drives pointing at the same DND share one slot via the use count.
   Overflow → `EINTRN` internally → `EPTHNF` to the caller.
   (`bdos/fsdrive.c:47-54`, `bdos/fsdir.c:1164-1188`)

5. **Path resolving to a file → `EPTHNF`.** `findit` mode 1 only matches
   directories; a file name gives no DND. (`bdos/fsdir.c:1134`)

6. **`ckdrv(TRUE)` inside `findit`.** Resolving the path logs in the drive and
   checks removable media; an invalid/unmounted drive prefix fails there and
   collapses to `EPTHNF`. (`bdos/fsdir.c:1715`)

7. **Inherited by `Pexec` children.** A child copies the parent's `p_curdir[]`
   at creation; later `Dsetpath` calls affect only the caller.

8. **Single-threaded ordering.** The "incr new, then decr old" order avoids
   freeing the DND that is being re-selected. EmuTOS is single-threaded, so no
   locking; NewTOS multitasking will need it around `dirtbl[]`.

---

## Dependencies

### Requires (must be initialized / functional)

- FAT subsystem logged in for the target drive (`findit`/`ckdrv`).
- The global `dirtbl[]` and the process's `p_curdir[]`.

### Used by

- Shell `cd` command, AES desktop, every program that changes its working
  directory before relative-path file operations.

---

## Related data structures

- `docs/api/gemdos/_state.md` §5 — the per-process current-path model
  (`PD.p_curdir[]`) and the reference-counted `dirtbl[]`.
- `docs/api/gemdos/directory/47-dgetpath.md` — reads back what this sets.
- `docs/api/gemdos/drive/0e-dsetdrv.md` — changes the *current drive*
  (orthogonal to the per-drive curdir).

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents `Dsetpath` and
  `EPTHNF`; the "prefix sets only that drive's curdir" nuance is TOS behaviour.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `NCURDIR = 40` (`dirtbl[]` size); `NUMCURDIR = 26` (`p_curdir[]` per drive).
  NewTOS may enlarge `dirtbl[]` (RPi has plenty of memory) as a free
  implementation choice.

---

## NewTOS implementation notes

(Empty during Phase 00.)

**2026-10-18 (TASK/PROC03/PROC04 point 2):** `run` here is the PD in front, not
necessarily the caller — an accessory's `Dsetpath` changes the front program's
directory; the AES shell changes the accdesk PD's directory before every launch
and EmuCON's `cd` changes the EmuCON PD's (`../process/_run-ownership-curdir.md`
§1.3, §4.3-4.4). Before PROC04 the directory was global, so a child's
`Dsetpath` changed its parent's (measured, `measurements/measurement-2026-10-17-proc03/`).
Built per PD (`tos-api/src/bdos/fs/curdir.rs`, `../process/_run-ownership-curdir.md`
§6.5; accessories still act on their own PD until B5 is decided).

PHASE-05: preserve the per-process/per-drive current-directory model and the
"prefix ≠ current drive" semantics. The `dirtbl[]` reference-counting is a free
NewTOS implementation choice; the observable contract is the resolved path and
the `EPTHNF`-for-everything error behaviour.
