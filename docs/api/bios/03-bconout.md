# `Bconout` (BIOS 0x03)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:991` (`bconout`).

## EmuTOS source

**Primary location:** `bios/bios.c:991` — `LONG bconout(WORD handle, WORD what)`

**Helpers called:**
- `util/miscasm.S:240` — `protect_ww(func, handle, what)` — d2/a2-preserving
  indirect call for a two-WORD-argument handler.
- Per-handle output routines: `bios/parport.c:144` `bconout0` (Centronics,
  long-timeout, ctrl-C escapable), `bios/serport.c:289` `bconout1` (RS-232),
  `bios/chardev.c:91` `bconout2` (screen via `cputc`), `bios/midi.c`
  `bconout3` (MIDI out), `bios/ikbd.c` `bconout4` (IKBD command out),
  `bios/chardev.c:98` `bconout5` (raw screen via `ascii_out`),
  `bios/chardev.c:55` `charout_dummy` (slots 6,7 → 0; with
  `BCONMAP_AVAILABLE` never reached, the Bconmap table answers handles ≥ 6).

**Headers / type definitions used:**
- `include/tosvars.h` — `bconout_vec[8]`
- `bios/bios.h` — `boot_status`, `CHARDEV_AVAILABLE`

**System variables / globals read:**
- `boot_status` (`bios/bios.c:997`)
- `bconout_vec[handle]` (TOS var `0x57e`)
- `bconmap_root.maptab[]` — when `BCONMAP_AVAILABLE`

**System variables / globals written:** none in `bconout`; handler 2/5
mutate the VT52/console state (cursor, escape parser) and screen RAM.

**Build configuration affecting this function:**
- `BCONMAP_AVAILABLE` — handle-remap stage for handles ≥ 6 (the reference
  build, every machine: `docs/api/xbios/2c-bconmap.md`, "Build
  configuration").
- `CONF_WITH_PRINTER_PORT` — without it, `bconout0`/`bcostat0` are inert.
- `CONF_SERIAL_CONSOLE` — makes `bconout5` also echo to handle 1.

---

## Source extraction summary

`Bconout` writes one character to an output device. Same three-stage
dispatcher as the other char functions, with one difference: the handlers
receive **both** the original `handle` and the character (`protect_ww`
forwards two WORDs), so a single C function can service several handles
(e.g. `bconout5` can delegate to `bconout(1, …)`).

Default routines (`bconout_init[]`, `bios/chardev.c:43`):
`{bconout0, bconout1, bconout2, bconout3, bconout4, bconout5, charout_dummy,
charout_dummy}`. Unlike the status/input tables, all of 0..5 have real
routines here.

Return-value convention is **not** the `-1/0` of the status calls. Screen and
serial/MIDI handlers return `1L` on success (`bconout2` → `return 1L`,
`bios/chardev.c:94`). The parallel handler `bconout0` is the exception: it
returns `0L` when the printer is busy/timed-out and the `prnout()` result
otherwise (`bios/parport.c:144-172`). This asymmetry is real and observable.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x03
- **Stack frame on entry (m68k):**
  ```
  +4: what   (word)
  +2: handle (word)
   0: function_id (word) = 0x0003
  ```
- **Total parameter bytes:** 6
- **Native (AArch64) ABI:** ID `0x03` in `x8`; `handle` in `x0`, `what` in
  `x1`; result in `x0`.

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `handle` | `WORD` (16-bit) |
  | +4         | `x1`        | `what`   | `WORD` (16-bit) |

---

## Inputs

| Position | Name     | Type | Size | Source location   | Notes |
|----------|----------|------|------|-------------------|-------|
| arg0     | `handle` | `WORD` | 2  | `bios/bios.c:991` | `0..5` table-indexed, from 6 the Bconmap table (with `BCONMAP_AVAILABLE`; else `0..7`) |
| arg1     | `what`   | `WORD` | 2  | `bios/bios.c:991` | byte to output; only low 8 bits used (`bconout5` does `LOBYTE(ch)`, `bios/chardev.c:104`) |

Handle meanings (default `bconout_vec`):

| Handle | Device (output)                | Default routine |
|-------:|--------------------------------|-----------------|
| 0      | PRT / Centronics               | `bconout0` |
| 1      | AUX / RS-232                   | `bconout1` |
| 2      | CON / screen (VT52)            | `bconout2` |
| 3      | MIDI out                       | `bconout3` |
| 4      | IKBD command out               | `bconout4` |
| 5      | raw screen (no VT52)           | `bconout5` |
| 6 .. 5+`maptabsize` | Bconmap entries (`BCONMAP_AVAILABLE`) | `maptab[handle-6].Bconout`; entry 0 = `bconout1` (`bios/serport.c:98,972`) |
| from 6+`maptabsize` | past the Bconmap table | `0L` (`bios/bios.c:1001-1003`) |

Without `BCONMAP_AVAILABLE` slots 6/7 are `charout_dummy` → 0.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding:** device-dependent (see table). `1L` = byte accepted for most
  handlers; `0L` = printer busy (handle 0 only), or boot-guard / invalid
  handle.
- **Source location:** the handler; e.g. `bios/chardev.c:94` (`return 1L`).

| Handler | Return | Source |
|---------|--------|--------|
| `bconout0` (PRT) | `0L` on busy/timeout; `prnout(c)` result on send | `bios/parport.c:144-172` |
| `bconout1` (AUX) | `1L` after byte transmitted | `bios/serport.c:289` |
| `bconout2` (CON) | `1L` | `bios/chardev.c:91-95` |
| `bconout3` (MIDI) | `1L` | `bios/midi.c` |
| `bconout4` (IKBD) | `1L` | `bios/ikbd.c` |
| `bconout5` (raw)  | `1L` | `bios/chardev.c:98-106` |
| `charout_dummy` (6,7 without `BCONMAP_AVAILABLE`) | `0L` | `bios/chardev.c:55` |
| Bconmap entry 0 (handle 6) | `bconout1`: `1L` | `bios/bios.c:1004-1005`, `bios/serport.c:98` |
| boot too early / invalid handle | `0L` | `bios/bios.c:997,1008-1010` |

### Side effects on global state

- Handle 2/5: VT52 console state (cursor position, escape-parser state) and
  screen RAM change via `cputc`/`ascii_out` (`bios/conout.c`).

### Side effects on hardware

- Handle 0/1/3/4: writes the byte to the printer/UART/ACIA data register.
- Handle 2/5: writes glyph cells to the framebuffer.
- Handle 0: may spin up to a long timeout waiting on the Centronics BUSY line.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `CHARDEV_AVAILABLE` clear | `bios/bios.c:997` | `0L` |
| `handle` outside `0..7` and not in Bconmap range | `bios/bios.c:1008-1010` | `0L` |
| Handle 0 printer busy / timed out | `bios/parport.c:171` | `0L` |
| Bconmap index ≥ `maptabsize` (when `BCONMAP_AVAILABLE`) | `bios/bios.c:1001-1003` | `0L` |
| Handles 6,7 (`charout_dummy`, without `BCONMAP_AVAILABLE`) | `bios/chardev.c:55` | `0L` |

`bconout0`'s `0L` is a genuine "not sent, retry" signal, distinct from the
other handlers' guaranteed success — the only handler where the return value
carries information a caller must act on.

**Error paths not in Compendium:** the printer-busy `0L` return and the
boot-guard `0L`.

---

## Verification criteria

- **Nominal (screen):** `Bconout(2, 'A')` returns `1` and places 'A' at the
  cursor (advancing the VT52 cursor).
- **Nominal (raw):** `Bconout(5, 'A')` returns `1` and writes 'A' literally
  (no VT52 escape interpretation).
- **Printer busy:** with the Centronics BUSY line asserted (or
  `!CONF_WITH_PRINTER_PORT`), `Bconout(0, 'A')` returns `0`.
- **Invalid handle:** `Bconout(6, 'A')` (no Bconmap) returns `0`; with
  `BCONMAP_AVAILABLE` `Bconout(6, 'A')` is `bconout1` (`1`) and
  `Bconout(6 + maptabsize, 'A')` returns `0`.

---

## Edge cases discovered in source

1. **Handlers get the handle too.** `protect_ww` forwards `handle` *and*
   `what` (`bios/bios.c:1005,1009`). This is why `bconout5` can call
   `bconout(1, ch)` to mirror to serial (`bios/chardev.c:100-103`) — it is not
   limited to its own channel.

2. **Handle 5 is "raw screen", not a separate device.** `bconout5` writes the
   same screen as handle 2 but via `ascii_out(LOBYTE(ch))` with **no** VT52
   escape parsing (`bios/chardev.c:104`). There is no `bcostat5` — the
   symmetric status must use `Bcostat(2)`.

3. **Only the low byte is emitted.** `bconout5` masks with `LOBYTE`; the other
   handlers likewise ignore the high byte of `what`.

4. **Handle 4 is a keyboard-controller command channel**, not user text —
   `bconout4` sends bytes to the IKBD. The higher-level API is `Ikbdws`
   (XBIOS 0x19). NewTOS does not interpret them (XBIOS04, "NewTOS
   implementation notes").

5. **Printer output can block for a long time.** `bconout0` waits up to
   `LONG_TIMEOUT` for BUSY to clear, polling `bconin2()` for ctrl-C to abort
   (`bios/parport.c:159-166`) — an EmuTOS convenience over Atari TOS.

---

## Dependencies

### Requires (must be initialized / functional)

- `chardev_init()` — `CHARDEV_AVAILABLE` + `bconout_vec`.
- Handle 2/5: screen initialised (`v_bas_ad`, `cputc`/`ascii_out` state).
- Handle 1: `init_serport()`. Handle 0: `parport_init()` (+
  `CONF_WITH_PRINTER_PORT`).

### Used by

- Every GEMDOS output call (`Cconout`, `Cauxout`, `Cprnout`, `Cconws`, …);
  kernel `kprintf()` (via handle 2); AES/VDI text output.

---

## Related data structures

- `docs/api/bios/_state.md` — `bconout_vec[8]`, VT52 console state
- `docs/api/_line-a.md` — VT52 cursor / colour variables handler 2 touches

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: agrees on
  the `(handle, char)` signature.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- ~~`CONF_WITH_VT52`~~ — **Korrektur (CON04, 2026-10-22):** dieses Makro gibt es in EmuTOS nicht (keine Fundstelle in `references/emutos`). Der VT52-Parser (`bios/vt52.c`) ist unbedingt kompiliert. Vollstaendige Spec des Automaten, aller Folgen und des Handle-2/5-Unterschieds: [`_vt52.md`](_vt52.md).
- `CONF_SERIAL_CONSOLE`: `bconout5` also writes handle 1.
- `CONF_WITH_PRINTER_PORT`: handle 0 active (else `0L`).
- `BCONMAP_AVAILABLE`: handles ≥ 6 via remap table (the reference build,
  every machine).

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

**[BIOS04] 2026-10-29 — MIDI (3) and IKBD (4) accept, user decision "like the
oracle" (supersedes the device-3 part of the next paragraph):** `Bconout(3,b)`
and `Bconout(4,b)` → `1`, the byte discarded — the oracle build has both ACIAs
(`CONF_WITH_MIDI_ACIA`, `CONF_WITH_IKBD_ACIA` = 1,
`references/emutos/include/config.h:994-996,1001-1003`): `bconout3` waits for
`bcostat3` (TDRE), writes the ACIA and returns `1L`
(`references/emutos/bios/midi.c:84-95`); `bconout4` calls `ikbd_writeb` and
returns `1L` (`references/emutos/bios/ikbd.c:933-937`). NewTOS has no MIDI or
IKBD transmitter and no IKBD command interpreter, so the byte has no effect
(`docs/api/xbios/19-ikbdws.md`). Implementation
`tos-api/src/bios/chardev.rs:175@MidiPort`,
`tos-api/src/bios/chardev.rs:248@IkbdPort`. Measured equal to the oracle:
`tosdump/probes/mdprobe.c` D06/D07 (the audit probe's B02/B03),
`measurements/measurement-2026-10-29-bios04/RUNINFO.TXT`. Before, both
returned `0` (finding BIOS-CORE-001, `docs/audits/2026-10-28-bios-core.md`).

**[XBIOS04] 2026-10-30 — IKBD (4): the byte is ignored, with one warning
per process (user decision, TASK/XBIOS04 option 1).** `Bconout(4, c)` still
answers `1` and drops `c`: NewTOS has no IKBD-command interpreter, so no
command takes effect and no response packet is made
(`docs/api/xbios/19-ikbdws.md`, `docs/COMPATIBILITY.md` "IKBD commands are
not interpreted"). Never silent (ADR-031 rule 1): the first IKBD byte of a
process writes one serial line `[hook] warning: slot <n> (PD …) sent IKBD
command bytes via Bconout(4); IKBD commands ignored: …`. The warning key is
the one `Ikbdws` uses, so a process warns once for both calls together; a new
process warns again. Only the IKBD entry warns (a driver installed in slot 4
would replace it, `install_device`); MIDI (3) does not. Implementation
`tos-api/src/bios/chardev.rs:259@bconout` (`IkbdPort`),
`tos-api/src/bdos/process/hooks.rs:724@note_ikbd_ignored`; host test
`tos-api/tests/bios15_os_hooks.rs`
(`ikbd_commands_warn_once_per_process_for_ikbdws_and_bconout_4`).

**[BIOS08] 2026-10-30 — handles 6 and up are the `Bconmap` table's (EmuTOS'
one-entry table, `TASK/XBIOS05.md` option 1):** the `Bconmap` branch runs
before the device table (`references/emutos/bios/bios.c:1000-1005`). Handle 6
is entry 0, the AUX port: `Bconout(6,b)` is `Bconout(1,b)`, through the same
AUX entry (`1`, the byte discarded; a driver in slot 1 gets the byte).
Handles from 7 up return `0` and send nothing (`map_index >= maptabsize`,
`:1001-1003`), as EmuTOS does on a machine with one entry
(`references/emutos/bios/serport.c:965`). Implementation
`tos-api/src/bios/chardev.rs:411@device`,
`tos-api/src/xbios/bconmap.rs:109@entry_device`. DUMPBIOS refuses
`Bconout(6..9)`; measured with the audit probe's B04/B05
(`Bconout(6/7,'A')`): `1`/`0` on NewTOS, oracle TT and Falcon `1`/`1` —
`Bconout(7)` is the oracle's SCC B port, which NewTOS does not have (device
parameter, ACCEPTED,
`docs/COMPATIBILITY.md` "`Bconmap`: one entry");
`measurements/measurement-2026-10-30-bios08/RUNINFO.TXT`. Before, both
returned `0`.

**Devices 0 (PRN), 3 (MIDI):** `Bconout` -> `0`, byte discarded
(`charout_dummy` semantics, `bios/chardev.c:55-58`). **Device 1 (AUX), option A:** `Bconout(1,b)` -> `1`, byte discarded; EmuTOS `bconout1` with an MFP port returns `1L` (`bios/serport.c:323`; wait loop `:296` on `bcostat1` = ring space, no CTS; `FLOW_CTRL_NONE` default `:89`, `flowctrl` unused). Consistency with EmuTOS: dev 0 without
`CONF_WITH_PRINTER_PORT` returns 0 at once (`bios/parport.c:144-172`, exact match).
**Inconsistent with EmuTOS' own `bconout3`** (documented divergence, semantics not
changed): without `CONF_WITH_MIDI_ACIA` it loops `while(!bcostat3())` and
`bcostat3` returns 0 (`bios/midi.c:78-79,86-87`), i.e. **never returns**; the
table row "`bconout3` (MIDI) -> `1L`" holds only with an ACIA. (Superseded for dev 1:
the earlier rule followed EmuTOS `bconout1` *without* an RS-232 backend, which returns 0 (`bios/serport.c:326-327`); option A follows the MFP-present return 1 instead.) The `bconout` wrapper also
returns 0 for a handle past the Bconmap table (`bios/bios.c:1001-1003`; > 7
without `BCONMAP_AVAILABLE`, `:1008-1010`) and before
`CHARDEV_AVAILABLE` (`:997-998`).

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`bconout2` calling `cputc`/VT52 over the
> Shifter screen; `bconout1` to the ACIA/UART; `bconout4` to the IKBD) at the
> NewTOS device-handle map, where CON output is the **framebuffer VT52
> emulator** (`fbcon.rs`). The **contract is unchanged** — `1L` = byte
> accepted for console handles, `0L` for absent devices. Peripheral +
> driver: see `docs/HARDWARE-PROFILE.md` §2, §3.1, §3.2.

### The Pi device-handle map (output)

| Handle | TOS device | Pi binding | Return |
|-------:|------------|------------|--------|
| 0 | PRT / Centronics | **absent** → discard | `0` |
| 1 | AUX / RS-232 | no-cable port entry (option A); PL011 UART0 later (`TASK/IO01.md`) | `1` (accept+discard) |
| 2 | CON / screen (VT52) | **framebuffer VT52** (`fbcon.rs` → `vdi/escape.rs` automaton) + UART echo | `1` |
| 3 | MIDI out | MIDI port entry (BIOS04) → discard | `1` |
| 4 | IKBD command out | IKBD port entry (BIOS04) → discard | `1` |
| 5 | raw screen (no VT52) | framebuffer `ascii_out` (`alpha_rawout`), no automaton, no UART echo | `1` |
| 6 | Bconmap entry 0 = AUX (BIOS08) | the AUX entry, as handle 1 | `1` (accept+discard) |
| 7.. | past the Bconmap table | nothing sent | `0` |

- **CON output = framebuffer VT52.** Handle 2 writes via
  `Host::console_write_byte`, which the kernel routes to **both** the
  framebuffer VT52 console (`kernel/src/fbcon.rs::write_byte` →
  `tos_api::vdi::alpha_conout`, the automaton of [`_vt52.md`](_vt52.md)) and
  the PL011 UART (a debug mirror, not the primary console). Handle 5 goes to
  the console's `ascii_out` directly (`tos_api::vdi::alpha_rawout`): every
  byte a glyph, no automaton, no UART mirror (EmuTOS `bconout5` without
  `CONF_SERIAL_CONSOLE`). Return `1` per `bconout2`/`bconout5`.
- **VT52 scope.** Since CON04 (2026-10-22) the full EmuTOS automaton:
  every sequence, cursor, colours, wrap, Handle 5 raw — [`_vt52.md`](_vt52.md)
  §15; deviations Q3/Q5/Q6/D4 in `docs/COMPATIBILITY.md`.
- **Handle 1 (AUX) discarded today.** `is_console_out` is true only for
  handles 2 and 5 (`chardev.rs:29-32`), so `Bconout(1, …)` returns `0`
  (dummy; **superseded by option A: returns `1`, byte discarded**) even though the PL011 exists — the UART is written only as the
  CON debug mirror, not as a proper AUX device. Minor divergence, recorded
  as K-series.
- **Absent PRT (handle 0).** Returns `0` (discarded), matching §3.9 — no
  printer. Note the `char_dummy` semantics differ from `bconout`'s
  `1L`-on-CON: an absent device returns `0`, an honest "not sent".
- **MIDI/IKBD-out (handles 3/4), BIOS04.** Return `1` and discard: the
  reference machine always has both ACIAs, so its programs see "sent"; on
  the Pi there is no MIDI ACIA and no IKBD command channel behind it. The
  first IKBD byte of a process writes the "IKBD commands ignored" warning
  (XBIOS04).
- **Datasheet.** Framebuffer transport = VideoCore mailbox (§1.3); AUX =
  PL011 UART0 (datasheet §11).
