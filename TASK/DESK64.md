# DESK64 — Hintergrund-Dialog: zeigt EmuTOS eine Vorschau von Farbe und Muster?

Status: DONE (2026-11-11) | Priority: P3 | Commit: 0e95dcedc | Receipt: DESK64-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/DESK64-STATUS-DONE.MD exists; commit 0e95dcedc; TASK/DESK64-STATUS-DONE.MD, commit 0e95dcedc (feat(desktop): Set background is EmuTOS ADBKGND with live sample (DESK), fix/feat commits naming the id: 0e95dcedc (n=1), measurements/measurement-2026-10-17-desk64/

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-17, auf echter Pi 4 abgenommen · Prioritaet: **P3** · Typ: Konformitaetsfrage — Befund des Anwenders auf echter Pi 4 (2026-10-16)

## Beobachtung

Beim Setzen des Desktop-Hintergrunds erscheint ein Dialog, in dem man Farbe
und Muster waehlt. NewTOS zeigt dabei **keine Vorschau** des Ergebnisses.
Der Anwender erinnert sich, dass EmuTOS dort eine Vorschau hat.

Ob das eine Abweichung ist, ist **nicht** belegt — erst die Quelle lesen.

## Auftrag (Researcher, keine Aenderung am Produktcode)

1. EmuTOS: den Dialog finden (vermutlich `desk/deskins.c` /
   `desk/deskmain.c`, Ressource in `desk/desk_rsc.c` bzw. der `.rsc`-Quelle)
   und belegen, ob und wie er eine Vorschau zeigt (welches Objekt, welche
   Funktion aktualisiert es bei Auswahl, `CONF_WITH_*`-Schalter im
   Falcon/TOS4-Profil, `docs/api/_emutos-config.md`).
2. NewTOS: denselben Dialog in `programs/desktop/` und den Ressourcen
   gegenueberstellen (Datei:Zeile beider Seiten).
3. Wenn moeglich die Referenz sichtbar machen (Hatari Falcon truecolor,
   `rscview` fuer die Dialogressource) und mit einem QEMU-Screendump von
   NewTOS vergleichen.
4. Ergebnis: Abweichung ja/nein mit Beleg; falls ja, Ursache (fehlender
   Code, fehlendes Objekt, Renderfehler in AES/VDI) und Fix-Vorschlag.

Quellen: `docs/api/desktop/`, `docs/analysis/`, EmuTOS unter `references/`.

[Gemeinsame Regeln](README.md) gelten.

## Ergebnis (2026-10-16, Researcher)

**Abweichung: ja.** EmuTOS hat `BGSAMPLE` (`desk_rsc.c:2334`) und zeichnet es pro Auswahl per `draw_fld` neu (`deskinf.c:866-891`). NewTOS baut einen eigenen Stepper-Dialog ohne Vorschau, Swatches und Window-Umschalter (`programs/desktop/src/inf.rs:3699-3808`). Ursache im Desktop, nicht in AES/VDI. Kein Bildbeleg erzeugt (nur Quelle). Details und Fix-Vorschlag: `docs/analysis/desk64-background-preview-2026-10-16.md`.

## Abschluss — 2026-10-17

Status: **ERLEDIGT (QEMU), committet (`9172349c`).** Auftrag (Anwender
2026-10-17): Dialog 1:1 nach EmuTOS bauen. `inf::inf_backgrounds` baut
`ADBKGND` Objekt fuer Objekt (`desk_rsc.c:2230-2452`, 3D-Flags wie dort) und
faehrt die Schleife aus `deskinf.c:825-921`: `draw_fld(BGSAMPLE)` vor jedem
`form_do`, Muster/Farbe aus dem angeklickten Feld, `HIDETREE` je
`gl_nplanes`. Fensterhintergrund wird angewendet (Fensterboxen `G_BOX` mit
`g_patcol.window`, `BACKGROUND_HAS_CHANGED` → `desk_all`). Das sofortige
Speichern bleibt als benannte Abweichung (Anwenderentscheidung, verallgemeinert
in [DESK65](DESK65.md)). Offen: `set_aes_background` (AES-intern, kein Aufruf
fuer das Programm NEWDESK); Muster 6 der AES weicht von EmuTOS ab (Nebenfund).
Quittung: [DESK64-STATUS-DONE.MD](DESK64-STATUS-DONE.MD); Belege
`measurements/measurement-2026-10-17-desk64/RUNINFO.TXT`.

## Anwenderentscheidungen zu den Restpunkten (2026-10-17)

- `set_aes_background` (`deskinf.c:904`): eigener Weg, Ticket [DESK66](DESK66.md).
- Nebenfund `IP_PATTERNS[6]` (`tos-api/src/aes/object.rs`) passt nicht zu
  EmuTOS (Hatari zeigt andere Zeilen): **geparkt** — Anwender: *„ist doch
  egal welche Füllmuster. Ich denke die meisten nehmen eh nur diese 25%
  einfärbung“*. Kein Ticket.

## Pi-Abnahme (2026-10-17)

Kern aus `c93d89e0` (sha256 `715462e5…`). Anwender auf echter Pi 4: Dialog wie
EmuTOS, Vorschau folgt Muster- und Farbklick, Desktop- und Fensterhintergrund
nach OK gesetzt (auch fuer neu geoeffnete Fenster), nach Neustart erhalten —
*„alles funktioniert wie gewünscht“*.
