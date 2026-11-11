# DESK37 — Hinter einer laufenden GEM-Anwendung steht Weiss statt des Desktop-Hintergrunds

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt DESK37-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 081ef4be5 (feat(desk): desktop background behind an app, drag), named by body

Line 3 before the migration:
Status: **ERLEDIGT** · Prioritaet: **P2** · Typ: Belegter Defekt — Pi-Runde + Hatari-Gegenprobe 2026-09-24

Abhaengigkeiten: [DESK35](DESK35.md) (erledigt)

## Sachverhalt

DESK35 raeumt den Schirm vor dem Start einer GEM-Anwendung. Er bleibt danach
**weiss**. EmuTOS zeigt dort den **Desktop-Hintergrund** (bei unserer
`#Q`-Einstellung gruen) samt Menueleiste.

**Gemessen, nicht geschlossen** (`measurements/measurement-2026-09-24-hatari-appbg/`):
Hatari 2.6.1, Falcon/VGA, EmuTOS 1.4.0+ `etos512us.img`, Start ueber die
`#Z`-Autorun-Zeile (also ueber den Desktop-Startweg). Zwei Laeufe, beide mit
gruenem Hintergrund:

* ein minimales m68k-GEM-Programm, das nur ein Fenster oeffnet und **kein**
  `v_clrwk` macht — Hintergrund gruen, Menueleiste `BGWIN.PRG`;
* das echte m68k-QED — Hintergrund gruen, QEDs eigene Menueleiste.

## Die Stelle

Zwei Schritte, nicht einer:

```
sh_main   gemshlib.c:715-720   if (gl_shgem) { wm_init(); ratinit();
                               sh_draw(D.s_cmd, TRUE); }      <- weiss loeschen
sh_ldapp  gemshlib.c:633       sh_show(fname);                <- und DANN das
sh_show   gemshlib.c:291-297   sh_draw(lcmd, FALSE)
sh_draw   gemshlib.c:280-288   ted->te_ptext = lcmd;
                               ob_draw(tree, ROOT, MAX_DEPTH)
```

`sh_draw(…, TRUE)` setzt `tree[ROOT].ob_spec = CLEAR_SCREEN` (weiss, `:64`)
und zeichnet **nur** die Wurzel; `sh_show(fname)` zeichnet danach den
**ganzen** Baum mit dem urspruenglichen `ob_spec` — Hintergrundmuster und
Menueleiste, in der der Programmname steht.

NewTOS macht seit DESK35 den ersten Schritt und nicht den zweiten.

**Merke fuer die Diagnose:** `CLEAR_SCREEN`s Kommentar „white desktop screen"
ist wahr und trotzdem irrefuehrend — er beschreibt einen Zwischenzustand von
Millisekunden. Genau daran bin ich zuerst haengengeblieben.

## Arbeit

`sh_show`/`sh_draw`s `FALSE`-Zweig nachziehen und an EmuTOS' Stelle rufen —
in `kernel/src/shell_loop.rs` bzw. `desk::main::desktop_relinquish`, direkt
nach dem Loeschen. Vorher klaeren, woher unser Gegenstueck zu
`rs_trees[DESKTOP]` seinen `ob_spec` nimmt: die `#Q`-Farbe fuehrt der Desktop
in `g_patcol` (DESK09), und der ist zu dem Zeitpunkt gerade dabei, die AES
abzugeben — es muss also die **AES**-Seite sein, die das Muster haelt.

Ob die Menueleiste den Programmnamen zeigt, ist Teil desselben Schrittes
(`ted->te_ptext = lcmd`) und gehoert mit gemessen.

Abnahme: QED aus dem Desktop starten, Hintergrund **gruen**; Vergleichsbild
`measurements/measurement-2026-09-24-hatari-appbg/screendumps/emutos-qed-falcon-tc.png`.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

Behoben. `sh_ldapp`s zweiter Schritt fehlte: `sh_show(fname)` (`gemshlib.c:632`),
also `sh_draw(lcmd, FALSE)` — der **ganze** `DESKTOP`-Baum mit dem echten
`ob_spec`, nachdem `sh_draw(…, TRUE)` ihn weiss geloescht hat.

**Die offene Frage des Tickets, am Quelltext beantwortet:** das Muster haelt
die **AES**. Unser Gegenstueck zu `rs_trees[DESKTOP]` ist der Drei-Objekt-Baum
`DESKTOP` in [`tos-api/src/aes/init.rs`](../tos-api/src/aes/init.rs) (`:139`,
aus `DESKTOP_ROM`, der Ressourcenkonstanten `4419` = `0x1143` von
`gem_rsc.c:551-556`); das untere Byte ersetzt `set_aes_background` beim
AES-Start aus der `#Q`-Zeile (`geminit.c:635-641,719-728`, bei uns
`process_inf1_q` + `gem_rsc`). `G.g_patcol` des Desktops (DESK09) ist eine
zweite, unabhaengige Lesung derselben Zeile und spielt hier keine Rolle:
`desktop_relinquish` hat den Desktop-Hintergrundbaum mit
`wind_set(DESKWH, WF_NEWDESK, NULL)` schon abgemeldet.

**Geaendert:** `crate::shell_loop::sh_gem_show`
([kernel/src/shell_loop.rs](../kernel/src/shell_loop.rs)), gerufen aus
`desk::main::run_gem_app` ([kernel/src/desk/main.rs](../kernel/src/desk/main.rs))
zwischen `desktop_relinquish()` und dem Lader — genau `sh_ldapp`s Platz
zwischen dem `sh_main`-Block (`:715-720`) und `dos_exec` (`:645`).
`tos_api::aes::shell::sh_show` gab es bereits (der Bootpfad ruft es fuer den
Desktop selbst); neu ist nur der Aufruf an der zweiten Stelle.

**Wie nah am Hatari-Vergleichsbild** (`measurements/measurement-2026-09-24-desk37-desk36-desk30/`,
QEMU raspi4b 640x480): strukturell deckungsgleich. In beiden Bildern sind die
Zeilen 0..17 weiss mit dem Programmnamen in Schwarz, Zeile 18 durchgehend
schwarz, die Zeilen 19..479 ein 50-%-Schachbrett aus Gruen und Weiss — 320
gruene und 320 weisse Pixel je Zeile in **beiden**, und in derselben Phase,
Pixel fuer Pixel (`WGWGWG…` in Zeile 19, `GWGWGW…` in Zeile 20). Es
unterscheiden sich nur die beiden Farbworte, und das ist Hataris
Falcon-VGA-Rampe: dort `(0,251,0)`/`(255,251,255)`, hier `(0,255,0)`/
`(255,255,255)` — ein Geraeteparameter, kein Unterschied im Zeichnen.

**Die Menueleiste traegt den Programmnamen**, wie EmuTOS' es tut: `GEMWIN.NAP`
bei uns gegen `BGWIN.PRG` dort. Bei QED ueberschreibt QEDs eigene Leiste ihn
sofort — in beiden Systemen.

**Belege:** `d37g-after-C-app.png` gegen
`measurements/measurement-2026-09-24-hatari-appbg/screendumps/emutos-bgwin-falcon-tc.png`,
`d37q-after-D-qed.png` gegen `emutos-qed-falcon-tc.png`; `d37g-before-C-app.png`
und `d37q-before-D-qed.png` zeigen den Defekt (alles unter der Leiste weiss).
Spec: [docs/api/aes/shell-loop.md](../docs/api/aes/shell-loop.md) §9,
Abschnitt „`sh_show(fname)`, the screen behind a launched GEM program".
Hardware-Gegenprobe: Punkt 61 in
[docs/api/desktop/_hw-test-2026-09-17.md](../docs/api/desktop/_hw-test-2026-09-17.md).
