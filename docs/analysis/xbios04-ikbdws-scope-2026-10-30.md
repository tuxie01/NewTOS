# XBIOS04 — Umfang von `Ikbdws` (XBIOS 0x19): Fakten und Optionen

Stand 2026-10-30, Researcher. Dies ist ein datierter Schnappschuss (historisch,
nicht autoritativ). Ticket: `TASK/XBIOS04.md`, Finding XBIOS-CHARDEV-004
(`docs/audits/2026-10-28-xbios-chardev.md`). Es gibt keine Produktcodeaenderung;
die Entscheidung trifft der Eigentuemer.

**Frage:** bleibt `Ikbdws` ein No-op mit `docs/COMPATIBILITY.md`-Eintrag, oder
interpretiert NewTOS (einige) IKBD-Kommandos und erzeugt die beobachtbaren
Wirkungen und Antwortpakete?

**Herkunft der Aussagen.** *Gelesen* = Quelltext dieser Sitzung, mit `file:line`.
*Abgeleitet* = von mir geschlossen, ausdruecklich so markiert. *Sekundaer* = die
Hatari-Quelle `ikbd.c` (siehe 1.2) und das IKBD-Protokolldokument, das **nicht** in
`references/` liegt (`references/README.md`); beides ist hier nur Hinweis, kein
Beleg fuer das Verhalten des gepinnten Orakels.

---

## 0. Kurzfassung

1. EmuTOS ruft `ikbdws` selbst nur an einer Stelle (`bios/clock.c`, IKBD-Uhr,
   `igetregs`/`isetregs`); die Maus-Kommandos gehen ueber `ikbd_writeb` aus
   `Initmous`. NewTOS benutzt keinen der beiden Pfade (eigene Uhr, eigener
   VDI-Mauspfad) — ein Interpreter beruehrt also **keinen** EmuTOS-eigenen Weg.
2. Von den portierten Programmen im Standardabbild ruft keines `Ikbdws` auf
   (Abschnitt 2). Der Schaden heute ist theoretisch: Fremdprogramme, die auf ein
   IKBD-Antwortpaket warten oder die Maus abschalten.
3. **Abhaengigkeit, die die Wahl bestimmt:** jedes Antwortpaket (Status `0xF6`,
   Uhr `0xFC`, Joystick `0xFD`) wird ueber `statvec`/`clockvec`/`joyvec`
   zugestellt. Diese Haken ruft NewTOS nie (XBIOS-CHARDEV-003, ADR-031, in
   COMPATIBILITY als akzeptierte Abweichung). Ein Interpreter allein bringt W04
   also **nicht** auf `1/1/1`; dazu muss die Zustellung der drei Haken (Upcall wie
   fuer `kbdvec`) erst gebaut werden. Nur Zustandswirkungen (Maus aus) sind ohne
   diese Zustellung beobachtbar.
4. Empfehlung: Abschnitt 4.

---

## 1. IKBD-Kommandos

### 1.1 Was EmuTOS selbst sendet (gelesen)

| Quelle | Bytes | Zweck |
|---|---|---|
| `references/emutos/bios/ikbd.c:1036` (`ikbd_reset`) | `0x80 0x01` | Reset, wartet auf Versionsbyte `0xF1..` (`:1041-1046`), Timeout 300 ms |
| `references/emutos/bios/mouse.c:48` | `0x12` | Initmous Typ 0: Maus aus |
| `references/emutos/bios/mouse.c:54-57` | `0x08`, `0x0B xp yp` | Typ 1: relativer Modus, Schwelle |
| `references/emutos/bios/mouse.c:65-76` | `0x09 xmax ymax`, `0x0C xs ys`, `0x0E 00 x y` | Typ 2: absoluter Modus, Skala, Anfangsposition |
| `references/emutos/bios/mouse.c:96-98` | `0x0A dx dy` | Typ 4: Cursortasten-Modus |
| `references/emutos/bios/mouse.c:109-114` | `0x0F`/`0x10`, `0x07 buttons` | Y-Achse, Tastenaktion |
| `references/emutos/bios/clock.c:985-986` (`igetregs`) | `0x1C` per `ikbdws(0,..)`, wartet bis 2 s auf `clockvec` (`:988-991`) | IKBD-Uhr lesen |
| `references/emutos/bios/clock.c:996-997` (`isetregs`) | `0x1B` + 6 BCD-Bytes per `ikbdws(6,..)` | IKBD-Uhr setzen |
| `references/emutos/bios/ikbd.c:935` (`bconout4`) | beliebiges Byte | `Bconout(4,c)` -> `ikbd_writeb` |

`Ikbdws` selbst ist nur `do { ikbd_writeb(*ptr++); } while (cnt--);`
(`references/emutos/bios/ikbd.c:948-954`, `cnt` ohne Vorzeichen). Der einzige
`ikbdws`-Aufrufer im EmuTOS-Quelltext ist `references/emutos/bios/clock.c:986`
und `:997`; `references/emutos/bios/xbios.c:578` ist der Trap-Eintrag.

**Die IKBD-Uhr ist im Oracle-Profil nur Rueckfall.** `clock_init` nimmt den
IKBD-Zweig erst, wenn weder NVRAM noch MegaRTC noch ICD-RTC da sind
(`references/emutos/bios/clock.c:1202-1213`, Zweige davor `:1161-1200`);
`CONF_WITH_IKBD_CLOCK` ist im Standard `1` (`references/emutos/include/config.h:1008-1010`),
auf einigen Zielen `0` (`:68-69`, `:716-717`). Auf TT/Falcon (NVRAM) liest EmuTOS
die IKBD-Uhr nicht; das Orakel-W01 kommt vom Sondenprogramm, nicht vom BIOS.

### 1.2 Kommandotabelle

Spalten: Befehlsbytes (Kommando + Parameter) und Antwort laut Hatari `ikbd.c`
(sekundaer — lokale Kopie ausserhalb des Repos, im Ordner `RuSTari/hatari/src`
unter dem iCloud-Ordner "Development"; Version nicht gepinnt und nicht die 2.6.1
des Orakels; Befehlstabelle dort `:225-265`, Rumpfzeilen in der letzten Spalte).
Der Zustell-Vektor folgt dem **gelesenen** EmuTOS-Code: Header `0xF6` ->
`statvec` (`references/emutos/bios/aciavecs.S:484-487`), `0xF7`..`0xFB` ->
`mousevec` (`:489-491`), `0xFC` -> `clockvec` (`:493-496`), `0xFD`..`0xFF` ->
`joyvec` (`:502-531`); Paketlaengen nach dem Header `7,5,2,2,2,2,6,2,1,1`
(`:456-458`).

Klasse **A** = ueber die TOS-API beobachtbar und auf USB-HID-Zustand abbildbar;
**B** = Hardware-Nicht-Ziel.

| Code | Parameter | Bedeutung | Wirkung / Antwort | Vektor | Klasse | Gelesen (Hatari) |
|---|---|---|---|---|---|---|
| `0x80 0x01` | 1 | Reset | setzt Modi zurueck; Antwort `0xF1` (Versionsbyte, **kein** Paketheader) | — (Byte `< 0xF6`; EmuTOS liest es per Polling, `ikbd.c:1041`) | A | `:1911-1923` |
| `0x07` | 1 | Maustasten-Aktion | Zustand | — | A (Zustand) | nur Tabelle |
| `0x08` | 0 | relativer Mausmodus | Zustand; Pakete `0xF8..0xFB dx dy` | `mousevec` | A | `:1963` |
| `0x09` | 4 | absoluter Mausmodus | Pakete `0xF7 b xh xl yh yl` | `mousevec` | A | nur Tabelle |
| `0x0A` | 2 | Maus-Cursortasten-Modus | Mausbewegung -> Cursor-Scancodes | `kbdvec` | A | nur Tabelle |
| `0x0B` | 2 | Schwelle (relativ) | Zustand | — | A | nur Tabelle |
| `0x0C` | 2 | Skala (absolut) | Zustand | — | A | nur Tabelle |
| `0x0D` | 0 | absolute Position lesen | Paket `0xF7` | `mousevec` | A | nur Tabelle |
| `0x0E` | 5 | interne Position setzen | Zustand | — | A | nur Tabelle |
| `0x0F`/`0x10` | 0 | Y unten/oben | Zustand | — | A | nur Tabelle |
| `0x11` | 0 | Ausgabe fortsetzen | Zustand | — | A | `:2150-2161` |
| `0x12` | 0 | Maus aus | keine Mauspakete mehr | — | A | `:2175-2182` |
| `0x13` | 0 | Ausgabe anhalten | Hatari: Ausgabe pausiert; jedes weitere Kommando setzt fort (`:2150-2154`) | — | A | `:2186-2198` |
| `0x14`/`0x15` | 0 | Joystick-Ereignisse an/aus | Zustand | `joyvec` | B | Tabelle `:239-240` |
| `0x16` | 0 | Joystick abfragen | Paket `0xFD j0 j1` | `joyvec` | B | nur Tabelle |
| `0x17`..`0x19`, `0x1A` | n | Joystick-Ueberwachung/Cursor/aus | Zustand | — | B | Tabelle `:242-245` |
| `0x1B` | 6 BCD | IKBD-Uhr setzen | Uhrzustand | — | B | nur Tabelle |
| `0x1C` | 0 | IKBD-Uhr lesen | Paket `0xFC yy MM dd hh mm ss` (BCD) | `clockvec` | B | `:2435-2452` |
| `0x20`/`0x21`/`0x22` | n | Speicher laden/lesen/ausfuehren (6301-RAM) | — | — | B | Tabelle `:248-250` |
| `0x87` | 0 | Tastenaktion erfragen | `0xF6 07 <action> 0..` | `statvec` | A | `:2543-2557` |
| `0x88`/`0x89`/`0x8A` | 0 | Mausmodus erfragen | `0xF6 08 00..` (rel), `0x09 maxX maxY` (abs), `0x0A dx dy` (Tasten) | `statvec` | A | `:2567-2604` |
| `0x8B`/`0x8C` | 0 | Schwelle/Skala erfragen | `0xF6 ..` | `statvec` | A | nur Tabelle |
| `0x8F`/`0x90` | 0 | Y-Achse erfragen | `0xF6 ..` | `statvec` | A | nur Tabelle |
| `0x92` | 0 | Maus vorhanden? | `0xF6 0x12` wenn aus, sonst `0x00` | `statvec` | A | `:2689-2705` |
| `0x94`/`0x95`/`0x99` | 0 | Joystickmodus erfragen | `0xF6 0x14`/`0x15` | `statvec` | B | `:2716-2744` |
| `0x9A` | 0 | Joystick vorhanden? | `0xF6 0x1A` wenn aus, sonst `0x00` | `statvec` | B | `:2754-2770` |

Lesehinweise (gelesen, Hatari): jede Antwort beginnt dort mit einer Verzoegerung
(`IKBD_Delay_Random(7000,7500)`), `0xF6`-Antworten sind 8 Byte lang. Die
`0x1C`-Uhr startet in Hatari bei `00..` und zaehlt ab dann pro VBL weiter
(`:536-553`, `:1143-1186`); sie ist **nicht** die Rechneruhr. Welche Bytes das
Orakel fuer W01 liefert, ist **nicht gemessen**: die Sonde druckt nur den
`0xF6`-Inhalt (`st=08000000000000`), nicht `clk[]`
(`measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c:406-408`).

Zeilen "nur Tabelle" tragen das Verhalten **nicht** als gelesen — vor einer
Spec muss der Rumpf gelesen werden.

### 1.3 Begruendung der Klassen

**Klasse A** (Mausmodi 0x08/0x09/0x0A/0x0B/0x0C, 0x12, 0x13/0x11, 0x80, Abfragen
0x87–0x9A der Maus): Der Zustand gehoert zu Tastatur/Maus, die NewTOS ueber
USB-HID bedient; die Pakete entstehen heute schon im Dekoder
(`kernel/src/ikbd.rs:150@decode_byte`, relative Maus
`kernel/src/ikbd.rs:215@mouse_event`). *Abgeleitet:* `0x12` und `0x13` lassen sich
dort als Sperre vor `mouse_event` bzw. `route_key` umsetzen; `0x09` braucht einen
Umrechner von Relativbewegung auf absolute Position samt Skala (mehr Zustand).

**Klasse B** (Joystick, 0x1B/0x1C, 0x20–0x22): kein Joystick-Port, kein
6301-RAM, keine IKBD-Uhr auf der Pi. *Abgeleitet*, nicht aus einem ADR gelesen
(siehe Abschnitt 5). Das Ticket stuft IKBD-Uhr und Joystick als
Hardware-Nicht-Ziele ein (`TASK/XBIOS04.md`, Sachverhalt). Zwei Einschraenkungen:

- `0x1C` ist technisch beantwortbar (NewTOS hat die Uhr:
  `tos-api/src/gemdos/time.rs:222@xbios_gettime`); ein Programm, das auf das Paket
  wartet, laeuft sonst in seinen Timeout. Siehe Option 3.
- `0x16`/`0x9A`/`0x94` koennten mit festen Antworten ("kein Joystick gedrueckt",
  "nicht vorhanden") bedient werden; USB-Gamepads sind ein eigenes Thema und
  nicht im Umfang.

---

## 2. Wer benutzt was

**EmuTOS selbst** (gelesen, Abschnitt 1.1): Reset beim Start, Maus-Kommandos aus
`Initmous` (aufgerufen vom VDI: `references/emutos/vdi/vdi_mouse.c:605` Typ 1,
`:634` Typ 0), IKBD-Uhr nur ohne andere RTC. NewTOS fuehrt keinen dieser Pfade
aus: `Initmous` ist in `docs/analysis/deps/mapping.tsv:717` als "nicht
implementiert, der Dispatcher beantwortet die Funktionsnummer" gefuehrt; die Uhr
kommt aus `tos-api/src/gemdos/time.rs:222@xbios_gettime`. **Ein Interpreter
beruehrt den Uhrpfad (`clock_init`, `igetdt`, ...) nicht** — das ist die Frage im
Ticket ("Lokalisierung", letzter Absatz): nein.

**Portierte Programme.** Suche mit `/usr/bin/grep -rn -i ikbdws` ueber
`sdk/apps`, `sdk/examples`, `sdk/overlays`, `programs`, `tosdump/cases`,
`tosdump/core` und (ohne `worktrees`) `Applications`: **kein Treffer** in einem
Programmquelltext der Karte. Treffer nur in den Bindungen
(`sdk/libc/mintlib/include/mint/osbind.h`,
`sdk/libc/mintlib/syscall/syscalls.master`, Kopien in `Applications/mintlib-src`),
in `tosdump/core/cases.c:3878@Ikbdws` (Fall als "excluded" gefuehrt,
`tosdump/cases/COVERAGE-BIOS.md:105@Ikbdws`) und in
`Applications/Vanilla-Conquer-Atari-ST/tiberiandawn/atarilib/ikbd.cpp:372,427,432`
(Reset `0x80 0x01`, `0x08`, Wiederherstellung `0x07 00, 0x14, 0x08, 0x0B 01 01`).
`Initmous` ruft ebenfalls kein Programmquelltext auf (derselbe Lauf).

**Luecke "kein lokaler Quelltext".** Die ausfuehrbaren Dateien der
Standardkarte (`REQUIRED` in `tools/make-sd-image.sh:60@REQUIRED`) sind: argvtest,
libtest, clocktest, setjmptst, MXPROBE, mathtst, EX_LIB/LDGTEST, ZVIEW, CORE,
COREVAL, DUMPVDI, VDIDRAW, QED, TeraDesk (DESKTOP), PRESENT. Jede hat ein
Bauverfahren aus lokalem Quelltext (Skript hinter dem `|` des jeweiligen
`REQUIRED`-Eintrags); in keinem lokalen Quelltext steht `Ikbdws`. Die
`sdk/examples/*.c` baut das Skript selbst (`build_example`,
`tools/make-sd-image.sh:100@build_example`). **Nicht geprueft:** vorkompilierte
Bibliotheken (die mintlib-Bibliothek unter `sdk/apps/highwire/modules/network.src/socklib`
enthaelt den Namen als Symbol der Bindung — kein Beleg fuer einen Aufruf) und
Quellbaeume, die nicht unter `sdk/` oder `Applications/` liegen. Die Luecke ist
damit fuer Quelltexte leer, fuer Binaer-Bibliotheken offen. Vanilla-Conquer ist
nicht auf der Karte.

**Was heute bricht** (aus dem Code, nicht gemessen ausser W01–W04):
- Programm schickt `0x1C` und wartet auf `clockvec`: laeuft in seinen eigenen
  Timeout (W04 `clockvec=0`,
  `measurements/measurement-2026-10-28-xbios-chardev-audit/probe-diff-emutos-tt-vs-newtos-us.txt:154,160`).
- Programm schickt `0x12` (Maus aus): wirkt nicht, der Dekoder liefert weiter
  Mauspakete an die VDI-Maus (`kernel/src/ikbd.rs:215@mouse_event`).
- Programm schickt `0x80 0x01` und `0x08` wie Vanilla-Conquer: ohne Wirkung;
  unschaedlich, weil NewTOS ohnehin relative Pakete liefert.
- `0x13` (Pause): Tastaturpakete laufen weiter.

*Abgeleitet:* Hauptnutzer waeren Spiele und Mausbeschleuniger der Atari-Szene
(eigener Mausmodus, Joystick); ein spaeterer m68k-Pfad wird sie bringen, die
heutigen LP64-Portierungen nicht.

---

## 3. Optionen

Gemeinsame Randbedingungen (gelesen):

- Die Zustellung der Antwortpakete haengt an `statvec`/`clockvec`/`joyvec`
  (`references/emutos/bios/aciavecs.S:484-531`). NewTOS ruft diese Haken nicht;
  nur `kbdvec` wird ueber `tos-api/src/xbios/kbdvecs.rs:350@route_key` als
  EL0-Upcall bedient, `check_events` meldet die uebrigen einmal als Warnung
  (`tos-api/src/xbios/kbdvecs.rs:447@check_events`, ADR-031).
- Das Versionsbyte `0xF1` der Reset-Antwort liegt unter `0xF6`; liefe es durch
  `feed`, behandelte der Dekoder es als **Tasten-Break-Scancode 0x71**
  (`kernel/src/ikbd.rs:150@decode_byte`; vgl. EmuTOS `ikbdraw`,
  `references/emutos/bios/aciavecs.S:409-414`). Antworten duerfen also nicht
  ungeprueft in `feed` laufen.
- Der Dekoder ueberspringt `0xF6` mit **6** Datenbytes
  (`kernel/src/ikbd.rs:51@packet_data_len`), EmuTOS erwartet **7**
  (`references/emutos/bios/aciavecs.S:456`, `:352`). Nur relevant, wenn
  Statuspakete in den Dekoder gelangen.
- Der Injektor speist Bytes in den Dekoder ein; ein Zustands-Gate ("Maus aus")
  muss im Dekoder sitzen, nicht im USB-Treiber, sonst sieht der Injektor es nicht.

### Option 1 — No-op bleibt, COMPATIBILITY-Eintrag

*Wirkung:* heutiger Zustand, dokumentiert. W01–W04 bleiben DIFF, werden
ACCEPTED (Ticket-Abnahme: bei "No-op" steht der Eintrag in COMPATIBILITY, der Fall
wird ACCEPTED).
*Kosten:* ein Absatz in `docs/COMPATIBILITY.md` (neben dem Haken-Absatz, der
dort schon die uebrigen Haken nennt) und eine Zeile in
`docs/api/xbios/19-ikbdws.md`.
*Wegwerfen:* nichts; Option 2/3 koennen die Entscheidung spaeter abloesen.
*Fahrplan:* konsistent mit ADR-031 (Haken nicht bedient, aber gemeldet).
`Ikbdws` selbst warnt heute nicht; eine einmalige Warnung je Prozess waere ein
kleiner Zusatz ohne Parser (*Entscheidung des Eigentuemers; ADR-031 nennt
`Ikbdws` nicht*).
*Abnahme:* W01–W04 unveraendert (NewTOS `0/0/0`), Eintrag vorhanden.

### Option 2 — Minimaler Interpreter fuer Klasse A (Zustand, ohne Antworten)

*Wirkung:* NewTOS wertet die Bytes von `Ikbdws` aus (Kommandolaengen aus 1.2)
und setzt Zustand im Dekoder: `0x12`/`0x08` (Mauspakete aus/an, Gate vor
`mouse_event`), `0x13`/`0x11` (Tastenausgabe halten), `0x80 0x01` (Zustand
zuruecksetzen). Keine Antwortpakete. `Bconout(4,c)` hat denselben Pfad
(`references/emutos/bios/ikbd.c:935`; heute "nimmt an und verwirft",
`docs/analysis/deps/mapping.tsv:614`).
*Kosten:* Zustandsbytes plus Kommando-Parser im Kernel (der Parser muss die
Parameterlaengen kennen: `0x0B`, `0x09` tragen Parameter; ein Parameter als
Kommando gelesen ist die typische Fehlerquelle); je Kommando ein Test. Ohne
`0x09`/`0x0A` bleibt der Modus-Wechsel unvollstaendig — dann ein Warnlog fuer
nicht ausgewertete Kommandos (nie still).
*Wegwerfen:* wenig; Parser und Zustand tragen Option 3.
*Fahrplan:* "syscall compat, not hardware emulation" (Projektgedaechtnis): ja,
solange nur Zustand abgebildet wird.
*Abnahme:* Mausmodus-Fall (3.1); W01–W04 bleiben DIFF (Zaehler 0), der
Abnahmetext braeuchte eine Ausnahme fuer die Antwortfaelle.

### Option 3 — Option 2 plus Antwortpakete (Status, Uhr, Joystick-nichts)

*Wirkung:* zusaetzlich Antworten fuer `0x87..0x92` (Maus, `0xF6 ...`), `0x1C`
(Paket `0xFC` aus `xbios_gettime` in BCD) und, wenn gewollt, feste Antworten fuer
Joystick-Abfragen. Zustellung ueber `statvec`/`clockvec`/`joyvec`.
*Kosten:* der Zustellweg ist neu und gross: EL0-Upcall je Haken (heute nur
`kbdvec`), der die XBIOS-CHARDEV-003-Abweichung fuer diese drei Haken aufhebt
(Tabelle je Prozess, Warteschlange, Ueberlauf, Reentranz; Entwurfsbegruendung in
der Moduldoku `tos-api/src/xbios/kbdvecs.rs`). Dazu: Antwort darf nicht durch
`feed` laufen (`0xF1`), `0xF6`-Laenge 7. Uhrwerte: Hatari liefert einen
freilaufenden Zaehler ab `00` (1.2, sekundaer), nicht die Rechneruhr; kein
Programm kann sich auf einen bestimmten Wert verlassen — NewTOS mit echter Uhr
waere "besser", nicht gleich.
*Wegwerfen:* nichts, wenn die Haken-Zustellung ohnehin kommt (sie schliesst auch
XBIOS-CHARDEV-003 fuer W04).
*Fahrplan:* haengt am Strang der Haken (ADR-031); Joystick-Antworten liegen
ausserhalb von 1.0 ("wie das Original auf der Pi").
*Abnahme:* W01–W04 gleich dem Orakel (je ein Haken-Aufruf,
`st=08000000000000`) **und** der Mausmodus-Fall.

### Option 3a — wie 3, aber ohne Joystick

Klasse-A-Abfragen und `0x1C`; Joystick bleibt ohne Antwort (W03 bleibt DIFF, in
COMPATIBILITY als Hardware-Nicht-Ziel). Kleinste Option, die den im Ticket
genannten Hang ("auf `0x1C` warten") beseitigt.

### 3.1 Abnahmefaelle

**W01–W04** (vorhanden):
`measurements/measurement-2026-10-28-xbios-chardev-audit/method/xcprobe.c:386-411`
— Haken austauschen, `Ikbdws(0,{0x1C})`, `{0x88}`, `{0x16}`, je 100 Ticks warten,
Zaehler und `st[]` ausgeben. Beide Seiten sind gelaufen. Erweiterung ohne
Wirkung auf die vorhandenen Orakelzeilen: `clk[]` mit ausgeben, damit der
`0xFC`-Inhalt der Referenz bekannt ist (Option 3 braucht ihn).

**Mausmodus-Fall (Entwurf, nicht gemessen).** Nach dem Muster der Eingabesonde
`xkprobe` (`measurements/measurement-2026-10-28-xbios-chardev-audit/method/xkprobe.c`,
Eingaben `measurements/measurement-2026-10-28-xbios-chardev-audit/method/xkeys-hatari.txt`
und `measurements/measurement-2026-10-28-xbios-chardev-audit/method/xkeys-ikbd.txt`):
1. `mousevec` mit Zaehler belegen, `Ikbdws(0,{0x12})`, "READY" ausgeben.
2. Eingabe: auf dem Orakel `hatari-event rightdown`/`rightup` (erste Datei),
   auf NewTOS dieselben Pakete `0xF9 0 0`/`0xF8 0 0` ueber den Injektor
   (zweite Datei).
3. Erwartung (zu **messen**, nicht angenommen): Orakel `mousevec calls=0`, wenn
   Hatari bei "Maus aus" auch Tastenpakete unterdrueckt — **nicht gelesen**, der
   Rumpf der Paketerzeugung wurde nicht untersucht. Gegenprobe: `Ikbdws(0,{0x08})`
   danach, dann `calls=2`.
Das passt in das vorhandene Werkzeug (Orakel: Hatari mit `--cmd-fifo`; NewTOS:
Injektorkernel) und braucht keine Hardware. Es ist eine Sonde, kein
tosdump-Tabellenfall (Haken und Eingabe liegen ausserhalb der Tabellenform,
`docs/audits/2026-10-28-xbios-chardev.md` Abschnitt 5, letzter Satz). Fuer
Option 1 braucht es den Fall nicht.

Die Messung wurde **nicht** ausgefuehrt (Auftrag: Entwurf; Laeufe sind
Hatari-/QEMU-Starts mit Scratch-Images und gehoeren in ein eigenes Ticket).

---

## 4. Empfehlung (Empfehlung des Researchers, Entscheidung beim Eigentuemer)

**Option 1 jetzt, Option 3a als Folgeticket hinter der Haken-Zustellung.**

Gruende:
1. Kein portiertes Programm im Standardabbild ruft `Ikbdws` (Abschnitt 2); der
   heutige Nutzen eines Interpreters ist null, der Nutzen liegt im spaeteren
   m68k-Pfad.
2. Antwortpakete sind ohne die Haken-Zustellung nicht beobachtbar (Abschnitt 0,
   Punkt 3). Option 2 bringt nur `0x12`/`0x13`; das ist ein halber Interpreter,
   der W01–W04 nicht schliesst und den ein spaeterer Antwortweg ohnehin neu
   anfasst.
3. "Nie still" (ADR-031) laesst sich auch bei Option 1 einhalten: eine
   einmalige Warnung je Prozess bei `Ikbdws`/`Bconout(4)` — falls gewollt, ein
   kleiner Anschlussauftrag ohne Parser.
4. Der Aufwand von Option 3a (Upcall fuer drei Haken, Zustandsmaschine,
   `0xF6`-Laenge) ist so gross, dass er zusammen mit XBIOS-CHARDEV-003
   entschieden werden sollte, nicht als Nebenwirkung von XBIOS04.

Folgt der Eigentuemer dem nicht und will die Mausabschaltung (`0x12`) sofort,
ist Option 2 ohne Antworten ein vertretbarer Zwischenschritt (Zustand im
Dekoder, Warnlog fuer alles andere).

---

## 5. Offene Punkte / nicht belegt

- Hatari-Verhalten bei "Maus aus" fuer Tastenpakete und die Rumpfzeilen der
  "nur Tabelle"-Kommandos (1.2) nicht gelesen; Hatari-Quelle nicht in
  `references/`, Version unbekannt. Gepinnt ist nur die Binaerdatei 2.6.1
  (`docs/analysis/ORACLE.md`).
- Welche Bytes das Orakel auf `0x1C` liefert (W01): nicht gemessen.
- ADR-Lage: ein ADR, der den IKBD-6301 ausdruecklich zum Nicht-Ziel erklaert,
  wurde nicht gefunden (Suche in `docs/MEMORY.md`); Klasse B ist *abgeleitet*.
- Ob ein vorkompiliertes Binaer `Ikbdws` aufruft: nicht untersucht.

Hinweise ausserhalb des Auftrags (je eine Zeile, nicht bearbeitet):
- `kernel/src/ikbd.rs:51@packet_data_len` ueberspringt `0xF6` mit 6 Datenbytes,
  EmuTOS erwartet 7 (`references/emutos/bios/aciavecs.S:456`).
- `Bconout(4,c)` (`references/emutos/bios/ikbd.c:935`) teilt das Schicksal von
  `Ikbdws`; die Entscheidung sollte beide gemeinsam erfassen.
- `bcostat4`-Vorbedingung (`bios-r1` 67): laut Ticket seit BIOS04 erfuellt, hier
  nicht nachgemessen.
