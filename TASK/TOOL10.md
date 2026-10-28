# TOOL10 — `cargo xtask build` legt `kernel8.img` im Repo-Wurzelverzeichnis ab

Status: **OFFEN** · Prioritaet: **P2** · Typ: Werkzeug — Anwenderentscheidung 2026-10-28

## Sachverhalt

- CLAUDE.md, „Source Tree Conventions“: „Nothing new in the repository
  root“ und „Built `kernel8.img` for the card is not kept in the root“.
- `cargo xtask build` schreibt `kernel8.img` trotzdem dorthin (am
  2026-10-28 nach dem Audit „BIOS-Speicher“ im Wurzelverzeichnis gesehen;
  die Datei ist gitignored).

## Arbeit

1. Messen: welche xtask-Unterbefehle und Skripte `kernel8.img` ins
   Wurzelverzeichnis schreiben oder von dort lesen (`tools/`,
   `measurements/*/method/` nur lesen, nicht aendern — Laeufe sind
   unveraenderlich), Liste im Bericht.
2. Ausgabe nach `target/` (z. B. neben das ELF des Boards und Profils); der
   Pfad wird am Ende des Baus ausgegeben. Leser in `tools/` und xtask
   anpassen.
3. Doku, die den alten Pfad nennt (CLAUDE.md-Wortlaut im Bericht vorschlagen,
   der Lead traegt ein).

## Abnahme

- Nach `cargo xtask build` und `cargo xtask build --release` liegt keine
  `kernel8.img` im Wurzelverzeichnis; `cargo xtask run --board rpi4` bootet
  bis zum Desktop.
- Hosttests gruen (`cargo xtask test`); doc-check der geaenderten Doku 0
  Fehler.
