# BIOS04 — MIDI und IKBD melden „nie bereit“ und verweigern Ausgabe (`Bcostat`/`Bconout` 3, 4)

Status: **ERLEDIGT** 2026-10-29 ([Quittung](BIOS04-STATUS-DONE.MD)) · Prioritaet: **P1** · Typ: Researcher (FS11-Entscheidung neu treffen), danach Developer — Anwenderentscheidung 2026-10-28, aus Audit `docs/audits/2026-10-28-bios-core.md`

## Sachverhalt

**BIOS-CORE-001 — S2 — CONFIRMED (`Bcostat`) + MEASURED (`Bconout`).** Beleg: Bericht Abschnitt 2, Lauf `measurements/measurement-2026-10-28-bios-core-audit/RUNINFO.TXT`.

- `bios-r1` 66 `Bcostat(3)`, 67 `Bcostat(4)` — DIFF, EmuTOS `-1`, NewTOS `0`, beide Profile (`measurements/measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt`).
- Sonde: B02 `Bconout(3,0xFE)` EmuTOS `1`, NewTOS `0`; B03 `Bconout(4,0x11)` `1` / `0`; B11/B12 `Bcostat(3)`/`(4)` nach der Ausgabe `-1` / `0` (`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b`).
- Lokalisiert: Geraetetabelle `tos-api/src/bios/chardev.rs:360@DEVICES` (Eintraege 3 MIDI und 4 IKBD = `Absent`, `tos-api/src/bios/chardev.rs:103@Absent`).
- EmuTOS-Stellen: `references/emutos/bios/chardev.c:38-42` (Slots 3/4 vertauscht), `references/emutos/bios/ikbd.c:916` `bcostat4`, `:933` `bconout4` (gibt `1`); `references/emutos/bios/midi.c:66` `bcostat3`, `:84` `bconout3` (gibt `1`). Auch EmuTOS ohne IKBD-ACIA antwortet `bcostat4` mit `-1` „OK (but output will be ignored)“ (`ikbd.c:916`ff., `#else`-Zweig).
- Entscheidung/Spec: FS11-Profil „absent device“ (Anwender 2026-10-13) steht in `docs/api/gemdos/file/3f-fread.md` (NewTOS-Notizen, Punkt 1), nicht in `docs/COMPATIBILITY.md`; NewTOS verhaelt sich wie dort beschrieben, die Abweichung ist beobachtbar und ohne COMPATIBILITY-Eintrag nicht ACCEPTED.
- Fehlerbild: ein MIDI-Programm mit `while (!Bcostat(4)) ;` haengt auf NewTOS **fuer immer** (neue Information fuer den Anwender; bei AUX hat FS11 den Fall erkannt, `tos-api/src/bios/chardev.rs:133@AuxNoCable`). Auch `Midiws` (XBIOS) geht ueber denselben Eintrag.

## Arbeit

1. Researcher: Entscheidung mit dem Haenger als neuer Information vorlegen — FS11 fuer MIDI/IKBD auf „bereit, Ausgabe verwerfen“ (EmuTOS-Muster) aendern oder die Abweichung in `docs/COMPATIBILITY.md` aufnehmen.
2. Developer: Fix gemaess Entscheidung; Hosttests fuer `Bcostat` und `Bconout` an 3/4.

## Entscheidung (Anwender, 2026-10-29)

„Wenn es in EmuTOS auch so ist, dann ja“: Geraete 3 und 4 verhalten sich wie
das Orakel (Referenzbau mit MIDI- und IKBD-ACIA, `references/emutos/bios/chardev.c:30-45`):

| | Geraet 3 (MIDI) | Geraet 4 (IKBD) |
|---|---|---|
| `Bconstat` | 0 (kein Eingang) | 0 (`char_dummy`) |
| `Bconin` | **wartet** wie EmuTOS `bconin3` (`references/emutos/bios/midi.c:35`), Wartestil wie AUX nach XBIOS15/XBIOS18 (`give_way`) | 0 (`char_dummy`) |
| `Bcostat` | -1 (vertauscht: `bcostat4`) | -1 (vertauscht: `bcostat3`, TDRE) |
| `Bconout` | 1, Ausgabe verworfen | 1, Ausgabe verworfen |

Damit kein COMPATIBILITY-Eintrag fuer `Bconin(3)` (anders als Vorschlag c2):
das blockierende Lesen ist EmuTOS-Verhalten, Praezedenz `Bconin(1)` (XBIOS15).
Die drei Stellen in `docs/api/xbios/0c-midiws.md` (Researcher, Beobachtung
ausserhalb des Umfangs) werden im selben Schritt berichtigt.

## Lokalisierung (Lead, 2026-10-29)

- Geraetetabelle `tos-api/src/bios/chardev.rs:360@DEVICES`, Eintraege 3/4 heute
  `Absent`; `mapping.tsv`: `bcostat3`, `bcostat4`, `bconout4`, `bconin3`,
  `bconstat3` = `design:hardware:*` mit Notiz „absent device“ — nach dem Fix
  `mapped` auf den neuen Eintrag (Ratsche); `bconout3` `mapped` auf
  `tos-api/src/xbios/system.rs@midiws`.
- `--up` (Wiedertestmenge): `bcostat`/`bconout`/`bconin`/`bconstat` in
  `tos-api/src/bios/chardev.rs`, `biostrap` → `tos-api/src/bios/mod.rs@dispatch`,
  `midiws`/`ikbdws` in `tos-api/src/xbios/system.rs`, GEMDOS `cauxos`, `cconos`,
  `cprnos`, `cauxin`, `getch`, `conbrk`, `constat` in
  `tos-api/src/gemdos/console.rs`. `unmapped` darin nur desk/vdi/aes-Aufrufer
  (`bios_conout`, `bios_conws`, `bios_prnout`, `bios_conin`, `bios_conis`,
  `fm_do`, `gchr_key`, `gloc_key`) — keine Geraete 3/4. Vollstaendig: nein,
  diese Liste.
- Hosttests heute fuer den alten Zustand: `tos-api/tests/fs11_device_rw.rs`
  (Absent-Faelle), apitest BC15/BC16/BC23/BC24 (`sdk/apps/apitest/apitest.c`).

## Abnahme

- Bestaetigende Faelle: `bios-r1` 66/67 (und 63–71) sowie Sonde B02/B03/B11/B12 EQUAL gegen das Orakel; vorgeschlagene Zeilen `13:3 w:3,w:0xFE` und `13:3 w:4,w:0x11` (Bericht Abschnitt 5) sind erst nach Lockerung des Round-1-Waechters in `bios-core-r1.tsv` aufnehmbar ([BIOS13](BIOS13.md)).
- Kein vorher gleicher Fall in `bios-r1.tsv`, `bios-core-r1.tsv`, `xbios-chardev-r1.tsv`, `xbios-system-r1.tsv` weicht neu ab.
- `cargo xtask test` gruen.
- `docs/STATUS.md` und Spec im selben Commit; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Vorschlag (Researcher, 2026-10-29)

Umfang: nur `Bcostat`/`Bconout` fuer Geraet 3 (MIDI) und 4 (IKBD) samt Folgen
fuer `Midiws`, `Ikbdws` und GEMDOS. Gelesen wurde der Quelltext am Pin
`1a7484ab`; **Quelle** = gelesen und zitiert, **Schluss** = aus der Quelle
gefolgert.

### 1. Was EmuTOS tut (Quelle)

- Tabelle: `bconout_init[3] = bconout3` (MIDI), `[4] = bconout4` (IKBD);
  `bcostat_init[3] = bcostat4`, `[4] = bcostat3` (vertauscht);
  `bconstat_init[3] = bconstat3`, `[4] = char_dummy`; `bconin_init[3] = bconin3`,
  `[4] = char_dummy` (`references/emutos/bios/chardev.c:13-42`).
- IKBD, `bcostat4` (`references/emutos/bios/ikbd.c:916-928`): mit
  `CONF_WITH_IKBD_ACIA` `-1` wenn TDRE, sonst `0`; mit `CONF_WITH_FLEXCAN`
  `-1`; **sonst (`#else`) `-1` „OK (but output will be ignored)"**.
  `bconout4` (`:933-937`) ruft `ikbd_writeb` und gibt **immer `1`**.
  `ikbd_writeb` (`:957-969`) wartet in `while (!bcostat4());`, schreibt dann
  je nach Konfiguration (ACIA, FlexCAN, Amiga) oder — ohne alle drei — **gar
  nicht**; es kehrt also auch ohne IKBD-Hardware zurueck.
- MIDI, `bcostat3` (`references/emutos/bios/midi.c:66-81`): mit
  `CONF_WITH_MIDI_ACIA` `-1` wenn TDRE, sonst `0`; **ohne ACIA immer `0`**.
  `bconout3` (`:84-95`) wartet `while(!bcostat3())` und gibt mit ACIA `1`.
  **Ohne ACIA haengt `bconout3` selbst fuer immer** (der `#else return 0L`-Zweig
  ist unerreichbar). EmuTOS hat fuer MIDI also **kein** „Absent-Muster"; das
  Muster „bereit, Ausgabe verworfen" gibt es nur fuer IKBD. (Das Audit
  BIOS-CORE-001 verallgemeinert den IKBD-`#else`-Zweig auf beide Geraete;
  fuer MIDI traegt die Quelle das nicht. Die Aussage trifft auf MIDI nur als
  Orakelwert zu: das Orakel hat die ACIA, `references/emutos/include/config.h:994-996`.)
- Konfiguration: Standard `CONF_WITH_MIDI_ACIA = 1`, `CONF_WITH_IKBD_ACIA = 1`
  (`references/emutos/include/config.h:994-996,1001-1003`); `0` nur fuer
  Nicht-Atari-Hardware (`#if !CONF_ATARI_HARDWARE`, `references/emutos/include/config.h:673,710-714`).
  Der Referenzbau (make 512, Atari-Hardware) liefert damit `-1`/`1` — das
  sind die Orakelwerte B02/B03/B11/B12 und `bios-r1` 66/67.
- XBIOS: `midiws` (`bios/midi.c:104-110`) ruft `bconout3(3, *ptr++)` in
  `do/while (cnt--)`, ignoriert das Ergebnis; `ikbdws` (`bios/ikbd.c:948-955`)
  ruft `ikbd_writeb` **direkt**, nicht `bconout4`; beide `void`
  (`bios/xbios.c:296-306,571-578`).
- GEMDOS: `osif` loest Handles nur zu `-1..-3` (CON/AUX/PRN) auf, `num < -3`
  → `EIHNDL`; `Bconout`/`Bconin` werden mit `HXFORM(num) = 3+h`, also nur mit
  Geraet 0..2, gerufen (`references/emutos/bdos/bdosmain.c:589-639`,
  `bdos/console.h:23-25,36`; Std-Handles 4/5 zeigen auf CON,
  `bdos/console.c:60-67`).

### 2. NewTOS heute (Quelle)

- `DEVICES[3]` und `[4]` = `Absent` (`tos-api/src/bios/chardev.rs:363@DEVICES`,
  `tos-api/src/bios/chardev.rs:105@Absent`): alles `0`. AUX ist seit FS11-AUX
  `AuxNoCable` (`tos-api/src/bios/chardev.rs:136@AuxNoCable`): `Bconout` 1,
  `Bcostat` -1, Eingabe `0` ohne Blockieren.
- Die Vertauschung beim Status steckt in
  `tos-api/src/bios/chardev.rs:446@bcostat_handle`; sie bleibt unter jeder
  Option unbeobachtbar, solange beide Eintraege dasselbe melden.
- `Midiws` ruft `bconout_handle(DEV_MIDI, …)` pro Byte, Ergebnis verworfen
  (`tos-api/src/xbios/system.rs:82@midiws`); `Ikbdws` ist ein reines No-op und
  ruft nie einen Geraeteeintrag (`tos-api/src/xbios/system.rs:136@ikbdws`).
- Kein Kernel-/GEMDOS-Pfad ruft Geraet 3/4: `bconout_handle`-Aufrufer in
  `gemdos/console.rs` laufen ueber `std_dev` → `hxform` (Geraet 0..2,
  `tos-api/src/gemdos/console.rs:75@std_dev`), sonst nur `Midiws`.

### 3. Betroffene Programme (Quelle: Greps 2026-10-29)

Gegreppt wurde `sdk/apps`, `sdk/overlays`, `sdk/examples`, `programs/`,
`tosdump/` sowie die Bibliotheken `sdk/libc`, `gemlib`, `cflib`, `windom`,
`ldg`, `libshare`, `zview` auf `Bcostat`, `Midiws`, `Ikbdws`,
`Bconout/Bconin/Bconstat(3|4…)`.

- **Kein portiertes Programm und keine Bibliothek** ruft `Bcostat`, `Midiws`,
  `Ikbdws` oder ein `Bconout/Bconin/Bconstat` auf 3/4; es gibt nur die
  Makro-Definitionen in `sdk/libc/mintlib/include/mint/osbind.h:577,621,652`.
  Die Treffer sonst sind Geraet 2 (`Bconout(2, 7)` Klingel in QED, cflib) und
  `_console_read_byte` mit `Bconin(f+3)` fuer `f ∈ {-1,-2,-3}` = Geraet 2/1/0
  (`sdk/libc/mintlib/mintlib/console.c:30-33`). MIDI/IKBD-Treffer in Doom,
  HighWire, Beispielen und `programs/` betreffen Tastatur-Scancodes, nicht die
  Ausgabe.
- **Einziger Nutzer:** `sdk/apps/apitest/apitest.c`, und zwar als **Pinning des
  heutigen Verhaltens**: BC15/BC16 `Bconout(3|4,'*') → 0`
  (`sdk/apps/apitest/apitest.c:1913@ck_eq`, `:1914`), BC23/BC24 `Bcostat(3|4) → 0`
  (`:1923`, `:1924`). Unter (a)/(c) werden diese vier Zeilen auf `1`/`1`/`-1`/`-1`
  umgestellt; BC04/BC05/BC08/BC09 (`Bconstat`/`Bconin` 3/4 → 0) bleiben.
- Luecke „kein lokaler Quelltext": ausser diesen Quellen gibt es auf dem
  Standard-Abbild keine Programme, die 3/4 ansprechen koennten; originale
  m68k-Binaries (Sequenzer, Spiele mit MIDI) gibt es nicht auf der Karte und
  kommen erst mit dem m68k-Thunk (Schluss). **Nicht geprueft:** die Manifeste
  einzelner Karten; der Befund „kein Programm" gilt fuer die Quellen oben, nicht
  fuer beliebige Binaries.
- Host-Tests, die die Absent-Antwort an 3/4 festhalten:
  `tos-api/tests/fs11_device_rw.rs` (Tabelle um Zeile 906; die Swap-Tests
  um Zeile 973-1000 installieren eigene Eintraege und bleiben gueltig).

### 4. Optionen

**(a) FS11 fuer 3/4 auf das EmuTOS-Muster aendern: `Bconout` → 1, `Bcostat` → -1,
Ausgabe verworfen; `Bconstat`/`Bconin` bleiben `0` ohne Blockieren.**

- Schaltet frei: `while(!Bcostat(n));`, `if (Bconout(3,b) == 0) retry` und jede
  Pollschleife vor MIDI/IKBD-Ausgabe kehren zurueck, genau wie auf dem Orakel;
  `bios-r1` 63–71 und die Sonden B02/B03/B11/B12 werden EQUAL; keine
  COMPATIBILITY-Zeile noetig fuer diese vier Werte. Gleiche Begruendung wie die
  AUX-Entscheidung (FS11-AUX, Option A: das Programm haengt sonst im Schreibloop).
- Kosten: ein kleiner Eintragstyp bzw. Wiederverwendung von `AuxNoCable` fuer
  die Ausgabeseite (Eingabe bleibt 0); vier Zeilen in `apitest.c`, die Tests in
  `fs11_device_rw.rs` und die Doku (`docs/api/bios/03-bconout.md`,
  `08-bcostat.md`, `docs/api/gemdos/file/3f-fread.md` Note 1, `docs/api/xbios/0c-midiws.md`)
  nachziehen. Aufwand klein.
- Bricht: nichts Gemessenes ausser dem eigenen Pinning (apitest BC15/16/23/24,
  Host-Test). Programme, die „Bcostat(4) == 0" als Erkennung „kein MIDI" nutzen,
  bekommen dasselbe `-1` wie auf jedem echten Atari, auch ohne Kabel — das ist
  die Orakelantwort.
- Betroffene Programme: keines der portierten (Abschnitt 3). Profitieren wuerden
  spaetere originale m68k-Programme.
- Offene Restdivergenz (**bleibt unter jeder Option**, nicht Teil dieses
  Tickets): `Bconin(3)` blockiert in EmuTOS ohne Daten ewig (`bios/midi.c:35-52`
  mit `bconstat3`), NewTOS liefert `0` sofort; wie bei AUX eine dokumentierte
  Abweichung zugunsten „kein Haenger". Die Eingabe von MIDI bleibt ohne Treiber
  stumm.

**(b) FS11 beibehalten, Abweichung in `docs/COMPATIBILITY.md` eintragen.**

- Schaltet frei: nichts Neues; spart die Codeaenderung.
- Kosten: der Eintrag muesste einen **unbegrenzten Haenger** als beobachtbare
  Abweichung ACCEPTED nennen; Begruendung „kein MIDI-Port" traegt nicht, weil
  EmuTOS fuer IKBD ohne Hardware selbst `-1`/`1` antwortet (`ikbd.c:926-928`) und
  die Atari-Hardware die Antwort immer gibt.
- Bricht: jedes Programm mit Statusschleife auf 3/4 (siehe Fehlerbild im
  Sachverhalt) bleibt ein Haenger; `bios-r1` 66/67 und B02/B03/B11/B12 bleiben
  DIFF mit „ACCEPTED". Widerspricht der Regel, EmuTOS 1:1 zu treffen und
  Abweichungen nicht zu rationalisieren.
- Betroffene Programme: dieselben wie (a), nur im Haengerfall.

**(c) Aufgeteilte Entscheidung nach Quelle, je Geraet.**

- (c1) **IKBD (4): EmuTOS-Muster exakt** (`-1` / `1` / verwerfen): die Quelle
  selbst traegt es (`ikbd.c:926-928` `#else`, `:933-937`); `Ikbdws` bleibt No-op.
- (c2) **MIDI (3): `-1` / `1` / verwerfen als „Port da, kein Kabel"**, nicht als
  „Absent": die Quelle kennt nur den ACIA-Fall (`-1`/`1`, `midi.c:66-72,84-93`);
  ohne ACIA steht dort ein Haenger, kein Muster. Der Grund fuer `-1`/`1` ist
  also das Orakel plus die AUX-Praezedenz, nicht ein EmuTOS-Fallback. Das ist
  in der Spec so zu benennen.
- Unterschied zu (a): gleiche Werte, aber getrennte Begruendung und je ein
  Eintrag, damit ein spaeterer MIDI-Treiber (`TASK/IO01.md`) nur Eintrag 3
  ersetzt. Kosten und Bruch wie (a), kein zusaetzlicher.
- (c3) Nur `Bcostat` auf `-1`, `Bconout` bei `0` lassen: beendet die
  `while(!Bcostat)`-Schleife, laesst aber B02/B03 DIFF und jede Schleife der
  Form „wiederhole bis `Bconout != 0`" haengen (Schluss: genau die AUX-stderr-
  Konstellation von FS11-AUX). Nicht empfohlen.
- (c4) Echter MIDI-/IKBD-Treiber (UART/USB-MIDI, Befehlsinterpreter): wuerde
  die Eintraege ersetzen (`install_device`); gehoert zu `TASK/IO01.md`, ist
  fuer den Haenger zu gross und ersetzt (a)/(c) nicht, sondern baut darauf auf.

### 5. Folgen fuer GEMDOS und XBIOS

- **Fread/Fwrite auf „dem MIDI-Handle":** es gibt keinen. EmuTOS kennt nur die
  Handles `-1..-3` und gibt fuer `num < -3` `EIHNDL` (`bdosmain.c:600-602`,
  `bdos/console.h:23-25`); `HXFORM` bildet auf Geraet 0..2 ab. Die Nennung
  von MIDI/IKBD in `3f-fread.md` Note 1 (Tabellenzeile und „Current entries")
  beschreibt die BIOS-Tabelle, nicht einen GEMDOS-Pfad. **Keine der Optionen
  aendert damit ein GEMDOS-Ergebnis.** Die Spec-Zeile ist nicht falsch, aber
  missverstaendlich; beim Nachziehen der Doku unter (a)/(c) klarstellen (kein
  Eingriff jetzt, weil Faktenfehler hier nicht vorliegt).
- **`Midiws`:** wartet weder heute noch unter (a)/(c) auf Status (NewTOS ruft nur
  `bconout_handle` und verwirft das Ergebnis), kehrt also unter allen Optionen
  zurueck. Die Aenderung ist nicht beobachtbar; EmuTOS ohne ACIA haengt hier
  (`midi.c:86-87`) — NewTOS soll das nicht nachahmen. `0c-midiws.md` benennt den
  Widerspruch bereits unter „Inconsistencies".
- **`Ikbdws`:** ruft in EmuTOS `ikbd_writeb` direkt (`ikbd.c:948-955`), in NewTOS
  nichts; unabhaengig von Eintrag 4, unveraendert. Schluss: Bytes mit Wirkung
  (Maus-/Joystickmodus, `0x12`/`0x1A`) werden unter (a)/(c) wie bisher
  verworfen, `Bconout(4)` meldet danach aber `1` (angenommen); das ist die
  Orakelantwort, die Wirkung fehlt weiterhin (`docs/api/xbios/19-ikbdws.md`).
- **`Bcostat`-Vertauschung:** bleibt Teil des Vertrags (`chardev.c:38-42`);
  unter (a)/(c) wegen gleicher Werte nicht beobachtbar, im Code und in
  `fs11_device_rw.rs` erhalten.

### 6. Empfehlung

**Option (c) in der Form c1+c2: Geraete 3 und 4 melden `Bcostat` `-1` und
`Bconout` `1` (Byte verworfen); `Bconstat`/`Bconin` bleiben `0` ohne
Blockieren.** Begruendung: das beseitigt den Haenger, macht `bios-r1` 63–71 und
B02/B03/B11/B12 EQUAL, braucht keine COMPATIBILITY-Ausnahme fuer die Ausgabe
und folgt demselben Muster wie die Anwenderentscheidung zu AUX. Fuer IKBD traegt
das die EmuTOS-Quelle selbst (`ikbd.c:926-928`), fuer MIDI das Orakel (ACIA
vorhanden) — nicht ein EmuTOS-Fallback; die Spec muss das so schreiben. Nur
`Bconin(3)` (Nichtblockieren statt Haenger) bleibt als bewusste Abweichung und
ist beim Developer-Schritt in COMPATIBILITY einzutragen. Kein portiertes
Programm ist betroffen; der Preis ist klein (Eintrag, vier apitest-Zeilen,
Host-Tests, Doku). Die Entscheidung ist reversibel, wenn ein MIDI-Treiber den
Eintrag ersetzt.

Offen fuer den Anwender: Zustimmung zu c1+c2 (oder Wahl von (a) ohne
Geraetetrennung / (b)). Nicht geprueft: Verhalten auf Real-Pi4 (gleiche
Tabelle, nur Hostebene ohne Treiber).

Out-of-scope: `0c-midiws.md` Abschnitte „Behaviour on the Pi"/Edge case 2/
Verification „No-MIDI profile" beschreiben das NewTOS-Ergebnis als EmuTOS-
konform; der Inconsistencies-Block im selben File stellt das bereits klar.
