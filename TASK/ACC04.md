# ACC04 — Desk-Menue: registrierte Titel, `AC_OPEN`/`AC_CLOSE`

Status: DONE (2026-11-11) | Priority: P2 | Commit: 96f178544 | Receipt: ACC04-STATUS-DONE.MD

Migrated (owner batch 18, 2026-11-11, to DONE), evidence: receipt TASK/ACC04-STATUS-DONE.MD exists, commit 96f178544 (feat(aes): NewTOS can run desk accessories), VERIFIED: 96f178544 (feat(aes): NewTOS can run desk accessories) names the id by body, its message names the ticket-title identifier ac_open, ac_close, touches code (31 files), legacy line: ERLEDIGT (2026-09-25)

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-25) · Prioritaet: **P2** · Typ: Funktionsluecke

Abhaengigkeiten: [ACC01](ACC01.md), [ACC03](ACC03.md)

## Sachverhalt

`menu_register` (35) legt bei uns Titel und `ap_id` ab
(`tos-api/src/aes/menu.rs`), mehr nicht: die Titel erscheinen in keinem
Menue, und niemand postet `AC_OPEN`. Der Kommentar im Code nennt das
ausdruecklich als offenen Punkt. EmuTOS traegt die Titel unter `THEDESK`
ein und postet beim Anklicken `AC_OPEN` an den Besitzer; `AC_CLOSE` geht an
alle Accessories, wenn eine Anwendung startet bzw. endet (`mn_cleanup`
`aes/gemmnlib.c:740-760`, Aufrufer `ap_exit` `gemaplib.c:254`, `sh_ldapp`
`gemshlib.c:652`). Genaue Woerter und Fundstellen liefert ACC01.

## Arbeit

1. Registrierte Titel in den `THEDESK`-Teilbaum des jeweils installierten
   Menuebaums eintragen (auch wenn die Anwendung ihren eigenen Baum
   installiert — EmuTOS' Regel aus ACC01 uebernehmen).
2. Klick auf einen Accessory-Titel → `AC_OPEN` mit dem Menue-Index an den
   Besitzerprozess.
3. `AC_CLOSE` an alle beim Programmstart und -ende (unser Sweep-Punkt aus
   A107 ist der Ort fuer das Ende).
4. Kernel-Desktop (`kernel/src/desk/menu.rs`): Platz fuer die Titel unter
   *Desk*, wie EmuTOS' Desktop es baut (`desk/desk_rsc.c`, `deskmain.c`).

## Abnahme

Hosttest: zwei registrierte Accessories erscheinen unter *Desk*, Klick
liefert `AC_OPEN` an den richtigen Slot, Programmstart liefert `AC_CLOSE` an
beide. QEMU: Titel sichtbar, Klick oeffnet das Testaccessory-Fenster.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-25)** — zusammen mit ACC02, ACC03 und ACC05.

**1. Titel im `THEDESK`-Teilbaum.** `menu_fixup` (`aes/gemmnlib.c:89-125`) ist
gebaut und laeuft an EmuTOS' beiden Stellen: in `mn_bar`s Anzeige-Zweig
(`:714`) — also bei **jedem** installierten Menuebaum, auch dem einer
Anwendung — und in `mn_register` (`:808`). Die Desk-Schachtel wird jedesmal neu
verkettet: `cnt = accreg ? 2 + accreg : 1`, d.h. ohne Accessory steht dort nur
*Desktop info...* (der Trenner verschwindet mit), mit zweien vier Eintraege.
`gl_dafirst`, `acc_display[]`/`build_menuid_lookup` und `mn_getownid` sind
dabei.

**2. Klick → `AC_OPEN`.** `hctl_rect`s Zweig (`aes/gemctrl.c:337-350`) sitzt in
`event.rs::menu_intercept`: `title == THEDESK && item >= gl_dafirst` →
`mn_getownid(item - gl_dafirst)` → Titel enthilfen (`do_chg(…, SELECTED, FALSE,
TRUE, TRUE)`, `:341`) → `AC_OPEN` an den **Besitzerprozess**, `msg[3] = THEDESK`,
`msg[4] =` Menue-Id. Besitzer ist die AES-Identitaet des Registrierenden
(EmuTOS' `rlr`, `:805`) und nicht `int_in[0]` — bei uns zwingend, weil jede
Anwendung der Kette Pid 0 meldet.

**3. `AC_CLOSE` bei Programmstart und -ende.** `mn_cleanup`
(`gemmnlib.c:740-758`) an beiden EmuTOS-Aufrufstellen: in `appl_exit`
(`gemaplib.c:253`) — das ist der Programm**start**, weil der Desktop vor jedem
Start `appl_exit` ruft — mit dem nachfolgenden `wait_for_accs(AP_ACCLOSE)`
(`:255`), und in `sh_ldapp`s `AP_OPEN`-Aufraeumung (`gemshlib.c:653`,
`pd::close_if_open`) fuer das Programm**ende**. `AP_ACCLOSE` setzt der
Ereignisverwalter selbst (`gemsuper.c:139-140,160-161`), nie das Accessory.

**4. Platz im Kernel-Desktop.** `kernel/src/desk/menu.rs` hat die sechs
`Entry::Acc`-Platzhalter, die EmuTOS' `ADMENU` als `"1"`…`"6"` reserviert
(`desk/desk_rsc.c:345-407`, Objekte 9…16) — ohne sie haette `menu_fixup` nichts,
worin es die Titel einhaengen koennte. Unsere Titelobjekte liegen wie dort bei
`dabox + 3 …`, `THEDESK` ist Objekt 3.

**Belege.** `measurements/measurement-2026-09-25-acc02-acc04/screendumps/`:
`b-deskmenu.png` (beide Titel unter *Desk*), `c-alert.png`/`e-alert2.png`
(`AC_OPEN` an den jeweils richtigen Slot — zwei Prozesse, zwei Menue-Ids),
`noacc-deskmenu.png` (ohne Accessory nur *Desktop info...*, EmuTOS' `cnt = 1`);
`serial/acc2-serial.log`: `AC_CLOSE seen by all: true` **vor** dem Programmstart.

**Eine benannte Luecke.** In einer *Anwendung* eigenem Menuebaum erscheint der
Titel heute nicht: die Titelkopie liegt im Kernelspeicher, und ein
App-eigener Baum loest jede Adresse ueber die Abbildung der App auf (A97), wo
eine Kernel-VA keine Uebersetzung hat — der Eintrag wird dann schlicht nicht
gezeichnet, nie ein Fehler. Loesung ist eine gemeinsame, nur lesbare Titelseite
in jedem Prozess; eigenes Ticket, siehe `docs/api/aes/accessories.md` §9.4.
