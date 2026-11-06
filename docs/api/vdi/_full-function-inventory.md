# VDI — vollständiges Funktions- und Unterfunktionsinventar

**Stand:** 2026-09-14 · **Quelle:** `references/emutos` @
`1a7484aba1bfba23e2c0899e243b6d8177fe41df` (mit `git -C references/emutos log -1`
bestätigt) · **Zielprofil:** `make 512`, Falcon030/TOS4 · **NewTOS-Seite:**
HEAD `19871c0`.

Formvorlage: `docs/api/aes/_full-function-inventory.md`.

---

## 0. Methode und Abgrenzung

**Die Liste kommt aus den Sprungtabellen, nicht aus Kopfdateien.**
Maßgeblich ist `vdi/vdi_main.c`:

* `jmptb1[]` (`:32-72`) — Opcodes **1…39**, 39 Einträge,
* `jmptb2[]` (`:74-112`) — Opcodes **100…134**, 35 Einträge im Zielprofil,
* der Verteiler `screen()` (`:121-192`), Bereichsprüfung `:150-157`.

`vdi_control.c` ist **nicht** der Verteiler — dort liegen nur die
Workstation-Funktionen. Die Angabe im Auftrag war insoweit ein Anhaltspunkt,
kein Treffer.

### Die vier Nicht-Wege des Verteilers

| Fall | EmuTOS | Beleg |
|---|---|---|
| Opcode ∉ {1…39, 100…134} | **stille Rückkehr**, `contrl[2]=contrl[4]=0` | `vdi_main.c:155-157` |
| Handle zeigt auf keine offene Workstation (außer bei 1/100) | **stille Rückkehr** | `:141-143` |
| `vdi_v_nop`-Eintrag | `just_rts` — der Opcode ist *belegt*, tut aber nichts | `:23` |
| `d0`-Rückgabe | `flip_y`, **nicht** ein Fehlercode — die VDI hat keinen | `:20,135` |

**Die VDI kennt keinen Fehlerrückgabewert.** Ein unbekannter Opcode ist
beobachtbar allein daran, dass `contrl[2]`/`contrl[4]` 0 bleiben und nichts
gezeichnet wurde.

### Profilgatter — nachgeprüft für `make 512`

Mit `cc -E -dM -Iinclude -DTARGET_512` gegen `include/config.h` gemessen:

| Schalter | Wert | VDI-Folge |
|---|---|---|
| `CONF_WITH_EXTENDED_MOUSE` | **1** | Opcodes **132, 133, 134** existieren (`vdi_main.c:107-111`) |
| `CONF_WITH_VDI_16BIT` | **1** | `vq_color`/`vs_color` nehmen den 16-Bit-Zweig (`vdi_col.c:862-868`) |
| `CONF_WITH_TT_SHIFTER` | **1** | `adjust_tt_colnum` in `vq_color` (`vdi_col.c:871-872`) |
| `CONF_WITH_GDOS` | **1** | `vst_load_fonts` (119) verkettet **echte** Fonts (`vdi_text.c:1294-1347`) |
| `CONF_WITH_VDI_VERTLINE` | 1 | interner Zeichenpfad, keine Opcode-Folge |
| `HAVE_BEZIER` | **0** | `vdi_defs.h:18` — **hart verdrahtet im Quelltext**, kein `CONF_WITH_*`. Damit existieren **weder** Escape-Unterfunktion 99 (`v_bez_qual`, `vdi_esc.c:340-345`) **noch** GDP-Unterfunktion 13 (`v_bez`, `vdi_gdp.c:474-478`) |

**Es gibt in der VDI keine Funktion, die im Zielprofil abgeschaltet wäre** —
außer den beiden Bezier-Unterfunktionen, die an `HAVE_BEZIER 0` hängen
(Kommentar im Quelltext: „entirely untested").

### Was „verdrahtet" heißt

Spalte **N**: **ja** = echte Implementierung · **no-op** = der Opcode ist in
EmuTOS ausdrücklich `vdi_v_nop`, NewTOS fällt in denselben leeren Zweig (also
deckungsgleich) · **Konstante** = NewTOS führt einen festen Wert aus, wo EmuTOS
im Zielprofil rechnet · **nein** = kein Arm.

---

## 1. Opcode-Ebene: 74 Plätze

NewTOS-Verteiler: `tos-api/src/vdi/mod.rs::dispatch` — Sonderarme `:283`
(Opcode 1) und `:287` (100), der Hauptschalter `:323-407`.

### 1.1 `jmptb1` — Opcodes 1…39

| Nr | Name | EmuTOS Tabelle | EmuTOS Rumpf | N | NewTOS |
|---|---|---|---|---|---|
| 1 | `v_opnwk` | `vdi_main.c:33` | `vdi_control.c:496` | ja | `vdi/mod.rs:283` |
| 2 | `v_clswk` | `:34` | `vdi_control.c:573` | ja | `:328` |
| 3 | `v_clrwk` | `:35` | `vdi_control.c:599` | ja | `:330` |
| **4** | **`v_updwk`** | `:36` | **`vdi_v_nop`** | no-op | `:407` (`_ => {}`) |
| 5 | `v_escape` | `:37` | `vdi_esc.c:334` | ja | `:362` |
| 6 | `v_pline` | `:38` | `vdi_line.c:1000` | ja | `:365` |
| 7 | `v_pmarker` | `:39` | `vdi_marker.c:86` | ja | `:366` |
| 8 | `v_gtext` | `:40` | `vdi_text.c:546` | ja | `:367` |
| 9 | `v_fillarea` | `:41` | `vdi_fill.c:617` | ja | `:368` |
| **10** | **`v_cellarray`** | `:42` | **`vdi_v_nop`** | no-op | `:407` |
| 11 | `v_gdp` | `:43` | `vdi_gdp.c:426` | ja | `:369` |
| 12 | `vst_height` | `:44` | `vdi_text.c:681` | ja | `:358` |
| 13 | `vst_rotation` | `:45` | `vdi_text.c:924` | ja | `:356` |
| 14 | `vs_color` | `:46` | `vdi_col.c:656` | ja | `:339` |
| 15 | `vsl_type` | `:47` | `vdi_line.c:90` | ja | `:340` |
| 16 | `vsl_width` | `:48` | `vdi_line.c:104` | ja | `:341` |
| 17 | `vsl_color` | `:49` | `vdi_line.c:143` | ja | `:342` |
| 18 | `vsm_type` | `:50` | `vdi_marker.c:57` | ja | `:345` |
| 19 | `vsm_height` | `:51` | `vdi_marker.c:30` | ja | `:346` |
| 20 | `vsm_color` | `:52` | `vdi_marker.c:72` | ja | `:347` |
| 21 | `vst_font` | `:53` | `vdi_text.c:939` | ja | `:357` |
| 22 | `vst_color` | `:54` | `vdi_text.c:988` | ja | `:349` |
| 23 | `vsf_interior` | `:55` | `vdi_fill.c:173` | ja | `:350` |
| 24 | `vsf_style` | `:56` | `vdi_fill.c:186` | ja | `:351` |
| 25 | `vsf_color` | `:57` | `vdi_fill.c:206` | ja | `:348` |
| 26 | `vq_color` | `:58` | `vdi_col.c:845` | ja | `:372` |
| **27** | **`vq_cellarray`** | `:59` | **`vdi_v_nop`** | no-op | `:407` |
| 28 | `v_locator` | `:60` | `vdi_mouse.c:280` | ja | `:380` |
| **29** | **`v_valuator`** | `:61` | **`vdi_v_nop`** | no-op | `:407` |
| 30 | `v_choice` | `:62` | `vdi_input.c:31` | ja | `:381` |
| 31 | `v_string` | `:63` | `vdi_input.c:79` | ja | `:382` |
| 32 | `vswr_mode` | `:64` | `vdi_control.c:256` | ja | `:353` |
| 33 | `vsin_mode` | `:65` | `vdi_input.c:118` | ja | `:378` |
| **34** | **nicht vergeben** | `:66` | **`vdi_v_nop`** („does not exist") | no-op | `:407` |
| 35 | `vql_attributes` | `:67` | `vdi_line.c:156` | ja | `:373` |
| 36 | `vqm_attributes` | `:68` | `vdi_marker.c:169` | ja | `:374` |
| 37 | `vqf_attributes` | `:69` | `vdi_fill.c:321` | ja | `:375` |
| 38 | `vqt_attributes` | `:70` | `vdi_text.c:999` | ja | `:376` |
| 39 | `vst_alignment` | `:71` | `vdi_text.c:887` | ja | `:354` |

### 1.2 `jmptb2` — Opcodes 100…134

| Nr | Name | EmuTOS Tabelle | EmuTOS Rumpf | N | NewTOS |
|---|---|---|---|---|---|
| 100 | `v_opnvwk` | `vdi_main.c:75` | `vdi_control.c:407` | ja | `vdi/mod.rs:287` |
| 101 | `v_clsvwk` | `:76` | `vdi_control.c:462` | ja | `:329` |
| 102 | `vq_extnd` | `:77` | `vdi_control.c:626` | ja | `:338` |
| 103 | `v_contourfill` | `:78` | `vdi_fill.c:1084` | ja | `:370` |
| 104 | `vsf_perimeter` | `:79` | `vdi_fill.c:219` | ja | `:352` |
| 105 | `v_get_pixel` | `:80` | `vdi_fill.c:1095` | ja | `:377` |
| 106 | `vst_effects` | `:81` | `vdi_text.c:881` | ja | `:355` |
| 107 | `vst_point` | `:82` | `vdi_text.c:820` | ja | `:359` |
| 108 | `vsl_ends` | `:83` | `vdi_line.c:128` | ja | `:343` |
| 109 | `vro_cpyfm` | `:84` | `vdi_raster.c:1471` | ja | `:400` |
| 110 | `vr_trnfm` | `:85` | `vdi_raster.c:269` | ja | `:401` |
| 111 | `vsc_form` | `:86` | `vdi_mouse.c:513` | ja | `:384` |
| 112 | `vsf_udpat` | `:87` | `vdi_fill.c:149` | ja | `:360` |
| 113 | `vsl_udsty` | `:88` | `vdi_line.c:81` | ja | `:344` |
| 114 | `vr_recfl` | `:89` | `vdi_fill.c:293` | ja | `:398` |
| 115 | `vqin_mode` | `:90` | `vdi_input.c:154` | ja | `:379` |
| 116 | `vqt_extent` | `:91` | `vdi_text.c:1024` | ja | `:393` |
| 117 | `vqt_width` | `:92` | `vdi_text.c:1062` | ja | `:394` |
| 118 | `vex_timv` | `:93` | `vdi_misc.c:100` | ja | `:388` |
| 119 | `vst_load_fonts` | `:94` | `vdi_text.c:1292` | ja (V33) | `tos-api/src/vdi/attributes.rs:416@vst_load_fonts` |
| 120 | `vst_unload_fonts` | `:95` | `vdi_text.c:1353` | ja (V33) | `tos-api/src/vdi/attributes.rs:459@vst_unload_fonts` |
| 121 | `vrt_cpyfm` | `:96` | `vdi_raster.c:1493` | ja | `:399` |
| 122 | `v_show_c` | `:97` | `vdi_mouse.c:321` | ja | `:385` |
| 123 | `v_hide_c` | `:98` | `vdi_mouse.c:331` | ja | `:386` |
| 124 | `vq_mouse` | `:99` | `vdi_mouse.c:341` | ja | `:387` |
| 125 | `vex_butv` | `:100` | `vdi_mouse.c:370` | ja | `:389` |
| 126 | `vex_motv` | `:101` | `vdi_mouse.c:391` | ja | `:390` |
| 127 | `vex_curv` | `:102` | `vdi_mouse.c:414` | ja | `:391` |
| 128 | `vq_key_s` | `:103` | `vdi_input.c:110` | ja | `:395` |
| 129 | `vs_clip` | `:104` | `vdi_control.c:235` | ja | `:361` |
| 130 | `vqt_name` | `:105` | `vdi_text.c:1100` | ja | `:396` |
| 131 | `vqt_fontinfo` | `:106` | `vdi_text.c:1137` | ja | `:397` |
| **132** | **`vqt_justified`** (PC-GEM) | `:108` (`CONF_WITH_EXTENDED_MOUSE`) | **`vdi_v_nop`** | no-op | `:407` |
| **133** | **`vs_grayoverride`** (PC-GEM/3) | `:109` (dito) | **`vdi_v_nop`** | no-op | `:407` |
| 134 | `vex_wheelv` | `:110` (dito) | `vdi_mouse.c:438` | ja | `:392` |

### 1.3 Summenzeile

| | Plätze | echte Rümpfe | `vdi_v_nop` |
|---|---|---|---|
| `jmptb1` (1–39) | **39** | **34** | **5** (4, 10, 27, 29, 34) |
| `jmptb2` (100–134) | **35** | **33** | **2** (132, 133) |
| **Summe** | **74** | **67** | **7** |

| | Anzahl |
|---|---|
| Opcodes mit echtem EmuTOS-Rumpf | **67** |
| davon in NewTOS verdrahtet | **67** |
| davon mit festem Wert statt Rechnung | **2** (119, 120 — `CONF_WITH_GDOS = 1`) |
| `vdi_v_nop`-Plätze (beidseitig leer, deckungsgleich) | **7** |
| unter einem bei uns abgeschalteten Schalter | **0** Opcodes (2 *Unterfunktionen*, s. §2.2/§2.3) |
| in NewTOS **nicht** verdrahtet | **0** |
| NewTOS-Erweiterungen über EmuTOS hinaus | **0** |

**Gegengezählt:** `tos-api/src/vdi/mod.rs` hat 2 Sonderarme (`:283`, `:287`),
3 Arme im äußeren Schalter (`:328-330`) und 62 Arme im inneren (`:338-401`) =
**67**. `jmptb1` + `jmptb2` haben 74 Einträge, davon 67 ≠ `vdi_v_nop`.
**Auf Opcode-Ebene ist NewTOS vollständig.**

### 1.4 Der Selektor des GEM-Traps (nicht Teil der Sprungtabellen)

| `d0` | EmuTOS | NewTOS |
|---|---|---|
| 115 | VDI | `exceptions.rs:383` |
| 200 | AES | `exceptions.rs:391` |
| −1 | Adresse des VDI-Einsprungs (`vdi_asm.S:51-52`) | **0** — dokumentierte Abweichung (`exceptions.rs:416`) |
| −2 und alles andere | `d0` bleibt **unverändert** (`vdi_asm.S:40-49`) | `frame.gpr[0]` zurück (`exceptions.rs:428`) |

---

## 2. Unterfunktionen

**Zählregel** wie im AES-Inventar: ein diskreter Wert eines Modus- oder
Auswahlparameters, auf den die Implementierung verzweigt. Klemmende Bereiche
(„alles außerhalb 1…7 wird zu 1") zählen als *ein* Rückfall, nicht als 32767
Werte.

### 2.1 `v_escape` (5) — `CONTRL[5]`

`vdi_v_escape`, `vdi_esc.c:334-350`; Tabelle `esctbl[]` (`:293-315`),
Bereichsprüfung `if (escfun > ldri_escape) return;` (`:347-348`),
`ldri_escape = 19` (`:23`).

| Wert | Name | EmuTOS | NewTOS |
|---|---|---|---|
| 0 | — (Platzhalter, damit die Tabelle 1-basiert passt) | leer, `:49-51` | `_ => {}` |
| 1 | `vq_chcells` | `:62-67` | `escape.rs:187` |
| 2 | `v_exit_cur` | `:81-84` | `:193` |
| 3 | `v_enter_cur` | `:93-96` | `:201` |
| 4 | `v_curup` | `:102-105` | `:209` |
| 5 | `v_curdown` | `:111-114` | `:210` |
| 6 | `v_curright` | `:120-123` | `:211` |
| 7 | `v_curleft` | `:129-132` | `:212` |
| 8 | `v_curhome` | `:138-141` | `:213` |
| 9 | `v_eeos` | `:147-150` | `:217` |
| 10 | `v_eeol` | `:156-159` | `:225` |
| 11 | `vs_curaddress` | `:172-184` | `:230` |
| 12 | `v_curtext` | `:196-207` | `:235` |
| 13 | `v_rvon` | `:213-216` | `:260` |
| 14 | `v_rvoff` | `:222-225` | `:261` |
| 15 | `vq_curaddress` | `:231-236` | `:262` |
| 16 | `vq_tabstatus` | `:248-252` | `:268` |
| 17 | `v_hardcopy` | `:260-263` (`Scrdmp()`) | `:275` — leer (kein Drucker) |
| 18 | `v_dspcur` | `:274-278` | `:281` |
| 19 | `v_rmcur` | `:284-287` | `:287` |
| **99** | `v_bez_qual` | **nicht übersetzt** (`HAVE_BEZIER 0`, `:340-345`) | nicht vorhanden |
| > 19 | — | stille Rückkehr `:347-348` | `_ => {}` `:288` |

→ **20 Unterfunktionswerte** (0–19) + 1 Rückfallklasse. EmuTOS = NewTOS.

### 2.2 `v_gdp` (11) — `CONTRL[5]`

`vdi_v_gdp`, `vdi_gdp.c:426-480`, `switch(CONTRL[5])` `:433`. **Kein
`default`** — ein unbekannter Wert ist eine stille Rückkehr.

| Wert | Name | EmuTOS | NewTOS |
|---|---|---|---|
| 1 | `v_bar` | `:434-446` | `gdp.rs:41` |
| 2 | `v_arc` | `:448,454` | `:44` |
| 3 | `v_pieslice` | `:449,454` | `:43` |
| 4 | `v_circle` | `:450,454` | `:42` |
| 5 | `v_ellipse` | `:451,454` | `:42` |
| 6 | `v_ellarc` | `:452,454` | `:44` |
| 7 | `v_ellpie` | `:453,454` | `:43` |
| 8 | `v_rbox` | `:457-465` | `:45` |
| 9 | `v_rfbox` | `:467-469` | `:46` |
| 10 | `v_justified` | `:471-473` | `:47` |
| **13** | `v_bez` | **nicht übersetzt** (`HAVE_BEZIER 0`, `:474-478`) | nicht vorhanden |
| sonst | — | stille Rückkehr | `_ => {}` `:48` |

→ **10 Unterfunktionswerte** + 1 Rückfallklasse. EmuTOS = NewTOS.

### 2.3 `vq_extnd` (102) — `INTIN[0]`

`vdi_vq_extnd`, `vdi_control.c:626-...`, Verzweigung `if (*(INTIN) == 0)`
(`:633`).

| Wert | Bedeutung |
|---|---|
| 0 | `SIZ_TAB` (12 Werte) → `PTSOUT`, `DEV_TAB` → `INTOUT` (`:634-638`) |
| ≠ 0 | Klemmbereich → `PTSOUT[0..3]`, Rest 0, `INQ_TAB` → `INTOUT`; setzt dabei `INQ_TAB[19] = vwk->clip` (`:641-651`) |

→ 2 Werte. **Kein Rückfall** — jeder Wert ≠ 0 ist der zweite Zweig.

### 2.4 `vro_cpyfm` (109) / `vrt_cpyfm` (121) — `INTIN[0]`

Gemeinsamer Rumpf `cpy_raster`, `vdi_raster.c:1330-...`.
Prüfung: `if (mode & ~0x001f) return;` (`:1341-1342`) — **jeder Wert > 31 ist
eine stille Rückkehr, kein Fehlercode**.
`PAT_FLAG = 16` (`:113`), wird abgezogen (`:1346-1349`).

| Bereich | Bedeutung |
|---|---|
| 0…15 | die 16 logischen Verknüpfungen |
| 16…31 | dieselben 16, aber mit Musterquelle (`PAT_FLAG` gesetzt) |
| > 31, < 0 | **stille Rückkehr** |

→ 16 Verknüpfungen × 2 (Muster an/aus) = **32 Werte** + 1 Rückfallklasse,
für **beide** Opcodes (`transparent`-Flag unterscheidet sie: `:1480` vs.
`:1502`).

### 2.5 `vsl_type` (15) / `vsl_ends` (108) / `vsm_type` (18)

| Opcode | Parameter | gültiger Bereich | Rückfall | Beleg |
|---|---|---|---|---|
| 15 `vsl_type` | `INTIN[0]` | 1…7 (`MIN/MAX_LINE_STYLE`) | `DEF_LINE_STYLE` | `vdi_line.c:94`, `vdi_defs.h:45-46` |
| 108 `vsl_ends` | `INTIN[0]`, `INTIN[1]` | `SQUARED`…`ROUND` = 0…2 | `DEF_END_STYLE` | `vdi_line.c:132-133`, `vdi_defs.h:49-50` |
| 18 `vsm_type` | `INTIN[0]` | 1…6 (`MIN/MAX_MARK_STYLE`) | `DEF_MARK_STYLE` | `vdi_marker.c:61`, `vdi_defs.h:55-56` |

→ 7 + (3 × 2 Enden) + 6 = **19 Werte** + 3 Rückfallklassen.

### 2.6 `vsf_interior` (23) / `vsf_style` (24)

| Opcode | Bereich | Beleg |
|---|---|---|
| 23 `vsf_interior` | 0…4 (`MIN/MAX_FILL_STYLE`), sonst `DEF_FILL_STYLE` | `vdi_fill.c:177`, `vdi_defs.h:59,65` |
| 24 `vsf_style` | **zustandsabhängig**: bei `fill_style == FIS_PATTERN` 1…24, sonst 1…12 | `vdi_fill.c:192-199`, `vdi_defs.h:68-73` |

`vsf_style` ist damit die einzige VDI-Unterfunktion, deren **gültiger Bereich
vom vorher gesetzten Zustand abhängt** — ein eigener Messfall je Füllart.

→ 5 + (24 + 12) = **41 Werte** + 2 Rückfallklassen.

### 2.7 `vswr_mode` (32)

`vdi_vswr_mode`, `vdi_control.c:256-267`: 1…4 (`MD_REPLACE`, `MD_TRANS`,
`MD_XOR`, `MD_ERASE`, `vdi_defs.h:77-80`), sonst `DEF_WRT_MODE`
(`:261`). Gespeichert wird `wm - 1`. → 4 Werte + 1 Rückfall.

### 2.8 `vst_alignment` (39)

`vdi_text.c:887-902`: `INTIN[0]` (waagerecht) 0…2 (`:894-895`), `INTIN[1]`
(senkrecht) 0…5 (`:899-900`); außerhalb → **0**. → 3 + 6 = **9 Werte** +
2 Rückfallklassen.

### 2.9 `vst_rotation` (13)

`vdi_text.c:924-937`: jeder WORD-Wert wird auf 0…3599 normalisiert
(`:928-932`) und dann auf ein Vielfaches von 900 gerundet (`:935`). Ergebnis:
**5 mögliche gespeicherte Werte** — 0, 900, 1800, 2700, **3600**. Der Quelltext
hält ausdrücklich fest, dass 3600 nur durch 3150…3599 erreichbar ist
(`:914-916`).

### 2.10 `vst_effects` (106)

`vdi_text.c:881-884`: `vwk->style = INTIN[0] & INQ_TAB[2]`, `INQ_TAB[2] = 0x1F`
(`vdi_control.c:79`). → **5 Bits** (fett, hell, kursiv, unterstrichen,
umrandet); alles darüber wird weggemaskt, ohne Fehler.

### 2.11 `vsin_mode` (33) / `vqin_mode` (115)

`vdi_input.c:118-142` bzw. `:154-176`. Beides `switch (INTIN[0])` **ohne
`default`**.

| `dev` | Gerät |
|---|---|
| 1 | locator |
| 2 | valuator |
| 3 | choice |
| 4 | string |
| sonst | **nichts geschieht** — `vsin_mode` speichert nicht, `vqin_mode` lässt `INTOUT[0]` unberührt |

`mode` (`INTIN[1]`): 1 = REQUEST, 2 = SAMPLE; gespeichert wird `mode - 1`, ohne
Bereichsprüfung. `vqin_mode` gibt den **gespeicherten** Wert zurück, also 0/1
statt 1/2 — eine TOS-Eigenheit, die EmuTOS nachbildet.

→ 4 Geräte × 2 Betriebsarten = **8 Werte** + 2 Rückfallklassen (je Opcode einer).

### 2.12 `v_locator` (28) / `v_choice` (30) / `v_string` (31)

Jede dieser drei verzweigt auf die in §2.11 gesetzte Betriebsart:

| Opcode | REQUEST | SAMPLE | Beleg |
|---|---|---|---|
| 28 `v_locator` | `vrq_locator` (blockiert) | `vsm_locator` (einmal abfragen) | `vdi_mouse.c:280-314` |
| 30 `v_choice` | **beide identisch** — `gchc_key` setzt immer `TERM_CH = 1` | dito | `vdi_input.c:31-36`, `gchc_key` `:199-203` |
| 31 `v_string` | blockiert bis `CR` oder `j` Zeichen | liest, solange etwas anliegt | `vdi_input.c:89-105` |

`v_string` hat eine zweite Unterfunktionsebene in `INTIN[0]`:

| Klasse | Wirkung | Beleg |
|---|---|---|
| `j >= 0` | Maske `0x00ff` — nur ASCII | `vdi_input.c:83` |
| `j < 0` | `j = -j`, Maske `0xffff` — Scancode **und** ASCII | `:85-88` |

→ 2 Betriebsarten × 3 Opcodes + 2 `v_string`-Klassen = **8 Werte**.

### 2.13 `vq_color` (26) — `INTIN[1]`

`vdi_col.c:845-...`:

| Wert | Bedeutung | Beleg |
|---|---|---|
| 0 | zuletzt **angeforderten** Wert zurückgeben | `:875-892` |
| ≠ 0 | den tatsächlich gesetzten Hardwarewert | (Rest der Funktion) |

Dazu die Bereichsklasse: `colnum < 0 || colnum >= numcolors` → `INTOUT[0] = -1`,
RGB = 0 (`:854-860`) — **der einzige Fehlerrückgabewert der ganzen VDI**.

→ 2 Werte + 1 Fehlerklasse.

### 2.14 `vs_clip` (129) — `INTIN[0]`

`vdi_control.c:235-252`: `≠ 0` = Rechteck aus `PTSIN` setzen, auf 0…`xres`/
`yres` geklemmt (`:238-245`); `0` = auf die ganze Fläche zurücksetzen
(`:245-250`). → 2 Werte.

### 2.15 `v_show_c` (122) — `INTIN[0]`

`vdi_v_show_c` → `linea_show_mouse` (`vdi_mouse.c:1125-1128`):
`if (!INTIN[0] && HIDE_CNT) HIDE_CNT = 1;` — **0 = unbedingt zeigen**
(Zähler zurücksetzen), **≠ 0 = nur den Zähler herunterzählen**. → 2 Werte.

### 2.16 `vqt_name` (130) — `INTIN[0]`

`vdi_text.c:1100-1132`: `element` ist der 1-basierte Index in der
Schriftartenkette. Nicht gefunden → **Rückfall auf `fon6x6`**, ohne Fehler
(`:1124-1126`). → n Werte (geräteabhängig) + 1 Rückfallklasse.

### 2.17 `vst_font` (21), `vst_height` (12), `vst_point` (107)

Alle drei wählen eine Schriftart bzw. -größe aus der geladenen Kette; die
gültige Menge ist geräteabhängig (`font_ring`). Im NewTOS-Profil ist die Kette
das eingebaute Systemfontpaar. **Als Datenwert geführt, nicht als
Unterfunktion** — außer dem Rückfall „nicht gefunden → Systemfont".

### 2.18 Opcodes **ohne** Unterfunktion

`v_opnwk` 1 · `v_clswk` 2 · `v_clrwk` 3 · `v_pline` 6 · `v_pmarker` 7 ·
`v_gtext` 8 · `v_fillarea` 9 · `vsl_width` 16 · `vsl_color` 17 ·
`vsm_height` 19 · `vsm_color` 20 · `vst_color` 22 · `vsf_color` 25 ·
`vql_attributes` 35 · `vqm_attributes` 36 · `vqf_attributes` 37 ·
`vqt_attributes` 38 · `v_opnvwk` 100 · `v_clsvwk` 101 · `v_contourfill` 103 ·
`vsf_perimeter` 104 · `v_get_pixel` 105 · `vr_trnfm` 110 · `vsc_form` 111 ·
`vsf_udpat` 112 · `vsl_udsty` 113 · `vr_recfl` 114 · `vqt_extent` 116 ·
`vqt_width` 117 · `vex_timv` 118 · `vst_load_fonts` 119 ·
`vst_unload_fonts` 120 · `v_hide_c` 123 · `vq_mouse` 124 · `vex_butv` 125 ·
`vex_motv` 126 · `vex_curv` 127 · `vq_key_s` 128 · `vqt_fontinfo` 131 ·
`vex_wheelv` 134.

(`vsl_color`/`vsm_color`/`vst_color`/`vsf_color` tragen alle dieselbe
Klemmfunktion `validate_color_index` — eine Bereichsklasse, keine
Unterfunktion.)

### 2.19 Summe der Unterfunktionswerte

| Träger | Werte | Rückfallklassen |
|---|---|---|
| `v_escape` `CONTRL[5]` | 20 | 1 |
| `v_gdp` `CONTRL[5]` | 10 | 1 |
| `vro_cpyfm`/`vrt_cpyfm` `INTIN[0]` (16 × 2, je Opcode) | 64 | 2 |
| `vsf_style` (24 Muster + 12 Schraffuren) | 36 | 2 |
| `vsl_type` + `vsl_ends` + `vsm_type` | 19 | 3 |
| `vsf_interior` | 5 | 1 |
| `vswr_mode` | 4 | 1 |
| `vst_alignment` (3 + 6) | 9 | 2 |
| `vst_rotation` (gespeicherte Werte) | 5 | — |
| `vst_effects` (Bits) | 5 | — |
| `vsin_mode`/`vqin_mode` (4 Geräte × 2) | 8 | 2 |
| `v_locator`/`v_choice`/`v_string` Betriebsart + `v_string` Maske | 8 | — |
| `vq_extnd` | 2 | — |
| `vq_color` `INTIN[1]` | 2 | 1 (Index außerhalb) |
| `vs_clip` | 2 | — |
| `v_show_c` | 2 | — |
| `vqt_name` | — | 1 |
| **Summe** | **201** | **17** |

**Also: 74 Opcode-Plätze, davon 67 mit echtem Rumpf; 17 davon tragen
Unterfunktionen, 50 nicht; zusammen 201 diskrete Unterfunktionswerte +
17 Rückfallklassen + 2 Nicht-Wege der Opcode-Ebene (unbekannter Opcode;
ungültiges Handle).**

---

## 3. Was eine vollständige Falltabelle abdecken muss

1. **Jeden der 67 Opcodes mit Rumpf** im Gutfall.
2. **Jeden der 7 `vdi_v_nop`-Plätze** — sie müssen `contrl[2]=contrl[4]=0`
   liefern und nichts zeichnen. Ein Unterschied hier wäre beobachtbar.
3. **Mindestens einen Opcode außerhalb {1…39, 100…134}** (z. B. 0, 40, 99, 135).
4. **Ein ungültiges Handle** für jeden Opcode außer 1 und 100 — EmuTOS kehrt
   **vor** dem Sprung zurück (`vdi_main.c:141-143`), also ohne jede Wirkung.
5. **Jede Unterfunktion** aus Abschnitt 2. `vsf_style` (24) braucht **beide**
   Füllarten.
6. **`flip_y`** als Rückgabewert: EmuTOS setzt 1 nur in neun Rümpfen; NewTOS
   führt die Liste in `vdi/mod.rs:418-421`. Jeder Opcode ist hier ein Messfall.

### Strukturunterschied, der die Messung betrifft

EmuTOS setzt `contrl[2]`/`contrl[4]` **vor** dem Sprung aus der Sprungtabelle
(`vdi_main.c:158-159`); NewTOS setzt beide auf 0 und überlässt es dem Rumpf
(`vdi/mod.rs:270-272`). Für die sieben `vdi_v_nop`-Plätze sind die
Tabellenwerte ohnehin `{0,0}`; für die übrigen setzen die NewTOS-Rümpfe die
Zählwerte selbst. **Eine Falltabelle muss `contrl[2]`/`contrl[4]` trotzdem für
jeden Opcode auslesen** — das ist genau die Stelle, an der ein vergessener
`set_contrl` sichtbar würde.

### Abschnitte für die Messtabelle

**A — frei messbar.** Alle Attributsetzer (12–25, 32, 39, 104, 106–108, 112,
113, 129), alle Abfragen (26, 35–38, 102, 105, 115–117, 124, 128, 130, 131),
`vsin_mode` 33, `v_choice` 30, die `vex_*` 118/125/126/127/134, `v_gdp` 11,
`v_pline`/`v_pmarker`/`v_gtext`/`v_fillarea`, `vr_recfl` 114, `vro_cpyfm` 109,
`vrt_cpyfm` 121, `vr_trnfm` 110, `v_escape` 1/15/16/18/19, alle
`vdi_v_nop`-Plätze, unbekannte Opcodes.

**B — blockiert auf Eingabe.** `v_locator` 28 in REQUEST, `v_string` 31 in
REQUEST.

**C — verändert den Bildschirm oder den Zustand grundlegend.** `v_opnwk` 1,
`v_clswk` 2, `v_clrwk` 3, `v_clsvwk` 101, `v_escape` 2/3 (Grafik-/Textumschaltung),
`v_escape` 17 (`Scrdmp`), `vsc_form` 111, `vst_load_fonts` 119.

**D — nur per Screendump.** Alles Zeichnende: 6–9, 11, 114, 109, 121, 122/123,
`v_escape` 4–14.

---

## 4. Belege

* Sprungtabellen: `vdi/vdi_main.c:32-72` (`jmptb1`), `:74-112` (`jmptb2`)
* Verteiler: `vdi/vdi_main.c:121-192`; Bereichsprüfung `:150-157`;
  Handle-Prüfung `:141-143`; `vdi_v_nop` `:23`
* Escape: `vdi/vdi_esc.c:23,293-315,334-350`
* GDP: `vdi/vdi_gdp.c:426-480`
* Raster-Modi: `vdi/vdi_raster.c:113,1330-1360`
* Wertebereiche: `vdi/vdi_defs.h:18,45-81`
* Profilgatter: `include/config.h` (mit `cc -E -dM -Iinclude -DTARGET_512`
  ausgewertet); `vdi/vdi_defs.h:18` für `HAVE_BEZIER`
* NewTOS-Verteiler: `tos-api/src/vdi/mod.rs:264-424`
* NewTOS-GEM-Trap: `kernel/src/arch/aarch64/exceptions.rs:373-430`
