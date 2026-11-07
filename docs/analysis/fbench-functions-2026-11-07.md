# Fbench: VDI- und AES-Funktionen gegen den NewTOS-Code (2026-11-07)

Auftrag: jede Funktion aus `VDI-AES-FUNKTIONEN.md` (alle Abschnitte, auch die
außerhalb der Zeitmessung) im NewTOS-Code finden und mit einem Status versehen.
Anlass: auf dem echten Pi zeigen viele Fbench-Tests ein leeres weißes Testfenster.
Dieser Bericht geht von **keiner** Erklärung des weißen Fensters aus; er sagt je
Funktion, was auf unserer Seite dahintersteht.

## 0. Kopf

| | |
|---|---|
| Liste | `/Volumes/Daten/Development/fbench/VDI-AES-FUNKTIONEN.md`, sha256 `206ca94d67e0d13e996fe4cd0d9d605c87570ffc472e99e6fc5cbc75fddc281b` (177 Zeilen) |
| Fbench-Quelle (nur gelesen) | tests.c sha256 `b80d6fee11a3a8db092d610a96ddba1e0668321b58414b07b455825cbe143c85` (1313 Zeilen), fbench.c sha256 `dc8232c2bf526332593780b7ce0fe4ec462de268762e16eb9f3ed7d09abc8d6b` (1683 Zeilen); ein anderer Lauf bearbeitet diesen Baum, die Hashes sind der Stand beim Lesen |
| NewTOS | Arbeitsbaum auf `8ec5e8d6` (HEAD beim Lesen; nichts gebaut, nichts gestartet) |
| EmuTOS | `references/emutos` (Pin `1a7484ab`); `references/tos4x/` nicht gelesen |
| Bindungen | gemlib im SDK: `sdk/gemlib/gemlib-0.44.0/` (VDI-Opcode je `VDI_TRAP(..., op, ...)`, AES-Opcode je `AES_PARAMS(op, ...)`) |

Lesehinweis. **Gelesener Code ist eine Hypothese.** "IMPLEMENTED" setzt voraus, dass
etwas die Wirkung der Funktion gegen das Orakel misst oder ein Hosttest sie prüft;
wo die einzige Fundstelle ein Fall ist, dessen verglichene Felder für diesen Opcode
leer sind (Zeichenprimitive schreiben nichts in die Parameterfelder), steht das
dabei und der Status ist höchstens IMPLEMENTED-UNMEASURED. Ich habe **nichts
ausgeführt** (kein `cargo`, kein Hatari, kein QEMU); alle Messzahlen stammen aus
vorhandenen Läufen und Quittungen, die ich zitiere.

## 1. Zusammenfassung

Gezählt wird je Funktion der Liste (Zeilen der Tabellen unten): 55 VDI-Funktionen
(37 aus Abschnitt 2, 15 aus Abschnitt 4, 3 neue aus Abschnitt 5) und 18 AES-Funktionen
(17 aus Abschnitt 3, `form_alert` aus Abschnitt 5). Unterfunktionen und Modi stehen
in Abschnitt 5 dieses Berichts, nicht in den Zählern.

| Status | VDI | AES | Summe |
|---|---|---|---|
| IMPLEMENTED | 45 | 16 | 61 |
| IMPLEMENTED-UNMEASURED | 7 | 1 | 8 |
| PARTIAL | 2 | 0 | 2 |
| STUB | 0 | 0 | 0 |
| MISSING | 0 | 1 | 1 |
| NOT-IN-EMUTOS | 1 | 0 | 1 |
| Summe | 55 | 18 | 73 |

### Alles, was nicht IMPLEMENTED ist

| Funktion | Status | Wirkung |
|---|---|---|
| `v_pmarker` | PARTIAL | EmuTOS setzt `vwk->clip = 1` und stellt es nicht zurück (`references/emutos/vdi/vdi_marker.c:114`, D-VDI-9 in `docs/api/vdi/_discrepancies.md`); NewTOS nimmt `&Vwk`, setzt `clip` nirgends (`tos-api/src/vdi/primitives.rs:849@v_pmarker`). Nach einem Marker-Aufruf gilt bei EmuTOS Clipping an, bei uns bleibt es wie vorher. Sichtbar über `vq_extnd` INTOUT[19] und über die Randfälle bei `vs_clip(0)`. Bei Fbench ohne Folge (Clip ist an). |
| `v_justified` | PARTIAL | Waagrechter Weg ignoriert `delh`: `h_align` 1/2 gegen `max_x` und der Kursiv-Versatz fehlen (`TASK/V37.md`, gemessen: Probe TVO 442, TVP 659 Wörter; ZG002/003/008/009 je 1 Pixel). Fbenchs `VDI.JUSTIFY` setzt links ausgerichtet und ohne Effekt, liegt also auf dem gemessenen Weg. |
| `v_rbox`, `v_rfbox` | IMPLEMENTED-UNMEASURED | Code arbeitet (`tos-api/src/vdi/gdp.rs:282@rbox`), aber kein Pixelvergleich und kein Test gefunden; DUMPVDI-Fälle 11/8, 11/9 vergleichen nur leere Felder. |
| `v_hide_c`, `v_show_c` | IMPLEMENTED-UNMEASURED | Zähler und Zeichnen im Code (`tos-api/src/vdi/mouse.rs:838@v_hide_c`, `:816@v_show_c`); die Fälle 122/123 in `vdi-all` haben leere Ausgaben, die Wirkung (Zeiger sichtbar oder nicht) ist nicht verglichen. |
| `v_clsvwk` | IMPLEMENTED-UNMEASURED | Kein Fall (im Dumper ausgeschlossen, `tosdump/cases/COVERAGE.md` Zeile 101); nur als Lifecycle genutzt (`tosdump/core/gem.c:120@v_clsvwk`). |
| `vq_gdos`, `vq_vgdos` | IMPLEMENTED-UNMEASURED | Der Trap kommt mit unverändertem `x0` zurück ("kein GDOS"), wie EmuTOS (`references/emutos/vdi/vdi_asm.S:40-49`). Der einzige Test dafür (`kernel/src/selftest_gemtrap.rs`) hat keinen Aufrufer. Folge für Fbench: GDOS-Zweige (`vst_load_fonts`, `GDOS.NAMES`, `GDOS.LOAD`) laufen auf NewTOS nie. |
| `objc_find` | IMPLEMENTED-UNMEASURED | Code liest alle Argumente (`tos-api/src/aes/object.rs:2322@objc_find`); weder ein tosdump-Fall noch eine Datei in `tos-api/tests/` nennt ihn. |
| `appl_yield` (als Fbench es aufruft: `_appl_yield`, Trap-Selektor 201) | MISSING | EmuTOS: `super()` ruft bei 201 `dsptch()` (`references/emutos/aes/gemsuper.c:455-466`). NewTOS: Selektor 201 fällt in den Arm "x0 unverändert" (`kernel/src/arch/aarch64/exceptions.rs:579@gpr`), das SDK sagt selbst "INERT" (`sdk/gemlib/gemlib-0.44.0/a__appl_yield.c:26@_appl_yield`). Fbench ruft das in **jedem** `drain_events` auf, weil NewTOS `ap_count` 1 und `ap_version` 0x0340 meldet (`references/emutos/aes/gemsuper.c:92-93`, `include/config.h:1347`). Der AES-Opcode 17 (`appl::appl_yield`) existiert und ist von dieser Lücke getrennt (Abschnitt 5). |
| `vst_arbpt` | NOT-IN-EMUTOS | Opcode 246 (SpeedoGDOS) steht in keiner der beiden Sprungtabellen (`references/emutos/vdi/vdi_main.c:32-112`); bei uns `_ => {}` (`tos-api/src/vdi/mod.rs:587@v_cellarray`). Fbench führt `GDOS.ARBPT` nur aus, wenn GDOS FSM/Speedo ist oder die Schrift Outline ist (`a_vector`), auf NewTOS also nicht. |

### IMPLEMENTED, aber mit offener, belegter Abweichung auf einem Fbench-Pfad

Das sind keine Status-Abstriche (die Funktion tut ihre Arbeit und ist gemessen), aber
wer ein Pixelbild von Fbench mit EmuTOS vergleicht, trifft sie:

- `v_gtext` mit `vst_effects(THICKENED|SKEWED|UNDERLINED|OUTLINED)` in `MD_REPLACE`
  (Fbench `VDI.TEXTEFF`): `TASK/V41.md` (neun Ursachen, Fix offen, "ans Listenende").
  Betrifft auch `GDOS.BOLD10` (fett, geladene Fonts).
- `v_gtext`/`vst_height`/`vst_point` skaliert und mit Clip: `TASK/V31.md`, `TASK/V45.md`
  (Probe ZG012/ZG013, 523 und 404 Pixel).
- `v_contourfill`: Notbremse `TASK/V38.md` (Robustheit, keine Pixelabweichung).
- `vro_cpyfm`: Überlauf bei extremen `PTSIN` `TASK/V27.md`; sechs Fälle von
  `vdi-v18-t2-raster.tsv` weichen nur hinter dem Formularende ab (`TASK/V28-STATUS-DONE.MD`).
- AES `wind_get` `WF_FIRSTXYWH/NEXTXYWH`: gegen das Orakel bleiben sieben Fälle der
  Rechteckliste (261, 280, 400, 481, 494, 574, 588 aus `aes-r3-wind-state.tsv`) in den
  Ausgabewörtern x/y **hinter dem Listenende** verschieden (EmuTOS lässt dort `31,63`
  usw. stehen, NewTOS `0,0`); `w` und `h` sind gleich und nur diese beiden liest
  Fbenchs Schleife (`measurements/measurement-2026-11-06-v32-test/runs/t5-head-r3/tosdiff-EMUTOS-1a7484ab-AES-r3.txt`;
  Einordnung `docs/analysis/aes-r3-wind-state-einordnung-2026-09-12.md`).

### Offene Frage, die ich nicht geklärt habe

Die jüngsten Läufe (`measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT` Zeile 22
und die V24-V33-Läufe) vermerken "aes-r2-wind 78 von 78 und aes-r3-wind-state 39 von 39
weichen vom Orakel ab, auf beiden Seiten gleich". Im tosdiff-Text von r2
(`measurements/measurement-2026-11-06-v32-test/runs/t5-head-r2/tosdiff-EMUTOS-1a7484ab-AES-r2wind.txt`)
sind das ausschließlich `global[3]`, `global[4]`, `global[10]` (234 Zeilen, keine andere
Zeile). Das sieht nach Bytefolge der `ap_private`-Zelle und nach `ap_planes` (Gerät) aus; **das ist
Lesen, nicht Messen**. Der Hosttest-Replay `tos-api/tests/aes_reference/harness.rs` führt
`GLOBAL_EXCLUDED` getrennt. Wer wissen will, ob die AES-Fensterfunktionen "gleich dem
Orakel" sind, muss `global[]` aus dem Vergleich nehmen und neu zählen; ich habe das nicht getan.

## 2. Binding und Trap-Ebene (gilt für alle Zeilen)

- VDI: gemlib `vdi()` ruft `svc #2` mit `x0 = 115`, `x1` = Parameterblock
  (`sdk/gemlib/gemlib-0.44.0/_gc_vdi.c:36@vdi`). Kernel: `115 => dispatch_in`
  (`kernel/src/arch/aarch64/exceptions.rs:531@dispatch_in`), dann `screen()`
  (`tos-api/src/vdi/mod.rs:442@screen`); EmuTOS: `references/emutos/vdi/vdi_main.c:121-192`.
- AES: `x0 = 200`; Opcode-Schalter `tos-api/src/aes/mod.rs:856@crysbind`
  (EmuTOS `references/emutos/aes/gemsuper.c:77-421`). Ein nicht aufgeführter Opcode liefert `-1`
  (`tos-api/src/aes/mod.rs:992@set_ret`, EmuTOS `gemsuper.c:406-417`).
- Ohne Rasterbackend (`host().raster()`) ist die VDI inaktiv (`tos-api/src/vdi/mod.rs:442@screen`,
  frühe Rückgabe). Alle Zeichenprimitive enden im Backend
  `kernel/src/vdi_backend.rs` (`fill_pattern` `:647@fill_pattern`, `copy_rect` `:797@copy_rect`,
  `abline` `:608@abline`, Glyphen `:678@draw_glyph_sized`); wie diese Schreibzugriffe auf der
  echten Pi den Bildschirm erreichen (Cache, Flush `:942@flush_rect`) habe ich **nicht** untersucht.
- Unbekannte Opcodes, `v_updwk`, `v_cellarray`, `vq_cellarray`, `v_valuator` sind No-ops
  (`tos-api/src/vdi/mod.rs:587@v_cellarray`; EmuTOS `vdi_main.c` Zeilen "vdi_v_nop").
- Das **GDOS-Probe** (`d0 = -2`) und die Anfrage `d0 = -1` sind keine VDI-Opcodes:
  `docs/api/vdi/_dispatcher.md` Abschnitt "`d0` special values" (Zeilen 55-62).

## 3. Legende der Evidenz

| Kürzel | Bedeutung |
|---|---|
| **A** | DUMPVDI-Tabelle `tosdump/cases/vdi-all.tsv` (182 Fälle, Orakel `tosdump/reference/EMUTOS-1a7484ab-VDI-all-falcontc.BIN`). NewTOS gegen Orakel zuletzt **0 von 182** abweichend (`measurements/measurement-2026-11-06-v18-t2/RUNINFO.TXT`, ebenso v32-test, v44-test). Verglichen werden `contrl`, `intin`, `ptsin`, `intout`, `ptsout`; Zeichenprimitive lassen die Ausgaben leer, **A sagt dort nichts über Pixel**. "A op N" = der Opcode kommt in der Tabelle vor. |
| **B** | `tosdump/cases/vdi-v18.tsv` (19 Fälle, zeichnet im Prelude und liest mit `v_get_pixel` zurück), 0 von 19. |
| **C** | Pixel-Probe `tosdump/probes/vdprobe.c` (zeichnet auf gelöschtem Schirm, liest per `vro_cpyfm` zurück, gleiche C-Quelle auf EmuTOS-Falcon-Truecolor und NewTOS, Fälle `K*`, `T*`, `TV*`, `TX*`, `TY*`, `Z*`); Fallnamen stehen in den Zellen. Der Messstand je Fall steht in den Quittungen der V-Reihe (zum Beispiel `TASK/V19-STATUS-DONE.MD`, `TASK/V33-STATUS-DONE.MD`, `TASK/V44-STATUS-DONE.MD`). |
| **D** | DUMPRAST-Tabellen `tosdump/cases/vdi-raster.tsv`, `vdi-raster-screen.tsv`, `vdi-v18-raster.tsv`, `vdi-v18-t2-raster.tsv`. Letzter Stand: raster-screen 0/62, v18-raster 0/17, raster 1/59 (Fall 157, in der Tabelle vorhergesagt), v18-t2-raster 6/26 (`measurements/measurement-2026-11-06-v44-test/RUNINFO.TXT` Zeilen 40-41). |
| **R** | AES-Replay gegen die Orakel-Dumps: `tos-api/tests/aes_reference/` (r1, r2-wind, m05, m06, m07). Läuft im `cargo xtask test` (2120 bestanden / 0 fehlgeschlagen / 1 ignoriert laut `measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT` Zeile 17); von mir nicht neu gelaufen. |
| **H** | Hosttest in `tos-api/tests/` (Dateiname in der Zelle), prüft NewTOS-Verhalten, nicht notwendig gegen das Orakel. |
| **P** | Pixelreferenz gegen Hatari-Falcon: `tos-api/tests/aes_window_frame_reference.rs`. |

Aktuelle Läufe der AES-Tabellen r2b/r3/r4 gegen NewTOS: r4 hat keine Orakeldatei
(`measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT` Zeile 22).

## 4. VDI

Spalten: Funktion | VDI-Opcode (gemlib) | NewTOS: Dispatch / Arbeit | EmuTOS | Spec | Evidenz | Status | zeichnet ins Testfenster (nach tests.c)?

### 4.1 Linien und Marker

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `v_pline` | 6 | `tos-api/src/vdi/mod.rs:547@v_pline` / `tos-api/src/vdi/primitives.rs:797@v_pline` (Breite 1: `polyline`, sonst `wideline`) → `kernel/src/vdi_backend.rs:608@abline`; liest `CONTRL[1]`, `PTSIN`, Linienattribute | `references/emutos/vdi/vdi_line.c:1000` | `docs/api/vdi/output/_lines-markers.md` | A op 6; B (+6, 127 Preludes); C K1-K3, K5C, K11, TP*; Fixes `TASK/V20-STATUS-DONE.MD`, `TASK/V23-STATUS-DONE.MD`; offen `TASK/V40.md` (Ein-Punkt-XOR, nicht Fbench) | IMPLEMENTED | ja |
| `v_pmarker` | 7 | `tos-api/src/vdi/mod.rs:548@v_pmarker` / `tos-api/src/vdi/primitives.rs:849@v_pmarker` (sechs Glyphtabellen, Polylinien, Skala) | `references/emutos/vdi/vdi_marker.c:86` | `docs/api/vdi/output/_lines-markers.md` | A op 7 (leer); C K5A/B (Kappung, Marker "plus"); Typ 6 (Fbench) nicht pixelverglichen | **PARTIAL**: Seiteneffekt `clip = 1` fehlt (siehe Zusammenfassung) | ja |

### 4.2 Flächen

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `vr_recfl` | 114 | `tos-api/src/vdi/mod.rs:580@vr_recfl` / `tos-api/src/vdi/primitives.rs:571@vr_recfl` → `kernel/src/vdi_backend.rs:647@fill_pattern` | `references/emutos/vdi/vdi_fill.c:293` | `docs/api/vdi/output/_fill.md` | C Fall CAL (Stifte 0..15); D raster-screen (+114 ×11); B (+114 ×4) | IMPLEMENTED | ja |
| `v_bar` (GDP 1) | 11/1 | `tos-api/src/vdi/mod.rs:551@v_gdp` / `tos-api/src/vdi/gdp.rs:80@bar` (`fill_pattern` + Rahmen bei `fill_per`) | `references/emutos/vdi/vdi_gdp.c:426` (Fall 1) | `docs/api/vdi/output/_gdp.md` | A op 11/1; B (+11/1); C K6A-F | IMPLEMENTED | ja |
| `v_fillarea` | 9 | `tos-api/src/vdi/mod.rs:550@v_fillarea` / `tos-api/src/vdi/primitives.rs:760@v_fillarea` → `fill_polygon` | `references/emutos/vdi/vdi_fill.c:617` | `docs/api/vdi/output/_fill.md` | A op 9; B 51/53; C K4A-H, K5D; `TASK/V22-STATUS-DONE.MD`, `TASK/V23-STATUS-DONE.MD` | IMPLEMENTED | ja |
| `v_contourfill` | 103 | `tos-api/src/vdi/mod.rs:552@v_contourfill` / `tos-api/src/vdi/contour.rs:353@v_contourfill` (Seed-Queue) | `references/emutos/vdi/vdi_fill.c:1084` | `docs/api/vdi/output/_fill.md` | A op 103; B; C K8A-J, K14A-C; `TASK/V21-STATUS-DONE.MD`; offen `TASK/V38.md` | IMPLEMENTED | ja |

### 4.3 Kreise, Bögen, Rundrechtecke

Alle GDP-Unteropcodes laufen über `tos-api/src/vdi/gdp.rs:39@v_gdp` (alle Unteropcodes 1-10 vorhanden, unbekannte = No-op wie in EmuTOS `references/emutos/vdi/vdi_gdp.c:426-476`; Unteropcode 13 (Bezier, bei EmuTOS nur mit `HAVE_BEZIER`) benutzt Fbench nicht und ist nicht geprüft).
Arbeit der Kurven: `tos-api/src/vdi/gdp.rs:162@curve`.
Pixelvergleiche gibt es für Randfälle (Radius 0-2, Clip, breite Bögen) und je eine Kontrolle; Fbenchs zufällig große Formen sind nicht einzeln verglichen.

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `v_circle` | 11/4 | `tos-api/src/vdi/mod.rs:551@v_gdp` / `tos-api/src/vdi/gdp.rs:162@curve` | `references/emutos/vdi/vdi_gdp.c:371` | `docs/api/vdi/output/_gdp.md` | A op 11/4; C K15A-E, K15Q; `TASK/V22-STATUS-DONE.MD` | IMPLEMENTED | ja |
| `v_ellipse` | 11/5 | wie oben | `references/emutos/vdi/vdi_gdp.c:371` | `docs/api/vdi/output/_gdp.md` | A op 11/5; C K15F-J | IMPLEMENTED | ja |
| `v_arc` | 11/2 | wie oben | `references/emutos/vdi/vdi_gdp.c:371` | `docs/api/vdi/output/_gdp.md` | A op 11/2; B (+11/2); C K11A, K11C, K11D, K17; `TASK/V25-STATUS-DONE.MD` | IMPLEMENTED | ja |
| `v_pieslice` | 11/3 | wie oben | `references/emutos/vdi/vdi_gdp.c:371` | `docs/api/vdi/output/_gdp.md` | A op 11/3; C K15K-N | IMPLEMENTED | ja |
| `v_ellarc` | 11/6 | wie oben | `references/emutos/vdi/vdi_gdp.c:371` | `docs/api/vdi/output/_gdp.md` | A op 11/6; C K11B | IMPLEMENTED | ja |
| `v_ellpie` | 11/7 | wie oben | `references/emutos/vdi/vdi_gdp.c:371` | `docs/api/vdi/output/_gdp.md` | A op 11/7; C K15O, K15P | IMPLEMENTED | ja |
| `v_rbox` | 11/8 | `tos-api/src/vdi/gdp.rs:282@rbox` | `references/emutos/vdi/vdi_gdp.c:229` | `docs/api/vdi/output/_gdp.md` | A op 11/8 (leer); kein Pixelfall, kein Hosttest | **IMPLEMENTED-UNMEASURED** | ja |
| `v_rfbox` | 11/9 | `tos-api/src/vdi/gdp.rs:282@rbox` (Parameter `filled`) | `references/emutos/vdi/vdi_gdp.c:229` | `docs/api/vdi/output/_gdp.md` | A op 11/9 (leer); wie `v_rbox` | **IMPLEMENTED-UNMEASURED** | ja |

### 4.4 Text

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `v_gtext` | 8 | `tos-api/src/vdi/mod.rs:549@v_gtext` / `tos-api/src/vdi/text.rs:1011@v_gtext` → `kernel/src/vdi_backend.rs:678@draw_glyph_sized` | `references/emutos/vdi/vdi_text.c:546` | `docs/api/vdi/output/_text.md` | A op 8; D/vdi-v18-t2 (+8 ×10); C T14*, TU*, TV*, TX*, TY*, Z*; Quittungen `TASK/V29-STATUS-DONE.MD`, `TASK/V30-STATUS-DONE.MD`, `TASK/V32-STATUS-DONE.MD`, `TASK/V33-STATUS-DONE.MD`, `TASK/V44-STATUS-DONE.MD`; offen `TASK/V31.md`, `TASK/V39.md`, `TASK/V41.md`, `TASK/V42.md`, `TASK/V43.md`, `TASK/V45.md` | IMPLEMENTED (offene Abweichungen siehe Zusammenfassung) | ja |
| `v_justified` (GDP 10) | 11/10 | `tos-api/src/vdi/mod.rs:551@v_gdp` / `tos-api/src/vdi/text.rs:1246@v_justified` | `references/emutos/vdi/vdi_text.c:1177` (`gdp_justified`) | `docs/api/vdi/output/_gdp.md` | A op 11/10; C T16/K16, T14H/I, TVM-TVQ, TWA; `TASK/V29-STATUS-DONE.MD`; offen `TASK/V37.md` | **PARTIAL** (`h_align` 1/2 gegen `max_x`, Kursiv-Versatz) | ja |
| `vqt_extent` | 116 | `tos-api/src/vdi/mod.rs:575@vqt_extent` / `tos-api/src/vdi/inquiry.rs:229@vqt_extent` | `references/emutos/vdi/vdi_text.c:1024` | `docs/api/vdi/inquiry/_text.md` | A op 116 (3 Fälle); vdi-v18-t2 op 116 (2); C TXI | IMPLEMENTED | nein |

### 4.5 Raster

Zu den Modi: Fbench benutzt `vro_cpyfm` nur mit `S_ONLY` (3) und `vrt_cpyfm` nur mit `MD_TRANS` (2); die Bildschirm-MFDB hat `fd_addr == NULL`. Auf dem Pi ist `planes` 16 (Truecolor).

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `vro_cpyfm` | 109 | `tos-api/src/vdi/mod.rs:582@vro_cpyfm` / `tos-api/src/vdi/raster.rs:796@vro_cpyfm` (Bildschirm→Bildschirm `kernel/src/vdi_backend.rs:797@copy_rect`, Speicher↔Bildschirm `mem_to_screen`/`screen_to_mem`) | `references/emutos/vdi/vdi_raster.c:1471` (`cpy_raster` `:1329`) | `docs/api/vdi/raster/_raster.md` | D (vdi-raster 59 Fälle, 58 gleich; raster-screen 45 Fälle op 109, 0 Abweichungen; v18-raster); `TASK/V19-STATUS-DONE.MD`, `TASK/V28-STATUS-DONE.MD`; offen `TASK/V27.md` | IMPLEMENTED | Bildschirm→Bildschirm: ja (schreibt Pixel), Speicher→Bildschirm: ja, Bildschirm→Speicher: nein |
| `vrt_cpyfm` | 121 | `tos-api/src/vdi/mod.rs:581@vrt_cpyfm` / `tos-api/src/vdi/raster.rs:544@vrt_cpyfm` (Modus 1-4, ungültig = No-op) | `references/emutos/vdi/vdi_raster.c:1493` | `docs/api/vdi/raster/_raster.md` | D (vdi-raster; raster-screen 17 Fälle op 121; v18-t2-raster 13 Fälle op 121) | IMPLEMENTED | ja |
| `vr_trnfm` | 110 | `tos-api/src/vdi/mod.rs:583@vr_trnfm` / `tos-api/src/vdi/raster.rs:1497@vr_trnfm` | `references/emutos/vdi/vdi_raster.c:269` | `docs/api/vdi/raster/_raster.md` | D (vdi-raster ×17) | IMPLEMENTED | nein (Speicher→Speicher; Fbench führt `VDI.TRNFM` bei `planes > 8` nicht aus) |
| `v_get_pixel` | 105 | `tos-api/src/vdi/mod.rs:559@v_get_pixel` / `tos-api/src/vdi/inquiry.rs:190@v_get_pixel` | `references/emutos/vdi/vdi_fill.c:1095` | `docs/api/vdi/inquiry/_color-pixel-input.md` | A op 105 (5); B op 105 (19); C K10, K16; `TASK/V24-STATUS-DONE.MD` | IMPLEMENTED | nein |

### 4.6 Attribute und Clipping (in der Messung)

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `vsl_color` | 17 | `tos-api/src/vdi/mod.rs:524@vsl_color` / `tos-api/src/vdi/attributes.rs:525@vsl_color` | `references/emutos/vdi/vdi_line.c:143` | `docs/api/vdi/attributes/_line-marker.md` | A op 17 (4, Echo in `intout`) | IMPLEMENTED | nein |
| `vst_color` | 22 | `tos-api/src/vdi/mod.rs:531@vst_color` / `tos-api/src/vdi/attributes.rs:541@vst_color` | `references/emutos/vdi/vdi_text.c:988` | `docs/api/vdi/attributes/_text.md` | A op 22 (2) | IMPLEMENTED | nein |
| `vsf_color` | 25 | `tos-api/src/vdi/mod.rs:530@vsf_color` / `tos-api/src/vdi/attributes.rs:533@vsf_color` | `references/emutos/vdi/vdi_fill.c:206` | `docs/api/vdi/attributes/_fill.md` | A op 25; `tosdump/cases/vdi-a131-palette.tsv` (16) | IMPLEMENTED | nein |
| `vsf_interior` | 23 | `tos-api/src/vdi/mod.rs:532@vsf_interior` / `tos-api/src/vdi/attributes.rs:563@vsf_interior` | `references/emutos/vdi/vdi_fill.c:173` | `docs/api/vdi/attributes/_fill.md` | A op 23 (6) | IMPLEMENTED | nein |
| `vs_clip` | 129 | `tos-api/src/vdi/mod.rs:543@vs_clip` / `tos-api/src/vdi/attributes.rs:50@vs_clip` | `references/emutos/vdi/vdi_control.c:235` | `docs/api/vdi/attributes/_mode-clip-color.md` | A op 129 (2); B (+129 ×5); D raster-screen (+129 ×4) | IMPLEMENTED | nein |

### 4.7 Maus und Workstation

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `v_hide_c` | 123 | `tos-api/src/vdi/mod.rs:568@v_hide_c` / `tos-api/src/vdi/mouse.rs:838@v_hide_c` | `references/emutos/vdi/vdi_mouse.c:331` (`linea_hide_mouse` `:1133`) | `docs/api/vdi/input/_mouse.md` | A op 123 (leere Ausgabe) | **IMPLEMENTED-UNMEASURED** | nein (nur Zeigerbild) |
| `v_show_c` | 122 | `tos-api/src/vdi/mod.rs:567@v_show_c` / `tos-api/src/vdi/mouse.rs:816@v_show_c` (INTIN[0]==0 setzt Zähler auf 1, wie `linea_show_mouse`) | `references/emutos/vdi/vdi_mouse.c:321` (`linea_show_mouse` `:1125`) | `docs/api/vdi/input/_mouse.md` | A op 122 (2, leere Ausgabe) | **IMPLEMENTED-UNMEASURED** | nein |
| `v_opnvwk` | 100 | `tos-api/src/vdi/mod.rs:463@v_opnvwk` / `tos-api/src/vdi/workstation.rs:283@v_opnvwk` (`alloc_handle`, `init_wk`) | `references/emutos/vdi/vdi_control.c:407` | `docs/api/vdi/workstation/100-v_opnvwk.md` | A op 100 (1) | IMPLEMENTED | nein |
| `v_clsvwk` | 101 | `tos-api/src/vdi/mod.rs:510@v_clsvwk` / `tos-api/src/vdi/workstation.rs:399@v_clsvwk` (Handle freigeben; physisches Handle geschützt) | `references/emutos/vdi/vdi_control.c:462` | `docs/api/vdi/workstation/101-v_clsvwk.md` | kein Fall (ausgeschlossen); Lifecycle im Dumper `tosdump/core/gem.c:120@v_clsvwk` | **IMPLEMENTED-UNMEASURED** | nein |
| `vq_gdos` | Trap `d0 = -2` (kein VDI-Opcode) | `sdk/gemlib/gemlib-0.44.0/_gc_vq_gdos.c:29@vq_gdos` → `svc #2` → `kernel/src/arch/aarch64/exceptions.rs:579@gpr` (x0 unverändert) | `references/emutos/vdi/vdi_asm.S:40-49` | `docs/api/vdi/_dispatcher.md` (Zeilen 55-62) | Selbsttest ohne Aufrufer (`kernel/src/selftest_gemtrap.rs`); keine Messung | **IMPLEMENTED-UNMEASURED** | nein |

### 4.8 GDOS-Schriften

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `vst_point` | 107 | `tos-api/src/vdi/mod.rs:541@vst_point` / `tos-api/src/vdi/attributes.rs:397@vst_point` | `references/emutos/vdi/vdi_text.c:820` | `docs/api/vdi/attributes/_text.md` | A op 107 (2); vdi-v18-t2 (4); C T15*, TX* | IMPLEMENTED | nein |
| `vst_font` | 21 | `tos-api/src/vdi/mod.rs:539@vst_font` / `tos-api/src/vdi/attributes.rs:176@vst_font` | `references/emutos/vdi/vdi_text.c:939` | `docs/api/vdi/attributes/_text.md` | A op 21 (3); vdi-v18-t2 (4); C T15*, TX* | IMPLEMENTED | nein |
| `vst_arbpt` | 246 (`sdk/gemlib/gemlib-0.44.0/n_vst_arbpt.c:35@VDI_TRAP`) | kein Arm; `tos-api/src/vdi/mod.rs:587@v_cellarray` (No-op) | nicht vorhanden (`references/emutos/vdi/vdi_main.c:32-112`) | nur Erwähnung in `docs/api/vdi/_dispatcher.md` (`vst_arbpt32`) | – | **NOT-IN-EMUTOS** | nein (gemlib liest danach nicht gesetztes `intout`) |
| `vqt_name` | 130 | `tos-api/src/vdi/mod.rs:578@vqt_name` / `tos-api/src/vdi/inquiry.rs:335@vqt_name` | `references/emutos/vdi/vdi_text.c:1100` | `docs/api/vdi/inquiry/_text.md` | A op 130 (4); C TXA, TXI | IMPLEMENTED | nein |
| `vst_load_fonts` | 119 | `tos-api/src/vdi/mod.rs:545@vst_load_fonts` / `tos-api/src/vdi/attributes.rs:416@vst_load_fonts` | `references/emutos/vdi/vdi_text.c:1292` | `docs/api/vdi/attributes/_text.md` | C T13A/B, TXA-TXH; `TASK/V33-STATUS-DONE.MD` (der frühere Stub, VDI-TEXT-010, ist behoben) | IMPLEMENTED | nein |
| `vst_unload_fonts` | 120 | `tos-api/src/vdi/mod.rs:546@vst_unload_fonts` / `tos-api/src/vdi/attributes.rs:459@vst_unload_fonts` | `references/emutos/vdi/vdi_text.c:1353` | `docs/api/vdi/attributes/_text.md` | C T13, TXG; `TASK/V33-STATUS-DONE.MD` | IMPLEMENTED | nein |

Fbench ruft `vst_load_fonts`/`vst_unload_fonts` nur, wenn `vq_gdos()` wahr ist (tests.c, `bench_open`, `fonts_init`, `b_gload`); auf NewTOS liefert `vq_gdos()` falsch, die Zeilen laufen also nie.

### 4.9 Nur im Setup (Abschnitt 4 der Liste)

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `vsl_width` | 16 | `tos-api/src/vdi/mod.rs:523@vsl_width` / `tos-api/src/vdi/attributes.rs:89@vsl_width` | `references/emutos/vdi/vdi_line.c:104` | `docs/api/vdi/attributes/_line-marker.md` | A op 16; B (+16) | IMPLEMENTED | nein |
| `vsl_ends` | 108 | `tos-api/src/vdi/mod.rs:525@vsl_ends` / `tos-api/src/vdi/attributes.rs:102@vsl_ends` | `references/emutos/vdi/vdi_line.c:128` | `docs/api/vdi/attributes/_line-marker.md` | A op 108 (4) | IMPLEMENTED | nein |
| `vsl_type` | 15 | `tos-api/src/vdi/mod.rs:522@vsl_type` / `tos-api/src/vdi/attributes.rs:79@vsl_type` | `references/emutos/vdi/vdi_line.c:90` | `docs/api/vdi/attributes/_line-marker.md` | A op 15 (5); B (+15) | IMPLEMENTED | nein |
| `vsl_udsty` | 113 | `tos-api/src/vdi/mod.rs:526@vsl_udsty` / `tos-api/src/vdi/attributes.rs:113@vsl_udsty` | `references/emutos/vdi/vdi_line.c:81` | `docs/api/vdi/attributes/_line-marker.md` | A op 113 (2); B (+113); C K2C/K3D | IMPLEMENTED | nein |
| `vsm_type` | 18 | `tos-api/src/vdi/mod.rs:527@vsm_type` / `tos-api/src/vdi/attributes.rs:119@vsm_type` | `references/emutos/vdi/vdi_marker.c:57` | `docs/api/vdi/attributes/_line-marker.md` | A op 18 (4) | IMPLEMENTED | nein |
| `vsm_height` | 19 | `tos-api/src/vdi/mod.rs:528@vsm_height` / `tos-api/src/vdi/attributes.rs:129@vsm_height` | `references/emutos/vdi/vdi_marker.c:30` | `docs/api/vdi/attributes/_line-marker.md` | A op 19 (3); vdi-v18-t2 (+19) | IMPLEMENTED | nein |
| `vsm_color` | 20 | `tos-api/src/vdi/mod.rs:529@vsm_color` / `tos-api/src/vdi/attributes.rs:140@vsm_color` | `references/emutos/vdi/vdi_marker.c:72` | `docs/api/vdi/attributes/_line-marker.md` | A op 20 (2) | IMPLEMENTED | nein |
| `vsf_style` | 24 | `tos-api/src/vdi/mod.rs:533@vsf_style` / `tos-api/src/vdi/attributes.rs:577@vsf_style` | `references/emutos/vdi/vdi_fill.c:186` | `docs/api/vdi/attributes/_fill.md` | A op 24 (4) | IMPLEMENTED | nein |
| `vsf_udpat` | 112 | `tos-api/src/vdi/mod.rs:542@vsf_udpat` / `tos-api/src/vdi/attributes.rs:616@vsf_udpat` (16 Wörter einplanig, `planes*16` mehrplanig, sonst keine Änderung) | `references/emutos/vdi/vdi_fill.c:149` | `docs/api/vdi/attributes/_fill.md` | C K6A-F (Ersetzen/Transparent, 16/64/256 Wörter); nicht in `vdi-all` (Dumper schließt aus) | IMPLEMENTED | nein |
| `vsf_perimeter` | 104 | `tos-api/src/vdi/mod.rs:534@vsf_perimeter` / `tos-api/src/vdi/attributes.rs:465@vsf_perimeter` | `references/emutos/vdi/vdi_fill.c:219` | `docs/api/vdi/attributes/_fill.md` | A op 104 (2) | IMPLEMENTED | nein |
| `vswr_mode` | 32 | `tos-api/src/vdi/mod.rs:535@vswr_mode` / `tos-api/src/vdi/attributes.rs:36@vswr_mode` | `references/emutos/vdi/vdi_control.c:256` | `docs/api/vdi/attributes/_mode-clip-color.md` | A op 32 (5); B (+32) | IMPLEMENTED | nein |
| `vst_effects` | 106 | `tos-api/src/vdi/mod.rs:537@vst_effects` / `tos-api/src/vdi/attributes.rs:149@vst_effects` | `references/emutos/vdi/vdi_text.c:881` | `docs/api/vdi/attributes/_text.md` | A op 106 (3, Echo); Wirkung auf Pixel siehe `v_gtext` (V41) | IMPLEMENTED | nein |
| `vst_rotation` | 13 | `tos-api/src/vdi/mod.rs:538@vst_rotation` / `tos-api/src/vdi/attributes.rs:160@vst_rotation` | `references/emutos/vdi/vdi_text.c:924` | `docs/api/vdi/attributes/_text.md` | A op 13 (5); vdi-v18-t2 (+13) | IMPLEMENTED | nein |
| `vst_alignment` | 39 | `tos-api/src/vdi/mod.rs:536@vst_alignment` / `tos-api/src/vdi/attributes.rs:551@vst_alignment` | `references/emutos/vdi/vdi_text.c:887` | `docs/api/vdi/attributes/_text.md` | A op 39 (4) | IMPLEMENTED | nein |
| `vst_height` | 12 | `tos-api/src/vdi/mod.rs:540@vst_height` / `tos-api/src/vdi/attributes.rs:349@vst_height` | `references/emutos/vdi/vdi_text.c:681` | `docs/api/vdi/attributes/_text.md` | A op 12 (3); vdi-v18-t2 (12 ×13); offen `TASK/V45.md` (mit Clip) | IMPLEMENTED | nein |

Alle Zeilen: Der Code liest sein Argument aus `INTIN`/`PTSIN` und schreibt den Zustand in die Workstation (nicht gelesen: ein fester Wert, ein ignoriertes Argument oder ein Default-Arm).
`vsl_udsty` hat keinen Echo-Ausgang in EmuTOS; die Messung dafür ist nur der Pixelfall (K2C/K3D).

### 4.10 Außerhalb der Tests (Abschnitt 5 der Liste, VDI-Teil)

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `vq_vgdos` | Trap `d0 = -2` | `sdk/gemlib/gemlib-0.44.0/_gc_vq_vgdos.c:22@vq_vgdos` → wie `vq_gdos` | `references/emutos/vdi/vdi_asm.S:40-49` | `docs/api/vdi/_dispatcher.md` | wie `vq_gdos` | **IMPLEMENTED-UNMEASURED** | nein |
| `vq_extnd` | 102 | `tos-api/src/vdi/mod.rs:520@vq_extnd` / `tos-api/src/vdi/workstation.rs:303@vq_extnd` (Modus 0: DEV_TAB/SIZ_TAB, Modus 1: INQ_TAB + Clip) | `references/emutos/vdi/vdi_control.c:626` | `docs/api/vdi/workstation/102-vq_extnd.md` | A op 102 (4) | IMPLEMENTED | nein |
| `vqt_attributes` | 38 | `tos-api/src/vdi/mod.rs:558@vqt_attributes` / `tos-api/src/vdi/inquiry.rs:63@vqt_attributes` | `references/emutos/vdi/vdi_text.c:999` | `docs/api/vdi/inquiry/_attributes.md` | A op 38 (8) | IMPLEMENTED | nein |
| `vst_load_fonts`, `vst_unload_fonts` | 119/120 | siehe 4.8 | | | | siehe 4.8 | nein |

`vq_extnd` Modus 1 meldet `INQ_TAB[8] = 1` (Textdrehung in 90-Grad-Schritten, `tos-api/src/vdi/workstation.rs:85@INQ_TAB_ROM`), also läuft `VDI.TEXTROT`; `INQ_TAB[4]` ist `planes` (16), also fällt `VDI.TRNFM` weg.

## 5. AES

Spalten wie oben; "Zeichnet" gilt für das **Testfenster** (nicht für Hilfsfenster).

### 5.1 Fensterverwaltung

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `wind_create` | 100 | `tos-api/src/aes/mod.rs:958@wind_create` / `tos-api/src/aes/window.rs:3379@wind_create` (Slot suchen, Rechtecke setzen) | `references/emutos/aes/gemwmlib.c:1369` (`wm_create`), Arm `gemsuper.c:330` | `docs/api/aes/window-graphics.md` | R (r2-wind: 11 Fälle op 100); H `aes_window_create_work.rs`, `aes_window_virgin_slot.rs`; tosdump r3 | IMPLEMENTED | nein |
| `wind_delete` | 103 | `tos-api/src/aes/mod.rs:961@wind_delete` / `tos-api/src/aes/window.rs:3752@wind_delete` (schließt offenes Fenster mit) | `references/emutos/aes/gemwmlib.c:1466` | `docs/api/aes/window-graphics.md` | R (r2-wind: 5); H `aes_window_delete_slot.rs` | IMPLEMENTED | nein |
| `wind_open` | 101 | `tos-api/src/aes/mod.rs:959@wind_open` / `tos-api/src/aes/window.rs:3447@wind_open` (setzt Rechteck, `bump_z`, zeichnet Rahmen, `send_redraw`) | `references/emutos/aes/gemwmlib.c:1410` | `docs/api/aes/window-graphics.md` | R (r2-wind: 5); P Rahmen-Pixel; H `aes_window_open_oldtop.rs` | IMPLEMENTED | indirekt (Rahmen + WM_REDRAW an den Besitzer) |
| `wind_close` | 102 | `tos-api/src/aes/mod.rs:960@wind_close` / `tos-api/src/aes/window.rs:3576@wind_close` | `references/emutos/aes/gemwmlib.c:1438` | `docs/api/aes/window-graphics.md` | R (r2-wind: 4); H `aes_window_hotclose.rs`, `aes_a105_frame_remnants.rs` | IMPLEMENTED | indirekt (Hintergrund + WM_REDRAW für aufgedeckte Fenster) |
| `wind_calc` | 108 | `tos-api/src/aes/mod.rs:968@wind_calc` / `tos-api/src/aes/window.rs:4732@wind_calc` | `references/emutos/aes/gemwmlib.c:1807` (`wm_calc`) | `docs/api/aes/window-graphics.md` | R (r1: 60 Fälle op 108) | IMPLEMENTED | nein |
| `wind_update` | 107 | `tos-api/src/aes/mod.rs:967@wind_update` / `tos-api/src/aes/window.rs:4300@wind_update` (BEG/END_UPDATE, MCTRL, Warteschlange) | `references/emutos/aes/gemwmlib.c:1778` (`wm_update`) | `docs/api/aes/window-graphics.md` | R (r2-wind: 7 Fälle op 107); H `aes_gh1_update_mutex.rs`, `aes_a118_lock_takers.rs` | IMPLEMENTED | nein |
| `wind_set` | 105 | `tos-api/src/aes/mod.rs:963@wind_set` / `tos-api/src/aes/window.rs:3999@wind_set` → `:4012@wm_set` | `references/emutos/aes/gemwmlib.c:1623` (`wm_set`) | `docs/api/aes/window-graphics.md` | siehe Modi unten | IMPLEMENTED | siehe Modi |
| `wind_get` | 104 | `tos-api/src/aes/mod.rs:962@wind_get` / `tos-api/src/aes/window.rs:3765@wind_get` | `references/emutos/aes/gemwmlib.c:1505` (`wm_get`) | `docs/api/aes/window-graphics.md` | siehe Modi unten | IMPLEMENTED | nein |

Modi, die Fbench benutzt (Konstanten gleich: `tos-types/src/aes.rs:453@WF_WXYWH`, `references/emutos/include/aesdefs.h:535-548`; gemlib-Namen `WF_WORKXYWH` 4, `WF_CURRXYWH` 5, `WF_VSLSIZE` 16 in `sdk/gemlib/gemlib-0.44.0/mt_gem.h:1507`):

| Aufruf | NewTOS | EmuTOS | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|
| `wind_set` `WF_CURRXYWH` (5) | `tos-api/src/aes/window.rs:4012@wm_set` → `:2226@draw_change_open` (Blit des obersten Fensters bei reinem Verschieben, sonst Abdeckung + `WM_REDRAW`) | `references/emutos/aes/gemwmlib.c:1676` | R (r2-wind: 8 Fälle op 105, r3 Preludes); H `aes_window_move_blit.rs` | IMPLEMENTED | indirekt |
| `wind_set` `WF_TOP` (10) | `tos-api/src/aes/window.rs:4012@wm_set` → `:878@wm_mktop` | `references/emutos/aes/gemwmlib.c:1679` | R (r2-wind: Fälle "WF_TOP"); H `aes_window_top_unopened.rs`, `aes_window_zorder_counter.rs` | IMPLEMENTED | indirekt |
| `wind_set` `WF_NAME` (2), `WF_INFO` (3) | `tos-api/src/aes/window.rs:3367@wf_str_ptr` (Zeiger aus `addr_in[0]` oder `int_in[2..]`), speichert den Zeiger, `redraw_open` | `references/emutos/aes/gemwmlib.c:1660`, `:1668` | H `aes_window_name_pointer.rs`, `aes_window_name_get.rs`; gemlib-Bindung `sdk/gemlib/gemlib-0.44.0/a_wind_set_str.c` (AIN_LEN 1) | IMPLEMENTED | indirekt (nur Fensterrahmen des Hilfsfensters) |
| `wind_set` `WF_VSLIDE` (9), `WF_VSLSIZE` (16) | `tos-api/src/aes/window.rs:4012@wm_set` (Klemmen 1..1000, Zeichnen nur bei Änderung) → `:3222@redraw_vslider` | `references/emutos/aes/gemwmlib.c:1697`, `:1711` | R (r2-wind: op 105 mit `WF_VSLIDE`/`WF_VSLSIZ`, teils als Prelude; r4 ohne Orakel); H `aes_window_slider_guard.rs`, `aes_window_slider_subtree.rs` | IMPLEMENTED | indirekt (Hilfsfenster) |
| `wind_get` `WF_WORKXYWH` (4) | `tos-api/src/aes/window.rs:3765@wind_get` (gespeichertes `work`) | `references/emutos/aes/gemwmlib.c:1529` | R (r1/r2: `wind_get` 22+31 Fälle); `aes_wind_get_desktop_geometry.rs` | IMPLEMENTED | nein |
| `wind_get` `WF_FIRSTXYWH` (11), `WF_NEXTXYWH` (12) | `tos-api/src/aes/window.rs:1765@compute_rlist`, `:1818@rlist_owns`, `:1058@border_pieces` | `references/emutos/aes/gemwmlib.c:1556` | tosdump r2b (15 Fälle) und r3 (38 Fälle) mit Orakeldatei; H `aes_window_rectlist_zorder.rs`, `aes_window_rectlist_capacity.rs`; Rest-Abweichung siehe Zusammenfassung (x/y hinter dem Listenende) | IMPLEMENTED | nein |
| `wind_calc` `WC_WORK` (1), `WC_BORDER` (0) | `tos-api/src/aes/window.rs:4732@wind_calc` → `:2383@calc` | `references/emutos/aes/gemwmlib.c:1807` | R (r1: 60 Fälle) | IMPLEMENTED | nein |
| `wind_update` `BEG_UPDATE`/`END_UPDATE` | `tos-api/src/aes/window.rs:4300@wind_update` | `references/emutos/aes/gemwmlib.c:1778` | R (r2-wind) | IMPLEMENTED | nein |

### 5.2 Objekte, Dialoge, Menü

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `objc_draw` | 42 | `tos-api/src/aes/mod.rs:939@objc_draw` / `tos-api/src/aes/object.rs:1123@objc_draw` → `render_tree` `:1252@render_tree_t` (liest Baum, Startobjekt, Tiefe, Clip) | `references/emutos/aes/gemoblib.c:959` | `docs/api/aes/object-form.md` | H `aes_object_3d_box_border.rs`, `aes_object_3d_text_button.rs`, `aes_object_nicelines.rs`, `aes_object_font.rs`; kein tosdump-Zeichenfall | IMPLEMENTED | ja (Baum bei `ax+8, ay+8`) |
| `objc_find` | 43 | `tos-api/src/aes/mod.rs:940@objc_find` / `tos-api/src/aes/object.rs:2322@objc_find` → `:4147@find_hit` | `references/emutos/aes/gemoblib.c:1034` | `docs/api/aes/object-form.md` | keine (kein Fall, kein Test) | **IMPLEMENTED-UNMEASURED** | nein |
| `form_dial` | 51 | `tos-api/src/aes/mod.rs:947@form_dial` / `tos-api/src/aes/form.rs:1019@form_dial` → `:987@fm_dial` (`FMD_FINISH` → `redraw_dialog_area` `tos-api/src/aes/window.rs:5054@redraw_dialog_area`) | `references/emutos/aes/gemfmlib.c:360` | `docs/api/aes/object-form.md` | H `aes_dialog_undraw.rs`, `aes_desk_background.rs`, `aes_desk_dialog_chain.rs` | IMPLEMENTED | indirekt (Desktop + `WM_REDRAW` an überlappende Fenster) |
| `menu_bar` | 30 | `tos-api/src/aes/mod.rs:927@menu_bar` / `tos-api/src/aes/menu.rs:345@menu_bar` (zeichnet Leiste, setzt Zustand) | `references/emutos/aes/gemmnlib.c:703` (`mn_bar`) | `docs/api/aes/menu.md` | H `aes_menu_bar_height.rs`, `aes_a100_a102_desk_menu_geometry.rs`, `aes_a106_restart.rs` | IMPLEMENTED | nein (Menüleiste am oberen Rand) |
| `form_alert` (nur bei Fehlern) | 52 | `tos-api/src/aes/mod.rs:948@form_alert` / `tos-api/src/aes/form.rs:120@form_alert` | `references/emutos/aes/gemfmalt.c:319` (`fm_alert`) | `docs/api/aes/object-form.md` | H `aes_form_alert_cursor_bracket.rs`; tosdump schließt aus (blockiert) | IMPLEMENTED | nein |

### 5.3 Maus, Ereignisse, Nachrichten

| Funktion | Op | NewTOS | EmuTOS | Spec | Evidenz | Status | Zeichnet |
|---|---|---|---|---|---|---|---|
| `graf_mkstate` | 79 | `tos-api/src/aes/mod.rs:926@graf_mkstate` / `tos-api/src/aes/graf.rs:814@graf_mkstate` (Position, Tasten, Umschalter aus VDI/GSX) | `references/emutos/aes/gemgrlib.c:458` | `docs/api/aes/event.md` | R (r1: Fall "void handler leaves ret = TRUE"); H `aes_a126_kstate_mask.rs` | IMPLEMENTED | nein |
| `graf_mouse` | 78 | `tos-api/src/aes/mod.rs:925@graf_mouse` / `tos-api/src/aes/graf.rs:755@graf_mouse` (`M_OFF`→`vdi::mouse::hide`, `M_ON`→`vdi::mouse::show`, Formen) | `references/emutos/aes/gemgrlib.c:419` (`gsx_moff`/`gsx_mon` `gemgsxif.c:440-455`) | `docs/api/aes/window-graphics.md` | H `aes_graf_mouse_invalid.rs`; r4 (17 Fälle, ohne Orakeldatei) | IMPLEMENTED | nein (nur Zeigerbild) |
| `evnt_multi` | 25 | `tos-api/src/aes/mod.rs:915@evnt_multi` / `tos-api/src/aes/event.rs:476@evnt_multi` (`MU_MESAG` Warteschlange, `MU_TIMER` Frist, `MU_KEYBD`, Tasten/Rechtecke; ruft `intercept()` des Control-Managers inline) | `references/emutos/aes/gemevlib.c:152` (`ev_multi`), Arm `gemsuper.c:145` | `docs/api/aes/event.md` | R (m07: 12 Fälle op 25); H `aes_a73_event_neighbours.rs`, `aes_a112_wake_conditions.rs` | IMPLEMENTED | indirekt (liefert `WM_REDRAW` an Fbenchs Handler) |
| `appl_write` | 12 | `tos-api/src/aes/mod.rs:902@appl_write` / `tos-api/src/aes/appl.rs:594@appl_write` (kopiert bis `MAX_MSG_WORDS`, `post_words`, blockiert bei voller Schlange) | `references/emutos/aes/gemaplib.c:69` (`ap_rdwr`), Arm `gemsuper.c:102` | `docs/api/aes/application.md` | H `aes_a116_appl_write_blocks.rs`, `aes_a113_accessory_pid.rs` | IMPLEMENTED | nein |
| `appl_yield`, wie Fbench es ruft: `_appl_yield()` | Trap-Selektor 201 (`sdk/gemlib/gemlib-0.44.0/a__appl_yield.c:26@_appl_yield`) | **kein Arm**: `kernel/src/arch/aarch64/exceptions.rs:579@gpr` (x0 unverändert, kein Dispatch) | `references/emutos/aes/gemsuper.c:455-466` (`case 201: dsptch()`) | `docs/api/aes/_dispatcher.md` Zeilen 91-93 und 273-275 ("offene Frage, geringe Priorität") | keine | **MISSING** | nein |
| – dazu der AES-Opcode 17 | 17 | `tos-api/src/aes/mod.rs:908@appl_yield` / `tos-api/src/aes/appl.rs:553@appl_yield` (`dsptch`, Yield an Peer) | `references/emutos/aes/gemsuper.c:116` (`#if CONF_WITH_PCGEM`, `docs/api/_emutos-config.md` Zeile 198; Schalter `include/config.h:1319-1320`) | `docs/api/aes/application.md` Abschnitt `appl_yield (17)` | H `aes_a112_wake_conditions.rs`; r4-Tabelle (ohne Orakel) | IMPLEMENTED (wird von Fbench nicht gerufen) | nein |

`evnt_multi` Modi: `MU_MESAG | MU_TIMER` (Zeit 0) und im Lauf `MU_KEYBD` sind die Bits `flags & MU_MESAG`, `MU_TIMER`, `MU_KEYBD` in `tos-api/src/aes/event.rs:476@evnt_multi`; alle drei haben einen Zweig, der Argumente liest und handelt (kein Default-Arm).
`graf_mouse`: EmuTOS hält einen AES-Zähler `gl_moff` und ruft die VDI nur beim Übergang 0↔1 (`references/emutos/aes/gemgsxif.c:440-455`); NewTOS reicht jeden `M_OFF`/`M_ON` an den VDI-Zähler durch (`tos-api/src/vdi/mouse.rs:855@hide`, `:864@show`). Bei ausgewogener Nutzung gleich; bei gemischtem `v_hide_c` im selben Programm kann der Zeigerzustand abweichen. **Gelesen, nicht gemessen.**

## 6. Fbench-Tests: soll der Testkörper ins Testfenster zeichnen?

Quelle: tests.c Tabelle ab Zeile 974 (`tests[]`) und die Körper (`b_*`) davor. "Testfenster" = `wins[WN_TEST]`, Zeichenfläche `tw_area`; vor jedem Test löscht `clear_area` sie auf Farbe 0 (tests.c `run_tests`). **Bedingungen** aus der Spalte `avail`; die Gruppen-Abkürzungen der Liste (Scroll-A, Scroll-B, Event) sind aufgelöst.
Einordnung: "ja" = der Körper schreibt Pixel in `tw_area`; "ja, aber gleiche Farbe" = er schreibt, aber nur Pixel, die dem gelöschten Hintergrund gleichen; "indirekt" = AES-Aufrufe, die über Hilfsfenster oder `WM_REDRAW` zeichnen lassen; "Hilfsfenster" = der Körper zeichnet, aber in das Hilfsfenster `aux`, nicht in das Testfenster; "nein".
Nicht bestimmbar: keiner der 115 Tests; wo der Inhalt vom Zufall abhängt (M2S), steht es in der Spalte "Bemerkung".

| Test | Zeile | Körper | Zeichnet | Bemerkung | Bedingung |
|---|---|---|---|---|---|
| VDI.PLINE2 | tests.c line 976 | b_pline2 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.PLINE32 | tests.c line 977 | b_pline32 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.WIDELN | tests.c line 978 | b_pline2 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.DASHLN | tests.c line 979 | b_pline2 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.USERLN | tests.c line 980 | b_pline2 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.XORLN | tests.c line 981 | b_pline2 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.MARKER | tests.c line 982 | b_marker | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.RECT32 | tests.c line 983 | b_rect32 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.RECTFUL | tests.c line 984 | b_rectfull | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.BAR | tests.c line 985 | b_bar | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.PATTERN | tests.c line 986 | b_rect32 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.UDPAT | tests.c line 987 | b_rect32 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.XORFILL | tests.c line 988 | b_rect32 | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.POLYGON | tests.c line 989 | b_polygon | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.CIRCLE | tests.c line 990 | b_circle | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.ELLIPSE | tests.c line 991 | b_ellipse | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.ARC | tests.c line 992 | b_arc | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.PIE | tests.c line 993 | b_pie | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.ELLARC | tests.c line 994 | b_ellarc | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.ELLPIE | tests.c line 995 | b_ellpie | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.RBOX | tests.c line 996 | b_rbox | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.RFBOX | tests.c line 997 | b_rfbox | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.CONTOUR | tests.c line 998 | b_contour | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.TEXT | tests.c line 999 | b_text | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.TEXT80 | tests.c line 1000 | b_textlong | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.TEXTEFF | tests.c line 1001 | b_text | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.TEXTROT | tests.c line 1002 | b_text | yes | primitive drawn straight into tw_area (clip set by area_setup) | runs only if ext_out[8] > 0 |
| VDI.JUSTIFY | tests.c line 1003 | b_justified | yes | primitive drawn straight into tw_area (clip set by area_setup) | - |
| VDI.EXTENT | tests.c line 1004 | b_extent | no | query only, vqt_extent returns PTSOUT | - |
| VDI.CPY64 | tests.c line 1005 | b_cpy64 | yes, but same colour | screen to screen copy of the cleared window area: copies background onto background, no visible change unless something was drawn first (nothing is in setup) | - |
| VDI.SCROLL | tests.c line 1006 | b_scroll | yes | setup paints stripes, body scrolls them | - |
| VDI.M2S | tests.c line 1007 | b_mem2scr | yes | memory to screen; the 64x64 source is a capture of the window corner made in setup (three shapes at random points, may be mostly background) | - |
| VDI.S2M | tests.c line 1008 | b_scr2mem | no | screen to memory, target is the off-screen buffer | - |
| VDI.VRTCPY | tests.c line 1009 | b_vrt | yes | mono form expanded onto the screen (MD_TRANS, colour col(k)) | - |
| VDI.TRNFM | tests.c line 1010 | b_trnfm | no | memory to memory; also skipped when planes > 8 (a_trnfm) | runs only if planes <= 8 |
| VDI.GETPIX | tests.c line 1011 | b_getpix | no | read only | - |
| VDI.MOUSE | tests.c line 1012 | b_mouse | no | hide/show cursor only | - |
| VDI.ATTR | tests.c line 1013 | b_attrs | no | three attribute setters | - |
| VDI.VWK | tests.c line 1014 | b_vwk | no | open/close workstation | - |
| AES.CREATE | tests.c line 1017 | b_wcreate | no | wind_create+wind_delete, window never opened | - |
| AES.OPEN | tests.c line 1018 | b_wopen | indirectly | AES draws the helper window aux (placed inside the test window rectangle) and repaints what it uncovers; the test window gets WM_REDRAW and redraws via its own handler | - |
| AES.MOVE | tests.c line 1019 | b_wmove | indirectly | AES draws the helper window aux (placed inside the test window rectangle) and repaints what it uncovers; the test window gets WM_REDRAW and redraws via its own handler | - |
| AES.SIZE | tests.c line 1020 | b_wsize | indirectly | AES draws the helper window aux (placed inside the test window rectangle) and repaints what it uncovers; the test window gets WM_REDRAW and redraws via its own handler | - |
| AES.TOP | tests.c line 1021 | b_wtop | indirectly | AES draws the helper window aux (placed inside the test window rectangle) and repaints what it uncovers; the test window gets WM_REDRAW and redraws via its own handler | - |
| AES.NAME | tests.c line 1022 | b_wname | indirectly | AES repaints gadgets of the open helper window aux, not the test window content | - |
| AES.INFO | tests.c line 1023 | b_winfo | indirectly | AES repaints gadgets of the open helper window aux, not the test window content | - |
| AES.SLIDER | tests.c line 1024 | b_wslide | indirectly | AES repaints gadgets of the open helper window aux, not the test window content | - |
| AES.SCROLL | tests.c line 1025 | b_wscroll | yes | scroll_win(test window): blit, vr_recfl, v_gtext in the test window work area | - |
| AES.WGET | tests.c line 1026 | b_wget | no | query / lock / event / menu bar strip | - |
| AES.RECTS | tests.c line 1027 | b_wrect | no | query / lock / event / menu bar strip | - |
| AES.CALC | tests.c line 1028 | b_wcalc | no | query / lock / event / menu bar strip | - |
| AES.UPDATE | tests.c line 1029 | b_wupd | no | query / lock / event / menu bar strip | - |
| AES.OBJDRAW | tests.c line 1030 | b_objc | yes | objc_draw_grect(otree, 0, 8, tw_area), tree placed at ax+8, ay+8 | - |
| AES.OBJFIND | tests.c line 1031 | b_ofind | no | query / lock / event / menu bar strip | - |
| AES.FDIAL | tests.c line 1032 | b_fdial | indirectly | FMD_FINISH repaints desktop and sends WM_REDRAW for the 64x48 rect; the test window redraws only through its WM_REDRAW handler | - |
| AES.MENU | tests.c line 1033 | b_menubar | no | query / lock / event / menu bar strip | - |
| AES.MKSTATE | tests.c line 1034 | b_mkstate | no | query / lock / event / menu bar strip | - |
| AES.GMOUSE | tests.c line 1035 | b_gmouse | no | query / lock / event / menu bar strip | - |
| AES.EVPOLL | tests.c line 1036 | b_evpoll | no | query / lock / event / menu bar strip | - |
| AES.MSG | tests.c line 1037 | b_msg | no | query / lock / event / menu bar strip | - |
| ALN.RECT.A | tests.c line 1040 | b_rect64 | yes | vr_recfl 64x64 | - |
| ALN.RECT.U | tests.c line 1041 | b_rect64 | yes | vr_recfl 64x64 | - |
| ALN.CPY.A | tests.c line 1042 | b_cpy64 | yes, but same colour | same as VDI.CPY64 | - |
| ALN.CPY.U | tests.c line 1043 | b_cpy64 | yes, but same colour | same as VDI.CPY64 | - |
| ALN.M2S.A | tests.c line 1044 | b_mem2scr | yes | as VDI.M2S | - |
| ALN.M2S.U | tests.c line 1045 | b_mem2scr | yes | as VDI.M2S | - |
| ALN.VRT.A | tests.c line 1046 | b_vrt | yes | as VDI.VRTCPY | - |
| ALN.VRT.U | tests.c line 1047 | b_vrt | yes | as VDI.VRTCPY | - |
| ALN.TEXT.A | tests.c line 1048 | b_text | yes | v_gtext | - |
| ALN.TEXT.U | tests.c line 1049 | b_text | yes | v_gtext | - |
| ALN.VSC.A | tests.c line 1050 | b_vsc | yes | setup paints stripes (s_scroll), body scrolls them | - |
| ALN.VSC.U | tests.c line 1051 | b_vsc | yes | setup paints stripes (s_scroll), body scrolls them | - |
| ALN.HSC.A | tests.c line 1052 | b_hsc | yes | setup paints stripes (s_scroll), body scrolls them | - |
| ALN.HSC.U | tests.c line 1053 | b_hsc | yes | setup paints stripes (s_scroll), body scrolls them | - |
| ALN.WMOVE.A | tests.c line 1054 | b_wmove | indirectly | helper window aux over the test window, as AES.MOVE / AES.OPEN | - |
| ALN.WMOVE.U | tests.c line 1055 | b_wmove | indirectly | helper window aux over the test window, as AES.MOVE / AES.OPEN | - |
| ALN.WOPEN.A | tests.c line 1056 | b_wopen | indirectly | helper window aux over the test window, as AES.MOVE / AES.OPEN | - |
| ALN.WOPEN.U | tests.c line 1057 | b_wopen | indirectly | helper window aux over the test window, as AES.MOVE / AES.OPEN | - |
| ALN.WSCR.A | tests.c line 1058 | b_ascroll | helper window | scroll_win(aux_h): blit and text go into the helper window aux, not the test window | - |
| ALN.WSCR.U | tests.c line 1059 | b_ascroll | helper window | scroll_win(aux_h): blit and text go into the helper window aux, not the test window | - |
| FULL.OPEN | tests.c line 1062 | b_wopen | indirectly | full-screen helper window aux covers the test window; the test window only redraws via WM_REDRAW | - |
| FULL.TOGGLE | tests.c line 1063 | b_full_toggle | indirectly | full-screen helper window aux covers the test window; the test window only redraws via WM_REDRAW | - |
| FULL.TOP | tests.c line 1064 | b_wtop | indirectly | full-screen helper window aux covers the test window; the test window only redraws via WM_REDRAW | - |
| FULL.FILL | tests.c line 1065 | b_full_fill | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.VSCR | tests.c line 1066 | b_full_vscr | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.HSCR16 | tests.c line 1067 | b_full_hscr16 | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.HSCR8 | tests.c line 1068 | b_full_hscr8 | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.HSCR3 | tests.c line 1069 | b_full_hscr3 | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR08 | tests.c line 1070 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR10 | tests.c line 1071 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR12 | tests.c line 1072 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR14 | tests.c line 1073 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR18 | tests.c line 1074 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR24 | tests.c line 1075 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR36 | tests.c line 1076 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.SCR48 | tests.c line 1077 | b_full_size | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| FULL.GDSCR | tests.c line 1078 | b_full_mix | helper window | draws into the full-screen helper window aux (aux_h), not the test window | - |
| GDOS.T06 | tests.c line 1081 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T08 | tests.c line 1082 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T10 | tests.c line 1083 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T12 | tests.c line 1084 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T14 | tests.c line 1085 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T18 | tests.c line 1086 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T24 | tests.c line 1087 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T36 | tests.c line 1088 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T48 | tests.c line 1089 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.T72 | tests.c line 1090 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | - |
| GDOS.BOLD10 | tests.c line 1091 | b_gtext | yes | v_gtext at random points (system font fallback when GDOS is absent) | off by default |
| GDOS.POINT | tests.c line 1092 | b_gpoint | no | vst_point / vst_font / vqt_extent / vqt_name / load calls only | - |
| GDOS.SWITCH | tests.c line 1093 | b_gswitch | no | vst_point / vst_font / vqt_extent / vqt_name / load calls only | runs only if nfonts >= 2 |
| GDOS.ARBPT | tests.c line 1094 | b_garb | yes | vst_arbpt then v_gtext (needs a_vector) | runs only if GDOS is FSM/Speedo or the font is outline |
| GDOS.SCRMIX | tests.c line 1095 | b_mixscroll | yes | scroll_rects(test window) with a different vst_point per line | - |
| GDOS.EXTENT | tests.c line 1096 | b_extent | no | vst_point / vst_font / vqt_extent / vqt_name / load calls only | - |
| GDOS.NAMES | tests.c line 1097 | b_gname | no | vst_point / vst_font / vqt_extent / vqt_name / load calls only | runs only if vq_gdos() != 0 |
| GDOS.LOAD | tests.c line 1098 | b_gload | no | vst_point / vst_font / vqt_extent / vqt_name / load calls only | runs only if vq_gdos() != 0; off by default |

Zählung: VDI 39, AES 21, ALN 20, FULL 17, GDOS 18 = 115.

## 7. Nicht geprüft

- **Nichts ausgeführt**: keine Hosttests, kein QEMU, kein Hatari, kein tosdiff; alle Zahlen aus vorhandenen Läufen (`measurements/`) und Quittungen (`TASK/`), Stand der Dateien, nicht neu gemessen. Ob die zitierten Läufe zum heutigen HEAD (`8ec5e8d6`) passen, habe ich nicht geprüft; die jüngsten sind vom 2026-11-06.
- **Echte Pi**: nichts. Weder wie `kernel/src/vdi_backend.rs` seine Schreibzugriffe zum Bildschirm bringt (Cache-Pflege `:942@flush_rect`, Framebuffer-Attribute), noch ob dort etwas fehlt. Das Backend habe ich nur an den Funktionsköpfen gelesen.
- **Fbench-Seite**: den Redraw-Handler (`win_redraw`, `w->draw`, `handle_message`) und die Fensterklasse `WN_TEST` habe ich nur bis zum Aufruf gelesen. Ob und wann das Testfenster nach `wind_open` ein `WM_REDRAW` bekommt und es beantwortet, ist hier nicht Gegenstand.
- **AES-Zeichnung** (`objc_draw` je Objekttyp, Menüleiste, Fensterrahmen): nur die Funktionsköpfe und die Namen der Tests gelesen; kein Pixelvergleich mit dem Orakel außer den Tests `P` und den zitierten Quittungen.
- **tosdump-Tabellen r2b, r3, r4** gegen NewTOS: nicht neu gerechnet; r4 hat keine Orakeldatei. Die Tabelle `aes-r3-wind-state.tsv` habe ich nur über den zitierten tosdiff-Text gesehen.
- **Zeichnende VDI-Pfade im Detail** (`wideline`, `curve`, `rbox`, `build_cell`): als Aufruf bis zum Backend verfolgt, nicht Zeile für Zeile gegen EmuTOS gelesen; die Gleichheit stützt sich auf die genannten Messungen.
- **Nicht in der Liste, aber von Fbench gerufen** (Beobachtung, nicht bearbeitet): `appl_init`, `appl_exit`, `appl_find`, `appl_getinfo` (Opcode 130, in EmuTOS nicht vorhanden: `references/emutos/aes/gemsuper.c` kennt ihn nicht, bei uns `-1`), `graf_handle`, `wind_find`, `rsrc_obfix`, `menu_ienable`, `menu_icheck`, `menu_tnormal`, `menu_register`, `form_do`, `form_center`, `fsel_input`, `fsel_exinput`, `shel_write`, `evnt_timer`, `evnt_button`, `vq_color`, `Getrez`, `Supexec`.
- `references/tos4x/` nicht gelesen.

## 8. Out-of-scope observations

- `appl_init` meldet `ap_version` 0x0340 und `ap_count` 1 wie EmuTOS; Fbench entscheidet daraus "Single-TOS mit AES < 4.0" und ruft `_appl_yield()`. Das ist der Grund, warum die MISSING-Zeile oben jeden `drain_events` trifft.
- Die r2-wind-Dumps zeigen `global[3]`/`global[4]` mit vertauschter Wortreihenfolge gegen das Orakel (4419 jeweils in der anderen Hälfte der `LONG`-Zelle): wahrscheinlich die little-endian-Lage einer `LONG` in einem WORD-Feld, nicht geprüft.
- Keine Korrektur an `docs/api/` vorgenommen. Zwei Spec-Stellen betreffen die Ergebnisse: `docs/api/aes/_dispatcher.md` Zeilen 273-275 führt Selektor 201 noch als "offene Frage"; `docs/api/vdi/_discrepancies.md` D-VDI-9 verlangt das Beibehalten von `clip = 1`, der Code tut es nicht.
