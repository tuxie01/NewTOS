# XBIOS triage — D5

> Phase 00 deliverable. Source: EmuTOS @ `1a7484ab` —
> `bios/xbios.c:1285` (`xbios_vecs[]`).
>
> **What the switches are in the reference build (SPEC07).** An earlier version of
> this header listed `CONF_WITH_VIDEL`, `CONF_WITH_DSP`, `CONF_WITH_DMASOUND`,
> `CONF_WITH_MFP`, `CONF_WITH_PRINTER_PORT` and the Atari device configs (floppy/ACSI,
> YM2149, ACIA) as "disabled", and `CONF_WITH_TT_SHIFTER` + `CONF_WITH_VT52` as the
> profile. That was wrong for the reference build. The reference build is `make 512`
> = `-DTARGET_512` only (`Makefile:561` of the oracle tree
> `references/emutos-oracle/1a7484ab`; `DEF =` starts empty, `:132`; no `localconf.h` in
> the tree), and every switch below takes the default of the unconditional block of
> `include/config.h`. The other lines that set these switches to 0 sit in blocks for other machines or targets
> and in the `!CONF_ATARI_HARDWARE` block (`:673`); `CONF_ATARI_HARDWARE` defaults to 1 (`:665-667`). That none of them is entered
> by `TARGET_512` is shown by the preprocessor run named below, not by reading each block. Values, all from
> `references/emutos-oracle/1a7484ab/include/config.h`:
>
> | Switch | Reference build | Default at | Other lines setting 0 (not entered by `TARGET_512`) | Linked in the oracle ROM (`emutos.map`) |
> |---|:--:|---|---|---|
> | `CONF_WITH_TT_SHIFTER` | 1 | `:1085-1086` | `:86,140,268,424,497,746` | `_esetshift` `:725` |
> | `CONF_WITH_VIDEL` | 1 | `:1092-1093` | `:271,427,749` | `_vsetmode` `:762` |
> | `CONF_WITH_DSP` | 1 | `:1139-1140` | `:143,280,430,521,761` | `_dsp_*` `:810-845` |
> | `CONF_WITH_DMASOUND` | 1 | `:1132-1133` | `:274,515,755` | `_locksnd` `:573`, `_setbuffer` `:577` |
> | `CONF_WITH_MFP` | 1 | `:929-930` | `:692` | `_mfpint` `:652`, `_jdisint` `:650`, `_jenabint` `:651`, `_xbtimer` `:654` |
> | `CONF_WITH_PRINTER_PORT` | 1 | `:987-988` | `:707` | `_setprt` `:697` |
> | `CONF_WITH_NVRAM` | 1 | `:1161-1162` | `:289,412,770` | `_nvmaccess` `:688` |
> | `CONF_WITH_YM2149` | 1 | `:980-981` | `:704` | `_giaccess` `:749`, `_dosound` `:752` |
> | `CONF_WITH_FDC` / `CONF_WITH_ACSI` | 1 / 1 | `:1028-1029` / `:1035-1036` | `:722` / `:74,179,728` | `_floprd` `:594`, `_DMAread` `:567` |
> | `CONF_WITH_MIDI_ACIA` / `CONF_WITH_IKBD_ACIA` | 1 / 1 | `:994-995` / `:1001-1002` | `:710` / `:713` | `_midiws` `:662`, `_ikbdws` `:622` |
> | `CONF_WITH_BLITTER` | 1 | `:1168-1169` | `:152,773` | — |
>
> (`emutos.map` line numbers are those of the oracle tree's `emutos.map`; a preprocessor run
> of `config.h` with `-DTARGET_512` over the oracle tree's `include/` gives 1 for every row, checked in SPEC07; the oracle's `include/config.h` is identical to
> the pin's.) **`CONF_WITH_VT52` is not a switch**: it does not occur in `include/config.h`
> (the word appears only in a comment at `:1434`). So in the reference build **no `xbios_vecs[]` slot is turned off by a
> `CONF_WITH_*` switch**; `LAST_ENTRY` is `0x8D` (`CONF_WITH_DMASOUND`,
> `bios/xbios.c:1273-1274` of the oracle tree), and the empty slots are the fixed
> `xbios_unimpl` entries of `docs/api/xbios/_full-function-inventory.md` §"Profilgatter".
> The generated table `docs/api/_emutos-config.md` marks these switches 1 as "compiled in for
> all machines", which is not "active on the Falcon profile"; the point here is only that the
> reference build is not built without them.
>
> **So where a function below is `unimpl`/stub on NewTOS, the reason is not a switch.**
> Each row gives its own reason (hardware the Pi lacks, an owner decision or an ADR) with a
> cite. `CONF_WITH_NVRAM` (TASK/XBIOS13) was the first statement corrected:
> `CONF_WITH_NVRAM` defaults to 1 (`include/config.h:1161-1163`);
> the three places that set it 0 are the FireBee block (`include/config.h:289-290`,
> `MACHINE_FIREBEE`), the `TARGET_192` block (`:412-413`) and the
> `!CONF_ATARI_HARDWARE` block (`:770-771`), none of which is `make 512`
> (`TARGET_512`, `Makefile:561`; `CONF_ATARI_HARDWARE` defaults 1, `:665-667`).
> The oracle ROM (`references/emutos-oracle/1a7484ab`) links `_nvmaccess`
> (`emutos.map:688`). So in the reference build slot 0x2E is the real
> `nvmaccess` (`bios/xbios.c:1348-1349`); only **NewTOS** leaves it
> unwired.
>
> **Setprt (D-060):** in a build with `CONF_WITH_PRINTER_PORT` off the slot would fall back
> to `xbios_unimpl` (`bios/xbios.c:1332-1336`); the reference build has it on, so the slot
> holds `setprt()`. `setprt()` itself never touches hardware (`bios/parport.c:49-57`);
> only the paired `prnout()` output path needs the (absent) Centronics/PSG wiring. NewTOS
> implements the `Setprt` config-word contract; see `docs/api/xbios/21-setprt.md`.

The row table below is the Phase-00 triage (opcodes 0x00..0x57, 88 rows); the table now
reaches 0x8D, see "Table length" below. A slot the reference build fills with
`xbios_unimpl` (0x01, 0x0B, 0x24, 0x27, 0x28, 0x2D, 0x2F..0x3F, 0x41..0x4F) returns the
function number itself — **not** a GEMDOS error (see *D-008* / *D-039*); the NewTOS-only
`unimpl` rows (0x1A, 0x1B, 0x2E, 0x50..0x57) take the same answer through the dispatcher's
catch-all arm.

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
| 0x08  | Floprd     | `floprd`                      | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | reference build: real (`CONF_WITH_FDC` = 1); NewTOS: no floppy controller, owner decision 2026-11-12 (`docs/COMPATIBILITY.md`, "XBIOS floppy group", ADR-012) |
| 0x09  | Flopwr     | `flopwr`                      | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | reference build: real (`CONF_WITH_FDC` = 1); NewTOS: no floppy controller, owner decision 2026-11-12 (`docs/COMPATIBILITY.md`, "XBIOS floppy group", ADR-012) |
| 0x0A  | Flopfmt    | `flopfmt`                     | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | reference build: real (`CONF_WITH_FDC` = 1); NewTOS: no floppy controller, owner decision 2026-11-12 (`docs/COMPATIBILITY.md`, "XBIOS floppy group", ADR-012) |
| 0x0B  | (used_by_bios) | `xbios_unimpl`            | ⛔ unimpl | (dispatcher) | reserved |
| 0x0C  | Midiws     | `midiws` (`midi.c:104`)       | 🟡 stub | ✅ | reference build: real (`CONF_WITH_MIDI_ACIA` = 1); NewTOS: the Pi has no MIDI ACIA, each byte is accepted and dropped (`tos-api/src/xbios/mod.rs:94@MIDIWS`, `docs/api/xbios/0c-midiws.md`) |
| 0x0D  | Mfpint     | `mfpint` (`bios/mfp.c:114-120`) | 🟡 warned installer | ✅ (`0d-mfpint.md`) | reference build: real (`CONF_WITH_MFP` = 1, `bios/xbios.c:1299-1300`); NewTOS: no MFP on the Pi, the handler is never called (ADR-031), `d0` as EmuTOS leaves it (`tos-api/src/xbios/mod.rs:140@MFPINT`, `tos-api/src/xbios/mfp.rs`; BIOS15) |
| 0x0E  | Iorec      | `iorec` (`bios/xbios.c:336`)       | 🟢 PHASE-07 | ✅ | input ring descriptor |
| 0x0F  | Rsconf     | `rsconf` (`bios/xbios.c:376`)      | 🟡 stub / PL011 | ✅ | RS-232 config |
| 0x10  | Keytbl     | `keytbl` (`ikbd.c:122`)       | 🟢 PHASE-07 | ✅ | keyboard-table swap |
| 0x11  | Random     | `random` (`bios/xbios.c:421`)      | 🟢 PHASE-07 | ✅ | LCG, seeds from `hz_200` |
| 0x12  | Protobt    | `protobt`                     | 🟢 PHASE-04 | ✅ (`_floppy-stubs.md`) | boot-sector image builder (software) |
| 0x13  | Flopver    | `flopver`                     | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | reference build: real (`CONF_WITH_FDC` = 1); NewTOS: no floppy controller, owner decision 2026-11-12 (`docs/COMPATIBILITY.md`, "XBIOS floppy group", ADR-012) |
| 0x14  | Scrdmp     | `scrdmp` (`bios/xbios.c:490`)      | 🟡 stub / PNG | ✅ | screen dump; `dump_vec` default no-op |
| 0x15  | Cursconf   | `cursconf` (`vt52.c:893`)     | 🟢 PHASE-08 | ✅ | VT52 cursor config |
| 0x16  | Settime    | `settime` (`clock.c:1218`)    | 🟢 PHASE-07 | ✅ | RTC + GEMDOS time globals |
| 0x17  | Gettime    | `gettime` (`clock.c:1266`)    | 🟢 PHASE-07 | ✅ | RTC read |
| 0x18  | Bioskeys   | `bioskeys` (`ikbd.c:136`)     | 🟢 PHASE-07 | ✅ | reload default key tables |
| 0x19  | Ikbdws     | `ikbdws` (`ikbd.c:948`)       | 🟡 stub | ✅ | send raw IKBD command |
| 0x1A  | Jdisint    | `jdisint`                     | ⛔ unimpl (NewTOS only) | (dispatcher) | reference build: real handler (`CONF_WITH_MFP` = 1, `bios/xbios.c:1316-1318`); NewTOS: no MFP, answers `0x1A` (catch-all arm, `tos-api/src/xbios/mod.rs:162@other`); return value against the oracle not measured, parked: `TASK/XBIOS16.md`, `docs/api/xbios/0d-mfpint.md` "Open" |
| 0x1B  | Jenabint   | `jenabint`                    | ⛔ unimpl (NewTOS only) | (dispatcher) | as 0x1A (`bios/xbios.c:1316-1318`); `TASK/XBIOS16.md` |
| 0x1C  | Giaccess   | `giaccess` (`sound.c`)        | 🟡 stub → 0 | ✅ (`_psg-group.md`) | reference build: acts on the PSG (`CONF_WITH_YM2149` = 1, `bios/sound.c:94-109`); NewTOS: the Pi has no YM2149, answers the value of the `#else` branch (`bios/sound.c:110-112`; ADR-012 amendment 2026-11-12) |
| 0x1D  | Offgibit   | `offgibit` (`sound.c`)        | 🟡 stub → void | ✅ (`_psg-group.md`) | as 0x1C (`bios/sound.c:130-142`) |
| 0x1E  | Ongibit    | `ongibit` (`sound.c`)         | 🟡 stub → void | ✅ (`_psg-group.md`) | as 0x1C (`bios/sound.c:115-127`) |
| 0x1F  | Xbtimer    | `xbtimer` (`bios/mfp.c:163-169`) | 🟡 warned installer | ✅ (`0d-mfpint.md`) | as 0x0D (`bios/xbios.c:1326-1327`; `tos-api/src/xbios/mod.rs:141@XBTIMER`) |
| 0x20  | Dosound    | `dosound` (`sound.c`)         | 🟡 stub → 0 | ✅ (`_psg-group.md`) | as 0x1C (`bios/sound.c:145-160`) |
| 0x21  | Setprt     | `setprt` (`parport.c:49-57`)  | 🟢 implement | ✅ (`21-setprt.md`) | pure software config word (D-060); `CONF_WITH_PRINTER_PORT` is 1 in the reference build, so the slot holds `setprt` there (`bios/xbios.c:1332-1333`); `setprt()` touches no hardware (`bios/parport.c:49-57`), NewTOS implements it (`tos-api/src/xbios/mod.rs:137@SETPRT`) |
| 0x22  | Kbdvbase   | `kbdvbase` (`bios/xbios.c:716`)    | 🟢 PHASE-07 | ✅ | returns `&kbdvecs` |
| 0x23  | Kbrate     | `kbrate` (`ikbd.c:439`)       | 🟢 PHASE-07 | ✅ | repeat-rate config |
| 0x24  | Prtblk     | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | no printer |
| 0x25  | Vsync      | `vsync` (`screen.c:1205`)     | 🟢 PHASE-08 | ✅ | wait for VBL |
| 0x26  | Supexec    | `supexec` (`bios/vectors.S:1376`)  | 🟢 PHASE-06 (ADR) | ✅ | run callback in supervisor mode |
| 0x27  | Puntaes    | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | drop AES (boot path) |
| 0x28  | (reserved) | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | |
| 0x29  | Floprate   | `floprate`                    | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | reference build: real (`CONF_WITH_FDC` = 1); NewTOS: no floppy controller, owner decision 2026-11-12 (`docs/COMPATIBILITY.md`, "XBIOS floppy group", ADR-012) |
| 0x2A  | DMAread    | `DMAread`                     | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | reference build: real (`CONF_WITH_ACSI` = 1); NewTOS: no ACSI/DMA bus, owner decision 2026-11-12 (`docs/COMPATIBILITY.md`, "XBIOS floppy group"; `tos-api/src/xbios/mod.rs:86@FLOPRATE`) |
| 0x2B  | DMAwrite   | `DMAwrite`                    | 🟡 stub → `EUNDEV` | ✅ (`_floppy-stubs.md`) | as 0x2A |
| 0x2C  | Bconmap    | `bconmap` (`serport.c:1082`)  | 🟢 one-entry table | ✅ (`2c-bconmap.md`) | not a function-number answer: `BCONMAP_AVAILABLE` = `CONF_WITH_SCC \|\| CONF_WITH_TT_MFP` (`bios/serport.h:18`); the one-entry table is built (XBIOS05, `tos-api/src/xbios/mod.rs:98@BCONMAP`, `tos-api/src/xbios/bconmap.rs`). Reference build: `CONF_WITH_TT_MFP` = 1 (`include/config.h:943-944`) and `CONF_WITH_SCC` = 1 (`:950-951`), so `BCONMAP_AVAILABLE` is true and `bconmap` does the table work (`bios/serport.c:1082-1100`; oracle `emutos.map:746`) |
| 0x2D  | (reserved) | `xbios_unimpl`                | ⛔ unimpl | (dispatcher) | |
| 0x2E  | NVMaccess  | `nvmaccess`                   | ⛔ unimpl (NewTOS only) | (dispatcher) | reference build: real handler (`CONF_WITH_NVRAM` = 1); NewTOS has no NVRAM and answers `0x2E` (catch-all arm, `tos-api/src/xbios/mod.rs:162@other`; `docs/COMPATIBILITY.md` "XBIOS for absent Atari hardware") |
| 0x2F..0x3F | (reserved) | `xbios_unimpl`            | ⛔ unimpl | (dispatcher) | 17 entries |
| 0x40  | Blitmode   | `blitmode` (`bios/xbios.c:886`)    | 🟡 stub → 0 | ✅ | no Atari blitter |
| 0x41..0x4F | (reserved) | `xbios_unimpl`            | ⛔ unimpl | (dispatcher) | 15 filler entries |
| 0x50..0x57 | Eset*/Eget* | `esetshift`…`esetsmear`  | ⛔ unimpl (NewTOS only) | ✅ (`_tt-shifter.md`) | reference build: real handlers (`CONF_WITH_TT_SHIFTER` = 1, `bios/xbios.c:1391-1409`); NewTOS: no TT shifter on the Pi, answers 0x50-0x57 (catch-all arm, `tos-api/src/xbios/mod.rs:162@other`; owner decision 2026-11-12, `docs/COMPATIBILITY.md`, "XBIOS for absent Atari hardware") |

## Table length: reference build and NewTOS

In the reference build `LAST_ENTRY = 0x8D` (`CONF_WITH_DMASOUND` = 1,
`bios/xbios.c:1273-1274`), so the table has 142 slots 0x00..0x8D, and a trap with
`ID > 0x8D` fails the range check and returns **the function number** (not
`EINVFN`; `bios/vectors.S:1325-1359`, `_dispatcher.md §3`, D-008, D-039).

NewTOS has the same table length since TASK/SND05 (2026-11-17; before that the
Phase-00 profile stopped at `LAST_ENTRY = 0x57`, 88 slots, which is what the row
table above and the first version of this file assumed):
`docs/api/xbios/_dispatcher.md §4`. The ranges beyond 0x57:

| Range        | Atari era        | NewTOS |
|--------------|------------------|--------|
| 0x58..0x5F   | Falcon Videl     | 0x58, 0x59, 0x5B, 0x5E describe the Pi screen (XBIOS22); 0x5A, 0x5C, 0x5D, 0x5F answer their number (no run-time mode switch, ADR-017) |
| 0x60..0x7F   | Falcon DSP       | answer their number: no DSP on the Pi (`docs/COMPATIBILITY.md`, "XBIOS for absent Atari hardware") |
| 0x80..0x8D   | Falcon DMA sound | real handlers once a sound backend is up, else their number (`_dmasound-group.md`, SND05) |

> **Correction (SPEC07):** an earlier version of this section said these ranges are
> "absent (not in table)" in the NewTOS profile because the Videl/DSP/DMA-sound
> switches were disabled. The switches are 1 in the reference build (header), and the NewTOS
> dispatcher already carries the table up to `0x8D`
> (`_dispatcher.md §4`, `docs/api/xbios/_full-function-inventory.md`).

## Discrepancy D-008

**`xbios_unimpl` (and the out-of-range range check) return the function
number, not `EINVFN`.** Compendium states "calling an unimplemented XBIOS
function returns `EINVFN` (-32)". EmuTOS's `xbios_do_unimpl(WORD number)
{ return number; }` (`bios/xbios.c:1251`) returns the positive number, and
the trampoline's `bx_ret_exc` path does the same. NewTOS **follows EmuTOS**:
programs that detect "unimplemented" do so by side-effect probing, not by a
`< 0` return check; matching real Atari hardware is more important than
matching the incorrect Compendium text.

## Implementation count (Phase-00 triage, 88 table slots; **historical, not maintained**)

| Category    | Count | NewTOS plan |
|-------------|------:|-------------|
| 🟢 implement | 23 | PHASE-04/07/08 (screen, keyboard, time, TT-shifter, Random, Protobt, Iorec, Kbdvbase, **Setprt**) |
| 🟡 stub      | 12 | return `EUNDEV`/`0`/default (floppy/DMA, PSG, Midiws/Ikbdws/Rsconf/Scrdmp/Blitmode) |
| ➡ dispatch   | 1 | `Bconmap` — **stale**: built since XBIOS05, see its row |
| ⛔ unimpl    | 52 | `xbios_unimpl` (MFP/NVRAM [NewTOS only, see the header]/reserved/Ssbrk/Prtblk/Puntaes/used_by_bios + 0x2F..0x3F + 0x41..0x4F) — **stale** for Mfpint/Xbtimer (warned installers) |
| **Total**   | 88 | (`LAST_ENTRY = 0x57`) |

The counts were approximate at the category boundary and are **not recomputed** in SPEC07
(they were the Phase-00 plan; the plan column of the row table is likewise partly historical:
e.g. 0x00, 0x02..0x07, 0x1C..0x20 changed since). The invariant is that every unimplemented slot
returns its own function number.

---

## Open points

- Corrected (SPEC07): the header's "disabled" list; every row whose reason named `CONF_WITH_MFP`, `CONF_WITH_YM2149`, `CONF_WITH_PRINTER_PORT`, `CONF_WITH_FDC`/`ACSI`, `CONF_WITH_MIDI_ACIA` or `BCONMAP_AVAILABLE`; the 0x50-0x57 row; the "Build-profile boundary" section (now "Table length"); the counts table is marked historical. Earlier correction (TASK/XBIOS13): the `CONF_WITH_NVRAM` statements (`include/config.h:1161-1163`, `emutos.map:688`).
- Cited: all switch values (oracle tree `include/config.h`, the `#ifndef` defaults) and the linked symbols (`emutos.map`). The value of each switch is also what a preprocessor run of the oracle tree's `include/` with `-DTARGET_512` printed (SPEC07, scratch run, not stored).
- Inferred: that `make 512` sets no other override (`Makefile:561` defines only `-DTARGET_512`; the oracle tree has no `localconf.h`); the Hatari-built ROM's own compile flags were not read from a build log.
- Not re-checked (out of scope): the plan/spec columns of the rows that name no switch (e.g. 0x00 Initmous, 0x02..0x07, 0x14, 0x19, 0x40); `CONF_WITH_BLITTER` = 1 in the reference build while NewTOS answers `Blitmode` with `0x0000` (`tos-api/src/xbios/mod.rs:147@BLITMODE`): the reason (no Atari blitter on the Pi) belongs to `docs/api/xbios/40-blitmode.md`, not re-read here.
- For the lead (not changed): `Giaccess`/`Ongibit`/`Offgibit`/`Dosound` (reference build acts on the PSG, NewTOS returns the `#else` values) and `Midiws`/`Ikbdws` (reference build transmits through the ACIA) are recorded only in the ADR-012 amendment and in per-function specs; `docs/DEVIATIONS.md` has no row for them. `Jdisint`/`Jenabint` are covered by the parked `TASK/XBIOS16.md`; the TT-shifter, DSP, NVRAM, floppy and DMA groups by `docs/COMPATIBILITY.md`.
