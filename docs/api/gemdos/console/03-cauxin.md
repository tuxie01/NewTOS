# `Cauxin` (GEMDOS 0x03)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bdos/console.c:432` (`xauxin`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../../COMPATIBILITY.md).
> Contract: trap ID/ABI, blocking, pass-through of the `Bconin` byte. Reference:
> the (unused) AUX typeahead buffer.

## EmuTOS source

**Primary location:** `bdos/console.c:432` — `long xauxin(void)`

**Installed handler / vector:** (none) — direct `funcs[]` entry `0x03` =
`xauxin`, `stdio_typ = 0x82` (stdaux), `wparms = 0` (`bdos/bdosmain.c:158`).
When stdaux is redirected to a file (`run->p_uft[2] > 0`) the dispatcher reads
one byte from the file and this body is not entered — see Edge case 3.

**Helpers called:**
- BIOS `Bconin(1)` (0x02) — read one byte from the AUX/serial device.

**Headers / type definitions used:**
- `bdos/console.h:24,36` — `H_Aux = -2`, `HXFORM(h) = 3+h` (→ BIOS handle 1).

**System variables / globals read:**
- `run->p_uft[2]` (`bdos/console.c:434`) — the stdaux standard handle.

**System variables / globals written:** none.

**Build configuration affecting this function:** none directly. Transitive: the
AUX BIOS device depends on `CONF_WITH_MFP` / `CONF_WITH_SCC` /
`CONF_SERIAL_CONSOLE` for which serial port backs handle 1.

---

## Source extraction summary

`Cauxin` reads and returns one character from the auxiliary (serial) device. It
is a **pure pass-through**: `xauxin` resolves stdaux from `run->p_uft[2]`
(default `H_Aux = -2` → BIOS handle 1) and returns `Bconin(HXFORM(p_uft[2]))`.
It blocks — the underlying `bconin1` spins until a byte arrives.

Unlike `Cconin`/`Cnecin`, `Cauxin` does **not** consult the BDOS typeahead
buffer. The `aux_typeahead` structure exists in `bdos/console.c:44` and is
initialised by `stdhdl_init`, but no GEMDOS function ever fills or drains it, so
`Cauxin` always reaches the BIOS directly (see D-003). It also performs no
Ctrl-S/Ctrl-C/echo processing — those live only in the CON cooked path.

The returned LONG is `Bconin`'s value for the serial device: the data byte in
bits 0-7, upper bytes device-driver-defined (typically 0). `xauxin` forwards it
unchanged.

---

## Trap interface

- **Trap number:** 1 (GEMDOS)
- **Function ID:** 0x03
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0003
  ```
  No parameters.
- **Total parameter bytes:** 2 → `wparms = 0`.
- **Native (AArch64) ABI:** ID `0x03` in `x8`; no argument registers; LONG
  result sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None. The device is `run->p_uft[2]`, not a parameter.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** data byte in bits 0-7; upper bytes as returned by the serial
  `Bconin` (normally 0).
- **Source location:** `bdos/console.c:434` (forwards `Bconin`).

### Side effects on global state

- None in the BDOS layer.

### Side effects on hardware

- Reads the serial UART/ACIA receive register (clears its data-ready flag).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Cauxin` has no GEMDOS error return. Framing/parity errors are absorbed by the
BIOS serial driver.

**Error paths not in Compendium:** none.

---

## Verification criteria

- **Nominal:** with byte `0x41` available on the AUX device, `Cauxin()` returns
  `0x00000041`.
- **Blocking:** with no byte available, the call does not return until one
  arrives.
- **No typeahead / no flow control:** a Ctrl-S arriving on AUX is returned as
  the data byte `0x13`; it does not pause anything.
- **Redirected stdaux:** with `run->p_uft[2]` forced to a file, one byte is
  read from the file; at EOF the call returns `EOF_INDICATOR` (`0x0000ff1a`).

---

## Edge cases discovered in source

1. **No typeahead integration.** `Cauxin` calls `Bconin` directly; the
   `aux_typeahead` buffer is allocated but never used by any GEMDOS path (D-003).

2. **No flow control or Ctrl-C.** AUX input is raw — Ctrl-S/Ctrl-Q/Ctrl-C are
   returned as ordinary data bytes.

3. **Redirected stdaux bypasses this body.** `run->p_uft[2] > 0` → the
   dispatcher does `xread(h,1,&ctmp)`; at EOF returns `EOF_INDICATOR`
   (`bdos/bdosmain.c:509-519`).

4. **The BIOS handle can be remapped.** `Bconmap` (XBIOS 0x2C) remaps the
   *serial* device family behind BIOS handle 1; `Cauxin` follows whatever
   handle 1 currently drives, but `HXFORM(p_uft[2])` itself is fixed at 1 for
   the default `H_Aux`.

---

## Dependencies

### Requires (must be initialized / functional)

- BIOS serial input `Bconin(1)` and its driver (`init_serport`).
- `stdhdl_init()` (sets `p_uft[2] = H_Aux`).

### Used by

- Programs reading the RS-232 / current AUX device; terminal programs.

---

## Related data structures

- `docs/api/bios/02-bconin.md` — the underlying byte-input BIOS call.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents `Cauxin` as a
  blocking serial read — consistent with EmuTOS.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- AUX BIOS device selection: `CONF_WITH_MFP`, `CONF_WITH_SCC`,
  `CONF_SERIAL_CONSOLE`.

---

## NewTOS implementation notes

(Empty during Phase 00.)

FS11 / IO01 prep (2026-10-13, user decision 1): see `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes" for the device table.
`Cauxin` is a pass-through to `Bconin(1)` of the **AUX device-table entry**. The
entry is currently the documented AUX profile "port present, no cable" (option A, 2026-10-13, see `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes" item 1a; deliberate, not a stub):
`Bconin` returns **0 immediately** (non-blocking; **documented divergence** from a real port, where `bconin1` would block forever), `Cauxin()` therefore returns `0x00000000` and
never blocks (the "Blocking" verification criterion above applies to a real
AUX driver only). Value matches EmuTOS' Bconmap dummy map entry `char_dummy`
(`bios/serport.c:96`, `bios/chardev.c:50-53`), **not** EmuTOS' own `bconin1`
(blocking, `bios/serport.c:260-263`, `bconin_iorec` loop `:202-205`). A later UART
driver (`TASK/IO01.md`) replaces only the entry.

*Spec inconsistency noted, semantics unchanged:* Edge case 1 / D-003 say the AUX
typeahead buffer is "never used by any GEMDOS path". EmuTOS' `conbrk(h)`
(`bdos/console.c:206-243`, reached from `conout(1, …)`, e.g. the echo of
`Fread(-2, …)`) *does* append to `buffer[1]`, and `getch`/`constat` read it
(`bdos/console.c:143-152,351-366`). `Cauxin` bypasses it, `Cauxis` does not. With a
real AUX device that is an EmuTOS quirk; in NewTOS it is unobservable (no BDOS
typeahead buffer; absent device has no status).
