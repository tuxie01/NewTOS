# EmuCON2 — Start, Umgebung, Beenden

Quelle: EmuTOS @ `1a7484ab`. Pfade relativ zu `references/emutos/`.
Legende und Beleglage: [README](README.md).

## 1. Wer startet EmuCON (EmuTOS) — drei Wege

Alle drei laufen auf denselben Einsprung `coma_start` (`cli/cmdasm.S:25`,
deklariert `cli/clistub.h:13`). Sie sind **`#if WITH_CLI`** (Abschnitt 5).

| # | Weg | Beleg | Basepage / Umgebung |
|---|---|---|---|
| A | **Desktop-Menue „Execute EmuCON"** (`CLIITEM`): `pro_run(FALSE, DEF_CONSOLE, G.g_work, -1, -1)` → `shel_write(SHW_EXEC, …, "EMUCON", …)` → `sh_write` setzt `sh_nextapp = CONSOLE_APP`, wenn `pcmd == DEF_CONSOLE` | `desk/deskmain.c:750-755`, `desk/deskpro.c:36-63`, `aes/gemshlib.c:168-176`, `include/aesdefs.h:627` (`DEF_CONSOLE "EMUCON"`) | `sh_ldapp` CONSOLE-Arm: `aes_run_rom_program(coma_start)` (`aes/gemshlib.c:614-620`): `Pexec(PE_BASEPAGEFLAGS, PF_STANDARD, "", NULL)` → leere Kommandozeile, **env = NULL = Kopie von `run->p_env`** (`bdos/proc.c:213-224`, `alloc_env` `:413-432`); danach `p_tbase = coma_start`, `dos_exec(PE_GOTHENFREE, NULL, pd, NULL)` (`gemshlib.c:570-580`) |
| B | **Frueh-Konsole** (Esc auf dem Startbildschirm → `BOOTFLAG_EARLY_CLI`) vor `autoexec()` | `bios/initinfo.c:425-427` (Esc setzt das Flag), `bios/bios.c:834-843` | `Pexec(PE_BASEPAGEFLAGS, PF_STANDARD, "", default_env)`, `p_tbase = coma_start`, `p_tlen=p_dlen=p_blen=0`, `Pexec(PE_GOTHENFREE, "", pd, default_env)`; env = `default_env` (Abschnitt 3) |
| C | **Ohne AES** als Haupt-UI: `exec_os` zeigt auf `coma_start`, wenn `WITH_AES` aus und `WITH_CLI` an ist | `bios/endrom.c:15-35` (`ui_mupb`), Start in `bios/bios.c:856-866` | wie `exec_os`-Start (`Pexec(PE_GO, …, default_env)`) |

Der **Rueckfall NewTOS→EmuCON** (D6-d, Ladefehler von NEWDESK) und der
Weg ohne Bildschirm entsprechen C; EmuTOS entscheidet das zur *Bauzeit*
(`endrom.c:28-35`), NewTOS zur Laufzeit (`kernel/src/shell_loop.rs:20-35`
Kopfkommentar, benannte Abweichung „Hardwareebene").

## 2. Eintritt: was `coma_start` tut (`cli/cmdasm.S:25-49`)

Reihenfolge:

1. `a5 = 4(sp)` — Zeiger auf die Basepage (`:26`).
2. Stack auf ein eigenes 4-KiB-BSS-Feld (`lea stack,sp`, `:27`; Feld
   `.ds.l 1024` `:117-118`).
3. `Mshrink(0, basepage, tlen+dlen+blen+256)` (`:28-37`, GEMDOS 74): bei
   ROM-Start sind `tlen/dlen/blen` null (`bdos/proc.c:370`, `bzero` in
   `init_pd_fields`; im Weg B zusaetzlich explizit `bios/bios.c:841`), also bleibt
   nur die Basepage (256 Byte, **Schluss**: `sizeof(PD)`) — der Rest des TPA
   wird frei und dient spaeter `Malloc` (Verlauf, Datei-Puffer).
4. `environment = basepage->p_env` (`:39-44`, Offset **44**; `p_env` ist an
   `0x2C` laut `docs/api/structures/basepage.md:36`).
5. `jsr cmdmain` (`:46`), danach `Pterm0` (`clr.w -(sp); trap #1`, `:48-49`).

`cmdmain` kehrt nur bei `exit` zurueck (Abschnitt 4); es gibt **keinen anderen
Ausgang** (`cmdmain.c:120-139`, Endlosschleife).

Die `jmp_gemdos/bios/xbios`-Huellen (`cmdasm.S:77-105`) sichern `a2`/`d2`
um jeden Trap, weil „jemand den Trap abfangen und sie zerstoeren" koennte
(`:79-80`) — reine m68k-ABI-Eigenschaft, **intern** (NewTOS: entfaellt).

## 3. Umgebung und PATH-Uebernahme

* Erster Schritt von `cmdmain` nach der Initialisierung der Cookies/Aufloesung
  (`cmdmain.c:70-83`), Banner (`:91-95`) und Verlaufspuffer (`:102-103`):
  `getenv(&largv[1], "PATH=")` (`:105-118`). `getenv` durchsucht die
  Umgebung ab `environment` bis zum doppelten NUL (`cmdmain.c:230-250`,
  „gleiche Implementierung wie AES `sh_envrn`", `:227-228`).
* Das EmuTOS-Umgebungsformat ist **`PATH=` NUL `<Laufwerk>:\` NUL NUL**:
  `init_default_environment` schreibt `PATH_ENV`, dahinter `DEF_PATH "A:\\"`
  mit auf das Boot-Laufwerk korrigiertem Buchstaben, dann Doppel-NUL
  (`bios/bios.c:626-636`; `ENV_SIZE 12`, Kommentar `bios/bios.c:68-69`).
  Darum die Zeile `if (!largv[1][0]) largv[1]++;` („skip NUL after PATH=",
  `cmdmain.c:110-111`).
* Ist danach ein Wert vorhanden, ruft EmuCON **den Befehl `path` auf**:
  `largv[0]="path"`, `execute(2, largv, redir_name)` (`cmdmain.c:112-116`) —
  also dieselbe Logik wie der Benutzerbefehl, `user_path` bekommt den Wert
  (`cmdint.c:486`). Danach wird die Umgebung **nie wieder gelesen** und nie
  veraendert: `path` aendert nur `user_path` (Programm-intern), nicht
  `PATH=` der Umgebung (`cmdint.c:475-490`).
* Fremdprogramme erhalten `Pexec(0, path, cmdline, NULL)`: Umgebung = `NULL`
  = Kopie der **eigenen** Umgebung von EmuCON (`cmdexec.c:52`, `bdos/proc.c:413-432`).
* Standard-Handles: 0/1/2 aus dem Elternprozess geerbt
  (`bdos/proc.c:387-409` `init_pd_files`). Aktuelles Laufwerk/Verzeichnis:
  ebenfalls vom Eltern-`run` kopiert (`bdos/proc.c:399-409`); beim Desktop-Weg A
  hat der Desktop es zuvor per `shel_wdef("", <Pfad des obersten Fensters>)`
  gesetzt (`desk/deskmain.c:756-768`, nur `CONF_WITH_PCGEM`) und `sh_chdef`
  macht daraus `set_drvdir(psh->sh_cdir)` (`aes/gemshlib.c:545-549`).
* Standard-DTA: `Fgetdta()` zu Beginn (`cmdmain.c:98`); fuer ein frisches PD
  ist das die Kommandozeilen-Flaeche (`bdos/proc.c:374-375`,
  `p_xdta = p_cmdlin`). EmuCON benutzt diese eine DTA fuer **alle**
  `Fsfirst/Fsnext` (Befehle, Tab-Vervollstaendigung, Programmsuche) und liest
  nach jedem Aufruf `dta->d_fname/d_attrib/d_length/d_date/d_time` direkt
  (`cmd.h:142-149`; Layout 44 Byte: 21 reserviert, Attr @21, Zeit @22, Datum @24,
  Laenge @26, Name[14] @30).
* **EmuCON liest die eigene Kommandozeile nicht** (kein Zugriff auf
  `p_cmdlin`); sie ist beim Start leer (`gemshlib.c:575`, `""`).

## 4. Beenden

* Befehl `exit` (`cmdint.c:147`, `LOOKUP_EXIT`): `execute` liefert -1
  (`cmdmain.c:160-161`); `cmdmain` ruft `change_res(original_res)` (stellt eine
  geaenderte Aufloesung wieder her, nur `CONF_ATARI_HARDWARE`, Abschnitt 5) und
  gibt 0 zurueck (`:132-135`); `coma_start` macht `Pterm0` (`cmdasm.S:48-49`).
* Es gibt **kein `quit`** in EmuCON (Befehlstabelle `cmdint.c:140-163`; der
  Kommentar „exit/quit" bei `cmdmain.c:160` ist nur ein Kommentar). Der
  NewTOS-Kernelkonsole `quit` (`kernel/src/shell.rs:121`) entspricht nichts.
* Der Rueckgabewert von `Pterm0` ist 0; `aes_run_rom_program` gibt ihn
  zurueck (`gemshlib.c:579`), der CONSOLE-Arm **ignoriert** ihn
  (`gemshlib.c:618`).
* Speicher: History-Puffer (`Malloc`, `cmdedit.c:152`) wird nie freigegeben —
  das Prozessende gibt ihn frei (GEMDOS-Standard; `PE_GOTHENFREE`,
  `bdos/proc.c:238-242` setzt den Besitzer auf das PD selbst).

## 5. Rueckkehr zum Desktop (Weg A)

Nach `aes_run_rom_program(coma_start)` ruft der CONSOLE-Arm
`set_default_desktop(psh)` und `return 0` (`aes/gemshlib.c:614-620`,
`:583-590`): `sh_nextapp = DESKTOP_APP`, `sh_isgem = TRUE`,
`sh_cdir = D.s_cdir`. Die Schleife in `sh_main` (`gemshlib.c:692-727`, Schleife `:710-727`) macht
einen weiteren Durchgang: `sh_chdef` (`:537-566`, stellt `sh_cdir` her),
`sh_chgrf` (`:513-523`, Wechsel Alpha→Grafik: `sh_tographic`, `:223-236`) und
startet den Desktop neu (DESKTOP-Arm `:600-612`, bei NewTOS das
EL0-Programm NEWDESK, ADR-026).

Beim **Start** von EmuCON ist `sh_isgem` FALSE (`pro_run(FALSE, …)`, `isgem`
`deskpro.c:41`, `gemshlib.c:173`), `sh_chgrf` schaltet vorher auf Alpha
(`sh_toalpha`, `gemshlib.c:246-257`: Arrow-Cursor, Fehlerhandler zurueck,
Maus aus, Bildschirmspeicher frei, `gsx_graphic(FALSE)`). EmuCON selbst macht
dann `ESC E` + `ESC e` (`cmdmain.c:91-92`).

**Wichtig fuer den Namen:** der CONSOLE-Arm ruft weder `p_nameit` noch
`p_setappdir` (`gemshlib.c:614-620` im Gegensatz zu `:603-605`) — EmuCON laeuft
unter dem Namen des Vorgaengers (so auch dokumentiert in
`kernel/src/shell_loop.rs:575-578`).

Weg B (Frueh-Konsole): nach `Pterm0` laeuft `autoexec()` weiter
(`bios/bios.c:846`). Weg C: kein Desktop, `exec_os`-Rueckkehr →
`shutdown()`/„System halted!" (`bios/bios.c:868-876`).

## 6. Der NewTOS-Ist-Zustand (gelesen)

| Aspekt | NewTOS heute | Beleg |
|---|---|---|
| Wo | **EL1, im Kernel**, kein Prozess; `repl()` in `kernel/src/shell.rs` | `kernel/src/shell.rs:26-31,57-66` |
| PD | Stellvertreter-PD `standin::enter_emucon()`/`leave_emucon()` (PROC03 Stufe 1): waehrend der Konsole ist er `run`; ein von `run` gestartetes Programm hat ihn als `p_parent` | `kernel/src/shell.rs:19-30`, `tos-api/src/bdos/process/standin.rs:445-465` (`enter_emucon` `:452`), `kernel/src/lib.rs:488-503` |
| Start | `run_console_arm()` im CONSOLE-Arm von `sh_ldapp` (`crate::shell::run_console()` + `set_default_desktop()`) — **ohne** `Pexec` | `kernel/src/shell_loop.rs:465-466,579-583` |
| Rueckfall | NEWDESK nicht ladbar → `run_console_instead_of_desktop(rc)`: `sh_isgem=false`, `sh_chgrf()`, `run_console_without_desktop` (Banner + „No desktop: NEWDESK could not be loaded (GEMDOS error N)."), dann `set_default_desktop()`; ein Neuversuch findet nicht statt (D6-c) | `kernel/src/shell_loop.rs:453-465,493-500,619-625`, `kernel/src/shell.rs:40-55`, ADR-026 Pkt. 4 |
| Kein Bildschirm | DESKTOP-Arm faellt in die Konsole | `kernel/src/shell_loop.rs:447-453` |
| Frueh-Konsole (Esc) | nicht verdrahtet: kein Verbraucher fuer `BOOTFLAG_EARLY_CLI` | `kernel/src/initinfo.rs:535-538` |
| Eingabe/Ausgabe | `Cnecin`/`Cconout` per `svc #1` im Kernel | `kernel/src/shell.rs:333-342` |

**EL0-Muster (ADR-026, NEWDESK), das EmuCON uebernehmen soll — gelesen:**

* Programm als Rust-`staticlib`: Bibliothek `programs/desktop/` + schmaler
  Binaer-Crate `programs/newdesk/`; Mitglieder in `programs/Cargo.toml:13`
  (`members = ["newtos-gem", "hello-gem", "desktop", "newdesk"]`).
* Bau: `cargo xtask build-programs` → `programs/build.sh` mit `sdk/crt0.S`,
  `sdk/newtos.ld`; Tabelle `PROGRAMS` in `tools/xtask/src/main.rs:421`
  (`("newdesk","NEWDESK.NAP")`).
* Einbettung: `kernel/embed.rs` `EMBEDDED` (`kernel/embed.rs:49`:
  `&[Embedded { name: "NEWDESK", file: "NEWDESK.NAP" }]`) → generiertes
  `EMBEDDED_PROGRAMS` (`kernel/build.rs:42-45`) → `embedded::register()`
  registriert beim Boot unter dem Registry-Namen (`kernel/src/embedded.rs`);
  `Pexec` fragt die Registry **vor** dem Dateisystem
  (`kernel/embed.rs`-Kopf; Namen mit Laufwerk oder Endung treffen nie). Ein
  fehlender Blob ist ein harter Baufehler (`embed.rs::generate`). Der
  Hosttest `newdesk_is_the_one_embedded_program` (`embed.rs`, letzter Test)
  pinnt die Liste auf genau einen Eintrag und muesste mit einem EMUCON-Eintrag
  angepasst werden.
* Start: DESKTOP-Arm → `crate::launch::run_desktop_program(name, cmd)`
  (`kernel/src/launch.rs:131-148`, `launch(…, Launch::Desktop)`): kein
  Sanduhrzeiger, **leere** Kommandozeile, kein Umschreiben von
  `D.s_cmd`/`ad_stail` — das ist ausdruecklich die Entsprechung von
  `aes_run_rom_program` (`launch.rs:122-135`). Die Umgebung ist
  `EnvSpec::Inherit` (`launch.rs:179-189`).
* Der Registry-Name des Desktops ist `DEF_DESKTOP`; der der Konsole steht
  schon da: `DEF_CONSOLE = b"EMUCON"` (`tos-api/src/aes/shell.rs:67-71`), und
  der Desktop schickt ihn bereits (`programs/desktop/src/pro.rs:21-24`,
  `programs/desktop/src/main.rs:1612-1660`). `sh_write` setzt `CONSOLE_APP` bei diesem Namen
  (`tos-api/src/aes/shell.rs:521-524`).

Folgerungen (**Schluss**, im Bau zu bestaetigen): Als echter EL0-Prozess hat
EmuCON seinen eigenen PD; ein `Pexec` aus EmuCON setzt dann von selbst
`p_parent` = EmuCON-PD (PROC03 §1: „`Pexec`-Kinder aus EL0: `p_parent` = PD des
Aufrufers"), der Stellvertreter-EmuCON-PD (`enter_emucon`/`emucon_pd`)
wird entbehrlich. Das entspricht PROC03 B2 („Pexec aus dem Kernel-EmuCON:
Eltern = AES-Stellvertreter statt EmuCON-PD").

## 7. Vertrag / intern

* **Vertrag:** Banner und Prompt (siehe 02, 03), die Rueckkehr in den Desktop
  (`set_default_desktop`), Pterm0-Rueckgabe 0, Umgebungsvererbung per `NULL`
  (Kopie), Kommandozeilenformat der Kinder (Laengenbyte + Text, 03 §6),
  Standard-Handles der Kinder, EmuCON-PD als Elternprozess der Kinder.
* **Intern (frei):** eigener 4-KiB-Stack, Mshrink auf 256 Byte, `jmp_*`-Huellen,
  Line-A-Aufruf fuer die Zellenzahl (Ersatz noetig, siehe 05 §4/06 D4).
