# XBIOS Trap-14 Dispatcher

> **Phase 00 deliverable D2 (Trap 14).** Source: EmuTOS @ `1a7484ab`.
> Cite-never-paste: the dispatch mechanics are described in prose with
> `file:line` cites; the opcode→handler map is a data table.
>
> XBIOS shares the trap dispatch trampoline with BIOS — the only difference
> is which table is consulted. See
> [`docs/api/bios/_dispatcher.md`](../bios/_dispatcher.md) for the shared
> `biosxbios` trampoline body; this file documents only the XBIOS wiring.

---

## 1. Vector installation

The trap-14 exception vector is installed alongside trap-13 in the boot
path (`vecs_init()`, `bios/bios.c`). The handler is `_xbiostrap`
(`bios/vectors.S:1275`).

---

## 2. `_xbiostrap` entry (`bios/vectors.S:1275-1277`)

`_xbiostrap` loads the table length into `d1` (`_xbios_ent`) and the table
base into `a0` (`_xbios_vecs`), then falls through to the shared
`biosxbios:` trampoline (`bios/vectors.S:1282`).

- `_xbios_ent` (`bios/xbios.c:1517`) = `ARRAY_SIZE(xbios_vecs)`; its value
  depends on build configuration (Section 4).
- `_xbios_vecs` (`bios/xbios.c:1285`) is the function-pointer table
  (Section 5).

The trampoline saves the caller's registers to the `savptr`-anchored save
area, switches to the caller's stack (USP if the caller was in user mode),
pops the function ID, range-checks it, indexes the table (ID×4), and calls
the handler (`bios/vectors.S:1282-1332`). See the BIOS dispatcher doc for
the full save-area mechanics.

---

## 3. "Not implemented" behaviour returns the **function number**, not `EINVFN`

There are two paths to "unimplemented", and **both return the (positive)
function number in D0**, not an error code:

1. **Out-of-range ID** (`d0 >= xbios_ent`): the trampoline's range check
   (`cmp.w d1,d0; jhs bx_ret_exc`, `bios/vectors.S:1325-1326`) skips the
   table lookup and returns with D0 still holding the function number (it
   was loaded by `moveq #0,d0` then `move.w (sp)+,d0`,
   `bios/vectors.S:1323-1324`).
2. **In-range but unimplemented slot**: EmuTOS plants `xbios_unimpl`
   (`bios/vectors.S:1366`) at that slot, which calls
   `xbios_do_unimpl(WORD number)` (`bios/xbios.c:1251`) — a one-liner that
   returns `number`.

So `Xbios(N, …)` for any unimplemented `N` (out-of-range **or** padded slot)
returns `N` itself. A caller testing `result < 0` sees "success". This is
the documented Atari-TOS behaviour; the Compendium's claim of `EINVFN`
(-32) is wrong for both paths. Tracked as **D-008**.

> **Correction vs. earlier drafts:** `_triage.md` and `_tt-shifter.md`
> previously stated that IDs ≥ `LAST_ENTRY` return `EINVFN` (-32). They do
> not — the range check returns the function number, identical to
> `xbios_unimpl`. Corrected there and logged as **D-039**.

---

## 4. Build-time table size (`bios/xbios.c:1273-1283`)

`LAST_ENTRY` selects the highest opcode compiled in; `xbios_vecs[]` is
padded with `xbios_unimpl` slots to fill up to `LAST_ENTRY + 1` entries:

| Highest `CONF_WITH_*` set | `LAST_ENTRY` | table entries |
|---------------------------|:------------:|:-------------:|
| `CONF_WITH_DMASOUND` (Falcon DMA sound) | `0x8D` | 142 |
| `CONF_WITH_DSP` (Falcon DSP) | `0x7F` | 128 |
| `CONF_WITH_VIDEL` (Falcon Videl) | `0x5F` | 96 |
| `CONF_WITH_TT_SHIFTER` (TT shifter) | `0x57` | 88 |
| (ST baseline) | `0x40` | 65 |

NewTOS target profile: **`LAST_ENTRY = 0x8D`**, table entries 0x00–0x8D (142
slots), since TASK/SND05 (2026-11-17; until then the TT-shifter baseline
`0x57`, 88 slots). The DMA sound group 0x80–0x8D has real handlers
(`docs/api/xbios/_dmasound-group.md`); they answer their function number while
no sound backend has initialised, as EmuTOS does without a sound chip. The
DSP range (0x60–0x7F) and the Videl slots 0x5A, 0x5C, 0x5D, 0x5F are unimplemented slots inside the
table and return their function number; of the Videl range, `VsetMode(-1)`, `VgetMonitor`, `VgetSize` and
`VgetRGB` (0x58, 0x59, 0x5B, 0x5E) describe the Pi screen since TASK/XBIOS22 (`58-vsetmode.md`,
`59-vgetmonitor.md`, `5b-vgetsize.md`, `5e-vgetrgb.md`; `VsetMode` with a mode answers 0x58); a trap with `ID > 0x8D` hits the range
check and returns the function number (§3). The two are indistinguishable from
outside, so moving the last entry changes no answer for 0x58–0x7F.

> **Profile-mixing note (re-validation finding, D-058):** `CONF_WITH_TT_SHIFTER`
> selects the *table shape* here (opcodes 0x50–0x57 get real handlers instead
> of `xbios_unimpl`). But EmuTOS's own `include/config.h:2151-2152` makes
> `CONF_WITH_TT_SHIFTER` **require** `CONF_WITH_ATARI_VIDEO` — no real EmuTOS
> build has this table shape *and* the "no Atari video" fallback behaviour
> (`Physbase() == Logbase()`, `Getrez()` returns the `sshiftmod` shadow,
> `Setcolor` fakes `return 0`, …) that the individual screen-function specs
> (`02-physbase.md`, `04-getrez.md`, `05-setscreen.md`, `06-setpalette.md`,
> `07-setcolor.md`, `25-vsync.md`) extract for the NewTOS profile. NewTOS
> deliberately combines the table-size branch of one `#if` with the
> runtime-behaviour branch of a mutually-exclusive one, because the real
> target (BCM2711 VideoCore) has neither a Shifter nor a TT shifter to back
> either branch faithfully — it is a documentation/extraction choice, not an
> EmuTOS-attested build. See `docs/api/_discrepancies.md` D-058.

---

## 5. `xbios_vecs[]` opcode→handler map (`bios/xbios.c:1285-1515`)

Handler is the `direct` target of each `VEC(wrapper, direct)` macro (the
non-debug build; `DBG_XBIOS = 0`). "Cfg" names the `CONF_WITH_*` gate; when
the gate is off the slot is `xbios_unimpl`.

| ID    | Handler        | TOS name  | Cfg | Handler file |
|------:|----------------|-----------|-----|--------------|
| 0x00  | `Initmous`     | Initmous  |                          | `bios/mouse.c:40` |
| 0x01  | `xbios_unimpl` | (Ssbrk)   | never (obsolete)         | — |
| 0x02  | `physbase`     | Physbase  |                          | `bios/screen.c:1042` |
| 0x03  | `logbase`      | Logbase   |                          | `bios/screen.c:1071` |
| 0x04  | `getrez`       | Getrez    |                          | `bios/screen.c:1076` |
| 0x05  | `setscreen`    | Setscreen |                          | `bios/screen.c:1102` |
| 0x06  | `setpalette`   | Setpalette|                          | `bios/screen.c:1168` |
| 0x07  | `setcolor`     | Setcolor  |                          | `bios/screen.c:1192` |
| 0x08  | `floprd`       | Floprd    |                          | `bios/floppy.c` |
| 0x09  | `flopwr`       | Flopwr    |                          | `bios/floppy.c` |
| 0x0A  | `flopfmt`      | Flopfmt   |                          | `bios/floppy.c` |
| 0x0B  | `xbios_unimpl` | (used_by_bios) | never                | — |
| 0x0C  | `midiws`       | Midiws    |                          | `bios/midi.c:104` |
| 0x0D  | `mfpint`       | Mfpint    | `CONF_WITH_MFP`          | `bios/mfp.c` |
| 0x0E  | `iorec`        | Iorec     |                          | `bios/xbios.c:336` |
| 0x0F  | `rsconf`       | Rsconf    |                          | `bios/xbios.c:376` → `*rsconfptr` |
| 0x10  | `keytbl`       | Keytbl    |                          | `bios/ikbd.c:122` |
| 0x11  | `random`       | Random    |                          | `bios/xbios.c:421` |
| 0x12  | `protobt`      | Protobt   |                          | `bios/floppy.c` |
| 0x13  | `flopver`      | Flopver   |                          | `bios/floppy.c` |
| 0x14  | `scrdmp`       | Scrdmp    |                          | `bios/xbios.c:490` |
| 0x15  | `cursconf`     | Cursconf  |                          | `bios/vt52.c:893` |
| 0x16  | `settime`      | Settime   |                          | `bios/clock.c:1218` |
| 0x17  | `gettime`      | Gettime   |                          | `bios/clock.c:1266` |
| 0x18  | `bioskeys`     | Bioskeys  |                          | `bios/ikbd.c:136` |
| 0x19  | `ikbdws`       | Ikbdws    |                          | `bios/ikbd.c:948` |
| 0x1A  | `jdisint`      | Jdisint   | `CONF_WITH_MFP`          | `bios/mfp.c` |
| 0x1B  | `jenabint`     | Jenabint  | `CONF_WITH_MFP`          | `bios/mfp.c` |
| 0x1C  | `giaccess`     | Giaccess  |                          | `bios/sound.c` |
| 0x1D  | `offgibit`     | Offgibit  |                          | `bios/sound.c` |
| 0x1E  | `ongibit`      | Ongibit   |                          | `bios/sound.c` |
| 0x1F  | `xbtimer`      | Xbtimer   | `CONF_WITH_MFP`          | `bios/mfp.c` |
| 0x20  | `dosound`      | Dosound   |                          | `bios/sound.c` |
| 0x21  | `setprt`       | Setprt    | `CONF_WITH_PRINTER_PORT` | `bios/parport.c` |
| 0x22  | `kbdvbase`     | Kbdvbase  |                          | `bios/xbios.c:716` |
| 0x23  | `kbrate`       | Kbrate    |                          | `bios/ikbd.c:439` |
| 0x24  | `xbios_unimpl` | (Prtblk)  | never                    | — |
| 0x25  | `vsync`        | Vsync     |                          | `bios/screen.c:1205` |
| 0x26  | `supexec`      | Supexec   |                          | `bios/vectors.S:1376` (asm) |
| 0x27  | `xbios_unimpl` | (Puntaes) | never                    | — |
| 0x28  | `xbios_unimpl` | —         | never                    | — |
| 0x29  | `floprate`     | Floprate  |                          | `bios/floppy.c` |
| 0x2A  | `DMAread`      | DMAread   |                          | `bios/disk.c` |
| 0x2B  | `DMAwrite`     | DMAwrite  |                          | `bios/disk.c` |
| 0x2C  | `bconmap`      | Bconmap   |                          | `bios/serport.c:1082` |
| 0x2D  | `xbios_unimpl` | —         | never                    | — |
| 0x2E  | `nvmaccess`    | NVMaccess | `CONF_WITH_NVRAM`        | `bios/nvram.c` |
| 0x2F–0x3F | `xbios_unimpl` | —     | never (17 slots)         | — |
| 0x40  | `blitmode`     | Blitmode  |                          | `bios/xbios.c:886` |
| 0x41–0x4F | `xbios_unimpl` | —     | filler when `LAST_ENTRY>0x40` | — |
| 0x50–0x57 | `esetshift`…`esetsmear` | Eset*/Eget* | `CONF_WITH_TT_SHIFTER` | `bios/screen.c` |

Beyond 0x57: Videl (0x58–0x5F, `CONF_WITH_VIDEL`; the four screen queries 0x58 (query form), 0x59, 0x5B,
0x5E answered since TASK/XBIOS22, the rest unimplemented) and DSP (0x60–0x7F,
`CONF_WITH_DSP`), unimplemented slots in the NewTOS profile; DMA sound
(0x80–0x8D, `CONF_WITH_DMASOUND`), in the NewTOS profile since TASK/SND05
(`docs/api/xbios/_dmasound-group.md` §2, §11). See `bios/xbios.c:1411-1514`.

> Note: the 25 in-scope functions are the ones present with a real handler
> in the NewTOS TT-shifter profile *and* documented as their own spec file.
> The floppy/DMA (`_floppy-stubs.md`), PSG (`_psg-group.md`) and TT-shifter
> (`_tt-shifter.md`) entries are present in the table but grouped; the MFP
> (0x0D/0x1A/0x1B/0x1F) and NVRAM (0x2E) entries are `xbios_unimpl` in the
> profile.
>
> **Setprt (0x21) is not in that unimpl group** despite `CONF_WITH_PRINTER_PORT`
> being off in the NewTOS profile: `setprt()` (`bios/parport.c:49-57`) is a
> pure software config-word getter/setter with no hardware dependency — only
> the paired `prnout()` output path needs the (absent) parallel port. NewTOS
> honours the `Setprt` contract regardless of the config gate; see
> `docs/api/xbios/21-setprt.md` and `docs/api/_discrepancies.md` D-060.

---

## 6. Source-location summary

| Concern | File:line |
|---------|-----------|
| Trap-14 entry (`_xbiostrap`) | `bios/vectors.S:1275` |
| Shared trampoline (`biosxbios`) | `bios/vectors.S:1282` |
| Range check → func-number return (`bx_ret_exc`) | `bios/vectors.S:1325-1359` |
| Unimpl shim (`_xbios_unimpl`) | `bios/vectors.S:1366` |
| Unimpl C tail (`xbios_do_unimpl`) | `bios/xbios.c:1251` |
| Supexec asm handler (`_supexec`) | `bios/vectors.S:1376` |
| Function table (`xbios_vecs[]`) | `bios/xbios.c:1285-1515` |
| `LAST_ENTRY` selector | `bios/xbios.c:1273-1283` |
| Table size (`_xbios_ent`) | `bios/xbios.c:1517` |
| Native ABI signatures (`xbios_*` macros) | `include/xbiosbind.h:19-64` |

---

## NewTOS ABI mapping notes

Function ID in `x8`; arguments in `x0..x6` in declaration order (one register
each); result in `x0` (WORD/LONG sign-extended, pointer full-64-bit). The
`include/xbiosbind.h` macro names encode the m68k signature
(`xbios_<ret>_<args>`) — e.g. `Setcolor` = `xbios_w_ww` (two WORDs, WORD
return), `Supexec` = `xbios_l_l` (one LONG, LONG return),
`Initmous` = `xbios_v_wll` (WORD + two pointers, void). Detailed per-function
ABI tables live in each function spec.

(Populated further during Phase 07.)
