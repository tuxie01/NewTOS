# TOOL33 — `mapping.tsv` fuer die AES-Ebene (EmuTOS `aes/` → NewTOS)

Status: **OFFEN** · Prioritaet: **P1** (Anwender 2026-11-05: nach [TOOL32](TOOL32.md) VDI, vor [A119](A119.md)) · Typ: Researcher (Zuordnung lesen), danach Lead (Pruefregel) — Werkzeug, das die Lokalisierung der folgenden Fixes braucht (CLAUDE.md "Testing")

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
16. `gemgsxif.c:98-105` gegen `tos-api/src/aes/gsxif.rs:346@gsx_mcalc`: Puffergroesse ganzer
    Schirm statt `MENU_BUFFER_SIZE`-Formel (sichtbar ueber `wind_get(WF_SCREEN)`).
17. `gemgsxif.c:440-446` gegen `tos-api/src/vdi/mouse.rs:855@hide`: AES verbirgt die Maus bei
    jedem Aufruf statt nur beim ersten (`gl_moff`).
18. `gemgraf.c:413-416` gegen `graf.rs`: `gl_wbox` fest 19 statt Formel.
19. `gemgsxif.c:238-252` gegen `tos-api/src/aes/gsxif.rs:252@gsx_graphic`: Mausvektoren werden nicht
    zurueckgestellt.
20. `gemaplib.c:176` gegen `tos-api/src/aes/appl.rs:807@appl_tplay`: liefert nur 1.

## Abnahme

- `cargo xtask deps --check-mapping` ohne Befund; `--coverage` zeigt fuer
  `aes` keine Funktion ohne Zeile.
- `MAPPED_DIRS` enthaelt `aes/`, Hosttests gruen.
- Liste der beim Lesen gefundenen Abweichungen im Ticket.

[Gemeinsame Regeln](README.md) gelten.
