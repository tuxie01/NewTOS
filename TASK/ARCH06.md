# ARCH06 — `_MCH` = `0xFFFFFFFF` umsetzen; `_VDO` erst messen

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer — Anwenderentscheidung 2026-10-28 (Nachtrag ADR-012, aus [ARCH05](ARCH05.md))

## Sachverhalt

- ADR-012, Nachtrag 2026-10-28 (`docs/MEMORY.md`): `_MCH` = `0xFFFFFFFF`
  (EmuTOS „keine Atari-Hardware“), `NTOS` ist die NewTOS-Kennung. Der bisherige
  Wert `0x00050000` ist die ARAnyM-Kennung.
- Kopien des Wertes (Liste aus
  `docs/analysis/machine-identity-adr-proposal-2026-10-28.md`, Abschnitt 6):
  `tos-api/src/cookie.rs:223@MCH`, `kernel/src/selftest_cookie.rs:65@MCH`,
  `kernel/src/initinfo.rs:127@_MCH`, `programs/desktop/src/inf.rs:2597@_MCH`,
  `docs/api/desktop/deskmain.md`, `docs/HARDWARE-PROFILE.md`,
  `TASK/COOK01.md`, `TASK/X03.md` (Tickets: Nachtrag, nicht umschreiben).
- `_CPU`, `_FPU`, `_SND`, `_FDC` bleiben weg. `_VDO`: TeraDesk liest `_VDO`
  (`sdk/apps/teradesk/video.c:158`) und nimmt ohne den Cookie den ST-Zweig;
  was das auf NewTOS bewirkt, ist ungemessen.

## Lokalisierung (Lead, 2026-10-29)

- EmuTOS-Funktionen der Maschinenidentitaet sind alle `unmapped`:
  `setvalue_mch`, `setvalue_vdo`, `machine_detect`, `add_cookie_frb`,
  `guess_machine_name` (Gruppe E aus TOOL17). Ihr NewTOS-Gegenstueck ist der
  feste Satz in `tos-api/src/cookie.rs@seed_defaults` (`_MCH` heute
  `0x00050000`); die Kopien des Wertes stehen oben unter „Sachverhalt“.
- `--up --layers bios,bdos` je Funktion unvollstaendig nur wegen der
  Startfunktionen (`biosmain`, `bios_init`, …).
- Der Fix traegt die Zeilen der Gruppe E in `mapping.tsv` ein (`mapped` auf
  `seed_defaults` bzw. `design:` mit Beleg — Nachtrag ADR-012 mit Fundstelle).
- Retest: Cookie-Selbsttest, `Ssystem(S_GETCOOKIE)`, Programme, die `_MCH`
  lesen (mintlib `sysinfo` nach SDK03, zView-Zweig), Desktop-About-Box
  (`programs/desktop/src/inf.rs`), beide Boards bis zum Desktop.

## Arbeit

0. Lokalisierung (Lead, vor Beginn) nach CLAUDE.md „Testing“.
1. Wert und alle Kopien umstellen; Selbsttest und Hosttests anpassen.
2. **`_VDO` messen:** TeraDesk auf QEMU `raspi4b` starten (Standardabbild),
   beobachten, was der ST-Zweig tut (Aufrufe per `svc-seq-trace`/
   `bios-arg-trace`, Bildschirm per Screendump); Ergebnis mit Beleg im
   RUNINFO, **Entscheidung zu `_VDO` beim Anwender**, kein Fix in diesem
   Ticket.
3. mintlib `sysinfo(SI_PLATFORM)` mit `0xFFFFFFFF`: erst nach [SDK03](SDK03.md)
   (sonst `strlen(NULL)`).

## Abnahme

- Hosttests gruen; Cookie-Selbsttest (`kernel/src/selftest_cookie.rs`) und ein
  Lesen per `Ssystem(S_GETCOOKIE)` auf QEMU `raspi4b` zeigen `0xFFFFFFFF`;
  QEMU `raspi4b` und `qemu-virt` bis zum Desktop. Der tosdiff-Fall folgt mit
  [TOOL18](TOOL18.md) (Werkzeug hinter dem Fix).
- `_VDO`-Messung im RUNINFO; doc-check 0 Fehler; STATUS im selben Commit.

## Hinweis aus SDK04 (2026-10-28)

- `Getcookie()` liefert Cookie-Werte **null-erweitert**
  (`docs/analysis/lp64-tos-struct-survey-2026-10-28.md`): ein LP64-Programm
  liest `_MCH` = `0xFFFFFFFF` als `0x00000000FFFFFFFF`, nicht als `-1L`. Fuer
  den Vergleichshinweis im Nachtrag zu ADR-012 mitpruefen (mintlib `sysinfo`
  ist laut Erhebung nicht betroffen, `short hi` ergibt beidseitig -1).
- TeraDesk findet ein spaeter gesetztes `_VDO` heute nicht (SDK04 C6): die
  `_VDO`-Messung hier zuerst gegen den 16-Byte-Schritt bewerten.

## Anwenderentscheidung 2026-10-28

- `Getcookie` erweitert mit Nullen: fuer native Ports ist `0xFFFFFFFF` ein
  grosser positiver Wert, numerische `_MCH`-Vergleiche nehmen den
  Hardwarepfad „ueber Falcon“. Eingetragen in den Nachtrag zu ADR-012 und in
  `docs/sdk/08-porting-checklist.md` („numerische `_MCH`/`_VDO`-Vergleiche
  pruefen“).
- **Vor der `_VDO`-Messung** wird TeraDesks `find_cookie` (16-Byte-Schritt
  unter LP64, SDK04 Fund C6) behoben — sonst misst die Messung den Fehler
  statt den ST-Zweig. Teil von [SDK04](SDK04.md), vor ARCH06 in der Reihe.

## Anwender 2026-10-29 (aus TOOL17)

- Gruppe E der TOOL17-Klassifizierung gehoert hierher: `setvalue_vdo`,
  `setvalue_mch`, `machine_detect`, `machine_init`, `guess_machine_name`,
  `add_cookie_frb`, `total_alt_ram`, `altram_init` (heute `unmapped`). Mit der
  Umsetzung von ARCH06 je Zeile einordnen (`mapped` bzw. `design:` mit Beleg).
