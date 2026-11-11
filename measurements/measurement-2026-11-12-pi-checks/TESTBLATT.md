# Testblatt Pi-Checks 2026-11-12 (BIOS06, BIOS11, BIOS09)

Karte: neuer Kernel (kern1112.img = kernel8.img) und Ordner `C:\PITEST\` mit vier Programmen
(GNPROBE, GNSUPER, KPROBE, AWPROBE). Pi mit HDMI-Monitor und USB-Tastatur. Start: Desktop erscheint,
Laufwerk C oeffnen, Ordner PITEST oeffnen, Programm per Doppelklick starten. Nach jedem Test den Pi
neu starten (Strom aus/ein oder Ctrl+Alt+Entf), damit jeder Test mit frischem System beginnt.
Fotos: Handy-Foto des Bildschirms genuegt.

## 1. BIOS06 - Getmpb(NULL) beendet das Programm, Absturzbild, echte Taste

A) GNPROBE.NAP doppelklicken.
   Erwartet: Absturzbild mit dem Text "Bus Error" (nicht: Pi haengt, schwarzer Bildschirm, Neustart).
   Foto vom Absturzbild machen.
   Dann EINE beliebige Taste auf der USB-Tastatur druecken (z. B. Leertaste).
   Erwartet: zurueck auf dem Desktop, Maus und Fenster funktionieren weiter.
B) Nach dem Zurueckkehren (kein Neustart noetig) GNSUPER.NAP doppelklicken (derselbe Fehler, aber im
   Supervisor-Zustand; reiner Messversuch).
   Erwartet: wieder "Bus Error", Taste, zurueck auf den Desktop. Foto.
PASS: A zeigt "Bus Error", die Taste bringt den Desktop zurueck, der Desktop reagiert danach.
FAIL: kein Absturzbild, Taste wirkt nicht, Desktop bleibt tot, Pi startet neu. Foto machen, Wortlaut notieren.
GNSUPER ist nur Zusatzinfo: sein Ergebnis zaehlt nicht fuer PASS/FAIL von A, aber berichten.

## 2. BIOS11 - Bconin(2) liefert den Shift-Zustand zum Tastendruck

KPROBE.NAP doppelklicken. Der Bildschirm zeigt "KEYREADY". Dann sofort (innerhalb 30 s, die
Pausen zwischen den Tasten unter 2 s) diese sechs Eingaben nacheinander, Tasten jeweils wieder loslassen:
  1. a
  2. Shift+a
  3. F1
  4. Strg+c
  5. Return
  6. Shift+b   (linke Shift-Taste halten, b druecken, beides loslassen)
Dann etwa 2 s nichts tun; "KPROBE done" erscheint, der Desktop kehrt zurueck.
Ergebnis steht in der Datei KPROBE.LOG (im Ordner PITEST, sonst im Wurzelverzeichnis von C:).
Mit der Maus oeffnen (QED, Doppelklick auf die Datei) und das Bild fotografieren, oder die Datei
vom Karten-Mac lesen lassen. Erwartete Zeilen (wie QEMU und EmuTOS):
  K11 ... Bconin(2)=001e0061
  K12 ... Bconin(2)=001e0041
  K13 ... Bconin(2)=003b0000
  K14 ... Bconin(2)=002e0003
  K15 ... Bconin(2)=001c000d
  K16 conterm|8 ... Bconin(2)=02300042
PASS: K16 endet auf ...02300042 (oberstes Byte 02 = linke Shift-Taste zum Tastendruck) und K11-K15 wie oben.
FAIL: K16 beginnt mit 00 (Shift-Zustand fehlt) oder andere Werte. Abweichung K11-K15 berichten, nicht
beurteilen. Beim Pi koennen Scancodes einer anderen Tastatur abweichen; dann Tastaturmodell notieren.

## 3. BIOS09 - Bconin(1) (AUX) wartet statt sofort 0 zu liefern

Kein serielles Geraet anschliessen (kein USB-Seriell-Adapter an GPIO 14/15). AWPROBE.NAP doppelklicken.
Erwartet auf dem Bildschirm: Zeile "W1 Bconstat(1)=... waiting in Bconin(1)" und danach NICHTS Neues vom
Hauptprogramm (kein endloser Strom von "R1 ... R2 ... R3"). Es kann einmal "R1 Bconin(1) ret=00000000"
kommen, wenn die offene Leitung ein Stoerbyte lieferte; danach muss "W2" stehen bleiben und warten.
Dann 3-4 Tasten druecken: hinter der Zeile erscheinen Hex-Paare (z. B. " 1e 9e"): der Tastatur-Haken
laeuft waehrend des Wartens. Foto nach 20 s Warten plus Tastendruecken. Zum Schluss Strg+Alt+Entf:
Der Pi muss neu starten (Haken zeigt vorher " 1d 38 53").
PASS: Programm wartet dauerhaft (Zeilenzahl waechst nicht), Tasten erscheinen als Hex-Paare, Strg+Alt+Entf startet neu.
FAIL: Die W/R-Zeilen laufen endlos weiter (Bconin kehrt sofort zurueck) oder Strg+Alt+Entf wirkt nicht.
Nicht pruefbar ohne Hardware: Empfang echter Bytes (kein serielles Geraet, kein MIDI-Geraet); nur
das Warten wird geprueft.

## Zurueck geben
Fotos (1A, 1B, 3), KPROBE.LOG (oder Foto davon), und je Check PASS/FAIL notieren. Falls ein Test den Pi
so haengt, dass keine Taste hilft: Foto, Strom aus, notieren was zuletzt zu sehen war.
