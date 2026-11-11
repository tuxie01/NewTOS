# BOOT01 — Bootschirm: Layout auf hohen Aufloesungen, Ausrichtung, Tippfehler

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/BOOT01.md status line: OFFEN (Punkt 1 zentrierte Box erledigt `2ace2639`, `measurements/measurement-2026-10-09-initinfo/`, , no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** (Punkt 1 zentrierte Box erledigt `2ace2639`, `measurements/measurement-2026-10-09-initinfo/`; Punkte 2–5 offen: `USB_INSET`, `Port`-Anzeige, „isnt“, Hold-Zeilen in `kernel/src/initinfo.rs`) · Prioritaet: **P3** · Typ: Optik / Feinschliff — Anwenderbefund am Geraet (2026-09-23)

Abhaengigkeiten: —

## Herkunft

Fotos vom laufenden System (Pi 4, 1920×1080). Das Bild ist im Kern richtig —
Rahmen, Logo, Trennlinien und Ausrichtung des Infoblocks stimmen; eine
fruehere Lesart „Linien laufen ins Leere" war ein Ausschnittsfehler und ist
hiermit zurueckgenommen.

## Die Stellen (`kernel/src/initinfo.rs`)

1. **Der Inhalt sitzt im oberen Drittel.** Die Layoutschritte sind feste
   Pixelwerte um den 8×16-Font (`y += 18/22/24/26`, `:219-321`), der Rahmen
   spannt dagegen den ganzen Schirm (`rect(16, 12, sw-17, sh-13)`, `:195-197`).
   Bei 640×480 fuellt das; bei 1920×1080 bleiben zwei Drittel leer.

   **Entscheid des Anwenders (2026-09-23): eine zentrierte Box.** Statt den
   Rahmen ueber den ganzen Schirm zu spannen, umschliesst ein Kasten genau
   den Inhalt — Logo, Untertitel, Infoblock, Geraeteliste, Hinweise, Fussnote —
   und dieser Kasten steht waagerecht **und** senkrecht mittig auf dem Schirm.
   Die Kastenbreite richtet sich nach der laengsten Zeile plus Rand, die
   Hoehe nach dem Inhalt; die Trennlinien laufen dann von Kastenrand zu
   Kastenrand statt von Schirmrand zu Schirmrand. Damit sieht das Bild auf
   640×480 und auf 1920×1080 gleich aufgeraeumt aus, ohne den Text zu
   skalieren. Ob der Schirm ausserhalb des Kastens weiss bleibt oder das
   Desktop-Muster traegt, entscheidet der Anwender am Bild (Vorschlag: weiss
   lassen, wie EmuTOS' Startschirm).
2. **Der USB-Block ist linksbuendig** (`tx = fx0 + 40`, `:259`), alles andere
   zentriert auf `cx = sw / 2` (`:192`).
3. **Alle vier USB-Geraete melden „Port 1"** (`:277`, `d.port`). Bei einem Hub
   waere das der Downstream-Port; vier Geraete auf demselben ist verdaechtig.
   Gegen die enumerierte Topologie pruefen (`kernel/src/drivers/usb/`), ggf.
   Hub-Adresse mit ausgeben.
4. **Tippfehler**: „This operating system isnt Original" (`:222`) → „isn't".
5. Die beiden *Hold*-Zeilen: „Hold \<Shift\> to pause this screen" ist
   invertiert, „Hold \<Control\> to skip AUTO/ACC" nicht (`:307-317`) —
   pruefen, ob beabsichtigt; sonst vereinheitlichen.

## Abnahme

Screendump in QEMU (640×480) **und** auf der Pi (1920×1080): der Kasten
steht in beiden Faellen mittig und umschliesst den Inhalt buendig, alle
Zeilen gleich ausgerichtet, Portnummern belegt, Text fehlerfrei. Die
Geraeteliste waechst mit der Zahl der Geraete — Kastenhoehe muss mitwachsen,
ohne den Schirm zu verlassen (Grenzfall: viele USB-Geraete bei 640×480,
dann Liste kuerzen oder Kasten am Schirm klemmen; im Ticketabschluss
benennen, was gewaehlt wurde).

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Sichtbefunde erst am Pixel belegen,
dann benennen.
