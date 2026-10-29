# `Fsfirst` (GEMDOS 0x4E)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsdir.c:523` (`xsfirst`) and the work helper
> `bdos/fsdir.c:452` (`ixsfirst`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the search-attribute matching rule, the 44-byte DTA
> filled at `run->p_xdta` (with **big-endian** time/date/size on the m68k ABI),
> the wildcard return convention, error codes. Reference: the DTA private-area
> encoding, `findit`/`scan`/`match`/`makbuf`.

## EmuTOS source

**Primary location:** `bdos/fsdir.c:523` — `long xsfirst(char *name, int att)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x4E` =
`xsfirst`, `wparms=3` (`bdos/bdosmain.c:276`).

**Helpers called:**
- `bdos/fsdir.c:452` — `ixsfirst()` — resolve, scan, fill the DTA private area.
- `bdos/fsdir.c:505` — `contains_wildcard_characters()`.
- `bdos/fsdir.c:1407` — `findit()`, `bdos/fsdir.c:1525` — `scan()` (which uses
  `bdos/fsdir.c:1802` `match()`), `bdos/fsdir.c:1851` — `makbuf()` (fill the
  public DTA fields, **byte-swapping** time/date/size to native order).

**Headers / type definitions used:**
- `bdos/fs.h:319-337` — `DTAINFO` (the EmuTOS-private view of the DTA).
- `bdos/fs.h:339` — `DTA_DRIVEMASK = 0x1F`.
- `include/bdosdefs.h:47-55` — public `DTA` (44 bytes).
- `include/bdosdefs.h:36-41` — `FA_*` bits.

**System variables / globals read:** `run->p_xdta` (the process DTA pointer).
**System variables / globals written:** the 44-byte DTA at `run->p_xdta`
(private search state + public fields).

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Fsfirst` finds the first directory entry matching a filename pattern and fills
the process's current DTA (at `run->p_xdta`, set by `Fsetdta`) with the match.
`xsfirst` first stamps the DTA's private `dt_offset_drive` field with `-1L` to
mark it uninitialised (so a later `Fsnext` on a failed search correctly reports
"no more"), then calls `ixsfirst`.

`ixsfirst` normalises the attribute mask: unless the caller asked for `FA_VOL`
exactly, it OR-s in `FA_ARCHIVE | FA_RO` — so a normal search **always** returns
read-only and archive files, and hidden/system/sub-directory entries appear only
if their bit was set in `att`. Volume labels are exclusive: `att == FA_VOL`
matches only labels. This is the `match()` contract (`bdos/fsdir.c:1840-1844`):
volume-label and sub-directory entries "must be specifically asked for", other
attribute bits widen the match.

It resolves the path with `findit` (failure → `EPTHNF`) and scans from the start
of the directory (`scan`; no match → `EFILNF`). On a match it records the search
state into the DTA's private area — the drive number (low 5 bits of
`dt_offset_drive`), the resume position (root: byte offset in
`dt_offset_drive`; sub-dir: `dt_cloffset`/`dt_clnum`), and a copy of the search
pattern and attribute — then `makbuf` fills the public fields.

Finally, `xsfirst` applies the wildcard convention: if the search failed it
returns the error; if the name had **no** wildcard it returns the raw result
(0 or error); if it **had** a wildcard it forces `E_OK` on success so the
paired `Fsnext` can continue the enumeration.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x4E
- **Stack frame on entry (m68k):** `Fsfirst(filename, attr)` =
  `trap1(0x4e, filename, attr)` (`include/bdosbind.h:51`):
  ```
  +6: attr     (word)
  +2: filename (LONG, pointer)
   0: function_id (word) = 0x004E
  ```
- **Total parameter bytes:** 6 (LONG + WORD) → `wparms = 3`.
- **Native (AArch64) ABI:** ID `0x4E` in `x8`; args in `x0..x1`; LONG result
  (0 or negative error) sign-extended into `x0`. The match is delivered via the
  **DTA output buffer** at `run->p_xdta`, not the register.

  | m68k stack | AArch64 reg | Name   | Type / size |
  |------------|-------------|--------|-------------|
  | +2         | `x0`        | `name` | `char *` (ptr) |
  | +6         | `x1`        | `att`  | `WORD` (16) — search-attribute mask |

---

## Inputs

| Position | Name   | Type     | Size | Source location    | Notes |
|----------|--------|----------|------|--------------------|-------|
| arg0     | `name` | `char *` | 4    | `bdos/fsdir.c:523` | path; wildcards `*`/`?` allowed in the final component only |
| arg1     | `att`  | `WORD`   | 2    | `bdos/fsdir.c:452` | search mask; `FA_ARCHIVE|FA_RO` are force-added unless `att == FA_VOL` |

### `att` matching semantics (`match()`, `bdos/fsdir.c:1802-1844`)

| Requested bit | Effect on the search |
|---------------|----------------------|
| `FA_RO` (0x01)      | always matched (force-added) |
| `FA_HIDDEN` (0x02)  | hidden files matched only if set |
| `FA_SYSTEM` (0x04)  | system files matched only if set |
| `FA_VOL` (0x08)     | **exclusive**: `att == FA_VOL` matches labels only |
| `FA_SUBDIR` (0x10)  | sub-directories matched only if set |
| `FA_ARCHIVE` (0x20) | always matched (force-added) |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `E_OK` (0) on a match; negative error on failure.
- **Source location:** `bdos/fsdir.c:535-538`.

### DTA output buffer (`run->p_xdta`, 44 bytes)

`makbuf` (`bdos/fsdir.c:1851`) fills the public portion; time/date/size are
**byte-swapped to native (m68k big-endian) order** via `swpw`/`swpl`:

| Offset | Field       | Type   | Contents |
|-------:|-------------|--------|----------|
| 0x00-0x14 | `d_reserved[21]` | private | drive id, resume position, pattern copy, search attr |
| 0x15   | `d_attrib`  | UBYTE  | attribute byte of the found entry |
| 0x16   | `d_time`    | UWORD  | packed FAT time (native byte order) |
| 0x18   | `d_date`    | UWORD  | packed FAT date (native byte order) |
| 0x1A   | `d_length`  | LONG   | file size in bytes (native byte order) |
| 0x1E   | `d_fname[14]`| char[]| dotted 8.3 name, NUL-terminated, upper-case; only name + NUL written |

### Side effects on global state

- The DTA at `run->p_xdta`: `dt_offset_drive = -1` first (`bdos/fsdir.c:530-531`);
  on a match the 21 private bytes (`bdos/fsdir.c:473-490`), 0x15..0x1D and
  `d_fname` up to and including its NUL (`makbuf`, `bdos/fsdir.c:1851-1862`).
  The `d_fname` bytes after the NUL are **not** written (edge case 6).

### Side effects on hardware

- Directory-sector reads via the buffer cache.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Path (directory portion) not found | `bdos/fsdir.c:463-464` | `EPTHNF` (-34) |
| No entry matches the pattern + mask | `bdos/fsdir.c:470-471` | `EFILNF` (-33) |

**Error paths not in Compendium:** the force-added `FA_ARCHIVE|FA_RO` mask
means "search always returns RO/archive files even if you didn't ask"; and the
wildcard-return convention (`E_OK` forced only when the *name* contains a
wildcard) is EmuTOS-specific glue.

---

## Verification criteria

- **Nominal exact:** `Fsetdta(&dta); Fsfirst("C:\\FILE.TXT", 0)` returns `E_OK`
  and fills `dta.d_fname == "FILE.TXT"`, with `d_length`/`d_time`/`d_date`
  matching the entry.
- **Wildcard:** `Fsfirst("C:\\*.TXT", 0)` returns `E_OK` on the first `.TXT`
  file and leaves the private area primed so `Fsnext` continues.
- **Hidden excluded by default:** a hidden-only file is not returned unless
  `att & FA_HIDDEN`.
- **Volume label exclusive:** `Fsfirst("C:\\*.*", FA_VOL)` returns only the
  label; `att == 0` never returns it.
- **Error — no match:** `Fsfirst("C:\\NONE.XYZ", 0)` returns `EFILNF` (-33).
- **Error — bad path:** `Fsfirst("C:\\NODIR\\*.*", 0)` returns `EPTHNF` (-34).

---

## Edge cases discovered in source

1. **`FA_ARCHIVE|FA_RO` are force-added.** Unless `att == FA_VOL`, the search
   always includes archive and read-only files (`bdos/fsdir.c:459-460`).

2. **Volume labels/sub-dirs must be asked for.** `match()` only returns them
   when their bit is set; a plain `att == 0` search skips both
   (`bdos/fsdir.c:1840-1844`).

3. **Private area is invalidated up front.** `xsfirst` writes `dt_offset_drive =
   -1L` before searching (`bdos/fsdir.c:531`), so a failed `Fsfirst` makes the
   next `Fsnext` return `ENMFIL` rather than walk stale state.

4. **Wildcard-return convention.** With a wildcard in the *name*, success is
   forced to `E_OK`; without one, the raw result is returned. Either way the
   match (if any) is in the DTA (`bdos/fsdir.c:535-538`).

5. **Time/date/size are native byte order in the DTA.** `makbuf` `swpw`/`swpl`
   from the little-endian on-disk FCB to native (big-endian on m68k)
   (`bdos/fsdir.c:1851-1862`). NewTOS must reproduce the m68k-visible big-endian
   values for compatibility.

6. **Filename is dotted 8.3, upper-case, NUL-terminated.** `packit` produces
   `NAME.EXT\0` (no dot if no extension); `d_fname` is 14 bytes
   (`"12345678.123\0"` fits) (`bdos/fsdir.c:1861`). `packit` writes the name
   and one NUL and nothing else (`bdos/fsdir.c:1297-1324`): the bytes after
   the NUL keep their previous content (a longer earlier name's tail, or the
   caller's bytes). Measured on the oracle, TOOL20 finding F1; NewTOS since
   FS23 (`docs/api/structures/dta.md`, `d_fname`).

7. **Wildcards only in the final component.** `findit` does not match wildcards
   in intermediate path components; `"C:\\*\\F"` fails at path resolution
   (`EPTHNF`).

---

## Dependencies

### Requires (must be initialized / functional)

- `run->p_xdta` set (defaults to the basepage command line — see
  `docs/api/gemdos/dta/1a-fsetdta.md`).
- FAT subsystem for the drive; buffer cache.

### Used by

- Every file manager, the AES desktop, shells; always paired with `Fsnext`.

---

## Related data structures

- `docs/api/structures/dta.md` — the 44-byte DTA and its private/public split.
- `docs/api/gemdos/dta/1a-fsetdta.md`, `2f-fgetdta.md` — the DTA pointer.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the DTA layout
  and search-attribute mask; the force-added `RO|ARCHIVE` behaviour matches
  DOS/TOS convention.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None; DTA layout and matching are build-independent.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: identical 44-byte DTA layout with native (big-endian) time/date/size
for m68k ABI compatibility. The private-area encoding is a free NewTOS choice as
long as `Fsnext` resumes correctly and programs treat bytes 0x00-0x14 as opaque.
