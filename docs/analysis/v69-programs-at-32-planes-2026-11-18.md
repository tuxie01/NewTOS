# V69: the programs at 32 planes (ADR-038 point 13 (iv)), what each does, where it breaks, what the owner can decide

Dated analysis, 2026-11-18, researcher, ticket `TASK/V69.md` (spec gate, before the owner decides the patches and before the
developer starts). Main at `43960455f`. Research only: no product code, no tosdump code, no patch file, no ADR edit.
Part A of the handover (tosdump at 32 planes) is **not** in this file: it is a live section of
`docs/TOS-REFERENCE-DUMPER.md` ("The 32-plane setting: tosdump at `vdiplanes=32`"), because that document is where the
dumper's record, table and guard rules live.

Labels: **read** = the source in front of me in this run (file:line); **measured** = a landed run recorded it (run named);
**[I]** = inferred from what was read; **[calc]** = arithmetic over a cited formula. Where a program's source is a vendored
upstream tree (`sdk/apps/teradesk/`, `sdk/apps/highwire/`, `sdk/zview/z-tools-04805b7/`) it is read only; NewTOS changes to
such a program are patch files (`sdk/overlays/<prog>/`, upstream unchanged; `docs/sdk/00-quickstart.md` section 5.3,
`docs/sdk/04-porting-anatomy.md`).

## 0. Result in one table

| Program | On the standard image | At 32 planes, by the source (read) | What a landed run measured | Defect also at 16 planes? | My recommendation (mine, not the contract) |
|---|---|---|---|---|---|
| TeraDesk | yes (`C:\Apps\TERADESK\`) | desktop, dialogs' save-under and colour icons work; the **Video options dialog** reads `npc[32]` out of a 17-entry table and overruns a 3-character resource string by 5 bytes | desktop with colour icons at 32 (V67, screendump): same picture as at 16; **dialogs and the Video options dialog not driven** | no (both sites are fine at <= 16) | patch the existing overlay `sdk/overlays/teradesk/video.c`: one line, `np` clamped to 16 |
| PRESENT | yes (`C:\Apps\PRESENT\`) | `capture_font` and `blit_frame` hardwire `fd_nplanes = 16`; both calls are refused at 32; the glyph buffer stays `malloc` garbage, so the font check fails or passes by chance | nothing at 32 (V65 note: "refused by design until V69") | no | edit `sdk/apps/present/present.c` (our own tracked source, no upstream tree): depth-aware capture and a convert-at-blit, about 50 lines [I] |
| HighWire | **no** (no build recipe, `Makefile` for m68k only) | `raster_32*` family writes through `ULONG *`, 8 bytes under LP64, into a 4-byte-per-pixel buffer; `cnvpal_true` reads the palette as `ULONG *`; and the program is not built for AArch64 at all | not run (no binary, V66 note) | the `ULONG *` defect: no (16 planes uses `UWORD *`); the missing port: yes | not a V69 program until a port ticket gives it a recipe; then nine functions get `uint32_t *dst` |
| zView | yes (`C:\ZVIEW\`) | takes the plain `raster_32` path, `uint32` pixels, 32-plane memory MFDBs of ADR-038 form, `vro_cpyfm` accepted; `1 << app.nplanes` is not reached | nothing at 32 | n/a | no patch; measure (section 3) |
| DOOM | no (own card, recipe `sdk/apps/build-doom.sh`) | EdDI route builds the 32-bit device form from `vq_scrninfo`; `dev_bpp = (planes+7)/8 = 4` | **measured** V66: default and `vdiplanes=32`, "EdDI check: 256 of 256 test colours exact", screendump viewed, correct colours | n/a | none; the interactive run stays open |
| HypView | no (recipe `sdk/apps/build-hypview.sh`) | `fd_nplanes = ext_workout[4]` memory forms, `vr_trnfm`/`vrt_cpyfm` into them; the 32-plane form is accepted | not run | no | none (read only) |
| QED, EmuDESK, `sdk/examples/*`, tosdump dumpers | yes | plane count read for a colour-config name or a pen pair, or not at all; direct-screen programs already see 32-bpp memory | QED/desktop on the boot screens (V63-V67) | no | none |

Everything in the "recommendation" column is the researcher's; the owner decides (section 6).

## 1. Method and the image survey (the gap the earlier gates left open)

Survey of `sdk/apps/`, `sdk/overlays/`, `sdk/examples/`, `programs/`, `tosdump/` with `/usr/bin/grep` for `vq_extnd`,
`vq_scrninfo`, `fd_nplanes`, `nplanes`, `Getrez`, `VsetMode`, `EdDI`, `vro_cpyfm`, `vrt_cpyfm`, `vr_trnfm`, `v_get_pixel`,
`Physbase`, `Logbase`. Files that matched outside the dumpers: `sdk/examples/{physbase,scrflip,uakid,crchild}.c`,
`programs/desktop/src/{gsx,global,lib,inf}.rs`, `programs/newtos-gem/src/vdi.rs`, `sdk/apps/present/present.c`,
`sdk/apps/doom/doomgeneric_gem.c`, `sdk/apps/doom/tests/c2p-host.c`. Programs in vendored trees were read in the trees.

**Image to recipe mapping** (the "no local source" gap, closed here): the standard image is built by
`tools/make-sd-image.sh`; its prebuilt programs are the `REQUIRED` list (`tools/make-sd-image.sh:68@REQUIRED`) and the
freestanding examples it builds itself with `build_example` (`tools/make-sd-image.sh:111@build_example`: hello, big,
physbase, scrflip, curdir, cmdline, dtachild, ptermch, ctrlcpar, fppexec, sysinfo, usbsnap, sysacc). The newest manifest of
a full standard card in the tree is `measurements/measurement-2026-11-07-desk70-test2/default.img.manifest`; its 43
executable names (`.NAP`, `.NCP`, `.NTP`, `.NAC`, `.NLD`) were each found in that list (`CMDLINE.NTP` is a copy of `CMDLINE.NCP`, `tools/make-sd-image.sh:299@CMDLINE`): ARGVTEST (`sdk/examples/argvtest.c` through
`sdk/libc/build-libc.sh`), AUTO\MXPROBE, MXPROBE, CLOCKTST, SETJMPT, LIBTEST, MATHTST, LDG\EX_LIB, LDGTEST, ZVIEW and its ten
codecs (`sdk/zview/build-zview.sh`), COREMARK\CORE and COREVAL, DUMPVDI, VDIDRAW, QED, TeraDesk's `DESKTOP.NAP`, PRESENT,
and the `build_example` programs; `HELLO.NCP` is assembled by `tools/mk-hello-ncp.py`, which names no depth call (0 hits for
`vq_extnd|planes|getrez|physbase|vro_cpyfm`; not read line by line). **Every executable on that manifest maps to a local
build recipe: the gap is empty by construction.** Caveats: the manifest is 11 days older than this run (the cards of
V62-V68 were `plain`, `apitest` and `doom` cards, not the standard image), so the mapping rests on the script plus that
manifest; `<image>.manifest` for a fresh standard image was not built here (docs only).

Of the standard-image programs only these read a depth: TeraDesk, PRESENT, zView (through WinDom), QED (name of a colour
file), EmuDESK (pen pair), `physbase.c`/`scrflip.c` (no query, 32-bpp direct writes: unchanged by ADR-038 point 9, they
already see the store), `uakid.c` (one `vq_extnd` call as an argument test). The dumpers are section A of
`docs/TOS-REFERENCE-DUMPER.md`.

## 2. Program by program

### 2.1 TeraDesk

**What the source does at 32 planes (read).**

- `xd_nplanes = work_out[4]` from `vq_extnd(handle, 1, ...)` (`sdk/apps/teradesk/library/xdialog/xdialog.c:2350-2351`): 32.
- The dialog save-under builds a memory MFDB with `fd_stand = 0`, `fd_nplanes = xd_nplanes` and a buffer of
  `fd_w * g_h * xd_nplanes / 8` bytes, `fd_w = (g_w + 16) & 0xFFF0`
  (`sdk/apps/teradesk/library/xdialog/xdutil.c:96-108`), copies the screen into and out of it with `vro_cpyfm`
  (`sdk/apps/teradesk/library/xdialog/xdialog.c:178-217`). At 32 planes the buffer is `fd_w * g_h * 4` bytes = `fd_wdwidth * 64 * g_h`:
  exactly the ADR-038 point 5 form (a row is `fd_wdwidth * 16` pixels of 4 bytes) [calc], and a 32-plane device MFDB is accepted
  (`docs/api/vdi/raster/_raster.md`, section "Which memory MFDBs are accepted"). The text cursor buffer is
  `fd_wdwidth * fd_h * xd_nplanes * 2` bytes (`sdk/apps/teradesk/library/xdialog/xddraw.c:1283-1285`), the same form [calc].
  Works unchanged **by the source**; not run.
- **The one break: the Video options dialog** (`voptions`, `sdk/apps/teradesk/video.c:435`), reachable from the menu
  (`sdk/apps/teradesk/main.c:1368-1375`), taken in the Falcon branch because NewTOS answers `_VDO` Falcon (`docs/analysis/teradesk-video-pi-mode-2026-11-18.md`
  sections 2.1 and 2.3). `np = xd_nplanes` (`sdk/apps/teradesk/video.c:454`) is 32 at 32 planes. Two sites use it unchecked:
  1. `npc[np]` at `sdk/apps/teradesk/video.c:592`, with `npc` a **17-entry** `static const char` table
     (`sdk/apps/teradesk/video.c:446`): index 32 reads 15 bytes past the table. A read of rodata, no fault expected [I]; the byte
     is ORed into `newmode` (`:591-592`). Then `np = minmax(npmin, np, npmax)` with `npmax = 16` (`:565`, `:625`) clamps to 16,
     `np != npp` rings the bell and loops (`:627-630`); from then on `npc[16]` is used. So the wrong value lives for one loop.
  2. The colour count text, **unconditionally at the top of every loop**: `ncc = 0x00000001L << np` (`:568`; `long` is 64 bits on
     LP64, so no undefined shift here), `ncc > 1024` once, `ncc /= 1024` (`:570-575`) = 4194304, `ltoa(ncc, s, 10)` and
     `strcat(s, "K")` (`:576-577`) into `vidoptions[VNCOL].ob_spec.free_string` (`:448`). That string is the **3-character** `"   "` of
     the resource (string table of `sdk/apps/teradesk/build/dist/desktop.rsc`, followed by `Colours:`; a gitignored build output). The
     text written is `"4194304K"`, 8 characters plus the NUL: **5 bytes past the end of the 4-byte string, into the neighbouring
     resource text** (what follows in the loaded resource is [I]; in the file's string table it is `Colours:`). At 16 planes the longest text
     is `"64K"`, which fits.
- Defect also at 16 planes today: **no**; `npc[16]` is in range and `"64K"` fits. The overflow needs `xd_nplanes` above 16.

**Measured.** V67 booted TeraDesk with its colour icons at 32 from `C:\Apps\TERADESK\` and compared the screendump with 16 and
main: same picture, top 5 bits per gun equal, largest step 4
(`measurements/measurement-2026-11-18-v67-test/RUNINFO.TXT`, item 9). "NOT driven: the SELECTED state of an icon". No dialog was
opened at 32.

**Options for the owner.**

| Option | What the program then shows at 32 | Cost, where | Risk |
|---|---|---|---|
| T1 patch `np` | everything as at 16; the Video options dialog opens, shows "64K" colours and describes the Falcon mode word `0x1C` it will return (16-bit truecolor, `docs/COMPATIBILITY.md` "The mode word and the plane count disagree") | one line in the existing overlay `sdk/overlays/teradesk/video.c` (a whole-file overlay, header lines 24-45 name its one existing change): `_WORD np = xd_nplanes > 16 ? 16 : xd_nplanes;` at `sdk/apps/teradesk/video.c:454` [I] | none beyond the overlay model; the dialog cannot show 32 planes because the Falcon word has no encoding for them |
| T2 give the dialog a 32-plane row | a "4G" colour row the dialog could not write back (no Falcon word) | larger patch and a new design question about what OK means | high, no source defines it |
| T3 stays a "16-plane program" | desktop, dialogs and icons work (by the source); **opening Options > Video at 32 corrupts the 5 bytes after the colour text once** and reads an out-of-range table byte | none | a small memory corruption in the resource, only in that dialog; DEVIATIONS row: trigger "TeraDesk Options > Video with `vdiplanes=32`" |

**Recommendation (mine): T1.** It is the smallest change, the program then behaves at 32 as at 16 in this dialog, and the dialog
is honest about the mode word.

### 2.2 PRESENT

PRESENT is **our own program** (`git ls-files sdk/apps/present`: `present.c`, `build-present.sh` and the sample slides; not a vendored upstream
tree), so the "patch file in `sdk/overlays/`, upstream unchanged" rule has no upstream to protect: the fix is an edit of the tracked
source. (The owner may still want the same name for the change; the ticket text says patch file.)

**What the source does at 32 planes (read).**

- `capture_font` (`sdk/apps/present/present.c:577@capture_font`): `malloc((capw + 15 & ~15) * caph * 2)` bytes (`:593`), draws the 224 glyph
  cells with `v_gtext`, copies the screen into `mem` with `fd_nplanes = 16` and `vro_cpyfm` (`:619-623`), reads `u16` words
  (`:581,631`). The copy is **refused at 32** (ADR-038 point 5, O5): the buffer keeps whatever `malloc` returned. `bgv = buf[0]`
  and `v != bgv` then run over uninitialised memory (`:625-636`); the check `inked == 0` / 'A' has ink in its middle row
  (`:638-642`) decides, and with a fresh page of zeros `inked == 0` -> `return 0` -> the alert "PRESENT: cannot read the system
  font" and exit (`sdk/apps/present/present.c:1206@capture_font`). With non-zero heap contents the outcome is [I] arbitrary garbage.
  So the likely result is the alert [I]; the program does not start a show.
- `blit_frame` (`:1087-1102`): `src.fd_nplanes = 16` and `vro_cpyfm(vh, S_ONLY, ...)` to the screen: refused, the screen is
  untouched.
- The frame buffer and every colour are RGB565: `u16 *fb` (`:376`), `RGB(r,g,b)` (`:378`), `blend565` (`:858`), `fit_picture` returns
  `u16 *` (`:869-872`), `memcpy(... * 2)` of picture rows (`:967`), `malloc(FBW * SH * 2)` (`:1195`), about 20 sites
  (`/usr/bin/grep -n "u16\|FBW\|fb\[" present.c`). The header comment says "NewTOS memory MFDBs are 16-bit RGB565 chunky"
  (`:47-52`), true at 16 planes, wrong at 32.
- Defect also at 16 planes today: **no**: at 16 the two MFDBs are exactly the device form.

**Measured.** Nothing at 32 (V65: "refused by design until V69", `measurements/measurement-2026-11-18-v65-test/RUNINFO.TXT:99`).
The "likely the font alert" reading is the researcher's [I]; a run would settle it.

**Options.**

| Option | What the program then shows at 32 | Cost, where | Risk |
|---|---|---|---|
| P1 guard only | an alert "needs `vdiplanes=16`" and a clean exit, instead of an arbitrary outcome | about 10 lines in `present.c`: `vq_extnd(1)` `intout[4]`, alert if not 16 [I] | none; the program stays a 16-plane program |
| P2 edge conversion (compose in RGB565 as now, convert at the two MFDB sites) | a correct slide show at 32; colours still 16-bit quality (the frame is RGB565) | about 50 lines [I]: planes query; `capture_font` allocates 4 bytes per pixel and compares `u32` at 32; `blit_frame` expands `fb` rows into a 32-bit buffer (`FBW * SH * 4` bytes = 1.2 MB at 640x480, or per band) and sets `fd_nplanes = 32` | the same binary serves both settings; the expansion is `0x00RRGGBB` from `RRRRRGGGGG?BBBBB` (the program's own 5-6-5 packing, `:378`) and must not use the 16-plane overlay bit |
| P3 true 32-bit frame | slides in 24-bit colour at 32 | about 20 sites, a runtime pixel type (the program must still run at 16 as the default) | largest; a second pixel path to test |
| P4 stays a 16-plane program, listed in DEVIATIONS | at 32: the font alert (likely) and exit; at 16 everything | none | a user with `vdiplanes=32` cannot use the standard image's slide show |

**Recommendation (mine): P2**, with P1's alert kept for any other count. It makes the program usable at both settings at a cost
of one session; P3 only pays if the slides are to carry 24-bit pictures.

### 2.3 HighWire

**State: not a buildable program of this repository.** `sdk/apps/highwire/` has the upstream sources and a `Makefile` for m68k
(`docs/analysis/lp64-tos-struct-survey-2026-10-28.md`, section 4: "Ohne Rezept: HighWire ... nur Makefile fuer m68k"); no
a build script for HighWire exists (`ls sdk/apps`: build scripts for coremark, doom, hypview, qed, teradesk, present only); no
card carries it. V66 recorded the same ("highwire has sources only", `measurements/measurement-2026-11-18-v66-test/RUNINFO.TXT:70`).
So "what HighWire shows at 32 planes" has no measurable answer today.

**By the source, at 32 planes (read).** It would pick `raster_32` from the table the same way zView does: `sdepth = out[2]`
= 32, `reverse = out[16] < out[48]` = `16 < 0` false, `z_trail = out[48] > 0` false
(`sdk/apps/highwire/src/raster.c:1713-1721,1797-1805`, values of `docs/api/vdi/eddi-vq_scrninfo-32.md`). Then:

- `raster_32`, `gscale_32`, `dither_32` and the `_32r`, `_32z` families (nine functions, `sdk/apps/highwire/src/raster.c:1392,1409,1423,1437,1455,1469,1484,1501,1515`)
  write `*(dst++)` through `ULONG *dst`. `ULONG` is `unsigned long`
  (`sdk/apps/highwire/src/hw-types.h:34`): **8 bytes under LP64**. A row of `DthWidth` pixels then takes `8 * DthWidth` bytes in a
  buffer sized for 4: the rows overlap and the writes run past the end of the image buffer (`mem_size = pg_size * 2 * n_planes`,
  `sdk/apps/highwire/src/image.c:904-917`): a heap overrun, the "memory error" of the ticket. At 16 planes `raster_16` writes
  `UWORD *dst` (`:1095-1103`) and at 24 planes `CHAR *dst` (`:1288-1298`): **the defect is only at 32 planes**.
- `cnvpal_true` reads the decoder's palette as `ULONG *` when `PalStep == 4` (`sdk/apps/highwire/src/raster.c:1660-1670`): with
  4-byte decoder entries that is a second LP64 mismatch, reached only on the true-colour depths.
- Behind both sits the port itself: the cookie jar walk reads `long` pairs (16 bytes under LP64) and does not find `EdDI` at all,
  so `hw_vq_scrninfo` would not ask the VDI (`docs/analysis/lp64-tos-struct-survey-2026-10-28.md`, rows C1-C5), a prior, bigger problem than
  the 32-plane one.

**Options.**

| Option | What it shows | Cost | Risk |
|---|---|---|---|
| H1 leave HighWire out of V69 | nothing to show; V69's "per program" criterion names zView, DOOM and TeraDesk/PRESENT only | none | the acceptance sentence of the ticket lists HighWire; the owner must strike it or accept "no binary" |
| H2 a port ticket first (recipe, LP64 survey rows C1-C5), then the nine-function patch | a browser that can show images at both depths | a port: not estimated here; the 32-plane part is `uint32_t *dst` in nine functions plus the palette read [I] | the port is its own project (network stack, fonts) |
| H3 list as "not built, not measurable" in DEVIATIONS | n/a | one row | none |

**Recommendation (mine): H1 plus H3.** Start no patch for a program that cannot be built; carry the nine-function note into the
port ticket if one is ever filed.

### 2.4 zView

By the source (read), `ZVIEW.NAP` of the standard image (built by `sdk/zview/build-zview.sh` from
`sdk/zview/z-tools-04805b7/` with the 14 overlay files of `docs/sdk/13-zview.md` section 3):

- needs the `EdDI` cookie, then `vq_scrninfo` (`sdk/zview/z-tools-04805b7/zview/zvdi/vdi.c:28-31`); `depth = (out[4] == 0x8000 ? 15 : out[2])` = 32,
  `reverse` false, `z_trail` false (`:37-41`) -> `case 32` -> `raster_32`, `gscale_32`, `dither_32`, `getPixel_32`, `resize_32`
  (`:164-186` of `vdi.c`) with `cnvpal_true` (`sdk/zview/overlay/zview/zvdi/color.c`, `cnvpal_true`: `0x00RRGGBB` from the palette).
- `raster_32` writes `uint32 *dst` (`sdk/zview/z-tools-04805b7/zview/zvdi/raster.c:1072-1085`); `uint32` is made 32 bits by the prelude
  (`docs/sdk/13-zview.md` section 2, `sdk/zview/include/zview-ints.h`): LP64-safe.
- picture MFDBs: `init_mfdb(&img, w, h, app.nplanes)` with `fd_stand = 0`, `fd_nplanes = app.nplanes`, `fd_wdwidth * 2 * nplanes * fd_h + 256`
  bytes (`sdk/zview/z-tools-04805b7/zview/mfdb.c:20-37`; `app.nplanes` = `work_out[4]` through WinDom,
  `sdk/windom/windom-1.21.3/src/appl_init.c:46`) = the ADR-038 form (row `fd_wdwidth * 64` bytes at 32) [calc]; the row advance is
  `LnSize = fd_wdwidth * n_planes` words (`sdk/zview/z-tools-04805b7/zview/pic_load.c:109`, `read_img` `:148-175` with a `uint16 *dst`) = `fd_wdwidth * 32` words = one
  32-plane row [calc]. They are put on the screen with `vro_cpyfm(S_ONLY, ...)` (`sdk/zview/z-tools-04805b7/zview/zaes.c:146-149` and the like): a 32-plane
  memory MFDB is accepted.
- the toolbar icons are `.GOD` pictures loaded through the same path and sliced in `app.nplanes << 4` words each
  (`sdk/zview/z-tools-04805b7/zview/catalog/catalog_icons.c:137-147`) = 16 x 16 x 4 bytes [calc].
- `1 << app.nplanes` (`sdk/zview/overlay/zview/zvdi/color.c`, `remap_color`, `max_color`) is only reached from `cnvpal_4_8` and `color_lookup`,
  which `vdi_init` uses for `depth == 4 || depth == 8` (`sdk/zview/z-tools-04805b7/zview/zvdi/vdi.c:33-36` and `:197-212`): **not reached at 32**.
- Not read: the browser's text path and the resample code (`sdk/zview/z-tools-04805b7/zview/resample.c:288` and `:299`, `init_mfdb(..., app.nplanes)`, same form).

Verdict by the source: **works unchanged**, with `INTOUT`-independent colours (the pixel is built from the table, not the 5-bit
word). **Not measured at 32 anywhere.** The earlier run at 16 planes was pixel-exact against the references at 5 bits per gun
(`docs/sdk/13-zview.md` section 5, `measurements/measurement-2026-10-06-zview-m4/RUNINFO.TXT`).

**Options.** Z1 no change (recommended by me; the program adapts through `vq_scrninfo`, the very reason for the table); Z2 nothing else
is on offer: if the measurement fails the fault is in a kernel row of the table or in the memory-MFDB rule, not in zView.

## 3. zView's path at 32 planes: what the tester must measure (acceptance, "measured and recorded")

By the source the path is the one in section 2.4. What is to be **recorded**, and how (proposal [I], from the method of the earlier
zView runs, `docs/sdk/13-zview.md` section 5):

1. A card with `C:\ZVIEW\` (ZVIEW.NAP, ZVIEW.RSC, ICONS, CODECS) and a sample image named on the command line, booted with
   `vdiplanes=32` and again with the default (the standard `#Z` launcher path of `docs/sdk/13-zview.md` section 5); a screendump after
   the window opened.
2. **Which `raster_*` function:** zView prints nothing. Decide it from two independent facts: (a) the wire: the kernel's call
   recorder (`svc-seq-trace`, `tools/svc-seq.py`) shows `vq_scrninfo` (VDI 102, `CONTRL[5] = 1`, `INTIN[0] = 2`) followed by `vro_cpyfm` (109)
   whose memory MFDB has `fd_nplanes = 32`; (b) the pixels: the window's image area against the decoded picture. At 32 with `raster_32` the
   picture is the 8-bit decode exactly (`0x00RRGGBB`); an R/B swap would mean `raster_32r`; 5-bit steps would mean a 16-bit route.
   Name the expected: sample pictures of `sdk/zview/samples/gen-samples.py` have known 8-bit values.
3. **Which result counts as "correct image":** pixel-exact against the decoded sample (no 5-bit quantisation at 32; the V67 tolerance of
   "top 5 bits equal, step <= 4" is for AES pen colours, not for a picture the program composes itself) [I]; the thumbnails of the
   browser icons visible; no EXC/PANIC.
4. Record the answer to "which path": `raster_32` (expected) / other, with the wire fact; and whether `vq_scrninfo` was called.

## 4. The other programs, shortly

- **DOOM** (`sdk/apps/doom/doomgeneric_gem.c`): `nplanes = INQ_TAB[4]` (`:514`); on the EdDI route it refuses a description with
  `si[2] != nplanes` (`:1109`), takes `dev_bpp = (planes + 7) / 8` = 4 and builds `fd_wdwidth * 16 * dev_bpp` byte rows (`:709-717`). **Measured**
  (V66, item 9): "EdDI: format 2, clut 2, 32 planes, bits r/g/b 8/8/8", "256 of 256 test colours exact", frames viewed, plausible colours,
  0 EXC/PANIC (`measurements/measurement-2026-11-18-v66-test/RUNINFO.TXT:65-71`). The `-noeddi` route (`vr_trnfm` + `vro_cpyfm`) at 32 and
  input are not measured.
- **HypView** (`sdk/overlays/hypview/hyp/prepare.c`): `mem_size` and the picture MFDB scale with `ext_workout[4]` (`:264-267,305`); above 8
  planes the 4-plane and 1-plane pictures go through `TrueColors`, `vrt_cpyfm` into the device MFDB (`:144-200,375`);
  `Mshrink(... plane_size * pic->mfdb.fd_nplanes)` (`:392`) = the 32-bit form [calc]. Works by the source; not run; not on the standard image.
- **QED** (`sdk/apps/qed/src/options.c:986` and `sdk/apps/qed/src/rsc.c:193`): `min(gl_planes, 8)` names a colour file; indifferent.
- **EmuDESK / newtos-gem** (`programs/desktop/src/gsx.rs:125@gsx_nplanes`, `global.rs:184@patcol_index`): plane count only picks a pen pair; indifferent.
- **`physbase.c`, `scrflip.c`**: write 32-bpp XRGB words straight into `Physbase` memory (`sdk/examples/physbase.c:19-25@XRGB8888`); X is
  unused at 32 and pens write 0 there, direct writes of `0xFF` are not read back: unchanged.
- **tosdump dumpers and probes**: section A of `docs/TOS-REFERENCE-DUMPER.md`.

## 5. The two stale sentences (ticket section "Added from the V68 test run"), corrected

- `docs/DEVIATIONS.md`, first row of "VDI at 32 planes": "host test to come in V68" now names `tos-api/tests/xbios_v68_planes_setting.rs` (TASK/V68.md).
- `docs/COMPATIBILITY.md`, the paragraph after the VDI-depth table: the pattern `tos-api/tests/vdi_*32*.rs` matched `vdi_v66_scrninfo32.rs` and, by accident,
  `vdi_v32_default_font.rs` (a V32 test about the 8x8/8x16 font, not a 32-plane test); it missed `vdi_v63_device_pixel.rs`, `vdi_v64_pens.rs`,
  `vdi_v65_memory_forms.rs`, `aes_v67_cicon32.rs`, `vt52_console_32.rs`, `vt52_console_32_default_palette.rs`, `xbios_v68_planes_setting.rs`. The
  sentence now lists the nine files and says V62-V68.

Not wrong, but incomplete (left as they are, ticket scope): the DEVIATIONS PRESENT row says "its copies are refused: no slide image"; section 2.2 above shows that the program
probably stops earlier, at the font check [I].

## 6. Decisions the owner is asked for

| # | Question | My reading |
|---|---|---|
| 1 | TeraDesk: patch the Video options dialog (T1), or list the dialog as a known deviation (T3)? | T1: one line in the existing overlay |
| 2 | PRESENT: P1 guard, P2 edge conversion (both settings), P3 24-bit frame, or P4 stays 16-plane? And: it is our own source, so a direct edit, not an overlay. | P2; direct edit |
| 3 | HighWire: strike it from V69's per-program list (no binary exists) and file a port ticket if wanted? | H1 + H3 |
| 4 | zView: accept "measure only, no patch" as the V69 deliverable, with the pass rule of section 3? | yes |
| 5 | Is the `vdi-v18*`/`vdi-a131-palette` table treatment of `docs/TOS-REFERENCE-DUMPER.md` ("the 16-plane tables stay oracle tables, 32-plane tables are new files") acceptable, including a new dump area for 32-plane raster records? | yes |

## 7. Acceptance criteria of TASK/V69.md, per criterion (not reworded)

| Criterion (the owner's text) | Testable? |
|---|---|
| "The 16-plane tables still compare against the pinned oracle unchanged; the 32-plane tables run and are labelled as NewTOS contract." | **Yes.** First half: the V66/V67 method: every table of the nine VDI/raster tables plus `aes-r1` run at default and `vdiplanes=16` against main, "X of Y cases differ" lines and tosdiff report texts identical, payload bytes identical but the header's build id (`measurements/measurement-2026-11-18-v67-test/RUNINFO.TXT` item 6). Second half: "run" = the dumper exits 0 and the report lists every case; "labelled" is testable once the developer fixes where the label lives (proposal in `docs/TOS-REFERENCE-DUMPER.md`: table header, dumper log line, area/file name): check by reading those three. |
| "Per program: it shows a correct image at 32 planes, or the owner has decided it stays a 16-plane program (listed in `docs/DEVIATIONS.md`)." | **Yes per program**, with the program's pass rule: TeraDesk (desktop and dialogs and Options > Video opened: a screendump and no string overrun), PRESENT (after the patch: the sample slides, screendump), zView (section 3), DOOM (V66 already measured, interactive part open). HighWire: not testable (no binary) unless the owner strikes it or files a port. "Listed in DEVIATIONS": testable by grep of the named trigger. Needs QEMU `virt` for any dialog interaction (no input on `raspi4b`, as V67 item 9). |
| "Measured and recorded: zView's path at 32 planes." | **Yes**, section 3; needs the zView card and the call recorder. |
| "`cargo xtask test` green; both board builds; warnings delta 0; doc-check of the edited docs 0 failures." | Mechanical. Note from V66-V68: doc-check in a fresh worktree reports "missing" for cites into gitignored extracted trees (`sdk/gemlib`, `sdk/zview/z-tools-...`) that `worktree-setup` does not link; that is not a defect of the cite. |

## 8. What can start without an owner decision, and what waits

- **Can start now (part A, tosdump, no program decision):** the parser, record, runner and probe changes and the 32-plane tables listed in the change-set table of
  `docs/TOS-REFERENCE-DUMPER.md` ("The 32-plane setting"). They need no program patch.
- **Waits for the owner:** every patch of part B (TeraDesk T1, PRESENT P1-P3, HighWire H1-H3), the DEVIATIONS rows of the programs that stay 16-plane, and the choice
  of where "NewTOS contract" labels live if the owner wants another place than the proposal.
- **Waits for a run, not a decision:** zView's path, TeraDesk's dialogs at 32, PRESENT's real failure mode, DOOM's `-noeddi` route.
- **Proposed DEVIATIONS rows** (not written; "nothing else in those files unless it is wrong"): (a) "TeraDesk, Options > Video, `vdiplanes=32`" if the owner chooses T3;
  (b) "PRESENT.NAP" row text per the owner's P-choice; (c) "HighWire: not built" if H3.

## Open points

STOP-blocks (needs deeper analysis / a run):

1. **PRESENT's real outcome at 32 is not read to a certainty:** `capture_font` runs over uninitialised heap (`sdk/apps/present/present.c:593@malloc`); "font alert" is likely when the heap is zero, arbitrary otherwise. A run (QEMU `virt` or `raspi4b` with `vdiplanes=32`) settles it.
2. **What follows the 3-character colour-count string in TeraDesk's loaded resource** (`vidoptions[VNCOL]`): the overflow of 5 bytes hits the next string in the file's string table (`Colours:`), whether the NewTOS resource loader keeps that order and layout is not read here (`tos-api/src/aes/resource.rs` loader not traced). The effect on screen needs the dialog opened at 32.
3. **HighWire's `cnvpal_true` palette entry width** (`sdk/apps/highwire/src/raster.c:1665-1670`, `sdk/apps/highwire/src/image_P.h:19`): the decoders that set `PalStep == 4` were not read; whether the `ULONG *` read is a real mismatch depends on them.
4. **zView, resample and browser text paths at 32** are not read (section 2.4, last bullet).

Inferred statements (every **[I]** above): TeraDesk `npc[32]` read raises no fault; PRESENT's patch sizes; the TeraDesk one-line patch is enough; HighWire patch size; zView's pass rule (pixel-exact, no 5-bit steps); the QEMU `virt`
requirement for dialog interaction (taken from V67 item 9).

Out-of-scope observations:

- `docs/sdk/14-present.md` and the header of `sdk/apps/present/present.c:47@RGB565` state "NewTOS memory MFDBs are 16-bit RGB565 chunky"; true at the default setting only. Fix with the PRESENT patch.
- `tos-api/src/cookie.rs:48@Doom` still says "Doom does not look yet" (stale, already noted in `docs/api/vdi/eddi-vq_scrninfo-32.md` section "Out-of-scope observations").
- The branch this work lies on is `worktree-agent-a4fd985da3a6683b8`, not `spec-v69`: the harness created it and the researcher's rules forbid a rename.
