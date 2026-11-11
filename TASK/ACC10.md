# ACC10 — `appl_exit` blockiert headless fuer immer in `wait_for_accs`

Status: DONE (2026-11-11) | Priority: P2 | Commit: d45eff86c | Receipt: ACC10-STATUS-DONE.MD

Migrated (owner batch 18, 2026-11-11, to DONE), evidence: receipt TASK/ACC10-STATUS-DONE.MD exists, commit d45eff86c (fix(sched): suspend the parent across Pexec, so the child cannot resta), VERIFIED: d45eff86c (fix(sched): suspend the parent across Pexec, so the child ca) names the id by body, its message names the ticket-title identifier appl_exit, wait_for_accs, touches code (33 files), legacy line: BEHOBEN (2026-09-26)

Line 3 before the migration:
Status: **BEHOBEN** (2026-09-26) · Prioritaet: **P2** · Typ: Belegter Defekt — aus PORT05 herausgefallen

Abhaengigkeiten: —

## Der Befund, gemessen

`appl_exit()` blockiert in `wait_for_accs(AP_ACCLOSE)`, und das schlaeft auf
der Weckklasse **`WAIT_INPUT`** (`tos-api/src/aes/acc.rs:164`). Die
Accessory-Schranke treibt diese Klasse nicht; `idle_wake_pass` weckt sie nur
bei **Taste oder Zeigerbewegung**. **Headless kommt das Programm nie
zurueck.**

**Unabhaengig von PORT05 gemessen:** ein vom Kernel gestartetes Programm
(kein `Pexec`) bleibt an derselben Stelle stehen, und
`measurements/measurement-2026-09-25-port05/serial/control-kernel-launch-unfixed.log`
gegen `...-fixed.log` sind **bis auf die Uhrzeile identisch**.

## Warum es bisher niemand gesehen hat

Interaktiv faellt es nicht auf: eine bewegte Maus weckt die Klasse. Und im
`Pexec`-Fall war der Stillstand vom PORT05-Defekt **verdeckt** — der Elter
wurde ohnehin neu gestartet, bevor jemand auf das Ende des Kindes wartete.

## Warum es zaehlt

Jeder headless-Lauf, der ein Programm bis zu seinem Ende fahren will —
Messungen, Selbsttests, `#Z`-Autorun-Ketten — bleibt hier haengen. Fuer die
Vorfuehrung selbst ist es harmlos, fuer unsere Messwerkzeuge nicht.

## Auftrag

Gegen EmuTOS halten: auf welcher Klasse wartet `wait_for_accs` dort, und wer
weckt sie (`aes/geminit.c:599-630`, `WAIT_TIMEOUT 500`). Der
**Zeitgrenzen**-Anteil ist der verdaechtige Unterschied — EmuTOS wartet
begrenzt, nicht unbegrenzt. Erst lesen, dann entscheiden.

## Abnahme

Ein headless-Lauf, in dem ein Programm `appl_exit` durchlaeuft und
terminiert, ohne dass jemand die Maus bewegt.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-26) — die Zeitgrenze war es nicht allein

Der Verdacht im Auftrag (`WAIT_TIMEOUT 500`) stimmt zur Haelfte, und die
**erste** Haelfte ist die wichtigere: es geht darum, **welche
Scheduler-Primitive** die Schranke benutzt.

EmuTOS' `wait_for_accs` (`aes/geminit.c:599-628`) verbringt jeden Durchgang in
`all_run()` (`:612`) = `dsptch()` je Accessory (`:559-562`). `dsptch`
(`gemasm.S:129-167`) → `disp` (`gemdisp.c:220-254`) verzweigt nach dem Zustand
**des Aufrufers**:

```c
if (p->p_stat & WAITIN)  mwait_act(p);   /* gemdisp.c:230 — auf nrl parken */
else                     disp_act(p);    /* :231 */
```

und `disp_act` ist `insert_process(p, &rlr)` (`:74-79`) — direkt zurueck in die
Bereitliste. `ap_exit` (`gemaplib.c:251-255`) hat kein `ev_block` dazwischen,
steht also im `else`-Zweig: **EmuTOS' Schranke ist ein begrenztes
Rundlauf-Weiterreichen, kein Warten auf ein Ereignis.** Sie braucht nichts als
den Dispatcher.

Unsere Fassung stand im anderen Zweig (`sched::sleep(wait_input())` =
`mwait_act`): der Aufrufer verliess die Laufmenge, und nur `idle_wake_pass`
(Taste/Zeiger) konnte ihn zurueckholen — headless also nie.

Die Zeitgrenze ist die zweite Haelfte, und sie brauchte eine eigene Ueberlegung:
EmuTOS' Zaehler `n` darf eine C-Automatik sein, weil seine Schleife die Funktion
nie verlaesst. Ein NewTOS-Durchgang verlaesst sie beim Weiterreichen und tritt
oben neu ein (S2-Restart) — ein Stapelzaehler wuerde bei jedem Weiterreichen
zurueckgesetzt, die Vorsichtsmassnahme gegen Endlosschleifen waere selbst eine.
Der Zaehler liegt jetzt ausserhalb des Rahmens, je AES-Identitaet.

**Spec zuerst** (Regel 5): `docs/api/aes/accessories.md` beschrieb alles ausser
genau diesem Punkt. §3a („The barrier is a *yield* loop, not a blocking wait")
und §9.5 nachgetragen, dazu `application.md` `appl_exit (19)` Schritt 3.
Zitate nachgezaehlt — die im Ticket genannten `:588-630` sind real `:599-628`.

**Neu:** `sched::dsptch()` als zweite Block-Primitive neben `sleep` — der
einzige Unterschied ist, dass sie `runnable`/`wait_cond` des Aufrufers **nicht**
anfasst. `aes::acc::all_run()`, `WAIT_PASSES`/`spend_pass`/`forget_wait`.
Reihenfolge wie EmuTOS: `all_run()` **vor** dem Flaggenlauf.

**Abnahme, headless, ohne Maus, ohne Taste, ohne Injektor.** Die
PORT05-Kontrolle (`#Z 01 C:\PEXKID.NAP`, kernelgestartet, kein `Pexec`): der
`diff` ist exakt sieben angehaengte Zeilen, vorher endete das Protokoll nach 65
Zeilen mit 65 Sekunden Stille.

```
> [KID] 5 appl_exit done, Pterm(0)
> [shell] sh_ldapp: 'C:\PEXKID.NAP' exited with rc=0
> [desk] desktop up (1 drives, 0 window(s) from the context)
```

Zweites Szenario: die **volle** PORT05-Vorrichtung (EL0-Elter `Pexec`t
EL0-Kind, **beide** mit echtem `appl_exit`) laeuft bis zu beiden `Pterm`s und
in den Desktop — genau das, was PORT05 noch mit `-DNO_APPL_EXIT` umgehen musste.

Basislinien: 956/0/1 (955 + 1), 64/94 beide Boards, 80/125 beide Feature-Saetze.

**Nicht nachgebildet, benannt:** die `wm_update(BEG/END)`-Klammer am Ende von
`all_run` (`:564-565`) — NewTOS' Sperre parkt niemanden, nehmen und sofort
abgeben ist beweisbar wirkungslos.

**Damit ist die Vorbedingung fuer [DESK43](DESK43.md) U2/U3 erfuellt.**
