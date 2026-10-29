# BIOS07 — Quittung: `Setexc` antwortet aus Seite 0 wie EmuTOS

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-bios07/](../measurements/measurement-2026-10-29-bios07/RUNINFO.TXT)
· Ticket [BIOS07](BIOS07.md) (Anwenderentscheidungen 2026-10-28 D1–D3, P3) · Spec
`docs/api/bios/05-setexc.md` („Pi / BCM2711 binding“, „Decisions“) · EmuTOS
`references/emutos/bios/bios.c:1070-1080`

## Fix

- `tos-api/src/bios/vectors.rs:114@setexc` — drei Bereiche, Seite 0 traegt jedes `num < 0x200` (D3):
  - `num <= 0x103` (Vektoren, `etv_*`): eine Zelle fuer alle Prozesse
    (`tos-api/src/bios/vectors.rs:90@VECTORS`, jetzt 32 Bit), jede geladene Seite 0 traegt ihre
    Kopie an `4*num` — geschrieben vom Loader (`tos-api/src/bios/vectors.rs:181@write_page0_vectors`,
    aufgerufen aus `tos-api/src/bdos/process/loader.rs:1304@write_page0_vectors`) und von jedem
    Schreibzugriff (`tos-api/src/bios/vectors.rs:166@store_shared`, auch der Owner-Restore aus
    ADR-031 Regel 3). Die Zelle bleibt eine fuer alle Prozesse, weil ADR-031 sie als geteilte Zelle
    mit Eigentuemer fuehrt und EmuTOS eine Vektortabelle hat.
  - `0x104 … 0x1FF`: die Seite-0-Zelle des aufrufenden Prozesses (vorher erst ab `0x120`; der
    Schatten `NVEC = 0x120` ist weg) — BIOS-CORE-006.
  - `>= 0x200` und negatives `num`: 0 (D2, unveraendert).
- Startwerte (`tos-api/src/bios/vectors.rs:79@default_of`): Vektoren 2…0xFF und
  `etv_timer`/`etv_critic`/`etv_term` = der gemeinsame `ret`-Stub (`JUST_RTS` = EmuTOS
  `just_rts`, 0x7E040) — BIOS-CORE-004/005; Vektor 0/1 = die ersten zwei LONGs des OSHEADER
  (`os_entry`/`os_version`, dann `reseth`), wie EmuTOS `vecs_init` sie nach 0 kopiert
  (`references/emutos/bios/bios.c:120-136`; `tos-api/src/bdos/process/loader.rs:1089@OSHEADER_LONG0`);
  `etv_xtra` 0. Die Platzhalter `ETV_DEFAULT_JUST_RTS`/`ETV_DEFAULT_CRITIC` sind entfernt.
- Breite: Antwort = 32-Bit-Zelle vorzeichenerweitert (D1, `tos-api/src/bios/vectors.rs:101@long`);
  ein Schreiben speichert die unteren 32 Bit; der Abfragetest `-1` prueft die unteren 32 Bit (P3) —
  erst nach der Sonde geaendert (unten).
- ADR-031: `tos-api/src/bdos/process/hooks.rs:311@check_page0` vergleicht die Seite-0-Kopien
  `$008`–`$408` mit ihrer geteilten Zelle statt mit 0 (sonst warnte jeder Prozess nach jedem
  `Setexc`); `etv_*` deshalb aus `PAGE0_HOOKS` genommen, Zuordnung in `page0_vector_hook`.
  Eine direkt geschriebene Kopie aendert die Zelle nicht und warnt (Regel 4) — dokumentierte
  Abweichung (`docs/api/bios/05-setexc.md` „Page 0 written directly“, `docs/COMPATIBILITY.md`
  „`Setexc` and page 0“).
- Kern: `kernel/src/arch/aarch64/exceptions.rs:1396@JUST_RTS` — `Pterm` ruft `etv_term` nicht,
  solange es `JUST_RTS` (oder 0) haelt (Wirkung wie EmuTOS' Aufruf von `just_rts`);
  `kernel/src/selftest_bios.rs`, `kernel/src/selftest_etv_term.rs` (nicht verdrahtet) auf die neuen
  Namen und einen Testwert < 2 GiB umgestellt.

## Abnahme

- **P3 zuerst gemessen** (dauerhafte Sonde `tosdump/probes/sxprobe.c`, Bau
  `make -C tosdump sxprobe-m68k sxprobe-newtos`, Lauf `tosdump/probes/run-sxprobe.sh`): EmuTOS tt
  und falcontc behandeln `Setexc(0x20, 0x00000000FFFFFFFF)` als Abfrage (Q03/Q04 `80000001`),
  NewTOS vorher als Schreiben (Q04 `ffffffff`). Danach: `SXPROBE.LOG` byte-gleich mit beiden
  Orakelprofilen; Eingabepruefung mit `bios-arg-trace` (`l:0xffffffff=0x00000000ffffffff` kommt an
  und wird als Abfrage beantwortet).
- **Tabellen:** `bios-core-r1` 46–62 und `bios-r1` 53–57 EQUAL gegen tt und falcontc
  (`bios-core-r1` tt 15 → 2, falcontc 14 → 1 verschieden; `bios-r1` tt 39 → 37, falcontc 43 → 41).
  Keine neue Abweichung in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1` (12 → 12),
  `xbios-system-r1` (9 → 9), alle acht Vergleiche fallweise verglichen. Rest in `bios-core-r1`:
  65/66 `Bcostat(8/9)` = BIOS-CORE-007 ([BIOS08](BIOS08.md)).
- **Sonden S01–S08** (BCPROBE.NAP des Audits): nachher zeilengleich mit dem Orakel tt und falcontc.
- **Nachgeprueft:** BIOS15 (`hkprobe`): Log vorher = nachher, dieselben vier Warnungen (die
  `Mfpint`-Zeilen nennen jetzt den Vektorwert 0x7e040 statt 0x0). MEM12 (P0PAR mit
  Injektor): alle Faelle wie in der MEM12-Abnahme, „P0PAR: PASS“. Boot qemu-virt bis „desktop
  up“; raspi4b bootet in jedem Lauf.
- **Hosttests:** `cargo xtask test` **1859 / 0 / 1** (Basis 1850/0/1, +9): neu
  `tos-api/tests/bios07_setexc.rs` (8 Tests: Rueckgabe je Vektorklasse — CPU-Vektoren, `etv_*`,
  `etv_xtra`, Systemvariablen `0x104…0x1FF`, `num >= 0x200`/negativ; MEM11-Invariante „jede
  gelieferte Adresse < 2 GiB“ ueber alle geteilten Zellen; Seite 0 traegt jede Zelle in jedem
  Prozess; direkte Kopie aendert die Zelle nicht; P3 und D1) und Loader-Test
  `page0_carries_the_vector_cells`; angepasst `tos-api/tests/bios15_os_hooks.rs` (neue
  Startwerte, `0x108` jetzt Seite 0) und `the_page_0_hook_defaults_are_what_the_loader_writes`.
- **Bau/Lint:** rpi4 und qemu-virt release bauen; rustc-Warnungen kernel 93 / tos-api 64 wie Basis
  (159 Warnzeilen je Board); Clippy kernel 121 (beide Boards) / tos-api 74, je Datei identisch zur
  Basis (`git archive 40e06519` in Scratch); keine Warnung in den beruehrten Dateien und Tests.
- **doc-check:** 0 neue Fehler; verschobene Anker nachgezogen in `docs/MEMORY.md` (ADR-031,
  `hooks.rs` `note`), `docs/sdk/05-abi-pitfalls.md` (Loader), `docs/api/bios/_state.md`,
  `docs/audits/2026-10-28-bios-core.md` (entfernte Symbole als „seit BIOS07 entfernt“, wie dort
  ueblich), `TASK/BIOS05.md`, `TASK/BIOS15.md`, `TASK/BIOS15-STATUS-DONE.MD`, `TASK/BIOS07.md`.
- **`mapping.tsv` (Ratsche):** neu `setexc` → `store_shared`, `vecs_init` → `write_page0_vectors`
  (der Teil „OSHEADER nach $000/$004“; die Design-Zeile fuer die m68k-Vektoren bleibt),
  `bios_init` → `default_of` (der `etv_*`-Teil; `bios_init` bleibt insgesamt `unmapped`);
  `default_etv_critic`-Notiz berichtigt. `cargo xtask deps --check-mapping`: 1051 Zeilen, alle
  aufgeloest.

## Nachgezogen

`docs/api/bios/05-setexc.md` (Pi-Binding neu, Startwert-Tabelle, Decisions als umgesetzt markiert,
Luecken), `docs/COMPATIBILITY.md` (neuer Absatz „`Setexc` and page 0“, OS-Hook-Absatz, LONG-Abschnitt
ohne die BIOS07-Abweichung, Line-A-Satz), `docs/api/_line-a.md`, `docs/api/bios/_state.md`,
`TASK/MEM12.md` (Tabelle und E5 mit Nachtrag), `TASK/BIOS13.md` (Code-Kommentar-Punkt erledigt),
`docs/STATUS.md` (Basislinie, Warteschlange, Recent Decisions; aelteste Zeile nach
`docs/archive/status-log-2026-10-26.md`), `tools/hatari-oracle.sh` (Sondenname `SXPROBE.PRG`),
`tosdump/Makefile`.

## Offen / nicht geprueft

- Echte Pi 4.
- Vektoren 0/1: der Wert folgt der EmuTOS-Quelle (`vecs_init`), nicht gemessen (kein Tabellenfall).
- Ein Programm, das die Seite-0-Kopie einer Vektorzelle direkt schreibt und danach `Setexc` fragt,
  sieht die Zelle, nicht seinen Wert (dokumentierte Abweichung; nur Hosttest).
- Folgeticket nach CLAUDE.md „Testing“ fuer den Sondenfall P3 in tosdiff: als Tabellenzeile geht
  das Setzen nicht (DUMPBIOS erlaubt bei `Setexc` nur die Abfrage, `tosdump/cases/bios-core-r1.tsv`);
  Entscheidung beim Lead.
