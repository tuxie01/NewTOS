# DESK70 — Textprogramm vom Desktop gestartet: Ausgabe verschwindet sofort (GitHub #12)

Status: **OFFEN** · Prioritaet: **P3** · Typ: Frage, erst EmuTOS-Verhalten pruefen — wosch163 (#12, 2026-10-22)

## Befund

#12: CoreMark (`APPS\COREMARK\CORE.NCP`) vom Desktop gestartet laeuft etwa
30 s und kehrt dann zum Desktop zurueck; die Ergebnisse sind nicht zu sehen.
Aus EmuCON gestartet stehen sie da (wosch163 bestaetigt). ragnar76 wuenscht
sich ein Konsolenfenster (toswin2) bzw. ein VT52-Programm.

## Auftrag

1. Erst messen, was EmuTOS' Desktop nach dem Ende eines TOS-Programms tut
   (`references/emutos/desk/` Startpfad fuer `.TOS`/`.TTP`; Orakel in Hatari
   mit einem kurzen TOS-Programm, das Text ausgibt und endet): kehrt er sofort
   zurueck, ist NewTOS konform und #12 wird mit Erklaerung (EmuCON als Weg)
   beantwortet; wartet er, ist es ein Fehler.
2. Ergebnis an den Anwender; der Konsolenfenster-Wunsch ist ein eigenes Thema
   (nach 1.0).

[Gemeinsame Regeln](README.md) gelten.
