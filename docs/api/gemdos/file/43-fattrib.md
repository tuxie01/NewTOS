# `Fattrib` (GEMDOS 0x43)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsdir.c:402` (`xchmod`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, get-vs-set semantics, the settable-bit mask
> (`FA_NORM`), the returned attribute byte, error codes. Reference: the in-place
> directory-entry rewrite via `ixlseek`/`ixwrite`.
>
> **Deliberate divergence:** NewTOS is a TOS4/FreeMiNT superset — GET works on
> folders (returns the folder attribute with `FA_SUBDIR` set) where EmuTOS/TOS
> 1-3 return `EFILNF`; SET on a folder stays rejected (`EACCDN`). See
> [NewTOS divergence](#source-extraction-summary) below.

## EmuTOS source

**Primary location:** `bdos/fsdir.c:402` — `long xchmod(char *p, int wrt, UBYTE mod)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x43` =
`xchmod`, `wparms=4` (`bdos/bdosmain.c:263`).

**Helpers called:**
- `bdos/fsdir.c:1407` — `findit()`, `bdos/fsdir.c:1525` — `scan()`.
- `bdos/fsio.c:354` — `ixlseek()`, `bdos/fsio.c:498` — `ixread()`,
  `bdos/fsio.c:554` — `ixwrite()`, `bdos/fsopnclo.c:511` — `ixclose()`.

**Headers / type definitions used:**
- `bdos/fs.h:195` — `FA_NORM` (`= FA_ARCHIVE|FA_SYSTEM|FA_HIDDEN|FA_RO = 0x27`).
- `include/bdosdefs.h:44-45` — `F_GETMOD (0)`, `F_SETMOD (1)` — values of `wrt`.
- `bdos/fs.h:183-191` — `FCB` layout (attribute at byte offset 11).

**System variables / globals read:** `run` (path context); the directory OFD
`dn->d_ofd`.
**System variables / globals written:** on set — the attribute byte in the
directory sector (via the buffer cache), flushed by `ixclose(CL_DIR)`.

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Fattrib` gets or sets the attribute byte of an existing file's directory
entry. `xchmod` resolves the path with `findit` (failure → `EPTHNF`) and scans
with the `FA_NORM` mask (`bdos/fsdir.c:415`). Because that mask excludes
`FA_VOL`/`FA_SUBDIR`, EmuTOS `Fattrib` operates only on **normal files**: a
folder or label returns `EFILNF`. A source comment (`bdos/fsdir.c:391-401`)
documents that EmuTOS matches Atari TOS 1-3 here — TOS 4 extended `Fattrib` to
folders, which EmuTOS deliberately does not.

> **NewTOS divergence (TOS4/FreeMiNT superset).** NewTOS adopts the TOS 4 lift
> of the folder restriction for the **GET** path only: `Fattrib(path, 0, 0)` on
> a directory returns the directory entry's attribute byte (with `FA_SUBDIR` =
> `0x10` set) instead of `EFILNF`. This is what FreeMiNT's `sys_f_attrib` does —
> it resolves any object and `return xattr.attr;` for both files and folders
> (`references/freemint/sys/dosdir.c:998-1002`); the TOS4 rule set is quoted in
> EmuTOS `bdos/fsdir.c:391-400`. The change unblocks cflib `path_exists`
> (`sdk/cflib/cflib-21/fsexists.c:41`, `Fattrib(path,0,0) >= 0 && (attr &
> FA_DIR)`), i.e. QED's clipboard init and every FreeMiNT/cflib app.
>
> The same GET path also accepts the **trailing-separator spelling**
> `Fattrib("C:\SUBDIR\", 0, 0)`, because that is the only spelling cflib ever
> passes: `make_normalpath` appends a separator to every path before it probes
> (`sdk/cflib/cflib-21/fsnorm.c:85-92`). FreeMiNT answers it with the folder's
> attributes — `relpath2cookie` peels an empty final component
> (`sys/filesys.c:986-1010`) and `fatfs_lookup` resolves the empty name **as
> `"."`** (`sys/fatfs.c:5753-5764`). The lift is strictly additive: only a path
> that names an existing **directory** after the separators are dropped answers
> differently than EmuTOS. A trailing separator on a **file** keeps `EPTHNF`
> (EmuTOS' answer, and FreeMiNT's ENOTDIR is the same `-34`,
> `sys/mint/errno.h:277`), a trailing separator on a **missing** name keeps
> `EPTHNF`, and the bare **drive root** `"C:\"` keeps `EFILNF` — there is no
> directory entry there whose attribute byte could be read, and synthesizing
> `FA_DIR` as FreeMiNT does is a separate decision that has **not** been taken.
>
> The **SET** path stays *stricter* than FreeMiNT: NewTOS rejects any set on a
> folder or volume label with `EACCDN`, so a directory's attribute byte can
> never be rewritten (the #38 corruption guard). FreeMiNT only forbids toggling
> the `FA_DIR` bit and setting a label (`sys/dosdir.c:976-990`); NewTOS forbids
> the whole set. File-set semantics are unchanged (`FA_VOL`/`FA_SUBDIR` in `mod`
> → `EACCDN`).

For a **get** (`wrt == 0`) it seeks to the attribute byte and reads one byte
into `mod`, then returns it. For a **set** (`wrt == 1`) it first rejects any bit
outside `FA_NORM` (`mod & ~FA_NORM` non-zero → `EACCDN`) — so you may set only
read-only/hidden/system/archive; you may **not** set the volume-label or
sub-directory bit via `Fattrib`. It writes the new byte in place and flushes
with `ixclose(CL_DIR)`.

Either way the return value is the attribute byte: the *current* attributes on a
get, or the just-written `mod` on a set. On the native ABI it is a `UBYTE`
sign-extended into `x0` — if bit 7 is ever set (unusual), it sign-extends;
callers should mask `& 0xFF`.

The seek offset `pos -= 21` (`bdos/fsdir.c:424`) is the attribute-byte position:
`scan` leaves `pos` one whole FCB (`sizeof(FCB)=32`) past the entry, and the
attribute is at FCB offset 11, so `32 - 11 = 21`.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x43
- **Stack frame on entry (m68k):** `Fattrib(filename, wflag, attrib)` =
  `trap1(0x43, filename, wflag, attrib)` (`include/bdosbind.h:44`):
  ```
  +8: attrib   (word)
  +6: wflag    (word)
  +2: filename (LONG, pointer)
   0: function_id (word) = 0x0043
  ```
- **Total parameter bytes:** 8 (LONG + WORD + WORD) → `wparms = 4`.
- **Native (AArch64) ABI:** ID `0x43` in `x8`; args in `x0..x2`; LONG result
  (attribute byte, sign-extended, or negative error) in `x0`.

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `p`   | `char *` (ptr) |
  | +6         | `x1`        | `wrt` | `WORD` (16) — 0 get, 1 set |
  | +8         | `x2`        | `mod` | `WORD` (16) — new attr (set only) |

---

## Inputs

| Position | Name  | Type     | Size | Source location    | Notes |
|----------|-------|----------|------|--------------------|-------|
| arg0     | `p`   | `char *` | 4    | `bdos/fsdir.c:409` | path to an existing file |
| arg1     | `wrt` | `int`    | 2    | `bdos/fsdir.c:427` | 0 = get, 1 = set |
| arg2     | `mod` | `UBYTE`  | 2    | `bdos/fsdir.c:402` | new attribute (only used when `wrt == 1`) |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (attribute byte, or negative error)
- **Encoding:** on get, the file's current attribute byte; on set, the written
  `mod`. Sign-extended from `UBYTE`.
- **Source location:** `bdos/fsdir.c:435`.

### Side effects on global state

- On set only: rewrites the attribute byte in the directory sector and flushes
  (`bdos/fsdir.c:431-432`). Get has no side effects on the entry.

### Side effects on hardware

- Directory-sector read (get and set) and write (set) via the block device.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Path (directory portion) not found | `bdos/fsdir.c:410-411` | `EPTHNF` (-34) |
| Name not found (neither a file nor a folder) | `bdos/fsdir.c:415-416` | `EFILNF` (-33) |
| **EmuTOS only:** name is a folder/label | `bdos/fsdir.c:415-416` | `EFILNF` (-33) |
| **NewTOS:** GET on a folder, with or without a trailing separator | (extension) | attribute byte, `FA_SUBDIR` (0x10) set, `>= 0` |
| **NewTOS:** GET on a *file* with a trailing separator (`"C:\F.TXT\"`) | `path::resolve_parent` (not a directory) | `EPTHNF` (-34) |
| **NewTOS:** GET on the bare drive root (`"C:\"`) | no entry to read | `EFILNF` (-33) |
| **NewTOS:** SET on a folder/label | (corruption guard) | `EACCDN` (-36) |
| Set requested with a bit outside `FA_NORM` (`mod & ~0x27`) | `bdos/fsdir.c:421-422` | `EACCDN` (-36) |

**Error paths not in Compendium:** the exact settable mask is `FA_NORM = 0x27`
(RO|HIDDEN|SYSTEM|ARCHIVE); attempting to set `FA_VOL` (0x08) or `FA_SUBDIR`
(0x10) yields `EACCDN`.

---

## Verification criteria

- **Get:** `Fattrib("C:\\F.TXT", 0, 0)` returns the file's attribute byte.
- **Set RO:** `Fattrib("C:\\F.TXT", 1, FA_RO)` returns `FA_RO` (1) and makes a
  later `Fopen(..., 2)` fail with `EACCDN`.
- **Set invalid bit:** `Fattrib("C:\\F.TXT", 1, FA_SUBDIR)` returns `EACCDN`.
- **Folder GET (NewTOS/TOS4):** `Fattrib("C:\\SUBDIR", 0, 0)` returns the folder's
  attribute byte with `FA_SUBDIR` (0x10) set — a value `>= 0`, **not** `EFILNF`.
  (EmuTOS/TOS 1-3 return `EFILNF` here.)
- **Folder GET, trailing-separator spelling (NewTOS/FreeMiNT):**
  `Fattrib("C:\\SUBDIR\\", 0, 0)` returns the **same** value as the spelling
  without the separator. This is the spelling cflib passes
  (`sdk/cflib/cflib-21/fsnorm.c:85-92`); measured on QEMU `raspi4b`, `16` for a plain folder.
- **Trailing separator elsewhere is unchanged:**
  `Fattrib("C:\\F.TXT\\", 0, 0)` = `EPTHNF` (-34),
  `Fattrib("C:\\NOSUCH\\", 0, 0)` = `EPTHNF` (-34),
  `Fattrib("C:\\", 0, 0)` = `EFILNF` (-33) — the drive root is deliberately
  **not** lifted.
- **Folder SET (NewTOS):** `Fattrib("C:\\SUBDIR", 1, 0)` returns `EACCDN` (-36) —
  a folder's attribute byte is never mutated (corruption guard).
- **Bad path:** `Fattrib("C:\\NODIR\\F", 0, 0)` returns `EPTHNF` (-34).

---

## Edge cases discovered in source

1. **Files only in EmuTOS (TOS 1-3); folders on GET in NewTOS (TOS4).** The
   `FA_NORM` scan mask excludes folders/labels → `EFILNF` in EmuTOS
   (`bdos/fsdir.c:391-401,415`). NewTOS implements the TOS 4 folder extension for
   the GET path (`Fattrib(dir, 0, 0)` returns the folder attribute, `FA_SUBDIR`
   set), matching FreeMiNT `sys_f_attrib` (`sys/dosdir.c:998-1002`); the SET path
   still rejects folders/labels with `EACCDN`.

   The GET extension covers the **trailing-separator** spelling as well — that
   is the only one cflib ever produces (`sdk/cflib/cflib-21/fsnorm.c:85-92`) — and nothing else: an
   empty final component still answers the way EmuTOS answers it for every other
   caller (`Fsfirst`/`Fopen` → `EFILNF` through `scan(builds(""))`,
   `bdos/fsdir.c:737-795`; `Dcreate` → `EPTHNF` through `ixcreat`'s own guard,
   `bdos/fsopnclo.c:117-118`), and inside `Fattrib` GET it still answers that way
   for a file, a missing name and the bare drive root.

2. **Settable bits limited to `FA_NORM`.** `mod & ~FA_NORM` on a set → `EACCDN`;
   `FA_VOL`/`FA_SUBDIR` cannot be toggled here (`bdos/fsdir.c:421-422`).

3. **Return is sign-extended `UBYTE`.** Callers should mask `& 0xFF`
   (`bdos/fsdir.c:435`).

4. **Direct in-place directory rewrite.** The attribute byte at FCB offset 11 is
   overwritten (`pos - 21`) and flushed via `ixclose(CL_DIR)`
   (`bdos/fsdir.c:424-432`).

5. **No effect on open handles.** `Fattrib` changes the directory entry, not any
   open OFD's recorded mode; a file currently open RW keeps its access. The new
   flag applies to the *next* `Fopen`.

6. **Idempotent set still flushes.** Setting the same value re-writes and
   flushes the directory sector; tight loops thrash the buffer cache.

7. **No wildcard iteration.** `Fattrib` acts on one resolved name; callers
   enumerate with `Fsfirst`/`Fsnext` and apply `Fattrib` per file.

---

## Dependencies

### Requires (must be initialized / functional)

- FAT subsystem for the drive; the directory OFD; buffer cache + `Rwabs`.

### Used by

- File managers, archivers, installers (clearing `FA_RO` before delete/rename).

---

## Related data structures

- `docs/api/structures/ofd.md` — FCB attribute byte.
- `docs/api/structures/dta.md` — `FA_*` bit definitions.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents get/set and the
  attribute bits; the TOS 1-3 files-only restriction matches EmuTOS.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred. (Notes TOS 4 folder
  support, which EmuTOS omits.)

---

## Configuration / version notes

- EmuTOS behaves as Atari TOS 1-3 (files only). `FA_NORM = 0x27`.
- NewTOS is a TOS4/FreeMiNT superset: GET works on folders too (returns the
  folder attribute with `FA_SUBDIR` set), in both the plain and the
  trailing-separator spelling; SET on a folder stays rejected.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: keep the TOS 1-3 files-only behaviour so programs that rely on
`EFILNF` for folders are not surprised.

2026-07-21 (TOS4/FreeMiNT dir-GET extension): `tos-api/src/bdos/fs/file.rs`
`fattrib`. GET (`wrt == 0`) now resolves files **and** directories via the
subdir-aware `dir::find_in_dir` (instead of `dir::find_file_in_dir`), so
`Fattrib(dir, 0, 0)` returns the directory entry's attribute byte with
`FA_SUBDIR` (0x10) set. SET (`wrt == 1`) resolves with `find_in_dir` too and
rejects any folder/label with `EACCDN` before the settable-bit check, so a
directory entry is never rewritten (the #38 corruption guard). File GET/SET,
`EPTHNF`/`EFILNF` and the `mod & 0x18` reject are byte-for-byte unchanged.
`Fopen` (`ixopen`) keeps `find_file_in_dir` and still excludes directories.
Rationale: cflib `path_exists` (`sdk/cflib/cflib-21/fsexists.c:41`) probes a
folder with `Fattrib(path, 0, 0) >= 0 && (attr & FA_DIR)`; without this QED's
`init_clipbrd` fell through to `Dcreate("C:\\CLIPBRD")` → `EACCDN` and raised the
NOSCRAP alert. Reference: FreeMiNT `sys_f_attrib` (`references/freemint/sys/
dosdir.c:946-1002`), EmuTOS `xchmod` + TOS4 note (`bdos/fsdir.c:382-436`,
`391-400`). Selftest: `kernel/src/selftest_emmc.rs` `fmeta_test` (dir-GET
assertions a-d) — since TASK/T06 hard `assert!`s that name the case, wired
into the boot behind the kernel feature `selftest-emmc` (`cargo xtask build
--board rpi4 --features selftest-emmc`, QEMU raspi4b; a failure is a `[PANIC]`
line). It is not part of a normal boot, which has no `selftest_*` call.

2026-09-12 (Q07 — the trailing-separator spelling): the 2026-07-21 entry above
claimed that `"C:\\CLIPBRD"` and `"C:\\CLIPBRD\\"` "resolve to the same
directory (`path::findit` drops the empty trailing component)", and the
verification criteria repeated it. **That was wrong.** `path::findit` does not
drop the empty trailing component — it hands it to `dir::fcb_name`, which turns
it into eleven blanks no directory entry can carry, so `find_in_dir` failed and
`Fattrib("C:\\CLIPBRD\\", 0, 0)` answered `EFILNF` (measured: `-33`). Since
cflib appends a separator to *every* path before it probes
(`sdk/cflib/cflib-21/fsnorm.c:85-92`), the one caller the 2026-07-21 extension
was made for never benefited from it: QED's clipboard init still fell through to
`Dcreate` → `EACCDN` → NOSCRAP alert on every run after the first, and cflib's
`app_init` (`sdk/cflib/cflib-21/appinit.c:104-107`) discarded `gl_appdir` and fell back to the
current directory. Fix in `tos-api/src/bdos/fs/file.rs`: the GET branch first
probes the separator-stripped spelling (`without_trailing_seps` +
`subdir_attrib`) and accepts it **only if it names a directory**; everything
else falls through to the unchanged code, so the lift is purely additive.
Measured on QEMU `raspi4b` with an EL0 probe over all 18 cases of the Q07
diagnosis plus the two `app_init` paths: exactly one value moved,
`Fattrib("C:\\CLIPBRD\\",0,0)` from `-33` to `16`. `Fattrib("C:\\",0,0)` stays
`-33`; `Fsfirst`/`Fopen`/`Dcreate` on a path ending in a separator are
untouched. Host tests: `tos-api/tests/gemdos_fattrib_dir.rs`.
