# DESK23 — Zwei Fensterrahmen-Eckpunkte bleiben bei `wind_open` ungeschrieben

Status: OBSOLETE (2026-11-11) | Priority: P4 | Reason: withdrawn 2026-09-24: EmuTOS-conformant, both drawings are right

Migrated (owner batch 7, 2026-11-11, to OBSOLETE), evidence: TASK/DESK23.md status line (ZURUECKGEZOGEN (2026-09-24, EmuTOS-konform)), TASK/DESK23-STATUS-DONE.MD exists

Line 3 before the migration:
Status: **ZURUECKGEZOGEN** (2026-09-24, EmuTOS-konform) · Prioritaet: **P4** · Typ: Defekt — Nebenbefund DESK06

Abhaengigkeiten: —

## Sachverhalt

Ein Fenster, das `wind_open` inkrementell zeichnet, und dasselbe Fenster nach
einer Ganzschirm-Neuzeichnung unterscheiden sich in genau **zwei** Punkten:
(593, 43) und (48, 444) bei 640x480 — die beiden Eckpunkte des Rahmens,
inkrementell **gruen** (Hintergrund), nach der Neuzeichnung **weiss**.

Das ist gemessen und gegen einen Kontrolllauf abgegrenzt: derselbe
Unterschied tritt ueber den `.NCP`-Startpfad auf, der nur dieselbe
`redraw::redraw_desktop` ruft, also gehoert er der Rahmenzeichnung und nicht
dem Programmstart (`measurements/measurement-2026-09-23-desk06/`, Laeufe `wdef` und
`redraw`).

Zu klaeren ist, welche der beiden Zeichnungen recht hat — EmuTOS' `w_draw`/
`draw_frame` gegen unsere — und die andere danach zu richten.

## Arbeit

Erst die Quelle: welche Pixel schreibt EmuTOS' Rahmenzeichnung in den zwei
Ecken. Dann die falsche Seite angleichen; zwei Screendumps (inkrementell und
neu gezeichnet) muessen danach pixelgleich sein.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Abschluss (2026-09-24)

**Zurueckgezogen — beide Zeichnungen haben recht, und es sind vier Punkte,
nicht zwei.** Kein Patch.

Fuer ein Fenster `(48,43,544,400)` mit Randdicke 1 zeichnet `ob_draw` den Rand
mit `gr_box(t.g_x,t.g_y,t.g_w,t.g_h,1)` (x 48..591, y 43..442) und die
`SHADOWED`-Dekoration **nach** `gr_inside(&t,th)`
(`aes/gemoblib.c:889-900`): unteres Band x 49..591 / y 443..444, rechtes Band
x 592..593 / y 44..444. Jedes Band beginnt einen Punkt **innerhalb** der
zugehoerigen Kante — vier Punkte gehoeren weder zum Rand noch zu einem Band:
`(592,43)`, `(593,43)`, `(48,443)`, `(48,444)`. Unser
`draw_frame`/`draw_shadow` rechnet dieselben Rechtecke aus.

Der Desktop malt sie auch nicht: alle vier liegen **innerhalb** des `WS_TRUE`
des Fensters (`gemwmlib.c:253-261`), die Rechtecklisten entstehen aus
`WS_TRUE` (`newrect`, `gemwrect.c:190`), und `w_drawdesk` laeuft die Liste des
Desktops ab (`do_walk(DESKWH, …)`, `gemwmlib.c:340-359`). Ihre Farbe ist also
schlicht das, was vorher auf dem Schirm stand — bei EmuTOS wie hier.

Genau das war der gemessene Unterschied: unter dem inkrementellen Bild lag der
Desktop, unter dem neu gezeichneten der weisse Schirm, den das Konsolenprogramm
hinterlassen hatte. Der Desktophintergrund ist ein Gruen/Weiss-Raster, weshalb
nur zwei der vier Punkte im Diff auftauchten — bei `(592,43)` und `(48,443)`
ist das Raster ohnehin weiss.

* Beleg: `measurements/measurement-2026-09-24-desk22-desk23-desk28-desk29/` §4, Tabelle aus
  `method/pixels.py` ueber alle 22 Screendumps. Der Kontrolllauf `d29` —
  fuenf Fensteroeffnungen, Toppings und Ganzschirm-Rahmenzeichnungen ohne
  jedes Loeschen — laesst alle vier Punkte von Anfang bis Ende auf dem
  Hintergrundmuster.
* Spezifikation: `docs/api/aes/window-graphics.md`, Abschnitt *NewTOS
  implementation notes — the drop shadow's four unwritten pixels (DESK23)*.
* Offen: Hardwarebestaetigung, Punkt **57** in
  `docs/api/desktop/_hw-test-2026-09-17.md` (nur ein Befund, wenn **mehr** als
  diese vier Punkte betroffen sind).
