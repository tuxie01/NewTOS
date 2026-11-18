# V64 spec gate, two questions: the dumper's `v_contourfill` extent, and the acceptance criteria (V64)

Researcher, 2026-11-18, ticket `TASK/V64.md`, main `ac3189f1a`. Research only: no product code, no ticket or ADR
edit. The spec itself is in `docs/api/vdi/raster/_raster.md` § "32-plane setting: pens, and every place a pen
becomes a pixel (ADR-038 point 4, V64)" and the pointers in `docs/api/vdi/attributes/_mode-clip-color.md`,
`docs/api/vdi/inquiry/_color-pixel-input.md`, `docs/api/vdi/output/_fill.md`. Labels: **[C]** read in the source
cited, **[I]** inferred, **[calc]** arithmetic over a formula, no kernel run.

## 1. Why the V63 tester run saw `v_contourfill` paint "only the seed pixel"

**Answer: the case design of the temporary table, not a NewTOS difference and not an EmuTOS behaviour of the fill.**

The input: `measurements/measurement-2026-11-18-v63-test/RUNINFO.TXT` item 7 NOTE ("the call painted the seed
pixel only, at every config including main") and "OUT-OF-SCOPE OBSERVATIONS" (`measurements/measurement-2026-11-18-v63-test/RUNINFO.TXT:51`, `:79`).

### 1.1 What the harness does [C]

- Every case of a DUMPVDI table starts with `td_reset_vdi`, whose first call is `v_clrwk`
  (`tosdump/core/runner.c:82@v_clrwk` `rs_call(3, ...)` "v_clrwk"; called per case at `tosdump/core/runner.c:132-133@td_reset_vdi`).
- A line whose opcode starts with `+` is a **prelude**: it is issued after the reset and before the **next**
  measured call, belongs to that one case only (`tosdump/core/cases.c:258-275@prelude` the `+` branch, `:320-324`
  `prelude_first`/`prelude_count = pending`, then `pending = 0`; run at `tosdump/core/runner.c:135-153@recorded`), and is not
  recorded. `tosdump/core/cases.h:14-19@PRELUDE` states the same ("Preludes stack; several may precede one case").
- So in the tester's table `measurements/measurement-2026-11-18-v63-test/method/tmp-v63-fill.tsv` the frame
  (`+6`, line 6), the fill colour and style (`+25`, `+23`) and the fill itself (`+103`, line 7:
  `+103 0 1 150,150`) are all preludes of **one** case, the first probe (line 8, `105 ... 150,150`). Every later
  probe (lines 9-20) is a new case that begins with `v_clrwk` and therefore reads a screen on which neither the
  frame nor the fill exists. The same structure repeats at lines 25 (`+103 ... -1`, probes 26-38) and 41
  (`+103 ... -1 on cleared`, probes 42 ff.).

### 1.2 What the recorded numbers show [C]

`measurements/measurement-2026-11-18-v63-test/checks/fill-pixels.txt` (columns main, default, v16, v32):

- case 8 (the seed, the only probe that inherits the preludes): `F800` in all four, the fill's red;
- case 18 (probe `100,150`, a point **on the black frame line** drawn by `+6`): `FFFF`/`FFDF`, white, in all
  four configurations. A frame that the same table drew one line above is gone, which is exactly what a per-case
  `v_clrwk` does and cannot be explained by the fill extent;
- every other probe: the cleared-screen word (`FFFF` at main/default/v16, `FFDF` at v32, V63's masked read).

The "seed pixel only" is therefore the one pixel a probe case can reach after the preludes; the fill may have
painted any extent before the next case's reset wiped it **[I]** (the table cannot tell).

### 1.3 EmuTOS and NewTOS both draw a region [C]

- EmuTOS `contourfill` pushes the seed span on its queue, then draws every popped span with `draw_rect_common`
  (`references/emutos/vdi/vdi_fill.c:998` queue seed, `:1057` the draw of each popped span). The
  seed span itself is drawn when it is popped, so the fill is a region fill, not a one-pixel write.
- NewTOS `Flood::run` seeds its queue with the seed span and draws each popped span through `draw_span`
  (`tos-api/src/vdi/contour.rs:359@run`, `:429@draw_span`, `:288@draw_span`), the same shape.

### 1.4 Where the region extent *is* measured

The tosdump probe `tosdump/probes/vdprobe.c` runs `v_contourfill` cases that draw and then `grab` a rectangle
**inside one case** (`tosdump/probes/vdprobe.c:508-518@k8` `K8A`/`K8B`: box, fill from `70,60`, `grab("K8A", 10, 10, 120, 100)`;
`K8E` at `:523`, the clip case `K8F` at `:532`). The raster table's screen readback (`tosdump/core/cases.h:582@SCREEN`, "THE
SCREEN") does read a rectangle after the measured call, but its prelude whitelist is 109, 121, 25, 114 and 129
(`tosdump/core/cases.h:609-613@Allowed`), so `v_contourfill` (103) cannot be a prelude there. The VDI table of
`DUMPVDI` therefore cannot show an extent with one probe per case; it can with one case that is itself the
`v_contourfill` and a later readback, which that table type has not.

### 1.5 Verdict

| Reading | Verdict |
|---|---|
| Clip / extent of the dumper's call | no: the call is `150,150` with the clip off (`vs_clip` off is part of the reset, `tosdump/core/runner.c:108@vs_clip`) |
| Case design | **yes**: preludes belong to the next case only and every case starts with `v_clrwk` (1.1, 1.2) |
| EmuTOS-conform behaviour | the fill is a region fill in EmuTOS (1.3); the "seed only" is not EmuTOS behaviour but the same table artefact at main, which is EmuTOS-equal for every probe the table has |
| NewTOS difference | **none shown**: main, default and v16 are identical in every probe (`fill-pixels.txt`), and the code has the same shape as EmuTOS (1.3) |

Nothing here is a NewTOS defect, so no new ticket is proposed for it. The follow-up the V63 run missed is a
measurement design, not a fix: to see the **region** at 32 planes through tosdump, use the `vdprobe` K8 cases
(boundary fill and fill-while-same with a grabbed rectangle) or a table type that allows `v_contourfill` as a
prelude before a readback. That is the tester's or the lead's ticket (a table design), not V64's.

## 2. The acceptance criteria of TASK/V64.md: testable with this spec?

Criteria quoted by position, not reworded (they are the owner's). Source: `TASK/V64.md` "Acceptance" and
"Added from the V63 spec gate".

| # | Criterion (by position) | Testable now? | What makes it so |
|---|---|---|---|
| A1 | host tests for the documented values of both formulas over the request range | **yes** | `q = (c*255+500)/1000` (P2) and `a = (q*1000+127)/255` (P5) with their fixed points and properties listed [calc]; request range: raw `i16`, stored unclamped, clamped to 0..1000 before `q` (P1). The `+127` against `+128` question is moot: identical for all 256 values [calc] |
| A2 | "... and the error input of `vs_color`/`vq_color` as today" | **yes** | index outside 0..255: `vs_color` silent no-op, `vq_color` `INTOUT[0] = -1`, `INTOUT[1..3] = 0` (P1, P4; EmuTOS `references/emutos/vdi/vdi_col.c:627-632` region and `:854-860`; code `tos-api/src/vdi/attributes.rs:492@vs_color`, `tos-api/src/vdi/inquiry.rs:93@vq_color`) |
| A3 | NewTOS 16 against NewTOS 32 on the same scenes, the 32-bit result projected to 16 bits, within one 5-bit step per gun | **yes, with a stated projection** | P9: projection = top 5 bits of each gun (`kernel/src/vdi_backend.rs:213@device_to_word`); for pens the bound 1 holds for every request 0..1000 and is reached [calc]. Not a bound on the **word**: bit 5 differs on a cleared screen by V63's design, so the tester compares guns. The interim `v_get_pixel` answer (F5) is that projection |
| A4 | at 16 planes nothing changes | **yes** | section 1 of the spec names the 16-plane functions that must not be edited; the standing tosdump/tosdiff bar at the default boot (ADR-038 point 13) is the check |
| A5 | `cargo xtask test` green; both board builds; warnings delta 0; doc-check 0 failures | not a spec matter | tooling checks |
| A6 | (added) at 32 planes a contour fill stops at a pen that differs from the fill pen only in the low 3 bits of one gun | **yes** | F1-F3 and the verified test pens: requests `(17,0,0)` and `(25,0,0)` are one colour at 16 planes and the pixels `0x00040000`/`0x00060000` at 32 [calc] |

Two limits on testability, both stated in the spec's Open points: (1) the VT52 console colour site has no slice
(`tos-api/src/vdi/escape.rs:234@word`), so a test of "all pen consumers agree at 32" cannot be complete until the
lead assigns it; (2) A3 depends on the answer shape of `v_get_pixel` staying the V63 interim in V64 (F5).

## Open points

- **Question for the lead:** the VT52 console colour site (`tos-api/src/vdi/escape.rs:234@word`) has no slice V63-V66; it must follow the pen table. My reading: the colour source in V64, the 16-bit row writer in V65.
- **Question for the lead:** confirm that `v_get_pixel`'s answer shape stays V63's interim in V64 (spec F5); the alternative is moving ADR-038 point 7 into V64.
- **Inferred:** section 1.2's "the fill may have painted any extent before the next reset" (the table cannot show it); the verdict rests on the harness code (1.1) and the frame-edge probe, not on an extent measurement.
- **Inferred:** no oracle run of `tmp-v63-fill.tsv` exists (it was run at four NewTOS configurations only), so "EmuTOS-conform" is read from the code shape and the harness, not measured.
- **Out-of-scope observation:** the V63 tester's `cargo xtask build --release` finding (`measurements/measurement-2026-11-18-v63-test/RUNINFO.TXT:77`) is a tooling matter, not read here.
