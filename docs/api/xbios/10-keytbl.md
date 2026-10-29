# `Keytbl` (XBIOS 0x10)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/ikbd.c:122` (`keytbl`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `(UBYTE *)-1` "keep current" sentinel, the
> returned `struct keytbl` layout (30 bytes), the 128-byte-table convention.
> Reference: `current_keytbl`'s storage and the ISR that reads it.

## EmuTOS source

**Primary location:** `bios/ikbd.c:122` —
`LONG keytbl(const UBYTE *norm, const UBYTE *shft, const UBYTE *caps)`, slot
0x10 of `xbios_vecs[]` (`bios/xbios.c:1306`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:** none.

**Headers / type definitions used:**
- `bios/ikbd.h:57-86` — `struct keytbl` (30 bytes).

**System variables / globals read/written:**
- `current_keytbl.norm/.shft/.caps` — overwritten unless the argument is the
  sentinel (`bios/ikbd.c:124-132`).

**Build configuration affecting this function:** none.

---

## Source extraction summary

`Keytbl` sets or leaves the three primary keyboard translation tables and
returns a pointer to the whole `current_keytbl` structure. Each of the three
arguments is applied only if it is **not** the sentinel `(UBYTE *)-1`: a
non-sentinel value overwrites the corresponding field
(`current_keytbl.norm/.shft/.caps`), while `(UBYTE *)-1` means "keep the
current pointer". Passing `-1` for all three is the idiom for "just give me
the current table pointer".

The return is `&current_keytbl`. The struct has six table pointers
(`norm`, `shft`, `caps`, `altnorm`, `altshft`, `altcaps`), a `dead`-key
table-array pointer, and a `features` word — 30 bytes total. `Keytbl` only
touches the first three; the alt/dead/features fields are set by `Bioskeys`
from the country tables and are readable through the returned pointer.

Each of `norm`/`shft`/`caps` points at a 128-byte table indexed by scancode,
producing the ASCII byte for that scancode. The IKBD ACIA ISR reads these
when translating keypresses.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x10
- **Stack frame on entry (m68k):** `Keytbl(norm, shft, caps)` =
  `xbios_l_lll(16, a, b, c)` (`include/xbiosbind.h:34`):
  ```
  +10: caps (LONG, pointer)
   +6: shft (LONG, pointer)
   +2: norm (LONG, pointer)
    0: function_id (word) = 0x0010
  ```
- **Total parameter bytes:** 14 (L + L + L).
- **Native (AArch64) ABI:** ID `0x10` in `x8`; args in `x0..x2`; result is a
  **pointer** in the full 64-bit `x0`.

  | m68k stack | AArch64 reg | Name   | Type / size |
  |------------|-------------|--------|-------------|
  | +2         | `x0`        | `norm` | `const UBYTE *` (ptr) — `-1` = keep |
  | +6         | `x1`        | `shft` | `const UBYTE *` (ptr) — `-1` = keep |
  | +10        | `x2`        | `caps` | `const UBYTE *` (ptr) — `-1` = keep |

---

## Inputs

| Position | Name   | Type   | Size | Source location    | Notes |
|----------|--------|--------|------|--------------------|-------|
| arg0     | `norm` | `const UBYTE *` | 4 | `bios/ikbd.c:124` | unshifted table; `(UBYTE*)-1` = keep |
| arg1     | `shft` | `const UBYTE *` | 4 | `bios/ikbd.c:127` | shifted table; `(UBYTE*)-1` = keep |
| arg2     | `caps` | `const UBYTE *` | 4 | `bios/ikbd.c:130` | caps-lock table; `(UBYTE*)-1` = keep |

Each non-sentinel pointer must reference a 128-byte scancode→ASCII table.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** pointer (`struct keytbl *`)
- **Encoding:** `&current_keytbl`.
- **Source location:** `bios/ikbd.c:133`.

### `struct keytbl` (`bios/ikbd.h:57-86`) — 30 bytes

| Offset | Field      | Type   | Meaning |
|-------:|------------|--------|---------|
| 0x00   | `norm`     | `const UBYTE *` | unshifted table (128 bytes) |
| 0x04   | `shft`     | `const UBYTE *` | shifted table |
| 0x08   | `caps`     | `const UBYTE *` | caps-lock table |
| 0x0C   | `altnorm`  | `const UBYTE *` | Alt table (variable format) |
| 0x10   | `altshft`  | `const UBYTE *` | Alt+Shift table |
| 0x14   | `altcaps`  | `const UBYTE *` | Alt+Caps table |
| 0x18   | `dead`     | `const UBYTE *const *` | dead-key table array |
| 0x1C   | `features` | `WORD` | feature flags (`DUAL_KEYBOARD = 0x0001`, …) |

### Side effects on global state / hardware

- Overwrites `current_keytbl.norm/.shft/.caps` for non-sentinel args. No
  hardware access.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Keytbl` always returns `&current_keytbl`.

---

## Verification criteria

- **Query current:** `Keytbl(-1, -1, -1)` changes nothing and returns
  `&current_keytbl`; a program can then read all eight fields.
- **Swap one table:** `Keytbl(mytbl, -1, -1)` sets `current_keytbl.norm ==
  mytbl` and leaves `shft`/`caps` unchanged.
- **Returned pointer is stable:** the same address is returned on every call
  (the struct lives in BIOS BSS).

---

## Edge cases discovered in source

1. **Sentinel is `(UBYTE *)-1`, not `NULL`.** `NULL` would be installed as an
   actual (null) table pointer and crash the ISR; only `-1` means "keep"
   (`bios/ikbd.c:124-132`). The old `_state.md` implied a `NULL` sentinel —
   corrected (D-042).
2. **Only three of eight fields are settable here.** `altnorm`/`altshft`/
   `altcaps`/`dead`/`features` are set by `Bioskeys` from the country tables,
   not by `Keytbl`; a program can read them via the returned pointer but not
   set them through `Keytbl`.
3. **Returned struct is 30 bytes, six table pointers.** Not the 3-pointer
   struct the old spec implied (D-042).

---

## Dependencies

### Requires (must be initialized / functional)

- `current_keytbl` initialised at boot (`Bioskeys` / `kbd_init`).

### Used by

- Alternate-layout installers, keyboard-remapping TSRs; the IKBD ISR reads
  the tables.

---

## Related data structures

- `docs/api/xbios/_state.md §2` — `current_keytbl` + `struct keytbl`.
- `docs/api/xbios/18-bioskeys.md` — resets the whole struct.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the three
  tables + the `-1` sentinel + the returned keytab pointer.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- The `features`/`dead`/alt fields exist across EmuTOS builds; the specific
  country default depends on the compiled keyboard tables.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07: NewTOS keeps the 30-byte `struct keytbl` layout; the USB-HID
translator consumes the same `norm`/`shft`/`caps` tables. The `(UBYTE*)-1`
sentinel and the returned pointer are preserved.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (the ACIA ISR that reads
> `current_keytbl` to translate 6301 scancodes) at the NewTOS profile, where
> the scancode→ASCII translation runs in `kernel/src/keymap.rs` over the
> USB-HID→IKBD scancode stream. The **contract is unchanged** if adopted —
> the `(UBYTE*)-1` "keep" sentinel, the 30-byte `struct keytbl`, the
> 128-byte table convention, `&current_keytbl` return. Peripheral + driver:
> see `docs/HARDWARE-PROFILE.md` §2, §3.3.

- **Where the tables live on the Pi.** The active layout is a mutable RAM
  copy of a `keymap::Layout` (`kernel/src/keymap.rs:26-36,187-200`), holding
  the three 128-byte scancode→ASCII tables (`norm`/`shft`/`caps`) plus AltGr
  `(scancode, ascii)` override pairs — the same three primary tables
  `Keytbl` sets. The scancode index is the **Atari IKBD scancode** produced
  by the USB-HID translator, so the tables are byte-for-byte the EmuTOS
  `keyb_us.h`/`keyb_de.h` tables (`keymap.rs:50-103` US, `:118-171` DE,
  extracted verbatim).
- **US + DE built-in, plus loadable `.KBD`.** Two compiled-in layouts (`US`
  `keymap.rs:106`, `DE` `:174`); the active one is chosen at boot by
  `init_from_disk` (`keymap.rs:266-281`): a `C:\KEYBOARD.KBD` file
  (`KBD1`-magic native format, `load_kbd` `:326`) overrides everything, else
  a `#K <layout>` line in `C:\NEWDESK.INF` selects `us`/`de`
  (`select_builtin` `:204`), else the compiled default US
  (`ACTIVE = US` `:193`). The `.KBD` blob layout (magic + 3×128 tables + 3
  AltGr groups) is documented at `keymap.rs:317-324`.
- **How ASCII is produced.** `keymap::to_ascii(scancode, shift, caps, alt)`
  (`keymap.rs:223-259`) picks `shft`/`caps`/`norm` by modifier, then applies
  the AltGr override pairs — the EmuTOS `ascii_cnv` (non-DUAL_KEYBOARD)
  selection logic. Ctrl-folding to a control code is done by the caller
  (`ikbd.rs:176-183`).
- **Wired since 2026-09-01 (desktop-user-space stage S1d).** `Keytbl` is in
  the NewTOS XBIOS dispatcher (`tos-api/src/xbios/mod.rs`, id `0x10` →
  `tos-api/src/xbios/keytbl.rs`). Both halves work:
  - **Get.** The returned `struct keytbl` and the six tables it points at are
    **materialised into the caller's own address space** on every call: into a
    running EL0 process's own page 0 (`KEYTBL_AREA_VA` = `0x768`,
    `tos-api/src/bdos/process/loader.rs`; page 0 is the one per-process page
    of the shared user address space since ARCH03 stage 1), written through
    the kernel alias of that page and returned as the process's **user VA**;
    into a kernel static, and returned as a kernel VA, when no process is
    running (the in-kernel desktop / boot path). This is what makes EmuTOS' desktop idiom
    `keytab = (KEYTAB *)Keytbl(-1,-1,-1); keytab->shift[…]`
    (`desk/deskmain.c:1144-1145`) work from EL0 instead of faulting.
    Pointer fields are written at the **caller's** ABI width (8 bytes LP64,
    4 bytes ILP32), so the struct is 58 or 30 bytes respectively.
    `dead` is `NULL` and `features` `0` — exactly as EmuTOS' own US and German
    tables end (`bios/keyb_us.h`, `bios/keyb_de.h`: `…, NULL, 0`).
  - **Set.** A non-sentinel argument really changes the translation: the 128
    bytes are taken over into the active `keymap::Layout` through
    `Host::set_key_table`. EmuTOS stores the caller's *pointer* and lets its
    ACIA ISR read the caller's memory; NewTOS has no such ISR, so it copies
    the bytes and *additionally* remembers the pointer, which a later
    `Keytbl(-1,-1,-1)` reports back verbatim as EmuTOS would. **Residual
    difference:** editing the table in place *after* installing it does not
    take effect until `Keytbl` is called again.
  - The AltGr arrays are emitted in EmuTOS' non-`DUAL_KEYBOARD` wire format
    (zero-terminated `(scancode, ascii)` pairs), which is exactly how
    `keymap::Layout` already holds them.
  - `Bioskeys` (`0x18`) undoes every set since XBIOS02: the remembered
    pointers are dropped and the translator's power-up layout is put back
    (`tos-api/src/xbios/keytbl.rs:237@bioskeys`, `18-bioskeys.md`).
- **Datasheet.** No peripheral register (software table).
