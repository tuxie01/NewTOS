# BIOS01 — BIOS-Speicher an EmuTOS angleichen (Befunde des Audits 2026-10-28)

Status: **ERLEDIGT** 2026-10-29 ([Quittung](BIOS01-STATUS-DONE.MD), S2: [BIOS01-S2-STATUS-DONE.MD](BIOS01-S2-STATUS-DONE.MD)); 004-Laufzeitgrenze zur Bestaetigung beim Anwender · Prioritaet: **P1** · Typ: Developer — Anwenderentscheidung 2026-10-28 · haengt an [TOOL07](TOOL07.md); Spec [BIOS02](BIOS02.md) freigegeben

## Sachverhalt

Das Audit „BIOS-Speicher“ (`docs/audits/2026-10-28-bios-storage.md`) hat per
eigener Sonde auf beiden Seiten gemessen, nicht per tosdiff; alle Befunde
sind PLAUSIBLE. Anwenderentscheidung 2026-10-28: **ein Fix beginnt erst, wenn
die DUMPBIOS-Plattenrunde (TOOL07) den Befund bestaetigt**; der bestaetigende
Fall ist danach der Regressionsfall. Reihenfolge: S2 zuerst.

| Befund | Schwere | Inhalt | Bedingung |
|---|---|---|---|
| BIOS-STOR-001 | S2 | forcechange-Handshake fehlt: Schreiben von logischem Sektor 0 liefert bei EmuTOS `E_CHNG`, danach `Mediach` 2 und `Rwabs` `E_CHNG` bis `Getbpb`; NewTOS ueberall 0 | TOOL07 |
| BIOS-STOR-003 | S3 | `Rwabs` mit Anzahl 0 ueberspringt die Bereichspruefung (EmuTOS `ESECNF`) | TOOL07 |
| BIOS-STOR-004 | S3 | `Getbpb` nimmt Sektorgroessen ueber 512 an (EmuTOS: NULL) | TOOL07 |
| BIOS-STOR-006 | S3 | `Rwabs` liefert nach gescheitertem `Getbpb` trotzdem Daten | TOOL07 |
| BIOS-STOR-005 | S3 | `Getbpb` bei leerer Sektorzahl im Bootsektor setzt die Partitionslaenge ein | TOOL07 + Absicht in BIOS02 |
| BIOS-STOR-002 | S3 | `RW_NOTRANSLATE` adressiert ueber den Laufwerksbuchstaben statt die physische Einheit | TOOL07 + Einheitenzuordnung in BIOS02 |
| BIOS-STOR-007 | S4 | `Rwabs` mit Puffer NULL auf A:/B: liefert EUNDEV statt 0 | TOOL07 + Entscheidung in BIOS02 |

Nicht hier: BIOS-STOR-008 (Spec, BIOS02), BIOS-STOR-009 (Vektoraufruf-Kanten
im Graphen, Track B), BIOS-STOR-010 (`hdv_*`-Vektoren, BIOS03).

## Arbeit

1. Je Befund: den TOOL07-Fall nennen, der ihn bestaetigt; ohne bestaetigten
   Fall kein Fix, der Befund geht mit Messwert zurueck an den Lead.
2. Spec zuerst, wo sie veraltet ist (BIOS02 liefert sie; ohne freigegebene
   Spec kein Fix fuer 002/005/007).
3. Fix gegen die EmuTOS-Stellen aus dem Bericht
   (`references/emutos/bios/blkdev.c`), Hosttests fuer Rueckgabe und
   Fehlercode je Befund.
4. 001: die Folgen fuer GEMDOS benennen (Neueinloggen des Laufwerks nach
   `Mediach` 2, `ckdrv`/`getbcb` bei EmuTOS) und mit einem Fall messen; ein
   Schreibvorgang mit alter Geometrie nach geaendertem Bootsektor ist S1,
   wenn er auftritt.

## Abnahme

- Die TOOL07-Faelle der behobenen Befunde sind EQUAL gegen das
  Plattenprofil; kein vorher gleicher Fall in `bios-r1.tsv`/`bios-r2.tsv`
  weicht neu ab.
- Hosttests gruen (`cargo xtask test`); QEMU `raspi4b` bootet bis zum
  Desktop, Testpartition nach dem Lauf `fsck_msdos`-sauber.
- `docs/STATUS.md` und Spec im selben Commit; doc-check 0 Fehler.

## Entscheidungen des Anwenders 2026-10-28

- Spec freigegeben (BIOS02): 001, 003, 005 (EmuTOS folgen), 007 (angleichen);
  002 = Variante A: SD = physische Einheit 26, jede andere Einheit `EUNDEV`
  (`docs/api/bios/04-rwabs.md`, Abschnitt „Decisions“).
- 001: der bestaetigende Fall belegt „schreibt nichts“ per Vergleich der
  Testpartition vor/nach (wie TOOL07), nicht nur per Rueckgabewert.
- Pruefung der SDK-Konstanten ([SDK02](SDK02.md) Punkt 1) erledigt
  2026-10-28: die Sonde nutzte Zahlenwerte, kein Header — keine Neumessung
  noetig. SDK02 selbst ist P3 und kein Vorlauf mehr. Beginn nach dem
  TOOL07-Rest.
- Reihenfolge: S2 (001) → dann sofort das S1 `Iorec` (eigenes Ticket aus dem
  Audit „XBIOS-Zeichengeraete“) → Rest von BIOS01.

## Lokalisierung (Lead, 2026-10-28, Graph nach TOOL12 `e70700d6`)

Regel (CLAUDE.md „Testing“): vor jedem Fix NewTOS-Funktion per
`cargo xtask deps <fn> --down` + `docs/analysis/deps/mapping.tsv`, dazu die
`--up`-Retest-Menge.

- **BIOS-STOR-001 (forcechange):** `lrwabs --down` → `blkdev_rwabs` (Kante
  `vector via hdv_rw`) → `blkdev_mediach` (`direct`); `mediach --down` →
  `blkdev_mediach` (`vector via hdv_mediach`); `getbpb --down` →
  `blkdev_getbpb` (`vector via hdv_bpb`). NewTOS:
  `tos-api/src/bios/mod.rs@rwabs`, `tos-api/src/bios/mod.rs@mediach`,
  `tos-api/src/bios/bpb.rs@getbpb` (dazu `bpb_for`, `parse`).
- **`--up`-Retest-Menge:** `blkdev_rwabs --up` 405 EmuTOS-Funktionen, davon 83
  mit NewTOS-Gegenstueck; `blkdev_mediach` 407/85; `blkdev_getbpb` 184/26.
  Die naechsten Ebenen (Tiefe ≤ 3) mit Gegenstueck: `biostrap` →
  `bios/mod.rs@dispatch`; `flush`/`getbcb`/`usrio`/`getrec` →
  `bdos/fs/cache.rs` (`write_back`, `get`, `victim`, `direct_read`,
  `direct_write`, `with_sector`), `bdos/fs/fsio.rs` (`usrio`, `partial`,
  `xrw_recs`), `bdos/fs/fat.rs@mount` (`ckdrv`), `fat.rs@with_fat_sector`,
  `bdos/fs/file.rs@ixclose`, `bdos/fs/find.rs@scan`, `gemdos/mod.rs@dispatch`.
  Retest also: `bios-r1.tsv`, `bios-r2.tsv` (beide Profile), die
  GEMDOS-Dateisystem-Hosttests unter `tos-api/tests/` und — 001 betrifft das
  Neueinloggen nach `Mediach` 2 (`ckdrv`) — ein Boot bis zum Desktop mit
  `fsck_msdos` der Testpartition.
- **Retest-Menge, Funktionen ohne NewTOS-Gegenstueck** (Anwender 2026-10-28:
  aus dem heutigen `mapping.tsv`, `-`-Zeilen in den beruehrten Schichten
  `bios` und `bdos`, Vereinigung der `--up`-Mengen von `blkdev_rwabs`,
  `blkdev_mediach`, `blkdev_getbpb`): 51 Funktionen.
  - `bdos` (1): `bdos_trap2`.
  - `bios` (50): `add_cookie_frb`, `altram_init`, `balloc_stram`, `bios_init`, `biosmain`, `blkdev_hdv_init`, `blkdev_init`, `bootstrap`, `bus_init`, `check_busses`, `check_read_byte`, `detect_blitter`, `detect_dip_switches`, `detect_dmasound`, `detect_dsp`, `detect_ide`, `detect_magnum`, `detect_megartc`, `detect_modectl`, `detect_monster`, `detect_nova`, `detect_nvram`, `detect_scc`, `detect_scsi`, `detect_tt_mfp`, `detect_ttram_size`, `detect_video`, `detect_vme`, `dsp_loadprog`, `dsp_lodtobinary`, `filesize`, `ide_init`, `ide_interface_exists`, `init_exc_vec`, `init_user_vec`, `int_illegal`, `int_priv`, `int_unimpint`, `machine_detect`, `memdone`, `panic`, `pun_info_setup`, `run_auto_program`, `screen_init_address`, `ttram_detect`, `vecs_init`, `xhdi_handler`, `xhdi_vec`, `XHInqDev`, `XHInqDev2`.
  - Die meisten sind Start- und Hardwareerkennung (`detect_*`, `*_init`,
    XHDI, DSP), die ueber Aufrufketten des Starts in der Menge stehen (z. B.
    `bios_init -> blkdev_init`, `detect_nova -> check_read_byte`). Nach
    [TOOL17](TOOL17.md) klassifiziert der Lead diese Liste neu (mit Absicht /
    nicht abgebildet). Der Fix ergaenzt die `mapping.tsv`-Zeilen der
    Funktionen, die er beruehrt (Ratsche, CLAUDE.md „Testing“).

## Stand 2026-10-28 (BIOS-STOR-001)

- Erledigt: forcechange-Handshake; `bios-r2` W01–W05, X13, X14 und die
  001-Faelle in x004–x006 EQUAL; neue Tabelle `bios-r2-x001.tsv` belegt
  „schreibt nichts“ per Abbildvergleich (NewTOS vorher 4/8 verschieden und
  Sektor 0 geaendert, nachher 0/8, Abbild = Original = Orakel). Kein vorher
  gleicher Fall neu verschieden. Beleg
  `measurements/measurement-2026-10-28-bios01-s2/RUNINFO.TXT`.
- Arbeit 4 (GEMDOS-Folge): S1 gefunden — GEMDOS schreibt nach geaendertem
  Bootsektor mit alter Geometrie; dazu bleibt `Mediach` 2 nach GEMDOS-Zugriff
  stehen (fehlende Neuanmeldung). Eigenes Ticket [FS20](FS20.md).

## Retest-Liste nach TOOL17 (2026-10-29)

`cargo xtask deps <wurzel> --up --layers bios,bdos`, vereinigt ueber `blkdev_rwabs`, `blkdev_mediach`, `blkdev_getbpb`; 407 Funktionen:
`unmapped` 20 statt 51 — `bdos`: `bdos_trap2`; `bios`: `add_cookie_frb`, `altram_init`, `balloc_stram`, `bios_init`, `biosmain`, `blkdev_hdv_init`, `blkdev_init`, `bus_init`, `dsp_loadprog`, `dsp_lodtobinary`, `filesize`, `machine_detect`, `panic`, `pun_info_setup`, `run_auto_program`, `screen_init_address`, `xhdi_handler`, `XHInqDev`, `XHInqDev2`. Die uebrigen frueheren `-`-Eintraege sind jetzt
`design:` (Hardware bzw. ADR) oder `mapped`.
