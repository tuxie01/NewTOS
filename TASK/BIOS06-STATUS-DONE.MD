# BIOS06 — Quittung: `Getmpb` liefert das LP64-`_MD`/`_MPB` (O1 = A), `Getmpb(NULL)` gemessen (O3)

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-bios06/](../measurements/measurement-2026-10-29-bios06/RUNINFO.TXT)
· Ticket [BIOS06](BIOS06.md) · Spec `docs/api/bios/00-getmpb.md`
(„NewTOS-Vertrag [BIOS06]“ V1-V5), `docs/api/structures/md.md` · EmuTOS
`references/emutos/bios/biosmem.c:134-151`.

## O3 zuerst: EmuTOS bei `Getmpb(NULL)`

Gemessen auf dem Orakel, Profile tt und falcontc gleich (Sonde
`GNPROBE`, Gruppe N): die erste Speicherung `mpb->mp_mfl = &themd`
(`bios/biosmem.c:149`, im ROM `_getmpb+0x2C` = `move.l a1,(a0)`, PC
`$E027E6`) loest einen Busfehler im Supervisorzustand aus; EmuTOS meldet
`Panic: Bus Error ... addr=00000000` und wartet auf eine Taste. Das Programm
kehrt nicht zurueck, das System steht. NewTOS: `Getmpb(NULL)` liefert 0,
schreibt nichts, das Programm laeuft weiter. Nach der Entscheidung O3 = (a)
bleibt der stille Erfolg; die Messung steht in der Spec (V3, O3, Error
conditions) zur Entscheidung des Anwenders.

## Aenderung

- `tos-api/src/bios/getmpb.rs`: eigener oeffentlicher Deskriptor `PublicMd`
  (`_MD`, 32 Byte: `md_next`/`md_start`/`md_length`/`md_owner` je 8) und
  `PublicMpb` (`_MPB`, 24 Byte); `offset_of!`/`size_of`-Pruefungen mit den
  Zahlen der Spec (`tos-api/src/bios/getmpb.rs:69-78@offset_of`); `THEMD`
  haelt einen `PublicMd`, `describe` fuellt ihn aus `[membot, memtop)`; die
  Offsets im Block des Aufrufers aus `PublicMpb`. Geschrieben werden nur
  `mp_free`/`mp_used`, `mp_rover` bleibt. Das interne BDOS-`MD`
  (`tos-types/src/memory.rs:61@MD`) ist unveraendert.
- `kernel/src/selftest_bios.rs`: der (nirgends verdrahtete) `Getmpb`-Selbsttest
  liest jetzt `PublicMpb`/`PublicMd` statt des internen `MD`.
- `sdk/apps/apitest/apitest.c`: `MDX`/`MPBX` entfernt, B3 nimmt `_MD`/`_MPB`
  aus dem Header (`sdk/apps/apitest/apitest.c:2188@_MPB`).

## Abnahme

- Fall 34 `tosdump/cases/xbios-ptr-r1.tsv`: vorher DIFF gegen beide
  (`md_start` `0x00A00000_00400000`, `md_length` 0), nachher **0 von 9
  Faellen verschieden** gegen tt und gegen falcontc (`md_start` 0x400000,
  `md_length` 0xA00000).
- Sonde G00-G06: NewTOS jetzt in jeder Relation wie das Orakel (G04 alle
  fuenf 1, G03 `start+length = memtop`). Die kopierte Sonde liest die
  Systemvariablen jetzt 32 Bit breit (die alte las unter LP64 8 Byte und
  machte G04 `start_is_membot` auf NewTOS immer 0).
- Kein Rueckschritt: `xbios-ptr-r1`, `bios-r1`, `xbios-chardev-r1`,
  `bios-core-r1`, `xbios-system-r1` je gegen tt und falcontc, vorher
  (`git archive 90f409ab`, `--tree`) und nachher: kein neu abweichender Fall
  in allen zehn Vergleichen; einzige Aenderung Fall 34 (DIFF -> gleich).
- Beide Boards booten: alle raspi4b-Laeufe bis zum Desktop; qemu-virt
  (Debug) bis „desktop up (0 drives)“.

## Tests, Bau

- `cargo xtask test` (voll): 1842/0/1 (Basis 1840/0/1 + 2 neue Hosttests in
  `getmpb.rs`: Inhalt des Deskriptors, Bytelage im LP64-Layout).
- `sdk/run-host-tests.sh`: 33/0/0. `make -C tosdump host-test`: gruen
  (40 Referenzen, layoutcheck 5 Layouts).
- Bau rpi4 release und qemu-virt: Warnungen kernel 93 / tos-api 64 (= Basis).
  Clippy kernel 121 / tos-api 74 auf beiden Boards, je Datei gleich der
  Basis (HEAD-Fassung der zwei Rust-Dateien voruebergehend eingesetzt und
  zurueckkopiert); host `clippy -p tos-api --tests` 112 Warnzeilen = Basis.
  `apitest.c`: 0 Warnungen mit `-Wall -Wextra` (wie vorher), baut und linkt.
- `mapping.tsv`: `getmpb` -> `getmpb` (Notiz mit NULL-Verhalten) und neu
  `getmpb` -> `describe`; `cargo xtask deps --check-mapping` gruen;
  `deps getmpb --down` vollstaendig. `--up --layers bios,bdos`: offen bleiben
  `bios_init`, `biosmain` (Start, wie lokalisiert).
- doc-check 0 Fehler: `docs/api/bios/00-getmpb.md`,
  `docs/api/structures/md.md`, `docs/sdk/05-abi-pitfalls.md`,
  `TASK/BIOS06.md`, `TASK/SDK02.md`. In den datierten Dokumenten
  (`docs/audits/2026-10-28-bios-core.md`, `docs/analysis/*-2026-10-28.md`)
  sind nur die durch diese Aenderung verschobenen Anker und das entfernte
  `MDX` berichtigt; dort bleiben 18 Altfehler anderer Dateien (`loader.rs`,
  `bios/mod.rs`, `bpb.rs`, `sysvars.h`, `sysinfo.c`, `cookie.c`) unberuehrt.

## Nicht gemacht / offen

- Kein Lauf von APITEST (liegt auf keiner Karte); keine echte Pi.
- `docs/STATUS.md` nicht bearbeitet (Lead).
- `sdk/libc/overlay/include/mint/ostruct.h` Kommentar „the kernel side
  follows in TASK/BIOS06“ ist jetzt Vergangenheit, nicht angefasst.
