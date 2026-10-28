# TOOL11 — Quittung: `PTR_RAM` mit Untergrenze 0x400, Laeufer mit denselben Grenzen

Datum 2026-10-28 · Beleg
[measurements/measurement-2026-10-28-tool11/](../measurements/measurement-2026-10-28-tool11/RUNINFO.TXT)
· Vorschlag `docs/analysis/ptr-ram-rule-2026-10-28.md` · Ticket [TOOL11](TOOL11.md)

- `TD_PTR_LOW 0x400` (`sysvars_start`, `references/emutos/tosvars.ld:31`) in
  `tosdump/core/record.h`; eine Pruefung `td_ptr_ram_violation` fuer
  tosdiff und Laeufer (null, nicht unter `0x80000000`, unter `0x400`,
  ungerade). Der Laeufer folgt einem Rueckgabezeiger (`r:`) nur noch, wenn
  er die Pruefung besteht; ein 64-Bit-Platzhalter faellt den EL0-Dumper
  nicht mehr an. Neue Backend-Funktion `td_abi_addr` (auf beiden Maschinen
  die Identitaet), damit der Host-Test eine TOS-Adresse abbilden kann.
- Tests erst rot, dann gruen: `test_tosdiff.sh` (0x0E, 0x3FE, 0x3FF
  abgelehnt, 0x400 angenommen), `test_runner_disk.c` (`test_ret_bounds`),
  `test_selfcheck.sh` mit Negativkontrolle (Iorec-Record auf 0x0E gepatcht
  wird abgelehnt). Selfcheck: 25 Referenzen bestehen.
- Neuvergleich aller NewTOS-Dumps: neu abweichend genau `bios-r1` 98–100
  (`Iorec` 0x0E), 184 (`Bconmap(-2)` 0x2C) — `tt` 42 → 46, `falcontc`
  46 → 50 — und `xbios-chardev-r1` 21–23, 44; `bios-core-r1`,
  `xbios-system-r1`, `bios-r2` unveraendert.
- Kanarienvoegel §6 (`tt`, `falcontc`) und Platte OK; `bios-r2` am Orakel
  mit dem geaenderten Dumper bytegleich zur Referenz. Keine Referenz muss
  neu aufgenommen werden.
- `make -C tosdump host-test` gruen, `cargo xtask test` 1745/0/1 (Lead-
  Gegenlauf gleich); doc-check der geaenderten Doku 0 Fehler.
