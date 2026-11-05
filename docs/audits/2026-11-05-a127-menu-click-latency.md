# A127 D1 — Menüklick-Latenz gegen das Orakel (2026-11-05)

## 1. Kopf

| | |
|---|---|
| Scope | Zeit vom Drücken der linken Taste auf einem Drop-down-Menüeintrag bis zur Rückkehr von `evnt_multi` mit `MN_SELECTED`, dazu das Ende der Menüverfolgung; EmuTOS gegen NewTOS head und base |
| Auftrag | TASK/A127 D1, Messfrage des Leads 2026-11-04 |
| Datum | 2026-11-04/05 |
| NewTOS | Messbaum `61d7cb03`, sauber bis auf die neuen, nicht eingecheckten Probe-Dateien; Kerne aus `git archive bae52dbd` (head) und `git archive 4d44a616` (base) |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df`, ROM `tools/oracle/etos512de-1a7484ab.img`, sha256 `323c87bb…2d2750c` |
| Hatari | 2.6.1, `tools/oracle/hatari.cfg`; Profil `tt` (Hauptmessung), `falcontc` (Gegenprobe) |
| QEMU | 11.0.2, `raspi4b`, `-append serial=on` |
| Kanarienvogel | `docs/analysis/ORACLE.md` §6: `canary tt: OK`, `canary falcontc: OK` |
| Lauf | `measurements/measurement-2026-11-04-a127-oracle/RUNINFO.TXT` (Methode, Hashes, alle Zahlen) |

**Urteil: CONFORM.** head stimmt mit EmuTOS innerhalb der Messauflösung überein. Die Auflösung ist ein AES-Tick (20 ms = 4 `_hz_200`-Ticks). base weicht ab.

## 2. Ergebnis

Die Werte sind `_hz_200`-Ticks zu je 5 ms. Beim Klick gilt Rückkehr minus Drücken, beim Ziehen Rückkehr minus Loslassen.

| Fall | EmuTOS (Orakel) | NewTOS head | NewTOS base |
|---|---|---|---|
| Klick, Desktop-Warte D2 (`MU_KEYBD\|MU_BUTTON\|MU_MESAG`, 2,1,1), Index 2 (275 ms) | 48 (Einspeisung im Probe), 49–52 (echter IKBD-Weg) | 48–52 | 0 |
| ebenso, Index 3 (220 ms) | 40 (Probe), 41–44 (IKBD) | 40–44 | 0 |
| Klick, nur `MU_MESAG`, **nach** einer D2-Warte | wie D2 | wie D2 | 0 |
| Klick, nur `MU_MESAG`, **vor** jeder Mehrfachklick-Warte | 0 | 0 | 0 |
| Klick, D1 (`bclicks` 1) nach D2 | wie D2 | wie D2 | 0 |
| Ziehen (Titel drücken, auf Eintrag loslassen) | 0 nach dem Loslassen | 0 | 0 |
| Ende der Verfolgung | `_ap_sendmsg` im selben Tick wie die Rückkehr | `track end` 0–1 Tick vor der Rückkehr | 0 nach dem Drücken |
| zusätzliche `MU_BUTTON`-Rückkehren je Lauf (Standardreihenfolge) | 0 | 0 | 175–179 |

Zur Auflösung: EmuTOS zählt das Doppelklickfenster in AES-Ticks herunter (`gl_dclick = rate/gl_ticktime` = 13 bzw. 11, `references/emutos/aes/gemevlib.c:33,300-308`; `b_delay`, `references/emutos/aes/geminput.c:150-171`). Die Zeit vom Drücken bis zur Freigabe liegt deshalb im Band 48–52 bzw. 40–44, je nach Phase des Drückens im Tick. Die Einspeisung im Probe trifft immer die Untergrenze. Der echte IKBD-Weg streut über das Band. Jeder head-Wert liegt im Band des Orakels.

Woher die Verzögerung kommt: Sie hängt an `gl_bpend` und nicht an der Warte, in der das Programm gerade steht. Eine Warte mit `bclicks > 1`, die durch eine Nachricht endet, lässt ihr Inkrement stehen (`references/emutos/aes/geminput.c:477-479,559-561`). Danach öffnet jedes Drücken das Fenster. EmuTOS und head zeigen das gleich: In der Reihenfolge M-zuerst gibt `MU_MESAG` den Wert 0, nach der ersten D2-Warte das volle Fenster.

Der Befund des Testers (head 40–42 Ticks mit Desktop und QED; NEWDESK.INF ohne `#E`, also Index 3) liegt im Orakelband 40–44 und ist **kein Defekt**. base wählt beim Drücken sofort aus und weicht damit überall dort ab, wo EmuTOS das Fenster öffnet. Als Folge findet die nächste D2-Warte des Programms die Taste noch gedrückt und kehrt bis zum Loslassen wiederholt mit `MU_BUTTON` zurück. EmuTOS verhält sich genauso, wenn die Auswahl wirklich beim Drücken kommt: 16 Rückkehren im M-zuerst-Lauf.

## 3. Findings

**A127-ORC-001 — S4, MEASURED, Route Researcher.** `docs/api/aes/menu.md` Punkt 9 („Button-delay window“) schreibt: „while any process waits for more than one click (`gl_bpend`)“. Gemessen öffnet das Fenster auf beiden Seiten auch dann, wenn keine Mehrfachklick-Warte mehr offen ist, sondern nur eine frühere ohne Tastendruck endete (Abschnitt 3.3 im RUNINFO: `MU_MESAG` nach D2 → 48/51; vor D2 → 0). `docs/api/aes/event.md` („The double-click window“) beschreibt das richtig: „leaves `gl_bpend > 0` for the rest of the session“. menu.md sollte darauf verweisen statt „waits“ zu sagen. Code nicht betroffen, head verhält sich wie EmuTOS. Lokalisierung: Spec-Text, kein Code. Retest: keiner nötig, Textänderung.

Gegen head gibt es keinen Befund.

## 4. Offene Tickets im Scope

TASK/A127 D1 (Commit `bae52dbd`): Die Verzögerung nach dem Klick ist **EQUAL** zu EmuTOS (Probe-Messung, siehe oben). D2/D3/D4/D7 und die Ursache auf Ragnars Pi gehören nicht zu diesem Scope und wurden nicht gemessen.

## 5. Coverage

| EmuTOS | NewTOS | Status | Beleg |
|---|---|---|---|
| `mn_do` Klick-Auswahl (INITEM, `buparm` Drücken) | `tos-api/src/aes/menu.rs` `track_owned` | EQUAL (Probe, innerhalb 1 AES-Tick) | Abschnitte 3.1–3.3 im RUNINFO |
| `mn_do` Ziehen-Auswahl (INTITLE → INITEM, Loslassen) | dto. | EQUAL | 3.4 |
| `b_click`/`b_delay` Fenster während der Verfolgung (beobachtet über `mn_do`) | `tos-api/src/aes/input.rs` `mn_button_arm`, `post_button`, `b_delay_poll` | EQUAL | 3.1–3.3 |
| `ev_multi` Zustellung `MN_SELECTED` (`ct_msgup` → `ap_sendmsg`) | Verfolgerende → Rückkehr | EQUAL (0–1 Tick) | 3.5 |

Gemessen wurde mit einem Probe, nicht mit tosdiff. Für Menüauswahl mit zeitgesteuerter Maus gibt es keine Fallzeile. Die Eingabeparameter von `evnt_multi` wurden auf keiner Seite per Instrument geprüft (Begründung in 3.6 des RUNINFO). Bug-Dichte: 0 bestätigte Bugs auf 4 gemessene Funktionen.

## 6. Neue Fälle / Werkzeuge (nicht eingecheckt)

* `tosdump/probes/mnprobe.c`: Probe, gleiche Quelle für beide Seiten. Auf EmuTOS speist ein eigener `etv_timer`-Hook die Maus ein; ein passiver `mousevec`-Beobachter zeichnet auf.
* `tosdump/probes/run-mnprobe.sh`: Läufer mit den Modi `emutos`, `kernel <commit>`, `newtos`. Optionen: `MNPROBE_E1` (INF `#E`), `MNPROBE_MFIRST`, `MNPROBE_INJ`/`MNPROBE_FIFO` für die Gegenprobe über den IKBD-Weg. Ein Makefile-Ziel gibt es nicht; der Läufer baut selbst.
* `tosdump/probes/mnprobe-mouse.txt`: das Maus-Skript beider Seiten.

## 7. Gemessen vs. offen

* Gemessen: alles unter 2. Wiederholungen: Orakel tt je Index 2× plus Breakpoint-Lauf plus v2; head Index 2 4×.
* Hatari 2.6.1 kann weder linke Taste noch Bewegung einspeisen (`control.c` der Version 2.6.1). Ersatz: die Einspeisung im Probe, gegengeprüft über die Alt-Pfeil/Alt-Einfg-Mausemulation von EmuTOS per `--cmd-fifo` (`references/emutos/bios/ikbd.c:285-389`). Beide Wege stimmen überein, bis auf die Phase.
* Offen: `INF` ohne `#E` auf EmuTOS (Hatari schreibt beim `--auto` immer ein `#E`). Das AES nimmt dann `ev_dclick(3)` (`references/emutos/aes/geminit.c:708`), wie NewTOS. PLAUSIBLE, nicht gemessen.

## 8. Real-Pi-Liste

* Ein Menüklick auf echter Pi mit USB-Maus: Nach jeder abgebrochenen Doppelklick-Warte erscheint die Auswahl 200–260 ms nach dem Drücken. Das ist EmuTOS-gleich. Es ist eine sichtbare Trägheit, aber keine verlorene Auswahl. Bei Ragnars Rückmeldung danach unterscheiden.

## 9. Akzeptable Abweichungen

Keine Abweichung zwischen head und EmuTOS. Die Burst-Anzahl der `MU_BUTTON`-Rückkehren beim Halten (EmuTOS 16, head 35) hängt an der CPU-Geschwindigkeit, nicht an der API: Gezählt wird, wie viele `evnt_multi`-Runden in 50 ms Haltezeit passen.

## 10. Out-of-scope observations

* `docs/analysis/ORACLE.md` nennt keine Methode für Mauseingaben mit linker Taste auf Hatari 2.6.1. Die beiden Wege dieses Laufs wären eine Ergänzung (Researcher).
* Das `aes-trace`-Feature schreibt nur nach einer Warte von mindestens 3 s. Für Probes mit kurzen Takten taugt es nicht als Eingabeprüfung.

## 11. Cleanup

Kein Produktcode angefasst, kein `AUDIT-TEMP`. Den dbg-Patch des Testers habe ich nur auf `git archive`-Kopien im Scratch angewandt. Scratch (das tmp-Verzeichnis im Session-Scratchpad: Kerne, Karten, Läufe) ist gelöscht, kein QEMU/Hatari läuft mehr. Fremde Dateien im Scratchpad blieben unberührt. Neue, nicht eingecheckte Pfade: der Laufordner, dieser Bericht und die drei Probe-Dateien unter `tosdump/probes/`.
