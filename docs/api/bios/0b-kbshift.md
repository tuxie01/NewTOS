# `Kbshift` (BIOS 0x0B)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/ikbd.c:145` (`kbshift`).
> [BIOS12] 2026-10-28: bit 5-7 statements, config note and NewTOS gaps corrected
> from `bios/ikbd.c:145-153,876-883` and the audit probes K01-K13
> (`measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`).
> [BIOS10] 2026-10-29: NewTOS keeps the whole byte (D1 = 1); key-path rule,
> keyboard mouse emulation note, VDI/AES readers and line ranges corrected from
> `bios/ikbd.c`, `vdi/vdi_input.c`, `aes/gemgsxif.c`; measured with
> `tosdump/probes/ksprobe.c` (`measurements/measurement-2026-10-29-bios10/RUNINFO.TXT`).

## NewTOS status: **implemented (wired at BIOS 0x0B)**

NewTOS handles the keyboard natively (USB-HID → `crate::ikbd::feed`); the
modifier state lives in the `ikbd` decoder. As of Task #24 `Kbshift` **is**
wired as an in-range dispatcher handler (`tos-api/src/bios/mod.rs`,
`kbshift`): the query path (`flag < 0`) returns
`host().keyboard_shift_state()` and the set path (`flag >= 0`) writes the
whole byte back through `host().set_keyboard_shift_state(flag as u8)`
(`kernel/src/ikbd.rs::set_shift_state`; one byte since BIOS10). `0x0B` is a real in-range function
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
`kbd_int` (`bios/ikbd.c:776-883`), which changes only the bit of the key it
handles: a Shift/Ctrl/Alt make sets its bit, the break clears it
(`:779-789`, `:832-843`); CapsLock toggles on its make, its break does nothing
(`:844-853`); a Clr/Home or Insert make sets `MODE_HOME`/`MODE_INSERT` only if
`MODE_ALT` is set at that moment (`:876-883`), the key's break clears the bit
unconditionally (`:807-814`). Every other bit -- bit 7, or a bit a program
stored -- stays until its own key event. Measured on the oracle (tt and
falcontc identical), `tosdump/probes/ksprobe.c`, transitions after each key:
Alt, Home, Home up, Alt up = `08 28 08 00`; Alt+Insert = `08 48 08 00`; Home
without Alt: no change; Alt, Home, Alt up, Home up = `08 28 20 00`; after
`Kbshift(0x80)`: LShift, LShift up, Alt, Home, Home up, Alt up =
`82 80 88 a8 88 80` (`measurements/measurement-2026-10-29-bios10/runs/ksprobe-EMUTOS-tt/KSPROBE.LOG`).

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
| 5   | 0x20 | `MODE_HOME`   | Clr/Home down; set only if Alt is down when the key is pressed (`bios/ikbd.c:876-880`), cleared on release of Home (`bios/ikbd.c:807-810`) |
| 6   | 0x40 | `MODE_INSERT` | Insert down; same rule (`bios/ikbd.c:881-883`, release `:811-814`) |
| 7   | 0x80 | (none)        | no `MODE_*` constant and no code that sets it; stores via `Kbshift(flag)` do keep it |

Combinations: `MODE_SHIFT = MODE_RSHIFT|MODE_LSHIFT = 0x03`;
`MODE_SCA = 0x0F` (`MODE_RSHIFT|MODE_LSHIFT|MODE_CTRL|MODE_ALT`,
`include/biosdefs.h:94`; [BIOS12] the earlier gloss "Shift+Ctrl+Alt" omitted
that both Shifts are separate bits); `HOTSWITCH_MODE = MODE_LSHIFT|MODE_ALT =
0x0A` (`bios/ikbd.h:26,28`). Per `biosdefs.h:96-98`, `MODE_HOME`/`MODE_INSERT`
are only set while Alt is already down, and stay set until their own key is
released even if Alt is released first.

Alt+Clr/Home and Alt+Insert are at the same time the buttons of the keyboard
mouse emulation (`KEY_EMULATE_RIGHT_BUTTON` = Clr/Home,
`KEY_EMULATE_LEFT_BUTTON` = Insert outside the Amiga build, `bios/ikbd.c:93-99`):
both keys count as "arrow keys" (`:766-775`, `:828-829`), so with Alt down
`convert_scancode` enters mouse mode and sends a button packet
(`:589-593`, `handle_mouse_mode` `:295-387`), and no key is queued while
mouse mode is on (`:904-908`). The bits are set all the same. Measured: the
oracle queues none of the Alt+Clr/Home / Alt+Insert presses of the probe,
only Clr/Home without Alt and Shift+a (`KSPROBE.LOG` Q lines).

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
- **All eight bits:** `Kbshift(0x1FF)` then `Kbshift(-1)` = `0xFF`;
  `Kbshift(0x7F81)` then `Kbshift(-1)` = `0x81` (audit probe K04/K05, K08/K09).
- **Key path:** Alt+Clr/Home reads `0x28` while both are down, Alt+Insert
  `0x48`; Clr/Home or Insert without Alt changes nothing; a stored bit 7 stays
  through key events (`tosdump/probes/ksprobe.c`, transition list above).

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
   interrupt context (`bios/ikbd.c:776-883`); a `Kbshift` write racing with a
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

- VDI `vq_key_s` (`vdi/vdi_input.c:110-113`): `gshift_s()` =
  `Kbshift(-1) & 0x000f` (`:185-188`) -- the four Shift/Ctrl/Alt bits only,
  not CapsLock nor bits 5-7.
- AES key state (`graf_mkstate`, the `evnt_*` key-state return): `kstate`
  comes from `gsx_kstate()` = VDI `KEY_STATE` (opcode 128, `vq_key_s`;
  `aes/gemgsxif.c:432-436`, `aes/funcdef.h:66`), polled in `chkkbd`
  (`aes/gemdisp.c:165`) and stored by `kchange` (`aes/geminput.c:288-293`),
  read by `gr_mkstate` (`aes/gemgrlib.c:458-464`) -- so the AES too reports
  `shifty & 0x0f`.
- The keyboard queue: every queued key carries `shifty` of its key press in
  bits 24-31 (`kb_last.k.shifty = shifty`, `bios/ikbd.c:898-909`), which
  `Bconin(2)` returns with `conterm` bit 3 -- not a `kbshift` call.
- `bios_init` itself (`kbshift(-1)` probed for Ctrl/Alt at boot,
  `bios/bios.c:517,812`) and the boot screen (`bios/initinfo.c:394,412,467`).

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

Wired at BIOS 0x0B. `tos-api/src/bios/mod.rs:132@kbshift` returns the
previous shift byte (zero-extended) always, and on `flag >= 0` stores `flag`
truncated to a byte through `Host::set_keyboard_shift_state`.

[BIOS10] The decoder keeps **one byte**, as EmuTOS' `shifty` (decision D1 =
1): `kernel/src/ikbd.rs:43@SHIFTY` is a `tos_api::bios::shifty::Shifty`
(`tos-api/src/bios/shifty.rs:119@Shifty`); `Kbshift` stores and returns all
eight bits (`kernel/src/ikbd.rs:366@set_shift_state`,
`kernel/src/ikbd.rs:358@shift_state`), and each IKBD key byte changes only its
own bit (`tos-api/src/bios/shifty.rs:89@key_event`, applied in
`kernel/src/ikbd.rs:272@SHIFTY` (`decode_key`) before the ASCII conversion and the queue
entry, as in `kbd_int`): left and right Shift apart (RST01), Ctrl, Alt,
CapsLock toggle, and `MODE_HOME`/`MODE_INSERT` for Clr/Home and Insert pressed
with Alt, cleared on their release. The previous five atomics (bits 0-4 only)
are gone. Measured (`measurements/measurement-2026-10-29-bios10/RUNINFO.TXT`):
audit probe lines K01-K13 and the `ksprobe` transitions T01-T24 and H01-H03
equal the oracle (tt = falcontc); `bios-r1` 77 (`-1`) EQUAL.

**Residual deviations (outside BIOS10, open):**

- **Keyboard mouse emulation missing.** NewTOS has no `handle_mouse_mode`
  (`docs/analysis/deps/mapping.tsv`, `unmapped`): Alt+Clr/Home and Alt+Insert
  are queued as keys (with `0x28`/`0x48` etc. in bits 24-31) instead of
  becoming mouse buttons. Measured: `ksprobe` Q lines, NewTOS 8 queued keys,
  oracle 2.
- **AES key state not masked** -- resolved by [A126](../../../TASK/A126.md)
  (2026-10-30): every AES reader (`graf_mkstate`, `evnt_*` `EV_KS`, the
  popup's `mn_keystate`) now takes the state from one helper,
  `tos-api/src/aes/gsxif.rs:286@gsx_kstate`, which reports `shifty & 0x0f` as
  EmuTOS' AES does through VDI `vq_key_s` (above); measured equal to the
  oracle with CapsLock + Alt held and with a program-set `0xf0`/`0xff`
  (`tosdump/probes/akprobe.c`,
  `measurements/measurement-2026-10-30-a126/RUNINFO.TXT`). VDI `vq_key_s`
  masks to `0x0f` as before (`tos-api/src/vdi/inquiry.rs:208@vq_key_s`).
- **The byte follows the input pump.** EmuTOS updates `shifty` in the IKBD
  interrupt; NewTOS decodes key bytes only in the trap-side input pump, and
  `Bconstat(2)` pumps only while the keyboard queue is empty
  (`kernel/src/host_env.rs:56@fill_from_device`); `Kbshift` itself does not
  pump. A program that polls `Kbshift(-1)` without reading its queued keys
  (or without any pumping call) sees a stale byte. Observed in a development
  run of `ksprobe` (probe then changed to read keys as they come); not a
  table case.

There is no `os_kbshift` low-memory alias on the Pi; access is the trap or the
host method.

### Open decisions

**Decision (user 2026-10-28):** D1 = **1**, one real byte in the decoder,
as EmuTOS' `shifty` -- implemented by BIOS10 (2026-10-29). The options stay
below as rationale.

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
make/break codes (`kernel/src/drivers/usb/hid.rs`), fed through
`crate::ikbd::feed`, and kept as one `shifty` byte
(`crate::ikbd::{shift_state, set_shift_state}`, BIOS10). The trap's query path
returns it via `Host::keyboard_shift_state`; the set path writes it via
`Host::set_keyboard_shift_state`. VDI `vq_key_s` reads it through the host
trait masked to `0x0f`, as EmuTOS; NewTOS' AES reads the whole byte (EmuTOS'
AES goes through `vq_key_s`; residual deviation above). There is no
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
- ~~**Bits 5-7 not represented.**~~ Closed by BIOS10 (2026-10-29): one byte,
  all eight bits round-trip through `Kbshift`, and the USB-HID key path sets
  and clears `MODE_HOME`/`MODE_INSERT` like `bios/ikbd.c:807-814,876-883`
  (measured with the IKBD injector, `ksprobe` T01-T24 equal the oracle).
- **Keyboard mouse emulation, AES mask, pump-driven byte**: see "Residual
  deviations" above.
