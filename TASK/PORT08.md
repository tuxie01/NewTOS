# PORT08 — Die Ball-Demo parkt beim Beenden in einem AES-Aufruf

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt PORT08-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 20, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: 39d8c53e2 (docs(task): SCHED01 - a launched GEM program is never in the) added the receipt TASK/PORT08-STATUS-DONE.MD itself, the report of this research or measurement ticket (16 files), bookkeeping or bulk commit, not accepted as evidence

Line 3 before the migration:
Status: **GEKLAERT — Ursache getrennt** (2026-09-29) · Prioritaet: **P2** · Typ: Belegter Defekt — Pi-Lauf 2026-09-28

Abhaengigkeiten: [PORT07](PORT07.md) (dort als ungeprueft benannt)

## Der Befund

Echte Pi 4, Kern `09f3e928`, 1920x1080. Die konforme Ball-Demo laeuft, liefert
ihre Messung — und **laesst sich nicht beenden**.

**Anwender auf die Rueckfrage: das Bild blieb stehen, die Maus lebte noch.**

Nach der Projektregel [[mouse-moves-but-nothing-responds-means-app-sleeping]]
heisst das: **die Anwendung schlaeft in einem AES-Aufruf.** `pump_input` laeuft
in der AES-Warteschleife, die Maus lebt also gerade **weil** die AES weiter
bedient wird. Es ist damit **kein** Kernel-Haenger wie [PORT05](PORT05.md) oder
[DESK46](DESK46.md) — dort war die Maus jedes Mal tot.

## Der zweite Beleg: die Logdatei bricht mitten im Wort ab

`C:\BALL.LOG`, gesichert als
`measurements/measurement-2026-09-28-port07/raw/pi01-BALL.LOG`: **1755 Byte, und das letzte
Zeichen ist das `l` von `cal`** — kein Zeilenende, kein Abschluss, mitten im
letzten Satz:

```
… one page over vro_cpyfm -- no XBIOS screen-base cal
```

`plat_close()` lief also weit genug, um die Zusammenfassung zusammenzustellen
**und mit dem Schreiben zu beginnen**, und hoerte mittendrin auf.

## Damit ist der Suchraum fuenf Aufrufe gross

Der Ausstieg von `plat_close()`, in seiner Reihenfolge:

1. `Fwrite` der Zusammenfassung — **hier bricht der Text ab**
2. `Fclose`
3. Palettenwiederherstellung (`vq_color`/`vs_color`)
4. `v_clsvwk`
5. `form_dial(FMD_FINISH)` und `appl_exit`

**Welcher es ist, ist ungemessen.** Die Regel dazu ist benannt: beim naechsten
Lauf den **letzten AES-Aufruf samt seinen Wire-Argumenten** feststellen, bevor
irgendwer eine Schicht verdaechtigt — eine tote Prozessebene zu vermuten war in
diesem Projekt schon mehrfach falsch.

## Wo der Fehler liegen koennte — und warum das offen bleibt

Er kann **in der Demo** stecken (`plat-newtos.c`, ausserhalb des Repos) oder
**bei uns**. Das ist ein echter Unterschied: im ersten Fall kostet die Reparatur
nichts am Kernel und ist risikofrei vor dem Termin, im zweiten nicht.

Nicht als Diagnose, sondern als Nachbarschaft: `appl_exit` ist die Stelle, die
[ACC10](ACC10.md) am 2026-09-26 repariert hat, und auf der Karte sind
Accessories geladen. Ob das hier beteiligt ist, ist **ungemessen** — ACC10s
Abnahme lief headless und ohne GEM-Anwendung davor.

## Nachtrag 2026-09-29 — der Suchraum ist zwei Aufrufe gross

Zweiter Pi-Lauf, mit mehr Wartezeit. **Der Anwender korrigiert seine eigene
erste Beschreibung:** beide Laeufe sehen **gleich** aus, nicht verschieden. Es
ist also **deterministisch**.

**Das Bild:** gruener Desktop-Hintergrund, und am oberen Rand ein Streifen
ueber die volle Breite, in dem noch der Hintergrund der Demo steht. Hoehe nach
Augenmass schwer zu schaetzen; die Arbeitsflaeche meldet sich als 1920x1061,
also sind **19 px** fuer die Menuezeile reserviert.

**Der entscheidende Satz des Anwenders: *„wenn ich ueber die Stellen hovere, wo
die Menues sind, kommt nichts."***

**Damit ist es kein Neuzeichenfehler.** Waere es einer, waeren die Menues da
und nur unsichtbar — ein Klick wuerde trotzdem aufklappen. Dass nichts kommt,
heisst: **die Menuezeile ist nicht angemeldet.** Zustandsproblem, nicht
Zeichenproblem.

### Was daraus folgt

Der gruene Hintergrund ist zurueck, also **hat `form_dial(FMD_FINISH)`
gearbeitet** und den Schirm zurueckgegeben; die AES hat gezeichnet, was sie von
sich aus zeichnen kann. Was fehlt, ist alles, wofuer **der Desktop selbst**
laufen muesste.

**Die Lesart der abgeschnittenen Logdatei war damit falsch — zum zweiten Mal
an derselben Datei.** Die Abschneidung ist **keine Ortsangabe**, sondern die
Folge einer nie geschlossenen Datei: der letzte Puffer blieb stehen. Die Demo
kam also **ueber** `write_report` **und ueber** `form_dial` hinaus.

**Es bleiben zwei Aufrufe:** `v_clsvwk` und `appl_exit`.

### Nachbarschaft, ausdruecklich keine Diagnose

`appl_exit` ist die Stelle, die [ACC10](ACC10.md) am 2026-09-26 repariert hat
(`wait_for_accs` schlief auf `WAIT_INPUT` statt weiterzureichen). Dessen
Abnahme lief **headless und ohne GEM-Anwendung davor**, und auf der Karte des
Anwenders sind **zwei Accessories geladen**. Ob das hier beteiligt ist, ist
**ungemessen**.

### Beide Laeufe brechen am **selben Byte** ab — es ist kein Zeitproblem

Die Logdatei des zweiten Laufs liegt als `raw/pi02-BALL.LOG`. Sie ist **neu
geschrieben** (fuenf Bytes Unterschied, alle in Messwerten) und endet
**ebenfalls bei 1755 Byte, mitten im selben Wort**.

```
sphere map  25,00 ms (5 Ticks)          ->  20,00 ms (4 Ticks)
blit        73,41 ns/px + 1,28 us/call  ->  73,41 ns/px + 1,41 us/call
```

**Die Vermutung des Anwenders (Langsamkeit beim Schreiben auf die Karte) ist
damit widerlegt** — mehr Wartezeit aendert nichts, es bricht deterministisch an
derselben Stelle ab. Das ist ein **Pfad**problem.

**Nebenbei belastbar geworden:** der Pixelpreis ist in beiden Laeufen
**73,41 ns** auf die zweite Nachkommastelle. Fuer [PORT06](PORT06.md) heisst
das 4,70 ms je 320x200-Bild, reproduziert.

**Kein Dateisystemfehler:** beide Laeufe tragen denselben Zeitstempel, weil die
Pi keine Echtzeituhr hat ([H01](H01.md) wartet auf Hardware) — die Uhr startet
bei jedem Boot beim selben Wert.

## Auftrag

1. **Headless nachstellen.** Die Demo laeuft in QEMU `raspi4b`
   (`measurements/measurement-2026-09-28-port07/method/`); dort fehlte bisher nur die
   Tastatur zum Beenden — der IKBD-Injektor liefert sie.
2. **Den letzten Aufruf feststellen**, mit dem Instrument aus
   [DESK43](DESK43.md) U1: der AES-Tracer sieht seit `c141be2` jeden `svc`,
   auch aus EL1.
3. Erst dann die Ursache suchen — und **sagen, ob sie in der Demo oder bei uns
   liegt**, bevor etwas geaendert wird.

## Abnahme

Die Demo beendet sich, der Desktop kommt vollstaendig neu gezeichnet zurueck,
die Palette ist wiederhergestellt, und `C:\BALL.LOG` endet mit einer
**vollstaendigen** letzten Zeile.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-29): zwei Fehler, und sie gehoeren verschiedenen Leuten

**Der Haenger gehoert uns** → **[SCHED01](SCHED01.md)**, P1. Gemessen: der
letzte Aufruf ist `appl_exit` (AES 19), und er kehrt nicht zurueck, weil ein
gestartetes GEM-Programm **nie in der kooperativen Laufmenge steht** —
`sched::admit` wird nur fuer Accessories gerufen, und `sched::dsptch` legt den
Aufrufer im Gegensatz zu EmuTOS' `disp_act` nicht ein. `appl_exit` verliert die
Anwendung damit an das erste Accessory. Betrifft **jede** EL0-GEM-Anwendung,
die `appl_exit` ruft, sobald ein Accessory geladen ist.

**Die abgeschnittene Logzeile gehoert der Demo** und kostet den Kern nichts:
`plat-newtos.c:543` hat `static char rbuf[200]`, der Text ist bei 1920x1080
201 Zeichen. Der Trace zeigt `Fwrite len=199 -> 199` und `Fclose -> 0` — der
Kern hat geschrieben und geschlossen, was er bekam. Bei 640x480 faellt nur das
LF weg; **die Zeile ist in jeder Aufloesung unvollstaendig**.

**Damit war meine Lesart dieser Datei zum dritten Mal falsch.** Erst „haengt im
Schreiben", dann „nie geschlossene Datei", und richtig ist: ein zu kleiner
Puffer in der Demo. Die Datei hat dreimal etwas anderes zu sagen geschienen,
als sie sagte — sie war nie eine Ortsangabe.
