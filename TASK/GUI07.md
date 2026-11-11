# GUI07 — `mem_to_screen` liest je Pixel den Bildschirm und rechnet um, auch wenn es das Ergebnis wegwirft

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt GUI07-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 20, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: e1cb3eabd (perf(vdi): vro_cpyfm copies row-wise and skips the screen re) added the receipt TASK/GUI07-STATUS-DONE.MD itself, the report of this research or measurement ticket (221 files), bookkeeping or bulk commit, not accepted as evidence

Line 3 before the migration:
Status: **ERLEDIGT** (2026-10-03, [Statusdatei](GUI07-STATUS-DONE.MD); vorgezogen; **Pi 2026-10-03: Doom x3/x4 40–75 fps** statt 7,6/4,2) · Prioritaet: **P2** · Typ: Durchsatz — aus [PORT07](PORT07.md), berichtet statt behoben

Abhaengigkeiten: — · **nicht vor dem OFAM-Termin** (kein Kerneleingriff in den Zeichenpfad)

## Warum das jetzt zaehlt

PORT07 hat die Ball-Demo auf den konformen Weg gebracht und dabei den Preis
gemessen: **80,49 ns je Pixel** im `vro_cpyfm`, gegen 4,38 ns auf dem alten
Direktweg. Fuer [PORT06](PORT06.md) (Doom) bedeutet das **5,15 ms je Bild**
allein fuer die Kopie — 18 % eines 35-Hz-Taktes. Jede Anwendung, die ihr Bild
konform auf den Schirm bringt, zahlt diesen Preis.

**Und genau das ist das Argument dafuer, ihn zu senken statt den konformen Weg
zu meiden:** wer ueber die VDI geht, erbt jede Verbesserung — wer daran vorbei
arbeitet, nicht ([PORT06](PORT06.md) §„Das staerkere Argument").

## Die zwei Befunde

**V-a — eine Leseoperation und eine Umrechnung je Pixel, umsonst.**
`tos-api/src/vdi/raster.rs:494-533` (`mem_to_screen`) liest fuer **jedes**
Pixel den lebenden Bildschirmpunkt und rechnet ihn nach RGB565 — auch bei
`S_ONLY` (Modus 3), wo `raster_op(3, s, d) = s` das Ergebnis **wegwirft**.
`S_ONLY` ist der haeufigste Kopiermodus ueberhaupt.

**V-b — zwei virtuelle Aufrufe je Pixel.** Dieselbe Schleife laeuft ueber
`&dyn RasterBackend`: `get_device_pixel`/`put_device_pixel` je Pixel, und
Basisadresse sowie Zeilenabstand werden je Pixel neu aus `active_fb()` geholt
statt einmal je Zeile.

Beides ist **Durchsatz, nicht Verhalten** — kein Bild wird falsch. Deshalb
wurde es berichtet und nicht behoben.

## Der Messstand liegt bereit

Die konforme Ball-Demo **ist** der Pruefstand: sie meldet `ns/px` im Blit und
die freilaufende Bildzeit. Vorher/Nachher ist damit eine Zahl, kein Eindruck.
Verfahren in `measurements/measurement-2026-09-28-port07/method/`.

## Auftrag

1. `S_ONLY` (und die anderen Modi, die `d` nicht lesen) **ohne** Bildschirmlesen
   bedienen — welche das sind, steht in der Wahrheitstabelle der 16
   Rasteroperationen und ist abzuleiten, nicht zu raten.
2. Die Schleife auf zeilenweise Arbeit bringen: Basis und Zeilenabstand einmal
   je Zeile, nicht je Pixel.
3. **Kein Verhalten aendern.** Der Beleg ist die Raster-Referenzsuite
   (`tos-api/tests/vdi_raster_reference/`, 59 Faelle gegen echte TT- und
   Falcon-Dumps) — sie muss **unveraendert** gruen bleiben, sonst ist es kein
   Durchsatzfix, sondern ein Verhaltensfehler.

## Die Staffelung bis zum Assembler — Anwenderfrage 2026-09-28

Frage war: was kostet es, die Blit-Routine in optimiertem Assembler zu bauen?
Antwort: **Assembler ist der letzte Schritt und wahrscheinlich der kleinste
Gewinn.** Die Reihenfolge ist wichtiger als die Technik.

**Null — messen.** Die **80,49 ns/px sind eine QEMU-Zahl**. `vro_cpyfm` ist
auf echter Hardware **nie** gemessen worden, und TCG hat sich bei der
Cachepflege gerade um Faktor 8 vertan ([GUI06](GUI06.md)). Ein Pi-Lauf der
konformen Ball-Demo liefert `ns/px` auf einem A72 — **ohne diese Zahl ist
alles Weitere Raten**.

**Eins — dieses Ticket (reines Rust).** Den Bildschirmlesezugriff weglassen,
wo das Ergebnis verworfen wird; Basis und Zeilenabstand einmal je Zeile.
Klein, begrenzt, und **vermutlich der groesste Einzelfaktor** der Kette.

**Zwei — Schnellpfade je Rasteroperation, auch Rust.** Reines Kopieren ist
zeilenweise ein `memcpy`; dafuer erzeugt LLVM bereits gute Befehlsfolgen. Von
den sechzehn Operationen sind die meisten Wort-Logik, die der Uebersetzer oft
selbst vektorisiert, sobald die Schleife sauber ist.

**Drei — NEON als Intrinsics in Rust** (`core::arch::aarch64`), **nicht** als
`.S`-Datei: vier Pixel je 128-Bit-Register, und es bleibt Code, den der
Uebersetzer gegenliest. Deutlich weniger Aufwand und Risiko als
handgeschriebener Assembler, bei praktisch demselben Ergebnis.

**Vier — echtes Assembler** nur dort, wo Schritt drei nachweislich nicht
liefert. Erwartung: ein bis zwei Operationen, nicht der Blit als Ganzes.

### Warum unser Fall einfacher ist als EmuTOS'

EmuTOS' Assembler-Blit existiert wegen der **Bitplanes** — Bits ueber
Wortgrenzen zwischen vier Ebenen schieben, mit Randmasken. Das ist auf einem
m68k echte Arbeit. **Wir sind chunky 32 Bit:** ein Pixel ist ein Wort, kein
Verschieben, keine Ebenenverzahnung. **Der Grund, aus dem EmuTOS Assembler
schreibt, existiert bei uns nicht.**

### Was es beherrschbar macht

Die Aenderung ist **backend-lokal** — dafuer war die `RasterBackend`-
Schnittstelle so geschnitten ([[vdi-draw-asm-optimization-later]]). Und es
gibt den Waechter, der das Ganze ueberhaupt wagbar macht: die **59 Faelle der
Referenzsuite gegen echte TT- und Falcon-Dumps**. Bleiben die bitgleich, ist
die Umschreibung nachweislich verhaltensneutral. **Ohne diesen Waechter waere
von handgeschriebenem Assembler im Zeichenpfad abzuraten.**

## Abnahme

`ns/px` der Ball-Demo vorher/nachher auf derselben Maschine; die 59 Faelle der
Referenzsuite unveraendert; beide Boards bildpunktgleich.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
