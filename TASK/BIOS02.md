# BIOS02 — Spec BIOS-Speicher nachziehen; physische Einheit der Pi-SD festlegen

Status: **ERLEDIGT** 2026-10-28 (Spec freigegeben) · Prioritaet: **P1** · Typ: Researcher — Anwenderentscheidung 2026-10-28

## Sachverhalt

Aus dem Audit „BIOS-Speicher“ (`docs/audits/2026-10-28-bios-storage.md`):

- BIOS-STOR-008: die Spec ist an mehreren Stellen veraltet oder durch die
  Sonde widerlegt, u. a. `docs/api/bios/04-rwabs.md` (der Schreibaufruf auf
  Sektor 0 liefert selbst `E_CHNG` und schreibt ohne `RW_NOMEDIACH` nicht)
  und `docs/api/bios/09-mediach.md` (forcechange als „Step-2 candidate“).
- BIOS-STOR-002: `RW_NOTRANSLATE` braucht eine Zuordnung physischer
  Einheiten (EmuTOS: Einheit 2 = ACSI 0, 18 = IDE-Master). Welche Einheit
  die SD der Pi hat, legt bisher nichts fest.
- BIOS-STOR-005: `Getbpb` bei leerer Sektorzahl im Bootsektor — ist das
  NewTOS-Verhalten (Partitionslaenge) beabsichtigt und belegt besser, oder
  EmuTOS folgen?
- BIOS-STOR-007: `Rwabs` mit Puffer NULL auf A:/B: — Angleichung oder
  `docs/COMPATIBILITY.md`-Eintrag (A:/B: sind auf der Pi keine Floppys).

## Arbeit

1. Spec-Eintraege fuer `Rwabs`, `Getbpb`, `Mediach`, `Drvmap` gegen den Pin
   (`references/emutos/`, Commit laut `docs/analysis/ORACLE.md`) neu pruefen
   und berichtigen, mit `file:line`; Sondenwerte aus
   `measurements/measurement-2026-10-28-bios-storage-audit/RUNINFO.TXT` als
   Gegenprobe.
2. Einheitenzuordnung fuer `RW_NOTRANSLATE` vorschlagen (Pi-SD, spaetere
   USB-Massenspeicher), mit Begruendung aus EmuTOS (`bios/blkdev.c`,
   `bios/disk.c`) und der Wirkung auf Programme, die physisch adressieren;
   Entscheidung beim Anwender.
3. 005 und 007: je Empfehlung (angleichen / COMPATIBILITY-Eintrag) mit
   Beleg; Entscheidung beim Anwender.
4. Die Faelle aus TOOL07, die den Spec-Stand messen, je Eintrag nennen.

## Abnahme

- Spec-Eintraege mit Zitaten, doc-check 0 Fehler; freigegeben vom Anwender.
- Entscheidungen zu 002/005/007 in der Spec bzw. `docs/COMPATIBILITY.md`
  vermerkt; danach sind die betroffenen BIOS01-Teile frei.

## Stand 2026-10-28 (Entwurf, Freigabe offen)

- Spec-Korrekturen in `docs/api/bios/04-rwabs.md`, `07-getbpb.md`,
  `09-mediach.md`, `0a-drvmap.md`, je mit `[BIOS02]` markiert: forcechange-
  Handshake samt Pruefreihenfolge (001), Anzahl 0 durchlaeuft die Pruefungen
  (003), `BLKDEVNUM` 26 statt 16, `RW_NOTRANSLATE` wirkt, nur `drive` wird als
  Laufwerksbuchstabe gelesen, `Getbpb`-Grenzen (004/005), Fall-Tabellen je
  Eintrag aus `bios-r2.tsv`.
- Entscheidungsvorlagen (Anwender): 002 Pi-SD = Einheit 26 (SD/MMC-Bus 3,
  `2 + 8*Bus + Geraet`) empfohlen, Alternativen 18 oder Status quo; 005
  EmuTOS folgen; 007 angleichen (NULL-Puffer auf A:/B: liefert 0).
- Nebenbefund SDK: mintlibs `ostruct.h` definiert `RW_NOMEDIACH 1`,
  `RW_NORETRIES 2`, `RW_NOTRANSLATE 3` als Werte, EmuTOS als Masken 2/4/8
  (`references/emutos/bios/disk.h:53-61`) — Triage offen.

## Entscheidungen des Anwenders 2026-10-28

- Spec fuer 001, 003, 005, 007 freigegeben; die `[BIOS02]`-Kopfzeilen der
  vier Dateien sagen „approved“.
- 002: Variante A — die SD der Pi ist physische Einheit **26** (EmuTOS-
  Zaehlung SD/MMC), jede andere Einheit `EUNDEV`. Falltabellen nennen die
  Einheit per Token, aufgeloest aus der Profildefinition (Orakel-Profil
  `disk` 18, NewTOS 26), keine Suchlogik. Regel in
  `docs/api/bios/04-rwabs.md`, Abschnitt „Decisions“.
- 005: EmuTOS folgen. 007: angleichen (NULL-Puffer auf A:/B: liefert 0).
- 001: der BIOS01-Fall muss „schreibt nichts“ per Abbildvergleich belegen,
  nicht nur per Rueckgabewert.
- Nebenbefund SDK (`RW_*`-Konstanten): [SDK02](SDK02.md).
