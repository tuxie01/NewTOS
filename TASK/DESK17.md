# DESK17 — `men_update`: `ntrash`-Regel fehlt; Menuetitel bleibt invertiert

Status: DONE (2026-11-11) | Priority: P4 | Commit: 561b99326 | Receipt: DESK17-STATUS-DONE.MD

Migrated (owner batch 19, 2026-11-11, to DONE), evidence: receipt TASK/DESK17-STATUS-DONE.MD exists, commit 561b99326 (feat(desk): the removable-media icon, the ntrash rule, Set background ), VERIFIED: 561b99326 (feat(desk): the removable-media icon, the ntrash rule, Set b) names the id by body, its message names the ticket-title identifier men_update, touches code (19 files), legacy line: ERLEDIGT (2026-09-23)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-23) · Prioritaet: **P4** · Typ: Konformitaet — Nebenbefunde DESK03

Abhaengigkeiten: [DESK03](DESK03.md) (erledigt)

## Die Stellen

1. **`ntrash` fehlt.** EmuTOS zaehlt in `men_update` neben `ndesk` auch die
   ausgewaehlten Papierkorb- und Druckersymbole und schaltet *Delete* ab,
   sobald eines davon in der Auswahl ist
   (`references/emutos/desk/deskmain.c:533-535`). `kernel/src/desk/menu.rs`
   hat seit DESK03 `ndesk`, aber nicht `ntrash` — *Delete* bleibt schwarz.
2. **Der Menuetitel bleibt invertiert, solange ein Dialog offen ist**
   (sichtbar in `measurements/measurement-2026-09-24-desk03-desk04/screendumps/ico-B-dialog.png`).
   Verhalten von `hndl_menu`, aelter als DESK03: EmuTOS gibt den Titel frei,
   bevor der Dialog laeuft (`menu_tnormal` in `deskmain.c`s `hndl_menu`,
   gegen die NewTOS-Fassung zu pruefen).

## Arbeit

1 ist eine Zaehlung neben der vorhandenen und gehoert in denselben Durchlauf
ueber die Auswahl. 2 zuerst gegen `hndl_menu` belegen (wo genau gibt EmuTOS
den Titel frei), dann nachziehen; ein Screendump vor/nach mit offenem Dialog
ist die Abnahme.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Erledigt am 2026-09-23**, Branch `desk-rebuild`, Basis HEAD `a8d4b42`,
Aenderungen **uncommitted** im Baum. Quittung:
[DESK17-STATUS-DONE.MD](DESK17-STATUS-DONE.MD).

**1 — `ntrash`: gebaut.** `desk::win::count_selected_desk_icons` liefert jetzt
`(ndesk, ntrash)` aus **einem** Durchlauf ueber die Auswahl, wie EmuTOS sie
zaehlt: der `AT_ISPRNT`/`AT_ISTRSH`-Zweig faellt in `AT_ISDISK` durch
(`deskmain.c:508-517`), Papierkorb und Drucker erhoehen also beides, ein
Laufwerk nur `ndesk`. `desk::menu`s `item_enabled` schaltet `Item::Delete` auf
`nsel >= 1 && ntrash == 0` — `ILL_NOSEL` plus die eine zusaetzliche Sperre
(`:533-535`).

**2 — Menuetitel: die Annahme des Tickets ist widerlegt, nichts geaendert.**
EmuTOS gibt den Titel **nicht** vor dem Dialog frei. `hndl_menu` ruft erst den
Bearbeiter und danach `menu_tnormal(desk_rs_trees[ADMENU], title, 1)`
(`deskmain.c:1012-1038`, die Freigabe steht in `:1036`, hinter dem `switch`).
Das ist der klassische GEM-Vertrag: die AES laesst den Titel invertiert, wenn
sie `MN_SELECTED` schickt, und die Anwendung nimmt ihn zurueck, wenn sie mit
der Auswahl fertig ist — genau deshalb darf dazwischen ein modaler Dialog
stehen. Der Tastenweg macht das Spiegelbild (`menu_tnormal(..., title, 0)`
zuerst, `:1240`).

`desk::main::hndl_menu` tut bereits genau das, an genau derselben Stelle;
`desk::main::hndl_kbd` hebt auf dem Kuerzelweg zuerst hervor. Der im Ticket
zitierte Screendump
(`measurements/measurement-2026-09-24-desk03-desk04/screendumps/ico-B-dialog.png`) zeigt
also konformes Verhalten. Trotzdem vorher **und** nachher neu gemessen, mit
offenem Kasten und nach dem Schliessen.

Beleg: [`measurements/measurement-2026-09-23-desk16-desk17-desk21/`](../measurements/measurement-2026-09-23-desk16-desk17-desk21/).
*Delete* mit Papierkorb: vorher schwarz, nachher **grau**; mit Laufwerk
(`ndesk` 1, `ntrash` 0): **schwarz**. *Open* und *Info/Rename...* bleiben in
beiden Nachher-Bildern schwarz, die neue Sperre ist also nicht in `ILL_NOSEL`
ausgelaufen. Dieselben drei Zustaende stehen als `dis=` in den seriellen
Protokollen.

Spezifikation nachgezogen: `docs/api/desktop/deskmain.md` (neuer Abschnitt,
beide Haelften). Hardware-Punkt **46** in
`docs/api/desktop/_hw-test-2026-09-17.md`.
