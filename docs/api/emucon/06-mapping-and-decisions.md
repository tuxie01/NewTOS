# EmuCON2 — Abbildung auf NewTOS, Unterschiede zur Kernelkonsole, Entscheidungen

Legende: [README](README.md). Dies ist die Vorlage fuer die Freigabe (Regel 5).
**Nichts darin ist entschieden.** Kennzeichnung: Beleg = `file:line`;
„Research-Sicht" = meine Einschaetzung aus dem Gelesenen, nicht bindend.

## 1. Abbildung EmuTOS-Funktion → geplanter Ort

Vorschlag nach dem ADR-022-/ADR-026-Muster („gleichnamige Funktionen, gleiche
Aufrufreihenfolge", `docs/MEMORY.md` ADR-022/026): ein Modul je EmuTOS-Datei
unter programs/emucon/src/. Ob Bibliothek + schmaler Binaer-Crate (wie
`programs/desktop/` + `programs/newdesk/`) oder ein einziger Crate: **D8**.

| EmuTOS | Geplant (NewTOS) | Anmerkung |
|---|---|---|
| `cmdasm.S: _coma_start` (`:25-49`) | `sdk/crt0.S` + `main(BASEPAGE*)` (`programs/newtos-gem/src/lib.rs:1-12`); `main` gibt 0 zurueck, crt0 beendet mit `Pterm` | `Mshrink`/eigener Stack entfallen (crt0: Mshrink auf Image + 64 KiB Stack). Umgebung: `basepage->p_env` bei Offset `0x2C` (`docs/api/structures/basepage.md:36`). Intern |
| `cmdasm.S: _getwh/_getht` (`:51-72`) | **offen, D4** | `getht` ungenutzt |
| `cmdasm.S: _jmp_gemdos/_bios/_xbios` (`:77-105`) | Bindungen in `programs/newtos-gem/src/{gemdos,xbios}.rs` + **neu** `bios.rs` | Luecken: `gemdos.rs` hat kein `Pexec`, `Fdup`, `Fforce`, `Dgetpath`, `Fgetdta`, `Sversion`, rohes `Dfree` (nur `dos_space`, `:318`), `dos_trap` kennt 3 Argumente (`:112`), `Pexec` braucht 4; `xbios.rs` nur `keytbl` (`:79`); BIOS-Bindung fehlt |
| `cmdmain.c: cmdmain` (`:63-142`) | `cmdmain.rs: cmdmain` | `Supexec(get_nflops)` entfaellt (ungenutzt, 05 §4) |
| `cmdmain.c: execute` (`:151-178`), `create_redir/close_redir` (`:180-202`), `strip_quotes` (`:207-220`) | `cmdmain.rs` | Verhalten siehe 03; Defekte D7 |
| `cmdmain.c: getenv` (`:230-250`) | `cmdmain.rs` | Format `PATH=\0X:\\\0\0` (01 §3) |
| `cmdmain.c: change_res/valid_res/get_nflops` (`:255-336`) | `cmdmain.rs`, abhaengig von **D5** | Hardwareanteil |
| `cmdparse.c: parse_line/get_redirect/get_next_arg` (`:21-161`) | `cmdparse.rs` | `arglist` 30 / `redir_name` 128: D7 |
| `cmdedit.c: *` (`:41-480`) | `cmdedit.rs` | `init_screen` haengt an **D4**; Escape-Ausgaben an **D3** |
| `cmdexec.c: exec_program` & Hilfen (`:29-234`) | `cmdexec.rs` | Endungsmenge **D6**; `Pexec(0, path, tail, NULL)` |
| `cmdint.c: cmdtable, lookup_builtin, run_*, copy_move, output*, help*, …` (`:140-1301`) | `cmdint.rs` | 21 Befehle + `X:`, siehe 04 |
| `cmdutil.c: escape/message/messagenl/errmsg/strlower/strupper/getword/decode_date_time/get_path_component/has_wildcard/program_extension/strequal/getcookie` (`:26-340`) | `cmdutil.rs` | `getcookie` → `dos_getcookie` (`programs/newtos-gem/src/gemdos.rs:453`); `strequal`, `strlower/upper` nur ASCII (`:117-140,294-313`) |
| `cmdutil.c` `#ifdef STANDALONE_CONSOLE` libc (`:342-455`) | entfaellt | Rust `core`/`alloc` |
| `clistub.h: coma_start` (`:13`) | Registry-Name `EMUCON` (`tos-api/src/aes/shell.rs:67-71`) | Start wie NEWDESK (01 §6) |

**Einbettung/Start (gelesen, 01 §6):** `programs/Cargo.toml:13` (Mitglied),
`tools/xtask/src/main.rs:421` (`PROGRAMS`), `kernel/embed.rs:49` (`EMBEDDED`;
Hosttest `newdesk_is_the_one_embedded_program` am Dateiende muesste angepasst
werden), Start: der CONSOLE-Arm (`kernel/src/shell_loop.rs:465-466,579-583`) ruft
statt `crate::shell::run_console()` die Launch-Funktion fuer ein
eingebettetes Programm (Vorbild `launch.rs:131-148`).

## 2. Was die Kernelkonsole (`kernel/src/shell.rs`) anders macht

| Aspekt | EmuCON2 (Beleg) | `shell.rs` (Beleg) |
|---|---|---|
| Laufzeit | EL0-aehnlich: eigener Prozess/PD | EL1, Kernelcode; Stellvertreter-PD (`kernel/src/shell.rs:26-31`) |
| Banner | `ESC E`, `ESC e`, `Welcome to EmuCON2 version <v>` / `Type HELP for builtin commands` / leer (`cmdmain.c:91-95`) | `NewTOS console (EmuCON) - 'help' for commands, 'exit' to return.` + leer (`kernel/src/shell.rs:14`); im Rueckfall zusaetzlich `No desktop: NEWDESK could not be loaded (GEMDOS error N).` (`:40-55`) |
| Prompt | `<L>:<pfad>>` ohne Leerzeichen (`cmdedit.c:52-54`, `cmdint.c:928-944`) | `<L>:\<pfad>> ` mit Leerzeichen (`kernel/src/shell.rs:296-309`) |
| Eingabe | `Bconin(2)`, Scancodes, Verlauf 10, Tab, Cursor/Wort-Bewegung, Einfuegen/Loeschen (02) | `Cnecin`; nur Backspace und druckbare Zeichen, 127 Zeichen (`kernel/src/shell.rs:11,78-100`) |
| Parser | Anfuehrungszeichen, `>`, Argumentliste, Fremdprogramme (03) | erstes Wort / Rest der Zeile (`kernel/src/shell.rs:105-120`); keine Umlenkung, keine Quotes |
| Befehle | 21 + `X:` (04) | `exit/quit`, `help`, `ver`, `cls`, `pwd`, `cd`, `dir/ls`, `type/cat`, `run`, `bench`, `init`, `X:` (`kernel/src/shell.rs:121-131,141-155`) |
| `cd` | `Dsetdrv`-Umweg, Laufwerksangabe, `cd x:` zeigt Pfad (4.2) | `Dsetpath`; Fehler `No such directory.` (`:211-221`) |
| `ls/dir` | Muster-Normalisierung, `-l`, 13er-Spalten (4.9) | `Fsfirst(.., 0x10)` + 14-Spalten-Zeile `<DIR>`/Groesse + `N item(s).` (`:223-262`) |
| `cat/type` | mehrere Dateien, Joker, `\n→\r\n`, Abbruch/Pause (4.1) | eine Datei, 512-Byte-Puffer, schliesst mit `\r\n`, `Cannot open file.` (`:265-293`) |
| Programmstart | Erweiterungssuche, PATH, `Pexec(0,…)`, Rueckgabe `error code N` (03 §6) | `run <datei>` = `Pexec(0, pfad)`, `[program exited]`, `Cannot run program (error N).` (`:158-175`) |
| Fehlertexte | tabellarisch (04 §2) | frei (`Unknown command: x`) |
| Beenden | nur `exit` → `Pterm0` | `exit`/`quit` → Rueckkehr in die Shell-Schleife (`:121`) |
| Text-Escapes | sendet VT52-Folgen an `Bconout(2)` | umgeht sie (`fbcon::clear_and_home()`, `:124`) |

## 3. Zu entscheiden (Regel 5) — mit Folgen

Reihenfolge nach Abhaengigkeit: D3 und D4 sind **Voraussetzungen** (ohne sie
laeuft EmuCON nicht sinnvoll), die uebrigen betreffen Umfang und Treue.

### D1 — Einbettung, Name, Rueckfall

* Vorschlag aus dem Gelesenen: eingebettetes Programm, Registry-Name `EMUCON`
  (`DEF_CONSOLE`), Dateiklasse `.NCP` (Konsolenprogramm, ADR-018) fuer die
  Bauausgabe; zweiter Eintrag in `EMBEDDED`/`PROGRAMS`.
* **Frage 1a — Kernelkonsole danach:** (a) `shell.rs` samt
  `standin::enter_emucon/leave_emucon` loeschen (Kernel „ohne Konsole",
  Spiegel von ADR-026 Pkt. 3: Ladefehler von EMUCON = Meldung auf Seriell + kein
  Rueckfall); (b) eine minimale Kernelkonsole als *letzter* Rueckfall
  stehen lassen. **Folge:** der heutige Rueckfall NEWDESK-kaputt → Konsole (D6-d)
  haengt dann an einem zweiten Blob. EmuTOS kennt diesen Fall nicht, weil ein
  ROM-Einsprung nicht scheitern kann (`gemshlib.c:614-620`). Der Anwender
  sagte: nicht in der Kernelkonsole nachruesten — das entscheidet (a)/(b)
  nicht.
* **Frage 1b — die Rueckfall-Meldung:** `kernel/src/shell.rs:40-55` schreibt
  „No desktop: NEWDESK could not be loaded (GEMDOS error N)." in den Schirm
  unter das Banner; Doku `kernel/src/shell.rs:36-39`: „a machine without a serial cable
  still tells its user". EmuCON macht als erstes **`ESC E`** (`cmdmain.c:91`) und
  loescht jede vorherige Kernelausgabe; ein Argument/Tail liest es nicht
  (01 §3). Optionen: (i) Meldung geht nur noch auf Seriell; (ii) EMUCON zeigt
  eine zusaetzliche Zeile nach dem Banner, wenn der Kernel es per Umgebung
  (z. B. eine `NEWDESK_ERROR=`-Variable) mitgibt — benannte Abweichung von
  `cmdmain.c:91-95`; (iii) Kernel wartet/gibt die Meldung nach EmuCONs Banner
  aus (nicht sauber machbar). Research-Sicht: (ii) ist die einzige, die die
  bisherige Zusage haelt; kostet eine kleine, markierte Abweichung.
* Messlaeufe, die den Text „No desktop:" heute pruefen, sind eingefroren
  (`measurements/measurement-2026-10-16-desk43-u10-test/method/crash3.sh`,
  `q-after3.sh`); neue Laeufe brauchen die neue Erwartung.

### D2 — NewTOS-eigene Befehle (`bench`, `init`, `run`, `ver`, `quit`): Fragen

Befund, keine Entscheidung:

* **`run <datei>`** (`kernel/src/shell.rs:158-175`) = `Pexec(0, pfad)` mit exaktem Namen.
  EmuCON startet jedes Wort, das kein Befehl ist (03 §6) — mit
  Endungsergaenzung und PATH. Unterschied: `run` nimmt den Dateinamen so,
  wie er ist (jede Endung), EmuCON nur Namen mit Programmendung (D6).
  Frage: weglassen, als Erweiterung behalten (dann gegen `[program exited]`
  und `Cannot run program (error N).`-Texte abwaegen) oder per Alias?
* **`init`** (`kernel/src/shell.rs:177-194`) = drei `Dcreate`: `C:\ACC`, `C:\AUTO`,
  `C:\GEMSYS`, Meldung je Verzeichnis (`created` / `already exists` /
  `FAILED`). Ist mit `mkdir` dreimal erreichbar. Frage: weglassen / behalten
  (neuer Befehl, nicht in 1:1) / in ein Setup-Programm?
* **`bench`** (`kernel/src/bench.rs`) benutzt Kerneninterna: TT-RAM-Pool
  (Pufferbeschaffung ausserhalb von `.bss`) und `CNTPCT_EL0`/`CNTFRQ_EL0`
  (`bench.rs:1-30`). Aus EL0 braucht es einen Zeitgeber und `Mxalloc(TT)`
  (`MXALLOC` ist in `newtos-gem` gebunden, `programs/newtos-gem/src/gemdos.rs:56`); ob EL0
  `CNTPCT_EL0` lesen darf, ist **nicht geprueft**. Frage: weglassen /
  eigenes `.NCP`-Programm (`BENCH.NCP`, Kernel-unabhaengig, kein EmuCON-Befehl) /
  im Kernel lassen und ueber einen anderen Weg erreichbar machen?
* **`ver`** (`kernel/src/shell.rs:123`, „NewTOS 0.6.1 - clean-room TOS for AArch64") hat
  in EmuCON **kein** Gegenstueck; `version` zeigt `0.30` (GEMDOS, 4.19).
  Frage: `ver` als zusaetzlicher Befehl? Was zeigt das Banner als
  `<version>` (**D9**)?
* **`quit`** (`kernel/src/shell.rs:121`): gibt es in EmuCON nicht; `quit` waere ein
  externer Programmname → `file not found`.

### D3 — VT52-Parser in `Bconout(2)` (Voraussetzung)

EmuCON sendet 10 Escape-Folgen (05 §3), NewTOS-`Bconout(2)` versteht sie nicht
(`tos-api/src/vdi/escape.rs:128-136,166`). Optionen:

* (a) **Vollstaendiger VT52-Parser** nach `bios/vt52.c` (unbedingt kompiliert, kein `CONF_WITH_VT52` — siehe `docs/api/bios/_vt52.md`;
  Spezifikation `docs/api/bios/03-bconout.md:230`). Schaltet EmuCON **und**
  jedes andere TOS-Textprogramm frei (mintlib-Programme senden `ESC E`
  u. a.). Aufwand: Zustandsmaschine in `alpha_conout` + Cursor-/Farb-/
  Zeilenoperationen auf der Alphaschicht (`escape.rs` fuehrt `CUR_X/CUR_Y`).
* (b) **Nur die 10 Folgen.** Kleiner, aber ein halber Parser ist ein
  Abdriftrisiko (ADR-022-Erfahrung: Umgehung statt Nachbau).
* (c) EmuCON bekommt private Steuerung (z. B. andere Syscall-Schiene) —
  Abweichung von „1:1 `cli/`", haelt EmuCON an NewTOS fest.

Research-Sicht: (a) ist die EmuTOS-treue Loesung; sie ist ein **eigenes
Ticket vor EmuCON**. `ESC b/c` sind nur im Aufloesungswechselpfad noetig
(D5).

### D4 — Zeilen-/Spaltenzahl im EL0-Programm

EmuCON liest `v_cel_mx/my` per Line-A (`cmdasm.S:51-60`); Line-A gibt es auf
AArch64 nicht (`docs/api/_line-a.md:86-90`, ADR-030); in `sdk/crt0.S`, `sdk/libc/newtos-arch`, `programs/newtos-gem` und `tos-api`
fand die Suche nach `linea`/`chcells` keine Line-A-Bindung; vorhanden ist nur
`vq_chcells` in `tos-api/src/vdi/escape.rs`/`docs/api/vdi/_escape.md:124-127`, das
eine VDI-Workstation voraussetzt (**Schluss**: `v_escape` braucht ein Handle). Optionen:

* (a) VDI `vq_chcells` — EmuCON braeuchte `v_opnvwk` (und damit AES-Handle):
  es waere keine reine Zeichenprogramm-Anwendung mehr; widerspricht
  `cmdmain.c:227-228` („not a GEM app").
* (b) Neue Abfrage (XBIOS-/`Ssystem`-Unterfunktion oder Cookie) fuer die
  Textzellen der Alphaschicht — Erweiterung der Schnittstelle, ADR noetig.
* ~~(c) Ein Linea-aehnlicher Block im Prozess~~ — entfaellt: ADR-030
  (`docs/MEMORY.md`, Accepted 2026-10-26) exportiert keinen LINEA-Block.

Betrifft: Zeilenbreite (`linesize`), `ls`-Spalten (`screen_cols/13`), `more`
(`screen_rows-1`), `help`-Pause. Im Messlauf bei 1920×1080/8×16 sind das
240×67 Zellen (**Schluss** aus Aufloesung und Zellgroesse, nicht
nachgemessen).

### D5 — Aufloesungswechsel (`mode con res=`, Start-/Endwechsel)

Hardwareanteil (`CONF_ATARI_HARDWARE`, `CONF_WITH_TT_SHIFTER`, 05 §1/§4).
Ohne `_VDO` und mit `Getrez() = 4` taeuscht EmuCON einen Wechsel nur vor
(05 §4). Optionen: (a) 1:1 mitnehmen (wirkungslos, aber textgleich);
(b) `res=` als `invalid parameter` ablehnen und die `Resolution:`-Zeile
weglassen (entspricht `_VDO = VIDEL`/Falcon: `valid_res` immer FALSE,
`original_res = -1`, `cmdmain.c:307-308`); (c) den Wechselteil
(`change_res`, `valid_res`, `requested_res`, `CHANGE_RES`) ganz weglassen
— dokumentierte Abweichung, hardwarenah („Hardwareebene frei",
`docs/COMPATIBILITY.md`). Research-Sicht: (b) ist das Falcon-Profil
(Referenzbau laut `emutos-target-profile-falcon-tos4`) und bleibt in `cli/`-
Code ausdrueckbar; ob dafuer ein `_VDO`-Cookie noetig ist, steht
nicht fest (NewTOS laesst ihn weg, `cookie.rs:217-220`).

### D6 — Programmendungen und `is_graphical`

EmuCON kennt `app/gtp/prg/tos/ttp` (`cmdutil.c:271-287`), grafisch `app/gtp/prg`
(`cmdexec.c:176-194`). NewTOS-native Programme: `.NAP/.NCP/.NTP` (+ `.NAC`
Accessory, nicht startbar). Fragen: (1) Abbildung `nap`→grafisch
(Cursor aus), `ncp`/`ntp`→Text; `.NAC` ausschliessen?; (2) die m68k-Endungen
weiter akzeptieren (sie scheitern beim Laden mit einem Ladefehler,
`kernel/src/launch.rs:379-382`, Kommentar zu `.PRG`/`.TOS`) oder ganz ausblenden; (3) Tab-Vervollstaendigung des
Befehlsworts nutzt dieselbe Menge (`cmdedit.c:454-460`). Auswirkung: das
Einzige im Code, das die Endungsliste fest verdrahtet, sind
`program_extension` und `is_graphical`.

### D7 — Bekannte Eigenheiten/Defekte von `cli/`: 1:1 oder berichtigen

Alle **(Schluss)** aus gelesenem Code, nicht ausgefuehrt (Referenzlauf in
Hatari/QEMU waere der Beleg, vgl. Notiz „TT-Referenzen per Hatari"):

| # | Eigenheit | Beleg |
|---|---|---|
| E1 | Ctrl-C-Zeile landet im Verlauf, `history_num` bleibt geblaettert | `cmdedit.c:78-82,101`, `cmdmain.c:124-127` |
| E2 | Tab kuerzt den Puffer am Cursor, `len` bleibt | `cmdedit.c:252` |
| E3 | Shift+Pfeil fuegt `8/2/4/6` ein | `scancode.h:36-39`, `cmdedit.c:281-282,65-98` |
| E4 | `redir_name` bleibt in 4 Wegen stehen → naechster Befehl wird umgelenkt | 03 §3 |
| E5 | `arglist[30]`, `redir_name[128]`, `user_path[128]`: ohne Laengenpruefung | `cmdparse.c:58,56`, `cmdint.c:486`, `cmdmain.c:37,44-45` |
| E6 | `rm -q`: `Fdelete(Muster)` kann die abgelehnte Datei loeschen | `cmdint.c:531-543`, `bdos/fsopnclo.c:588-612` |
| E7 | `rm a b`: `b` stillschweigend ignoriert | `cmdint.c:157,513-531` |
| E8 | `cp`/`mv` auf sich selbst: Textvergleich, nicht Identitaet → Quelle wird abgeschnitten | `cmdint.c:850-870` |
| E9 | `strip_quotes` kuerzt das letzte Zeichen, auch wenn es kein `"` ist | `cmdmain.c:207-220` |
| E10 | `Fopen`/`Fcreate`-Fehler in `cp` ohne ` ... ` vor der Meldung | `cmdint.c:863-873` |
| E11 | `wrap`-Status ist eine interne Variable, nicht der Terminalzustand | `cmdmain.c:97`, `cmdint.c:619-637` |
| E12 | Ausgabe der Meldungen `errmsg` fuer **positive** Exit-Codes (`error code N`) | 03 §6 |

Optionen je Zeile: (a) bugkompatibel nachbilden (in Rust ist ein
Pufferueberlauf **nicht** nachbildbar — E5 braucht in jedem Fall eine
Festlegung: Fehlermeldung oder Kuerzung), (b) mit markierter Abweichung
berichtigen. Die Projektlinie ist „angleichen" (Notiz
`no-rationalizing-emutos-deviations`), E4/E6/E8 koennen aber Daten kosten;
deshalb Frage an den Anwender statt stiller Entscheidung.

### D8 — Crate-Aufteilung und gemeinsame Bindungen

(a) Bibliothek `programs/<x>/` + schmaler Binaer-Crate (Muster NEWDESK; erlaubt
Hosttests der reinen Logik: Tokenizer, Editor-Zustandsmaschine, `fixup_filespec`,
`decode_date_time`), (b) ein Crate programs/emucon/. Die Bindungen
(`Pexec`, `Fdup/Fforce`, `Dgetpath`, `Fgetdta`, `Sversion`, BIOS,
XBIOS-Teile) gehoeren in `newtos-gem` („die einzige Kopie", `programs/newtos-gem/src/lib.rs:13-16`); das
ist Entwicklerarbeit und Teil des Tickets, kein Entscheid. Research-Sicht: (a).

### D9 — Banner-Version

EmuCON zeigt `version[]` (EmuTOS-Versionstext, `cmdmain.c:93`,
`version.mk:25-40`; `version.mk:40` setzt `VERSION` aus dem Datum). Frage: was
steht bei NewTOS dort — NewTOS-Version (heute `kernel/src/shell.rs:123` hartkodiert
„0.6.1"), der EmuCON2-Name? Der Satzbau `Welcome to EmuCON2 version <v>`
bleibt, wenn der Name EmuCON2 bleibt.

### D10 — Frueh-Konsole (Esc auf dem Startbildschirm)

Weg B aus 01 §1 hat in NewTOS heute keinen Verbraucher
(`kernel/src/initinfo.rs:535-538`). Mit einem EL0-EmuCON waere er machbar
(Start vor `run_auto_folder`). Nicht Teil von CON03; **nur als Frage**: gleich
mitnehmen oder eigenes Ticket?

## 4. Offene technische Fragen (keine Entscheidungen, im Bau zu klaeren)

1. Liefert `Bconin(2)` aus EL0 `(Scancode<<16)|ASCII` mit den Atari-Scancodes
   (Pfeile `0x48/0x50/0x4b/0x4d`, Ctrl-Pfeil `0x73/0x74`, Delete `0x53/0x7f`,
   Backspace `0x0e/0x08`, Tab `0x0f/0x09`)? Nicht in `kernel/src/keymap.rs`
   geprueft.
2. Pumpt `Bconin(2)` aus EL0 beim Blockieren die Eingabe (USB/IKBD)? (Notiz
   „Live mouse + dead UI = app parked in an AES call".)
3. Gibt `Pexec` aus EL0 den zero-extended Pterm-Code zurueck (PROC05 belegt es
   fuer die Kernel-Seite)? (03 §8)
4. Liefert `Fgetdta()` in einem frisch gestarteten EL0-Prozess die
   Kommandozeilen-Flaeche wie `bdos/proc.c:374-375` und bleibt sie ueber
   `Fsfirst`-Folgen stabil?
5. Laeuft der Skript-Injektor (Tests „per Injektor", CON03) waehrend
   `Bconin(2)`? Notiz „Injektor laeuft nicht waehrend Cconin": eine
   `key`-Zeile allein friert einen Messlauf ein, wenn der Leser eine eigene
   Schleife hat (`console_read_key_blocking`). **Offen**, ob `Bconin(2)` aus
   EL0 betroffen ist.
6. `Dfree`/`Fsfirst`/`Dgetpath`-Ergebnisse unter den NewTOS-Datentraegern
   (FAT16): Spezifikationen `docs/api/gemdos/directory/36-dfree.md` etc.;
   nicht gegengelesen.
7. Verhalten bei Zeilenlaenge: `linesize` richtet sich nach der Spaltenzahl
   (02 §1); bei sehr breitem Schirm (240 Spalten) begrenzt `MAX_LINE_SIZE`
   auf 198 Zeichen.

## 5. Folgearbeiten ausserhalb von CON03 (je eine Zeile)

* `docs/api/gemdos/process/_parent-and-environment.md` und
  `TASK/PROC03.md` beschreiben den Stellvertreter-EmuCON-PD; nach dem Umbau
  nachziehen.
* `tos-api/src/bdos/process/standin.rs:445-465` (`enter/leave_emucon`) und
  `kernel/src/lib.rs:488-503` entfallen, wenn D1 (a) gilt.
* `docs/api/gemdos/version/30-sversion.md:35-52` nennt den Standardwert
  `0x2000`, `tos-api/src/gemdos/version.rs:14` liefert `0x3000` — Spec und Code
  stimmen nicht ueberein (nicht Teil von CON03).
* `kernel/src/shell.rs:69` (`pub fn run() -> !`) hat im Gelesenen keinen Aufrufer.
