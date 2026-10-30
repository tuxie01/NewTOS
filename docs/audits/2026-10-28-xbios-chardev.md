# Audit XBIOS Zeichengeraete und Tastatur: `Midiws`, `Iorec`, `Rsconf`, `Keytbl`, `Bioskeys`, `Ikbdws`, `Kbdvbase`, `Kbrate`, `Bconmap` — 2026-10-28

## 1. Kopf

| | |
|---|---|
| Scope | XBIOS trap 14 ohne Plattenzugriff, Zeichengeraete und Tastatur: `Midiws` (0x0C), `Iorec` (0x0E), `Rsconf` (0x0F), `Keytbl` (0x10), `Bioskeys` (0x18), `Ikbdws` (0x19), `Kbdvbase` (0x22), `Kbrate` (0x23), `Bconmap` (0x2C). Der Auftrag nannte „`Kbrate` (0x1E)“; 0x1E ist `Ongibit`, `Kbrate` ist 0x23 und wurde dort geprueft (Abschnitt 11). Scope-Slug `xbios-chardev` |
| Datum | 2026-10-28 |
| NewTOS | `b81d0ddcdd4eb84ee1693095bde80a23f29df271`, sauber — gebaut und gemessen nur im abgekoppelten Worktree (Sitzungs-Scratchpad `wt-audit2`); der Hauptbaum hatte fremde, nicht genutzte TOOL07-Aenderungen (Abschnitt 10) |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df` |
| ROM | `tools/oracle/etos512de-1a7484ab.img`, sha256 `323c87bb…d2750c` (Beginn und Ende) |
| Hatari | 2.6.1, Binary sha256 `953f4e78…dcd6f6`, `tools/oracle/hatari.cfg` sha256 `c8f4998a…f8ae46` (Beginn und Ende) |
| Kanarienvogel | ORACLE §6 aus dem Worktree: `canary tt: OK`, `canary falcontc: OK` (Plattenprofil nicht benutzt: kein Plattenaufruf im Scope) |
| Kernel | `cargo xtask build --board rpi4 --release`, `kernel8.img` sha256 `feab5e0f…a207`; fuer die Eingabelaeufe ein Injektor-Kernel aus einem Scratch-Klon mit frischem `target/` (`83372fd7…4d14`, Abschnitt 10) |
| Karten | je Lauf `tools/mkcard.sh` (64 MiB, MBR + FAT16), alle Manifeste `optional_missing none` |
| Boards | NewTOS: QEMU 11.0.2 `raspi4b`, `-append serial=on`; EmuTOS: Hatari TT (`tt`) und Falcon (`falcontc`) |
| Beleg | `measurements/measurement-2026-10-28-xbios-chardev-audit/RUNINFO.TXT` |

**Wie gemessen wurde.**

1. **tosdiff, vorhandene Tabelle** `tosdump/cases/bios-r1.tsv` (14 Faelle im
   Scope) gegen `EMUTOS-1a7484ab-BIOS{,-falcontc}`, NewTOS am auditierten
   Commit (`runs/NEWTOS-bios-r1`, dieselbe Menge abweichender Faelle wie im
   BIOS-Kern-Audit).
2. **tosdiff, neue Tabelle** `tosdump/cases/xbios-chardev-r1.tsv` (23 Faelle,
   CRC `5C3A5F7A`) innerhalb des Round-1-Waechters: `Iorec`-Randgeraete, die
   zwei `Rsconf`-Abfragen, `Keytbl`/`Kbdvbase` PTR_RAM, `Kbrate(-1,-1)`,
   `Bconmap` auf allen erlaubten Randwerten. Referenzen neu auf dem Pin,
   Wiederholung bitgleich, `tosdiff --selfcheck` gruen auf beiden Profilen.
3. **Sonden** (Verdikt MEASURED): alles, was der Waechter abweist — die
   Setz-Arme von `Rsconf`, `Keytbl`, `Kbrate`, `Bconmap`, die Aufrufe
   `Midiws`, `Bioskeys`, `Ikbdws` — und **was hinter einem Zeigerergebnis
   steht** (`Iorec`, `Keytbl`, `Kbdvbase`, `Bconmap(-2)`), das PTR_RAM nicht
   sieht (TASK/TOOL11). Eine C-Quelle, fuer m68k (mintlib) und AArch64
   (SDK-mintlib) gebaut, Aufrufe ueber mintlibs `osbind.h`/`trap_14_*`,
   Strukturen aus mintlibs `ostruct.h`. Jede Sonde lief je Seite zweimal
   (EmuTOS zusaetzlich auf Falcon) mit gleichem Ergebnis.
4. **Eingabesonde** `xkprobe`: echte Eingabe auf beiden Seiten — EmuTOS per
   Hatari `--cmd-fifo` (`rightdown`/`rightup`, Taste `a`), NewTOS per
   T10-Injektor (`0xF9 0 0`, `0xF8 0 0`, `0x1E/0x9E`).

## 2. Findings

Lokalisierung, allgemein: `cargo xtask deps <fn> --down` ist fuer `iorec`,
`rsconf`, `keytbl`, `kbdvbase`, `kbrate`, `bconmap` leer; `bioskeys` reicht nur
zu `get_keytbl` und Laenderfunktionen ohne NewTOS-Abbild; `midiws` und `ikbdws`
reichen zu `bconout3`/`bcostat3` bzw. `ikbd_writeb`/`bcostat4`, deren Faelle im
BIOS-Kern-Audit DIFF sind (BIOS-CORE-001). Die Kanten von `rsconf` ueber
`rsconfptr` und vom Paketempfang zu den `kbdvecs`-Haken fehlen im Graphen
(XBIOS-CHARDEV-008).

### XBIOS-CHARDEV-001 — S1 — CONFIRMED (Rueckgabe) + MEASURED (Struktur, Absturz)

**`Iorec` ist nicht verdrahtet: jedes Geraet liefert die Funktionsnummer `0x0E`; das gaengige Tastaturpuffer-Leeren beendet das Programm mit Bus Error.**

- LOCALIZED: `tos-api/src/xbios/mod.rs:65-138@dispatch` (Sammelzweig
  `other => other as i64`, kein `Iorec`-Arm; Absicht im Kommentar
  `tos-api/src/xbios/mod.rs:26-31@PHYSBASE`). `deps iorec --down` leer.
  Beobachtet ueber `Iorec`.
- Faelle: `bios-r1` 101/102, `xbios-chardev-r1` 24–27 — EXACT-DIFF, EmuTOS
  `-1`, NewTOS `0x0E`, beide Profile
  (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt`).
  `bios-r1` 98–100 und `xbios-chardev-r1` 21–23 bestehen PTR_RAM nur, weil
  `0x0E` gerade und ungleich 0 ist (TOOL11). Sonde I01–I08
  (`runs/PROBE-*/XCPROBE.LOG`): EmuTOS `Iorec(0/1/2)` = `0x4A54`/`0x30D4`/`0x30E2`,
  IOREC mit Puffer, `ibufsiz` 256/256/128, Wassermarken 64/192 bzw. 32/96,
  Geraet 0 mit Ausgabe-IOREC und Baud/Flow/UCR; NewTOS fuer alle `0x0E`, die
  „Struktur“ dort sind Nullbytes aus Seite 0. Sonde XCIOFL
  (`io = Iorec(1); io->ibufhd = io->ibuftl;` im Usermodus): EmuTOS laeuft
  weiter (`F02 after write`, TT und Falcon), NewTOS „Bus Error … page-0
  system-variable memory, write“, `far` `0x18`, Programm beendet
  (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/XCIOFL-NEWTOS-raspi4b/BOOT-SERIAL.txt`).
- Eingabepruefung: EmuTOS per `--trace bios,xbios`
  (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/TRACE-EMUTOS-xc/trace-program.txt`):
  Argumente wie gestellt. NewTOS: nicht moeglich (TASK/TOOL08); hier ohne
  Belang, die Antwort haengt vom Argument nicht ab.
- EmuTOS: `references/emutos/bios/xbios.c:336-348`.
- Spec: `docs/api/xbios/0e-iorec.md` „decision deferred“; kein
  COMPATIBILITY-Eintrag. Die Spec behauptet `EINVFN` — falsch (XBIOS-CHARDEV-007).
- Fehlerbild: `Iorec(1)` und das Gleichsetzen von Kopf und Schwanz ist das
  verbreitete „Tastaturpuffer leeren“ (Spiele, Editoren vor einer Abfrage).
  Auf NewTOS endet jedes solche Programm sofort. Lesende Nutzer (Puffer
  pruefen, Wassermarken) sehen Nullen.
- Retest: `deps iorec --up` (2: `xbiostrap`, `biostrap`); Faelle `bios-r1`
  98–102, `xbios-chardev-r1` 21–27; Sonde I01–I08, XCIOFL.
- Route: Developer (die zurueckgestellte Entscheidung der Spec ist damit
  faellig; Researcher fuer den Vertrag des IOREC-Abbilds).

### XBIOS-CHARDEV-002 — S2 — MEASURED

**`Bioskeys` ist nicht verdrahtet: eine per `Keytbl` installierte Tabelle bleibt aktiv.**

- LOCALIZED: `tos-api/src/xbios/mod.rs:65-138@dispatch` (kein `Bioskeys`-Arm,
  Rueckgabe `0x18`). `deps bioskeys --down`: `get_keytbl` … ohne
  NewTOS-Abbild; was `get_keytbl` liefert, ist ueber `Keytbl(-1,-1,-1)`
  gemessen gleich (K02: US-Tabellen bitgleich), die Bedingung gilt damit als
  erfuellt.
- Faelle: Sonde K04–K10: EmuTOS stellt nach `Bioskeys()` alle drei Zeiger
  auf die Ausgangstabellen zurueck (`unshift_is_n0=1 shift_is_s0=1
  caps_is_c0=1`), NewTOS behaelt die Programm-Tabellen (`unshift_is_my=1`).
  Eingabesonde (`runs/KEYS-*/XKPROBE.LOG`): Taste `a` nach `Keytbl(my_n)`
  auf beiden `0x001E004B` (gleich), Taste `a` **nach** `Bioskeys()` EmuTOS
  `0x001E0061`, NewTOS `0x001E004B`. XCKTNL: nach `Keytbl(NULL,-1,-1)` und
  `Bioskeys()` zeigt `norm` auf EmuTOS wieder auf eine Tabelle, auf NewTOS
  ist er NULL (NewTOS hat dabei 128 Byte aus Seite 0 als Tabelle uebernommen,
  `tos-api/src/xbios/keytbl.rs:210@set_key_table`).
- Eingabepruefung: EmuTOS-Trace (`measurements/measurement-2026-10-28-xbios-chardev-audit/runs/PROBE-EMUTOS-tt/trace-program.txt`:
  `Bioskeys()` zweimal). NewTOS: TOOL08.
- EmuTOS: `references/emutos/bios/ikbd.c:136-140`.
- Spec: `docs/api/xbios/18-bioskeys.md` „Step-2 — not wired“; kein
  COMPATIBILITY-Eintrag. `docs/COMPATIBILITY.md` (Tastaturlayout-Abschnitt)
  sagt „XBIOS `Keytbl` works as on EmuTOS“ — fuer `Keytbl` gemessen wahr, das
  Gegenstueck `Bioskeys` fehlt.
- Fehlerbild: ein Programm installiert eigene Tabellen und stellt beim Ende
  mit `Bioskeys()` zurueck (das dokumentierte Verfahren). Auf NewTOS bleibt
  die Tastatur verstellt; weil `set_key_table` den globalen Uebersetzer
  aendert (`kernel/src/host_env.rs:330@set_key_table`), auch fuer den Desktop
  und alle folgenden Programme bis zum Neustart (diese Folge nach
  Programmende: PLAUSIBLE, Fall: XCKTNL, dann nach Programmende eine Taste
  im Desktop pruefen).
- Retest: `deps bioskeys --up` (6: `kbd_init`, `bios_init`, …); Sonde K04–K11,
  XCKTNL, Eingabesonde X01/X03.
- Route: Developer (`keymap::select_builtin` existiert laut Spec).

### XBIOS-CHARDEV-003 — S2 — MEASURED

**Haken in `mousevec`, `statvec`, `clockvec`, `joyvec` der `Kbdvbase`-Tabelle werden nie gerufen.**

- LOCALIZED: `kernel/src/ikbd.rs:187@MS_BUTTONS` (Mauspaket-Kopf geht in den
  VDI-Mauszustand, `kernel/src/ikbd.rs:219@mouse_event`), `kernel/src/ikbd.rs:194@PKT_REMAINING`
  (Status-/Uhr-/Joystickpakete werden verschluckt); nur `kbdvec` wird
  ausgewertet (`tos-api/src/xbios/kbdvecs.rs:352@route_key`).
  `deps kbdvbase --down` leer.
- Faelle: Eingabesonde X04: zwei echte Mauspakete (rechte Taste ab/auf) —
  EmuTOS `mousevec calls=2`, letztes Paket `F8 00 00` (TT, Falcon, Wdh.),
  NewTOS `calls=0` (beide Laeufe), obwohl der Injektor die sechs Bytes
  nachweislich eingespeist hat (`[inj] … feed 0xf9/0xf8` in
  `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/KEYS-NEWTOS-raspi4b-injector/BOOT-SERIAL.txt`). Sonde W04:
  `clockvec/statvec/joyvec` EmuTOS `1/1/1` (Statuspaket `08 00…` = relativer
  Mausmodus), NewTOS `0/0/0` — hier ist die Wurzel nicht trennbar von
  XBIOS-CHARDEV-004 (auf NewTOS entsteht kein Paket).
- Gleich: Zeiger gerade und < 2 GiB, zweiter Aufruf gleicher Zeiger, alle neun
  Slots gueltige Adressen, `kbstate` 0, `kbdvec` direkt davor gueltig, Slots
  im Usermodus beschreibbar und zuruecksetzbar (V01–V03; MEM12 private Seite).
  Unterschied ohne Befund: NewTOS belegt alle neun Slots mit demselben
  `ret`-Stub (EmuTOS 7 verschiedene Werte, `midisys`/`ikbdsys` echte
  Handler) — dokumentiert (`tos-api/src/xbios/kbdvecs.rs:147@write_table`).
- EmuTOS: `references/emutos/bios/aciavecs.S:470-530` (Paket → Vektor,
  `kbd_jump_vec`).
- Spec: `docs/api/xbios/22-kbdvbase.md` C6 „D7-Luecke“, `docs/STATUS.md`
  „Blocked“ (KEYS02/D7); **nicht** in `docs/COMPATIBILITY.md` (dort nur
  Tabelle je Prozess und `kbdvec`-Upcall).
- Fehlerbild: Programme, die Rohdaten der Maus ueber `mousevec` lesen
  (Spiele, Maus-Beschleuniger, Malprogramme mit eigener Cursorfuehrung),
  bekommen still nichts; `Kbdvbase` meldet die Tabelle wie gewohnt.
- Retest: `deps kbdvbase --up` (404, darunter `vdimouse_init`,
  `mouse_int`); Eingabesonde X04, Sonde V01–V03/W04.
- Route: Developer (oder Researcher, falls die D7-Luecke als Abweichung in
  COMPATIBILITY sanktioniert werden soll).

### XBIOS-CHARDEV-004 — S3 — MEASURED — NOT LOCALIZED

**`Ikbdws` ist ein No-op: IKBD-Kommandos erzeugen keine Antwortpakete.**

- NOT LOCALIZED: vermutete Wurzel `tos-api/src/xbios/system.rs:136@ikbdws`
  (gibt 0 zurueck, ignoriert beide Argumente — No-Stub-Regel). Die
  `--down`-Abhaengigkeit `bcostat4` ist DIFF (BIOS-CORE-001, `Bcostat(4)`
  EmuTOS `-1`, NewTOS `0`); fehlender Unterbau: `bios-r1` 67 muss erst
  gleich sein.
- Faelle: Sonde W01–W04: `Ikbdws(0,{0x1C})` (Uhr abfragen),
  `Ikbdws(0,{0x88})` (Mausmodus erfragen), `Ikbdws(0,{0x16})` (Joystick
  abfragen) — EmuTOS ruft je den Haken einmal, NewTOS nie. Rueckgabe `void`
  (EmuTOS `d0` = 2, NewTOS 0, nicht verglichen).
- EmuTOS: `references/emutos/bios/ikbd.c:948-954`.
- Spec: `docs/api/xbios/19-ikbdws.md` (No-op, Interpreter „does not exist
  yet“); kein COMPATIBILITY-Eintrag. IKBD-Uhr und Joystick fallen unter die
  Hardware-Nicht-Ziele; Mausmodus-Kommandos (0x08/0x09/0x0A/0x12) betreffen
  aber beobachtbares Verhalten (nicht gemessen, Abschnitt 6).
- Fehlerbild: ein Programm, das die IKBD-Uhr per `0x1C` liest und auf das
  Paket wartet, haengt; ein Spiel, das die Maus per `0x12` abschaltet, bekommt
  weiter Mausbewegungen.
- Retest: `deps ikbdws --up` (20: `clock_init`, `gettime`, …); Sonde W01–W04.
- Route: Researcher (COMPATIBILITY-Eintrag oder Umfang eines Interpreters
  entscheiden).

### XBIOS-CHARDEV-005 — S3 — CONFIRMED

**`Bconmap` meldet „nicht vorhanden“ (`0x2C`) fuer jedes Argument; der gepinnte EmuTOS hat `Bconmap` auf jeder Maschine.**

- LOCALIZED: `tos-api/src/xbios/system.rs` (`bconmap`, bis XBIOS05; seitdem `tos-api/src/xbios/bconmap.rs`) (konstant `0x2c`).
  `deps bconmap --down` leer.
- Faelle: `bios-r1` 183, 185, 186; `xbios-chardev-r1` 43, 45–53 — EXACT-DIFF
  (EmuTOS `6` TT / `7` Falcon fuer `-1`, `0` fuer ungueltige Geraete; NewTOS
  `44`), beide Profile. `bios-r1` 184 / `xbios-chardev-r1` 44 (`Bconmap(-2)`)
  bestehen PTR_RAM mit `0x2C` (TOOL11). Sonde B01–B24: EmuTOS `BCONMAP` mit
  gueltigem `maptab`, `maptabsize` 4 (TT) / 3 (Falcon), Umschalten auf 7, 8, 9
  und zurueck aendert `Iorec(0)`; NewTOS dereferenziert an `0x2C`:
  `maptab` NULL, Groesse 0.
- EmuTOS: `references/emutos/bios/serport.c:1082-1124`; Vorbelegung
  `references/emutos/bios/serport.c:955-1003` — auch eine ST-Maschine unter
  diesem ROM hat `maptabsize = 1`, `mapped_device = 6` (`:962-965`).
- Spec: `docs/api/xbios/2c-bconmap.md` waehlt `0x2c` „as on a single-serial
  build“; der Referenzbau (make 512) ist kein solcher Bau. Kein
  COMPATIBILITY-Eintrag. Wurzel von BIOS-CORE-007 (Handles 6–9).
- Fehlerbild: Programme pruefen `Bconmap(0) == 0x2C` und fallen auf Handle 1
  zurueck — das funktioniert. Wer `Bconmap(-2)` ohne diese Pruefung liest,
  liest Seite-0-Bytes als Tabelle.
- Retest: `deps bconmap --up` (2); Faelle oben, Sonde B01–B24; danach
  BIOS-CORE-007.
- Route: Researcher (Profil „ein Port, `maptabsize` 1“ wie EmuTOS auf dem ST,
  oder die Abweichung in COMPATIBILITY eintragen).

### XBIOS-CHARDEV-006 — S3 — CONFIRMED

**`Rsconf(-1 x6)`: TSR-Byte ohne Bit 7 und die oberen 32 Bit null statt vorzeichenerweitert.**

- LOCALIZED: `tos-api/src/xbios/system.rs:239@RS_TSR` (Schatten `0x01`) und
  `tos-api/src/xbios/system.rs:247-279@rsconf` (`old as i64`, nullerweitert).
- Faelle: `bios-r1` 105, `xbios-chardev-r1` 31 — EXACT-DIFF: TT
  `0xFFFFFFFF_88018100`, Falcon `0xFFFFFFFF_88000000` (SCC), NewTOS
  `0x00000000_88010100`. Sonde S01–S18: die Semantik ist sonst gleich (Baud
  setzen/abfragen, `-1` behaelt, `16`/`-3` ausserhalb, UCR/RSR setzen und
  zuruecklesen, Maskierung auf ein Byte, Wiederherstellung); nur das TSR-Byte
  (`81` gegen `01`) und die obere Haelfte weichen ab.
- EmuTOS: `references/emutos/bios/serport.c:217-249` (`rsconf_mfp` liest die
  MFP-Register; Bit 7 von TSR ist „Puffer leer“).
- Spec: `docs/api/xbios/0f-rsconf.md` sagt „zero-extended into `x0`“ —
  widerspricht `tos-api/src/trap.rs` („sign-extended `i64`“) und der
  Dumper-Regel (`docs/TOS-REFERENCE-DUMPER.md` §3 `ret_hi`). Die
  Registerbytes sind Geraeteparameter (TT ≠ Falcon); ohne COMPATIBILITY-Eintrag
  DIFF.
- Fehlerbild: ein LP64-Programm, das `long r = Rsconf(-1,…)` als negativ
  erwartet, oder das TSR-Bit „Sender leer“ abfragt, sieht etwas anderes.
- Retest: `deps rsconf --up` (2); Faelle oben, Sonde S01–S18.
- Route: Developer (Erweiterung), Researcher (Spec gegen ABI, Geraeteprofil
  der Registerbytes).

### XBIOS-CHARDEV-007 — S4 — MEASURED (Doku widerspricht Messung oder Code)

Route: Researcher.

1. `docs/api/xbios/0e-iorec.md` (Pi binding) und `docs/api/xbios/18-bioskeys.md`
   (Pi binding): „returns `EINVFN`“ — gemessen: die Funktionsnummer (`0x0E`,
   `0x18`).
2. `tos-api/src/xbios/mod.rs:26-31@PHYSBASE`: `Iorec`/`Bioskeys` „need NewTOS
   keyboard/mouse infrastructure that does not exist yet“ — fuer `Bioskeys`
   nennt die eigene Spec `keymap::select_builtin` als vorhanden.
3. `docs/COMPATIBILITY.md`, Abschnitt „The IKBD vector table is per process“:
   „NewTOS keeps the table in each process's own page 0 (`0x9A0` …)“ —
   gemessen `0x7F010` (private RW-Seite seit MEM12, wie der MEM12-Absatz
   weiter unten richtig sagt).
4. `docs/api/xbios/2c-bconmap.md` nennt das NewTOS-Profil einen
   „single-serial build“; der gepinnte Referenzbau hat `BCONMAP_AVAILABLE`
   (XBIOS-CHARDEV-005).
5. `docs/api/xbios/0f-rsconf.md` „zero-extended“ gegen das Trap-ABI
   (XBIOS-CHARDEV-006).
6. Ledger: `tosdump/cases/COVERAGE-BIOS.md` braucht einen Abschnitt fuer
   `xbios-chardev-r1.tsv` (der Auditor aendert es nicht).

### XBIOS-CHARDEV-008 — S4 — PLAUSIBLE (Werkzeugbefund)

Route: Researcher. Graph und Mapping (`docs/analysis/deps/`):
(a) `rsconf` hat keine Kanten ueber `rsconfptr` zu `rsconf1`/`rsconfB`/
`rsconfTT`/`rsconf_dummy`, `bconmap` keine zu den `MAPTAB`-Zielen;
(b) `kbdvbase` hat keine Kante zu den Rufern der Haken (`aciavecs.S`
`kbd_jump_vec`); (c) `mapping.tsv` bildet `bconout3` auf
`tos-api/src/xbios/system.rs@midiws` ab (`deps midiws --down`), richtig waere
der MIDI-Eintrag der Geraetetabelle `tos-api/src/bios/chardev.rs`;
(d) `couplings.md` fehlt weiterhin (wie BIOS-CORE-012). Fall, der es klaert:
`deps rsconf --down` listet die vier `rsconf*`-Ziele.

## 3. Offene Tickets im Scope

| Ticket | Was nachgemessen | Ergebnis |
|---|---|---|
| `TASK/TOOL11.md` (Frage: ist `Keytbl` → `0x768` eine gueltige Antwort, sind `Iorec`/`Bconmap(-2)` Zeiger?) | Sonde K01–K03, I01–I03, B02–B04 | `Keytbl`: **EQUAL** — `0x768` zeigt auf ein gueltiges `struct keytbl`, Tabellen bitgleich zu EmuTOS (US). `Iorec`, `Bconmap(-2)`: **DIFF** — keine Zeiger (XBIOS-CHARDEV-001, -005). Die Regel selbst ist TOOL11s |
| `TASK/MEM12.md` (Kbdvbase-Tabelle in der privaten RW-Seite, `Keytbl`-Bereich in Seite 0) | Sonde V01–V03 (Haken im Usermodus schreiben und zuruecksetzen), K01–K02 (Lesen im Usermodus) | **EQUAL** fuer Schreib-/Lesbarkeit im Usermodus |
| `TASK/IO01.md` | Idee, kein Defektticket; die gemessenen Geraeteprofile stehen in XBIOS-CHARDEV-005/-006 und BIOS-CORE-001 | — |

## 4. Coverage-Matrix

| EmuTOS | NewTOS (mapping) | Schicht | Status | Faelle |
|---|---|---|---|---|
| `midiws` `bios/midi.c:104` | `tos-api/src/xbios/system.rs:82@midiws` | xbios | **DIFF** — Wurzel BIOS-CORE-001 (MIDI-Eintrag „absent“, referenziert, nicht neu berichtet): Bytes verworfen, 256 Byte in 0 Ticks statt ≥ 10, `Bcostat(3)` 0 statt -1. API-eigenes Ergebnis gleich (`void`, kehrt zurueck) | Sonde M01–M05 |
| `iorec` `bios/xbios.c:336` | — (Sammelzweig `tos-api/src/xbios/mod.rs:65-138@dispatch`) | xbios | **DIFF** — XBIOS-CHARDEV-001 | `bios-r1` 98–102, `xbios-chardev-r1` 21–27, Sonde I01–I08, XCIOFL |
| `rsconf` `bios/xbios.c:376` | `tos-api/src/xbios/system.rs:247@rsconf` | xbios | **DIFF** — XBIOS-CHARDEV-006 (Setz-/Abfragesemantik gleich) | `bios-r1` 105–106, `xbios-chardev-r1` 30–31, Sonde S01–S18 |
| `keytbl` `bios/ikbd.c:122` | `tos-api/src/xbios/keytbl.rs:185@keytbl` | xbios | **EQUAL** — Abfrage, Tabelleninhalt (US), Setz-Arme, Uebersetzung mit eigener Tabelle, `Keytbl(NULL)` | `bios-r1` 109, `xbios-chardev-r1` 34, Sonde K01–K06/K10, XCKTNL N02, Eingabe X02 |
| `bioskeys` `bios/ikbd.c:136` | — (Sammelzweig) | xbios | **DIFF** — XBIOS-CHARDEV-002 | Sonde K07–K11, XCKTNL, Eingabe X01/X03 |
| `ikbdws` `bios/ikbd.c:948` | `tos-api/src/xbios/system.rs:136@ikbdws` | xbios | **DIFF** — XBIOS-CHARDEV-004 | Sonde W01–W04 |
| `kbdvbase` `bios/xbios.c:716` | `tos-api/src/xbios/kbdvecs.rs:198@kbdvbase` | xbios | **DIFF** — XBIOS-CHARDEV-003 (Zeiger, Layout, Beschreibbarkeit gleich) | `bios-r1` 136, `xbios-chardev-r1` 37, Sonde V01–V03, Eingabe X04 |
| `kbrate` `bios/ikbd.c:439` | `tos-api/src/xbios/system.rs:165@kbrate` | xbios | **EQUAL** — Abfrage, Setzen, Maskierung, negative Argumente, `0xFFFF` als WORD, Wiederherstellung (Wirkung auf die Wiederholung: offen, Abschnitt 6) | `bios-r1` 137, `xbios-chardev-r1` 40, Sonde R01–R13 |
| `bconmap` `bios/serport.c:1082` | `tos-api/src/xbios/system.rs` (`bconmap`, bis XBIOS05; seitdem `tos-api/src/xbios/bconmap.rs`) | xbios | **DIFF** — XBIOS-CHARDEV-005 | `bios-r1` 183–186, `xbios-chardev-r1` 43–53, Sonde B01–B24 |

equal 2, accepted 0, diff 7, unmeasured 0 von 9.
Bug-Dichte: 3 CONFIRMED / 9 gemessene Funktionen = 0,33 (mit den vier
MEASURED-Sachfunden 001-Absturz, 002, 003, 004: 7 / 9).

## 5. Neue Faelle

- **`tosdump/cases/xbios-chardev-r1.tsv`** (23 Faelle, CRC `5C3A5F7A`),
  Referenzen `tosdump/reference/EMUTOS-1a7484ab-XBIOS-chardev.{BIN,LOG}` (tt)
  und `-XBIOS-chardev-falcontc.{BIN,LOG}`, gemessen mit `DUMPBIOS.PRG` Build
  `b81d`, `tosdiff --selfcheck` gruen, Wiederholung bitgleich. Ledger-Eintrag
  fehlt (XBIOS-CHARDEV-007 Punkt 6).
- **Vorgeschlagene Zeilen, die der Waechter heute abweist** (brauchen eine
  wiederherstellende Vorstufe oder eine Freigabe im Ledger; die Sonden zeigen,
  dass sie auf beiden Orakelprofilen sicher laufen):

  ```
  14:0x23  w:10,w:5 then w:-1,w:-1     EXACT.W  -  Kbrate set, query 0x0A05, restore 15/2       (R02/R03, EQUAL)
  14:0x23  w:0x1FF,w:0x1FF then -1,-1  EXACT.W  -  Kbrate mask, query 0xFFFF, restore            (R04/R05, EQUAL)
  14:0x0F  w:4,w:-1.. then w:-2,..     EXACT    -  Rsconf baud 4, query 4, restore 1              (S04/S05, EQUAL)
  14:0x0F  w:-1,w:-1,w:0x98,.. then -1 EXACT    -  Rsconf ucr 0x98 read back, restore             (S10/S11, DIFF -006)
  14:0x2C  w:7 then w:-1               EXACT    -  Bconmap(7) remap, query 7, restore             (B13/B14, DIFF -005)
  14:0x18  -                           IGNORE   -  Bioskeys after Keytbl(b0,-1,-1): Keytbl(-1..)
                                                   norm != b0 (needs a pointer-compare class)    (K08, DIFF -002)
  14:0x0E  w:1                         -        ret:14  capture through the returned pointer
                                                   (IOREC: ibufsiz 256, low 64, high 192)         (I02b, DIFF -001)
  ```

  Die letzte Zeile braucht eine Capture-Form „durch den Rueckgabezeiger“, die
  es heute nicht gibt (Abschnitt 11). `Midiws`, `Ikbdws` und Haken-Aufrufe
  bleiben Sonde: sie brauchen Hardware-Antworten bzw. Rueckruf-Code.

## 6. Gemessen vs. offen

Gemessen (beide Seiten, wiederholt): alle 14 Scope-Faelle von `bios-r1`, alle
23 von `xbios-chardev-r1` (TT und Falcon als Referenz), Sonde mit 9 Gruppen
(Setz-Arme, Strukturinhalte, Haken), Varianten Puffer-Leeren und
`Keytbl(NULL)`, Eingabesonde mit Maus und Taste.

Offen:

- NewTOS-seitige Eingabepruefung (TOOL08) — nur indirekt.
- `Kbrate`: Wirkung auf die tatsaechliche Wiederholung (Taste halten);
  COMPATIBILITY dokumentiert die Pumpen-Variante. Fall: Injektor/Hatari
  `keydown` 1 s halten nach `Kbrate(10,5)`, Zahl der Woerter im Puffer.
- `Keytbl`: Aenderung der Tabellenbytes **nach** der Installation (NewTOS
  uebernimmt per Wert, `tos-api/src/xbios/keytbl.rs:185@keytbl` Moduldoku) —
  nicht gemessen; Fall: Tabelle installieren, Byte aendern, Taste.
- `Ikbdws`-Kommandos, die IKBD-Zustand aendern (0x08/0x0A/0x12/0x1A),
  beobachtet ueber Mauspakete — nicht gemessen.
- `Midiws(-1, p)` (65536 Byte) und `Midiws` mit ungueltigem Zeiger — nicht
  gemessen (EmuTOS 21 s bzw. Busfehler).
- Bytes auf der MIDI-Leitung: Hatari 2.6.1 gibt MIDI nur ueber PortMidi aus,
  nicht in eine Datei — EmuTOS-Seite nicht messbar.
- `Rsconf` gegen eine echte serielle Leitung (Baudrate des PL011).
- Tastaturzustand nach Programmende mit fremder Tabelle (XBIOS-CHARDEV-002,
  PLAUSIBLE-Teil).

## 7. Real-Pi4-Liste

1. Programm mit `Keytbl(my_n)` + `Bioskeys()` (Eingabesonde `XKPROBE.NAP`):
   zweite Taste `a` — EmuTOS-Soll `0x001E0061`; danach im Desktop tippen, ob
   die Tastatur normal ist (XBIOS-CHARDEV-002).
2. `mousevec`-Haken mit echter USB-Maus: Klick muss den Haken rufen (EmuTOS
   2 Aufrufe fuer ab/auf; NewTOS in QEMU 0) (XBIOS-CHARDEV-003).
3. `Kbrate(10,5)` mit echter Tastatur: Verzoegerung 200 ms, Rate 100 ms
   gegenueber 300/40 ms Vorgabe.
4. `Rsconf(4,…)` (2400 Baud) und Ausgabe auf AUX: aendert sich die
   PL011-Baudrate (EmuTOS programmiert den Takt um)?

## 8. Akzeptable Abweichungen

Keine als ACCEPTED gezaehlt. `Midiws` folgt dem FS11-Profil „MIDI abwesend“
(`docs/api/xbios/0c-midiws.md`); das API-eigene Ergebnis ist gleich, die
Abweichung entsteht im BIOS-Eintrag (BIOS-CORE-001) und wird dort
entschieden. `Ikbdws` (IKBD-Uhr, Joystick) ist Hardware im Sinne der
Nicht-Ziele, aber ohne COMPATIBILITY-Eintrag und mit beobachtbarer Wirkung
(XBIOS-CHARDEV-004). Gleich und daher keine Abweichung: `Keytbl`-Struktur mit
8-Byte-Zeigern (LP64-Strukturregel von COMPATIBILITY), `Kbdvbase`-Tabelle je
Prozess (COMPATIBILITY-Eintrag, gemessen wie dort beschrieben: Haken im
Usermodus schreibbar, Zeiger stabil). Die `void`-Rueckgaben (`Midiws`,
`Ikbdws`, `Bioskeys`: EmuTOS `d0` Restwert) werden nicht verglichen.

## 9. Out-of-scope observations

- XBIOS 0x1E `Ongibit` (im Auftrag als Nummer genannt) nicht geprueft.
- EmuTOS' VDI setzt seinen Maushandler ueber `Kbdvbase` (`deps kbdvbase --up`: `vdimouse_init`) — Kopplung VDI ↔ `mousevec`, nicht verfolgt.
- Nach einem Absturz wartet NewTOS auf eine Taste („Press any key“); ein unbeaufsichtigter `#Z`-Lauf haengt dort bis zum Timeout.
- Das Orakel uebersetzt mit der US-Tabelle, obwohl das ROM `etos512de` ist (NVRAM-Vorgabe von Hatari) — fuer Tastenvergleiche relevant, nicht verfolgt.

## 10. Cleanup

- Produktcode: nicht angefasst. `AUDIT-TEMP`: keine Zeile gesetzt;
  `grep -rn AUDIT-TEMP` ausserhalb `docs/` und `.claude/` im Hauptbaum und im
  Worktree: leer.
- Injektor: nur im Scratch-Klon `wt-inject` des Worktrees (Marken
  `INJ-ARM-TEMP`, Diff in
  `measurements/measurement-2026-10-28-xbios-chardev-audit/method/injector-scratch.diff`);
  `.git`-Datei und `target/` vor jedem Befehl entfernt, frisch gebaut (Kernel
  `83372fd7…`, Worktree-Kernel blieb `feab5e0f…`); Klon danach geloescht.
- Worktree `wt-audit2`: `git status` leer vor und nach dem Audit; gebaute,
  ignorierte Artefakte (`target/`, `kernel8.img`, `sdk/libc/libc.a`,
  `tosdump/build`, `tosdump/host/build`) bleiben dort.
- Hauptbaum, von mir neu: dieser Bericht, die Laufmappe,
  `tosdump/cases/xbios-chardev-r1.tsv`, vier Referenzdateien
  `tosdump/reference/EMUTOS-1a7484ab-XBIOS-chardev*`. Nichts sonst.
- Fremde Aenderungen im Hauptbaum (nicht von mir, nicht angefasst, nicht
  genutzt): `tools/mkcard.sh`, `tosdump/core/cases.{c,h}`,
  `tosdump/core/dumpbios.c`, `tosdump/core/runner_call.{c,h}`,
  `tosdump/host/Makefile`, `tosdump/tests/stub_abi.{c,h}` geaendert,
  `measurements/measurement-2026-10-28-tool07/` neu (TOOL07) — zu Beginn;
  am Ende waren sie als `0495d622`/`8656379f` committet. Der Audit hat nur
  `b81d0ddc` gebaut und gemessen.
- Laufmappe: die vier EmuTOS-Variantenlaeufe liefen durch einen falschen
  Endparameter 240 s mit ruhendem Desktop; ihr `trace.txt` ist nach dem
  letzten Programmaufruf + 200 Zeilen gekuerzt (Hash und Zeilenzahl des
  vollen Trace in `trace.full.sha256`). Karten nach jedem Lauf geloescht.
- Eigener Schoenheitsfehler: die Kommentarzeile `Bconmap` in
  `tosdump/cases/xbios-chardev-r1.tsv` nennt fuer `serport.c` die Zeilen
  1082 bis 1131; die Datei endet bei Zeile 1126 (`bconmap` endet 1124). Nicht korrigiert: der
  Kommentar steckt in der CRC der gemessenen Referenzen.

## 11. Was den Ablauf erschwert hat oder unklar war

1. **Auftrag nannte `Kbrate (0x1E)`:** 0x1E ist `Ongibit`. Ich habe `Kbrate`
   an 0x23 geprueft und `Ongibit` ausgelassen.
2. **Kein Weg, hinter einen Rueckgabezeiger zu sehen:** PTR_RAM prueft nur die
   Adressklasse (TOOL11); `capture` liest nur Pufferargumente. Vier der neun
   Funktionen liefern Zeiger — ihre Inhalte gingen nur per Sonde. Eine
   Capture-Form „durch `ret`“ (mit Laenge und Einheit) wuerde `Iorec`,
   `Keytbl`, `Kbdvbase`, `Bconmap(-2)` tosdiff-faehig machen.
3. **Der Waechter laesst keinen Setz-Arm und keinen der Aufrufe `Midiws`,
   `Bioskeys`, `Ikbdws` zu:** vier der sechs Sachfunde sind daher MEASURED,
   nicht CONFIRMED (wie im BIOS-Kern-Audit, Punkt 4).
4. **Kein XBIOS-Argument-Tracer auf NewTOS (TOOL08).**
5. **Laeufer-Vorlagen:** `run-emutos.sh` koppelt das Wartende an `BCHANG.LOG`
   (mein falscher Aufruf kostete 8 min und 80 MB Trace); `run-newtos.sh`
   wechselt vor `mkcard.sh` ins Repo, ein relativer NAP-Pfad laesst den Lauf
   still leer enden (einmal passiert, Lauf verworfen).
6. **Absturzbildschirm wartet auf eine Taste:** ein erwarteter Absturz
   (XCIOFL) laesst den QEMU-Lauf bis zum Timeout stehen; der Laeufer erkennt
   „Panic“ nicht.
7. **Tastaturlayout des Orakels nicht dokumentiert:** `etos512de` uebersetzt
   auf Hatari mit der US-Tabelle; erst ein zweiter NewTOS-Lauf ohne `#K de`
   machte die Tabellen vergleichbar. Gehoert in `docs/analysis/ORACLE.md`.
8. **MIDI-Ausgabe am Orakel nicht abgreifbar** (Hatari nur PortMidi).
9. **Widerspruch in den Anweisungen:** eine allgemeine Agentennotiz sagt,
   keine Berichtsdateien zu schreiben; Rolle und Auftrag verlangen diesen
   Bericht. Ich bin dem Auftrag gefolgt.

Re-Anker: Scope waren die neun XBIOS-Zeichengeraete-/Tastaturfunktionen ohne
Plattenzugriff; jede ist gemessen (tosdiff und/oder Sonde), offene Modi stehen
in Abschnitt 6.
