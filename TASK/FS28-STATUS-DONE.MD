# FS28 — Quittung: `Dsetpath` nach Neuanmeldung setzt den Pfad

Datum 2026-10-30 · Beleg
[measurements/measurement-2026-10-30-fs28/](../measurements/measurement-2026-10-30-fs28/RUNINFO.TXT)
· Ticket [FS28](FS28.md) (Fund N1 aus [FS24](FS24.md))
· Spec `docs/api/gemdos/directory/3b-dsetpath.md` (Beschreibung, NewTOS-Notiz 2026-10-30)

## Ursache (per Hosttest gegen HEAD belegt)

- `set_curdir` (`tos-api/src/bdos/fs/curdir.rs:229@set_curdir`) nahm zuerst den
  Slot fuer den neuen Pfad (`incr`: der erste freie) und pruefte erst danach das
  alte PD-Byte mit `live()`. Nach einer Neuanmeldung hat `free_drive`
  (`tos-api/src/bdos/fs/curdir.rs:335@free_drive`) den Slot geleert, das Byte
  aber stehen lassen; `incr` vergibt denselben Index neu, `live(old)` ist dann
  wahr, `decr` gibt den eben genommenen Slot frei, `curdir()` liest die Wurzel.
- Nicht nur nach einem Neustart des Aufrufs: jedes veraltete Byte loest es aus
  (gegen HEAD schlug im selben Testprogramm auch der bestehende Test
  `current_directories_on_the_changed_drive_become_the_root` fehl, sobald ein
  neuer Test ein veraltetes Byte hinterlassen hatte).
- EmuTOS: `xchdir` liest `olddir` vor `findit`
  (`references/emutos/bdos/fsdir.c:1129`, `:1134`); `ckdrv` repariert ein
  Byte ohne DND auf einen Wurzel-Slot, den ersten freien
  (`references/emutos/bdos/fsdrive.c:121-130`,
  `references/emutos/bdos/fsdir.c:1176-1179`); das abschliessende
  `decr_curdir_usage(olddir)` trifft diesen Wurzel-Slot.

## Fix

- `set_curdir` liest das alte Byte und beurteilt es **vor** `incr`; ein
  veraltetes Byte ist die Wurzel (NewTOS gibt der Wurzel keinen Slot) und gibt
  nichts frei. Sonst Reihenfolge wie `xchdir`: neue Nutzung, speichern, alte
  abgeben.
- `docs/analysis/deps/mapping.tsv`: Zeile `xchdir` → `curdir.rs@set_curdir`
  (Ratsche).

## Messung (QEMU raspi4b, `cargo xtask tosdump`, vorher `ec9e260f` / nachher)

| Tabelle | Referenz | vorher | nachher |
|---|---|---|---|
| `gemdos-mc-fs20` | `-mc-fs20-disk` | 0 von 17 | 0 |
| `gemdos-mc-fs22` | `-mc-fs22-disk` | 0 von 13 | 0 |
| `gemdos-mc-paths` | `-mc-paths-disk` | 1 von 49 (67) | 1 (67) |
| `gemdos-mc-fs24` | `-mc-fs24-disk` | 5 von 84 (76 87 118 149 150) | **2** (76 149) |
| `bios-r1` (Kanarie) | tt | 32 von 147 | 32, gleiche Faelle |
| `bios-r1` (Kanarie) | falcontc | 37 von 147 | 37, gleiche Faelle |

- 87, 118, 150 EQUAL (`Dgetpath` `\SUB` wie das Orakel); keine neue Abweichung.
- paths 67, fs24 76/149 bleiben: `Mediach` nach `Dsetpath` auf einen im
  DND-Baum angemeldeten Ordner — FS24, ACCEPTED unter ADR-033.
- NewTOS-Testpartition nach dem Lauf vorher = nachher (sha256, alle vier
  zerstoerenden Tabellen); `fsck_msdos` fs20 206 (Geometriewechsel, wie FS23),
  sonst 0.
- Boot: raspi4b in jedem Lauf bis „desktop up“; qemu-virt (frischer
  Release-Bau) „[desk] desktop up (0 drives)“, kein PANIC.

## Tests

- `tos-api/tests/fs20_media_change.rs`: neu
  `fs28_dsetpath_that_meets_the_change_sets_the_path` (V1a: `Dsetpath` bei
  anstehendem Wechsel, `Mediach` danach 0, `Dgetpath` `\SUB`, eine Nutzung des
  Slots) und `fs28_dsetpath_after_a_relogin_by_another_call_sets_the_path`
  (Neuanmeldung durch `Dfree`, dann `Dsetpath` + `Dgetpath`). Gegen HEAD
  schlagen beide fehl.
- `cargo xtask test`: **1907 / 0 / 1** (Ausgangsstand 1905 / 0 / 1, +2).
- Build beide Boards ohne neue Warnung; clippy tos-api 74 / kernel 121 auf
  beiden Boards (unveraendert), kein Treffer in `curdir.rs` oder im Test.

## Offen

- Abgeleitet, nicht gemessen: in EmuTOS haengt das Ergebnis von der
  Slot-Belegung ab — ist beim Neustart ein niedrigerer Slot frei als der
  veraltete Index, nimmt die Wurzel ihn, der neue Ordner kann den veralteten
  Index bekommen und das `decr` gibt ihn wieder frei; `Dgetpath` laese dann die
  Wurzel. NewTOS (Wurzel ohne Slot) liefert immer den gesetzten Pfad. Nicht
  nachgebildet; Entscheidung beim Anwender, ob ein Messfall dafuer entstehen
  soll.
- Echte Pi: nicht gelaufen (erreichbar ueber `Rwabs` auf Sektor 0, siehe Ticket).
