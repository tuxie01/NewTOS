# PRESENT01 — PRESENT.NAP: Folien-Präsentation im Vollbild (eigene GEM-Anwendung)

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt PRESENT01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: only the commit 0bda4b9a3 (feat(sdk): PRESENT.NAP, a full-screen slide show f) that added the receipt itself

Line 3 before the migration:
Status: **ERLEDIGT (QEMU), 2026-10-08, ein offener Punkt (Bild auf/ab, Kernel-Tastaturtabelle)** — [Quittung](PRESENT01-STATUS-DONE.MD) · Priorität: **P1** (Vortrag Atari-Treffen, Samstag) · Typ: NewTOS-Anwendung (nur SDK/Werkzeugkette) — Anwender 2026-10-08

Abhängigkeiten: SDK (gemlib, mintlib, [libpng/zlib aus LDG-M2](../docs/sdk/11-codec-libraries.md)),
VDI-Speicher-MFDB-Format RGB565 (`tos-api/src/vdi/raster.rs`), AES `evnt_multi`,
`form_dial`, `wind_update`, `graf_mouse`.

## Anforderungen des Anwenders (2026-10-08)

* Eine native NewTOS-GEM-Anwendung `PRESENT.NAP` (eigene Software, kein Port),
  normal gebaut wie die SDK-Beispiele (Build-Skript unter `sdk/apps/present/`).
* Folien **englisch**, **Vollbild**, **große Schrift**, **optional ein Bild je Folie**.
* Folientexte aus einer Textdatei auf der Karte, änderbar ohne Neubau:
  `C:\APPS\PRESENT\SLIDES.TXT` (ASCII, LF oder CRLF). Einfaches Format
  (`# ` Titel = neue Folie, `- ` Punkt, `  - ` Unterpunkt, `@image DATEI.PNG`,
  `@note …` ignoriert, Leerzeile = Abstand), dokumentiert im Kopfkommentar und
  in `docs/sdk/`. Bis ~12 Folien, ~10 Punkte je Folie; robust gegen lange
  Zeilen (Umbruch) und fehlende Bilder.
* Vollbild wie eine GEM-Anwendung, der der Bildschirm gehört
  (`wind_update(BEG_UPDATE)` + `form_dial(FMD_START)` o. ä.); beim Ende Desktop
  wiederherstellen (`FMD_FINISH`/Redraw). Maus während der Folien aus
  (`graf_mouse M_OFF`), danach wieder an.
* Große Schrift aus dem einzigen 8x16-Systemfont, selbst skaliert (Titel ~4×,
  Punkte ~2,5–3×), sauber ohne Lücken; Titel oben, Punkte darunter, Bild rechts
  oder darunter; „n / N" unten rechts; stimmiges, kontrastreiches Farbschema
  für den Beamer.
* Bilder: PNG über libpng (statisch), eingepasst, in Truecolor über
  `vro_cpyfm`; fehlendes/kaputtes Bild → kleiner Hinweis, nie ein Absturz.
* Bedienung: Rechts/Runter/Leertaste/Bild ab/Linksklick = weiter;
  Links/Hoch/Backspace/Bild auf/Rechtsklick = zurück; Pos1/Ende = erste/letzte;
  Esc oder Q = Ende. `evnt_multi` mit `MU_KEYBD|MU_BUTTON`.
* Leistung: Folienaufbau bei 1920x1080 auf der Pi deutlich unter einer Sekunde.
* Tests: QEMU raspi4b (`serial=on`) bei 640x480 und 1920x1080, Beispieldatei
  mit 3 Folien inkl. einem PNG und einem fehlenden Bild, Navigation
  vor/zurück/Ende/Beenden über den IKBD-Injektor, Screendumps genau
  angesehen, einmal `memprot=strict`, APITEST unverändert. Kein Kernel-/AES-
  Eingriff — wenn nötig: STOP und melden.
* Lieferung (nicht auf eine Karte kopiert):
  `measurements/measurement-2026-10-08-present/bin/` mit PRESENT.NAP,
  Beispiel-SLIDES.TXT, Testbild, sha256, deutsches LIESMICH.TXT;
  Installationshaken in `tools/make-sd-image.sh` (nur QEMU-Abbild) unter
  `C:\APPS\PRESENT\`.
