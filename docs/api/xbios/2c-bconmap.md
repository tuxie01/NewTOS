# `Bconmap` (XBIOS 0x2C)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/serport.c:1082` (`bconmap`).
> NewTOS profile: **one-entry table like EmuTOS on a machine that is neither
> Falcon nor TT** — user decision 2026-10-28 (`TASK/XBIOS05.md`, option 1),
> implemented by XBIOS05 (2026-10-29).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the `-1`/`-2` queries, the `BCONMAP_START_HANDLE`
> (6) base, `0` for a device outside the table, the `BCONMAP`/`MAPTAB`
> field order (LP64 layout: `docs/api/structures/bconmap.md`). Device
> parameters (ACCEPTED per profile): `maptabsize` and the default
> `mapped_device` depend on the machine (TT 4/6, Falcon 3/7, NewTOS 1/6).
> Not contract: the addresses; the `MAPTAB` function pointers are not
> callable on NewTOS.

## EmuTOS source

**Primary location:** `bios/serport.c:1082` — `LONG bconmap(WORD dev)`, slot
0x2C of `xbios_vecs[]` (`bios/xbios.c:1347`).

**Installed handler / vector:** rewires the low-memory char-device vectors
(`bconstat_vec`/`bconin_vec`/`bcostat_vec`/`bconout_vec` for device 1)
and the `rsconfptr`/`rs232iorecptr` globals from the selected `MAPTAB` entry.

**Helpers called:** none (inline vector copy).

**Headers / type definitions used:**
- `bios/serport.h:106-113` — `MAPTAB` (24 bytes).
- `bios/serport.h:115-119` — `BCONMAP` (`bconmap_root`).
- `bios/serport.h:121` — `BCONMAP_START_HANDLE = 6`.
- `bios/serport.h:18` — `BCONMAP_AVAILABLE = (CONF_WITH_SCC || CONF_WITH_TT_MFP)`.

**System variables / globals read/written:**
- `bconmap_root.mapped_device`/`.maptab`/`.maptabsize`.
- `bconstat_vec[1]`/`bconin_vec[1]`/`bcostat_vec[1]`/`bconout_vec[1]`,
  `rsconfptr`, `rs232iorecptr`.

**Build configuration affecting this function:**
- `BCONMAP_AVAILABLE` — true when `CONF_WITH_SCC` or `CONF_WITH_TT_MFP` is
  set. The pinned reference build (make 512, Falcon/TT profile) has both
  (`docs/api/_emutos-config.md:201,211`; `include/config.h:89-90`), so
  `BCONMAP_AVAILABLE` is true and `Bconmap` works on **every** machine under
  that ROM [XBIOS07]. The table is built by `init_bconmap`
  (`bios/serport.c:956-1004`), defined and called under the same gate
  (`:944`, `:1056-1059`, from `init_serport`). Only a build without SCC and
  TT-MFP compiles the body out and returns `0x2c` (`:1123-1124`); NewTOS is
  not such a build (decision 2026-10-28).

---

## Source extraction summary

`Bconmap` maps the "current RS-232" logical device (BIOS device 1 and the
`Rsconf`/`Iorec(0)` family) onto one of several physical serial ports
(Falcon: MFP, SCC B, SCC A; TT: MFP, SCC B, TT-MFP, SCC A). `dev == -1`
returns the currently-mapped device number (`bios/serport.c:1089-1090`);
`dev == -2` returns `&bconmap_root` (`:1092-1093`), so a program can walk
the `MAPTAB` array; a valid device (`6 .. 6 + maptabsize - 1`) stores the
current vectors into the old device's slot (`:1104-1110`), installs the new
slot's vectors + `rsconfptr`/`rs232iorecptr` (`:1113-1119`), updates
`mapped_device` (`:1121`) and returns the old device number (`:1122`); any
other device returns `0L` (`:1095-1097`).

`init_bconmap` (`bios/serport.c:956-1004`): four entries filled with
`maptable_dummy`, `maptab = maptable`, `maptabsize = 1`, `mapped_device =
7` on Falcon/ARAnyM else 6 (`:962-966`); entry 0 = `maptable_mfp` =
`{bconstat1, bconin1, bcostat1, bconout1, rsconf1, &iorec1}` (`:98`, `:972`);
Falcon/MSTe/ARAnyM: `maptabsize = 3` (`:974-981`), TT: `maptabsize = 4`
(`:982-993`); every other machine keeps the one entry. Then device 1's
vectors are taken from the mapped entry (`:996-1003`).

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
- **Native (AArch64) ABI:** ID `0x2C` in `x8`; arg in `x0` (low 16 bits, a
  signed `WORD`); result in `x0`: a device number or `0` sign-extended, for
  the `-2` case a **pointer** in the full 64-bit `x0` (below 4 GiB on NewTOS).

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `dev` | `WORD` (16) — device / `-1` / `-2` |

---

## Inputs

| Position | Name  | Type   | Size | Source location      | Notes |
|----------|-------|--------|------|----------------------|-------|
| arg0     | `dev` | `WORD` | 2    | `bios/serport.c:1082`| `-1` query, `-2` table ptr, `6 .. 6+maptabsize-1` select device |

`BCONMAP_START_HANDLE = 6` — valid devices are `6 .. 6 + maptabsize - 1`.
`map_index = dev - 6` is computed in a `WORD` (`:1095`): `dev` below
`-32762` wraps to a large index, which is past the table like any other.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG (device number / pointer / `0`)
- **Encoding:**

  | `dev`   | Return | Source |
  |---------|--------|--------|
  | `-1`    | currently-mapped device number | `bios/serport.c:1089-1090` |
  | `-2`    | pointer to `bconmap_root` (`BCONMAP`) | `bios/serport.c:1092-1093` |
  | valid (`6 .. 6+maptabsize-1`) | previously-mapped device number | `bios/serport.c:1122` |
  | anything else | `0L` | `bios/serport.c:1095-1097` |
  | (build without SCC and TT-MFP only) | `0x2c` (function number) | `bios/serport.c:1124` |

### `BCONMAP` / `MAPTAB`

Layouts (m68k and LP64), EmuTOS instances and the dumper layout:
`docs/api/structures/bconmap.md`. `BCONMAP = {maptab, maptabsize,
mapped_device}`, `maptab` first (D-044); `MAPTAB` = six pointers `Bconstat`,
`Bconin`, `Bcostat`, `Bconout`, `Rsconf`, `EXT_IOREC *Iorec`.

### Side effects on global state

- Valid-device path: the old device's slot gets the current vectors, device
  1's vectors + `rsconfptr`/`rs232iorecptr` come from the new slot,
  `mapped_device` updated. `-1`, `-2` and invalid devices change nothing.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `dev` not `-1`/`-2` and outside `6 .. 6+maptabsize-1` | `bios/serport.c:1095-1097` | `0L` |

---

## Verification criteria

Measured cases (oracle tt / falcontc; NewTOS):

| Case | Call | Oracle TT | Oracle Falcon | NewTOS | Status |
|---|---|---|---|---|---|
| `bios-r1` 183, `xbios-chardev-r1` 43 | `Bconmap(-1)` | 6 | 7 | 6 | TT EQUAL; Falcon ACCEPTED (device parameter) |
| `bios-r1` 184, `xbios-chardev-r1` 44 | `Bconmap(-2)` | `&bconmap_root` | same | the `BCONMAP` in the IOREC page | PTR_RAM, EQUAL |
| `bios-r1` 185/186, `xbios-chardev-r1` 45-53 | `Bconmap(0..5, 10, 11, 0x7FFF, -3, -32768)` | 0 | 0 | 0 | EQUAL |
| `xbios-bconmap-r1` 18 | `Bconmap(-2)`, `BCONMAP` + `MAPTAB[0]` through the pointer | `maptabsize` 4, `mapped_device` 6 | 3, 7 | 1, 6 | pointers PTR_RAM EQUAL; `maptabsize` (both) and `mapped_device` (Falcon) ACCEPTED |

Not in a table (the dumper's guard refuses devices 6..9, they remap AUX on
the oracle): `Bconmap(6)` returns 6 and keeps device 1 the AUX port;
`Bconmap(7..9)` return 0 on NewTOS (oracle TT/Falcon: a switch). Host tests:
`tos-api/tests/xbios05_bconmap.rs`. The probe
`measurements/measurement-2026-10-29-xbios05/method/xcprobe.c` group B
(B01-B24) on both systems.

---

## Edge cases discovered in source

1. **`BCONMAP_AVAILABLE = (CONF_WITH_SCC || CONF_WITH_TT_MFP)`.** True on
   the reference build; then `Bconmap` exists on every machine, an ST
   included (one entry). See D-045.
2. **`BCONMAP` field order is `{maptab, maptabsize, mapped_device}`.** The old
   spec (and `_state.md`) had it reversed. `maptab` is at offset 0. See D-044.
3. **`MAPTAB.Iorec` is an `EXT_IOREC *`.** Six pointer fields; the sixth is
   the RS-232 `EXT_IOREC` pointer (not a bare `IOREC`).
4. **Vector save-on-switch.** Before installing a new slot, `Bconmap` copies
   the current low-memory vectors back into the *old* slot
   (`bios/serport.c:1104-1110`), so edits made to device 1's vectors survive
   a switch — and edits made to the old slot itself are overwritten.
5. **Default `mapped_device`.** 7 on Falcon/ARAnyM, else 6
   (`bios/serport.c:966`).
6. **`maptabsize` by machine.** 1 unless Falcon/MSTe/ARAnyM (3) or TT (4)
   (`bios/serport.c:965,981,993`); the extra entries exist whether or not
   the SCC / TT-MFP is detected (a missing port keeps `maptable_dummy`).

---

## Dependencies

### Requires (must be initialized / functional)

- `bconmap_root` initialised (`init_bconmap`, from `init_serport`).

### Used by

- Terminal programs targeting a second serial port, "select serial port"
  UIs. No ported NewTOS program calls `Bconmap`
  (`docs/analysis/abi-assumptions-check-2026-10-28.md` §5.4).
- EmuTOS' own BIOS for handles `>= 6` (`bios/bios.c:924-928`, `:968-972`,
  `:1001-1005`, `:1153-1157`): out of range → `0`, else through the entry.
  On NewTOS the handles are the device table's (`TASK/BIOS08.md`), not this
  table's.

---

## Related data structures

- `docs/api/structures/bconmap.md` — `BCONMAP`/`MAPTAB`, both ABIs.
- `docs/api/xbios/_state.md §3` — `bconmap_root`, `rsconfptr`, `rs232iorecptr`.
- `docs/api/xbios/0f-rsconf.md`, `docs/api/xbios/0e-iorec.md` — retargeted by
  `Bconmap` on EmuTOS.
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

- A program that probes `Bconmap(0) == 0x2C` (the function-number answer of
  a TOS without `Bconmap`) and falls back to handle 1 (audit
  `docs/audits/2026-10-28-xbios-chardev.md`, XBIOS-CHARDEV-005, "Fehlerbild")
  now gets `0`, as on the oracle: `Bconmap` exists.

---

## NewTOS implementation (XBIOS05, 2026-10-29)

**Decision (user, 2026-10-28, `TASK/XBIOS05.md` option 1):** the table
EmuTOS builds on a machine that is neither Falcon nor TT — one entry, the AUX
port (`bios/serport.c:962-966,972`). Code:
`tos-api/src/xbios/bconmap.rs:222@bconmap`, dispatch arm
`tos-api/src/xbios/mod.rs:98@bconmap`.

- **Answers.** `Bconmap(-1)` = 6. `Bconmap(-2)` = the address of the
  `BCONMAP`, `{maptab, 1, 6}`. `Bconmap(6)` = 6: the entry is stored back
  (the save step) and 6 stays mapped; device 1 stays the AUX port of the BIOS
  device table. Every other device = 0, `7..9` included.
- **Place.** `BCONMAP`, `MAPTAB[0]` and the five call targets lie in the
  IOREC page (`tos-api/src/bdos/process/mmu.rs:264@IOREC_PAGE_VA`, 0x7D000 + 0x80;
  `tos-api/src/xbios/iorec.rs:130@BCONMAP_AREA`): the same address in every
  process, EL0 read-write, never executable, below 4 GiB. Written with the
  page's records, before the first pointer is handed out. The address is not
  contract.
- **Layout.** LP64, as the SDK's `_BCONMAP`/`_MAPTAB`
  (`docs/api/structures/bconmap.md`): `BCONMAP` 16 bytes, `maptab` @0,
  `maptabsize` @8, `mapped_device` @10; `MAPTAB` 48 bytes, six 8-byte
  pointers. Compile-time check: `sdk/libc/tests/bconmap-layout.c`.
- **`MAPTAB[0].Iorec`** = `Iorec(0)`'s record (EmuTOS `maptable_mfp` holds
  `&iorec1`, which `Iorec(0)` returns: `bios/serport.c:98,1003`,
  `bios/xbios.c:339-340`).
- **The five function pointers are not callable** (user decision
  2026-10-28; `docs/COMPATIBILITY.md`, "`Bconmap` MAPTAB function pointers").
  Each holds the address of its own 4-byte call target in a reserved,
  never-executable stretch of the IOREC page
  (`tos-api/src/xbios/bconmap.rs:129@vector_va`). A program that calls through one
  takes an instruction abort; the crash report names it — "Memory violation:
  Bconmap MAPTAB function (not callable) memory, execute (permission fault,
  level 3)" (`tos-api/src/bios/crash.rs:196@BconmapVector`, classified in
  `tos-api/src/bdos/process/memprot.rs:1977@region_kind`) — and the program ends
  with `Pterm(-1)`. Reading the pointers is harmless. The kernel never calls
  through the table.
- **The kernel's state is its own.** `Bconmap` answers from the kernel's
  mapped device and table size and never reads the page back: a program
  that writes `maptab`, `maptabsize` or `mapped_device` changes what it
  reads, not what `Bconmap` does (EmuTOS reads `bconmap_root` live,
  `bios/serport.c:1086-1096`; documented in `docs/COMPATIBILITY.md`). A
  write into `MAPTAB[0]` is overwritten by the next `Bconmap(6)`, as
  EmuTOS' save step overwrites the old slot.
- **Device parameters (ACCEPTED per profile).** The oracle profiles are TT
  (4 entries, device 6) and Falcon (3 entries, device 7); NewTOS has one
  entry and device 6. `Bconmap(-1)` on Falcon, `maptabsize` on both and
  `mapped_device` on Falcon differ by the machine
  (`docs/COMPATIBILITY.md`, "`Bconmap`: one entry").

---

## Pi / BCM2711 binding

> Re-points the hardware reference from the Atari **multi-port serial** models
> (Falcon SCC A/B + MFP, TT ST-MFP + TT-MFP + SCC) to the **single
> BCM2711 PL011 UART0**. See `HARDWARE-PROFILE.md` §3.2.

**Peripheral / driver.** The Pi has one TOS-visible serial port — **PL011
UART0** (`kernel/src/drivers/bcm2711/pl011.rs`) behind BIOS device 1 (AUX;
on raspi4b the "no cable" entry, on qemu-virt the serial line, XBIOS15). It
is entry 0 of the table, device 6. There is no SCC and no second MFP, so the
table has no further entries — the shape EmuTOS gives a machine without them.

**Absent second serial ports.** The BCM2711 has four more PL011s (UART2..5)
and the mini-UART (DS §11.1 p.144); NewTOS drives only UART0. A second AUX
device (USB-CDC, another UART) would be a second entry (option 3 of the
decision, not chosen) and a new decision on the BIOS handles 6+.
