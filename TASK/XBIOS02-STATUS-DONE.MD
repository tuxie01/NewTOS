# XBIOS02 — Quittung: `Bioskeys` stellt die Ausgangstabellen wieder her

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-xbios02/](../measurements/measurement-2026-10-29-xbios02/RUNINFO.TXT)
· Ticket [XBIOS02](XBIOS02.md) · Spec `docs/api/xbios/18-bioskeys.md` · EmuTOS
`references/emutos/bios/ikbd.c:136-140` (`bioskeys`),
`references/emutos/bios/country.c:195-202` (`get_keytbl`).

## Aenderung

- `tos-api/src/xbios/mod.rs`: Arm `BIOSKEYS` (0x18) →
  `keytbl::bioskeys` (`tos-api/src/xbios/mod.rs:107@BIOSKEYS`); der Kommentar
  ueber den Kennungen (`tos-api/src/xbios/mod.rs:28-31@PHYSBASE`) nennt
  `Bioskeys` nicht mehr als „infrastructure does not exist yet“ (Punkt 2; der
  `Iorec`-Teil war mit XBIOS01 schon korrigiert).
- `tos-api/src/xbios/keytbl.rs`: `bioskeys` (`:237@bioskeys`) vergisst die von
  `Keytbl` gemerkten Zeiger (`OVERRIDE`), laesst den Uebersetzer die
  Ausgangstabelle zuruecksetzen (`Host::reset_key_tables`) und schreibt die
  `struct keytbl` neu; `publish` (`:251@publish`) ist der gemeinsame Schluss von
  `Keytbl` und `Bioskeys`.
- `tos-api/src/host.rs`: `Host::reset_key_tables` (`:278@reset_key_tables`),
  Vorgabe ohne Wirkung (Brett ohne Uebersetzer), wie `set_key_table`.
- `kernel/src/keymap.rs`: `DEFAULT` (`:210@DEFAULT`) = die beim Start gewaehlte
  Tabelle (`KEYBOARD.KBD`, sonst `#K`, sonst US), geschrieben nur von der
  Startwahl (`install_power_up`, `:248@install_power_up`, aus `select_builtin`
  und `load_kbd`); `restore_default` (`:264@restore_default`) kopiert sie samt
  AltGr-Paaren zurueck und setzt die von `Ssystem(S_NEWTOS_KEYBOARD)` gemeldete
  Quelle zurueck. Gegenstueck zu EmuTOS' `get_keytbl()`, dessen Ergebnis
  ebenfalls nach dem Start feststeht (`cookie_akp` nur von `detect_akp`
  geschrieben, `references/emutos/bios/country.c:162`).
- `kernel/src/host_env.rs`: `reset_key_tables` → `keymap::restore_default`.
- Rueckgabewert (`void`): die Adresse der zurueckgesetzten Struktur (EmuTOS:
  `d0` = `get_keytbl()`, Adresse der ROM-Struktur, gemessen `0x00E06E8C`; aus
  `references/emutos-oracle/1a7484ab/obj/ikbd.o` gelesen). Spec „Return value“
  und „NewTOS implementation notes“. **Zur Bestaetigung beim Lead/Anwender:**
  Alternativen waeren 0 (wie andere `void`-Aufrufe) oder eine eigene,
  unveraenderliche Ausgangsstruktur im Adressraum des Aufrufers.

## Spec / Doku

- `docs/api/xbios/18-bioskeys.md`: `get_keytbl` genauer (Auswahl ueber
  `cookie_akp`, nach dem Start fest), „Return value“ mit Disassembly und
  Messung, Verifikationskriterien mit den Orakelwerten, Edge cases 2/3,
  Implementierungsteil und Pi-Bindung neu („Step-2 — not wired“ entfernt).
- `docs/api/xbios/10-keytbl.md`: Satz „Still not wired: Bioskeys“ ersetzt.
- `docs/api/xbios/_full-function-inventory.md`: Zeile 0x18.
- `docs/COMPATIBILITY.md` (Tastaturlayout-Abschnitt): `Bioskeys` stellt die
  Startwahl wieder her.
- `TASK/FS22.md:31`: Zeilenanker in `kernel/src/keymap.rs` nachgezogen (durch
  diese Aenderung verschoben).
- `docs/analysis/deps/mapping.tsv`: `bioskeys` → `keytbl.rs bioskeys` und
  `keymap.rs restore_default`; `get_keytbl` → `restore_default`;
  `get_kbd_index` → `init_from_disk`; `get_country_record` →
  `select_builtin`; neue Zeile `kbd_init` → `init_from_disk` (sein
  abschliessendes `bioskeys()`). `cargo xtask deps --check-mapping`: alle
  1046 Zeilen loesen auf. `deps bioskeys --down`: noch `unmapped`
  `get_current_country_input`, `get_default_country` (im Ticket gelistet);
  `--up`: `bios_init`, `biosmain` wie vorher.

## Abnahme

- Sonde (Binaerdateien des Audits unveraendert): K08 `unshift_is_n0=1
  shift_is_s0=1 caps_is_c0=1 unshift_is_my=0`, K09 Ausgangstabellen (US-CRCs
  gleich dem Orakel), K10 gleich; K07/K11 Zeiger statt `0x18` (siehe oben).
  XCKTNL N03 `unshift_tosbyte=1` wie das Orakel. Lauf mit `#K de`: K09 = die
  deutschen Tabellen (Startwahl).
- Eingabesonde: X01 `unshift_is_my=0 unshift_is_n0=1`, X02 `0x001E004B`,
  X03 `0x001E0061` — gleich dem Orakel.
- Punkt 3: nach dem Ende von XCKTNL liefert `a` im Desktop vorher `0x1E00`,
  nachher `0x1E61` (Schlusswort-Log im Kern, nur Scratch).
- Kein Rueckschritt: `bios-r1`, `xbios-chardev-r1`, `bios-core-r1`,
  `xbios-system-r1` je gegen tt und falcontc, vorher (`git archive 3bbb8fd3`,
  `--tree`) und nachher: in allen acht Vergleichen dieselben Faelle
  verschieden, kein neu abweichender Fall.

## Tests, Bau

- Neuer Hosttest `tos-api/tests/xbios02_bioskeys.rs` (`Keytbl(my_n)`,
  `Keytbl(-1,my_s,my_c)`, `Bioskeys()`: alle drei Zeiger, die Tabelleninhalte,
  AltGr, `dead`/`features` und der Uebersetzer wieder auf der Ausgangstabelle);
  ohne den `BIOSKEYS`-Arm schlaegt er fehl (Rueckgabe 24).
- `cargo xtask test` (voll): 1843/0/1 (Basis 1842/0/1 + 1).
- Bau rpi4 und qemu-virt (Debug), rpi4 release: gruen. Clippy
  `-p kernel` (beide Boards) 195 Meldungen (kernel 121, tos-api 74),
  `-p tos-api` 74, je Datei gleich der Basis (`git archive 3bbb8fd3`); keine
  Meldung in den beruehrten Dateien.
- raspi4b: alle Laeufe bis „desktop up“; qemu-virt Debug bis
  „desktop up (0 drives)“.

## Offen / Funde

- `--features aes-trace` linkt in einem Release-Bau nicht mehr („kernel image
  + stack overruns STRAM_OS_RESERVE (membot)“), vorher wie nachher — Werkzeugbefund.
- `docs/api/xbios/_full-function-inventory.md`: Zaehlung und Liste der nicht
  beantworteten Nummern (Abschnitt Summen) sind seit XBIOS01/BIOS15 veraltet;
  nur die Zeile 0x18 nachgezogen.
- Echte Pi 4: nicht gemessen.
