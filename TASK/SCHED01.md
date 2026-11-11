# SCHED01 — `dsptch` legt den Aufrufer nicht in die Laufmenge ein; ein Vollbildprogramm ohne Ereignisschleife haengt in `appl_exit`

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified (only docs(task) commit); receipt SCHED01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 21, 2026-11-11, to PARKED), evidence: owner 2026-11-11 (batch 6 review): only commit 9d6ef6916 is docs(task); receipt TASK/SCHED01-STATUS-DONE.MD names the old hash 3460eef; reopen-derived

Line 3 before the migration:
Status: **BEHOBEN — auf echter Pi 4 abgenommen** (2026-09-29) · Prioritaet: **P1** · Typ: Belegter Defekt / Haenger — aus [PORT08](PORT08.md), gemessen 2026-09-29

Abhaengigkeiten: — · **Tragweite am 2026-09-29 korrigiert, siehe unten**

## Der Befund, direkt gemessen

Letzter Aufruf der Ball-Demo beim Beenden, mit Wire-Argumenten:

```
266 VDI op=101 v_clsvwk  handle=2          -> 0
267 AES op=19  appl_exit ctl=[19,0,1,0,0]  -> KEIN RUECKSPRUNG
```

Die Laufmenge im selben Augenblick:

```
runset cur=2 : s0(run=0 ctx=1 cond=0x200000000000000)
               s1(run=0 ctx=1 cond=0x200000000000001)
               s2(run=0 ctx=0 cond=0x0)
```

**`s2` ist die Anwendung — sie laeuft auf der CPU, steht aber mit `run=0` nicht
in der Laufmenge.** `sched::admit` wird **ausschliesslich** fuer Accessories
gerufen (`tos-api/src/aes/acc.rs:103`); ein gestartetes GEM-Programm wird
nirgends aufgenommen.

**Der Ablauf:** `appl_exit` → `mn_cleanup()` weckt beide Accessories →
`wait_for_accs(AP_ACCLOSE)` → `all_run()` → `sched::dsptch()` gibt die CPU ab
→ die Accessories holen `AC_CLOSE` und blockieren wieder → `pick_next` sucht
`runnable == true` und findet **nichts**: die Anwendung hat `run=0` **und**
`cond=0`, ist also weder waehlbar noch weckbar. Gemessen 103 600 000
Leerlaufdurchgaenge lang unveraendert.

**Warum die Maus dabei lebt:** ein Accessory dreht in `sleep`s Leerlaufausgang,
dessen Rumpf `host().pump_input()` ist. Die Projektregel
[[mouse-moves-but-nothing-responds-means-app-sleeping]] hat hier das richtige
Ziel getroffen, aber **aus dem falschen Grund** — nicht die Anwendung wird
bedient, sondern ein Accessory laeuft leer.

## Gegen die Quelle

EmuTOS' `disp` nimmt den laufenden Prozess vom Kopf von `rlr` und legt ihn ohne
`WAITIN` per `disp_act` → **`insert_process(p, &rlr)` wieder hinein**
(`references/emutos/aes/gemdisp.c:74-79,220-238`). Dort ist der laufende
Prozess **per Konstruktion** auf der Bereitliste.

Unser `sched::dsptch` legt nichts ein, sondern **setzt Mitgliedschaft voraus**
(`tos-api/src/bdos/process/sched.rs:592-605`). Der Kommentar dort — *„will
pick the caller again when it comes round"* — ist fuer ein gestartetes Programm
schlicht falsch. **ACC10s Commit `814de2c` zitiert genau dieses
`insert_process`; der Einfuegeschritt selbst ist nicht mitgekommen.**

## Tragweite — **korrigiert 2026-09-29, meine erste Fassung war zu weit**

**Falsch war:** *„betrifft jede EL0-GEM-Anwendung, sobald mindestens ein
Accessory geladen ist."*

**Der Anwender hat widersprochen** — TeraDesk beendet sich seit langem sauber,
auf derselben Karte, mit denselben zwei Accessories —, und die Gegenmessung
gibt ihm recht. TeraDesk wurde unter demselben Geruest bis zu seinem
`appl_exit` gefahren (*File → Quit*): es kehrt zurueck, `Pterm`t, und der
eingebaute Desktop kommt vollstaendig mit Menuezeile zurueck.

**Ein einziges Bit unterscheidet die Faelle:**

```
Ball-Demo   runset … s2(st=2 run=0 ctx=0 cond=0x0)   appl_exit -> kein Ruecksprung
TeraDesk    runset … s2(st=2 run=1 ctx=0 cond=0x0)   appl_exit -> kehrt zurueck
```

Accessories, `AC_CLOSE`, Barriere und die Reihenfolge der Umschaltungen sind
Schritt fuer Schritt **identisch**.

### Woher TeraDesks `run=1` kommt — und es ist **nicht** `sched::admit`

Auch TeraDesk wird nirgends aufgenommen. Das Bit setzt **`sleep()`s
Leerlaufausgang selbst** (*„WAITING → READY: the caller now re-evaluates its
full predicate"*, `sched.rs`: `runnable.store(true)`) — und es wird danach nie
wieder geloescht.

**Eine Anwendung, die auch nur einmal in einem AES-Warteruf blockiert hat, ist
ab da in der Laufmenge — als Nebenwirkung eines fremden Codepfads, nicht als
Zusage.**

Die Ball-Demo blockiert **nie**: statisch nachgezaehlt ueber `ball.c`,
`ball_neu.c`, `plat-newtos.c`, `ball.h` — **null** Treffer fuer `evnt_*`,
`form_do`, `form_alert`. Sie nimmt den Schirm mit `form_dial(FMD_START)`,
zeichnet selbst und fragt die Taste ueber `Bconstat(2)` ab. Ihr `runnable`
bleibt bis zuletzt das `false` aus `Process::empty()`.

### Die richtige Klasse

**Eine GEM-Anwendung, die `appl_exit` erreicht, ohne je in einem AES-Warteruf
blockiert zu haben, bei mindestens einem registrierten Accessory.** Also:
**Vollbildprogramm ohne Ereignisschleife** — Schirm nehmen, selbst zeichnen,
Taste ueber BIOS abfragen, beenden. Die Ball-Demo ist genau das; TeraDesk und
QED sind es nicht.

### Zwei geprüfte Verdachte, die **nicht** zutrafen

* **`mn_cleanup` nur fuer *offene* Accessories?** Nein — weder bei uns
  (`tos-api/src/aes/menu.rs:1490-1511`) noch bei EmuTOS
  (`aes/gemmnlib.c:740-748`); es geht an jedes **registrierte**. In beiden
  Laeufen haben beide Accessories ihr `AC_CLOSE` bekommen. **Das war ein
  Lead-Verdacht, und er ist widerlegt.**
* **Schirmbesitz (`form_dial(FMD_START)`, `wind_update(BEG_MCTRL)`)?** Nicht
  die Ursache — das unterscheidende Bit ist direkt gemessen.

### Warum die Prioritaet **nicht** sinkt

Der Mechanismus bleibt ein Defekt: `dsptch` setzt Mitgliedschaft voraus, statt
sie herzustellen. Dass es fuer die meisten Anwendungen trotzdem geht, liegt an
einer **beilaeufigen Nebenwirkung** statt an einer Invariante — und genau
deshalb faellt die eine Klasse durch, die diesen Pfad nie betritt.

**Und das betroffene Programm ist das Vorfuehrprogramm fuer den 9./10. Oktober.**

## Gegenprobe, gefahren

Dieselbe Karte **ohne** die beiden `.NAC`: `appl_exit` kehrt sofort zurueck,
die Demo schreibt ihre Schlusszeilen, gibt ihre Puffer frei, der Desktop meldet
seine Menuezeile wieder an (`AES op=30 menu_bar in=[1,…]`) und landet in
`evnt_multi`. Vollstaendiger Desktop im Screendump.

## Kein eigener Defekt: der Streifen und die fehlende Menuezeile

EmuTOS' `fm_dial(FMD_FINISH)` zeichnet die Menuezeile **ebenfalls nicht** neu
(`aes/gemfmlib.c:375-379` → `w_drawdesk` → `do_walk(DESKWH, …)`,
`gemwmlib.c:340-359`). Der Streifen ist der sichtbare Fingerabdruck davon, dass
der Desktop nie wieder die CPU bekommt.

## Vermutet, nicht gefahren

Vor `814de2c` schlief die Barriere auf `wait_input()`; der Slot haette
`cond != 0` gehabt, und `idle_wake_pass` setzt solche Warter bei jeder
Mausbewegung wieder bereit (`sched.rs:704-707`). Auf der interaktiven Pi waere
der Stillstand vorher also **vermutlich von selbst weitergegangen**; heute ist
`cond=0` und keine Weckklasse greift. **Nicht reproduziert** — das haette einen
Rueckbau von Produktcode verlangt.

## Auftrag

`sched::dsptch` muss tun, was `disp_act` tut: den Aufrufer in die Laufmenge
**einlegen**, statt Mitgliedschaft vorauszusetzen. Zu klaeren ist dabei, ob
stattdessen (oder zusaetzlich) der **Programmstart** den Slot aufnehmen muss —
EmuTOS' Anwendung ist per Konstruktion auf `rlr`, und das ist die strukturell
naehere Nachbildung. **Erst lesen, dann entscheiden.**

## Abnahme

Die Ball-Demo beendet sich **mit** geladenen Accessories; der Desktop kommt
vollstaendig zurueck und meldet seine Menuezeile an. Dazu TeraDesk beenden,
ebenfalls mit Accessories. Regressionsflaeche: ACC02–ACC06 (der
Accessory-Weg selbst) und jeder Programmstart.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-29)

**Eine Wirkzeile:**

```rust
PROCESS_TABLE[cur].runnable.store(true, Ordering::Release);   // vor on_yield(cur)
```

Dazu der Kommentar, der den Defekt bisher **begruendet** hat, und die
`Dispatch`-Doku richtiggestellt; `docs/api/aes/accessories.md` §3a ergaenzt
(*„es ist ein Insert, kein No-op"*). Neuer Hosttest
`tos-api/tests/sched_dsptch_admits_the_caller.rs`.

### Die Entscheidung: `dsptch` allein, und warum nicht der Startpfad

EmuTOS hat fuer ein **gestartetes Programm keinen Aufnahmeschritt, weil es
keinen braucht**: `sh_ldapp` laeuft unter der AESPD der Shell — es benennt
`rlr` um und ruft `dos_exec` (`aes/gemshlib.c:604-605,634-636,645`). `pstart`
(→ `drl` → `rlr`) gibt es nur fuer interne Prozesse (`geminit.c:214-227`) und
Accessories (`:255`) — genau wie unser `sched::admit`. **Ein `admit` beim
Programmstart waere eine NewTOS-Erfindung ohne Quellenbeleg, und `dsptch`
bliebe trotzdem falsch.**

`wait_cond` bleibt unberuehrt. Die Auflage des Auftrags (nichts wecken, was
schlafen soll) ist eingehalten: `dsptch` fasst **nur** `running_slot()` an, und
ein fuer sein Kind ausgesetzter Elter ist das nie —
`suspend_parent_for_child` steht **vor** `set_current(child_basepage)`
(`kernel/src/arch/aarch64/exceptions.rs:1787`/`:1794`). Als Testfall
festgeschrieben.

### Die fuenf Beweise, alle gefahren

1. **Die Ball-Demo beendet sich**, dreimal unabhaengig, mit **beiden**
   Accessories: `appl_exit` mit `s2(run=0)` → Umschaltung → **zurueck zu
   slot 2** → `appl_exit` mit `s2(run=1)` → `-> 0` → `Pterm` → `menu_bar` →
   `[desk] desktop up`. `C:\BALL.LOG` **2893 Byte und vollstaendig**.
2. **TeraDesk unveraendert sauber**, dieselben Aufrufnummern wie in PORT08
   Runde 2.
3. **Accessory-Weg unbeschaedigt** — und das haerteste Stueck Beleg:
   **vier der Bildabzuege sind bitgleich mit der ACC02-Abnahme vom
   2026-09-25.**
4. **Alle Startwege**: `\AUTO\`, Doppelklick, `#Z` dreimal (BALL, TERADESK,
   QED), QED mit eigener Menuezeile.
5. **Startbilder bitgleich auf beiden Boards**, serielle Protokolle
   diff-leer.

Basislinien: **962/0/1** (961 + der neue Test), 64/94 beide Boards, 80/125
beide Feature-Saetze, Driftprobe 32.

### Ehrlich benannte Luecken

Kein Bild vom laufenden TeraDesk-Quit-Dialog (dieser Lauf war schneller als
PORT08s; Protokoll und die PORT08-Bilder decken es ab). Die
Doppelklick-Koordinate der 2026-09-25-Liste trifft heute ein anderes Symbol —
der **Startweg** ist gefahren und belegt, das Bild zeigt aber kein laufendes
Programm. **T11 einmal gesehen** (`sched_pexec_parent_suspend`), danach sieben
Laeufe gruen; die Datei ruft `dsptch` nirgends.

### Pi-Gegenprobe

Ball-Demo starten, laufen lassen, mit einer Taste beenden: der Desktop muss
**vollstaendig mit Menuezeile** zurueckkommen, und `C:\BALL.LOG` muss
vollstaendig enden. Der 19-px-Streifen und die tote Menuezeile duerfen nicht
mehr auftreten.

## Hardware-Abnahme (2026-09-29)

Kern `9251440d`, echte Pi 4, **ein** registriertes Accessory
(`C:\ACC\SYSACC.NAC`). Beleg: `measurements/measurement-2026-09-29-sched01/pi-BALL.LOG`.

**`BALL.LOG` ist 2891 Byte und endet mit einer vollstaendigen Zeile** — vorher
1755 Byte, mitten im Wort. Die Demo hat also ihren gesamten Bericht
geschrieben, die Datei geschlossen, ihre Puffer freigegeben und beendet.
**Der Haenger ist weg.**

**Und die zweite Haelfte von [PORT08](PORT08.md) bestaetigt sich dabei
beilaeufig:** Zeile 16 (*„NewTOS screen 1920x1080x16 …"*) ist **weiterhin bei
199 Zeichen abgeschnitten** — das ist der 200-Byte-Berichtspuffer der Demo,
ausserhalb unseres Baums, den wir bewusst nicht angefasst haben. Die Datei
laeuft jetzt nur darueber hinaus weiter, weil nichts mehr haengt. Damit ist die
Trennung „Haenger gehoert uns, abgeschnittene Zeile gehoert der Demo"
gemessen bestaetigt.

**Sichtbar bestaetigt (Anwender, 2026-09-29):** *„der Desktop kam wieder
zurueck und war auch bedienbar."* Damit ist auch die Bildseite abgenommen —
der 19-px-Streifen und die tote Menuezeile treten nicht mehr auf.
