# Pi queue triage for a merged Pi test program (PITEST02)

Date 2026-11-12. Ticket: `TASK/PITEST02.md`. Role: researcher. Nothing built, no ticket edited, no
`Missing` field rewritten.

## 1. Method and sources

- Population: every `TASK/*.md` whose line 3 starts `Status: IMPLEMENTED` at main HEAD `98c1cdd3e`: 69
  tickets. 44 of them have a `Missing` field that names a real-Pi, hardware or Pi-visual run (section 3);
  the other 25 are listed in section 5.
- "What must be observed" is taken from the ticket or its receipt. Where the ticket only says "Pi-Sichtabnahme
  offen" and the per-point Pi checklist `docs/api/desktop/_hw-test-2026-09-17.md` (not named in the handover,
  but the file every one of those tickets points to: A106 cites "HW-Checkliste Punkt 33", R05 "Punkt 31",
  DESK13 "HW-Punkt 42") states the expected result, the row cites that file's section. Where neither states
  a Pi-specific observation the cell says "not stated".
- `docs/api/desktop/_hw-test-checklist.md` (the handover input) is the older D3-D7 desktop checklist
  (sections A-K); none of the 44 tickets is cited in it except DESK13 (line 151, B12 Alt-letter, driven in
  QEMU, "what is left for the Pi is a real keyboard"). It is not used further.
- Kinds: `auto` the program calls the API and checks itself; `capture` the program builds the scene and saves
  the framebuffer, a tester judges on the Mac; `ask` eye or hand at the Pi; `none` not drivable by a program.
  A "(+x)" after a kind names a secondary part.
- Statements are cited (ticket, receipt or source path) unless marked **inferred**. The Pi kind assignment
  itself is a judgement of this triage (inferred) wherever the row's cell says so; the observation sentence
  is the cited part.
- A row marked **covered** has an owner-reported Pi observation in `docs/HW-TEST-LOG.md` or in the result
  section of `docs/api/desktop/_hw-test-2026-09-17.md` (lines 1632-1672, run 2026-09-24, kernel `b49a65d`).
  Whether that closes the ticket is the lead's decision.

## 2. What PITEST01 already provides (cited)

All from `TASK/PITEST01.md` (state DONE 2026-11-11, commit `f55fb4919`) unless noted.

- **Card tool** `tools/sync-card.sh`: `kernels` builds two kernels (`kernela.img` release, `kernelb.img`
  with `crash-nowait`,`fs-io-trace`) and writes `KERNELS.TXT`; `sync` is a dry run without `--write`, with
  `--write` backs up every replaced file plus `kernel8.img`/`cmdline.txt`/`config.txt` to
  `<run>/card-before/`, copies, re-reads every file by sha256 and writes `<run>/card.manifest` and
  `C:\CARDMAN.TXT`; `verify` only checks. Never formats, never deletes (AppleDouble excepted), never writes
  firmware, refuses wrong volumes. Kernel switch by file swap; `cmdline.txt` per kernel with
  `--cmdline` + `--allow-boot-config` (phase A none, phase B `serial=on`).
- **Card list** `measurements/measurement-2026-10-31-pitest01-part3/method/pitest01-card.list`: one line per
  file (destination | source | build command). A merged Pi test program is one more line there
  (**inferred**: the format allows it; no ticket states it).
- **Probes on the card list**: `P0PAR.NAP`, `P0KID.NAP`, `P0REP.NAP` (needs `crash-nowait` kernel, phase B
  only), `HZPROBE.NAP`, `AWPROBE.NAP`, `C:\AUTO\DCPROBE.NCP`; everyday programs QED, TeraDesk, HypView
  (+`HYP_VIEW.CFG`, `ZVIEW.HYP`), DOOM, zView.
- **Run sheet** `docs/analysis/tester-run-sheet-2026-10-31.md`: phases A (release kernel, no UART) and B
  (`crash-nowait` + `serial=on` + UART adapter), per-step PASS/FAIL/NOT RUN, photos, `C:` copied back after
  the run, `tools/sync-card.sh verify` afterwards.
- **Log convention**: probes write a log to `C:\` (`P0PAR.LOG`, `HZPROBE.LOG`, `AWPROBE.LOG`,
  `DCPROBE.LOG`).
- **Not provided (grep, not exhaustive)**: no tool that saves the framebuffer to the card; a grep for
  screenshot/screendump terms over `kernel/src`, `sdk/examples`, `tosdump/probes` found only code comments
  and QEMU-side drivers (`sdk/examples/a96zombie.c` and `a79color.c` are "screendump probes" whose dump is
  taken from the outside by QMP, which has no counterpart on the Pi). A `capture` kind needs a new
  facility; `sdk/examples/physbase.c` and `scrflip.c` exist as `Physbase`/`Setscreen` examples
  (**inferred**: screen memory is reachable from EL0 through `Physbase`, then `Fwrite` to `C:`; not verified here).
- **Existing scene programs usable as a base** (QEMU-driven, source in `sdk/examples/`): `a96zombie.c`,
  `a79color.c`, `a107parent.c`, `a112acc.c`/`a112mesg.c`, `a116acc.c`/`a116w.c`, `a25reinit.c`;
  `measurements/measurement-2026-10-17-a113-fix/method/idprobe.c` (A113 identity probe);
  tosdump dumpers `DUMPVDI`, `DUMPRAST` and `tosdump/probes/*.c`.
- **Pi runs of earlier dumpers**: `TASK/V11.md` cites the file run1-pi4/VDI.BIN of the 2026-09-09 measurement (named by the ticket, not present in the tree)
  (a Pi-4 `DUMPVDI` run); the Pi checklist section 31 (`_hw-test-2026-09-17.md:402-428`) describes the
  `DUMPRAST` run on the Pi with `tosdiff` on the Mac.

## 3. The 44 tickets

Columns: id | what must be observed on the Pi | kind | scene or API call | ask question (yes/no).
Priorities in brackets.

### 3.1 AES identity, resources, messages (no pointer needed)

| id | observed on the Pi | kind | scene / API call | ask |
|---|---|---|---|---|
| A35 [P1] | after `rsrc_load` the caller's `AESGLOBAL` holds the resource base in `ap_ptree`, `ap_rscmem`, `ap_rsclen` instead of 0 (`TASK/A35.md`, "Der Kern"; Missing: hardware acceptance by the tester) | auto | `appl_init`, `rsrc_load(<any .RSC>)`, read `global[5..9]`, compare with `rsrc_gaddr(R_TREE,i)` | - |
| A56 [P2] | `ap_id`, `appl_find`, `WF_OWNER` report 0 for the application chain, `appl_find("SCRENMGR")` = 1, `appl_write(1)` accepted (`TASK/A56.md` receipt block; case 644 "kippt auf der Hardware") | auto | `appl_init`, `appl_find("SCRENMGR")`, `wind_get(0,WF_OWNER)`, `appl_write(1,..)`; note A113 later changed ids of accessories (row A113) | - |
| A57 [P2] | `wind_get(WF_SCREEN)` returns address + length (4 147 200 at 1080p), an EL0 write through the pointer does not fault; alerts and menu panels vanish without residue (`TASK/A57.md` receipt; `_hw-test-2026-09-17.md:253-261`) | auto (+capture) | `wind_get(0,WF_SCREEN)`, write/readback the block (the receipt names `WFSCREEN.NAP`, QEMU, 614 400 B); residue part needs an alert + menu (hand) | - |
| A75 [P3] | `appl_read` blocks until a message arrives, as `evnt_mesag` (`TASK/A75.md`: "kein Bordpfad uebt ein blockierendes `appl_read`") | auto | needs a second process that writes after a delay: probe accessory as in `sdk/examples/a116acc.c` | - |
| A113 [P2] | accessories get ap_id 2.. and are addressable: `appl_find("ACCX")`=2, `("ACCY")`=3, program 0; `appl_write` reaches the accessory; `msg[1]` 2/3 in `WM_REDRAW`; "Pi mit TeraDesk + AV-Accessory" (`TASK/A113.md`, "Ergebnis", Fix-Vorschlag) | auto | `idprobe.c` + two probe accessories (`ACCX`, `ACCY`) on the card | - |
| A115 [P3] | message wake order follows EmuTOS's ready list: MAIN, then ACCC, then ACCB; desk menu lists accessories in reverse load order (`TASK/A115.md`, "Abschluss", "Tester-Abnahme") | auto (+capture for the menu order) | `a112mesg`/f2 style: program + two accessories with different titles; log to `C:` | - |
| A116 [P3] | `appl_write` to a full 128-byte queue of another process blocks, then returns 1 after the accessory reads; a self-writer still gets 0 (`TASK/A116.md`, "Auftrag" point 3) | auto | `a116w.c` + `a116acc.c` (exist; log goes to `Cconws`, so to the screen on the Pi or serial in phase B) | - |
| A118 [P3] | remaining `wind_update` lock takers (`wm_close`, `wm_delete`, `graf_dragbox`, ...) wait for the holder as in EmuTOS; the receipt lists "echte Pi 4, HypView-Serie" as open; **the HypView series is not described in the ticket** | auto | two processes: accessory holds `BEG_UPDATE`, program calls the lock takers (`TASK/A118.md` table; `measurements/measurement-2026-10-26-a118-*` probe not read here) | - |
| A117 [P2] | with a probe accessory waiting in `evnt_multi(MU_MESAG)`, a desktop menu choice (*Desk > Desktop info*, TeraDesk *File > Quit*) is delivered to the menu-bar owner: the dialog appears, the accessory sees no `MN_SELECTED` (`TASK/A117.md`, "Abschluss"; also GitHub #14 with X-Shot) | ask | probe accessory (a112acc style) loaded; hand chooses the menu entry | "With the probe accessory open, does *Desk > Desktop info...* open its dialog (and the accessory print no MN_SELECTED)?" |
| A128 [P1] | file selector: click beside the slider pages 9 rows, dragging the slider moves the list, wheel moves 1 row per notch (`TASK/A128.md`, "Stand 2026-11-04"). **Partly covered:** `HW-TEST-LOG.md` 2026-11-04 row says "file selector with wheel" in daily use; slider click/drag not reported | ask | `fsel_input` from any program (QED open dialog or a test program) | "In the file selector, does a click in the slider track page the list and does dragging the thumb scroll it?" |

### 3.2 Window and object drawing (capture)

| id | observed on the Pi | kind | scene / API call | ask |
|---|---|---|---|---|
| A68 [P3] | text object with `te_thickness = 255` gets a 1 px border, not a 255 px one (`TASK/A68.md`); checklist: dialogs look as before, regression only (`_hw-test-2026-09-17.md:272-279`) | capture | object-draw dialog scene (below) | - |
| A69 [P3] | `G_TEXT`->`G_BOXTEXT` 3D promotion honours the `MD_REPLACE` gate (same checklist item) | capture | same scene | - |
| A70 [P3] | FL3DBAK `G_BUTTON` stays hollow white, no 3D face colour (same) | capture | same scene | - |
| A85 [P3] | SELECTED title leaves a 1 px white edge, list line in a mask dialog is a dark XOR bar with white text (`_hw-test-2026-09-17.md:311-323`) | capture | same scene: a SELECTED menu title/object and a list line | - |
| A99 [P2] | `objc_change` toggles by XOR: a clicked list line shows exactly one inverted line (`_hw-test-2026-09-17.md:311-323`, "A99 nachgezogen") | capture (+ask) | same scene; the click itself needs a hand, the program can instead call `objc_change` itself | - |
| A111 [P3] | `objc_edit` caret is `curfld`'s XOR line, `gl_hchar+6` rows from `y-3`, on/off protocol; QEMU pixel diff showed one 22-pixel column per image (`TASK/A111.md`, "Nachtrag 2026-10-25") | capture | form with an editable field, `objc_edit` EDINIT/EDCHAR/EDEND; also a file selector | - |
| A78 [P2] | moving a window blits the content, no flicker, no extra redraw; one `WM_REDRAW(h,60,80,322,242)` for `wind_set(h,WF_CXYWH,60,80,320,240)` (`TASK/A78.md` table T1; checklist `_hw-test-2026-09-17.md:237-252`) | auto (+capture) | window test program: open a window with content, `wind_set(WF_CXYWH)`, count `WM_REDRAW`, save the frame | - |
| A81 [P2] | `wind_set(0,WF_NEWDESK)` from an LP64 caller registers the tree/root intact (no `0x2_xxxx_xxxx` pointer), background repaints; TeraDesk desktop and ` qed ` title look as before (`TASK/A81.md`; `_hw-test-2026-09-17.md:262-271`) | auto (+capture) | window test program: `wind_set(0,WF_NEWDESK,tree,root)`, `wind_get` readback, repaint | - |
| A83 [P3] | window names/infos are read from the caller's buffer (pointer, not a 79-char copy); titles/info lines unchanged, no empty title, no crash (`TASK/A83.md`; `_hw-test-2026-09-17.md:300-310`) | capture (+auto: `wind_get(WF_NAME)` readback) | window test program with a >79-char title and a buffer changed after `wind_set` | - |
| A94 [P2] | old top window loses its title stipple when a second window opens; only the front one is stippled (`_hw-test-2026-09-17.md:334-343`; reference image falcon-tc-two-windows.png (Hatari Falcon reference shots, not in the tree)) | capture | window test program: two overlapping windows | - |
| A96 [P2] | after an app that left a window open ends, no foreign frame stays on the desktop (`_hw-test-2026-09-17.md:353-363`). **Scene doubt:** the fix point `wm_new_keep_desktop`/`desktop_resume` was deleted by DESK43 U10 (`TASK/DESK43.md`, "Offen"; no hit in `tos-api/src`, `kernel/src`, `programs`) | ask | `sdk/examples/a96zombie.c` started from the desktop | "After `A96ZOMB.NAP` ends, is no window frame of it left on the desktop?" |

### 3.3 Pointer, menu and event feel (hand needed)

| id | observed on the Pi | kind | scene / API call | ask |
|---|---|---|---|---|
| A49 [P3] | a window with `HOTCLOSE` closes on the press, not on release; only a real click can show it (`TASK/A49.md`, "Messbarkeit": "Nur auf echter Pi 4 - es braucht einen Klick") | ask | one window with `CLOSER`+`HOTCLOSE` (0x1000); program logs `WM_CLOSED` with the button state | "Do you press and hold the close box: does the window close at once on the press?" |
| A66 [P2] | no ghost arrow at the pointer position after an alert is closed; pointer outside the alert box: nothing either (`_hw-test-2026-09-17.md:142-154`; `TASK/A66.md`) | ask | `form_alert` over a visible pointer (TeraDesk *Desk > About* or a test program) | "After closing the alert, is nothing left where the pointer was?" |
| A87 [P3] | pointer resting on the menu-bar row right of the last title: the app is not held in the menu track; HELP gives its dialog at once; a click there changes nothing (`_hw-test-2026-09-17.md:324-333`) | ask | TeraDesk or a test program calling `evnt_multi(MU_TIMER 100 ms)` that prints the elapsed time (receipt: timer at 303/316 ms before, ~100 ms after) | "With the pointer resting on the menu bar right of the last title, does HELP open the help dialog at once?" |
| A102 [P3] | menu titles start 16 px from the left; pointer right of *Options* + Return clears the selection at once, no menu track (`_hw-test-2026-09-17.md:364-374`; receipt `TASK/A102.md`, "CR bei (400,8) loescht die Auswahl sofort"). Code moved from the deleted kernel desk folder (gone) to `programs/desktop/src/menu.rs` (exists): the observation is still to be made on NEWDESK (**inferred**) | ask | NEWDESK menu bar, pointer parked | "With the pointer right of *Options*, does Return clear the selection at once?" |
| A89 [P3] | in a menu panel with a submenu, a press on the parent item or in no-man's-land returns the parent item; no `open_box` for a DISABLED title (`TASK/A89.md`, "Abschluss"); the Pi checklist has no own section | ask | menu tree with submenu + a DISABLED title (`menu_attach`), hand moves the pointer | "Does pressing the parent item of an open submenu select the parent (not the submenu item)?" |
| A90 [P3] | `evnt_mouse` follows `amouse`: "nichts sichtbar veraendert; hoechstens eine App mit `evnt_mouse`" (`TASK/A90.md`). **Specific Pi observation not stated** | ask | `evnt_mouse(rect)` loop, hand moves the pointer in/out | "not stated" - needs the lead to name the case |
| A92 [P3] | `evnt_multi` samples the mouse after the first pump; observable "nur bei Eingang zwischen zwei Traps" (`TASK/A92.md`) | ask | `evnt_multi(MU_M1)` with the pointer already outside the rectangle, hand moving the pointer | "With the pointer already outside the rectangle, does `evnt_multi(MU_M1)` return at once?" |

### 3.4 Desktop (NEWDESK) behaviour (hand needed)

| id | observed on the Pi | kind | scene / API call | ask |
|---|---|---|---|---|
| A108 [P3] | menu titles on the 8 px grid without extra spacing (File/View/Options 4/8/12 columns further left), info line one column left and one row lower; reference image falcon-tc-desktop.png (Hatari Falcon reference shots, not in the tree) (`_hw-test-2026-09-17.md:375-382`) | capture | idle NEWDESK with one window open; image of the bar row and info line compared with the reference | - |
| DESK08 [P2] | `Options > Set background...` box centred below the menu bar (19 px below screen centre at 1080p), after `Cancel` the screen is as before, no bar flicker (`_hw-test-2026-09-17.md:504-526`) | ask | NEWDESK dialogs: Set background, New folder, Show info, Install application | "After Cancel, is the screen exactly as before the dialog?" |
| DESK09 [P2] | moved `C:\` window and view mode survive *Save desktop* + cold boot; position snaps to the character grid (up to 15 px up is correct) (`_hw-test-2026-09-17.md:527-543`) | ask | NEWDESK: drag window, *Options > Save desktop...*, power off/on | "After the cold boot, is the window back at the saved place?" |
| DESK13 [P3] | `Set preferences` has six groups; copy and overwrite prompts (`COPY FILE(S)`, `NAME CONFLICT DURING COPY`); delete confirmed once; one date separator (`_hw-test-2026-09-17.md:859-915`). **Partly covered:** the 2026-09-24 result table lists "multiple files dragged to another drive: count box appears, copy works - DESK13's `ADCPYDEL` on hardware" (`_hw-test-2026-09-17.md:1655-1656`, run kernel `b49a65d`); overwrite prompt, delete box, date separator, Alt-letter not reported | ask | NEWDESK with drives C: to F: | "Does a copy onto a drive with a same-named file show `NAME CONFLICT DURING COPY` with an editable name?" |
| DESK43 [P1] | with a defective NEWDESK, the boot falls back to the EmuCON text console (one load attempt) (`TASK/DESK43.md`: "offen nur: Pi-Gegenprobe des EmuCON-Rueckfalls bei defektem NEWDESK") | ask | card variant with an unloadable NEWDESK (a card state set on the Mac, not a program) | "With NEWDESK made unloadable, does the Pi come up in the EmuCON text console?" |

### 3.5 Application-specific

| id | observed on the Pi | kind | scene / API call | ask |
|---|---|---|---|---|
| A60 [P2] | TeraDesk starts with the AESGLOBAL overlay, menu/icons as before, *File > Quit* ends without error (`TASK/A60.md`; `_hw-test-2026-09-17.md:205-212`). **Covered:** `HW-TEST-LOG.md` 2026-09-20 row: `DESKTOP.NAP` `fb37bb3b...` (A60) on the card, TeraDesk "starts/runs with the overlay" "implicitly"; quit path H03 accepted 2026-09-18 (earlier overlay build `db31aae1...`) | none | nothing left to program; whether "implicit" is enough is the lead's call | - |
| T04 [P2] | TeraDesk union reading the upper 32 bits of `long size` (TeraDesk resource.c lines 233-241, 307, 315): only a negative value deletes 13 menu entries, in QEMU all present (`TASK/T04.md`, "Offen"). **Which menus/entries not stated** | ask | TeraDesk session | "In TeraDesk, are all menu entries present (none of the 13 missing)?" |
| A106 [P3] | TeraDesk *Video options > OK* ends TeraDesk after about 2.5 s, desktop returns fresh, twice in a row, no `[EXC]`/`[PANIC]`, `gsx_malloc` logged once (`_hw-test-2026-09-17.md:450-474`) | ask (alt. capture) | TeraDesk video dialog (hand). A program variant: `shel_write(SHW_RESCHNG)` then exit, with a marker file for the autostart instance (**inferred**, needs design) | "After the video dialog, does a fresh desktop come back, twice in a row?" |
| X04 [P3] | TeraDesk *Quit > Reboot* restarts the machine instead of a crash report; Pi-side test needs the new `DESKTOP.NAP` on the card (`TASK/X04.md`, `TASK/X04-STATUS-DONE.MD`). `HW-TEST-LOG.md` 2026-10-18 (RST01): "reset funktioniert" - TeraDesk "Reboot" "not reported back individually" | ask | TeraDesk quit dialog | "Does *Quit > Reboot* in TeraDesk restart the Pi?" |

### 3.6 Console, files, libc, graphics numbers, sound

| id | observed on the Pi | kind | scene / API call | ask |
|---|---|---|---|---|
| CON01 [P3] | cost of the USB poll per output byte (`TASK/CON01.md`, "Erledigt": "Offen: Pi (Kosten des USB-Polls je Ausgabebyte)"). **Covered:** `HW-TEST-LOG.md` 2026-10-18 row: "CON01 (USB poll per output byte without visible slowdown)" | none | nothing left; the other open item (Ctrl-U `newline` decision) is an owner decision, not Pi | - |
| FS11 [P3] | `Fwrite`/`Fread` on AUX/PRN/CON handles; `Fwrite(2,...)` with AUX not hanging mwintlib's write loop (`TASK/FS11.md`, "Nachtrag 2026-10-13"). **Pi-specific observation not stated** (AUX is the real UART) | auto | `Fwrite` to handles 1,2,3 and -1,-2,-3, check return values | - |
| FS18 [P2] | `Frename` of an open file in the same folder: length and clusters kept after write+close; fsck clean (`TASK/FS18.md`, "Befund", "Erledigt"). **Pi-specific observation not stated** | auto (+Mac fsck of the card) | `Fcreate`, `Frename`, `Fwrite`, `Fclose`, `Fsfirst` length; fsck on the Mac afterwards | - |
| LIBC-BUF [P3] | after the first `printf` on redirected stdout the buffer stays 32 KiB, as for an `fopen` file (`TASK/LIBC-BUF.md`, "Anwenderentscheid"). **Pi-specific observation not stated** | auto | probe `LBUF` (QEMU run `measurements/measurement-2026-10-22-libc-buf/`) | - |
| V11 [P1] | `v_clrwk` then `v_get_pixel` returns `0xFFFF` (not `0xFFDF`) on a Pi and the picture is unchanged (`TASK/V11-STATUS-DONE.MD`, "Pi-Abnahme": cases 54/160/161) | auto | `DUMPVDI` cases 54, 160, 161 (a Pi run exists: the file run1-pi4/VDI.BIN of the 2026-09-09 measurement (named by the ticket, not present in the tree) per `TASK/V11.md`) | - |
| R05 [P1] | `DUMPRAST` with `vdi-raster-screen.tsv`: log "all 62 cases ran", `tosdiff` 0 of 62 against `EMUTOS14-RASTER-screen-falcontc.BIN` (`_hw-test-2026-09-17.md:402-428`; `TASK/R05.md`) | auto (+Mac tosdiff) | `DUMPRAST.NAP` + `C:\RASTER.TSV` (62 cases); card steps are written out in the checklist | - |
| SND03 [P2] | the owner hears the PWM test tone at the 3.5 mm jack; H1-H4 numbers are already measured on the Pi (`TASK/SND03.md`, "Real-Pi run (owner, 2026-11-12)": hearing test open, owner had no device with a jack) | ask | probe kernel `C:\kerns03.img` already on the card (not a program) | "Do you hear the test tone at the jack?" |

## 4. Tickets that share a scene

The program needs each scene once; the tickets in the group are judged from it.

| scene | tickets | note |
|---|---|---|
| S1 object-draw dialog (3D text, FL3DBAK button, SELECTED object, list with XOR line, edit field with caret) | A68, A69, A70, A85, A99, A111 | one form tree, one capture |
| S2 window test program (content window, `WF_CXYWH` move, `WF_NAME`/`WF_INFO` long buffer, `WF_NEWDESK`, two overlapping windows) | A78, A81, A83, A94 (A96 as variant: leave a window open at exit) | `a96zombie.c`/`a79color.c` as base |
| S3 program + two probe accessories (`ACCX`, `ACCY`, slow reader, lock holder) | A75, A113, A115, A116, A118 (A117 needs the same accessory plus a hand-chosen menu entry; A56 identity part) | `a112acc`, `a112mesg`, `a116acc`, `a116w`, `idprobe` exist |
| S4 AES identity/global probe | A35, A56, A57 | no pointer, no scene |
| S5 menu-bar pointer park + `evnt_multi` timing/mouse rectangle | A87, A102, A90, A92 | one hand action, program prints the numbers |
| S6 NEWDESK desktop idle | A108 (A102) | capture of the bar row and info line |
| S7 NEWDESK dialogs and INF round trip | DESK08, DESK09, DESK13 | owner session on the desktop |
| S8 TeraDesk session | T04, A106, X04 (A66, A87, A117 can use TeraDesk) | one TeraDesk start |
| S9 tosdump dumpers on the Pi | V11 (`DUMPVDI`), R05 (`DUMPRAST`) | existing programs, Mac-side `tosdiff` |
| S10 GEMDOS/libc probe | FS11, FS18, LIBC-BUF | one program |
| S11 alert over a visible pointer | A66 (A57 residue check) | hand |
| S12 file selector | A128 (A111 caret in the selector) | hand |

## 5. Tickets in the population whose `Missing` field does not name a Pi run

25 of the 69. `Missing` text verbatim (from line 3):

- A132, A133, DESK39, DESK47, FS08, MEM03, CON02, V36, XBIOS04, ARCH05: "tester acceptance through cargo
  xtask accept" (A132/A133 add "fix and independent Tester run exist, line 3 was never updated").
- A19, A42, LIBC-FSTAT, LIBC-TTY, V17: "tester acceptance".
- A37: "application-side acceptance"; SDK01, T08: "acceptance"; SYSINFO02: "one open point (Esc, kernel AES)";
  PROC06: "check that the change is committed (legacy line says uncommitted)"; PORT12: "ADR decision (owner)
  for stage 2"; SPEC04: "developer points 3 (generator) and 5 (run the measurement case)"; TOOL18: "seven
  cookies open with the owner"; V18: "parts 2 and 3: one fix ticket per finding"; DESK14: "item 4 (deferred)".

Whether any of these also hides a Pi step in its body was not read (out of scope).

## 6. In the 44, but the Pi run is doubtful after all

- **Covered by existing owner Pi observations (lead decides):** A60 (`HW-TEST-LOG.md` 2026-09-20 row), CON01
  (2026-10-18 row). Partly covered: A128 (wheel, 2026-11-04 row), DESK13 (copy box, `_hw-test-2026-09-17.md:1655-1656`).
- **Fix point deleted, scene doubtful:** A96 (`wm_new_keep_desktop` gone after DESK43 U10).
- **No Pi-specific observation stated in ticket or receipt:** FS11, FS18, LIBC-BUF (only "Pi offen"), A90
  ("nichts sichtbar veraendert"), T04 (which 13 entries), A118 ("HypView-Serie" undefined).
- **Not an observation of a program:** SND03 (hearing test only; H1-H4 measured), DESK43 (card state, boot
  path).
- **`Missing` names the Pi but the checklist says the Pi adds nothing a QEMU run did not show (regression
  only):** A68, A69, A70 (`_hw-test-2026-09-17.md:272-279`: "Sichtbar wuerde nur eine Regression"), A83
  ("Sichtbar aendert sich nichts"), A81 (same for TeraDesk root 0).

## 7. Count by kind (first-named kind of each row)

auto 14 (A35, A56, A57, A75, A113, A115, A116, A118, A78, A81, FS11, FS18, LIBC-BUF, V11) + R05 = 15;
capture 8 (A68, A69, A70, A85, A99, A111, A83, A94) + A108 = 9;
ask 18 (A117, A128, A96, A49, A66, A87, A102, A89, A90, A92, DESK08, DESK09, DESK13, DESK43, T04, A106, X04,
SND03);
none 2 (A60, CON01). Total 15 + 9 + 18 + 2 = 44.

## Open points

- STOP: A90 - the ticket states no Pi-specific observation; the program cannot be designed until the lead names the case.
- STOP: A118 - "HypView-Serie" is open in the receipts but never defined in the ticket text read; not guessed.
- STOP: T04 - "13 menu entries" of the TeraDesk union: which menus is not stated in the ticket.
- STOP: FS11, FS18, LIBC-BUF - no Pi-specific observation stated; the `auto` kind is the program checking the
  API result, which the QEMU receipts already did.
- Inferred: kind assignment per row (judgement of this triage); `capture` feasibility from EL0 via `Physbase`
  + `Fwrite` (not verified; no capture tool found by a non-exhaustive grep); new card-list line for the merged
  program; A102/A108 observation moves to NEWDESK after the deletion of the deleted kernel desk folder.
- Inferred: A106 program variant (`shel_write(SHW_RESCHNG)` + marker file).
- Not read: the receipts `A56`, `A57`, `A75`, `A113` bodies beyond the ticket text and the cited checklist
  sections; probe sources `idprobe.c` content; A118 probe under `measurements/`.
- Out-of-scope observations: `TASK/A35.md` carries both `A35-STATUS-DONE.MD` and `A35-STATUS-UNFINISHED.MD`
  receipts (not examined); `docs/HW-TEST-LOG.md` row 2026-09-20 and the 2026-11-11 correction row disagree
  on which of A39/A40/A46/H05 are confirmed (not in this scope).
