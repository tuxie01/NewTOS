# ARCH02 — `Pexec`-Eltern verlieren ihre FP/SIMD-Register

Status: DONE (2026-11-11) | Priority: P2 | Commit: 4e668641a | Receipt: ARCH02-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/ARCH02-STATUS-DONE.MD exists; commit 4e668641a; TASK/ARCH02-STATUS-DONE.MD, commit 4e668641a (fix(process): restore a Pexec caller's FP/SIMD registers), fix/feat commits naming the id: - (n=0), measurements/measurement-2026-10-17-arch02/

Line 3 before the migration:
Status: **ERLEDIGT** (2026-10-17, QEMU; Pi abgenommen 2026-10-17, `docs/HW-TEST-LOG.md`) · Prioritaet: **P2** · Typ: Belegter Defekt / Architektur — Nebenbefund ACC03

Abhaengigkeiten: [ACC03](ACC03.md) (erledigt)

## Sachverhalt

ACC03 hat einen echten Fehler gefunden und behoben: der **Scheduler**-Wechsel
sicherte `v0..v31`, `FPCR` und `FPSR` nicht. Das Symptom war unmissverstaendlich
— nach einem GEM-Programm trat ein Accessory mit dem `d1` **jenes Programms**
als AES-Opcode in den Trap, 110 484 `appl_exit`-Aufrufe aus einem Binaerstueck,
das die Funktion nie ruft; clang hatte eine Schleifenkonstante in `d1` geparkt.

**Derselbe Mechanismus ist auf dem `Pexec`-Pfad noch offen.** Der Fix deckt den
Schedulerwechsel; der Weg EL0-Elter → EL0-Kind → EL0-Elter
(`Pexec`/`Pterm`) sichert die FP/SIMD-Register weiterhin nicht.

## Warum P2 und nicht P4

Es ist heute latent — die Programme, die wir starten, gehen ueber
`run_program`, nicht ueber `Pexec` aus einem anderen EL0-Prozess. Aber:

* der Mechanismus ist **derselbe**, der gerade zwei Tage Diagnose gekostet
  haette, waere er nicht zufaellig an einer lauten Stelle aufgetreten;
* das Symptom ist Datenverfaelschung ohne Absturz, also die Sorte Fehler, die
  man erst drei Schichten spaeter bemerkt;
* und jede portierte Anwendung, die ein Hilfsprogramm startet, faellt hinein.

## Arbeit

`FpContext` und `save_fp_context`/`restore_fp_context` (ACC03,
`kernel/src/arch/aarch64/exceptions.rs`) auf den `Pexec`/`Pterm`-Pfad ziehen.
Vorher am Quelltext feststellen, **wo genau** dort der Elternrahmen gesichert
und wiederhergestellt wird — es ist nicht derselbe Ort wie beim
Schedulerwechsel.

Abnahme wie bei ACC03: ein Programm, das eine FP-Konstante ueber einen
`Pexec`-Aufruf hinweg haelt und sie danach prueft. Vor dem Fix muss es
scheitern, sonst misst der Test nicht, was er soll.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Architekturdatei — Registerverhalten
nur aus der ARM ARM.

## Abschluss

**ERLEDIGT (2026-10-17)** — `8bac146b`, Quittung [ARCH02-STATUS-DONE.MD](ARCH02-STATUS-DONE.MD).

* **Wo der Elternrahmen liegt:** gesichert im Startzweig von `handle_pexec`
  (alle startenden Modi: 0, 4, 6; Modi 100+ gibt es im Baum nicht),
  zurueckgespielt in `finish_terminate`, dem gemeinsamen Schluss jedes
  Prozessendes (`Pterm`/`Pterm0`/`Ptermres`, Absturz, Ctrl-C, `etv_term`).
  Kernel-Start (AUTO, `sh_ldapp`, NEWDESK und sein Neustart) und der Kernel-Park
  der Accessories setzen EL1-Code fort, der keine FP-Register benutzt.
* **Fix:** `Process::parent_fp` am Kind-Slot; `save_fp_context` in
  `handle_pexec`, `restore_fp_context` im `parent_frame_pexec`-Arm von
  `finish_terminate` — dieselben Routinen wie ACC03. `CPACR_EL1.FPEN = 0b11`,
  also kein faules FP-Sichern.
* **Abnahme:** `sdk/examples/fppexec.c` haelt ein Muster in allen 32
  v-Registern plus FPCR/FPSR ueber `Pexec` 0, 3+6 und 3+4. Vorher: 0/32,
  FPCR/FPSR = die Werte des Kindes, `RESULT FAIL`. Nachher: 32/32, `RESULT PASS`
  in memprot on/off/strict. APITEST 109/0/5 vor und nach dem Fix gleich.
  Belege: `measurements/measurement-2026-10-17-arch02/RUNINFO.TXT`.
* **Offen:** derselbe Lauf auf der echten Pi 4 (`SD_FPPEXEC_AUTO=1` in
  `tools/make-sd-image.sh` gilt nur fuer das QEMU-Abbild).
