# BIOS01 — BIOS-Speicher an EmuTOS angleichen (Befunde des Audits 2026-10-28)

Status: **OFFEN** · Prioritaet: **P1** · Typ: Developer — Anwenderentscheidung 2026-10-28 · haengt an [TOOL07](TOOL07.md), Teile an [BIOS02](BIOS02.md)

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
