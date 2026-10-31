# TICK01 — `_hz_200` ($4BA) nur in der Seite 0 des laufenden Prozesses

Status: **OFFEN** (Vor-Start-Pruefung beim Anwender) · Prioritaet: **P3** · Typ: Developer — Fund F1 aus [PITEST01](PITEST01.md) Teil 1 (2026-10-31)

## Sachverhalt (gemessen)

- Der 200-Hz-Tick schreibt `$4BA` nur in die Seite 0 des **gerade laufenden**
  Prozesses (`tos-api/src/bios/tick.rs:108@publish`, `current_page0_kva`).
- Folge: ein Prozess, der nicht lief (Elternprozess waehrend `Pexec`, ein
  anderer Prozess im kooperativen Wechsel), liest nach dem Zurueckkommen bis
  zu seinem naechsten Tick (<= 5 ms) einen veralteten Wert. Gemessen: eine
  P0REP-Fassung sah ueber 7,5 s Tastenwarten im Kind keine vergangene Zeit
  (`measurements/measurement-2026-10-31-pitest01-part1/RUNINFO.TXT`, Lauf
  `runs/p0rep-release-inj-v1`).
- EmuTOS: eine einzige globale Zelle, jeder liest den aktuellen Wert.

## Bezug

- [DOOM01](DOOM01.md) Kandidat K6 ("DOOMs `_hz_200` steht"): gleiche Zelle,
  anderer Mechanismus; HZPROBE (Testszenario A2) startet kein Kind und erfasst
  diesen Fall nicht.

## Arbeit (wenn freigegeben)

1. Beim Einschalten eines Prozesses (Kontextwechsel, Rueckkehr aus `Pexec`)
   den aktuellen Zaehler in dessen Seite 0 schreiben, bevor er weiterlaeuft —
   oder gleichwertig; Hosttest; Sonde wie P0REP v1 als Abnahme.

[Gemeinsame Regeln](README.md) gelten.
