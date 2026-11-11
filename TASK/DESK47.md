# DESK47 — `opn_appl`: der Parameterdialog fehlt, und ohne ihn ist `.NTP` sinnlos

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: 4b3d73828 | Missing: tester acceptance through cargo xtask accept

Migrated (owner batch 17, 2026-11-11, to IMPLEMENTED), evidence: commit 4b3d73828 (feat(desk): opn_appl asks a .NTP for its parameters, and the line reac), VERIFIED: 4b3d73828 (feat(desk): opn_appl asks a .NTP for its parameters, and the) names the id by body, its message names the ticket-title identifier opn_appl, touches code (47 files), legacy line: ERLEDIGT (2026-09-30, auf der Pi abgenommen)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-30, auf der Pi abgenommen) · Prioritaet: **P2** · Typ: Fehlende EmuTOS-Funktion — Anwenderbefund Pi-Runde 2026-09-28

Abhaengigkeiten: [DESK44](DESK44.md) (dort als „nicht einloesbare Haelfte" vermerkt)

## Der Befund

Anwender, nach dem Doppelklick auf ein `.NTP`: *„da oeffnet sich kein
Argument-Eingabefenster; der Desktop blitzt kurz auf und das war's."*

**Genau so ist es gebaut.** DESK44 hat `.NTP` auf den Konsolenarm gestellt —
das war richtig und ist erledigt. Was fehlt, ist der Schritt davor:
`opn_appl` (`references/emutos/desk/deskinf.c:1253-1271`) fragt die
Parameterzeile ab, bevor `pro_run` startet. Ohne ihn laeuft ein `.NTP` mit
**leerer** Parameterzeile — und ist damit von einem `.NCP` nicht zu
unterscheiden.

Die Endung `.NTP` heisst „**T**akes params" (ADR-018). Ohne Dialog traegt sie
ihren Namen nicht.

## Was gebaut wird

`opn_appl` nach `deskinf.c:1253-1271`, in EmuTOS' Form und an EmuTOS' Stelle im
Ablauf: der Dialog kommt aus `pro_run`s Aufrufer, nicht aus dem Startpfad.
Der Baum ist `ADOPENAP` (`desk_rsc.c`), die Vorlage fuer den Nachbau sind
DESK42s `ADDISKIN`/`ADFFINFO` — Objekt fuer Objekt, gleiche Nummern, gleiche
Masse.

**Mitzuklaeren, weil es in derselben Ecke offen steht** (aus dem
U3-Statusbericht): `pro_run`s `(wh, curr)`-Zweig und `do_wopen`
(`desksupp.c:294`) fehlen ebenfalls. Der Zweig war **bis U3 unerreichbar** und
ist es seitdem nicht mehr — das ist eine echte Luecke, keine Formalie.

## Abnahme

* Doppelklick auf ein `.NTP` oeffnet den Parameterdialog; die eingegebene Zeile
  kommt beim Programm an. `C:\\CMDLINE.NTP` gibt sie aus und ist damit der
  Selbstbeleg.
* Screendump des **Konsolenschirms** waehrend des Laufs — der Nachweis, der
  fuer `.NCP` bereits vorliegt (`measurements/measurement-2026-09-27-desk46/screendumps/
  a3-3-console.png`) und fuer `.NTP` noch aussteht.
* Gegenprobe: `.NAP` bleibt GEM ohne Dialog, `.NCP` bleibt Konsole ohne Dialog.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Bestaetigt auf der Pi, 2026-09-30

Zweite unabhaengige Beobachtung, Kern `2597951a`: Doppelklick auf
`C:\CMDLINE.NTP` **aus unserem eigenen Desktop** — kein Kommandozeilen-Dialog,
der Schirm wird „ganz kurz weiss und dann wieder im Desktop".

Damit ist der Befund vom 2026-09-28 reproduziert und die Zuordnung gesichert:

- **Kein Dialog** — das ist dieses Ticket. `opn_appl` ist nicht gebaut, also
  startet `.NTP` mit leerer Parameterzeile.
- **Das kurze Weiss ist kein zweiter Fehler.** Der Konsolenschirm steht
  gemessene 25 ms einschliesslich Programmlauf ([DESK46](DESK46.md)). Das
  Programm **laeuft** und endet; mit blossem Auge ist daraus nichts weiter zu
  schliessen.

Der Anwender hat auf Rueckfrage ausdruecklich bestaetigt, dass es **unser**
Desktop war und nicht TeraDesk. Das ist der Unterschied, der zaehlt: TeraDesk
wuerde bei `appt=4` selbst nach der Kommandozeile fragen (`applik.c:1390`),
unser Desktop kann es ohne `opn_appl` nicht.

---

## Abschluss (2026-09-30) — IMPLEMENTIERT, HW-Abnahme offen

`opn_appl` ist gebaut, an EmuTOS' Stelle im Ablauf, und `ADOPENAP` Objekt fuer
Objekt nach DESK42s Vorlage.

**Fundstelle korrigiert — der Fehler stand im Ticket, also beim Lead.** Dieses
Ticket nannte urspruenglich `deskpro.c:71-121`. Dort steht `opn_appl` nicht:
`deskpro.c` hat in der gepinnten Quelle **64 Zeilen** und enthaelt nur
`pro_exec` und `pro_run`. `opn_appl` steht in
**`references/emutos/desk/deskinf.c:1253-1271`**, der letzten Funktion der
Datei; die Aufrufstelle ist `desksupp.c:969-985` (`do_aopen`).

Die falsche Angabe war in **fuenf** Dateien kopiert — dieses Ticket,
[DESK44](DESK44.md), zwei Quittungsdateien und `docs/STATUS.md`. Alle sind
korrigiert; das ist wieder
[„Eine Begruendung steht selten nur einmal“](../docs/MEMORY.md), diesmal an
einer Quellenangabe statt an einer Begruendung.

Der Baum ist `desk_rsc.c:877-919`, `desk_rsc.h:93-98`, die beiden `TEDINFO`s
`desk_rsc.c:62-104` (Eintraege 12/13). Die Spezifikation ist in
`docs/api/desktop/deskinf.md` nachgetragen (EmuTOS-Verhalten, die Tabelle der
sieben Objekte, beide `TEDINFO`s) — vorher stand dort nur der Zeilenverweis.

### Was gebaut ist

* `kernel/src/desk/inf.rs` — `opn_appl(papname, ptail) -> bool` und
  `inf_sget` (`util/optimize.c:119-126`) als Gegenstueck zu `inf_sset`.
  Sieben Objekte, EmuTOS' Nummern, Beschriftungen, Masse: 40 x (10 Zellen +
  8 px) = 320 x 168, Knoepfe auf Zeile 8 + 8 px. `inf_show(tree, APPLPARM)` —
  das Startfeld ist die Parameterzeile, nicht `ROOT`. `inf_what`s dritte
  Antwort `-1` zaehlt wie in der Quelle als TRUE (`if (inf_what(...))` ist ein
  blanker Nicht-Null-Test).
* `kernel/src/desk/main.rs` — `isparm` (`desksupp.c:877`) neben `isgraf`, und
  der Aufruf in **beiden** `do_aopen`-Auspraegungen: `open_item`s `App`-Arm
  (Doppelklick / Datei→Oeffnen) und `do_aopen_drop`. Dessen `tail` ist jetzt
  `Option<&[u8]>`, weil EmuTOS auf `tail == NULL` verzweigt und nicht auf eine
  leere Zeichenkette: `None` ist `process_funkey` (`deskmain.c:1099` uebergibt
  `NULL`, also **bekommt** ein per Funktionstaste gestartetes `.NTP` den
  Kasten), `Some` sind die drei Wurf-Pfade in `dir.rs`, wo der Tail schon
  feststeht und der Kasten uebersprungen wird (`desksupp.c:969-985`, der
  `tail`-Zweig hat Vorrang).
* Bytes statt `String` fuer den Tail: `APPLPARM`s `te_pvalid` ist `X`, das
  jedes Zeichen durchlaesst, das `check()` durchlaesst — auch solche ueber
  0x7E, die kein UTF-8 sind.
* Drei veraltete Gegenbehauptungen mitgezogen
  ([[justification-is-copied-to-several-places]]): `ftype.rs` („Stage-3
  follow-on"), `docs/sdk/01-overview.md` (Tabellenzeile **und** der
  Einschraenkungspunkt „No `.NTP` tail dialog") und
  `docs/api/desktop/deskmain.md` („NewTOS has no prompt").

### Abnahme, soweit ohne Pi einloesbar

`measurements/measurement-2026-09-30-desk47/` — drei QEMU-raspi4b-Laeufe mit dem
Scratch-IKBD-Injektor, 0 `[EXC]`/`[PANIC]`, `fsck_msdos -n` sauber auf allen
drei Abbildern.

* **Lauf A** (`screendumps/a3-dialog.png`, `a4-typed.png`, `a5-console.png`):
  Doppelklick auf `C:\D47\CMDLINE.NTP` oeffnet den Kasten; getippt `hello`,
  Return; `[dlg] opn_appl tail len 5 = "hello"`, dann
  `sh_ldapp: starting C:\D47\CMDLINE.NTP … sh_isgem=0` und auf dem
  **Konsolenschirm** `CMDLINE: len 5 arg "hello"`. Damit ist auch der
  Screendump-Nachweis erbracht, den DESK44 fuer `.NTP` schuldig geblieben ist.
  Der Baum ist im selben Lauf Objekt fuer Objekt ausgegeben
  (`serial/d47a-serial.log`, `[dlg] obj …`) und in `RUNINFO.TXT` §1 Zeile fuer
  Zeile gegen `desk_rsc.c` gehalten.
* **Lauf B**: `.NCP` bleibt Konsole **ohne** Dialog — keine einzige
  `[dlg]`-Zeile, `CMDLINE: len 0 arg ""`.
* **Lauf C**: *Cancel* startet nichts — `inf_what = 0`, kein `sh_ldapp` fuer
  das Programm, Desktop intakt (`screendumps/c2-after-cancel.png`).
* `.NAP` wurde **nicht** gefahren; der Beleg ist `isparm(Flavor::App) = false`,
  dieselbe Verzweigung, die Lauf B beobachtbar macht. Gehoert in den
  Testerlauf.

**Offen: die echte Pi.** Doppelklick auf `C:\CMDLINE.NTP` aus unserem Desktop,
Zeile eintippen, Return — die Zeile muss auf dem Textschirm stehen. Nur dort
haengt auch eine **physische** Tastatur hinter dem VL805 an der Kette; im
Injektor kommen die Zeichen direkt aus `crate::ikbd::feed`.

### Das zweite Thema: `pro_run`s `(wh, curr)` und `do_wopen` — getrennt, nicht halb gebaut

Aus dem Quelltext festgestellt, nicht geschaetzt:

* `do_wopen` hat in EmuTOS **zwei** Aufrufer: `pro_run` (`deskpro.c:61`) und
  `do_diropen` (`desksupp.c:454`) — und `do_diropen` selbst vier
  (`deskfun.c:413`, `deskmain.c:1496`, `desksupp.c:1067`, `:1156`). Es ist der
  gemeinsame Schwanz **jedes** Fensteroeffnens, kein `pro_run`-Helfer.
* NewTOS hat auch kein `do_diropen`: dessen Rumpf liegt verteilt in
  `open_dir_window_at`, `navigate_window`, `rebuild_window`,
  `open_root_window` und dem `cnx_get`-Wiederherstellungspfad. **Keiner** von
  ihnen macht `do_wopen`s `graf_growbox`-Zoom oder die Abwahl
  `act_chg(…, FALSE, FALSE)` — `do_fopen`s Doc-Kommentar haelt genau diese
  Luecke schon von der Fensterseite fest.
* Es fehlt ausserdem eine `graf_growbox`-Bindung in `kernel/src/desk/mod.rs`
  (AES-Opcode 73 ist in `tos-api/src/aes/graf.rs:356` da, der Desktop hat
  keinen Wrapper). `do_xyfix`, `obj_extent` (= `get_xywh` + Wurzelversatz),
  `act_chg` und `g_wlastsel` sind vorhanden.

`do_wopen` nur fuer `pro_run` zu bauen ergaebe eine Funktion, die den Namen
traegt, ohne es zu sein. **Eigenes Arbeitspaket**, mit eigener Abnahme (der
Zoom ist sichtbar, er gehoert in einen Screendump). Der Befund steht im
Doc-Kommentar von `kernel/src/desk/pro.rs` und in
`docs/api/desktop/deskinf.md` §`opn_appl`, damit er dort steht, wo der
Naechste ihn sucht.

### Basislinien

962/0/1 Hosttests; release nach `cargo clean -p kernel -p tos-api --target
aarch64-unknown-none-softfloat --release` je Board 64/94; Clippy 80/125 in
beiden Feature-Saetzen; `tools/desk-drift-probe.sh` **32** (unveraendert).

---

## Abgenommen auf der Pi, 2026-09-30

Anwender nach der Runde mit Kern `470bfeed`: *„Das Eingabefenster taucht jetzt
auch auf."*

Damit ist die erste Zeile der Abnahme auf echter Hardware erfuellt. Der
Konsolen-Screendump und die Gegenproben (`.NCP`/`.NAP` ohne Kasten) liegen aus
QEMU vor (`measurements/measurement-2026-09-30-desk47/`). **DESK47 ist erledigt.**
