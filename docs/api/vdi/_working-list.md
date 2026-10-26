# VDI spec extraction — working list (Phase 00b)

> Tracks the per-function VDI spec extraction from EmuTOS @ `1a7484ab`.
> Convention mirrors `docs/api/_working-list.md`. The trap
 /
> calling convention + full opcode map is done — see
> [`_dispatcher.md`](_dispatcher.md). Per-function specs go in
> `docs/api/vdi/<group>/<op>-<name>.md`.
>
> Status: ☐ pending · ◐ drafted · ☑ done. Batches are ordered so the
> Phase-12 implementation can start at the top (lifecycle → attributes
> → output → inquiry → raster/mouse).

## Source map (EmuTOS `vdi/`)

| File | Covers |
|------|--------|
| `vdi_main.c` | dispatch, jump tables, `screen()` entry — **done** (`_dispatcher.md`) |
| `vdi_control.c` | `v_opnwk` / `v_opnvwk` / `v_clswk` / `v_clsvwk` / `v_clrwk`, `vq_extnd`, `vs_clip` |
| `vdi_line.c` | `v_pline`, `vsl_*` (line attributes), `vsl_ends`/`vsl_type`/`vsl_width`/`vsl_color`/`vsl_udsty` |
| `vdi_marker.c` | `v_pmarker`, `vsm_*` |
| `vdi_text.c` / `vdi_textblit.c` / `vdi_tblit.S` | `v_gtext`, `vst_*`, `vqt_*`, font load/unload |
| `vdi_fill.c` | `v_fillarea`, `v_contourfill`, `vr_recfl`, `vsf_*` |
| `vdi_gdp.c` | `v_gdp` (bar/arc/pieslice/circle/ellipse/rounded-rect/justified-text) |
| `vdi_col.c` | `vs_color`, `vq_color`, palette |
| `vdi_raster.c` / `vdi_blit.S` | `vro_cpyfm`, `vrt_cpyfm`, `vr_trnfm`, `v_get_pixel` |
| `vdi_mouse.c` | `v_show_c`/`v_hide_c`, `vsc_form`, `vq_mouse`, `vex_*v`, cursor |
| `vdi_input.c` | `v_locator`/`v_choice`/`v_string`, `vsin_mode`/`vqin_mode`, `vq_key_s` |
| `vdi_esc.c` | `v_escape` sub-functions (cursor/alpha text, inquire) |
| `vdi_misc.c` | `vex_timv`, helpers |
| `vdi_defs.h` | `Vwk`, opcode constants, attribute defaults — **structure spec TODO** |
| `lineavars.S` / `lineaasm.S` | Line-A globals + primitives — see `docs/api/_line-a.md` |

## Batch 1 — workstation lifecycle + clear (D2)

- [x] `v_opnvwk` (100) — [`workstation/100-v_opnvwk.md`](workstation/100-v_opnvwk.md):
      handle alloc + `init_wk` (11 `work_in`) + 45 `INTOUT` (`DEV_TAB`)
      + 12 `PTSOUT` (`SIZ_TAB`). Full work_in/work_out tables.
- [x] `v_opnwk` (1) — [`workstation/01-v_opnwk.md`](workstation/01-v_opnwk.md):
      physical screen ws; `work_in[0] = rez + 2`; shares `init_wk`.
- [x] `v_clsvwk` (101) / `v_clswk` (2) —
      [`workstation/101-v_clsvwk.md`](workstation/101-v_clsvwk.md).
- [x] `v_clrwk` (3) —
      [`workstation/03-v_clrwk.md`](workstation/03-v_clrwk.md). `v_updwk` (4) = no-op.
- [x] `vq_extnd` (102) —
      [`workstation/102-vq_extnd.md`](workstation/102-vq_extnd.md):
      mode 0 = standard tables, mode 1 = `INQ_TAB` (45) + live clip rect.
- [x] **`Vwk` structure spec** →
      [`../structures/vwk.md`](../structures/vwk.md): the attribute area
      grouped (line/marker/fill/text/mode/clip), with the ADR-011
      per-device `device` link noted.

**Batch 1 complete.** Workstation lifecycle (D2) done.

> **Refinement pass 2026-07-03 (full-contract).** Re-verified D1
> (`_dispatcher.md`), D2 (`workstation/*.md`), and D6-escape
> (`_escape.md`) against `vdi/vdi_control.c` / `vdi_main.c` / `vdi_esc.c`
> / `vdi_asm.S` / `vdi_defs.h` at `1a7484ab`. Removed all pasted C bodies
> (cite-never-paste), lifted the docs to the sharpened full-contract
> template, added a workstation-level [`_state.md`](_state.md), and logged
> the corrections in [`_discrepancies.md`](_discrepancies.md) D-VDI-2..7.
> Key fixes: handle cap is `NUM_VDI_HANDLES` (64/128, **not 16**);
> `vq_tabstatus` returns 1 (not 0); `v_hardcopy` calls `Scrdmp()`;
> `DEV_TAB[35]` ROM=0 patched to 1; VDI trap ABI is `trap #2 / d0=0x73 /
> d1=VDIPB*` with the `PTSIN`/`MAX_VERTICES` clamp; opcodes 132-134 are
> `CONF_WITH_EXTENDED_MOUSE`-gated.

> **Re-validation pass 2026-07-09 (bottom-up sweep, VDI leg — dispatcher +
> lifecycle + inquiry).** Re-read `_dispatcher.md`'s dispatch/no-op claim
> line-by-line against `vdi_main.c:121-192`, `vdi_control.c:178-184`,
> `vdi_asm.S`, `aes/gemdosif.S:222-304`, `bios/bios.c:173` — **confirmed
> correct**, logged as [`_discrepancies.md`](_discrepancies.md) via
> [`../_discrepancies.md`](../_discrepancies.md) **D-079** (global log;
> this file's own `D-VDI-N` series continues to track VDI-source-vs-doc
> conflicts locally). Added the explicit three-way contrast (GEMDOS
> `EINVFN` / BIOS-XBIOS function-number echo / VDI silent no-op) to
> `_dispatcher.md`, and fixed an imprecise trap-chain description
> (`_vditrap` does not itself forward anywhere; `_aestrap` sits above it
> and tail-calls the saved vector). Re-verified `v_opnwk`/`v_opnvwk`/
> `v_clswk`/`v_clsvwk`/`v_clrwk`/`v_updwk`/`vq_extnd`/`vq_color` against
> `vdi_control.c`/`vdi_col.c`/`bios/screen.c` — all byte-index tables and
> citations confirmed accurate, no source-behaviour correction needed.
> One semantic finding: `DEV_TAB[39]` ("palette size" per EmuTOS's own
> comment) is actually `get_palette()`'s RGB-precision figure (4096/2/512/
> 0), a different quantity than `DEV_TAB[13]`'s colour count — flagged as
> Mismatch #4 in [`workstation/100-v_opnvwk.md`](workstation/100-v_opnvwk.md#mismatches-flagged-step-2-candidates)
> since NewTOS currently reports the colour count there instead. Added
> ADR-017 cross-references to the Pi/BCM2711 binding sections of
> `01-v_opnwk.md`/`100-v_opnvwk.md`/`102-vq_extnd.md`: the 640×480×16
> numbers there are today's *staged* bring-up geometry (ADR-017's Staging
> rule), not the final NewTOS target — the VDI table code already reads
> the live `RasterBackend` geometry, so no VDI code change is needed when
> the framebuffer driver's target geometry is raised.

## Batch 2 — attribute setters (D3)

- [x] line + marker — [`attributes/_line-marker.md`](attributes/_line-marker.md):
      `vsl_type` (15), `vsl_width` (16), `vsl_color` (17),
      `vsl_ends` (108), `vsl_udsty` (113); `vsm_type` (18),
      `vsm_height` (19), `vsm_color` (20).
- [x] fill — [`attributes/_fill.md`](attributes/_fill.md):
      `vsf_interior` (23), `vsf_style` (24), `vsf_color` (25),
      `vsf_perimeter` (104), `vsf_udpat` (112).
- [x] text — [`attributes/_text.md`](attributes/_text.md):
      `vst_height` (12), `vst_point` (107), `vst_rotation` (13),
      `vst_font` (21), `vst_color` (22), `vst_effects` (106),
      `vst_alignment` (39).
- [x] mode / clip / colour —
      [`attributes/_mode-clip-color.md`](attributes/_mode-clip-color.md):
      `vswr_mode` (32), `vs_clip` (129), `vs_color` (14).

**Batch 2 complete.** Attribute setters (D3) done. Common shape:
validate (clamp to range, else default) → store in `Vwk` → echo the
accepted value; none can fail.

> **Re-validation pass 2026-07-10 (bottom-up sweep, VDI leg — attribute
> setters).** Re-read every setter's implementation incl. helpers
> line-by-line against `vdi_line.c`, `vdi_marker.c`, `vdi_fill.c`,
> `vdi_text.c`, `vdi_control.c`, `vdi_col.c`, `vdi_defs.h`, and the
> `jmptb1`/`jmptb2` output-count declarations in `vdi_main.c` at
> `1a7484ab`. All clamp ranges, defaults, 0-/1-based storage, and echoed
> values in all four spec files (`_line-marker.md`, `_fill.md`,
> `_text.md`, `_mode-clip-color.md`) were confirmed **accurate**,
> including the "echo the actually-set (clamped/quantised) value, not
> the request" contract for every setter, and every `PTSOUT`/`INTOUT`
> word count cross-checked against the jump table. (Note: the task
> brief's opcode list had `vsl_udsty` at 112 — the source and the
> existing spec both correctly have it at **113**; opcode **112** is
> `vsf_udpat`. No doc change needed there, already right.) Four findings
> from this pass:
> - **D-VDI-15** (local): `_line-marker.md`'s "NewTOS implementation
>   notes" cited `validate_color_index` as `vdi_col.c:224-230` — wrong
>   file, it's `vdi_control.c:224-230`. Fixed (pure citation typo, no
>   semantic error).
> - **D-VDI-16** (local): `_mode-clip-color.md` described `vswr_mode`'s
>   explicit `CONTRL[4] = 1` as "overriding the jump-table pre-load" —
>   overstated; the jump table already declares `nintout = 1` for op 32
>   and `vswr_mode` has no other call site, so the line is redundant,
>   not an observable override. Reworded.
> - **D-080** (global): `vsm_height` (19), `vst_height` (12), `vst_point`
>   (107), and (as a leaked side effect) `vst_font` (21) set the global
>   `flip_y` to `1`, which the raw trap returns in `d0`
>   (`vdi_asm.S:124-125`) — previously undocumented in the attribute
>   specs even though `_dispatcher.md` already treats `d0 = flip_y` as
>   part of the native-trap contract. Documented in `_line-marker.md`
>   and `_text.md`.
>
>   **Answered (A53, 2026-09-20; fixed 2026-09-12 by V16/`675e311`):** this
>   entry used to log an open question ("whether NewTOS's selector-based ABI
>   needs an equivalent signal at all"). It does, and NewTOS's GEM trap now
>   produces it: `kernel/src/arch/aarch64/exceptions.rs`, selector 115
>   returns `i64::from(vdi::dispatch(...))`, i.e. `flip_y` in `x0`
>   (previously a hard-wired `0`, the V16 stub found and fixed by the
>   same commit). Verified against the pinned source in
>   `docs/analysis/vdi-r2-improvement-traced-2026-09-12.md` §3.
> - **D-081** (global): `vsf_interior` never resets/re-validates
>   `fill_index` when `fill_style` changes; `st_fl_ptr()` re-interprets
>   the stale value under the new style with no bounds check. A
>   `FIS_PATTERN` (`fill_index` up to 23) → `FIS_HATCH` switch without an
>   intervening `vsf_style` call reads up to 176 words past the end of
>   `HATCH1[96]` — a genuine EmuTOS source-level out-of-bounds gap
>   (unlike the line-style table, which does guard its index). Documented
>   in `_fill.md` as a new edge case; open question logged for whether
>   NewTOS should reproduce or defensively fix this.
>
> All four opcode groups (line/marker, fill, text, mode/clip/color —
> 23 opcodes total) are now re-validated and ready for the Phase-12
> code-audit batch against `tos-api/src/vdi/attributes.rs`.

## Batch 3 — output primitives (D4)

- [x] lines + markers — [`output/_lines-markers.md`](output/_lines-markers.md):
      `v_pline` (6), `v_pmarker` (7).
- [x] text — [`output/_text.md`](output/_text.md): `v_gtext` (8)
      (`output_text` blit path; justified variant = GDP 10).
- [x] fill — [`output/_fill.md`](output/_fill.md): `v_fillarea` (9),
      `vr_recfl` (114), `v_contourfill` (103).
- [x] GDP — [`output/_gdp.md`](output/_gdp.md): `v_gdp` (11) sub-ops
      1–10 (bar/arc/pieslice/circle/ellipse/ellarc/ellpie/rbox/rfbox/
      justified-text); `CONTRL[5]` selects.

**Batch 3 complete.** Output primitives (D4) done. All reduce to the
line/fill/text framebuffer ops; `v_gdp` adds geometry only. Bezier
(`CONTRL[5]==13`) deferred per build-config scope.

> **Re-validation pass 2026-07-10 (VDI leg — output primitives, Batch
> 3).** Re-read every output primitive's implementation incl. helpers
> line-by-line against `vdi_line.c`, `vdi_marker.c`, `vdi_text.c`,
> `vdi_fill.c`, `vdi_gdp.c`, and the `jmptb1`/`jmptb2` output-count
> declarations in `vdi_main.c` at `1a7484ab`. Confirmed **accurate**
> across all four spec files (`output/_lines-markers.md`, `_text.md`,
> `_fill.md`, `_gdp.md`): the `PTSIN`/`CONTRL[1]` vertex/point-count
> conventions, the "no output" shape (`{nptsout,nintout}={0,0}` for
> opcodes 6/7/8/9/11/103/114, cross-checked against every jump-table
> entry individually), the per-primitive `wrt_mode` application, the
> `vwk->clip`-gated clipping on every primitive (`v_pline`'s
> `clip_line`, `polygon`'s y-band clip, `vr_recfl`'s `clipbox`,
> `contourfill`'s seed-in-clip guard, `gdp_curve`'s bounding-box
> early-out), the GDP sub-opcode (`CONTRL[5]` 1–10) parameter layout
> including the circle-vs-arc/pieslice radius slot difference
> (`PTSIN[4]` vs `PTSIN[6]`), the open/closed + line-vs-fill-colour
> perimeter rules, and `v_justified`'s `CONTRL[3]-2` char count. `HAVE_
> BEZIER=0` (`vdi_defs.h:18`) confirmed to gate out `vdi_bezier.c`
> wholesale (the entire file body, lines 21-572, is `#if HAVE_BEZIER`)
> and every `CONTRL[5]==13`/Bezier branch in `vdi_line.c`/`vdi_fill.c`/
> `vdi_gdp.c` — out of scope, correctly noted in all four files.
> `v_cellarray` (10) confirmed a hard `vdi_v_nop` in `jmptb1`
> (`vdi_main.c:42`, comment "not usually implemented by drivers") —
> already documented in [`_dispatcher.md`](_dispatcher.md), not an
> output primitive. Two findings:
> - **D-VDI-17** (local): `output/_gdp.md` edge case 6 mislabelled the
>   `del_ang`-wrap example — called the un-drawn 2400° span "the short
>   one" when the wrapped 1200° span EmuTOS actually draws is the
>   shorter of the two. Reworded with the explicit arithmetic.
> - **D-VDI-18** (local): `attributes/_text.md`'s "NewTOS implementation
>   notes" (`:213-215`) contradicted that same file's own contract and
>   verification criteria (`:26,76-79,113-114,127-129`) by claiming
>   "3600 folds to 0" for `vst_rotation`. Source (`vdi_text.c:924-935`)
>   holds no modulo — `chup=3600`/`INTOUT[0]=3600` is genuinely
>   stored/echoed for inputs normalising to `3150..3599`. Only
>   `output_text`'s *rendering* switch (`vdi_text.c:408-441`, no `case
>   3600`) treats it like `0` visually. Corrected the implementation
>   note to separate "stored value" from "render path" — the Developer
>   was concurrently removing an erroneous `% 3600` from
>   `tos-api/src/vdi/attributes.rs`'s `vst_rotation`, which this wrong
>   note likely motivated.
>
> All four output-primitive spec files are re-validated and ready for
> the Phase-12 code-audit batch against `tos-api/src/vdi/primitives.rs`,
> `gdp.rs`, `text.rs`, and `contour.rs`.

## Batch 4 — inquiry (D5)

- [x] attribute inquiries — [`inquiry/_attributes.md`](inquiry/_attributes.md):
      `vql_attributes` (35), `vqm_attributes` (36), `vqf_attributes`
      (37), `vqt_attributes` (38). (Note the `vqm` 0-based quirk.)
- [x] text metrics — [`inquiry/_text.md`](inquiry/_text.md):
      `vqt_extent` (116), `vqt_width` (117), `vqt_name` (130),
      `vqt_fontinfo` (131).
- [x] colour / pixel / input mode —
      [`inquiry/_color-pixel-input.md`](inquiry/_color-pixel-input.md):
      `vq_color` (26), `v_get_pixel` (105), `vqin_mode` (115),
      `vq_key_s` (128).
- `vq_extnd` (102) already done in
      [Batch 1](workstation/102-vq_extnd.md).

**Batch 4 complete.** Inquiry (D5) done — all read-only `Vwk`/device
state; only `vq_color`/`vqt_width` have a `-1` out-of-range marker.

## Batch 5 — raster + mouse + input (D6)

- [x] raster — [`raster/_raster.md`](raster/_raster.md): `vro_cpyfm`
      (109), `vrt_cpyfm` (121), `vr_trnfm` (110) + MFDB struct +
      BitBlt logic ops.
- [x] mouse + vectors — [`input/_mouse.md`](input/_mouse.md):
      `v_show_c` (122), `v_hide_c` (123), `vsc_form` (111),
      `vq_mouse` (124), `vex_butv/motv/curv/wheelv` (125–127, 134),
      `vex_timv` (118).
- [x] input — [`input/_input.md`](input/_input.md): `v_locator` (28),
      `v_choice` (30), `v_string` (31), `vsin_mode` (33). (`vqin_mode`
      115 done in Batch 4; no `v_valuator` in EmuTOS.)
- [x] `v_escape` (5) sub-functions (0–19) —
      [`_escape.md`](_escape.md): alpha/VT52 cursor + inquiries.

**Batch 5 complete.** Raster/mouse/input/escape (D6) done. Raster =
real BitBlt (all 16 ops). Mouse/input bind to the not-yet-existing
QEMU input subsystem → REQUEST-mode blocks on the real event queue,
no canned values.

> **Re-validation pass 2026-07-10 (raster + mouse/input).** Re-checked
> `raster/_raster.md` against `vdi_raster.c` (full file, `vdi_vro_cpyfm`/
> `vdi_vrt_cpyfm`/`vdi_vr_trnfm`/`cpy_raster`/`bit_blt`/`do_blit`/
> `setup_info`/`setup_pattern`) and `vdi_blit.S`'s `P_ADDR` test, and
> `input/_mouse.md` + `input/_mouse-packet.md` + `input/_input.md`
> against the full `vdi_mouse.c`, `vdi_input.c`, `vdi_asm.S`, and the
> `vdi_main.c` jump tables at `1a7484ab`. MFDB layout, the 16-op BitBlt
> table, `MD_*` codes, `v_get_pixel`'s two-value output, all
> opcode/output-count dispatcher entries, the hide-count model, the
> `vex_*` exchange shape, and the four logical-input-device
> request/sample contracts were all confirmed **accurate**. Four
> findings:
> - **D-082** (global): `PAT_FLAG` (bit 5) on `vro_cpyfm`/`vrt_cpyfm`/
>   line-A raster copy never actually applies a pattern — `cpy_raster`
>   nulls the pattern pointer unconditionally before the flag is even
>   tested, and `setup_pattern` never restores it. A genuine EmuTOS
>   source-level gap dating to commit `d7ac3170` (2013), confirmed dead
>   across every build path (assembler `fast_bit_blt` and the C
>   `bit_blt`/`do_blit` fallback alike).
> - **D-VDI-19** (local): `input/_mouse-packet.md`'s `cur_ms_stat`
>   bit-6/7 table had left/right-button-changed backwards, copying the
>   source's own wrong docstring comment (`vdi_mouse.c:186-193`) instead
>   of the actual bit assignment computed by the mouse ISR
>   (`vdi_asm.S:295-297`) and consumed by `gloc_key`
>   (`vdi_mouse.c:203-207`) — both of which agree with each other and
>   disagree with the comment. Corrected to bit 6 = left-changed, bit 7
>   = right-changed.
> - **D-VDI-20** (local): `input/_mouse.md`'s default `arrow_mform`
>   `mf_bg`/`mf_fg` values were swapped (a positional-initializer
>   misread). Corrected to `mf_bg=0, mf_fg=1`.
> - **Flagged, not fixed (STOP):** `input/_input.md`'s "Atari-TOS bug
>   preserved" claim for `vrq_string`'s extended-mode (`INTIN[0]<0`) CR
>   handling does not survive a line-by-line trace of the current
>   `vdi_v_string` C code — the CR test is hard-coded to `& 0x00ff`,
>   which behaves identically regardless of `mask`. Left as an open
>   question in the doc (candidate discrepancy, no D-VDI number
>   assigned) pending empirical confirmation, since it contradicts an
>   explicit EmuTOS source comment and this tool could not execute code
>   to verify beyond a manual trace.
>
> Raster and input/mouse specs are re-validated and ready for the
> Phase-12 code-audit batch against `tos-api/src/vdi/raster.rs`,
> `backend.rs`, `input.rs`, and `mouse.rs`.

---

**PHASE-00b VDI spec extraction COMPLETE** — D1 (dispatcher) +
D2–D6 (all ~70 opcodes). Per-function semantics extracted from EmuTOS
`1a7484ab`; ready for Phase-12 implementation against `docs/api/vdi/`.

---

## Implementation status (Phase 12) — COMPLETE

Every functional opcode is implemented in `tos-api/src/vdi/` and routed
in `mod.rs`; only the genuine VDI no-ops remain unrouted (`v_updwk` 4,
`v_cellarray` 10/27, `v_valuator` 29, `vqt_justified` 132,
`vs_grayoverride` 133). Verified end-to-end via the `svc #2` trap path
(QMP screendump, 8/8 pixel checks).

| Area | Opcodes | Module |
|------|---------|--------|
| Workstation | 1,2,3,100,101,102 | `workstation.rs` |
| Attributes | 12–25,32,39,104,106–108,112,113,129 | `attributes.rs` |
| Palette | 14 (+ vq_color 26) | `attributes`/`inquiry` + backend |
| Output | 6,7,8,9,11(GDP),114 | `primitives.rs`, `gdp.rs`, `text.rs` |
| Curves | GDP 2–9 (circle/ellipse/arc/pie/rbox) | `gdp.rs` + `trig.rs` |
| Flood fill | 103 | `contour.rs` |
| Inquiry | 26,35–38,105,115–117,128,130,131 | `inquiry.rs` |
| Raster | 109,110,121 | `raster.rs` |
| Mouse/input | 28,30,31,33,111,118,122–127,134 | `mouse.rs`, `input.rs` |
| Escape (alpha) | 5 (sub 0–19) | `escape.rs` |
| Text effects | thicken/light/skew/underline in v_gtext | `text.rs` |

**Documented bring-up limitations** (not stubs): mouse position/buttons
are parked until the IKBD mouse driver feeds `mouse::set_state`;
`v_hardcopy` is a no-op (no printer); a single fixed 8×16 face, so the
font-metric calls report its real metrics; Truecolor/Bezier out of scope.

## Build-config scope (TBD-ADR, matches the Phase-00 profile)

- `CONF_WITH_VDI_16BIT` (Truecolor) — **out** for bring-up (8-bit /
  palette screen first).
- `CONF_WITH_EXTENDED_MOUSE` (wheel) — optional.
- Bezier (`vdi_bezier.c`) — GDP extension, low priority.

## Discrepancy log

Open `docs/api/vdi/_discrepancies.md` when the first source-vs-doc
conflict appears (mirrors the GEMDOS/XBIOS `_discrepancies.md`).
