# DESK14 — `Read .INF file` ist nicht `app_read_inf`

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: 7dd743e1f | Missing: item 4 (deferred)

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/DESK14-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit 7dd743e1f (feat(desk): Read .INF file asks, checks and reports), VERIFIED: 7dd743e1f (feat(desk): Read .INF file asks, checks and reports) names the id by body, its message names the ticket-title identifier app_read_inf, touches code (12 files), legacy line: ERLEDIGT (Punkte 1-3, Punkt 4 zurueckgestellt, siehe unten) ·

Line 3 before the migration:
Status: **ERLEDIGT** (Punkte 1-3; Punkt 4 zurueckgestellt, siehe unten) ·
Prioritaet: **P3** · Typ: Konformitaet — Nebenbefund DESK03/DESK04

Abhaengigkeiten: [DESK03](DESK03.md) (erledigt), [DESK09](DESK09.md) (erledigt)

## Sachverhalt

`Options → Read .INF file` ist in EmuTOS `app_read_inf`
(`references/emutos/desk/deskapp.c:1492-1548`) und tut vier Dinge, die
`kernel/src/desk/main.rs`s `do_read_inf` nicht tut:

1. **Dateiauswahl.** EmuTOS fragt mit `fsel_exinput(path, fname, &button,
   desktop_str_addr(STRDINF))` (`:1505`), Vorgabe `INF_FILE_WILD` auf dem
   Startlaufwerk. NewTOS liest fest `NEWDESK.INF` und fragt nichts.
2. **Signaturpruefung.** EmuTOS verlangt `#R` **oder** `#E` als erste zwei
   Zeichen (`:1523-1526`); NewTOS prueft seit DESK03 nur noch das `#`.
3. **Rueckmeldung im Fehlerfall.** `EFILNF` → `fun_alert(1, STFILENF)` mit
   dem Dateinamen (`:1536-1538`), alles andere → `fun_alert(1, STINVINF)`
   = `[1][Illegal .INF file format.][  OK  ]` (`desk_rsc.c:2560`). NewTOS
   kehrt **still** zurueck — die Abweichung ist mit DESK03 entstanden und
   dort benannt worden: vorher setzte derselbe Pfad Ansicht, Sortierung und
   Hintergrund stillschweigend auf die Vorgaben zurueck.
4. **Neustart statt Nachziehen.** Bei Erfolg schreibt EmuTOS die Datei in den
   Shell-Puffer (`shel_get`/`shel_put`, `:1532-1533`) und **verlaesst den
   Desktop**, der daraufhin neu eintritt und die Daten wie beim Start durch
   `app_start` liest (`deskmain.c:918-920` und `:2127-2135`). NewTOS wendet
   sie im laufenden Desktop an (`load_and_apply` + `cnx_get_prefs` +
   Neuaufbau), was einen zweiten, EmuTOS unbekannten Anwendungspfad ist.

## Lead-Entscheidung zu Punkt 4 (2026-09-23): zurueckgestellt

Der Neustartpfad wird **jetzt nicht** gebaut. Er ist kein Detail dieses
Tickets, sondern der Umbau des Desktops von einem Kernelaufruf zu einem
Prozess, den `sh_main` starten und neu eintreten kann: `global::init` ist
idempotent, es gibt kein `win_start()` beim Wiedereintritt, `wm_init()`
wuerde die AES-Fenstertabelle leeren, und die Bruecke `cnx_put();
app_save(FALSE)` → `shel_get` (`deskmain.c:2152-2153`) fehlt. DESK06 hat
denselben Punkt getroffen und faehrt den Konsolenlauf deshalb verschachtelt
(`shell_loop::sh_run_pending_console`), benannt als Abweichung.

Der Umbau gehoert zum Umzug des Desktops in den Benutzerbereich (Notiz
`desktop-must-run-in-user-scope`) und ist vor dem Vorfuehrtermin zu riskant.
Bis dahin gilt: Nachziehen im laufenden Desktop, als Abweichung
dokumentiert — an **einer** Stelle, nicht in jedem betroffenen Ticket.

## Arbeit

Punkte 2 und 3 (Signaturpruefung `#R`/`#E`, `STFILENF`/`STINVINF`-Alerts),
dann Punkt 1 (`fsel_exinput`). Punkt 4 bleibt nach der Entscheidung oben
liegen; der bestehende Pfad bekommt den Verweis darauf im Quelltext.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss

**2026-09-23.** Punkte 1-3 gebaut, Punkt 4 unangetastet nach der
Lead-Entscheidung oben.

1. **Dateiauswahl.** `inf::app_read_inf` fragt mit `fsel_exinput(path, fname,
   &button, STRDINF)` (`deskapp.c:1500-1508`), Vorgabe `INF_FILE_WILD` =
   `C:\*.INF` und `INF_FILE_NAME+3` = `NEWDESK.INF`, Ueberschrift
   `Select .INF file` (`desk_rsc.c:2488`); der gewaehlte Name wird mit
   `strcpy(filename_start(path), fname)` (`:1509`) angehaengt, und
   `rc == 0 || button == 0` bricht ab. Die AES-Bindung dafuer fehlte im
   Desktop und ist neu: `desk/mod.rs::fsel_exinput`, Opcode 91,
   `AES_CTRL_CODE(FSEL_EXINPUT, 0, 2, 3)` wie `desk/gembind.c:702-712`.
2. **Signaturpruefung.** `is_inf_signature` verlangt `#R` **oder** `#E`
   (`deskapp.c:1523-1525`). Der Bootpfad in `inf::load_and_apply` behaelt
   bewusst die schwaechere Regel (`deskapp.c:833`, nur `'#'`) — beide sind
   EmuTOS', und sie sind nicht austauschbar: beim Start steht dort, was
   `shel_get` geliefert hat, beim Lesen eine gerade ausgesuchte Datei. Die
   Stelle sagt das jetzt selbst.
3. **Rueckmeldung.** `EFILNF` → `STFILENF` = `[1][%s not found.][  OK  ]`
   (`desk_rsc.c:2548`) mit eingesetztem Namen (`fun_alert_merge`,
   `deskfun.c:88`); alles andere — anderer Fehler, leere Datei, falsche
   Signatur — → `STINVINF` = `[1][Illegal .INF file format.][  OK  ]`
   (`desk_rsc.c:2560`). Beide ueber `dir::fun_alert`.
4. **Nicht gebaut, wie entschieden.** Der Neustartpfad bleibt liegen. Der
   bestehende Pfad traegt den Verweis darauf im Quelltext
   (`main::do_read_inf`, mit `deskmain.c:919-925` und `:2136-2148`) und in
   `docs/api/desktop/deskapp.md` und `deskmain.md`. Es wurde **nichts** um
   sein Fehlen herumgebaut.

**Benannte Abweichungen** (jeweils mit Quelle im Code):
* `main::do_read_inf` — kein Desktop-Neustart, stattdessen Nachziehen im
  laufenden Desktop (`deskmain.c:919-925`, `:2136-2148`). Lead-Entscheidung.
* `inf::INF_FILE_WILD` — Laufwerksbuchstabe fest `C:` statt `*path +=
  G.g_stdrv` (`deskapp.c:1502`), weil NewTOS' `INF_PATH` ebenso fest ist; die
  EmuTOS-Invariante „Auswaehler-Laufwerk = INF-Laufwerk" bleibt.
* `inf::FSEL_PATH_LEN`/`FSEL_NAME_LEN` — 128/32 statt `MAXPATHLEN`/
  `LEN_ZFNAME` (81/13), weil unser AES eigene 96- und 16-Byte-Felder
  zurueckschreibt (`tos-api/src/aes/fsel.rs:135-136`). Nichts Beobachtbares.
* `inf::read_file` liest bis `MAX_INF_BYTES` (64 KiB) statt bis
  `SIZE_SHELBUF-CPDATA_LEN-1` (`sysconf.h:74`) — EmuTOS' Grenze ist die des
  Shell-Puffers, in den es die Datei legt; ohne Punkt 4 gibt es den hier
  nicht. Folge einer Mechanik, nicht des Vertrags.

**Pruefung:** Hostsuite **938 / 0 / 1** (Basislinie, Delta 0), beide Boards
frisch gebaut (`cargo clean -p kernel -p tos-api` vor jedem) `tos-api 64 /
kernel 94` (Delta 0), Clippy `80 / 125` auf beiden Boards (Delta 0), null
Treffer in `kernel/src/desk/`. Vier QEMU-raspi4b-Laeufe (640x480,
`-display none`, QMP, Injektor), 22 Screendumps, `fsck_msdos -n` auf allen
vier Kartenabbildern sauber, 0 `[EXC]`/`[PANIC]`.
Belege: `measurements/measurement-2026-09-23-desk14/`.

**Offen:** Pi-Sichtabnahme (HW-Punkt 41 in
`docs/api/desktop/_hw-test-2026-09-17.md`) — 1920x1080-Optik, die vier
Laufwerke `C:`–`F:` im Laufwerksraster, ein getippter Name auf echter
Tastatur, `Cancel`.
