# `Mediach` (BIOS 0x09)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1184` (`mediach`) and the installed handler
> `bios/blkdev.c:783` (`blkdev_mediach`).

> **BIOS02 revision (2026-10-28) — for user approval.** Changes tagged
> **[BIOS02]**. Source re-read at the pin `1a7484ab`
> (`references/emutos/bios/blkdev.c:783-820`). Cross-check: oracle
> `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN` vs NewTOS, run
> `measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`. The
> `forcechange` handshake is no longer a "Step-2 candidate": it is a measured
> difference (W02, W05, X13) and a requirement (BIOS-STOR-001).

## EmuTOS source

**Primary location:** `bios/bios.c:1184` — `LONG mediach(WORD drv)`
(body: `return protect_w(hdv_mediach, drv);`).

**Handler behind the vector:** `bios/blkdev.c:783` — `blkdev_mediach()`,
installed as `hdv_mediach` by `blkdev_init()` (`bios/blkdev.c:103`).

**Helpers called:**
- `util/miscasm.S:225` — `protect_w(func, drv)` — d2/a2-preserving thunk.
- `bios/floppy.c` — `flop_mediach(dev)` (units < `NUMFLOPPIES`).
- `bios/disk.c` — `disk_mediach(unit)` (hard-disk units).
- `bios/blkdev.c` — `call_etv_critic()` (critical-error retry).

**Headers / type definitions used:**
- `bios/disk.h:69-71` — `MEDIANOCHANGE` (0), `MEDIAMAYCHANGE` (1),
  `MEDIACHANGE` (2)
- `include/gemerror.h` — `EUNDEV` (-15)
- `include/tosvars.h` — `hdv_mediach`

**System variables / globals read/written:**
- `hdv_mediach` (TOS var **`0x47e`**) — the media-change vector
  (`bios/blkdev.c:103`)
- `blkdev[dev].mediachange` (read; latched), `blkdev[dev].forcechange` (read)
- `units[unit].status` (`UNIT_CHANGED` bit set on change, `bios/blkdev.c:815`)

**Build configuration affecting this function:** the underlying
`flop_mediach`/`disk_mediach` behaviour depends on the floppy/ACSI/IDE/SCSI
driver `CONF_*` flags (behaviour reference).

---

## Source extraction summary

`Mediach` reports whether the media in a drive may have changed since the last
access, as a 3-state value. The trap-level `mediach` forwards to
`hdv_mediach`; on EmuTOS that is `blkdev_mediach`.

`blkdev_mediach` validates the drive (`0..BLKDEVNUM-1`, `DEVICE_VALID`) and
returns `EUNDEV` for an invalid one. If the drive is already latched as
`MEDIACHANGE`, it returns that immediately. If a `forcechange` is armed (e.g.
by a prior write to logical sector 0, see `04-rwabs.md`) it returns
`MEDIACHANGE`. Otherwise it calls `flop_mediach`/`disk_mediach` (with
`call_etv_critic` retry). If that returns a **negative** error, `Mediach`
**returns that error unchanged**. If it reports a change (`!= MEDIANOCHANGE`),
it marks `units[unit].status |= UNIT_CHANGED` and latches
`blkdev[dev].mediachange`. Finally it returns the latched `mediachange`.

The 3-state distinction (`MAYCHANGE` vs `CHANGE`) exists because some hardware
cannot tell "new disk" from "same disk reinserted" without re-reading the boot
sector; `MEDIAMAYCHANGE` forces GEMDOS to re-validate the BPB/serial.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x09
- **Stack frame on entry (m68k):**
  ```
  +2: drv (word)
   0: function_id (word) = 0x0009
  ```
- **Total parameter bytes:** 4
- **Native (AArch64) ABI:** ID `0x09` in `x8`; `drv` in `x0`; result
  sign-extended into `x0` (0/1/2, or a negative error code).

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `drv` | `WORD` (16) |

---

## Inputs

| Position | Name  | Type   | Size | Source location     | Notes |
|----------|-------|--------|------|---------------------|-------|
| arg0     | `drv` | `WORD` | 2    | `bios/blkdev.c:783` | logical drive 0=A:..25=Z: **[BIOS02]**; validated against `BLKDEVNUM` (26, `include/sysconf.h:43`) |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding (success):** 3-state — `0` no change, `1` may have changed, `2`
  definitely changed (`bios/disk.h:69-71`).
- **Encoding (error):** a negative BIOS error code (see below), passed through
  from the driver.
- **Source location:** `bios/blkdev.c:819` (success) / `:790,811` (errors).

| Value | Constant        | Meaning |
|------:|-----------------|---------|
| 0     | `MEDIANOCHANGE` | media definitely has not changed |
| 1     | `MEDIAMAYCHANGE`| media may have changed (re-validate BPB) |
| 2     | `MEDIACHANGE`   | media definitely has changed |
| < 0   | (driver error)  | e.g. `EUNDEV`, or `flop_/disk_mediach` failure |

### Side effects on global state

- `units[unit].status |= UNIT_CHANGED` and `blkdev[dev].mediachange` latched
  when a change is detected (`bios/blkdev.c:814-816`).
- The latched `mediachange` is **not** cleared here (it is cleared by
  `Getbpb`, `bios/blkdev.c:589`). A repeated `Mediach` keeps returning the
  latched value.

### Side effects on hardware

- May read the floppy WD1772 status register (which itself clears the
  hardware change latch) or issue a Test-Unit-Ready to a removable SCSI/USB
  unit, per the driver.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `drv` invalid / not `DEVICE_VALID` | `bios/blkdev.c:789` | `EUNDEV` (-15) |
| Underlying `flop_mediach`/`disk_mediach` returns negative (after `etv_critic` retry) | `bios/blkdev.c:810-811` | that negative code |

**Corrections vs. prior spec:** an out-of-range drive returns **`EUNDEV`, not
`0`**; and driver I/O errors are **propagated as negative codes, not collapsed
to `1` (may have changed)**. Both prior claims were wrong.

**Error paths not in Compendium:** the `EUNDEV` and driver-error pass-through.

---

## Verification criteria

- **Nominal (no change):** `Mediach(2)` on a settled fixed disk returns `0`.
- **Nominal (changed):** after a floppy swap, `Mediach(0)` returns `2`.
- **After a sector-0 write [BIOS02, measured]:** once a logical `Rwabs` write
  starting at sector 0 has been issued on C: (whether it returned `E_CHNG` or,
  with `RW_NOMEDIACH`, 0), `Mediach(2)` returns `2` (oracle W02, W05, X13) —
  each time, until `Getbpb(2)`; then `0` (W07, X16). A write to any other
  sector leaves it `0` (W10).
- **Latched:** two successive `Mediach(d)` calls without an intervening
  `Getbpb(d)` return the same latched value; `Getbpb(d)` then resets it to `0`.
- **Invalid drive:** `Mediach(15)` with no P: returns `EUNDEV` (-15), **not**
  `0`.

---

## Edge cases discovered in source

1. **Out-of-range returns `EUNDEV`, not 0.** `bios/blkdev.c:789-790`. (Prior
   spec incorrectly said "returns 0 to avoid panic".)

2. **Negative driver errors are passed through.** If `flop_mediach`/
   `disk_mediach` returns `< 0` after the critical-error retry, that exact code
   is returned (`bios/blkdev.c:810-811`) — it is **not** normalised to
   `MEDIAMAYCHANGE`.

3. **`forcechange` short-circuits to `MEDIACHANGE`.** If a prior write to
   logical sector 0 armed `forcechange`, `Mediach` returns `2` without probing
   hardware, until `Getbpb` clears it (`bios/blkdev.c:802-803`). **[BIOS02]**
   This return neither stores `mediachange` nor sets `UNIT_CHANGED`
   (`bios/blkdev.c:795-819` — those are written only on the probe path), so
   the answer depends on `forcechange` alone; it is the same state `Rwabs`
   step 5 tests (`04-rwabs.md`, order table). Note the test order inside
   `blkdev_mediach`: the latched `mediachange == MEDIACHANGE` (line 795) is
   tested before `forcechange` (line 802).

4. **The change is latched, not one-shot.** Once `mediachange == MEDIACHANGE`
   is set, `Mediach` keeps returning `2` (`bios/blkdev.c:795-796`); only
   `Getbpb` clears it. (Contrast the hardware WD1772 latch, which the *first*
   read clears — but EmuTOS re-latches in `blkdev[]`.)

5. **Media-change protocol is caller responsibility.** `Rwabs` only auto-checks
   when `RW_NOMEDIACH` is *not* set; direct `Rwabs` callers that set it and
   skip `Mediach` can corrupt a swapped disk.

---

## Dependencies

### Requires (must be initialized / functional)

- `hdv_mediach` populated (`blkdev_init`).
- Per-drive `blkdev[]` state; a live driver with a `mediach` probe.

### Used by

- GEMDOS FAT-write paths (mandatory pre-flight check); `Getbpb` (resets the
  flag); `Rwabs` (internal check in logical mode).

---

## Related data structures

- `docs/api/bios/_state.md` — `hdv_mediach` vector + address, media constants
- `docs/api/structures/bpb.md` — re-validated on `MEDIAMAYCHANGE`

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically documents
  the 3-state return; does not mention `EUNDEV`/negative pass-through.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

None specific to `Mediach`; behaviour of the probes varies by driver `CONF_*`.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-04: SD-card removal is detected via the card-detect line; USB-MSC via
hot-plug events. Both feed a per-drive `mediachange` flag with the same
latch/reset contract (`Mediach` reads the latch; `Getbpb` clears it). The
`EUNDEV`-on-invalid-drive and negative-error pass-through must be preserved.

FS09 (2026-10-07): when `Mediach` ever reports a change, the block read cache
must drop that medium's sectors as well (FreeMiNT `fatfs_dskchng` →
`bio_free_di` → `bio_invalidate`; `docs/analysis/fs09-block-cache-2026-10-07.md`
§2.7). Today `Mediach` is constant 0 and the only invalidation point is
`bios::mount()` (new partition scan).

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the WD1772 floppy change-line / ACSI
> removable probe to the **BCM2711 EMMC2 SD card**. The 3-state contract
> (`0`/`1`/`2`, `EUNDEV` on invalid drive) is unchanged; only the backing model
> is pinned here. See `docs/HARDWARE-PROFILE.md` §3.4.

### The SD card is treated as non-removable → `0` for a valid drive

`Mediach` (`tos-api/src/bios/mod.rs:159-173`) **validates the drive first**
(`drive::resolve`): an out-of-range / unmounted drive returns `EUNDEV` (-15), a
valid drive returns **`0` (`MEDIANOCHANGE`)** — it never probes hardware for a
change. Rationale: the on-board microSD is a fixed boot card; the boot filesystem
does not change under the running system, so a valid drive never has a media
change to report (`docs/HARDWARE-PROFILE.md` §3.4, project memory "Mediach treats
the card as non-removable"). Matches EmuTOS `blkdev.c:789-790` (validate, then
report no-change).

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| WD1772 floppy change-line (first status read clears the latch) | **no probe** — the SD has no removal event wired |
| ACSI/SCSI removable Test-Unit-Ready | **no probe** |
| `forcechange` armed by a sector-0 write | **not armed** — `Rwabs` does not set any change flag (see `04-rwabs.md`) |

### `forcechange` / sector-0-write contract vs. the Pi

The EmuTOS contract has two ways `Mediach` reports a change without hardware
help: (1) a latched `mediachange` from a prior probe, and (2) a `forcechange`
armed by a logical write to sector 0 (forces a re-login when the boot sector is
edited). **Neither is implemented on NewTOS today:** there is no per-drive
`mediachange`/`forcechange` state, and `Rwabs`'s sector-0 write does not arm
anything, so `Mediach` always returns `0` even immediately after rewriting the
boot sector. With a genuinely non-removable card this is behaviourally safe
*only until a program writes logical sector 0* — **[BIOS02]** that case is
measured (W01-W05, X12-X14) and differs; it is a **divergence from the
contract**, not a cosmetic one.

### Divergences vs. the contract (Step-2 candidates)

- ~~**Invalid-drive `EUNDEV` not returned.**~~ **[Resolved (finding S1) —
  `mediach` validates `drv` via `drive::resolve` and returns `EUNDEV` (-15) for
  an out-of-range / unmounted drive, `0` only for a valid one
  (`tos-api/src/bios/mod.rs:237-243`, matching `blkdev.c:789-790`). This was the one observable
  deviation; it no longer diverges.]**
- **No `forcechange` after a sector-0 write — [BIOS02] measured DIFF, not a
  candidate any more.** Oracle W02 = 2, W05 = 2, X13 = 2; NewTOS 0 in all
  three. What must be reproduced: one `forcechange` flag per valid logical
  drive, armed by `Rwabs` (`04-rwabs.md` step 6), returned as `2` here
  **without** the "SD is non-removable" shortcut, cleared by `Getbpb` on
  the same drive (`bios/blkdev.c:590`). The "non-removable ⇒ constant 0"
  model of this section holds only while nobody writes sector 0.
- **Latch/reset pair absent.** With `Mediach`≡0 there is no latch for `Getbpb`
  to clear; the `Getbpb`→`Mediach` reset handshake is a no-op. (Measured:
  W06/W07 and X15/X16 are EQUAL only because NewTOS never left 0.)
- **`MEDIAMAYCHANGE` (1) is never produced** — the Pi SD path has no "same card
  reinserted?" ambiguity, so only `0` is ever returned. (If USB-MSC hot-plug
  drives land in a later phase, they would reintroduce the full 3-state latch.)

### Measured cross-check — `bios-r2.tsv` cases per statement [BIOS02]

Oracle `tosdump/reference/EMUTOS-1a7484ab-BIOS-r2-disk.BIN`; NewTOS run
`measurements/measurement-2026-10-28-tool07/RUNINFO.TXT`. Dumper no. = line of
the row in `tosdump/cases/bios-r2.tsv`.

| Statement | Cases (label : dumper no.) | EmuTOS | NewTOS | Verdict |
|-----------|----------------------------|--------|--------|---------|
| valid, settled drive → 0 | `Mediach(2)` 50; W07 98; W10 102; X16 109 | 0 | 0 | EQUAL |
| absent / out-of-range drive → `EUNDEV` | `Mediach(5/15/16/-1)` 51-54 | -15 | -15 | EQUAL |
| **after sector-0 write: 2, repeated** | W02 93, W05 96, X13 106 | 2 | 0 | **DIFF** (BIOS-STOR-001) |
| `Getbpb` clears it | W06 97 → W07 98; X15 108 → X16 109 | 0 | 0 | EQUAL (vacuous on NewTOS) |

Not measured: `MEDIAMAYCHANGE`, driver-error pass-through, the
`mediachange` latch from a probe, the 0.5 s `last_access` rule
(`bios/disk.c:389`, would need a removable unit), A:/B: (`Mediach(0/1)` is a
device parameter, audit §8).
