# PORT07 — Die Ball-Demo auf der Pi konform machen (und damit Dooms Bildweg beweisen)

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt PORT07-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 83d3003e2 (fix(vdi): the device pixel carries the Falcon word), named by receipt-named

Line 3 before the migration:
Status: **ERLEDIGT — ein Pi-Lauf offen** (2026-09-28) · Prioritaet: **P2** · Typ: Portierung / Konformitaet — Anwenderwunsch 2026-09-28

Abhaengigkeiten: — · **Vorbedingung im Sinne eines Beweises** fuer [PORT06](PORT06.md)

## Der Befund

Die Ball-Demo liegt **ausserhalb des Repos** in
`/Volumes/Daten/Development/ball-demo` ([[ball-demo-as-benchmark]]) und hat je
Maschine eine Plattformdatei. Gezaehlt am 2026-09-28:

| Datei | `vro_cpyfm` | `Setscreen` | `Physbase` | `Logbase` | `vs_color` |
|---|---|---|---|---|---|
| **`plat-newtos.c`** | **0** | **8** | **1** | **7** | **0** |
| `plat-tt.c` | 5 | 0 | 1 | 0 | 3 |
| `plat-pistorm.c` | 6 | 0 | 0 | 0 | 5 |

**Unsere Plattformdatei ist die einzige nicht-konforme.** Sie baut den
Doppelpuffer ausschliesslich ueber `Setscreen`/`Logbase`/`Physbase` und benutzt
**kein** `vro_cpyfm` und **kein** `vs_color`. TT und PiStorm gehen den
konformen Weg.

Der Anwender dazu: die Vorgabe war Konformitaet, und hier ist sie verfehlt
worden.

## Warum es sich dreifach lohnt

1. **Es behebt die Abweichung.** Was ueber `Physbase` geht, laeuft nur dort, wo
   ein Shifter sitzt — nicht auf einer Nova, nicht auf einer Grafikkarte, und
   es bekommt keine Hardwarebeschleunigung
   ([PORT06](PORT06.md) §„Das staerkere Argument").
2. **Es beweist Dooms Bildweg empirisch.** PORT06s Entwurf steht auf
   „Truecolor-Speicher-MFDB → `vro_cpyfm`". Laeuft die Ball-Demo so auf der Pi,
   ist das keine Annahme mehr. **Das ist der eigentliche Grund, es vor PORT06
   zu machen.**
3. **Es macht die Demo vergleichbar.** Ihr Zweck war ein kontrolliertes
   Experiment je Maschine — TT gegen PiStorm gegen Pi. Solange die Pi einen
   anderen Weg nimmt, vergleicht die Messung Aepfel mit Birnen.

## Die Arbeit

**Kein Forschungsprojekt: `plat-newtos.c` ist 9,5 KB, und zwei fertige konforme
Fassungen derselben Plattformschicht liegen direkt daneben.** `plat-tt.c` und
`plat-pistorm.c` sind die Vorlage.

Zu beachten: unser Bildschirm ist **Truecolor** (`PLANES = 16`,
`kernel/src/vdi_backend.rs:43`), TT und PiStorm sind es nicht. Der Puffer ist
also in unserer Pixelform anzulegen, nicht in 8 Bit — genau die Frage, die
PORT06 §3a fuer Doom entschieden hat. `vs_color` entfaellt dort entsprechend
oder wird zur Umsetztabelle.

**Kein Kerneleingriff.** Faellt etwas an der VDI auf, wird es **berichtet**,
nicht behoben — elf Tage vor dem Termin wird der Zeichenpfad nicht angefasst.

## Abnahme

* Die Demo laeuft auf der Pi, sichtbar, ohne `Setscreen`/`Physbase`/`Logbase` —
  `/usr/bin/grep` ueber `plat-newtos.c` findet keines der drei.
* **Bildrate vorher und nachher**, auf derselben Maschine gemessen. Wird sie
  schlechter, ist das ein Befund und kein Grund zurueckzurudern — dann wissen
  wir, was unser `vro_cpyfm` kostet, und genau das braucht PORT06.
* Der Quelltext der Demo (`ball.c`, `ball_neu.c`, `ball-tex.c`) bleibt
  **unangetastet**; geaendert wird nur die Plattformdatei.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Die Demo liegt ausserhalb des Repos —
Aenderungen dort sind **nicht** Teil eines NewTOS-Commits; der Messbericht
schon.

## Abschluss (2026-09-28)

`plat-newtos.c` geht jetzt denselben Weg wie `plat-tt.c` und `plat-pistorm.c`:
Speicher-MFDB plus **ein `vro_cpyfm` je Rechteck**, kein XBIOS 2/3/5, kein
Framebuffer-Zeiger. **Am NewTOS-Baum ist keine Quelltextzeile geaendert**
(`git diff --stat` leer, Basislinien alle unveraendert). `ball.c`,
`ball_neu.c`, `ball-tex.c` unangetastet.

### Der Befund, der zuerst kam: die Demo war seit dem 21. September kaputt

**Der vorgefundene Stand startet auf `b9a2444` ueberhaupt nicht**, frisch
uebersetzt wie auch das ausgelieferte Binaerbild vom 2026-09-09:

```
BALL: could not measure the screen pitch
```

Ursache per `git log -S` belegt: **`2d6b3cc`** (2026-09-21, „the device pixel
carries the Falcon word's overlay bit") nahm `hal/framebuffer.rs` das
unbedingte `0xFF00_0000` aus `rgb()`. Die Zeilenbreitenmessung der Demo suchte
„das erste Pixel, das die VDI geaendert hat", begruendet mit *„every VDI pen is
opaque (alpha 0xFF)"*. Stift 1 ist Schwarz, und Schwarz ist seither das
Nullwort — die Suche kann nicht mehr anschlagen.

**An der zugesagten Schnittstelle bewegte sich nichts; nur an einem Interna,
von dem die Demo nie haette abhaengen duerfen.** Das ist PORT06s
Konformitaetsargument, unfreiwillig belegt. Fuer den Vorher-Arm war eine
einzeilige Reparatur in einer Scratch-Kopie noetig (Stift 2 statt 1), sonst
gaebe es keine Vorher-Zahl.

### Die Zahlen (QEMU `raspi4b`, 640x480, 3 Vorher- / 2 Nachher-Laeufe)

```
freilaufendes Bild   vorher 0,242 ms   nachher 1,303 ms   Faktor 5,39
Pixel im Blit        vorher  4,38 ns   nachher 80,49 ns   Faktor 18,4
dargeboten           60,00 fps beide; 4 verworfen/22 500 vs 1/16 500
Farbtiefe            159 Farben (158 nicht 5-Bit) -> 116 Farben (0 nicht 5-Bit)
```

Der Blit waechst von 28 % auf 86 % des freilaufenden Bildes. **Auf dem Schirm
sieht man nichts davon** — Vsync deckelt beides bei 60 fps, die Abzuege nach
430 s zeigen dieselbe Szene, keine Reste.

### Urteil fuer [PORT06](PORT06.md)

**Dooms Bildweg ist belegt, nicht mehr angenommen.** Zwei Praezisierungen
fuer dessen §3a, beide gemessen:

1. **„Truecolor" heisst hier 15 Bit (5-5-5), nicht 24** — das VDI-Gerätewort
   ist der Falcon-Pixel. Von 159 Farben im Vorher-Bild sind 158 nicht in 5 Bit
   darstellbar, von 116 im Nachher-Bild **keine einzige**.
2. **Preis je Bild:** 320x200 = 64 000 px x 80,49 ns = **5,15 ms allein im
   `vro_cpyfm`** (18 % eines 35-Hz-Taktes). Der Umrechnungsdurchlauf, den §3a
   mit „unter einer Millisekunde" veranschlagt, ist der **billige** Teil.
   Teuer ist die Kopie.

### Der eine offene Punkt — und er kostet genau einen Lauf

**1920x1080 auf der echten Pi.** QEMU meldet 640x480; dort sind es ~134 000
statt ~14 300 Pixel je Bild. Ob das im 16,7-ms-Budget bleibt, sagt nur die Pi.
Die Hochrechnung (≈ 4,1 ms/Bild) steht in `NUMBERS.TXT` §5.2 und ist
**ausdruecklich keine Messung** — ein Store-Loop und ein Pfad aus zwei
virtuellen Aufrufen plus Cachepflege sind nicht dieselbe Art Arbeit, und QEMU
hat kein Cachemodell.

Ebenfalls nur auf der Pi: der saubere Ausstieg (`raspi4b` hat keine Tastatur),
also Abschlusszusammenfassung, Palettenwiederherstellung und
`form_dial(FMD_FINISH)`.

### Herausgefallen

**[GUI07](GUI07.md)** — zwei Durchsatzbefunde in `vdi/raster.rs`, berichtet und
**nicht** behoben (kein Kerneleingriff vor dem Termin).

## Die Hochrechnung ist gemessen — und sie lag um Faktor 2,4 daneben

Pi-Lauf vom 2026-09-29 (Kern `9251440d`, 1920x1080),
`measurements/measurement-2026-09-29-sched01/pi-BALL.LOG`:

```
blit 75.08 ns/px + 1.41 us/call
je Bild:  step 0.02 ms  compose 0.76 ms  blit 9.93 ms  wait 5.93 ms
ganzer Lauf: 126 459 px in 2,05 Rechtecken je Bild = 6,10 % des Schirms
```

**`NUMBERS.TXT` §5.2 schaetzte ≈ 4,1 ms je Bild; gemessen sind es 9,93 ms.**
Die Schaetzung war ausdruecklich als *keine Messung* gekennzeichnet — zu Recht.
Die **Pixelzahl** war dagegen gut getroffen (geschaetzt ~134 000, gemessen
126 460).

Die Bildzeit bleibt trotzdem im Rahmen: 9,93 ms Blit plus 0,78 ms Rechnen
gegen ein 16,7-ms-Budget bei 60 Hz, der Rest ist Warten.

**Der Pixelpreis ist ueber drei Laeufe stabil:** 73,41 / 73,41 / **75,08** ns.
Der dritte Lauf enthaelt [GUI06](GUI06.md) — die zeilenweise Cachepflege hat
den Blit also **nicht** verbilligt, was zu GUI06s eigenem Befund passt
(Cachepflege war ≤ 5 % der Geste, nicht des Blits).
