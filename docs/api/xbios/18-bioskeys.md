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
- `bios/country.c:195-202` — `get_keytbl()` — return `keytables[j]`, the
  default `struct keytbl` of the keyboard the low byte of `cookie_akp`
  names (`get_kbd_index` → `get_current_country_input` →
  `get_country_record`, `bios/country.c:75-106`; an unknown code falls back
  to the OS-header default country, `:60-73`). `cookie_akp` is written once,
  at boot, by `detect_akp` (`bios/country.c:140-163`: OS-header country,
  overridden by the NVRAM keyboard byte); the cookie jar only receives a copy
  (`bios/machine.c:742`), so the table `get_keytbl` returns is fixed after
  boot — the **power-up** table.

**Headers / type definitions used:**
- `bios/ikbd.h:57-86` — `struct keytbl`.

**System variables / globals read:** `cookie_akp` (through `get_keytbl`) and
the compiled country keyboard tables (`keytables[]`).

**System variables / globals written:**
- `current_keytbl` — reset to `*get_keytbl()` (`bios/ikbd.c:138-139`).

**Build configuration affecting this function:**
- The compiled country tables and the OS-header country; with
  `CONF_WITH_NVRAM` the NVRAM keyboard byte (which tables `get_keytbl`
  returns).

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
  declared **`void`**; for the value left in `x0` see "Return value".

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Declared:** `void` (`bios/ikbd.c:136`, `bios/ikbd.h:116`).
- **What `d0` holds on the oracle:** the pointer `get_keytbl()` returned —
  the address of the country-default `struct keytbl` in ROM. The pinned
  build (`m68k-atari-mint-objdump -dr
  references/emutos-oracle/1a7484ab/obj/ikbd.o`, `_bioskeys`) is
  `jsr _get_keytbl; moveal %d0,%a0`, eight moves through `%a0`, `rts` —
  `d0` is not touched after the call. Measured on EmuTOS TT and Falcon:
  `0x00E06E8C` (audit probe K07/K11,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt/XCPROBE.LOG`).
  A program cannot rely on it (the function is `void`); a tosdump case
  compares it at most as a pointer (`IGNORE` or a pointer class).

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
  `Keytbl(-1,-1,-1)->norm` equals the country default again (not `mytbl`);
  the same for `shft`/`caps`. Measured on the oracle (audit probe K04–K10,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt/XCPROBE.LOG`):
  `unshift_is_n0=1 shift_is_s0=1 caps_is_c0=1 unshift_is_my=0`, and the
  struct address `Keytbl` returns is unchanged (`same_ptr=1`).
- **Translation follows:** a key typed after `Bioskeys()` is translated with
  the default table again (oracle: `a` after `Keytbl(my_n)` = `0x001E004B`,
  after `Bioskeys()` = `0x001E0061`; input probe X02/X03,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/KEYS-EMUTOS-tt/XKPROBE.LOG`).
- **Undoes a NULL table:** after `Keytbl(NULL, -1, -1)` then `Bioskeys()`,
  `norm` is a valid table pointer again (XCKTNL,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCKTNL-EMUTOS-tt/XCKTNL.LOG`).
- **Resets all fields:** `alt*`/`dead`/`features` are restored to the country
  defaults, not just `norm`/`shft`/`caps`.

---

## Edge cases discovered in source

1. **Whole-struct reset.** `Bioskeys` overwrites all eight `struct keytbl`
   fields, so it also restores `altnorm/altshft/altcaps/dead/features` that
   `Keytbl` never touches (`bios/ikbd.c:138-139`).
2. **Country-dependent result.** The default tables come from the country
   chosen at boot (`get_keytbl` via `cookie_akp`), so the "power-up state"
   differs between a US and a German machine. Writing a different `_AKP`
   value into the cookie jar does not change it (the jar holds a copy,
   `bios/machine.c:742`).
3. **`d0` is a pointer.** Although `void`, the oracle leaves the address of
   the default `struct keytbl` in `d0` (see "Return value").

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

**Wired since XBIOS02 (2026-10-29).** `tos-api/src/xbios/mod.rs:107@BIOSKEYS`
routes id `0x18` to `tos-api/src/xbios/keytbl.rs:237@bioskeys`:

1. The pointers `Keytbl` remembered for `norm`/`shft`/`caps` are dropped
   (`OVERRIDE`, `tos-api/src/xbios/keytbl.rs:86@OVERRIDE`), so the struct
   reports the area's own copies of the default tables again — EmuTOS'
   `current_keytbl = *get_keytbl()` overwrites the caller's pointers the same
   way.
2. The translator puts its power-up layout back
   (`Host::reset_key_tables`, `tos-api/src/host.rs:278@reset_key_tables`; on
   the kernel host `kernel/src/keymap.rs:264@restore_default`): all three
   primary tables and the AltGr arrays, so the keys typed afterwards are
   translated with the defaults, not only the reported struct. The source
   `Ssystem(S_NEWTOS_KEYBOARD)` reports goes back to the boot selection.
3. The `struct keytbl` is rewritten into the caller's address space as
   `Keytbl` does (`tos-api/src/xbios/keytbl.rs:251@publish`).

**Return value.** The address of that rewritten `struct keytbl` (the same
address `Keytbl(-1,-1,-1)` returns, now holding the defaults); 0 on a board
without a keyboard translator, as `Keytbl`. The oracle's `d0` is the address
of a separate, read-only default struct in ROM (`0x00E06E8C`); NewTOS has no
such struct. Both are a valid pointer to a `struct keytbl` holding the
country defaults right after the call; they differ in the address and in
that NewTOS' struct is rewritten by a later `Keytbl`.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`get_keytbl` returning the compiled
> country tables the ACIA ISR then reads) at the NewTOS profile. The
> **contract is unchanged** — "reset all keytable fields to the power-up
> country default". Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2,
> §3.3.

- **The "country default" on the Pi = the boot selection.** NewTOS has no
  ROM country and no NVRAM. The power-up layout is what the boot chose
  (`kernel/src/keymap.rs:367@init_from_disk`): `C:\KEYBOARD.KBD`, else a
  `#K us`/`#K de` line in `C:\NEWDESK.INF`, else the compiled-in US layout
  (`kernel/src/keymap.rs:210@DEFAULT`; `docs/COMPATIBILITY.md`, "The boot
  keyboard layout comes from …"). The boot selection writes the active
  layout and this power-up copy together
  (`kernel/src/keymap.rs:248@install_power_up`); nothing else writes the
  copy, so — as on EmuTOS, where `cookie_akp` is set once by `detect_akp` —
  what `Bioskeys` restores is fixed after boot. `_AKP` stays `0` (US)
  whichever layout is active (`docs/COMPATIBILITY.md`, same entry; TASK
  COOK02) and is not read by `Bioskeys`.
- **`dead` and `features`.** NewTOS' layouts have no dead-key tables and no
  `DUAL_KEYBOARD`; the struct reports `dead = NULL`, `features = 0`, as
  EmuTOS' US and German tables do (`10-keytbl.md`).
- **[XBIOS07] correction (history):** before XBIOS02 the trap had no
  dispatcher arm and returned the function number `0x18` (not `EINVFN` as
  earlier text said); after `Bioskeys()` NewTOS kept the program's tables
  (audit XBIOS-CHARDEV-002,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`,
  probe K04–K10). Fixed by XBIOS02; measured equal in
  `measurements/measurement-2026-10-29-xbios02/RUNINFO.TXT`.
- **Datasheet.** No peripheral register (software table swap).
