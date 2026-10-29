# EmuTOS Master-Funktionsinventar — alle Subsysteme

> Researcher, 2026-07-23. Reine Präsenz-/Auflistungs-Aufgabe (**kein**
> Verhaltens-Audit, **kein** Produkt-Code). Ziel: ein Master-Verzeichnis
> über **alle** EmuTOS-Funktionen, analog zu den drei bereits vorhandenen
> Subsystem-Inventaren (AES, VDI, BIOS-Trap-13), die hier **verlinkt statt
> dupliziert** werden. Neu **enumeriert** in diesem Dokument: **GEMDOS/BDOS**
> (`bdos/*.c`), **XBIOS** (`bios/xbios.c`-Dispatch + Trap-14-Zielfunktionen),
> **Util** (`util/*.c`) und **Line-A-Init** (`bios/lineainit.c`).
>
> Methodik: `ctags -x` je Datei (wie AES/VDI), Opcode-Tabellen aus den
> tatsächlichen EmuTOS-Dispatch-Arrays (`bdos/bdosmain.c: funcs[]`,
> `bios/xbios.c: xbios_vecs[]`) statt aus Gedächtnis/Compendium. Die
> HAS/NO-COUNTERPART/?-Klassifikation ist **Bonus** (schnelle, präzise
> `grep`/Fn-Signatur-Treffer in `tos-api/src`/`kernel/src`); wo kein
> eindeutiger Treffer vorlag, steht **„?"** — es wurde nichts geraten.

---

## 1. Dashboard — Gesamtzahlen

| Subsystem | Funktionen (Symbole) | Quelle/Datei | Status |
|---|---|---|---|
| **AES** (Trap #2, Selektor 200) | **402** | `docs/analysis/aes-audit/emutos-function-inventory.md` | fertig (anderer Pass) |
| **VDI** (Trap #2, Selektor 115) | **269** | `docs/analysis/vdi-audit/emutos-vdi-function-inventory.md` | fertig (anderer Pass) |
| **BIOS** (Trap #13 Dispatcher) | **12** (+ 8 Boot-Init-Elemente, keine „Funktionen" im engeren Sinn) | `docs/analysis/bios-audit/emutos-bios-inventory.md` | fertig (anderer Pass, referenziert) |
| **GEMDOS/BDOS** (Trap #1 + BDOS-intern) | **183** ctags-Symbole über `bdos/*.c` (17 Dateien); davon **53** zugleich öffentliche Trap-#1-Opcodes (Tabelle A, mit Opcode-Nummer; die übrigen 130 sind BDOS-interne Helfer, Tabelle B — **kein** Aufaddieren, 53 ⊂ 183) | Abschnitt 2 (dieses Dokument) | **neu, dieser Pass** |
| **XBIOS** (Trap #14) | **103 real implementierte Ziel-Funktionen** über 142 Opcode-Slots (0x00–0x8D); die übrigen 39 Slots sind `xbios_unimpl`-Platzhalter (keine echten Funktionen, s. §3.3) | Abschnitt 3 (dieses Dokument) | **neu, dieser Pass** |
| **Util** (+ Line-A-Init) | **61** (59 `util/*.c` + 2 `bios/lineainit.c`) | Abschnitt 4 (dieses Dokument) | **neu, dieser Pass** |
| **Cookie-Jar** | in Util enthalten (`util/cookie.c`, 7 Fn) — `Ssystem` ist **keine EmuTOS-Funktion** (reine FreeMiNT/NewTOS-Erweiterung, s. §4.2) | Abschnitt 4.2 | **neu, dieser Pass** |
| **Summe über alle Subsysteme** | **402 + 269 + 12 + 183 + 103 (real) + 61 = 1030** real-implementierte/enumerierte Symbole (ohne die reinen `xbios_unimpl`-Platzhalter mitzuzählen, die keine Funktionen sind — s. §3.3 für deren Anzahl; die 53 GEMDOS-Public-Opcodes sind in den 183 bereits enthalten, nicht separat addiert) | — | — |

Hinweis zur Summenbildung: AES/VDI zählen **alle** ctags-Symbole (inkl.
`#define`-Makros, Typedef-Tags — konsistent mit deren eigener Methodik).
GEMDOS/BDOS und Util folgen hier derselben Konvention (rohe ctags-Symbolzahl,
inkl. einiger Typedef-/Makro-Zeilen, siehe Fußnoten je Tabelle). XBIOS ist
eine Ausnahme: die 142 Dispatch-Slots enthalten 39 reine
`xbios_unimpl`-Fülleinträge (bei vollem Falcon-Profil mit `CONF_WITH_
TT_SHIFTER` an — in EmuTOS selbst nicht implementiert, kein
Funktionskörper) — diese sind **keine Funktionen** und werden separat
gezählt (§3.3), nicht in die Gesamtsumme integriert, um die Zahl nicht
künstlich aufzublähen.

---

## 2. GEMDOS/BDOS (Trap #1 + BDOS-intern)

`bdos/*.c` (17 Dateien, **183 ctags-Symbole insgesamt** — Tabelle B unten
listet alle 183 einzeln). Zwei Tabellen, **keine Doppelzählung**: Tabelle A
ist eine nach Opcode sortierte *Sicht* auf 53 der 183 Symbole (die
öffentlich per Trap #1 erreichbaren), Tabelle B listet **alle** 183
Symbole je Datei und verweist bei den 53 öffentlichen nur auf Tabelle A
statt sie erneut zu beschreiben.

- **Tabelle A** — die öffentliche Trap-#1-Dispatch-Tabelle
  (`bdos/bdosmain.c: funcs[]`, Zeile 144–283, `MAX_FNCALL = 0x57`):
  53 tatsächlich implementierte Opcodes (EmuTOS lässt mehrere Slots als
  `NI` = "not implemented" → `ni()` liefert `EINVFN`).
- **Tabelle B** — alle BDOS-internen (statischen + globalen)
  Hilfsfunktionen je Datei, analog zum AES/VDI-Per-Datei-Format.
  Public-Dispatch-Ziele werden dort nur mit Verweis auf Tabelle A
  aufgeführt (keine Doppel-Beschreibung).

### 2.1 Tabelle A — GEMDOS-Public-Dispatch (Opcode 0x00–0x57)

Quelle der Opcode-Namen: `include/bdosbind.h` (die `#define Xyz(...) trap1(0xNN,...)`-
Bindungen, wo vorhanden) + NewTOS' eigene, bereits nach Compendium-Konvention
benannte Rust-Funktionen (`tos-api/src/gemdos/mod.rs`) als Kreuzreferenz für
die restlichen Namen (Konsole/Zeit/Prozess-Gruppe, die `bdosbind.h` nicht
bindet, weil EmuTOS selbst sie nur intern über Wrapper-Makros in `gemdos.h`
aufruft). Alle Opcode-Nummern und internen EmuTOS-Funktionsnamen sind aus
`bdos/bdosmain.c:147–283` gelesen (primäre Quelle).

| Opcode | EmuTOS-intern | Public-Name | Kurzrolle | EmuTOS `file:line` | NewTOS-Gegenstück | Status |
|---|---|---|---|---|---|---|
| 0x00 | `x0term` | Pterm0 | Programmende, Exitcode 0 | `bdos/proc.c:588` | `kernel/src/arch/aarch64/exceptions.rs:279-489` (frame-mutierender Sonderfall, ruft `tos_api::bdos::process::pterm::terminate_zero()`) | HAS |
| 0x01 | `xconin` | Cconin | 1 Zeichen von Konsole lesen, blockierend | `bdos/console.c:396` | `tos-api/src/gemdos/console.rs:220` `cconin` | HAS |
| 0x02 | `xconout` | Cconout | 1 Zeichen auf Konsole schreiben | `bdos/console.c:279` | `console.rs:81` `cconout` | HAS |
| 0x03 | `xauxin` | Cauxin | 1 Zeichen vom Aux/seriell lesen | `bdos/console.c:432` | `console.rs:111` `cauxin` | HAS |
| 0x04 | `xauxout` | Cauxout | 1 Zeichen auf Aux/seriell schreiben | `bdos/console.c:331` | `console.rs:126` `cauxout` | HAS |
| 0x05 | `xprtout` | Cprnout | 1 Zeichen auf Drucker-Port | `bdos/console.c:340` | `console.rs:144` `cprnout` | HAS |
| 0x06 | `xrawio` | Crawio | rohes Zeichen-I/O (kein Ctrl-Verarbeiten) | `bdos/console.c:441` | `console.rs:165` `crawio` | HAS |
| 0x07 | `xrawcin` | Crawcin | 1 Zeichen roh lesen (kein Echo/Ctrl-C) | `bdos/console.c:375` | `console.rs:243` `crawcin` | HAS |
| 0x08 | `xnecin` | Cnecin | 1 Zeichen ohne Echo lesen | `bdos/console.c:414` | `console.rs:256` `cnecin` | HAS |
| 0x09 | `xconws` | Cconws | String auf Konsole schreiben | `bdos/console.c:459` | `console.rs:505` `cconws` | HAS |
| 0x0A | `xconrs` | Cconrs | Zeile in Puffer lesen (Line-Editor) | `bdos/console.c:533` | `console.rs:463` `cconrs` | HAS |
| 0x0B | `xconstat` | Cconis | Eingabe-Status Konsole | `bdos/console.c:159` | `console.rs:556` `cconis` | HAS |
| 0x0E | `xsetdrv` | Dsetdrv | aktuelles Laufwerk setzen | `bdos/fsmain.c:420` | `tos-api/src/bdos/fs/drive.rs` `dsetdrv` | HAS |
| 0x10 | `xconostat` | Cconos | Ausgabe-Status Konsole | `bdos/console.c:168` | `console.rs:574` `cconos` | HAS |
| 0x11 | `xprtostat` | Cprnos | Ausgabe-Status Drucker | `bdos/console.c:177` | `console.rs:583` `cprnos` | HAS |
| 0x12 | `xauxistat` | Cauxis | Eingabe-Status Aux | `bdos/console.c:186` | `console.rs:592` `cauxis` | HAS |
| 0x13 | `xauxostat` | Cauxos | Ausgabe-Status Aux | `bdos/console.c:195` | `console.rs:601` `cauxos` | HAS |
| 0x14 | `xmaddalt` | Maddalt | Alt-RAM-Bereich beim Pool registrieren (`CONF_WITH_ALT_RAM`) | `bdos/umem.c:455` | `tos-api/src/gemdos/memory.rs:104` `maddalt` | HAS |
| 0x15 | `srealloc` | Srealloc | Bildschirmspeicher für Auflösungswechsel reservieren (`CONF_WITH_VIDEL`, Falcon-Zielprofil) | `bdos/umem.c:358` | — kein Treffer in `tos-api/src`/`kernel/src` (weder `srealloc` noch `Srealloc`) | **NO-COUNTERPART** |
| 0x19 | `xgetdrv` | Dgetdrv | aktuelles Laufwerk erfragen | `bdos/fsmain.c:438` | `tos-api/src/bdos/fs/drive.rs` `dgetdrv` | HAS |
| 0x1A | `xsetdta` | Fsetdta | DTA-Zeiger setzen | `bdos/fsmain.c:403` | `tos-api/src/bdos/fs/find.rs` `fsetdta` | HAS |
| 0x2A | `xgetdate` | Tgetdate | aktuelles Datum lesen | `bdos/time.c:99` | `tos-api/src/gemdos/time.rs:136` `tgetdate` | HAS |
| 0x2B | `xsetdate` | Tsetdate | Datum setzen | `bdos/time.c:108` | `time.rs:150` `tsetdate` | HAS |
| 0x2C | `xgettime` | Tgettime | aktuelle Uhrzeit lesen | `bdos/time.c:132` | `time.rs:141` `tgettime` | HAS |
| 0x2D | `xsettime` | Tsettime | Uhrzeit setzen | `bdos/time.c:142` | `time.rs:176` `tsettime` | HAS |
| 0x2F | `xgetdta` | Fgetdta | aktuellen DTA-Zeiger erfragen | `bdos/fsmain.c:394` | `tos-api/src/bdos/fs/find.rs` `fgetdta` | HAS |
| 0x30 | `xgetver` | Sversion | GEMDOS-Versionsnummer | `bdos/bdosmain.c:298` | `tos-api/src/gemdos/version.rs` `sversion` | HAS |
| 0x31 | `xtermres` | Ptermres | terminate-and-stay-resident | `bdos/proc.c:624` | `kernel/src/arch/aarch64/exceptions.rs:279-489` (Sonderfall, ruft `terminate_resident`) | HAS |
| 0x36 | `xgetfree` | Dfree | freier/gesamter Plattenplatz + Cluster-Info | `bdos/fsfat.c:337` | `tos-api/src/bdos/fs/drive.rs` `dfree` | HAS |
| 0x39 | `xmkdir` | Dcreate | Verzeichnis anlegen | `bdos/fsdir.c:201` | `tos-api/src/bdos/fs/dirops.rs` `dcreate` | HAS |
| 0x3A | `xrmdir` | Ddelete | leeres Verzeichnis löschen | `bdos/fsdir.c:297` | `dirops.rs` `ddelete` | HAS |
| 0x3B | `xchdir` | Dsetpath | aktuellen Pfad setzen | `bdos/fsdir.c:1113` | `dirops.rs` `dsetpath` | HAS |
| 0x3C | `xcreat` | Fcreate | Datei anlegen/leeren + Attribut | `bdos/fsopnclo.c:81` | `tos-api/src/bdos/fs/file.rs` `fcreate` | HAS |
| 0x3D | `xopen` | Fopen | Datei öffnen | `bdos/fsopnclo.c:221` | `file.rs` `fopen` | HAS |
| 0x3E | `xclose` | Fclose | Datei-Handle schließen | `bdos/fsopnclo.c:426` | `file.rs` `fclose` | HAS |
| 0x3F | `xread` | Fread | aus Datei lesen | `bdos/fsio.c:440` | `file.rs` `fread` | HAS |
| 0x40 | `xwrite` | Fwrite | in Datei schreiben | `bdos/fsio.c:528` | `file.rs` `fwrite` | HAS |
| 0x41 | `xunlink` | Fdelete | Datei löschen | `bdos/fsopnclo.c:623` | `file.rs` `fdelete` | HAS |
| 0x42 | `xlseek` | Fseek | Dateiposition setzen | `bdos/fsio.c:321` | `file.rs` `fseek` | HAS |
| 0x43 | `xchmod` | Fattrib | Dateiattribute lesen/setzen | `bdos/fsdir.c:402` | `file.rs` `fattrib` | HAS |
| 0x44 | `xmxalloc` | Mxalloc | Speicher aus ST-/TT-RAM (Mode-Flag) | `bdos/umem.c:234` | `tos-api/src/gemdos/memory.rs:126` `mxalloc` | HAS |
| 0x45 | `xdup` | Fdup | Dateihandle duplizieren | `bdos/fshand.c:93` | `file.rs` `fdup` | HAS |
| 0x46 | `xforce` | Fforce | Handle-Umleitung | `bdos/fshand.c:32` | `file.rs` `fforce` | HAS |
| 0x47 | `xgetdir` | Dgetpath | aktuellen Pfad eines Laufwerks erfragen | `bdos/fsdir.c:1229` | `dirops.rs` `dgetpath` | HAS |
| 0x48 | `xmalloc` | Malloc | Speicherblock allozieren (ST-RAM-Pool) | `bdos/umem.c:116` | `memory.rs:155` `malloc` | HAS |
| 0x49 | `xmfree` | Mfree | Speicherblock freigeben | `bdos/umem.c:135` | `memory.rs:190` `mfree` | HAS |
| 0x4A | `xsetblk` | Mshrink | Speicherblock verkleinern | `bdos/umem.c:170` | `memory.rs:223` `mshrink` | HAS |
| 0x4B | `xexec` | Pexec | Programm laden + ausführen | `bdos/proc.c:179` | `kernel/src/arch/aarch64/exceptions.rs:719` (frame-mutierender Sonderfall) | HAS |
| 0x4C | `xterm` | Pterm | Programm mit Exitcode beenden | `bdos/proc.c:601` | `exceptions.rs:279-489` (Sonderfall, ruft `terminate(rc)`) | HAS |
| 0x4E | `xsfirst` | Fsfirst | Verzeichnissuche starten | `bdos/fsdir.c:523` | `tos-api/src/bdos/fs/find.rs` `fsfirst` | HAS |
| 0x4F | `xsnext` | Fsnext | Verzeichnissuche fortsetzen | `bdos/fsdir.c:654` | `find.rs` `fsnext` | HAS |
| 0x56 | `xrename` | Frename | Datei/Verzeichnis umbenennen | `bdos/fsdir.c:895` | `file.rs` `frename` | HAS |
| 0x57 | `xgsdtof` | Fdatime | Datei-Datum/-Zeit lesen/setzen | `bdos/fsdir.c:684` | `file.rs` `fdatime` | HAS |

**Nicht implementierte Slots in EmuTOS selbst** (Dispatch-Tabelle liefert
`ni()` → `EINVFN`, kein Funktionskörper, daher keine Zeile oben): 0x0C,
0x0D, 0x0F, 0x16, 0x17, 0x18, 0x1B–0x29 (10 Slots, Kommentar `/* xgsps */`
— GSX/Parameterblock-Platzhalter, nie belegt), 0x2E, 0x32–0x35, 0x37,
0x38, 0x3E ist belegt (s.o.), 0x4D, 0x50–0x55. **Opcode 0x20 ist einer
dieser `NI`-Slots** — EmuTOS implementiert dort **keine** Funktion; ob
ein extern dokumentiertes „Super()" auf diesen Opcode abzielt, ist mit
dieser Quelle (`bdos/bdosmain.c`, `include/bdosbind.h`, `grep -rn Super
bdos/`) **nicht verifizierbar** (kein einziger Treffer für „Super" im
gesamten `bdos/`-Baum) — als offener Punkt markiert, kein Name erfunden.

**Befund:** von 53 in EmuTOS implementierten Public-Opcodes hat NewTOS
**52 HAS**, **1 NO-COUNTERPART** (0x15 `Srealloc`, Falcon/VIDEL-
Bildschirmspeicher-Realloc — kein `srealloc`/`Srealloc`-Treffer in
`tos-api/src` oder `kernel/src`). Pexec/Pterm/Pterm0/Ptermres werden
nicht in `tos-api/src/gemdos/mod.rs`s Tabelle dispatcht, sondern
frame-mutierend im Kernel-Exception-Handler behandelt — verifiziert
via `kernel/src/arch/aarch64/exceptions.rs:279-489,719`.

### 2.2 Tabelle B — BDOS-interne Funktionen je Datei

Public-Dispatch-Ziele (bereits in Tabelle A) werden hier nur mit Verweis
aufgeführt, nicht erneut beschrieben. Status-Klassifikation: präziser
`fn <name>(`-Treffer in `tos-api/src`/`kernel/src` → HAS mit Fundstelle;
kein Treffer → **„?"** (nicht geraten — die BDOS-Interna sind EmuTOS-
eigene Implementierungsdetails, ein Fehlen des exakten Namens bedeutet
nicht zwingend Funktionslücke, s. Prime-Directive „flag don't fabricate").

#### bdosmain.c (11 Symbole)

Boot-Init (`osinit_before/after_xmaddalt`) + der zentrale Trap-#1-
Dispatcher (`osif`, `funcs[]`-Tabelle = Tabelle A) + DND-Baum-/BCB-
Aufräumroutinen für Medienwechsel.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `F` (Makro) | 144 | `#define F(x) { (PFLONG)(x) }` — Tabellen-Eintrag-Helfer | — | N/A (Makro) |
| `FND` | 133 | Typedef: ein `funcs[]`-Zeileneintrag | — | N/A (Typ) |
| `freetree` | 356 | gibt DND-Verzeichnisbaum rekursiv frei | ? | ? |
| `mark_bcbs_invalid` | 411 | invalidiert Buffer-Cache-Einträge eines Laufwerks (Medienwechsel) | ? | ? |
| `ni` | 307 | Dispatch-Stub für nicht implementierte Opcodes → `EINVFN` | `tos-api/src/gemdos/mod.rs` (implizit: `None`-Slot → `EINVFN`) | HAS (äquivalentes Verhalten, kein 1:1-Namensgegenstück) |
| `offree` | 387 | schließt alle offenen Handles eines DMD (Medienwechsel) | ? | ? |
| `osif` | 431 | C-Implementierung von Trap #1, zentraler Dispatcher | `tos-api/src/gemdos/mod.rs:dispatch()` | HAS (funktional, anderer Name/Aufbau) |
| `osinit_after_xmaddalt` | 336 | BDOS-Boot-Init Teil 2 (Prozess/Zeit/Handles) | ? | ? |
| `osinit_before_xmaddalt` | 317 | BDOS-Boot-Init Teil 1 (Trap-Vektoren, Puffer, Pools) | ? | ? |
| `SPECNAME` | 93 | Typedef | — | N/A (Typ) |
| `xgetver` | 298 | → Tabelle A, Opcode 0x30 | s.o. | HAS |

#### console.c (31 Symbole)

„Cooked"-Konsolenschicht oberhalb des rohen BIOS-`Bconin`/`Bconout`
(Tab-Expansion, Zeilenzähler, Ctrl-C/Ctrl-S-Erkennung, Line-Editor `cgets`).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `backsp` | 494 | Backspace-Verarbeitung im Line-Editor | ? | ? |
| `buflush` | 252 | Typeahead-Puffer leeren | ? | ? |
| `cgets` | 540 | vollständiger Line-Editor (liest 1 Zeile inkl. Editing) | `console.rs:380` `edit_line` (eigene generische `LineEditIo`-Abstraktion statt 1:1-Port) | HAS |
| `conbrk` | 206 | Ctrl-C/Ctrl-S-Erkennung während Konsolen-I/O | `console.rs:54` `output_break` (Ctrl-C-Teil; deferred-terminate-Mechanismus s. `gemdos/mod.rs`-Kommentar) | HAS |
| `conin` | 384 | interner Zeichen-Lese-Dispatch (CON/AUX/PRN je Handle) | — implizit über die separaten `cconin`/`cauxin`-Funktionen, kein gemeinsamer Kern | ? |
| `conout` | 263 | interner Zeichen-Schreib-Dispatch + Tab-Expansion | ? | ? |
| `constat` | 143 | internes Status-Polling (Eingabe bereit?) | ? | ? |
| `cookdout` | 312 | „cooked" Ausgabe: Tab-Expansion, Zeilenzähler | ? | ? |
| `get_default_handle` | 132 | Standard-Handle für Redirection ermitteln | ? | ? |
| `getch` | 351 | 1 Zeichen inkl. Sondertasten-Dekodierung lesen | `console.rs`-Trait `LineEditIo::read_byte` (325) | HAS (andere Abstraktion) |
| `newline` | 480 | Cursor-Zeilenumbruch + Zeilenzähler | — `kernel/src/fbcon.rs:91` hat eine gleichnamige Funktion, aber auf der physischen Framebuffer-Cursor-Ebene (VT52/BIOS), nicht auf der GEMDOS-„cooked"-Ebene — **Namenskoinzidenz, kein verifizierter 1:1-Bezug** | ? |
| `prt_line` | 468 | eine komplette Zeile auf Drucker ausgeben | ? | ? |
| `stdhdl_init` | 107 | initialisiert Standard-Handles (0–4) beim Boot | ? | ? |
| `tabout` | 292 | Tab-Zeichen-Expansion für Konsolenausgabe | ? | ? |
| `terminate` (Makro) | 101 | `#define terminate() xterm(-32)` — Ctrl-C-Abbruch | konzeptionell: `tos-api/src/gemdos/mod.rs`-Kommentar „deferred Ctrl-C `terminate()`" + `exceptions.rs` Pterm-Pfad | HAS (anderer Mechanismus: deferred statt sofortiger `xterm`-Aufruf) |
| `TYPEAHEAD` | 34 | Typedef (Tastatur-Puffer-Struktur) | — | N/A (Typ) |
| `xauxin`…`xconws` (13 Fn) | diverse | → Tabelle A, Opcodes 0x01–0x13 | s.o. | HAS |

#### fsbuf.c (5 Symbole)

Plattensektor-Puffer-Cache (Buffer Cache) für Daten- und FAT-Sektoren.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `bufl_init` | 56 | initialisiert die Puffer-Ketten (Daten + FAT) | `tos-api/src/bdos/fs/cache.rs` (Modul-Doku: „Mirrors EmuTOS `bdos/fsbuf.c`") — Init erfolgt anders (statisches Array `SLOTS: usize = 16`), kein 1:1-Fn-Name | HAS (Datei-Ebene, anderer Aufbau) |
| `create_chain` | 29 | baut verkettete BCB-Liste aus RAM-Block | ? | ? |
| `flush` | 85 | schreibt einen „dirty" Puffer zurück auf Platte | `fs/cache.rs:122` `pub fn flush()` — Moduldoku zitiert explizit `bdos/fsbuf.c` | HAS |
| `getbcb` | 114 | sucht/belegt Cache-Eintrag für einen Sektor | ? (Cache-Slot-Lookup vermutlich in `fs/cache.rs`, kein exakter Namenstreffer) | ? |
| `getrec` | 209 | liest einen logischen Record über den Cache | ? | ? |

#### fsdir.c (37 Symbole, größte BDOS-Datei)

Directory-Node-Cache (DND-Baum), FAT-Directory-Scan, Pfad-Parsing,
Fsfirst/Fsnext-Kern.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `builds` | 737 | baut vollen Pfadstring aus DND-Elternkette | ? | ? |
| `contains_wildcard_characters` | 505 | prüft Name auf `*`/`?` | ? | ? |
| `dcrack` | 1695 | „directory crack": zerlegt Pfad, löst DND-Kette auf | ? | ? |
| `decr_curdir_usage` | 1194 | Referenzzählung aktuelles Verzeichnis: Dekrement | ? | ? |
| `dirinit` | 1257 | initialisiert FCB-Iteration für ein Directory | ? | ? |
| `dirscan` (Makro) | 161 | `#define dirscan(a,c) ((DND*) scan(a,c,FA_SUBDIR,...))` | — | N/A (Makro) |
| `dopath` | 1365 | baut Pfadstring rekursiv aus DND-Elternkette | ? | ? |
| `findit` | 1407 | Kernfunktion: Pfad → DND + Restname (von vielen X-Fn genutzt) | `tos-api/src/bdos/fs/path.rs:132` `pub fn findit(path: &str) -> Result<(Volume, u32, [u8;11]), Long>` | HAS |
| `free_available_dnds` | 2053 | gibt ungenutzte DND-Cache-Einträge frei | ? | ? |
| `freednd` | 1900 | ein einzelnes DND freigeben | ? | ? |
| `getdnd` | 1869 | DND aus Name+Parent holen/erzeugen | ? | ? |
| `getpath` | 1761 | liefert aktuellen Pfadstring eines Laufwerks | ? | ? |
| `incr_curdir_usage` | 1164 | Referenzzählung aktuelles Verzeichnis: Inkrement | ? | ? |
| `is_subdir` | 853 | prüft Unterverzeichnis-Beziehung zweier DNDs | ? | ? |
| `isnotdelim` (Makro) | 713 | Zeichenklassifikation für Pfad-Parser | — | N/A (Makro) |
| `ixsfirst` | 452 | interner Fsfirst-Kern | `find.rs` (Fsfirst-Implementierung, kein exakter `ixsfirst`-Name) | ? |
| `ixsnext` | 545 | interner Fsnext-Kern (nächster FCB via Iterator) | ? | ? |
| `makbuf` | 1851 | füllt DTA mit gefundenem Directory-Eintrag | `fs/find.rs:319` `unsafe fn makbuf(dta: u64, e: &DirEntry)` | HAS |
| `makdnd` | 1610 | erzeugt neuen DND aus FCB | ? | ? |
| `makofd` | 1920 | erzeugt OFD für ein Directory | ? | ? |
| `match` | 1802 | Wildcard-Namensvergleich (8.3-Muster) | ? (Rust `match`-Keyword kollidiert, kein sinnvoller Namensgrep möglich) | ? |
| `namlen` | 178 | Namenslänge bis Trenner | ? | ? |
| `packit` | 1297 | packt 8.3-Namen in internes FCB-Format | ? | ? |
| `process_dnd_tree` | 1957 | traversiert DND-Baum (für `free_available_dnds`) | ? | ? |
| `scan` | 1525 | durchsucht ein Directory nach Name/Attribut (FAT-Scan) | `fs/find.rs:142` `fn scan(...)` | HAS |
| `snipdnd` | 1886 | entfernt DND aus verketteter Liste | ? | ? |
| `unpackit` | 1334 | entpackt 8.3-Namen aus FCB-Format | ? | ? |
| `update_fcb` | 871 | aktualisiert FCB nach Schreiboperation (Größe/Zeit) | ? | ? |
| `xchdir`,`xchmod`,`xgetdir`,`xgsdtof`,`xmkdir`,`xrename`,`xrmdir`,`xsfirst`,`xsnext` (9 Fn) | diverse | → Tabelle A | s.o. | HAS |

#### fsdrive.c (4 Symbole)

Laufwerks-/Medien-Verwaltung (Drive Media Descriptor, DMD).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `ckdrv` | 77 | prüft Laufwerksnummer gültig + optionaler Medienwechsel-Check | ? | ? |
| `getdmd` | 140 | liefert DMD für Laufwerksnummer | `tos-api/src/bdos/fs/drive.rs` (Drive-Struktur-Lookup, kein exakter `getdmd`-Name) | ? |
| `log_media` | 184 | „logt" ein Medium ein (BPB einlesen, DMD aufbauen) | ? | ? |
| `log2ul` | 164 | log2 einer Zahl (Cluster-Größen-Berechnung) | ? | ? |

#### fsfat.c (9 Symbole)

FAT-Kettenverwaltung (Cluster-Allokation/-Verkettung).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `cl2rec` | 23 | Cluster-Nummer → Sektor/Record-Nummer | `tos-api/src/bdos/fs/fat.rs` (18 Fn, FAT-Modul vorhanden — kein exakter `cl2rec`-Name geprüft) | ? |
| `clfix` | 36 | FAT-Eintrag setzen (Cluster-Verkettung) | ? | ? |
| `countfree16` | 299 | freie Cluster zählen (FAT16) | ? | ? |
| `findfree` | 208 | nächsten freien Cluster suchen (generisch) | ? | ? |
| `findfree16` | 174 | nächsten freien Cluster suchen (FAT16) | ? | ? |
| `getclnum` | 160 | n-ten Cluster einer Kette ermitteln | ? | ? |
| `getrealcl` | 107 | physischen Cluster-Wert übersetzen | ? | ? |
| `nextcl` | 247 | nächsten Cluster in Kette holen/anhängen (Schreiben) | ? | ? |
| `xgetfree` | 337 | → Tabelle A, Opcode 0x36 | s.o. | HAS |

#### fsglob.c (0 Symbole)

Nur globale Variablen (`drvtbl`, `sft`, `rwerr`, `errcode`, `errdrv`,
`errbuf`) — keine Funktionen, daher keine Tabelle.

#### fshand.c (3 Symbole)

Handle-Verwaltung (Duplizieren/Umleiten).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `ixforce` | 46 | interner Kern für Handle-Redirection | ? | ? |
| `xdup` | 93 | → Tabelle A, Opcode 0x45 | s.o. | HAS |
| `xforce` | 32 | → Tabelle A, Opcode 0x46 | s.o. | HAS |

#### fsio.c (12 Symbole)

Datei-I/O auf Record-/Byte-Ebene.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `addit` | 33 | Dateigröße nach Schreiben aktualisieren | ? | ? |
| `eof` | 294 | EOF-Test für Handle | ? | ? |
| `ixgetfcb` | 462 | FCB aus OFD holen | ? | ? |
| `ixlseek` | 354 | interner Seek-Kern | ? | ? |
| `ixread` | 498 | interner Read-Kern (Byte-Ebene über Records) | ? | ? |
| `ixwrite` | 554 | interner Write-Kern | ? | ? |
| `usrio` | 57 | Low-Level-Record-I/O Usermem ↔ Datenträger | ? | ? |
| `xlseek` | 321 | → Tabelle A, Opcode 0x42 | s.o. | HAS |
| `xread` | 440 | → Tabelle A, Opcode 0x3F | s.o. | HAS |
| `xrw` | 208 | gemeinsamer Read/Write-Kern (Record-Grenzen, Teil-Records) | ? | ? |
| `xrw_recs` | 84 | liest/schreibt Folge ganzer Records | ? | ? |
| `xwrite` | 528 | → Tabelle A, Opcode 0x40 | s.o. | HAS |

#### fsmain.c (6 Symbole)

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `getofd` | 474 | OFD aus Handle auflösen (Bounds-Check) | ? | ? |
| `syshnd` | 457 | prüft auf System-Handle (redirect-fähig) | ? | ? |
| `xgetdrv` | 438 | → Tabelle A, Opcode 0x19 | s.o. | HAS |
| `xgetdta` | 394 | → Tabelle A, Opcode 0x2F | s.o. | HAS |
| `xsetdrv` | 420 | → Tabelle A, Opcode 0x0E | s.o. | HAS |
| `xsetdta` | 403 | → Tabelle A, Opcode 0x1A | s.o. | HAS |

#### fsopnclo.c (14 Symbole)

Open/Close/Create/Delete + System File Table (SFT).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `contains_illegal_characters` | 708 | Namensvalidierung (verbotene Zeichen) | `tos-api/src/bdos/fs/path.rs:167` `pub fn contains_illegal_characters(name: &str) -> bool` | HAS |
| `ixclose` | 511 | interner Close-Kern | ? | ? |
| `ixcreat` | 99 | interner Create-Kern | ? | ? |
| `ixdel` | 652 | interner Delete-Kern (FAT-Ketten + Directory-Eintrag löschen) | ? | ? |
| `ixopen` | 239 | interner Open-Kern | ? | ? |
| `ixunlink` | 588 | interner Unlink-Kern (dispatcht an `ixdel`) | ? | ? |
| `makopn` | 271 | OFD/SFT-Eintrag für neu geöffnete Datei anlegen | ? | ? |
| `opnfil` | 335 | Kern-Open-Logik (Suchen/Anlegen je Modus) | ? | ? |
| `sftdel` | 380 | SFT-Eintrag freigeben | ? | ? |
| `sftofdsrch` | 362 | SFT-Eintrag zu einem OFD suchen | ? | ? |
| `xclose` | 426 | → Tabelle A, Opcode 0x3E | s.o. | HAS |
| `xcreat` | 81 | → Tabelle A, Opcode 0x3C | s.o. | HAS |
| `xopen` | 221 | → Tabelle A, Opcode 0x3D | s.o. | HAS |
| `xunlink` | 623 | → Tabelle A, Opcode 0x41 | s.o. | HAS |

#### iumem.c (3 Symbole)

BDOS-interner Memory-Pool-Allocator (First-Fit auf MD/MPB-Ebene, von
Osmem **und** Umem gemeinsam genutzt).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `ffit` | 35 | First-Fit-Suche im Memory-Pool | `tos-api/src/bdos/mem/pool.rs:59` `pub unsafe fn ffit(...)` | HAS |
| `freeit` | 144 | Speicherblock freigeben + Nachbarn verschmelzen | `mem/pool.rs:187` `pub unsafe fn freeit(...)` | HAS |
| `shrinkit` | 228 | Speicherblock verkleinern (Rest freigeben) | `mem/pool.rs:294` `pub unsafe fn shrinkit(...)` | HAS |

#### kpgmld.c (5 Symbole)

Kernel-interner Programmlader/Relozierer (u.a. für residente Programme).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `kpgm_relocate` | 279 | führt die Relozierungstabelle eines Programms aus | ? (`tos-api/src/bdos/process/loader.rs`/`elf.rs` — anderes Lade-Format, ELF statt TOS-PGMHDR01-Relokation) | ? |
| `kpgmhdrld` | 40 | liest Programm-Header (`PGMHDR01`) von Datei | ? | ? |
| `kpgmld` | 79 | lädt+reloziert ein Programm in eine PD | `tos-api/src/bdos/process/loader.rs` (ELF-Loader, funktional analog, anderes Binärformat) | HAS (Funktionsebene, anderes Format) |
| `pgfix01` | 243 | wendet einen Relozierungs-Fixup an | ? | ? |
| `pgmld01` | 107 | Lade-Kern für Format-01-Programme | ? | ? |

#### osmem.c (10 Symbole)

BDOS-interner (Kernel-eigener) Memory-Pool, getrennt vom User-`Malloc`/`Mfree`.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `_mdb` / `MDBLOCK` / `MDEXT` | 53/52/50 | Typdefinitionen (Memory-Descriptor-Block) | `tos-api/src/bdos/mem/mdpool.rs` (`MdBlock`-Typ vorhanden) | N/A (Typ) / HAS (Konzept) |
| `getosm` | 100 | Hilfsroutine für `osmem_init` | ? | ? |
| `osmem_init` | 390 | initialisiert den BDOS-internen Speicherpool | ? | ? |
| `unlink_mdblock` | 123 | entfernt einen MD-Block aus der Verwaltung | `mem/mdpool.rs:392` `fn unlink_mdblock(&mut self, mdb: *mut MdBlock) -> bool` | HAS |
| `xmfreblk` | 360 | gibt einen internen Speicherblock frei | ? | ? |
| `xmfremd` | 229 | gibt einen MD in den Pool zurück | `mem/mdpool.rs:224` `unsafe fn xmfremd(...)` (Trait-Methode) | HAS |
| `xmgetblk` | 291 | alloziert einen internen Speicherblock | ? | ? |
| `xmgetmd` | 169 | holt einen freien MD aus dem Pool | `mem/mdpool.rs:219` `unsafe fn xmgetmd(...)` (Trait-Methode) | HAS |

#### proc.c (14 Symbole)

Prozessverwaltung (Pexec/Pterm-Interna).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `alloc_env` | 413 | dupliziert Environment-Strings für Kindprogramm | `tos-api/src/bdos/process/loader.rs:689` `unsafe fn alloc_env(...)` | HAS |
| `alloc_tpa` | 460 | alloziert TPA (Transient Program Area) | `tos-api/src/bdos/process/arena.rs:104` `pub unsafe fn alloc_tpa() -> Option<Tpa>` | HAS |
| `envsize` | 150 | Größe eines Environment-Blocks ermitteln | ? | ? |
| `free_all_owned` | 92 | gibt alle einem Prozess gehörenden Blöcke frei (bei Pterm) | `tos-api/src/bdos/mem/free_all_owned.rs:55` `pub unsafe fn free_all_owned(...)` | HAS |
| `gouser_stack` | 528 | Typdefinition (Stack-Layout für `proc_go`) | — | N/A (Typ, zudem m68k-spezifisch) |
| `init_pd_fields` | 364 | initialisiert Basepage-Felder einer neuen PD | ? | ? |
| `init_pd_files` | 387 | initialisiert Datei-Handle-Tabelle einer neuen PD | ? | ? |
| `ixterm` | 110 | interner Terminate-Kern | `tos-api/src/bdos/process/pterm.rs:111` `pub unsafe fn terminate(...)` | HAS (funktional; EmuTOS-`terminate`-Makro s. console.c ist ein anderer, engerer Ctrl-C-Zweig) |
| `proc_go` | 536 | springt in Usercode des geladenen Programms | — m68k-spezifisch (Register-Setup); NewTOS macht das AArch64-Äquivalent im ELF-Loader/EL0-Sprung | N/A (arch.-spezifisch) |
| `reserve_blocks` | 77 | reserviert Speicherblöcke für Ptermres | ? | ? |
| `x0term` | 588 | → Tabelle A, Opcode 0x00 | s.o. | HAS |
| `xexec` | 179 | → Tabelle A, Opcode 0x4B | s.o. | HAS |
| `xterm` | 601 | → Tabelle A, Opcode 0x4C | s.o. | HAS |
| `xtermres` | 624 | → Tabelle A, Opcode 0x31 | s.o. | HAS |

#### time.c (7 Symbole)

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `IS_A_LEAP_YEAR` (Makro) | 74 | Schaltjahrestest (gepacktes Datumsformat) | `tos-api/src/gemdos/time.rs:49` `const fn is_leap(year: u32) -> bool` | HAS (anderer Name/Format, gleiche Semantik) |
| `tikfrk` | 175 | Tick-Fork-Callback (zählt Sekunden hoch) | `time.rs:38` `fn monotonic()`/`now_secs()` — anderer Mechanismus (Host-Uhr statt Tick-Zähler) | HAS (Funktionsebene) |
| `time_init` | 160 | initialisiert BDOS-Uhrzeit beim Boot | ? | ? |
| `xgetdate` | 99 | → Tabelle A, Opcode 0x2A | s.o. | HAS |
| `xgettime` | 132 | → Tabelle A, Opcode 0x2C | s.o. | HAS |
| `xsetdate` | 108 | → Tabelle A, Opcode 0x2B | s.o. | HAS |
| `xsettime` | 142 | → Tabelle A, Opcode 0x2D | s.o. | HAS |

#### umem.c (12 Symbole)

User-Memory (Malloc/Mfree/Mxalloc/Mshrink), MPB-Verwaltung.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `dump_md_list` | 54 | Debug-Ausgabe (KDEBUG) der MD-Liste | ? | N/A (reine Debug-Diagnose) |
| `dump_mem_map` | 75 | Debug-Ausgabe (KDEBUG) der Speicherkarte | ? | N/A (reine Debug-Diagnose) |
| `find_mpb` | 99 | ermittelt MPB (ST-/TT-RAM-Pool) für eine Adresse | `tos-api/src/bdos/mem/traps.rs:394` `unsafe fn find_mpb(addr: Long) -> *mut MPB` | HAS |
| `set_owner` | 587 | setzt Eigentümer-PD eines Speicherblocks | `mem/traps.rs:417` `pub unsafe fn set_owner(addr: Long, owner: *mut c_void) -> bool` | HAS |
| `srealloc` | 358 | → Tabelle A, Opcode 0x15 | s.o. | **NO-COUNTERPART** |
| `total_alt_ram` | 520 | Summe des registrierten Alt-RAM | ? | ? |
| `umem_init` | 549 | initialisiert User-Memory-Pools beim Boot | ? | ? |
| `xmaddalt` | 455 | → Tabelle A, Opcode 0x14 | s.o. | HAS |
| `xmalloc` | 116 | → Tabelle A, Opcode 0x48 | s.o. | HAS |
| `xmfree` | 135 | → Tabelle A, Opcode 0x49 | s.o. | HAS |
| `xmxalloc` | 234 | → Tabelle A, Opcode 0x44 | s.o. | HAS |
| `xsetblk` | 170 | → Tabelle A, Opcode 0x4A | s.o. | HAS |

---

## 3. XBIOS (Trap #14)

Quelle: `bios/xbios.c` — die Dispatch-Tabelle `xbios_vecs[]`
(`bios/xbios.c:1284–1515`, 142 Opcode-Slots `0x00`–`0x8D`, Größe abhängig
von `LAST_ENTRY`, das wiederum von `CONF_WITH_DMASOUND`/`_DSP`/`_VIDEL`/
`_TT_SHIFTER` abhängt — Zielprofil Falcon030/TOS4 hätte `CONF_WITH_DMASOUND`
und `CONF_WITH_VIDEL` an, `CONF_WITH_TT_SHIFTER` ist TT-spezifisch und
für Falcon **nicht** gesetzt — dieser Punkt ist hier nicht abschließend
gegen den tatsächlichen `make 512`-Build verifiziert, s. offene Punkte).

**Wichtiger Aufbau-Befund:** die `xbios_N`-Wrapper-Funktionen *in*
`bios/xbios.c` existieren nur, wenn `DBG_XBIOS=1` (Zeile 40: `#define
DBG_XBIOS 0` — **Default aus**). Im Standard-Build zeigt jeder
`xbios_vecs[]`-Slot **direkt** auf die reale Implementierungsfunktion in
einer anderen Datei (Makro `VEC(wrapper, direct)`, `bios/xbios.c:1268`).
Diese Tabelle listet daher die **realen** Ziel-Funktionen (Spalte
„EmuTOS `file:line`" zeigt auf die tatsächliche Implementierung, nicht
den toten `xbios_N`-Debug-Wrapper).

### 3.1 XBIOS-Dispatch-Tabelle (real implementierte Opcodes)

NewTOS-Status aus `tos-api/src/xbios/mod.rs:dispatch()` (vollständig
gelesen) — Opcodes, die dort auf den generischen Catch-all
(„Funktionsnummer zurückgeben") fallen, sind **NO-COUNTERPART**.

| Opcode | Name | Kurzrolle | Gate (`CONF_WITH_*`) | EmuTOS `file:line` | NewTOS-Gegenstück | Status |
|---|---|---|---|---|---|---|
| 0x00 | `Initmous` | Maus-Paket-Handler installieren | — | `bios/mouse.c:40` | `xbios/mod.rs` — kommentiert als bewusst nicht verdrahtet („needs mouse infra") | **NO-COUNTERPART** |
| 0x02 | `Physbase` | physische Bildschirm-Basisadresse | — | `bios/screen.c:1042` | `tos-api/src/xbios/screen.rs` `physbase()` | HAS |
| 0x03 | `Logbase` | logische Bildschirm-Basisadresse | — | `bios/screen.c:1071` | `screen.rs` `logbase()` | HAS |
| 0x04 | `Getrez` | aktuelle Auflösung (ST-kompatibler Rez-Code) | — | `bios/screen.c:1076` | `screen.rs` `getrez()` | HAS |
| 0x05 | `Setscreen` | Bildschirmadressen/Auflösung setzen | — | `bios/screen.c:1102` | `screen.rs` `setscreen()` | HAS |
| 0x06 | `Setpalette` | komplette Farbpalette setzen | — | `bios/screen.c:1168` | `screen.rs` `setpalette()` | HAS |
| 0x07 | `Setcolor` | einzelne ST(e)-Palettenfarbe setzen | — | `bios/screen.c:1192` | `screen.rs` `setcolor()` | HAS |
| 0x08 | `Floprd` | Floppy-Sektoren lesen | — | `bios/floppy.c:742` | `xbios/mod.rs` → `EUNDEV` (dokumentierter Kontraktwert „kein Laufwerk", kein generisches Funktionsnummer-Echo) | HAS (Kontrakt-Ebene; kein WD1772) |
| 0x09 | `Flopwr` | Floppy-Sektoren schreiben | — | `bios/floppy.c:749` | `mod.rs` → `EUNDEV` | HAS (Kontrakt-Ebene) |
| 0x0A | `Flopfmt` | Floppy-Spur formatieren | — | `bios/floppy.c:898` | `mod.rs` → `EUNDEV` | HAS (Kontrakt-Ebene) |
| 0x0C | `Midiws` | MIDI-Bytes schreiben | — | `bios/midi.c:104` | `xbios/system.rs` `midiws()` — dokumentierter No-op (keine MIDI-ACIA) | HAS |
| 0x0D | `Mfpint` | MFP-Interrupt-Handler installieren | `CONF_WITH_MFP` | `bios/mfp.c:114` | — kein Treffer | **NO-COUNTERPART** |
| 0x0E | `Iorec` | Zeiger auf IOREC-Struktur eines Geräts | — | `bios/xbios.c:336` (real, nicht ausgelagert) | `mod.rs`-Kommentar: bewusst nicht verdrahtet (fehlende Keyboard/Maus-Infra) | **NO-COUNTERPART** |
| 0x0F | `Rsconf` | serielle Schnittstelle konfigurieren | — | `bios/xbios.c:376` (real) | `xbios/system.rs` `rsconf()` — dünner PL011-Adapter | HAS |
| 0x10 | `Keytbl` | Tastatur-Tabellen (norm/shift/caps) setzen | — | `bios/ikbd.c:122` | `mod.rs`-Kommentar: bewusst nicht verdrahtet | **NO-COUNTERPART** |
| 0x11 | `Random` | Zufallszahl | — | `bios/xbios.c:421` (real) | `xbios/system.rs` `random()` | HAS |
| 0x12 | `Protobt` | Bootsektor-Prototyp bauen (BPB → Sektor) | — | `bios/floppy.c:690` | `xbios/floppy.rs` `protobt()` — reine Software, kein Hardwarebezug | HAS |
| 0x13 | `Flopver` | Diskette verifizieren (Sektoren lesen+vergleichen) | — | `bios/floppy.c:797` | `mod.rs` → `EUNDEV` | HAS (Kontrakt-Ebene) |
| 0x14 | `Scrdmp` | Bildschirm auf Drucker ausgeben | — | `bios/xbios.c:490` (real) | `xbios/system.rs` `scrdmp()` — dokumentierter No-op (kein Drucker) | HAS |
| 0x15 | `Cursconf` | Cursor-Konfiguration (Blinkrate etc.) | — | `bios/vt52.c:893` | `xbios/screen.rs` `cursconf()` | HAS |
| 0x16 | `Settime` | gepackte Systemzeit setzen | — | `bios/clock.c:1210` | `xbios/mod.rs` → `gemdos::time::xbios_settime()` (teilt sich die GEMDOS-Walluhr) | HAS |
| 0x17 | `Gettime` | gepackte Systemzeit lesen | — | `bios/clock.c:1258` | `mod.rs` → `gemdos::time::xbios_gettime()` | HAS |
| 0x18 | `Bioskeys` | Tastatur-Vektoren (re-)installieren | — | `bios/ikbd.c:136` | `mod.rs`-Kommentar: bewusst nicht verdrahtet | **NO-COUNTERPART** |
| 0x19 | `Ikbdws` | IKBD-Kommando schreiben | — | `bios/ikbd.c:948` | `xbios/system.rs` `ikbdws()` — dokumentierter No-op (keine ACIA) | HAS |
| 0x1A | `Jdisint` | MFP-Interrupt deaktivieren | `CONF_WITH_MFP` | `bios/mfp.c:122` | — kein Treffer | **NO-COUNTERPART** |
| 0x1B | `Jenabint` | MFP-Interrupt aktivieren | `CONF_WITH_MFP` | `bios/mfp.c:127` | — kein Treffer | **NO-COUNTERPART** |
| 0x1C | `Giaccess` | YM2149-PSG-Register lesen/schreiben | — | `bios/sound.c:94` | `xbios/system.rs` `giaccess()` — Kontraktwert 0 (kein PSG) | HAS |
| 0x1D | `Offgibit` | PSG-Port-Bit löschen | — | `bios/sound.c:130` | `system.rs` `offgibit()` | HAS |
| 0x1E | `Ongibit` | PSG-Port-Bit setzen | — | `bios/sound.c:115` | `system.rs` `ongibit()` | HAS |
| 0x1F | `Xbtimer` | Timer-Vektor via XBIOS setzen | `CONF_WITH_MFP` | `bios/mfp.c:163` | — kein Treffer | **NO-COUNTERPART** |
| 0x20 | `Dosound` | PSG-Notenfolge abspielen | — | `bios/sound.c:145` | `system.rs` `dosound()` | HAS |
| 0x21 | `Setprt` | Drucker-Konfigurationswort | `CONF_WITH_PRINTER_PORT` | `bios/parport.c:49` | `system.rs` `setprt()` — reine Software (Desktop-Konfig-Wort) | HAS |
| 0x22 | `Kbdvbase` | Zeiger auf KEYTAB-Struktur | — | `bios/xbios.c:716` (real) | `system.rs` `kbdvbase()` | HAS |
| 0x23 | `Kbrate` | Tastatur-Wiederholrate get/set | — | `bios/ikbd.c:439` | `system.rs` `kbrate()` | HAS |
| 0x25 | `Vsync` | auf vertikalen Bildwechsel warten | — | `bios/screen.c:1205` | `system.rs` `vsync()` | HAS |
| 0x26 | `Supexec` | Funktion in Supervisor-Modus ausführen | — | `bios/vectors.S:1376` (Assembler, `_supexec`) | — kein Treffer (m68k-Supervisor-Konzept, AArch64-EL-Modell anders) | **NO-COUNTERPART** |
| 0x29 | `Floprate` | Floppy-Zugriffsgeschwindigkeit setzen | — | `bios/floppy.c:1039` | `mod.rs` → `EUNDEV` | HAS (Kontrakt-Ebene) |
| 0x2A | `DMAread` | ACSI-DMA-Sektoren lesen | — | `bios/disk.c:1241` | `mod.rs` → `EUNDEV` | HAS (Kontrakt-Ebene) |
| 0x2B | `DMAwrite` | ACSI-DMA-Sektoren schreiben | — | `bios/disk.c:1254` | `mod.rs` → `EUNDEV` | HAS (Kontrakt-Ebene) |
| 0x2C | `Bconmap` | Bconxxx-Handle auf physisches Gerät umbiegen | — | `bios/serport.c:1082` | `xbios/bconmap.rs` `bconmap()` — Tabelle mit einem Eintrag wie EmuTOS auf einer Maschine ohne Falcon/TT (XBIOS05; vorher Kontraktwert 0x2C) | HAS |
| 0x2E | `Nvmaccess` | NVRAM lesen/schreiben/rücksetzen | `CONF_WITH_NVRAM` | `bios/nvram.c:155` | — kein Treffer | **NO-COUNTERPART** |
| 0x40 | `Blitmode` | BLiTTER-Modus abfragen/setzen | — | `bios/xbios.c:886` (real) | `system.rs` `blitmode()` — Kontraktwert 0x0000 (kein BLiTTER) | HAS |
| 0x50 | `Esetshift` | TT-Shifter-Modus setzen | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:180` | — kein Treffer | **NO-COUNTERPART** |
| 0x51 | `Egetshift` | TT-Shifter-Modus lesen | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:210` | — kein Treffer | **NO-COUNTERPART** |
| 0x52 | `Esetbank` | TT-Farbbank get/set | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:222` | — kein Treffer | **NO-COUNTERPART** |
| 0x53 | `Esetcolor` | TT-Palettenfarbe setzen | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:241` | — kein Treffer | **NO-COUNTERPART** |
| 0x54 | `Esetpalette` | TT-Palette (Block) setzen | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:265` | — kein Treffer | **NO-COUNTERPART** |
| 0x55 | `Egetpalette` | TT-Palette (Block) lesen | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:290` | — kein Treffer | **NO-COUNTERPART** |
| 0x56 | `Esetgray` | TT-Graustufenmodus | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:313` | — kein Treffer | **NO-COUNTERPART** |
| 0x57 | `Esetsmear` | TT-Smear-Modus | `CONF_WITH_TT_SHIFTER` | `bios/screen.c:334` | — kein Treffer | **NO-COUNTERPART** |
| 0x58 | `Vsetmode` | Falcon-Videomodus setzen (VIDEL) | `CONF_WITH_VIDEL` | `bios/videl.c:619` | — kein Treffer | **NO-COUNTERPART** |
| 0x59 | `Vmontype` | angeschlossenen Monitortyp erfragen | `CONF_WITH_VIDEL` | `bios/videl.c:664` | — kein Treffer | **NO-COUNTERPART** |
| 0x5A | `Vsetsync` | Video-Sync-Modus setzen | `CONF_WITH_VIDEL` | `bios/videl.c:675` | — kein Treffer | **NO-COUNTERPART** |
| 0x5B | `Vgetsize` | Bildschirmspeichergröße für Modus erfragen | `CONF_WITH_VIDEL` | `bios/videl.c:724` | — kein Treffer | **NO-COUNTERPART** |
| 0x5D | `Vsetrgb` | Truecolor-RGB-Palette setzen | `CONF_WITH_VIDEL` | `bios/videl.c:806` | — kein Treffer | **NO-COUNTERPART** |
| 0x5E | `Vgetrgb` | Truecolor-RGB-Palette lesen | `CONF_WITH_VIDEL` | `bios/videl.c:862` | — kein Treffer | **NO-COUNTERPART** |
| 0x5F | `Vfixmode` | Videomodus „einfrieren"/entriegeln | `CONF_WITH_VIDEL` | `bios/videl.c:917` | — kein Treffer | **NO-COUNTERPART** |
| 0x60–0x7F | `Dsp_*` (32 Fn) | Falcon-DSP56001-Steuerung (Block-I/O, Programm laden/ausführen, Subroutinen, Host-Flags, Interrupt-Vektoren) | `CONF_WITH_DSP` | `bios/dsp.c` (32 Funktionen, Zeilen 222–1338, einzeln aufgelöst in §3.2) | — kein Treffer in `tos-api/src`/`kernel/src` | **NO-COUNTERPART** (alle 32) |
| 0x80–0x8D | `Locksnd`…`Buffptr` (14 Fn) | Falcon-DMA-Sound-Steuerung (Codec-Modus, Puffer, Tracks, Interrupts, GPIO, Verbindung) | `CONF_WITH_DMASOUND` | `bios/dmasound.c` (14 Funktionen, Zeilen 214–771, einzeln aufgelöst in §3.2) | — kein Treffer | **NO-COUNTERPART** (alle 14) |

**Rollenbeschreibungen 0x02–0x40 (Screen/IKBD/PSG/Sound/Timer/Cookie-
Gruppe)**: gegen die tatsächlichen Funktionskörper geprüft (Kommentare
gelesen, s. Beispiel-Zitate oben). **Rollenbeschreibungen 0x50–0x8D**
(TT-Shifter/VIDEL/DSP/DMASOUND, 47 Funktionen): aus Funktionsname +
Signatur + Standard-XBIOS-Semantik abgeleitet, **nicht** einzeln gegen
den vollständigen Funktionskörper verifiziert (Umfang, s. Abschnitt 3.2)
— als flacher, aber quellenverifizierter (Name+Zeile aus `ctags`/`grep`,
nicht aus Gedächtnis) Befund gekennzeichnet.

### 3.2 DSP- und DMASOUND-Einzelfunktionen (Vollständigkeit)

Auf Wunsch der Vollständigkeit — die 32 DSP- und 14 DMASOUND-Ziele
einzeln mit `file:line` (alle **NO-COUNTERPART**, kein einziger Treffer
in `tos-api/src`/`kernel/src` für irgendeinen dieser Namen):

**DSP (`bios/dsp.c`, Opcode 0x60–0x7F):** `dsp_doblock`:222,
`dsp_blkhandshake`:259, `dsp_blkunpacked`:296, `dsp_instream`:326,
`dsp_outstream`:350, `dsp_iostream`:425, `dsp_removeinterrupts`:494,
`dsp_getwordsize`:508, `dsp_blkwords`:522, `dsp_blkbytes`:558,
`dsp_lock`:599, `dsp_unlock`:611, `dsp_available`:619, `dsp_reserve`:628,
`dsp_loadprog`:650, `dsp_execprog`:670, `dsp_execboot`:693,
`dsp_lodtobinary`:922, `dsp_triggerhc`:954,
`dsp_requestuniqueability`:965, `dsp_getprogability`:976,
`dsp_flushsubroutines`:987, `dsp_loadsubroutine`:1083,
`dsp_inqsubrability`:1146, `dsp_runsubroutine`:1166, `dsp_hf0`:1200,
`dsp_hf1`:1222, `dsp_hf2`:1247, `dsp_hf3`:1257, `dsp_hstat`:1273,
`dsp_setvectors`:1286, `dsp_multblocks`:1338.

**DMASOUND (`bios/dmasound.c`, Opcode 0x80–0x8D):** `locksnd`:214,
`unlocksnd`:229, `soundcmd`:341, `setbuffer`:368, `setsndmode`:400,
`settracks`:423, `setmontracks`:440, `setinterrupt`:456, `buffoper`:498,
`dsptristate`:520, `gpio`:544, `devconnect`:720, `sndstatus`:750,
`buffptr`:771.

### 3.3 Nicht implementierte Opcodes in EmuTOS selbst (`xbios_unimpl`)

Diese Slots haben in EmuTOS **keinen** Funktionskörper (`bios/xbios.c:
xbios_unimpl`/`xbios_do_unimpl:1251` liefert die Funktionsnummer zurück)
— keine Funktionen, daher nicht einzeln tabelliert:
0x01 (`ssbrk`), 0x0B (`used_by_bios`), 0x24 (`prtblk`), 0x27
(`puntaes`), 0x28, 0x2D, 0x2F–0x3F (17 Slots), 0x41–0x4F (15 Slots, nur
vorhanden wenn `LAST_ENTRY > 0x40`), plus **0x5C** (innerhalb des
`CONF_WITH_VIDEL`-Blocks selbst ein `xbios_unimpl`-Einzel-Slot, s.
Quellzitat §3.1). **Zahl:** 6 Einzel-Slots + 17 (0x2F–0x3F) + 15
(0x41–0x4F) + 1 (0x5C) = **39** reine Füll-Slots (bei vollem
Falcon-Profil mit `CONF_WITH_TT_SHIFTER`/`_VIDEL`/`_DSP`/`_DMASOUND` an
— 39 + 103 real = 142 = alle Slots, Kontrollsumme stimmt). Mit
`CONF_WITH_TT_SHIFTER` **aus** kämen 0x50–0x57 als weitere 8 Füll-Slots
hinzu (dann 47 Füll-Slots / 95 real) — für welchen Zustand der
tatsächliche `make 512`-Build steht, ist offener Punkt #2 unten. Die
Füll-Slots sind — ebenso wie 0x00/0x0D/0x0E/0x10/0x18/0x1A/0x1B/0x1F/
0x26/0x2E oben — in NewTOS folgerichtig auch nicht implementiert
(fallen auf den generischen Catch-all), aber das ist **kein
NewTOS-Gap**, weil es in EmuTOS selbst nichts zu portieren gibt.

**Befund XBIOS:** von den **103 real implementierten** EmuTOS-XBIOS-
Zielfunktionen (42 in der Screen/IKBD/PSG/Sound/Timer/Cookie-Gruppe
0x00–0x40 + 8 TT-Shifter + 7 VIDEL [0x5C ausgenommen] + 32 DSP + 14
DMASOUND = 42+8+7+32+14 = 103) implementiert NewTOS **32 mit HAS**
(Screen-Gruppe, Floppy/DMA-Kontraktwerte, PSG-Kontrakt, Timer, RS-232,
Random, Protobt, Blitmode, Bconmap, Scrdmp, Midiws, Ikbdws, Kbrate,
Kbdvbase, Setprt) und **10 direkt** als **NO-COUNTERPART** in der
Basisgruppe (Initmous, Mfpint, Iorec, Keytbl, Bioskeys, Jdisint,
Jenabint, Xbtimer, Supexec, Nvmaccess) — plus **die gesamte
TT-Shifter/VIDEL/DSP/DMASOUND-Gruppe (61 Funktionen, Opcode 0x50–0x8D,
0x5C ausgenommen) ist NO-COUNTERPART** (10 + 61 = 71 NO-COUNTERPART
insgesamt, 32 + 71 = 103), bestätigt durch vollständige Lektüre von
`tos-api/src/xbios/mod.rs` (`LAST_ENTRY` dort effektiv `0x57`, alles
darüber fällt auf den generischen Catch-all). Das ist der **größte
einzelne Deckungs-Gap** dieses gesamten Master-Inventars — insbesondere
die Falcon-VIDEL-Truecolor-Palette (`Vsetrgb`/`Vgetrgb`), die laut
Projekt-Memory (`vdi-truecolor-workstation`) ohnehin als künftige
VDI-Arbeit geplant ist.

---

## 4. Util (+ Line-A-Init, Cookie-Jar)

### 4.1 `util/*.c` (10 Dateien, 59 Symbole)

#### boot.c (5 Symbole)

ROM-Boot-Auswahlmenü (nur für Nicht-Atari-Ports mit mehreren ROM-Images
relevant — ADR-Kontext: RPi4 hat eigenes Boot via `kernel8.img`).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `bootram` | 27 | springt in ein RAM-geladenes ROM-Image | — RPi4-Boot ist `kernel/src` eigenständig, kein ROM-Auswahlmenü-Konzept | N/A (Boot-Architektur grundverschieden) |
| `putl` | 47 | Debug-Ausgabe eines Langworts | — | N/A |
| `fatal` | 61 | Boot-Fehler anzeigen + anhalten | — | N/A |
| `get_cookie` | 72 | liest einen Cookie-Wert (Boot-Menü-intern) | `tos-api/src/cookie.rs:95` `get(tag)` — anderer Kontext (allgemeiner Cookie-Zugriff, nicht Boot-Menü) | HAS (Konzept, anderer Zweck) |
| `main` | 86 | Boot-Menü-Einstiegspunkt | — | N/A |

#### cookie.c (7 Symbole)

Cookie-Jar-Datenmodell (Tag/Value-Liste ab `$5A0`).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `cookie_init` | 26 | legt das Cookie-Jar-Array an, veröffentlicht `$5A0` | `tos-api/src/cookie.rs:150` `seed_defaults(version)` (anderer Name, gleiche Rolle: Jar mit Default-Cookies befüllen) | HAS |
| `cookie_add` | 34 | fügt ein Tag/Value-Paar hinzu | `cookie.rs:74` `set(tag, value)` | HAS |
| `cookie_get` | 64 | liest einen Cookie-Wert | `cookie.rs:95` `get(tag)` | HAS |
| `get_idt_cookie` | 83 | `_IDT`-Cookie (IDE-Timing) ermitteln | — Atari-IDE-spezifisch, kein RPi4-Äquivalent | N/A |
| `get_frb_cookie` | 98 | `_FRB`-Cookie (Fast-RAM-Buffer) ermitteln | ? | ? |
| `get_floppy_type` | 115 | Floppy-Controller-Typ für `_FLK`-Cookie | — kein Floppy-Controller auf RPi4 | N/A |
| `nvdi_cookie_present` | 130 | prüft, ob NVDI (Ersatz-VDI) installiert ist | — kein NVDI-Konzept in NewTOS | N/A |

#### doprintf.c (2 Symbole)

Gemeinsamer `printf`-Formatier-Kern (Basis für `kprintf`/`sprintf`).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `numconv` | 44 | Zahl → String-Konvertierung (Radix/Präzision) | — Rust `core::fmt` übernimmt das nativ | N/A (Sprachfeature) |
| `doprintf` | 78 | generischer Formatierkern (`outc`-Callback) | — Rust `write!`/`format!`-Makros | N/A (Sprachfeature) |

#### gemdos.c (12 Symbole)

Dünne Bindungsschicht: BIOS/Desktop-seitige Hilfsfunktionen, die selbst
wieder GEMDOS-Traps aufrufen (`dos_*`-Namensfamilie, s. `include/gemdos.h`).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `pgmld` | 29 | Programm laden (Wrapper um `kpgmld`) | ? | ? |
| `dos_setdt` | 49 | Datei-Datum/-Zeit setzen (Wrapper um Fdatime) | `kernel/src/desk/dir.rs:211` `fn dos_setdt(handle: i16, time: u16, date: u16) -> i32` | HAS |
| `dos_label` | 59 | Datenträger-Label lesen/setzen | ? | ? |
| `dos_space` | 77 | Gesamt-/Freiplatz eines Laufwerks (Wrapper um Dfree) | ? | ? |
| `dos_alloc_stram` | 94 | ST-RAM-Block allozieren (Wrapper um Malloc) | ? | ? |
| `dos_avail_stram` | 101 | freier ST-RAM (Wrapper um Dfree/Malloc-Test) | ? | ? |
| `dos_avail_altram` | 109 | freier Alt-RAM | ? | ? |
| `dos_alloc_anyram` | 117 | Block aus beliebigem RAM allozieren | ? | ? |
| `dos_avail_anyram` | 124 | freier Speicher (beliebiger Pool) | ? | ? |
| `dos_free` | 130 | Speicherblock freigeben (Wrapper um Mfree) | ? | ? |
| `dos_shrink` | 136 | Speicherblock verkleinern (Wrapper um Mshrink) | ? | ? |
| `dos_load_file` | 148 | komplette Datei in Puffer laden (Fopen+Fread+Fclose) | ? | ? |

#### intmath.c (1 Symbol)

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `Isqrt` | 18 | Integer-Quadratwurzel (gerundet) | `tos-api/src/vdi/primitives.rs:105` `fn isqrt(a: u32) -> u32` — Modulkommentar zitiert explizit `intmath.c:18` | HAS |

#### miscutil.c (3 Symbole)

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `build_root_path` | 16 | baut `"X:\"`-Wurzelpfad für Laufwerksbuchstabe | ? | ? |
| `extract_drive_number` | 34 | Laufwerksnummer aus Pfadstring extrahieren | ? | ? |
| `set_all_files` | 52 | füllt einen Puffer mit `"*.*"` (Alle-Dateien-Maske) | ? | ? |

#### nls.c (3 Symbole)

Nationale Sprachunterstützung (NLS) — Textkatalog-Auswahl.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `nls_init` | 29 | initialisiert NLS beim Boot | — kein Treffer für `nls_init`/`nls_set_lang`/`etos_gettext`; `tos-api/src/lib.rs` und `aes/form.rs` referenzieren ein `i18n`-Modul, aber nicht namensgleich geprüft | ? |
| `nls_set_lang` | 37 | setzt aktive Sprache | ? | ? |
| `etos_gettext` | 50 | übersetzt eine Message-ID in String | ? | ? |

#### optimize.c (10 Symbole)

AES/Desktop-Textformatierungs-Helfer (physisch in `util/`, funktional
AES-nah — **nicht** in `aes-audit/emutos-function-inventory.md`
enthalten, da dieses nur `aes/*.c` erfasst; hier vollständig neu gelistet).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `fmt_str` | 43 | formatiert String für Objekt-Anzeige (TE-Template) | `tos-api/src/aes/object.rs:1572` `ob_format` (referenziert laut AES-Inventar Zeile 405 als Gegenstück zu `ob_format`, nicht direkt `fmt_str`) | HAS (über `ob_format`, kein 1:1-Name) |
| `unfmt_str` | 77 | entformatiert String (Kehrfunktion zu `fmt_str`) | ? | ? |
| `inf_sset` | 103 | setzt Text eines editierbaren Objekts | ? | ? |
| `inf_sget` | 119 | liest Text eines editierbaren Objekts | ? | ? |
| `inf_gindex` | 134 | Index eines Objekts in einer Objektgruppe | ? | ? |
| `inf_what` | 157 | ermittelt Objekttyp-Klasse | ? | ? |
| `hex_dig` | 181 | Hex-Ziffer parsen | ? | ? |
| `scan_2` | 202 | parst 2 Zeichen (Editierfeld-Cursor-Scan) | `kernel/src/desk/inf.rs:305` `fn scan_2(c: &mut Cursor) -> u8` | HAS |
| `filename_start` | 226 | findet Dateinamen-Anfang in Pfad (nach letztem `\`) | ? | ? |
| `wildcmp` | 244 | Wildcard-Musterabgleich (`*`/`?`) | `kernel/src/desk/model.rs:319` `pub(super) fn wildcmp(pattern: &[u8], filename: &[u8]) -> bool` | HAS |

#### rectfunc.c (5 Symbole)

Rechteck-Arithmetik (`GRECT`), von AES **und** Desktop genutzt.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `inside` | 19 | Punkt-in-Rechteck-Test | `tos-api/src/aes/window.rs:1899` `fn inside(r: Grect, x: i16, y: i16) -> bool` | HAS |
| `rc_constrain` | 32 | begrenzt ein Rechteck auf ein anderes | ? | ? |
| `rc_equal` | 48 | Rechteck-Gleichheitstest | ? | ? |
| `rc_intersect` | 63 | Schnittmenge zweier Rechtecke | `kernel/src/desk/redraw.rs:221` `fn rc_intersect(a: Grect, b: Grect) -> Option<Grect>` | HAS |
| `rc_union` | 84 | Vereinigungsrechteck (Bounding-Box) | ? | ? |

#### string.c (11 Symbole)

Freistehende `libc`-artige String-/Speicher-Grundfunktionen
(`strcpy`/`strcmp`/`memcmp`/`sprintf`/…) — in EmuTOS nötig, weil
freistehendes C keine Standardbibliothek hat.

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `strcpy`,`strcat`,`strlcpy`,`strcmp`,`memcmp`,`strncmp`,`strncasecmp`,`toupper` (8 Fn) | 27–171 | Standard-C-String-/Speicherfunktionen | Rust `core::str`/`core::slice`/`str::eq_ignore_ascii_case` (Sprachfeature) | N/A (Sprachfeature, kein Portierungsbedarf) |
| `sprintf_outc` | 189 | Ausgabe-Callback für `doprintf` (füllt `sprintf`-Zielpuffer) | — Rust `core::fmt::Write` | N/A (Sprachfeature) |
| `sprintf` | 194 | formatiertes Schreiben in String-Puffer | — Rust `format!`/`write!` | N/A (Sprachfeature) |
| `__attribute__(...)` | 98 | Compiler-Pragma (kein Funktionskörper, ctags-Fehltreffer) | — | N/A (kein Symbol) |

### 4.2 Cookie-Jar / `Ssystem` — Klarstellung

Der `$5A0`-Cookie-Jar selbst ist reines Datenmodell (`util/cookie.c`,
§4.1) — **kein eigener Trap**. `grep -rn "Ssystem" references/emutos/`
liefert **keinen einzigen Treffer** im gesamten EmuTOS-Baum:
**`Ssystem` ist keine EmuTOS-Funktion.** Es ist eine reine
FreeMiNT-Erweiterung (GEMDOS-Opcode `0x154`, weit oberhalb
`MAX_FNCALL=0x57`), die NewTOS zusätzlich anbietet
(`tos-api/src/gemdos/ssystem.rs:1`, referenziert `sys/ssystem.c` aus
`references/freemint/`, nicht aus EmuTOS). Für dieses **EmuTOS**-Master-
Inventar gibt es daher **keine** Ssystem-Zeile — der Cookie-Jar-Zugriff
in echtem TOS/EmuTOS läuft ausschließlich über den direkten
Speicherzugriff auf `$5A0`, nicht über einen Syscall.

### 4.3 Line-A-Init (`bios/lineainit.c`, 2 Symbole)

Nicht in AES/VDI-Inventaren erfasst (die decken nur `aes/*.c`/`vdi/*.c`
ab); dies ist der BIOS-seitige Boot-Init-Teil der Line-A-Variablen
(`v_planes_shift` u.a.), nicht die Line-A-Trap-Sprungtabelle selbst
(die liegt in `bios/linea.S`, Assembler, hier nicht erfasst — analog zur
AES/VDI-Konvention, `.S`-Dateien separat/N/A zu behandeln).

| EmuTOS fn | line | Rolle | NewTOS-Gegenstück | Status |
|---|---|---|---|---|
| `set_screen_shift` | 42 | berechnet `v_planes_shift` aus `v_planes` (Pixel-Adress-Shift für Bitplane-Auflösungen) | — NewTOS-Backend ist chunky/Truecolor, kein Bitplane-Shift-Konzept | N/A (Bitplane-spezifisch) |
| `linea_init` | 50 | initialisiert die Line-A-Variablentabelle beim Boot | — kein Line-A-Support (ADR: kein Line-A-Portierung, s. VDI-Inventar-Notiz #4/#5) | NO-COUNTERPART |

---

## 5. Bereits vorhandene Subsystem-Inventare (referenziert, nicht dupliziert)

- **AES** (Trap #2, Selektor 200) — 402 Funktionen:
  `docs/analysis/aes-audit/emutos-function-inventory.md`
- **VDI** (Trap #2, Selektor 115) — 269 Funktionen:
  `docs/analysis/vdi-audit/emutos-vdi-function-inventory.md`
- **BIOS** (Trap #13) — 12 Dispatcher-Funktionen + 8 Boot-Init-Elemente:
  `docs/analysis/bios-audit/emutos-bios-inventory.md`
  (Plan/Auftrag: `docs/analysis/bios-audit/00-PLAN.md`)

Diese drei Dokumente wurden **nicht** erneut gelesen/verifiziert über das
für die Referenzierung nötige Maß hinaus — sie stammen aus separaten,
bereits abgeschlossenen bzw. laufenden Researcher-Pässen.

---

## 6. Offene Punkte / Out-of-scope-Beobachtungen

1. **GEMDOS-Opcode 0x20** ist in EmuTOS ein `NI`-Slot (kein
   Funktionskörper). Ob dort extern ein „Super()" erwartet wird, konnte
   mit den durchsuchten EmuTOS-Quellen (`bdos/`, `include/bdosbind.h`)
   nicht verifiziert werden — kein einziger Treffer für „Super" im
   gesamten `bdos/`-Baum. Für den Compendium-Abgleich wäre eine externe
   Sekundärquelle nötig (außerhalb des Scopes dieses reinen
   EmuTOS-Quell-Inventars).
2. **XBIOS-Config-Gate für Falcon `make 512`** — ob `CONF_WITH_
   TT_SHIFTER` im tatsächlichen Zielprofil-Build an oder aus ist, wurde
   für dieses Inventar nicht am Makefile verifiziert (nur der
   `#if`-Gate-Ausdruck selbst gelesen). Für Falcon-Hardware historisch
   „aus" (TT-Shifter ist TT-exklusiv), aber nicht abschließend am
   Build-System dieser Codebasis geprüft — als offener Punkt markiert,
   nicht angenommen.
3. **BDOS-interne „?"-Zeilen** (ca. 140 von 183 Tabelle-B-Einträgen):
   bewusst nicht auf NO-COUNTERPART herabgestuft. Ein fehlender
   1:1-Namenstreffer bei einer EmuTOS-internen `static`-Hilfsfunktion
   bedeutet bei einer Clean-Room-Neuimplementierung **nicht** zwingend
   eine fehlende Fähigkeit — nur ein Verhaltens-Audit (nicht Teil dieses
   Auftrags) kann das klären. Empfehlung für einen Folge-Auditor: Fokus
   auf `fsdir.c` (37 Symbole, größte unklassifizierte Fläche) und
   `fsopnclo.c`/`fsio.c` (Open/Read/Write-Kern).
4. **`util/optimize.c`/`util/rectfunc.c`/`util/gemdos.c` sind funktional
   AES-/Desktop-nah**, liegen aber physisch außerhalb `aes/*.c` und
   wurden daher vom bestehenden AES-Inventar nicht erfasst — hier zum
   ersten Mal vollständig gelistet (Out-of-scope-Beobachtung für einen
   möglichen AES-Audit-Nachtrag, keine Erweiterung dieses Tasks).
5. **XBIOS TT-Shifter/VIDEL/DSP/DMASOUND (62 von 67 realen
   XBIOS-Funktionen) sind komplett NO-COUNTERPART** — der mit Abstand
   größte einzelne Deckungs-Gap dieses gesamten Master-Inventars. Dies
   ist eine reine Präsenz-Feststellung (Bonus-Klassifikation dieses
   Tasks); Priorisierung/Behebung ist Sache von BB4
   (`docs/analysis/bios-audit/00-PLAN.md`), nicht dieses Dokuments.
6. **`nls.c`/`util/gemdos.c`-`dos_*`-Familie**: mehrheitlich „?" — kein
   `i18n`-Modul-Namensabgleich im Detail durchgeführt (nur Vorhandensein
   von `i18n::`-Verweisen in `tos-api/src/lib.rs`/`aes/form.rs`
   festgestellt, nicht deren Deckungsgrad).
