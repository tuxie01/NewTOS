# BIOS16 — Direkt geschriebene Vektorzelle in Seite 0 wird nicht uebernommen

Status: **OFFEN** (Entscheidung beim Anwender) · Prioritaet: **P2** · Typ: Developer — Fund aus [BIOS07](BIOS07.md) (2026-10-29)

## Sachverhalt

- Seit BIOS07 sind die Vektorzellen `num <= 0x103` **eine** geteilte Zelle
  (`tos-api/src/bios/vectors.rs`, ADR-031 Regel 3); jede Seite 0 traegt eine
  Kopie. Schreibt ein Programm die Kopie direkt (Super-Zustand, z. B.
  `*(long *)0x400 = handler` fuer `etv_timer`), aendert das die Zelle nicht;
  nur die Seite-0-Warnung (BIOS15) kommt. `Setexc(n,-1)` liefert danach den
  alten Wert, auf EmuTOS den neuen (eine Vektortabelle, direkte Schreibung
  IST der Vektor). Dokumentiert in `docs/COMPATIBILITY.md` („`Setexc` and
  page 0“).
- Nebenbefunde BIOS07: `swv_vec` (`$46E`) startet auf EmuTOS mit `just_rts`
  (`references/emutos/bios/bios.c:363`), auf NewTOS mit 0; veraltete
  Kommentare `sdk/apps/apitest/apitest.c:516@Setexc` („vector shadow holds 64-bit
  values“) und `tos-api/src/bios/tick.rs:53@etv_timer` („shadows the vector“).

## Vorschlag (Lead)

Am Trap-Ende, wo `check_page0` die Kopie mit der Zelle vergleicht und warnt
(BIOS15), den neuen Wert in die geteilte Zelle **uebernehmen** (weiter warnen,
nie aufrufen — ADR-031 bleibt). Dann liefert `Setexc(n,-1)` wie EmuTOS den
direkt geschriebenen Wert. `swv_vec` mit `just_rts` vorbelegen; Kommentare
richtigstellen.

## Abnahme

- Sonde: direkte Schreibung, danach `Setexc(n,-1)` gleich dem Orakel; Warnung
  wie BIOS15; keine neue Abweichung; Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.
