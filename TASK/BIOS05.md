# BIOS05 — Ein per `Setexc(0x100, …)` eingehaengter `etv_timer`-Haken wird nie aufgerufen

Status: **ACCEPTED-Abweichung** 2026-10-28 (ADR-031), offen nur die Warnung in [BIOS15](BIOS15.md) · Prioritaet: **P1** · Typ: Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md` · verwandt mit [BIOS03](BIOS03.md) und [XBIOS03](XBIOS03.md) (Haken, die NewTOS nie ruft)

## Sachverhalt

**BIOS-CORE-002 — S2 — MEASURED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- Sonde T01: Haken zaehlt ueber 400 `_hz_200`-Ticks — EmuTOS `calls=100` (alle 20 ms, TT und Falcon), NewTOS `calls=0` (beide Laeufe) (`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b`).
- Lokalisiert: `tos-api/src/bios/vectors.rs:149@vector` hat fuer `ETV_TIMER` keinen Aufrufer; einzig `etv_term` wird gelesen (`kernel/src/arch/aarch64/exceptions.rs:1385@etv`).
- Eingabepruefung: EmuTOS-Trace zeigt `Setexc(0x100 VEC_TIMER, <Haken>)` und die Rueckgabe des alten Vektors; NewTOS indirekt (S03 `q1_is_new=1`: der Vektor kam an und steht im Schatten).
- EmuTOS-Stelle: `references/emutos/bios/vectors.S:540-544` (Timer-C-Handler ruft `etv_timer` mit `timer_ms` auf dem Stapel, alle Register gesichert).
- Spec: `docs/api/bios/05-setexc.md` „Gaps vs. the contract (Step-2 candidates)“ nennt die Luecke; kein COMPATIBILITY-Eintrag.
- Fehlerbild: TSRs, Uhren, Spiele im 50-Hz-Takt laufen still nicht; `Setexc` meldet Erfolg.

## Arbeit

1. Ein Haken-Aufruf braucht EL0-Code im Kernpfad: gleiche Zielfrage wie bei den `hdv_*`-Vektoren ([BIOS03](BIOS03.md)) und den `Kbdvbase`-Haken ([XBIOS03](XBIOS03.md)). **Anwender 2026-10-28: ja — gemeinsames ADR „OS hooks installed by programs“ in [BIOS03](BIOS03.md)** (Tabelle je Haken; nie still: ein nicht bedienter Haken wird einmal im Log gewarnt). Bis zum ADR kein Fix.
2. Danach Fix gegen `references/emutos/bios/vectors.S:540-544`; Hosttest/Sonde T01 als Messfall.

## Abnahme

- Bestaetigender Fall: Sonde T01 (`calls=100` ueber 400 Ticks) gleich dem Orakel; vorgeschlagene Zeile `13:5 w:0x100,f:<count>` (Bericht Abschnitt 5) ist erst nach Lockerung des Waechters tabellenfaehig ([BIOS13](BIOS13.md)).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Anwenderentscheidung 2026-10-28

- ADR-031 (`docs/MEMORY.md`): der Haken wird nicht bedient, BIOS-CORE-002 ist eine
  ACCEPTED-Abweichung mit dem ADR als Beleg. Kein Fix; die einmalige Warnung
  je Haken und Prozess setzt [BIOS15](BIOS15.md) um.
