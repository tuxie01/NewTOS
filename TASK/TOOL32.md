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

Folgeticket: [TOOL33](TOOL33.md) (AES).

[Gemeinsame Regeln](README.md) gelten.
