# `Bconstat` (BIOS 0x01)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:914` (`bconstat`).

## EmuTOS source

**Primary location:** `bios/bios.c:914` — `LONG bconstat(WORD handle)`

**Helpers called:**
- `util/miscasm.S:213` — `protect_v(func)` — calls the handler through the
  device vector, saving/restoring `d2`/`a2` around it (old asm drivers used
  `d0-d2/a0-a2` as scratch, violating the C ABI).
- The per-handle status routines the vector points at, e.g.
  `bios/serport.c:255` `bconstat1` (RS-232), `bios/ikbd.c:235` `bconstat2`
  (IKBD/console), `bios/midi.c` `bconstat3` (MIDI), `bios/chardev.c:50`
  `char_dummy` (returns 0).

**Headers / type definitions used:**
- `include/tosvars.h` — `bconstat_vec[8]`, `bconin_vec` etc.
- `bios/bios.h` — `boot_status`, `CHARDEV_AVAILABLE` (`0x10`)
- `bios/serport.h` — `bconmap_root`, `BCONMAP_START_HANDLE` (when
  `BCONMAP_AVAILABLE`)

**System variables / globals read:**
- `boot_status` (`bios/bios.c:920`) — must have `CHARDEV_AVAILABLE` set
- `bconstat_vec[handle]` (TOS var `0x51e`) — the per-handle status routine
- `bconmap_root.maptab[]` / `.maptabsize` — when `BCONMAP_AVAILABLE`

**System variables / globals written:** none directly (the handler it calls
may touch device state).

**Build configuration affecting this function:**
- `BCONMAP_AVAILABLE` — inserts the handle-remap stage for handles ≥
  `BCONMAP_START_HANDLE` (6). The reference build has it on every machine,
  with `maptabsize` 1 unless Falcon/MSTe/ARAnyM (3) or TT (4)
  (`docs/api/xbios/2c-bconmap.md`, "Build configuration"). Without it,
  handles 0..7 index `bconstat_vec` directly and anything else returns `0L`.
- `CONF_WITH_PRINTER_PORT`, `CONF_SERIAL_CONSOLE_POLLING_MODE`,
  `CONF_ATARI_HARDWARE`/`CONF_WITH_MFP` — affect the *handlers* the vector
  points at, not `bconstat` itself (see per-handler notes).

---

## Source extraction summary

`Bconstat` reports whether a character-input device has data ready. It is a
three-stage dispatcher: (1) if `chardev_init()` has not run yet
(`CHARDEV_AVAILABLE` clear in `boot_status`) it returns `0L`; (2) under
`BCONMAP_AVAILABLE`, handles ≥ `BCONMAP_START_HANDLE` index the
`bconmap_root.maptab[]` remap table (out-of-range → `0L`); (3) otherwise, for
`handle` in `0..7`, it calls `bconstat_vec[handle]` through `protect_v`.
Anything else returns `0L`.

The actual "ready" test lives in the per-handle routine, not here. The
routines are installed by `chardev_init()` (`bios/chardev.c:63`) from the
`bconstat_init[]` table: `{char_dummy, bconstat1, bconstat2, bconstat3,
char_dummy, char_dummy, char_dummy, char_dummy}`. So by default only handles
1 (AUX/RS-232), 2 (CON/IKBD) and 3 (MIDI) have real status routines; handles
0, 4, 5, 6, 7 map to `char_dummy` which always returns `0L`. With
`BCONMAP_AVAILABLE` (the reference build) slots 6 and 7 are never read:
handle 6 is `maptab[0].Bconstat` = `bconstat1` (`bios/serport.c:98,972`),
handles from `6 + maptabsize` up return `0L` (`bios/bios.c:923-928`).

Every default status routine is heavily Atari-hardware gated: `bconstat1`
reads the MFP/SCC receiver flag; `bconstat2` (`bios/ikbd.c:235`) compares
`ikbdiorec.head`/`.tail` (or polls the serial port under
`CONF_SERIAL_CONSOLE_POLLING_MODE`). These are behaviour reference, not
contract — NewTOS supplies equivalent status routines for its own devices.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x01
- **Stack frame on entry (m68k):**
  ```
  +2: handle (word)
   0: function_id (word) = 0x0001
  ```
- **Total parameter bytes:** 4
- **Native (AArch64) ABI:** ID `0x01` in `x8`; `handle` in `x0`; result
  sign-extended into `x0` (`0` or `-1`).

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `handle` | `WORD` (16-bit) |

---

## Inputs

| Position | Name     | Type | Size | Source location    | Notes |
|----------|----------|------|------|--------------------|-------|
| arg0     | `handle` | `WORD` | 2  | `bios/bios.c:914`  | device handle; only `0..7` are table-indexed |

Handle meanings (default table, `bios/chardev.c:30`):

| Handle | Device (status)             | Default routine |
|-------:|-----------------------------|-----------------|
| 0      | PRT / Centronics            | `char_dummy` → always 0 |
| 1      | AUX / RS-232                | `bconstat1` |
| 2      | CON / IKBD (console input)  | `bconstat2` |
| 3      | MIDI in                     | `bconstat3` |
| 4      | (IKBD-in, historical)       | `char_dummy` → 0 |
| 5      | raw CON (no input)          | `char_dummy` → 0 |
| 6 .. 5+`maptabsize` | Bconmap entries (`BCONMAP_AVAILABLE`) | `maptab[handle-6].Bconstat`; entry 0 = `bconstat1` (the MFP port) |
| from 6+`maptabsize` | past the Bconmap table | `0L` (`bios/bios.c:925-926`) |

Without `BCONMAP_AVAILABLE` handles 6/7 are `char_dummy` slots and the
validation is `handle >= 0 && handle <= 7` (`bios/bios.c:931`).

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `-1L` = data available; `0L` = no data / unavailable /
  invalid handle. Any handler is expected to return only these two values.
- **Source location:** `bios/bios.c:928-933` (dispatch); the value is the
  handler's return.

### Side effects on global state

None in `bconstat` itself. (Handlers only read device state.)

### Side effects on hardware

The default handlers read hardware status registers (MFP GPIP, SCC/ACIA
status). No writes, no state change.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `chardev_init` not yet run (`CHARDEV_AVAILABLE` clear) | `bios/bios.c:920` | `0L` |
| `handle` outside `0..7` and not in Bconmap range | `bios/bios.c:931-933` | `0L` |
| Bconmap index ≥ `maptabsize` (when `BCONMAP_AVAILABLE`) | `bios/bios.c:925` | `0L` |
| Handle wired to `char_dummy` | `bios/chardev.c:50` | `0L` |

There is **no distinct error code**: `0L` doubles as "not ready", "invalid
handle" and "device unavailable". Callers needing to distinguish must know
the device is present.

**Error paths not in Compendium:** the "boot-stage before `chardev_init`
returns 0" guard is EmuTOS-specific and invisible to normal applications
(they run long after boot).

---

## Verification criteria

- **Nominal (ready):** with `CHARDEV_AVAILABLE` set and a byte queued for a
  handle whose status routine reports ready, `Bconstat(handle) == -1`.
- **Nominal (empty):** same handle, no byte queued → `Bconstat(handle) == 0`.
- **Invalid handle:** `Bconstat(9)` (no Bconmap) → `0`.
- **Before chardev init:** `Bconstat(2)` with `CHARDEV_AVAILABLE` clear
  → `0`.

---

## Edge cases discovered in source

1. **Negative handle path.** With `BCONMAP_AVAILABLE`, `map_index = handle -
   BCONMAP_START_HANDLE` for a negative `handle` is even more negative, so
   the `map_index >= 0` test fails and control falls through to the
   `handle >= 0` guard, returning `0L`. Without `BCONMAP_AVAILABLE`, the
   `handle >= 0` guard catches it directly.

2. **`bconstat2` is IKBD queue, not "keyboard hardware".** It compares
   `ikbdiorec.head == ikbdiorec.tail` (`bios/ikbd.c:242`). Under
   `CONF_SERIAL_CONSOLE_POLLING_MODE` it instead delegates to `bconstat(1)`
   (serial). Either way it is a queue/flag test, never a physical key scan.

3. **Handle 0 (PRT) has no input.** `bconstat_vec[0] == char_dummy`;
   printer-port input is not implemented (`bconin0` is a stub too,
   `bios/parport.c:123`). Always `0L`.

4. **Vector overwrite is the extension mechanism.** `bconstat_vec` sits at a
   fixed TOS address (`0x51e`); programs install custom drivers by writing
   the slot. This is contract for m68k binaries (Phase 10); native NewTOS
   ports use a NewTOS-native registration path.

5. **`protect_v` does not catch faults.** A buggy handler crashes inside the
   BIOS call; there is no sandbox.

---

## Dependencies

### Requires (must be initialized / functional)

- `chardev_init()` (`bios/chardev.c:63`) — sets `CHARDEV_AVAILABLE` and
  populates `bconstat_vec`.
- For handle 1: `init_serport()`; for handle 2: `kbd_init()` +
  `init_acia_vecs()`; for handle 3: `midi_init()`.

### Used by

- GEMDOS `Cconis` (0x0B), `Cauxis` (0x12); `constat()` in `bdos/console.c`;
  every typeahead-aware GEMDOS reader.

---

## Related data structures

- `docs/api/bios/_state.md` — the four `bcon*_vec[8]` tables and `boot_status`
- `docs/api/structures/iorec.md` — `ikbdiorec` queue read by `bconstat2`

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: agrees
  on `-1`/`0` encoding.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `BCONMAP_AVAILABLE` (the reference build, every machine) adds the
  handle-remap stage.
- `CONF_SERIAL_CONSOLE_POLLING_MODE` makes `bconstat2` poll RS-232 instead of
  the IKBD queue.
- Handler availability depends on `CONF_WITH_MFP`/`CONF_WITH_SCC`/
  `CONF_ATARI_HARDWARE` (which devices exist).

---

## NewTOS implementation notes

(Empty during Phase 00. Filled in during Phase 02 / Phase 07.)

FS11 / IO01 prep (2026-10-13, user decision 1): device-table dispatch and the
documented absent-device profile (deliberate, not a stub) are defined in
`docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes"; all BIOS
character calls, `Fread`/`Fwrite` on device handles and the `C*aux*`/`Cprn*`
GEMDOS calls use the **same** per-device entry. A later UART/MIDI/printer driver
(`TASK/IO01.md`) replaces only that entry.

**AUX (1) revised 2026-10-13 (option A, user decision, after FS11 acceptance finding 1; see `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes" item 1a):** AUX is "port present, no cable" and is **no longer** part of the absent-device profile below: `Bconout(1)` → 1 (accept+discard), `Bcostat(1)` → -1, `Bconstat(1)` → 0, `Bconin(1)` → 0 non-blocking. PRN and MIDI stay absent.

**[XBIOS15] 2026-10-29 — qemu-virt only:** AUX is a serial driver over the input
ring of device 0 (`Iorec(0)`), filled by the PL011 receive interrupt as EmuTOS'
`push_serial_iorec` (`bios/serport.c:331-343`): `Bconstat(1)` → `-1` while a byte
waits, else `0` (`bconstat_iorec`, `:191-199`); `Bconin(1)` **waits** for a byte
and returns it zero-extended, as EmuTOS' `bconin1` (`:260-263`, `bconin_iorec`
`:202-210`); `Bconout(1)`/`Bcostat(1)` unchanged (1, discarded / -1).
Implementation `kernel/src/console.rs:216@AuxSerial`, installed at boot by
`install_aux`; measured equal to the oracle
(`docs/audits/2026-10-29-xbios15-serial-rx.md` SERRX-001,
`measurements/measurement-2026-10-29-xbios15/RUNINFO.TXT`). Real Pi 4 keeps the
"no cable" entry above (serial input is dropped there).

**[BIOS04] 2026-10-29 — MIDI (3) and IKBD (4), user decision "like the
oracle":** `Bconstat(3)` is EmuTOS' `bconstat3` over the MIDI ring of
`Iorec(2)` (`references/emutos/bios/midi.c:22-33`): `-1` while a byte waits,
else `0`. No kernel producer fills that ring (no MIDI driver), so the answer
is `0` unless a program put a byte there through the `Iorec(2)` record — then
`-1`, as on the oracle (probe `tosdump/probes/mdprobe.c` R02, equal on both
sides, `measurements/measurement-2026-10-29-bios04/RUNINFO.TXT`).
`Bconstat(4)` stays `char_dummy` → `0` (`bconstat_init[4]`,
`references/emutos/bios/chardev.c:30-32`). Implementation
`tos-api/src/bios/chardev.rs:175@MidiPort`, `tos-api/src/bios/chardev.rs:248@IkbdPort`.

**[BIOS08] 2026-10-30 — handles 6 and up are the `Bconmap` table's (EmuTOS'
one-entry table, `TASK/XBIOS05.md` option 1):** the `Bconmap` branch runs
before the device table (`references/emutos/bios/bios.c:923-928`). Handle 6
is entry 0, the AUX port: `Bconstat(6)` answers as `Bconstat(1)`, through the
same AUX entry (`0` on `board-rpi4`, the serial input ring on
`board-qemu-virt`). Handles from 7 up answer `0` (`map_index >= maptabsize`,
`:925-926`), as EmuTOS does on a machine with one entry
(`references/emutos/bios/serport.c:965`). Implementation
`tos-api/src/bios/chardev.rs:411@device`,
`tos-api/src/xbios/bconmap.rs:109@entry_device`; the device table ends at 5.
Measured: `bios-r1` 36 (`Bconstat(6)`), `bios-core-r1` 20-22
(`Bconstat(7..9)`) EQUAL (`0` on both sides: the oracle's ports have no
byte waiting); `measurements/measurement-2026-10-30-bios08/RUNINFO.TXT`.

**Devices 0 (PRN), 1 (AUX), 3 (MIDI):** `Bconstat` → `0` (no byte; for AUX unchanged by option A: EmuTOS `bconstat1` is also 0 on empty iorec, `serport.c:255-258`). Consistency
with EmuTOS (read this session): dev 0 = `char_dummy` (`bios/chardev.c:31`);
dev 1 = dummy Bconmap entry `char_dummy` (`bios/serport.c:96`) — EmuTOS' own
`bconstat1` is the real iorec test (`serport.c:255-258`) and is what a later
driver provides; dev 3: EmuTOS `bconstat3` returns 0 while its iorec is empty
(`bios/midi.c:23-33`), same value. The "Pi / BCM2711 binding" paragraph below
calling AUX-as-dummy a "minor divergence / K-series" is superseded: it is now the
decided, documented profile until `TASK/IO01.md`.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (per-handle status routines reading MFP/
> SCC/ACIA receiver flags and the `ikbdiorec` queue) at the NewTOS
> device-handle map. The **contract is unchanged** — `-1` = byte ready, `0`
> = not ready / absent / boot-guard. Peripheral + driver: see
> `docs/HARDWARE-PROFILE.md` §2, §3.2, §3.3.

### The Pi device-handle map

| Handle | TOS device | Pi binding | Status source |
|-------:|------------|------------|---------------|
| 0 | PRT / Centronics | **absent** → always `0` | no printer (§3.9) |
| 1 | AUX / RS-232 | no-cable port entry (option A); PL011 UART0 later (`TASK/IO01.md`) | always `0` (no input) |
| 2 | CON | framebuffer VT52 + **USB-HID keyboard** | console RX ring |
| 3 | MIDI in | MIDI port entry (BIOS04); no MIDI driver yet | the `Iorec(2)` MIDI ring (`bconstat3`) |
| 4 | IKBD in (historical) | IKBD port entry (BIOS04), input slot `char_dummy` → always `0` | `char_dummy` |
| 5 | raw CON | no input → `0` | — |
| 6 | Bconmap entry 0 = AUX (BIOS08) | the AUX entry, as handle 1 | as handle 1 |
| 7.. | past the Bconmap table | `0` | — |

- **What "ready" tests on the Pi.** For **CON (handle 2 — the only
  console-input handle)** `bconstat` returns `-1` iff the console RX ring is
  non-empty (`tos-api/src/bios/chardev.rs` `is_console_in` →
  `Host::console_has_input` → `console::rx_has_input`,
  `kernel/src/console.rs`). This is the queue/flag test the contract calls
  for — never a physical key scan. Absent devices (0/1-as-dummy/5/7..) and
  the IKBD input slot (**4**) return `0`, the `char_dummy` equivalent; MIDI
  (3) tests the `Iorec(2)` ring (BIOS04).

  **Correction (2026-09-05):** an earlier revision of this section claimed
  handle 4 (IKBD-in) was "aliased to the console keyboard ring". It is not,
  and must not be: EmuTOS' status table is
  `bconstat_init[4] = char_dummy` (`references/emutos/bios/chardev.c:30-32`) —
  handle 4 is the keyboard-controller **TX** channel, whose *input* slot is a
  dummy. Treating it as console input made `Bconin(4)` block forever waiting
  for a key that never routes there. The code returns `0` for handle 4;
  measured by `APITEST` checks BC05 / BC09. The boot-guard (`CHARDEV_AVAILABLE`) maps to the
  "is a host installed" check (`host::host()` → `0` before it is).
- **CON input is USB-HID, not the UART (policy).** Handle 2's ready-test
  reads the console RX ring, which is fed by the USB-HID keyboard decoder
  (`ikbd.rs:139` → `console::push_rx`). Per `HARDWARE-PROFILE.md` §3.2 the
  console must not depend on the PL011 UART — but today the **same ring is
  also filled from UART0** (cite stale 2026-10-26: console.rs:91-107, UART0 RX feeding the console ring), so `Bconstat(2)` currently
  reports ready for UART bytes too. Recorded as K1 (Step-2).
- **AUX (handle 1).** In this spec's table handle 1 is `bconstat1`
  (RS-232). NewTOS `chardev.rs` treats handle 1 as an **absent/dummy** input
  (`is_console_in` returns true only for 2 and 4, `chardev.rs:25-27`), so
  `Bconstat(1)` returns `0` even though the PL011 exists and could back AUX (value stays 0 under option A, "port present, no cable").
  A minor divergence — the UART is a debug channel, and its bytes currently
  leak into the CON ring instead of a proper AUX record. Recorded as
  K-series. **[XBIOS15] 2026-10-29:** no longer — on `board-rpi4` the bytes
  are dropped, on `board-qemu-virt` they fill device 0's ring and AUX reads
  them (`Bconstat(1)` -1 while one waits; note above).
- **Datasheet.** Keyboard transport = VL805 xHCI (§2); AUX = PL011 UART0,
  BCM2711 peripherals datasheet §11.
