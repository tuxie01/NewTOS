# `Fdatime` (GEMDOS 0x57)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsdir.c:684` (`xgsdtof`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, get-vs-set on an **open handle**, the `DOSTIME`
> buffer format and its byte-swap direction, the `EIHNDL` error, the
> set-marks-dirty behaviour. Reference: the `swpcopyw` implementation, the DFD.

## EmuTOS source

**Primary location:** `bdos/fsdir.c:684` — `long xgsdtof(DOSTIME *buf, int h, int wrt)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x57` =
`xgsdtof`, `wparms=4` (`bdos/bdosmain.c:287`).

**Helpers called:**
- `bdos/fsmain.c:474` — `getofd()` — handle → OFD.
- `include/asm.h:115` — `swpcopyw()` — copy a WORD with byte-swap.

**Headers / type definitions used:**
- `bdos/fs.h:96-100` — `DOSTIME` (`{ UWORD time; UWORD date; }`).
- `bdos/fs.h:116-124` — `DFD` (`o_td`, `o_flag`); `bdos/fs.h:130` — `O_DIRTY`.

**System variables / globals read:** the OFD's DFD `o_td` (get) or the caller's
buffer (set).
**System variables / globals written:** on set — the DFD `o_td` and
`o_flag |= O_DIRTY`; the on-disk directory entry is updated later by `Fclose`.

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Fdatime` gets or sets the time/date stamp of a file via an **open handle**
(there is no path-based variant). `xgsdtof` looks up the OFD (invalid handle →
`EIHNDL`) and reaches its DFD, which mirrors the FCB.

The direction of `wrt` selects source and destination, and every copy goes
through `swpcopyw`, which byte-swaps a WORD (`include/asm.h:115-122`). This is
because the DFD stores `o_td.time`/`o_td.date` in **little-endian** (the on-disk
FCB/FAT convention), while the caller's `DOSTIME` buffer is in **native (m68k
big-endian)** order:

- **Set** (`wrt == 1`): copy the caller's `buf->time`/`buf->date` (big-endian)
  into the DFD `o_td` (byte-swapped to little-endian) and set `O_DIRTY`, so the
  change is written back to the directory when the file is closed.
- **Get** (`wrt == 0`): copy the DFD `o_td` (little-endian) into the caller's
  `buf` (byte-swapped to big-endian). No dirty flag is set.

The `DOSTIME` fields use the standard packed DOS format (2-second time
resolution, year relative to 1980). Setting the same file's time twice still
re-marks it dirty. Because the write only takes effect on `Fclose`/flush, a file
opened read-only is sufficient to *stamp* it (the dirty flush happens on close);
but there is no way to touch a *closed* file — `Fopen` it first.

A source comment marks the `O_DIRTY` set as EmuTOS bug-fix M01.01.0918.01: the
original Atari TOS forgot it, so set-time was lost on close. EmuTOS (and NewTOS)
keep the fix.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x57
- **Stack frame on entry (m68k):** `Fdatime(timeptr, handle, wflag)` =
  `trap1(0x57, timeptr, handle, wflag)` (`include/bdosbind.h:54`):
  ```
  +8: wflag   (word)
  +6: handle  (word)
  +2: timeptr (LONG, pointer to DOSTIME)
   0: function_id (word) = 0x0057
  ```
- **Total parameter bytes:** 8 (LONG + WORD + WORD) → `wparms = 4`.
- **Native (AArch64) ABI:** ID `0x57` in `x8`; args in `x0..x2`; LONG result
  (0 or negative error) sign-extended into `x0`. On get, the result is written
  into the `DOSTIME` **output buffer** at `buf`.

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `buf` | `DOSTIME *` (ptr) |
  | +6         | `x1`        | `h`   | `WORD` (16) — open handle |
  | +8         | `x2`        | `wrt` | `WORD` (16) — 0 get, 1 set |

---

## Inputs

| Position | Name  | Type       | Size | Source location    | Notes |
|----------|-------|------------|------|--------------------|-------|
| arg0     | `buf` | `DOSTIME *`| 4    | `bdos/fsdir.c:684` | dest on get, source on set |
| arg1     | `h`   | `int`      | 2    | `bdos/fsdir.c:686` | open file handle |
| arg2     | `wrt` | `int`      | 2    | `bdos/fsdir.c:694` | 0 = get, 1 = set |

### `DOSTIME` buffer (4 bytes, native/m68k byte order in the caller's buffer)

| Offset | Field  | Packed encoding |
|-------:|--------|-----------------|
| 0      | `time` | bits 15-11 hour (0-23), 10-5 minute (0-59), 4-0 seconds/2 (0-29) |
| 2      | `date` | bits 15-9 year-1980 (0-127), 8-5 month (1-12), 4-0 day (1-31) |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `E_OK` (0) on success; negative error otherwise.
- **Source location:** `bdos/fsdir.c:706`.

### Side effects on global state

- Set only: writes the DFD `o_td` (byte-swapped) and sets `O_DIRTY`
  (`bdos/fsdir.c:696-698`), so `Fclose` writes it to the directory entry.
- Get: fills the caller's `buf` (`bdos/fsdir.c:701-703`), no dirty flag.

### Side effects on hardware

- None directly; the actual directory write happens at `Fclose`/flush.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Invalid handle (`getofd` NULL) | `bdos/fsdir.c:688-690` | `EIHNDL` (-37) |

---

## Verification criteria

- **Get:** on an open file, `Fdatime(&t, h, 0)` returns `E_OK` and fills `t`
  with the entry's packed time/date in native byte order.
- **Set then persist:** `Fdatime(&t, h, 1)` returns `E_OK`, marks the DFD dirty;
  after `Fclose(h)` a fresh `Fopen`+`Fdatime` get returns the value just set.
- **Set is lost without close:** setting time then never closing (crash) does
  not update the directory entry.
- **Error:** `Fdatime(&t, 9999, 0)` returns `EIHNDL` (-37).

---

## Edge cases discovered in source

1. **Endianness asymmetry.** DFD `o_td` is little-endian (disk order); the
   caller's `DOSTIME` is native/big-endian; every copy is byte-swapped via
   `swpcopyw` (`bdos/fsdir.c:696-703`, `include/asm.h:115-122`). On AArch64 the
   swap is preserved so m68k-ABI callers see big-endian words.

2. **Set marks the DFD dirty; get does not.** `O_DIRTY` is set only on write, so
   a read-only close does not rewrite the directory (`bdos/fsdir.c:698`).

3. **Persisted on close, not immediately.** The directory entry is updated by
   `Fclose`/buffer flush, not by `Fdatime` itself.

4. **Open handle only — no path form.** Unlike POSIX `utime`, there is no
   path-based variant; `Fopen` the file first (read access is enough to stamp,
   because the flush is on close).

5. **2-second time resolution, year 1980-2107.** The packed format has a 5-bit
   seconds/2 field and a 7-bit year-1980 field; odd seconds round down, and the
   representable year range is 1980-2107.

6. **EmuTOS bug-fix M01.01.0918.01.** Adds the missing `O_DIRTY` on set that
   original Atari TOS lacked (`bdos/fsdir.c:698`). Kept in NewTOS.

---

## Dependencies

### Requires (must be initialized / functional)

- A valid open OFD (from `Fopen`/`Fcreate`); `Fclose` to persist a set.

### Used by

- File managers, build tools, archivers preserving timestamps.

---

## Related data structures

- `docs/api/structures/ofd.md` — DFD `o_td`, `O_DIRTY`.
- `docs/api/gemdos/time/2a-tgetdate.md` — the system-clock analogue.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents get/set and the
  packed `DOSTIME` format.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None; behaviour independent of build flags.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: the byte-swap is preserved as an ABI requirement — TOS programs pass
and expect big-endian `DOSTIME`. On the little-endian AArch64 host, `swpcopyw`
becomes an explicit swap so the observable buffer bytes match m68k order.

TASK/FS21 (2026-10-31): NewTOS writes a set time/date into the directory
entry at once (it keeps no `o_td`) and, as EmuTOS' `O_DIRTY`
(`bdos/fsdir.c:698`), marks the OFD dirty, so the `Fclose` rewrites the entry
and sets `FA_ARCHIVE` (`3e-fclose.md` edge case 8;
`tos-api/src/bdos/fs/file.rs:1148@fdatime`). A file deleted while open is
unchanged (PROC04 point 5).
