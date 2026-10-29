# TOOL18 — Quittung: Cookie-Falltabelle, gemessen auf dem Pin (tt, falcontc, disk)

Datum 2026-10-29 · Ticket [TOOL18](TOOL18.md) · Lauf
`measurements/measurement-2026-10-29-tool18/RUNINFO.TXT` · Basis `main` @ 1ca7d09a
plus die ungebuchte Aenderung unten. Kein NewTOS-Produktcode geaendert, keine
Hardware, nichts gebucht.

## 1. Zeilentyp (Arbeit 1)

Spec: `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6, „The cookie jar“ (dazu
Abschnitt 3, Trap-Feld 256, und Abschnitt 7, DUMPBIOS-Zeile).

| Zeile | Inhalt | Klassen |
|---|---|---|
| `jar:<TAG>` | Wert des ersten Eintrags mit dem Tag (wie `Getcookie`), oder „absent“ | `EXACT`, `NONZERO`, `PTR_RAM`, `ABSENT` (neu, nur Jar-Faelle) |
| `jar:*` | Zahl der Eintraege vor dem Terminator und die Tags in Jar-Reihenfolge | `EXACT` |

- `args` und `capture` sind `-`; kein Vorlauf, nicht in einer `Super`-Klammer,
  nicht nach `Getmpb`; `ABSENT` auf einem Trap-Fall wird abgewiesen.
- Kein Trap: der Runner liest das Jar je Fall ueber `td_jar_read` des
  SCRNINFO-Backends (m68k: `Supexec`; NewTOS: Load aus der Seite 0 des
  Prozesses) — die eine Kopie dieses Lesens; beide DUMPBIOS-Bauten linken es.
- Satz: der Aufrufsatz ohne Formatwechsel. `trap` 256, `opcode` 0/1,
  `args[0]` das Tag, `ret_hi:ret` der Wert **null-erweitert** oder
  `0xFFFFFFFF_FFFFFFFF` („absent“). Ein Cookie `0xFFFFFFFF` (`_MCH` auf NewTOS)
  steht also als `0x00000000_FFFFFFFF` und ist von „absent“ getrennt.
  `jar:*`: `ret` = Anzahl, Capture = Tags als big-endian LONGs (max. 64).
- `tosdiff` (`jar_check`): Kopf „case N  jar _MCH …“, Werte als `0x…`/`absent`,
  `jar:*` druckt beide Tag-Listen; die Selbstpruefung wendet `NONZERO`,
  `PTR_RAM`, `ABSENT` auf das Orakel allein an.
- Nicht gemessen: die Kapazitaet des Jars (Wert des Terminators).

Tests: neu `tosdump/tests/test_jar.c` (Form, 18 Abweisungen, Runner gegen das
Stub-Jar: Reihenfolge, Null-Erweiterung, absent, `0xFFFFFFFF`, kein Jar,
70 Eintraege, doppeltes Tag), `stub_abi.{h,c}` (Stub-Jar), `test_tosdiff.sh`
Abschnitt „jar cases“ (EXACT, PTR_RAM, NONZERO, ABSENT, `0xFFFFFFFF` gegen
absent, Reihenfolge, Anzahl, Selbstpruefung).

## 2. Tabelle und Orakel (Arbeit 2)

`tosdump/cases/jar-r1.tsv`, 22 Faelle, CRC `2D0D269D`: `jar:*`, alle Tags aus
EmuTOS `include/cookie.h:26-44` (die von `fill_cookie_jar` gesetzten in dessen
Reihenfolge), dann `NTOS` und `EdDI`.

Referenzen `tosdump/reference/EMUTOS-1a7484ab-BIOS-jar` (tt), `-BIOS-jar-falcontc`,
`-BIOS-jar-disk`: je zwei Laeufe BIN und LOG bytegleich, Selbstpruefung bestanden.
falcontc und disk sind bytegleich.

| Cookie | Klasse | tt | falcontc | disk |
|---|---|---|---|---|
| `jar:*` | EXACT | 12: `_CPU _VDO _FPU _MCH _SWI _SND _FRB _AKP _IDT _FDC XHDI SCSI` | gleich | gleich |
| `_CPU` | EXACT | `0x0000001E` | `0x0000001E` | `0x0000001E` |
| `_VDO` | EXACT | `0x00020000` | `0x00030000` | `0x00030000` |
| `_FPU` | EXACT | `0x00060000` | `0x00000000` | `0x00000000` |
| `_MCH` | EXACT | `0x00020000` | `0x00030000` | `0x00030000` |
| `_SWI` | EXACT | `0x000000BF` | `0x000000BF` | `0x000000BF` |
| `_SND` | EXACT | `0x00000003` | `0x0000001F` | `0x0000001F` |
| `_FRB` | PTR_RAM | `0x00013350` | `0x00013350` | `0x00013350` |
| `_AKP` | EXACT | `0x00000100` | `0x00000100` | `0x00000100` |
| `_IDT` | EXACT | `0x0000112E` | `0x0000112E` | `0x0000112E` |
| `_FDC` | EXACT | `0x01415443` | `0x01415443` | `0x01415443` |
| `XHDI` | NONZERO | `0x00E0F702` | `0x00E0F702` | `0x00E0F702` |
| `SCSI` | PTR_RAM | `0x000050B6` | `0x000050B6` | `0x000050B6` |
| `_FLK __NF _5MS _CF_ _MCF _NET NVDI NTOS EdDI` | ABSENT | absent | absent | absent |

## 3. NewTOS (Arbeit 3)

`cargo xtask tosdump bios tosdump/cases/jar-r1.tsv`, QEMU raspi4b, Kernel
release dieses Baums: Lauf normal beendet, gegen jede der drei Referenzen
**15 von 22 Faellen verschieden** (dieselben 15). Jar: 5 Eintraege
`NTOS _MCH _IDT _AKP EdDI`; `_MCH 0xFFFFFFFF`, `_AKP 0`, `_IDT 0x112F`,
`NTOS 0x100`, `EdDI 1`. Gleich: die sieben ABSENT-Faelle `_FLK __NF _5MS _CF_
_MCF _NET NVDI`. Ergebnis je Fall: RUNINFO, Abschnitt „NEWTOS“.

## 4. Vorschlag fuer `docs/COMPATIBILITY.md` (Arbeit 4, der Lead traegt ein)

Englisch, als Ersatz des Satzes „values from TASK/TOOL18.md once measured“ im
`_MCH`-Eintrag bzw. als eigener Eintrag „Cookie jar vs. the oracle“. Status nach
der heutigen Entscheidungslage: ACCEPTED nur, wo ADR-012 (mit Nachtraegen) oder
ein bestehender Eintrag es traegt; sonst OPEN.

```markdown
**Cookie jar vs. the oracle** — measured 2026-10-29 on the pinned ROM,
table `tosdump/cases/jar-r1.tsv`, references `EMUTOS-1a7484ab-BIOS-jar` (tt),
`-BIOS-jar-falcontc`, `-BIOS-jar-disk`
(`measurements/measurement-2026-10-29-tool18/RUNINFO.TXT`). NewTOS seeds five
cookies (`tos-api/src/cookie.rs:223@seed_defaults`); the oracle's jar has twelve.
falcontc and disk are byte-identical, so their columns are one.

| Case | Cookie | Oracle tt | Oracle falcontc = disk | NewTOS | Status | Reason |
|---|---|---|---|---|---|---|
| 41 | `jar:*` | 12: `_CPU _VDO _FPU _MCH _SWI _SND _FRB _AKP _IDT _FDC XHDI SCSI` | same | 5: `NTOS _MCH _IDT _AKP EdDI` | ACCEPTED | follows from the per-cookie rows; re-measure when the seeded set changes |
| 44 | `_CPU` | `0x0000001E` | `0x0000001E` | absent | ACCEPTED | ADR-012 amendment 2026-10-28: no m68k CPU |
| 45 | `_VDO` | `0x00020000` | `0x00030000` | absent | OPEN | ADR-012 amendment: decided after TASK/ARCH06 (TeraDesk's ST branch, `TASK/ARCH06-STATUS-DONE.MD` §3) |
| 46 | `_FPU` | `0x00060000` | `0x00000000` | absent | ACCEPTED | ADR-012: no m68k FPU |
| 47 | `_MCH` | `0x00020000` | `0x00030000` | `0xFFFFFFFF` | ACCEPTED | ADR-012 amendment 2026-10-28: `MCH_NOHARD` |
| 48 | `_SWI` | `0x000000BF` | `0x000000BF` | absent | OPEN | no decision; EmuTOS adds it only with DIP switches (`bios/machine.c:702-706`) |
| 49 | `_SND` | `0x00000003` | `0x0000001F` | absent | ACCEPTED | ADR-012: no PSG / DMA sound |
| 50 | `_FRB` | `0x00013350` | `0x00013350` | absent | OPEN | no decision; ACSI/SCSI DMA bounce buffer with alt RAM (`bios/machine.c:478`) |
| 51 | `_FLK` | absent | absent | absent | EQUAL | ADR-012 amendment 2026-10-26: not set |
| 52 | `_AKP` | `0x00000100` | `0x00000100` | `0x00000000` | OPEN | oracle = `COUNTRY=de` ROM; NewTOS keeps US (keyboard-layout entry above, difference 2) |
| 53 | `_IDT` | `0x0000112E` | `0x0000112E` | `0x0000112F` | OPEN | oracle = `COUNTRY=de` (`.`); NewTOS `/`, bring-up policy |
| 54 | `_FDC` | `0x01415443` | `0x01415443` | absent | ACCEPTED | ADR-012: no floppy controller |
| 55 | `__NF` | absent | absent | absent | EQUAL | — |
| 56 | `XHDI` | `0x00E0F702` | `0x00E0F702` | absent | OPEN | no decision; NewTOS has no XHDI driver interface |
| 57 | `SCSI` | `0x000050B6` | `0x000050B6` | absent | OPEN | no decision; NewTOS has no SCSI driver interface |
| 58, 61-64 | `_5MS _CF_ _MCF _NET NVDI` | absent | absent | absent | EQUAL | — |
| 67 | `NTOS` | absent | absent | `0x00000100` | ACCEPTED | ADR-012 (+ amendment 2026-10-28): NewTOS identity |
| 68 | `EdDI` | absent | absent | `0x00000001` | ACCEPTED | "The VDI speaks EdDI" (this file): marker, spec E2 |
```

Bilanz: 15 Unterschiede, davon 8 durch bestehende Entscheidungen gedeckt
(`jar:*`, `_CPU`, `_FPU`, `_MCH`, `_SND`, `_FDC`, `NTOS`, `EdDI`), 7 OPEN
(`_VDO`, `_SWI`, `_FRB`, `_AKP`, `_IDT`, `XHDI`, `SCSI`) — die brauchen eine
Anwenderentscheidung, bevor sie ACCEPTED heissen.

## 5. Abnahme

- `tosdiff --selfcheck`: alle drei neuen Referenzen bestehen; `make -C tosdump
  host-test` gruen, Selbstpruefung ueber 40 Referenzen (37 + 3).
- `cargo xtask test`: **1828 passed, 0 failed, 1 ignored** (= Basis).
- Keine Regression des Dumpers: `canary-ptr` (TOOL20) und `canary-disk`
  (TOOL07b) unveraendert ausgefuehrt, beide bytegleich mit den Referenzen.
- doc-check: `docs/TOS-REFERENCE-DUMPER.md` 0, diese Quittung 0. Alle durch die
  Zeilenverschiebung in `cases.c`/`runner_call.c` gebrochenen Anker sind
  nachgezogen (9 Dateien, Liste im RUNINFO). Verbleibend in den Dateien mit
  Ankerkorrektur: 6 **vorbestehende** Fehler in
  `docs/analysis/abi-assumptions-check-2026-10-28.md` (4) und
  `docs/analysis/iorec-survey-2026-10-28.md` (2), alle auf `sdk/`-Dateien, die
  SDK04/SDK05 verschoben haben, keiner auf einer geaenderten Zeile.

## 6. Hinweise

- `docs/COMPATIBILITY.md`, `docs/STATUS.md`, `TASK/TOOL18.md` nicht angefasst (Lead).
- Die `-disk`-Referenz ist bytegleich mit `-falcontc`; sie bleibt als eigene
  Datei, damit die Selbstpruefung jede Profil-Referenz einzeln fuehrt.
