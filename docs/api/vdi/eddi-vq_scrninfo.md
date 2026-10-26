# EdDI-Erweiterung: Cookie `EdDI` und `vq_scrninfo` (VDI 102, Sub-Opcode 1)

> **Status: FREIGEGEBEN (Anwender 2026-10-02).** E1–E9 wie in §7 empfohlen;
> **E3 = `0x02`** (Falcon-Antwort): die Bitnummern beschreiben das Pixelwort so,
> wie die CPU es liest — auf NewTOS lesen Programm und `vro_cpyfm` dasselbe
> native Wort, es gibt keinen Unterschied CPU↔Geraet. Beleg der Wahl: Dooms
> EdDI-Weg liefert auf NewTOS ein farbrichtiges Bild. Die UNKNOWN-Punkte zu
> Nova (Version, 15/16/24/32 Bit, Byte-Reihenfolge) klaert ein Messprogramm
> `SCRNINFO.TTP` auf dem echten TT mit Nova.
> **Umgesetzt 2026-10-02** (TASK/PORT06-EDDI): `tos-api/src/vdi/eddi.rs`,
> Cookie `tos-api/src/cookie.rs` (`EDDI`, Wert `EDDI_MARKER = 0x00000001`:
> ungleich 0 und ungerade, also nie aufrufbar — E2 (b); kein Dispatcher, keine
> Versionsnummer), Tests `tos-api/tests/vdi_eddi_scrninfo.rs`, Abweichung in
> `docs/COMPATIBILITY.md`, Messung `measurements/measurement-2026-10-02-eddi/`.
> Stand 2026-10-02. Verfasser: Researcher. Scope: nur das, was Doom
> (TASK/PORT06, "Pixelformat, Weg 2") und eine NewTOS-Implementierung davon
> brauchen — Cookie, Dispatcher-Opcode 0, `vq_scrninfo`, und die Einordnung der
> uebrigen EdDI-Funktionen.

## 0. Wichtig vorab: es gibt keine EmuTOS-Quelle

**EmuTOS kennt kein EdDI.** `grep -rIn "EdDI\|scrninfo\|0x45644449"` ueber
`references/emutos/` liefert **keinen Treffer** (geprueft 2026-10-02, Stand
`1a7484ab`). Diese Spec ist daher **kein EmuTOS-Extrakt**, sondern stuetzt sich
auf die beste verfuegbare Primaerdokumentation (Behne&Behne/NVDI-Handbuch, in
`tos.hyp` und im Enhancer-Archiv) und die Implementierung `fVDI`. Sie ist eine
**Erweiterung ueber EmuTOS hinaus** (PORT06 "Entscheidungen 2026-10-02").
Lesehilfe: **[D]** = in einer Dokumentation belegt, **[C]** = in Code gelesen
(EmuTOS, gemlib, fVDI, Verbraucher), **[I]** = von mir gefolgert (nicht
tragend ohne Pruefung), **UNKNOWN** = nirgends gefunden.

**Was EmuTOS (und damit NewTOS heute) auf `vq_scrninfo` antwortet:** Der
Dispatcher wertet `CONTRL[5]` nicht aus; Opcode 102 laeuft immer in
`vdi_vq_extnd`; `INTIN[0] = 2` ist `!= 0`, also Modus 1: 45 Woerter `INQ_TAB`
nach `INTOUT[0..44]`, Clip-Rechteck nach `PTSOUT[0..3]`
([C] `references/emutos/vdi/vdi_main.c:77` Tabelleneintrag 102,
`vdi/vdi_control.c:626-658`). `work_out[0..44]` ist dann `INQ_TAB`
(`INQ_TAB_rom` `vdi_control.c:76-123`; `INQ_TAB[4] = v_planes`, `[5]` Lookup-
Tabelle `:210-213`), `work_out[45..272]` bleibt unberuehrt. NewTOS tut dasselbe
([C] `tos-api/src/vdi/mod.rs:338`, `tos-api/src/vdi/workstation.rs:285-330`).
HighWire dokumentiert genau das als "garbage" des ROM-VDI
([C] `sdk/apps/highwire/src/romvdi.c:311-316`). **Konsequenz:** ein Programm
darf `vq_scrninfo` **nur** aufrufen, wenn der Cookie `EdDI` da ist.

## 1. Quellen (alle gelesen, 2026-10-02)

| Kuerzel | Quelle | Art |
|---|---|---|
| TH-VQ | https://raw.githubusercontent.com/freemint/tos.hyp/master/gem/vdi/inquire/vq_scrninfo.ui (Quelltext der tos.hyp-Seite `vq_scrninfo`, EN-Teil) | [D] Hauptquelle |
| TH-CK | https://raw.githubusercontent.com/freemint/tos.hyp/master/bios/cookie/description/EdDI.ui (Cookie `EdDI`, "by Sven&Wilfried Behne, BELA") | [D] |
| TH-OB | https://raw.githubusercontent.com/freemint/tos.hyp/master/gem/vdi/control/v_opnbm.ui, .../v_clsbm.ui, .../v_open_bm.ui, .../v_resize_bm.ui | [D] |
| TH-TAB | https://freemint.github.io/tos.hyp/en/vdi_functions.html (Opcode-Tabelle, "As of EdDI 1.00 / 1.20") | [D] |
| TH-NV | https://raw.githubusercontent.com/freemint/tos.hyp/master/gem/vdi/inquire/vq_ext_devinfo.ui, .../vqt_xfntinfo.ui ("As of NVDI 3.00 / 3.02") | [D] |
| NV250 | `Applications/z-tools/zview/_dist/goodies/Enhancer/Docs/German/Neue_Fkt.txt` (NVDI-2.50-Anleitung "Neue Funktionen", Latin-1): Abschnitte 1 (Bitmaps), 2 (`vq_scrninfo`), 3 (Erkennung) | [D] Primaerquelle in-tree, Stand EdDI 1.00 |
| ENH | `Applications/z-tools/zview/_dist/goodies/Enhancer/Docs/English/Enhancer.txt` | [D] |
| GEMLIB | `sdk/gemlib/gemlib-0.44.0-m68k/n_vq_scrninfo.c` (identisch zu `sdk/gemlib/gemlib-0.44.0/n_vq_scrninfo.c`, diff geprueft), `gem_vdiP.h:42,86,110,124-133`, `sdk/gemlib/gemlib-0.44.0-m68k/vdi_dummy.c:4`, `sdk/gemlib/gemlib-0.44.0-m68k/mt_gemx.h:1293-1309` | [C] Binding |
| fVDI | https://github.com/freemint/fvdi (master): fvdi/engine/fvdi.s:484-494 (`_eddi_dispatch`), engine/startup.c:370-373, engine/workstn.c:575-585 (`lib_vq_extnd`), include/fvdi.h (`typedef struct dev_`), drivers/common/init.c:102-175 (`setup_scrninfo`), drivers/aranym/spec.c:10-86 (Modus-Tabelle) | [C] Dritt-Implementierung (nicht NVDI/Nova) |
| Verbraucher | `Applications/z-tools/zview/zvdi/vdi.c:28-58,62-157`; `sdk/apps/highwire/src/romvdi.c:317-355`, `sdk/apps/highwire/src/raster.c:1711-1780`; `sdk/apps/doom/doomgeneric_gem.c:43` | [C] wie reale Programme `work_out` lesen |
| FreeMiNT | `references/freemint/doc/freemint.stg:6775` (Beispiel-Jar `0x45644449 (EdDI): 0x0005EEE6`); `sdk/libc/mintlib/include/mint/cookie.h:125` (`C_EdDI 0x45644449L`) | [D]/[C] |

**Nicht gefunden:** das NVDI-Handbuch jenseits 2.50, die Nova-Treiber-
Dokumentation, jede Quelle, die zeigt, **was ein NVDI-/Nova-Treiber real fuer
15/16/24/32 Bit eintraegt** (siehe §6). Der EN-Text von tos.hyp
(`vq_scrninfo.ui`) stimmt mit dem gemlib-Doxygen-Kommentar und mit NV250 im
Layout ueberein (Abgleich unten in §3.4).

## 2. Der Cookie `EdDI`

* **Tag:** `'EdDI'` = `0x45644449` ([C] `sdk/libc/mintlib/include/mint/cookie.h:125`; [D] TH-CK, Beispiel
  `freemint.stg:6775`).
* **Wert:** "die Adresse eines Funktionsdispatchers" ([D] TH-CK; NV250 §3).
  Aufruf **nach Pure-C-Konvention**: Opcode in Register `d0`, `d1-d2/a0-a1`
  und Stack fuer weitere Parameter ([D] TH-CK; NV250 §3). Das ist eine
  **m68k-Registerkonvention** — fuer AArch64 gibt es keine definierte
  Abbildung (Entscheidung E2).
* **Erkennung:** Cookie-Jar nach `'EdDI'` durchsuchen; fehlt er, ist `vq_scrninfo`
  nicht verfuegbar ([D] NV250 §3: "Wie erkennt man, ob die neuen Funktionen
  vorhanden sind?"; [C] Verbraucher: `Applications/z-tools/zview/zvdi/vdi.c:28` ruft
  `vq_scrninfo` nur bei gefundenem `C_EdDI`; HighWire `sdk/apps/highwire/src/romvdi.c:70,329`).
* **Dispatcher-Opcodes:** bisher **nur Opcode 0**; Rueckgabe = Versionsnummer
  ([D] TH-CK "Till now only opcode 0 is implemented"; NV250 §3). Es gibt
  **kein** Opcode-Fehlerverhalten in der Doku. fVDI: Opcode `!= 0` liefert
  `-1`, Opcode 0 liefert `$0100` ([C] fvdi/engine/fvdi.s:484-494).
* **Versionssemantik** ([D] TH-CK):
  * `$100` (1.00): `v_opnbm`, `v_clsbm`, `vq_scrninfo`.
  * 1.10: `v_opnbm` und `vq_scrninfo` haben **zusaetzliche Parameter**
    (bei `vq_scrninfo`: `work_out[5..7]` und `work_out[14]`,
    [D] TH-VQ "as of EdDI 1.1"; "Check the cookie version before you use
    them").
  * 1.20: `v_open_bm`, `v_resize_bm`.
  * **Rueckgabewert fuer 1.10/1.20 nicht dokumentiert** — nur `$100` fuer
    1.00 ist belegt ("return value is $100"). `$110`/`$120` ist **[I]**.
* Wer installiert ihn: NVDI und der Atari-VDI-**Enhancer** ([D] TH-CK, ENH;
  der Enhancer "simuliert" den Aufruf ohne NVDI), fVDI ([C]
  fvdi/engine/startup.c:370-373 setzt `EdDI` auf `eddi_dispatch`).
* **fVDI-Beobachtung [C]:** fVDI meldet Version `$0100` (fvdi.s:489),
  befuellt aber `work_out[5..7]`/`[14]` (`setup_scrninfo`,
  drivers/common/init.c:121, 16_bit/16b_spec.c:368-374). Ein Verbraucher,
  der der Doku folgt ("Version pruefen"), liest dort `[14]` nicht.
  → **Bit-Ordnung ist bei `$100` nicht zuverlaessig lieferbar** (Konsequenz
  fuer E3).
* **Hardware-/Wertbreite in NewTOS:** der NewTOS-Cookie-Jar speichert Werte als
  `u32` ([C] `tos-api/src/cookie.rs:34-37`) und gibt Programmen keinen
  Zeiger auf den Jar, nur `Ssystem`-Zugriff ([C] `cookie.rs:9-11`). Ein
  Dispatcher-**Zeiger** passt in 32 Bit nur, wenn er in der unteren 4-GiB-
  Region liegt (E2).

## 3. `vq_scrninfo`

### 3.1 Aufruf (VDI-Arrays)

[D] TH-VQ "Bindings for vq_scrninfo" und NV250 §2; [C] GEMLIB
`n_vq_scrninfo.c` (letzte Zeilen) — beide Quellen stimmen ueberein.

| Feld | Wert | Anmerkung |
|---|---|---|
| `CONTRL[0]` | 102 | Opcode |
| `CONTRL[1]` | 0 | `ptsin` |
| `CONTRL[2]` | 0 | `ptsout` (vom Treiber nicht zu beschreiben; **Achtung**: EmuTOS' `vq_extnd` schriebe 12 Woerter in `ptsout`, deshalb gibt gemlib `vdi_dummy[1024]` als `ptsout` mit, [C] `n_vq_scrninfo.c`, `sdk/gemlib/gemlib-0.44.0-m68k/vdi_dummy.c:4`) |
| `CONTRL[3]` | 1 | `intin` |
| `CONTRL[4]` | 272 (Doku) | Ausgabezaehler; **gemlib setzt ihn nicht** (nur `[0],[1],[3],[5],[6]`, `VDI_TRAP_ESC(...,102,1,0,1)`, `gem_vdiP.h:110-116`) — der Treiber traegt ihn ein |
| `CONTRL[5]` | **1** | Sub-Opcode |
| `CONTRL[6]` | handle | |
| `INTIN[0]` | **2** | "erweiterte Informationen ausgeben" [D] NV250 |
| `INTOUT[0..272]` | `work_out[0..272]` | **273 Woerter**, obwohl `CONTRL[4]=272` dokumentiert ist; TH-VQ: der Enhancer aus NVDI 2.5x kopiere "sometimes 273 values" → Puffer fuer 273 Woerter vorsehen [D] |

Verhalten bei `INTIN[0] != 2`: **UNKNOWN** (Doku schweigt). fVDI unterscheidet
`flag == 2 && subfunction == 1` von `flag != 0` (= `vq_extnd` Modus 1)
([C] fvdi/engine/workstn.c:577-583); dass `flag` = `INTIN[0]` ist, ist **[I]**.
Verhalten bei einem Bitmap-Handle (`v_opnbm`): **UNKNOWN**.

### 3.2 `work_out`-Felder

[D] TH-VQ, bestaetigt durch NV250 (Layout bis 13 + Tabellen) und GEMLIB;
Typ-Spiegel [C] fVDI `typedef struct dev_` (include/fvdi.h:
`format, clut, bit_depth, dummy1, colours, byte_width, address, bits{red,
green, blue, alpha, genlock, unused, organization}, dummy2, scrmap`).

| Index | Inhalt |
|---|---|
| 0 | **Format:** `0` interleaved Planes, wortweise (Atari); `1` Standardformat (ganze Planes); `2` Packed Pixels; `-1` unbekannt, nicht direkt beschreibbar |
| 1 | **CLUT:** `0` keine CLUT (z. B. TTM 194); `1` Hardware-CLUT; `2` Software-CLUT (HiColor/TrueColor, "pseudo CLUT") |
| 2 | Planes (Bits) pro Pixel |
| 3/4 | Farbanzahl als **LONG** oder `0L` ("mehr als 2*10^31 Farben" — so steht es im Doku-Text, offenkundig `2^31` gemeint, **[I]**). Wortreihenfolge nicht ausdruecklich dokumentiert; fVDI setzt `[3]` = `(1L<<bits)>>16` (Hi), `[4]` = `& 0xffff` (Lo) [C] init.c:140-141,158-159; zview/HighWire erkennen 15-Bit-Modi an `(uint16)out[4] == 0x8000` [C] `Applications/z-tools/zview/zvdi/vdi.c:39`, `sdk/apps/highwire/src/raster.c:1713` → Hi-Wort zuerst **[I aus C]** |
| 5 | Zeilenbreite in Bytes (**ab EdDI 1.1**) |
| 6/7 | Adresse der Bitmap, LONG (**ab EdDI 1.1**) |
| 8 / 9 / 10 | Bits fuer Rot / Gruen / Blau (bei CLUT: Bits der CLUT-Einträge, z. B. 6/6/6 beim Falcon-256) |
| 11 | Bits Alpha-Kanal |
| 12 | Bits Genlock |
| 13 | Anzahl ungenutzter Bits |
| 14 | **Bit-Ordnung** (**ab EdDI 1.1**), siehe 3.3 |
| 15 | nicht dokumentiert (fVDI: `dummy2 = 0`) — UNKNOWN, wie 0 behandeln |
| 16..271 | bei `work_out[1] == 1`: Pixelwert zum VDI-Farbindex 0..255 |
| 16..31 / 32..47 / 48..63 | bei HiColor/TrueColor: fuer Rot / Gruen / Blau je Intensitaetsbit 0 (LSB) .. n-1 die **Bitnummer im Pixel**; nicht benutzte Eintraege `-1` |
| 64..79 / 80..95 | Bitnummern Alpha / Genlock |
| 96..127 | Bitnummern ungenutzter Bits |
| 128..271 | reserviert, 0 |

Die Bitnummern der Tabelle zaehlen vom LSB des Pixelworts; die Beispiele
legen fest, **wie** bei vertauschter Byte-Ordnung gezaehlt wird (Crazy-Dots-
Beispiel, [D] TH-VQ "Examples": `gggbbbbb-xrrrrrgg` ergibt Rot-Bits 2..6,
Gruen 13,14,15,0,1, Blau 8..12, ungenutzt 7): **die Bits werden so
numeriert, als sei das Pixel als Big-Endian-Wort aus den Speicherbytes
gelesen** — das Byte mit der niedrigeren Adresse steht oben. **[I]** aus den
Beispielen, nicht ausdruecklich formuliert.

### 3.3 `work_out[14]` Bit-Ordnung [D] TH-VQ

* 2..256 Farben: Bit 0 = uebliche Ordnung.
* 32768 Farben (16 Planes): Bit 0 = uebliche (`xrrrrrgg-gggbbbbb`, Overlay/
  ungenutzt oben); Bit 1 = **Falcon**-Ordnung (`rrrrrggg-ggxbbbbb`, 1 Bit
  Overlay in der Mitte); Bit 7 = vertauschte Bytes (Intel), z. B. Crazy Dots
  (`gggbbbbb-xrrrrrgg`).
* 65536 Farben (16 Planes): Bit 0 = `rrrrrggg-gggbbbbb`; Bit 7 = vertauschte
  Bytes (`gggbbbbb-rrrrrggg`).
* 16,7 Mio. (24 Planes): Bit 0 = `rrrrrrrr-gggggggg-bbbbbbbb`; Bit 7 =
  vertauscht (`bbbbbbbb-gggggggg-rrrrrrrr`).
* 16,7 Mio. (32 Planes): Bit 0 = `xxxxxxxx-rrrrrrrr-gggggggg-bbbbbbbb`
  (8 Bit Overlay/ungenutzt zuerst); Bit 7 = vertauscht
  (`bbbbbbbb-gggggggg-rrrrrrrr-xxxxxxxx`).
* Die Doku-Beispiele tragen `1` (Falcon-256, "usual"), `2` (Falcon HiColor),
  `129` = `0x81` (Crazy Dots) ein; Bit 0/1 sind also **Auswahl**-Werte, Bit 7
  ein **Zusatz-Flag** (`0x80`). Zwei Flags gleichzeitig (`0x82`) sind in der
  Doku nicht ausdruecklich vorgesehen — fVDI/ARAnyM verwendet `0x81` fuer
  "fb layout" [C] aranym/spec.c:60-63,83-86, d. h. Kombination
  "Basis + 0x80" ist ueblich **[I]**.

### 3.4 Abgleich Doku ↔ gemlib ↔ Verbraucher; Doku-Fehler

* Feldlayout, Format-/CLUT-Werte und Bit-Ordnung: tos.hyp-Text, gemlib-Kommentar
  und NV250 **stimmen ueberein** (NV250 kennt `[5..7]`, `[14]` noch nicht,
  Stand 1.00 — erwartungsgemaess).
* gemlib dokumentiert zusaetzlich die konkrete `INTIN[0]=2`-Belegung im Code;
  die Doku-Aussage `CONTRL[4]=272` vs. `INTOUT[0..272]` (273 Woerter) ist
  **in sich uneinheitlich** (siehe 3.1).
* **Fehler im Crazy-Dots-Beispiel (TH-VQ, ebenso gemlib):** `work_out[12]`
  steht dort mit Wert `1` und der Beschreibung "No genlock/overlay",
  `work_out[13]` mit `0` und "1 unused bits", dabei markiert `work_out[96] = 7`
  ein ungenutztes Bit und `80..95 = -1` "kein Genlock". Gemeint ist offenbar
  `[12] = 0`, `[13] = 1` **[I]** — die Felder sind vertauscht. Nicht auf die
  Beispielwerte, sondern auf die Tabellen 80..127 verlassen.
* HighWire (`sdk/apps/highwire/src/raster.c:1715-1716`) und zview (`Applications/z-tools/zview/zvdi/vdi.c:40-41`) werten aus:
  `reverse = (out[16] < out[48])` (Rot-LSB-Bitnummer kleiner als Blau-LSB →
  "Intel-Ordnung"), `z_trail = (out[48] > 0)`. **Beide Programme rechnen
  also mit den Bitnummern-Tabellen, nicht mit `[14]`** [C]. Eine korrekte
  `16..63`-Tabelle ist damit wichtiger als `[14]` (wichtig fuer E3).

### 3.5 Verhalten Palette- vs. Truecolor-Geraet

[D] TH-VQ; [C] fVDI `setup_scrninfo` (drivers/common/init.c:102-175).

* **Palettengeraet** (`work_out[1] == 1`, Format 0 oder 2, 1..8 Bit):
  `[16..271]` = Pixelwert pro VDI-Farbindex; die Bitnummern-Tabellen sind
  **nicht** definiert (fVDI: Union, d. h. nur eine der beiden Deutungen
  gilt, `fvdi.h` `scrmap`). Beispiel Falcon-256 in TH-VQ: Index 0→0,
  1→255, 2→2 ... 255→15 (ST-Pen-Reihenfolge).
* **HiColor/TrueColor** (`work_out[1] == 2`): `[16..127]` sind die Bitnummern-
  Tabellen; VDI-Farbindex-Zuordnung entfaellt ("pseudo CLUT"); das Programm
  rechnet Farbe → Pixel selbst.
* `work_out[1] == 0` ("keine CLUT"): nur Beispiel TTM 194 genannt; Inhalt von
  `16..` **UNKNOWN**.

### 3.6 Fehler / nicht unterstuetzt

Die Doku nennt **nur** `work_out[0] == -1` ("unbekanntes Format; nicht direkt
beschreibbar") als Fehlerfall. Ein Rueckgabewert/Fehlercode des Aufrufs existiert
nicht (`void`). Was bei unbekanntem Handle geschieht: **UNKNOWN**. Verbraucher
fangen unplausible Antworten selbst ab: HighWire verwirft eine Antwort mit
`out[0]` ausserhalb 0..2 oder `out[2]` ausserhalb 1..32 und raet selbst
[C] `sdk/apps/highwire/src/romvdi.c:329-335`.

## 4. Weitere EdDI-Funktionen und Doom-Bedarf

| Funktion | Opcode | ab | Doom? |
|---|---|---|---|
| `vq_scrninfo` | 102, sub 1 | EdDI 1.00 ([D] TH-TAB) | **JA — diese Spec** |
| `v_opnbm` | 100, sub 1 | EdDI 1.00 | nein (Doom zeichnet nicht off-screen; es blittet aus eigenem MFDB) |
| `v_clsbm` | 101, sub 1 | EdDI 1.00 | nein |
| `v_resize_bm` | 100, sub 2 | EdDI 1.20 | nein |
| `v_open_bm` | 100, sub 3 | EdDI 1.20 | nein |
| Dispatcher-Opcode 0 (Version) | – | 1.00 | **bedingt** (nur, wenn Doom `[5..7]`/`[14]` versionsabhaengig lesen will; ohne ausfuehrbaren Dispatcher nicht noetig, E2) |

* `vq_ext_devinfo` (248/4242) und `vqt_xfntinfo` (229) sind **NVDI**-Funktionen
  (3.00 / 3.02), **nicht EdDI** ([D] TH-NV). **Out of scope.**
* Farbmanagement-Aufrufe: in den gelesenen Quellen **keinem EdDI-Versionsstand
  zugeordnet** → **UNKNOWN / out of scope**.
* Die Bitmap-Funktionen werden **nicht spezifiziert**.

## 5. Wie Doom die Antwort benutzt (Kontext, kein Vertrag)

Der Weg, den PORT06 beschreibt ([C] `sdk/apps/doom/doomgeneric_gem.c:43`):
bei vorhandenem Cookie `vq_scrninfo`, dann pro Palette-Eintrag
(8-Bit-RGB) das Geraete-Pixel aus den Bitnummern-Tabellen aufbauen
(Truecolor) bzw. aus `[16+i]` entnehmen (Palette), im Geraeteformat eine MFDB
fuellen (`fd_stand = 0`, `fd_nplanes = work_out[2]`), `vro_cpyfm`.
Die Anforderung an NewTOS folgt daraus: **was `vq_scrninfo` sagt, muss exakt
das sein, was `vro_cpyfm` aus einer Geraete-MFDB liest.** Das ist pruefbar
(Selbsttest: gebautes Pixel → `vro_cpyfm` → `v_get_pixel`).

## 6. Wie melden NVDI und Nova 15/16/24/32 Bit? Und die NewTOS-Antwort

### 6.1 Was belegt ist

* **NVDI-Dokumentation** [D] TH-VQ: beschreibt die **Bit-Ordnungs-Werte** fuer
  32768/65536/16,7 Mio. Farben (3.3) und drei durchgerechnete Beispiele
  (Falcon-256, Falcon-HiColor, Crazy Dots HiColor). **24/32 Bit sind nur als
  Ordnungsschema beschrieben, ohne durchgerechnetes Beispiel.**
* **Real gelieferte Tabellen von NVDI oder Nova-Treibern: UNKNOWN.** Weder
  NVDI-Quelle noch Nova-Treiber-Quelle liegt vor; ich habe keine geschaetzt.
* Ersatzweise belegt ist, was **fVDI** (freie Implementierung, nutzt dieselbe
  Struktur und ist fuer Nova-/ARAnyM-/PCI-Karten gedacht) einträgt
  [C] drivers/aranym/spec.c:11-86, drivers/common/init.c:102-175:
  * 16 Bit, "Falcon-/usual"-Modus: Bits R 11..15, G 5..10 (6 Bit), B 0..4
    (`r_16f/g_16f/b_16f`), Format 2, CLUT 2, Org `1`.
  * 16 Bit, "fb layout": R 3..7, G 13,14,15,0,1,2 (6 Bit), B 8..12
    (`r_16/g_16/b_16`), Format 2, CLUT 2, Org `0x81`.
  * 32 Bit: R 16..23, G 8..15, B 0..7 (`r_32/g_32/b_32`), Format 2, CLUT 2,
    Org `1` bzw. `0x81`. 24 Bit nutzt in der "usual"-Zeile die `_32f`-Tabellen
    (R 8..15, G 16..23, B 24..31) — das widerspricht der Doku-Beschreibung
    "rrrrrrrr-gggggggg-bbbbbbbb" und ist ein fVDI-eigener Befund; **nicht als
    NVDI-Verhalten uebernehmen**.
  * Dies ist **Evidenz fuer die Struktur**, **nicht** fuer NVDI/Nova.

### 6.2 NewTOS-Geraeteformat (Eingaben)

* Geraetepixel: 16-Bit-Wort `RRRRRGGGGG0BBBBB`, Bit 5 = `OVERLAY_BIT`, vom VDI
  nie gesetzt ("as in TOS4, the VDI never sets the least-significant bit of
  green") [C] `references/emutos/vdi/vdi_col.c:599-613`, `vdi_fill.c:26`;
  NewTOS identisch (ADR-024, `docs/MEMORY.md:1885-1965`; V-Serie
  `docs/analysis/vdi-v-series-classification-2026-09-21.md:76-110`).
* Memory-MFDB (`fd_stand = 0`, `fd_nplanes = 16`) ist **gepacktes 16 Bit pro
  Pixel**, ohne Palette ([C] `docs/MEMORY.md` ADR-024 Punkt 4;
  EmuTOS `vdi_raster.c:975-1221`, `:190-256`). NewTOS liest/schreibt das
  Wort **nativ** per `read_unaligned`/`write_unaligned` auf `u16`
  ([C] `tos-api/src/vdi/raster.rs:164-193`) — auf AArch64 (little-endian
  **[I]**, nicht im Code nachgeschlagen) liegt das niederwertige Byte zuerst im
  Speicher.
* Der physische Pi-Framebuffer ist 32 bpp XRGB8888 — NewTOS-intern, **kein**
  Vertrag; ein 16-Bit-Linear-Screen existiert nicht (ADR-024 Punkt 3).
* `vq_extnd` meldet bereits `INQ_TAB[4] = 16`, `INQ_TAB[5] = 0` (keine
  Lookup-Tabelle) und `DEV_TAB[13] = 256` Stifte ([C]
  `workstation.rs:285-330`, EmuTOS `vdi_control.c:206-213`).

### 6.3 NewTOS-Antwort — Ableitung (alles **[I]** ausser den zitierten Beispielen)

Der Falcon-HiColor-Beispielsatz in TH-VQ ist **bit-fuer-bit** das
NewTOS-Wort (`rrrrrggg-ggxbbbbb`, 5/5/5 + 1 Genlock-Bit in der Mitte). Ableitung:

| work_out | Wert | Begruendung |
|---|---|---|
| 0 | 2 | Packed Pixels (TH-VQ Falcon-HiColor) |
| 1 | 2 | Software-CLUT/TrueColor (TH-VQ; passt zu `INQ_TAB[5] = 0`) |
| 2 | 16 | `v_planes` |
| 3 / 4 | 0 / 0x8000 (= 32768) | wie TH-VQ-Beispiel; **E4**: 65536 waere falsch, weil Bit 5 nie genutzt wird |
| 5 | **E5** | Zeilenbreite des (nicht existierenden) 16-Bit-Screens |
| 6/7 | **E5** | Adresse; 64-Bit-Zeiger passt nicht in 2 Woerter |
| 8 / 9 / 10 | 5 / 5 / 5 | Intensitaetsbits |
| 11 | 0 | kein Alpha |
| 12 | 1 | 1 Bit Genlock = `OVERLAY_BIT` (wie TH-VQ Falcon) |
| 13 | 0 | keine ungenutzten Bits |
| 14 | **E3** | Falcon-Ordnung `0x02`, ggf. `0x82` |
| 15 | 0 | (undokumentiert) |
| 16..31 | siehe unten | |
| 32..47 | | |
| 48..63 | | |
| 64..79 | -1 | kein Alpha |
| 80..95 | Genlock-Bit | |
| 96..127 | -1 | |
| 128..271 | 0 | reserviert |

**Tabellen — Fall A, "Wort wie VDI-Wert, Bytes im Speicher little-endian"**
(NewTOS liest nativ-LE, 6.2). Nach Konvention 3.2 (Bits als Big-Endian aus
Speicherbytes gelesen) ergibt sich fuer die Speicherbytes
`[ggxbbbbb][rrrrrggg]`:
Rot `[16..20] = 3,4,5,6,7`; Gruen `[32..36] = 14,15,0,1,2`;
Blau `[48..52] = 8,9,10,11,12`; Genlock `[80] = 13`; Rest `-1`; **`[14] = 0x82`**.
Das ist dieselbe Struktur wie die Crazy-Dots-Zeile (Bit 7 gesetzt), nur mit
Falcon-Bitlage. `reverse = out[16] < out[48]` ist dann wahr (3 < 8), d. h.
zview/HighWire wuerden das Pixel als "Intel-Ordnung" lesen — konsistent.

**Fall B, "logisches Wort, Big-Endian im Speicher"** (Falcon-Beispiel 1:1):
Rot `11..15`, Gruen `6..10`, Blau `0..4`, Genlock `[80] = 5`, `[14] = 0x02`.
Nur richtig, wenn die MFDB-Woerter Big-Endian abgelegt werden — NewTOS liest
aber nativ (6.2) → **B waere fuer ein woertlich lesendes Programm falsch**,
sofern NewTOS die Reads nicht auf Big-Endian umstellt.

Beide Faelle sind **abgeleitet, nicht belegt**; eine Messung (Selbsttest
6.4) entscheidet, nicht die Lektuere.

### 6.4 Pruefbarkeit

Ein Host-Test `vq_scrninfo` → Pixel aus Bitnummern bauen → `vro_cpyfm` →
`v_get_pixel` muss fuer alle 256 Doom-Palettenfarben das Gebaute zurueckgeben.
Der Test gehoert zum spaeteren Ticket, nicht zu dieser Spec.

## 7. Entscheidungen noetig (vor jeder Implementierung)

1. **E1 — Scope/Ticket:** nur Cookie + `vq_scrninfo` + Dispatcher-Opcode 0
   (Version). `v_opnbm`/`v_clsbm`/`v_resize_bm`/`v_open_bm` bleiben draussen
   (Doom braucht sie nicht). Einverstanden?
2. **E2 — Dispatcher-Ausfuehrbarkeit:** Cookie-Wert ist 32 Bit
   (`cookie.rs:36`); der echte Dispatcher ist eine m68k-Registerkonvention.
   Optionen: (a) Wert = Adresse eines kleinen EL0-lesbaren Trampolins, das den
   Opcode in einen `svc` uebersetzt (Aufwand: eigene ABI definieren, im
   ILP32-/LP64-Pfad unterschiedlich); (b) Wert = nicht-null Marker/Versionszahl,
   **nicht** aufrufbar (dokumentierte Abweichung; Programme, die den Dispatcher
   aufrufen, stuerzen ab); (c) Dispatcher nur fuer m68k-Emulator-Phase. Empfehlung:
   (b) jetzt, (a) spaeter; Doom ruft den Dispatcher nicht (Erkennung =
   Cookie-Praesenz, Layout = `vq_scrninfo`).
3. **E3 — Versionsnummer und Bit-Ordnung:** NewTOS muss `[14]` liefern, damit
   die Byte-Reihenfolge ausdrueckbar ist, also Version ≥ 1.10 melden
   (`$110`, **[I]**, Wert nicht belegt). Ausserdem: Fall A (`0x82`, Tabellen
   nach 6.3) oder Fall B (`0x02`, Falcon-Tabelle + Big-Endian-Speicher)? Folge
   fuer B: die MFDB-Pfade (`raster.rs`) muessten Big-Endian lesen — Eingriff in
   die Truecolor-Wirklichkeit (ADR-024), nicht in dieser Spec geloest.
4. **E4 — Farbanzahl 32768 vs. 65536:** Das NewTOS-Wort ist 5/5/5 + nie
   gesetztes Overlay-Bit; die Falcon-HiColor-Doku meldet 32768. Empfehlung
   32768. (Verbraucher wie zview/HighWire erkennen 15 Bit genau daran,
   `(uint16)out[4] == 0x8000`.)
5. **E5 — `[5]` Zeilenbreite und `[6/7]` Adresse:** NewTOS hat keinen
   zugreifbaren 16-Bit-Linear-Screen (32-bpp-Framebuffer intern, ADR-024) und
   64-Bit-Adressen passen nicht in zwei Woerter. Vorschlag: `[5] = 0`,
   `[6/7] = 0` ("kein direkter Zugriff"); Doku verlangt dazu nichts, Doom
   braucht beides nicht (es geht ueber `vro_cpyfm`). Bestaetigung noetig.
   Ob Programme `0` als "nicht vorhanden" tolerieren: UNKNOWN (kein Verbraucher
   in-tree liest `[5..7]`; geprueft: zview, HighWire).
6. **E6 — Handle-Semantik:** Verhalten fuer virtuelle Workstation, v_opnbm-
   Handle und ungueltigen Handle ist **nirgends dokumentiert**. Vorschlag:
   jeder gueltige Bildschirm-Handle (physisch oder virtuell) antwortet
   gleich; ungueltiger Handle wie in `vq_extnd` (EmuTOS-Handle-Pruefung des
   Dispatchers, `_dispatcher.md`). Bestaetigung noetig.
7. **E7 — `INTIN[0] != 2`:** unspezifiziert. Vorschlag: wie bisher
   (`vq_extnd`-Modus 1) beibehalten, `INTIN[0] == 2` **und** `CONTRL[5] == 1`
   → `vq_scrninfo`. Das verlangt, dass der Dispatcher `CONTRL[5]` fuer Opcode 102
   auswertet (EmuTOS tut es nicht; **Abweichung** zu `vdi_main.c:77`).
8. **E8 — Wirkung auf bestehende Programme:** Aufrufer ohne EdDI-Pruefung
   (vgl. 0.) erhielten heute `INQ_TAB`; mit EdDI antwortet dieselbe Aufrufform
   anders. Das ist die beabsichtigte Erweiterung, aber eine bewusste
   **Abweichung von EmuTOS** — im Compat-Vertrag (`docs/COMPATIBILITY.md`) als
   Erweiterung festhalten.
9. **E9 — Genlock-Bit nach aussen:** wenn Doom das Overlay-Bit setzt (wuerde
   `vro_cpyfm` mit gesetztem Bit sehen) — NewTOS-VDI setzt es nie und
   maskiert es teils (`vdi_fill.c:702-755`). Empfehlung: in der Spec fordern,
   dass Programme das Genlock-Bit (`[80]`) **auf 0 lassen**; was NewTOS bei
   gesetztem Bit zeigt, ist Backend-Frage (`vdi_backend.rs` verliert Bit 5, siehe
   V11) und hier nicht geklaert.

## 8. Out-of-scope-Beobachtungen

* `docs/api/vdi/workstation/102-vq_extnd.md` kennt den Sub-Opcode-Fall
  `102/1` nicht (EmuTOS ignoriert `CONTRL[5]`); dort einen Hinweis auf diese
  Spec setzen, wenn sie freigegeben ist.
* `docs/analysis/vdi-audit/truecolor-conversion-plan.md` und ADR-024 gehen
  von 16-Bit-**nativem** `u16` im MFDB aus; die Endianness-Frage (E3) ist dort
  nicht ausdruecklich beantwortet.
* Nova-Treiber-Verhalten: ohne Quelle. **Lead 2026-10-02, geprueft am lokalen
  Hatari-Quellbaum** (`/Volumes/Daten/Development/RuSTari/hatari`, `4491dc0c1`):
  Hatari emuliert **keine** Nova/ET4000; seine Grafikerweiterung ist der
  VDI-Modus `--vdi`/`--vdi-planes` mit 1/2/4/8 Planes im Quellbaum
  (src/options.c:333-339, src/vdi.c:128-143 im Hatari-Quellbaum) — die **installierte**
  Hatari 2.6.1 (`/Volumes/Daten/hatari-2.6.1_macos`) bietet laut `--help` nur 1/2/4 — hoehere Aufloesung mit Palette, kein Truecolor, kein
  EdDI. Taugt fuer Dooms `vr_trnfm`-Weg bei 8 Planes; den EdDI-Weg auf einer
  Nova prueft nur der echte TT; **echte EdDI-Daten im Emulator** liefert der
  VDI-Enhancer 1.00 aus dem Baum (`measurements/measurement-2026-10-02-scrninfo/`):
  Falcon-Truecolor Format 2, CLUT 2, 16 Planes, 0x8000 Farben, [14]=0, Rot 11..15,
  Gruen 6..10, Blau 0..4, Genlock 5 — passend zu den kopierten Pixeln als
  Wort gelesen; stuetzt E3; ggf. per
  Hatari-Beobachtung (`vq_scrninfo` dumpen) zu klaeren — nicht von mir getan.

## 9. Re-Anchor

Scope: EdDI-Cookie und `vq_scrninfo` (plus Einordnung der uebrigen EdDI-
Funktionen) fuer Doom und eine NewTOS-Implementierung. Jeder Abschnitt traegt
[D]/[C]-Zitate oder ist als [I]/UNKNOWN markiert; die einzige EmuTOS-
Quellenaussage ist die **Abwesenheit** von EdDI und das `vq_extnd`-Verhalten
(zitiert).
