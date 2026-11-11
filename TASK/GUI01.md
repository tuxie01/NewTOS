# GUI01 — `CONF_WITH_SHOW_FILE`: Datei per Doppelklick anzeigen

Status: PARKED (2026-11-11) | Priority: P4 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 17, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 37b42d622 (docs(compat): name the missing built-in file viewe), named by subject

Line 3 before the migration:
Status: **GESCHLOSSEN 2026-10-22** (Anwenderentscheid 2026-10-18: Betrachter als Anwendung + Zuordnung, kein eingebautes `show_file`; Abweichung benannt in `docs/COMPATIBILITY.md`) · Prioritaet: **P4** · Typ: Funktionsluecke AES/Desktop — aus der Schaltertabelle (REF01)

Abhaengigkeiten: —

## Sachverhalt

`CONF_WITH_SHOW_FILE` ist im Referenzbau **1**
(`docs/api/_emutos-config.md`): EmuDesk zeigt den Inhalt einer Datei, die
keiner Anwendung zugeordnet ist, in einer eingebauten Ansicht — Doppelklick
genuegt. Beschreibung des Schalters: `references/emutos/include/config.h:1838-1844`.

In `kernel/src/desk/` findet sich dazu nichts (geprueft 2026-09-24). Ein
Doppelklick auf eine gewoehnliche Datei ist bei uns ein No-op.

## Arbeit

Die EmuTOS-Stelle lesen (Aufrufer in `desk/deskmain.c`/`desk/desksupp.c`,
die Ansicht selbst), dann nachbauen. Die Ansicht haengt mit
[GUI05](GUI05.md) (`CONF_WITH_VIEWER_SUPPORT`) zusammen: dort geht es um
einen **externen** Betrachter, hier um die **eingebaute** Anzeige — erst an
der Quelle klaeren, welche von beiden EmuTOS zuerst versucht.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Zurueckgestellt — Lead-Entscheidung 2026-09-24

**Nicht als eingebaute Funktion bauen.** Der Benutzer haelt den Weg ueber eine
**Anwendung plus Dateizuordnung** fuer flexibler als eine fest eingebaute
Ansicht: wer eine Datei ansehen will, hinterlegt einen Betrachter und ordnet
ihm die Endung zu — dann gilt dieselbe Mechanik wie fuer jedes andere
Dokument, und der Betrachter ist austauschbar.

Das Ticket bleibt stehen, weil der Schalter in der Referenzkonfiguration an
ist und die Lueckenliste vollstaendig sein soll; es ist aber **kein Auftrag**,
solange die Zuordnungsmechanik den Fall abdeckt. Vor einer Wiederaufnahme
zuerst pruefen, ob eine Zuordnung auf einen `.NAP`-Betrachter heute schon
genuegt — wenn ja, ist dieses Ticket gegenstandslos und wird geschlossen.

## Bestaetigt (2026-10-18)

Anwender: *„es macht Sinn, eine App als Betrachter zu haben, damit ist man
etwas flexibler“* — weiter kein eingebautes `show_file`; Betrachter ueber die
Anwendungs-Zuordnung (z. B. `.TXT` → QED). EmuCON-`more` → [CON03](CON03.md).
