# GUI06 — Der Boden der Geste: zwei Vollbild-Neuzeichnen und 400–600 MB Cachepflege

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt GUI06-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: only the commit 4ef536366 (perf(desk): a drop redraws its window, not the who) that added the receipt itself

Line 3 before the migration:
Status: **BEHOBEN — Pi-Abnahme offen** (2026-09-28) · Prioritaet: **P2** · Typ: Belegter Defekt / Laufzeit — aus der FS04-Messung herausgeloest

Abhaengigkeiten: — (unabhaengig von [FS04](FS04.md); dieselbe Messung, andere Ursache)

## Der Befund, gemessen

Pi 4, **1920x1080**, Sonde aus `measurements/measurement-2026-09-25-fs04/`, Bericht
`raw/pi04-FS04.TXT`. Sobald die Karte warm ist, kostet ein Fallenlassen im
Desktop noch **267–340 ms**. Davon:

| | |
|---|---|
| Kopie (GEMDOS gesamt) | 14–87 ms |
| **2x `redraw_desktop`** | **219–234 ms** |

**Rund 85 % der verbleibenden Zeit sind die zwei Vollbild-Neuzeichnen**, je
~110 ms. Das ist der Boden, unter den die Geste nicht kommt, gleich wie warm
die Karte ist. In QEMU bei 640x480 kostete dasselbe 3,13 ms je Aufruf — die
gemessene Skalierung auf 1920x1080 ist x6,19, und der Rest ist die echte
Pixelzahl.

## Die Cachepflege daneben

Je Fallenlassen: `fb.clean` **2 176–3 647 Aufrufe ueber 313–618 MB**,
groesster Einzelaufruf **3 486 720 Byte** fuer ein Primitiv. Der Bildschirm
selbst hat 1920x1080x4 = **8,3 MB**. Es wird also das **40- bis 75-fache** des
Bildschirms durch die Cachepflege geschickt.

Lesbare Ursache: `kernel/src/vdi_backend.rs:284-305` pflegt die
**einschliessende Adressspanne** eines Primitivs statt der beruehrten Zeilen.
Ein schmales, hohes Rechteck bezahlt damit die volle Breite mal Hoehe. In
QEMUs TCG ist `dc cvac` kostenlos, auf einem Cortex-A72 nicht.

## Zwei Fragen, getrennt zu beantworten

1. **Warum zwei Vollbild-Neuzeichnen?** Ein Fallenlassen zeichnet den Desktop
   zweimal komplett neu (`desk.redraw_desk n=2` in **jedem** der 14 Bloecke).
   Gegen EmuTOS zu halten: `deskfun.c`/`deskdir.c` nach dem Kopieren —
   wieviele Neuzeichnen loest EmuTOS aus, und ueber welchen Bereich? Ein
   Vollbild-Neuzeichnen fuer eine geaenderte Verzeichnisliste ist zu pruefen,
   nicht anzunehmen.
2. **Warum die einschliessende Spanne?** `vdi_backend.rs:284-305` gegen die
   tatsaechlich geschriebenen Zeilen. Zeilenweise Pflege ist die naheliegende
   Form; ob sie hier richtig ist, entscheidet die Schreibgeometrie des
   jeweiligen Primitivs.

## Auftrag

Erst messen, welcher der beiden Anteile wieviel der ~230 ms traegt (die Sonde
hat beide Zaehler bereits: `desk.redraw_desk` und `fb.clean`/`fb.clean_lines`),
dann je Anteil die Ursache belegen. Kein Fix vor der Zuordnung.

## Warum P2

Der Oktober-Termin verlangt, dass nichts haengt. Eine Vierteilsekunde je
Mausgeste ist kein Haenger, faellt aber bei jedem Klick auf — und sie liegt
nicht an der Karte, laesst sich also unabhaengig von FS04 beheben.

## Abnahme

Dieselbe Geste, gemessene Zeit vorher/nachher aus derselben Sonde, und die
Zuordnung der Ersparnis auf die beiden Anteile.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-28)

### Schritt 1 — die Zuordnung, vor jedem Fix, und **ohne neuen Hardwarelauf**

`measurements/measurement-2026-09-25-fs04/raw/pi04-FS04.TXT` ist selbst ein natuerliches
Experiment: ueber seine 13 Bloecke schwankt die Zahl der `dc cvac` um Faktor
**1,98**, die Neuzeichenzeit um **1,07** — und in die falsche Richtung.

| | |
|---|---|
| Block 7 | 4 892 879 Cachezeilen → 2x redraw = 226 917 us |
| Block 10 | 9 667 213 Cachezeilen → 2x redraw = 227 676 us |
| | **+98 % Cachepflege → +0,3 % Neuzeichenzeit** |

Regression ueber die Bloecke 3–14: Neuzeichenzeit gegen `fb.clean_lines`
**−1,50 ns/Zeile** (r = −0,48, **negativ**); der Rest gegen `fb.clean_lines`
**+1,32 ns/Zeile** (r = +0,56) — das ist der echte Grenzpreis einer `dc cvac`
auf der Pi 4, rund **zwei Zyklen**, also der Preis fuer eine **nicht
schmutzige** Zeile. Und das sind fast alle: der Bildschirm hat 129 600
Cachezeilen, gepflegt werden 4,9–9,7 Millionen. **Die 313–618 MB des Tickets
sind echt, aber es sind fast nur Nachschlaege.**

| Anteil | an den ~230 ms |
|---|---|
| **Zwei Vollbild-Neuzeichnen** | **≥ 95 %** (208–225 ms) |
| **Spannenpflege** | **≤ 5 %** (6–13 ms) |

Damit gehoert der Fix an die **Zahl der Neuzeichnen**.

### Schritt 2 — die Ursache, belegt

**EmuTOS loest nach einem Ziehen-Kopieren null Vollbild-Neuzeichnen aus**,
sondern genau ein `WM_REDRAW` je neu aufgebautem Fenster ueber **dessen
eigenes Rechteck** (`deskfun.c:133-143`; `fun_msg` `:100-113` ruft sofort
`hndl_msg`, dessen `WM_REDRAW`-Arm `do_wredraw` ist, `deskmain.c:1268-1272`).
Die vier Arme: `fun_win2desk` `:1129-1130`, `fun_win2win` `:986-999`,
`fun_desk2win` `:1253-1254`, `fun_desk2desk` `:1260-1298` (gar keines). Der
vom Kasten verdeckte Bereich ist da laengst von `end_dialog` wiederhergestellt
(`deskinf.c:363-378`) — diese Kette gibt es bei uns vollstaendig.

NewTOS rief stattdessen zweimal `redraw_desktop()`. **Der Doc-Kommentar von
`rebuild_window` zitierte die richtige EmuTOS-Zeile und tat dann etwas
anderes** — dasselbe Muster wie bei den drei Kopien in U1.

Fuer Anteil 2 wurde die Schreibgeometrie **aller acht** `clean`-Aufrufer
geprueft: keiner schreibt ausserhalb seines Rechtecks, zeilenweise Pflege
deckt also jedes geschriebene Byte ab. Ein 2 px breites, 401 Zeilen hohes
Primitiv kostete als Spanne 3 072 008 Byte statt 3 208.

### Gemessen, drei Staende, damit die Ersparnis aufteilbar ist

| | vorher | nur Anteil 2 | nachher |
|---|---|---|---|
| **Wandzeit** | **23 825 us** | 13 000 us | **8 598 us** |
| `desk.redraw_desk` | n=2, 14 094 us | n=2, 6 238 us | **n=0** |
| `fb.clean_lines` | 1 362 715 | 106 321 | **44 839** |

Bei 1920×1080 je Neuzeichnen **256 202 → 134 135** Cachezeilen (−47,6 %); mit
offenem Fenster — der Lage der Pi — **−92,2 %**.

**Bildpunktgleichheit** (sha256): Geste und Startbild beider Boards identisch,
0 `[EXC]`/`[PANIC]`, `fsck_msdos -n` sauber.

### Ehrlich zur Uebertragbarkeit

**QEMU ist fuer Anteil 2 kein Massstab und uebertreibt ihn um Faktor 8:**
10,9 ns je `dc cvac` in TCG gegen **1,32 ns** auf der Pi. Die dortige
Halbierung der Neuzeichenzeit durch den Zeilen-Fix wird auf der Pi **nicht**
eintreten.

**Vorhersage fuer die Pi** (Rechnung, nicht Messung): zwei `redraw_desktop` à
110 ms weg, ein `do_wredraw` à ~17 ms dazu. Block 7: 240 → **~30 ms**;
Block 14: 267 → **~63 ms**; Block 10: 340 → **~129 ms** (davon 87 ms Karte).
**Aus 267–340 ms werden 30–130 ms**, und der Rest ist dann die Karte. Sichtbar:
das Fenster erscheint mit neuer Liste, ohne dass der ganze Bildschirm
durchblitzt.

### Herausgefallen

* **[DESK48](DESK48.md)** — vier weitere `redraw_desktop()`-Stellen, bewusst
  stehen gelassen; eine davon liegt auf dem Klickpfad **jedes**
  Fensterwechsels.
* **[HW07](HW07.md)** — die Bloecke 1–2 jedes Pi-Laufs zeichnen 2,5-mal
  schneller als ab Block 3, bei gleicher Arbeit. Verhaeltnis 2,50 bzw. 2,49 in
  **beiden** Laeufen; die ARM-Takte der Pi 4 verhalten sich wie 1500/600 = 2,50.
