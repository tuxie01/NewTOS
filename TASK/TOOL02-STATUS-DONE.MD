# TOOL02 — Quittung: Manifest fuer jedes Abbild eines Messlaufs

Datum 2026-10-27 · Commit `69b8d227` · Belege
[measurements/measurement-2026-10-27-tool02/](../measurements/measurement-2026-10-27-tool02/RUNINFO.TXT)
· Ticket [TOOL02](TOOL02.md)

- `tools/sd-manifest.sh` schreibt das Manifest v1 fuer `tools/make-sd-image.sh`
  und `tools/mkcard.sh`. make-sd-image-Manifeste sind byte-gleich zu vorher,
  ausser den Feldern, die schon zwischen zwei Bauten abweichen
  (`image_sha256`, `created`, drei Dateien aus `/dev/urandom`).
- `tools/mkcard.sh` ersetzt die Kopien je Lauf (gleiche Schnittstelle;
  Label, Groesse, Ordner, Zeitstempel per Umgebung); `switch n/a`,
  `optional_missing none`.
- Demo-Karte bootet auf QEMU raspi4b bis zum Desktop, Manifest gegen das
  Abbild per mtools gegengeprueft.
- CLAUDE.md: die Manifest-Regel gilt fuer jedes Abbild, das ein Lauf bootet.
