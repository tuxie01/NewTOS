# `Bcostat` (BIOS 0x08)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:1143` (`bcostat`).

## EmuTOS source

**Primary location:** `bios/bios.c:1143` — `LONG bcostat(WORD handle)`

**Helpers called:**
- `util/miscasm.S:213` — `protect_v(func)` — d2/a2-preserving indirect call.
- Per-handle output-status routines: `bios/parport.c:129` `bcostat0`
  (Centronics BUSY via MFP GPIP), `bios/serport.c:271` `bcostat1` (UART
  TX-empty), `bios/chardev.c:112` `bcostat2` (screen — always ready),
  `bios/midi.c` `bcostat3` (MIDI TX), `bios/ikbd.c` `bcostat4` (IKBD TX),
  `bios/chardev.c:50` `char_dummy` (returns 0).

**Headers / type definitions used:**
- `include/tosvars.h` — `bcostat_vec[8]`
- `bios/bios.h` — `boot_status`, `CHARDEV_AVAILABLE`

**System variables / globals read:**
- `boot_status` (`bios/bios.c:1149`)
- `bcostat_vec[handle]` (TOS var `0x55e`)
- `bconmap_root.maptab[]` — when `BCONMAP_AVAILABLE`

**System variables / globals written:** none in `bcostat`.

**Build configuration affecting this function:**
- `BCONMAP_AVAILABLE` — handle-remap stage for handles ≥ 6 (the reference
  build, every machine: `docs/api/xbios/2c-bconmap.md`, "Build
  configuration").
- `CONF_WITH_PRINTER_PORT` — without it, `bcostat0` returns 0 (busy).
- `CONF_SERIAL_CONSOLE` — makes `bcostat2` delegate to `bcostat(1)`.

---

## Source extraction summary

`Bcostat` reports whether an output device can accept the next byte. Identical
dispatcher shape as `Bconstat`: boot guard → optional Bconmap remap →
`bcostat_vec[handle]` via `protect_v`; else `0L`.

The default `bcostat_vec` (`bcostat_init[]`, `bios/chardev.c:38`) is
`{bcostat0, bcostat1, bcostat2, bcostat4, bcostat3, char_dummy, char_dummy,
char_dummy}` — note that **the MIDI (`bcostat3`) and IKBD (`bcostat4`) slots
are swapped** relative to their handle numbers, per the source comment
`/* note that IKBD and MIDI bcostat() are swapped! */` (`bios/chardev.c:40`).
So handle 3 → `bcostat4` (IKBD TX) and handle 4 → `bcostat3` (MIDI TX). This
swap is historical Atari behaviour applications depend on.

`bcostat2` (screen) returns `-1` unconditionally (always ready), unless
`CONF_SERIAL_CONSOLE` is set, in which case it returns `bcostat(1)` (serial
readiness). There is deliberately **no** `bcostat5` for the raw-screen output
channel — programs writing via handle 5 must query readiness with
`Bcostat(2)`.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x08
- **Stack frame on entry (m68k):**
  ```
  +2: handle (word)
   0: function_id (word) = 0x0008
  ```
- **Total parameter bytes:** 4
- **Native (AArch64) ABI:** ID `0x08` in `x8`; `handle` in `x0`; result
  (`0` or `-1`) sign-extended into `x0`.

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `handle` | `WORD` (16) |

---

## Inputs

| Position | Name     | Type | Size | Source location    | Notes |
|----------|----------|------|------|--------------------|-------|
| arg0     | `handle` | `WORD` | 2  | `bios/bios.c:1143` | `0..5` table-indexed, from 6 the Bconmap table (with `BCONMAP_AVAILABLE`; else `0..7`); else `0L` |

Handle meanings — comment `bios/bios.c:1141` documents `0:PRT 1:AUX 2:CON
3:MID 4:KEYB`, but note the swap in the actual table:

| Handle | Device (output-status) | Default routine | Notes |
|-------:|------------------------|-----------------|-------|
| 0      | PRT / Centronics       | `bcostat0` | BUSY via MFP GPIP |
| 1      | AUX / RS-232           | `bcostat1` | UART TX-empty |
| 2      | CON / screen           | `bcostat2` | always `-1` (or `bcostat(1)`) |
| 3      | **IKBD TX**            | `bcostat4` | **swapped** |
| 4      | **MIDI TX**            | `bcostat3` | **swapped** |
| 5      | raw screen             | `char_dummy` → 0 | |
| 6 .. 5+`maptabsize` | Bconmap entries (`BCONMAP_AVAILABLE`) | `maptab[handle-6].Bcostat`; entry 0 = `bcostat1` (`bios/serport.c:98,972`) | not swapped |
| from 6+`maptabsize` | past the Bconmap table | `0L` (`bios/bios.c:1153-1154`) | |

Without `BCONMAP_AVAILABLE` slots 6/7 are `char_dummy` → 0.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** `-1L` = ready to accept a byte; `0L` = busy / unavailable /
  invalid handle. Only these two values.
- **Source location:** `bios/bios.c:1157-1161`; e.g. `bcostat2` returns `-1`
  (`bios/chardev.c:117`).

### Side effects on global state

None.

### Side effects on hardware

Reads a status register (MFP GPIP for printer BUSY, UART TX-empty for
serial/MIDI/IKBD). No writes.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `CHARDEV_AVAILABLE` clear | `bios/bios.c:1149` | `0L` |
| `handle` outside `0..7` and not in Bconmap range | `bios/bios.c:1160-1162` | `0L` |
| Bconmap index ≥ `maptabsize` | `bios/bios.c:1154` | `0L` |
| Handle wired to `char_dummy` (5; 6/7 only without `BCONMAP_AVAILABLE`) | `bios/chardev.c:50` | `0L` |
| Printer busy or `!CONF_WITH_PRINTER_PORT` (handle 0) | `bios/parport.c:129-141` | `0L` |

No distinct error code — `0L` means both "busy" and "unavailable/invalid".

**Error paths not in Compendium:** the boot-guard `0L`.

---

## Verification criteria

- **Nominal (screen ready):** `Bcostat(2) == -1` whenever
  `CHARDEV_AVAILABLE` is set (screen is always ready).
- **Nominal (printer busy):** with BUSY asserted (or
  `!CONF_WITH_PRINTER_PORT`), `Bcostat(0) == 0`.
- **Swap observable:** with MIDI TX ready but IKBD TX busy, `Bcostat(4)`
  (MIDI) and `Bcostat(3)` (IKBD) report the *swapped* device readiness.
- **Invalid handle:** `Bcostat(9)` → `0`.

---

## Edge cases discovered in source

1. **MIDI/IKBD status slots are swapped.** Handle 3 → `bcostat4` (IKBD) and
   handle 4 → `bcostat3` (MIDI) — the opposite of `bconout`/`bconin`
   (`bios/chardev.c:38-41`). This is intentional historical behaviour, kept for
   application compatibility.

2. **Screen status is a constant `-1`.** `bcostat2` (`bios/chardev.c:112-118`)
   returns `-1` (always ready), or `bcostat(1)` under `CONF_SERIAL_CONSOLE`.

3. **No `bcostat5`.** The raw-screen output handle (5) has no status routine;
   `bcostat_vec[5] == char_dummy` → `0L`. Programs must use `Bcostat(2)`.

4. **`bcostat0` returns `0` when the printer port is compiled out.** Under
   `!CONF_WITH_PRINTER_PORT` it always reports busy (`bios/parport.c:140`).

5. **Any non-zero means ready, but EmuTOS returns exactly `-1`.** It is not a
   byte count or enable mask.

---

## Dependencies

### Requires (must be initialized / functional)

- `chardev_init()` — `CHARDEV_AVAILABLE` + `bcostat_vec`.

### Used by

- GEMDOS `Cconos` (0x10), `Cprnos` (0x11), `Cauxos` (0x13); output-throttling
  loops in user programs.

---

## Related data structures

- `docs/api/bios/_state.md` — `bcostat_vec[8]` (with the swap documented)

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically documents
  the swap and the `-1`/`0` encoding.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `BCONMAP_AVAILABLE` (the reference build, every machine),
  `CONF_SERIAL_CONSOLE`, `CONF_WITH_PRINTER_PORT`.

---

## NewTOS implementation notes

(Empty during Phase 00.)

FS11 / IO01 prep (2026-10-13, user decision 1): device-table dispatch and the
documented absent-device profile (deliberate, not a stub) are defined in
`docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes"; all BIOS
character calls, `Fread`/`Fwrite` on device handles and the `C*aux*`/`Cprn*`
GEMDOS calls use the **same** per-device entry. A later UART/MIDI/printer driver
(`TASK/IO01.md`) replaces only that entry.

**AUX (1) revised 2026-10-13 (option A, user decision, after FS11 acceptance finding 1; see `docs/api/gemdos/file/3f-fread.md` "NewTOS implementation notes" item 1a):** AUX is "port present, no cable" and is **no longer** part of the absent-device profile below: `Bconout(1)` → 1 (accept+discard), `Bcostat(1)` → -1, `Bconstat(1)` → 0, `Bconin(1)` → 0 non-blocking. PRN and MIDI stay absent.

**[BIOS04] 2026-10-29 — handles 3 and 4 ready, user decision "like the oracle"
(supersedes the 3/4 part of the next paragraph):** `Bcostat(3)` and
`Bcostat(4)` → **`-1`**. The oracle build has both ACIAs
(`references/emutos/include/config.h:994-996,1001-1003`); with an empty
transmit register `bcostat4` (slot 3) and `bcostat3` (slot 4) answer `-1`
(`references/emutos/bios/ikbd.c:916-930`, `references/emutos/bios/midi.c:66-81`);
EmuTOS' `bcostat4` answers `-1` "OK (but output will be ignored)" even on a
build without an IKBD ACIA. NewTOS never has a byte in transmission (output is
discarded at once), so both entries are always ready
(`tos-api/src/bios/chardev.rs:175@MidiPort`,
`tos-api/src/bios/chardev.rs:248@IkbdPort`). The swap stays in
`tos-api/src/bios/chardev.rs:446@bcostat_handle`; with both entries `-1` it is
not observable through the default entries, a host test pins it with a test
entry (`tos-api/tests/fs11_device_rw.rs` `bcostat_keeps_the_ikbd_midi_swap`).
Measured equal to the oracle: `bios-r1` cases 66/67 and `tosdump/probes/mdprobe.c`
D04/D05/D08/D09 (the audit probe's B11/B12),
`measurements/measurement-2026-10-29-bios04/RUNINFO.TXT`. Before, both answered
`0` and a program waiting `while (!Bcostat(4));` hung for good (finding
BIOS-CORE-001, `docs/audits/2026-10-28-bios-core.md`).

**[BIOS08] 2026-10-30 — handles 6 and up are the `Bconmap` table's (EmuTOS'
one-entry table, `TASK/XBIOS05.md` option 1):** the `Bconmap` branch runs
before the device table and its 3/4 swap (`references/emutos/bios/bios.c:1152-1157`).
Handle 6 is entry 0, the AUX port: `Bcostat(6)` answers as `Bcostat(1)`,
through the same AUX entry (`-1`). Handles from 7 up answer `0`
(`map_index >= maptabsize`, `:1153-1154`), as EmuTOS does on a machine with
one entry (`references/emutos/bios/serport.c:965`). Implementation
`tos-api/src/bios/chardev.rs:411@device`,
`tos-api/src/xbios/bconmap.rs:109@entry_device`. Against the oracle:

| Case | Call | Oracle TT | Oracle Falcon | NewTOS (before → after) | Status |
|---|---|---|---|---|---|
| `bios-r1` 69 | `Bcostat(6)` | -1 (`bcostat1`) | -1 | 0 → -1 | EQUAL |
| `bios-r1` 70 | `Bcostat(7)` | -1 (SCC B, `serport.c:993`) | -1 (`:981`) | 0 → 0 | ACCEPTED (no port 7) |
| `bios-core-r1` 65 | `Bcostat(8)` | -1 (TT-MFP) | -1 (SCC A) | 0 → 0 | ACCEPTED (no port 8) |
| `bios-core-r1` 66 | `Bcostat(9)` | -1 (SCC A, 4 entries) | 0 (3 entries) | 0 → 0 | TT ACCEPTED, Falcon EQUAL |

A machine with one entry answers 0 for 7..9 under the same ROM
(`references/emutos/bios/bios.c:1153-1154` with `maptabsize` 1,
`references/emutos/bios/serport.c:965`): the differences are the port count,
a device parameter (`docs/COMPATIBILITY.md`, "`Bconmap`: one entry").
`measurements/measurement-2026-10-30-bios08/RUNINFO.TXT`.

**Devices 0 (PRN), 3, 4 (IKBD-TX/MIDI-TX):** `Bcostat` -> `0` (not ready). **Device 1 (AUX), option A:** `Bcostat(1)` -> **-1** (ready) always; EmuTOS `bcostat1` returns -1 while the out-ring has room (`bios/serport.c:271-283`).
Consistency with EmuTOS: dev 0 `bcostat0` returns 0 without
`CONF_WITH_PRINTER_PORT` (`bios/parport.c:139-140`, exact match); dev 1: superseded by option A (was dummy map entry, `bios/serport.c:96`; now agrees with `bcostat1`, -1); MIDI `bcostat3` returns 0 without ACIA
(`bios/midi.c:78-79`), same value. **The slot swap must survive the device table:**
in EmuTOS `bcostat_init[3] = bcostat4` (IKBD) and `[4] = bcostat3` (MIDI)
(`bios/chardev.c:38-42`), unlike Bconin/Bconout. Both are 0 today, so the swap is
unobservable; a future MIDI driver (`TASK/IO01.md`) must install its *status*
function in slot 4 and the IKBD one in slot 3 (or the table must carry the swap
explicitly). `Cauxos`/`Cprnos` call `Bcostat(1)`/`Bcostat(0)` through the same entries.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (per-handle TX-ready routines reading
> MFP/SCC/ACIA transmitter flags) at the NewTOS device-handle map. The
> **contract is unchanged** — `-1` = ready to accept a byte, `0` = busy /
> absent; and the historical **MIDI/IKBD status-slot swap** (handle 3 →
> `bcostat4` IKBD, handle 4 → `bcostat3` MIDI). Peripheral + driver: see
> `docs/HARDWARE-PROFILE.md` §2, §3.1, §3.2.

### The Pi device-handle map (output status)

| Handle | TOS device (per swap) | Pi binding | Status |
|-------:|-----------------------|------------|--------|
| 0 | PRT / Centronics | **absent** | `0` (busy) |
| 1 | AUX / RS-232 | no-cable port entry (option A); PL011 UART0 later | `-1` (ready) |
| 2 | CON / screen | **framebuffer VT52** | `-1` (always ready) |
| 3 | **IKBD TX** (swapped) | IKBD port entry (BIOS04) | `-1` (ready) |
| 4 | **MIDI TX** (swapped) | MIDI port entry (BIOS04) | `-1` (ready) |
| 5 | raw screen | **`char_dummy` slot** | `0` (busy) |
| 6 | Bconmap entry 0 = AUX (BIOS08) | the AUX entry, as handle 1 (not swapped) | `-1` (ready) |
| 7.. | past the Bconmap table | — | `0` |

- **CON output always ready — handle 2 only.** The framebuffer VT52 console
  never blocks — a glyph is composited synchronously — so **CON (handle 2)**
  returns `-1` (`tos-api/src/bios/chardev.rs` `bcostat`). This matches the
  Atari `bcostat2 == -1` "screen always ready". No hardware TX FIFO to poll
  (unlike the Shifter-era screen, which was also always ready).

  **Correction (2026-09-05):** an earlier revision of the table above gave
  handle 5 (raw screen) `-1` as well, i.e. the same handle set as `Bconout`.
  That is wrong: `bcostat_init[5]` is `char_dummy`
  (`references/emutos/bios/chardev.c:38-41`), so `Bcostat(5)` is `0` even
  though `Bconout(5, …)` writes and returns `1`. The asymmetry is part of the
  contract, not a NewTOS shortcut; the code implements it. Measured by
  `APITEST` checks BC12 (`Bconout(5) → 1`) and BC20 (`Bcostat(5) → 0`).
- **The MIDI/IKBD swap on the Pi.** *Superseded by BIOS04 (2026-10-29): both
  entries answer `-1`, the swap is kept in `bcostat_handle` (see "NewTOS
  implementation notes").* The swap is a property of the *handle→
  routine* table on Atari; on the Pi both MIDI and IKBD-TX are **absent**
  (§3.9 — no MIDI ACIA, no outbound IKBD command channel), so handles 3 and
  4 both return `0` (busy) regardless of the swap. The swap is therefore
  **unobservable** on this hardware (both swapped slots resolve to "absent →
  0"); the contract's swap semantics are preserved in the sense that neither
  handle 3 nor 4 is a live device. NewTOS `chardev.rs` treats only handles 2
  and 5 as console output; 0/1/3/4/6/7 fall to the `_ => 0` arm
  (`chardev.rs:76-82`), so no explicit swap table is needed. Recorded as a
  NOTE in the Step-2 section (swap is contract-relevant only if MIDI/IKBD
  ever come back).
- **AUX (handle 1).** Returned `0` (dummy) before option A; **now `-1`**. Earlier text: returns `0` (dummy) today — the PL011 exists but is not
  exposed as a BIOS AUX device (see `03-bconout.md`); its TX is always ready
  in practice (the driver drains by polling), so a proper AUX binding would
  return `-1`. Recorded as K-series.
- **Datasheet.** Framebuffer transport = VideoCore mailbox (§1.3); AUX =
  PL011 UART0 (datasheet §11).
