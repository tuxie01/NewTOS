# DESK28 — `\AUTO\` setzt kein Arbeitsverzeichnis

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK28-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by b48703149 (fix(desk): AUTO launches start from the root again), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-24, teils zurueckgezogen) · Prioritaet: **P2** · Typ: Kompatibilitaetsdefekt — Nebenbefund DESK18

Abhaengigkeiten: [DESK18](DESK18.md) (erledigt)

## Sachverhalt

DESK18 hat den Verzeichnisschritt fuer jeden Start **aus dem Desktop**
gebaut. Der zweite Startweg hat ihn nicht: `crate::launch::run_auto_folder`
(`kernel/src/launch.rs`) ruft den Lader direkt, ohne `Dsetdrv`/`Dsetpath`.
Ein `.NCP` in `C:\AUTO\`, das eine Datei mit blossem Namen oeffnet, sieht
deshalb die Wurzel.

Das ist derselbe Defekt wie DESK18 an der anderen Stelle, und es ist der
Weg, ueber den auf einer echten Karte die Treiber und Hilfsprogramme
starten.

## Arbeit

Den Verzeichnisschritt aus `desk::pro::set_default_path` an den
AUTO-Startpfad anlegen — EmuTOS' Ort ist `bios/bios.c`s `autoexec`, dort
gegenpruefen, welchen Pfad TOS dort setzt (Wurzel oder `\AUTO\`). Abnahme:
`CURDIR.NCP` (aus DESK18, `sdk/examples/curdir.c`) nach `C:\AUTO\` legen und
sein Protokoll lesen.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-24)

**Gebaut — die Praemisse aber zur Haelfte zurueckgezogen.** Die Quelle sagt
nicht `\AUTO\`, sondern **die Wurzel des Bootlaufwerks**: `run_auto_program`
uebergibt den vollen Pfad an `Pexec(PE_LOADGO, …)` und `autoexec` ruft weder
`Dsetdrv` noch `Dsetpath` (`bios/bios.c:685-733`). Das Arbeitsverzeichnis des
AUTO-Kindes ist reines Erbe: `Pexec` kopiert Laufwerk und Verzeichnistabelle
aus der Basepage des Rufers (`init_pd_files`, `bdos/proc.c:400-410`), und der
Rufer ist das BIOS mit `Dsetdrv(bootdev)` (`bios.c:827`) und der Wurzel-DND,
die `ckdrv` beim ersten Zugriff einhaengt (`bdos/fsdrive.c:119-131`). Dass ein
AUTO-Programm die Wurzel sieht, ist also **nicht** der Defekt, den der Ticket
vermutete — das ist TOS.

Was bleibt und gebaut wurde: NewTOS haelt Laufwerk und Verzeichnisse global
(dokumentierter Platzhalter fuer `p_curdrv`/`p_curdir`), also *leckt* ein
`Dsetpath` des einen AUTO-Programms in das naechste und in den Desktop —
was auf EmuTOS durch die Kopie nicht passieren kann.
`crate::launch::set_auto_default_path` stellt den BIOS-Zustand (Bootlaufwerk,
Wurzel auf jedem Laufwerk) vor jedem Programm und einmal nach der Schleife
wieder her.

* Code: `kernel/src/launch.rs` (`set_auto_default_path`, zwei Aufrufe in
  `run_auto_folder`).
* Spezifikation: `docs/api/emutos-boot-flow.md` §3, Abschnitt
  *The working directory an AUTO program starts in (DESK28)*.
* Messung: `measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/` §1-2.
  Vorher `CURDIR: drive C:\SUB` / `read "PROOF: C:\SUB"`, nachher
  `CURDIR: drive C:` / `read "PROOF: C:\ (root)"`, gleiche Karte, gleicher
  Bootvorgang, drei Dateien Unterschied.
* Pruefung: `cargo xtask build` rpi4/qemu-virt 64/94 (Basis 64/94),
  `cargo clippy` 80/125 (Basis 80/125), 0 Treffer in `desk/`,
  `cargo run -p xtask -- test` 938/0/1 (Basis 938/0/1), `fsck_msdos -n`
  sauber, 0 `[EXC]`/`[PANIC]`.
* Offen: Hardwareabnahme, Punkt **55** in
  `docs/api/desktop/_hw-test-2026-09-17.md` — der Mehrlaufwerksteil (C-F) ist
  in QEMU mit einer Partition nicht pruefbar.
