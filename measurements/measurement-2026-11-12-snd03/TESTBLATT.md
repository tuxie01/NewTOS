# Testblatt SND03 — Ton-Hardwaresonde (Klinke, DMA, PWM-Takt)

Karte: Kernel `kerns03.img` (= `kernel8.img`, Bau mit `--features snd-probe`), sonst nichts geaendert.
Pi 4 mit HDMI-Monitor und USB-Tastatur. **Kopfhoerer oder Lautsprecher an der 3,5-mm-Klinke.**

**Lautstaerke:** Der Ton ist absichtlich leise (Rechteck, 1/16 des Aussteuerbereichs um die Mitte), aber
ein Rechteck klingt hart. Lautsprecher leise stellen; Kopfhoerer zuerst nicht aufsetzen, sondern in der Hand
halten, und erst aufsetzen, wenn der erste Ton angenehm ist.

Fotos: Handy-Foto des Bildschirms genuegt. Die Sonde zeigt drei Seiten; jede bleibt stehen, bis eine Taste
gedrueckt wird.

## Ablauf

1. Pi einschalten. Begruessungsbild wie gewohnt, danach wird der Bildschirm geloescht und es erscheint
   `[snd03] ===== SND03 sound hardware probe (rpi4), page 1 of 3 =====`.
2. **Seite 1 (H4):** System-Timer und DMA-Zustand. Am Ende steht `----- page 1 of 3 (H4) done ...`.
   Foto. Eine Taste druecken (z. B. Leertaste).
3. **Seite 2 (H3, H5, Takt aus Sicht der Firmware):** DMA-Kopie zweimal, danach `H1 firmware view of
   clock id PWM`. Foto. Taste.
4. **Seite 3 (H2, H1, Ton):** zuerst zwei bis drei Zeilen `H2 ...` und `H1 timing ...` (etwa 1 s). Wenn
   dort `clk_pwm NOT running` steht, folgt `H1 asking the firmware ...` und eine zweite Messung.
   Dann, wenn ein Takt gemessen wurde, die Toene, je mit einer Zeile `TONE ...` auf dem Bildschirm:

   | Ton | Dauer | erwartet | bitte notieren |
   |---|---|---|---|
   | T1 | 2 s | 440 Hz (Kammerton a) auf beiden Ohren | gehoert? Rauschen, Brummen, Pfeifen dabei? |
   | T2 | 1,5 s | 440 Hz, nur ein Kanal ("left only (GPIO 40)") | **auf welchem Ohr** (links/rechts)? |
   | T3 | 1,5 s | 880 Hz (hoeher), nur der andere Kanal ("right only (GPIO 41)") | **auf welchem Ohr**? |
   | T4 | 2 s | wie T1, andere Betriebsart ("mark/space mode") | klingt T4 anders als T1 (lauter, leiser, sauberer, rauer)? |

   Zwischen den Toenen je 0,7 s Pause; ein Knacken am Anfang und Ende jedes Tons ist erwartet.
   Danach `TONE done ...` und eine Zeile `SUMMARY ...`. Foto der ganzen Seite 3. Taste.
5. Der Pi startet weiter bis zum Desktop. Danach wird der alte Kernel wieder auf die Karte gespielt (Lead).

## Was zaehlt

PASS (Sonde): alle drei Seiten erscheinen, die Fotos sind lesbar.
PASS (Ton, Abnahme SND03): mindestens T1 ist hoerbar.
Bitte immer berichten: Ohr von T2 und T3, Unterschied T1/T4, Nebengeraeusche.

FAIL / bitte melden mit Foto:
* Der Pi bleibt vor Seite 1 stehen (schwarz oder nur Begruessungsbild) oder mitten auf einer Seite.
* Seite 3 sagt `TONE skipped` (kein PWM-Takt): dann ist das Ergebnis die Meldung selbst, Foto genuegt.
* Kein Ton, obwohl `TONE T1 ... played` dasteht.
* Eine Taste wirkt nicht (dann Pi ausschalten; die Fotos bis dahin reichen).

## Nicht noetig

Kein serieller Adapter (mit `serial=on` in `cmdline.txt` kaeme derselbe Text zusaetzlich auf GPIO 14/15).
Die Sonde schreibt nichts auf die Karte.
