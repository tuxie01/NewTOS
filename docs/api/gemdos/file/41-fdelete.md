# `Fdelete` (GEMDOS 0x41)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsopnclo.c:623` (`xunlink`), the static helper
> `bdos/fsopnclo.c:588` (`ixunlink`), and the work routine
> `bdos/fsopnclo.c:652` (`ixdel`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, RO/folder/other-process refusals, error codes,
> the on-disk `0xE5` erase marker. Reference: FAT-chain free, `ixdel`
> internals, buffer flush.

## EmuTOS source

**Primary location:** `bdos/fsopnclo.c:623` — `long xunlink(char *name)`
(thin wrapper over `ixunlink`).

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x41` =
`xunlink`, `wparms=2` (`bdos/bdosmain.c:261`).

**Helpers called:**
- `bdos/fsopnclo.c:588` — `ixunlink()` — resolve + scan + RO check + `ixdel`.
- `bdos/fsdir.c:1407` — `findit()`, `bdos/fsdir.c:1525` — `scan()`.
- `bdos/fsopnclo.c:652` — `ixdel()` — close-if-mine-else-refuse, free FAT
  chain, mark the directory entry erased.
- `bdos/fsopnclo.c:511` — `ixclose()`; `bdos/fsfat.c` — `getrealcl()`,
  `clfix()`, `endofchain()`.

**Headers / type definitions used:**
- `bdos/fs.h:193` — `ERASE_MARKER = 0xE5`.
- `bdos/fs.h:195` — `FA_NORM = 0x27`.
- `bdos/fs.h:483-485` — `FREECLUSTER`, `ENDOFCHAIN`, `endofchain()`.

**System variables / globals read:** `run` (for the owner check in `ixdel`);
`dn->d_files` (open OFDs on the directory), `sft[]`.
**System variables / globals written:** the FAT (clusters freed), the directory
sector (first name byte → `0xE5`), buffer cache.

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Fdelete` removes a file by name. `ixunlink` resolves the path (`findit`;
failure → `EFILNF`) and scans the directory with the `FA_NORM` mask
(`bdos/fsopnclo.c:604`). Because `FA_NORM` excludes `FA_VOL`/`FA_SUBDIR`,
`Fdelete` matches only normal files — a folder or a volume label is simply "not
found" and returns `EFILNF`. If the found entry is read-only (`FA_RO`), it
returns `EACCDN`; the caller must clear the flag with `Fattrib` first.

`ixdel` then does the real work. It walks the directory's list of open files
(`dn->d_files`) looking for an OFD at the same directory offset; for each, it
checks `sft[]`: if the file is open by the **current** process it is closed
first (`ixclose`), but if it is open by **another** process, `ixdel` returns
`EACCDN`. Next it frees the file's FAT cluster chain (`getrealcl`/`clfix` write
`FREECLUSTER` into each entry). Finally it overwrites the first byte of the
directory entry with the erase marker `0xE5` and flushes (`ixclose(fd,
CL_DIR)`). On success `E_OK`. Physical failures during these steps take the
BIOS `longjmp` path, so `ixdel` can simply return `E_OK` afterward.

Note the interface: `Fdelete` has no wildcard iteration and no error for
"already gone"; it targets one named file.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x41
- **Stack frame on entry (m68k):** `Fdelete(fname)` = `trap1(0x41, fname)`:
  ```
  +2: fname (LONG, pointer)
   0: function_id (word) = 0x0041
  ```
- **Total parameter bytes:** 4 (LONG) → `wparms = 2`.
- **Native (AArch64) ABI:** ID `0x41` in `x8`; `name` in `x0`; LONG result
  sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name   | Type / size |
  |------------|-------------|--------|-------------|
  | +2         | `x0`        | `name` | `char *` (ptr) |

---

## Inputs

| Position | Name   | Type     | Size | Source location       | Notes |
|----------|--------|----------|------|-----------------------|-------|
| arg0     | `name` | `char *` | 4    | `bdos/fsopnclo.c:588` | path of file to delete; exact name, no wildcards |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `E_OK` (0) on success; negative error otherwise.
- **Source location:** `bdos/fsopnclo.c:699` (`ixdel` success).

### Side effects on global state

- Frees the file's FAT cluster chain (`bdos/fsopnclo.c:674-683`).
- Marks the directory entry erased (`0xE5`) and flushes
  (`bdos/fsopnclo.c:688-692`).
- May `ixclose` an OFD owned by the current process before deleting (the
  handle stays valid, see edge case 3).

### Side effects on hardware

- FAT and directory-sector writes via the block device.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Path/file not found (`findit` NULL, or `scan` no match) | `bdos/fsopnclo.c:598-599`, `:604-605` | `EFILNF` (-33) |
| Entry is read-only (`FA_RO`) | `bdos/fsopnclo.c:607-608` | `EACCDN` (-36) |
| File open by **another** process | `bdos/fsopnclo.c:668` (`ixdel`) | `EACCDN` (-36) |

**Note:** because the scan uses `FA_NORM`, a **folder** or **volume label** of
that name is not matched and the result is `EFILNF`, not `EACCDN` — the old
spec's "delete on a folder returns EACCDN" is incorrect; delete folders with
`Ddelete` (0x3A).

**Error paths not in Compendium:** the "open by another process → `EACCDN`"
distinction and the same-process `ixclose` before delete (the handle is not
invalidated).

---

## Verification criteria

- **Nominal:** `Fdelete("C:\\TMP.DAT")` on an existing, non-RO, unopened file
  returns `E_OK` (0); a following `Fopen` of that name returns `EFILNF`.
- **RO refusal:** `Fdelete` on an `FA_RO` file returns `EACCDN` (-36); after
  `Fattrib(name, 1, 0)` the delete succeeds.
- **Folder not matched:** `Fdelete("C:\\SUBDIR")` returns `EFILNF` (-33).
- **Missing file:** `Fdelete("C:\\NONE")` returns `EFILNF` (-33).

---

## Edge cases discovered in source

1. **RO files are refused.** `FA_RO` → `EACCDN` (`bdos/fsopnclo.c:607-608`).

2. **Folders/labels are "not found", not "denied".** The `FA_NORM` scan mask
   excludes them, so `Fdelete` returns `EFILNF` (`bdos/fsopnclo.c:604`).

3. **Same-process open file: `ixclose`d, then deleted — the handle is NOT
   invalidated** (re-validated 2026-10-18, TASK/PROC04; the earlier wording
   "the handle becomes stale" was imprecise and led to NewTOS's F6 deviation).
   Read in source (`bdos/fsopnclo.c:652-669`): `ixdel` walks `dn->d_files`
   for OFDs with `o_dirbyt == pos`, and for every `sft[n]` whose `f_ofd` is
   that OFD checks `sft[n].f_own == run`; if yes it calls `ixclose(fd,0)` and
   **only that** (`:665-666`). `ixclose(fd,0)` (`:511-581`) (a) writes the
   dirty directory entry back (date/time, start cluster, length, archive
   bit), (b) unlinks the OFD from `dn->d_files`, (c) flushes the dirty FAT /
   data buffers. It does **not** touch `sft[]`, `f_use`, `f_own`, `f_ofd`, or
   any `p_uft[]`. Then `ixdel` frees the cluster chain and writes `0xE5`.
   Consequences (all read from the code, none from comments):
   - `Fdelete` returns `E_OK`; the caller's handle (and every `Fdup`/`Fforce`
     alias of it) stays valid — `getofd` still returns the OFD
     (`fsmain.c:474-486`).
   - `Fread`/`Fwrite`/`Fseek` on that handle keep working through the OFD's
     remembered cluster fields (`o_strtcl`, `o_curcl`, …) — i.e. on a chain the
     FAT now lists as free. EmuTOS does not protect against that; a write
     after the delete can land in clusters another file later allocates, and
     a dirty OFD's later `Fclose` rewrites the (possibly re-used) directory
     slot's cluster/length/attribute fields (`ixclose` `:530-549`, no check
     that the slot still holds the same file). This is an EmuTOS hazard,
     reported here as fact; whether NewTOS replicates it is a decision, see
     "NewTOS implementation notes".
   - `Fclose` of that handle: `ixclose(fd,0)` finds the OFD no longer in
     `d_files` (`:552-566`) and returns **`EINTRN`** (-65), but `xclose` still
     decrements `f_use` and `sftdel`s the entry (`:484-496`). Same quirk as
     the second close of a shared OFD (`_state.md` §2).
   - The loop does not stop at the first match: with several `sft[]` entries
     on the OFD (Fdup / Fforce / inheritance) each owned-by-`run` entry calls
     `ixclose` (the 2nd+ return `EINTRN`, ignored); the first entry owned by
     **another** process returns `EACCDN` immediately — entries examined
     earlier may already have been `ixclose`d (`:660-669`, no rollback).
   - Same `ixdel` is reached from `Fcreate` over an existing file
     (`fsopnclo.c:163`; `EACCDN` on `ixdel < 0`), `Ddelete` (`fsdir.c:377`) and
     the two error paths of `Dcreate` (`fsdir.c:228,239`).

4. **Open by another process blocks.** `sft[n].f_own != run` → `EACCDN`
   (`bdos/fsopnclo.c:667-668`).

5. **Erase marker `0xE5`.** The first name byte is set to `0xE5`
   (`ERASE_MARKER`); the slot is not compacted and is reused by the next
   `Fcreate` (`bdos/fsopnclo.c:688-692`). **Correction:** an earlier draft of
   this spec claimed a name legitimately beginning with byte `0xE5` is stored
   on disk as `0x05` by the create path (the classic MS-DOS convention to avoid
   false-erasure). **This EmuTOS build implements no such substitution** —
   `builds()`/`ixcreat()` (`bdos/fsopnclo.c:99-208`, `bdos/fsdir.c:737-796`)
   copy/uppercase bytes verbatim (`toupper()` is a plain `'a'..'z'` fold,
   `util/string.c:171-177`) with no `0xE5`→`0x05` special case anywhere in
   `bdos/fsdir.c`/`bdos/fsopnclo.c`. A file whose 8.3-encoded name legitimately
   starts with the byte `0xE5` is therefore indistinguishable from an erased
   entry on the next directory scan (`match()`/`ixgetfcb()` both treat
   `f_name[0] == 0xE5` as "deleted", `bdos/fsdir.c:1818`, `:614`) — a latent,
   real EmuTOS limitation, not a NewTOS bug. See `_discrepancies.md` D-063.

6. **FAT chain freed cluster-by-cluster.** `getrealcl`/`clfix` write
   `FREECLUSTER`; a crash mid-free leaves lost clusters that a CHKDSK-style scan
   would reclaim (`bdos/fsopnclo.c:674-683`).

---

## Dependencies

### Requires (must be initialized / functional)

- FAT subsystem for the drive; buffer cache + `Rwabs`; `sft[]`/OFD chain.

### Used by

- File managers, installers, temp-file cleanup; `Frename`'s overwrite path via
  `ixcreat`→`ixdel` for a pre-existing target.

---

## Related data structures

- `docs/api/structures/ofd.md` — FCB, `ERASE_MARKER`, FAT chain fields.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents "returns 0 or
  error"; the "another process → EACCDN" and folder-is-EFILNF details are
  EmuTOS's.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: FAT delete with cluster-chain free; the `0xE5`/`0x05` on-disk erase
convention is preserved for native-ST media compatibility.

**FS01 (2026-09-24): a deleted entry takes its long-name slots with it.**
EmuTOS frees the 8.3 entry alone, because classic TOS knows no long names —
its directory scan only *skips* `FA_LFN` records (`bdos/fsdir.c:326,1810`) and
nothing ever erases them. NewTOS does not write long-name slots either, but
every card written by a modern host carries them, and leaving them behind on
delete accumulates orphan chains: after emptying a card completely, `fsck_msdos
-n` no longer completes (measured: it dies in phase 2 on signal 5,
`measurements/measurement-2026-09-24-fs01/serial/fsck.txt`).

So `Fdelete` (and `Ddelete`, and the `Fcreate`-over-an-existing-file path,
which shares `erase_entry`) now marks the 8.3 entry `0xE5` **and then** the
`attrib == 0x0F` slots in front of it — `dir::erase_slot_with_lfn`
(`tos-api/src/bdos/fs/dir.rs`). Which slots belong to the entry is decided the
way FreeMiNT's FAT layer decides it (`references/freemint/sys/fatfs.c:779-790`
for the `LDIR` record, `:3690-3748` for the ordinal/checksum validation), and
the erase order — short entry first, then the chain — is FreeMiNT's
`unlink_cookie_i` (`:4246-4272`). A chain that does not validate (wrong
checksum, broken ordinal sequence, already erased) is left untouched: it was
not this entry's name to begin with, and FreeMiNT's `__readvfat` rejects it the
same way. This is a **NewTOS decision** taken from the FAT layout, not
behaviour extracted from EmuTOS — EmuTOS has no answer here.

**PROC04 point 5 (2026-10-18) — `ixdel` as EmuTOS, plus a sanctioned
protection after the delete** (user decision 2026-10-18, variant b,
`docs/COMPATIBILITY.md` "Sanctioned deviations", "A file deleted while open no
longer reaches the card through its handles"). Replaces audit fix F6
(`00c8137a`), which freed every `sft[]` entry on the OFD — handles answered
`EIHNDL` afterwards — and deleted files open in another process; both were a
misreading of the old edge case 3.

`tos-api/src/bdos/fs/file.rs` `ixdel` is shared by `fdelete`, `fcreate` (an
existing file of that name; any `Err` ends the call — `EACCDN`, or a device
code as EmuTOS' `longjmp`) and `dirops::ddelete` (`fsdir.c:377`). It does what
`ixdel` does (`fsopnclo.c:652-699`):
- walks the OFDs still listed on the directory at the entry's position
  (`Ofd::listed` = on `d_files`), newest first (`Ofd::seq`, `makopn`
  `:294-295`), and for every `sft[]` entry on such an OFD: `f_own` = the running
  PD (`run_pd()`) → `ixclose`; otherwise → `EACCDN` at once, no rollback;
- reads the start cluster from the entry **after** those closes (a file written
  but not closed has it only in its OFD; `ixclose` writes it back, as EmuTOS's
  `f->f_clust` points into the buffer `ixclose` updated) and frees the chain;
- erases the entry and its long-name slots (FS01).

NewTOS additions: every OFD that still names the entry gets
`Ofd::deleted = Some(time/date)` and is unlisted; an unlisted OFD the owner check
does not see (a shared handle after its first close) has its pending length/start
cluster written back first, so the freed chain is complete. On a deleted OFD:
`Fread` → 0 (position unchanged), `Fwrite` (len ≥ 1) → `EACCDN`, len ≤ 0 → 0,
`Fseek` → target or `ERANGE` from `o_bytnum`/`o_fileln` without a chain walk
(`seek_deleted`), `Fdatime` get/set on the kept time/date only, `Fclose` →
`EINTRN` with no directory write and no flush (`ixclose`), redirected character
I/O via `xread`/`xwrite` as `Fread`/`Fwrite`; `eof` unchanged (needs no card).
A deleted OFD is never matched by a later `ixdel`, so a file or folder that
reuses the slot is unaffected (EmuTOS: not on `d_files` either).

`Dcreate`'s two `ixdel` error paths (`fsdir.c:228,239`) have no counterpart:
NewTOS's `dcreate` writes the directory entry last, after the cluster is
allocated and initialised, so there is no half-made entry to delete.

Tests `tos-api/tests/proc04_fdelete_open.rs` (9; red on the F6 code: 9/9
failed, incl. 3 clusters lost when a written, unclosed file was deleted);
probe `measurements/measurement-2026-10-18-proc04-fdelete/`.

**FS31 (2026-11-06) — the OFDs of the directory are those on its drive.**
EmuTOS `ixdel` walks `dn->d_files` (`bdos/fsopnclo.c:660`); that list hangs on
the directory node (`bdos/fs.h:224`), and a node belongs to one drive (`d_drv`,
`bdos/fs.h:219`). NewTOS has no node: an OFD names its entry by the directory's
start cluster and the slot, numbers every drive hands out alike (every root is
0, two folders made in the same order start at the same cluster). So `ixdel`
matches the OFD's drive as well, as `move_ofds` always did. Before, `Fdelete`,
`Fcreate` over an existing file and `Ddelete` on one drive treated a same-numbered
open file on another drive as theirs: refused with `EACCDN` if another process
had it open, else closed and marked deleted. Measured on the pinned oracle, same
card bytes on both sides, equal numbers logged:
`measurements/measurement-2026-11-06-fs31/RUNINFO.TXT`, probe
`tosdump/probes/xdprobe.c`, tests `tos-api/tests/fs31_two_drives.rs`.
