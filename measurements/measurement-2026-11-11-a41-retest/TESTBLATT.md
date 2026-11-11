# Testblatt A41-Nachtest auf der echten Pi 4 (2026-11-11)

Prüft A41 (Farbicons), A46 (gepunktete Menütrenner), A40 (Menüleiste volle Breite), A39/H05
(Programm starten und zurück). Karte: Kernel `kern1111.img` = `kernel8.img` (sha256 `5853cf79…`),
TeraDesk mit `cicons.rsc` (`ee7de7f3…`), QED — Details in `RUNINFO.TXT`. Bildschirm 1920x1080.
Jedes Foto bitte mit Schrittnummer benennen (z. B. `3a.jpg`). Bei FAIL: Foto + ein Satz, was anders ist.

| Nr | Was tun | Erwartetes Bild | Foto | PASS, wenn … |
|---|---|---|---|---|
| 1 | Pi mit der Karte starten. | Bootschirm, dann der NewTOS-Desktop. | — | Desktop erscheint, kein Absturz. |
| 2 | Im NewTOS-Desktop ein Menü mit Trennlinie öffnen (z. B. das zweite Menü von links). | Die Trenner zwischen den Gruppen sind **gepunktete Linien** über die volle Breite der Tafel, keine Reihe `-----`. | 2 | Linie gepunktet, keine Bindestriche (A46). |
| 3 | `C:\Apps\TERADESK\DESKTOP.NAP` doppelklicken. | TeraDesk-Desktop: Menüleiste `Desk File View Window Options`. Laufwerks-Icons „HARD DISC“: **graue Schublade mit buntem Ordnerstapel (rot/gelb/blau)**; „TRASH“: **grüne Tonne mit roten Akzenten**. | 3a Gesamtbild, 3b Nahaufnahme Icons | Icons farbig (A41). FAIL: nur schwarz-weiß. |
| 4 | Menüleiste von TeraDesk ansehen. | Die weiße Leiste reicht bis **ganz an den rechten Bildschirmrand**. | im Foto 3a | Leiste volle Breite, nicht nach ca. 1/3 zu Ende (A40). |
| 5 | In TeraDesk Menü **File** öffnen. | Trenner zwischen den Gruppen als **gepunktete Linien** über die volle Tafelbreite. | 5 | wie Nr. 2 (A46). Menü wieder schließen. |
| 6 | Laufwerk C doppelklicken; im Fenster den Ordner `Apps` doppelklicken. Zeigt ein Fenster Text statt Icons: **View → Show as icons**. | Zwei Fenster (`C:\` und `C:\APPS\`), **gelbe Ordner-Icons**, farbige Datei-Icons; unter der Titelzeile die Infozeile „… bytes used in … items“. | 6 | Ordner gelb, Infozeile sichtbar (A41). Lage beider Fenster merken. |
| 7 | Im Fenster `C:\APPS\` den Ordner `QED` öffnen, `QED.NAP` doppelklicken. | QED startet mit eigener Menüleiste (`Datei` …) über die **volle Breite**. | 7 | QED läuft, Leiste volle Breite (A40). |
| 8 | In QED **Datei → Beenden** (oder Ctrl+Q). | Zurück in TeraDesk: **dieselben Fenster an derselben Stelle**, gleiche Pfade, Infozeilen da, Icons weiter **farbig**, Menüleiste TeraDesks. | 8 | alles wie vor Nr. 7 (H05, A39). FAIL: Fenster weg, leere/schwarz-weiße Icons, kaputte Menütexte. |
| 9 | In TeraDesk Menü **File** öffnen, Texte prüfen; dann **File → Quit/Shutdown → Quit**. | Menütexte korrekt; danach direkt der NewTOS-Desktop (kein zweites TeraDesk). | 9 | Rückkehr zum NewTOS-Desktop ohne Absturz (H05, A39). |

Gesamt: PASS nur, wenn 1–9 PASS. Rückmeldung je Nummer: PASS/FAIL + Foto.
Hinweis: Zeigt QED beim Start einen Hinweis (z. B. zum Ordner `CLIPBOARD`), bestätigen und notieren —
kein Teil dieses Tests.
