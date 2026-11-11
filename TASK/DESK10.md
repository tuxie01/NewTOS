# DESK10 — `desk/` ruft GEMDOS/BIOS/XBIOS als Trap statt als Rust-Funktion

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK10-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 398c45cc2 (feat(desk): File -> Execute EmuCON starts the cons), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen** · Prioritaet: **P3** · Typ: Strukturluecke desk/ (G6 der Lueckenliste)

Abhaengigkeiten: —

## Sachverhalt

`docs/analysis/desk-aes-gaps-2026-09-14.md` § G6: `desk/` ruft GEMDOS
direkt als Rust-Funktion (`dir.rs` 9 Funktionen, `model.rs`
`Fsfirst`/`Fsnext`/`Fsetdta`), nicht ueber den Trap. Damit laeuft der
Desktop an der ABI vorbei, die jede Anwendung benutzt — Fehler in der
Trap-Schicht bleiben unsichtbar (Memory `desktop-bypasses-app-abi-path`).

## Arbeit

Aufrufstellen auf den Trap umstellen (Muster: die AES-Aufrufe in
`desk/mod.rs` gehen bereits ueber `svc`), Verhalten unveraendert;
Regressionsbeleg per Screendump und `xtask test`.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Status:** ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen.
**Artefakt:** uncommitted im Baum auf `desk-rebuild`, Basis HEAD `8e855d2`;
Quittung [DESK10-STATUS-DONE.MD](DESK10-STATUS-DONE.MD), Belege in
`measurements/measurement-2026-09-23-desk10/`.

Neu ist [`kernel/src/desk/dosbind.rs`](../kernel/src/desk/dosbind.rs), das
GEMDOS/BIOS-Gegenstueck zum aesbind-Block in `desk/mod.rs`, als eigenes Modul
genau dort getrennt, wo EmuTOS trennt: die AES-Bindung ist desk-lokal
(`desk/gembind.c`), die GEMDOS-Bindung ist der gemeinsame Kopf
`include/gemdos.h` ueber `include/bdosbind.h`s `trap1()`. Es stellt `svc #1`
(GEMDOS) und `svc #13` (BIOS) mit der Funktionsnummer in `x8` und den
Argumenten in `x0..x7` in TOS-Reihenfolge (`docs/ARCHITECTURE.md`) und
enthaelt 14 Bindungen mit EmuTOS' eigenen Namen: `dos_sdta`, `dos_sfirst`,
`dos_snext`, `dos_open`, `dos_create`, `dos_close`, `dos_read`, `dos_write`,
`dos_delete`, `dos_rename`, `dos_mkdir`, `dos_rmdir`, `dos_setdt` und
`drvmap` (EmuTOS `Drvmap`, klein nach `CLAUDE.md`). Der Desktop ruft keine
XBIOS-Funktion, also gibt es keine `svc #14`-Bindung.

Umgestellt wurden `dir.rs` (zehn Huellen, gleiche Signaturen — **keine**
Aufrufstelle musste sich aendern), `inf.rs` (`NEWDESK.INF` lesen und
schreiben), `model.rs` (der Verzeichnis-Scan) sowie `app.rs`/`main.rs`/
`mod.rs` (`drvbits()` → `drvmap()`). Die Suche
`/usr/bin/grep -rn "tos_api::bdos\|tos_api::gemdos\|tos_api::bios\|tos_api::xbios" kernel/src/desk/`
ging von 11 Zeilen mit 8 echten Aufrufen auf **4 Zeilen, alles Kommentare**.

**Die zwei Stellen, an denen der Unterschied echt ist**, waren vorher zu
klaeren, und keine hat etwas verschoben: die **DTA** liegt im Basepage des
laufenden Prozesses (`bdos/fsmain.c:396-405`), `current()` ist waehrend
`desk::run` **null**, und `tos-api/src/bdos/fs/find.rs:52-105` leitet genau
diesen Aufrufer auf seine `KERNEL_DTA`-Zelle — dieselbe Zelle und derselbe
44-Byte-Stapelpuffer wie beim direkten Aufruf; **Zeigerargumente** liest die
GEMDOS-Dateischicht **roh** (`bdos/fs/mod.rs:23-33`, `fs/file.rs:297,373`),
`usermem::resolve_kva` liegt nur im AES-Titelleser und in XBIOS `Setscreen`,
also kommt eine Kernel-VA unveraendert an.

**Tatsaechlich ausgefuehrte Pruefung.** Hosttest ist fuer diese Aenderung
**nicht moeglich** — sie liegt ganz in `kernel/` mit forciertem Ziel, und
`xtask test` faehrt nur `tos-api`/`tos-types`/`xtask`
(`tools/xtask/src/main.rs:582`). Der Nachweis ist in QEMU raspi4b gefuehrt,
in vier **Paaren** vorher/nachher, alle ohne `[EXC]`/`[PANIC]`: (1) Startbild
**dateigleich** und Startprotokoll `diff`-leer, inklusive der
`Drvmap`-Zeile; (2) Lauf mit vorgegebener `NEWDESK.INF` — beide Fenster
kommen mit vollem Inhalt zurueck (26 bzw. 1 Eintrag), Bild dateigleich;
(3) `Options → Save desktop` ueber die echte Oberflaeche, Abbild eingehaengt,
Datei ausgelesen — **bytegleich**, SHA-256 `3671808d…` auf beiden Seiten;
(4) der Dateioperationslauf (`Dcreate`, `Fcreate`+`Fwrite`, Kopie mit
`Fdatime`, `Frename`, rekursives Loeschen, Zaehllauf) **12 von 12 bestanden
auf beiden Seiten, gleiche Ausgabe und bytegleiches Kartenabbild**;
`fsck_msdos` ohne Beanstandung. `cargo run -p xtask -- test` 929/0/1; beide
Boards frisch gebaut, tos-api 64 / kernel 94, Clippy 80/125 — Delta 0, null
Treffer in `kernel/src/desk/`.

**Die Vergleichsbilder aus `measurements/measurement-2026-09-23-desk08-desk09/` taugen
dafuer nicht mehr** — sie liegen vor `8e855d2` („kein Fenster beim Start")
und tragen noch das Startfenster; der Vergleich ergibt 118314 Pixel, die mit
DESK10 nichts zu tun haben. Die Paare sind daher beide auf `8e855d2`
aufgenommen, die „vorher"-Seite aus einem losgeloesten Worktree.

**Eine benannte Abweichung bleibt bewusst stehen:** EmuTOS liest die
Laufwerksliste mit `dos_sdrv(dos_gdrv())` (`deskapp.c:714`), NewTOS mit
`Drvmap` — gleiche Bitmaske aus derselben Tabelle. Die Angleichung waere eine
Verhaltensaenderung und braucht ein eigenes Ticket.

**Offene Hardwareabnahme:** Pi 4, HW-Checkliste Punkt **36**
(`docs/api/desktop/_hw-test-2026-09-17.md`). QEMU zeigt ein Laufwerk, die
Karte hat vier.
