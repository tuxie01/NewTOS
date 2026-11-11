# DESK35 — Beim Start einer GEM-Anwendung bleiben die Desktop-Fenster stehen und frieren ein

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt DESK35-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 16, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by c0906fe4f (fix(desk): a folder navigates its window, a GEM ap), named by body

Line 3 before the migration:
Status: **IMPLEMENTIERT — HW OFFEN** · Prioritaet: **P1** · Typ: Belegter Defekt — Pi-Runde 2026-09-24

Abhaengigkeiten: [DESK06](DESK06.md) (erledigt), [DESK14](DESK14.md) §4 (zurueckgestellt)

## Sachverhalt

Auf der echten Pi 4 bleiben beim Start von QED oder TeraDesk die offenen
Desktop-Fenster auf dem Schirm stehen. Anklicken laesst sich alles, aber
nichts reagiert — sie sind gezeichnete Pixel ohne laufende Ereignisschleife.

Das widerspricht der Projektentscheidung „eine gestartete Anwendung besitzt
den **ganzen** Schirm" und sieht nach einem haengenden System aus.

EmuTOS' Stelle ist `sh_main` (`references/emutos/aes/gemshlib.c:710-720`),
**vor** jedem Programmlauf:

```c
sh_chdef(psh);
sh_chgrf(psh);
if (gl_shgem)
{
    wm_init();                  /* Fenstertabelle zuruecksetzen */
    ratinit();
    sh_draw(D.s_cmd, TRUE);     /* clear the screen */
}
rc = sh_ldapp(psh);             /* jetzt erst laeuft das Programm */
```

DESK06 hat den verschachtelten Lauf gebaut und `wm_init()` + `sh_draw(…,
TRUE)` **bewusst ausgelassen** (Luecke G2), weil der Desktop danach ohnehin
neu zeichnet — das stimmte fuer die **Konsole**, deren Textschirm alles
ueberdeckt. Fuer eine GEM-Anwendung stimmt es nicht: sie zeichnet nur in ihre
eigenen Rechtecke, und der Rest des Schirms bleibt stehen.

## Abgrenzung — was dieses Ticket NICHT ist

Nicht der vollstaendige Desktop-Neustartpfad. Der ist [DESK14](DESK14.md) §4
und bleibt zurueckgestellt. Hier geht es um die zwei Schritte **vor** dem
Programmlauf, die EmuTOS auch im verschachtelten Fall braeuchte.

## Arbeit

Erst am Quelltext klaeren, was `wm_init` und `sh_draw` in unserem
verschachtelten Lauf jeweils anrichten wuerden (DESK06s Begruendung fuer das
Auslassen steht in `kernel/src/shell_loop.rs` und ist zu pruefen, nicht zu
uebernehmen) — insbesondere, ob die AES-Fenstertabelle danach noch den
Zustand traegt, den der zurueckkehrende Desktop erwartet, oder ob dafuer
`cnx_put`/`cnx_get` einspringen muessen. Dann den Schirm vor dem Start
raeumen und nach der Rueckkehr wiederherstellen.

Abnahme auf der Pi: zwei Fenster offen, QED starten → **kein** Fensterrest
sichtbar, QED besitzt den Schirm; QED beenden → Desktop mit seinen zwei
Fenstern zurueck. Dasselbe mit TeraDesk.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**Implementiert, QEMU-belegt, Pi-Abnahme offen.** Quittung:
[DESK35-STATUS-DONE.MD](DESK35-STATUS-DONE.MD).

**Was DESK06s Begruendung wert war — geprueft, nicht uebernommen.** Sie steht
in `kernel/src/shell_loop.rs` bei `sh_run_pending_console` und lautete: beide
Schritte bereiteten einen Desktop vor, der gleich neu gebaut wird, also
zerstoerten sie hier genau den Zustand, der danach gezeichnet wird. Am
Quelltext stellt sich heraus: fuer die **Konsole** ist das Auslassen nicht
bloss vertretbar, es ist EmuTOS' eigene Entscheidung. Die drei Anweisungen
stehen in `if (gl_shgem)` (`gemshlib.c:715-720`), und der Konsolenlauf hat
`gl_shgem` in `sh_chgrf` (`:713`) gerade auf FALSE gesetzt — EmuTOS
ueberspringt den Block dort selbst. Es kann sich das leisten: der Textschirm
deckt alles ab, und die Fenster, die der Desktop in der AES-Tabelle
zuruecklaesst, raeumt der **naechste** Grafikdurchgang ab. Fuer eine
GEM-Anwendung bleibt `gl_shgem` TRUE, EmuTOS nimmt alle drei Schritte, und es
muss sie nehmen. DESK06s Begruendung war also fuer die Konsole richtig und
fuer den GEM-Pfad falsch — sie ist dort jetzt beim Namen genannt statt
vererbt.

**Was `wm_init` anrichtet, und was dafuer einspringt.**
`tos_api::aes::window::wm_init` ist `wm_new` + Desktop-Fenster-Reset: jedes
Fensterhandle wird geschlossen und geloescht, `gl_newdesk` auf NULL gesetzt.
Die AES-Fenstertabelle traegt danach **nichts** mehr von dem, was der
zurueckkehrende Desktop erwartet — seine `WNODE`s halten tote Handles. Genau
das ist auch EmuTOS' Lage, und genau deshalb oeffnet sein neugestartetes
`deskmain` die Fenster aus dem Kontextspeicher wieder (`cnx_get`,
`deskmain.c:2055`). `cnx_put`/`cnx_get` (DESK09) sind hier also keine
Alternative, sondern die Voraussetzung: `desktop_relinquish` ruft `cnx_put()`
(wie bisher, `deskmain.c:2154`), `desktop_resume` ruft jetzt `cnx_get()`.
Luecke G2 ist damit zu.

`sh_draw(D.s_cmd, TRUE)` ist dagegen harmlos fuer den Zustand — es malt nur
(`CLEAR_SCREEN` ueber den ROOT des eingebauten DESKTOP-Baums,
`gemshlib.c:64,269-289`).

**Abgrenzung eingehalten:** kein Desktop-Neustartpfad. `crate::desk::run`
kehrt weiterhin nicht zurueck, `desk::global::init` bleibt einmalig, DESK14 §4
bleibt zurueckgestellt. Gebaut wurden nur die zwei Schritte **vor** dem
Programmlauf (`crate::shell_loop::sh_gem_enter`), ihr Gegenstueck danach
(`sh_gem_leave` = `wm_new`, `gemshlib.c:676-677`) und der
`deskmain`-Prolog-Anteil, den `cnx_get` braucht.

**Abnahme (QEMU raspi4b, `measurements/measurement-2026-09-24-desk34-desk35/`, §3/§4):**
zwei Fenster offen, dann gestartet. `d35q-after-E-qed.png` — QED besitzt den
Schirm, kein Fensterrest; `d35q-before-E-qed.png` — der Defekt: QEDs
Menueleiste ueber dem eingefrorenen Desktop. `d35-after-E-back.png` — nach dem
Programmende ist der Desktop mit **beiden** Fenstern an ihren alten Stellen
zurueck. TeraDesk ist nicht gefahren (kein Selbstbeenden im Skript) und steht
mit QED als Pruefpunkt 60 in `docs/api/desktop/_hw-test-2026-09-17.md`.

Benannte Abweichungen: `docs/api/aes/shell-loop.md`, Abschnitt
„the `gl_shgem` block for a GEM program launched by the built-in desktop
(DESK35)".
