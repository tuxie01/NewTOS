# TOOL33 — `mapping.tsv` fuer die AES-Ebene (EmuTOS `aes/` → NewTOS)

Status: **ERLEDIGT** 2026-11-05 ([Quittung](TOOL33-STATUS-DONE.MD)) · Prioritaet: **P1** (Anwender 2026-11-05: nach [TOOL32](TOOL32.md) VDI, vor [A119](A119.md)) · Typ: Researcher (Zuordnung lesen), danach Lead (Pruefregel) — Werkzeug, das die Lokalisierung der folgenden Fixes braucht (CLAUDE.md "Testing")

## Sachverhalt

`cargo xtask deps --coverage` am 2026-11-05: von den 418 EmuTOS-Funktionen
der Ebene `aes` haben 29 eine Zeile in `docs/analysis/deps/mapping.tsv`
(nachgezogen von A127, A128, A132, A133 u. a.), 389 keine. Gleiche Ursache wie
bei der VDI: die Zuordnung deckt von Hand nur `bios/` und `bdos/` ab
(`MAPPED_DIRS`, `tools/xtask/src/deps.rs:63@MAPPED_DIRS`).

Funktionen je Datei: `gemgsxif.c` 43, `gemwmlib.c` 39, `gemgraf.c` 30,
`gemrslib.c` 28, `geminput.c` 26, `gemshlib.c` 23, `gemoblib.c` 23,
`geminit.c` 21, `gemgrlib.c` 19, `gemmnext.c` 17, `gemfslib.c` 17,
`gemmnlib.c` 15, `gemdosif.S` 13, `gemobed.c` 12, `gemfmlib.c` 10,
`gemctrl.c` 9, `gemevlib.c` 8, `gemasync.c` 8, `gempd.c` 7, `gemdisp.c` 7,
`gemwrect.c` 6, `gemaplib.c` 6, `gemfmalt.c` 4, `gemflag.c` 4, `gemasm.S` 4,
`gemsuper.c` 3, `gemstart.S` 3, `gemsclib.c` 3, `gemobjop.c` 3,
`gemqueue.c` 2, `gem_rsc.c` 2, `gsx2.c`/`gemevlib.h`/`aesvars.h` je 1.

## Auftrag

Wie [TOOL32](TOOL32.md): jede `aes/`-Funktion bekommt eine Zeile (Format und
Klassen `tools/deps/README.md`), vorhandene Zeilen werden geprueft, beim
Lesen gefundene Abweichungen nur gemeldet. Vier Auftraege nacheinander, je ein
frischer Researcher, von unten nach oben (2026-11-05):

1. **Grafik und Objekte:** `gemgsxif.c`, `gemgraf.c`, `gemoblib.c`, `gemobed.c`,
   `gemobjop.c`, `gsx2.c` (112).
2. **Fenster, Formulare, Dateiauswahl:** `gemwmlib.c`, `gemgrlib.c`, `gemwrect.c`,
   `gemfmlib.c`, `gemfmalt.c`, `gemfslib.c` (95).
3. **Menue, Ressourcen, Shell:** `gemmnlib.c`, `gemmnext.c`, `gemrslib.c`, `gemshlib.c`,
   `gem_rsc.c`, `gemsclib.c` (88).
4. **Ereignisse, Prozesse, Start:** `geminput.c`, `geminit.c`, `gemdosif.S`, `gemctrl.c`,
   `gemevlib.c`, `gemevlib.h`, `gemasync.c`, `gempd.c`, `gemdisp.c`, `gemaplib.c`,
   `gemflag.c`, `gemasm.S`, `gemsuper.c`, `gemstart.S`, `gemqueue.c`, `aesvars.h` (123).
Danach (Lead): `"aes/"` in `MAPPED_DIRS`, `cargo xtask test` gruen.

## Stand Teil 1 (2026-11-05, Researcher)

127 Zeilen fuer die 112 Funktionen von `gemgsxif.c`, `gemgraf.c`, `gemoblib.c`, `gemobed.c`,
`gemobjop.c`, `gsx2.c`; die 12 vorhandenen Zeilen gelesen und belassen. `--check-mapping`
sauber, `--coverage` aes danach 116 mapped / 0 design / 302 unmapped (Rest = Teile 2–4).
Je Funktion 97 `mapped`, 15 `unmapped` mit Grund (u. a. `gsx_gclip`, `gsx_chkclip`,
`gsx_xcbox`, `gsx_mfree`, `gsx_resetmb`, `gsx_setmousexy`, `vs_clip`, `vrn_trnfm`).

Abweichungs-Kandidaten aus dem Lesen (nicht gemessen; EmuTOS-Zeilen in `aes/`):

1. `gemgrlib.c:127-143,225-235,266-278,296-335` gegen `tos-api/src/aes/graf.rs:286@animate`:
   Grow/Shrink ohne Zentrierphase und Eckstuecke (`gsx_xcbox`), ein Vollumriss je Frame + 15 ms.
2. `gemgraf.c:140-163` gegen `graf.rs:66@gsx_xline`: Muster gleich, Stift ignoriert, XOR nur
   ueber `WriteMode::Xor`; gegen den VDI-XOR nicht geprueft.
3. `gemoblib.c:~597-600` gegen `tos-api/src/aes/object.rs:2926@just_draw`: `G_BOXCHAR`
   immer transparent — bekannt als [A135](A135.md).
4. `gemoblib.c:903-908` gegen `just_draw`: `CHECKED`-Haken nur bei `G_STRING`.
5. `gemoblib.c:~858-868`: Unterstreichung der erweiterten Objekte fehlt.
6. `gemoblib.c:917-929` gegen `just_draw`: `DISABLED` auf `FL3DBAK` mit fester Farbe statt
   `backgrcol` (`objc_sysvar`).
7. `gemoblib.c:533-541` gegen `just_draw`: keine Clip-Vorabpruefung, `G_USERDEF` auch fuer
   ausgeklippte Objekte.
8. `gemgraf.c:735-748` gegen `tos-api/src/aes/object.rs:3914@draw_icon`: WHITEBAK-Sonderfall nur im Farbzweig.
9. `gemgraf.c:662-667` gegen `draw_icon`: Farb-Icon immer `S_AND_D` statt `S_OR_D` bis 8 Ebenen.
10. `gemoblib.c:838-841` gegen `just_draw`: Trennlinie in `G_STRING` schwarz statt `gl_lcolor`.
11. `gemoblib.c:1066-1096` gegen `tos-api/src/aes/object.rs:4147@find_hit`: `objc_find` mit nominalem statt
    3D-vergroessertem Elternursprung.
12. `gemoblib.c:1224-1245` gegen `tos-api/src/aes/object.rs:1451@ob_order`: Grenzfaelle anders.
13. `gemobed.c:176-182` gegen `tos-api/src/aes/object.rs:1966@pxl_rect`: Cursor in `FL3DOBJ`-Feldern.
14. `scancode.h` gegen `tos-api/src/aes/object.rs:2005@edit_char`: Tastenwort statt Byte — bekannt als
    [A122](A122.md).
15. `gemobed.c:236-240,272-274` gegen `edit_char`: leeres `te_pvalid` nimmt jedes Zeichen.
16. `gemgsxif.c:98-105` gegen `tos-api/src/aes/gsxif.rs:358@gsx_mcalc`: Puffergroesse ganzer
    Schirm statt `MENU_BUFFER_SIZE`-Formel (sichtbar ueber `wind_get(WF_SCREEN)`).
17. `gemgsxif.c:440-446` gegen `tos-api/src/vdi/mouse.rs:966@hide`: AES verbirgt die Maus bei
    jedem Aufruf statt nur beim ersten (`gl_moff`).
18. `gemgraf.c:413-416` gegen `graf.rs`: `gl_wbox` fest 19 statt Formel.
19. `gemgsxif.c:238-252` gegen `tos-api/src/aes/gsxif.rs:252@gsx_graphic`: Mausvektoren werden nicht
    zurueckgestellt.
20. `gemaplib.c:176` gegen `tos-api/src/aes/appl.rs:807@appl_tplay`: liefert nur 1.

## Stand Teil 2 (2026-11-05, Researcher)

150 Zeilen fuer die 84 neuen Funktionen von `gemwmlib.c`, `gemgrlib.c`, `gemwrect.c`,
`gemfmlib.c`, `gemfmalt.c`, `gemfslib.c`; 11 vorhandene gelesen und belassen (zu `gr_dragbox`
eine zweite Zeile `graf_dragbox`). `--coverage` aes danach 195 mapped / 0 design / 223 unmapped.
Je Funktion 79 `mapped`, 5 `unmapped` (`w_nilit`, `w_obadd`, `or_start`, `w_redraw_desktop`,
`eralert`). Nur die `CONF_WITH_3D_OBJECTS`-Zweige gemappt.

Weitere Kandidaten (nicht gemessen; EmuTOS-Zeilen in `aes/`):

21. `gemwmlib.c:1222,887-907` gegen `tos-api/src/aes/window.rs:1859@send_redraw` (`wind_open`): `WM_REDRAW`
    nur mit dem Arbeitsbereich statt Schaden ∩ Fensterhuelle (Rahmen, Schatten).
22. `gemsuper.c:358-360` gegen `tos-api/src/aes/window.rs:4755@wind_new`: zusaetzliches `redraw_all`.
23. `gemgrlib.c:243-267` gegen `tos-api/src/aes/graf.rs:237@graf_dragbox`: eigene Schleife,
    Greifversatz ohne `max(.., 0)`, `clamp` statt `rc_constrain`; ebenso `graf_rubberbox`.
24. `gemctrl.c:305` gegen `drag_elevator` (`tos-api/src/aes/window.rs` Zeile 5516 auf eae13cec): Fenster-Schieber folgt live statt
    XOR-Umriss (`gr_slidebox`) — Spur fuer [A130](A130.md) (#2); mit A130 behoben (`tos-api/src/aes/window.rs:5592@slide_elevator`).
25. `gemgrlib.c:127-152,271-280` gegen `graf.rs:286@animate`: XOR-Spur ohne Pause (zu 1).
26. `gemwmlib.c:1719-1726,1754-1755` gegen `window.rs`: `WF_COLOR` zeichnet den ganzen Rahmen
    (Klasse von [A119](A119.md)); Schattenfarbe fest `DROP_SHADOW_COLOR` statt aus dem
    W_BOX-Farbwort — moegliche Spur fuer [A131](A131.md) (#7); der Schattenteil ist mit A131
    gemessen und behoben (2026-11-05, `DROP_SHADOW_COLOR` entfernt), der Rahmenteil ungemessen.
27. `gemwmlib.c:431-434` gegen `tos-api/src/aes/window.rs:3010@thumb_geom`: zusaetzliche Kappung der Schieberlaenge.
28. `gemfmalt.c:188-282` gegen `tos-api/src/aes/form.rs:191@padx` (`do_alert`): Alert-Layout in Pixeln statt
    Zeichenzellen, Maus/`backgrcol` fehlen, eigene Modalschleife statt `fm_do`.
29. `gemfmlib.c:163-198` gegen `form.rs:705@fm_keybd`: Teilbyte- statt Wortvergleich (wie A122).
30. `gemfslib.c:851-865,590-597` gegen `tos-api/src/aes/fsel.rs:1387@FS1STDRV` (`modal`),
    `fsel.rs:355@set_drive_states`: Laufwerksklick und Laufwerk aus dem Pfad — zu [A134](A134.md).
31. `gemfslib.c:640-679,905`: Dateiauswahl ohne Fehler-/Retry-Zweig, Sanduhr, `FSOK`-SELECTED,
    `ALFSMEM` — zu [A134](A134.md).
32. `gemsuper.c:415` gegen `tos-api/src/aes/mod.rs:989@ALNOFUNC`: `fm_show(ALNOFUNC)` fehlt (als Referenzebene entschieden, D-084).

## Stand Teil 3 (2026-11-05, Researcher)

92 Zeilen fuer die 86 neuen Funktionen von `gemmnlib.c`, `gemmnext.c`, `gemrslib.c`,
`gemshlib.c`, `gem_rsc.c`, `gemsclib.c`; `mn_do` (A127) und `handle_popup` (A126) gelesen und
belassen. `--coverage` aes danach 277 mapped / 0 design / 141 unmapped. Je Funktion 82 `mapped`,
4 `unmapped` (`rect_change`, `setup_moblk`: Zeiger-Polling statt MOBLK-Waits;
`xlate_obj_array`; `sh_path`).

Weitere Kandidaten (nicht gemessen; EmuTOS-Zeilen in `aes/`):

33. `gemmnlib.c:422` gegen `tos-api/src/aes/menu.rs:621@child_at_t` (`track_owned`): `ob_find(tree, cur_menu, 1, …)`
    liefert im Rand/in der Luecke der Drop-down-Box die Box selbst (`cur_item = cur_menu`),
    NewTOS `child_at_t` liefert NIL (Niemandsland); fuer Untermenues nachgebaut, fuer die
    Elternbox nicht — Spur fuer [A127](A127.md) D2/D3 (#19), nicht bewiesen. `menu_sr`/`bb_save`/
    `bb_restore` und `rehilite` ohne Befund.
34. `gemmnext.c:258` gegen `tos-api/src/aes/menu.rs:2113@popup_shown`: Start-Item des Popups nicht vorab SELECTED.
35. `gemmnext.c:625-677` gegen `tos-api/src/aes/menu.rs:935@open_submenu`: Untermenue-Lage pixelgenau statt in
    Zeichenschritten mit `MENU_THICKNESS`.
36. `gemmnlib.c:751-756` gegen `tos-api/src/aes/menu.rs:1718@mn_cleanup`: Untermenue-Tabelle ueberlebt das
    Programmende.
37. `gemmnext.c:401-432` gegen `tos-api/src/aes/menu.rs:2451@query_submenu`: Flag-/Markerpruefung, `mn_scroll`,
    geteilte SMIB-Eintraege.
38. `gemmnlib.c:703-731` gegen `tos-api/src/aes/menu.rs:345@menu_bar`: Balken wird weiss gefuellt, beim Ausblenden
    weiss statt nichts; `post_keybd(ctl_pd->p_cda, 0)` fehlt.
39. `gemshlib.c:445-460` gegen `tos-api/src/aes/shell.rs:1131@PATH` (`sh_find`): `PATH=`-Suche fehlt,
    Existenztest ohne Fsfirst (wirkt auf `rsrc_load`).
40. `gemrslib.c:826-848`: klassische `.RSC` nicht ladbar — ADR-013, gewollt.
41. `gemsclib.c:74-107` gegen `tos-api/src/aes/shell.rs:274@scrp_clear`: Momentaufnahme statt Loeschen in der Schleife.

## Stand Teil 4 und Abschluss (2026-11-05)

Researcher: 132 Zeilen fuer die 123 Funktionen der Ereignis-, Prozess- und Startdateien;
6 vorhandene gelesen und belassen, zu `ev_multi`, `mwait`, `acancel` je eine Zeile fuer das
allgemeine Gegenstueck. 103 `mapped`, 9 `design` (6 `hardware:m68k-cpu`, `takeerr`/`giveerr`
ADR-031 `etv_critic`, `switchto` ADR-029), 20 `unmapped` (Tastenwarteschlange `dq/nq/fq/kchange/
post_keybd`, EVB-Listen, Fork-Ring und Tape, Teile von Steuerung und Start).
`--coverage` aes: 365 mapped / 9 design / 44 unmapped, keine Funktion ohne Zeile.
Lead: `"aes/"` in `MAPPED_DIRS`; Gegenprobe ohne `dq` meldet "no row"; `cargo xtask test` 1974/0/1.
Knopf/Doppelklick (`b_click`, `b_delay`, `bchange`, `gl_bpend`) ohne Befund.

Weitere Kandidaten (nicht gemessen; EmuTOS-Zeilen in `aes/`):

42. `gemctrl.c:125-136` gegen `tos-api/src/aes/window.rs` Zeile 5934 auf eae13cec (`wait_release` im Pfeil-Arm): Pfeil-Gadget sendet
    `WM_ARROWED` einmal nach dem Loslassen statt wiederholt (A109 als gewollt vermerkt); mit A130 behoben (`tos-api/src/aes/window.rs:5668@arrow_repeat_busy`).
43. `gemctrl.c:405,415` gegen `tos-api/src/aes/input.rs:754@ct_mouse`: beim Menue-Tracking kein
    Pfeil-Zeiger und keine Rueckstellung der Zeigerform.
44. `gemsuper.c:461-462`, `gemdosif.S:223-225` gegen `kernel/src/arch/aarch64/exceptions.rs:579@gpr`:
    AES-Selektor 201 (`dsptch`) nicht bedient.
45. `gemdisp.c:158-177` gegen `tos-api/src/aes/mod.rs:849@chkkbd`: Tasten nicht in die Warteschlange
    des Mausbesitzers (`forkq(kchange)`), bleiben im Konsolen-Ring; betrifft auch `kstate`.
46. `gemaplib.c:119,189` gegen `tos-api/src/aes/appl.rs:807@appl_tplay`: `appl_tplay`/`appl_trecord`
    ohne Wirkung (A45, wie 20).
47. `gemevlib.c:102-113` gegen `tos-api/src/aes/event.rs:924@evnt_mesag`: `p_msg`-Weg fehlt (zu 42).

## Abnahme

- `cargo xtask deps --check-mapping` ohne Befund; `--coverage` zeigt fuer
  `aes` keine Funktion ohne Zeile.
- `MAPPED_DIRS` enthaelt `aes/`, Hosttests gruen.
- Liste der beim Lesen gefundenen Abweichungen im Ticket.

[Gemeinsame Regeln](README.md) gelten.
