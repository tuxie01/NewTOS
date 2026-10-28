# TOOL12 — Quittung: Trap #2 je Opcode aufgeloest, Kantenart `vector`

Datum 2026-10-28 · Ticket [TOOL12](TOOL12.md) · Generator
`tools/deps/gen-deps.py`, Beschreibung `tools/deps/README.md`, Abfrage
`tools/xtask/src/deps.rs`

- **Trap #2 je Opcode.** Konfiguration `TRAP2` im Generator: `gem` = AES,
  `gsx2` = VDI. Maschinell gelesen und bei jedem Lauf geprueft: der d0-Wert
  der Bindung (AES 0xc8, VDI 0x73), ein installierter Trap-#2-Handler, der
  d0 mit diesem Wert vergleicht und die Verteilung erreicht (`aestrap` →
  `crysbind`, `vditrap` → `screen`), und dass die Datei der Bindung
  `&control`/`&contrl` in ein Feld des Parameterblocks legt. Festgelegt,
  nicht gelesen: Element 0 ist der Opcode (Parameterblock-ABI). Der Opcode
  wird in gimple Anweisung fuer Anweisung verfolgt (Konstante in Element 0,
  Parameter mit angewandten Operationen, z. B. `gem_if0`
  `(ctrlcode >> 9) + 10`, Weitergabe ueber `gem_if`/`gsx_0code`/
  `gsx_1code`/`gsx_ncode`). Kante Aufrufer → Funktion im Slot, Detail
  `trap #2 0xOP <verteilung>[0xOP] via <gerufene Funktion>`. AES ueber den
  `crysbind`-Switch (neu: 87 `table`-Kanten `crysbind[case]`, Fallthrough
  und Sprungziele innerhalb der Quellzeilen des Switch), VDI ueber
  `jmptb1/2`. Die sechs Kanten `not resolved` von `gem`/`gsx2` zu jedem
  Handler entfallen.
- **Kantenart `vector`** (neu, Standardart der Abfrage): Aufruf durch eine
  Variable, ein Feld oder ein Feldelement mit Funktionsadresse → jede dort
  abgelegte Funktion, Detail `via <slot>`. Ablagen aus gimple (Zuweisung,
  Kopie aus einem anderen Slot), C-Initialisierern, `Setexc`/absoluter
  Adresse ab 0x400 (Namen und Adressen aus `tosvars.ld`, wie der Build sie
  vorverarbeitet) und Assembler (Adressregister je Region verfolgt, an jeder
  Marke zurueckgesetzt). Aufrufe: gimple-Aufruf durch einen aus dem Slot
  gelesenen Wert oder den Rueckgabewert von `Setexc`, Slotwert als Argument
  an `protect_v/_w/_ww/_wlwwwl` (`CALLS_ARGUMENT`, aus `include/asm.h`
  festgelegt, geprueft auf berechnetes `jsr`) oder an eine C-Funktion, die
  durch ihren Parameter ruft; Assembler `jsr/jmp (aN)` nach Laden aus dem
  Slot. 34 Slots, u. a. `hdv_rw/_bpb/_mediach`, `etv_timer/_critic/_term`,
  `con_state`, `kcl_hook`, `bell_hook`, `dump_vec`, `swv_vec`,
  `bconstat_vec[]` usw., Kbdvbase-Eintraege (`midivec`, `vkbderr`,
  `vmiderr`, `mousevec`, `midisys`, `ikbdsys`), Line-A-Vektoren
  (`user_but/_cur/_mot/_wheel`), `mousexvec`, `esctbl[]`, VT52-Tabellen.
- Nebenbei behoben (noetig fuer die Erkennung): gimple-Parameterlisten
  werden an Kommas der obersten Ebene getrennt (ein
  Funktionszeiger-Parameter verschob den Index); keine andere Kante
  aendert sich dadurch.
- Kanten je Art vorher → nachher: direct 4235 → 4235, table 316 → 403,
  callback 224 → 224, asm 117 → 117, layer 145 → 230 (−6 `not resolved`,
  +46 AES, +45 VDI), vector 0 → 114, data 19787 → 19787; def 2014.
- Abfragen (Standardarten; vorher alter Graph, alte Arten): `deps lrwabs
  --down` 1 → 155, `getbpb --down` 1 → 158, `mediach --down` 1 → 150,
  `blkdev_rwabs --up` 11 → 404. `--up` von `lrwabs` 545 → 388, `getrec`
  539 → 382, `ap_init` 394 → 19: die alte Auffaecherung von Trap #2 zu jedem
  Handler zog jede AES/Desk-Funktion mit hinein.
- `mapping.tsv`: `blkdev_drvmap/_getbpb/_mediach/_rwabs` n:m auf `drvmap`,
  `bpb::getbpb`/`bpb_for`/`parse`, `mediach`, `rwabs` (BIOS-STOR-009);
  geprueft alle bios/bdos-Handler hinter neuen `vector`-Kanten: 21
  VT52-Befehle (`cursor_up`, `do_tab`, `get_row` …) stehen als Arme von
  `esc_ch1`/`normal_ascii`/`cputc` in `tos-api/src/vdi/escape.rs`, nicht
  mehr als „no counterpart“; `default_etv_critic` bleibt ohne Gegenstueck
  (nur der Platzhalter `ETV_DEFAULT_CRITIC`), Grund neu gefasst. Die
  uebrigen (serielle/MIDI/IKBD/Drucker-Handler, `do_keyclick`, `tikfrk`)
  waren schon richtig.
- Abschlusskriterium: bestehender Test
  `up_getrec_reaches_the_gemdos_file_calls_over_the_osif_table` gruen; neu
  `down_lrwabs_reaches_blkdev_rwabs_over_the_hdv_rw_vector` (auch
  `getbpb`/`mediach`, und `--up blkdev_rwabs` erreicht `biostrap`),
  `aes_opcode_resolves_through_the_crysbind_switch` (`appl_init` →
  `ap_init`, `evnt_multi` → `ev_multi` via `gem`),
  `vdi_opcode_resolves_through_the_jmptb_tables` (`gsx_tblt` →
  `vdi_v_gtext`, `gsx_moff` → `vdi_v_hide_c` via `gsx_0code`).
- `gen-deps.py --check` bytegleich (auch mit `PYTHONHASHSEED` 1 und 4242);
  `cargo xtask test` 1748/0/1 (Basis 1745/0/1 plus drei neue); clippy
  xtask ohne Warnung in `deps.rs`; `cargo xtask deps --check-mapping` OK;
  doc-check der geaenderten Doku 0 Fehler.
- Verbleibende Luecken (README „Misses“, Listen in `layers.md`): zwei
  Trap-#2-Aufrufe ohne Funktion im Slot (`graf_handle`, `GRAF_HANDLE`
  kopiert nur Globale; `gsx_attr` legt 0 = FALSE ab); Verzweigungen in
  einer Funktion werden fuer den Opcode nicht verfolgt; 23 Aufrufe durch
  Slots ohne abgelegte Funktion (`next_handler` der XHDI, DSP `user_rcv/
  _send`, `save_etv_critic`); 23 Ablagen nicht verfolgbarer Werte
  (Laufzeit-Installationen `vex_*`, `maptabptr->…`); 30 C-Aufrufe durch
  nicht aus einer Variable gelesene Werte (Tabellenverteilung `osif`/
  `screen`, NatFeats, `bconmap_root.maptab[i]`); 62 berechnete
  Assembler-Spruenge (Tabellen, `vblqueue`-Schleife, Blitter);
  `Setexc` aus Assembler mit dem Handler im Register (`takeerr` →
  `criterr_handler` in `etv_critic`).
