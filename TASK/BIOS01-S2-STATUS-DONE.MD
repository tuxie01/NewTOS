# BIOS01 (S2, BIOS-STOR-001) — Quittung: forcechange-Handshake

Datum 2026-10-28 · Beleg
[measurements/measurement-2026-10-28-bios01-s2/](../measurements/measurement-2026-10-28-bios01-s2/RUNINFO.TXT)
· Ticket [BIOS01](BIOS01.md), nur Befund BIOS-STOR-001; die uebrigen Befunde
bleiben offen.

- Fix nach `docs/api/bios/04-rwabs.md` (Reihenfolgetabelle), `09-mediach.md`,
  `07-getbpb.md`: ein `forcechange` je Laufwerk
  (`tos-api/src/bios/drive.rs:152@FORCECHANGE`). `Rwabs` logisch: Schritt 5
  (gesetzt -> `E_CHNG`, auch mit `RW_NOMEDIACH`), Schritt 6 (Schreiben ab
  logischem Sektor 0 setzt ihn, vor Bereichs- und Medienwechselpruefung),
  Schritt 10 (`media_change` != 0 ohne `RW_NOMEDIACH` -> `E_CHNG`, nichts
  geschrieben) — `tos-api/src/bios/mod.rs:161@rwabs`. `Mediach` = EmuTOS
  `blkdev_mediach` (`tos-api/src/bios/mod.rs:292@media_change`): EUNDEV,
  `forcechange` -> 2, sonst 0. `Getbpb` loescht nach der Laufwerkspruefung,
  vor dem Lesen, bei jedem Ausgang — fuer den Trap und fuer die
  BDOS-Anmeldung (`tos-api/src/bios/bpb.rs:242@boot_sector`). Physisch
  (`RW_NOTRANSLATE`) setzt nie.
- tosdiff: W01-W05, X13, X14 (bios-r2) und x004 39, x005 39/50, x006 38 jetzt
  EQUAL. bios-r2 3 von 51 verschieden (vorher 10; Rest 003/007), x004 6/10,
  x005 1/13, x006 5/11, phys 11/12 unveraendert; bios-r1 tt 46/147,
  falcontc 50/147 = Fallliste aus TOOL11. Kein vorher gleicher Fall weicht neu
  ab (Falllisten verglichen). Endzustaende x004/x005/x006 = Orakel.
- Abnahme „schreibt nichts“ per Abbildvergleich: `bios-r2.tsv` W01 schreibt
  die gelesenen Bytes zurueck und kann das nicht zeigen. Neue zerstoerende
  Abschlusstabelle `tosdump/cases/bios-r2-x001.tsv` (Sektor 0 mit einem
  geaenderten OEM-Byte, ohne `RW_NOMEDIACH`) und Referenz
  `EMUTOS-1a7484ab-BIOS-r2-x001-disk` (Orakel zweimal bytegleich, Abbild
  unveraendert). NewTOS vorher: 4/8 verschieden, Sektor 0 auf der Platte
  geaendert. Nachher: 0/8, `testpart: identical`, Endzustand = Original =
  Orakel. Urteil: schreibt nichts, gemessen.
- GEMDOS-Folgen (Arbeit 4, Sonde `gcprobe.c`, beide Seiten): EmuTOS meldet C:
  beim naechsten GEMDOS-Zugriff neu an (`getbcb`/`longjmp_rwabs` -> `osif`,
  `Getbpb` loescht `forcechange`); NewTOS' BDOS liest am BIOS vorbei und meldet
  nie neu an. Folge 1: nach dem Fix bleiben `Mediach` 2 und logisches `Rwabs`
  `E_CHNG` nach einem GEMDOS-Zugriff stehen, bis jemand `Getbpb` ruft (EmuTOS
  0) — neues Fenster, Ursache die fehlende Neuanmeldung. Folge 2 — **S1, tritt
  auf** (vorher wie nachher): nach geaenderter Bootsektor-Geometrie schreibt
  NewTOS' GEMDOS mit der alten Geometrie (Dfree 32672 statt 32676; Cluster 3
  landet in Sektor 293 statt 277, d. h. in Cluster 7 laut Platte). Berichtet,
  nicht behoben.
- Hosttests: `tos-api/tests/bios01_forcechange.rs` (9 Faelle: W01-W11,
  X12-X16, Anzahl 0, Bereich, physisch, Getbpb-NULL, BDOS-Anmeldung);
  `gemdos_fs_device_error.rs` schreibt Sektor 0 jetzt mit `RW_NOMEDIACH`.
  `cargo xtask test` 1760/0/1 (Basis 1751/0/1), `make -C tosdump host-test`
  gruen (30 Referenzen), Bau rpi4 release und qemu-virt ohne neue Warnungen
  (Clippy tos-api 74 = Basis).
- `mapping.tsv`: `blkdev_rwabs`, `blkdev_mediach`, `blkdev_getbpb` um die
  neuen NewTOS-Funktionen ergaenzt; `deps --check-mapping` gruen. Spec: nur
  Pi-Bindungszeilen als umgesetzt markiert, verschobene Anker korrigiert;
  doc-check 0 Fehler.
- Offen fuer den Lead: `docs/STATUS.md`-Eintrag; Ticket fuer die
  GEMDOS-Neuanmeldung (S1 oben).
