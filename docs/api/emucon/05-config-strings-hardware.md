# EmuCON2 — CONF-Gating, Sprache, Hardwareanteile

Quelle: `cli/*`, `Makefile`, `include/config.h`, `include/nls.h`,
`docs/api/_emutos-config.md` (erzeugt; Pin `1a7484ab`), `bios/vt52.c`.
Legende: [README](README.md).

## 1. Gating — was ist in unserem Referenzprofil an?

Referenzbau = `make 512` (Falcon030/TOS4-Profil laut
`docs/api/_emutos-config.md`; Projektnotiz „Referenzbau make 512"). Nur
`-DTARGET_512` ist gesetzt; `make 512` setzt `WITH_AES=1 WITH_CLI=1`
(`Makefile:267`, `_emutos-config.md:22-26`).

| Schalter | Wirkung auf EmuCON | Wert in `make 512` | Beleg |
|---|---|---|---|
| `WITH_CLI` | ganzes Verzeichnis `cli/` wird gebaut (`optional_dirs += cli`); `coma_start` existiert; Desktop-Menuepunkt `CLIITEM`; `shel_wdef`; Esc-Zeile im Startbildschirm | **1** (Makefile-Vorgabe `WITH_CLI=1`; nur `192` setzt 0) | `Makefile:149-150,455-457,534`, `desk/deskmain.c:203,750`, `desk/gembind.c:962`, `bios/initinfo.c:291,358,424` |
| `STANDALONE_CONSOLE` | eigenes `emucon2.tos` mit eigener libc: andere `#define`s (`cmd.h:12-29`), B:→9:-Umschreibung bei einem Diskettenlaufwerk (`cmdparse.c:40-54`), eigene `strlen/strcpy/memcpy/memset/…/sprintf` (`cmdutil.c:342-455`), `no-tree-loop-distribute-patterns`-Attribute (`cmdedit.c:293-296`) | **aus** im ROM-Bau; nur `cli/Makefile:16` setzt es | `cli/Makefile:12-25` |
| `CONF_ATARI_HARDWARE` | Koerper von `change_res()` (`Setscreen`/`Setcolor`/`ESC b/c`); ohne den Schalter tut `change_res` nichts (`cmdmain.c:255-300`) | **1**, Klasse HW „Master-Schalter fuer den ganzen Atari-Hardware-Backend", NewTOS' Backend ist die RPi4-HAL | `_emutos-config.md:285,541`, `include/config.h:663-667` |
| `CONF_WITH_TT_SHIFTER` | `original_res`-Bedingung (`<_VDO_VIDEL` statt `<_VDO_TT`, `cmdmain.c:75-79`) und die TT-Aufloesungen `TT_LOW/TT_MEDIUM/ST_HIGH` in `valid_res` (`:317-324`) | **1**, Klasse HW | `_emutos-config.md:597` |
| `CONF_WITH_PCGEM` | nur der Desktop-Teil: `shel_wdef` setzt das Startverzeichnis von EmuCON | **1** | `desk/deskmain.c:756-768`, `_emutos-config.md:198` |
| `CONF_WITH_NLS` | `_()`/`N_()`/`gettext()` (Abschnitt 2) | **0** fuer `make 512` mit `UNIQUE=us` | `include/nls.h:20-37`, `tools/localise.c:650-654` |

`cli/` benutzt sonst **keine** `CONF_*`-Schalter (Suche nach `CONF_`, `WITH_`
und `#if` ueber `cli/`: nur die Zeilen der Tabelle sowie `__mcoldfire__` in
`cmdasm.S:39,53,64` — m68k-ColdFire-Varianten der Assembler-Huellen, fuer
AArch64 ohne Bedeutung). Die
Befehle selbst sind also **nicht** einzeln abschaltbar; alle 21 sind in
unserem Profil dabei.

**Was die Gating-Tabelle nicht entscheidet:** ob NewTOS die Hardwareanteile
(Aufloesungswechsel, TT-Schifter, Cookie `_VDO`) ueberhaupt nachbilden soll —
das ist eine NewTOS-Entscheidung ([06 D5](06-mapping-and-decisions.md)).

## 2. Sprache / Zeichenketten

* **Alle sichtbaren Texte** in `cmd*.c` stehen als englische Literale, im
  Normalfall mit `_()` (Auswahl zur Laufzeit) oder `N_()`/`gettext()`
  (Hilfetexte, `cmdint.c:72-134,339,980`). Ohne NLS sind die Makros
  Identitaet (`nls.h:33-37`).
* `make 512` hat `UNIQUE = $(COUNTRY)` (`Makefile:560`), `COUNTRY = us`
  (`Makefile:123`); `localise` schreibt fuer einen Ein-Land-Bau
  `CONF_MULTILANG 0` und `CONF_WITH_NLS 0` (`tools/localise.c:650-654`).
  Das Kontrollstueck im Checkout, `include/i18nconf.h` (erzeugt am
  2026-08-29), sagt dasselbe. **Folge: in unserem Referenzprofil ist EmuCON
  englisch.**
* Der ROM-Bau mit mehreren Sprachen uebersetzt EmuCON dagegen sehr wohl
  (`util/langs.c` enthaelt z. B. „Willkommen bei EmuCON2 Version", `:1306`,
  und „EmuCON2 verlassen", `:1252`); `cli/readme.txt:39-41` („nur auf
  Englisch") gilt nur fuer das Standalone-Programm.
* NewTOS: Systemtexte sind englisch, `tos-api/src/i18n.rs:1-12`
  („Only English is wired up now"; spaeter ueber `_AKP`).
  → EmuCON kann seine Texte als feste englische Literale fuehren; die
  `_()`-Struktur muss nicht mit.
* Die Hilfetexte und Meldungen sind vollstaendig in [04](04-builtins.md).

## 3. VT52-Escapes, die EmuCON an `Bconout(2)` sendet

EmuCON benutzt den Zeichenstrom der Konsole zur Terminalsteuerung. Es sind
**genau diese zehn Folgen** (`cmd.h:189-193`, `cmdutil.c:26-30`,
`cmdedit.c:330`, `cmdint.c:293,629,632`, `cmdmain.c:91-92,291-294`):

| Folge | Zweck | wo | EmuTOS-Handler |
|---|---|---|---|
| `ESC E` | Bildschirm loeschen + Cursor Home | Start, `cls`, Aufloesungswechsel | `am_tab` Index 4 `clear_and_home`, `bios/vt52.c:82-96`, `:338` |
| `ESC e` | Cursor an | Start, `change_res` | `bw_tab` Index 3 `cursor_on`, `:98-122,:630` |
| `ESC D` | Cursor links | Zeileneditor (`cursor_left()`) | `am_tab` 3 `cursor_left`, `:420` |
| `ESC C` | Cursor rechts | Zeileneditor | `am_tab` 2 `cursor_right`, `:396` |
| `ESC K` | bis Zeilenende loeschen | Editor (`erase_line()`) | `am_tab` 10 `erase_to_eol`, `:506` |
| `ESC l` | Zeile loeschen, Cursor Spalte 0 | `blank_line()` (`help`-Pause, `more`) | `bw_tab` 10 `erase_line`, `:733-742` |
| `ESC v` | Zeilenumbruch an | `wrap on` | `bw_tab` 20 `line_wrap_on`, `:795` |
| `ESC w` | Zeilenumbruch aus | `wrap off` | `bw_tab` 21 `line_wrap_off`, `:807` |
| `ESC b <n>` | Vordergrundfarbe (1 Folgebyte) | nur `change_res` | `bw_tab` 0 `set_fg` |
| `ESC c <n>` | Hintergrundfarbe (1 Folgebyte) | nur `change_res` | `bw_tab` 1 `set_bg` |

Dazu die Steuerzeichen CR (0x0D) und LF (0x0A) in jeder Ausgabe
(`messagenl` = `\r\n`) und rohe Bytes 0x20..0xFF.

**NewTOS-Stand (gelesen): diese Folgen werden im Zeichenstrom nicht
verstanden.** `Bconout(2, …)` landet in `tos_api::vdi::alpha_conout`; das
behandelt `CR`, `LF`, `BS`, `TAB` und druckbare Zeichen `0x20..=0xFF`,
**alle anderen Steuerzeichen werden ignoriert**
(`tos-api/src/vdi/escape.rs:128-136,166`; Kommentar `fbcon.rs:24-25`:
„`ESC` sequences in the byte stream are not parsed"). Ein `ESC E` aus EmuCON
wuerde also `ESC` verschlucken und den Buchstaben `E` als Zeichen ausgeben.
Die Kernel-Konsole umgeht das, indem sie `fbcon::clear_and_home()` direkt
aufruft (`kernel/src/shell.rs:124`) statt `ESC E` zu senden. Die BIOS-Spezifikation
beschreibt den Parser (`docs/api/bios/03-bconout.md:30,230`,
`CONF_WITH_VT52`), er ist aber nicht gebaut. **Das ist eine Voraussetzung**
fuer EmuCON als EL0-Programm (und fuer jedes TOS-Textprogramm) —
Entscheidung [06 D3](06-mapping-and-decisions.md). Zusaetzlich: der
Textcursor ist in NewTOS „nur logisch", es wird kein sichtbarer Block
gezeichnet (`tos-api/src/vdi/escape.rs:21-24`), `Cursconf` fuehrt nur
Zustand (cite stale 2026-10-26: tos-api/src/xbios/screen.rs:380-411).

## 4. Hardwareabhaengige Aufrufe und was NewTOS dafuer hat

| EmuCON-Aufruf | Zweck | Beleg | NewTOS-Stand (gelesen) |
|---|---|---|---|
| `getcookie(_IDT)` / `getcookie(_VDO)` ueber `Supexec(getjar)` (Lesen von `*(LONG*)0x5a0`, Schleife bis Tag 0) | Datumsformat; Videohardware | `cmdutil.c:315-340`, `cmdmain.c:70-74` | `Supexec` ist offene ADR (`programs/newtos-gem/src/gemdos.rs:434-452`); Ersatz: `dos_getcookie` (`Ssystem(S_GETCOOKIE)`, `programs/newtos-gem/src/gemdos.rs:453-456`, `Option<u32>`; Kommentar `:447-451`: bei Wert 0 vs. Fehler uneindeutig, nicht fuer Zeigerwerte). `_IDT` ist gesetzt (`0x112F`, `tos-api/src/cookie.rs:224`), **`_VDO` fehlt absichtlich** (`cookie.rs:217-220`) |
| `Getrez()` | Startaufloesung | `cmdmain.c:76,78` | liefert immer `TT_MEDIUM` = 4 (`tos-api/src/xbios/screen.rs:160-168`, ADR-017 rev 2) |
| `Setscreen(-1,-1,res,0)`, `Setcolor(3,…)` | Aufloesung/Farbe wechseln | `cmdmain.c:263-274` | `rez` wird angenommen und ignoriert (`screen.rs:190-191`, `docs/api/xbios/05-setscreen.md:192`) |
| `getwh()` (Line-A-Init, liest `v_cel_mx/my` bei `-44(a0)`) | Spalten/Zeilen | `cmdasm.S:51-60`, `cmdedit.c:134-136`; `include/lineavars.h:50-52` | **kein Line-A auf AArch64** (`docs/api/_line-a.md:86-90`, ADR-030); ersatzlos **OFFEN** (D4). `getht()` (`cmdasm.S:62-72`) ist deklariert, **nirgends benutzt** (Suche ueber `cli/`) |
| `Supexec(get_nflops)` (liest `0x4a6`) | Diskettenzahl | `cmdmain.c:83,333-336` | **im ROM-Bau ungenutzt** (Wert wird nur von der `STANDALONE`-Programmpfad-Umschreibung gelesen, `cmdparse.c:40-54`) → entfaellt |
| `Cursconf(0|1,0)` | Cursor aus/an um Programmstarts | `cmdexec.c:51,53` | vorhanden (cite stale 2026-10-26: tos-api/src/xbios/screen.rs:386-411), Zustand ohne sichtbaren Cursor |
| `Kbrate(-1,-1)` / `Kbrate(d,r)` | Tastenwiederholung | `cmdint.c:431,454` | XBIOS `Kbrate` ist spezifiziert (`docs/api/xbios/23-kbrate.md`) |
| `Sversion()` | `version` | `cmdint.c:611` | `0x3000` (`tos-api/src/gemdos/version.rs:14`) |
| `Bconin/Bconstat/Bconout(2,…)` | gesamte Ein-/Ausgabe | `cmd.h:83-85,194-196` | BIOS-Traps vorhanden; `newtos-gem` hat **keine** BIOS-Bindung (nur `Cconout/Cconws/Crawcin/Cconis`) |

### Aufloesungs-Logik gegen NewTOS (Schluss aus dem Code)

Mit `_VDO` fehlend → `vdo_value = _VDO_ST` (`cmdmain.c:73-74`);
`original_res = Getrez() = 4` (mit `CONF_WITH_TT_SHIFTER`, `:75-77`); kein
Start-Wechsel (4 ≠ `ST_LOW`). `mode con` zeigt `  Resolution:       4`.
`mode con res=0` oder `res=1` laeuft durch `valid_res` (`ST_LOW`/`ST_MEDIUM`
sind ohne weitere Pruefung `TRUE`, `:325-327`; `TT_LOW/TT_MEDIUM/ST_HIGH`
scheitern an `vdo_value != _VDO_TT`, `:318-322`) und ruft dann `change_res`:
`Setscreen` (wirkungslos), `Setcolor(3, BLACK)`, `ESC b 15`/`ESC c 0`,
`ESC E`, `current_res = res` (`:263-298`) — es **taeuscht** einen Wechsel
vor. Beim `exit` ruft `change_res(original_res)` (`:133`) das Gegenstueck.
Mit gesetztem `_VDO = VIDEL` (Falcon-Profil) waere `original_res = -1`, jedes
`res=` ungueltig und die `Resolution:`-Zeile fehlte (`:307-308`,
`cmdint.c:427`). Welche Variante NewTOS will — **OFFEN**, [06 D5](06-mapping-and-decisions.md).
