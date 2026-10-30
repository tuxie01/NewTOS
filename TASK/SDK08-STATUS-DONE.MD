# SDK08 — Quittung: Rebuild-Paritaet blendet nur die Compiler-Kennung aus

Datum 2026-10-30 · Ticket [SDK08](SDK08.md) · Fund aus [COOK02](COOK02.md) ·
Lauf `measurements/measurement-2026-10-30-sdk08/RUNINFO.TXT`

## 1. Ursache, abschnittsweise

Alle 1049 Teile der drei ausgelieferten Archive (`libclang_rt.builtins.a` 151,
`libm.a` 256, `libc.a` 642) mit dem Host-clang `Apple clang version 21.0.0
(clang-2100.4.1.1)` neu gebaut (Flags wie in den Tests, libc aus
`sdk/libc/build/compile-commands.txt`) und als ELF64 verglichen: ELF-Kopf ohne
`e_shoff`, Abschnittsliste, jeder Abschnittskopf ohne `sh_offset`, jeder Inhalt.

| Archiv | Teile | Ergebnis |
|---|---|---|
| builtins | 151 | nur `.comment` (Inhalt + Groesse) |
| libm | 256 | nur `.comment` (Inhalt + Groesse) |
| libc | 638 | nur `.comment` (Inhalt + Groesse) |
| libc | 4 | byte-gleich (Assembler-Teile ohne `.comment`) |

Ausgeliefert `clang-2100.3.34.2`, neu `clang-2100.4.1.1`. Kein Code-, Daten-,
Relokations- oder Symbolabschnitt verschieden — kein echter Befund. Die Archive
wurden nicht neu gebaut.

## 2. Aenderung

| Datei | Inhalt |
|---|---|
| `sdk/compiler-rt/tests/rt-host.sh` | Schritt 2 vergleicht mit `parity()`: byte-gleich → gleich; sonst beide Objekte per `llvm-objcopy --remove-section=.comment`, die Kopien byte-gleich **und** die Symboltabellen der Originale gleich (`llvm-readobj --symbols`; das Entfernen nimmt das `$d`-Symbol in `.comment` mit) → „bis auf die Compiler-Kennung gleich“; sonst FAIL. Ausgabe nennt Host-clang und clang der ausgelieferten Teile; eingebaute Gegenprobe (ein Byte in `.text` von `lib_builtins_multf3.o` invertiert → muss abweichen) |
| `sdk/libc/tests/ldbl-host.sh` | dasselbe fuer die gelinkten Teile; Gegenprobe am ersten Teil, das nur in `.comment` abweicht |
| `tools/sd-manifest.sh` | `write` schreibt `host_clang <clang --version, Zeile 1>` bzw. `host_clang not found` nach `created`; Format v1, Leser waehlen per Schluessel |
| `docs/sdk/06-image-and-testing.md` | Zeile beschrieben; RUNINFO.TXT nennt sie mit den `optional_missing`-Zeilen |

## 3. Pruefung

- SDK-Hosttests vorher 33/2/0, nachher **35/0/0**.
- Eingebaute Gegenprobe in beiden Tests: `differ` wie erwartet.
- Gegenprobe je Abschnitt (`measurements/measurement-2026-10-30-sdk08/method/counter-check.sh`, vier Teile, jeder
  nicht leere Abschnitt einzeln invertiert): 30/30 wie erwartet, nur `.comment`
  ausgeblendet. Die erste Fassung ohne Symbolvergleich uebersah ein Byte im
  `$d`-Symbol von `.comment` (`libc.a(arch__cmdargs.o)`); behoben vor dem Lauf.
- `cargo xtask test` 1895/0/1 (unveraendert).

## 4. Offen

- `CLAUDE.md` (Abschnitt Measurement runs) nennt fuer RUNINFO.TXT nur die
  `optional_missing`-Zeilen; die `host_clang`-Zeile steht bisher nur in
  `docs/sdk/06-image-and-testing.md`. Aenderung von `CLAUDE.md` ist Sache des
  Anwenders.
- `cargo xtask tosdump` uebernimmt aus dem Manifest nur `image_sha256` und
  `optional_missing` in `tosdump-run.txt`; `host_clang` dort nicht (kein
  xtask-Eingriff in SDK08).
