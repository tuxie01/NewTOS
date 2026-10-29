# BIOS07 — `Setexc(n, -1)`: NULL, 64-Bit-Platzhalter und Schatten statt der EmuTOS-Werte

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (004 ggf. Researcher, falls als Abweichung sanktioniert) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

Drei Befunde derselben Funktion und desselben Zweigs (`Setexc` mit `-1` fragt ab). Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`; EmuTOS-Stelle fuer alle: `references/emutos/bios/bios.c:1070-1080`.

- **BIOS-CORE-004 — S3 — CONFIRMED.** `Setexc(n,-1)` fuer CPU-Vektoren liefert NULL statt einer Handleradresse. Faelle: `bios-r1` 53 (`0x2`), 54 (`0x21`) NONZERO-DIFF; `bios-core-r1` 46–54 (`0x2, 0x3, 0x4, 0x1C, 0x21, 0x22, 0x2D, 0x2E, 0x45`) PTR_RAM-DIFF „is null“, beide Profile. Sonde S04/S05: Setz-/Ruecksetz-Zyklus gleich, nur der Startwert ist auf NewTOS 0. Lokalisiert: `tos-api/src/bios/vectors.rs:56@VECTORS` (nur `etv_*` vorbelegt). Spec: `docs/api/bios/05-setexc.md` „shadowed but not dispatched“.
- **BIOS-CORE-005 — S3 — CONFIRMED.** `Setexc(0x100/0x101/0x102, -1)` liefert 64-Bit-Platzhalter `0x4E75_0000_0000_0000` (`tos-api/src/bios/vectors.rs:45@ETV_DEFAULT_JUST_RTS`) und `0x4372_6974_0000_0000` (`tos-api/src/bios/vectors.rs:54@ETV_DEFAULT_CRITIC`); EmuTOS echte ROM-Adressen (`0x00E2A190` usw.). Faelle: `bios-core-r1` 55–57 PTR_RAM-DIFF „not < 0x80000000“, beide Profile (`bios-r1` 55–57 bestehen nur wegen NONZERO, siehe [BIOS13](BIOS13.md)). Sonde S01–S03: `old_ok=0`, `old_hi=4e750000`/`43726974`. Verstoss gegen MEM11s Invariante „jede gelieferte Adresse < 2 GiB“; ein 32-Bit-Feld (XBRA `xb_oldvec`) schneidet auf 0 ab.
- **BIOS-CORE-006 — S3 — CONFIRMED.** `Setexc(0x103 … 0x11F, -1)` liest den Schatten statt der Systemvariable. Fall: `bios-core-r1` 59 `Setexc(0x108,-1)` (`memvalid`, `$420`): EmuTOS `0x752019F3`, NewTOS `0` — EXACT-DIFF, beide Profile; Sonde S08. Gleich: 58 (`$40C`), 60–62 (`$5A0/$5A4/$5A8`, Seite-0-Zweig). Lokalisiert: `tos-api/src/bios/vectors.rs:21@NVEC` (Schatten `0x120` Eintraege) und `tos-api/src/bios/vectors.rs:98@setexc` (Seite-0-Zweig erst ab `NVEC`). Spec/TASK: `docs/api/bios/05-setexc.md` „Pi binding“ begruendet `0x120` mit „GEM hooks to `$047F`“ (solche Haken gibt es in EmuTOS nicht); `TASK/MEM12.md` „beliebige Zelle < 0x800“ fuer `$40C–$47F` falsch (siehe [BIOS13](BIOS13.md)).

Fehlerbild 004: XBRA-Installer sehen „kein Handler“, Programme mit NULL-als-Fehler brechen ab. Laeufe: `measurements/measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-core-r1`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b`.

## Lokalisierung (Lead, 2026-10-29)

- `setexc` → `tos-api/src/bios/vectors.rs@setexc` und `@note_write` (`mapped`);
  `--down`: keine weitere Funktion, vollstaendig.
- `--up --layers bios,bdos` (Wiedertestmenge): `biostrap` →
  `tos-api/src/bios/mod.rs@dispatch`; `osinit_before_xmaddalt` →
  `tos-api/src/bdos/mem/init.rs@init` (Startbelegung der Vektoren);
  `xterm`/`x0term` → `tos-api/src/bdos/process/pterm.rs@terminate`,
  `@terminate_zero` (setzen `etv_term` zurueck); `cgets`, `conbrk`, `xconin`
  (`tos-api/src/gemdos/console.rs`), `kill_program`
  (`kernel/src/arch/aarch64/exceptions.rs@kill_program`). `unmapped` in diesen
  Schichten: `bdos_trap2` ([PROC07](PROC07.md)), `bios_init`; dazu aes/vdi-
  Aufrufer (`retake`, `timer_init`, `timer_exit`) ohne Gegenstueck.
  Vollstaendig: nein, diese Liste.
- Zusaetzliche Wiedertests: ADR-031-Warnungen (`hooks.rs`, BIOS15) und
  MEM12 (Seite 0 nur im Super-Zustand beschreibbar), weil D3 Seite 0 fuer
  alle `num < 0x200` traegt.

## Arbeit

1. Je Befund Fix gegen `references/emutos/bios/bios.c:1070-1080`; Hosttests fuer Rueckgabe je Vektorklasse (CPU, `etv_*`, `$103–$11F`).
2. Spec zuerst, wo veraltet ([BIOS12](BIOS12.md)).
3. 005: jede gelieferte Adresse < 2 GiB (MEM11-Invariante) im Hosttest pruefen.

## Abnahme

- Bestaetigende Faelle: `bios-core-r1` 46–62 und `bios-r1` 53–57 EQUAL gegen das Orakel (TT und Falcon); Sonde S01–S08 gleich.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Entscheidungen des Anwenders 2026-10-28 (BIOS12)

- D1: Vorzeichenerweiterung bleibt (`tos-api/src/trap.rs:6@TrapArgs`); die
  Platzhalter von `Setexc(0x100…0x102, -1)` (Faelle 55–57) liefern mit D3
  echte Handler-Adressen.
- D2: `num >= 0x200` liefert 0 (dokumentierte Abweichung).
- D3: Seite 0 traegt alle `num < 0x200`; nicht bediente Vektoren zeigen auf
  den `just_rts`-Stub.
- P3, eigener Teilpunkt: der Abfragetest `v == -1` prueft die unteren 32 Bit
  wie EmuTOS (`references/emutos/bios/bios.c:1074`) — Sonde zuerst (Vorschlag
  in `docs/analysis/abi-assumptions-check-2026-10-28.md` §1).
- Code-Kommentare in `tos-api/src/bios/vectors.rs` mitziehen (BIOS13).
- Spec: `docs/api/bios/05-setexc.md`, Abschnitt „Decisions“.
