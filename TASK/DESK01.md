# DESK01 — Desk → About (Desktop-Info): Versionsdialog

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 14, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 0ffb333ae (feat(desk): drive map through GEMDOS, Desktop info), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen** · Prioritaet: **P2** · Typ: Funktionsluecke desk/

Abhaengigkeiten: [DESK08](DESK08.md) (`form_center`/`form_dial`) — Dialog zentriert sich sonst nicht

## Sachverhalt

`kernel/src/desk/main.rs` `hndl_menu`: `Item::About` hat keinen Handler
(`_ => {}`). EmuTOS zeigt den Desktop-Info-Dialog ueber `do_aboutdesk`
(`references/emutos/desk/deskmain.c`, Baum `ADDINFO` in `desk/desk_rsc.c`)
mit Version, Datum und GPL-Hinweis.

## Arbeit

Quelle lesen (`do_aboutdesk`, `ADDINFO`-Baum, Textzeilen), Dialog ueber
unsere `form_do`-Kette zeigen (Muster: die vorhandenen desk/-Dialoge in
`dlg.rs`/`inf.rs`), Texte auf NewTOS anpassen (Name, Version, Datum aus dem
Build). Hosttest auf den Baumaufbau, Screendump gegen den EmuTOS-Dialog
(Hatari-Falcon: Desktop-Info ist per Tastatur nicht erreichbar → Injektor
oder Vergleich gegen `rscview`).

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen.** Quittung:
[DESK01-STATUS-DONE.MD](DESK01-STATUS-DONE.MD).

`kernel/src/desk/inf.rs::do_aboutdesk` baut den Baum und laesst ihn ueber
`inf_show` laufen; `main::do_deskmenu` ruft ihn fuer `Item::About`. Der
`form_alert` ist weg — ein Alert ist kein Dialog (eigener AES-Baum, eigenes
`bb_save`, kein reserviertes Rechteck, kein `WM_REDRAW` an die verdeckten
Fenster). Baumform wie `ADDINFO` (`desk_rsc.c:772-873`): zentrierter Kasten,
Titelzeile, ein `label`/`value`-Block, freie Textzeilen, ein
`DEFAULT|EXIT`-Knopf. Inhalt eigen und aus **einer** Quelle mit dem
Startschirm — `initinfo::VERSION_STRING`, `INDEPENDENCE_NOTICE`,
`AUTHORSHIP_LINE`, `machine_name()`. Spec:
`docs/api/desktop/deskmain.md`, Abschnitt "NewTOS implementation notes —
`do_deskmenu`'s About box (DESK01)".

**Ausgefuehrte Pruefung.** Bau/Clippy/Hosttests siehe
[DESK01-STATUS-DONE.MD](DESK01-STATUS-DONE.MD). QEMU raspi4b mit dem
Scratch-Injektor: der Kasten misst am Bild **(128, 111, 384, 276)** — genau
`form_center`s Wert (`(640-384)/2 = 128`, `19 + (480-19-276)/2 = 111`) —, und
nach `OK` ist der Schirm **bytegleich** zu dem vor dem Oeffnen (md5
`710d7f24cc078d43e9351725a205a16e` auf beiden Seiten, pixdiff 0). Null
`[EXC]`/`[PANIC]`. Belege: `measurements/measurement-2026-09-24-desk01-desk02/`.

**Kein Baudatum, und das ist ein Befund.** `ADDINFO` traegt `DECOPYRT` aus
`COPYRIGHT_YEAR`; NewTOS hat **keine** Baudatums- oder Baujahrskonstante — der
`os_date` im OSHEADER ist ein ausdruecklicher 1980-01-01-Platzhalter mit
schon festgehaltener Luecke (`tos-api/src/bdos/process/loader.rs:806-809`),
und die `Boot time`-Zeile des Startschirms ist die GEMDOS-Uhr, nicht der Bau.
Die Zeile fehlt, statt etwas Unwahres ueber den Bau zu behaupten.

**Offene Hardwareabnahme.** Pi 4, HW-Checkliste Punkt **37**, Schritt 2 (die
1920x1080-Optik des Kastens).
