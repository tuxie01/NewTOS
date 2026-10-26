# `Tickcal` (BIOS 0x06)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1096` (`tickcal`).

## EmuTOS source

**Primary location:** `bios/bios.c:1096` — `LONG tickcal(void)`
(body: `return 20L;`).

**Helpers called:** none.

**Headers / type definitions used:** none.

**System variables / globals read/written:** none — the return is a literal
constant `20L`.

**Build configuration affecting this function:** none. The constant does not
vary with any `CONF_*` flag; it is hard-coded even if the underlying timer
were reconfigured.

---

## Source extraction summary

`Tickcal` returns the period, in milliseconds, of the system tick that drives
GEMDOS/AES timing. On Atari hardware the 200 Hz timer-C interrupt runs the
50 Hz "tick" processing (`init_system_timer`), and one 50 Hz tick is 20 ms —
hence the literal `20L`. The value is a constant, unrelated to any runtime
measurement; even the `_hz_200` counter (which ticks at 200 Hz) is a separate
mechanism (`Tickcal` is not derived from it).

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x06
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0006
  ```
- **Total parameter bytes:** 2 (just the ID)
- **Native (AArch64) ABI:** ID `0x06` in `x8`; no arguments; result `20`
  sign-extended into `x0`.

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
- **Encoding:** the integer `20` (milliseconds per system tick).
- **Source location:** `bios/bios.c:1098`.

### Side effects on global state

None.

### Side effects on hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none) | `bios/bios.c:1096` | always `20L` |

---

## Verification criteria

- **Nominal:** `Tickcal()` returns exactly `20` (decimal), with no inputs and
  no side effects, on every call.

---

## Edge cases discovered in source

1. **Pure constant — never reflects real hardware.** It is `20L` regardless of
   the actual timer rate. A build with a different tick rate would still report
   20; the value is an ABI constant, not a measurement.

2. **Not related to `_hz_200`.** Programs that want elapsed time read the
   200 Hz counter `_hz_200` (`0x4ba`) directly and convert with `×5` ms/tick;
   `Tickcal` is only the *GEMDOS/AES* 50 Hz tick period, used e.g. for
   `evnt_timer` scaling. Confusing the two is a common documentation error.

---

## Dependencies

### Requires (must be initialized / functional)

- None (constant). The associated 50 Hz processing is enabled later
  (`timer_c_sieve`), but `Tickcal`'s return does not depend on it.

### Used by

- AES `evnt_timer` period computation; user programs converting ticks → ms.

---

## Related data structures

- `docs/api/bios/_state.md` — `_hz_200` system-tick counter (distinct
  mechanism)

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: "period
  of the system timer in ms" = 20.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

None. `20L` across all EmuTOS builds and TOS versions.

---

## NewTOS implementation notes

(Empty during Phase 00.)

NewTOS keeps returning `20` for ABI purposes irrespective of its internal
tick rate, so timing-conversion code in ported programs stays correct.

---

## Pi / BCM2711 binding

> Re-points the timing reference from the **MFP 68901 timer-C 200 Hz interrupt**
> to the **ARM generic timer** (`CNTPCT_EL0` / `CNTFRQ_EL0`). The contract — a
> constant `20` (ms per GEMDOS/AES tick) — is unchanged; only the underlying
> clock is re-pointed here. See `docs/HARDWARE-PROFILE.md` §3.5.

### `Tickcal` is a pure constant `20`

`Tickcal` (BIOS 0x06) returns `TICKCAL_MS = 20` unconditionally
(`tos-api/src/bios/mod.rs:41,54`), matching the EmuTOS `return 20L`. It is an
ABI constant and is **not** derived from the real timer, so it is truthful
regardless of the actual `CNTFRQ`.

### The timer source: ARM generic timer, not the MFP or the BCM system timer

NewTOS reads time from the **ARMv8 generic timer** — the EL0-readable system
registers `CNTPCT_EL0` (64-bit physical count) and `CNTFRQ_EL0` (its frequency
in Hz), e.g. `kernel/src/host_env.rs:149-166` (`monotonic_seconds`),
`:168-189` (`monotonic_millis`), `:191-206` (`frame_clock`), and
`kernel/src/arch/aarch64/mod.rs:24-48` (`delay_ms`). The frequency is **read at
runtime from `CNTFRQ_EL0`**, never hardcoded — so no board-specific Hz value is
baked in (on a Pi-4 `CNTFRQ` is the crystal-derived generic-timer frequency,
typically 54 MHz, but the code never assumes it).

> **Datasheet note.** The ARM generic timer is an **ARMv8 CPU feature**
> (ARM ARM D17.6 "Generic Timer"), *not* a BCM2711 peripheral, so it is **not**
> in the BCM2711 peripherals datasheet. The datasheet's own "System Timer"
> (Ch. 10, p.142-143; phys base `0x7e003000`, a 1 MHz 64-bit free-running
> counter) and the SP804-derived "ARM Timer" (Ch. 12) are **not** used by
> NewTOS for the TOS clock. If a future phase needs a memory-mapped counter
> (e.g. before the CPU timer is trusted), the System Timer at `0x7e003000` is
> the datasheet-sourced option.

### The 200 Hz / 20 ms tick derivation

| Rate | Value | How NewTOS models it |
|------|------:|----------------------|
| `Tickcal` (GEMDOS/AES tick period) | **20 ms** (50 Hz) | constant (`tos-api/src/bios/mod.rs:77`) |
| `_hz_200` (200 Hz free-running counter) | 5 ms/tick | **materialised** — a real 200 Hz interrupt (ARM generic timer, EL1 physical timer, PPI INTID 30) increments the counter and mirrors it into the running process's `$4BA` (`kernel/src/drivers/timer.rs`, `tos-api/src/bios/tick.rs`) |
| Frame clock (`Vsync`, ~60 Hz) | `CNTPCT*60/CNTFRQ` | `host_env.rs:191-206` |
| Millisecond clock (AES `evnt_timer`) | `CNTPCT*1000/CNTFRQ` | `host_env.rs:168-189` |

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| MFP timer-C fires 200 Hz IRQ; the ISR runs 50 Hz "tick" work and advances `_hz_200` | ARM generic timer (EL1 physical, PPI INTID 30) fires a 200 Hz IRQ; the ISR advances `_hz_200` — the 50 Hz half of the sieve is not run in the ISR: key repeat runs polled in the input pump off `_hz_200` (KEYS01), `dosound` and `etv_timer` are still not run |
| `Tickcal` = 20 (constant, unrelated to the timer) | `Tickcal` = 20 (constant) — identical |

### The 200 Hz tick (implemented 2026-09-04)

`kernel/src/drivers/timer.rs` programs the **ARM generic timer's EL1 physical
timer** (`CNTP_CVAL_EL0` / `CNTP_CTL_EL0`) to a 5 ms period; its interrupt is
the non-secure physical timer PPI, **INTID 30** on both boards (BCM2711
datasheet §6.3 Figure 7 "Core n PNS timer IRQ — PPI ID 30"; QEMU `virt`'s own
device tree `/timer` = `GIC_PPI 14` → 16 + 14 = 30). `drivers::gic` unmasks it
via `enable_ppi` and dispatches it from `rust_irq_handler`; the lower-EL IRQ
vector (`0x480`) now routes to the same handler, so the tick (and the PL011 RX
IRQ) is delivered while an EL0 program runs instead of hitting the halting
catch-all.

`tos_api::bios::tick` holds the counter and mirrors it into the **running**
process's `$4BA` cell in its per-process low-memory sysvar area, so
`*(volatile long *)0x4ba` is live for an app. Two deliberate deviations from
EmuTOS, both documented in that module: the handler credits *every* elapsed
5 ms period (so a late interrupt — IRQs are masked inside the synchronous SVC
handler — cannot lose time, where m68k timer C does), and the **50 Hz half of
the `timer_c_sieve`** (`bios/vectors.S:521`) is not run in the interrupt:
key repeat (`kb_timerc_int`) runs polled in the input pump, stepping on the
multiples of 4 of `_hz_200` (KEYS01, 2026-10-03; `docs/api/xbios/_state.md`
§"Key-repeat handler"), while `dosound` and `etv_timer` are still not run.

### Gaps vs. the contract (Step-2 candidates)

- **No 50 Hz tick processing in the interrupt.** EmuTOS' timer-C ISR runs
  `kb_timerc_int`, `sndirq` and the `etv_timer` hook on every 4th interrupt.
  NewTOS runs the first polled in the input pump (KEYS01), has no equivalent of
  the second and never dispatches the third, so a program that
  installs an `etv_timer` hook expecting 20 ms callbacks still gets **nothing**
  — see the `05-setexc.md` binding.
- **`Random`'s `hz_200` auto-seed** still falls back to the frame clock,
  although `hz_200` now exists — see `docs/api/xbios/11-random.md`.
