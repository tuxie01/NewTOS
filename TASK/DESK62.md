# DESK62 — Sanduhr bleibt nach Drop einer Datei auf eine Desktop-Verknuepfung

Status: DONE (2026-11-11) | Priority: P2 | Commit: b3e095d64 | Receipt: DESK62-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/DESK62-STATUS-DONE.MD exists; commit b3e095d64; TASK/DESK62-STATUS-DONE.MD, commit b3e095d64 (fix(desk): pass exit_desktop back from drop-launches (DESK62)), fix/feat commits naming the id: b3e095d64 (n=1), measurements/measurement-2026-10-16-desk62/

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-16 ([Quittung](DESK62-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Fehler — Befund des Anwenders auf echter Pi 4 (2026-10-16, Abnahme DESK43 U9-6)

## Befund

1. Verknuepfung von `QED.NAP` auf dem Desktop anlegen.
2. Eine Textdatei per Drag & Drop auf diese Verknuepfung ziehen.
3. Die Start-Animation laeuft, danach bleibt der Mauszeiger die **Sanduhr**;
   QED startet **nicht**. Das System laeuft weiter.
   (Korrigiert 2026-10-16: der Anwender hat auf der Pi bestaetigt, dass QED
   nach dem Drop nicht startete — nur Animation und Sanduhr, wie in der
   QEMU-Reproduktion. Die erste Fassung sagte „QED startet".)

Gegenproben des Anwenders:

- Doppelklick auf die Verknuepfung (ohne Datei): funktioniert.
- Datei im Verzeichnisfenster direkt auf `QED.NAP` ziehen: funktioniert.
- Der Fehler bestand schon **vor** DESK43 (eingebauter Desktop), er ist also
  keine Folge des Umzugs nach EL0.

Geklaert (QEMU, 2026-10-16): die Sanduhr bleibt ueber Mausbewegung und Klick;
sie verschwindet erst, wenn der Desktop auf anderem Weg endet (z. B.
Doppelklick auf die Verknuepfung, QED ohne Datei, Rueckkehr). Serielles Log:
`measurements/measurement-2026-10-16-desk62/serial/ds-serial.log`.

## Auftrag

Den Drop-auf-Desktop-Icon-Pfad in EmuTOS lesen (`desk/deskact.c`,
`desk/deskfun.c`, `desk/deskmain.c` — Ziel-Icon ist Desktop-Objekt statt
Fensterobjekt) und mit `programs/desktop/` vergleichen, insbesondere wo
EmuTOS die Maus nach dem Start/der Rueckkehr wieder auf `ARROW` setzt.
Ursache mit Beleg vorlegen (Datei:Zeile beider Seiten) und in QEMU per
Injektor reproduzieren, **bevor** gefixt wird.

[Gemeinsame Regeln](README.md) gelten.

## Abschluss — 2026-10-16

Status: **ERLEDIGT (QEMU), committet.** Ursache (`c5018c19`): `fun_file2desk`
(`AT_ISFILE`) und `fun_desk2win` verwarfen `do_aopen`s Antwort, `fun_drag` gab
`false` — der Desktop blieb nach `shel_write(SHW_EXEC)` unter der Sanduhr von
`pro_exec` in seiner Schleife, der Start lief nie. Fix (`a8b03c81`):
modulweites `EXIT_DESKTOP` wie EmuTOS `deskfun.c:61,967,1034,1249,1304,1338`.
Rot (HEAD) → gruen fuer Drop auf Verknuepfung und Desktop-Symbol auf Programm
im Fenster; Fenster-Drop unveraendert gruen. Quittung:
[DESK62-STATUS-DONE.MD](DESK62-STATUS-DONE.MD); Belege
`measurements/measurement-2026-10-16-desk62/RUNINFO.TXT` (Ursache),
`measurements/measurement-2026-10-16-desk62-fix/RUNINFO.TXT` (Fix),
`docs/analysis/desk62-hourglass-after-drop-on-shortcut-2026-10-16.md`.
**Auf echter Pi 4 abgenommen 2026-10-16** (Kern aus `970b87d8`, sha256 `50278e3b…`): Drop auf Verknuepfung startet QED mit Datei, Pfeil zurueck; Fenster-Drop und Desktop-Symbol ins Fenster gruen. Anwender: *„alles laeuft wie es soll“*.
