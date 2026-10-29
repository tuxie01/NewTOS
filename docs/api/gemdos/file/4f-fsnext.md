# `Fsnext` (GEMDOS 0x4F)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/fsdir.c:654` (`xsnext`) and the work helper
> `bdos/fsdir.c:545` (`ixsnext`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, "resume from the DTA private state", `ENMFIL` on
> exhaustion / uninitialised DTA, the refilled public DTA fields. Reference:
> the private-area encoding, `ixsnext`/`match` walk.

## EmuTOS source

**Primary location:** `bdos/fsdir.c:654` — `long xsnext(void)`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x4F` =
`xsnext`, `wparms=0` (`bdos/bdosmain.c:277`).

**Helpers called:**
- `bdos/fsdir.c:545` — `ixsnext()` — resume the directory walk from the DTA's
  saved position.
- `bdos/fsdir.c:1802` — `match()`, `bdos/fsdir.c:1851` — `makbuf()`;
  `bdos/fsfat.c` — `getrealcl()`, `cl2rec()`; `bdos/fsbuf.c` — `getbcb()`.
- `bdos/fsdir.c` — `builds()` (rebuild the FCB-style pattern from `dt_name`).

**Headers / type definitions used:**
- `bdos/fs.h:319-337` — `DTAINFO`; `bdos/fs.h:339` — `DTA_DRIVEMASK`.

**System variables / globals read:** `run->p_xdta` (private search state);
`drvtbl[]` (the DMD for the search drive).
**System variables / globals written:** the DTA private area (advanced) and
public fields (refilled), or `dt_offset_drive = -1` on exhaustion.

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Fsnext` returns the next entry matching the pattern established by the most
recent `Fsfirst` on the same DTA. It takes no arguments — all state lives in the
process's current DTA at `run->p_xdta`.

`xsnext` first checks the private `dt_offset_drive`: if it is negative the DTA is
"uninitialised" — either `Fsfirst` was never called, it failed, or a previous
`Fsnext` already exhausted the search — and it returns `ENMFIL`. Otherwise it
calls `ixsnext`.

`ixsnext` reconstructs the search: it extracts the drive (low 5 bits of
`dt_offset_drive`), rebuilds the FCB-style pattern from `dt_name` and `dt_attr`,
and picks up either the root-directory scan (`dt_cloffset == 0 && dt_clnum == 0`,
resume byte offset from `dt_offset_drive`) or the sub-directory scan (resume
`dt_cloffset` within cluster `dt_clnum`). It walks entries with `match()`,
handling cross-cluster advance via `getrealcl` and stopping at end-of-chain or
at a directory entry whose first byte is `0x00` (never-used → end of directory).
On a match it updates the private area to point just past the found FCB and
returns it; `xsnext` calls `makbuf` to refill the public DTA fields.

If `ixsnext` finds nothing, `xsnext` sets `dt_offset_drive = -1` (so further
`Fsnext` calls keep returning `ENMFIL`) and returns `ENMFIL`.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x4F
- **Stack frame on entry (m68k):** `Fsnext()` = `trap1(0x4f)`:
  ```
   0: function_id (word) = 0x004F
  ```
- **Total parameter bytes:** 0 → `wparms = 0`.
- **Native (AArch64) ABI:** ID `0x4F` in `x8`; no argument registers; LONG
  result (0 or negative error) sign-extended into `x0`. The next match is
  delivered via the **DTA output buffer** at `run->p_xdta`.

  (no stack parameters)

---

## Inputs

None — the search state is entirely in the per-process DTA at `run->p_xdta`.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `E_OK` (0) with the DTA refilled on a match; `ENMFIL` (-49) when
  there are no more entries or the DTA is uninitialised.
- **Source location:** `bdos/fsdir.c:663,670,675`.

### DTA output buffer

Same public fields as `Fsfirst` (attribute, packed time/date in native byte
order, size, dotted 8.3 name), refilled by `makbuf`
(`bdos/fsdir.c:673,1851`). See `docs/api/gemdos/file/4e-fsfirst.md`.
`d_fname` is written only up to and including its NUL (`packit`,
`bdos/fsdir.c:1297-1324`): after a shorter name the tail of the previous,
longer one stays behind the NUL (`docs/api/structures/dta.md`, `d_fname`).

### Side effects on global state

- The DTA private area is advanced to the next resume point, or set to the
  "exhausted" sentinel `-1`.

### Side effects on hardware

- Directory-sector reads via the buffer cache.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| DTA uninitialised (`dt_offset_drive < 0`: no prior `Fsfirst`, failed search, or already exhausted) | `bdos/fsdir.c:662-663` | `ENMFIL` (-49) |
| No more matching entries | `bdos/fsdir.c:667-671` | `ENMFIL` (-49) |
| Drive id in DTA out of range (`>= BLKDEVNUM`) | `bdos/fsdir.c:557-558` (`ixsnext` → NULL) | `ENMFIL` (-49) |

**Correction:** the single "no more / not initialised" signal is **`ENMFIL`
(-49)** in every case. An earlier draft claimed `Fsnext` returns `EINTRN` (-65)
when called without a prior `Fsfirst` — the source has no such path.

---

## Verification criteria

- **Enumerate:** after `Fsfirst("C:\\*.TXT", 0)` returns `E_OK`, repeated
  `Fsnext()` return `E_OK` for each further `.TXT` file, refilling the DTA, until
  `ENMFIL` (-49).
- **Exhaustion sticks:** once `Fsnext` returns `ENMFIL`, further `Fsnext` calls
  keep returning `ENMFIL` (private state is `-1`).
- **No prior Fsfirst:** calling `Fsnext()` on a DTA that never had a successful
  `Fsfirst` returns `ENMFIL` (-49).
- **Single exact match:** after `Fsfirst("C:\\ONE.TXT", 0)` (no wildcard),
  `Fsnext()` returns `ENMFIL`.

---

## Edge cases discovered in source

1. **One sentinel covers three cases.** `dt_offset_drive < 0` means "never
   initialised", "search failed", or "already exhausted" — all return `ENMFIL`
   (`bdos/fsdir.c:662-663`).

2. **Root vs. sub-directory resume.** `dt_cloffset == 0 && dt_clnum == 0`
   selects the fixed-size root scan; otherwise the cluster-chained sub-dir scan
   (`bdos/fsdir.c:568-583`).

3. **End-of-directory marker.** A directory entry with first byte `0x00` is
   "never used" and terminates the scan immediately (`bdos/fsdir.c:614-615`).
   Erased (`0xE5`) and long-filename entries are skipped by `match()`.

4. **Cross-cluster advance.** When a cluster is exhausted, `getrealcl` chains to
   the next; end-of-chain → no more matches (`bdos/fsdir.c:601-607`).

5. **No `Fsclose`.** There is no explicit close for a search; the private-area
   sentinel after exhaustion is the only "end" signal — no resources to free
   (`bdos/fsdir.c:669`).

6. **State is per-DTA, per-process.** Nested/concurrent searches must each use
   their own DTA (swap via `Fsetdta`/`Fgetdta`); overwriting bytes 0x00-0x14
   corrupts the walk.

7. **Media change aborts.** A disk swap between `Fsfirst` and `Fsnext` on the
   same drive surfaces as a BIOS `E_CHNG` on the next directory read (via the
   `longjmp` path), aborting the enumeration.

---

## Dependencies

### Requires (must be initialized / functional)

- A prior successful `Fsfirst` on the same `run->p_xdta`.
- The DMD for the search drive still logged in; buffer cache.

### Used by

- Paired with every `Fsfirst`; file managers, AES desktop menu population,
  batch/shell globbing.

---

## Related data structures

- `docs/api/structures/dta.md` — DTA private/public layout.
- `docs/api/gemdos/file/4e-fsfirst.md` — establishes the search.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents `ENMFIL` on
  exhaustion.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- None.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-05: identical resume model; the private-area encoding is free as long as
`Fsnext` resumes correctly and returns `ENMFIL` for all three exhausted/
uninitialised cases.
