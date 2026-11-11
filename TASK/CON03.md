# CON03 — Die Kernel-Konsole ist nicht EmuCON: Befehlssatz und Ort

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/CON03.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Abweichung von EmuTOS — Befund 2026-10-18 (Frage des Anwenders nach einem Textbetrachter)

## Befund

EmuTOS' EmuCON (`references/emutos/cli/`) hat 21 Befehle (`cmdint.c:139-162`):
`cat/type`, `cd`, `chmod`, `cls/clear`, `cp/copy`, `echo`, `exit`, `help`,
`ls/dir`, `mkdir/md`, `mode`, `more`, `mv/move`, `path`, `pwd`, `ren`,
`rm/del`, `rmdir/rd`, `show`, `version`, `wrap`.

NewTOS' Konsole (`kernel/src/shell.rs:120-131`) kennt `exit/quit`, `help`,
`ver`, `cls`, `pwd`, `cd`, `dir/ls`, `type/cat`, `run` sowie die
NewTOS-eigenen `bench`, `init`. Es fehlen u. a. `more` (seitenweise Anzeige —
der einzige Textbetrachter in EmuCON), `cp`, `mv`, `ren`, `rm`, `mkdir`,
`rmdir`, `chmod`, `path`, `echo`, `mode`, `show`, `wrap`.

Ort: EmuCON ist in EmuTOS ein eigener Prozess (`aes_run_rom_program(coma_start)`,
`aes/gemshlib.c:614-620`, `cli/cmdasm.S`), NewTOS' Konsole laeuft im Kern (EL1).
PROC03 hat dafuer einen EmuCON-Stellvertreter-PD eingefuehrt.

## Auftrag (spaeter)

1. Spec aus `cli/` (Befehle, Parser, Umlenkung `>`/`<`, Hilfe-Texte).
2. Anwender entscheidet: Befehlssatz 1:1 in der Kernel-Konsole nachruesten
   oder EmuCON wie NEWDESK als EL0-Programm (ADR-026-Muster).
3. Bauen, Tests per Injektor.

Bezug: [GUI01](GUI01.md) — Textbetrachter im Desktop bleibt eine angemeldete
Anwendung (Anwender 2026-10-18 bestaetigt).

[Gemeinsame Regeln](README.md) gelten.

## Anwenderentscheid 2026-10-22

EmuCON als eigenes EL0-Programm (ADR-026-Muster wie NEWDESK), Befehlssatz
1:1 nach `references/emutos/cli/`. Nicht in der Kernel-Konsole nachruesten.

## Spec und Anwenderentscheide 2026-10-22

Spec: `docs/api/emucon/` (`e4454ea0`). Anwender folgt den Empfehlungen zu
D1–D10:

- D1 `kernel/src/shell.rs` entfaellt, EmuCON ersetzt sie; „No desktop …“ nur
  noch seriell. Rueckfall bei defektem NEWDESK bleibt als Beigabe (derselbe
  Blob), ist aber nicht der Grund fuer EmuCON — Anwender: eine Notfallkonsole
  allein lohnt kaum, EmuCON als Werkzeug (*Execute EmuCON*, spaeter Esc beim
  Boot) schon.
- D2 `bench`/`init`/`run`/`ver`/`quit` entfallen (`bench` ggf. als eigenes `.NCP`).
- D3 Voraussetzung: VT52-Parser → [CON04](CON04.md), vor EmuCON.
- D4 Bildschirmgroesse in Zeichen ueber eine neue `Ssystem`-Unterfunktion
  (wie `S_NEWTOS_KVERSION`); Line-A-artiger Block erst, wenn Ports ihn brauchen.
- D5 `mode con res=` ablehnen wie das Falcon-Profil.
- D6 Endungssuche `.NAP/.NCP/.NTP` (ADR-018).
- D7 1:1, ausser Datenverlust und Ueberlaeufen: diese berichtigen und in
  COMPATIBILITY benennen; vorher in Hatari am echten EmuTOS nachstellen.
- D8 Bibliothek + schmales Programm wie NEWDESK.
- D9 Banner „Welcome to EmuCON2 version …“ mit NewTOS-Version.
- D10 Esc beim Boot: spaeter, eigenes Ticket.

Nebenbefund: `Sversion` liefert `0x3000` (`tos-api/src/gemdos/version.rs:14`),
`docs/api/gemdos/version/30-sversion.md` nennt `0x2000` — klaeren.
