# SER01 — Serielle Ausgabe auf der Pi abschaltbar, Vorgabe aus

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt SER01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 91be77f9b (feat(console): serial output off by default on the), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-10-02, [Statusdatei](SER01-STATUS-DONE.MD); Pi-Abnahme offen) · Prioritaet: **P1** (vor dem Termin) · Typ: Kernel / Konsole — Anwenderentscheidung 2026-10-02

Abhaengigkeiten: —

## Sachverhalt

Jede Kernel-Meldung (`kprintln!`, `hal/uart.rs:48-58`) und jede
Konsolenausgabe eines Programms (`host_env.rs:58-68`) geht synchron auf die
PL011. `write_byte` wartet aktiv, solange die Sende-FIFO voll ist
(`drivers/bcm2711/pl011.rs:165-172`); bei 115200 Baud kostet ein Byte dann
etwa 87 us, und das ganze System steht. Die UART sendet ohne Flusskontrolle
auch ohne angeschlossenes Kabel — und an der echten Pi haengt keins
(`hal/uart.rs:52`).

## Entscheidung (Anwender 2026-10-02)

Auf der Pi ist die serielle **Ausgabe standardmaessig aus**. Zum Debuggen
wird sie ohne Neubau eingeschaltet: eine Option in `cmdline.txt` auf der
Boot-Partition (die Pi-Firmware reicht sie ueber den Device Tree,
`/chosen/bootargs`, an den Kernel), z. B. `serial=on`.

* **Panic/Exception-Ausgaben gehen immer auf die UART**, auch wenn sie aus ist.
* **RX bleibt unberuehrt**, nur das Senden wird abgeschaltet.
* **QEMU** (`qemu-virt` und QEMU `raspi4b`) braucht die Ausgabe: alle
  Messlaeufe werten das serielle Log aus. Dort muss sie an sein — ueber
  dieselbe Option in den QEMU-Startskripten/Abbildern oder ueber eine
  belegte Erkennung; nicht raten.

## Abnahme

Pi ohne `serial=on`: kein Byte geht an die UART (ausser Panic); mit
`serial=on`: wie heute. Alle bestehenden QEMU-Messwege liefern weiter ihr
serielles Log. Basislinien gehalten.
