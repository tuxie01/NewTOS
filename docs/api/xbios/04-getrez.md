# `Getrez` (XBIOS 0x04)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/screen.c:1076` (`getrez`).

> **Contract vs. reference** — see [`docs/COMPATIBILITY.md`](../../COMPATIBILITY.md).
> Contract: trap ID/ABI, the WORD result, the resolution-code values a
> program branches on. Reference: the Shifter/Videl register reads and
> `sshiftmod`.

## EmuTOS source

**Primary location:** `bios/screen.c:1076` — `WORD getrez(void)`, slot 4 of
`xbios_vecs[]` (`bios/xbios.c:1290`).

**Installed handler / vector:** (none) — direct table entry.

**Helpers called:**
- `bios/screen.c` — `atari_getrez()` — reads the resolution from the Videl
  mode (`vsetmode(-1)` mapped to a ST-compatible code), the TT shifter
  register (`0xFFFF8262`, masked), or the ST shifter register (`0xFFFF8260`,
  masked), depending on the compiled video.

**Headers / type definitions used:**
- `bios/screen.h` — resolution-code constants (`ST_LOW`=0, `ST_MEDIUM`=1,
  `ST_HIGH`=2, `FALCON_REZ`=3, `TT_MEDIUM`=4, `TT_HIGH`=6, `TT_LOW`=7).
- `include/tosvars.h` — `sshiftmod`.

**System variables / globals read:**
- Shifter/Videl mode register (Atari video), or
- `sshiftmod` (`0x44C`) — returned when there is no Atari video.

**System variables / globals written:** none.

**Build configuration affecting this function:**
- `CONF_WITH_ATARI_VIDEO`, `CONF_WITH_VIDEL`, `CONF_WITH_TT_SHIFTER` — which
  register is read. Without Atari video, returns `sshiftmod`.

---

## Source extraction summary

`Getrez` returns the current screen resolution as a small code. On Atari
video `atari_getrez` reads the active Shifter (ST/TT) or Videl mode and maps
it to the classic 0/1/2 (ST) plus the TT/Falcon extensions. Without Atari
video hardware it returns the shadow variable `sshiftmod`, which
`Setscreen`/`atari_setrez` keep in sync.

The codes are the contract programs branch on to pick a drawing strategy
(mono vs 4-plane vs 16-colour). NewTOS reports the code corresponding to the
framebuffer mode it presents.

---

## Trap interface

- **Trap number:** 14 (XBIOS)
- **Function ID:** 0x04
- **Stack frame on entry (m68k):**
  ```
   0: function_id (word) = 0x0004
  ```
  No parameters (`Getrez()` = `xbios_w_v(4)`, `include/xbiosbind.h:23`).
- **Total parameter bytes:** 2.
- **Native (AArch64) ABI:** ID `0x04` in `x8`; no argument registers; the
  WORD result **sign-extended** into `x0`.

  | m68k stack | AArch64 reg | Name | Type / size |
  |------------|-------------|------|-------------|
  | (none)     | —           | —    | —           |

---

## Inputs

None.

---

## Outputs

### Return value

- **Register:** D0 / `x0`
- **Type:** WORD
- **Encoding:** resolution code:

  | Code | Name | Geometry (Atari) |
  |-----:|------|------------------|
  | 0 | `ST_LOW` | 320×200, 16 colours |
  | 1 | `ST_MEDIUM` | 640×200, 4 colours |
  | 2 | `ST_HIGH` | 640×400, mono |
  | 3 | `FALCON_REZ` | Videl mode (Falcon) |
  | 4 | `TT_MEDIUM` | 640×480, 16 colours |
  | 6 | `TT_HIGH` | 1280×960, mono |
  | 7 | `TT_LOW` | 320×480, 256 colours |

  (Code 5 is unused.)
- **Source location:** `bios/screen.c:1082` (`sshiftmod` fallback) or the
  `atari_getrez` register read.

### Side effects on global state / hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (no error return) | — | — |

`Getrez` cannot fail.

---

## Verification criteria

- **Nominal (mono):** in a mono framebuffer mode, `Getrez()` returns `2`
  (`ST_HIGH`).
- **Consistency with `sshiftmod`:** on the no-Atari-video path, `Getrez()`
  equals the current `sshiftmod`.
- **Round-trip with `Setscreen`:** after `Setscreen(-1, -1, r)` for a valid
  `r`, `Getrez()` returns `r`.

---

## Edge cases discovered in source

1. **`sshiftmod` fallback.** With no Atari video, `Getrez` returns the
   `sshiftmod` shadow (`bios/screen.c:1082`), which `atari_setrez` writes on
   every `Setscreen` rez change — so `Getrez` and `Setscreen` stay coherent
   without real hardware.
2. **Videl maps to ST-compatible codes — never to `FALCON_REZ`.**
   (Corrected 2026-11-12, XBIOS23 spec step: earlier text said "or reports
   `FALCON_REZ`"; the source never returns 3.) On Videl `atari_getrez` takes the
   current mode word `vsetmode(-1)` (the stored `current_video_mode`, not a
   register read-back, `bios/videl.c:619-627`) and returns
   (`bios/screen.c:948-987`):

   | Current Videl mode word | `Getrez()` | Source |
   |-------------------------|-----------:|--------|
   | `VIDEL_COMPAT` set, 1bpp (ST high) | 2 | `bios/screen.c:956-959` |
   | `VIDEL_COMPAT` set, 2bpp (ST medium) | 1 | `:961-962` |
   | `VIDEL_COMPAT` set, 4bpp (ST low) | 0 | `:964-965` |
   | `VIDEL_COMPAT` set, any other bpp field | 0, with `KINFO("Problem - unsupported video mode")` | `:967-969` |
   | `VIDEL_COMPAT` clear — **every** Falcon mode (1/2/4/8bpp, **truecolor**) | **2** | `:971-972` |

   `FALCON_REZ` (3) exists only as the value `vsetmode` stores in `sshiftmod` for a
   non-ST-compatible mode (`bios/videl.c:648-656`), so on Videl `Getrez()` and
   `sshiftmod` differ for every Falcon mode (2 vs 3). The oracle's falcontc boot
   mode is `0x1C` (VGA, 80 columns, truecolor; case 211 = `0x1C`), non-compat, hence
   case 87 = 2 (`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`).
   Other hardware: TT shifter `TT_SHIFTER & 0x07` (`:976-979`), ST shifter
   `ST_SHIFTER & 0x03` (`:981-984`), no Atari video: `sshiftmod` (`:1076-1085`).
   What EmuTOS itself concludes from `Getrez`: the AES opens its workstation with
   `FALCON_REZ + 2` on Videl and `Getrez() + 2` elsewhere (`aes/gemgsxif.c:159-165,180`),
   i.e. it does **not** consult `Getrez` on a Videl machine; the desktop takes the Videl
   mode word and falls back to `0xFF00 | Getrez()` without Videl (`desk/deskapp.c:1085-1096`);
   `v_opnwk` switches mode only for `work_in[0]-2` equal to an ST/TT code
   (`vdi/vdi_control.c:506-513`); `shifter_rez_changeable` is the other internal consumer
   (`bios/screen.c:112`).

---

## Dependencies

### Requires (must be initialized / functional)

- `sshiftmod` initialised by the display driver / `Setscreen`.

### Used by

- Every graphics program that adapts to resolution; VDI open; AES.

---

## Related data structures

- `docs/api/xbios/_state.md §4` — `sshiftmod` and the resolution codes.

---

## Cross-references

### Atari Compendium

- Not present in `references/`. Cross-check deferred. (Documents 0/1/2 for
  ST; TT/Falcon codes added later.)

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

---

## Configuration / version notes

- ST returns 0/1/2; TT adds 4/6/7; Falcon adds 3 (`FALCON_REZ`). The value
  set depends on `CONF_WITH_VIDEL`/`CONF_WITH_TT_SHIFTER`.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-08: `Getrez` reports the code matching the active framebuffer mode; the
TT-shifter profile can return TT codes (4/6/7) for HD-friendly modes.

> **Under review (TASK/XBIOS23, 2026-11-12):** the ground below (16-colour
> framebuffer) was superseded by ADR-024 (truecolor VDI); options and the draft
> amendment are in `TASK/XBIOS23.md`, "Draft for the owner". Until the owner decides,
> the code still returns `TT_MEDIUM`.

### NewTOS behavior (settled — ADR-017 rev 2, 2026-07-16)

`getrez()` returns a **fixed constant `TT_MEDIUM` (4)** —
`tos-api/src/xbios/screen.rs`. NewTOS is a native **16-colour** framebuffer,
and `TT_MEDIUM` (640×480, 16 colours) is the classic code whose *colour depth*
matches. `Getrez` here reports **colour depth, not geometry**: the one thing a
coarse rez code meaningfully conveys about NewTOS that VDI `DEV_TAB` /
`Physbase` do not is "16 colours / *not monochrome*". A program that read
`ST_HIGH` (2, monochrome) would wrongly avoid colour; `TT_MEDIUM` is the honest
answer.

The native geometry is **config.txt-driven and auto-detected at boot** (ADR-017
rev 2) and has **no classic `vmode_table[]` entry** (`bios/screen.c:724-735`),
so the code deliberately does not attempt to encode 1920×1080 / 1280×800 as a
rez value — geometry is read from VDI `DEV_TAB` / `Physbase`. There is **no
in-system switch**: `Getrez` only reports, it never changes (see
`05-setscreen.md`). Revisit at the truecolor VDI step, where a wider-depth code
may become the more honest answer.

---

## Pi / BCM2711 binding

> Re-points the *hardware reference* (Shifter/Videl mode register, or the
> `sshiftmod` shadow at `0x44C`) at the live NewTOS framebuffer mode. The
> **contract is unchanged** — a WORD resolution code, no error path.
> Peripheral + driver: see `docs/HARDWARE-PROFILE.md` §2/§3.1.

> **Resolution policy (ADR-017 rev 2, 2026-07-16, `docs/MEMORY.md`):**
> NewTOS renders **natively at the config.txt-driven HDMI geometry**
> (auto-detected at boot; `drivers::BOOT_WIDTH/BOOT_HEIGHT` = 1920×1080 is the
> fallback / QEMU-ramfb default), at 16 VDI colours. There is **no in-system
> resolution switch** — the firmware `config.txt` owns the mode. This rev 2
> **settles** the previously-open `Getrez`-value question below.

- **Settled: `Getrez` returns the fixed constant `TT_MEDIUM` (`4`).** The
  return value reports **colour depth (16 colours / not monochrome), not
  geometry** — the live geometry has **no** `vmode_table[]` entry
  (`bios/screen.c:724-735`) and is read from VDI `DEV_TAB` / `Physbase`
  instead. `TT_MEDIUM` is chosen over `ST_HIGH` (2) precisely because
  `ST_HIGH` means *monochrome* and would make colour-aware programs avoid the
  16-colour palette NewTOS actually has.
- **EmuTOS precedent for a fixed off-table code (retained reference).** EmuTOS
  itself has no single mandated answer for an off-`vmode_table` mode and
  simply hardcodes one: `atari_getrez()` (`bios/screen.c:948-971`) returns a
  fixed `rez=2` (`ST_HIGH`) for *any* non-ST-compatible Videl mode regardless
  of its real geometry/depth, while `vsetmode()` (`bios/videl.c:619-659`) sets
  the shadow `sshiftmod=FALCON_REZ` (`:648-656`) for the same case. NewTOS
  follows the same "one honest fixed code for the off-table native mode"
  shape — choosing `TT_MEDIUM` because colour depth, not geometry, is what the
  code can truthfully carry. Real Falcon software queries `VsetMode(-1)` for
  ground-truth geometry; NewTOS software reads VDI `DEV_TAB` / `Physbase`.
- **`Setscreen`'s `rez` is correspondingly accepted-but-ignored** — see
  `docs/api/xbios/05-setscreen.md` (Pi binding).
- **A NewTOS-native true-geometry query** (cookie-jar entry or new trap
  returning live width/height/depth) remains a possible future refinement,
  mirroring the Falcon `Getrez`(hint)/`VsetMode`(truth) split; not needed
  today since VDI `DEV_TAB` / `Physbase` already carry the live geometry.
