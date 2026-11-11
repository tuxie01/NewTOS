# DESK11 — Desktop liest die Laufwerksliste mit `Drvmap`, EmuTOS mit `Dsetdrv(Dgetdrv())`

Status: PARKED (2026-11-11) | Priority: P4 | Reason: legacy, evidence unverified; receipt DESK11-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 0ffb333ae (feat(desk): drive map through GEMDOS, Desktop info), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen** · Prioritaet: **P4** · Typ: Konformitaet desk/ — Nebenbefund DESK10

Abhaengigkeiten: [DESK10](DESK10.md) (erledigt)

## Sachverhalt

EmuTOS' Desktop holt die Laufwerksmaske ueber GEMDOS: `dos_sdrv(dos_gdrv())`
= `Dsetdrv(Dgetdrv())`, dessen Rueckgabe die Bitmaske ist
(`references/emutos/desk/deskapp.c:714`). NewTOS ruft BIOS `Drvmap` (0x0A)
— dieselbe Bitmaske aus derselben Tabelle (`tos-api/src/bios/mod.rs:223-224`,
`bios/drive.rs:145`), aber ein anderer Aufruf. DESK10 hat den Aufruf auf den
Trap umgestellt, ohne zu aendern, **welcher** Aufruf es ist (Auftrag lautete
„Verhalten unveraendert").

## Arbeit

Auf `dos_sdrv(dos_gdrv())` umstellen (`dosbind.rs` hat die Bindung noch
nicht — zwei Zeilen), Aufrufstellen `app.rs`, `main.rs`, `mod.rs`;
Bootprotokoll und Laufwerksliste unveraendert belegen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen.** Quittung:
[DESK11-STATUS-DONE.MD](DESK11-STATUS-DONE.MD).

`kernel/src/desk/dosbind.rs` hat zwei neue Bindungen — `dos_gdrv()`
(`Dgetdrv`, 0x19) und `dos_sdrv(drv)` (`Dsetdrv`, 0x0E) — und die drei
Aufrufstellen (`app::app_blddesk`, `main::check_alt_letter_key`,
`mod::run`s Startzeile) lesen die Maske jetzt als `dos_sdrv(dos_gdrv())`.
`drvmap()`, der `bios_trap0`-Helfer und die `DRVMAP`-Konstante sind
geloescht; damit macht der Desktop **keinen BIOS- und keinen
XBIOS-Aufruf** mehr.

**Ausgefuehrte Pruefung.** Frisch je Board gebaut (`cargo clean -p kernel
-p tos-api` davor): tos-api 64 / kernel 94 auf beiden, rc 0 (Basis 64/94,
Delta 0); Clippy tos-api 80 / kernel 125 auf beiden (Basis 80/125), null
Treffer in `kernel/src/desk/`; `cargo run -p xtask -- test` 932/0/1.
QEMU raspi4b, gleiches Kartenabbild, vorher (losgeloester Worktree auf
`8d271cb`) gegen nachher: Startprotokoll unterscheidet sich in **einer**
Zeile — der Kernelgroesse —, die Laufwerkszeile
`[desk] desktop up (1 drives, 0 window(s) from the context)` ist
identisch, die Startbilder sind bytegleich (md5
`a53b0a3f076d7040d0a0f92f9152e611`, pixdiff 0), und die beiden
Kartenabbilder vergleichen sich byteweise gleich. Null `[EXC]`/`[PANIC]`.
Belege: `measurements/measurement-2026-09-24-desk01-desk02/`.

**Offene Hardwareabnahme.** Pi 4, HW-Checkliste Punkt **37**, Schritt 1:
QEMU zeigt **ein** Laufwerk, die Karte hat **vier** (`C:`–`F:`) — genau der
Teil, den der Emulator nicht abdeckt.
