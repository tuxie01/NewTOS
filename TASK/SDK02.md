# SDK02 — `RW_*` in mintlibs `ostruct.h` sind Bitnummern: Kommentar und Bau-Pruefung

Status: **OFFEN** (Punkt 1 erledigt 2026-10-28) · Prioritaet: **P3** · Typ: Developer (SDK) — Anwenderentscheidung 2026-10-28 (korrigiert am selben Tag), Nebenbefund aus [BIOS02](BIOS02.md)

## Sachverhalt

- `sdk/libc/overlay/include/mint/ostruct.h:240-245@RW_NOTRANSLATE`: `RW_READ 0`,
  `RW_WRITE 0`, `RW_NOMEDIACH 1`, `RW_NORETRIES 2`, `RW_NOTRANSLATE 3`;
  mintlib 0.60.1 (`sdk/libc/mintlib/`, nicht versioniert) gleich. Das sind
  **Bitnummern** (READ/WRITE = Bit 0): `1 << RW_x` ergibt genau die Masken der
  Spec 1/2/4/8 (`references/emutos/bios/disk.h:52-61`,
  `docs/api/bios/04-rwabs.md`). Der Kommentar im Header („Mode bitmask“)
  fuehrt in die Irre.
- Anwenderentscheidung 2026-10-28: **Werte nicht aendern** — das braeche die
  Quellkompatibilitaet zu mintlib. Die fruehere Fassung dieses Tickets
  („auf Masken setzen“) und die Lesart „`RW_WRITE` 0 macht Schreiben zum
  Lesen“ sind zurueckgenommen.

## Arbeit

1. **Pruefung (Lead, erledigt 2026-10-28):** die Sonde des Audits
   „BIOS-Speicher“ nutzt Zahlenwerte, keinen Header, auf beiden Seiten
   dieselbe Quelle
   (`measurements/measurement-2026-10-28-bios-storage-audit/method/bstprobe.c`);
   `bios-r2.tsv` ebenso. Kein SDK-Programm und keine portierte Quelle nutzt
   die Header-Konstanten; `sdk/apps/apitest/apitest.c:505@RW_NOTRANSLATE`
   definiert die Maske 8 selbst.
2. Upstream-Werte behalten; im Overlay einen Kommentar „bit numbers, use
   `(1 << RW_x)`“ an die Stelle setzen (benannte Patch-Stelle, Upstream
   unveraendert).
3. Pruefung zur Uebersetzungszeit: `(1 << RW_WRITE) == 1`,
   `(1 << RW_NOMEDIACH) == 2`, `(1 << RW_NORETRIES) == 4`,
   `(1 << RW_NOTRANSLATE) == 8` gegen Werte aus `docs/api/bios/04-rwabs.md`
   (Form waehlt der Developer; aendert sich eine Seite, schlaegt der Bau
   fehl).

## Abnahme

- SDK-Hosttests gruen (`sdk/run-host-tests.sh`); ein absichtlich falscher
  Wert laesst die Pruefung fehlschlagen (im Bericht belegt).
- `cargo xtask test` gruen; doc-check der geaenderten Doku 0 Fehler.
