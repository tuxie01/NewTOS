# ARCH05 — ADR: welche Maschine meldet NewTOS (`_MCH`) und was folgt daraus

Status: **OFFEN** · Prioritaet: **P2** · Typ: Researcher (ADR-Vorschlag), Entscheidung Anwender — Anwenderentscheidung 2026-10-28, aus [XBIOS10](XBIOS10.md)

## Sachverhalt

- XBIOS-SYS-003 (`NVMaccess` nicht verdrahtet, [XBIOS10](XBIOS10.md)):
  Anwender 2026-10-28 — die NVRAM-Frage folgt daraus, welche Maschine NewTOS
  meldet.
- Geprueft (Lead 2026-10-28): ADR-012 (`docs/MEMORY.md`) haelt nur fest, dass
  NewTOS `_MCH` als eigene Maschinenkennung behaelt. Der Wert `0x0005_0000`
  ist im Code als vorlaeufig markiert
  (`tos-api/src/cookie.rs:223@seed_defaults`: „Values are provisional
  bring-up policy“). Kein ADR leitet daraus maschinenabhaengiges Verhalten ab
  (NVRAM, weitere XBIOS-Funktionen, die EmuTOS je Maschine schaltet).
- Referenzprofil der Spec ist EmuTOS make-512 als Falcon030/TOS4
  (`docs/api/_emutos-config.md`).

## Arbeit

1. Bestand: wo EmuTOS Verhalten am Maschinentyp festmacht (`_MCH`-Werte,
   `cookie_mch`, `HAS_*`/`IS_*`-Pruefungen fuer NVRAM, RTC, DSP, Videomodi
   usw.), welche davon fuer NewTOS beobachtbar sind.
2. Was Programme mit `_MCH` tun (portierte Programme, SDK), mit Quelle.
3. Bedeutung von `0x0005_0000` belegen (EmuTOS/FreeMiNT-Quellen); kollidiert
   der Wert mit einer vergebenen Kennung?
4. ADR-Vorschlag mit Optionen (z. B. eigene Kennung ohne Atari-Hardware,
   Falcon-Kennung, Falcon-Kennung mit Abweichungsliste), je mit Folgen fuer
   NVRAM (`NVMaccess`), RTC und die anderen maschinenabhaengigen Funktionen,
   und mit Messfall.
5. Entscheidung beim Anwender; danach XBIOS10 entsprechend.

## Abnahme

- ADR-Vorschlag mit Quellen; vom Anwender entschieden und als ADR bzw.
  Nachtrag zu ADR-012 in `docs/MEMORY.md` eingetragen (Lead).
- doc-check der geaenderten Doku 0 Fehler.
