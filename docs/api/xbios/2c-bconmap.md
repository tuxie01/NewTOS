# `Bconmap` (XBIOS 0x2C)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/serport.c:1082` (`bconmap`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `-1`/`-2` queries, the `BCONMAP_START_HANDLE`
> (6) base, the `BCONMAP`/`MAPTAB` layouts, the `0x2c` return when the
> feature is absent. Reference: the vector-swap mechanics.

## EmuTOS source

**Primary location:** `bios/serport.c:1082` — `LONG bconmap(WORD dev)`, slot
0x2C of `xbios_vecs[]` (`bios/xbios.c:1347`).

**Installed handler / vector:** rewires the low-memory char-device vectors
(`bconstat_vec`/`bconin_vec`/`bcostat_vec`/`bconout_vec` for the mapped slot)
and the `rsconfptr`/`rs232iorecptr` globals from the selected `MAPTAB` entry.

**Helpers called:** none (inline vector copy).

**Headers / type definitions used:**
- `bios/serport.h:106-113` — `MAPTAB` (24 bytes).
- `bios/serport.h:115-119` — `BCONMAP` (`bconmap_root`).
- `bios/serport.h:121` — `BCONMAP_START_HANDLE = 6`.
- `bios/serport.h:18` — `BCONMAP_AVAILABLE = (CONF_WITH_SCC || CONF_WITH_TT_MFP)`.

**System variables / globals read/written:**
- `bconmap_root.mapped_device`/`.maptab`/`.maptabsize`.
- `bconstat_vec[…]`/`bconin_vec[…]`/`bcostat_vec[…]`/`bconout_vec[…]`,
  `rsconfptr`, `rs232iorecptr`.

**Build configuration affecting this function:**
- `BCONMAP_AVAILABLE` — **only true** when `CONF_WITH_SCC` or
  `CONF_WITH_TT_MFP` is set. On a single-serial build it is **false** and
  `bconmap` returns the function number `0x2c`.

---

## Source extraction summary

`Bconmap` maps the "current RS-232" logical device (BIOS handle 1 and the
`Rsconf`/`Iorec`/`Bconout(1,…)` family) onto one of several physical serial
ports (Falcon has modem-1/modem-2/serial-1/serial-2; TT has the ST MFP + TT
MFP; etc.). `dev == -1` returns the currently-mapped device number; `dev ==
-2` returns `&bconmap_root` (so a program can walk the `MAPTAB` array and
pre-configure it); a valid device (`>= BCONMAP_START_HANDLE = 6`) saves the
current vectors into the previously-mapped slot, installs the new slot's
vectors + `rsconfptr`/`rs232iorecptr`, updates `mapped_device`, and returns
the old device number; an out-of-range device returns `0L`.

When `BCONMAP_AVAILABLE` is false (a single-serial build — the NewTOS
profile), the whole body is compiled out and `bconmap` returns `0x2c` (the
function number), i.e. the D-008 "unimplemented" signal. So on the stock
NewTOS profile `Bconmap` reports "not available", not a working single-entry
table.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x2C
- **Stack frame on entry (m68k):** one WORD arg + LONG return, per the
  handler signature `LONG bconmap(WORD dev)` (`bios/serport.c:1082`); XBIOS
  opcode 0x2C. (`include/xbiosbind.h` does not define a `Bconmap` macro — the
  shape is `xbios_l_w`-style: `WORD` arg, `LONG` return.)
  ```
  +2: dev (word)
   0: function_id (word) = 0x002C
  ```
- **Total parameter bytes:** 4 (W).
- **Native (AArch64) ABI:** ID `0x2C` in `x8`; arg in `x0`; result is a LONG
  (device number, `0`, or `0x2c`) — for the `-2` case a **pointer** in the
  full 64-bit `x0`.

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `dev` | `WORD` (16) — device / `-1` / `-2` |

---

## Inputs

| Position | Name  | Type   | Size | Source location      | Notes |
|----------|-------|--------|------|----------------------|-------|
| arg0     | `dev` | `WORD` | 2    | `bios/serport.c:1088`| `-1` query, `-2` table ptr, `>= 6` select device |

`BCONMAP_START_HANDLE = 6` — valid devices are `6 .. 6 + maptabsize - 1`.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (device number / pointer / `0` / `0x2c`)
- **Encoding:**

  | `dev`   | Return | Source |
  |---------|--------|--------|
  | `-1`    | currently-mapped device number | `bios/serport.c:1089` |
  | `-2`    | pointer to `bconmap_root` (`BCONMAP`) | `bios/serport.c:1092` |
  | valid `>= 6` | previously-mapped device number | `bios/serport.c` (switch tail) |
  | invalid | `0L` | `bios/serport.c:1096` |
  | (`BCONMAP_AVAILABLE == 0`) | `0x2c` (function number) | `bios/serport.c:1124` |

### `BCONMAP` (`bconmap_root`, `bios/serport.h:115-119`)

| Offset | Field           | Type | Meaning |
|-------:|-----------------|------|---------|
| 0x00   | `maptab`        | `MAPTAB *` | per-device vector-set array |
| 0x04   | `maptabsize`    | `WORD` | number of entries |
| 0x06   | `mapped_device` | `WORD` | currently mapped device |

### `MAPTAB` (`bios/serport.h:106-113`) — 24 bytes

Six 4-byte vectors in order: `Bconstat`, `Bconin`, `Bcostat`, `Bconout`,
`Rsconf`, `EXT_IOREC *Iorec`.

### Side effects on global state

- The char-device vectors for the mapped slot + `rsconfptr`/`rs232iorecptr`
  are swapped (valid-device path); `mapped_device` updated.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `dev` out of range (`< 6` or `>= 6+maptabsize`) | `bios/serport.c:1096` | `0L` |
| `BCONMAP_AVAILABLE == 0` (single-serial build) | `bios/serport.c:1124` | `0x2c` |

---

## Verification criteria

- **Query current:** `Bconmap(-1)` returns the current mapped device (default
  6, or 7 on Falcon/ARAnyM).
- **Get table:** `Bconmap(-2)` returns `&bconmap_root`, a `BCONMAP` whose
  first field is the `maptab` pointer.
- **Invalid device:** `Bconmap(99)` returns `0L`.
- **Unavailable build:** on a build without SCC/TT-MFP (the NewTOS profile),
  `Bconmap(anything)` returns `0x2c` (the function number).

---

## Edge cases discovered in source

1. **`BCONMAP_AVAILABLE = (CONF_WITH_SCC || CONF_WITH_TT_MFP)`.** On a
   single-serial build it is false and `Bconmap` returns `0x2c`. The old spec
   claimed NewTOS "enables `BCONMAP_AVAILABLE = 1` with a 1-entry table" —
   that is a design *choice* NewTOS has not made; the source default on a
   single-port build is the `0x2c` return. See D-045.
2. **`BCONMAP` field order is `{maptab, maptabsize, mapped_device}`.** The old
   spec (and `_state.md`) had it reversed. `maptab` is at offset 0. See D-044.
3. **`MAPTAB.Iorec` is an `EXT_IOREC *`.** 24-byte struct, six 4-byte fields;
   the sixth is the RS-232 `EXT_IOREC` pointer (not a bare `IOREC`).
4. **Vector save-on-switch.** Before installing a new slot, `Bconmap` copies
   the current low-memory vectors back into the *old* slot, so direct edits
   made via `Bconmap`-managed slots survive a switch.
5. **Default `mapped_device`.** 7 on Falcon/ARAnyM, else 6
   (`bios/serport.c:966`).

---

## Dependencies

### Requires (must be initialized / functional)

- `BCONMAP_AVAILABLE` true (SCC or TT-MFP) for the function to do anything;
  `bconmap_root` initialised (`init_bconmap`).

### Used by

- Terminal programs targeting a second serial port, AES serial routing,
  "select serial port" UIs.

---

## Related data structures

- `docs/api/xbios/_state.md §3` — `BCONMAP`/`MAPTAB`/`EXT_IOREC`.
- `docs/api/xbios/0f-rsconf.md`, `docs/api/xbios/0e-iorec.md` — retargeted by
  `Bconmap`.
- `docs/api/bios/_state.md` — the char-device vector tables.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents the `-1`/`-2`
  queries and the device-6 base.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `BCONMAP_AVAILABLE` requires `CONF_WITH_SCC` or `CONF_WITH_TT_MFP`. NewTOS
  profile: neither → `Bconmap` returns `0x2c`. If NewTOS later exposes
  multiple serial ports it must deliberately enable a `MAPTAB` and document
  the choice.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07: with a single PL011 UART the honest EmuTOS behaviour is the `0x2c`
"unavailable" return. If NewTOS grows multiple serial devices (USB-CDC), it
can enable a `MAPTAB`; until then `Bconmap` reports unavailable rather than
faking a one-entry map. This is a deliberate departure from the old spec's
plan — see D-045.

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the Atari **multi-port serial** models
> (Falcon modem-1/2 + serial-1/2, TT ST-MFP + TT-MFP, SCC) to the **single
> BCM2711 PL011 UART0**. The contract above (trap ID/ABI, the `-1`/`-2`
> queries, `BCONMAP_START_HANDLE = 6`, the `BCONMAP`/`MAPTAB` layouts, the
> `0x2c` unavailable return) is **unchanged**. See `HARDWARE-PROFILE.md` §3.2.

**Peripheral / driver.** The Pi has exactly **one** TOS-visible serial port —
**PL011 UART0** (`kernel/src/drivers/bcm2711/pl011.rs`), used as the AUX/debug channel. There is
**no** SCC and **no** second MFP, so `BCONMAP_AVAILABLE = (CONF_WITH_SCC ||
CONF_WITH_TT_MFP)` is **false** on the NewTOS-Pi profile
(`HARDWARE-PROFILE.md` §4 — both off).

**Pi rationale for the `0x2c` return.** `Bconmap` exists to *re-route* the
"current RS-232" logical device across **several physical serial ports**. With
a single UART there is nothing to map:
- The `MAPTAB` array is **absent** (`maptabsize` would be 1, and a 1-entry map
  is the no-op identity — EmuTOS compiles the whole body out rather than build
  it).
- `mapped_device` never changes from its single value.
- `bconmap_root` is not initialised.

So the honest, EmuTOS-faithful behaviour is the **`0x2c` "unavailable"** signal
(the function number) — *not* a synthesized 1-entry table. This matches the
default AUX wiring: `rsconfptr`/`rs232iorecptr` point at the one PL011 handler
and are never swapped.

**Absent second serial ports.** The BCM2711 does expose four *other* PL011s
(UART2..5) plus the mini-UART (UART1) (DS §11.1 p.144), but NewTOS drives only
UART0; a native Atari-IKBD UART2 is a shelved **(plan)** (`HARDWARE-PROFILE.md`
§3.3) and would be the keyboard transport, not a `Bconmap`-selectable AUX port.
If NewTOS later exposes a USB-CDC or second serial as a real second AUX device,
it must **deliberately** enable a `MAPTAB` and document the choice (D-045).

### Current-code gaps → Step-2

- **R3 (GAP):** `Bconmap (0x2C)` is **not in the XBIOS dispatcher**
  (`tos-api/src/xbios/mod.rs:33-55`), so it returns **`EINVFN`**, **not** the
  contractually-correct **`0x2c`** ("unavailable"). This is an observable
  deviation: a caller checking for the `0x2c` "no second port" signal instead
  sees the generic `EINVFN`. Fix: wire a handler that returns `0x2c`.
