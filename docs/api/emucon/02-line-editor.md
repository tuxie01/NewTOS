# EmuCON2 — Prompt, Zeileneditor, Verlauf, Tab-Vervollstaendigung

Quelle: `cli/cmdedit.c` (480 Zeilen), `cli/cmd.h`, `cli/cmdmain.c`,
`include/scancode.h`. Legende: [README](README.md).

## 1. Konstanten und Zustand

| Name | Wert | Beleg |
|---|---|---|
| `MAX_LINE_SIZE` | 200 | `cmd.h:103` (Kommentar: groesser als die groesste Bildschirmbreite) |
| `HISTORY_SIZE` | 10 | `cmd.h:104` |
| `MAX_ARGS` | 30 | `cmd.h:105` |
| `linesize` | `screen_cols + 1 - 3`, hoechstens `MAX_LINE_SIZE` | `cmdedit.c:134-139` („Platz fuer NUL und Prompt") |
| `input_line[200]` | Eingabepuffer | `cmdmain.c:43` |
| `history_line[10]` | Zeiger in einen `Malloc(200*10)`-Block, je Zeile 200 Byte | `cmdedit.c:152-159` |
| `history_num` | naechster zu schreibender Slot, zirkulaer; **-1 = Verlauf nicht verfuegbar** (Malloc fehlgeschlagen) | `cmdedit.c:150,161,173` |
| `insert`, `fsfbuf[128]` | Tab-Vervollstaendigung: Einfuegestelle, Fsfirst-Muster | `cmdedit.c:22-23` |

`init_screen()` liest die Bildschirmgroesse **einmal je Durchlauf der aeusseren
Schleife**, d. h. beim Start und nach einem Aufloesungswechsel
(`cmdmain.c:120-121,138`), nicht bei jedem Prompt. Quelle der Zahlen:
Line-A-Init, `v_cel_mx`/`v_cel_my` (`cmdasm.S:51-60`, `cmdedit.c:134-136`:
`cols = mx+1`, `rows = my+1`). Auf AArch64 gibt es kein Line-A — siehe
[05 §4](05-config-strings-hardware.md) / [06 D4](06-mapping-and-decisions.md).

## 2. Prompt

`read_line` (`cmdedit.c:52-54`): `get_path(prompt, 0)`, dann `message(prompt)`,
dann `message(">")`. Das Ergebnis ist z. B. **`C:\>`** oder **`C:\FOO\BAR>`** —
**ohne Leerzeichen nach `>`** (anders als `kernel/src/shell.rs:296-309`, das
`> ` ausgibt).

`get_path(buf, drive)` (`cmdint.c:928-944`): schreibt `'A' + (drive ? drive-1 :
Dgetdrv())`, `':'`, dann `Dgetpath(p, drive)`; ist der Rest leer (Wurzel),
wird `\` angehaengt. `Dgetpath` liefert den Pfad **ohne** Laufwerk, mit
fuehrendem `\` und **ohne** abschliessenden `\` (`bdos/fsdir.c:1229-1251`,
`*--buf = 0 /* null as last char, not slash */`). Der Fehlercode von
`Dgetpath` wird im Prompt ignoriert. Der Prompt geht ueber `message()` =
`Bconout(2, c)` je Zeichen (`cmdutil.c:35-41`, `cmd.h:196`), **nie** in eine
Umlenkdatei.

## 3. Eingabeschleife `read_line` (`cmdedit.c:41-104`)

Pro Taste: `charcode = Bconin(2)` (blockierend, `cmd.h:194`) und
`scancode = ((charcode >> 8) | charcode) & 0xffff` (`:57`). Fuer eine normale
Taste ergibt das `(BIOS-Scancode << 8) | ASCII` (Bits 8..15 von `charcode` sind
bei TOS null; **Schluss**, aus den Konstanten unten). Reihenfolge:

1. `edit_line(…)` bekommt `scancode` und den Rueckgabewert des vorigen Aufrufs
   (`prevcode`, nur fuer „TAB nach TAB"). Rueckgabe ≥ 0 = Taste verarbeitet →
   naechste Taste (`:60-62`).
2. Sonst `c = LOBYTE(charcode)`; `c == 0` (Sondertaste ohne ASCII) → ignoriert
   (`:65-67`).
3. `\r` (0x0D): `conout('\r'); conout('\n')`, Puffer mit NUL abschliessen,
   Schleife verlassen (`:70-75`). **Nur `\r` beendet die Zeile**; `\n`
   (Ctrl-J) ist ein Steuerzeichen < 0x20 und wird ignoriert (`:85-86`).
4. Ctrl-C (`CTL_C`, `cmd.h:185`): `messagenl("^C")`, Puffer NUL-terminieren,
   Rueckgabe **-1** (Zeile verworfen; `:78-82`).
5. Jedes andere Zeichen `< 0x20` → ignoriert (`:85-86`); `0x20..0xFF` sind
   druckbar (kein Test auf `0x7F`/`≥ 0x80`).
6. Ist `len >= linesize - 2`: ignoriert (`:89-90`) → hoechstens `linesize-2`
   Zeichen je Zeile (bei 80 Spalten: 76; bei 240 Spalten: 236, gedeckelt auf
   198 weil `linesize ≤ 200`).
7. Sonst: `conout(c)`; steht der Cursor nicht am Zeilenende, `insert_char`
   (Einfuegen, Rest neu zeichnen, mit `ESC D` zurueck, `:109-125`), sonst
   anhaengen; `pos++`, `len++` (`:93-98`).

Am Ende von `read_line` (Normalfall): `history_num = save_history_num`
(`:101`) — **nicht** auf dem Ctrl-C-Weg (`:81` kehrt vorher zurueck).

`cmdmain` ruft danach **immer** `save_history(input_line)` auf, **auch bei -1**
(`cmdmain.c:124-127`), dann erst `if (rc < 0) continue;`.

## 4. Editiertasten (`edit_line`, `cmdedit.c:195-286`)

Scancode-Konstanten: `include/scancode.h:21-42`.

| Taste | Scancode | Wirkung | Beleg |
|---|---|---|---|
| Pfeil hoch | `0x4800` | Verlauf zurueck (siehe 5), nur wenn `history_num >= 0` | `:202-207` |
| Pfeil runter | `0x5000` | Verlauf vor | `:208-213` |
| Pfeil links | `0x4b00` | Cursor 1 Zeichen links (`ESC D`), wenn `pos > 0` | `:217-225` |
| Ctrl-Pfeil links | `0x7300` | wortweise links (`previous_word_count`) | `:214-225`, `:428-442` |
| Pfeil rechts | `0x4d00` | Cursor 1 rechts (`ESC C`), wenn `pos < len` | `:229-237` |
| Ctrl-Pfeil rechts | `0x7400` | wortweise rechts (`next_word_count`) | `:226-237`, `:408-422` |
| Backspace | `0x0e08` | Zeichen links loeschen, Rest neu zeichnen | `:238-244`, `:297-319` |
| Delete | `0x537f` | Zeichen am Cursor loeschen | `:245-250` |
| Tab | `0x0f09` | Dateinamen-Vervollstaendigung (6) | `:251-280` |

Alles andere liefert `-1` (nicht verarbeitet, `:281-282`) und faellt in die
ASCII-Behandlung oben. **Folgen (Schluss, aus den Konstanten):** Shift+Pfeil
(`0x4838/0x5032/0x4b34/0x4d36`, `scancode.h:36-39`) ist kein Editierscancode; sein
ASCII-Byte ist `'8'/'2'/'4'/'6'` und wird **als Zeichen eingefuegt**. Ctrl-Backspace
(ASCII 0x7F, Scancode ≠ `0x0e08`) wird als Zeichen 0x7F eingefuegt. Home/End/Insert
haben ASCII 0 und werden ignoriert. Ob ein NewTOS-Tastaturtreiber dieselben
Scancodes liefert (insb. Ctrl-Pfeil `0x7300/0x7400`), ist **OFFEN** (Abschnitt 8).

Wortgrenzen: ein Wort endet am Leerzeichen; `next_word_count` zaehlt bis zum
ersten Nicht-Leerzeichen nach einer Leerstelle, sonst bis zum Zeilenende;
`previous_word_count` laeuft rueckwaerts ueber Nicht-Leerzeichen bis zur
naechsten Leerstelle, sonst bis zum Anfang (`:408-442`).

Terminalsteuerung, die der Editor ausgibt (alle ueber `escape()` = ESC + Zeichen,
`cmdutil.c:26-30`): `ESC D` (links), `ESC C` (rechts), `ESC K` (bis Zeilenende
loeschen, `erase_line`, `cmdedit.c:324-333`), ausserdem Leerzeichen/Zeichen zum
Neuzeichnen. Das sind **VT52-Escapes im Zeichenstrom von `Bconout(2)`**
([05 §3](05-config-strings-hardware.md)).

## 5. Verlauf

* Ringpuffer mit 10 Zeilen a 200 Byte, anfangs leer (`:152-159`). Fehlt der
  Speicher, meldet `cmdmain` `warning: no history buffers` und der Verlauf ist
  aus (`history_num == -1`, `cmdmain.c:102-103`, `cmdedit.c:150-153,173`).
* `save_history(line)` (`:170-190`): fuehrende Leerzeichen werden uebersprungen,
  **leere Zeilen** und **Dubletten der vorigen Zeile** (`strncasecmp`, also
  ohne Beachtung der Gross-/Kleinschreibung, ueber `MAX_LINE_SIZE`) nicht
  gespeichert; sonst `strcpy` nach `history_line[history_num]` und
  `history_num` zirkulaer weiter. Gespeichert wird die **Rohzeile vor dem
  Parsen** (`cmdmain.c:124-128`: `read_line` → `save_history` → `parse_line`,
  das den Puffer veraendert).
* Pfeil hoch/runter (`:338-355`): `history_num` wird zirkulaer um 1
  zurueck/vor gesetzt und die dortige Zeile ausgegeben (`replace_line`,
  `:360-371`). Vorher `erase_line(line, *pos)` (Cursor `*pos` Mal nach links,
  `ESC K`). Der Verlauf ist also ein **Ring ohne Ende**: Hoch am aeltesten
  Eintrag geht weiter in die leeren Slots / zum neuesten; leere Slots werden als
  leere Zeile angezeigt.
* Das Blaettern wirkt nur bis zum Zeilenende: `read_line` setzt `history_num`
  wieder zurueck (`:50,101`), ausser nach Ctrl-C (siehe oben und Abschnitt 7).

## 6. Tab-Vervollstaendigung (`cmdedit.c:251-280`, `:447-480`)

Nur Dateinamen (kein Befehlsnamen-, kein Variablenabgleich).

1. `line[*pos] = '\0'` (Zeile am Cursor abschneiden, `:252`). Hinweis
   **(Schluss)**: `*len` wird **nicht** angepasst; der Text hinter dem
   Cursor bleibt auf dem Schirm, ist im Puffer aber weg.
2. `start = start_of_current_word(line, *pos)` (`:376-387`): hinter dem letzten
   Leerzeichen links vom Cursor (oder Zeilenanfang).
3. Beim **ersten** Tab (`prevcode != TAB`): `insert = insertion_point(start)`
   (hinter dem letzten `:` oder `\` im Wort, `:392-402`); Muster
   `fsfbuf = <Wort bis Cursor>*.*` (`:261-267`).
4. Ueberlaufschutz: `insert + sizeof(dta->d_fname) - line >= linesize` →
   nichts tun (`:256-257`; `sizeof` = 14).
5. Suche: Beim ersten Tab (`rc = -1` erzwingt `getfirstnondot`) oder wenn
   `getnextfile` fertig ist: `getfirstnondot(fsfbuf, start==line)`
   (`:447-461`): `Fsfirst(buffer, 0x17)` und `Fsnext`, Namen mit fuehrendem
   `.` (`.`/`..`) werden uebersprungen. **Steht das Wort am Zeilenanfang
   (`start == line`, d. h. das Befehlswort), werden nur Programme akzeptiert**
   (`program_extension(dta) != NULL`, also keine Ordner; Endungen siehe
   [03 §7](03-parser-redirect-exec.md)). Bei weiteren Tab-Tasten
   (`prevcode == TAB`): `getnextfile` (`Fsnext`, `:466-480`); ist die Liste
   zu Ende (`rc < 0`), beginnt sie von vorn.
6. Treffer (`rc == 0`): `erase_line(insert, *pos - (insert-line))`, dann der
   Name aus `dta->d_fname` (wie im Verzeichnis gespeichert) ausgeben und
   einsetzen; `*pos = *len = Ende` (`:271-279`). Kein Treffer: keine Aenderung.

Die gemeinsame DTA wird dabei ueberschrieben (`dta` aus `Fgetdta`), was
unproblematisch ist, weil `Fsfirst/Fsnext` in jedem Befehl neu aufsetzen.

## 7. Auffaelligkeiten im Quelltext (Pruefauftrag)

Alle **(Schluss)** aus dem gelesenen Code, nicht ausgefuehrt. Sie sind dem
Anwender zur Entscheidung vorgelegt ([06 D7](06-mapping-and-decisions.md)),
nicht stillschweigend „berichtigt":

* **E1** Ctrl-C-Weg ruft `save_history` mit dem (abgebrochenen) Rest und laesst
  `history_num` auf der geblaetterten Position (`cmdedit.c:78-82` vs.
  `:101`; `cmdmain.c:124-127`). Die abgebrochene Zeile landet im Verlauf und
  kann an der geblaetterten Stelle eine alte ueberschreiben.
* **E2** Tab schneidet den Puffer am Cursor ab, ohne `len` nachzufuehren
  (`:252`).
* **E3** Shift+Pfeil fuegt Ziffern ein (siehe 4).

## 8. Offene Fragen zum Editor

* Liefert `Bconin(2)` aus EL0 in NewTOS `(Scancode<<16)|ASCII` mit den
  Atari-Scancodes dieser Tabelle, einschliesslich Ctrl-Pfeil und Delete?
  **OFFEN** — `kernel/src/keymap.rs` wurde nicht auf diese Codes geprueft.
* Blockiert `Bconin(2)` aus EL0 so, dass die Eingabe (USB/IKBD) weiter
  gepollt wird? **OFFEN** (heutige Konsole nutzt `Cnecin` im Kernel,
  `kernel/src/shell.rs:333-338`; `docs/api/bios/02-bconin.md` nicht auf dieses Detail
  geprueft).
* Die Zeilenbreite haengt an `screen_cols` (siehe 05 §4).
