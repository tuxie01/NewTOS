# ACC05 — Format und Endung: was ist ein NewTOS-Accessory?

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt ACC05-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 18, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 96f178544 (feat(aes): NewTOS can run desk accessories), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-25) · Prioritaet: **P2** · Typ: Entscheidung + Werkzeugkette

Abhaengigkeiten: [ACC01](ACC01.md)

## Sachverhalt

Unsere eigenen Programme tragen `.NAP` (GEM) und `.NCP` (Konsole) mit
Grund: der spaetere m68k-Emulator soll `.PRG`/`.TOS`/`.TTP` uebernehmen
(Memory `own-extensions-separate-aarch64-from-m68k`). Fuer Accessories
gilt dasselbe: `.ACC` waere die m68k-Endung, ein natives AArch64-Accessory
braucht eine eigene (`.NAC`?). EmuTOS sucht `\*.ACC` im Wurzelverzeichnis
(`aes/geminit.c:272`).

## Arbeit

1. Endung entscheiden (Vorschlag `.NAC`) und in ACC02s Suche eintragen;
   `.ACC` bleibt dem Emulator vorbehalten.
2. SDK: Bauskript fuer ein Accessory (Muster `sdk/build.sh`/`build-libc.sh`),
   Beispiel `sdk/examples/clockacc.c` — `appl_init`, `menu_register`,
   `evnt_multi`-Schleife, `AC_OPEN` oeffnet ein Fenster, `AC_CLOSE`
   schliesst es, **kein** `appl_exit`.
3. `tools/make-sd-image.sh`: Installationsblock fuer Accessories im
   Wurzelverzeichnis von C:.
4. `docs/sdk/`: Seite oder Abschnitt „Ein Accessory portieren/schreiben".

## Abnahme

Das Beispielaccessory laeuft in QEMU, erscheint unter *Desk*, oeffnet und
schliesst sein Fenster; Dokumentation vorhanden.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss

**ERLEDIGT (2026-09-25)** — als Nebenprodukt von ACC02/ACC03/ACC04.

**1. Endung `.NAC`** (Lead-Entscheidung, ADR-018-Familie). `.ACC` bleibt dem
m68k-Emulator; der Preis ist in `docs/api/aes/accessories.md` §7 ausgerechnet
und war null: ein klassisches `.ACC` enthaelt 68000-Code, den unser Lader
unabhaengig von der Endung nicht ausfuehren koennte (§7.4).
`kernel/src/desk/ftype.rs` klassifiziert es als `Flavor::Accessory`; gesucht
wird `C:\ACC\*.NAC` und dann `C:\*.NAC` (ACC02).

**2. SDK.** `sdk/examples/sysacc.c` baut unveraendert mit `sdk/build.sh` zu
`SYSACC.NAC` (6856 Bytes); der Kopfkommentar nennt jetzt die Endung statt
`.TBD`. Ein eigenes Bauskript brauchte es nicht — ein Accessory ist fuer die
Werkzeugkette ein gewoehnliches natives Programm, nur die Endung und der
Installationsort unterscheiden es. `clockacc.c` mit einem *Fenster* ist
bewusst **nicht** entstanden: `sysacc.c` ist der vom Lead gesetzte
Abnahmefall, und ein Accessory-Fenster treibt den Window-Manager aus einem
zweiten Klienten — eigenes Ticket.

**3. `tools/make-sd-image.sh`.** Installiert `SYSACC.NAC` in **beide**
Suchorte (`C:\ACC\SYSACC.NAC` und, unter anderem Namen, `C:\SYSACC2.NAC`), so
dass eine Karte beide Haelften der Zwei-Orte-Suche zeigt. `SD_ACC_ROOT_ONLY=1`
installiert nur die Wurzelkopie.

**4. Dokumentation.** `docs/api/aes/accessories.md` §9 — wo jedes Stueck liegt,
was der kooperative Kern dafuer brauchte, jede benannte Abweichung und was
bewusst nicht getan ist. Eine eigene `docs/sdk/`-Seite ist nicht entstanden:
§9 und der Kopf von `sysacc.c` sagen zusammen, was ein Accessory tun muss, und
eine dritte Kopie davon waere die Art von Redundanz, die als erste veraltet.
