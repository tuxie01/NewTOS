# Audit-Liste: angefangene, nicht fertige Tickets (2026-10-26)

Alle Tickets unter `TASK/`, an denen schon gearbeitet wurde, die aber nicht
abgeschlossen sind (Stand `d6619f6f`). Ersetzt fuer den Audit die Listen
`audit-50-tickets`, `audit-50-started-tickets` und `audit-50-worked-tickets`
vom selben Tag.

Auswahlregel:
- „Gearbeitet“ = Quittung `-STATUS-*.MD`, eigener Messlauf unter
  `measurements/`, Status TEILWEISE oder ein Stand-/Abschluss-Abschnitt mit
  erledigten Teilpunkten.
- Nicht aufgenommen: ERLEDIGT (auch mit offener Pi-Abnahme, z. B. A111, A115,
  A117), BEHOBEN und auf der Pi abgenommen (DESK46, PORT05, SCHED01), BEHOBEN
  ohne offenen Rest (ACC10, FS04), GEKLAERT (PORT08), GEGENSTANDSLOS (A31, A51,
  A58), ZURUECKGEZOGEN (DESK23, DESK29), GESCHLOSSEN ohne Umsetzung (GUI01,
  BALL01) und alle offenen Tickets ohne jede Spur von Arbeit.
- „Zuletzt“ = letzter Commit auf Ticket oder Quittung; „Messl.“ = Anzahl
  `measurements/measurement-*-<id>*`.

## A. Umgesetzt, Abnahme offen

| Ticket | Zuletzt | Messl. | Was fehlt | Thema |
|---|---|---|---|---|
| A16 | 2026-10-01 | 0 | HW-Abnahme | wind_close: WS_CURR / WS_PREV / WS_WORK bleiben unaktualisiert |
| A34 | 2026-10-01 | 1 | Abnahme (A33) | `wind_create` schreibt `WS_WORK`, EmuTOS nie |
| A37 | 2026-10-01 | 2 | anwendungsseitige Abnahme | fix_tedinfo_std fehlt: te_txtlen wird nicht neu berechnet |
| A78 | 2026-10-01 | 1 | Tester- und HW-Abnahme | wind_set(WF_CXYWH): kein w_move-Blit, ein WM_REDRAW zu viel, Fan-out in Handle- statt z-Reihenfolge |
| DESK34 | 2026-10-01 | 1 | HW-Abnahme | Ein Unterverzeichnis oeffnet ein neues Fenster statt im selben zu navigieren |
| DESK35 | 2026-10-01 | 1 | HW-Abnahme | Beim Start einer GEM-Anwendung bleiben die Desktop-Fenster stehen und frieren ein |
| DESK42 | 2026-10-01 | 1 | HW-Abnahme | Der `Show info`-Kasten sieht EmuTOS' nicht aehnlich |
| DESK44 | 2026-10-01 | 0 | HW-Gegenprobe | `.NTP` laeuft als GEM, soll aber Konsole mit Parametern sein |
| GUI06 | 2026-10-01 | 1 | Pi-Abnahme | Der Boden der Geste: zwei Vollbild-Neuzeichnen und 400–600 MB Cachepflege |
| HW03 | 2026-10-01 | 1 | drei Pi-Gegenproben | Schreibfehler der Karte erreichen niemanden, und ein Sektor kann still verloren gehen |
| V17 | 2026-10-01 | 1 | Tester-Abnahme | Mauszeiger startet einen Pixel zu weit unten rechts |
| FS09 | 2026-10-07 | 1 | Quittung FERTIG (QEMU), Status steht noch OFFEN; Pi-Abnahme | Block read cache after FreeMiNT block_IO |
| INPUT01 | 2026-10-08 | 1 | Quittung vorhanden, Status steht noch OFFEN; Abnahme | the mouse pointer freezes for 2-3 s while zView starts |
| ZVIEW01 | 2026-10-07 | 1 | Quittung FERTIG (QEMU), Status OFFEN; Pi-Nachweis | zView browser shows no file or folder names |
| ZVIEW02 | 2026-10-07 | 1 | Quittung vorhanden, Status OFFEN; Abnahme | double-click on a folder tile in zView's browser only selects |
| ZVIEW04 | 2026-10-08 | 1 | Quittung vorhanden, Status OFFEN; Abnahme | File > Quit does not quit zView while the browser window is open |

## B. Teilweise umgesetzt

| Ticket | Zuletzt | Messl. | Was fehlt | Thema |
|---|---|---|---|---|
| A118 | 2026-10-26 | 4 | Umbau auf EmuTOS 1:1 fuer abgelehnte Wiederausfuehrung, 5 Testerfaelle | Restliche Nehmer der `wind_update`-Sperre nach EmuTOS |
| USB01 | 2026-10-25 | 14 | F1–F3 erledigt; F4/F6/F7, SET_IDLE, Gamepads | USB-Maus: nur Boot-Protokoll, Steckkombinationen, Diagnosewerkzeug |
| USB02 | 2026-10-24 | 4 | Punkte 10/11 erledigt; Punkte 1–9 | Kleinbefunde aus USB01 F1/F2/F8 |
| ARCH03 | 2026-10-13 | 10 | Stufe 2b, `asid=on` auf echter Pi | Prozess- und Speicherarchitektur nach FreeMiNT: ein gemeinsamer Adressraum mit Schutzrechten |
| FS17 | 2026-10-21 | 0 | Punkt 3 erledigt (FS18); Punkte 1, 2, 4, 5, 6 offen, 7 ungeprueft | Kleinere GEMDOS-Datei-Abweichungen aus PROC04 Punkt 5 |
| PORT01 | 2026-10-01 | 1 | Ursache behoben; Abnahme blockiert durch zweiten Defekt | Eine auf QED gezogene Datei erreicht die Anwendung nicht |
| PORT11 | 2026-10-21 | 0 | Plain-crt0-Absatz in `docs/sdk/03` | SDK-Doku auf den Stand nach E9 bringen (`malloc` ueber `Malloc`, crt0) |
| T08 | 2026-10-21 | 0 | Test fuer den Vektortausch | VDI `vex_*`: Vektortausch in `CONTRL[7..]`/`[9..]` fest 8 Byte an festem Index |
| T10 | 2026-10-21 | 0 | Abnahme alter a78-Lauf | Die duale Injektor-Bewaffnung waehlt seit den Accessories immer das leere Skript |
| T13 | 2026-10-01 | 0 | `newtos_pause()` fertig; SDK-Beispiele zurueckgestellt | Die Testprogramme sollen ihr Ergebnis stehenlassen |

Summe: 16 umgesetzt mit offener Abnahme, 10 teilweise umgesetzt.
