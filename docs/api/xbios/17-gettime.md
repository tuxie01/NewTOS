# `Gettime` (XBIOS 0x17)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/clock.c:1266` (`gettime`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the LONG result, the DOS-packed encoding (date high,
> time low). Reference: the RTC backend chain and the `DEFAULT_DATETIME`
> fallback.

## EmuTOS source

**Primary location:** `bios/clock.c:1266` — `LONG gettime(void)`, slot 0x17
of `xbios_vecs[]` (`bios/xbios.c:1313`). GEMDOS boot seeds `current_date`/
`current_time` from this (`time_init`, `bdos/time.c:160-165`;
`#define Gettime() xbios_l_v(23)`, `include/xbiosbind.h:41`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called (RTC backends, one per build):**
- `bios/clock.c` — `ngetdt`/`mgetdt`/`monstergetdt`/`icdgetdt`/
  `ultrasatan_getdt`/`igetdt` — read the RTC hardware
  (`bios/clock.c:1284-1321`).

**Headers / type definitions used:**
- `include/portab.h` — `MAKE_ULONG(hi, lo)`.
- `bios/ikbd.h:95` — `DEFAULT_DATETIME = MAKE_ULONG(os_header.os_dosdate, 0)`.

**System variables / globals read:**
- the RTC hardware, or `os_header.os_dosdate` (the fallback build date).

**System variables / globals written:** none.

**Build configuration affecting this function:**
- `MACHINE_AMIGA`/`MACHINE_LISA`, `CONF_WITH_NVRAM`/`MEGARTC`/`MONSTER`/
  `ICDRTC`/`ULTRASATAN_CLOCK`/`IKBD_CLOCK` — which RTC is read. With none,
  returns `DEFAULT_DATETIME`.

---

## Source extraction summary

`Gettime` returns the current date and time as a 32-bit DOS-format value:
the date in the high word, the time in the low word (via `MAKE_ULONG(date,
time)`). It reads whichever RTC the machine has, checked in the same order as
`Settime`; if no RTC backend is compiled in it returns `DEFAULT_DATETIME`,
which is the OS-header build date (`os_header.os_dosdate`) with a zero time
(midnight).

At boot, GEMDOS `time_init` calls `Gettime()` once to seed the cached
`current_date`/`current_time` globals; thereafter GEMDOS advances those from
the 200 Hz timer and `Tgetdate`/`Tgettime` read the cache, not the RTC. So
`Gettime` is chiefly the boot-time and RTC-direct reader.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x17
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0017
  ```
  No parameters (`Gettime()` = `xbios_l_v(23)`, `include/xbiosbind.h:41`).
- **Total parameter bytes:** 2.
- **Native (AArch64) ABI:** ID `0x17` in `x8`; no argument registers; the
  LONG result **sign-extended** into `x0` (a packed 32-bit datetime).

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
- **Type:** LONG
- **Encoding:** `MAKE_ULONG(date, time)`:
  - high word (date): bits 0-4 day, 5-8 month, 9-15 year-since-1980.
  - low word (time): bits 0-4 double-seconds, 5-10 minute, 11-15 hour.
- **Source location:** `bios/clock.c:1319` (fallback) or the RTC backend read.

### Side effects on global state / hardware

None (a pure read).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| No RTC backend present | `bios/clock.c:1319` | `DEFAULT_DATETIME` (build date, midnight) |

There is no error **code**; a missing RTC returns the build-date fallback, not
an error.

---

## Verification criteria

- **Round-trips with `Settime`:** after `Settime(v)`, on an RTC build
  `Gettime()` returns a value whose fields match `v` (subject to RTC
  granularity).
- **Fallback:** on a no-RTC build, `Gettime()` returns `DEFAULT_DATETIME`
  (the OS build date with time 0), not garbage.
- **Encoding:** the high word is the date, the low word is the time
  (`(Gettime() >> 16)` is a valid packed date; `(Gettime() & 0xFFFF)` a
  packed time).

---

## Edge cases discovered in source

1. **`DEFAULT_DATETIME` fallback.** With no RTC (and no IKBD clock),
   `Gettime` returns `MAKE_ULONG(os_header.os_dosdate, 0)`
   (`bios/clock.c:1319`, `bios/ikbd.h:95`) — the build date at midnight, so
   time is deterministic but stuck until set.
2. **Boot seed only.** GEMDOS reads `Gettime` once at `time_init`
   (`bdos/time.c:160`); afterwards the GEMDOS cache is authoritative and
   advances on the timer, so `Gettime` and `Tgettime` can diverge if the RTC
   drifts from the cache.
3. **Same packing as GEMDOS.** The date/time bit layout is identical to
   `current_date`/`current_time` and to FAT directory timestamps, so no
   conversion is needed across the boundary.

---

## Dependencies

### Requires (must be initialized / functional)

- An RTC backend (optional) or `os_header.os_dosdate` for the fallback.

### Used by

- GEMDOS `time_init` (boot seed), time-display utilities, RTC readers.

---

## Related data structures

- `docs/api/gemdos/_state.md §12` — the cache globals + packing.
- `docs/api/gemdos/_state.md §9` — `os_header.os_dosdate` (the fallback seed).
- `docs/api/xbios/16-settime.md` — the writer.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the packed
  datetime return.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- The RTC read depends on the machine/build; the packed encoding is constant.
  Falcon NVRAM uses a dynamic year base handled inside the backend.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07: `Gettime` reads the kernel clock (AArch64 generic timer + an RTC
abstraction: PL031 on QEMU, an SD/config-persisted or I²C RTC on RPi4). The
`DEFAULT_DATETIME`-style fallback (a fixed build date) applies when no RTC is
configured; the packed encoding is contract.

---

## Pi / BCM2711 binding

> Re-points the clock source from the **RTC chain / `DEFAULT_DATETIME`
> fallback** to the **seeded software epoch** shared with GEMDOS. The LONG
> result and the packed `date<<16 | time` encoding are unchanged. See
> `docs/HARDWARE-PROFILE.md` §2 (RTC row), §3.5.

### `Gettime` reads the same seeded epoch as GEMDOS

`Gettime` (XBIOS 0x17) is dispatched (`tos-api/src/xbios/mod.rs:99`) to
`gemdos::time::xbios_gettime` (`tos-api/src/gemdos/time.rs:220-223`), which reads
`now = BASE_SECS + CNTPCT/CNTFRQ`, decomposes it, and returns
`(pack_date << 16) | pack_time` — the EmuTOS `MAKE_ULONG(current_date,
current_time)` fallback path, but over the seeded epoch instead of an RTC. It is
a pure read (no side effects), matching the contract.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| Reads whichever RTC the machine has | reads `BASE_SECS + monotonic` (no RTC) |
| No-RTC fallback = `DEFAULT_DATETIME` (build date, midnight) | the same: `time_init` starts the clock at `DEFAULT_DATETIME` at boot, then it advances by the monotonic delta (like the IKBD clock, `bios/clock.c:1211`) |
| `Gettime`/`Tgettime` can diverge (RTC drifts from the GEMDOS cache) | **cannot diverge** — both read the *same* `BASE_SECS`+monotonic clock; there is no separate cache to drift |

### Note: no cache/RTC split

On EmuTOS `Gettime` reads the RTC while `Tgettime` reads a GEMDOS cache that the
200 Hz tick advances, so the two can drift. NewTOS has a **single** clock
(`BASE_SECS` + generic-timer delta), so `Gettime`, `Tgetdate`, and `Tgettime`
are always consistent — a benign simplification of the contract.

### Boot value: the build date at 00:00:00 (TASK/XBIOS12)

`DEFAULT_DATETIME` (`tos-api/src/gemdos/time.rs:237@DEFAULT_DATETIME`) is
`(OS_DOSDATE << 16) | 0`: the kernel's build date GEMDOS-packed, the same
`crate::build::OS_DOSDATE` the loader writes into the OSHEADER's `os_dosdate`
(+0x1E), time 00:00:00 — EmuTOS's `MAKE_ULONG(os_header.os_dosdate, 0)`
(`bios/ikbd.h:95`). The build date comes from `tos-api/build.rs` (the build
day in UTC, pinned by `SOURCE_DATE_EPOCH` when set, so a reproducible build
stays reproducible). The kernel calls `time_init`
(`tos-api/src/gemdos/time.rs:247@time_init`, EmuTOS `time_init`,
`bdos/time.c:160-165`) once at boot, right after the host is installed, and
logs `[clock] no RTC: clock starts at the build date YYYY-MM-DD 00:00:00`
(`kernel/src/lib.rs:1310@log_boot_clock`). So `Gettime()` right after boot
reads `os_dosdate << 16` plus the seconds since that call. Until 2026-10-31
the boot value was 1980-01-01, or on the Pi board the FAT date of
`C:\KERNEL8.IMG`; EmuTOS has no such source, and it is gone.

### Gaps vs. the contract (Step-2 candidates)

- **No RTC read, no persistence** — every boot starts at the build date.
  Persistence is the same item as `Settime` (an RTC backend, TASK/RTC01;
  `docs/COMPATIBILITY.md`, "No RTC").
