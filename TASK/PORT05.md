# PORT05 — QED aus TeraDesk gestartet: die Pi friert ein, oder QED beendet sich sofort

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt PORT05-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by d45eff86c (fix(sched): suspend the parent across Pexec, so th), named by body

Line 3 before the migration:
Status: **BEHOBEN — auf echter Pi 4 abgenommen** (2026-09-26) · Prioritaet: **P1** · Typ: Belegter Defekt — Anwender 2026-09-25

Abhaengigkeiten: —

## Der Befund

Echte Pi 4, Release-Kern `1ab0a413`, Karte C:–F:. **QED aus TeraDesk heraus
gestartet**: entweder **friert die Pi ein**, oder **QED beendet sich sofort**.

Wortlaut des Anwenders: *„Wenn ich aus TeraDesk heraus QED starte, freezed
der Pi, oder QED beendet sich sofort."*

Das ist alles, was gesichert ist. Insbesondere ist **nicht** gesichert, ob
die beiden Ausgaenge von der Art des Startens abhaengen oder derselbe
Vorgang mal so und mal so endet.

## Beide Ausgaenge, im Wortlaut (Anwender 2026-09-25)

Auf die Frage „immer gleich oder mal so mal so":

1. *„Einmal startet QED, und nachdem QED gestartet ist, friert das System
   komplett ein, auch die Maus bewegt sich nicht mehr."*
2. *„Zweitens: QED starten, es beendet sich direkt wieder, man ist im
   TeraDesk — und dann kommt der Freeze."*

**Zwei Dinge folgen daraus, ohne zu raten:**

**Die Maus ist tot.** Nach der Projektregel
[[mouse-moves-but-nothing-responds-means-app-sleeping]] heisst *lebendige*
Maus bei totem Bild, dass eine Anwendung in einem AES-Aufruf schlaeft — denn
`pump_input` laeuft in der AES-Warteschleife. **Tote** Maus heisst also das
Gegenteil: es steckt **unterhalb** der AES. Interrupts weg,
Ausnahmeschleife oder Verklemmung im Kernel. Eine ganze Ebene ist damit
ausgeschlossen, durch die Beobachtung und nicht durch eine Vermutung.

**Der Freeze kommt in beiden Faellen *danach*.** Einmal nachdem QED laeuft,
einmal nachdem QED sich beendet hat und TeraDesk wieder da ist. Der
gemeinsame Nenner ist damit **nicht** der Start, sondern was danach passiert.
In Fall 2 ist der `Pexec` nachweislich zurueckgekehrt — TeraDesk zeichnet
sich wieder — und erst dann faellt das System.

Dass beide Faelle bei derselben Handlung auftreten koennen, ist selbst ein
Befund: **es ist nicht deterministisch.**

## Was sich seit dem letzten Lauf geaendert hat

Genau eines, und es gehoert hierhergeschrieben, damit es nicht uebersehen
wird: **der Lead hat `TERADESK.INF` auf der Karte veraendert** (2026-09-25,
~20:58). Ergaenzt wurden zwei `ptype`-Eintraege (`*.NTP` mit `appt=4`,
`*.NAC` mit `flag=0x2001`) und vier `ftype`-Masken (`*.NAP`, `*.NCP`,
`*.NTP`, `*.NAC`). Die Vorlage liegt unveraendert unter
`measurements/hw-round-2026-09-24/TERADESK.INF`.

Das ist **kein Verdacht**, sondern die eine bekannte Aenderung. Ob sie
beteiligt ist, entscheidet ein Rueckbau, nicht eine Ueberlegung.

## Gemessen 2026-09-25: der Desktop startet QED einwandfrei

Anwender, nach der ersten Trennung: *„Aus NewDesk funktioniert QED
einwandfrei."*

**Damit ist QED entlastet und der Startweg belastet.** Der Unterschied
zwischen den beiden Wegen ist nicht die Anwendung, sondern **wer sie
startet**:

* Unser Desktop (`kernel/src/desk/`) laeuft **im Kernel-Bootpfad** und startet
  ueber `kernel/src/launch.rs` / `shell_loop.rs` — kernelseitig.
* TeraDesk ist eine **EL0-Anwendung**, die ihrerseits eine EL0-Anwendung
  startet.

Ein von einer Anwendung ausgeloester Start hat also bei uns noch nie
funktioniert bewiesen bekommen — und ist ein anderer Pfad als der, ueber den
jede bisherige QED-Abnahme lief.

**`TERADESK.INF` ist dadurch nicht entlastet.** Die Datei ist ausschliesslich
TeraDesks Konfiguration; unser Desktop liest sie nicht. Der Rueckbau bleibt
der billigste Trenner und steht weiterhin aus.

**Offene Ticketnachbarschaft, als Verweis und nicht als Diagnose:**
[ARCH02](ARCH02.md) steht offen und betrifft genau diesen Pfad
(FP/SIMD-Register auf dem `Pexec`-Eltern/Kind-Weg). ACC03 hat denselben
Mangel fuer den Accessory-Kontextwechsel gefunden und behoben; fuer `Pexec`
ist er **nicht** behoben. Ob das hier beteiligt ist, ist ungemessen.

## Erste Trennung (billig, ohne neuen Kern)

1. **QED vom NewTOS-Desktop starten** statt aus TeraDesk. Geht es dort, liegt
   es am Startweg von TeraDesk; friert es auch dort ein, ist TeraDesk nur der
   Bote.
2. **Die unveraenderte `TERADESK.INF` zurueckspielen** und denselben Start
   wiederholen.

## Was vorher schon bekannt war

* TeraDesk startet selbst sauber (PORT03-Abnahme, `screendumps/tera-b-up.png`).
* QED startet und laeuft (PORT03: oeffnet eine gezogene Datei mit vollem
  Titel `C:\README.TXT`).
* Ein Start von TeraDesk **aus** und der Weg `shel_write` sind bisher
  **nirgends** abgenommen worden — [Start TeraDesk via the real
  mechanism](../docs/MEMORY.md) beschreibt den Weg, nicht seine Pruefung.

## Warum P1

Der Oktober-Termin verlangt: nichts darf haengen. Ein einfrierendes System
beim Start einer Anwendung ist der sichtbarste denkbare Fehler, und eine
Anwendung zu starten ist das Erste, was jemand ausprobiert.

## Auftrag

Erst die beiden Trennungen oben, dann messen, **wo** der Start stehenbleibt —
nicht raten. Der Weg ist `shel_write`/`Pexec`; Zeitstempel und der letzte
erreichte Punkt vor dem Stillstand sind die Groessen, die ihn eingrenzen.
Ein `[EXC]`/`[PANIC]` auf der seriellen Schnittstelle waere der Glueckfall —
der Anwender hat dort aber keine Aufzeichnung, also muss der Kernel selbst
sagen, wo er war.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Ursache und Fix (2026-09-25)

**Ein EL0-Elter, der `Pexec` ruft, blieb in der kooperativen Laufmenge stehen
und wurde vom blockierenden Kind an seinem Einsprungpunkt neu gestartet.**

In QEMU `raspi4b` deterministisch reproduziert, **ohne** Injektor, ohne Klick,
in 16 Sekunden — es braucht nur eine EL0-Anwendung, die `Pexec` ruft und
weiterlebt, plus ein Kind, das in einem AES-Warteruf blockiert
(`method/pexpar.c`, `method/pexkid.c`, 80 Zeilen, gebaut mit derselben
Werkzeugkette wie QED und TeraDesk).

```
[KID] 3 entering evnt_timer(1500)   <- das Kind blockiert
[sched] switch -> slot 2            <- Slot 2 IST DER ELTER
[PAR] 1 main entry                  <- der Elter wird NEU GESTARTET
[exec] preparing program (slot 4)   <- und startet das Kind noch einmal
```

Mit dem **echten QED**: **14 Neustarts**, QED in die Slots 3…15, bis die
Prozesstabelle voll war; der 14. `Pexec` antwortete `-39 ENSMEM`.

**Drei am Quelltext belegte Schritte:** (1) jedes EL0-Programm, das einmal in
`evnt_*` blockiert hat, ist danach `runnable` — `sleep`s Leerlauf-Ausgang
setzt das Bit auf jedem Durchgang neu; (2) `handle_pexec` fasste die
Laufmengen-Mitgliedschaft des Elters nicht an, und sein `has_ctx` ist `false`,
er ist also ein **`fresh`**-Ziel; (3) der erste blockierende AES-Aufruf des
Kindes → `sleep` → `on_yield` → `pick_next` → der Elter → `apply_switch(fresh)`.

**Gebrochener Kontrakt, keine Geschmacksfrage:** `Pexec` ist synchron
(EmuTOS `proc_go`: `run = p; gouser();`, `bdos/proc.c:536-577`; zurueck erst
ueber `xterm`, `:601-618`), und EmuTOS' Rundlauf kann den Elter gar nicht
sehen, weil startende und gestartete Anwendung **eine** AESPD teilen.

**Es war nicht [ARCH02](ARCH02.md).** Kein FP/SIMD-Register geht verloren;
ARCH02 bleibt unberuehrt offen.

**Fix**, 123 Zeilen, nur additiv: `Process::parent_runnable` am **Kind**-Slot
(wie `parent_frame`/`parent_l0_pa`), `suspend_parent_for_child()` /
`resume_parent_of_child()` im Scheduler, aufgerufen in `handle_pexec` und —
**vor** `release_slot` — in `finish_terminate`. Zurueckgegeben wird das
gespeicherte **Bit**, kein erzwungenes `true`; `wait_cond` wird beim Aussetzen
geloescht, sonst macht `wake()` den ausgesetzten Elter wieder scharf.

**Nachher:** genau **ein** `[exec] preparing program`, genau **ein**
`[PAR] 1 main entry`, `Pexec` liefert `rc=0`, QED steht mit voller Menueleiste
(`screendumps/after-qed-up.png`). Echtes A/B — die Vorher-Laeufe liefen mit
einem Kern, in dem die zwei neuen Aufrufstellen von Hand stillgelegt waren.

## Was ausdruecklich NICHT belegt ist

* **Der harte Einfrierer der Pi selbst.** Reproduziert ist die **Ursache**
  (unbegrenzte Neustartschleife bis ENSMEM), nicht der Stillstand mit toter
  Maus; in QEMU gab es kein `[EXC]`. Dass ein 14-mal neu gestartetes TeraDesk
  in `unhandled()` → `halt()` endet, ist **plausibel und unbewiesen**.
* **TeraDesk selbst** wurde nicht gefahren — reproduziert ist seine
  *Handlung* (chdir + `Pexec` + am Leben bleiben).
* Der **`TERADESK.INF`-Rueckbau** ist nicht gemacht. Durch den Befund aber
  unwahrscheinlich geworden: der Defekt tritt ohne TeraDesk und ohne dessen
  `.INF` auf.

## Die Zahl, die es auf der Pi zeigt

QED aus TeraDesk starten: **QED darf genau einen Prozessslot belegen** und
kommt genau einmal mit eigener Menueleiste hoch; das System bleibt bedienbar.
Vorher waren es in QEMU 13 Slots plus ein `Pexec` mit `-39`.

## Herausgefallen

* **[ACC10](ACC10.md)** — `appl_exit()` blockiert headless fuer immer in
  `wait_for_accs(AP_ACCLOSE)`. Unabhaengig von PORT05 gemessen.
* Notiert, nicht angefasst: `handle_pexec` aktualisiert `sched::set_running`/
  `CURRENT` nicht, also schreiben `arm_or_get_deadline`/`clear_deadline` die
  Frist des **Kindes** in den Slot des **Elters** (latent); `sched::on_exit`
  hat keinen Aufrufer.

## Hardware-Abnahme (2026-09-26)

Kern `346c385c` (Release, rpi4, enthaelt ausserdem den FS04-EMMC-Fix) auf der
Karte. Anwender: *„super, das Freeze-Problem ist behoben, alles funktioniert
sauber."*

**Damit schliesst sich die Luecke, die im QEMU-Befund ausdruecklich offen
blieb.** Dort war nur die *Ursache* reproduziert — die unbegrenzte
Neustartschleife bis `ENSMEM` —, nicht der Stillstand mit toter Maus; dass
ein vielfach neu gestartetes TeraDesk in `unhandled()` → `halt()` endet, war
als **plausibel und unbewiesen** vermerkt. Der Lauf auf der Pi bestaetigt es:
mit behobener Schleife ist der Einfrierer weg. Es war **ein** Fehler, nicht
zwei.

Der Kopiertest (FS04-Gegenprobe) steht noch aus, der Anwender holt ihn nach.
