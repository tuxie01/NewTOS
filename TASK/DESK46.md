# DESK46 — Konsolenprogramme nach U3: Bildschirm blitzt weiss, und aus EmuCON heraus friert es ein

Status: DONE (2026-11-11) | Priority: P1 | Commit: 1de06fb94 | Receipt: DESK46-STATUS-DONE.MD

Migrated (owner batch 6, 2026-11-11), evidence: receipt TASK/DESK46-STATUS-DONE.MD exists, commit 1de06fb94, TASK/DESK46-STATUS-DONE.MD, commit 1de06fb94 (docs(task): DESK46 - console programs freeze from EmuCON, and flash wh), fix/feat commits naming the id: - (n=0), receipt names the old pre-rewrite hash 4d00e83 (docs/commit-map.txt maps it to 1de06fb94)

Line 3 before the migration:
Status: **BEHOBEN — auf echter Pi 4 abgenommen** (2026-09-28) · Prioritaet: **P1** · Typ: Belegter Defekt — Pi-Runde 2026-09-26

Abhaengigkeiten: [DESK43](DESK43.md) U3/U4 (`e0069e5`, `5a485be`) — der Konsolenarm wurde dort umgebaut

## Was auf der Pi lief (damit der Rahmen stimmt)

Kern `14065b06`. **Funktioniert:** QED per Doppelklick starten und beenden;
TeraDesk laeuft; QED **aus TeraDesk heraus** starten und sauber beenden; die
offenen EmuDesk-Fenster werden nach der Rueckkehr **rekonstruiert**. Damit sind
U3s Hardware-Punkte 1 und 2 bestanden.

## Befund 1 — `.NCP` vom Desktop: kurz weiss, dann weg

Anwender: *„beim Starten der NCP-Apps, da wird das Bild kurz weiss und dann
weg."* Auf die Rueckfrage, ob im Weiss **Text** stand: *„zu schnell, kann ich
nicht sagen."*

**Beides ist moeglich und noch nicht getrennt:**
* Es ist die Folge der Entscheidung vom 2026-09-26 — `console_pause_for_key`
  ist entfallen (EmuTOS-konform), also erscheint die Ausgabe und der Desktop
  kommt sofort zurueck. Dann ist es **kein Defekt**.
* Oder der Textschirm wird betreten und sofort wieder verlassen, **ohne** dass
  das Programm zum Zeichnen kommt. Dann ist es einer.

**Zu messen, nicht zu raten:** ein Screendump im Moment des Konsolenschirms,
bevor der Desktop zurueckkehrt.

## Befund 2 — aus EmuCON heraus friert es ein

Anwender: *„dann bin ich in EmuCon gegangen, was auch funktioniert; dort konnte
ich `sysinfo.ncp` aufrufen, es hat die Ausgabe gebracht und dann war er
eingefroren."*

**Das ist ein Defekt.** Die Ausgabe kommt vollstaendig — das Programm laeuft
also —, und **danach** steht es.

Nicht erhoben, aber fuer die Eingrenzung noetig: ob die **Maus** dabei noch
lebt. Bei PORT05 war genau das der Trenner — tote Maus heisst unterhalb der
AES, lebende Maus heisst eine Anwendung schlaeft in einem AES-Aufruf
([[mouse-moves-but-nothing-responds-means-app-sleeping]]). In EmuCON ist der
Zeiger allerdings ohnehin abgeschaltet, die Frage ist also anders zu stellen:
reagiert die **Tastatur** noch?

## Ticketnachbarschaft, als Verweis und nicht als Diagnose

* U3 hat den Konsolenarm umgebaut: `launch_native`s Console-Arm,
  `sh_run_pending_console` und `sh_alpha_enter`/`sh_alpha_leave` sind
  **geloescht**; der Weg laeuft jetzt ueber `sh_main`s `CONSOLE_APP`-Arm
  (`gemshlib.c:614-622`).
* EmuCON startet ein Programm als **EL0-Anwendung, die eine EL0-Anwendung
  startet** — dieselbe Familie wie [PORT05](PORT05.md), nur auf dem
  Konsolenarm statt dem GEM-Arm. PORT05 ist behoben und auf Hardware
  abgenommen; ob hier etwas Verwandtes offen ist, ist **ungemessen**.
* `console_pause_for_key` ist entfallen (Anwenderentscheidung 2026-09-26). Das
  betrifft Befund 1, nicht Befund 2 — bei Befund 2 friert es ein, statt
  zurueckzukehren.

## Auftrag

1. **Befund 1 trennen**, headless: Screendump des Konsolenschirms waehrend
   eines `.NCP`-Starts vom Desktop. Steht dort die Ausgabe, ist es das
   gewaehlte Verhalten und das Ticket schrumpft auf Befund 2.
2. **Befund 2 nachstellen**, headless: EmuCON starten, darin ein `.NCP`
   ausfuehren. Tritt es in QEMU nicht auf, sagen was sich unterscheidet.
3. Erst wenn feststeht, **wo** es stehenbleibt, nach der Ursache suchen.

## Warum P1

„Nichts darf haengen" ist die Vorgabe fuer den Oktober-Termin, und ein
eingefrorenes System nach einem Konsolenprogramm ist genau das.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-27)

### Befund 1: **kein Defekt** — das Ticket schrumpfte auf Befund 2

Der Textschirm wird betreten **und** das Programm kommt zum Zeichnen.
Screendump im Augenblick, in dem `SYSINFO.NCP` fertig ist und bevor
`sh_chgrf` zurueckschaltet: alle sieben Ausgabezeilen stehen da
(`screendumps/a3-3-console.png`).

**Dauer gemessen statt geschaetzt**, 200-Hz-Marken an `sh_chgrf`:
`want=0` bei `t=4333`, `want=1` bei `t=4338` — **5 Ticks = 25 ms**, Programmlauf
und Ausgabe inbegriffen. Das ist die Folge der Anwenderentscheidung vom
2026-09-26 (`console_pause_for_key` entfaellt). **Nichts repariert.**

### Befund 2: ein Defekt, aelter als U3, und behoben

Headless im **Produktpfad** reproduziert (Ctrl-Z = die `#K`-Abkuerzung von
*File → Execute EmuCON*, gleiche Route wie der Klick): `run sysinfo.ncp` →
volle Ausgabe → die Eingabeaufforderung bricht nach `C:\` ab.

**Die Tastaturfrage des Tickets ist beantwortet: nein.** Der Kernel nimmt einen
Daten-Abort (`ESR=0x96000004`, `FAR=0`) und parkt in `unhandled` → `halt()`.
Als Ersatz fuer den Maus-Trenner diente ein Herzschlag aus **derselben
Schleife, die die Taste lesen wuerde** — er endet mit dem `[EXC]`.

**Ursache, an der Maschine belegt:** `finish_terminate` stellte die Register des
Eltern-Rahmens nur unter `if from_el0 || full` her, und `parent_frame_full`
setzt **nur** `adopt_kernel_park_as_parent` (Accessory, ACC03), `handle_pexec`
nie. EmuCON ruft den `Pexec`-Trap **aus EL1** — also traf keiner der Arme:
`elr`/`spsr`/`sp_el0` kamen zurueck, **kein einziges GPR**. EmuCON lief mit dem
Registersatz des Kindes weiter (x19…x29 = 0), und der erste Zugriff ueber ein
callee-saved Register las aus Adresse 0. Drei Laeufe, drei verschiedene
Register, dieselbe Ursache.

Zweite, stillere Folge: `frame.gpr[0] = rc` lief ebenfalls nur im
`from_el0`-Arm — ein EL1-Aufrufer bekam **nie** den Beendigungscode, den
`docs/api/gemdos/process/4b-pexec.md` verlangt.

**Kein U3-Rueckschritt und nicht PORT05.** Das `if from_el0`-Gate ist aelter als
`03394c2`; U3 und U4 fassen `exceptions.rs`, `process.rs` und `shell.rs` nicht
an. PORT05s `suspend_`/`resume_parent_of_child` laufen hier korrekt — der Fehler
sitzt eine Ebene tiefer, im Registerrestore.

**Fix:** neues Kennzeichen `Process::parent_frame_pexec` — gesetzt von
`handle_pexec` fuer **jeden** umgeleiteten Start, gleich von welcher
Ausnahmestufe; `adopt_kernel_park_as_parent` setzt es ausdruecklich auf `false`;
`finish_terminate` entscheidet an den zwei Kennzeichen statt an `SPSR.M`. Grund
fuer das eigene Kennzeichen: es gibt **drei** Arten von `parent_frame` und zwei
davon sind `EL1t` — genau daran hing der Defekt.

**Nachweis:** derselbe Lauf mit Fix geht durch bis `[program exited]`, Prompt,
`ver` antwortet, `exit` bringt den Desktop; **0** `EXC`/`PANIC`. Der Desktop-
und AUTO-Startweg bleibt unveraendert (`pexec_caller=0 full=0`).

Basislinien 956/0/1 (dreimal), 64/94 beide Boards, 80/125 beide Feature-Saetze,
Driftprobe 32.

**Pi-Gegenprobe:** EmuCON per Tastatur, `run c:\sysinfo.ncp` → **vollstaendige**
Eingabeaufforderung `C:\…>`, die auf Tasten antwortet; `exit` bringt den
Desktop neu gezeichnet zurueck.

## Hardware-Abnahme (2026-09-28)

Kern `92ec0ab7`. Anwender zu Punkt 1 der Testliste: *„funktioniert."* EmuCON
laeuft, `run c:\sysinfo.ncp` bringt die Ausgabe, der Prompt kommt
**vollstaendig** zurueck und antwortet auf Tasten. Damit ist der
EL1-`Pexec`-Registerrestore auf echter Hardware bestaetigt.
