# Audit V18 Teil 3 — VDI Workstation und Maus (TOOL32-Kandidaten K9/K24, K21, K22, K23, K26)

## 1. Kopf

| | |
|---|---|
| Scope | `TASK/V18.md` Teil 3: K9/K24 (Zeiger im VBL gegen synchron — Wirkung zeigen), K21 (`v_opnwk` setzt den Mauszustand zurück), K22 (`v_clswk`), K23 (Videomodus per `INTIN[0]-2`), K26 (Workstation-Kette). Ausgenommen laut Ticket: K12/K20 (ADR-031), K17 (COMPATIBILITY Q6), K25 (ADR-030) |
| Datum | 2026-11-07 |
| NewTOS gemessen | `5a8e01b506903b6865d1ab320e80ca25c18a5d85`, sauber zu Beginn (`git archive` in den Session-Scratch, `references/` verlinkt) |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df`, ROM sha256 `323c87bb…2d2750c`, `make 512 COUNTRY=de` |
| Hatari | 2.6.1, sha256 `953f4e78…dffcd6f6`, `tools/oracle/hatari.cfg` sha256 `c8f4998a…38f8ae46` |
| Kernel | `kernel8.img` sha256 `d0f31276e07fb09a9a1cd4fb3b50e0480711aa3b7b9893eaca19aff2696fbae3` (rpi4, release; derselbe, den `cargo xtask tosdump` baute); Probe-Kernel mit Injektor (`tools/inject-arm.py … launch:WKPROBE`) sha256 `1f235766f18219669f1445b5e54ecd294c945e564a87853ca5ca879c572e812a` |
| Board | QEMU 11.0.2 `raspi4b`, `serial=on`; Orakel `falcontc` (640×480, 16-Bit-Truecolor = NewTOS-Gerätemodell, ADR-024), Tabelle zusätzlich `tt` |
| Kanarie | `canary tt: OK`, `canary falcontc: OK` (`docs/analysis/ORACLE.md` §6) |
| Lauf | `measurements/measurement-2026-11-07-v18-t3/RUNINFO.TXT` |

Vorflug NewTOS am gemessenen Stand: `vdi-all.tsv` gegen `EMUTOS-1a7484ab-VDI-all-falcontc` 0 von 182 abweichend
(`measurements/measurement-2026-11-07-v18-t3/runs/newtos-vdi-all/`).

Instrumente: neue Probe `tosdump/probes/wkprobe.c` (Rücklese wie `vdprobe.c`: `vro_cpyfm` Schirm → 16-Ebenen-Speicherformular),
Läufer `tosdump/probes/run-wkprobe.sh`, Mauspakete `tosdump/probes/wkprobe-mouse.txt`, Vergleich
`tosdump/probes/wkprobe-compare.py` (Kopie von `vdprobe-compare.py` für `WKPROBE.BIN` plus Zeilenvergleich des LOG); neue Tabelle
`tosdump/cases/vdi-v18-t3.tsv` (DUMPVDI, 2 Fälle, CRC `AE9CB4E7`) mit Referenz
`tosdump/reference/EMUTOS-1a7484ab-VDI-v18t3-falcontc.BIN/.LOG` (zwei Orakelläufe bytegleich, `tt` BIN ebenfalls bytegleich).
Probe je Seite zweimal: EmuTOS BIN und LOG bytegleich, NewTOS BIN bytegleich, LOG gleich bis auf die Wartezeilen (`N `, `T0`);
dazu ein Gegenlauf auf dem Produktkernel ohne Injektor (§6).

Mauspakete (Eingang): EmuTOS — die Probe übergibt jedes Paket selbst an `Kbdvbase()->mousevec` (a0 = Paket, wie
`call_mousevec`, `bios/aciavecs.S:538-553`) in einem `Supexec` bei IPL 7 und liest im selben Aufruf `vq_mouse` und den Schirm
(„sofort“: bevor ein VBL `vb_draw` laufen kann). NewTOS — der Injektor-Kernel speist dieselben sechs Pakete in den IKBD-Decoder
beim Pumpen im `Bconstat(2)`-Warteloop der Probe; sie liest sofort, wenn `vq_mouse` sich ändert. Eingangsprüfung der Pakete:
jedes Paket kam einzeln an, die Deltas im LOG sind auf beiden Seiten gleich (`packet f8 20 8: 200 350 -> 220 358` usw.).
Für die VDI gibt es keinen NewTOS-Argument-Tracer; DUMPVDI zeichnet `contrl`/`intin`/`ptsin` auf beiden Seiten auf, tosdiff meldet
für die Tabelle nur `intout`-Unterschiede (Eingangsprüfung des CONFIRMED-Falls 13).

**Messbefund am Rand, der das Design bestimmt:** ein `v_opnwk`, während die physische Workstation offen ist, kehrt auf EmuTOS nicht
zurück (Probe W9, LOG endet bei „W9 before“, zweimal). Alle K21-Fälle laufen deshalb so, wie EmuTOS' AES selbst es tut: `v_clswk`,
dann `v_opnwk`.

## 2. Befunde

### VDI-WK-001 — S3 — K21: `v_opnwk` setzt den Mauszustand nicht zurück (Position, Tasten, Versteckzähler, Form, `vex_*`-Vektoren)

- **MEASURED** (Probe, reproduzierbar). W1: Kreuzform, Position (100,100), Versteckzähler 3, eigene Vektoren in
  `butv/motv/curv/timv/wheelv`, linke Taste gedrückt, dann `v_clswk` + `v_opnwk`:
  - `vq_mouse` danach: EmuTOS `0 319 239` (Tasten 0, Mitte), NewTOS `1 100 100`.
  - `vex_*` danach: EmuTOS liefert für alle fünf den Standard (alter Wert ≠ eigener; `curv` = `mov_cur` von vorher), NewTOS für alle
    fünf den eigenen Vektor (`old == own 1`). `vex_timv` gehört dazu: EmuTOS `timer_init` setzt `tim_addr = just_rts`.
  - W1b (ein `v_show_c(1)`): EmuTOS zeigt den Pfeil in der Mitte (Zähler war 1), NewTOS nichts (Zähler 3 → 2), 54 px.
  - W1c (`v_show_c(0)`): NewTOS zeigt die Kreuzform bei (100,100), EmuTOS den Pfeil in der Mitte, 142 px.
  - W2 (Zeiger sichtbar über einem Muster, dann `v_clswk` + `v_opnwk`): W2a gleich (beide löschen den Schirm). W2b nach `v_hide_c`:
    NewTOS malt das 16×16-Hintergrundstück von **vor** dem Löschen zurück (256 px Musterrest auf leerem Schirm), EmuTOS nichts
    (`HIDE_CNT = 1` ohne `cur_replace`). W2c: NewTOS Kreuz bei (100,100), EmuTOS Pfeil in der Mitte, 310 px.
  - Gleich: `vqin_mode(1)` = 0 (der Teil `reset_input_modes` ist schon EmuTOS-gleich), `work_out` von `v_opnwk`.
  `measurements/measurement-2026-11-07-v18-t3/probe/compare-e1-n1.txt`, Karten `measurements/measurement-2026-11-07-v18-t3/probe/maps/29-W1b.txt`, `30-W1c.txt`, `32-W2b.txt`, `33-W2c.txt`.
- EmuTOS: `references/emutos/vdi/vdi_control.c:562-563` (`timer_init`, `vdimouse_init` in jedem `v_opnwk`),
  `vdi/vdi_mouse.c:565-618` (`HIDE_CNT = 1` `:581`, Mitte `:582-583`, `user_*` `:585-589`, Standardform, `MOUSE_BT = 0`,
  `cur_ms_stat = 0`), `vdi/vdi_misc.c:121-135` (`tim_addr = just_rts`).
- NewTOS: `tos-api/src/vdi/mouse.rs:343@ensure_init` (einmalig über `INIT_DONE`, `tos-api/src/vdi/mouse.rs:37@INIT_DONE`), gerufen aus
  `tos-api/src/vdi/workstation.rs:273@init_wk` über `tos-api/src/vdi/mouse.rs:602@init`; `tos-api/src/vdi/workstation.rs:362@v_opnwk`.
- **LOCALIZED: `ensure_init`** (Gegenstück von `vdimouse_init`, `mapping.tsv`). `--down` von `vdimouse_init`: `set_mouse_form` /
  `validate_color_index` gleich (P2, P3), Zeigerbild gleich (P1–P5); `Initmous`/`kbdvbase` (bios) nicht eigens gemessen — ihre
  API-Wirkung (Pakete erreichen `mouse_int`) ist in M1–M3 gleich.
- Spec: `docs/api/vdi/workstation/01-v_opnwk.md` nennt `vdimouse_init` als „reference, NewTOS runs the equivalent“ — die Messung zeigt,
  dass das Äquivalent nur einmal läuft. Mapping-Zeile `vdimouse_init` sagt es richtig („NewTOS initialises once“).
- Schadensbild: ein Programm, das die Workstation selbst schließt und öffnet (oder eine neue AES-Sitzung, `gsx_init`), findet Zeiger,
  Zähler, Form, Tasten und Vektoren des Vorgängers; ein Zähler > 1 lässt den Zeiger unsichtbar, ein sichtbarer Zeiger hinterlässt nach
  dem nächsten `v_hide_c` einen alten Bildrest.
- Retest (`vdimouse_init --up`, 11: `vdi_v_opnwk`, `v_opnwk`/`gsx_wsopen`/`gsx_init`, `run_accs_and_desktop` …,
  `measurements/measurement-2026-11-07-v18-t3/deps/vdimouse_init-up.txt`); Probe W1, W2.
- Route: Developer.

### VDI-WK-002 — S3 — K22: `v_clswk` schaltet die VDI-Maus nicht ab

- **MEASURED** (Probe, reproduzierbar). C1: Pfeil sichtbar bei (150,150) über einem Muster, eigene `butv/motv/timv/wheelv`, dann
  `v_clswk`:
  - Paket +40,+20 danach: EmuTOS `vq_mouse` bleibt `150 150` (der Vektor ist `just_rts`), NewTOS `190 170`.
  - C1b/C1c: NewTOS malt den Zeiger bei (190,170) auf den Konsolenschirm und das alte Musterstück bei (150,150) zurück (310 px),
    EmuTOS zeichnet nichts. C1d (nach `v_hide_c`) wieder gleich: EmuTOS malt das Musterstück jetzt zurück.
  - `vex_*` nach `v_clswk`: EmuTOS `butv/motv/wheelv/curv` = `just_rts` (auch `curv`, nicht `mov_cur`), NewTOS die eigenen bzw.
    den alten `curv`. `timv` gleich (EmuTOS `timer_exit` lässt `tim_addr` stehen).
  - Gleich: C1a (Schirm nach `esc_exit`), die virtuellen Workstations sind zu (`vq_extnd` auf dem alten Handle: beide ohne Antwort).
- EmuTOS: `vdi/vdi_control.c:587-589`, `vdi/vdi_mouse.c:622-645` (`vdimouse_exit`: `user_* = just_rts`, `Initmous(0,0,0)`
  `:634`), `bios/mouse.c:47-49,121-122` (Typ 0 → `kbdvecs.mousevec = just_rts`).
- NewTOS: `tos-api/src/vdi/workstation.rs:388@v_clswk` (nur `close_all_virtual` und `esc_exit`; Kommentar „out of this package's
  scope“); `vdimouse_exit` ist in `mapping.tsv` `unmapped`.
- **LOCALIZED: `v_clswk`** (das fehlende `vdimouse_exit`). `--down` von `vdi_v_clswk`: `close_all_virtual` gleich (W1-Log),
  `esc_exit` gleich (C1a, W1a, W2a), `timer_exit` design ADR-031 und in der Wirkung auf `tim_addr` gleich.
- Schadensbild: nach `v_clswk` (Rückkehr in den Konsolenmodus) läuft der Zeiger weiter über den Text, `vq_mouse` meldet Bewegung, ein
  alter Bildrest wird in die Konsole gemalt.
- Retest (`vdimouse_exit --up`, 9: `vdi_v_clswk`, `gsx_wsclose`, `run_accs_and_desktop` …; `vdi_v_clswk --up`); Probe C1. Der Fix
  gehört mit 001 zusammen: was `v_clswk` abschaltet, muss das nächste `v_opnwk` wieder einschalten.
- Route: Developer (+ Researcher: Mapping-Zeile `vdimouse_exit` nach dem Fix).

### VDI-WK-003 — S3 — K24: `vsc_form` zeichnet einen sichtbaren Zeiger sofort neu; EmuTOS lässt das alte Bild bis zum nächsten Neuzeichnen stehen

- **MEASURED** (Probe, reproduzierbar). F1: Pfeil sichtbar bei (110,260) auf einem Muster, `vsc_form` (Block, Stifte 2/3): F1b direkt
  danach — EmuTOS zeigt weiter den Pfeil, NewTOS den Block (243 px). F2: nur der Hotspot wechselt (0,0 → 8,8): EmuTOS lässt den Pfeil
  an der alten Stelle, NewTOS zeichnet ihn 8 px links oben (180 px). Gleich: F1a, F1c (nach `v_hide_c` beide das Muster), F1d (nach
  `v_show_c(0)` beide den Block), F2c. Karten `measurements/measurement-2026-11-07-v18-t3/probe/maps/14-F1b.txt`, `17-F2b.txt`.
- EmuTOS: `vdi/vdi_mouse.c:450-492` (`set_mouse_form` speichert nur, `mouse_flag` hält den VBL fern; kein `cur_replace`/`cur_display`).
  Über die AES ist es nicht sichtbar: `gsx_mfset` klammert mit `gsx_moff`/`gsx_mon` (`aes/gemgsxif.c:407-420`).
- NewTOS: `tos-api/src/vdi/mouse.rs:773@vsc_form` (erase, speichern, draw).
- **LOCALIZED: `vsc_form`.** `--down` (`linea_transform_mouse`, `set_mouse_form`, `validate_color_index`): Speicherung, Hotspot-Maske
  und Farbprüfung gleich (P2 mit Hotspot `0x17`, P3 mit Stift 300/−2), Zeichnen/Restaurieren gleich (P1–P6).
- Retest (`vdi_vsc_form --up`, 184; die AES-Aufrufer `gsx_mfset`, `gr_mouse`, `ct_mouse` … gehen in NewTOS über
  `tos-api/src/aes/graf.rs` und `install_form`, die eigene Klammer hat — beim Fix gegenprüfen); Probe F1, F2.
- Route: Developer.

### VDI-WK-004 — S3 — K9/K24: der Zeiger folgt einem Mauspaket sofort; EmuTOS zeichnet ihn erst im nächsten VBL

- **MEASURED** (Probe, reproduzierbar). M1: Pfeil sichtbar bei (200,350), Paket +20,+8. Sofort danach melden beide `vq_mouse` =
  (220,358); der Schirm zeigt bei EmuTOS den Pfeil noch bei (200,350) und nichts bei (220,358), bei NewTOS den Pfeil schon bei
  (220,358) (194 px). M2: `v_bar` über den sichtbaren Pfeil gezeichnet, dann Paket +30,0: sofort danach ist bei EmuTOS der Balken noch
  ganz (kein Loch, kein neuer Zeiger), bei NewTOS schon das 16×16-Loch mit dem alten Hintergrund und der Pfeil bei (380,350) (353 px).
  Nach drei `Vsync` (M1b, M2b) gleich — auch das „Mauskrümel“-Loch selbst. M3 (Paket bei verstecktem Zeiger) und D1 (Balken über dem
  Zeiger, dann `v_hide_c`) gleich. Karten `measurements/measurement-2026-11-07-v18-t3/probe/maps/22-M1a.txt`, `24-M2a.txt`.
- Was ein Programm sieht: zwischen Mausinterrupt und nächstem VBL (höchstens ein Bild) liefern `vq_mouse` die neue Position und
  `v_get_pixel`/`vro_cpyfm` den Schirm mit dem Zeiger an der alten. Mehrere Pakete in einem Bild zeichnet EmuTOS einmal (an der letzten
  Position), NewTOS je Paket. Der Endzustand ist gleich.
- `v_show_c`/`v_hide_c` selbst sind auf beiden Seiten synchron: P1 liest direkt nach `v_show_c(0)` den Zeiger (gleich), `v_get_pixel`
  unter dem Zeiger gleich (`ffdf` Maske, `0000` Daten, `ffff` daneben).
- EmuTOS: `vdi/vdi_asm.S:496-534` (`mov_cur`: nur `newx/newy`, `draw_flag`), `vdi/vdi_mouse.c:602` (`vblqueue[0] = vb_draw`),
  `:668-686` (`vb_draw`).
- NewTOS: `tos-api/src/vdi/mouse.rs:190@feed_relative` → `tos-api/src/vdi/mouse.rs:169@set_state` (erase + draw im Paket).
- **LOCALIZED: `set_state`** (Gegenstück von `mov_cur` und `vb_draw`). `--down` von `vb_draw` (`cur_display16`, `cur_replace16`,
  `cur_display_clip`): gleich (P1–P7, D1, M1b, M2b, M3).
- Spec: `docs/api/vdi/input/_mouse.md` (Kopf) führt den VBL-Zeichenweg als „Reference (NewTOS reimplements freely)“; die Messung zeigt
  eine beobachtbare Wirkung (Researcher).
- Retest (`mov_cur --up`: `mouse_int`; `vb_draw --up` leer, siehe 007); Probe M1–M3, D1.
- Route: Researcher (Entscheidung: Zeichnen in einen 60-Hz-Takt verschieben wie `vb_draw`, oder COMPATIBILITY-Eintrag), dann Developer.

### VDI-WK-005 — S4 — `v_opnwk` bei offener physischer Workstation: EmuTOS kehrt nicht zurück, NewTOS schon (und lässt die virtuellen Workstations offen)

- **MEASURED** (Probe W9, zweimal): EmuTOS-LOG endet bei „W9 before“, Hatari nach 20 s beendet (`run.txt`: `hatari w9-no-end`); NewTOS
  kehrt zurück, drei `Vsync` laufen, `hz_200` zählt. Ursache gelesen, nicht gemessen: der zweite `timer_init`
  (`vdi/vdi_misc.c:131-132`) bekommt von `Setexc(0x100, tick_int)` `tick_int` selbst als `tim_chain`, `tick_int` ruft sich dann ohne Ende
  (`:88`).
- Kein definiertes EmuTOS-Ergebnis, also kein NewTOS-Defekt — aber eine Abweichung ohne Eintrag. Dazu: NewTOS
  `tos-api/src/vdi/vwk.rs:373@open_physical` leert die virtuellen Slots nicht (EmuTOS `vdi/vdi_control.c:544-545`); die Mapping-Zeile
  `vdi_v_opnwk → open_physical` behauptet „virtual slots cleared“ (falsch), die Spec `01-v_opnwk.md` nennt es richtig als offenen Punkt.
- **LOCALIZED: `v_opnwk` / `open_physical`.**
- Retest: Probe W9.
- Route: Researcher (COMPATIBILITY-Eintrag wie Entscheidung C, Mapping-Text).

### VDI-WK-006 — S4 — PLAUSIBLE: das erste `v_opnwk` nach einem AUTO-Programm, das eine virtuelle Workstation offen ließ, schließt sie auf NewTOS nicht

- **PLAUSIBLE** (gelesen). EmuTOS' erstes `v_opnwk` (AES-Start, kein Hänger, `timer_init` zum ersten Mal) setzt `vwk_ptr[2..] = NULL`
  (`vdi/vdi_control.c:544-545`); NewTOS `open_physical` lässt sie stehen. Im Normalweg gleich gemessen: `graf_handle` 1, erste
  `v_opnvwk` der Probe Handle 2 auf beiden Seiten.
- Fall, der es entscheidet: ein AUTO-Programm (EmuTOS `.PRG`, NewTOS `.NCP`), das `v_opnvwk` ruft und ohne `v_clsvwk` endet, dann eine
  GEM-Probe, die auf dem alten Handle `vq_extnd` ruft und das Handle ihrer eigenen `v_opnvwk` meldet.
- **NOT LOCALIZED** (nicht gemessen; vermutet `open_physical`).
- Route: Researcher (Messfall), dann Developer.

### VDI-WK-007 — S4 — Abhängigkeitsgraph: Vektorkanten fehlen, eine Asm-Kante ist falsch

- `vb_draw --up` und `mouse_int --up` sind leer (`measurements/measurement-2026-11-07-v18-t3/deps/vb_draw-up.txt`,
  `mouse_int-up.txt`): es fehlen die Kanten über `vblqueue[0] = vb_draw` (`vdi/vdi_mouse.c:602,631`) und über
  `Initmous(…, mouse_int)` → `kbdvecs.mousevec` (`:605`, `bios/mouse.c:121-122`).
- Das lokale Label `scrn_clip` (`vdi/vdi_asm.S:187`, zwischen `_GSX_ENTRY` `:64` und `_mouse_int` `:231`) wird `GSX_ENTRY`
  zugeschlagen: die Kante `mouse_int -> GSX_ENTRY [asm jbsr scrn_clip]` bläht `mouse_int --down` auf 430 Funktionen und hängt
  `mouse_int` in jede `--up`-Menge über `GSX_ENTRY`.
- Route: Researcher (Graph).

### VDI-WK-008 — S4 — Spec-/Mapping-Texte widersprechen der Messung

- `docs/api/vdi/input/_mouse.md` (Kopf): VBL-Zeichenweg „reimplement freely“ — beobachtbar (004).
- `docs/api/vdi/workstation/101-v_clsvwk.md` § `v_clswk`: „effectively a no-op there“ — veraltet; die AES ruft es (`gsx_wsclose`), und
  es schließt alle virtuellen Workstations (gemessen gleich); die fehlende Maus-Abschaltung ist 002.
- `docs/analysis/deps/mapping.tsv`: `vdi_v_opnwk → vwk.rs open_physical` „virtual slots cleared“ (falsch, 005); `build_vwk_chain`
  `unmapped` — Vorschlag `design:ADR-030` (K26, unten).
- Route: Researcher.

## 3. Offene Tickets im Scope

- `TASK/G-VDI-001.md` (`v_opnwk` messen): **DIFF** — Tabellenfall 13 (`vdi-v18-t3.tsv`, K23, ACCEPTED), Probe W1/W2 (001), W9 (005);
  `work_out` nach `v_clswk` ohne Moduswunsch gleich (Probe Z1, W1).
- `TASK/G-VDI-002.md` (`v_clswk` messen): **DIFF** — Tabellenfall 12 gleich (keine Ausgaben), Probe C1 (002).
- Berührt, nicht Auftrag: `G-VDI-124` (`vq_mouse`: im Normalweg gleich, M1–M3/W1 vor dem Schließen), `G-VDI-118/125/126/127/134`
  (`vex_*`: gemessen nur die Rückgabe-Relation nach `v_clswk`/`v_opnwk`, 001/002).

## 4. Coverage-Matrix

| EmuTOS | NewTOS (mapping) | Ebene | Status | Fälle |
|---|---|---|---|---|
| `mouse_int` | `feed_relative`, `update_ms_stat` | vdi | EQUAL (Position, Tasten, Klippen der Pakete) | Probe M1–M3, W1 Taste |
| `cur_display16` | `draw` | vdi | EQUAL | P1–P5, P7c, F1d, D1c, M1b, M3b |
| `cur_display_clip` | design ADR-024 | vdi | EQUAL (Ränder) | P4, P5 |
| `cur_replace16` | `erase` | vdi | EQUAL | P6b, F1c, F2c, D1b |
| `dis_cur` / `linea_show_mouse` | `dis_cur`, `v_show_c` | vdi | EQUAL (synchron, Zähler) | P1, P7a–c |
| `hide_cur` | `v_hide_c` | vdi | EQUAL | P6, P7, D1 |
| `vdi_vq_mouse` | `vq_mouse` | vdi | EQUAL (im Normalweg) | M1–M3, W1 |
| `set_mouse_form` / `vdi_vsc_form` | `vsc_form` | vdi | DIFF → 003 (Speichern/Prüfen gleich) | P2, P3, F1, F2 |
| `mov_cur` | `set_state` | vdi | DIFF → 004 | M1a, M2a |
| `vb_draw` | `set_state` | vdi | DIFF → 004 (Endzustand gleich) | M1, M2 |
| `vdimouse_init` | `ensure_init` | vdi | DIFF → 001 | W1, W2 |
| `timer_init` | design ADR-031 | vdi | DIFF → 001 (`tim_addr`-Reset) | W1-Log |
| `vdi_v_opnwk` | `v_opnwk`, `open_physical` | vdi | DIFF → 001, 005; `work_out` gleich | W1, W2, W9, Z1 |
| `vdi_v_opnwk` Moduswunsch `INTIN[0]-2` (K23) | — (ADR-017) | vdi | ACCEPTED | `vdi-v18-t3.tsv` 13, Probe Z2–Z8 |
| `vdimouse_exit` | unmapped | vdi | DIFF → 002 | C1 |
| `vdi_v_clswk` | `v_clswk`, `close_all_virtual` | vdi | DIFF → 002; Schließen/`esc_exit` gleich | `vdi-v18-t3.tsv` 12, C1, W1-Log |
| `timer_exit` | design ADR-031 | vdi | EQUAL (`tim_addr` bleibt) | C1-Log |
| `build_vwk_chain` (K26) | unmapped | vdi | UNMEASURED — nicht messbar (unten) | — |

equal 8, accepted 1, diff 8, unmeasured 1 von 18 Zeilen.
Bug-Dichte: 0 CONFIRMED / 17 gemessene Zeilen = 0; mit den 4 MEASURED S3-Befunden 4/17 = 0,24.

Urteil je Kandidat:

| Punkt | Urteil |
|---|---|
| K9/K24 Zeiger im VBL gegen synchron | FINDING 004 (Wirkung ≤ 1 Bild: Schirm hinter `vq_mouse`; Endzustand gleich); dazu FINDING 003 (`vsc_form` zeichnet sofort) |
| K21 `v_opnwk` setzt Mauszustand zurück | FINDING 001 (Position, Tasten, Zähler, Form, fünf Vektoren; Bildrest nach `v_hide_c`) |
| K22 `v_clswk` | FINDING 002 (Maus läuft weiter, Vektoren bleiben); Schließen der Workstations und `esc_exit` gleich |
| K23 Videomodus per `INTIN[0]-2` | ACCEPTED (CONFIRMED-Abweichung, Fall 13; ADR-017 Rev. 2) |
| K26 Workstation-Kette | NOT MEASURABLE (keine API, Line-A ADR-030) |
| Nebenfund `v_opnwk` ohne `v_clswk` | FINDING 005 (EmuTOS hängt; Eintrag fehlt) |

## 5. Neue Fälle

- `tosdump/cases/vdi-v18-t3.tsv` (2 Fälle, CRC `AE9CB4E7`) + `tosdump/reference/EMUTOS-1a7484ab-VDI-v18t3-falcontc.BIN/.LOG`. Nur ein
  `v_opnwk` pro Lauf ausdrückbar (Kopf der Tabelle).
- `tosdump/probes/wkprobe.c`, `run-wkprobe.sh`, `wkprobe-mouse.txt`, `wkprobe-compare.py`: Fälle CAL, P0–P7, F1–F2, D1, M1–M3, W1–W2,
  C1, Z1–Z8, W9 (Kopf von `wkprobe.c`). Kein Makefile-Ziel (wie `run-mnprobe.sh`, gebaut im Läufer); `tools/hatari-oracle.sh` kennt
  nur seine gelisteten Proben, deshalb startet der Läufer Hatari mit dessen Befehlszeile selbst.
- Vorgeschlagen, mit den Werkzeugen nicht ausdrückbar:
  - DUMPVDI-Fälle auf dem `graf_handle`-Handle (heute nur das eigene virtuelle), damit Fälle nach `v_clswk`/`v_opnwk` messbar bleiben
    (001: `vq_mouse` nach `v_clswk`+`v_opnwk`).
  - Mauspakete zwischen zwei Fällen (004, 002).
  - Der AUTO-Fall von 006.

## 6. Gemessen vs. offen

Gemessen auf beiden Seiten: alle Kandidaten außer K26; Probe je Seite zweimal (EmuTOS bytegleich, NewTOS BIN bytegleich), Tabelle
zweimal auf dem Orakel bytegleich.

Offen bzw. nur eingeschränkt:
- K26: kein API-Weg (die Kette `Vwk.next_work` ist nur über den Line-A-Zeiger `CUR_WORK` erreichbar, ADR-030; kein VDI-Aufruf liefert
  einen `Vwk`-Zeiger). Nicht messbar.
- Gegenlauf mit dem unveränderten Produktkernel (ohne Injektor, `measurements/measurement-2026-11-07-v18-t3/probe/np/`): jede
  Paketwartezeit läuft aus; gegen den Injektor-Kernel unterscheiden sich genau die sieben Aufnahmen nach einem Paket, die übrigen 30
  (alle K21-Aufnahmen W1/W2, beide `vsc_form`-Aufnahmen F1b/F2b) sind bytegleich (`measurements/measurement-2026-11-07-v18-t3/probe/compare-n1-np.txt`) — 001 und 003
  gelten für den Produktkernel. Zwei frühere Versuche dieses Gegenlaufs wurden von außen abgebrochen (SIGTERM vom Abnahmeskript
  accept.sh des V51-Arbeiters, `measurements/measurement-2026-11-07-v18-t3/probe/np-try1.stdout`,
  `measurements/measurement-2026-11-07-v18-t3/probe/np-try2.stdout`); keine Belege.
- 006 nur gelesen (AUTO-Fall).
- Ursache des EmuTOS-Hängers in W9 gelesen, nicht im Debugger gezeigt.
- Nur Profil falcontc für die Probe; die Tabelle zusätzlich tt (bytegleiche BIN).

## 7. Real-Pi4-Liste

- 004: auf HDMI mit echter USB-Maus — läuft der Zeiger sichtbar ruhiger/unruhiger als auf EmuTOS-Hardware (NewTOS zeichnet je Paket,
  EmuTOS einmal pro Bild)? Kein Abnahmepunkt, Beobachtung für die Entscheidung.

## 8. Akzeptable Abweichungen

- **K23 (Moduswunsch in `v_opnwk`)** — gemessen: EmuTOS falcontc schaltet bei `work_in[0]` 2/3/4 auf ST-Niedrig/-Mittel/-Hoch
  (`work_out` 319×199/16 Farben, 639×199/4, 639×399/2) und mit 5 + `INTOUT[45] = 0x1B` auf 8 Ebenen; NewTOS meldet immer die
  laufende Geometrie 639×479, 256 Farben, 16 Ebenen (Fall 13, Probe Z2–Z8). Abgedeckt durch ADR-017 Rev. 2 („there is no runtime
  resolution switch“) und die Hardware-Ausnahme Shifter/Videl; verhält sich wie in `docs/api/vdi/workstation/01-v_opnwk.md`
  (NewTOS-Notizen: `newrez` ignoriert, `DEV_TAB` = laufende Geometrie) beschrieben. Kein Eintrag in `docs/COMPATIBILITY.md` und keine
  Zeile in `docs/DEVIATIONS.md` — Vorschlag unten.

### Zeilenvorschläge für `docs/DEVIATIONS.md` (Abschnitt VDI; nicht eingetragen)

| Trigger | NewTOS | EmuTOS | Basis | Status |
|---|---|---|---|---|
| `v_clswk` + `v_opnwk` (Programm oder neue AES-Sitzung) | Zeigerposition, Tasten, Versteckzähler, Form und `vex_*`-Vektoren bleiben; ein sichtbarer Zeiger malt beim nächsten `v_hide_c` einen alten Bildrest | Mitte, Tasten 0, Zähler 1, Standardpfeil, Vektoren auf Standard | gemessen (wkprobe W1, W2) | VDI-WK-001, Ticket offen |
| Mauspakete nach `v_clswk` | Zeiger bewegt und zeichnet sich weiter, `vq_mouse` meldet Bewegung, `vex_*` bleiben | Maus abgeschaltet (`Initmous(0)`), Vektoren `just_rts` | gemessen (wkprobe C1) | VDI-WK-002, Ticket offen |
| `vsc_form` bei sichtbarem Zeiger (direkt, nicht über `graf_mouse`) | neue Form/Hotspot sofort auf dem Schirm | altes Bild bis zum nächsten Neuzeichnen | gemessen (wkprobe F1, F2) | VDI-WK-003, Ticket offen |
| Schirm lesen direkt nach einem Mauspaket (vor dem nächsten Bild) | Zeiger schon an der neuen Position | Zeiger noch an der alten, `vq_mouse` schon neu | gemessen (wkprobe M1, M2) | VDI-WK-004, Entscheidung offen |
| `v_opnwk` bei offener physischer Workstation | kehrt zurück, virtuelle Workstations bleiben offen | hängt (`tick_int` verkettet sich mit sich selbst) | gemessen (wkprobe W9) | VDI-WK-005, COMPATIBILITY-Eintrag offen |
| `v_opnwk` mit `work_in[0]` = Modus + 2 | kein Moduswechsel, `work_out` = laufende Geometrie | `Setscreen`/`VsetMode` auf den Modus | gemessen (`vdi-v18-t3.tsv` 13, wkprobe Z) | ADR-017 Rev. 2 |

## 9. Out-of-scope observations

- P0: das Zeigerbild, das die AES beim Programmstart hinterlässt — EmuTOS die Sanduhr (vom Starten des Programms), NewTOS der Pfeil
  (`measurements/measurement-2026-11-07-v18-t3/probe/maps/02-P0.txt`) — AES/Desktop, nicht verfolgt.
- `Getrez` im falcontc-Truecolor-Modus: EmuTOS 2, NewTOS 4 (Probe Z-Zeilen) — XBIOS, nicht verfolgt.
- `docs/api/vdi/input/_mouse.md` sagt „`vex_*` vectors are real function-pointer slots the IRQ/timer path calls“, im Widerspruch zu
  ADR-031 und den eigenen ARCH03-Notizen derselben Datei.

## 10. Cleanup

- Kein `AUDIT-TEMP` gesetzt; kein Produktcode berührt. Der Injektor patcht nur eine Scratch-Kopie (`tools/inject-arm.py`).
- Neu (dauerhaft): `tosdump/probes/wkprobe.c`, `run-wkprobe.sh`, `wkprobe-mouse.txt`, `wkprobe-compare.py`,
  `tosdump/cases/vdi-v18-t3.tsv`, zwei Referenzdateien, der Laufordner, dieser Bericht, der Abschnitt „Teil 3 gemessen“ in
  `TASK/V18.md`. Keine bestehende Datei unter `tosdump/` oder `tools/` geändert.
- Fremd gesehen: der V51-Arbeiter (eigene Worktree) beendete mit seinem Abnahmeskript (accept.sh in seiner Worktree) zweimal den QEMU dieses Audits per SIGTERM; nichts
  von ihm berührt. Sein QEMU lief parallel unter seinem eigenen Pfad im selben Session-Scratchpad (`scratchpad/v51`), unberührt.
- Scratch (Baum, Kartenklone, Dumper, Kanarie) im Session-Scratchpad (Unterordner t3), am Ende gelöscht.
