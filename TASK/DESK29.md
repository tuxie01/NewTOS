# DESK29 — Eine Auswahl ueberlebt keinen Fensterwechsel

Status: OBSOLETE (2026-11-11) | Priority: P3 | Reason: withdrawn 2026-09-24: EmuTOS-conformant, EmuTOS deletes unconditionally at two places

Migrated (owner batch 7, 2026-11-11, to OBSOLETE), evidence: TASK/DESK29.md status line (ZURUECKGEZOGEN (2026-09-24, EmuTOS-konform)), TASK/DESK29-STATUS-DONE.MD exists

Line 3 before the migration:
Status: **ZURUECKGEZOGEN** (2026-09-24, EmuTOS-konform) · Prioritaet: **P3** · Typ: Verhaltensabweichung — Nebenbefund DESK18

Abhaengigkeiten: —

## Sachverhalt

`main::top_msg` ruft `desk_clear(g_cwin)`, wenn ein anderes Fenster nach oben
kommt. Damit ist jede Auswahl beim Fensterwechsel weg, und EmuTOS' Fall
„etwas in einem **anderen** Fenster auswaehlen, dann `File → Open`" ist hier
nicht erreichbar.

Gefunden bei DESK18, wo er die zweite Stellung von *Default dir*
unerreichbar macht: EmuTOS' `do_aopen` waehlt das Verzeichnis des **obersten**
Fensters, wenn `G.g_appdir` aus ist — was voraussetzt, dass die Auswahl aus
einem anderen Fenster stammen kann.

Zu klaeren an der Quelle: wann genau loescht EmuTOS die Auswahl beim Toppen
(`desk/deskmain.c` `hndl_msg`s `WM_TOPPED`-Arm, `desk/desksupp.c`
`desk_clear`), und ob unser unbedingtes Loeschen eine Vereinfachung ist.

## Arbeit

Quelle lesen, die Bedingung angleichen, Abnahme mit zwei offenen Fenstern.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-24)

**Zurueckgezogen — EmuTOS loescht unbedingt, an zwei Stellen.** Kein Patch.

* `hndl_msg`s `WM_TOPPED`-Arm (`deskmain.c:1282-1292`): `desk_clear(G.g_cwin)`
  **zuerst**, dann `wind_set(WF_TOP)`, `win_top`, `desk_verify(…, FALSE)`.
  `G.g_cwin` ist dort noch das alte Fenster — gesetzt wird es erst in
  `desk_verify` (`desksupp.c:196`).
* `hndl_button` (`deskmain.c:957-959`): `if (wh != G.g_cwin) desk_clear_all();`
  — jeder Klick in ein anderes Fenster loescht die Auswahl **aller** Fenster
  und die des Desktops.

Unser `main::top_msg` und `main::hndl_button` sind beides wortgleich. Eine
Auswahl kann also auch auf EmuTOS keinen Fensterwechsel ueberleben.

Die zweite Stellung von *Default dir* braucht das auch nicht: `do_aopen` nimmt
das Verzeichnis von `win_ontop()` (`desksupp.c:878, 898-901`) — das **oberste
Fenster**, unabhaengig davon, was ausgewaehlt ist. Erreichbar ist der Fall,
sobald das geoeffnete Objekt nicht im obersten Fenster liegt: ein
Desktop-Icon bei offenem Fenster (`G.g_cwin == DESKWH`, `desksupp.c:1205`),
oder ein Ziehen mit einem anderen Fenster als Ziel (`fun_drag` → `do_aopen`,
`deskfun.c:967,1034,1249`). Die DESK18-Begruendung „unerreichbar" war eine
Fehllesung und ist damit widerlegt.

* Beleg: `measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/` §5,
  `screendumps/d29-D-selected.png` (Auswahl im oberen Fenster, Infozeile
  „64 bytes used in 1 selected items.") gegen `d29-F-win2-back.png` (nach dem
  Fensterwechsel keine Auswahl, „424568 bytes used in 28 items."). Die
  AES-Spur zeigt die EmuTOS-Reihenfolge: `wind_set(2, WF_INFO)` — das ist
  `desk_clear`s `win_sinfo` (`desksupp.c:148`) — vor `wind_set(1, WF_TOP)`.
* Spezifikation: `docs/api/desktop/deskmain.md`, Abschnitt *NewTOS
  implementation notes — a selection does not survive a window change
  (DESK29)*.
* Offen: Hardwarebestaetigung, Punkt **58** in
  `docs/api/desktop/_hw-test-2026-09-17.md`.
