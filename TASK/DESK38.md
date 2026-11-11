# DESK38 — `Show info` gibt es auf einem Desktop-Symbol nicht

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK38-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by f7c61f863 (feat(desk): rebuild the Show-info dialogs as EmuTO), named by body

Line 3 before the migration:
Status: **ERLEDIGT** · Prioritaet: **P3** · Typ: Funktionsluecke — Nebenbefund DESK30

Abhaengigkeiten: [DESK30](DESK30.md) (erledigt)

## Sachverhalt

`main::do_info_rename` kehrt sofort um, wenn `G.g_cwin == DESKWH` — auf der
Schreibtischflaeche gibt es also kein *Show info*. Betroffen sind
Laufwerkssymbole, Papierkorb, Drucker und seit DESK30 auch Verknuepfungen.

EmuTOS hat dafuer `do_info` (`references/emutos/desk/desksupp.c:1300-1330`)
mit eigenen Kaesten je Symboltyp — ein Laufwerk zeigt Belegung, der
Papierkorb seinen eigenen Hinweis (`trash_alert`, `:1165-1172`).

Die Luecke ist aelter als DESK30; sie faellt nur jetzt staerker auf, weil es
mehr Symbole gibt, auf die man sie anwenden moechte.

## Arbeit

`do_info` nachziehen, Fall fuer Fall nach der Quelle. Abnahme: *Show info* auf
Laufwerk, Papierkorb, Drucker und einer Verknuepfung, je ein Screendump.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

`do_info` nachgezogen, 2026-09-25, Basis `desk-rebuild` HEAD `12fddc1`; die
Aenderung liegt **uncommitted** im gemeinsamen Baum.

`main::do_info_rename` ist `main::do_showitem` + `main::do_info` geworden:

* **`do_info(curr)`** — der Schalter auf `i_find`s `a_type`
  (`desksupp.c:1288-1354`) mit allen vier Zweigen: `AT_ISFOLD`/`AT_ISFILE`
  (Fenstereintrag **und** Verknuepfung, letztere ueber `file_exists(a_pappl)`,
  mit `remove_locate_shortcut` fuer die Waise), `AT_ISDISK` → `inf_disk`,
  `AT_ISTRSH` → `trash_alert`, `AT_ISPRNT` → `printer_alert`.
* **`do_showitem`** — EmuTOS' `SHOWITEM`-Schleife ueber die Auswahl
  (`deskmain.c:674-689`), `fun_rebld_marked()` dahinter, und der Fall „nichts
  markiert" → `inf_disk` des obersten Fensters.
* **`inf::inf_disk`** — der `ADDISKIN`-Kasten, neu, samt `count_ffs`
  (`dir.rs`), `dos_space`/`dos_label` (`dosbind.rs`, `Dfree` und `Fsfirst`
  mit `FA_VOL`) und `valid_drive` (`main.rs`, mit `STNODRIV`).
* **`inf::inf_file_folder`** gibt jetzt EmuTOS' drei Antworten zurueck
  (`InfFF`), weil *Cancel* die Schleife beenden muss und vorher von „OK, nichts
  geaendert" nicht zu unterscheiden war.

Benannte Abweichungen, beide **aelter** als DESK38 und von ihm nicht
angefasst: EmuTOS benennt in `inf_file_folder` selbst um, in einer Schleife mit
`STRENAME`-*Retry* (`deskinf.c:551,629-634`), und schreibt ein geaendertes
Schreibschutz-Attribut zurueck (`:591-609`); bei uns benennt der Aufrufer
einmal um und die Attributzeile ist nur Anzeige.

Messung: [`measurements/measurement-2026-09-25-desk-rest/`](../measurements/measurement-2026-09-25-desk-rest/)
§3 — Laufwerk, Papierkorb, Drucker, Verknuepfung und Fenstereintrag, je ein
Screendump, alle mit `cwin=0` ausser dem Fenstereintrag. Spezifikation:
[`desksupp-redraw.md`](../docs/api/desktop/desksupp-redraw.md) und
[`deskinf.md`](../docs/api/desktop/deskinf.md). Hardwarepunkt **69** — dort
zaehlt `count_ffs` ueber vier echte Partitionen, und wie lange das dauert ist
der eigentliche Punkt.
