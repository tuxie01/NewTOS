# `Tgetdate` (GEMDOS 0x2A)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/time.c:99` (`xgetdate`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the packed-date WORD bit layout (1980 epoch), the
> zero-extend-to-LONG return shape. Reference: the `current_date` global and
> the BIOS clock path that maintains it.

## EmuTOS source

**Primary location:** `bdos/time.c:99` — `long xgetdate(void)`. The body is a
single `return current_date;` — a pure read of the cached GEMDOS date word,
zero-extended to LONG. No I/O, no validation.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x2A` =
`xgetdate`, `stdio_typ = 0`, `wparms = 0` (`bdos/bdosmain.c:232`).

**Helpers called:** none.

**Headers / type definitions used:**
- `bdos/time.c:53` — `current_date` (`UWORD`) declaration.
- `bdos/time.c:58-71` — date/time field masks + shifts (`DAY_BM`, `MTH_BM`,
  `YRS_BM`, `MTH_SHIFT`, `YRS_SHIFT`).

**System variables / globals read:**
- `current_date` (`bdos/time.c:53`) — the packed GEMDOS date WORD.

**System variables / globals written:** none.

**Build configuration affecting this function:** none. The value of
`current_date` depends on the RTC path (see Dependencies), but `xgetdate`
itself is configuration-independent.

---

## Source extraction summary

`Tgetdate` returns the current system date as a FAT/DOS-packed WORD, epoch
1980. EmuTOS keeps this in the file-local global `current_date`, a `UWORD`
that is initialised at boot from the RTC (`time_init`, `bdos/time.c:160`,
reads `Gettime()`), advanced by the 200 Hz tick handler `tikfrk` on midnight
rollover (`bdos/time.c:175-230`), and overwritten whenever the date is set via
XBIOS `Settime` (which the `settime()` C function applies directly, see
`bios/clock.c:1221-1222`). `xgetdate` performs none of that work — it is a
bare read of the cached word.

The C function is declared `long` and returns the `UWORD` directly, so the
compiler **zero-extends** the 16-bit value into the 32-bit result: the high
16 bits are always zero. There is no error path.

The packed layout is the standard MS-DOS/FAT directory date: bits 0-4 day
(1-31), bits 5-8 month (1-12), bits 9-15 year-offset-from-1980 (0-127). It is
the same encoding written into FAT directory entries and reported in the DTA
(`docs/api/gemdos/_state.md` §6). Note EmuTOS `extract_date`
(`bios/clock.c:74-79`) treats the year as `date >> 9` (a full 7-bit field) and
the month as `(date >> 5) & 0x0f` (4 bits) — see Edge case 3 for the month-field
width nuance.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x2A
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x002A
  ```
- **Total parameter bytes:** 2 (just the ID) → `wparms = 0`.
- **Native (AArch64) ABI:** ID `0x2A` in `x8`; no arguments; the packed-date
  WORD is returned **zero-extended** into `x0` (bits 16-63 = 0).

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (a zero-extended packed-date UWORD)
- **Encoding:** MS-DOS/FAT packed date (see table); high 16 bits are zero.
- **Source location:** `bdos/time.c:101`.

### Packed date WORD encoding

| Bits  | Field | Range / meaning                                  |
|------:|-------|--------------------------------------------------|
| 15-9  | year  | offset from 1980 (0 ⇒ 1980, 119 ⇒ 2099, max 127 ⇒ 2107) |
| 8-5   | month | 1-12                                             |
| 4-0   | day   | 1-31                                             |

### Side effects on global state

None.

### Side effects on hardware

None — the value is read from a cached global, not from the RTC.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none) | `bdos/time.c:99` | always the packed `current_date` |

`xgetdate` never fails and never validates. Whatever value `current_date`
holds (even an out-of-range one placed there by a lax `Tsetdate` — see D-007)
is returned verbatim.

---

## Verification criteria

- **Nominal:** with `current_date` representing 2026-07-03 (year offset
  `46 = 2026-1980`, month `7`, day `3`), the packed word is
  `(46 << 9) | (7 << 5) | 3 = 0x5CE3`. `Tgetdate()` returns `0x00005CE3` in
  `x0` (high 16 bits zero), no side effects.
- **Epoch:** for 1980-01-01 the packed word is `(0 << 9)|(1 << 5)|1 = 0x0021`;
  `Tgetdate()` returns `0x00000021`.
- **Round-trip:** `Tsetdate(w)` (with a valid `w`) followed by `Tgetdate()`
  returns the same `w` (the two share `current_date`).
- **No side effects:** two successive `Tgetdate()` calls with no intervening
  tick return identical values and leave all state unchanged.

---

## Edge cases discovered in source

1. **No I/O on the read.** `xgetdate` reads the cached `current_date` only; it
   never queries the RTC. The cache is advanced by `tikfrk` on midnight
   rollover (`bdos/time.c:213-229`) and set by `Tsetdate`/XBIOS `Settime`.

2. **High word guaranteed zero.** The `UWORD`→`long` return zero-extends, so a
   caller reading the full LONG sees `0x0000xxxx`. Contract for the AArch64 ABI
   (`x0` bits 16-63 are zero), not sign-extended.

3. **Month field is 4 bits in the reader.** `extract_date`
   (`bios/clock.c:77`) masks the month with `0x0f`, so a bit-8 spill would be
   lost when the RTC path re-derives the month. In practice `Tsetdate` never
   produces month > 12, so bit 8 stays 0; the packed layout nominally allots
   bits 5-8 (4 bits) to the month. `Tgetdate` returns the word unmasked.

4. **Pre-1980 not representable.** The year field is an unsigned offset from
   1980; dates before 1980 cannot be encoded.

5. **Beyond 2099 is representable but never set by `Tsetdate`.** The 7-bit
   year field reaches offset 127 (= 2107), but `Tsetdate` rejects offsets
   > 119 (2099) — "warranty expires 12/31/2099" (`bdos/time.c:116`). A value
   in 2100-2107 could only appear via direct RTC state or XBIOS `Settime`;
   `Tgetdate` returns it unchallenged.

---

## Dependencies

### Requires (must be initialized / functional)

- `current_date` global, initialised by `time_init` (`bdos/time.c:160`) from
  `Gettime()` (XBIOS 23) at boot.
- The 200 Hz tick / `tikfrk` handler (`etv_timer`) to advance the date across
  midnight.

### Used by

- `Fcreate`/`Fwrite` timestamping of FAT directory entries.
- `Fsfirst`/`Fsnext` DTA date field.
- `Fdatime` (0x57), shell `DATE`, AES control panel.

---

## Related data structures

- `docs/api/gemdos/_state.md` §12 — GEMDOS `current_date`/`current_time`
  globals and the RTC path.
- `docs/api/gemdos/time/2b-tsetdate.md` — the setter (same encoding).
- `docs/api/structures/dta.md` — the DTA `d_date` field uses this encoding.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: "returns
  the current date in GEMDOS/DOS packed format" — agrees with EmuTOS.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

### Atari ST Internals

- Packed-date layout is the standard MS-DOS/FAT format.

---

## Configuration / version notes

- The packed encoding is invariant across TOS versions and EmuTOS builds.
- The *source* of the boot-time value varies by RTC hardware
  (`CONF_WITH_NVRAM` / `CONF_WITH_MEGARTC` / `CONF_WITH_IKBD_CLOCK` / …); with
  no RTC, `Gettime()` falls back to `DEFAULT_DATETIME` (the OS-header build
  date, `bios/ikbd.h:95`, `bios/clock.c:1319`).

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07 plan: `current_date` is driven by an AArch64 generic-timer tick plus
an RTC abstraction (RPi4 has no battery-backed RTC by default; QEMU
`aarch64-virt` exposes a PL031). `Tgetdate` stays a cached read; the boot-time
default remains a build constant. NewTOS may add an atomic combined
`(date << 16) | time` snapshot accessor so multitasking readers cannot observe
a date/time torn across a midnight tick (see `2c-tgettime.md`).

---

## Pi / BCM2711 binding

> Re-points the date source from the **RTC-seeded `current_date` cache advanced
> by the 200 Hz `tikfrk` handler** to a **seeded software epoch** computed on
> demand from the ARM generic timer. The packed-date WORD encoding and the
> zero-extended LONG return are unchanged. See `docs/HARDWARE-PROFILE.md` §2
> (RTC row), §3.5.

### `Tgetdate` = pack_date of the seeded epoch

There is no RTC and no `current_date` cache advanced by a timer ISR. NewTOS
computes `now = BASE_SECS + CNTPCT/CNTFRQ`
(`tos-api/src/gemdos/time.rs:25-45`), decomposes it, and packs the date
(`tos-api/src/gemdos/time.rs:141@tgetdate`, `tos-api/src/gemdos/time.rs:122@pack_date`). At boot
`time_init` (`tos-api/src/gemdos/time.rs:247@time_init`, EmuTOS `time_init`,
`bdos/time.c:160`) sets the clock to `DEFAULT_DATETIME`, the kernel's build
date (the OSHEADER's `os_dosdate`) at 00:00:00, as EmuTOS does with no RTC
(`bios/ikbd.h:95`, `bios/clock.c:1319`; TASK/XBIOS12); it reads that until
set via `Tsetdate` / XBIOS `Settime`.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| `current_date` cache read (no I/O) | `pack_date(decompose(BASE_SECS + CNTPCT/CNTFRQ))` — computed, no cache |
| Cache seeded from RTC at boot, advanced by `tikfrk` on midnight | epoch advances continuously with the generic-timer count; no midnight-rollover handler needed |
| Boot default = `DEFAULT_DATETIME` (OS build date) | boot default = `DEFAULT_DATETIME` (kernel build date, 00:00:00) |

Leap rule: EmuTOS's simplified "every 4th year" (`time.rs:47-51`), exact over
1980..2099. Same packed encoding as the DTA and FAT directory entries.

### Gaps vs. the contract (Step-2 candidates)

- **No RTC / no persistence** — the clock starts at the build date on every
  power-up (shared item with `Settime`/`Gettime`; TASK/RTC01 adds an RTC).
- **No torn-read accessor** — `Tgetdate` and `Tgettime` are two separate reads
  of `now_secs()`; a reader can still observe a date/time split across a
  midnight second. An atomic combined snapshot is the recommended Step-2 add.
