# SDL01 — Ein SDL-Backend fuer NewTOS: ein Traeger statt fuenf Portierungen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/SDL01.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** (nach dem OFAM-Termin) · Typ: Portierung / Unterbau — Anwenderidee 2026-09-28

Abhaengigkeiten: [SND01](SND01.md) (fuer den Audio-Teil; Video/Eingabe/Timer gehen ohne)

## Die Idee, und warum sie besser ist als jede einzelne Portierung

Der Veranstalter und Freund des Anwenders hat nach Vorfuehrmaterial gefragt und
mehreres genannt: Doom, ein SDL-Port, CastAway, Atari800, der FireBee-m68k-Kern.
**Alle diese Programme kommen ueber denselben Weg** — sie brauchen
Framebuffer, Eingabe, Timer, Ton. Ein SDL-Backend baut diesen Weg **einmal**;
danach ist jede weitere Anwendung eine Uebersetzung statt einer Portierung.

## Was SDL braucht und was wir haben

| SDL-Teil | bei uns | Stand |
|---|---|---|
| Video | VDI-Framebuffer | **vorhanden** — alle ~70 Opcodes |
| Ereignisse | AES `evnt_*`, IKBD, USB-HID | **vorhanden** — inkl. Mausrad seit GUI04 |
| Timer | 200-Hz-Takt (`tos-api/src/bios/tick.rs`) | **vorhanden** |
| Audio | — | **fehlt**, [SND01](SND01.md) |
| Threads | — | kooperativ, kein Preemption-Modell |

## Der Punkt, der die Frage des Anwenders beantwortet

*„Die Frage ist, ob wir es ohne FreeMiNT zum Laufen bekommen."*

**Ja — und FreeMiNT ist gar nicht die Huerde.** SDL ist portabel; was
plattformabhaengig ist, ist sein **Backend**. FreeMiNTs SDL-Port bringt ein
Backend mit, das gegen MiNT-Syscalls geschrieben ist; das laeuft bei uns nicht,
weil es gegen die falsche Plattform gebaut ist, nicht weil FreeMiNT fehlt.

**Die Arbeit ist in beiden Faellen dieselbe: ein NewTOS-Backend schreiben.**
Ob wir dafuer von FreeMiNTs Fassung ausgehen (naeher an TOS-Denkweise) oder von
SDL 1.2 upstream (sauberer), ist eine Abwaegung, die in dieses Ticket gehoert
und nicht geraten wird.

## Zuerst zu entscheiden

* **SDL 1.2 oder SDL2.** 1.2 ist kleiner, hat ein einfacheres Backend-Modell
  und ist das, was im Atari-Umfeld existiert; SDL2 will mehr vom System
  (Threads, Render-API). Fuer einen kooperativen Einzelkern ist 1.2
  wahrscheinlich die ehrlichere Wahl — **zu pruefen, nicht anzunehmen**.
* **Woher der Quelltext kommt** und wie er gepinnt wird
  ([[port-reuse-dont-rewrite]] — retargeten, nie neu schreiben;
  [[port-target-app-unmodified]] — Korrekturen gehoeren in `sdk/`, nicht in den
  fremden Quelltext).

## Abnahme

Ein SDL-Programm laeuft: Fenster bzw. Vollbild, Tastatur und Maus kommen an,
der Timer stimmt. Ton erst mit SND01. Der Beleg ist ein **fremdes**, nicht
angepasstes SDL-Programm — ein selbstgeschriebenes beweist nur unser eigenes
Backend gegen sich selbst.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
