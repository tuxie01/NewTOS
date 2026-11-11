# ACC02 — `.ACC` laden: `count_accs`/`load_one_acc` im Startpfad

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt ACC02-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 18, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 96f178544 (feat(aes): NewTOS can run desk accessories), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-25) · Prioritaet: **P2** · Typ: Funktionsluecke

Abhaengigkeiten: [ACC01](ACC01.md) (Spec), [A106](A106.md) (beruehrt denselben Startpfad)

## Sachverhalt

EmuTOS sucht beim Start bis zu `NUM_ACCS = 6` Dateien `\*.ACC` im Wurzel-
verzeichnis des Bootlaufwerks (`aes/geminit.c:265-284`), laedt jede per
`pgmld` und startet sie als eigenen Prozess (`:236-257`), **vor**
`sh_tographic` und dem Desktop (`:686`), und gibt ihren Speicher nach
`sh_main` wieder frei (`:288-303, 746`). NewTOS hat davon nichts.

## Arbeit

1. Laden in `kernel/src/shell_loop.rs`/`lib.rs` an der Stelle, die EmuTOS'
   `run_accs_and_desktop` entspricht (vor `sh_tographic`; unser Aequivalent
   benennen). Dateisuche ueber unsere GEMDOS-Fsfirst/Fsnext, Ladeweg ueber
   den vorhandenen ELF-Lader (`tos-api/src/bdos/process/loader.rs`) —
   `.ACC` ist bei uns ein natives `.NAP`-artiges Programm; Dateiendung und
   Format im Ticket ACC05 entschieden.
2. Prozess je Accessory ueber den kooperativen Unterbau (`sched.rs`,
   `wait_mesg`), **ohne** `Pterm` am Ende — ein Accessory laeuft weiter.
3. `free_accs`-Aequivalent beim Herunterfahren.
4. Bootflag „Accessories ueberspringen" (EmuTOS `BOOTFLAG_SKIP_AUTO_ACC`,
   Control beim Start) — unsere Entsprechung benennen (Tastatur ueber IKBD
   zu diesem Zeitpunkt verfuegbar? sonst `NEWDESK.INF`-Schalter).

## Abnahme

Hosttest fuer die Suche (bis 6, Reihenfolge, fehlende Dateien); QEMU raspi4b
mit einem Test-`.ACC` auf C:\ — Serial zeigt Laden und Prozessstart, Desktop
kommt hoch, `wait_for_accs`-Aequivalent blockiert nicht ewig.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-25)** — zusammen mit ACC03, ACC04 und ACC05 als **eine**
Arbeitseinheit; ein Messverzeichnis, `measurements/measurement-2026-09-25-acc02-acc04/`.

**1. Laden im Startpfad.** `kernel/src/accs.rs` ist EmuTOS' Quartett:
`count_accs` (`aes/geminit.c:264-284`), `load_one_acc` (`:236-257`),
`load_accs` (`:309-315`) und `free_accs` (`:296-303`). Aufgerufen aus
`kernel/src/lib.rs`' Sitzungsschleife an genau EmuTOS' Stellen — `load_accs`
vor `sh_tographic` (`:687`), `free_accs` nach `sh_main` (`:741`). Der Ladeweg
ist `pexec::load` (der native ELF-Lader, EmuTOS' `pgmld`), die Dateisuche
`list_dir_ex` ueber unser GEMDOS.

**2. Prozess je Accessory, ohne `Pterm`.** `load_one_acc` legt den Prozess an
und macht ihn *lauffaehig*, fuehrt ihn aber nicht aus — EmuTOS' `pstart(gotopgm,
…)` (`:255`). Ausgefuehrt wird er erst beim `dsptch()` in
`wait_for_accs_mesag` (`:736-737`), nach `process_inf2` und vor `sh_main`, weil
EmuTOS es dort tut und begruendet (`:579-583`: Speicher aus der Initialisierung
gehoert dann der AES, nicht dem naechsten Programm). Ein Accessory laeuft
weiter; terminiert es doch, traegt der Kern das (siehe ACC03, `parent_frame_full`).

**3. `free_accs`.** Gibt jeden Slot frei (`pterm::release_slot`) — bei uns
**nicht** optional wie in EmuTOS (`:288-295`), weil es keinen uebergeordneten
Prozess gibt, dessen Ende den Speicher mitnaehme.

**4. Bootflag.** `BOOTFLAG_SKIP_AUTO_ACC` hat jetzt seinen zweiten EmuTOS-
Verbraucher: `launch::skip_auto_acc()` → `accs::load_accs(skip)` unterdrueckt
die *Entdeckung* (`geminit.c:269-270`), womit nichts weiter unten etwas zu tun
hat. Kein `NEWDESK.INF`-Schalter noetig — Control auf dem Startschirm genuegt,
die Tastatur ist zu diesem Zeitpunkt da.

**Endung `.NAC`, zwei Suchorte.** `C:\ACC\` zuerst, dann die Wurzel; ein
Dateiname, den der Ordner schon geliefert hat, wird in der Wurzel
uebersprungen; ein gemeinsames Budget von sechs. `kernel/src/desk/ftype.rs`
kennt `Flavor::Accessory`, und ein Doppelklick auf ein `.NAC` startet es
**nicht** (EmuTOS hat dafuer auch keinen Pfad) sondern sagt es.

**Belege.** `measurements/measurement-2026-09-25-acc02-acc04/serial/acc2-serial.log`:
beide Orte geladen (`2 file(s) found, 2 process(es) started`);
`serial/dedupe-serial.log`: gleicher Name in beiden Orten → `1 file(s) found`;
`serial/noacc-serial.log` + `screendumps/noacc-deskmenu.png`: ohne `.NAC` keine
`[acc]`-Zeile und das *Desk*-Menue faellt auf den einen Eintrag zurueck.
Hosttests 944/0/1 unveraendert; `fsck_msdos -n` sauber (`build/fsck.txt`).
