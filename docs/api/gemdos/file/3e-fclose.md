# `Fclose` (GEMDOS 0x3E)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsopnclo.c:426` (`xclose`) and the flush helper
> `bdos/fsopnclo.c:511` (`ixclose`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the handle-class behaviour (negative = no-op,
> standard = revert-to-default, non-standard = real close), reference-count
> semantics observable via `Fdup`/`Fforce`, error codes. Reference: `sftdel`,
> buffer-cache flush order.

## EmuTOS source

**Primary location:** `bdos/fsopnclo.c:426` — `long xclose(int h)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x3E` =
`xclose`, `wparms=1` (`bdos/bdosmain.c:258`; comment "will handle its own
redirection").

**Helpers called:**
- `bdos/fsmain.c:474` — `getofd()` — handle → OFD.
- `bdos/console.c:132` — `get_default_handle()` — the boot-default device for a
  standard handle (`default_handle[]`, `bdos/console.c:60`).
- `bdos/fsopnclo.c:511` — `ixclose()` — flush the FCB (date/time/start/length)
  to the directory entry, then flush all dirty buffers.
- `bdos/fsopnclo.c:380` — `sftdel()` — release the SFT entry and, when the last
  user is gone, free the OFD/DFD.

**Headers / type definitions used:**
- `bdos/fs.h:45` — `NUMHANDLES = 81`; `include/bdosdefs.h:61` — `NUMSTD = 6`.
- `bdos/fs.h:490-491` — `CL_DIR`, `CL_FULL` (`ixclose` part flags).
- `bdos/fs.h:130` — `O_DIRTY`.

**System variables / globals read/written:**
- `run->p_uft[h]` (standard-handle revert, `bdos/fsopnclo.c:452-453`).
- `sft[h-NUMSTD]` (`f_use`, `f_ofd`, `f_own`).
- Buffer-cache `bufl[]` (dirty-flush, `bdos/fsopnclo.c:576-579`).

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Fclose` closes a handle. Its behaviour depends on the handle class:

- **Negative handle** (`h < 0`): a raw BIOS character device — always a
  successful no-op, returns `E_OK` (`bdos/fsopnclo.c:432-433`).
- **`h >= NUMHANDLES` (81)**: out of range → `EIHNDL`
  (`bdos/fsopnclo.c:435-436`).
- **Standard handle `0..5`**: `p_uft[h]` is reverted to its boot default
  (`get_default_handle`), and the *previously mapped* handle is what actually
  gets closed. If that mapping was a BIOS device (negative), we are done
  (`E_OK`). If — "can't happen" — it mapped to another standard handle,
  `EIHNDL` (`bdos/fsopnclo.c:450-458`).
- **Non-standard handle `6..80`**: if its `sft[]` entry is mapped to a BIOS
  device (`f_ofd < 0`), just decrement `f_use`, freeing the slot at zero
  (`E_OK`). Otherwise fall through to the real close.

The real close validates the OFD (`getofd`; NULL → `EIHNDL`), calls `ixclose`
to write back date/time/start-cluster/length and set the archive bit (for
files) into the directory entry and flush all dirty buffers, then decrements
`f_use`; when it reaches zero, `sftdel` releases the slot and — when the last
OFD sharing the DFD is gone — frees the OFD/DFD.

The reference count `f_use` is why `Fdup`/`Fforce` interact with close: a
duplicated or forced handle bumps `f_use`, so several `Fclose` calls are needed
before the underlying OFD is actually released.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x3E
- **Stack frame on entry (m68k):** `Fclose(handle)` = `trap1(0x3e, handle)`:
  ```
  +2: handle (word)
   0: function_id (word) = 0x003E
  ```
- **Total parameter bytes:** 2 (WORD) → `wparms = 1`.
- **Native (AArch64) ABI:** ID `0x3E` in `x8`; `h` in `x0`; LONG result
  sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | +2         | `x0`        | `h`  | `WORD` (16, signed) |

---

## Inputs

| Position | Name | Type  | Size | Source location      | Notes |
|----------|------|-------|------|----------------------|-------|
| arg0     | `h`  | `int` | 2    | `bdos/fsopnclo.c:426`| `<0` BIOS dev; `0..5` standard; `6..80` non-standard |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `E_OK` (0) on success; negative error otherwise.
- **Source location:** `bdos/fsopnclo.c:433,456,471,495`.

### Side effects on global state

- Standard handle: `p_uft[h]` reset to its default device
  (`bdos/fsopnclo.c:453`).
- `sft[].f_use` decremented; at zero the slot is cleared and the OFD/DFD freed
  (`sftdel`, `bdos/fsopnclo.c:492-493`).
- `ixclose` writes the directory entry (date/time/start/length, archive bit)
  **only if the file is `O_DIRTY`** (edge case 8), and flushes all dirty
  buffer-cache blocks.

### Side effects on hardware

- Directory-sector and dirty-data writeback to the block device.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `h >= NUMHANDLES` (81) | `bdos/fsopnclo.c:435-436` | `EIHNDL` (-37) |
| Standard handle mapped to another standard handle ("can't happen") | `bdos/fsopnclo.c:457-458` | `EIHNDL` (-37) |
| Non-standard handle with no OFD (`getofd` NULL) | `bdos/fsopnclo.c:481-482` | `EIHNDL` (-37) |
| Internal: OFD not found on its directory's open-file list | `bdos/fsopnclo.c:565` (`ixclose`) | `EINTRN` (-65) |

Underlying BIOS write errors during the flush propagate through the
GEMDOS `longjmp` recovery path (see `docs/api/gemdos/_dispatcher.md` §8), not as
a direct `Fclose` return.

**Error paths not in Compendium:** the `EINTRN` from `ixclose` when the OFD is
missing from `d_files`, and the negative-handle no-op (`E_OK` for `h < 0`).

---

## Verification criteria

- **Nominal:** `Fclose(h)` on a handle from `Fopen`/`Fcreate` returns `E_OK`
  (0) and writes back the file's final length/time; a following op on `h`
  returns `EIHNDL`.
- **Negative handle no-op:** `Fclose(-1)` returns `E_OK` and changes nothing.
- **Out-of-range:** `Fclose(81)` returns `EIHNDL` (-37).
- **Standard-handle revert:** after `Fforce(1, fh)` then `Fclose(1)`,
  handle 1 reads/writes the default console again, and `fh`'s OFD is released
  only when `f_use` hits zero.

---

## Edge cases discovered in source

1. **`h < 0` always succeeds and does nothing.** BIOS character devices have no
   OFD to release (`bdos/fsopnclo.c:432-433`). AES uses `Fclose(-1)` as a
   harmless clean-up.

2. **Closing a standard handle reverts, does not free the device.**
   `p_uft[h]` is set back to `default_handle[h]` (`bdos/fsopnclo.c:453`); the
   underlying *forced* handle is what gets closed afterward.

3. **Reference counting via `f_use`.** Starts at 1 in `opnfil`, bumped by
   `Fforce` (non-BIOS mapping) and set to 1 for each new `Fdup` slot. Only at
   `f_use == 0` does `sftdel` free the OFD (`bdos/fsopnclo.c:492-493`).

4. **DFD sharing survives one close.** If multiple OFDs share a DFD (same file
   opened twice), `sftdel` decrements `o_usecnt` and only frees the base OFD
   when the last user is gone (`bdos/fsopnclo.c:396-409`).

5. **Flush order: directory entry before data.** `ixclose` first rewrites the
   FCB (date/time/start/length + archive), then flushes all dirty buffers
   (`bdos/fsopnclo.c:530-579`). `CL_DIR`/`CL_FULL` are internal partial-close
   flags; public `Fclose` uses part `0` (full close).

6. **Second close of a shared OFD → `EINTRN`, no flush.** `ixclose(fd, 0)`
   unlinks the OFD from `o_dnode->d_files` (`bdos/fsopnclo.c:552-566`). An OFD
   shared by several handles — `Fforce`d onto a standard handle, inherited by
   a child (`init_pd_files`), or `Fdup`ed — is therefore unlinked by the first
   close; every later close still writes a dirty directory entry
   (`:530-550`), then finds no list entry and returns `EINTRN` (-65) **before**
   the buffer flush (`:576-579`). `xclose` decrements `f_use` regardless
   (`:492-493`). In the mintlib `system("prog >file")` sequence the child's
   `Pterm` performs the first close, so the parent's `Fclose(outfd)` returns
   `EINTRN` (ignored by `system`).

7. **Directory lengths are stored as zero on disk.** When closing with `CL_DIR`
   the on-disk `f_fileln` is forced to 0 (`bdos/fsopnclo.c:542-543`); regular
   files get the archive bit set instead.

8. **`FA_ARCHIVE` is set only for a changed file, OR-ed into the entry's
   current attribute** [FS21]. The directory-entry rewrite runs only when the
   DFD is `O_DIRTY` (`bdos/fsopnclo.c:530`); it reads the attribute byte the
   entry has *now* (`attr = fcb->f_attrib`, `:537`), copies time/date, start
   cluster and length from the DFD (`:538-540`), OR-s in `FA_ARCHIVE` for a
   file (`:544-545`) and writes the byte back (`:547-548`). `O_DIRTY` is set
   by:
   - `Fcreate` — the new DFD (`bdos/fsopnclo.c:206`): `Fcreate` + `Fclose`
     with no write sets the bit;
   - a write that grows the file (`addit`, `bdos/fsio.c:42-46`) or gives it
     its first cluster (`nextcl`, `bdos/fsfat.c:279-282`);
   - an `Fdatime` set (`bdos/fsdir.c:698`), even on a file opened read-only;
   - a cross-directory `Frename` of a file (`bdos/fsdir.c:1066`, then
     `xclose` of the new handle, `:1072`).
   Not dirty, so the attribute is left as it is: a file only opened, read,
   sought, or written *inside* its length, and an `Fdatime` get. An `Fattrib`
   made while the file is open changes the entry directly
   (`bdos/fsdir.c:424-432`); the later close OR-s into that value, so a
   cleared archive bit comes back and a set `FA_RO` stays. Folders are closed
   with `CL_DIR` and get no bit (edge case 7). Measured: the FS20 probe's
   `GCPROBE.DAT` (Fcreate, 512-byte Fwrite, Fclose) ends with attribute
   `0x20` on the oracle (`measurements/measurement-2026-10-28-fs20/RUNINFO.TXT`).

---

## Dependencies

### Requires (must be initialized / functional)

- `sft[]`, `run->p_uft[]`; a valid OFD for non-standard handles.
- Buffer cache + `Rwabs` for the flush.

### Used by

- Every program that opened a handle; process termination auto-closes handles
  owned by the exiting PD.

---

## Related data structures

- `docs/api/structures/ofd.md` — OFD/DFD, `f_use`/`o_usecnt`.
- `docs/api/gemdos/_state.md` §2 — SFT and standard-handle model.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents "returns 0 on
  success"; the negative-handle no-op and `EINTRN` internal path are EmuTOS's.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `NUMHANDLES = 81`, `NUMSTD = 6`.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: identical SFT / `p_uft[]` semantics; the flush ordering (directory
entry before data) is a crash-consistency contract worth preserving.

TASK/PROC02 (2026-10-06): `tos-api/src/bdos/fs/file.rs` `fclose` follows
`xclose` (`bdos/fsopnclo.c:426-496`): negative → `E_OK`; `h >= NUMHANDLES`
(`NUMSTD + OPEN_FILES` = 81 as in EmuTOS since TASK/PROC04 point 4,
2026-10-18; 22 before, `_state.md` §2) → `EIHNDL`; a standard handle reverts to
`default_handle[h]` (`-1,-1,-2,-3,-1,-1`) and closes what it mapped; a
BIOS-mapped entry only loses a use; otherwise `ixclose` (dirty directory
entry; `EINTRN` if the OFD is no longer listed; buffer flush, whose failure
is the result) and `f_use - 1`, `sftdel` at zero (the OFD goes with the last
entry that points at it). Before PROC02 every close released the slot at
once, so a handle `Fforce`d onto a standard handle died with the first
`Fclose` (PORT10: `Fwrite(1)` → `EIHNDL` after `system("… >file")`).

TASK/FS21 (2026-10-31): the archive bit. Before FS21 NewTOS' close wrote
cluster and length only, so every file it wrote ended with attribute `0x00`
where the oracle has `0x20` (FS20 probe, `gemdos-mc-fs20`/`-fs24` end
states). Now `flush_ofd`
(`tos-api/src/bdos/fs/file.rs:736@flush_ofd`), called by `ixclose` for a
dirty OFD, OR-s `FA_ARCHIVE` into the attribute the entry has at close
(edge case 8); `fcreate` opens its OFD dirty and an `Fdatime` set marks it
dirty, as `O_DIRTY` in EmuTOS. NewTOS keeps no `o_td`: `Fcreate` and
`Fdatime` write time/date into the entry directly, so the close does not copy
them. An OFD whose file was deleted while open writes nothing (PROC04
point 5); the pre-flush in `ixdel` of an OFD no longer listed sets the bit
on the entry it then erases, as that OFD's later EmuTOS close would. Host
test `tos-api/tests/fs21_close_archive.rs`.
