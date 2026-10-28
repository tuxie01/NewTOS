# SDK04 — LP64: `long`-Zugriffe auf TOS-Strukturen in allen Ports; Typen fester Breite im SDK

Status: **OFFEN** · Prioritaet: **P1** · Typ: Researcher (Erhebung) + Developer (SDK-Typen, Fixes nach Messung) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Dritter Fall derselben Klasse: ein portiertes Programm liest eine
  TOS-Struktur mit `long`, das unter LP64 8 statt 4 Byte breit ist:
  - TeraDesk `hdv_*` und `_sysbase` (TASK/MEM12.md, Bestand der
    Seite-0-Leser);
  - Cookie-Jar mit `long`-Paaren: HighWire
    (`sdk/apps/highwire/src/romvdi.c:36-52@getcookie`) und TeraDesk
    (`docs/analysis/machine-identity-adr-proposal-2026-10-28.md`, Abschnitt 7;
    Wirkung ungeprueft).
- Bekannte Regel: TOS-Strukturen sind auf dem Draht 4-Byte-breit, mintlibs
  Strukturen unter LP64 nicht (BASEPAGE/DTA/AESGLOBAL ueber Overlays
  geloest).

## Arbeit

1. **Erhebung (Researcher):** alle Ports (`sdk/apps/`, `sdk/overlays/`,
   `sdk/examples/`, `programs/`, Bibliotheken unter `sdk/`) auf Zugriffe mit
   `long`/`unsigned long`/Zeigertypen auf TOS-Strukturen: Cookie-Jar
   (`_p_cookies`, `0x5A0`), Systemvariablen (Seite 0, `_sysbase`/OSHEADER),
   Basepage, DTA, weitere (IOREC, MD/MPB, BPB). Tabelle: Programm, Stelle
   (`file:line`), Struktur, Typ, erwartete Wirkung unter LP64 (gelesen vs.
   abgeleitet). Verfahren wie `.claude/agents/researcher.md`
   „Program surveys“.
2. **Messen, wo gefunden (Developer/Tester):** je Fund ein Lauf auf QEMU
   `raspi4b`, der die Wirkung zeigt (z. B. findet HighWire `EdDI`?); Funde ohne
   Wirkung bleiben dokumentiert. Fixes als benannte Patches in
   `sdk/overlays/<prog>/` erst nach Messung.
3. **SDK-Typen fester Breite** fuer diese Strukturen (z. B. Cookie-Eintrag,
   Systemvariablen-Zugriff, OSHEADER), mit Pruefung zur Uebersetzungszeit
   (`sizeof`/`offsetof` gegen `docs/api/`), damit neuer Code den Fehler nicht
   wiederholt; Doku in `docs/sdk/`.

## Abnahme

- Erhebungstabelle mit Belegen; je Fund Messung oder Begruendung.
- SDK-Hosttests gruen mit den neuen Pruefungen; doc-check 0 Fehler.
