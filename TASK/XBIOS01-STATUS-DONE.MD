# XBIOS01 — Quittung: `Iorec` verdrahtet, der IKBD-`IOREC` ist die Tastenschlange (schliesst BIOS11)

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-xbios01/](../measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT)
· Spec `docs/api/xbios/0e-iorec.md` (Vertrag F1-F4, Entscheidungen E1-E6, neuer
Abschnitt "NewTOS implementation") · Ticket [XBIOS01](XBIOS01.md), mit
[BIOS11](BIOS11.md)

- **`Iorec`** (`tos-api/src/xbios/iorec.rs:454@iorec`, Arm
  `tos-api/src/xbios/mod.rs:107@IOREC`): `0/1/2` liefern Zeiger auf die
  Records, jedes andere Geraet `-1` (vorzeichenbehafteter `WORD`). Der
  Kommentar „intentionally NOT wired“ in `tos-api/src/xbios/mod.rs` ist
  richtiggestellt (Arbeit Punkt 3).
- **Ort (E1):** eine gemeinsame Seite `0x7D000`, in jedem Prozess EL0
  les-/schreibbar, nie ausfuehrbar
  (`tos-api/src/bdos/process/mmu.rs:264@IOREC_PAGE_VA`, eingetragen in
  `init_slot_tables`, von `map_shared` ausgenommen, `private_page_alias`
  loest sie auf). Layout LP64-`_IOREC` 24 Byte, Geraet 0 56 Byte, Werte wie
  das Orakel (E5).
- **Schlange (E1):** `RX_RING` entfaellt. Dekoder, Wiederholung, qemu-virt-
  Seriell, `Bconstat(2)`/`Bconin(2)`, GEMDOS-Konsole, AES, `Fread` auf
  `CON:` gehen ueber den IKBD-Ring (`push_key`/`pop_key`/`key_pending`,
  Schrittweite 4, 63 Eintraege, voll → verworfen, IRQs maskiert).
- **Eintrag (E2, BIOS11):** `shifty<<24|scancode<<16|ascii`, Shift zum
  Push-Zeitpunkt (Taste und Wiederholung); `pack_console_key` maskiert Bits
  24-31 ohne `conterm & 8`. `shift_fold` entfernt. Host-Schnittstelle:
  `console_read_entry_blocking`/`_restartable` (Default aus dem Wort mit
  Shift 0).
- **Misstrauen (E1, E4 (a)):** Pruefung vor jeder Nutzung und nach jedem Trap
  (`kernel/src/console.rs:267@check_iorec_after_trap`); ersetztes
  `buf`/`size` → Kernelpuffer zurueck, Ring leer; Indizes ausserhalb → Ring
  leer; je Reparaturart eine Warnung `[iorec] warning: …` je Prozess
  (Ruecksetzen bei Prozessende). Debug-Asserts nur auf Kernel-Invarianten
  (validierte Indizes), nie auf Programmdaten.
- **E3 (a)** dokumentiert (Spec, `docs/COMPATIBILITY.md` neuer Eintrag
  „`Iorec`“). **E6:** `kbdvec`-Haken bekommt `x1` = `Iorec(1)`
  (`docs/api/xbios/22-kbdvbase.md` D2a/O2 nachgetragen).
- **Layout-Pruefung:** `sdk/libc/tests/iorec-layout.c` (+ `.sh`, in
  `sdk/run-host-tests.sh`): `_Static_assert` von `offsetof`/`sizeof` des
  SDK-`_IOREC` und des Geraet-0-Records gegen F1; Gegenprobe mit falschen
  Offsets schlaegt fehl. SDK-Hosttests 27/0/0.
- **Ratsche:** `docs/analysis/deps/mapping.tsv` Zeilen fuer `iorec`
  (`iorec`, `record_va`), `push_ikbdiorec` (`push_key`, `push_rx`),
  `bconstat2` + `key_pending`, `bconin2` + `pop_key`; Zeile `shift_fold`
  entfernt.

## Abnahme

- `xbios-ptr-r1` Faelle 24-31: vorher DIFF, jetzt EQUAL gegen tt und
  falcontc (nur 34 `Getmpb` = BIOS06 bleibt). `bios-r1` 101/102 (und
  98-100) und `xbios-chardev-r1` 24-27 (und 21-23): EQUAL beidseitig.
- Kein vorher gleicher Fall in `xbios-ptr-r1`, `bios-r1`, `xbios-chardev-r1`,
  `bios-core-r1`, `xbios-system-r1` neu verschieden (zehn Vergleiche, vorher
  = `git archive 00b7c9cd` gemessen;
  `measurements/measurement-2026-10-29-xbios01/runs/before-after.txt`).
- **Flush mit Tasten im Ring:** neue Tabelle `tosdump/cases/xbios-flush-r1.tsv`
  (207 Faelle, CRC `965AE625`); drei Tasten vor dem Flush — Orakel ueber
  Hatari `--cmd-fifo` (`hatari-event keydown/keyup`, `--fast-forward off`),
  NewTOS ueber den Scratch-Injektor (`tools/inject-arm.py`,
  `launch:DUMPBIOS`). Orakel tt/falcontc je zweimal bytegleich; NewTOS 0 von
  207 verschieden gegen beide: `Bconstat(2)` -1 vor, 0 nach dem Flush, head 0
  / tail 12 vor, 12/12 nach. Referenzen
  `tosdump/reference/EMUTOS-1a7484ab-XBIOS-flush{,-falcontc}.{BIN,LOG}`.
- XCIOFL: kein Bus Error, `F02 after write hd=0 tl=0` wie EmuTOS. Sonde
  I01-I08 gleich bis auf Adressen und `sizeof` 24 (LP64, F1).
- BIOS11/K16: `0x02300042` wie EmuTOS (vorher `0x00300042`); K11-K15, K20
  gleich.
- Retest: `Fread(0,32)` auf `CON:` liest injiziertes `ab` + Return (2 Byte);
  Desktop mit Menueleiste, Klick + `Ctrl+I` oeffnet „DISK INFORMATION“,
  Return schliesst (Screendumps im Lauf); qemu-virt bootet bis „desktop up“.
- `cargo xtask test` 1783/0/1 (Basis 1767/0/1, +9 Unit, +7 Integration
  `tos-api/tests/xbios01_iorec.rs`); `make -C tosdump host-test` gruen (34
  Referenzen). Build beide Boards ohne neue Warnung (159 Warnzeilen wie
  Basis), Clippy kernel (beide Boards) 121 / tos-api 74 wie Basis.

## Offen / nicht geprueft

- Echte Pi 4 (USB-Tastatur, K16 mit echter Umschalttaste) — Lead/Anwender.
- Eine EL0-Programmseite, die `buf`/`size` ersetzt oder Indizes zerstoert, nur
  per Hosttest; die Warnzeile ist in keinem Lauf aufgetreten.
- Beobachtungen ausserhalb des Auftrags (Spec, "Out-of-scope observations"):
  `Rsconf` schreibt `baudrate`/`flowctrl` nicht in den Geraet-0-Record (EmuTOS
  `bios/serport.c:222-240` schon); qemu-virt-Seriell schiebt Scancode 0 statt
  EmuTOS' Tabellen-Suche (`push_ascii_ikbdiorec`).
