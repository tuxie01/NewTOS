# XBIOS18 — `give_way` fehlt im AUX-Warten: `kbdvec`-Bytes verzoegert, solange ein Prozess auf AUX wartet

Status: DONE (2026-11-11) | Priority: P3 | Commit: daa2973ea | Receipt: XBIOS18-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/XBIOS18-STATUS-DONE.MD exists; commit daa2973ea; TASK/XBIOS18-STATUS-DONE.MD, commit daa2973ea (feat(deps): ADR evidence with location, hardware slots, AUX path (TOOL), fix/feat commits naming the id: dcaeb4c94 (n=1)

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-29 ([Quittung](XBIOS18-STATUS-DONE.MD)) · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-29, Fund aus [XBIOS15](XBIOS15.md)

## Sachverhalt

- `Bconin(2)` bricht sein Warten ab, wenn der laufende Prozess Bytes fuer
  seinen `kbdvec`-Haken in der Schlange hat, und laesst den Trap neu starten
  (`kernel/src/host_env.rs:600@read_key_loop`, `give_way`; KEYS02).
- Das AUX-Warten auf qemu-virt (`kernel/src/console.rs:267@aux_read_blocking`,
  XBIOS15) tut das nicht: der Haken eines Prozesses, der in `Bconin(1)`/
  `Cauxin` wartet, bekommt seine Tastenbytes erst, wenn ein serielles Byte
  ankommt. EmuTOS ruft den Haken aus dem Interrupt sofort. Nicht gemessen;
  kein portierter Nutzer bekannt.
- Das Warten selbst (kein Abgeben an andere Prozesse) bleibt wie EmuTOS
  (Anwender 2026-10-29, keine Erweiterung von ADR-029).

## Lokalisierung (Lead, 2026-10-29)

- `bconin1` → `kernel/src/console.rs@AuxSerial` (`bconin`, Warten in
  `aux_read_blocking`); `kbdvec` / `kbd_int` → `kernel/src/ikbd.rs@kbd_int`,
  Zustellung des Hakens ueber `tos-api/src/xbios/kbdvecs.rs` (KEYS02,
  `pending_for_running`); Vorbild `kernel/src/host_env.rs:600@read_key_loop`
  (`give_way`).
- `--up --layers bios`: `bconin1` 283 Funktionen, vollstaendig (keine
  `unmapped`); `kbdvec`/`kbd_int` unvollstaendig: `bios_init`, `biosmain`
  (`unmapped`, Start).

## Arbeit

0. Lokalisierung (Lead) nach CLAUDE.md „Testing“.
1. Messen zuerst (Sondenregel): Sonde mit `kbdvec`-Haken, die in `Bconin(1)`
   wartet; Taste einspeisen (Orakel: Hatari `--cmd-fifo`; QEMU: Injektor),
   Zeitpunkt und Anzahl der Haken-Aufrufe vor dem ersten seriellen Byte.
2. Fix: `give_way` auch im AUX-Weg.

## Abnahme

- Sonde beidseitig gleich (Haken-Aufrufe vor dem seriellen Byte); keine neue
  Abweichung; Hosttests gruen; doc-check 0 Fehler.
