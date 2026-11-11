# IDEA01 — Energieverwaltung als Accessory (Zukunft, kein Auftrag)

Status: PARKED (2026-11-11) | Priority: P4 | Reason: idea (owner 2026-09-25): expressly not now | Reopen: the owner takes up the idea

Migrated (owner batch 10, 2026-11-11), evidence: TASK/IDEA01.md status line: IDEE (Anwender 2026-09-25, ausdruecklich „nicht jetzt")

Line 3 before the migration:
Status: **IDEE** (Anwender 2026-09-25, ausdruecklich „nicht jetzt") · Prioritaet: **—** · Typ: Richtungsnotiz

Abhaengigkeiten: [ACC06](ACC06.md) (`Ssystem`-Grenze, erledigt),
[ACC02](ACC02.md)-[ACC04](ACC04.md) (Accessories laufen, erledigt),
[HW02](HW02.md), [ACC09](ACC09.md)

## Die Idee

Ein Accessory laeuft neben dem Desktop — koennte es perspektivisch eine
Energieverwaltung fuehren?

## Warum der Zuschnitt passt

**Politik ins Accessory, Mechanismus in den Kernel.** Die Mailbox ist von EL0
nicht erreichbar; die Grenze dafuer steht seit ACC06 als `Ssystem` mit
NewTOS-eigenen Unterfunktionen ab 2000 (heute lesend: Takt 2000, Temperatur
2001). Eine Verwaltung braucht die **schreibende** Richtung — dieselbe Grenze,
neue Unterfunktionen, kein neuer Mechanismus.

**Ein Accessory ist aber kein Daemon.** Es wird nur geweckt, wenn Nachricht
oder Eingabe kommt (gemessen in ACC03). Eine Regelschleife braucht
`evnt_timer` in seiner `evnt_multi`-Schleife.

## Was die Pi 4 hergibt — und was nicht

* Taktwechsel zur Laufzeit: Mailbox `SET_CLOCK_RATE` (`0x0003_8002`).
* Spannung: `SET_VOLTAGE` (`0x0003_8003`).
* Bildschirm abschalten.
* **Kein Suspend-to-RAM.** Die Pi 4 hat keinen echten Schlafzustand — mehr als
  „langsamer und dunkler" wird es nicht. Wer mehr erwartet, wird enttaeuscht.

(Tag-Nummern aus derselben Quelle wie ACC06s lesende Tags; vor Benutzung
gegen `references/circle/include/circle/bcmpropertytags.h` gegenpruefen, nicht
aus dieser Notiz uebernehmen.)

## Der Baustein, der zuerst fehlt

**Eine Leerlaufmessung.** `wfi` gibt es an zwei Stellen
(`kernel/src/host_env.rs:171,226`, beim Warten auf Konsoleneingabe), aber
niemand misst, **wie** untaetig das System ist. Ohne diese Zahl hat eine
Regelung nichts, woran sie entscheiden koennte. Das waere der erste Schritt —
und er ist unabhaengig von Accessories nuetzlich (Scheduler, Messungen,
Benchmarks).

## Einordnung

Klassisches TOS kennt keine Energieverwaltung; das ist eine **reine
NewTOS-Erweiterung**. Nach der festgehaltenen Reihenfolge gehoert so etwas
hinter Meilenstein 2b (sauberer EmuTOS-Stand auf echter Hardware) — also
**nicht** vor dem Oktober-Termin und nicht vor den offenen Konformitaetsluecken.

Verwandt: [ACC09](ACC09.md) stellt `arm_freq`/`over_voltage` statisch in der
`config.txt` ein; diese Notiz waere die dynamische Fassung desselben Gedankens.
[HW02](HW02.md) klaert, welchen Takt wir ueberhaupt melden — eine Regelung
haette ohne diese Antwort keinen verlaesslichen Messwert.

## Arbeitsregel

Kein Auftrag. Wird zur Aufgabe, wenn der Anwender es aufruft.
