# `Tsetdate` (GEMDOS 0x2B)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/time.c:108` (`xsetdate`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the packed-date WORD input encoding, the
> upper-bound validation and the single `ERR` (-1) error, that a valid set
> both updates the GEMDOS date and persists to the RTC. Reference: the
> `nday_*` tables, the `IS_A_LEAP_YEAR` shortcut, the XBIOS/`settime()`
> internals.

## EmuTOS source

**Primary location:** `bdos/time.c:108` — `long xsetdate(UWORD d)`. Validates
the packed date's upper bounds, then forwards to XBIOS `Settime`.

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x2B` =
`xsetdate`, `stdio_typ = 0`, `wparms = 1` (`bdos/bdosmain.c:233`).

**Helpers called:**
- `include/xbiosbind.h:40` — `Settime(a)` = `xbios_v_l(22, a)` — **XBIOS
  function 22 (0x16)**, not a BIOS call. Reaches `settime()`
  (`bios/clock.c:1218`).
- `bios/clock.c:1218` — `settime(LONG time)` — writes `current_time`/
  `current_date` from the packed LONG, then persists to whatever RTC is
  present (NVRAM / MegaRTC / MonSTer / ICD / UltraSatan / IKBD clock).

**Headers / type definitions used:**
- `bdos/time.c:58-74` — field masks/shifts (`DAY_BM=0x1F`, `MTH_BM=0x1E0`,
  `MTH_SHIFT=5`, `YRS_SHIFT=9`) + `IS_A_LEAP_YEAR(a) = !(a & 0x0600)`.
- `bdos/time.c:87-88` — `nday_norm[13]` / `nday_leap[13]` days-per-month
  tables (index 0 = 0).
- `include/portab.h:139` — `MAKE_ULONG(hi, lo)` — packs date (hi) + time (lo).
- `include/gemerror.h` — `ERR` (-1), `E_OK` (0).

**System variables / globals read:**
- `current_time` (`bdos/time.c:53`) — repacked with the new date so the RTC's
  time half is preserved.

**System variables / globals written (via `settime()`):**
- `current_date`, `current_time` (`bios/clock.c:1221-1222`) — updated directly
  by the XBIOS-level `settime()`.

**Build configuration affecting this function:** `xsetdate` itself is
config-independent. Which RTC `settime()` persists to depends on
`CONF_WITH_NVRAM` / `CONF_WITH_MEGARTC` / `CONF_WITH_MONSTER` /
`CONF_WITH_ICDRTC` / `CONF_WITH_ULTRASATAN_CLOCK` / `CONF_WITH_IKBD_CLOCK`.
The Atari standard/default path is the IKBD clock (`isetdt`,
`bios/clock.c:1067`).

---

## Source extraction summary

`Tsetdate` sets the system date from a FAT/DOS-packed WORD (epoch 1980). It
extracts month and day from the word, selects the leap or normal
days-per-month table via `IS_A_LEAP_YEAR`, and validates **upper bounds
only**: year-offset > 119 (post-2099), month > 12, or day > days-in-month all
return `ERR` (-1). Nothing else is checked.

On success it calls XBIOS `Settime` with the new date packed against the
**current** time (`MAKE_ULONG(d, current_time)`). At the XBIOS level,
`settime()` (`bios/clock.c:1218`) both updates the GEMDOS globals
`current_date`/`current_time` directly and writes the value out to the RTC
hardware present on the machine — so a successful `Tsetdate` **persists** (it
is not merely a RAM update). On the standard Atari path with no add-on RTC,
that persistence is the IKBD clock (`isetdt`).

The leap-year test is a shortcut: `IS_A_LEAP_YEAR(d) = !(d & 0x0600)` is true
when bits 10-9 of the packed word are both zero, i.e. the year-offset is
divisible by 4. Over the valid 1980-2099 window this is exactly correct (no
century year in range breaks the /4 rule); the Gregorian 100/400 exceptions
never apply because 2100 (offset 120) is already rejected.

**Note (validation gap, D-007):** because the day check is
`day > nday[curmo]` and `nday[0] == 0`, the value `day == 0` passes for any
month, and `month == 0` passes the `curmo > 12` check — so EmuTOS **accepts
`(year, 0, 0)` and `(year, m, 0)`**, committing an invalid date. See Edge
case 1.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x2B
- **Stack frame on entry (m68k):**
  ```
  +2: d  (word)  — packed date
   0: function_id (word) = 0x002B
  ```
- **Total parameter bytes:** 4 (ID + WORD) → `wparms = 1`.
- **Native (AArch64) ABI:** ID `0x2B` in `x8`; the packed-date WORD in `x0`;
  LONG result (`E_OK` or `ERR`) **sign-extended** into `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | +2         | `x0`        | `d`  | `UWORD` (16) — packed date |

---

## Inputs

| Position | Name | Type    | Size | Source location   | Notes |
|----------|------|---------|------|-------------------|-------|
| arg0     | `d`  | `UWORD` | 2    | `bdos/time.c:108` | packed date (same encoding as `Tgetdate`) |

### Packed date WORD encoding

| Bits  | Field | Validated range        | Rejected when                       |
|------:|-------|------------------------|-------------------------------------|
| 15-9  | year  | 0-119 (1980-2099)      | `d >> 9 > 119`                      |
| 8-5   | month | (1-12 intended)        | `(d & 0x1E0) >> 5 > 12`             |
| 4-0   | day   | (1-days-in-month)      | `(d & 0x1F) > nday[month]`         |

(Lower bounds `month >= 1` / `day >= 1` are **not** enforced — see Edge case 1.)

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (GEMDOS error code)
- **Encoding:** `E_OK` (0) on success; `ERR` (-1) on any range failure.
- **Source location:** `bdos/time.c:119` (`ERR`), `bdos/time.c:124` (`E_OK`).

| Case                                     | Return       |
|------------------------------------------|--------------|
| Date accepted, GEMDOS + RTC updated      | `E_OK` (0)   |
| Year offset > 119 (post-2099)            | `ERR` (-1)   |
| Month > 12                               | `ERR` (-1)   |
| Day > days-in-month for that month       | `ERR` (-1)   |

### Side effects on global state

- On success, `current_date` and `current_time` are updated
  (`bios/clock.c:1221-1222`, via `Settime`).

### Side effects on hardware

- On success, the new date is written to the machine's RTC (`settime()`
  branches on the detected clock: NVRAM/MegaRTC/MonSTer/ICD/UltraSatan/IKBD).
  On a failed range check the function returns before `Settime`, so **no**
  hardware or global state changes.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| Year offset > 119 (year after 2099) | `bdos/time.c:116` | `ERR` (-1) |
| Month field > 12 | `bdos/time.c:117` | `ERR` (-1) |
| Day field > days-in-month (leap-aware) | `bdos/time.c:118` | `ERR` (-1) |

`ERR` (-1) is the **only** error — not `EBADRQ`/`EINVFN`. It is the literal
generic error from `include/gemerror.h`.

**Error paths not in Compendium:** none extra; but the *absence* of a lower
-bound check means two invalid dates are **accepted** rather than rejected
(D-007), which is a behaviour difference the other direction.

---

## Verification criteria

- **Nominal:** `Tsetdate(0x5CE3)` (year offset 46 = 2026, month 7, day 3)
  returns `E_OK` (0); afterwards `Tgetdate()` returns `0x00005CE3` and the RTC
  holds the new date.
- **Leap-day accepted:** `Tsetdate` for 2024-02-29 (offset 44 → `!(0x5800 &
  0x0600)`? offset 44 is divisible by 4, so leap) packs to
  `(44 << 9)|(2 << 5)|29 = 0x585D`; returns `E_OK`.
- **Non-leap Feb 29 rejected:** 2025-02-29 (offset 45, not /4) →
  `(45 << 9)|(2 << 5)|29 = 0x5A5D`; day 29 > `nday_norm[2]=28` → `ERR` (-1),
  no state change.
- **Post-2099 rejected:** any word with year field 120-127 → `ERR` (-1).
- **Month 13 rejected:** month field 13 → `ERR` (-1).
- **EmuTOS quirk (D-007):** `(year, 0, 0)` and `(year, m, 0)` return `E_OK`
  under EmuTOS — NewTOS **diverges** and returns `ERR` (adds `month >= 1 &&
  day >= 1`); a test must assert the NewTOS behaviour, not the EmuTOS one.

---

## Edge cases discovered in source

1. **Day 0 / month 0 accepted (D-007).** The checks are upper-bound only.
   `day > nday[curmo]` with `nday[0] == 0` lets `day == 0` pass for any month,
   and `month == 0` passes `curmo > 12`. So `(year, 0, 0)` and `(year, m, 0)`
   are **accepted** and committed as invalid dates (`bdos/time.c:118`,
   `bdos/time.c:87-88`). NewTOS adds explicit lower-bound checks — see D-007.

2. **Persists to the RTC, not just RAM.** Unlike a naive "cache write",
   `Settime` → `settime()` writes the date to the real clock hardware
   (`bios/clock.c:1228-1263`). A successful `Tsetdate` therefore survives the
   XBIOS round-trip and is durable across the RTC read at next boot.

3. **Repacks with the *current* time — one-tick race.** `MAKE_ULONG(d,
   current_time)` reads `current_time` after validation; a 200 Hz tick between
   the read and the `Settime` call can make the committed time up to one tick
   (5 ms) stale. EmuTOS does not guard this window; the effect is benign.

4. **Leap-year shortcut is exact in range.** `!(d & 0x0600)` (year-offset /4)
   correctly flags 2000/2004/…/2096 as leap; the Gregorian century exceptions
   (2100 non-leap) never matter because offset ≥ 120 is rejected first.

5. **Year 0 (= 1980) is valid.** The bound is `> 119`, so offset 0 passes.

---

## Dependencies

### Requires (must be initialized / functional)

- XBIOS `Settime` (22 / 0x16) → `settime()` → an RTC path (or the IKBD-clock
  fallback). See `docs/api/xbios/` (Settime) once written.
- `current_time` global (read for repacking).
- `nday_norm` / `nday_leap` tables.

### Used by

- Shell `DATE` command, AES control panel, RTC-resync utilities.

---

## Related data structures

- `docs/api/gemdos/_state.md` §12 — `current_date`/`current_time` globals + RTC
  path.
- `docs/api/gemdos/time/2a-tgetdate.md` — the getter (same encoding).

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: "sets the
  date; returns 0 on success, non-zero if the date is invalid." Compendium
  implies day/month 0 are invalid — see D-007.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- The packed encoding and the `> 119` / `> 12` / days-in-month checks are
  invariant across TOS versions and EmuTOS builds.
- The RTC target of the persist step varies by `CONF_WITH_*RTC*`/
  `CONF_WITH_IKBD_CLOCK`. The Atari standard path is the IKBD clock.

---

## NewTOS implementation notes

(Empty during Phase 00.)

Apply the `month >= 1 && day >= 1` fix (D-007) in PHASE-07. ABI-visible only
for programs that incidentally relied on `(0,0)` succeeding (none known); they
see `ERR`, consistent with Compendium's "valid date required". NewTOS drives
persistence through its own RTC abstraction (PL031 on QEMU; a software clock on
RPi4), replacing the IKBD-clock write.

---

## Pi / BCM2711 binding

> Re-points the commit target from the **RTC (via XBIOS `Settime`) + the
> `current_date` cache** to the **seeded software epoch**. The validation
> contract (`year_off > 119`, `month` 1-12, day ≤ days-in-month → `ERROR`;
> `E_OK` otherwise) is unchanged; only where the date is stored is re-pointed.
> See `docs/HARDWARE-PROFILE.md` §2 (RTC row), §3.5.

### `Tsetdate` recomputes `BASE_SECS`

`Tsetdate` (0x2B) validates the packed date (`time.rs:150-163`, applying the
D-007 `month >= 1 && day >= 1` fix), then recomputes the base epoch so the
clock reads the requested date at the current time-of-day:
`BASE_SECS = compose(...) - CNTPCT/CNTFRQ` (`time.rs:164-167`). No RTC write;
nothing persists across boots. NewTOS reads `now_secs()` **once** to get the
current time-of-day, so it has no separate `current_time`-vs-`Settime` tick race.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| Validated, then committed through XBIOS `Settime` → `current_date` + RTC | validated, then `BASE_SECS` recomputed (the single shared clock) |
| Persists in the RTC (build-dependent) | **not persisted** — resets to the build date (`DEFAULT_DATETIME`) each power-up unless re-set |

### Gaps vs. the contract (Step-2 candidates)

- **No persistence** — shared item with `Settime`/`Gettime` (needs an RTC
  backend). The validation and `E_OK`/`ERROR` returns are fully implemented.
