# XBIOS05 — Quittung: `Bconmap` hat die Tabelle mit einem Eintrag wie EmuTOS auf einer Maschine ohne Falcon/TT

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-xbios05/](../measurements/measurement-2026-10-29-xbios05/RUNINFO.TXT)
· Spec `docs/api/xbios/2c-bconmap.md` (neu geschrieben, Abschnitt „NewTOS
implementation“), Layout `docs/api/structures/bconmap.md` (neu) · Ticket
[XBIOS05](XBIOS05.md), Anwenderentscheidung 2026-10-28 Option 1

- **`Bconmap`** (`tos-api/src/xbios/bconmap.rs:222@bconmap`, Arm
  `tos-api/src/xbios/mod.rs:98@bconmap`; vorher konstant `0x2c` in
  `tos-api/src/xbios/system.rs`): `-1` = 6, `-2` = Zeiger auf
  `BCONMAP {maptab, 1, 6}`, `6` = 6 mit dem Rueckspeichern des Eintrags
  (`references/emutos/bios/serport.c:1104-1110,1121-1122`), jedes andere Geraet
  0 (`:1095-1097`; `WORD`-Arithmetik fuer `map_index` wie EmuTOS).
- **Ort und Layout:** `BCONMAP` (16 Byte), `MAPTAB[0]` (48 Byte) und fuenf
  Aufrufziele in der IOREC-Seite bei `0x7D080`
  (`tos-api/src/xbios/iorec.rs:130@BCONMAP_AREA`), in jedem Prozess EL0
  les-/schreibbar, nie ausfuehrbar, mit den Records geschrieben. LP64 wie das
  SDK-`_BCONMAP`/`_MAPTAB`; `mapped_device` liegt bei 10 im Padding des
  SDK-Typs. `MAPTAB[0].Iorec` = Record von `Iorec(0)`.
- **Funktionszeiger nicht aufrufbar (Entscheidung):** jeder der fuenf zeigt
  auf sein eigenes Ziel im nie ausfuehrbaren Bereich
  (`tos-api/src/xbios/bconmap.rs:129@vector_va`); ein Aufruf ist ein
  Instruction Abort, der Crash-Bericht nennt „Memory violation: Bconmap
  MAPTAB function (not callable) memory, execute (permission fault, level
  3)“ (`tos-api/src/bios/crash.rs:196@BconmapVector`, Einordnung in
  `tos-api/src/bdos/process/memprot.rs:1977@region_kind`), danach `Pterm(-1)`
  ueber den bestehenden Absturzpfad. Gemessen mit der Sonde BMCALL
  (Serielle und Screendump im Lauf).
- **Kernzustand:** `Bconmap` liest die Seite nie zurueck; Schreiben eines
  Programms in `maptab`/`maptabsize`/`mapped_device` aendert, was es liest,
  nicht was `Bconmap` tut (EmuTOS liest live). Dokumentiert in Spec und
  `docs/COMPATIBILITY.md`.
- **Spec zuerst:** `docs/api/xbios/2c-bconmap.md` auf Option 1 umgestellt (die
  „needs deeper analysis“-Frage nach dem Gate von `init_bconmap` beantwortet:
  `references/emutos/bios/serport.c:944,1056-1059`, `BCONMAP_AVAILABLE`);
  neues `docs/api/structures/bconmap.md` mit `tosdump-layout`-Block.
  `docs/COMPATIBILITY.md`: neuer Eintrag „`Bconmap`: one entry, the AUX port,
  device 6“ (Falcon `7`, `maptabsize` 4/3, `Bconmap(7..9)` ACCEPTED je
  Profil), MAPTAB-Eintrag konkretisiert. Kopien der alten
  „`0x2c`“-Begruendung nachgezogen: `docs/HARDWARE-PROFILE.md`,
  `docs/api/xbios/_state.md`, `docs/api/xbios/0e-iorec.md`,
  `docs/api/bios/_state.md`, `docs/analysis/emutos-full-function-inventory.md`,
  `docs/analysis/bios-audit/bios-xbios-feasibility-triage.md`.
- **SDK-Layoutpruefung:** `sdk/libc/tests/bconmap-layout.c` (+ `.sh`, in
  `sdk/run-host-tests.sh`): `_Static_assert` fuer `_MAPTAB` (48, sechs
  Offsets) und `_BCONMAP` (16, `maptab` 0, `maptabsize` 8) und die
  EmuTOS-Dreifeldform (`mapped_device` 10); Gegenprobe mit falschen Offsets
  schlaegt fehl. Header unveraendert.
- **Neue Orakeltabelle** `tosdump/cases/xbios-bconmap-r1.tsv` (1 Fall,
  CRC `DD429BF8`): `Bconmap(-2)` `r:bconmap` = 8 Byte `BCONMAP` + 24 Byte
  `MAPTAB[0]` (m68k). Referenzen
  `tosdump/reference/EMUTOS-1a7484ab-XBIOS-bconmap{,-falcontc}.{BIN,LOG}`, je
  zweimal bytegleich, Selfcheck gruen; `tosdump/cases/COVERAGE-BIOS.md`
  ergaenzt.
- **Ratsche** `docs/analysis/deps/mapping.tsv`: `bconmap` →
  `bconmap.rs` `bconmap` + `store_entry`; `init_bconmap` (Tabellenhaelfte)
  `unmapped` → `bconmap.rs` `init`; `rsconf_dummy` Notiz (keine freie Zeile).
  `deps --check-mapping` gruen, `deps bconmap --up` vollstaendig.
- **Verschobene Anker** (nur Zeilennummern, 38 Stellen) in Spec-, Analyse-,
  Audit- und TASK-Dateien nachgezogen; die sechs Zitate der
  alten `bconmap` in `tos-api/src/xbios/system.rs` (Zeile 106) auf „bis XBIOS05, seitdem `bconmap.rs`“ umgestellt.

## Abnahme

- `bios-r1` 183/185/186 und `xbios-chardev-r1` 43, 45–53: EQUAL gegen tt;
  gegen falcontc EQUAL bis auf 183 und 43 (`Bconmap(-1)` Falcon 7 / NewTOS 6,
  ACCEPTED). `bios-r1` 184 / `xbios-chardev-r1` 44 (`Bconmap(-2)`) EQUAL
  (PTR_RAM, vorher „below 0x400“).
- `xbios-bconmap-r1` 18: alle Zeiger EQUAL (PTR_RAM); verschieden nur
  `maptabsize` (tt 4, falcontc 3, NewTOS 1) und `mapped_device` (falcontc 7)
  — Geraeteparameter, ACCEPTED.
- Sonde B02–B04: `Bconmap(-2)` = `0x7D080`, `tosaddr` 1, `maptab` gueltig,
  `maptabsize` 1, `mapped_device` 6; B05 Eintrag 0 wie EmuTOS. B01–B24
  sonst gleich EmuTOS bis auf die fehlenden Ports 7–9 (B13–B15, B17, B20–B23).
- Kein vorher gleicher Fall in `bios-r1`, `bios-core-r1`, `xbios-chardev-r1`,
  `xbios-system-r1`, `xbios-ptr-r1` neu verschieden (zehn Vergleiche, vorher
  = `git archive f7e80033`).
- Wiedertest `Iorec(0)`: `xbios-ptr-r1` 24 EQUAL vorher und nachher, Sonde
  B15/B19 und Gruppe I unveraendert. `Rsconf`: `xbios-chardev-r1` Rsconf-Faelle
  unveraendert (31 bleibt DIFF, XBIOS06), Sonde B16 und Gruppe S unveraendert.
- `cargo xtask test` 1881/0/1 (Basis 1872/0/1: +6 Integration
  `tos-api/tests/xbios05_bconmap.rs`, +3 Unit). `sdk/run-host-tests.sh`
  35/0/0 (Basis 34). `make -C tosdump host-test` gruen (43 Referenzen,
  7 Layouts). Build beide Boards 159 Warnzeilen wie Basis (kernel 93 /
  tos-api 64), Clippy kernel 121 / tos-api 74 beide Boards wie Basis;
  Clippy `tos-api --tests` ohne Warnung in den beruehrten Dateien.

## Fuer BIOS08

Mit `maptabsize` 1 gilt nach EmuTOS (`references/emutos/bios/bios.c:923-928`,
`:967-972`, `:1000-1005`, `:1152-1157`): Handle 6 = Eintrag 0 = AUX, also
dieselben Antworten wie Handle 1 (raspi4b: `Bconstat` 0, `Bconin` 0 sofort,
`Bcostat` -1, `Bconout` 1; qemu-virt: die serielle Leitung); Handles 7, 8, 9
und hoeher liefern fuer alle vier Funktionen 0. Gegen das Orakel hiesse das:
`bios-r1` 69 `Bcostat(6)` -1 = EQUAL; 70 `Bcostat(7)` 0 gegen -1, `bios-core-r1`
65 `Bcostat(8)` 0 gegen -1 und 66 `Bcostat(9)` 0 gegen TT -1 / Falcon 0 —
Geraeteparameter wie hier, zu entscheiden in BIOS08. `Bconin(6)` wartet auf
EmuTOS TT (HANG-6), auf NewTOS raspi4b nicht (AUX ohne Kabel). Heute sind
6/7 `Absent` und 8/9 jenseits `NDEV` (`tos-api/src/bios/chardev.rs` `DEVICES`),
unveraendert.

## Offen / nicht geprueft

- qemu-virt nur gebaut und gelintet, nicht gebootet (Tester).
- Echte Pi 4: keine hardwarespezifische Aenderung; nicht gelaufen.
- Nach dem BMCALL-Crash-Bericht die Taste und das `Pterm(-1)`: nicht im
  Headless-Lauf getrieben (bestehender Pfad).
