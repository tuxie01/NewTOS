# DESK21 — `Set background` steht im falschen Menue

Status: PARKED (2026-11-11) | Priority: P4 | Reason: legacy, evidence unverified; receipt DESK21-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 561b99326 (feat(desk): the removable-media icon, the ntrash r), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P4** · Typ: Konformitaet — Nebenbefund DESK05

Abhaengigkeiten: —

## Sachverhalt

`BACKGRND` ist in EmuTOS Objekt **46** und damit das **letzte** Element des
**View**-Menues, hinter `SEP_VW2` (45); das Options-Menue beginnt erst bei
`IICNITEM` (48) — `references/emutos/desk/desk_rsc.h:35-48`.

NewTOS zeigt `Set background...` als **erstes** Element von *Options*
(`kernel/src/desk/menu.rs`, sichtbar in
`measurements/measurement-2026-09-23-desk05/screendumps/conf-C-options.png`).

Betrifft nur die Einordnung im Menue, nicht die Kuerzelzuordnung (die ist in
`shortcut_mapping` semantisch, nicht nach Position).

## Arbeit

Den Punkt ans Ende des View-Menues verschieben, Trennstrich davor wie im
Baum; `shortcut_mapping` und `men_update` entsprechend mitziehen. Screendump
beider Menues vor/nach.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Erledigt am 2026-09-23**, Branch `desk-rebuild`, Basis HEAD `a8d4b42`,
Aenderungen **uncommitted** im Baum. Quittung:
[DESK21-STATUS-DONE.MD](DESK21-STATUS-DONE.MD).

Der Punkt steht jetzt mit seinem Trennstrich am Ende des View-Menues. Belegt
am Baum selbst: Objekt 34 ist der View-Kasten mit `ob_head` 35 und `ob_tail`
**46** (`desk_rsc.c:446`), `SEP_VW2` (45) ist der Trennstrich davor
(`:473-478`), der Options-Kasten ist Objekt 47 und faengt bei `IICNITEM` (48)
an — `desk_rsc.h:35-48`.

Mitgezogen wurde nur, was musste:

* **`shortcut_mapping` nicht** — es ist semantisch, nicht nach Position, und
  traegt `BACKGRND` schon immer als letzten *View*-Platz
  (`deskmain.c:211-217`). Genau der DESK05-Punkt.
* **`men_update` nicht** — EmuTOS nennt `BACKGRND` in keiner `ILL_*`-Tabelle
  und in keinem `menu_ienable`-Aufruf; der Eintrag ist unabhaengig von der
  Auswahl schwarz, vorher wie nachher.
* **Die Verteilung schon** — `hndl_menu` verzweigt nach Titel, und EmuTOS
  bearbeitet `BACKGRND` in `do_viewmenu` (`:823-828`). Der Arm ist von
  `do_optnmenu` dorthin gewandert.
* **Die Kastenbreite auch, und richtig so** — `copy_menu_items` polstert auf
  `max_strlen + 1 + SHORTCUT_SIZE` (`:1781`), `adjust_menu` nimmt die
  gepolsterte Laenge plus eins (`:1608-1620`). Mit `"  Set background..."`
  (19) darin sind das 23 Zeichenzellen; vorher waren es 20. Der Lauf misst
  genau das: `box 2 abs (112,19) 160x160` vorher, `184x192` nachher.

**Eine benannte Abweichung, im Praedikat, nicht in der Einordnung.** EmuTOS'
Arm lautet `if (inf_backgrounds()) rc = BACKGROUND_HAS_CHANGED;`, und
`inf_backgrounds` gibt nur dann TRUE zurueck, wenn sich der **Fenster**-
hintergrund geaendert hat (`deskinf.c:910-916`); eine Aenderung des
*Schreibtisch*-hintergrunds setzt es selbst um, zeichnet mit
`do_wredraw(DESKWH, &G.g_desk)` neu und gibt FALSE zurueck (`:900-908`).
NewTOS' Ersatzkasten bietet nur die Schreibtischhaelfte an — der `#Q`-
Fenstereimer wird zwar durch die Datei gereicht, hat aber keinen Verbraucher
—, und sein `true` meint genau diese Aenderung, die `inf::set_background`
bereits auf demselben Weg neu gezeichnet hat. Das EmuTOS-gleiche `rc` ist
fuer jeden Ausgang dieses Kastens also `0`; `BACKGROUND_HAS_CHANGED` hat
keine NewTOS-Entsprechung zu setzen, und ein `desk_all` wuerde hier jedes
Fenster fuer eine Aenderung neu zeichnen, die keines betrifft. Die
Fensterhintergrund-Haelfte von `ADBKGND` bleibt die Luecke, die sie war.

Beleg: [`measurements/measurement-2026-09-23-desk16-desk17-desk21/`](../measurements/measurement-2026-09-23-desk16-desk17-desk21/),
beide Klappmenues vorher und nachher.

Spezifikation nachgezogen: `docs/api/desktop/deskmain.md` (neuer Abschnitt).
Hardware-Punkt **47** in `docs/api/desktop/_hw-test-2026-09-17.md`.
