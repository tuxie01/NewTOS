# A115 — STATUS: DONE (2026-10-25, QEMU) — Bereitliste wie EmuTOS `rlr`/`drl`

**Auftrag:** Auswahl des naechsten Prozesses 1:1 nach EmuTOS' `rlr`/`drl`
statt Slot-Reihenfolge; alle Stellen, die `runnable` setzen.

## Wirkung (vor dem Fix, HEAD `587e0f68`)

| Fall | vorher | EmuTOS / nachher |
|---|---|---|
| Accessory in `evnt_mesag`, Programm `appl_write` an es | Wechsel zum Leser schon beim Abschluss-Dispatch des Schreibers | Leser hinter dem Schreiber; Schreiber laeuft weiter, sein naechster AES-Aufruf gibt ab |
| A (100 ms) und C (200 ms) in `evnt_timer`, beide abgelaufen, Programm blockiert | A zuerst (Slot) | C zuerst (`drl` Push-front, umgekehrte Weckreihenfolge) |
| zwei Accessories X, Y nacheinander gestartet, Kernel-Client dispatcht | X zuerst | erster Dispatch behaelt der Kernel-Client, dann Y, X, Kernel-Client |
| QEMU f2 (MAIN schliesst Deckfenster ueber ACCB/ACCC) | ACCB, ACCC, dann „closed+deleted“ | „closed+deleted“, dann ACCC, ACCB |

Hosttest `tos-api/tests/sched_a115_ready_list.rs`: 0/3 auf der Basis, 3/3 neu.

## Stellen (EmuTOS, gepinnt)

`aes/gemdisp.c:74-79` `disp_act`, `:82-93` `mwait_act`, `:101-155` `forker`,
`:181-205` `schedule`, `:221-253` `disp`; `aes/gempd.c:125-128` `pstart`,
`:134-145` `insert_process`; `aes/gemasync.c:35-61` `signal`;
`aes/gemflag.c:32-68` `tchange`; `aes/geminput.c:496-542` `adelay`.

## Fix (`e7754b16`)

`tos-api/src/bdos/process/sched.rs` „The ready list“: `ready` = `disp_act`,
`unready` = `nrl`/weg, `push_drl` = `signal`, `drain_drl` = `schedule`,
`pick_head` = Kopf von `rlr` (Slot oder geparkter Kernel-Client); `admit` =
`pstart`; `wake_timers` in `dlr`-Reihenfolge. `pick_next` entfaellt. Alle
`runnable`-Schreiber ueber `sched` (`free_slot`, 9 Testfixturen, 2 unverdrahtete
Selftests). Spec `docs/api/aes/event.md` (A115-Bindung), `accessories.md`.

## Pruefung

Bau beide Boards dev+release rc 0, 159 = 159 Warnzeilen; Clippy tos-api 74,
kernel 121 unveraendert; `cargo xtask test` 160 Binaries 1641/0/1 → 161,
1644/0/1. QEMU raspi4b: NEWDESK-, TeraDesk-, QED-Reihe (gh1-reg-Methode),
Standardkarte mit zwei Accessories, A117-Methode, TeraDesk beenden → NEWDESK,
A112-Sonden f1–f3, APITEST 109/0/5 in memprot on/off/strict — pixel- bzw.
ergebnisgleich, ausser der f2-Reihenfolge (gewollt) und einer QED-Cursorphase
(Basis-Lauf schwankt selbst). 0 EXC/PANIC. Beleg
`measurements/measurement-2026-10-25-a115/RUNINFO.TXT`.

## Offen

* Echte Pi 4.
* Desk-Menue-Reihenfolge zweier Accessories mit verschiedenen Titeln (jetzt
  umgekehrte Ladereihenfolge wie EmuTOS) nicht im Bild belegt.
* `selftest_sleepwake_switch` scheitert auf Basis und neu vor jedem
  Scheduling („load R: NoTpaMemory“) — vorbestehend, eigener Befund.
