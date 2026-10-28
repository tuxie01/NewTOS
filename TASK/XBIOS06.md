# XBIOS06 — `Rsconf(-1 x6)`: TSR-Byte ohne Bit 7, obere 32 Bit null statt vorzeichenerweitert

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (Erweiterung) + Researcher (Spec gegen ABI, Geraeteprofil der Registerbytes) — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-xbios-chardev.md`

## Sachverhalt

**XBIOS-CHARDEV-006 — S3 — CONFIRMED.** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT`.

- Faelle: `bios-r1` 105, `xbios-chardev-r1` 31 — EXACT-DIFF: TT `0xFFFFFFFF_88018100`, Falcon `0xFFFFFFFF_88000000` (SCC), NewTOS `0x00000000_88010100` (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt`).
- Sonde S01–S18: Baud setzen/abfragen, `-1` behaelt, `16`/`-3` ausserhalb, UCR/RSR setzen und zuruecklesen, Maskierung, Wiederherstellung sind gleich; nur das TSR-Byte (`81` gegen `01`) und die obere Haelfte weichen ab (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-NEWTOS-raspi4b`).
- Lokalisiert: `tos-api/src/xbios/system.rs:248@RS_TSR` (Schatten `0x01`) und `tos-api/src/xbios/system.rs:263-295@rsconf` (`old as i64`, nullerweitert).
- EmuTOS-Stelle: `references/emutos/bios/serport.c:217-249` (`rsconf_mfp` liest die MFP-Register; TSR Bit 7 = „Puffer leer“).
- Spec: `docs/api/xbios/0f-rsconf.md` sagt „zero-extended into `x0`“ — widerspricht `tos-api/src/trap.rs` („sign-extended `i64`“) und `docs/TOS-REFERENCE-DUMPER.md` §3 (`ret_hi`). Die Registerbytes sind Geraeteparameter (TT ≠ Falcon); ohne COMPATIBILITY-Eintrag DIFF.
- Fehlerbild: LP64-Programm, das `long r = Rsconf(-1,…)` als negativ erwartet, oder das TSR-Bit „Sender leer“ abfragt.

## Arbeit

1. Researcher: Spec `docs/api/xbios/0f-rsconf.md` gegen das Trap-ABI korrigieren; Geraeteprofil der Registerbytes (TT/Falcon) entscheiden.
2. Developer: Vorzeichenerweiterung und TSR-Bit 7 gemaess Entscheidung; Hosttest je Rueckgabe.

## Abnahme

- Bestaetigende Faelle: `bios-r1` 105 und `xbios-chardev-r1` 31 EQUAL gegen das Orakel (bei Geraeteprofil-Entscheidung: DIFF nur in den Registerbytes, mit COMPATIBILITY-Eintrag ACCEPTED); Sonde S01–S18 gleich.
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.
