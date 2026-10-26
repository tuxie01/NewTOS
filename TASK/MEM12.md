# MEM12 — EL0 darf Seite 0 beschreiben

Status: **OFFEN** · Prioritaet: **P1** · Typ: Kernfehler (Schutzgrenze fehlt) — Anwenderentscheidung 2026-10-26

## Sachverhalt

Seite 0 `[0, 0x1000)` ist in jedem Prozess `UserRwExec` abgebildet
(`tos-api/src/bdos/process/mmu.rs:643`), in jedem Speicherschutzmodus
(`tos-api/src/bios/reset.rs:26`). Ein EL0-Programm kann dort also lesen,
schreiben und ausfuehren.

Auf dem Atari loest ein Zugriff auf die unteren Adressen im Usermodus einen
Busfehler aus; die Systemvariablen sind nur im Supervisormodus erreichbar.
Anwenderentscheidung 2026-10-26: **EL0 darf Seite 0 nie beschreiben.** Ob EL0
sie lesen darf, haengt davon ab, ob native Programme dort Systemvariablen
lesen; das klaert dieses Ticket durch Suche und Messung. Liest keines, wird
Seite 0 fuer EL0 ganz ausgeblendet.

Was heute in Seite 0 liegt (`docs/COMPATIBILITY.md`, Abschnitt „One shared user
address space, page 0 per process“): Systemvariablen, OSHEADER mit `os_run`,
Cookie-Jar-Spiegel, Trampoline, `Keytbl`-Bereich, KEYS02-Tabelle, die
Vektortabelle bei `0x9A0`, der Reset-Stub (`RESET_STUB_VA`,
`tos-api/src/bios/reset.rs`) und der `S_NEWTOS_KBDVEC`-Stub. Seite 0 ist je
Prozess eigen.

Bekannte Folge: `__aline` ist per ADR-030 NULL; `__aline->Feld` mit positivem
Offset liest Seite 0 und schreibt dorthin, statt zu fehlen
(`docs/COMPATIBILITY.md`, Abschnitt „No Line-A“).

## Fragen, die das Ticket mit Belegen beantwortet

1. **Lesen:** Welche nativen Programme (`programs/`, `sdk/apps/`, mintlib,
   gemlib, cflib, AUTO-Programme, tosdump) lesen Adressen in `[0, 0x1000)` —
   direkt, ueber `_sysbase`/`Setexc`/`Supexec`-Rueckrufe oder ueber mintlib-
   Makros? `file:line` je Fundstelle; wo die Suche nicht reicht, messen
   (Seite 0 testweise ausblenden, Boot bis Desktop und die Anwendungen der
   Karte starten, Datenabbrueche protokollieren).
2. **Ausfuehren:** Welche Stubs in Seite 0 springt EL0 an (Reset-Stub,
   KBDVEC-Stub, Trampoline)? Brauchen sie Ausfuehrungsrecht in Seite 0, oder
   koennen sie in eine eigene, nur ausfuehrbare Seite umziehen?
3. **Schreiben:** Schreibt heute irgendein natives Programm nach Seite 0
   (z. B. `conterm` nach `Super()`)? Was macht EmuTOS dann im Supervisormodus,
   und was soll NewTOS stattdessen tun (Fehler, Prozessende)? Liest der Kern
   selbst Werte aus der Seite 0 des Prozesses zurueck, die ein Programm
   veraendert haben koennte?

## Arbeit

- Ergebnis von 1–3 als Abschnitt in diesem Ticket.
- Seite 0 fuer EL0 nicht beschreibbar machen (mindestens `UserRo`/`UserRx`,
  je nach 2); ohne Leser ganz ausblenden.
- Ein Schreibzugriff von EL0 nach Seite 0 beendet das Programm, der Kern
  laeuft weiter (Test).
- Das Ergebnis in `docs/COMPATIBILITY.md` festhalten (Seite-0-Abschnitt und
  der Hinweis im Abschnitt „No Line-A“).

## Abnahme

- Hosttest bzw. Selftest: EL0-Schreiben nach Seite 0 endet mit Prozessende,
  nicht mit Kernpanik; Lesen verhaelt sich wie unter 1 festgelegt.
- Frischer Bau beider Boards, QEMU-Boot bis Desktop (raspi4b mit
  `serial=on`); die Anwendungen der Karte starten wie vorher.
- `docs/COMPATIBILITY.md` beschreibt den neuen Stand.
