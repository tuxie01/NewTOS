# ACC07 — Accessory-Titel im Menuebaum einer Anwendung

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 17, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 05b3376d1 (fix(aes): accessory titles in an app's desk menu a), named by status-line-named

Line 3 before the migration:
Status: **ERLEDIGT 2026-10-07** (QEMU, `05b3376d`: Titelkopien auf gemeinsamer read-only Seite, ADR-023; TeraDesk und QED zeigen die Titel, [Quittung](ACCTITLE-STATUS-DONE.MD), `measurements/measurement-2026-10-07-acctitle/`) · Prioritaet: **P3** · Typ: Funktionsluecke — Nebenbefund ACC04

Abhaengigkeiten: [ACC04](ACC04.md) (erledigt)

## Sachverhalt

`menu_fixup` traegt die registrierten Accessory-Titel in den Desk-Klappkasten
ein. Im Menuebaum des **Desktops** funktioniert das; im Baum einer
**Anwendung** nicht: die Titelkopie liegt im Kernelspeicher, und ein
app-eigener Baum loest jede Adresse ueber die Abbildung der Anwendung auf
(A97). Der Eintrag wird deshalb nicht gezeichnet (`spec_none`) — kein Fehler,
nur unsichtbar.

Beschreibung und Fundstellen: `docs/api/aes/accessories.md` §9.4.

## Arbeit

Eine gemeinsame, **nur lesbare** Titelseite, die in jedem Prozess an
derselben Adresse erreichbar ist — dann stimmt die Aufloesung in beiden
Baeumen. Vorher pruefen, ob das Speichermodell (FreeMiNTs `PROT_*`, siehe
`docs/MEMORY.md`) dafuer schon einen Platz vorsieht.

Abnahme: QED starten, dessen Desk-Menue aufklappen, die Accessory-Titel
stehen darin und oeffnen auf Klick.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
