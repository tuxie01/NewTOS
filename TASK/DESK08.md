# DESK08 — Dialoge: `form_center` und `form_dial(FMD_START/FMD_FINISH)`

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: f16fef8f0 | Missing: real-Pi visual acceptance

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/DESK08-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit f16fef8f0 (feat(desk): dialogs go through form_center and form_dial, the context ), VERIFIED: f16fef8f0 (feat(desk): dialogs go through form_center and form_dial, th) names the id by body, its message names the ticket-title identifier form_center, fmd_start, touches code (68 files), legacy line: ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen** · Prioritaet: **P2** · Typ: Strukturluecke desk/ (G7 der Lueckenliste)

Abhaengigkeiten: —

## Sachverhalt

`docs/analysis/desk-aes-gaps-2026-09-14.md` § G7: `desk/` zeigt seine
Dialoge ohne `form_center` (Zentrierung) und ohne
`form_dial(FMD_START)`/`FMD_FINISH` (Hintergrund sichern/wiederherstellen,
Wachstums-/Schrumpfkasten). EmuTOS macht beides
(`desk/deskinf.c`, `desk/deskins.c`, jeweils vor `form_do`).

## Arbeit

Die Kette `form_center` → `form_dial(FMD_START)` → `objc_draw` → `form_do`
→ `form_dial(FMD_FINISH)` in den vorhandenen desk/-Dialogen nachziehen
(`dlg.rs`, `inf.rs`, `insapp.rs`); Hosttest: Dialog zentriert, Hintergrund
nach dem Schliessen bytegleich.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Erledigt am 2026-09-23**, Quittung [DESK08-STATUS-DONE.MD](DESK08-STATUS-DONE.MD),
Artefakte `measurements/measurement-2026-09-23-desk08-desk09/`. Aenderungen uncommitted im
Baum (Branch `desk-rebuild`, Basis `326ef4c`).

`inf::start_dialog`/`end_dialog`/`inf_show` (EmuTOS `deskinf.c:349-402`) und
`dir::draw_dial` (`deskdir.c:79-85`) liegen an derselben Stelle wie in der
Quelle; alle vier Dialogerzeuger (`dlg.rs`, `inf.rs` zweimal, `insapp.rs`)
bauen ihren Baum im Ursprung und gehen hindurch. Sechs `redraw_desktop()`
hinter Dialogen sind entfallen.

**Richtigstellung zum Sachverhalt:** `form_dial(FMD_START)` sichert den
Hintergrund **nicht** — in EmuTOS ist der Arm ein nacktes `break`
(`aes/gemfmlib.c:365-369`). Wiederhergestellt wird bei `FMD_FINISH` durch
Neuzeichnen (`w_drawdesk(pt)` + `w_update`, `:375-379`), und `end_dialog`
leert die dabei entstandenen Nachrichten selbst — mit EmuTOS' eigener
Begruendung (`deskinf.c:371-378`). Der A57-Schirmpuffer wird nicht gebraucht.

**Tatsaechlich ausgefuehrte Pruefung.** Hosttest
`tos-api/tests/aes_desk_dialog_chain.rs` (3 Faelle, neu): vorher **2 von 3
rot** gegen die alte Desktop-Rechnung (`(175,165)` statt `(175,174)`; 251540
beruehrte Punkte ausserhalb des Kastens statt 0), nachher 3/3 gruen. QEMU
raspi4b: `Options → Set background...` steht gemessen bei
**(170,159,300,180)** = `form_center`s Wert (die alte Rechnung haette y=150
ergeben), und nach `Cancel` ist der Schirm **0 px** verschieden vom Bild
davor. `cargo run -p xtask -- test` 929/0/1; beide Boards frisch gebaut,
tos-api 64 / kernel 94, Clippy 80/125 — Delta 0.

**Ergebnis:** die Kette steht, und sie ist die einzige Stelle, an der ein
Desktop-Dialog auf- und wieder abgebaut wird.

**Offene Hardwareabnahme:** Pi 4, HW-Checkliste Punkt **35**, Teil A
(`docs/api/desktop/_hw-test-2026-09-17.md`).
