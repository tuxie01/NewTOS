# TOOL17 — Quittung: Klassen in `mapping.tsv`, Abdeckung je Schicht

Datum 2026-10-29 · Ticket [TOOL17](TOOL17.md) · Werkzeug
`tools/xtask/src/deps.rs`, Beschreibung `tools/deps/README.md`
(Abschnitte "Classes", "Coverage and the ratchet", "Checks")

- **Format:** neue Spalte `class` vor `note`
  (`emutos_fn emutos_file newtos_file newtos_fn class note`), im Kopf von
  `docs/analysis/deps/mapping.tsv` beschrieben. Werte `mapped`,
  `unmapped`, `design:<beleg>` mit Beleg `ADR-NNN` (Ueberschrift
  `## ADR-NNN:` in `docs/MEMORY.md`), `hardware:<geraet>` (Atari-Hardware
  ohne Gegenstueck auf der Pi) oder `source:<pfad>` (anderer benannter
  Grund, Datei im Repo; derzeit ungenutzt). Das Praefix `no counterpart: `
  der alten `-`-Notizen ist entfallen.
- **Regeln der Einteilung:** `hardware:` nur fuer Code, der Atari-Hardware
  ansteuert, erkennt oder einrichtet. Ein Trap-Tabellenplatz (TOS-API) ist
  nie allein wegen fehlender Hardware `design`: der Aufruf existiert auch
  ohne Geraet, also `unmapped`, bis ein ADR entscheidet; ebenso reine
  Software-Helfer nur solcher Plaetze (DSP-LOD-Lader). Alles ohne Beleg ist
  `unmapped`, auch "no NewTOS function found" und Gegenstuecke, die es in
  NewTOS gibt, die aber noch keine Zeile haben.
- **Klasse je Definition** (`fn` + Datei): `unmapped`, wenn eine Zeile es
  ist, sonst `mapped`, wenn eine Zeile es ist, sonst `design`. Ohne Zeile
  (Schichten `vdi`, `aes`, `desk`, `include`; `ext` in Abfragemengen):
  `unmapped`.
- **`cargo xtask deps --coverage`:** Tabelle je Schicht (Zahl, Anteil),
  danach die `design`-Zeilen nach Beleg. **`--up`/`--down`:** neue Spalte
  `class` je Funktion, am Ende dieselbe Tabelle fuer die Menge, die
  beruehrten Schichten (Standard: die Schichten der Wurzel; `--layers l,..`
  waehlt), deren `unmapped`-Funktionen und das Urteil
  `complete: yes|no`.
- **Pruefung** (`deps::tests::mapping_rows_exist_on_both_sides` ueber
  `check_mapping`): Klasse lesbar; `mapped` nennt eine NewTOS-Funktion,
  `design`/`unmapped` nicht; ADR existiert; `source:`-Pfad existiert.
  Neue Hosttests: `parse_class_reads_the_three_classes_and_the_evidence_forms`,
  `check_mapping_reports_each_kind_of_class_problem`,
  `coverage_counts_definitions_per_layer`,
  `report_lists_unmapped_of_the_touched_layers_and_the_verdict`,
  `up_blkdev_rwabs_lists_its_unmapped_bios_functions` (Abnahmebeispiel
  BIOS01).

## Abdeckung

Vorher (nur Gegenstueck ja/nein messbar): `bios` 157 mit Gegenstueck,
632 `-`; `bdos` 152 / 21; `vdi`, `aes`, `desk`, `include` ohne Zeilen.

Nachher (`cargo xtask deps --coverage`, Definitionen):

| Schicht | Def. | mapped | design | unmapped |
|---|---|---|---|---|
| aes | 418 | 0 (0,0 %) | 0 | 418 (100 %) |
| bdos | 173 | 152 (87,9 %) | 2 (1,2 %) | 19 (11,0 %) |
| bios | 789 | 161 (20,4 %) | 399 (50,6 %) | 229 (29,0 %) |
| desk | 325 | 0 | 0 | 325 (100 %) |
| include | 59 | 0 | 0 | 59 (100 %) |
| vdi | 250 | 0 | 0 | 250 (100 %) |
| gesamt | 2014 | 313 (15,5 %) | 401 (19,9 %) | 1300 (64,5 %) |

Neu gelesene `-`-Zeilen mit Gegenstueck (jetzt `mapped`):
`mfp_rs232_rx_interrupt` und `sccb_rx_interrupt` (`bios/vectors.S`) →
`kernel/src/drivers/gic.rs@rust_irq_handler` (PL011-Arm, XBIOS15);
`supexec` → `kernel/src/arch/aarch64/exceptions.rs@handle_supexec` (alte
Zeile sagte "not implemented", seit MEM12 S6 falsch); `nvmaccess` →
`tos-api/src/xbios/mod.rs@dispatch` (Funktionsnummer-Arm = EmuTOS ohne
NVRAM, `bios/nvram.c:161-162`, ADR-012 Nachtrag).

## Unklare Faelle (alle `unmapped`, Entscheidung Lead/Anwender)

- **A. API-Plaetze fuer Hardware, die die Pi nicht hat:** DSP XBIOS
  0x60-0x7F (32 Plaetze) plus LOD-Lader (`convert_lod filesize handle_DATA
  hex inword iseol load_sub outword skipspaces skiptoeol update_dspvect`);
  DMA-Sound XBIOS 0x80-0x8D (14) plus `devconnect_falcon devconnect_ste
  sndcmd_falcon sndcmd_ste` (Ton gehoert laut Fahrplan zu 1.0); TT-Shifter
  XBIOS 0x50-0x57 (`esetshift egetshift esetbank esetcolor esetpalette
  egetpalette esetgray esetsmear`); VIDEL XBIOS 0x58-0x5F (`vsetmode
  vmontype vsetsync vgetsize vsetrgb vgetrgb vfixmode`) plus
  `get_videl_mode lookup_videl_mode`; GEMDOS 0x15 `srealloc`.
- **B. Programmierschnittstellen ohne Bus:** XHDI (22, `bios/xhdi.c`),
  SCSIDRV (16, `bios/scsidriv.c`).
- **C. Ersetzte EmuTOS-Interna ohne ADR:** OS-Pool `getosm osmem_init
  xmgetblk xmfreblk` (ADR-025 ersetzt nur die MD-Versorgung); DND-Baum
  `free_available_dnds freednd getdnd makdnd process_dnd_tree snipdnd
  makofd dopath`.
- **D. AUX-Pfad (PL011/AuxNoCable vorhanden, keine Zeile):** `bconout1
  bcostat1 rsconf1 mfp_rs232_tx_interrupt mfp_rs232_tx_interrupt_handler
  bconinB bconoutB bconstatB bcostatB rsconfB scc_tx_interrupt_handler
  sccb_tx_interrupt put_iorecbuf init_serport init_bconmap rsconf_dummy`.
- **E. Maschine/Cookies:** `setvalue_vdo` (ARCH06 offen), `setvalue_mch`
  (`_MCH` gesetzt in `tos-api/src/cookie.rs`), `machine_detect
  machine_init guess_machine_name add_cookie_frb total_alt_ram
  altram_init`.
- **F. Start/Laufzeit mit NewTOS-Gegenstueck im Kern, nicht zugeordnet:**
  `bios_init biosmain bmem_init balloc_stram os_header osxhbootdelay
  screen_init_address calc_vram_size set_rez_hacked int_vbl
  flush_data_cache invalidate_data_cache invalidate_instruction_cache
  panic shutdown can_shutdown time_init bdos_trap2 run_auto_program`.
- **G. Ermessen, als `design` eingetragen** (zur Kenntnis): NatFeats
  (`hardware:natfeats`, 20 Zeilen: Schnittstelle des Emulator-Wirts, auf
  der Pi wie auf echter Atari-Hardware nicht vorhanden); `rez_changeable`,
  `check_moderez`, `atari_setrez`, `videl_setrez` u. a. (`ADR-017`, kein
  Aufloesungswechsel zur Laufzeit; `xbios/screen.rs@setscreen` ignoriert
  `rez`); `jdisint`, `jenabint`, `call_etv_critic`, `default_etv_critic`,
  `clockvec`, `call_mousevec`, `ikbdsys`, `midisys`, `midivec`
  (`ADR-031`); `setvalue_snd` (`ADR-012`).

Uebrige `unmapped` ohne Design-Verdacht (Gegenstueck nicht gefunden):
`blkdev.c` (14), `disk.c` (16), `country.c` (8), `initinfo.c` (10),
`kprint.c` (7), `conout.c` (4), `delay.c`/`delayasm.S` (3), `font.c` (2),
`vt52.c cursor_down_impl`, `ikbd.c bioskeys convert_scancode
handle_mouse_mode kb_timerc_int`, `lineainit.c set_screen_shift`,
`screen.c get_pixel_size screen_get_current_mode_info`, `bdos`
`get_default_handle namlen log2ul`, `is_text_pointer`.

## Retest-Mengen neu (`--up`, `--layers bios,bdos`, Vereinigung der Wurzeln)

- **BIOS01** (`blkdev_rwabs`, `blkdev_mediach`, `blkdev_getbpb`; 407
  Funktionen), 20 statt 51: `bdos` `bdos_trap2`; `bios` `add_cookie_frb
  altram_init balloc_stram bios_init biosmain blkdev_hdv_init blkdev_init
  bus_init dsp_loadprog dsp_lodtobinary filesize machine_detect panic
  pun_info_setup run_auto_program screen_init_address xhdi_handler XHInqDev
  XHInqDev2`.
- **FS20** (`ckdrv`, `getbcb`, `log_media`, `mark_bcbs_invalid`; 394), 16:
  `bdos_trap2`; `add_cookie_frb altram_init balloc_stram bios_init biosmain
  blkdev_hdv_init blkdev_init bus_init dsp_loadprog dsp_lodtobinary
  filesize machine_detect panic run_auto_program screen_init_address`.
  `mark_bcbs_invalid` selbst ist seit FS20 `mapped`; nur in `bdos`
  (Standard) sind `ckdrv`, `log_media`, `mark_bcbs_invalid` vollstaendig,
  `getbcb` nicht (`bdos_trap2`).
- **FS22** (`xexec`, `kpgmld`, `kpgmhdrld`, `osif`; 18), 2: `biosmain
  run_auto_program`; in `bdos` (Standard) alle vier vollstaendig.
- **XBIOS01** (`iorec`, `push_ikbdiorec`, `kbd_int`, `bconstat2`,
  `bconin2`; 375), 12: `add_cookie_frb altram_init balloc_stram bios_init
  biosmain blkdev_hdv_init blkdev_init bus_init kb_timerc_int
  machine_detect panic screen_init_address`. `iorec` und `push_ikbdiorec`
  sind seit XBIOS01 `mapped`; `iorec` allein vollstaendig.
- **XBIOS15** (`mfp_rs232_rx_interrupt_handler`, `push_serial_iorec`,
  `scc_rx_interrupt_handler`; 6), 0: vollstaendig.
- **BIOS15** (`setexc`, `kbdvbase`, `mfpint`, `xbtimer`, `xterm`; 322), 13:
  `bdos_trap2`; `add_cookie_frb altram_init balloc_stram bios_init biosmain
  blkdev_hdv_init blkdev_init bus_init init_serport machine_detect panic
  screen_init_address`. `kbdvbase` vollstaendig.

## Pruefung

- `cargo xtask test` (voll): 1821 passed, 0 failed, 1 ignored (Basislinie 1816/0/1 plus 5 neue).
- `cargo xtask deps --check-mapping`: 1032 Zeilen, alles aufgeloest.
- `cargo clippy --tests` (xtask): keine Warnung in `deps.rs`; die 8
  vorhandenen Warnungen liegen in anderen Dateien.
- `cargo xtask doc-check tools/deps/README.md TASK/TOOL17-STATUS-DONE.MD`:
  0 Fehler.
- `tools/deps/gen-deps.py` nicht beruehrt (`edges.tsv` unveraendert).
