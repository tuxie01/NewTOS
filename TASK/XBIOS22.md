# XBIOS22 — Falcon video queries: `VgetMonitor`, `VgetSize`, `VgetRGB` answer for the Pi screen

Status: OPEN (2026-11-12) | Priority: P2

Type: research (spec), then developer. Owner decision 2026-11-12, from the full XBIOS run
`measurements/measurement-2026-11-12-xbios-full/RUNINFO.TXT`.

## Owner decision 2026-11-12

"müssen wir anders lösen da wir die config.txt dazu anpassen müssen. Ausser VgetMonitor, VgetSize, VgetRGB brauchen
wir das nicht." (We have to solve this differently, since that would mean adapting config.txt. Apart from VgetMonitor,
VgetSize, VgetRGB we do not need it.)

- `VgetMonitor` (0x59), `VgetSize` (0x5B), `VgetRGB` (0x5E): implement, answering for the Pi's screen.
- `VsetMode` (0x58) and `Vfixmode` (0x5F): not implemented; a mode switch at run time would need `config.txt`
  (ADR-017). They keep answering their function number; a sanctioned-deviation entry in `docs/COMPATIBILITY.md`
  records it (part of this ticket). The other Videl calls (0x5A, 0x5C-0x5D, ...) are not in scope.

## Facts

- `tosdump/cases/bios-r1.tsv` cases 211-215 differ (falcontc; 215 also tt): oracle `VsetMode(-1)` 0x1C,
  `VgetMonitor` 2, `VgetSize(0x1C)` 614400, `VgetRGB(0,16,…)` 0 plus the palette, `Vfixmode(0x1C)` 0x1C (tt -32);
  NewTOS answers the function number and leaves the buffer untouched (catch-all arm,
  `tos-api/src/xbios/mod.rs:157@other`).
- EmuTOS: `references/emutos/bios/videl.c:664` (`vmontype`), `:724` (`vgetsize`), `:862` (`vgetrgb`).
- The screen is a native 32-bpp framebuffer with a Falcon RGB565 VDI device pixel (ADR-024), geometry from
  `config.txt` (ADR-017).

## Open for the owner (from the spec step, before code)

- Which monitor type `VgetMonitor` reports for HDMI (EmuTOS knows mono, colour, VGA, TV), and which mode word
  `VgetSize` accepts (a Falcon mode word cannot express 1920x1080; whether it answers for the current screen only).
- What `VgetRGB` returns in a truecolor mode (EmuTOS: the Videl palette registers), and its relation to the
  `Setcolor` shadow (XBIOS21).
- Whether `VsetMode(-1)` (the query of the current mode, used by Falcon programs to detect the mode) may stay
  unanswered; the owner's decision above excludes `VsetMode` as a whole.

## Work

1. Researcher: spec entries for 0x59, 0x5B, 0x5E from `references/emutos/bios/videl.c` per
   `docs/SOURCE-ANALYSIS-PROTOCOL.md`, with the NewTOS mapping proposed for the points above; owner approval.
2. Developer: implement; COMPATIBILITY entry for `VsetMode`/`Vfixmode`.
3. Host tests per function: the documented return for documented input and the documented error.

## Acceptance

- Cases 212-214 equal where the spec says equal, otherwise listed in the COMPATIBILITY entry; 211 and 215 ACCEPTED
  by the entry. No case equal in the full XBIOS run differs anew.
- `cargo xtask test` green; warnings delta 0; doc-check of the edited docs 0 failures.

## Spec step (researcher, 2026-11-12)

Written (new): `docs/api/xbios/59-vgetmonitor.md`, `docs/api/xbios/5b-vgetsize.md`,
`docs/api/xbios/5e-vgetrgb.md`; cross-reference `docs/api/xbios/07-setcolor.md` ("Do `Setpalette` and
`VgetRGB` ..."). Source: `references/emutos/bios/videl.c:664` (`vmontype`), `:724` (`vgetsize`), `:862`
(`vgetrgb`); measured cases 212-214 (falcontc: 2, 614400 for `0x1C`, return 0 plus the Falcon default
palette; tt: function number).

Facts worth the owner's attention:
- The function-number answer NewTOS gives today is **EmuTOS's own no-Videl answer** (`videl.c:666-667`,
  `:730-731`, `:869-870`), so it is a source-valid "no Videl" claim; the questions are only about claiming Videl.
- `0x58` read as a mode word is `OVERSCAN|VGA|80COL`, 1bpp (a valid-looking mono word); `0x59` as a monitor type is
  out of range 0..3.
- No Falcon mode word can express 1920x1080 (no width field, VGA table tops at 640x480); `VgetSize(0x1C)` is
  614400 by formula (worked in the spec).
- `VgetRGB` limit is 16 entries in every mode except 8bpp (`videl.c:875-876`); the truecolor answer is the
  Falcon default table at `0xFF` full scale, not derived from `Setcolor`.
- `VsetRGB` (0x5D) is outside this ticket: `VgetRGB` is then a constant table.

Owner questions (options with consequences are in the three spec files):
1. `VgetMonitor` for HDMI: VGA 2 (recommended, equals oracle), RGB 1, mono 0, TV 3, or keep the function number.
2. `VgetSize(mode)`: A Falcon-table formula (case 213 equal; ignores the real screen), B current screen only
   (case 213 differs), C hybrid. If B or C: bytes of the 32-bpp `Physbase` screen (8 294 400 at 1920x1080,
   `tos-api/src/host.rs:579@size_bytes`) or of the 16-bit VDI device pixel (4 147 200)?
3. `VgetRGB` in truecolor: A separate Falcon table of 16 entries (recommended, equals case 214), B derived from
   the `Setcolor` shadow, C from the VDI table, D `-1`.
4. `VsetMode(-1)` (query only): leave unanswered (returns `0x58`, reads as a mono mode word; `Getrez` option
   in XBIOS23 interacts), or answer the query with a fixed word (e.g. `0x1C`-class, geometry inexpressible) while
   `VsetMode(mode != -1)` stays out. Also: publish the `_VDO` cookie (ADR-012 amendment, open with TASK/ARCH06)?
   Programs gate the Videl calls on it (`sdk/apps/teradesk/video.c:140-160`).

## Owner decision 2026-11-12

Owner 2026-11-12: "ich stimme allem so zu wie du vorgeschlagen hast" (I agree to everything as you proposed), answering the lead's recommendations.

1. `VgetMonitor` answers 2 (VGA), equal to the oracle.
3. `VgetRGB`: A, a separate Falcon table of 16 entries.
4. `VsetMode(-1)` (query only) answers a fixed word; `VsetMode(mode != -1)` stays out. The exact word is proposed
   in the spec (`docs/api/xbios/`) with its reasoning before the developer builds it; the lead brings it back.

Owner 2026-11-12, second answer: "VgetSize : B" and "_VDO-Cookie ja" (_VDO cookie yes).

2. `VgetSize(mode)`: B, the size of the current screen only (case 213 then differs from the oracle; a
   COMPATIBILITY entry). Which byte count (32-bpp `Physbase` screen, 8 294 400 at 1920x1080, or the 16-bit VDI
   device pixel, 4 147 200) is still asked.
5. `_VDO` is published: an ADR-012 amendment (with [ARCH06](ARCH06.md)); value and text drafted from the spec before
   the developer builds it.

Superseded by the answer above: Still open: question 2 (`VgetSize`: A, B or C, and for B/C which byte count), and whether to publish `_VDO`
(ADR-012 amendment, with [ARCH06](ARCH06.md)). The lead made no recommendation on these; the owner answers them
separately.
