# PROC07 — `trap #2` mit `d0.w = 0` beendet den Prozess wie `Pterm0`

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: undokumentierte CP/M-Reliquie; gemlib sendet nur Opcodes != 0) · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-29 (Empfehlung des Leads: wie EmuTOS), aus [TOOL17](TOOL17.md) Gruppe F

## Sachverhalt

- EmuTOS: `bdos_trap2` (`references/emutos/bdos/rwa.S:135-149`) prueft `d0.w`;
  0 springt nach `oterm` = `Pterm0` („undocumented feature of Atari TOS,
  apparently a relic of CP/M 68K“), sonst weiter an den VDI/AES-Handler.
- NewTOS: `trap #2` mit `d0 = 0` faellt in `kernel/src/arch/aarch64/exceptions.rs:575@gpr`
  in den Restzweig, liefert `x0` unveraendert zurueck, der Prozess laeuft weiter.

## Arbeit

0. Lokalisierung (Lead, vor Beginn) nach CLAUDE.md „Testing“; Spec
   `docs/api/vdi/_dispatcher.md` bzw. GEMDOS-`Pterm0` um den Fall ergaenzen.
1. Fix nach EmuTOS; Hosttest.
2. Fall beidseitig (Orakel/NewTOS): Programm ruft `trap #2` mit `d0 = 0`,
   Rueckgabewert beim Elternprozess 0.

## Abnahme

- Fall gleich dem Orakel; `cargo xtask test` gruen; `mapping.tsv` `bdos_trap2`
  `mapped`; STATUS im selben Commit.

[Gemeinsame Regeln](README.md) gelten.
