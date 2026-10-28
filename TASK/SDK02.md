# SDK02 — `RW_*`-Konstanten in mintlibs `ostruct.h` sind Bitnummern, keine Masken

Status: **OFFEN** (Punkt 1 erledigt 2026-10-28) · Prioritaet: **P1** · Typ: Developer (SDK) — Anwenderentscheidung 2026-10-28, Nebenbefund aus [BIOS02](BIOS02.md)

## Sachverhalt

- `sdk/libc/overlay/include/mint/ostruct.h:240-245@RW_NOTRANSLATE` (Kommentar „Mode bitmask
  used in Rwabs()“): `RW_READ 0`, `RW_WRITE 0`, `RW_NOMEDIACH 1`,
  `RW_NORETRIES 2`, `RW_NOTRANSLATE 3`. Die Overlay-Datei ist eine Kopie des
  Upstream-Headers; mintlib 0.60.1 (`sdk/libc/mintlib/`, nicht versioniert)
  hat dieselben Werte.
- Spec und EmuTOS: Masken `RW_RW 1`, `RW_NOMEDIACH 2`, `RW_NORETRIES 4`,
  `RW_NOTRANSLATE 8`, `RW_NOBYTESWAP 0x80`
  (`references/emutos/bios/disk.h:52-61`, `docs/api/bios/04-rwabs.md`).
  Wer mit dem Header `Rwabs(RW_WRITE|RW_NOTRANSLATE, …)` schreibt, schickt 3
  = Schreiben + „kein Medienwechsel-Test“ und liest/schreibt logisch statt
  physisch; `RW_WRITE` 0 macht jeden Schreibaufruf zum Lesen.

## Arbeit

1. **Pruefung vor BIOS01 (Lead, erledigt 2026-10-28):**
   - Die Sonde des Audits „BIOS-Speicher“ nutzt keinen Header, sondern
     Zahlenwerte (2, 4, 0x80, 3, 11) — dieselbe Quelle auf beiden Seiten
     (`measurements/measurement-2026-10-28-bios-storage-audit/method/bstprobe.c`,
     Zeilen 216-283). `bios-r2.tsv` ebenso. Die flagabhaengigen Befunde sind
     also keine Artefakte; keine Neumessung noetig.
   - Nutzer im SDK und in portierten Quellen (`sdk/`, `programs/`,
     `sdk/overlays/`): keiner der Header-Konstanten. `sdk/apps/apitest/apitest.c:517@RW_NOTRANSLATE`
     definiert `RW_NOTRANSLATE 0x08` selbst (richtig). Kernel- und
     Hosttests nutzen eigene Konstanten mit Wert 8.
2. Overlay-`ostruct.h` auf die Masken der Spec setzen (`RW_READ 0`,
   `RW_WRITE 1`, `RW_NOMEDIACH 2`, `RW_NORETRIES 4`, `RW_NOTRANSLATE 8`,
   `RW_NOBYTESWAP 0x80` falls im Header), als benannte Patch-Stelle im Overlay
   (Upstream bleibt unveraendert).
3. Pruefung zur Uebersetzungszeit gegen `docs/api/`: die Werte kommen aus
   einer Quelle, die gegen die Spec geprueft wird (z. B. ein
   `_Static_assert`-Test im SDK-Hostlauf gegen Werte, die ein xtask-Test aus
   `docs/api/bios/04-rwabs.md` liest — Form waehlt der Developer, Hauptsache:
   aendert sich eine Seite, schlaegt der Bau fehl).
4. `apitest.c` danach auf den Header umstellen ist optional; im Bericht
   nennen.

## Abnahme

- SDK-Hosttests gruen (`sdk/run-host-tests.sh`, Basislinie 26/0/0 plus neue);
  ein absichtlich falscher Wert laesst die Pruefung fehlschlagen (im Bericht
  belegt).
- `cargo xtask test` gruen; doc-check der geaenderten Doku 0 Fehler.
