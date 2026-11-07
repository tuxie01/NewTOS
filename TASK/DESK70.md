# DESK70 — Textprogramm vom Desktop gestartet: Ausgabe verschwindet sofort (GitHub #12)

Status: **ERLEDIGT — Tester, Pi und Antwort auf #12 offen** · Prioritaet: **P3** · Typ: Frage, erst EmuTOS-Verhalten pruefen — wosch163 (#12, 2026-10-22)

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

## Stand 2026-11-07 (Umsetzung)

Punkt 1 ist gemessen (`7d700cf3`, `docs/audits/2026-11-07-desk70-tos-exit.md`): Weder EmuTOS' Desktop noch unserer wartet nach dem Programmende. Der Desktop bleibt wie er ist (Anwender).

Umgesetzt (Developer, Quittung [DESK70-STATUS-DONE.MD](DESK70-STATUS-DONE.MD)):

- `portable_fini` in `sdk/overlays/coremark/newtos/core_portme.c` wartet nach dem letzten Ergebnis immer auf eine Taste (`Press any key to exit.`, `Crawcin`), ausserhalb der Zeitmessung. Geaendert ist nur `core_portme.c` (CoreMark-Regeln). Der erste Bau (`729bab3b`) wartete nur, wenn das aktuelle Verzeichnis nicht die Wurzel war (`Dgetpath`), damit CORE/COREVAL aus `C:\AUTO` weiter durchliefen; das hat der Anwender verworfen (siehe unten).
- Vier Textstellen aus dem Audit richtiggestellt (`sdk/apps/build-coremark.sh`, `docs/sdk/06-image-and-testing.md`, `docs/sdk/00-quickstart.md`, `docs/benchmarks/coremark.md`), dazu der CoreMark-Kommentar in `tools/make-sd-image.sh`, der nach der Aenderung falsch geworden waere.
- QEMU raspi4b, erster Bau `measurements/measurement-2026-11-07-desk70-build/RUNINFO.TXT`: vom Desktop gestartet steht das Ergebnis 53 s bis zur Taste, danach ist der Desktop pixelgleich mit dem Bild vor dem Klick. (Der AUTO-Teil dieses Laufs gilt nicht mehr.)

## Stand 2026-11-07 (Anwenderentscheidung: immer warten)

Anwender 2026-11-07: CoreMark wartet nach dem Ergebnis immer auf eine Taste, ohne Verzeichnistest, ohne Bauschalter, ohne Variante. AUTO ist fuer Treiber; CoreMark ist ein Programm, das vom Desktop startet. Ein Messlauf startet es per `#Z`-Zeile in `NEWDESK.INF`, liest das Ergebnis von der seriellen Leitung und beendet QEMU.

Umgesetzt (Developer, Quittung ergaenzt):

- `portable_fini`: `Dgetpath`-Test entfernt, Aufforderung und `Crawcin` bleiben.
- `tools/make-sd-image.sh`: Schalter `SD_COREMARK_AUTO` entfernt. Er haette jeden damit gebauten Bootvorgang angehalten; kein lebendes Skript nutzte ihn, nur vergangene, unveraenderliche Laufverzeichnisse. Ersatz fuer Messlaeufe ist die `#Z`-Zeile (`#Z 00 C:\COREMARK\CORE.NCP@`), derselbe Weg wie `tools/xtask/src/tosdump.rs` und `tosdump/probes/run-akprobe.sh`; kein neues Werkzeug.
- Texte angepasst: `sdk/apps/build-coremark.sh`, `docs/benchmarks/coremark.md`, `docs/sdk/06-image-and-testing.md` (Schalterliste, Manifest-Beispiel), Kommentar in `tools/make-sd-image.sh`. `docs/sdk/00-quickstart.md` sagt nichts ueber AUTO oder die Wurzel und bleibt.
- QEMU raspi4b, `measurements/measurement-2026-11-07-desk70-build2/RUNINFO.TXT`: `#Z`-Start von `C:\COREMARK\CORE.NCP`, Ergebnis und Aufforderung stehen 15,6 s bis zur eingespeisten Taste, danach `desktop up`; ohne Eingabe steht der vollstaendige Bericht vor der Aufforderung auf der seriellen Leitung, der Schirm bleibt 48 s bis zum Ende von QEMU stehen.

Offen: Tester-Abnahme; echte Pi (Testszenario); die Antwort auf #12 (Anwender).

[Gemeinsame Regeln](README.md) gelten.
