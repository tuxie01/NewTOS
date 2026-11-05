# TOOL32 — `mapping.tsv` fuer die VDI-Ebene (EmuTOS `vdi/` → NewTOS)

Status: **OFFEN** · Prioritaet: **P1** (Anwender 2026-11-05: vor [A119](A119.md), VDI vor AES, weil sie ueber dem BIOS liegt) · Typ: Researcher (Zuordnung lesen), danach Lead (Pruefregel) — Werkzeug, das die Lokalisierung der folgenden Fixes braucht (CLAUDE.md "Testing")

## Sachverhalt

`cargo xtask deps --coverage` am 2026-11-05: von den 250 EmuTOS-Funktionen
der Ebene `vdi` haben 2 eine Zeile in `docs/analysis/deps/mapping.tsv`, 248
keine. Die Zuordnung wurde am 2026-10-27 von Hand nur fuer `bios/` und `bdos/`
angelegt (TOOL12); die Pruefregel verlangt nur dort eine Zeile je Funktion
(`MAPPED_DIRS`, `tools/xtask/src/deps.rs:63@MAPPED_DIRS`). Fuer die VDI zeigt `deps --up`
deshalb nur die EmuTOS-Seite. Der Graph selbst ist aktuell
(`tools/deps/gen-deps.py --check` byte-gleich am 2026-11-05).

Funktionen je Datei (`def`-Zeilen von `docs/analysis/deps/edges.tsv`):
`vdi_line.c` 42, `vdi_fill.c` 29, `vdi_text.c` 27, `vdi_mouse.c` 25,
`vdi_esc.c` 25, `vdi_col.c` 19, `vdi_textblit.c` 16, `vdi_raster.c` 15,
`vdi_control.c` 13, `vdi_input.c` 8, `vdi_gdp.c` 8, `vdi_misc.c` 7,
`vdi_asm.S` 7, `vdi_marker.c` 5, `vdi_tblit.S`/`vdi_main.c`/`vdi_inline.h`/`vdi_blit.S` je 1.

## Auftrag

Jede `vdi/`-Funktion bekommt eine Zeile, wie bei `bios/`/`bdos/`
(Format und Klassen: `tools/deps/README.md`, "Coverage and the ratchet" und
"Checks"): beide Quellen lesen, n:m erlaubt, `mapped` mit NewTOS-Datei und
-Funktion (`tos-api/src/vdi/`, `kernel/src/vdi_backend.rs` u. a.), `design`
nur mit ADR- oder Hardware-Beleg, sonst `unmapped` mit Begruendung.
Abweichungen, die beim Lesen auffallen, werden nicht behoben, sondern als
Liste im Bericht gemeldet (Kandidaten fuer neue Tickets).

In drei Auftraegen nacheinander, je ein frischer Researcher:

1. **Zeichnen:** `vdi_line.c`, `vdi_fill.c`, `vdi_gdp.c`, `vdi_marker.c`,
   `vdi_raster.c`, `vdi_asm.S`, `vdi_blit.S`, `vdi_tblit.S`, `vdi_inline.h` (109).
2. **Text und Farbe:** `vdi_text.c`, `vdi_textblit.c`, `vdi_esc.c`, `vdi_col.c` (87).
3. **Steuerung und Eingabe:** `vdi_mouse.c`, `vdi_control.c`, `vdi_input.c`,
   `vdi_misc.c`, `vdi_main.c` (54; die 2 vorhandenen Zeilen pruefen).

Danach (Lead): `"vdi/"` in `MAPPED_DIRS`, `cargo xtask test` gruen.

## Abnahme

- `cargo xtask deps --check-mapping` ohne Befund; `--coverage` zeigt fuer
  `vdi` keine Funktion ohne Zeile.
- `MAPPED_DIRS` enthaelt `vdi/`, Hosttests gruen.
- Liste der beim Lesen gefundenen Abweichungen im Ticket.

## Stand Teil 1 (2026-11-05, Researcher)

123 Zeilen fuer die 109 Funktionen der Zeichendateien (`docs/analysis/deps/mapping.tsv`),
`--check-mapping` sauber, `--coverage` vdi danach 84 mapped / 22 design / 144 ohne Zeile.
Klassen je Funktion: 82 `mapped`, 22 `design` (12 `ADR-030` Line-A, 4 `hardware:blitter`,
1 `hardware:ikbd-acia` `wheel_int`, 5 `ADR-024`), 5 `unmapped`.
Lead-Entscheidung: die reinen Bildschirm-Planarpfade `draw_line`, `vertical_line`,
`swblit_rect_common`, `draw_rect_setup`, `get_color` auf `design:ADR-024` (die Workstation
hat immer 16 Ebenen, `docs/MEMORY.md` ADR-024); `bit_blt`, `fast_bit_blt`, `setup_pattern`
bleiben `unmapped`, weil EmuTOS sie auch fuer Speicher-nach-Speicher-Kopien mit planaren
MFDBs nimmt (Kandidat 7). Ebenfalls `unmapped`: `clip_code`, `crunch_queue`.

Abweichungs-Kandidaten aus dem Lesen (nicht gemessen; EmuTOS-Zeilen in `vdi/`):

1. `vdi_line.c:2205-2228` gegen `kernel/src/vdi_backend.rs:578@abline`: XOR-Letztpunkt-Kuerzung
   auch bei vertikalen Segmenten (EmuTOS verzweigt Vertikallinien vorher ab).
2. `vdi_line.c:1057,1101` gegen `abline`: EmuTOS clippt das Segment vorab, NewTOS pro Pixel
   (Strichphase/Rundung); NewTOS clippt auch bei `vs_clip(0)`.
3. `vdi_line.c:1623` gegen `abline`: anderer Bresenham-Fehlerterm, Gleichheit fuer alle
   Steigungen nicht gezeigt.
4. `vdi_fill.c:560` gegen `tos-api/src/vdi/primitives.rs:707@v_fillarea`: bei 1–2 Punkten zeichnet
   EmuTOS mit `fill_per` den Umriss, NewTOS nichts.
5. `vdi_asm.S:84-95` gegen `primitives.rs:788@v_pmarker`: Kappung von `CONTRL[1]` auf
   `MAX_VERTICES` fehlt ausser in `v_pline`/`v_fillarea`.
6. `vdi_fill.c:149` gegen `tos-api/src/vdi/attributes.rs:512@vsf_udpat`: NewTOS speichert nur 16
   Woerter statt `INQ_TAB[4]*16`.
7. `vdi_raster.c:1331,588` gegen `tos-api/src/vdi/raster.rs:638@vro_cpyfm`: Speicher-nach-Speicher
   mit gleich vielen Ebenen <= 8: EmuTOS planarer BitBlt, NewTOS 16-Bit-Chunky; PAT_FLAG fehlt.
8. `vdi_fill.c:888` gegen `tos-api/src/vdi/contour.rs:153@v_contourfill`: Seed-Warteschlange gegen
   Seed-Stack + Visited-Bitmap; Muster, Ueberlauf, Reihenfolge nicht gezeigt.
9. `vdi_asm.S:496` gegen `tos-api/src/vdi/mouse.rs:190@feed_relative`: EmuTOS zeichnet den Zeiger
   im VBL, NewTOS sofort.
10. `vdi_fill.c:671` gegen `tos-api/src/vdi/inquiry.rs:183@v_get_pixel`: ausserhalb des Schirms.
11. `vdi_gdp.c:386-392` gegen `tos-api/src/vdi/gdp.rs:162@curve`: frueher Ausstieg fehlt, wohl
    nicht beobachtbar.
(12. `call_user_but`: ADR-031, bekannt.)

## Stand Teil 2 (2026-11-05, Researcher)

97 Zeilen fuer die 87 Funktionen von `vdi_text.c`, `vdi_textblit.c`, `vdi_esc.c`, `vdi_col.c`;
`--check-mapping` sauber, `--coverage` vdi danach 159 mapped / 32 design / 59 unmapped
(davon 54 ohne Zeile = Teil 3). Je Funktion 75 `mapped`, 10 `design` (8 `hardware:video-shifter`,
2 `hardware:videl`), 2 `unmapped` (`ok_for_direct_blit`, `trnsfont`). `screen_blit` und
`direct_screen_blit` `mapped` auf `draw_glyph_sized` (enthalten auch den 16-Bit-Zweig).

Weitere Kandidaten (nicht gemessen; EmuTOS-Zeilen in `vdi/`):

13. `vdi_text.c:1292-1350` gegen `tos-api/src/vdi/attributes.rs:351@vst_load_fonts`: GDOS-Fontkette
    (`CONF_WITH_GDOS=1`) fehlt, NewTOS liefert immer 0.
14. `vdi_textblit.c:672-741` gegen `tos-api/src/vdi/text.rs:71@resize`: bei `dda_inc == 0xFFFF`
    driftet EmuTOS' `XDDA`, NewTOS liefert absichtlich genau `2*size` (Kommentar im Code).
15. `vdi_text.c:770-817` gegen `text.rs:103@scaled_header` und `attributes.rs:171@vst_font`:
    `point*2` im skalierten Header, `vst_font` waehlt die Groesse in NewTOS nicht neu.
16. `vdi_text.c:1177-1289` gegen `text.rs:810@v_justified`: Drehung 900/1800/2700 fehlt.
17. `vdi_esc.c:100-107` gegen `tos-api/src/vdi/escape.rs:793@v_escape`: ESC Y (Spalten > 223,
    Byte-Abschneiden) anders, als Entscheidung Q6 im Code.
18. `vdi_esc.c:241-247` gegen `escape.rs:793@v_escape`: Escape 17 ruft `Scrdmp()` nicht; `dump_vec`
    nicht verfolgt.
19. `vdi_text.c:592-665` gegen `tos-api/src/vdi/font.rs:250@DEFAULT`: Default-Font fest 8x16 statt
    nach `V_REZ_VT >= 400`.

Folgeticket: [TOOL33](TOOL33.md) (AES).

[Gemeinsame Regeln](README.md) gelten.
