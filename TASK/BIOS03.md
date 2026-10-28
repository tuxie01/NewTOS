# BIOS03 — ADR „OS hooks installed by programs“: welche eingehaengten Haken wirken

Status: **OFFEN** · Prioritaet: **P1** · Typ: Researcher (Bestand + ADR-Vorschlag), Entscheidung Anwender — Anwenderentscheidung 2026-10-28 (erweitert am selben Tag)

## Sachverhalt

- Programme haengen sich bei TOS in OS-Haken ein; NewTOS ruft mehrere davon
  nie:
  - `hdv_rw`/`hdv_bpb`/`hdv_mediach` — BIOS-STOR-010
    (`docs/audits/2026-10-28-bios-storage.md`); RAM-Disks, Plattencaches.
  - `etv_timer` per `Setexc(0x100)` — [BIOS05](BIOS05.md).
  - `mousevec`/`statvec`/`clockvec`/`joyvec` der `Kbdvbase`-Tabelle —
    [XBIOS03](XBIOS03.md).
- Anwenderentscheidung 2026-10-28: **ein gemeinsames ADR** „OS hooks
  installed by programs“, wie bei Line-A (ADR-030 in `docs/MEMORY.md`), mit
  einer Tabelle je Haken: wirkt / wirkt mit Semantik X / wirkt nicht +
  Grund + beobachtbare Folge. **Je Haken nach Beleg entscheiden:** zuerst
  der Bestand, welche portierten Programme welchen Haken nutzen (wie die
  Line-A-Erhebung).
- **Regel: nie still.** Haengt ein Programm einen Haken ein, den NewTOS nicht
  bedient, schreibt NewTOS einmal eine Warnung ins Log.
- Bezug: seit MEM12 ist Seite 0 fuer EL0 nur im Super-Zustand beschreibbar
  (`docs/api/_super-state.md`).

## Arbeit

1. Liste der Haken: alle Vektoren und Tabellen, ueber die EmuTOS
   programm-eingehaengten Code ruft (Systemvektoren in Seite 0, `Setexc`-
   Nummern, `Kbdvbase`, `etv_*`, `hdv_*`, weitere aus
   `references/emutos/bios/`), je mit Aufrufstelle.
2. Bestand: welche portierten Programme (`sdk/apps/`, `sdk/overlays/`,
   Portliste, Kandidaten fuer 1.0/2.0) welchen Haken setzen, mit Quelle —
   Vorbild die Line-A-Erhebung hinter ADR-030.
3. Was NewTOS heute je Haken tut (Adresse vorhanden, gelesen, geschrieben,
   gerufen), mit Codezitat.
4. ADR-Vorschlag mit der Tabelle je Haken (wirkt / wirkt mit Semantik X /
   wirkt nicht + Grund + Folge), Aufwand und Sicherheitsfolgen (EL0-Code im
   Kernpfad), der Warnregel (einmal je Haken und Programm? — Vorschlag) und
   dem Messfall je Haken.
5. Entscheidung beim Anwender; danach Developer-Tickets (BIOS05, XBIOS03,
   `hdv_*`) gemaess Tabelle.

## Abnahme

- ADR-Vorschlag mit Quellen; vom Anwender entschieden und als ADR in
  `docs/MEMORY.md` eingetragen (Lead).
- doc-check der geaenderten Doku 0 Fehler.

## Nachtrag 2026-10-28

- E6 aus [XBIOS01](XBIOS01.md): der `kbdvec`-Haken bekommt den
  IOREC-Zeiger als Argument (EmuTOS `a0`, `references/emutos/bios/aciavecs.S:412`)
  — Anwender: ja, im ADR mit festzuhalten.
