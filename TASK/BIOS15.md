# BIOS15 — ADR-031 umsetzen: nicht bediente OS-Haken melden, Besitz, `prt_*`-Standard

Status: **ERLEDIGT** 2026-10-29 ([Quittung](BIOS15-STATUS-DONE.MD)), vom Anwender abgenommen · Prioritaet: **P1** · Typ: Developer — Anwenderentscheidung 2026-10-28 (ADR-031, aus [BIOS03](BIOS03.md))

## Sachverhalt

- ADR-031 „OS hooks installed by programs“ (`docs/MEMORY.md`): ausser
  `kbdvec`, `etv_term`, `vex_wheelv`, `Supexec`, `G_USERDEF` bedient NewTOS
  keinen von Programmen eingehaengten Haken; Regel „nie still“. Tabelle je
  Haken und Belege: ADR-031 und
  `docs/analysis/os-hooks-adr-proposal-2026-10-28.md`.
- Heute: Warnung nur fuer Accessory-Haken der `Kbdvbase`-Tabelle
  (`accessory_hook_unwarned` (seit BIOS15 aufgegangen in `tos-api/src/bdos/process/hooks.rs:504@note`)); `vex_butv`/
  `vex_motv`/`vex_curv`/`vex_timv` speichern still; `prt_stat`/`prt_vec` sind 0
  statt des `just_rts`-Stubs.
- Damit werden BIOS-CORE-002 ([BIOS05](BIOS05.md)), XBIOS-CHARDEV-003
  ([XBIOS03](XBIOS03.md)) und BIOS-STOR-010 zu ACCEPTED-Abweichungen; offen
  bleibt nur die Warnung.

## Lokalisierung (Lead, 2026-10-29)

- Erkennungsstellen → NewTOS (`cargo xtask deps <fn> --down` + `mapping.tsv`):
  - `setexc` → `tos-api/src/bios/vectors.rs@setexc` (`--up` 313 / 27 mit
    Gegenstueck).
  - Seite-0-Zellen beim Verlassen des Super-Zustands →
    `tos-api/src/bdos/process/superstate.rs:38@set_super`, `:66@super_call`
    (EmuTOS-Seite: keine eigene Funktion, Zellen werden direkt beschrieben).
  - `Kbdvbase`-Slots → `tos-api/src/xbios/kbdvecs.rs@kbdvbase` (`--up` 18 / 3);
    Warnmuster `accessory_hook_unwarned` (seit BIOS15 `tos-api/src/bdos/process/hooks.rs:504@note`).
  - `Initmous` → Arm im XBIOS-Dispatcher `tos-api/src/xbios/mod.rs@dispatch`
    (keine Zeile in `mapping.tsv`); `mfpint` (`--up` 10 / 3) und `xbtimer`
    (7 / 2) **ohne Gegenstueck**.
  - `vex_butv`/`motv`/`curv`/`timv` → `tos-api/src/vdi/mouse.rs:967@vex_butv`,
    `:972@vex_motv`, `:977@vex_curv`, `:1025@vex_timv` (VDI-Schicht, ohne
    `mapping.tsv`-Zeilen).
  - `prt_stat`/`prt_vec`-Startwert →
    `tos-api/src/bdos/process/loader.rs:1281@prt_stat` (`populate_sysvars`).
  - Besitz bei `Pterm`: `xterm` → `tos-api/src/bdos/process/pterm.rs@terminate`
    (`--up` 299 / 25).
- Ohne Gegenstueck in `bios`/`bdos` (heutiges `mapping.tsv`): `mfpint`,
  `xbtimer` und die Start-/Erkennungsfunktionen wie in BIOS01; nach TOOL17 neu
  klassifizieren. Der Fix ergaenzt die Zeilen der Funktionen, die er beruehrt.
- Retest: `bios-core-r1` (Setexc-Faelle), `xbios-chardev-r1` (Kbdvbase),
  `xbios-system-r1`, `super-r1`, Hosttests der Prozessverwaltung, Doom
  (`kbdvec` muss weiter wirken), TeraDesk-Start und -Shutdown (haengt
  `etv_critic`, `hdv_*`, Vektoren ein → genau die Warnungen, kein Absturz).

## Arbeit

0. Lokalisierung (Lead, vor Beginn): `cargo xtask deps <fn> --down` +
   `mapping.tsv` je Erkennungsstelle, `--up`-Retest-Menge hier eintragen
   (CLAUDE.md „Testing“).
1. **Warnung einmal je (Haken, Prozess)** fuer jeden nicht bedienten Haken
   der ADR-Tabelle; keine Warnung fuer Haken, die EmuTOS selbst nie ruft.
   Erkennungsstellen (ADR-031 Punkt 5): in `Setexc`; Seite-0-Zellen beim
   Verlassen des Super-Zustands (Vergleich mit den Standardwerten, nur lesen,
   nie rufen); `Kbdvbase`-Slots beim ersten Ereignis, das sie riefe;
   `Initmous`, `Mfpint`, `Xbtimer`, `vex_butv/motv/curv/timv` im Aufruf.
2. **Besitz:** der einhaengende Prozess besitzt den Haken. `Pterm` ohne
   Aushaengen: alten Vektor nur zurueckstellen, wenn der Vektor noch auf den
   Haken des endenden Prozesses zeigt (oben in der XBRA-Kette), sonst stehen
   lassen und warnen. `Ptermres`: Haken bleibt, Besitzer bleibt resident.
3. `prt_stat`/`prt_vec`: Standard = `just_rts`-Stub wie EmuTOS
   (`references/emutos/bios/chardev.c` bzw. die Stelle, die der Bericht
   nennt), nicht 0.
4. Hosttests je Erkennungsstelle (Warnung genau einmal je Haken und Prozess,
   zweiter Prozess warnt erneut), Besitzregel (oben/nicht oben in der Kette,
   `Ptermres`), `prt_*`-Startwert.
5. Messfall gegen das Orakel, wo beobachtbar (`prt_stat`/`prt_vec`-Werte in
   Seite 0; Sondenregel CLAUDE.md „Testing“, wenn kein tosdiff-Fall moeglich).

## Abnahme

- Hosttests gruen (`cargo xtask test`); QEMU `raspi4b` bis zum Desktop; das
  Log eines Programms, das `hdv_mediach` und `etv_timer` einhaengt, zeigt je
  eine Warnung.
- `docs/COMPATIBILITY.md` „OS hooks“ stimmt mit dem Verhalten; doc-check 0
  Fehler; `docs/STATUS.md` im selben Commit.

## Anwender 2026-10-29 (Umfang)

- `Mfpint` und `Xbtimer`: nicht bedient, einmal warnen (ADR-031), **und** der
  Rueckgabewert wie das Orakel — gemessen (tt, falcontc), Fall per Tabelle
  oder dauerhafte Sonde.

## Stand 2026-10-29

- Umgesetzt (`tos-api/src/bdos/process/hooks.rs`): eine Warnung je Haken und
  Prozess, Besitzregel bei `Pterm`/`Ptermres`, `prt_stat`/`prt_vec` auf den
  `ret`-Stub (gemessen gleich EmuTOS), `Mfpint`/`Xbtimer` liefern den
  Rueckgabewert des Orakels (IMRB 0x60 / IMRA 0x14, `Xbtimer` mit ungueltigem
  Timer `control`; Spec `docs/api/xbios/0d-mfpint.md`). Sonde `hkprobe`
  dauerhaft unter `tosdump/probes/`. Beleg
  `measurements/measurement-2026-10-29-bios15/RUNINFO.TXT`.
- **Bestaetigt (Anwender 2026-10-29, Nachtrag zu ADR-031):** zusaetzliche Erkennungsstelle „nach jedem
  Trap im Super-Zustand“ fuer Seite-0-Zellen. ADR-031 Regel 4 nennt nur das
  Verlassen des Super-Zustands; ohne die Zusatzstelle bliebe TeraDesks
  `force_mediach` still, weil es `hdv_*` vor dem Verlassen zuruecksetzt.
  Gelesen wird weiterhin nur zum Warnen.
- Offen notiert: `Jdisint`/`Jenabint` liefern weiter ihre Funktionsnummer;
  `Setexc(0x40+n, -1)` liefert fuer MFP-Vektoren 0 (EmuTOS die Adresse) —
  BIOS07; `docs/api/xbios/00-initmous.md` nennt `EINVFN` statt
  Funktionsnummer; `docs/api/xbios/_triage.md` fuehrt `CONF_WITH_MFP` als aus.

