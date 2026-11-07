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

## Stand 2026-11-07

Anwender, echte Pi, Kern `39ab2a6a…`, CoreMark frisch gebaut (`measurements/measurement-2026-11-07-hw08-pi4-card/RUNINFO.TXT`): Ausgabe sichtbar, am Ende keine Tastenabfrage, zurueck zum Desktop. Kein Absturz.

Gelesen, noch nicht am Orakel gemessen: In `references/emutos/desk/`, `aes/` und `bdos/` steht keine Tastenabfrage nach dem Programmende, `sh_ldapp` (`aes/gemshlib.c:645`) laeuft nach `dos_exec` weiter. Die Abfrage, die der Kommentar in `sdk/apps/build-coremark.sh` dem Desktop zuschreibt, gibt es dort nicht mehr; der Kommentar ist falsch und wird mit korrigiert.

Vorschlag des Leads: Punkt 1 am Orakel messen. Bestaetigt sich das, bleibt der Desktop wie er ist, und die Abfrage kommt ans Ende von `portable_fini` in `sdk/overlays/coremark/newtos/core_portme.c` (dort sind Aenderungen nach den CoreMark-Regeln erlaubt). Zu beachten: `COREVAL.NCP` laeuft in Messlaeufen aus `C:\AUTO`, und der Injektor laeuft nicht waehrend `Cconin`.

[Gemeinsame Regeln](README.md) gelten.
