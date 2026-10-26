# `Iorec` (XBIOS 0x0E)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/xbios.c:336` (`iorec`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `devno` → record mapping, the `-1` out-of-range
> return, and the `IOREC`/`EXT_IOREC` layouts a program reads/writes.
> Reference: the ACIA/UART drivers behind each record.

## NewTOS status: **decision deferred**

`Iorec` returns a pointer to the Atari `IOREC` ring-buffer descriptor
(IKBD/MIDI/RS-232). NewTOS has a native console-RX ring + an `ikbd` record;
whether to expose an `IOREC`-shaped view is a port-vs-reimplement call
(`docs/COMPATIBILITY.md`), **deferred until a program needs it**. If exposed,
the layouts below are the contract.

## EmuTOS source

**Primary location:** `bios/xbios.c:336` — `static LONG iorec(WORD devno)`,
slot 0x0E of `xbios_vecs[]` (`bios/xbios.c:1304`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:** none.

**Headers / type definitions used:**
- `bios/iorec.h:20-27` — `IOREC` (14-byte bare input ring descriptor).
- `bios/serport.h:57-65` — `EXT_IOREC` (RS-232 record: input + output ring +
  serial state).

**System variables / globals read:**
- `rs232iorecptr` (`bios/serport.c:75`) — pointer to the active RS-232
  `EXT_IOREC` (device 0).
- `ikbdiorec` (device 1), `midiiorec` (device 2) — static `IOREC`s.

**System variables / globals written:** none.

**Build configuration affecting this function:** none in the wrapper; the
records exist per `CONF_WITH_IKBD_ACIA` / `CONF_WITH_MIDI_ACIA` / serial.

---

## Source extraction summary

`Iorec` returns the address of the input-buffer descriptor for a serial
device so a program can inspect or drive the ring directly. `devno == 0`
returns `rs232iorecptr` (an `EXT_IOREC *`); `devno == 1` returns
`&ikbdiorec`; `devno == 2` returns `&midiiorec`; any other value returns
`-1` (`bios/xbios.c:338-347`).

The IKBD and MIDI records are **bare `IOREC`** structures (14 bytes,
input-only). The RS-232 record is an `EXT_IOREC` (an input `IOREC` **plus** an
output `IOREC` plus baud/flow/UCR state), because RS-232 needs a TX ring and
can be remapped by `Bconmap`. That is why device 0 is reached through a
*pointer variable* (`rs232iorecptr`) while devices 1/2 return fixed struct
addresses.

Programs use the returned pointer to poll `head != tail` for "data available"
without going through `Bconin`, or to replace `buf`/`size` with a larger
ring.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x0E
- **Stack frame on entry (m68k):** `Iorec(devno)` = `xbios_l_w(14, a)`
  (`include/xbiosbind.h:32`):
  ```
  +2: devno (word)
   0: function_id (word) = 0x000E
  ```
- **Total parameter bytes:** 4 (W).
- **Native (AArch64) ABI:** ID `0x0E` in `x8`; arg in `x0`; result is a
  **pointer** in the full 64-bit `x0` on success, or the LONG `-1`
  (sign-extended) on error.

  | m68k stack | AArch64 reg | Name    | Type / size |
  |------------|-------------|---------|-------------|
  | +2         | `x0`        | `devno` | `WORD` (16) |

---

## Inputs

| Position | Name    | Type   | Size | Source location     | Notes |
|----------|---------|--------|------|---------------------|-------|
| arg0     | `devno` | `WORD` | 2    | `bios/xbios.c:338`  | 0=RS-232, 1=IKBD, 2=MIDI; else `-1` |

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (pointer, or `-1`)
- **Encoding:** `EXT_IOREC *` (device 0) / `IOREC *` (devices 1,2) / `-1`.
- **Source location:** `bios/xbios.c:340,342,344,346`.

### Side effects on global state / hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `devno` not in {0,1,2} | `bios/xbios.c:345-346` | `-1` |

---

## Verification criteria

- **IKBD record:** `Iorec(1)` returns a non-`-1` pointer to a 14-byte `IOREC`
  whose `size` is 256 (the IKBD buffer).
- **MIDI record:** `Iorec(2)` returns a pointer to a 14-byte `IOREC` whose
  `size` is 128.
- **RS-232 record:** `Iorec(0)` returns `rs232iorecptr` (an `EXT_IOREC` with
  both `in` and `out` rings).
- **Out of range:** `Iorec(3)` returns `-1`.

---

## Edge cases discovered in source

1. **Device 0 is a pointer-to-pointer indirection.** `Iorec(0)` returns
   `rs232iorecptr` (`bios/xbios.c:340`), which `Bconmap` can retarget; devices
   1/2 return fixed struct addresses. So the RS-232 record can change at run
   time, the IKBD/MIDI ones cannot.
2. **Device 0 record is bigger and different-shaped.** `Iorec(0)` →
   `EXT_IOREC` (`in`/`out` rings + serial state); `Iorec(1/2)` → bare
   `IOREC` (input only). A program must not treat a device-0 pointer as a
   14-byte `IOREC`. (Old spec described all three as one shape — corrected,
   D-043.)
3. **Devices 3..7 are unsupported.** Falcon TOS extends `Iorec` to more
   serial ports; EmuTOS clamps to 2 and returns `-1` above that — a
   cross-reference, not a discrepancy.
4. **Buffer sizes: IKBD 256, MIDI 128.** (`bios/aciavecs.S`.) The old spec
   said IKBD 128 — corrected (D-043).

---

## Dependencies

### Requires (must be initialized / functional)

- `rs232iorecptr`, `ikbdiorec`, `midiiorec` initialised at boot.

### Used by

- Terminal programs, custom keyboard handlers, MIDI applications.

---

## Related data structures

- `docs/api/xbios/_state.md §3` — `IOREC` (14 bytes) + `EXT_IOREC` (24 bytes)
  layouts.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents devices 0..2
  and the Falcon 3..7 extension.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `CONF_WITH_IKBD_ACIA` / `CONF_WITH_MIDI_ACIA` / serial configs decide which
  records are live; on NewTOS the records would be synthesised.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07 (if adopted): expose synthetic `IOREC`s — the IKBD ring filled by
the USB-HID translator, the RS-232 `EXT_IOREC` by the UART driver, MIDI
present but empty. Keep the 14-byte `IOREC` / device-0 `EXT_IOREC`
distinction.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the ACIA/UART `IOREC` ring descriptors
> behind IKBD/MIDI/RS-232) at the NewTOS profile. The **contract is
> unchanged** if adopted — `devno` 0/1/2 → record pointer, `-1` out of
> range, the 14-byte `IOREC` vs. device-0 `EXT_IOREC` shapes. **Status stays
> decision-deferred** (see the header). Peripheral + driver: see
> `docs/HARDWARE-PROFILE.md` §2 (Keyboard/mouse → USB-HID; AUX → PL011).

- **What the underlying rings are on the Pi.**
  - **`devno == 1` (IKBD):** the NewTOS console RX ring is `RX_RING` in
    `kernel/src/console.rs:215` — a 128-entry SPSC ring of **key words**
    (`scancode << 8 | ascii`, `kernel/src/console.rs:276`), filled by the USB-HID
    keyboard decoder (`ikbd.rs::decode_key` → `console::push_rx`,
    `ikbd.rs:139`). This is **not** laid out as a 14-byte Atari `IOREC`; a
    synthetic `IOREC` view over it would have to be built to satisfy the
    contract.
  - **`devno == 0` (RS-232 / AUX):** the PL011 UART0 debug channel
    (`kernel/src/drivers/bcm2711/pl011.rs`), which also feeds `RX_RING`
    today ((cite stale 2026-10-26: console.rs:91-107, UART0 RX feeding RX_RING), see the UART-console Step-2 finding). No
    `EXT_IOREC` is exposed.
  - **`devno == 2` (MIDI):** no MIDI hardware (`HARDWARE-PROFILE.md` §3.9) —
    an empty record if exposed.
- **Step-2 — not wired, and the buffer shape differs.** `Iorec` (XBIOS
  `0x0E`) is **not in the NewTOS XBIOS dispatcher**
  (`tos-api/src/xbios/mod.rs`), so it returns `EINVFN`, not a record
  pointer. Exposing it would require synthesising `IOREC`/`EXT_IOREC` headers
  over the native `RX_RING` (whose element is a 16-bit key word, not the
  4-byte IKBD record the Atari ring holds). Recorded as K-series below;
  matches the doc's "decision deferred" status.
- **Datasheet.** IKBD/mouse transport = VL805 xHCI (§2); AUX = PL011 UART0,
  BCM2711 peripherals datasheet §11 (see the serial specs).
