# SDK03 — mintlib `sysinfo(SI_PLATFORM)`: NULL-Pruefung und fehlendes Komma

Status: DONE (2026-11-11) | Priority: P2 | Commit: 1b28ba371 | Receipt: SDK03-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/SDK03-STATUS-DONE.MD exists, commit 1b28ba371, TASK/SDK03-STATUS-DONE.MD, commit 1b28ba371 (fix(sdk): mintlib sysinfo(SI_PLATFORM) for _MCH 0xFFFFFFFF (SDK03)), fix/feat commits naming the id: 1b28ba371 (n=1), the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-29 ([Quittung](SDK03-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: Developer (SDK) — Anwenderentscheidung 2026-10-28, aus [ARCH05](ARCH05.md)

## Sachverhalt

- `sdk/libc/mintlib/unix/sysinfo.c:515-562` waehlt den Plattformnamen aus
  `_MCH` (abgeleitet, nicht ausgefuehrt;
  `docs/analysis/machine-identity-adr-proposal-2026-10-28.md`, Abschnitt 2):
  - `0xFFFFFFFF` (ab ARCH06 der NewTOS-Wert) laesst `platform` ohne ColdFire
    `NULL`, danach `strlen(NULL)`;
  - in `platforms[]` fehlt bei `"cfevb"` ein Komma, eine Familie ab 6 liest
    ausserhalb des Feldes.

## Arbeit

1. Korrektur als benannte Patch-Stelle im SDK-libc-Overlay
   (`sdk/libc/overlay/`), Upstream unveraendert: NULL-Pruefung (Name fuer
   „keine Atari-Hardware“ nach Vorbild EmuTOS `machine_name()`,
   `references/emutos/bios/machine.c:807-821`) und das Komma.
2. Test im SDK-Hostlauf (`sdk/run-host-tests.sh`) fuer `_MCH` =
   `0xFFFFFFFF`, eine Familie ab 6 und die bekannten Werte.
3. Upstream-Bericht optional: Text im Bericht vorschlagen, nicht senden.

## Abnahme

- SDK-Hosttests gruen (Basislinie 26/0/0 plus neue); der Test schlaegt ohne
  den Patch fehl (belegt).
