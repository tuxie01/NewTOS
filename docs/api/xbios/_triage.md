# XBIOS triage — D5

> Phase 00 deliverable. Source: EmuTOS @ `1a7484ab` —
> `bios/xbios.c:1285` (`xbios_vecs[]`). Build profile per `CLAUDE.md`:
> `CONF_WITH_TT_SHIFTER` + `CONF_WITH_VT52`, **disable** `CONF_WITH_VIDEL`,
> `CONF_WITH_DSP`, `CONF_WITH_DMASOUND`, `CONF_WITH_MFP`,
> `CONF_WITH_PRINTER_PORT`, `CONF_WITH_NVRAM`, and the Atari-hardware
> device configs (no floppy/ACSI, no YM2149, no ACIA).
>
> **Exception (D-060):** `CONF_WITH_PRINTER_PORT` off means EmuTOS's
> `setprt()` (0x21) is not compiled and its `xbios_vecs[]` slot falls back
> to `xbios_unimpl` (`bios/xbios.c:1332-1336`) — but `setprt()` itself never
> touches hardware (`bios/parport.c:49-57`); only the paired `prnout()`
> output path needs the (absent) Centronics/PSG wiring. NewTOS honours the
> `Setprt` config-word contract regardless of the printer-port gate; see
> `docs/api/xbios/21-setprt.md`.

For NewTOS, `LAST_ENTRY = 0x57` — the XBIOS table covers 0x00..0x57
(88 entries). Functions outside the build profile (Falcon DSP / DMA sound /
Videl / MFP / etc.) sit in the table as `xbios_unimpl`, which returns the
function number itself — **not** a GEMDOS error (see *D-008* / *D-039*).

## Status legend

- **✅ spec** — full per-function spec written (`docs/api/xbios/<NN>-<name>.md`
  or a group file)
- **🟢 implement** — NewTOS will implement (PHASE-04/07/08)
- **🟡 stub** — return `EUNDEV`/`0`/default; ABI surface only
- **⛔ unimpl** — `xbios_unimpl` (returns the function number); not built
- **➡ dispatch** — present in table but always routes elsewhere

| #     | Name       | EmuTOS handler                | NewTOS plan | Spec | Notes |
|------:|------------|-------------------------------|-------------|------|-------|
| 0x00  | Initmous   | `Initmous` (`mouse.c:40`)     | 🟢 PHASE-07 (USB HID) | ✅ | mouse-mode + `mousevec` configurator |
| 0x01  | Ssbrk      | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | obsolete since TOS 1.0 |
| 0x02  | Physbase   | `physbase` (`screen.c:1042`)  | 🟢 PHASE-08 | ✅ | physical screen base |
| 0x03  | Logbase    | `logbase` (`screen.c:1071`)   | 🟢 PHASE-08 | ✅ | `v_bas_ad` |
| 0x04  | Getrez     | `getrez` (`screen.c:1076`)    | 🟢 PHASE-08 | ✅ | resolution code |
| 0x05  | Setscreen  | `setscreen` (`screen.c:1102`) | 🟢 PHASE-08 | ✅ | reconfigure display |
| 0x06  | Setpalette | `setpalette` (`screen.c:1168`)| 🟢 PHASE-08 | ✅ | full 16-entry palette (deferred to VBL) |
| 0x07  | Setcolor   | `setcolor` (`screen.c:1192`)  | 🟢 PHASE-08 | ✅ | single-entry palette |
| 0x08  | Floprd     | `floprd`                      | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | no floppy |
| 0x09  | Flopwr     | `flopwr`                      | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | no floppy |
| 0x0A  | Flopfmt    | `flopfmt`                     | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | no floppy |
| 0x0B  | (used_by_bios) | `xbios_unimpl`            | ⛔ unimpl | (dispatcher) | reserved |
| 0x0C  | Midiws     | `midiws` (`midi.c:104`)       | 🟡 stub | ✅ | no MIDI ACIA (PHASE-07?) |
| 0x0D  | Mfpint     | `mfpint`                      | ⛔ unimpl | (dispatcher) | `CONF_WITH_MFP` off |
| 0x0E  | Iorec      | `iorec` (`bios/xbios.c:336`)       | 🟢 PHASE-07 | ✅ | input ring descriptor |
| 0x0F  | Rsconf     | `rsconf` (`bios/xbios.c:376`)      | 🟡 stub / PL011 | ✅ | RS-232 config |
| 0x10  | Keytbl     | `keytbl` (`ikbd.c:122`)       | 🟢 PHASE-07 | ✅ | keyboard-table swap |
| 0x11  | Random     | `random` (`bios/xbios.c:421`)      | 🟢 PHASE-07 | ✅ | LCG, seeds from `hz_200` |
| 0x12  | Protobt    | `protobt`                     | 🟢 PHASE-04 | ✅ (`_floppy-stubs.md`) | boot-sector image builder (software) |
| 0x13  | Flopver    | `flopver`                     | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | no floppy |
| 0x14  | Scrdmp     | `scrdmp` (`bios/xbios.c:490`)      | 🟡 stub / PNG | ✅ | screen dump; `dump_vec` default no-op |
| 0x15  | Cursconf   | `cursconf` (`vt52.c:893`)     | 🟢 PHASE-08 | ✅ | VT52 cursor config |
| 0x16  | Settime    | `settime` (`clock.c:1218`)    | 🟢 PHASE-07 | ✅ | RTC + GEMDOS time globals |
| 0x17  | Gettime    | `gettime` (`clock.c:1266`)    | 🟢 PHASE-07 | ✅ | RTC read |
| 0x18  | Bioskeys   | `bioskeys` (`ikbd.c:136`)     | 🟢 PHASE-07 | ✅ | reload default key tables |
| 0x19  | Ikbdws     | `ikbdws` (`ikbd.c:948`)       | 🟡 stub | ✅ | send raw IKBD command |
| 0x1A  | Jdisint    | `jdisint`                     | ⛔ unimpl | (dispatcher) | `CONF_WITH_MFP` off |
| 0x1B  | Jenabint   | `jenabint`                    | ⛔ unimpl | (dispatcher) | `CONF_WITH_MFP` off |
| 0x1C  | Giaccess   | `giaccess` (`sound.c`)        | 🟡 stub → 0 | ✅ (`_psg-group.md`) | no YM2149 |
| 0x1D  | Offgibit   | `offgibit` (`sound.c`)        | 🟡 stub → void | ✅ (`_psg-group.md`) | no YM2149 |
| 0x1E  | Ongibit    | `ongibit` (`sound.c`)         | 🟡 stub → void | ✅ (`_psg-group.md`) | no YM2149 |
| 0x1F  | Xbtimer    | `xbtimer`                     | ⛔ unimpl | (dispatcher) | `CONF_WITH_MFP` off |
| 0x20  | Dosound    | `dosound` (`sound.c`)         | 🟡 stub → 0 | ✅ (`_psg-group.md`) | no YM2149 |
| 0x21  | Setprt     | `setprt` (`parport.c:49-57`)  | 🟢 implement | ✅ (`21-setprt.md`) | pure software config word (D-060); `CONF_WITH_PRINTER_PORT` off does **not** apply — no HW touch |
| 0x22  | Kbdvbase   | `kbdvbase` (`bios/xbios.c:716`)    | 🟢 PHASE-07 | ✅ | returns `&kbdvecs` |
| 0x23  | Kbrate     | `kbrate` (`ikbd.c:439`)       | 🟢 PHASE-07 | ✅ | repeat-rate config |
| 0x24  | Prtblk     | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | no printer |
| 0x25  | Vsync      | `vsync` (`screen.c:1205`)     | 🟢 PHASE-08 | ✅ | wait for VBL |
| 0x26  | Supexec    | `supexec` (`bios/vectors.S:1376`)  | 🟢 PHASE-06 (ADR) | ✅ | run callback in supervisor mode |
| 0x27  | Puntaes    | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | drop AES (boot path) |
| 0x28  | (reserved) | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | |
| 0x29  | Floprate   | `floprate`                    | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | no floppy |
| 0x2A  | DMAread    | `DMAread`                     | 🟢 PHASE-04 / stub | ✅ (`_floppy-stubs.md`) | ACSI/SCSI raw I/O |
| 0x2B  | DMAwrite   | `DMAwrite`                    | 🟢 PHASE-04 / stub | ✅ (`_floppy-stubs.md`) | ACSI/SCSI raw I/O |
| 0x2C  | Bconmap    | `bconmap` (`serport.c:1082`)  | ➡ func-number | ✅ | `BCONMAP_AVAILABLE` off → returns `0x2c` |
| 0x2D  | (reserved) | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | |
| 0x2E  | NVMaccess  | `nvmaccess`                   | ⛔ unimpl | (dispatcher) | `CONF_WITH_NVRAM` off |
| 0x2F..0x3F | (reserved) | `xbios_unimpl`            | ⛔ unimpl | (dispatcher) | 17 entries |
| 0x40  | Blitmode   | `blitmode` (`bios/xbios.c:886`)    | 🟡 stub → 0 | ✅ | no Atari blitter |
| 0x41..0x4F | (reserved) | `xbios_unimpl`            | ⛔ unimpl | (dispatcher) | 15 filler entries |
| 0x50..0x57 | Eset*/Eget* | `esetshift`…`esetsmear`  | 🟢 PHASE-08 | ✅ (`_tt-shifter.md`) | TT shifter — ABI-only on framebuffer |

## Build-profile boundary

Functions 0x58..0xFF are **not present** in the NewTOS XBIOS table. A trap
with `ID > LAST_ENTRY (0x57)` fails the dispatcher range check and returns
**the function number** (not `EINVFN`) — see `_dispatcher.md §3`, D-008,
D-039.

> **Correction (D-039):** an earlier version of this section said such a
> trap "returns `EINVFN` (-32)". It does not: `bios/vectors.S:1325-1359`
> returns D0 unchanged (the function number). Corrected here and in
> `_tt-shifter.md`.

| Range        | Atari era        | NewTOS profile |
|--------------|------------------|----------------|
| 0x58..0x5F   | Falcon Videl     | absent (not in table) |
| 0x60..0x7F   | Falcon DSP       | absent |
| 0x80..0x8D   | Falcon DMA sound | absent |

## Discrepancy D-008

**`xbios_unimpl` (and the out-of-range range check) return the function
number, not `EINVFN`.** Compendium states "calling an unimplemented XBIOS
function returns `EINVFN` (-32)". EmuTOS's `xbios_do_unimpl(WORD number)
{ return number; }` (`bios/xbios.c:1251`) returns the positive number, and
the trampoline's `bx_ret_exc` path does the same. NewTOS **follows EmuTOS**:
programs that detect "unimplemented" do so by side-effect probing, not by a
`< 0` return check; matching real Atari hardware is more important than
matching the incorrect Compendium text.

## Implementation count (NewTOS TT-shifter profile, 88 table slots)

| Category    | Count | NewTOS plan |
|-------------|------:|-------------|
| 🟢 implement | 23 | PHASE-04/07/08 (screen, keyboard, time, TT-shifter, Random, Protobt, Iorec, Kbdvbase, **Setprt**) |
| 🟡 stub      | 12 | return `EUNDEV`/`0`/default (floppy/DMA, PSG, Midiws/Ikbdws/Rsconf/Scrdmp/Blitmode) |
| ➡ dispatch   | 1 | `Bconmap` returns the func number (`BCONMAP_AVAILABLE` off) |
| ⛔ unimpl    | 52 | `xbios_unimpl` (MFP/NVRAM/reserved/Ssbrk/Prtblk/Puntaes/used_by_bios + 0x2F..0x3F + 0x41..0x4F) |
| **Total**   | 88 | (`LAST_ENTRY = 0x57`) |

The counts are approximate at the category boundary (e.g. `Bconmap` could be
counted as a stub); the invariant is the 88-slot table and that every
unimplemented slot returns its own function number.
