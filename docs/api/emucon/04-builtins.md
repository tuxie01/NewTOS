# EmuCON2 — Eingebaute Befehle

Quelle: `cli/cmdint.c` (1301 Zeilen), `cli/cmdutil.c`, `cli/cmd.h`.
Legende: [README](README.md). GEMDOS-Aufrufe sind in **Aufrufreihenfolge**
notiert (`Cmd(Argumente)`), `Bconin/Bconout/Bconstat` mit Geraet 2 (CON,
`cmd.h:83-85,194-196`). Alle Zahlen in `file:line` sind
`references/emutos/cli/…`.

**Ausgabewege.** `output()`/`outputnl()` sind **umlenkbar** (`cmdint.c:1192-1206`):
bei `redir_handle >= 0` `Fwrite(handle, strlen, s)`, sonst `message()` =
`Bconout(2, c)` je Zeichen (`cmdutil.c:35-41`); `outputnl` haengt `\r\n` an.
`message()`/`messagenl()` gehen **immer** auf den Schirm. Fehler (`errmsg`) immer
auf den Schirm. Zeilenende ist stets `\r\n`.

## 1. Befehlstabelle (`cmdint.c:140-163`)

Argumentzahlen zaehlen **ohne** den Befehl (`argc-1`, `:200`); Pruefung nach
der `-h`-Abkuerzung (03 §5). Der Eintrag `exit` ist `LOOKUP_EXIT`.

| Befehl | Synonym | min | max | Hilfe (Zeile 1 = Syntax) | Funktion |
|---|---|---|---|---|---|
| `cat` | `type` | 1 | 255 | `<filespec> ...` | `output_files(…,0)` |
| `cd` | – | 0 | 1 | `[<dir>]` | `run_cd` |
| `chmod` | – | 2 | 2 | `<mode> <filename>` | `run_chmod` |
| `cls` | `clear` | 0 | 0 | `` (leer) | `run_cls` |
| `cp` | `copy` | 2 | 2 | `<filespec> <dest>` | `copy_move(…,0)` |
| `echo` | – | 0 | 255 | `<string> ...` | `run_echo` |
| `exit` | – | 0 | 0 | `` | `LOOKUP_EXIT` |
| `help` | – | 0 | 1 | `[<cmd>]` | `run_help` |
| `ls` | `dir` | 0 | 2 | `[-l] <path>` | `run_ls` |
| `mkdir` | `md` | 1 | 1 | `<dir>` | `run_mkdir` |
| `mode` | – | 1 | 4 | `con[:] [res=<r>] [delay=<m>] [rate=<n>]` | `run_mode` |
| `more` | – | 1 | 1 | `<filespec>` | `output_files(…,1)` |
| `mv` | `move` | 2 | 2 | `<filespec> <dir>` | `copy_move(…,1)` |
| `path` | – | 0 | 1 | `[<searchpath>]` | `run_path` |
| `pwd` | – | 0 | 0 | `` | `run_pwd` |
| `ren` | – | 2 | 2 | `<oldname> <newname>` | `run_ren` |
| `rm` | `del` | 1 | 2 | ` [-q] <filespec>` (fuehrendes Leerzeichen im Quelltext) | `run_rm` |
| `rmdir` | `rd` | 1 | 1 | `<dir>` | `run_rmdir` |
| `show` | – | 0 | 1 | `[<drive>]` | `run_show` |
| `version` | – | 0 | 0 | `` | `run_version` |
| `wrap` | – | 0 | 1 | `[on|off]` | `run_wrap` |

Dazu `X:` (Laufwerkswechsel, nicht in der Tabelle, `cmdint.c:174-176`,
`run_setdrv` `:561-570`). Die Zahl **21** ist die Zahl der Tabelleneintraege
vor dem Endmarker (`:141-161`); mit Synonymen sind es 29 Namen.
`type` und `dir` gibt es nur als Synonyme von `cat` und `ls`; **`quit` und `ver` gibt es in EmuCON nicht** (Gegenstueck: NewTOS-Kernelkonsole, `kernel/src/shell.rs:121,123`).

Hilfetexte (Zeilen 2ff, englisch, `cmdint.c:72-134`) — **fachlich noetig fuer
1:1**, deshalb vollstaendig; sie sind EmuTOS-Quelltext (GPL-2+, nicht
Atari-Dokumentation, vgl. `references/README.md`-Lizenzabschnitt, der die
SDA-Spezifikationen betrifft):

| Befehl | Zeilen nach der Syntaxzeile |
|---|---|
| `cat` | `Copy <filespec> ... to standard output` |
| `cd` | `Change current directory to <dir>` / `or display current directory` |
| `chmod` | `Change attributes for <filename>` / `<mode> specifies the new attribute(s):` / `r=read-only  h=hidden  s=system  -=none` |
| `cls` | `Clear screen` |
| `cp` | `Copy files matching <filespec> to <dest>` / `If <filespec> matches multiple files,` / `<dest> must be a directory` |
| `echo` | `Copy <string> ... to standard output` / `Strings may be surrounded by ""` |
| `exit` | `Exit EmuCON2` |
| `help` | `Get help about <cmd> or list available commands` / `Use HELP ALL for help on all commands` / `Use HELP EDIT for help on line editing` |
| `ls` | `List files (default terse, horizontal)` / `Specify -l for detailed list` |
| `mkdir` | `Create directory <dir>` |
| `mode` | `Set or display console settings:` / `res for screen resolution (0,1)[ST] (0,1,2,4,7)[TT];` / `delay/rate for the keyboard` |
| `more` | `Copy <filespec> to standard output,` / `pausing every screenful` |
| `mv` | `Copy files matching <filespec> to <dir>,` / `then delete input files` |
| `path` | `Set search path for external programs` / `or display current search path` |
| `pwd` | `Display current drive and directory` |
| `ren` | `Rename <oldname> to <newname>` |
| `rm` | `Delete files matching <filespec>` / `Specify -q to be prompted each time` |
| `rmdir` | `Delete directory <dir>` |
| `show` | `Show info for <drive> or current drive` |
| `version` | `Display GEMDOS version` |
| `wrap` | `Set line wrap or show current status` |
| `help edit` | `up/down arrow = previous/next line in history` / `left/right arrow = previous/next character` / `control-left/right arrow = previous/next word` (`:131-134`) |

## 2. Fehlermeldungen (`cmdutil.c:55-112` `errmsg`, Codes `cmd.h:164-180`)

`execute` ruft `errmsg(rc)` nach **jedem** Befehl (`cmdmain.c:175`); `rc == 0`
schweigt. Die Meldung geht mit `messagenl` auf den Schirm.

| `rc` | Konstante | Text |
|---|---|---|
| -33 | `EFILNF` | `file not found` |
| -34 | `EPTHNF` | `path not found` |
| -35 | `ENHNDL` | `no more file handles` |
| -36 | `EACCDN` | `can't create file` |
| -39 | `ENSMEM` | `not enough memory` |
| -46 | `EDRIVE` | `invalid drive` |
| -100 | `USER_BREAK` | `interrupted` |
| -101 | `NOT_DIRECTORY` | `invalid path` |
| -102 | `DISK_FULL` | `can't write to disk (full?)` |
| -103 | `CMDLINE_LENGTH` | `cmdline too long` |
| -104 | `DIR_NOT_EMPTY` | `directory not empty` |
| -105 | `CANT_DELETE` | `can't delete file (read-only?)` |
| -126 | `INVALID_PARAM` | `invalid parameter` |
| -127 | `WRONG_NUM_ARGS` | `wrong number of arguments` |
| sonst | – | `error code <rc %ld>` |

`-49 ENMFIL` hat **keinen** eigenen Text; `Fsnext`-Schleifen machen daraus
0 (`cmdint.c:406,546,681,912`). `-125 CHANGE_RES` wird vor `errmsg` verbraucht
(`cmdmain.c:170-171`). Die Zahlen -100…-105 und -125…-127 sind
**EmuCON-intern** (`cmd.h:171-180`), keine GEMDOS-Codes.

## 3. Gemeinsame Hilfen

* **`user_input(c)`** (`:1282-1301`): `c < 0` → Taste per `Bconin(2)` holen
  (LOBYTE). Ctrl-C, `Q`, `q` → -1 (Abbruch). Ctrl-S (`0x13`) → Pause: weiter
  mit Ctrl-Q (`0x11`), Ctrl-C bricht ab (-1). Jede andere Taste wird
  verbraucht und ignoriert. Wird nur gerufen, wenn `Bconstat(2)` Eingabe meldet.
* **`user_break()`** (`:1265-1274`): `Bconin(2)`; nur Ctrl-C → -1.
* **`getyn()`** (`:1167-1176`): gibt ` (y/N)?` aus, **eine** Taste per
  `Bconin(2)` (`LOBYTE | 0x20`, also `Y`≡`y`), dann `\r\n`; liefert das Zeichen.
  Nur `'y'` bestaetigt.
* **`blank_line()`** = `ESC l` (`cmd.h:189`): Zeile loeschen, Cursor in Spalte
  0 (`bios/vt52.c:733-742`).
* **`is_valid_drive(letter)`** (`:1105-1116`): `drvbits = Dsetdrv(Dgetdrv())`;
  `n = (letter|0x20) - 'a'`; gueltig, wenn `0 <= n < BLKDEVNUM (26)` und
  Bit `n` gesetzt.
* **`extract_path(dest, src)`** (`:951-964`): kopiert `src`, liefert den
  Zeiger hinter dem letzten `\` oder `:`.
* **`fixup_filespec`**, **`make_absolute`**, **`check_path_component`**:
  siehe 4.9 / 4.5.

## 4. Die Befehle

### 4.1 `cat` / `type` / `more` (`output_files`, `cmdint.c:646-693`; `run_cat` `:207`, `run_more` `:465`)

Syntax `cat <filespec> ...` (1–255 Argumente), `more <filespec>` (genau 1).

Ablauf: `Malloc(16384)` (`IOBUFSIZE`, `cmd.h:101`) — fehlt der Speicher
`ENSMEM`; je Argument: `Fsfirst(arg, 0x07)` (RO|Hidden|System, keine Ordner);
je Treffer: Name = Pfadteil des Arguments + `dta->d_fname` (`extract_path`,
`:659-661`), `Fopen(name, 0)`, `linecount = 0`, Schleife `Fread(h, 16384, buf)` →
`outputbuf` → (Pruefung `rc != n` → `DISK_FULL`) bis Lesen 0 liefert,
`Fclose`, `Fsnext`; am Ende `Mfree`. `ENMFIL` → 0; jeder andere negative Code
bricht ab und wird gemeldet (`file not found` auch, wenn **kein** Treffer
existiert: `Fsfirst` liefert `EFILNF`/`EPTHNF`).

`outputbuf(s, len, paging)` (`:1212-1260`):

* **Bildschirm** (`redir_handle < 0`): je Byte; vor jedem Byte `if (constat())
  if (user_input(-1)) return USER_BREAK;`. `\n` ohne vorangehendes `\r`
  (im selben Puffer) wird als `\r\n` ausgegeben (Un*x→TOS, `:1224-1226`;
  **(Schluss)** ein `\r\n`-Paar, das genau auf eine 16-KiB-Pufferkante faellt,
  bekommt ein zusaetzliches `\r`, weil `cprev` je Aufruf auf 0 steht, `:1215`).
  Es wird **jedes** Byte ausgegeben (Steuerzeichen, auch `ESC`, gehen roh an
  den Schirm).
* **`more` (`paging = 1`)**: nach jedem `\n` `++linecount`; ist
  `linecount >= screen_rows - 1`: `--More--`; Taste: `\r` = naechste Zeile
  (Zaehler bleibt), Leertaste = naechste Seite (`linecount = 0`), `q`/`Q`/Ctrl-C
  = Abbruch (`user_input`, `:1239-1242`: `blank_line()` + `USER_BREAK`), Ctrl-S
  pausiert; andere Tasten wiederholen die Frage ohne Wirkung. Danach
  `blank_line()` (`ESC l`) loescht die Meldung (`:1244`).
* **Umgelenkt**: `Fwrite(handle, len, s)`; bei Erfolg Pruefung
  `constat()`/`user_break()` (nur Ctrl-C) (`:1252-1259`). **Kein**
  Seitenumbruch, keine `\n→\r\n`-Wandlung.

`more` unterdrueckt `USER_BREAK` (keine Meldung `interrupted`, `:689-690`),
`cat` zeigt sie. `linecount` wird je Datei auf 0 gesetzt (`:666`).

### 4.2 `cd` (`:212-258`)

`cd` ohne Argument: `get_path(path, 0)` → `outputnl(path)` (z. B. `C:\FOO`),
Rueckgabe `rc` von `Dgetpath`.
Sonst: `current_drive = Dgetdrv()`; hat das Argument an Position 1 ein `:`:
`is_valid_drive(*p)` sonst `EDRIVE`; `cd x:` (Argument genau `x:`) →
`get_path(path, drive+1)` + `outputnl` + Rueckgabe (**kein** Laufwerkwechsel);
sonst `Dsetdrv(temp_drive)`, Argument hinter `x:` weiterreichen.
Dann `Dsetpath(p)`, **`Dsetdrv(current_drive)`** (stets zurueck, auch bei
Fehler), Rueckgabe `rc`.
Kommentar `:225-231`: wird vor dem Setzen kurz auf das Ziellaufwerk gewechselt,
weil manche BDOS-Versionen Laufwerksbuchstaben in Pfaden falsch behandeln.
`cd` aendert also **nur das Verzeichnis**, nie das aktuelle Laufwerk
(Laufwerk: `X:`, 4.17).

### 4.3 `chmod <mode> <filename>` (`:260-289`)

`<mode>` besteht aus den Zeichen `r` (0x01), `h` (0x02), `s` (0x04), `-`
(nichts); **nur Kleinbuchstaben**. Anderes Zeichen → `messagenl("invalid mode
argument")`, Rueckgabe 0 (keine weitere Meldung, **nichts** wird geaendert).
Sonst `Fattrib(filename, 1, attr)` (setzen; **ersetzt alle Attribute** durch
die genannten, Archiv-Bit geht verloren); `rc < 0` → `rc`, sonst 0.

### 4.4 `cls` / `clear` (`:291-296`)

`ESC E` (Bildschirm loeschen, Cursor Home) per `Bconout(2)`; nicht umlenkbar.

### 4.5 `cp` / `copy`, `mv` / `move` (`copy_move`, `:786-918`; `run_cp` `:298`, `run_mv` `:470`)

Syntax `cp <filespec> <dest>` / `mv <filespec> <dir>` (genau 2 Argumente).

1. `inptr = extract_path(inname, argv[1])`, `outptr = extract_path(outname,
   argv[2])` (`:794-795`).
2. Quelldateien zaehlen: `Fsfirst(inname, 0x07)` + `Fsnext`-Schleife → `n`
   (`:798-799`).
3. Ziel pruefen: `make_absolute(fullname, outname)` (`:734-781`: Laufwerk
   davor (`Dgetdrv`), relativer Pfad mit `Dgetpath(…, laufwerk+1)`
   ergaenzt, `.` ignoriert, `..` ein Verzeichnis hoch (nicht ueber die
   Wurzel), ein anderer mit `.` beginnender Name → `EPTHNF`);
   danach `check_path_component(fullname)` (`:1125-1165`: Laufwerk gueltig?
   `X:` und `X:\` = Verzeichnis; `?`/`*` → `EPTHNF`; abschliessender `\`
   wird fuer die Pruefung abgeschnitten; `Fsfirst(name, 0x17)`;
   Treffer, der kein Ordner ist → `NOT_DIRECTORY`). `rc == 0` → Ziel ist
   Verzeichnis (`output_is_dir`); `NOT_DIRECTORY` oder `EFILNF` → Ziel ist
   eine (neue oder bestehende) Datei, `rc = 0` (`:804-810`).
4. Fehler (`rc < 0`) **oder** (`n > 1` und Ziel kein Verzeichnis): `message(outname)`
   (die **Eingabe** des Benutzers), `messagenl(" is not a directory")`,
   Rueckgabe 0 (`:816-821`).
5. Ab hier arbeitet es mit dem absoluten Namen; ist das Ziel ein Ordner, wird
   ein `\` angehaengt, wenn der Name nicht auf `\`/`:` endet (`:826-832`).
6. `Malloc(16384)`; fehlt → `ENSMEM`.
7. Fuer jede Quelldatei (`Fsfirst(inname, 0x07)`/`Fsnext`): bei
   `constat()` `user_input(-1)` → `USER_BREAK`; Name einsetzen
   (`strcpy(inptr, d_fname)`, bei Ordnerziel auch `outptr`); ist
   `strequal(inname, outname)`: `<in> and <out> are the same file` und
   weiter (`:850-856`) — der Vergleich ist **textuell** (Eingabename wie
   getippt+Dateiname gegen den **absoluten** Zielnamen), **(Schluss)** ein
   Kopieren auf sich selbst unter abweichender Schreibweise wird so nicht
   erkannt;
   Ausgabe `Copying <in> to <out>` (ohne Zeilenende);
   `Fopen(in, 0)` — Fehler → Schleife verlassen **ohne** ` ... ` (die
   Fehlermeldung haengt dann an derselben Zeile, `:863-866`);
   `Fcreate(out, 0)` — Fehler → `Fclose(in)`, verlassen (`:868-872`);
   Kopierschleife: bei `constat()` nur Ctrl-C (`user_break`) → `USER_BREAK`;
   `Fread(in, 16384, buf)`; `Fwrite(out, n, buf)`; kurzes Schreiben → `DISK_FULL`
   (`:875-891`); `Fclose(in)`, `Fclose(out)`;
   bei Fehler `Fdelete(out)` (halbfertige Datei weg, `:895-897`);
   **`mv`**: nur bei erfolgreicher Kopie `message(" ... deleting ")` +
   Quellname (nur der Dateiname) + `Fdelete(in)` (`:899-903`);
   bei `rc < 0`: `message(" ... ")`, Abbruch; sonst `messagenl(" ... done")`.
8. `ENMFIL` → 0; `Mfree`; Rueckgabe `rc`.

Beispielausgabe (Schluss, aus den `message`-Aufrufen): `Copying A.TXT to C:\X\A.TXT ... done`.

### 4.6 `echo` (`:303-315`)

Die Argumente (Anfuehrungszeichen gestrippt, 03 §2), durch je **ein** Leerzeichen
getrennt, plus `\r\n`, per `output()` (umlenkbar). Keine Optionen. `echo` ohne
Argumente → leere Zeile. **`echo -h` zeigt die Hilfe**, nicht `-h` (03 §5).

### 4.7 `exit` (`:147`)

Siehe 01 §4. Kein eigener Code; `LOOKUP_EXIT`.

### 4.8 `help` (`:317-359`, `:966-1029`)

* `help` (ohne Argument): `Builtin commands:` dann je Tabelleneintrag eine
  Zeile `  <name>` bzw. `  <name>/<synonym>` (`output`, umlenkbar).
* `help edit` (case-insensitiv): je Zeile von `help_edit` `  <text>` (`:131-134,336-342`).
* `help <befehl>` / `help all`: fuer jeden Eintrag, auf den `help_wanted`
  passt (`all`, `name` oder `synonym`, case-insensitiv, `:1018-1029`):
  `help_display` (`:966-982`): Zeile `<name>[ or <synonym>] <Syntaxzeile>` (**ein**
  Leerzeichen zwischen Name/Synonym-Teil und Syntax, auch wenn diese leer ist),
  dann `  <Hilfezeile>` je weitere Zeile. **Unbekannter Befehl** → keine
  Ausgabe, kein Fehler (`rc` 0).
* Seitenpause, **nur wenn nicht umgelenkt** (`redir_handle < 0`): vor jedem
  Eintrag `lines += help_lines(p)` (Zahl der Hilfestrings inkl. Syntaxzeile);
  ist `lines >= screen_rows` → `CR to continue ...` (`message`), warten auf `\r`
  (weiter, `lines = 0`) oder Ctrl-C (Abbruch der Ausgabe), dann `blank_line()`
  (`:344-356`, `:995-1016`).
* `<befehl> -h` ist dasselbe wie `help <befehl>` (03 §5).

### 4.9 `ls` / `dir` (`:361-410`, `:1031-1066`, `:1068-1103`)

Syntax `ls [-l] [<path>]` (0–2 Argumente). Ist das erste Argument `-l`
(case-insensitiv), `detail = 1` und die Argumente rutschen eins weiter; ein
**zweites** Argument, das nicht `-l` ist, wird still **ignoriert**
(`ls a b` listet `a`).

* `names_per_line = screen_cols / 13` (`:377`).
* `filespec = <Argument>` oder leer; `fixup_filespec(filespec)` (`:1031-1066`):
  1. leer → `*.*`;
  2. endet auf `:` → aktuellen Pfad des Laufwerks (`Dgetpath`) und `\` anhaengen;
  3. endet auf `\` → `*.*` anhaengen;
  4. **alles in Grossbuchstaben** (`strupper`);
  5. enthaelt der letzte Pfadteil ein `*` → fertig (nur `*` zaehlt, nicht `?`);
  6. sonst `Fsfirst(filespec, 0xff)`: gefunden und **kein** Ordner → fertig
     (einzelne Datei); in allen anderen Faellen (Ordner **oder nicht gefunden**)
     wird `\*.*` angehaengt (`:1055-1065`). Ein nicht vorhandener Name wird
     also als Ordner behandelt und endet in `path not found`
     (**Schluss**).
* Detailmodus: zuerst `Listing of <filespec>` (`outputnl`, die
  **normalisierte** Grossschreibung).
* `Fsfirst(filespec, 0x17)` (RO|Hidden|System|Ordner) und `Fsnext`; je Eintrag
  bei `constat()` `user_input(-1)` → `USER_BREAK`.
* **Knapp** (Standard): Eintraege, deren Name mit `.` beginnt, werden
  **uebersprungen**; jeder Name wird auf **13 Zeichen** mit Leerzeichen
  aufgefuellt (`padname`, `:1068-1079`), nach `names_per_line` Namen
  `\r\n`; Restzeile am Ende abschliessen (`:397-404`). Keine
  Kennzeichnung von Ordnern/Attributen.
* **Detail** (`-l`): je Eintrag (**auch `.` und `..`**, kein Filter)
  `display_dta_detail` (`:1081-1103`): `%-13.13s` Name, dann
  `decode_date_time` (4.21), dann bei Ordner `<dir>`, sonst `%15lu` Laenge
  und die Attributbuchstaben `r`, `h`, `s` **ueber die fuehrenden Leerzeichen
  des Laengenfelds geschrieben**: Beginn bei Spalte 1 des 15-stelligen Feldes,
  `r` (0x01) und `h` (0x02) rueckt je eins weiter, `s` (0x04) nicht; ist keines
  der drei Bits gesetzt, `-` an Spalte 1 (`:1091-1100`). Das Archiv-Bit
  (0x20) wird nicht angezeigt. Die Zeile geht mit `outputnl`.
* Ende: `ENMFIL` → 0; sonst Rueckgabe des `Fsfirst`/`Fsnext`-Fehlers (z. B.
  `file not found`, `path not found`).

### 4.10 `mkdir` / `md` (`:412-415`)

`Dcreate(argv[1])`; Rueckgabe das Ergebnis. (`EACCDN` z. B. bei vorhandenem
Namen erscheint als `can't create file`.)

### 4.11 `mode` (`:417-463`)

Syntax `mode con[:] [res=<r>] [delay=<m>] [rate=<n>]`. Das erste Argument
muss `con` oder `con:` sein (case-insensitiv), sonst `INVALID_PARAM`.

* Nur `mode con`: `Status for CON:`; ist `current_res >= 0`
  `  Resolution:     %3d`; dann `Kbrate(-1, -1)` abfragen und
  `  Keyboard delay: %3d` (`HIBYTE`), `  Keyboard rate:  %3d` (`LOBYTE`) —
  jeweils `outputnl` (umlenkbar). Rueckgabe 0.
* Mit Parametern (`res=`/`delay=`/`rate=`, Praefix case-insensitiv):
  Werte per `getword` (nur Ziffern, ≤ 32767, sonst -1,
  `cmdutil.c:147-162`); `res` zusaetzlich `valid_res(res)`; `delay`/`rate` < 0 →
  `INVALID_PARAM`; unbekanntes Argument → `INVALID_PARAM`. Erst **nach** der
  Schleife wirken die Werte: `Kbrate(delay, rate)` (nicht gesetzte = -1), und
  `res != current_res` → `requested_res = res`, Rueckgabe `CHANGE_RES`
  (Wechsel in `cmdmain`, Abschnitt 01/05).

### 4.12 `path` (`:475-490`)

Ohne Argument: `message(" ")`, dann `user_path` oder `(empty)` mit
`messagenl` — also **fuehrendes Leerzeichen**, **nicht umlenkbar**. Mit
Argument: `strcpy(user_path, argv[1])` (`user_path[128]`, **keine
Laengenpruefung**, **Schluss**: Ueberlauf moeglich); Komponenten durch `;` oder
`,` getrennt (03 §6). Die Umgebung (`PATH=`) bleibt unveraendert.

### 4.13 `pwd` (`:492-501`)

`get_path(buf, 0)` → `outputnl(buf)`; Rueckgabe `Dgetpath`-Code.

### 4.14 `ren <oldname> <newname>` (`:503-506`)

`Frename(0, argv[1], argv[2])`; Rueckgabe das Ergebnis. (Kein Pfad-/Ziel-Check
in EmuCON.)

### 4.15 `rm` / `del` (`:508-550`)

Syntax `rm [-q] <filespec>` (1–2 Argumente). `-q` (case-insensitiv) **nur als
erstes** Argument; `rm -q` allein → nichts (`:522-523`, Rueckgabe 0). Ohne `-q`
und mit zwei Argumenten wird das **zweite stillschweigend ignoriert**
(`rm a b` loescht nur `a`; **Schluss** — `maxargs 2`, `:157`).

* Enthaelt das Muster `*`/`?` (`has_wildcard`): `Delete ALL matching files`
  (`message`) + `getyn()`; nur `y` → weiter, sonst Ende mit 0 (`:525-529`).
* Schleife `Fsfirst(*argv, 0x17)`/`Fsnext`: mit `-q` je Datei
  `Delete file <name>` (`message`, `dta->d_fname`) + `getyn()`; bei nein
  `continue`. Dann **`Fdelete(*argv)`** — das ist das **Muster**, nicht
  der gefundene Name (`:538`). Fehler `EACCDN` → `CANT_DELETE`
  (`can't delete file (read-only?)`), Abbruch.
* `ENMFIL` → 0.

**(Schluss — Auffaelligkeit, nicht ausgefuehrt):** `Fdelete` mit Muster
loescht den **ersten** passenden Eintrag (`bdos/fsopnclo.c:588-612`:
`scan(dn, s, FA_NORM, &pos)` behandelt `*`/`?` als Joker; `FA_NORM` =
Archiv|System|Hidden|RO, `bdos/fs.h:195`). Ohne `-q` fuehrt das trotzdem zum
Loeschen aller Treffer. **Mit `-q` kann es die falsche Datei treffen:** wird
beim ersten Treffer `n` geantwortet, trifft das darauf folgende `Fdelete(…)`
genau diese erste Datei, obwohl sie abgelehnt wurde. Ordner (`0x10`) werden
von `scan` nicht erfasst. Entscheidung: [06 D7](06-mapping-and-decisions.md).

### 4.16 `rmdir` / `rd` (`:552-559`)

`Ddelete(argv[1])`; `EACCDN` → `DIR_NOT_EMPTY` (`directory not empty`), sonst `rc`.

### 4.17 `X:` Laufwerkswechsel (`:561-570`)

Nur wenn die Zeile aus **einem** Wort von genau zwei Zeichen mit `:` an
Position 1 besteht (`cmdint.c:174-176`). `is_valid_drive(argv[0][0])` sonst
`EDRIVE`; `strlower`; `Dsetdrv(letter - 'a')`.

### 4.18 `show [<drive>]` (`:572-604`)

Ohne Argument `drive = Dgetdrv()`, sonst Kleinbuchstabe von `argv[1][0]` minus
`'a'` (**keine Gueltigkeitspruefung des Zeichens**). `Dfree(info, drive+1)`
(vier 32-Bit-LONGs: `[0]` freie Cluster, `[1]` Cluster gesamt, `[2]` Bytes/Sektor,
`[3]` Sektoren/Cluster — `bdos/fsfat.c:337-367`); Fehler → `EDRIVE`.
`bpc = info[2]*info[3]`. Ausgabe (`outputnl`, `show_line` =
`"%s%10lu"`, `:1178-1184`):

```
Allocation info for drive <X:>
  Size (bytes):   <total*bpc>
  Bytes used:     <(total-free)*bpc>
  Bytes free:     <free*bpc>
  Total clusters: <total>
  Free clusters:  <free>
  Sectors/cluster:<spc>
  Bytes/sector:   <bps>
```

Die Zahlen sind `ULONG` (32 Bit): groessere Datentraeger laufen wie in
EmuTOS ueber (Schluss; die NewTOS-Helfer rechnen mit 64 Bit,
`programs/newtos-gem/src/gemdos.rs:318-325`, aber `show` ist EmuTOS-`ULONG`).

### 4.19 `version` (`:606-617`)

`n = Sversion() >> 8`; Ausgabe `0.` + zwei Ziffern aus den Nibbles von `n`
(`(n>>4)+'0'`, `(n&0xf)+'0'`), z. B. `0.20` fuer `Sversion = 0x2000`
(EmuTOS `GEMDOS_VERSION`, `include/config.h:1589-1591`). **NewTOS liefert
`0x3000`** (`tos-api/src/gemdos/version.rs:14`): `0.30`. Hat ein Nibble einen
Wert ≥ 10, entsteht ein Nicht-Ziffernzeichen (kein Pruefcode). Wohlgemerkt:
`version` zeigt die **GEMDOS**-, nicht die Systemversion.

### 4.20 `wrap [on|off]` (`:619-637`)

Ohne Argument: `ON` oder `OFF` (`messagenl`, nicht umlenkbar) aus der
**internen Variable** `linewrap` — sie beginnt bei 0 und wird **nicht**
mit dem Zustand des Terminals abgeglichen (`cmdmain.c:97`). Mit `on`/`off`
(case-insensitiv): `ESC v` bzw. `ESC w` und `linewrap` setzen. Anderes
Argument: stumm ohne Wirkung. EmuTOS-VT52 startet ohne Umbruch
(`v_stat_0 = M_CFLASH`, `bios/vt52.c:932`).

### 4.21 Datum/Zeit (`decode_date_time`, `cmdutil.c:169-220`)

Aus `idt_value` (`_IDT`-Cookie): Trenner = `LOBYTE` (nur `0x20..0x7F`, sonst
`/`); Datumsreihenfolge = `HIBYTE & 3` (0 MDY, 1 DMY, 2 YMD, 3 YDM, sonst YMD);
Uhrzeitformat = Bit 12 (`0` = 12 h mit `am`/`pm`, sonst 24 h mit `"  "`,
`:203-216`). Format: `%02d<sep>%02d<sep>%04d` bzw. `%04d<sep>…` (Jahr =
1980 + `date>>9`, Monat `(date>>5)&0x0f`, Tag `date&0x1f`), dann
`"  %02d:%02d:%02d%s  "` (Stunde `time>>11`, Minute `(time>>5)&0x3f`,
Sekunde `(time&0x1f)<<1`). 12 h: Stunde ≥ 13 → -12, Stunde 0 → 12.
NewTOS setzt `_IDT = 0x112E` (`tos-api/src/cookie.rs:226@IDT`; seit COOK02, vorher
`0x112F`): `.`, DMY, 24 h.

## 5. Pruefauftrag

* 4.5 (gleiche Datei), 4.15 (`-q`-Fehlziel) und 03 §3 (stale redirect) sind
  Code-Lesungen. Vor einer Entscheidung 1:1/berichtigt: im Test gegen die
  EmuTOS-Referenz (QEMU/Hatari) nachstellen — dies ist **nicht**
  geschehen.
* `Dfree`-Puffer im NewTOS-Programm als 4×`u32` anlegen (kein `usize`).
* `DTA` als `repr(C)`-Struktur mit den 44 Byte on-wire (`cmd.h:142-149`,
  `docs/api/structures/dta.md`); keine Zeiger darin.
