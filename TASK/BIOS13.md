# BIOS13 — Querdokumente der drei Audits: COMPATIBILITY, MEM12, Ledger, Vergleichsregel, ERRATA, Waechter

Status: **OFFEN** · Prioritaet: **P3** · Typ: Researcher — Anwenderentscheidung 2026-10-28, aus den Audits `docs/audits/2026-10-28-bios-core.md`, `docs/audits/2026-10-28-xbios-chardev.md`, `docs/audits/2026-10-28-xbios-system.md`

## Sachverhalt

Dokument- und Regelbefunde, die keiner Funktion gehoeren (alle S4, MEASURED; Belege in den Berichten Abschnitt 2):

1. **BIOS-CORE-011 Punkt 1:** `bios-r1.tsv` Zeilen 53–57 (`Setexc`) mit NONZERO lassen NULL-freie Nicht-Adressen durch (BIOS-CORE-005); PTR_RAM (wie `bios-core-r1.tsv`) faengt beides. Regeln gehoeren dem Researcher.
2. **BIOS-CORE-011 Punkt 4:** `docs/COMPATIBILITY.md` (Abschnitt „Only an owner frees …“, Beobachtbare Unterschiede (3)): „a program walking the `Getmpb` lists finds a TSR's blocks on the allocated list“ — `Getmpb` liefert auf beiden Systemen `mp_mal = NULL` und ein einzelnes freies MD (Sonde G02/G03, `measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-EMUTOS-tt`).
3. **BIOS-CORE-011 Punkt 6:** `TASK/MEM12.md` Tabelle, Zeile „beliebige Zelle < 0x800“: fuer `$40C–$47F` falsch (BIOS-CORE-006, [BIOS07](BIOS07.md)).
4. **BIOS-CORE-011 Punkt 5:** `tosdump/cases/COVERAGE-BIOS.md`, Zeile `0x02 Bconin`: „Handles 0-3 … wait for input“ — `Bconin(0)` kehrt auf EmuTOS sofort mit 0 zurueck (Sonde B09, HANG-H0; `references/emutos/bios/parport.c:123`).
5. **Ledger-Eintraege fehlen** fuer `bios-core-r1.tsv` (BIOS-CORE-011 Punkt 5), `xbios-chardev-r1.tsv` (XBIOS-CHARDEV-007 Punkt 6), `xbios-system-r1.tsv` (XBIOS-SYS-006 Punkt 6) in `tosdump/cases/COVERAGE-BIOS.md`.
6. **XBIOS-SYS-006 Punkt 4:** `tosdump/cases/bios-r1.tsv` Kommentare zitieren `bios/nvram.c:180-181`, `:206-208`, `bios/floppy.c:703-717`, `:725-735`, `:711`; am Pin stehen die Stellen bei `references/emutos/bios/nvram.c:178-179`, `:208-210`, `references/emutos/bios/floppy.c:698-713`, `:727-735`, `:706`. Tabelle eingefroren (CRC); Kandidat fuer `tosdump/cases/ERRATA.md`.
7. **Waechter (BIOS-CORE Abschnitt 11.4 / XBIOS-CHARDEV 11.3):** der Round-1-Waechter weist Setz-Arme (`Kbshift`, `Setexc`), `Getmpb`, `Bconout` an Geraete, `Midiws`, `Bioskeys`, `Ikbdws` ab; die in den Berichten Abschnitt 5 vorgeschlagenen Zeilen sind dadurch nicht tabellenfaehig. Fehlt: eine wiederherstellende Vorstufe (Ledger „later“) und eine Capture-Form „durch den Rueckgabezeiger“ (XBIOS-CHARDEV 11.2). Das ist Werkzeugcode — Entscheidung des Anwenders, ob/als welches TOOL-Ticket.

## Arbeit

1. Punkte 1–6: aus Quelle und Messlaeufen korrigieren, jede Aussage mit `file:line` oder Laufdatei belegen.
2. Punkt 7: Umfang festlegen und dem Lead vorlegen; keine Werkzeugaenderung in diesem Ticket.

## Abnahme

- Jede korrigierte Aussage ist belegt; die drei Ledger-Eintraege nennen Tabelle, Faellezahl und CRC (`C056A3E0`, `5C3A5F7A`, `3944A677`).
- `cargo xtask test` gruen (unveraendert); kein vorher gleicher tosdump-Fall weicht neu ab; `cargo xtask doc-check` der geaenderten Doku 0 Fehler.

## Nachtrag 2026-10-28 (aus BIOS12)

- `tos-api/src/bios/vectors.rs`: Kommentare zu `ETV_DEFAULT_JUST_RTS` nennen
  `just_rts` auch fuer `etv_timer`; EmuTOS setzt dort `tikfrk`.
- `docs/api/bios/_state.md:292` verweist auf veraltete Zeilen in `vectors.rs`.
- `TASK/MEM12.md`: die `Setexc`-Fallzeile nennt „jede Zelle < 0x800“.
