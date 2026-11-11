# DESK06 — File → Execute EmuCON: Konsole aus dem Desktop starten

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK06-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 7dd743e1f (feat(desk): Read .INF file asks, checks and report), named by body

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen** · Prioritaet: **P3** · Typ: Funktionsluecke desk/

Abhaengigkeiten: —

## Sachverhalt

`Item::ExecEmuCon` ohne Handler. EmuTOS startet seine eingebaute Konsole
(`deskmain.c`, `CLIITEM` → `run_cli`/`pgm_exec`). NewTOS hat eine eigene
Konsole (`kernel/src/shell.rs`, EmuCON-artig, heute nur auf qemu-virt und
ueber die Serielle erreichbar).

## Arbeit

Quelle lesen (wie EmuTOS die Konsole startet und zurueckkehrt), unsere
Konsole aus dem Desktop heraus starten — im Grafikmodus ueber ein
Textfenster oder durch Umschalten nach `sh_toalpha` und zurueck
(`gemshlib.c`); Rueckkehr zum Desktop wie nach einem Programmlauf.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Status:** ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen.
**Artefakt:** uncommitted im Baum auf `desk-rebuild`, Basis HEAD `84587a5`;
Quittung [DESK06-STATUS-DONE.MD](DESK06-STATUS-DONE.MD), Belege in
`measurements/measurement-2026-09-23-desk06/`.

### Der Weg: `sh_toalpha`, nicht ein Textfenster

Die Frage des Tickets („Textfenster oder Modusumschaltung") beantwortet die
Quelle und nicht der Geschmack: **EmuTOS kennt kein Textfenster im
Grafikschirm.** `pro_run(FALSE, DEF_CONSOLE, …)` (`deskmain.c:753`) traegt
`isgraf = FALSE` in `shel_write` ein, `sh_write` macht daraus `sh_isgem = 0`
(`gemshlib.c:173`), und `sh_chgrf` (`:513-523`) schaltet daraufhin mit
`sh_toalpha` (`:246-260`) den Schirm ab — Pfeilzeiger, `ratexit`,
Workstation zu — und mit `sh_tographic` (`:223-239`) wieder an. Die
Gedaechtnisregel „eine gestartete TOS-Anwendung besitzt den ganzen Schirm"
gilt fuer die Konsole **staerker** als fuer eine Anwendung: EmuCON zeichnet
nicht ueber den Desktop, der Desktop-*Prozess* endet und EmuCON hat die
Maschine.

### Was gebaut wurde

* Neu [`kernel/src/desk/pro.rs`](../kernel/src/desk/pro.rs) = EmuTOS
  `desk/deskpro.c`: `pro_run` (Kommandoschwanzbild: Laengenbyte, NUL,
  `0x0D` — „just like Atari TOS", `deskpro.c:57`), `pro_exec`
  (Sanduhr + `shel_write`, Sanduhr nur bei **Misserfolg** wieder weg) und
  `desk_busy_on`/`desk_busy_off` (`desksupp.c:1840-1852`).
* Zwei neue aesbind-Wrapper in `desk/mod.rs`: `shel_write` (121,
  `AES_CTRL_CODE(SHEL_WRITE, 3, 1, 2)`) und `shel_wdef` (127,
  `(SHEL_WDEF, 0, 1, 2)`) — `desk/gembind.c:907-968`. Beide gehen ueber
  `svc #2`; im AES-Protokoll des Laufs stehen sie in EmuTOS' Reihenfolge.
* `main::do_execcli` = `deskmain.c:750-770` Zeile fuer Zeile, einschliesslich
  des `CONF_WITH_PCGEM`-Schrittes (Verzeichnis des obersten Fensters an
  `shel_wdef`, damit `sh_chdef`s `CONSOLE_APP`-Zweig EmuCON dort startet).
* `menu::item_enabled`: `Item::ExecEmuCon => true`. Der `_ => false`-Zweig,
  der ihn grau hielt, und sein Kommentar sind weg.
* `shell_loop::sh_run_pending_console` — siehe die Abweichung unten.

### Die eine benannte Abweichung

`do_execcli` liefert `done = TRUE` und die Ereignisschleife endet, wie in
EmuTOS. **Danach** weicht NewTOS ab: dort endet der Desktop-Prozess und
`sh_main` (`gemshlib.c:708-731`) startet das wartende Programm und danach den
Desktop von vorn. NewTOS' Desktop ist ein Kernelaufruf, der sich nicht neu
starten laesst — `desk::global::init` ist einmalig, es gibt kein `win_start()`
beim Wiedereintritt, und das zwischenzeitliche `wm_init()` haette die
Fensterliste schon geloescht, auf die der neu eingetretene Desktop sich
berufen wuerde. Das ist derselbe Neustartpfad, den [DESK14](DESK14.md) §4
ausdruecklich als **Lead-Entscheidung** zurueckhaelt.

Also faehrt `desk::run` jetzt eine Schleife: `event_loop()` → wartenden Lauf
**verschachtelt** durch den Schleifencode selbst
(`shell_loop::sh_run_pending_console`) → neu zeichnen → `event_loop()`. Die
Funktion ist ein Durchgang von `sh_main`s Rumpf fuer den `CONSOLE_APP`-Fall
(`sh_chdef`, `sh_chgrf`, `sh_ldapp`s Konsolenzweig, dann `sh_chdef`/
`sh_chgrf` des naechsten Durchgangs) — **ohne** `wm_init()` und
`sh_draw(…, TRUE)` (`:717-719`), weil beide den Schirm und die
AES-Fenstertabelle fuer einen Desktop vorbereiten, der gleich neu gebaut
wird, und genau den Zustand zerstoeren wuerden, den der noch stehende
Desktop gleich zeichnet (Luecke G2, auf dem GEM-Startpfad schon so benannt).

### Tatsaechlich ausgefuehrte Pruefung

`cargo run -p xtask -- test` **932 / 0 / 1** (Basis 932/0/1 — kein Hosttest
hinzugekommen; der ganze Pfad liegt in der `no_std`-aarch64-Kiste `kernel`,
die die Hostsuite nicht faehrt). Beide Boards frisch gebaut
(`cargo clean -p kernel -p tos-api --target aarch64-unknown-none-softfloat`
vor **jedem**): **tos-api 64 / kernel 94** auf rpi4 **und** qemu-virt.
Clippy **80 / 125** auf beiden, Delta 0, **null Treffer** in
`kernel/src/desk/`. Protokolle in `measurements/measurement-2026-09-23-desk06/build/`.

QEMU raspi4b, 640x480, `-display none`, QMP-Screendumps, Injektor in der
Kratzkopie (`measurements/measurement-2026-09-23-desk06/method/`), vier Laeufe, alle ohne
`[EXC]`/`[PANIC]`, `fsck_msdos -n` auf allen drei Kartenabbildern sauber:

1. **probe** — der Eintrag ist schwarz, alle anderen grau (Objektzustaende
   im seriellen Auszug: `state=0x8` ueberall, `0x0` auf Eintrag 13).
2. **cli** — Konsole im Vollbild ohne Mauszeiger; `ver` und `dir`
   hineingetippt und beantwortet (26 Eintraege in `C:\`); `exit` bringt den
   Desktop **pixelgleich** zurueck (0 Pixel Unterschied gegen das Bild
   davor).
3. **wdef** — mit offenem Fenster: `op 121` dann `op 127` im AES-Protokoll,
   `pwd` in der Konsole antwortet mit dem Verzeichnis. Der Schirm danach
   unterscheidet sich in genau **zwei** Punkten: (593,43) und (48,444), die
   beiden Eckpunkte des Fensterrahmens, vorher gruen, nachher weiss.
4. **redraw (Kontrolllauf, ohne Konsole)** — derselbe Rahmen, aber ueber den
   **vorhandenen** `.NCP`-Startpfad (`main::launch_native` → dieselbe
   `redraw::redraw_desktop`): **dieselben zwei Punkte** kippen. Der
   Unterschied gehoert der Ganzschirm-Neuzeichnung gegenueber `wind_open`s
   Rahmenzeichnung und ist nicht von diesem Ticket. Er braucht ein eigenes
   Ticket.

**Offene Hardwareabnahme:** Pi 4, HW-Checkliste Punkt **40**
(`docs/api/desktop/_hw-test-2026-09-17.md`) — der `shel_wdef`-Schritt mit
einem echten Unterordner, zwei Laeufe hintereinander, `Ctrl-Z`, und die
1920x1080-Optik des Textschirms.
