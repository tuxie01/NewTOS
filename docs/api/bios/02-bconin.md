# `Bconin` (BIOS 0x02)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/bios.c:958` (`bconin`).

## EmuTOS source

**Primary location:** `bios/bios.c:958` — `LONG bconin(WORD handle)`

**Helpers called:**
- `util/miscasm.S:213` — `protect_v(func)` — d2/a2-preserving indirect call.
- Per-handle input routines: `bios/parport.c:123` `bconin0` (**stub**,
  returns 0), `bios/serport.c:260` `bconin1` (RS-232, blocking),
  `bios/ikbd.c:250` `bconin2` (IKBD queue dequeue, blocking),
  `bios/midi.c` `bconin3` (MIDI, blocking), `bios/chardev.c:50` `char_dummy`
  (returns 0).

**Headers / type definitions used:**
- `include/tosvars.h` — `bconin_vec[8]`
- `bios/bios.h` — `boot_status`, `CHARDEV_AVAILABLE`
- `bios/iorec.h` — `ikbdiorec` (queue read by `bconin2`)

**System variables / globals read:**
- `boot_status` (`bios/bios.c:964`)
- `bconin_vec[handle]` (TOS var `0x53e`)
- `bconmap_root.maptab[]` — when `BCONMAP_AVAILABLE`
- `conterm` (TOS var `0x484`, `bios/ikbd.c:279`) — `bconin2` only: bit 3
  gates whether the `shifty` byte is folded into the returned LONG's top
  byte (see Outputs; gap filled this pass, was previously undocumented)

**System variables / globals written:** none in `bconin`; `bconin2` advances
`ikbdiorec.head` (`bios/ikbd.c:269`).

**Build configuration affecting this function:**
- `BCONMAP_AVAILABLE` — handle-remap stage for handles ≥ 6 (the reference
  build, every machine: `docs/api/xbios/2c-bconmap.md`, "Build
  configuration").
- `CONF_SERIAL_CONSOLE_POLLING_MODE` — makes `bconin2` read RS-232 and
  synthesise an IKBD record via `ikbdiorec_from_ascii` (`bios/ikbd.c:255`).

---

## Source extraction summary

`Bconin` reads (and consumes) one character from an input device, blocking
until one is available. Structure is identical to `Bconstat`: boot guard →
optional Bconmap remap → `bconin_vec[handle]` via `protect_v`; out-of-range
returns `0L`.

The default routines (`bconin_init[]`, `bios/chardev.c:34`) are
`{bconin0, bconin1, bconin2, bconin3, char_dummy×4}`. Blocking is a property
of the underlying routine, not of `bconin`: `bconin1`/`bconin2`/`bconin3`
spin until their device delivers a byte. `bconin2` (`bios/ikbd.c:250`) busy-
waits on `bconstat2()`, disables interrupts (`set_sr(0x2700)`), advances
`ikbdiorec.head` by 4 and reads the 4-byte record — so IKBD input is a
32-bit packed record, not a raw byte.

The **return encoding** is documented in the header comment at
`bios/bios.c:948-956`: for CON (handle 2) the high word carries the GSX-2.0
scancode in its low byte and the ASCII value is in the low byte of the low
word (0 if the key is non-ASCII); for AUX (handle 1) the character is in the
low byte only.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x02
- **Stack frame on entry (m68k):**
  ```
  +2: handle (word)
   0: function_id (word) = 0x0002
  ```
- **Total parameter bytes:** 4
- **Native (AArch64) ABI:** ID `0x02` in `x8`; `handle` in `x0`; the packed
  LONG result in `x0` (see encoding below).

  | m68k stack | AArch64 reg | Name     | Type / size |
  |------------|-------------|----------|-------------|
  | +2         | `x0`        | `handle` | `WORD` (16-bit) |

---

## Inputs

| Position | Name     | Type | Size | Source location   | Notes |
|----------|----------|------|------|-------------------|-------|
| arg0     | `handle` | `WORD` | 2  | `bios/bios.c:958` | `0..5` table-indexed, from 6 the Bconmap table (with `BCONMAP_AVAILABLE`); else `0L` |

Handle meanings (default `bconin_vec`, `bios/chardev.c:34`):

| Handle | Device (input)         | Default routine | Blocking? |
|-------:|------------------------|-----------------|-----------|
| 0      | PRT (parallel)         | `bconin0` (stub → 0) | no |
| 1      | AUX / RS-232           | `bconin1`       | yes |
| 2      | CON / IKBD             | `bconin2`       | yes |
| 3      | MIDI in                | `bconin3`       | yes |
| 4, 5   | IKBD in / raw CON in   | `char_dummy` → 0 | no |
| 6 .. 5+`maptabsize` | Bconmap entries (`BCONMAP_AVAILABLE`) | `maptab[handle-6].Bconin`; entry 0 = `bconin1` (`bios/serport.c:98,972`) | yes (entry 0) |
| from 6+`maptabsize` | past the Bconmap table | `0L` (`bios/bios.c:968-969`) | no |

Without `BCONMAP_AVAILABLE` slots 6/7 are `char_dummy` (no blocking) and
`0..7` index `bconin_vec`.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** LONG
- **Encoding (CON, handle 2):** bits 0–7 = ASCII (0 if non-ASCII key);
  bits 8–15 = 0; bits 16–23 = GSX-2.0 IKBD scancode; bits 24–31 = the
  `shifty` modifier byte **latched at key-press time** — *if and only if*
  `conterm` bit 3 (`0x08`) is set — **else 0**. See the correction below:
  this is not the unconditional "never folded" behaviour a prior draft
  claimed.
- **Encoding (AUX, handle 1 / MIDI, handle 3):** byte in bits 0–7, upper
  bits 0.
- **Source location:** header comment `bios/bios.c:948-956`; value produced by
  the handler (`bconin2` packs the 4-byte `ikbdiorec` record and then masks
  it, `bios/ikbd.c:279-280`).

**Correction (this pass):** a prior draft stated flatly "shift state is not
folded into this word by `bconin`; use `Kbshift`". That is only true for the
**default** boot configuration. The actual code
(`bios/ikbd.c:279-280`):

```
if (!(conterm & 8))         /* shift status not wanted? */
    value &= 0x00ffffffL;   /* true, so clean it out */
```

masks bits 24–31 to zero **only when `conterm` bit 3 is clear**. `conterm`
(TOS sysvar `0x484`, a directly-pokeable `UBYTE`) is initialised to `7` at
boot (`bios/ikbd.c:1100`, "keyclick, key repeat & system bell on by
default") — bit 3 is *not* one of those three, so it is clear by default,
and the masking *does* happen out of the box. But a program that sets
`conterm |= 8` (poking `$484` directly, or via whatever TOS exposes for it)
changes `Bconin(2)`'s contract: the returned LONG then carries the `shifty`
snapshot from the moment the key was generated (`kb_last.k.shifty`, set at
`bios/ikbd.c:900`) in its top byte, on top of the existing
scancode/ASCII packing. This is real, source-backed, observable behaviour
that the prior spec missed entirely; `conterm` has been added to
`docs/api/bios/_state.md`'s sysvar table.

### Side effects on global state

- `bconin2` advances `ikbdiorec.head` (consumes one 4-byte queue record,
  `bios/ikbd.c:269-273`), wrapping at `ikbdiorec.size`.

### Side effects on hardware

Reads UART/ACIA data registers (which clears the receiver-ready flag). No
device configuration change.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| `CHARDEV_AVAILABLE` clear | `bios/bios.c:964` | `0L` |
| `handle` outside `0..7` and not in Bconmap range | `bios/bios.c:975-977` | `0L` |
| Bconmap index ≥ `maptabsize` (when `BCONMAP_AVAILABLE`) | `bios/bios.c:968-969` | `0L` |
| Handle wired to `char_dummy` (4, 5; 6/7 only without `BCONMAP_AVAILABLE`), or 0 via stub | `bios/chardev.c:50` | `0L` |

**No distinct error code.** `0L` is ambiguous: it means "no device" *and* is a
legitimate result — a genuine NUL byte, or (handle 2) a non-ASCII key whose
ASCII byte is 0 with the scancode in the high word. Callers use `Bconstat`
first when the distinction matters.

**Error paths not in Compendium:** the boot-guard `0L`.

---

## Verification criteria

- **Nominal (AUX):** with a byte `0x41` ('A') available on RS-232,
  `Bconin(1) == 0x00000041`.
- **Nominal (CON, ASCII):** pressing 'A' → low byte `0x41`, high word = its
  scancode; `Bconin(2) & 0xFF == 0x41`.
- **Nominal (CON, non-ASCII):** pressing an arrow/function key → low byte 0,
  bits 16–23 = scancode; `Bconin(2) & 0xFF == 0` while `(Bconin(2) >> 16) &
  0xFF != 0`.
- **Consumption:** two successive `Bconstat(2)` reads without an intervening
  `Bconin(2)` still report ready for the same byte; `Bconin(2)` then removes
  it and `Bconstat(2) == 0`.
- **Invalid handle:** `Bconin(9)` → `0`.
- **`conterm` bit 3 set (gap-filled):** with `conterm |= 8` and a real key
  press with Shift held, `Bconin(2) >> 24 == shifty-at-press-time` (a
  non-zero byte with `MODE_LSHIFT`/`MODE_RSHIFT` set); with `conterm & 8 ==
  0` (the boot default) the same press yields `Bconin(2) >> 24 == 0`.

---

## Edge cases discovered in source

1. **`bconin0` (PRT input) is a stub.** `bios/parport.c:123` returns `0`
   unconditionally with a `/* not implemented */` comment. The old spec's
   claim that it "reads the parport status/data lines" is incorrect.

2. **IKBD input is a 4-byte packed record.** `bconin2` reads a full `ULONG`
   from `ikbdiorec.buf` (`bios/ikbd.c:273`), not a byte — the packing is what
   carries the scancode in the high word. The queue advances in units of 4.

3. **`bconin2` disables interrupts during dequeue.** `set_sr(0x2700)` around
   the head advance (`bios/ikbd.c:267`) makes the dequeue atomic against the
   IKBD ISR. This is the concurrency contract the queue relies on.

4. **Blocking, but the system keeps running.** The busy-wait in `bconin2`
   optionally issues `stop_until_interrupt()` (`USE_STOP_INSN_TO_FREE_HOST_CPU`)
   so timer/VBL/keyboard interrupts continue; the machine does not freeze.

5. **No timeout on AUX.** `bconin1` blocks indefinitely on a stuck UART; only
   the IKBD path has a reset-driven recovery.

6. **`conterm` bit 3 conditionally folds `shifty` into bits 24–31 (gap
   filled this pass).** `bconin2` always builds the full 32-bit record with
   the modifier byte in the top 8 bits, then masks it to 0 unless `conterm &
   8` (`bios/ikbd.c:279-280`). The default boot value `conterm == 7`
   (`bios/ikbd.c:1100`) has bit 3 clear, so out of the box the mask always
   applies and bits 24–31 read 0 — which is why a prior draft's blanket "not
   folded" claim looked correct under casual testing. A program (or the
   Desktop) that sets `conterm |= 8` changes this. This is the one instance
   in this function group where a plain TOS system variable, not a build
   `CONF_*` flag, silently changes a trap's return-value contract.

---

## Dependencies

### Requires (must be initialized / functional)

- `chardev_init()` — `CHARDEV_AVAILABLE` + `bconin_vec`.
- Handle 2: `kbd_init()`, `init_acia_vecs()`, live `ikbdiorec`.
- Handle 1: `init_serport()`.

### Used by

- GEMDOS `Cconin` (0x01), `Cnecin` (0x08), `Crawcin` (0x07), `Cauxin` (0x03),
  `Crawio` (0x06, read mode) — the console-input paths.

---

## Related data structures

- `docs/api/structures/iorec.md` — `ikbdiorec` (the 4-byte-record queue)
- `docs/api/bios/_state.md` — `bconin_vec[8]`

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Historically: agrees on
  the scancode/ASCII packing and on blocking.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- `BCONMAP_AVAILABLE` (the reference build, every machine).
- `CONF_SERIAL_CONSOLE_POLLING_MODE`: `bconin2` reads RS-232 and synthesises
  the IKBD record.

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
**[XBIOS18] 2026-10-29:** the wait gives way to a `kbdvec` hook as the CON wait
does (KEYS02, `docs/api/xbios/22-kbdvbase.md` C3 "O1"): when the running
process has bytes queued for its hook, the trap entry
(`AuxSerial::bconin_restartable`, `kernel/src/console.rs:267@aux_read_blocking`)
returns without a byte, the trap restarts after the hook ran and waits again —
so the hook is called while `Bconin(1)` waits, as EmuTOS' IKBD interrupt calls
it during `bconin1`'s spin (`bios/aciavecs.S:407-414`). Measured equal to the
oracle with `tosdump/probes/axprobe.c`
(`measurements/measurement-2026-10-29-xbios18/RUNINFO.TXT`).

**[BIOS04] 2026-10-29 — MIDI (3) waits, IKBD (4) answers 0 (user decision "like
the oracle"; supersedes the device-3 part of the next paragraph):**
`Bconin(3)` is EmuTOS' `bconin3` (`references/emutos/bios/midi.c:35-62`): it
**waits** until the MIDI ring of `Iorec(2)` holds a byte (`while
(!bconstat3())`, `:37-38`), then advances `ibufhd` and returns the byte
zero-extended (`:46-57`). No kernel producer fills the ring (no MIDI driver,
`TASK/IO01.md`), so on an empty ring the call waits for good — as the oracle
does with nothing on the MIDI cable; a byte a program put into the ring is
returned at once. The wait has the AUX wait's style (XBIOS15/XBIOS18): the
input is pumped meanwhile (`Host::pump_input`), the CPU is not given to another
process, and on the trap path it gives way to a `kbdvec` hook (restart, nothing
consumed; `tos-api/src/bios/chardev.rs:221@midi_read`). Before a host is
installed it returns `0` (the boot gate). `Bconin(4)` stays `char_dummy` → `0`
at once. Measured with `tosdump/probes/mdprobe.c` (R03-R05 ring read, W01 still
waiting after 10 s; equal on both sides,
`measurements/measurement-2026-10-29-bios04/RUNINFO.TXT`); DUMPBIOS refuses
`Bconin(3)` (`tosdump/core/cases.c` `CR_BCONIN`: it may wait).

**[BIOS08] 2026-10-30 — handles 6 and up are the `Bconmap` table's (EmuTOS'
one-entry table, `TASK/XBIOS05.md` option 1):** the `Bconmap` branch runs
before the device table (`references/emutos/bios/bios.c:967-972`). Handle 6
is entry 0, the AUX port: `Bconin(6)` is `Bconin(1)`, through the same AUX
entry and its trap entry — it **waits like `Bconin(1)`**: on
`board-qemu-virt` for a byte in the serial input ring, giving way to a
`kbdvec` hook (XBIOS15/XBIOS18); on `board-rpi4` the no-cable entry returns
`0` at once, as `Bconin(1)` does there. Handles from 7 up return `0` without
waiting (`map_index >= maptabsize`, `:968-969`), as EmuTOS does on a machine
with one entry (`references/emutos/bios/serport.c:965`; the oracle's TT has
further ports there, whose `Bconin` would wait). Implementation
`tos-api/src/bios/chardev.rs:411@device`,
`tos-api/src/xbios/bconmap.rs:109@entry_device`. DUMPBIOS refuses
`Bconin(6..9)` (they may wait). `board-rpi4`: measured with the audit's hang
probe (`bcprobe.c`, handles 0, 1, 6, 7: all return `0` at once,
`measurements/measurement-2026-10-30-bios08/RUNINFO.TXT`); that handle 6
reaches the trap entry of whatever driver sits in slot 1 (the qemu-virt
wait) is pinned by `tos-api/tests/bios08_bconmap_handles.rs`.

**Devices 0 (PRN), 1 (AUX), 3 (MIDI):** `Bconin` -> `0`, **non-blocking** (AUX: unchanged by option A; a **documented divergence** from a real present port, where `bconin1` blocks forever, `serport.c:260-263`).
Consistency with EmuTOS: dev 0 `bconin0` returns 0 (`bios/parport.c:123-127`,
exact match); dev 1 = dummy Bconmap entry `char_dummy` (`bios/serport.c:96`).
**Inconsistent with EmuTOS' own routines** for 1 and 3 (documented divergence,
semantics not changed): `bconin1` blocks until a byte arrives
(`serport.c:260-263`, `bconin_iorec` loop `:202-205`) and `bconin3` spins on
`bconstat3` (`bios/midi.c:35-38`) -- without an ACIA/UART it would never return.
NewTOS never blocks on an absent device. Consequence: `Bconin(1)` can be tested
without hang risk (the 2026-09-05 APITEST note in `docs/STATUS.md` skipped it for
that reason). Table rows above marked "blocking: yes" for 1/3 describe the real
drivers only.

## Pi / BCM2711 binding

> Re-points the *hardware reference* (`bconin2` dequeuing a 4-byte
> `ikbdiorec` record; `bconin1` reading the UART/ACIA) at the NewTOS
> device-handle map. The **contract is unchanged** — blocking read, and for
> CON (handle 2) the **scancode in bits 16-23, ASCII in bits 0-7** packing.
> Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2, §3.2, §3.3.

- **Device-handle map:** as in `01-bconstat.md` — **CON (2) alone** reads the
  console key ring fed by the USB-HID keyboard; MIDI (3) waits for its
  `Iorec(2)` ring (BIOS04, above); handle 6 reads the AUX entry as handle 1
  does (BIOS08, above); PRT (0)/AUX (1, `board-rpi4`)/**IKBD (4)**/5/7.. are
  `char_dummy` and return `0` without blocking
  (EmuTOS `bconin_init[4] = char_dummy`,
  `references/emutos/bios/chardev.c:34-36`). An earlier revision of this
  section listed handle 4 as a second console-input handle; that was wrong
  and made `Bconin(4)` hang. Measured by `APITEST` check BC09.
- **How a byte is read.** Console-input handles block via
  `Host::console_read_byte_blocking` (`tos-api/src/bios/chardev.rs:49-55`),
  which spins/`wfi` until the RX ring delivers a byte
  (`kernel/src/host_env.rs:83-147`) and returns the **ASCII low byte**
  (`console::rx_pop`, `kernel/src/console.rs:120-127`, which skips
  scancode-only keys with ascii 0).
- **CON scancode high word — FIXED (was K2 Step-2).** The contract for CON
  (handle 2) is a packed LONG with the **GSX/IKBD scancode in bits 16-23**;
  a program pressing a non-ASCII key (arrow/function) must read ASCII `0`
  with a non-zero scancode in the high word. NewTOS `bconin` now reads the
  whole key word via `Host::console_read_key_blocking` (the scancode-carrying
  `rx_pop_key` path, not the ASCII-only `rx_pop`) and packs it with
  `chardev::pack_console_key` — scancode into bits 16-23, ASCII into bits 0-7
  (`tos-api/src/bios/chardev.rs`). So non-ASCII keys are visible to `Bconin(2)`
  and ASCII keys carry their scancode. The earlier K2 finding ("scancode
  dropped") no longer holds. Shift-state folding into bits 24-31 (the
  `conterm & 8` behaviour) **is** implemented as well. **[XBIOS01/BIOS11,
  2026-10-29]** The key ring is the IKBD `IOREC` (`docs/api/xbios/0e-iorec.md`,
  "NewTOS implementation"): its 32-bit entries carry the shift state **of the
  key press** (EmuTOS `bios/ikbd.c:898-909`), and
  `tos-api/src/bios/chardev.rs:513@pack_console_key` keeps bits 24-31 only when
  the calling process's `conterm` (`$484`) has bit 3 set, else clears them
  (`value &= 0x00ffffff`, `bios/ikbd.c:279-280`). Until XBIOS01 the fold read
  the *current* `Kbshift` state at read time (`shift_fold`, removed) — a Shift
  released before the read was lost (BIOS11, probe K16: now `0x02300042` as
  EmuTOS). The boot default `conterm = 7` yields bits 24-31 = 0. (An earlier
  revision of this bullet said the fold was missing.) Measured by `APITEST`
  check BC28 and `measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`.
- **CON input is USB-HID, not the UART — K1 is closed on `board-rpi4`.** The
  blocking read drains USB-HID only (`crate::drivers::usb::poll` in
  `kernel/src/host_env.rs`); there is no `pl011_rx_poll` (an earlier revision
  of this bullet named one). On `board-rpi4` a byte arriving on UART0 is
  **dropped** rather than queued for CON (`kernel/src/console.rs`
  `serial_rx_byte` under `#[cfg(not(feature = "board-qemu-virt"))]`), because
  this board is EmuTOS' `CONF_SERIAL_CONSOLE = 0` profile. **[XBIOS15]
  2026-10-29:** `board-qemu-virt` is that profile too — its console keyboard
  is the USB `usb-kbd`; serial bytes go into device 0's ring and are read
  through AUX (`Bconin(1)`), never into the key ring. (Before, qemu-virt was
  declared the `CONF_SERIAL_CONSOLE = 1` profile because it had "no other
  console input device" — wrong, the USB keyboard was there,
  `docs/audits/2026-10-29-xbios15-serial-rx.md` SERRX-003.) **Consequence for testing:** QEMU's `raspi4b` has no emulated
  USB-HID keyboard, so `Bconin(2)`/`Cconin` cannot be satisfied there at all
  — the interactive `Bconin` checks are real-hardware-only.
- **Datasheet.** Keyboard transport = VL805 xHCI (§2); AUX = PL011 UART0
  (datasheet §11).
