# DESK09 — Kontextspeicher: `cnx_put`/`cnx_get` (Fenster und Einstellungen ueberleben)

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: f16fef8f0 | Missing: real-Pi visual acceptance

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/DESK09-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit f16fef8f0 (feat(desk): dialogs go through form_center and form_dial, the context ), VERIFIED: f16fef8f0 (feat(desk): dialogs go through form_center and form_dial, th) names the id by body, its message names the ticket-title identifier cnx_get, cnx_put, touches code (68 files), legacy line: ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen** · Prioritaet: **P2** · Typ: Strukturluecke desk/ (G2 der Lueckenliste)

Abhaengigkeiten: —

## Sachverhalt

`docs/analysis/desk-aes-gaps-2026-09-14.md` § G2: es gibt keinen
Kontextspeicher. EmuTOS `cnx_put` (`desk/deskmain.c:1365+`) sichert
Fensterpositionen, Anzeigeart, Sortierung und Einstellungen in den
`.INF`-Satz; `cnx_get` liest sie beim Start zurueck. Bei uns ueberlebt
nichts davon einen Neustart — `Save desktop` schreibt nur einen Teil.

## Arbeit

`cnx_put`/`cnx_get` 1:1 nach der Quelle (welche Felder, welche
`#W`/`#E`-Zeilen), Anbindung an `Save desktop` und an den Startpfad
(`inf.rs`); Hosttest ueber einen `.INF`-Rundlauf, QEMU: Fenster oeffnen,
sichern, neu starten, Fenster steht wieder.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Erledigt am 2026-09-23**, Quittung [DESK09-STATUS-DONE.MD](DESK09-STATUS-DONE.MD),
Artefakte `measurements/measurement-2026-09-23-desk08-desk09/`. Aenderungen uncommitted im
Baum (Branch `desk-rebuild`, Basis `326ef4c`).

`G.g_cnxsave` existiert als `global::CSave`/`WSave` (voller `WSAVE`-Feldsatz);
`main::cnx_put`/`cnx_get` sind 1:1 nach `deskmain.c:1371-1501` gebaut,
einschliesslich der Steckplatzordnung (Platz 0 = unterstes Fenster) und der
Gueltigkeitspruefung gegen das aktuelle `G.g_desk` samt `do_xyfix`.
Verdrahtung: `cnx_get` im Startpfad zwischen `wind_update(BEG_UPDATE)` und
`END_UPDATE` (`:2053-2057`); `cnx_put` an `Save desktop` (`:891`), am Ende von
`hndl_msg` bei `change` (`:1352`) und im Desktop-Epilog (`:2154`).
`inf::apply`/`build_from_state` gehen jetzt beide ueber den Kontextspeicher,
und `app_save` schreibt **alle sieben** `#W`-Zeilen mit `hsl`/`vsl`, wie
EmuTOS.

**Tatsaechlich ausgefuehrte Pruefung.** Ein Hosttest ist hier **nicht
moeglich** — die Logik liegt vollstaendig in `kernel/`, das
`forced-target = "aarch64-unknown-none-softfloat"` traegt, und
`cargo xtask test` faehrt nur `tos-api`/`tos-types`/`xtask`. Der Nachweis ist
in QEMU raspi4b gefuehrt, in vier Laeufen, alle ohne `[EXC]`/`[PANIC]`:
(1) Serienabbild gegen die Vorgaenger-Messung — 50 px, nur der Mauszeiger,
keine Regression; (2) vorgegebene `#W`/`#E`-Zeilen — beide Fenster kommen
gemessen bei (32,80,240,160) und (192,240,320,192) zurueck, in der
gespeicherten Stapelordnung und in **Textansicht** aus der `#E`-Zeile;
(3) Fenster nach (144,141) gezogen, `Options → Save desktop` geklickt, die
geschriebene Datei gelesen: `#W 00 00 12 08 44 19 00 C:\*.*@`;
(4) **Kaltstart auf demselben Abbild mit dem Produktkernel** — das Fenster
steht wieder bei (144,128). Die 13 px sind die Zeichenzellen-Speicherung der
`#W`-Zeile, TOS-treu. `cargo run -p xtask -- test` 929/0/1; beide Boards
frisch gebaut, tos-api 64 / kernel 94, Clippy 80/125 — Delta 0.

**Vier benannte Abweichungen** (Einzelheiten in der Quittung und in
`docs/api/desktop/deskmain.md`): kein `shel_get`/`shel_put`-Zwischenspeicher,
also kein `app_save(FALSE)`; kein `cnx_get` im Wiedereintritt nach einem
Programm; kein `obj_get_obid`; und ein **ungenutzter** Steckplatz liefert kein
Vorgaberechteck, weil NewTOS EmuTOS' vier eingebaute `#W`-Zeilen nicht
mitliefert. Die letzte ist im ersten QEMU-Lauf gemessen worden (jedes frische
Fenster oeffnete bildschirmfuellend) und braucht eine Lead-Entscheidung:
`desk_inf_data1`s Vorgabezeilen uebernehmen aendert das Startbild.

**Offene Hardwareabnahme:** Pi 4, HW-Checkliste Punkt **35**, Teil B
(`docs/api/desktop/_hw-test-2026-09-17.md`).
