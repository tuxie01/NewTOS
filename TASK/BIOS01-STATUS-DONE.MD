# BIOS01 — Quittung: Rest (BIOS-STOR-003, -004, -005, -006, -002, -007)

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-bios01-rest/](../measurements/measurement-2026-10-29-bios01-rest/RUNINFO.TXT)
· Ticket [BIOS01](BIOS01.md); BIOS-STOR-001 siehe
[BIOS01-S2-STATUS-DONE.MD](BIOS01-S2-STATUS-DONE.MD). Spec
`docs/api/bios/04-rwabs.md` (Reihenfolgetabelle, „Decisions“),
`07-getbpb.md` (Randfaelle 8 und 9), EmuTOS `references/emutos/bios/blkdev.c`.

## Je Befund: Fix, bestaetigender Fall vorher/nachher

| Befund | Fix (NewTOS) | EmuTOS | Fall | vorher | nachher |
|---|---|---|---|---|---|
| 003 | `Rwabs`: Bereichspruefung auch bei Anzahl 0, danach 0 ohne Transfer (`tos-api/src/bios/mod.rs:281@part`, `:211@numb`) | `blkdev.c:461-463` | `bios-r2` 68 (R10) | 0 | -8 EQUAL |
| 007 | `Rwabs`: Einheit 0/1 mit Puffer NULL -> 0 vor jeder Laufwerkspruefung, in beiden Modi (`tos-api/src/bios/mod.rs:184@NUMFLOPPIES`) | `blkdev.c:425-428` | `bios-r2` 83, 84 (F03, F03b) | -15 | 0 EQUAL |
| 004 | Startabtastung wie `pun_info_setup`: `Getbpb` jedes Laufwerks mit Grenze 32768, danach Grenze = max(groesstes `recsiz`, 512) (`tos-api/src/bios/bpb.rs:315@scan`, aufgerufen von `bios::mount`); `parse` prueft gegen die aktuelle Grenze | `blkdev.c:112-163`, `:642` | `bios-r2-x004` 40, 42-46 | BPB / Daten | 0 / -8 EQUAL |
| 005 | `parse`: `tot16`, sonst `tot32` gleich welcher Wert; beide 0 -> `numcl` 0, kein `B_16` (`tos-api/src/bios/bpb.rs:202@tot32`) | `blkdev.c:730-733` | `bios-r2-x005` 40 (X06b) | `…,32672,1` | `…,0,0` EQUAL |
| 006 | `Getbpb` setzt `recsiz` des Laufwerks nach dem Ruecksetzen auf 0, nur ein angenommener Bootsektor setzt es neu (`tos-api/src/bios/bpb.rs:277@recsiz`); logisches `Rwabs` Schritt 4: `recsiz` 0 -> `ESECNF` (`tos-api/src/bios/mod.rs:272@recsiz`). Gilt fuer Trap-, BDOS- und Start-`Getbpb` (gemeinsam `log_in`) | `blkdev.c:443-444`, `:601`, `:736-741` | `bios-r2-x006` 41-44, 46 | Daten | -8 EQUAL |
| 002 | `RW_NOTRANSLATE`: `drive` ist die physische Einheit; SD = 26 (`tos-api/src/bios/drive.rs:23@SD_UNIT`), jede andere `EUNDEV`; Bereich gegen das ganze Geraet mit `>=` (`tos-api/src/bios/mod.rs:303@SD_UNIT`) | `blkdev.c:484-490`, `bios/disk.h:30`, `bios/machine.h:42` | `bios-r2-phys` 32-35, 37-43 | 11/12 verschieden | 0/12 EQUAL |

Tabellen gesamt nachher: `bios-r2` 0/51 (vorher 3), `x004` 0/10 (6),
`x005` 0/13 (1), `x006` 0/11 (5), `phys` 0/12 (11), `x001` 0/8 (0).

## Abbildvergleich (zerstoerende Tabellen)

Testpartition nach dem Lauf, NewTOS vorher = nachher = Orakel: `x001`
84bcadfb (= Original), `x004` a7cf3d2c (bps 1024), `x005` 84bcadfb
(zurueckgeschrieben), `x006` ea645cfc (spf 0). `fsck_msdos -n` 0 bzw. 206/202
wie beim Orakel. Die Befunde 004/006 aendern Antworten, keine Schreibvorgaenge.

## Kein Rueckschritt

Je Tabelle die Fallliste vorher (Klon von 6f38e9b9) und nachher verglichen,
`comm -13` leer: `bios-r1` tt 41/147, falcontc 45/147 (gleiche Liste),
`gemdos-mc-fs20` 0/17, `-fs22` 6/13, `-paths` 12/49 (TOOL20-Funde F1/F2,
unveraendert; die Tabellen tragen die gcprobe-/pxprobe-Faelle); Endzustaende
fs20/fs22/paths vorher = nachher. Alle 20 Laeufe raspi4b bis zum Desktop;
qemu-virt (Debug) bootet bis „desktop up (0 drives)“.

## Tests, Bau

- Neu `tos-api/tests/bios01_storage.rs` (7 Tests: Rueckgabewert und
  Fehlercode je Befund). Angepasst: `bios01_forcechange.rs` (physisch auf
  Einheit 26; nach abgelehntem `Getbpb` ist der logische Rueckweg `ESECNF`,
  wie EmuTOS), `gemdos_fs_bcache.rs` (`mount` laesst MBR + Bootsektor im Cache;
  physisch auf Einheit 26).
- `cargo xtask test` 1840/0/1 (Basis 1833/0/1 + 7), `make -C tosdump
  host-test` gruen (40 Referenzen), `cargo xtask deps --check-mapping` gruen.
- Bau rpi4 release, qemu-virt, rpi4 `--features selftest-emmc`: Warnungen
  kernel 93 / tos-api 64 = Basis; Clippy tos-api 74 = Basis.
- `mapping.tsv`: `pun_info_setup` -> `bpb.rs scan`, `blkdev_hdv_init` ->
  `mod.rs mount`, `blkdev_rwabs` + `logical_lba`/`physical_lba`/`recsiz`,
  `blkdev_getbpb` `boot_sector` -> `log_in`.
- Spec: Pi-Bindungen von `04-rwabs.md`/`07-getbpb.md` auf den neuen Stand,
  verschobene Anker in `09-mediach.md`, `0b-kbshift.md`; doc-check 0 Fehler
  auf den bearbeiteten und den zitierenden Spec-Dateien.

## Offen / nicht nachgebildet (in der Spec benannt)

- `Rwabs` Schritt 7 (Skalierung logisch -> physisch, `blkdev.c:456-458`):
  nicht gebaut; ein `recsiz` > 512 wird nur noch angemeldet, wenn die Karte
  beim Start eine solche Partition hat.
- FAT32-Ausgang von `Getbpb`: EmuTOS hat `clsiz`..`datrec` des BPB schon
  ueberschrieben, NewTOS laesst sie stehen (nur ueber einen alten Zeiger
  sichtbar, nicht gemessen).
- `GETBPB_ALLOWED`-Filter und AHDI-`PUN_INFO`/`pun_ptr`: kein Gegenstueck.
- `sdk/apps/apitest/apitest.c` BB13-BB16 nutzen jetzt Einheit 26; gebaut,
  nicht gelaufen (APITEST liegt auf keiner Karte).
