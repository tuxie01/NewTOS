# Line-A-Leser in Ports und NewTOS-Komponenten (2026-10-26)

**Zweck:** Beleg für ADR-030 (`docs/MEMORY.md`, *No Line-A emulation*,
Status Proposed), offene Fragen **Q1** (LINEA-Variablenblock read-only
exportieren?) und **Q2** (C-Funktionsgruppe `tos-linea.h` aus Phase 00 D10,
`docs/api/_line-a.md:65-76`, streichen?). Gesucht ist jede Stelle, die
Line-A-Variablen liest oder Line-A-Aufrufe / `tos-linea.h` benutzt.

**Stand:** HEAD `86608e41`, EmuTOS-Referenz `references/emutos` @ `1a7484ab`.
Datierter Schnappschuss; nichts außer dieser Datei wurde geändert.

## Methode

Durchsucht: `sdk/` (Apps, Overlays, mintlib, gemlib, cflib, WinDom, LDG,
zView, Codec-Bibliotheken, Beispiele, Vorlagen), `Applications/` (lokale,
gitignorierte Checkouts: cflib-git, Conquer-VDI (leer), doom-wad, doomgeneric,
ldg-2, ldg-src, mintlib-src, pml-2.03, qed, thing-neo-codeSVN-r62,
Vanilla-Conquer-Atari-ST, windom-1.21.3, z-tools), `programs/`, `tosdump/`,
`kernel/`, `tos-api/`, `tos-types/`, `tools/`. `references/` nur für Tabelle 3.
Immer `/usr/bin/grep -rnI` (Binärdateien übersprungen), `target/` und
`.claude/worktrees/` ausgeschlossen (Worktrees separat gezählt).

Muster (case-sensitive, außer wo `-i` steht):

1. Aufrufe/Symbole: `\blinea[0-9a-f]\b|\blinea_init\b|__aline|__fonts\b|__funcs\b|\bla_init\b|tos-linea|\bLINEA\b|__LINEA|line_a_vars|lineavars`
2. Header: `#\s*include\s*[<"]([a-z]+/)?(tos-)?linea\.h`
3. Opcodes (`-i`): `(0x|\$)a00[0-9a-f]\b|dc\.w[^\n]*a00[0-9a-f]|\.word[^\n]*0xa00|__asm__[^\n]*a00[0-9a-f]` (Treffer mit fünfstelligen Hex-Werten verworfen)
4. Variablen: `\b(V_REZ_HZ|V_REZ_VT|MOUSE_BT|GCURX|GCURY|V_CEL_(HT|MX|MY|WR)|V_X_MAX|V_Y_MAX|V_BYTES_LIN|BYTES_LIN|V_LIN_WR|CUR_MS_STAT|M_HID_CT|V_HID_CNT|CUR_FONT|DEF_FONT|FONT_RING|CUR_WORK|DEV_TAB|SIZ_TAB|INQ_TAB|VPLANES|VWRAP|v_cel_m[xy]|v_rez_hz|v_rez_vt|v_lin_wr|v_planes|v_bytes_lin|mouse_bt|gcurx|gcury|Vdiesc|vdiesc|LA_VARS|la_vars)\b`
5. Pure-C-Namen: `Vdiesc|VDIESC|Linea->|\blinea_init\b|\bLINEA_FUNP\b|Fonts->|<linea\.h>|"linea\.h"`
6. Wortlaut (`-i`): `\bline[- _]?a\b|\blinea\b`

Jeder Treffer wurde gelesen; reine Namensgleichheit (z. B. `planes` als
Bildformat-Feld, `SIZ_TAB` als Name für `vq_extnd`-Ausgaben, EXIF-Tag `0xa000`)
ist unten als „VDI-Weg“ bzw. „kein Leser“ eingeordnet.

Zusätzlich **Binärprüfung**: alle 62 gebauten NewTOS-Programme unter `sdk/`,
`tosdump/`, `programs/` (`*.NAP/*.NCP/*.NAC/*.NTP/*.LDG`, alle ELF64 AArch64,
nicht gestrippt) mit `llvm-nm` aus dem gepinnten Nightly auf die Symbole
`__aline`, `__fonts`, `__funcs`, `linea0`…`lineaf` geprüft — **kein einziges
Programm enthält eines davon**. `sdk/libc/libc.a` enthält `__aline`,
`__fonts`, `__funcs` (BSS, Member `mintlib__alglobal.o`), aber keine
`linea*`-Funktion.

Was ein Leser heute auf NewTOS erlebt: `linea0()` … `lineaf()` sind in
`sdk/libc/mintlib/include/mint/linea.h:760-775` deklariert, aber nirgends
definiert → **Linkfehler** (undefined symbol). `__aline` bleibt **null**
(BSS); jedes Makro aus `mint/linea.h:154-315` ist `*(short *)((char
*)__aline + -N)`, also ein Lesezugriff knapp unter `2^64` → EL0-Datenabbruch →
`kill_crashed_program` beendet das Programm
(`kernel/src/arch/aarch64/exceptions.rs:303-309`). Ein `$Axxx`-Opcode existiert
auf AArch64 nicht; m68k-Binärcode läuft nicht.

---

## Tabelle 1 — Portierte Anwendungen und SDK-Bibliotheken

| # | Datei:Zeile | Was | Gebaut / ausgeliefert? | Wirkung auf NewTOS heute |
|---|---|---|---|---|
| 1 | `sdk/libc/mintlib/mintlib/alglobal.c:4` | `#include <mint/linea.h>` | ja, `libc.a` (`mintlib__alglobal.o`, `.err` leer) | — |
| 2 | `sdk/libc/mintlib/mintlib/alglobal.c:7`, `:11`, `:17` | **definiert** `__aline`, `__fonts`, `__funcs` (liest nichts) | ja, in `libc.a`; in **keines** der 62 Programme gelinkt | null-Zeiger, niemand liest ihn |
| 3 | `sdk/libc/mintlib/mintlib/linea.c:39` (`LINEA_OPCODE_BASE 0xa000`), `:46`–`:246` (`linea0`…`lineaf`) | die 16 Line-A-Bindungen, m68k-Inline-asm | **nein**: Compilerfehler, verbucht in `sdk/libc/expected-failures.txt:21`, `sdk/libc/build/obj/mintlib__linea.err`, `sdk/libc/build/failed.txt:2` | Port mit `linea*()`-Aufruf linkt nicht |
| 4 | `sdk/libc/mintlib/unix/ioctl.c:11` (include), `:122` (`linea0()`), `:123`–`:126` (`V_CEL_MY`, `V_CEL_MX`, `V_X_MAX`, `V_Y_MAX`) | `ioctl(TIOCGWINSZ)`-Rückfall „under TOS“ | **nein**: `EXCLUDE` in `sdk/libc/build-libc.sh:222`, ersetzt durch `sdk/libc/newtos-arch/ioctl.c` (reicht an `Fcntl` weiter, kein Line-A-Zweig); in `sdk/libc/build/excluded.txt` | kein Leser; außerhalb mintlib ruft kein Programm `TIOCGWINSZ` |
| 5 | `sdk/libc/mintlib/include/mint/linea.h:154`–`:315` (Makros mit negativem Offset), `:485`ff. (`__aline->_X`-Makros), `:748`–`:775` (Deklarationen) | Header; auf dem Include-Pfad (`sdk/bin/newtos-cc:277`) | Header, kein Code | **kein** gebautes Quellfile außer #1/#3/#4 bindet ihn ein |
| 6 | QED (`sdk/apps/qed`, `sdk/overlays/qed`), TeraDesk (`sdk/apps/teradesk`, `sdk/overlays/teradesk`), HypView, zView + Plugins (`sdk/zview`), HighWire, Doom (`doomgeneric_gem.c`), CoreMark, present, gemtest/apitest/chaintest, cflib, gemlib, WinDom, LDG, libshare, Codec-Bibliotheken, `sdk/examples`, `sdk/templates` | — | ja (soweit gebaut) | **0 Treffer.** Bildschirm-/Mausdaten nur über VDI/AES, z. B. `doomgeneric_gem.c:514-516` (`INQ_TAB[4/5]`, `DEV_TAB[3/4]` aus `vq_extnd`), `sdk/examples/uakid.c:200-201` (`vq_extnd`) |
| 7 | `Applications/mintlib-src/mintlib-0.60.1/…` (`alglobal.c`, `mintlib/linea.c`, `unix/ioctl.c`, `include/mint/linea.h`) | Upstream-Kopie derselben Dateien wie #1–#5 | nein (Bau läuft aus `sdk/libc/mintlib`) | — |
| 8 | `Applications/Vanilla-Conquer-Atari-ST/tiberiandawn/atarilib/startup.cpp:45` (include), `:225` (`linea0()`) | Line-A-Init beim Start | **nur m68k**: ST-Bau `makefile` mit `CROSS_TARGET` m68k-atari-mint[elf] (`makefile:32-41`); kein NewTOS-Bau | nicht für NewTOS; m68k-Binärprogramm |
| 9 | `…/atarilib/st_linea0.S:8-19` (`__aline`/`__fonts`/`__funcs`), `:24`–`:30` (`linea0`, `.short 0xa000` in `:26`) | eigener `linea0`-Shim für libcmini | nur m68k-ST-Bau mit `USE_LIBCMINI=1` (`makefile:600-607`) | wie #8 |
| 10 | `…/atarilib/stvq/stvq_hw.c:168` (`dc.w 0xa00a`, hide mouse), `:216` (`dc.w 0xa009`, show mouse) | Line-A-Opcodes direkt | nur m68k-ST-Bau (`makefile:587`) | wie #8 |
| 11 | `…/atarilib/mouseww.cpp:17` | `#include <mint/linea.h>` (Kommentar: GCURX, GCURY, MOUSE_BT) — **keine** Makro-Verwendung in der Datei gefunden | nur m68k-ST-Bau | wie #8 |
| 12 | Conquer-GEM-Variante `tiberiandawn/gem.mk`, `tiberiandawn/gem/*` | — | m68k (TT, `gem.mk:1-18`); `atarilib/` wird nicht benutzt | **0 Treffer** (nur Kommentar `gem/video_gem.cpp:239`) |
| 13 | `Applications/thing-neo-codeSVN-r62`, `qed`, `z-tools`, `cflib-git`, `ldg-2`, `ldg-src`, `windom-1.21.3`, `doomgeneric`, `pml-2.03` | — | lokale Checkouts | **0 Treffer** |

Vanilla-Conquer hat zusätzlich 20 Agent-Worktrees unter
`.claude/worktrees/`; sie zeigen dieselben Treffer an denselben Zeilen und sind
oben nicht mitgezählt.

## Tabelle 2 — NewTOS-Komponenten

| # | Datei:Zeile | Was | Art |
|---|---|---|---|
| 1 | `kernel/`, `tos-api/`, `tos-types/` gesamt | kein Line-A-Trap, kein Vektor-`$28`-Dispatcher, kein LINEA-Block, kein `__aline`; Muster 1/3/5 ohne Code-Treffer | — |
| 2 | `tos-api/src/bios/vectors.rs:98-109` | `Setexc(10, …)` speichert nur in den Vektor-Schatten | kein Dispatch |
| 3 | `kernel/src/arch/aarch64/exceptions.rs:303-309` | EL0-Abbruch / undefinierter Befehl → Programm beendet | Folge eines Line-A-Zugriffs |
| 4 | `tos-api/src/vdi/mouse.rs:24-26` (41 Trefferzeilen in der Datei, alle Zustand oder Kommentar) | `GCURX`, `GCURY`, `MOUSE_BT` als kernelinterne Atomics; nach außen über `vq_mouse` usw. | intern, nicht veröffentlicht |
| 5 | `tos-api/src/vdi/escape.rs:110` | `v_cel_mx`/`v_cel_my` bei jedem Eintritt aus der Backend-Geometrie (für `vq_chcells`) | intern |
| 6 | `tos-api/src/vdi/req_col.rs:8-13` | `REQ_COL` gerätespezifisch „(Line-A)“ | intern |
| 7 | `tos-api/src/vdi/workstation.rs:363`, `:372` | `CUR_WORK` nicht modelliert (AUD-V1-010) | Kommentar |
| 8 | `tos-api/src/vdi/{attributes,backend,contour,eddi,font,input,inquiry,mod,primitives,raster,vwk,gdp}.rs`, `tos-api/src/aes/{appl,graf,gsxif,resource}.rs`, `tos-api/src/xbios/screen.rs` | EmuTOS-Variablennamen (`v_planes`, `INQ_TAB`, `DEV_TAB`, `SIZ_TAB`, `V_REZ_VT` …) in Doku-Kommentaren und Feldnamen | Kommentar / intern |
| 9 | `kernel/src/vdi_backend.rs:12` (7 Zeilen), `kernel/src/hal/framebuffer.rs:10` („VDI / Line-A, later phases“), `kernel/src/shell_loop.rs:393` (`GCURX = xres / 2` als EmuTOS-Zitat), `kernel/src/selftest_vdi.rs:82`, `kernel/src/selftest_aes.rs:98` | Kommentare | Kommentar |
| 10 | `kernel/src/shell.rs` (EmuCON-Stand-in im Kernel) | keine Line-A-Lesung, kein `getwh` | — |
| 11 | `programs/desktop/src/main.rs:2814-2826` | `do_xyfix` **ohne** den `V_REZ_HZ`-Schubs (`desksupp.c:265-269` weggelassen) | Abweichung, Q3 |
| 12 | `programs/desktop/src/` | kein `get_key`/`show_file`, also kein `MOUSE_BT` | — |
| 13 | `programs/desktop/src/gsx.rs:94`, `:109` | `SIZ_TAB[1]` über `vq_extnd(h,0)[46]` | VDI-Weg |
| 14 | `programs/newtos-gem/src/vdi.rs:264` | `SIZ_TAB[1]` als Doku zu `vq_extnd` | VDI-Weg |
| 15 | `programs/newdesk`, `programs/hello-gem` | — | 0 Treffer |
| 16 | `tosdump/core/gem.h:24-25`, `gem.c:17`, `dumpvdi.c:108-114`, `dumprast.c:103-109`, `scrninfo.c:456`, `:532`, `runner.c:27-32`, `runner_rast.c:150`, `runner_rast.h:32`, `tests/test_runner*.c` | `DEV_TAB`/`SIZ_TAB`/`INQ_TAB` als Namen für `v_opnvwk`/`vq_extnd`-Ausgaben | VDI-Weg |
| 17 | `tosdump/core/cases.h:599` | beschreibt, dass EmuTOS' `setup_info` intern `v_planes`/`v_lin_wr` liest | Kommentar über EmuTOS-VDI |
| 18 | `tos-api/tests/vdi_mouse_park.rs`, `vdi_reference/{harness,cases_v_series,cases_r2,overlay_bit}.rs`, `vdi_eddi_scrninfo.rs`, `vdi_vex_wheelv.rs`, `aes_k1_gl_handle.rs`, `aes_menu_bar_height.rs`, `aes_object_font.rs`, `aes_reference/harness.rs` | Host-Tests der internen Zustände unter EmuTOS-Namen | Test, intern |
| 19 | `tools/xtask/emutos-defines.snapshot` (2 Zeilen) | EmuTOS-Config-Schnappschuss | Daten |

`tos-linea.h` existiert als Datei nirgends; erwähnt nur in
`docs/api/_line-a.md:75`, `docs/MEMORY.md:2827`, `:2914`,
`docs/api/emucon/06-mapping-and-decisions.md:150`.

## Tabelle 3 — EmuTOS-Referenz (nur Kontext, @ `1a7484ab`)

Nur **Leser**; die Anbieter-Seite (`bios/linea.S`, `bios/lineavars.S`,
`bios/lineainit.c`, `vdi/*`, übrige `bios/*`) ist Implementierung.

| # | Datei:Zeile | Was | Gating |
|---|---|---|---|
| 1 | `desk/desksupp.c:56` | `#include "lineavars.h"` (für `MOUSE_BT`, `V_REZ_HZ`) | — |
| 2 | `desk/desksupp.c:267` | `V_REZ_HZ` in `do_xyfix` | `#if CONF_WITH_3D_OBJECTS` (`:265-269`) |
| 3 | `desk/desksupp.c:515`, `:517` | `MOUSE_BT & 2` / `& 1` in `get_key` (Seitenwarten); Aufrufer `:543`, `:555`, `:740`, `:819` | `#if CONF_WITH_SHOW_FILE \|\| CONF_WITH_PRINTER_ICON` (`:474-664`) |
| 4 | `desk/deskmain.c:1509` | nur Kommentar (Systemfontbreite „documented in lineavars.h“) | — |
| 5 | `cli/cmdasm.S:51-60` `_getwh` | `.dc.w 0xa000` (`:56`; ColdFire `0xa920` `:54`), `move.l -44(a0),d0` = `v_cel_mx`/`v_cel_my` (`:58`) | — |
| 6 | `cli/cmdasm.S:62-72` `_getht` | `.dc.w 0xa000` (`:67`), `move.w -46(a0),d0` = `v_cel_ht` (`:70`) | — |
| 7 | `cli/cmdedit.c:134` | `getwh()` in `init_screen` → `screen_cols`/`screen_rows`/`linesize` | — |
| 8 | `cli/cmd.h:253-254` | Deklarationen; `getht` hat im Baum keinen Aufrufer | — |
| 9 | `aes/` | kein Line-A-Opcode, kein `lineavars.h`; Maus über `gsx_moff`/`gsx_mon` (VDI) | — |

## Tabelle 4 — gebaut vs. nicht gebaut

| Stelle | Gebaut für NewTOS? | Beleg |
|---|---|---|
| mintlib `alglobal.c` (`__aline`/`__fonts`/`__funcs`) | ja, `libc.a` | `sdk/libc/build/compiled.txt:240`, `llvm-nm libc.a` |
| … davon in ein Programm gelinkt | **nein** (0 von 62) | `llvm-nm` über alle gebauten Programme |
| mintlib `linea.c` (`linea0`…`lineaf`) | nein, Compilerfehler | `sdk/libc/expected-failures.txt:21` |
| mintlib `unix/ioctl.c` (`TIOCGWINSZ` → `V_CEL_*`, `V_X/Y_MAX`) | nein, ausgeschlossen | `sdk/libc/build-libc.sh:222`, `sdk/libc/build/excluded.txt` |
| `mint/linea.h` | Header auf dem Include-Pfad, von keinem gebauten Quellfile außer mintlib benutzt | Muster 2 |
| QED, TeraDesk, HypView, zView, HighWire, Doom, CoreMark, present, Testprogramme, Bibliotheken | ja | 0 Line-A-Treffer |
| NEWDESK (`programs/desktop`), `newdesk`, `newtos-gem`, `hello-gem` | ja (Rust, EL0) | 0 Line-A-Lesungen |
| tosdump-Dumper (`DUMPVDI`, `DUMPRAST`, `DUMPAES`, `VDIDRAW`, `SCRNINFO`) | ja | 0 Line-A-Lesungen |
| Vanilla-Conquer `atarilib` (`startup.cpp`, `st_linea0.S`, `stvq_hw.c`, `mouseww.cpp`) | nein — nur m68k-ST-Bau, kein NewTOS-Ziel | `tiberiandawn/makefile:32-41`, `:600-607` |
| EmuDESK `desksupp.c`, EmuCON `cli/` | Referenz, nicht Teil des Baus | — |

---

## Zusammenfassung (Fakten)

- **Q1 (LINEA-Block):** Kein ausgeliefertes NewTOS-Programm liest einen
  LINEA-Variablenblock. Kein gebautes Programm referenziert `__aline`. Die
  einzige Lesestelle im SDK-Quellbaum (mintlib `unix/ioctl.c:122-126`,
  `TIOCGWINSZ`) wird nicht gebaut. Leser gibt es nur außerhalb des
  NewTOS-Baus: EmuDESK (`V_REZ_HZ`, `MOUSE_BT`; NEWDESK baut beides nicht
  nach, Q3 legt den VDI-Weg fest), EmuCON (`v_cel_mx`/`v_cel_my`, `v_cel_ht`
  ohne Aufrufer; NewTOS' EmuCON ist ein Kernel-Stand-in ohne Line-A, D4 offen)
  und das m68k-only Vanilla-Conquer-ST (`GCURX`/`GCURY`/`MOUSE_BT` per
  `linea0` vorbereitet).
- **Q2 (`tos-linea.h`):** Kein ausgeliefertes Programm ruft eine
  Line-A-Funktion. Die einzigen Bindungen (mintlib `linea.c`) bauen nicht und
  sind als bekannter Fehler verbucht; `tos-linea.h` existiert nicht. Aufrufer
  von Line-A-Funktionen gibt es nur in m68k-Code: EmuCON (`$A000` für die
  Variablen) und Vanilla-Conquer-ST (`$A000`, `$A009`, `$A00A`) — beide kein
  NewTOS-Bau.
