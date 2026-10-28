# `Bioskeys` (XBIOS 0x18)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/ikbd.c:136` (`bioskeys`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, `void` return, the "reset to power-up country
> defaults" behaviour. Reference: `current_keytbl` and the country tables.

## EmuTOS source

**Primary location:** `bios/ikbd.c:136` — `void bioskeys(void)`, slot 0x18
of `xbios_vecs[]` (`bios/xbios.c:1314`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:**
- `bios/country.c` — `get_keytbl()` — return the compiled default keyboard
  tables for the configured country.

**Headers / type definitions used:**
- `bios/ikbd.h:57-86` — `struct keytbl`.

**System variables / globals read:** the compiled country keyboard tables.

**System variables / globals written:**
- `current_keytbl` — reset to `*get_keytbl()` (`bios/ikbd.c:138-139`).

**Build configuration affecting this function:**
- The compiled country / keyboard-layout selection (which tables
  `get_keytbl` returns).

---

## Source extraction summary

`Bioskeys` restores the keyboard translation tables to their power-up state.
It fetches the country-default `struct keytbl` from `get_keytbl()` and copies
it wholesale into `current_keytbl`, undoing any `Keytbl` (0x10) overrides —
including the alt/dead/features fields that `Keytbl` cannot set. There are no
arguments and no return value.

This is the inverse of `Keytbl`: `Keytbl` swaps individual tables in;
`Bioskeys` puts the whole default set back.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x18
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0018
  ```
  No parameters (`Bioskeys()` = `xbios_v_v(24)`, `include/xbiosbind.h:42`).
- **Total parameter bytes:** 2.
- **Native (AArch64) ABI:** ID `0x18` in `x8`; no argument registers;
  **`void`** — no return value.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** none (`void`).
- **Type:** `void`.
- **Source location:** `bios/ikbd.c:136` (declared `void`).

### Side effects on global state

- `current_keytbl = *get_keytbl()` — all eight fields reset
  (`bios/ikbd.c:138-139`).

### Side effects on hardware

None (software table swap; the ISR picks up the new tables on the next key).

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

---

## Verification criteria

- **Undoes `Keytbl`:** after `Keytbl(mytbl, -1, -1)` then `Bioskeys()`,
  `Keytbl(-1,-1,-1)->norm` equals the country default again (not `mytbl`).
- **Resets all fields:** `alt*`/`dead`/`features` are restored to the country
  defaults, not just `norm`/`shft`/`caps`.

---

## Edge cases discovered in source

1. **Whole-struct reset.** `Bioskeys` overwrites all eight `struct keytbl`
   fields, so it also restores `altnorm/altshft/altcaps/dead/features` that
   `Keytbl` never touches (`bios/ikbd.c:138-139`).
2. **Country-dependent result.** The default tables come from the compiled
   country (`get_keytbl`), so the "power-up state" differs between a US and a
   German build.

---

## Dependencies

### Requires (must be initialized / functional)

- The compiled country keyboard tables; `current_keytbl` storage.

### Used by

- Programs that installed a custom layout and want to restore the default on
  exit; the desktop.

---

## Related data structures

- `docs/api/xbios/_state.md §2` — `current_keytbl`.
- `docs/api/xbios/10-keytbl.md` — the per-table setter.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents "restore
  power-up keyboard tables".)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- The default tables depend on the compiled country / keyboard layout.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-07: `Bioskeys` reloads the NewTOS country-default tables into
`current_keytbl`; the USB-HID translator uses them thereafter.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`get_keytbl` returning the compiled
> country tables the ACIA ISR then reads) at the NewTOS profile. The
> **contract is unchanged** if adopted — `void`, "reset all keytable fields
> to the power-up country default". Peripheral + driver: see
> `docs/HARDWARE-PROFILE.md` §2, §3.3.

- **The "country default" on the Pi.** The NewTOS default is the compiled-in
  **US** layout (`keymap.rs:193` `ACTIVE = US`), unless boot selected DE or a
  loadable `.KBD` (`init_from_disk`, `keymap.rs:266`). The `_AKP` cookie is
  seeded `0x00000000` = US (`HARDWARE-PROFILE.md` §3.7). `Bioskeys` would
  reinstall this default, undoing a run-time `Keytbl` override — the inverse
  of `Keytbl` (`10-keytbl.md`), which shares `keymap::install`
  (`keymap.rs:214-217`).
- **Step-2 — not wired.** `Bioskeys` (XBIOS `0x18`) has **no entry in the
  NewTOS XBIOS dispatcher** (`tos-api/src/xbios/mod.rs:65-138@dispatch`,
  the catch-all `other => other as i64`), so the trap returns the **function
  number `0x18`** (zero-extended, not `EINVFN`) and the active layout is not
  reset. `kernel/src/keymap.rs:223@select_builtin` provides the "install a
  named built-in" mechanism the handler would call, but nothing routes
  `Bioskeys` to it. Recorded as K-series below. (`Keytbl` is wired,
  `tos-api/src/xbios/mod.rs:65-138@dispatch`; `Initmous` is not.)
  **[XBIOS07] correction:** earlier text said "returns `EINVFN`". Measured:
  the function number — the same catch-all as `Iorec` (`0x0E`), see
  `docs/api/xbios/0e-iorec.md`; audit XBIOS-CHARDEV-002/-007
  (`measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`,
  sonde K04-K10: after `Bioskeys()` NewTOS keeps the program's tables,
  EmuTOS restores all three pointers).
- **Datasheet.** No peripheral register (software table swap).
