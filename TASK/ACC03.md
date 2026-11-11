# ACC03 — Accessory-Prozessmodell: paralleles `evnt_multi`, `wait_for_accs`

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt ACC03-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 18, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 7a250028e (fix(aes,desk): wind_open with a preset rectangle, ), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-25) · Prioritaet: **P2** · Typ: Funktionsluecke / Kern

Abhaengigkeiten: [ACC01](ACC01.md), [ACC02](ACC02.md), [A91](A91.md) (Inline-Kontrollmanager)

## Sachverhalt

Ein Accessory ist in EmuTOS ein eigener `AESPD`, der neben dem Desktop und
der laufenden Anwendung in `evnt_multi` parkt (`aes/geminit.c:654-745`);
nach `dsptch()` wartet der Start mit `wait_for_accs(AP_MESAG)` (`:588-630`,
Timeout 500 Ticks), bis jedes Accessory seine Initialisierung abgeschlossen
hat. NewTOS faehrt den Kontrollmanager **inline** in `evnt_multi`
(`tos-api/src/aes/event.rs`, A91): waehrend eines Menue-Tracks laeuft kein
anderer Prozess. Der Sleep/Wake-Unterbau (`sched.rs`, `wait_mesg`, A75)
traegt aber bereits mehrere wartende Klienten.

## Arbeit

1. Klaeren, was der kooperative Kern fuer Accessories mindestens braucht:
   mehrere Prozesse in `evnt_multi`, Weckung per `post_msg`, Umschaltung im
   Trap-Dispatcher (`Option<Switch>`-Pfad, A75). Belegen, welche Teile
   fehlen (Zeitgeber je Prozess? `MU_TIMER` waehrend fremder Tracks, A91).
2. `wait_for_accs`-Aequivalent nach dem Start.
3. Entscheiden, ob der Inline-Kontrollmanager dafuer fallen muss (A91) oder
   ob Accessories damit leben koennen (sie bekommen waehrend eines Tracks
   keine Ereignisse — EmuTOS schon).

## Abnahme

Zwei Test-Accessories + Desktop laufen gleichzeitig; Hosttest mit zwei
wartenden Klienten; QEMU: beide melden sich, Desktop bedienbar.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-25)** — zusammen mit ACC02, ACC04 und ACC05.

**1. Was der kooperative Kern mindestens brauchte — belegt, nicht vermutet.**
Vor dieser Arbeit konnte unsere AES **keinen zweiten Klienten tragen**, aus drei
Gruenden, alle am Quelltext festgemacht:

* `sched::sleep` kannte nur EL0-Slots. Der Desktop und der Startpfad laufen auf
  EL1 und besitzen keinen Slot (`running_slot() == NONE`), nahmen also immer den
  Leerlaufzweig — ein lauffaehiges Accessory haette **nie** die CPU bekommen,
  solange der Desktop wartet.
* Umgekehrt gab es fuer einen blockierenden EL0-Prozess keinen Rueckweg zum
  geparkten EL1-Aufrufer: ohne lauffaehigen EL0-Partner haette das Accessory im
  Leerlauf gedreht und der Start haette den Desktop nie erreicht.
* Der Kontextwechsel sicherte die **FP/SIMD-Register nicht**. Das ist der teure
  Fund: `ExceptionFrame` traegt nur die GP-Register, was genuegte, solange nie
  zwei EL0-Kontexte verschraenkt liefen. Ein Accessory ist per Konstruktion mit
  Hart-Float-Programmen verschraenkt, und clang haelt den AES-Kontrollblock als
  Schleifenkonstante in `d1` (`str d1, [x9]` vor jedem Trap). Nach einem
  GEM-Programm trat das Accessory mit **dessen** `d1` als Opcode in die AES —
  gemessen: 110 484 `appl_exit`-Aufrufe aus einem Programm, das die Funktion
  nie aufruft.

Nicht noetig war: der Inline-Kontrollmanager (A91) faellt **nicht**. Die
S2-Neustart-Disziplin wickelt die einzige AES-Faser vor jedem Wechsel ab, also
beginnt kein zweiter Klient einen AES-Aufruf, waehrend der eines anderen
eingefroren ist. Ein Accessory bekommt waehrend eines fremden Menue-Tracks keine
Ereignisse — EmuTOS schon; das bleibt der bekannte, angenommene Unterschied.

**2. Gebaut.** Der „Kernel-Park": `park_kernel_frame`/`restore_kernel_park` in
`kernel/src/arch/aarch64/exceptions.rs` friert den EL1-Rahmen ein (ELR auf den
`svc` zurueckgedreht) und gibt einem Accessory die CPU; `sched::switch_to_kernel`
(das dafuer reservierte `el1_internal`-Flag) holt sie zurueck, sobald das
Accessory blockiert. `Pwait` aus dem Kernkontext ist damit EmuTOS' `all_run();
dsptch();`. `FpContext` + `save_fp_context`/`restore_fp_context` sichern
`v0..v31`, FPCR und FPSR — die AArch64-Schreibweise dessen, was EmuTOS' `switchto`
(`gemasm.S:169-194`) laengst tat. Terminiert ein Accessory doch, traegt
`parent_frame_full` den geparkten EL1-Rahmen als Elternrahmen, damit
`finish_terminate` ihn vollstaendig zurueckspielt.

**3. `wait_for_accs`.** `tos-api/src/aes/acc.rs` (`geminit.c:599-628`), mit
beiden EmuTOS-Aufrufstellen: `AP_MESAG` nach dem Start (kernelseitig,
`accs::wait_for_accs_mesag`) und `AP_ACCLOSE` in `appl_exit`
(`gemaplib.c:255`), das dafuer ein S2-neustartfaehiger Syscall geworden ist.

**Belege.** `measurements/measurement-2026-09-25-acc02-acc04/`: zwei Accessories + Desktop
gleichzeitig; `screendumps/g-window.png` (Desktop neben ihnen bedienbar),
`screendumps/h-gemwin.png`/`i-after.png` (ein GEM-Programm laeuft dazwischen und
der Desktop kommt zurueck), `serial/acc2-serial.log` mit den
`[sched] switch`-Ketten. 436 Zeilen Serial fuer den ganzen Lauf — kein
Ping-Pong: ein Accessory wird nur geweckt, wenn eine Nachricht oder Eingabe
kommt.
