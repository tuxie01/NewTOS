# TeraDesk's video code and a "Pi mode" description (XBIOS24 step 1)

> Dated analysis, 2026-11-18, researcher, ticket `TASK/XBIOS24.md` step 1. Nothing was run or measured; every
> TeraDesk cite is a line of the vendored upstream tree `sdk/apps/teradesk/` (gitignored, reached through
> `cargo xtask worktree-setup`). The overlay `sdk/overlays/teradesk/video.c` differs from upstream only in the ST
> overscan path (its header, lines 24-45): nothing below depends on that difference except where said.
> "Read" = in the source in front of me; "inferred" = reasoned from it. No option is decided here.

## 1. Where TeraDesk's video code is

All of it is in `video.c`, three entry points:

- `get_set_video(0|1|2)` (`sdk/apps/teradesk/video.c:117`): 0 reads the state, 1 re-lays the desktop out, 2 asks for a
  resolution change. Callers: defaults (`sdk/apps/teradesk/main.c:840`, `:902`), options load (`sdk/apps/teradesk/main.c:954`), defaults reset
  (`sdk/apps/teradesk/main.c:1020`), shutdown path with a pending change (`sdk/apps/teradesk/main.c:2025`).
- `voptions()` (`sdk/apps/teradesk/video.c:435`): the "Video options" dialog; called from the menu item `MVOPTS`
  (`sdk/apps/teradesk/main.c:1368-1375`), a non-zero result sets `chrez` and ends TeraDesk (`sdk/apps/teradesk/main.c:1372-1375`), whereupon
  `get_set_video(2)` runs (`sdk/apps/teradesk/main.c:2015-2025`).
- Helpers `vd_setstring`, `vd_enable_rez` (`sdk/apps/teradesk/video.c:408-426`).

What the dialog can show is fixed by its resource objects: six resolution radio buttons `VSTLOW..VTTHIGH`
(`sdk/apps/teradesk/desktop.h:455-460`), "Save palette" `SVCOLORS`, `VBLITTER` (ST blitter, or "Doubleline" on Falcon,
`sdk/apps/teradesk/video.c:519`), `VOVERSCN`, the colour count `VNCOL` with up/down `VNCOLDN/VNCOLUP`, Cancel and OK
(`sdk/apps/teradesk/desktop.h:461-469`). **No object shows a pixel width/height.** The Falcon labels of the two buttons are the free
strings "VGA-40-column" and "VGA-80-column", `VBLITTER` becomes "Doubleline" (`sdk/apps/teradesk/rsc/en/teradesk.pot:1911-1921`,
set at `sdk/apps/teradesk/video.c:517-519`). The alert on OK is `ARESCH`: "A change of video mode will terminate all running
applications." `[OK|Cancel]` (`sdk/apps/teradesk/rsc/en/teradesk.pot:2139-2141`, `sdk/apps/teradesk/desktop.h:922`).

## 2. The branches

The branch key is the **upper word of the `_VDO` cookie**, `vdohi` (`sdk/apps/teradesk/video.c:158-160`), a `short`. Values:
`ST 0, STe 1, TT 2, Falcon 3, Milan 4, Aranym 5, other 6` (`sdk/apps/teradesk/video.c:66-72`).

### 2.1 Read side, `get_set_video(0)` (`sdk/apps/teradesk/video.c:142-258`)

Always, first: `xd_screensize()` (`sdk/apps/teradesk/video.c:146`) = `vq_extnd(handle, 0, ...)`, width/height `work_out[0..1]+1`
(`sdk/apps/teradesk/library/xdialog/xdialog.c:379-389`); `currez = Getrez()` (`sdk/apps/teradesk/video.c:150`). `xd_nplanes` is `vq_extnd(handle, 1,
..)` `work_out[4]`, read once at VDI open (`sdk/apps/teradesk/library/xdialog/xdialog.c:2350-2351`).

| `vdohi` | Branch | Reads | Cite |
|---------|--------|-------|------|
| 3, 4, 5 | Falcon / Milan / Aranym (one branch, `fal_mil = TRUE`) | `falmode = VsetMode(-1)`, `fmtype = VgetMonitor()`, `bltstat = 0`; **overrides `currez`** from the mode word: STMODE bit clear -> `TT_LOWRES` (80COL clear) or `TT_MEDRES`; STMODE set -> `ST_LOWRES` / `ST_HIGHRES` (if `xd_nplanes == 1`) / `ST_MEDRES` | `sdk/apps/teradesk/video.c:172-201` |
| 0, 1 | ST / STe | OVER/Lace cookie probe (only `#if _OVSCAN`), `Blitmode(-1)` if `tos_version >= 0x104`; `currez` stays `Getrez()` | `sdk/apps/teradesk/video.c:202-258` |
| 2 | TT | nothing (no branch in the read side); `currez` stays `Getrez()` | `sdk/apps/teradesk/video.c:142-259` (absence) |
| 6 and any other | none | nothing; `currez` stays `Getrez()` | same |

Cookie absent: the comment says `vdo` "will be 0xffffffff without cookie" (`sdk/apps/teradesk/video.c:156`), but the code does not
do that. `vdo` is a static `long` (`sdk/apps/teradesk/video.c:60`, zero-initialised) and `find_cookie` returns FALSE **without
touching `*value`** when the jar or the tag is missing (`sdk/apps/teradesk/library/utility/cookie.c:47-48` and `:58-63`, value
assigned only at `:65-69`). So an absent `_VDO` reads as `vdohi = 0` = the **ST branch** (read; this is what the
XBIOS22 run saw on `main`: ST/TT buttons, `Blitmode(-1)`, no `VsetMode`). Whether the build is `_MINT_` with
`have_ssystem` (`sdk/apps/teradesk/library/utility/cookie.c:44-48`) is not read here; both paths leave `*value` untouched.

### 2.2 Dialog setup, `voptions()` (`sdk/apps/teradesk/video.c:473-546`)

`np = xd_nplanes` (`sdk/apps/teradesk/video.c:454`), `newmode = falmode` (`sdk/apps/teradesk/video.c:459`), `get_set_video(0)`, radio set to
`rmap[currez]` (`sdk/apps/teradesk/video.c:477`, map at `:108`). Then a switch on `vdohi`:

- `ST_VIDEO`: overscan box if `over != -1L`; falls into `STE_VIDEO`: only ST-Low..ST-Med enabled (or ST-High if
  `currez == ST_HIGHRES`), blitter box if present (`sdk/apps/teradesk/video.c:483-507`).
- `TT_VIDEO`: if `currez == TT_HIGHRES` only TT-High, else ST-Low..TT-Med enabled (`sdk/apps/teradesk/video.c:508-513`).
- `FAL/MIL/ARA`: Falcon labels; if `fmtype != MONO_MON` (0): colour editing allowed (`editcol`), ST-Low..TT-Med
  enabled, ST-High disabled, "Doubleline" enabled, Overscan enabled unless `fmtype == VGA_MON`; for MONO or VGA
  ST-High enabled again; Overscan/Doubleline ticked from `newmode` bits `0x40`/`0x100` (`sdk/apps/teradesk/video.c:514-540`).
- default (`vdohi` 6 or other): all six radio buttons enabled, nothing else (`sdk/apps/teradesk/video.c:541-545`).

No branch reads the mode a graphics card or an extended mode would have: there is none besides the Falcon mode
word (`VM_*` flags, `sdk/apps/teradesk/video.c:81-88`) and the `Getrez` codes (`sdk/apps/teradesk/video.c:99-104`) (read: grep of `*.c` for
`VsetMode|VgetMonitor|Vsetscreen|Setscreen|Getrez|_VDO` finds only `video.c`; `Setscreen` appears once, in the ST
overscan path `sdk/apps/teradesk/video.c:319`).

### 2.3 The OK path (`sdk/apps/teradesk/video.c:581-697`)

After each `xd_form_do` (`sdk/apps/teradesk/video.c:581`):

1. `newrez = rimap[selected radio]` (`:585`).
2. Falcon branch only (`if (fal_mil)`, `:589`): `newmode` is **rebuilt** from the radio and `npc[np]`:
   clears STMODE|80COL|NPLANES, ORs in `npc[np]` (`np` -> depth code, `sdk/apps/teradesk/video.c:446`: 16 -> 4, 8 -> 3, ...), STMODE
   if `newrez <= ST_HIGHRES`, 80COL unless Low (`:591-598`). Then per-resolution colour limits (`:600-623`): ST
   modes pin `np`; **TT-Low/TT-Med on a VGA monitor: `npmax = 8` for 80 columns, `npmin = 2` for 40**
   (`:614-622`). `np = minmax(npmin, np, npmax)` (`:625`, signature `sdk/apps/teradesk/library/utility/minmax.c:58`); **if `np`
   changed: `bell()` and `goto next`, i.e. the OK press is consumed** (`:625-630`). Then Doubleline/Overscan bits
   are read back from the boxes (`:632-633`).
3. ST/STe: blitter/overscan options read back (`:634-644`).
4. `VIDOK` (`:656`): `qquit`; **alert iff `(newrez != currez && newrez != -1) || newmode != falmode`**
   (`:665`); on `[OK]`: `currez = newrez`, `falmode = newmode`, `rcode = 1` (`:667-672`).
5. `get_set_video(1)` after OK (`:694-697`) re-lays out windows and regenerates the desktop (`:331-353`).
6. `rcode == 1` -> `get_set_video(2)` (`sdk/apps/teradesk/main.c:2017-2025`): `shel_write(SHW_RESCHNG, wisgr, wiscr, ...)` if
   `aes_version >= 0x340` with `wisgr = falmode, wiscr = 1` on Falcon, else `wisgr = currez + 2, wiscr = 0`
   (`sdk/apps/teradesk/video.c:360-379`); then `wait_to_quit()` (`:383`; waits 2 s for `AP_TERM`/`RESCH_COMPLETED`/`SHUT_COMPLETED`,
   `sdk/apps/teradesk/main.c:1664-1700`). There is no fallback that sets the mode directly (`sdk/apps/teradesk/video.c:386-397` is `#if 0`).
   NewTOS records `SHW_RESCHNG` and restarts the AES, never switches (`tos-api/src/aes/shell.rs:484@SHW_RESCHNG`,
   `docs/COMPATIBILITY.md` XBIOS Falcon video entry).

The alert is therefore not tied to the Falcon branch: it fires in **every** branch whenever the selected radio
differs from `currez` or, on Falcon, the rebuilt word differs from `falmode`.

## 3. The Colours-256 observation, explained from the source

Observed (`measurements/measurement-2026-11-18-xbios22-test/RUNINFO.TXT` "TERADESK GATE" b, c): on the XBIOS22
commit the first Return left *Colours* at 256 and the alert came after the second; on `main` with the arch06
injection the alert came after the first.

**Commit (`VsetMode` = `0x1C`, `VgetMonitor` = 2, `xd_nplanes` = 16 — the 64K shown, `sdk/apps/teradesk/video.c:454,567-577`):**

1. Open: `falmode = 0x1C`: STMODE clear, 80COL set -> `currez = TT_MEDRES` (`sdk/apps/teradesk/video.c:179-186`).
2. First Return (OK is the default exit, measured): `newmode = (0x1C & ~0x8F) | npc[16]` = `0x14`, `newrez =
   TT_MEDRES` (not `<= ST_HIGHRES`), 80COL set -> `0x1C` (`:591-598`). `default:` arm with `fmtype == VGA_MON` and
   80COL set -> `npmax = 8` (`:614-618`). `np = minmax(1, 16, 8) = 8 != npp = 16` -> `bell()`, `goto next`
   (`:625-629`). `editcol` is TRUE (`fmtype != 0`, `:521-523`), so `np` stays 8 (`:688-689`) and the loop redraws
   `ncc = 1 << 8` = **256** (`:567-579`). The dialog stays open; no alert.
3. Second Return: `np = 8` is inside the limits, so no clamp; `newmode = 0x10 | npc[8] = 0x13`, `|= 80COL` ->
   `0x1B`, `!= falmode (0x1C)` -> alert (`:665-667`). Matches the observation exactly (read + computed).

**`main` + injected `_VDO` (`VsetMode` answers `0x58`, `VgetMonitor` answers `0x59` = function numbers, measured
a):** `fmtype = 0x59` is neither MONO nor VGA, so the `fmtype == VGA_MON` clamp (`:615`) is skipped; `np` stays
16; `newmode = (0x58 & ~0x8F) | 4 = 0x54`, + 80COL = `0x5C != 0x58` -> alert on the first Return
(computed from `:591-598,665`). Matches.

Consequence (inferred from the above): on the commit a user who presses OK twice has chosen "8 bit, VGA-80" and
`falmode = 0x1B` goes into `shel_write(SHW_RESCHNG, 0x1B, 1)` (`sdk/apps/teradesk/video.c:670,362-379`); NewTOS restarts the AES
and TeraDesk reads `0x1C` again (RUNINFO c). The 256 is TeraDesk's own VGA colour limit for 80 columns, not a
NewTOS value; it can only disappear if the word makes TeraDesk's 80-column VGA limit not apply or the dialog
does not reach `:625`.

Side observation (inferred, not measured): `npc[]` has 17 entries (`sdk/apps/teradesk/video.c:446`) and `np = xd_nplanes` indexes it
(`:592`); a VDI that reported 32 planes would index out of bounds. NewTOS's VDI device depth is 16 (the spec
`docs/api/xbios/58-vsetmode.md` cites ADR-024: 16-plane RGB565), so this does not happen today.

## 4. Options for a "Pi mode"

Goal as the ticket words it: the dialog shows the screen's real resolution and depth; OK on the unchanged dialog
raises no mode-change alert and restarts nothing. Facts shared by all options: the dialog has **no object that
can show width x height** (section 1); depth comes from `vq_extnd` (`xd_nplanes`, 16 at present) and is shown by
`VNCOL` as "64K"; the Pi's HDMI mode is fixed by `config.txt` and there is no runtime switch (ADR-017 Revision 2,
`docs/MEMORY.md` ADR-017). Whether the handover's "32 bpp" (framebuffer) versus the VDI's 16 planes (what the
dialog would show) is the intended "depth" is for the owner.

| Option | What it is | Dialog shows | OK on the unchanged dialog | Cost | Other programs |
|--------|-----------|--------------|----------------------------|------|----------------|
| **0. Status quo** | `_VDO` `0x00030000`, `VsetMode` `0x1C`, `VgetMonitor` 2 | VGA-80 selected, ST-Low/Med + VGA-40 selectable, Overscan greyed, Doubleline on, Colours 64K (RUNINFO b); no resolution | first Return clamps Colours to 256, second raises the alert, OK restarts the AES (section 3) | none | none seen (RUNINFO d) |
| **1. Patch only** (TeraDesk overlay `sdk/overlays/teradesk/video.c`; kernel unchanged) | a NewTOS branch in `get_set_video`/`voptions`, gated by the existing helper `newtos_mch_nohard()` (`sdk/overlays/teradesk/nohard.h:32@newtos_mch_nohard`, read: true when `_MCH == 0xFFFFFFFF`) | free: e.g. write the real size into a button string via the `vd_setstring` mechanism (`sdk/apps/teradesk/video.c:408-411`, needs a buffer; whether a string like "1920x1080" fits the button width must be **measured/read from the RSC**), disable the radios and colour arrows, keep `VNCOL` from `xd_nplanes` | free: skip the `newmode`/radio comparison so `:665` is never true; no alert, no `shel_write` | overlay file only (a new patch site in an already overlaid file, `video.c`); possibly a dialog-resource edit if the label needs a new object (RSC is converted by `mkrsc.py`, `sdk/apps/build-teradesk.sh:215@MKRSC`) | none: kernel and cookies unchanged |
| **2. Kernel: `_VDO` = TT** (`0x00020000`) | read side has no TT branch (`sdk/apps/teradesk/video.c:142-259`), `Getrez()` drives the radio (`:477`) | ST-Low..TT-Med enabled, radio = `rmap[Getrez()]` (TT-Medium today; ST-High if XBIOS23's `Getrez` = 2 on the branch, `rmap[2] = VSTHIGH` `:108`, then only ST-High enabled `:495-496`); Colours = `VNCOL` 64K, not editable (`editcol` FALSE, `:688-689`, no clamp since `fal_mil` FALSE `:589`); no resolution | no alert unless the user moves the radio (`newrez == currez`, `falmode` stays 0 and `newmode == falmode`, `:665`) | kernel: cookie value (reverses the 2026-11-18 `_VDO` amendment, ADR-012); `VsetMode`/`VgetMonitor` no longer needed by TeraDesk | claims TT hardware to every `_VDO` reader (the XBIOS22 amendment chose Falcon on purpose); `Falcon` calls stay answered. Survey: no other program in `sdk/apps`, `sdk/overlays`, `sdk/examples`, `programs` references `_VDO`, `VsetMode`, `VgetMonitor` (grep, see Open points for the manifest gap). Still offers six modes it cannot honour |
| **3. Kernel: `_VDO` outside 0-5** (e.g. `0x00060000` "other") | default arm (`:541-545`) | all six radios enabled, radio from `Getrez()`, nothing else; no resolution | no alert unless radio moved | kernel: cookie value; `0x6` is a TeraDesk constant (`sdk/apps/teradesk/video.c:72`), **not found in EmuTOS's `cookie.h` here (not read)**, so no EmuTOS basis | same as 2; also defeats the Falcon gate for programs that need `_VDO` to call `VsetMode`/`VgetSize` |
| **4. Kernel: Falcon with other answers** (`VgetMonitor` 0 mono, or another mode word) | stays in the Falcon branch | mono: all radios disabled except ST-High (`:532-533`), `editcol` FALSE (no colour clamp); radio is `ST_MEDRES` while `xd_nplanes != 1` (`:191-200`) -> a disabled button selected (inferred); or other words: the clamp `:615-618` applies to every VGA/80-column word | depends on word; a mono word leaves `newmode == falmode` only if the rebuild at `:591-598` reproduces it, which it does not for depth 16 (inferred) | kernel: `VgetMonitor`/`VsetMode` answers | `VgetMonitor` = 0 claims a mono monitor to every Falcon-video reader and contradicts `59-vgetmonitor.md` decision 1 and XBIOS23 coherence; not recommended as a way to reach the goal, listed for completeness |
| **5. Patch: remove the menu entry / dialog** | disable `MVOPTS` | no dialog | n/a | overlay | none; does **not** meet "the dialog shows the real resolution and depth" |

Notes the owner needs for deciding (inferred unless cited):

- Only option 1 can show a resolution at all, because no TeraDesk branch has a place for it; options 2-4 only
  change which radio is selected and whether an alert appears.
- Options 2-3 trade the `_VDO` amendment of 2026-11-18 (ADR-012, `docs/MEMORY.md`) for dialog behaviour; the
  amendment says a wrong dialog state or alert "sends the value back to the owner". Option 1 keeps it.
- Option 1 is a fourth patch site in `sdk/overlays/teradesk/` next to SDK07 (`video.c` header lines 24-45); the
  upstream tree stays unchanged (project rule).
- After XBIOS23 (`Getrez` = 2, branch `xbios23`, not read by me): the Falcon branch is unaffected (it overrides
  `currez`, `sdk/apps/teradesk/video.c:172-201`); branches 2 (TT), 3 (other) and the ST branch take `currez = 2` = ST-High.

## Open points

- **To be measured** (I measure nothing): that a "NN x MM" string fits a dialog button (object widths are in the
  binary RSC, not read); the dialog and OK behaviour of options 2-4 (computed from the source only); the XBIOS23
  interaction on the TT/other branches; the TeraDesk build's `_MINT_`/`have_ssystem` setting
  (`sdk/apps/teradesk/library/utility/cookie.c:44-48`).
- Inferred: the section 3 numbers are computed from the cited lines, consistent with the two recorded
  observations but not re-run; option 4's mono-monitor dialog state; the `npc[]` bounds side note.
- Inferred: "depth" intended by the owner (VDI 16 planes shown as 64K, versus 32 bpp framebuffer); not decided here.
- Out-of-scope observations: the stale comment `sdk/apps/teradesk/video.c:156` (cookie absent is read as ST, not -1); only
  `sdk/apps/` sources were searched for other users of the video data (QED, zView, HighWire, hypview have no
  `VsetMode`/`VgetMonitor`/`_VDO` hit; `Getrez` appears in `sdk/apps/highwire/src/Nice_VDI.c` and
  `sdk/overlays/xgdos/xos_newtos.c`, not read); the card-manifest-to-recipe mapping for the "no local source" gap
  was not done (zView has no source tree under `sdk/apps/`, so its video use is **unknown**; RUNINFO d saw no effect).
