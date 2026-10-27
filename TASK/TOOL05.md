# TOOL05 — doc-check prueft auch die Doku, die eine geaenderte Datei zitiert

Status: **OFFEN** · Prioritaet: **P2** · Typ: Werkzeug (Zitierfestigkeit) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Die Commit-Regel (CLAUDE.md, „Commit Discipline“) prueft mit
  `cargo xtask doc-check <files you edited>` nur die Doku, die ein Commit
  selbst aendert. Aendert ein Commit eine **zitierte** Datei, bleiben die
  Zitate in anderen Dokumenten ungeprueft.
- Fall: `60d3c4fd` (MEM12) kuerzte `sdk/libc/overlay/include/mint/osbind.h`
  von ueber 812 auf 775 Zeilen. Das Zitat auf die Zeilen 804–812 von
  `osbind.h` in
  `docs/sdk/05-abi-pitfalls.md` (Abschnitt B4) zeigte danach hinter das Dateiende, der
  Absatz beschrieb den Stand vor MEM12. Der Commit aenderte keine Doku unter
  `docs/sdk/`, also lief dort kein doc-check; aufgefallen ist es erst im
  vollen Lauf am 2026-10-28 (753 statt 752 Fehler, `docs/sdk/` nicht mehr
  sauber). Behoben in derselben Arbeitseinheit wie dieses Ticket.
- Symbol-Anker (`<path>:<line>@<symbol>`) fangen Verschiebungen nur bei neuen
  Zitaten; die rund 2300 alten, unverankerten Zitate in NewTOS-Code bleiben
  bis zur naechsten Beruehrung ungeprueft.

## Arbeit

1. doc-check bekommt eine Auswahl „betroffene Doku“: zu einem Commit bzw.
   zum Arbeitsbaum gegen `--base` alle Dateien, die sich geaendert haben
   (Code wie Doku, auch geloeschte und umbenannte), und dazu jede Doku im
   Pruefumfang, die eine dieser Dateien zitiert (Pfad oder `file:line`). Die
   so gefundenen Dokumente werden voll geprueft. Aufruf z. B.
   `cargo xtask doc-check --affected [--base <rev>]`.
2. Fehler in einem betroffenen Dokument, das der Commit nicht selbst
   aendert, nennt die Ausgabe getrennt, mit der geaenderten Datei, die sie
   ausgeloest hat.
3. CLAUDE.md „Commit Discipline“: der Doku-Schritt vor dem Commit nutzt diese
   Auswahl statt der Liste der selbst bearbeiteten Dateien (Wortlaut im
   Bericht vorschlagen, der Lead traegt ihn ein). Gleiches fuer die
   Rollen-Dateien, soweit sie doc-check nennen.
4. Hosttest: ein kuenstlicher Baum, in dem eine zitierte Datei schrumpft;
   `--affected` meldet das Zitat, ohne das Dokument als Argument.

## Abnahme

- `cargo xtask doc-check --affected --base 60d3c4fd^` gegen den Stand von
  `60d3c4fd` meldet `docs/sdk/05-abi-pitfalls.md` mit dem Zitat hinter dem
  Dateiende (Nachweis im Bericht, z. B. per Arbeitsverzeichnis dieses
  Commits im Job-Scratch).
- Hosttests gruen; die Auswahl laeuft auf diesem Host in Sekunden, nicht
  Minuten (Messwert im Bericht).
- doc-check der geaenderten Doku 0 Fehler.
