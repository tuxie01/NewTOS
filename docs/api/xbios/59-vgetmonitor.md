# `VgetMonitor` (XBIOS 0x59)

> Spec written 2026-11-12 (TASK/XBIOS22 spec step, researcher). Source: EmuTOS @ `1a7484ab`,
> `bios/videl.c:664` (`vmontype`), table slot `bios/xbios.c:1413`. Measured: case 212 of
> `tosdump/cases/bios-r1.tsv` (`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: the trap ID, the WORD result and its four values (0..3), the
> "function number when no Videl" answer. Reference: the register read at `0xFFFF8006`.

## EmuTOS source

**Primary location:** `bios/videl.c:664-670` — `WORD vmontype(void)`, slot `0x59` of
`xbios_vecs[]` (`bios/xbios.c:1413`, inside `#if CONF_WITH_VIDEL`, `:1411`).

**Behaviour (read):**
- `!has_videl` -> returns `0x59` (the function number, "unimplemented xbios call",
  `bios/videl.c:666-667`). `has_videl` is set by probing `FALCON_HHT`
  (`bios/machine.c:136-141`).
- otherwise `(*(UBYTE *)0xffff8006 >> 6) & 3` (`bios/videl.c:669`): the monitor-type
  bits of the Falcon monitor-sense register.

**Values** (`include/biosdefs.h:136-139`):

| Value | Name | Meaning |
|------:|------|---------|
| 0 | `MON_MONO`  | ST monochrome |
| 1 | `MON_COLOR` | ST colour (RGB) |
| 2 | `MON_VGA`   | VGA |
| 3 | `MON_TV`    | TV via RF modulator |

**Build configuration:** `CONF_WITH_VIDEL` (slot present only then; without it the slot is
`xbios_unimpl` or absent, `bios/xbios.c:1420-1429`).

**Who uses the value inside EmuTOS** (`vmontype()` callers, all `bios/videl.c`):
`vfixmode` (`:917-956`, forces `FALCON_ST_HIGH` on `MON_MONO`, VGA/TV vertical-bit fix-up),
`vgetsize` (`:724-757`), `use_ste_palette` (`:778`), `set_videl_vga` (`:511`, mono branch `:559`).

## Trap interface

- **Trap number:** 14. **Function ID:** `0x59` (89). `VgetMonitor()` = `xbios_w_v(89)`
  (`include/xbiosbind.h:62`). No parameters (2 bytes on the m68k stack).
- **Native (AArch64) ABI:** ID `0x59` in `x8`; no argument registers; WORD result sign-extended
  into `x0`.

## Outputs

- **Return:** WORD, 0..3 on a Videl machine; `0x59` otherwise (no error code).
- **Side effects:** none.
- **Error conditions:** none.

## Measured (oracle)

| Profile | Case 212 | Reading |
|---------|---------:|---------|
| `tt` (no Videl) | `0x0059` | function number (`:666-667`); NewTOS equal today |
| `falcontc` (Hatari Falcon, VGA monitor) | `0x0002` | `MON_VGA`; NewTOS today `0x0059` (DIFF) |

## Edge cases discovered in source

1. The "function number" return is **EmuTOS's own no-Videl answer**, so today's NewTOS catch-all
   (`tos-api/src/xbios/mod.rs:157@other`) is indistinguishable from "this machine has no Videl";
   it is a valid, source-cited answer on that profile, not an error.
2. A program that gets `0x59` = 89 from `VgetMonitor` sees a value outside 0..3. Programs that
   gate on the `_VDO` cookie never call it; NewTOS omits `_VDO` on purpose
   (`tos-api/src/cookie.rs:219@seed_defaults`, ADR-012 amendment 2026-10-28; "open until
   TASK/ARCH06").
3. The value changes the answers of `VgetSize` (0x5B) and `Vfixmode` (0x5F), because both call
   `vmontype()`; implementing it alone and keeping the others unanswered is consistent.

## Dependencies

- Used by: Falcon-aware programs picking a mode (`sdk/apps/teradesk/video.c:140-160`, behind
  the `_VDO` cookie test; `fmtype = VgetMonitor()`).

## NewTOS mapping — options for the owner (no decision taken here)

Question: which value does the HDMI screen report? Facts: the Pi has no monitor-sense lines; the
screen is a progressive, digital, config.txt-driven mode (ADR-017 rev 2, 1920x1080 default).

| Option | Value | Consequence for a program that reads it |
|--------|------:|-----------------------------------------|
| A. VGA (recommended) | 2 | Equal to the measured oracle (case 212, falcontc). `vfixmode`/`VgetSize` conventions then take the VGA branch (`bios/videl.c:933-942`): progressive, "double-line" vertical flag, no overscan. Closest to a progressive digital screen. |
| B. ST colour (RGB) | 1 | A program picks the TV/RGB branch (`:945-956`): interlace/overscan assumptions and 40-column PAL/NTSC choices that make no sense on HDMI. Differs from the oracle. |
| C. Mono | 0 | `vfixmode` would force `FALCON_ST_HIGH` (`:925-926`) and programs choose a 640x400 monochrome UI. Rejected: contradicts the truecolor VDI (ADR-024). |
| D. TV | 3 | RF-modulator conventions (PAL/NTSC interlace). No source or screen reason. |
| E. Keep the function number | 0x59 | Valid only as "no Videl" (see edge case 1); stays DIFF on case 212 and must then be recorded as a COMPATIBILITY entry. |

Open point tied to this choice: whether NewTOS publishes `_VDO` (ADR-012 amendment, open with
TASK/ARCH06). Without it, programs that test the cookie first never reach 0x59; with it they do
and also call `VsetMode`, which the owner excluded (TASK/XBIOS22).

Host tests (`TASK/XBIOS22.md` Work 3): the documented value for the chosen option; no error
path exists to test (state that in the test).
