# 14 — PRESENT.NAP: the slide show

Stand 2026-10-08, TASK/PRESENT01. Our own GEM application (not a port) for
showing slides full screen on NewTOS — written for the Atari meeting talk, but
an ordinary SDK program: one C file, `sdk/apps/present/present.c`, built by
`sdk/apps/present/build-present.sh` against gemlib, libpng, zlib, OpenLibm and
mintlib (the archives of [11](11-codec-libraries.md), the gemlib flags of
[03](03-first-programs.md)). Evidence:
`measurements/measurement-2026-10-08-present/RUNINFO.TXT`.

## 1. Files

| Where | What |
|---|---|
| `C:\APPS\PRESENT\PRESENT.NAP` | the program (start by double-click) |
| `C:\APPS\PRESENT\SLIDES.TXT` | the slides — plain text, edit without rebuilding |
| `C:\APPS\PRESENT\*.PNG` | pictures named by `@image` |
| `C:\APPS\PRESENT\PRESENT.LOG` | timings, written on exit only if the file says `@log` |

The slide file is `argv[1]` if one is given (drop a `.TXT` on the program),
else `SLIDES.TXT` in the current directory (the desktop starts a program in its
own folder), else `C:\APPS\PRESENT\SLIDES.TXT`. Pictures are looked up beside
the slide file unless the name is absolute (`C:\...`).

`tools/make-sd-image.sh` installs the program with the **sample** slide file
`sdk/apps/present/sample/SLIDES.TXT` and its test picture `TEST.PNG`
(`sdk/apps/present/sample/gen-sample.py`) into `C:\Apps\PRESENT\` of the QEMU image. A real card
gets its slides by hand.

## 2. Slide file format

Plain ASCII, LF or CRLF. UTF-8 is accepted and converted to the Atari
character set (ISO-8859-1 letters with the table of EmuTOS `tools/bug.c`
`i2a[]`, e.g. ä ö ü ß é; typographic quotes, dashes and `…` become ASCII; `•`
becomes the middle dot; anything else becomes `?`). One line each:

| Line | Meaning |
|---|---|
| `# Title` | starts a new slide with this title (column 0) |
| `- text` or `* text` | bullet |
| `  - text` | sub-bullet (indented by 2+ spaces or a tab) |
| `text` | plain paragraph, no bullet |
| *(blank)* | vertical space (several in a row count once; none at the top or end of a slide) |
| `@image NAME.PNG` | picture for this slide (the last one counts) |
| `@center` | centred layout (title slide): large title, accent bar, centred lines |
| `@note text` | speaker note — ignored |
| `; text` | comment — ignored |

Everything before the first `# ` line is a free header and is not shown,
except three global directives that only work there:

| Line | Meaning |
|---|---|
| `@footer text` | small text bottom left on every slide |
| `@log` | write `PRESENT.LOG` (per slide: how often shown, longest render / picture / blit time) beside the slide file on exit |
| `@spacing N` | extra space after each bullet, sub-bullet or text line, in twelfths of that item's character height, `0`..`24` (larger values are cut to 24; no number = ignored). Default `4` (a third of a line). Lines wrapped inside one item keep their tight distance (1/8), so a wrapped bullet still reads as one block. More spacing makes a full slide shrink to the next smaller size sooner |

Unknown `@` directives are ignored. Limits: 64 slides, 40 lines per slide
(more are dropped), file up to 256 KB. Long lines wrap at blanks; a word longer
than the line is broken hard. The bottom right shows `n / N`.

Example:

```
@footer NewTOS - Atari meeting 2026

# NewTOS
@center
A TOS-compatible operating system
for the Raspberry Pi 4

# What runs today
- GEMDOS, BIOS, XBIOS, VDI and AES
  - EmuTOS-faithful
- Ported GEM apps
@image DESKTOP.PNG
@note mention QED and TeraDesk
```

## 3. Keys

| Action | Keys |
|---|---|
| next slide | Right, Down, Space, Return, Enter, N, left mouse button |
| previous slide | Left, Up, Backspace, P, right mouse button |
| first / last | ClrHome / Shift+ClrHome |
| quit | Esc, Q |

PageUp, PageDown and End are **not** available: the NewTOS USB keyboard map
(`kernel/src/drivers/usb/hid.rs`, `translate_key`) sends nothing for those USB
keys, so presentation remotes that send PageUp/PageDown do nothing. Remotes
that send cursor keys work.

## 4. Look and layout

Dark blue background, white titles, an orange accent bar under the title,
light text, orange bullet dots, sub-bullets with a dash in a dimmer colour.
Sizes follow the screen height (the system font cell is 8×16):

| Screen | Title | Bullets | Sub-bullets | Footer |
|---|---|---|---|---|
| 1920×1080 | ×4 (32×64) | ×3 | ×2 | ×2 |
| 1024×768 | ×3 | ×2 | ×2 | ×1 |
| 640×480 | ×2 | ×2 (×1 on a full slide) | same as bullets | ×1 |

A centred (`@center`) title is two steps larger (×6 at 1080 lines). If the
bullets of a slide do not fit, the slide's bullet size steps down until they do
(then the rest is clipped at the bottom); a title takes at most two lines and
shrinks otherwise. With a picture the text gets the left 56 % and the picture
the right part, centred; a slide with only a picture uses the whole area. A
picture is enlarged by the largest whole factor that fits (sharp pixels — good
for screenshots) or reduced with a box filter, keeping the aspect ratio; its
alpha channel is blended onto the background. A missing or unreadable picture
shows a small note `[picture not found: NAME]` / `[not a PNG picture: …]` /
`[cannot decode picture: …]` instead; nothing else changes.

## 5. How it works

* **Screen ownership.** `wind_update(BEG_UPDATE)` + `wind_update(BEG_MCTRL)`,
  `form_dial(FMD_START)` over the whole screen, `graf_mouse(M_OFF)`; on exit
  `M_ON`, `form_dial(FMD_FINISH)` (the AES has the area redrawn),
  `END_MCTRL`, `END_UPDATE`. Started from the desktop, the desktop is back after
  Esc.
* **Large text from the system font.** The 224 cells 32…255 are drawn once with
  `v_gtext` (black on white, top-aligned) and copied back with `vro_cpyfm` into
  a memory MFDB — on NewTOS a memory MFDB is 16-bit RGB565 chunky
  (`tos-api/src/vdi/raster.rs`), so ink is "pixel ≠ background". Larger sizes
  come from those bitmaps with Scale2x / Scale3x (EPX; ×4 = 2×2, ×6 = 3×2,
  ×8 = 2×2×2), which smooth diagonals instead of producing blocks; each glyph is
  scaled once and cached.
* **One blit per slide.** A slide is composed into one off-screen RGB565 frame
  (screen size, stride rounded to 16 pixels) and copied to the screen with a
  single `vro_cpyfm(S_ONLY)`. Measured in QEMU at 1920×1080 (`@log`, 5 ms
  clock): compose ≤ 5 ms, blit 15–20 ms, picture decode + scale 40–45 ms once.
* **Pictures.** libpng (static) → RGBA8 (`png_set_expand`, `strip_16`,
  `gray_to_rgb`, filler), size limit 8192×8192 / 16 Mpixel, own error handler
  (`setjmp`), so a broken file never reaches the screen console. The scaled
  picture is cached per slide.
* **Events.** `evnt_multi(MU_KEYBD | MU_BUTTON | MU_MESAG)`; the button wait is
  "any button goes down" (`clicks = 0x101`, mask 3, state 0 — the flag in bit 8
  inverts the match, `tos-api/src/aes/input.rs` `downorup`) with a single click,
  so a press is reported at once with the pressed button; the next wait is for
  the release. `AP_TERM` quits.

## 6. Build

```sh
sdk/apps/present/build-present.sh      # -> sdk/apps/present/build/PRESENT.NAP + TEST.PNG
tools/make-sd-image.sh /tmp/sd.img 64  # installs C:\Apps\PRESENT\ (QEMU image)
```
