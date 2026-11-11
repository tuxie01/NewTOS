# GUI02 — `CONF_WITH_VIEWER_SUPPORT`: Standardbetrachter fuer nicht zugeordnete Dateien

Status: PARKED (2026-11-11) | Priority: P4 | Reason: deferred by the lead 2026-09-24 (viewer support, depends on GUI01) | Reopen: the owner decides to build default-viewer handover

Migrated (owner batch 10, 2026-11-11), evidence: TASK/GUI02.md status line: ZURUECKGESTELLT (Lead 2026-09-24)

Line 3 before the migration:
Status: **ZURUECKGESTELLT** (Lead 2026-09-24) · Prioritaet: **P4** · Typ: Funktionsluecke AES/Desktop — aus der Schaltertabelle (REF01)

Abhaengigkeiten: [GUI01](GUI01.md)

## Sachverhalt

`CONF_WITH_VIEWER_SUPPORT` ist im Referenzbau **1**
(`references/emutos/include/config.h:1854-1861`): eine Datei ohne passende
Anwendungszuordnung wird an einen eingetragenen **Standardbetrachter**
uebergeben.

Unser eigener Quelltext sagt, dass es das nicht gibt —
`kernel/src/desk/inf.rs:1847-1850`: *„AF_VIEWER is CONF_WITH_VIEWER_SUPPORT,
which this desktop does not implement."* Die `AF_VIEWER`-Flagge wird beim
Einlesen der `NEWDESK.INF` also gelesen und ignoriert.

## Arbeit

`AF_VIEWER` in der Zuordnungsliste auswerten und den Startweg bauen. Reihenfolge
gegen [GUI01](GUI01.md) an der Quelle klaeren: eingebaute Ansicht gegen
externen Betrachter — welcher gewinnt, und was passiert, wenn der eingetragene
Betrachter fehlt.

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
