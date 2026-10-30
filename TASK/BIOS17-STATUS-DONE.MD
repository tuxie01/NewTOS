# BIOS17 — Quittung: `swv_vec` haelt den Reset-Einstieg; `RESET_STUB_*` → `RESET_ENTRY_*`

Datum 2026-10-30 · Beleg
[measurements/measurement-2026-10-30-bios17/](../measurements/measurement-2026-10-30-bios17/RUNINFO.TXT)
· Ticket [BIOS17](BIOS17.md) (Anwenderentscheidung 2026-10-30) · EmuTOS
`references/emutos/bios/bios.c:363`, `references/emutos/bios/bios.c:552` · ADR-031 (`docs/MEMORY.md`)

## 1. `swv_vec` vorbelegt

- **Lader:** `tos-api/src/bdos/process/loader.rs:1242@RESET_ENTRY_VA` (`populate_sysvars`) schreibt in
  jede Seite 0 bei `$46E` den Reset-Einstieg, denselben Wert wie OSHEADER `reseth`/`os_res1`. EmuTOS
  setzt `swv_vec = just_rts` und am Ende von `bios_init` unbedingt `swv_vec = os_header.reseth`.
- **Hook-Pruefung:** `tos-api/src/bdos/process/hooks.rs:284@SwvVec` — Startwert in `PAGE0_HOOKS` ist
  `RESET_ENTRY_VA as u32`; die BIOS15-Warnung schweigt fuer den Startwert, jede andere Belegung (auch
  0) warnt wie bisher. Nie aufgerufen (kein Monitorwechsel, ADR-031).
- **Hosttest neu:** `swv_vec_is_reseth_the_reset_entry` (`loader.rs`, Testmodul): `$46E` = `reseth`
  der OSHEADER-Instanz = `RESET_ENTRY_VA`, nicht `just_rts`, und gleich dem `PAGE0_HOOKS`-Startwert.
  Der bestehende Test `the_page_0_hook_defaults_are_what_the_loader_writes` deckt Lader = Startwert ab.
- **Testvorrichtungen angepasst (keine Zusicherung geaendert):** `tos-api/tests/bios07_setexc.rs`,
  `bios15_os_hooks.rs`, `bios16_page0_write.rs` bauten Seite 0 aus Nullen plus Vektorzellen; mit dem
  neuen Startwert warnte jede Pruefung `swv_vec = 0` (8 Tests rot). Die Vorrichtung schreibt jetzt
  zusaetzlich die `PAGE0_HOOKS`-Startwerte, wie der Lader — Lead: bitte bestaetigen, das Ticket sagt
  „Hosttests unveraendert ausser dem neuen Test“; Anzahl und Zusicherungen sind unveraendert.

## 2. Umbenennung

`RESET_STUB_VA` → `RESET_ENTRY_VA`, `RESET_STUB_CODE` → `RESET_ENTRY_CODE`; „reset stub“ → „reset
entry“ in Code-Kommentaren (`tos-api/src/bios/reset.rs`, `bios/vectors.rs`, `bdos/process/loader.rs`,
`bdos/process/mmu.rs`, `gemdos/ssystem.rs`, `host.rs`, `kernel/src/reset.rs`, Tests `bios07_setexc.rs`,
`bios_reset.rs`) und in den Specs (`docs/api/bios/05-setexc.md`, `docs/api/structures/osheader.md`,
`docs/api/gemdos/154-ssystem.md`, `docs/api/_super-state.md`, `docs/api/bios/_reset-warm-cold.md`,
`docs/COMPATIBILITY.md`). Kein Verhaltenswechsel, gemessen: ein Kern nur mit der Umbenennung ist
bytegleich mit dem von `f2652440` (`bc15a3db…`). `mapping.tsv` nannte die Konstanten nicht.

Bewusst **nicht** umbenannt: `RESET_STUB_AREA_VA`/`RESET_STUB_AREA_SIZE` (der Bereich in der
Stub-Seite, Reihe mit `KBDVEC_STUB_AREA_*`), `write_stub`, der Testname
`the_reset_stub_calls_ssystem_2021`, `mmu::STUB_PAGE_VA`; das Zitat „the Reset stub reads memvalid“
in `docs/api/_super-state.md:18` (Anwenderwort); `TASK/` und Messordner (Geschichte).

## Abnahme

- **`vwprobe` V01:** vorher `is-reseth=0 … is-0=1`, nachher `is-reseth=1 … is-0=0` = Orakel tt und
  falcontc (BIOS16-Lauf). Uebrige Zeilen unveraendert; der Rest-Unterschied zum Orakel bleibt E01/T02
  (dokumentiert seit BIOS16). `[hook]`-Zeilen vorher = nachher.
- **`sxprobe`** vorher = nachher = Orakel; **`hkprobe`** vorher = nachher (Log und `[hook]`);
  **MEM12 P0PAR** „P0PAR: PASS“, Zeilen gleich dem BIOS16-Lauf. qemu-virt bis „desktop up“.
- **Hosttests:** `cargo xtask test` **1893 / 0 / 1** (Basis 1892/0/1, +1).
- **Bau/Lint:** beide Boards release; rustc kernel 93 / tos-api 64, Clippy 121 / 74 auf beiden Boards
  = Basis, Warnungsmenge je Datei gleich dem BIOS16-Lauf. `clippy -p tos-api --tests`: keine Warnung in
  den geaenderten Tests.
- **`mapping.tsv`:** +1 `bios_init` → `populate_sysvars` (die `swv_vec`-Haelfte); `cargo xtask deps
  --check-mapping` 1066 Zeilen, alle aufgeloest.
- **doc-check:** 0 Fehler in allen bearbeiteten Docs. Durch die neuen Zeilen verschobene Anker
  nachgezogen: `docs/MEMORY.md` (`note`), `docs/STATUS.md` (`check_page0`),
  `docs/api/bios/05-setexc.md` (`write_page0_vectors`, `before_trap`, `note`),
  `docs/sdk/05-abi-pitfalls.md` (`refresh_cookie_mirrors`, `wr_long`).

## Offen / nicht geprueft

- Echte Pi 4. Orakelseite nicht neu gelaufen (BIOS16-Laeufe, gleicher Pin, gleiche Sonde).
- Testvorrichtungen (oben) — Bestaetigung durch den Lead.
- Ausserhalb: `0xA40` als Ort des Reset-Einstiegs in `docs/api/structures/osheader.md`,
  `docs/api/bios/_reset-warm-cold.md` §5 und `docs/api/gemdos/154-ssystem.md` ist seit MEM12 veraltet
  (Stub-Seite `0x7E000` + `0x80`); `kernel/src/reset.rs:15@S_NEWTOS_RESET` sagt „page-0 reset entry“. Nicht geaendert.
- Anker in `TASK/BIOS15*.md`, `TASK/BIOS16-STATUS-DONE.MD` auf `hooks.rs`/`loader.rs` sind verschoben
  (Tickets/Quittungen = Geschichte, nicht geaendert).
