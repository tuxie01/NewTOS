# TOOL03 — Reproduzierbare Messabbilder: gleiche Eingaben, gleiche sha256

Status: **OFFEN** · Prioritaet: **P2** · Typ: Werkzeug (Reproduzierbarkeit) — Anwenderentscheidung 2026-10-27

## Sachverhalt

- Zwei Bauten von `tools/make-sd-image.sh` aus denselben Eingaben ergeben
  verschiedene Abbilder (`measurements/measurement-2026-10-27-tool02/RUNINFO.TXT`,
  Manifestvergleich): `image_sha256` und `created` weichen ab, dazu die
  Dateien `C:\KERNEL8.IMG`, `C:\LYNXBOOT.IMG` und `C:\DATA.BIN`, die das
  Skript aus `/dev/urandom` fuellt.
- Damit identifiziert die Manifest-sha256 nicht das Messabbild. Das braucht
  der DUMPGDOS-Abbildvergleich (`TASK/FS19.md`).
- Anwenderentscheidung 2026-10-27: „replace /dev/urandom in the image build
  with a seeded PRNG (fixed seed recorded in the manifest) and use fixed file
  timestamps. Goal: two builds from the same inputs give the same image
  sha256, so the manifest hash identifies the measurement image.“

## Arbeit

1. **Erst messen:** zwei Bauten aus denselben Eingaben, Abbilder byte-weise
   vergleichen, jede abweichende Stelle einer Ursache zuordnen (Dateiinhalt,
   FAT-Zeitstempel von Dateien und Verzeichnissen, Volume-Seriennummer im
   Bootsektor, macOS-Metadaten wie `.fseventsd`, Reihenfolge der
   Verzeichniseintraege, nicht reproduzierbare Eingabedateien). Die Liste
   steht im Bericht, bevor etwas geaendert wird.
2. `/dev/urandom` durch einen PRNG mit festem Startwert ersetzen; der Startwert
   steht im Manifest (`seed`-Zeile, Format v1 erweitert und im Kopfkommentar
   von `tools/sd-manifest.sh` beschrieben).
3. Feste Zeitstempel fuer jede Datei und jedes Verzeichnis auf der Karte;
   was sonst je Bau variiert (Seriennummer, Metadaten), fest setzen oder
   entfernen.
4. Gilt fuer `tools/make-sd-image.sh` und `tools/mkcard.sh`.
5. `created` bleibt als Zeitpunkt des Baus im Manifest, ist aber nicht Teil
   des Abbilds; die Doku sagt, dass die Manifest-sha256 das Abbild
   identifiziert.
6. Eingaben, die selbst nicht reproduzierbar sind (z. B. m68k-PRGs mit
   zufaelligen Objektnamen in der Symboltabelle, ref07-v3-RUNINFO), benennen;
   sie veraendern die Abbild-sha256 nur, wenn sich die Eingabe aendert, und
   das zeigen die `file`-Zeilen des Manifests.

## Abnahme

- Zwei Bauten aus denselben Eingaben: gleiche Abbild-sha256, fuer
  `make-sd-image.sh` (Standard und mit `SD_FSBENCH_D`) und fuer `mkcard.sh`.
- Abbild bootet auf QEMU raspi4b bis zum Desktop wie vorher.
- doc-check der geaenderten Doku 0 Fehler.
