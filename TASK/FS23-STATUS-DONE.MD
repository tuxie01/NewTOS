# FS23 — Quittung: `d_fname` hinter dem NUL bleibt unberuehrt

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-fs23/](../measurements/measurement-2026-10-29-fs23/RUNINFO.TXT)
· Ticket [FS23](FS23.md) (Anwenderentscheidung 2026-10-29, Fund F1 aus [TOOL20](TOOL20.md))
· Spec `docs/api/structures/dta.md` (`d_fname`, Lebenslauf Punkt 2),
`docs/api/gemdos/file/4e-fsfirst.md` (Seiteneffekte, Randfall 6),
`docs/api/gemdos/file/4f-fsnext.md` (DTA-Ausgabe)

## Fix

- `tos-api/src/bdos/fs/find.rs:465@makbuf` schreibt Attribut, Zeit, Datum,
  Laenge (DTA 0x15..0x1D) und von `d_fname` nur Name und NUL: der Name kommt
  aus `name_83` (Gegenstueck von `packit`) und wird bis einschliesslich NUL
  kopiert. Vorher wurden alle 14 Bytes geschrieben, hinter dem Namen Nullen.
  EmuTOS: `makbuf` ruft `packit`, das `NAME[.EXT]` und ein `'\0'` schreibt
  und zurueckkehrt (`references/emutos/bdos/fsdir.c:1297-1324`, `:1851-1862`).
  Die Bytes hinter dem NUL behalten ihren Inhalt: die Bytes des Aufrufers vor
  der ersten Suche, der Rest eines laengeren frueheren Namens danach.
- Aufrufer im Kern und in `programs/` lesen `d_fname` bis zum NUL
  (`kernel/src/shell.rs`, `kernel/src/selftest_emmc.rs`, `programs/newtos-gem`);
  der Desktop kopiert die 14 Bytes und vergleicht sie beim Sortieren als
  Bytefeld (`programs/desktop/src/model.rs`) — zwei verschiedene Namen
  unterscheiden sich spaetestens am NUL des kuerzeren, die Reihenfolge bleibt
  die von `strcmp`; EmuTOS' Desktop kopiert die 23 Bytes ebenso
  (`docs/api/desktop/deskdir.md`).

## Spec

- `docs/api/structures/dta.md`: `d_fname` sagt jetzt, dass nur Name und NUL
  geschrieben werden, mit Quelle und Messung. Lebenslauf Punkt 2 sagte, `Fsfirst`
  initialisiere die ganze 44-Byte-DTA und fuelle 0x15..0x2B — falsch; jetzt
  nach `xsfirst`/`ixsfirst`/`makbuf` (`bdos/fsdir.c:530-531`, `:473-490`,
  `:1851-1862`).
- `docs/api/gemdos/file/4e-fsfirst.md`: „The entire 44-byte DTA … is
  (re)written“ ersetzt; Tabellenzeile `d_fname` und Randfall 6 ergaenzt.
- `docs/api/gemdos/file/4f-fsnext.md`: DTA-Ausgabe ergaenzt.

## Messung (QEMU raspi4b, `cargo xtask tosdump`, vorher `eccc6ecc` / nachher)

| Tabelle | Referenz | vorher | nachher |
|---|---|---|---|
| `gemdos-mc-fs20` | `-mc-fs20-disk` | 0 von 17 | 0 |
| `gemdos-mc-fs22` | `-mc-fs22-disk` | 6 von 13 (33 37 39 44 46 47) | **0** |
| `gemdos-mc-paths` | `-mc-paths-disk` | 12 von 49 (45 47–52 56 58–60 67) | **1** (67) |
| `bios-r1` (Kanarie) | tt | 39 von 147 | 39, gleiche Faelle |
| `bios-r1` (Kanarie) | falcontc | 43 von 147 | 43, gleiche Faelle |

- Alle 17 F1-Faelle EQUAL; keine neue Abweichung in den fuenf Vergleichen.
- paths Fall 67 bleibt: F2 (`Dsetpath` nach einem Medienwechsel), [FS24](FS24.md).
- NewTOS-Testpartition nach dem Lauf vorher = nachher (sha256, alle drei
  zerstoerenden Tabellen); `fsck_msdos` wie vorher.

## Tests

- Neu `tos-api/tests/gemdos_dta_fname_tail.rs` (3 Tests): DTA mit `0xA5`
  vorbelegt; `Fsfirst`, ein kuerzerer Name nach einem laengeren auf derselben
  DTA, `Fsnext` durch `C:\SUB`; jedes Byte hinter dem NUL geprueft. Gegen das
  alte `makbuf` schlagen alle drei fehl.
- `cargo xtask test`: **1850 / 0 / 1** (Ausgangsstand 1847 / 0 / 1, +3).
- Bau `--board rpi4` und `--board qemu-virt`: exit 0. Clippy `kernel` 121 /
  `tos-api` 74 auf beiden Boards, vorher wie nachher; keine Meldung in der
  neuen Testdatei.

## Ratsche

- `docs/analysis/deps/mapping.tsv`: Notizen zu `makbuf` und `packit`
  (Gegenstuecke unveraendert: `makbuf` → `makbuf`, `packit` → `name_83`).

## Funde ausserhalb des Tickets (nicht behoben)

- Der GEMDOS-Tracer (Feature `gemdos-trace`) begruendet seine DTA-Erfassung
  mit „`put_state` + `makbuf` wrote all 44 bytes“
  (`tos-api/src/gemdos/trace.rs:34@makbuf`, `tos-api/src/gemdos/trace.rs:618@makbuf`): seit FS23 schreibt der Aufruf
  die Bytes hinter dem NUL nicht mehr; die 44-Byte-Erfassung liest damit
  Speicher, den der Aufruf nicht beruehrt hat (Regel C1). Kommentar und/oder
  Erfassungslaenge brauchen eine Entscheidung.
- `name_83` folgt `packit` nicht in zwei Randfaellen: `packit` endet auch an
  einem NUL im Namen (`(*s)`) und haengt bei Eintraegen, die mit `.` beginnen,
  keine Endung an (`bdos/fsdir.c:1306-1316`). Bei `.`/`..` ist die Endung leer
  (gleiche Ausgabe); ein NUL im Namensteil kommt nur auf beschaedigten
  Datentraegern vor. Nicht gemessen.
