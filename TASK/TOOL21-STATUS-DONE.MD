# TOOL21 — Quittung: ADR-Beleg mit Fundstelle, Gruppe A als Hardware, AUX-Pfad nach XBIOS15

Datum 2026-10-29 · Ticket [TOOL21](TOOL21.md) · Werkzeug
`tools/xtask/src/deps.rs`, Zuordnung `docs/analysis/deps/mapping.tsv`,
Beschreibung `tools/deps/README.md` (Abschnitte "Classes", "Checks")

## 1. Belegform `design:ADR-NNN@<datei>:<zeile>`

- `parse_class` liest nur noch die Form mit Fundstelle (`ADR-NNN` ohne
  `@<datei>:<zeile>` ist ein Fehler; Zeile >= 1, ohne fuehrende Null).
- `check_mapping` liest die genannte Datei und prueft mit
  `adr_section_at`, dass die Zeile die Ueberschrift `## ADR-NNN:` ist oder
  in deren Abschnitt liegt. Der Abschnitt reicht bis zur naechsten
  `#`-/`##`-Ueberschrift; `###` und Codezaeune beenden ihn nicht.
  Meldungen: Datei fehlt; Zeile liegt in einem anderen ADR; Zeile liegt in
  keinem ADR-Abschnitt (auch hinter dem Dateiende). Die alte Pruefung
  "ADR ist eine Ueberschrift von `docs/MEMORY.md`" ist darin aufgegangen.
- Alle 20 ADR-Zeilen umgestellt, Zeilen geprueft (unveraendert):
  ADR-012 `docs/MEMORY.md:295` (1), ADR-017 `:824` (7), ADR-030 `:2823`
  (3), ADR-031 `:3041` (9).
- Verschiebt sich ein ADR in `docs/MEMORY.md`, wird der Test rot; die
  Zeilennummer wandert dann mit (README).

## 2. Gruppe A → `design:hardware:<name>` (78 Zeilen)

Anwenderentscheidung 2026-10-29: absichtliches Fehlen der Hardware, auch
fuer Trap-Plaetze. Die README-Regel ("ein Trap-Platz ist nie allein
`hardware:`") nennt diese Entscheidung jetzt als Ausnahme.

| Geraet | Zeilen | Funktionen |
|---|---|---|
| `dsp56001` | 43 | XBIOS 0x60-0x7F (32) + LOD-Lader `convert_lod filesize handle_DATA hex inword iseol load_sub outword skipspaces skiptoeol update_dspvect` |
| `dma-sound` | 18 | XBIOS 0x80-0x8D (14) + `devconnect_falcon devconnect_ste sndcmd_falcon sndcmd_ste` |
| `video-shifter` | 8 | XBIOS 0x50-0x57 `esetshift egetshift esetbank esetcolor esetpalette egetpalette esetgray esetsmear` |
| `videl` | 9 | XBIOS 0x58-0x5F `vsetmode vmontype vsetsync vgetsize vsetrgb vgetrgb vfixmode` + `get_videl_mode lookup_videl_mode` |

Geraetenamen wie die schon vorhandenen `design:hardware:`-Zeilen derselben
Dateien. `xbios_vecs[0x5c]` = `xbios_unimpl` bleibt `mapped` (Default-Arm).

## 3. AUX-Pfad (Gruppe D, 16 Funktionen → 21 Zeilen)

Gelesen gegen `bios/serport.c` (Pin 1a7484ab) und das XBIOS15-Ergebnis.

`mapped` (8 Definitionen):
- `bconout1`, `bconoutB` → `kernel/src/console.rs@bconout` (AuxSerial,
  qemu-virt) und `tos-api/src/bios/chardev.rs@bconout` (AuxNoCable: Rumpf,
  rpi4-Eintrag; Ausgabe wird verworfen, EmuTOS sendet).
- `bcostat1`, `bcostatB` → dieselben `bcostat` (immer bereit, -1).
- `bconinB`, `bconstatB` → `kernel/src/console.rs@bconin`/`bconstat`
  (Falcon-Profil: AUX = SCC B, `init_bconmap` maptable[1]; wie SERRX-004).
- `rsconf1` → `tos-api/src/xbios/system.rs@rsconf` (rsconf1 → rsconf_mfp,
  der Vertrag, den NewTOS ueber Schatten umsetzt).
- `init_serport` → `tos-api/src/xbios/iorec.rs@init` (Page::init: Satz von
  Geraet 0 = iorec_init; SCC-/TT-MFP-Haelfte ohne Hardware).

`unmapped` mit neuer Notiz:
- `mfp_rs232_tx_interrupt`, `mfp_rs232_tx_interrupt_handler`,
  `scc_tx_interrupt_handler`, `sccb_tx_interrupt`, `put_iorecbuf`: NewTOS
  hat keinen AUX-Ausgabering und keinen Sende-Interrupt; Ausgabe wird
  sofort verworfen.
- `rsconfB`: NewTOS-Rsconf folgt rsconf_mfp, nicht rsconf_scc.
- `rsconf_dummy`: keine Bconmap-maptable (Bconmap liefert 0x2c).
- `init_bconmap`: zwei Zeilen — `mapped` → `kernel/src/console.rs@install_aux`
  (Einsetzen in Slot 1), `unmapped` fuer die maptable/bconmap_root-Haelfte;
  die Definition zaehlt damit als `unmapped`.

## 4. Abdeckung (`cargo xtask deps --coverage`)

| Schicht | vorher mapped / design / unmapped | nachher |
|---|---|---|
| bios (789) | 161 / 399 / 229 | 169 / 477 / 143 |
| bdos (173) | 152 / 2 / 19 | unveraendert |
| aes, desk, include, vdi | 0 / 0 / alles | unveraendert |
| gesamt (2014) | 313 (15,5 %) / 401 (19,9 %) / 1300 (64,5 %) | 321 (15,9 %) / 479 (23,8 %) / 1214 (60,3 %) |

## 5. Pruefung

- Neue Hosttests: `deps::tests::adr_section_at_finds_the_adr_around_a_line`
  (Ueberschrift, Abschnitt, `###`, Codezaun, Nicht-ADR-Ueberschrift,
  Dateiende), `deps::tests::check_mapping_checks_the_adr_line` (gute und
  schlechte Zeilen gegen das echte `docs/MEMORY.md`, Zeilen zur Testzeit
  ermittelt). Angepasst: `parse_class_reads_the_three_classes_and_the_evidence_forms`,
  `check_mapping_reports_each_kind_of_class_problem`.
- `cargo xtask test` (voll): 1823 passed, 0 failed, 1 ignored (Basislinie
  1821/0/1 plus 2 neue).
- `cargo xtask deps --check-mapping`: 1037 Zeilen, alles aufgeloest.
- `cargo clippy --tests` (xtask): keine Warnung in `deps.rs`; die 8
  vorhandenen liegen in anderen Dateien (unveraendert).
- `cargo xtask doc-check tools/deps/README.md docs/analysis/deps/mapping.tsv TASK/TOOL21-STATUS-DONE.MD`:
  0 Fehler.
- Kernel/tos-api nicht beruehrt; kein Boardbau noetig.
