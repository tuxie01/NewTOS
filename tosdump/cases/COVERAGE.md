# VDI case coverage ledger

What `vdi-r1.tsv` covers, what is still to come, and what is excluded with the
reason. The point is that expanding the table is mechanical: pick the next
`later` row, write the case, move it to `round 1`.

Opcodes are the complete EmuTOS jump tables (`vdi/vdi_main.c:32-111`); names
and input arities come from each handler and from `docs/api/vdi/`.

**One table per round.** A case id is the physical line number and the file's
CRC binds every dump to it, so *any* edit -- an appended case included --
invalidates the dumps already taken. Round 1 is frozen as `vdi-r1.tsv`
(CRC `FFF2281E`, measured on EmuTOS 1.4 and TOS 3.06); round 2 gets its own
`vdi-r2.tsv`, so earlier dumps stay valid and comparable forever instead of
having to be re-measured on every machine each time the table grows.

## Status

| # | Name | Status | Note |
|---|------|--------|------|
| 1 | `v_opnwk` | excluded | opens the **physical** workstation; replaces the device state the whole run rests on |
| 2 | `v_clswk` | excluded | closes it; every case after it would measure nothing |
| 3 | `v_clrwk` | later | the reset already issues it before every case; measure it as a case too |
| 4 | `v_updwk` | later | `v_nop` in EmuTOS — a no-op is observable behaviour and worth measuring |
| 5 | `v_escape` | later | 19 sub-ops, see below |
| 6 | `v_pline` | **round 1** | two cases |
| 7 | `v_pmarker` | later | D-VDI-9: forces `clip` on and never restores it — a real finding to confirm on the TT |
| 8 | `v_gtext` | later | needs a character-code list in `intin` |
| 9 | `v_fillarea` | **round 1** | |
| 10 | `v_cellarray` | later | `v_nop` |
| 11 | `v_gdp` | later | sub-ops 1-10 (bar/arc/pieslice/circle/ellipse/ellarc/ellpie/rbox/rfbox/justified) |
| 12 | `vst_height` | later | `ptsin`; sets `flip_y`, so the `d0` field matters here (D-080) |
| 13 | `vst_rotation` | **round 1** | two cases, valid and not a multiple of 900 |
| 14 | `vs_color` | excluded | device-global palette; the reset does not restore it (`core/runner.c`) |
| 15 | `vsl_type` | **round 1** | valid and out of range |
| 16 | `vsl_width` | later | `ptsin` |
| 17 | `vsl_color` | **round 1** | valid and out of range |
| 18 | `vsm_type` | later | |
| 19 | `vsm_height` | later | `ptsin`; sets `flip_y` (D-080) |
| 20 | `vsm_color` | later | |
| 21 | `vst_font` | later | sets `flip_y` as a leaked side effect (D-080) |
| 22 | `vst_color` | later | |
| 23 | `vsf_interior` | **round 1** | valid and out of range |
| 24 | `vsf_style` | **round 1** | D-VDI-8: setter is 0-based, `init_wk` is 1-based |
| 25 | `vsf_color` | later | |
| 26 | `vq_color` | later | `intin` = index, requested/realised flag |
| 27 | `vq_cellarray` | later | `v_nop` |
| 28 | `v_locator` | excluded | request-mode input; blocks on the operator |
| 29 | `v_valuator` | later | `v_nop`, no analogue device on the ST |
| 30 | `v_choice` | later | D-VDI-10: a DRI dummy that always returns 1 at once, so deterministic |
| 31 | `v_string` | excluded | request-mode input; blocks on the operator |
| 32 | `vswr_mode` | **round 1** | valid and out of range |
| 33 | `vsin_mode` | later | changes input mode; the reset does not restore it |
| 34 | (does not exist) | later | `v_nop`; measuring an undefined opcode is worth one case |
| 35 | `vql_attributes` | **round 1** | |
| 36 | `vqm_attributes` | **round 1** | |
| 37 | `vqf_attributes` | **round 1** | |
| 38 | `vqt_attributes` | **round 1** | |
| 39 | `vst_alignment` | **round 1** | |
| 100 | `v_opnvwk` | excluded | allocates a handle; the run owns one already, and handles legitimately differ (spec section 5) |
| 101 | `v_clsvwk` | excluded | closes the handle every later case needs |
| 102 | `vq_extnd` | **round 1** | both `intin[0] = 0` and `= 1` |
| 103 | `v_contourfill` | later | seed fill; needs a known screen state, so it must follow a drawing case |
| 104 | `vsf_perimeter` | later | |
| 105 | `v_get_pixel` | **round 1** | |
| 106 | `vst_effects` | **round 1** | |
| 107 | `vst_point` | later | sets `flip_y` (D-080) |
| 108 | `vsl_ends` | later | two `intin` |
| 109 | `vro_cpyfm` | later | needs MFDB pointers in `contrl[7..10]`, which differ per machine — needs a comparison rule for those words first |
| 110 | `vr_trnfm` | later | same MFDB problem |
| 111 | `vsc_form` | later | D-VDI-14: transforms a user-defined mouse cursor, not a raster expand |
| 112 | `vsf_udpat` | excluded | the reset does not restore `ud_patrn` (`core/runner.c`); needs the table to restore it itself |
| 113 | `vsl_udsty` | later | the reset already sets it; measure it as a case too |
| 114 | `vr_recfl` | later | |
| 115 | `vqin_mode` | **round 1** | D-VDI-11: returns `mode - 1`, not 1/2 |
| 116 | `vqt_extent` | later | needs a string in `intin` |
| 117 | `vqt_width` | later | |
| 118 | `vex_timv` | excluded | installs a vector; a dangling handler after exit corrupts the machine |
| 119 | `vst_load_fonts` | excluded | changes global font state the reset cannot restore; measured by the probe `tosdump/probes/vdprobe.c` T13, TX* (TASK/V33) |
| 120 | `vst_unload_fonts` | excluded | same; probe T13, TX* |
| 121 | `vrt_cpyfm` | later | same MFDB problem as 109 |
| 122 | `v_show_c` | later | changes cursor visibility; pairs with 123 |
| 123 | `v_hide_c` | later | |
| 124 | `vq_mouse` | excluded | live pointer position; differs between two runs on one machine |
| 125 | `vex_butv` | excluded | installs a vector |
| 126 | `vex_motv` | excluded | installs a vector |
| 127 | `vex_curv` | excluded | installs a vector |
| 128 | `vq_key_s` | excluded | live shift state; D-VDI-13 masks it to the low 4 bits |
| 129 | `vs_clip` | later | the reset already sets it; measure it as a case too |
| 130 | `vqt_name` | later | 33 `intout` |
| 131 | `vqt_fontinfo` | later | |
| 132 | `vqt_justified` | later | `v_nop`, `CONF_WITH_EXTENDED_MOUSE`-gated |
| 133 | `vs_grayoverride` | later | `v_nop`, gated |
| 134 | `vex_wheelv` | excluded | installs a vector; gated |

## `v_escape` (opcode 5) sub-ops

From `docs/api/vdi/_escape.md`. All `later` unless marked.

| sub | Name | Note |
|---|------|------|
| 1 | `vq_chcells` | inquiry, safe |
| 2 | `v_exit_cur` | **excluded** — switches the screen to graphics mode |
| 3 | `v_enter_cur` | **excluded** — switches the screen to alpha mode |
| 4-8 | `v_curup`/`down`/`right`/`left`/`home` | move the alpha cursor; safe but change console state |
| 9-10 | `v_eeos`, `v_eeol` | erase; safe after a `v_clrwk` |
| 11 | `vs_curaddress` | |
| 12 | `v_curtext` | needs a string |
| 13-14 | `v_rvon`, `v_rvoff` | |
| 15 | `vq_curaddress` | inquiry, safe |
| 16 | `vq_tabstatus` | inquiry; D-VDI-2 corrected the return to 1 |
| 17 | `v_hardcopy` | **excluded** — triggers `Scrdmp()`, a real printer dump |
| 18 | `v_dspcur` | |
| 19 | `v_rmcur` | |

## Why some things are excluded rather than deferred

Three separate reasons, and they are not interchangeable:

- **Lifecycle** (1, 2, 100, 101): the call would destroy or replace the
  workstation the rest of the run measures through.
- **Not reproducible** (124, 128): the result depends on where the mouse is or
  which keys are held, so two runs on one machine would already differ. A
  reference dump may do anything except that.
- **Leaves state behind** (14, 112, 118, 119, 120, 125-127, 134): the reset in
  `core/runner.c` cannot undo it, so the case would silently change the meaning
  of every case after it. Any of these becomes measurable the moment the reset
  is extended to cover it, or the case restores it itself.
