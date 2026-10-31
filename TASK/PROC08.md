# PROC08 — Rueckkehr aus `Pexec(4)` findet die Code-Seite des Elternprozesses nicht

Status: **OFFEN** 2026-10-31 (Anwender: Ticket anlegen) · Prioritaet: **P1** · Typ: Kernfehler — Ursachenforschung zuerst, Fix erst nach Vorlage der Ursache — Fund C1-2 aus [PITEST01](PITEST01.md), Lauf C1

Vor-Start-Pruefung: Produkt-Ticket, Kartenprogramm betroffen (zView laedt
seine Bildformat-Module so); jedes Programm, das per `Pexec` 3+4 Module oder
Kinder startet, kann es treffen.

## Sachverhalt (gemessen, echte Pi 4)

- Lauf: `measurements/measurement-2026-10-31-pitest01-c1/RUNINFO.TXT`
  (Release-Kern `e0fe680b...` aus `6796787c`, `serial=on`), serielle
  Mitschrift
  `measurements/measurement-2026-10-31-pitest01-c1/serial/screenlog-c1.txt` Zeilen 6660–6691.
- Erster zView-Start nach DOOM und DOOMNOKB: zView startet zwei Kinder
  (`[exec] preparing program (slot 3)` zweimal), dann
  `[crash] slot 2 Bus Error esr=0x82000007 pc=0x1cb497c far=Some(30099836)`,
  "private memory, execute (translation fault, level 3)",
  basepage `01c85000`, text `01c86000`, "Crash at text+0002e97c",
  X30 = `0x1cb4964`.
- Die Adresse liegt in zViews eigenem Code. `ZVIEW.NAP` auf der Karte ist
  bytegleich mit `sdk/zview/build/out/ZVIEW.NAP` (sha256 `efb1aebbd303ba57...`,
  nicht gestrippt); Lastbasis = basepage (ELF-Eintrag `0x1000` -> `0x1c86000`).
  pc `0x1cb497c` = ELF `0x2f97c` in `ldg_load`:

  ```
  2f944  svc #1          ; Pexec(3, ...)  Modul laden
  2f960  bl  ldg_cpush
  2f964  mov w0,#4 ; mov w8,#0x4b ; x1=0, x2=basepage, x3=0
  2f978  svc #1          ; Pexec(4, 0, basepage, 0)  Modul starten
  2f97c  bl  ldg_cpush   ; <- Befehlsabruf scheitert
  ```

  Der erste Befehl nach der Rueckkehr aus `Pexec(4)` in den Elternprozess
  hatte keine Uebersetzung auf Stufe 3.
- Zweiter zView-Start im selben Boot: zehn Modulstarts, kein Absturz,
  normales Ende. Der Fehler haengt also von Zustand oder Zeitpunkt ab.
- Seitenschutz-Fehlerbild "Bus Error" mit Tastendruck-Abbruch: Release-Kern
  ohne `crash-nowait`.

## Bezug

- Kernstellen, an denen der Elternprozess nach dem Ende des Kindes wieder
  eingeschaltet wird bzw. das Kind startet (Ausgangspunkt, nicht Befund):
  `kernel/src/arch/aarch64/exceptions.rs:1781@install_ttbr0`,
  `kernel/src/arch/aarch64/exceptions.rs:2160@handle_pexec`,
  `kernel/src/arch/aarch64/exceptions.rs:2416@install_ttbr0`,
  `tos-api/src/bdos/process/mmu.rs:1688@install_ttbr0`.
- [PROC01](PROC01.md) (Pexec-Modi), [MEM12](MEM12.md) (Seite-0-Schutz,
  erledigt), QUIRK-001 in `docs/MEMORY.md`.
- [DOOM01](DOOM01.md): Verbindung nicht belegt.

## Arbeit

1. Researcher: Ursache belegen — welcher Zustand fuehrt dazu, dass beim
   Ruecksprung aus `Pexec(4)` die Code-Seite des Elternprozesses nicht
   abgebildet ist (falsche Wurzel aktiv, Eintrag fehlt/entfernt, Reihenfolge
   von Tabellenpflege und Umschalten, Prozesswechsel waehrend des Kindes, ...).
   Reproduktion in QEMU versuchen (zView-Modulstarts, auch nach DOOM-Lauf);
   Bericht an den Lead vor jedem Fix.
2. Developer erst nach Entscheid des Anwenders; Abnahme: Wiederholungslauf
   mit vielen `Pexec` 3+4 (Sonde oder zView) in QEMU und auf der echten Pi.

[Gemeinsame Regeln](README.md) gelten.
