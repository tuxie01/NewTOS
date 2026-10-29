# `Kbshift` (BIOS 0x0B)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/ikbd.c:145` (`kbshift`).
> [BIOS12] 2026-10-28: bit 5-7 statements, config note and NewTOS gaps corrected
> from `bios/ikbd.c:145-153,876-883` and the audit probes K01-K13
> (`measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`).

## NewTOS status: **implemented (wired at BIOS 0x0B)**

NewTOS handles the keyboard natively (USB-HID → `crate::ikbd::feed`); the
modifier state lives in the `ikbd` decoder. As of Task #24 `Kbshift` **is**
wired as an in-range dispatcher handler (`tos-api/src/bios/mod.rs`,
`kbshift`): the query path (`flag < 0`) returns
`host().keyboard_shift_state()` and the set path (`flag >= 0`) writes it back
through `host().set_keyboard_shift_state(flag as u8)`
(`kernel/src/ikbd.rs::set_shift_state`). `0x0B` is a real in-range function
id, so it dispatches to `kbshift` — it is **not** the dispatcher's
out-of-range fall-through (which returns the function number for ids
`> 0x0B` only). See `docs/STATUS.md` and the "Keyboard via Atari IKBD"
decision.

---

## EmuTOS source

**Primary location:** `bios/ikbd.c:145` — `LONG kbshift(WORD flag)`

**Helpers called:** none.

**Headers / type definitions used:**
- `bios/ikbd.h:30` — `extern UBYTE shifty;` (the modifier-state byte)
- `include/biosdefs.h:90-100` — `MODE_RSHIFT`/`MODE_LSHIFT`/`MODE_CTRL`/
  `MODE_ALT`/`MODE_SCA`/`MODE_CAPS`/`MODE_HOME`/`MODE_INSERT`
- `bios/ikbd.h:26,28` — `MODE_SHIFT`, `HOTSWITCH_MODE` (private combinations)

**System variables / globals read/written:**
- `shifty` (`bios/ikbd.c:147,150`) — a `UBYTE` maintained by the IKBD scancode
  ISR; also published via `os_kbshift` in the OSHEADER
  (`include/biosdefs.h:59`) and pointed to from `bios/startup.S:109`.

**Build configuration affecting this function:** none — `kbshift` itself is
configuration-independent. (Whether `shifty` is updated depends on the IKBD/
ACIA being present, i.e. `CONF_ATARI_HARDWARE`/keyboard init.)

---

## Source extraction summary

`Kbshift` gets or sets the keyboard modifier-state byte `shifty`. It saves the
current value, and — only if `flag >= 0` (a **signed** `WORD` test) — stores
`flag` into `shifty`; it always returns the saved (previous) value. So
`flag == -1` (0xFFFF, negative) is the "query only" idiom.

Note the width mismatch: the parameter is a `WORD` but `shifty` is a `UBYTE`.
Storing `flag` into `shifty` (`bios/ikbd.c:150`) **truncates to 8 bits** — any
value in bits 8..15 of `flag` is silently discarded. **All 8 bits are kept**,
incl. bits 5-7: measured `Kbshift(0x1FF)` then `Kbshift(-1)` = `0xFF`,
`Kbshift(0x7F81)` then `Kbshift(-1)` = `0x81` on EmuTOS (Sonde K04/K05,
K08/K09; the audit table lists `13:11 w:0x1FF` and `w:0x7F81` as EXACT). The return is
`WORD oldshifty = shifty;` (zero-extended from the byte), delivered as a LONG
by the dispatcher (upper bits 0).

`shifty` is updated in interrupt context by the IKBD scancode handler
(`bios/ikbd.c:780-839`): key-down sets a bit, key-up clears it; CapsLock is
toggled on key-down.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x0B
- **Stack frame on entry (m68k):**
  ```
  +2: flag (word)
   0: function_id (word) = 0x000B
  ```
- **Total parameter bytes:** 4
- **Native (AArch64) ABI:** ID `0x0B` in `x8`; `flag` in `x0`; the previous
  (zero-extended) modifier byte in `x0`.

  | m68k stack | AArch64 reg | Name   | Type / size |
  |------------|-------------|--------|-------------|
  | +2         | `x0`        | `flag` | `WORD` (16, signed test) |

---

## Inputs

| Position | Name   | Type   | Size | Source location   | Notes |
|----------|--------|--------|------|-------------------|-------|
| arg0     | `flag` | `WORD` | 2    | `bios/ikbd.c:145` | new state if `>= 0`; `-1` = query. Only low 8 bits are stored (truncated into a `UBYTE`). |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (zero-extended from the `UBYTE` `shifty`)
- **Encoding:** the **previous** `shifty` value in bits 0–7, bits 8–31 zero.
  Returned unconditionally.
- **Source location:** `bios/ikbd.c:147,152`.

### `shifty` bit layout (`include/biosdefs.h:90-100`)

| Bit | Mask | Constant      | Meaning |
|----:|-----:|---------------|---------|
| 0   | 0x01 | `MODE_RSHIFT` | Right Shift down |
| 1   | 0x02 | `MODE_LSHIFT` | Left Shift down |
| 2   | 0x04 | `MODE_CTRL`   | Control down |
| 3   | 0x08 | `MODE_ALT`    | Alternate down |
| 4   | 0x10 | `MODE_CAPS`   | CapsLock engaged (toggle) |
| 5   | 0x20 | `MODE_HOME`   | Clr/Home down; set only if Alt is down when the key is pressed (`bios/ikbd.c:876-883`), cleared on release of Home (`bios/ikbd.c:807-810`) |
| 6   | 0x40 | `MODE_INSERT` | Insert down; same rule (`bios/ikbd.c:881-883`, release `:811-814`) |
| 7   | 0x80 | (none)        | no `MODE_*` constant and no code that sets it; stores via `Kbshift(flag)` do keep it |

Combinations: `MODE_SHIFT = MODE_RSHIFT|MODE_LSHIFT = 0x03`;
`MODE_SCA = 0x0F` (`MODE_RSHIFT|MODE_LSHIFT|MODE_CTRL|MODE_ALT`,
`include/biosdefs.h:94`; [BIOS12] the earlier gloss "Shift+Ctrl+Alt" omitted
that both Shifts are separate bits); `HOTSWITCH_MODE = MODE_LSHIFT|MODE_ALT =
0x0A` (`bios/ikbd.h:26,28`). Per `biosdefs.h:96-98`, `MODE_HOME`/`MODE_INSERT`
are only set while Alt is already down, and stay set until their own key is
released even if Alt is released first.

### Side effects on global state

- `shifty` overwritten with `(UBYTE)flag` when `flag >= 0`
  (`bios/ikbd.c:150`). No other state.

### Side effects on hardware

None (does not command the keyboard controller).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none — cannot fail) | `bios/ikbd.c:145` | previous `shifty` |

---

## Verification criteria

- **Query:** `Kbshift(-1)` returns the current `shifty` and does not change it.
- **Set + return old:** `Kbshift(MODE_CAPS)` returns the previous value and
  then `Kbshift(-1) == MODE_CAPS` (0x10).
- **Truncation:** `Kbshift(0x0104)` stores only `0x04` into `shifty`
  (`Kbshift(-1)` afterwards returns `0x04`); the high byte is dropped.
- **Always zero-extended:** the return never has bits 8–31 set.

---

## Edge cases discovered in source

1. **`flag` is tested as signed; stored as a byte.** `flag >= 0`
   (`bios/ikbd.c:149`) treats `-1` (0xFFFF) as query; a stored value is
   truncated to 8 bits because `shifty` is a `UBYTE` (`bios/ikbd.c:150`). Prior
   spec noted the truncation correctly.

2. **You can set states the keyboard isn't in.** `Kbshift(MODE_CAPS)` makes the
   system believe Caps Lock is on even if the key is up. EmuTOS does not
   reconcile the written state against physical key positions.

3. **Race with the IKBD ISR.** The scancode handler updates `shifty` from
   interrupt context (`bios/ikbd.c:780-839`); a `Kbshift` write racing with a
   key event can lose the event's bit change. On Atari the ISR runs at high IPL
   so the window is small.

4. **`os_kbshift` aliases `&shifty`.** Programs can read modifier state without
   trapping by dereferencing `os_kbshift` (`include/biosdefs.h:59`); both paths
   see the same byte.

5. **CapsLock is a sticky toggle** toggled on key-down (not key-up), so it
   persists until the next CapsLock press (`bios/ikbd.c:845-852`; Amiga build
   excepted, see Configuration notes).

6. **No NumLock.** The Atari keyboard has none; the numpad is numeric unless
   Alt selects cursor mode.

---

## Dependencies

### Requires (must be initialized / functional)

- `shifty` maintained by the IKBD scancode handler (`kbd_init` + ACIA IRQ).

### Used by

- AES `evnt_keybd` / key-state queries; GEMDOS console reads that report shift
  state; `bios_init` itself (`kbshift(-1)` probed for Ctrl/Alt at boot,
  `bios/bios.c:517,812`).

---

## Related data structures

- `docs/api/structures/osheader.md` — `os_kbshift` field aliasing `shifty`
- `docs/api/bios/_state.md` — `shifty` in the BIOS state catalog

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: `flag = -1`
  query, same bit layout.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- [BIOS12] correction: the "TT/Falcon-era, never set on a plain ST" claim was
  wrong. The keyboard driver sets `MODE_HOME`/`MODE_INSERT` on Alt+Clr/Home and
  Alt+Insert with no machine or `CONF_*` switch around it
  (`bios/ikbd.c:876-883`, release `:807-814`). `kbshift` itself is
  version-independent. Only CapsLock differs by machine: toggle
  (`shifty ^= MODE_CAPS`) except `MACHINE_AMIGA`, where it is set on press and
  cleared on release (`bios/ikbd.c:845-852`, `:800-805`).

---

## NewTOS implementation notes

Wired at BIOS 0x0B. `tos-api/src/bios/mod.rs:120@kbshift` returns the
previous shift byte (zero-extended) always, and on `flag >= 0` stores `flag`
truncated to a byte through `Host::set_keyboard_shift_state`. The byte is
assembled from five atomics in the IKBD decoder
(`kernel/src/ikbd.rs:362@shift_state`, `kernel/src/ikbd.rs:389@set_shift_state`):
left and right Shift apart (RST01), Ctrl, Alt, Caps.

**[BIOS12] Current deviation (BIOS-CORE-009, MEASURED):** bits 5-7 are not
represented. `Kbshift(0x1FF)` then `Kbshift(-1)` reads `0x1F` (EmuTOS `0xFF`);
`0x7F81` reads `0x01` (EmuTOS `0x81`). The earlier justification "TT/Falcon-era"
is refuted (`bios/ikbd.c:876-883`). Equal on both: the return is the previous
value, the store truncates to a byte, negative flags only query (K06, K10),
restore works (K12/K13), `bios-r1` 77 (`-1`) EQUAL.
Not measured (PLAUSIBLE from code): the key path never sets bit 5/6 on NewTOS,
so Alt+Clr/Home reads without `0x20`. Contract target: the whole byte
round-trips, and the decoder sets/clears bits 5/6 like `bios/ikbd.c:807-814,
876-883`. Fix ticket: [BIOS10](../../../TASK/BIOS10.md); update this section
after it lands.

There is no `os_kbshift` low-memory alias on the Pi; access is the trap or the
host method.

### Open decisions

**Decision (user 2026-10-28):** D1 = **1**, one real byte in the decoder,
as EmuTOS' `shifty`. The options stay below as rationale.

**D1. Storage of bits 5-7 written by `Kbshift` (BIOS10).** EmuTOS keeps one
byte (`shifty`); a write replaces all eight bits and a later key event only
touches its own bit (`bios/ikbd.c:780-839`).
1. One real byte in the decoder (faithful). The five atomics become derived
   or are replaced; the write stores all 8 bits. Cost: touches every reader
   of the five atomics (`to_ascii` etc.).
2. Keep the five atomics, add a `u8` "extra" field for bits 5-7 (smaller
   change). Round-trip and K01-K13 equal; key events set bits 5/6 there.
   Cost: two representations of one byte.
3. Keep as is and record the deviation in COMPATIBILITY. Contradicts the
   audit probes K04/K05/K08/K09; no user of bits 5-7 is known in the ported
   programs (not surveyed here).

## Pi / BCM2711 binding

> Re-points the modifier-state source from the **IKBD 6301 / ACIA** (`shifty`
> maintained by the ACIA scancode ISR) to the **USB-HID keyboard over
> xHCI/PCIe** decoded into an IKBD `shifty`-equivalent. `Kbshift` the **trap**
> is wired (Task #24) over that byte; this binding pins where the modifier
> byte actually lives on the Pi. See `docs/HARDWARE-PROFILE.md` §3.3.

### The modifier byte lives in the IKBD decoder; the trap reads/writes it

`Kbshift` (BIOS 0x0B) is wired in the BIOS dispatcher
(`tos-api/src/bios/mod.rs::kbshift`). The modifier state is maintained by the
USB-HID stack: the HID report-modifier byte is translated into IKBD modifier
make/break codes (`drivers/usb/hid.rs`), fed through `crate::ikbd::feed`, and
kept as a `shifty`-equivalent (`crate::ikbd::{shift_state, set_shift_state}`).
The trap's query path returns it via `Host::keyboard_shift_state`; the set
path writes it via `Host::set_keyboard_shift_state`. The same byte reaches the
AES / VDI key-state consumers through the host trait. There is no
`os_kbshift` low-memory alias on the Pi.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| `shifty` UBYTE updated by the ACIA/IKBD scancode ISR | a `shifty`-equivalent kept in the `ikbd` decoder, updated from USB-HID make/break |
| `Kbshift(flag)` trap gets/sets it | in-range handler `kbshift`; get = `keyboard_shift_state()`, set = `set_keyboard_shift_state()` |
| `os_kbshift` aliases `&shifty` for pointer reads | no `os_kbshift` slot; access is `host().keyboard_shift_state()` (EL1 only) |
| IKBD 6301 on ACIA | USB-HID keyboard over xHCI (VL805/PCIe) → `ikbd::feed` |

### Gaps vs. the contract (residual)

- ~~**L/R shift not distinguished.**~~ Closed by RST01 (2026-10-17): the
  decoder keeps `MODE_RSHIFT` and `MODE_LSHIFT` as two bits, set and cleared
  by their own make/break codes (`0x36`/`0x2A`), as EmuTOS does
  (`bios/ikbd.c:779-783,830-834`); both round-trip through a `Kbshift` set.
- **Bits 5-7 not represented** [BIOS12 corrected]: `MODE_HOME`/`MODE_INSERT`
  are not "TT/Falcon-era"; EmuTOS sets them on every machine
  (`bios/ikbd.c:876-883`). NewTOS neither stores them via `Kbshift` (MEASURED,
  K04/K05, K08/K09) nor, PLAUSIBLE, sets them from the USB-HID key path.
  Bits 0-4 round-trip. Open: BIOS10 / decision D1 above.
