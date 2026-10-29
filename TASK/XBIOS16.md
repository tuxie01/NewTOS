# XBIOS16 — `Jdisint`/`Jenabint`: Rueckgabewert gegen das Orakel messen

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer (Messung zuerst) — Anwenderentscheidung 2026-10-29, Nebenbefund aus [BIOS15](BIOS15.md)

## Sachverhalt

- `Jdisint` (XBIOS 0x1A) und `Jenabint` (0x1B) liefern auf NewTOS ihre
  Funktionsnummer (`tos-api/src/xbios/mod.rs@dispatch`, Sammelzweig). EmuTOS
  laesst nach `Mfpint`/`Xbtimer` das MFP-Maskenregister in `d0` (gemessen in
  BIOS15: IMRB 0x60, IMRA 0x14; `docs/api/xbios/0d-mfpint.md`); ob `Jdisint`/
  `Jenabint` ebenso etwas Definiertes in `d0` lassen, ist nicht gemessen.

## Arbeit

0. Lokalisierung (Lead, vor Beginn) nach CLAUDE.md „Testing“.
1. Messen am Orakel (tt, falcontc) mit der Sonde `tosdump/probes/hkprobe.c`
   oder einer Erweiterung: `Jdisint(n)`, `Jenabint(n)` fuer Interrupts aus
   IMRA und IMRB, Rueckgabe in `d0`.
2. Nach Messung: NewTOS liefert dasselbe aus dem Maskenabbild von
   `tos-api/src/xbios/mfp.rs`; ADR-031 bleibt (nicht bedient).

## Abnahme

- Sonde beidseitig gleich; keine neue Abweichung; Hosttests gruen;
  doc-check 0 Fehler.
