# BIOS16 — Quittung: direkt geschriebene Vektorzelle `$008`–`$407` wird die Zelle

Datum 2026-10-30 · Beleg
[measurements/measurement-2026-10-30-bios16/](../measurements/measurement-2026-10-30-bios16/RUNINFO.TXT)
· Ticket [BIOS16](BIOS16.md) (Anwenderentscheidung 2026-10-30 nach dem Bericht: Option B,
Besitz ja, Umfang `$008`–`$408`) · Spec `docs/api/bios/05-setexc.md` („Page 0 written directly“)
· ADR-031 (`docs/MEMORY.md`) · EmuTOS `references/emutos/bios/bios.c:1070-1080`,
`references/emutos/bdos/proc.c:607-608`

## Fix

- **Uebernahme:** `tos-api/src/bios/vectors.rs:286@adopt_page0_write` — eine Seite-0-Kopie von
  `$008`–`$407` (Vektoren 2…0xFF, `etv_timer`, `etv_critic`; `tos-api/src/bios/vectors.rs:273@adopts_page0_write`),
  die vom geteilten Wert abweicht, wird die geteilte Zelle (`store_shared`: jede Seite 0 traegt
  den Wert, `Setexc(n,-1)` liefert ihn). Besitz und Warnung wie bei `Setexc`, ein Mechanismus
  (`tos-api/src/bios/vectors.rs:256@note_shared_write`, `Via::Page0`): Besitzer nach ADR-031
  Regel 3 (`Pterm` schreibt den Vorwert zurueck, solange die Zelle den geschriebenen Wert haelt;
  darueber gekettet → bleibt + Warnung; `Ptermres` behaelt), Warnung einmal je Haken und Prozess;
  das Zurueckschreiben des Startwerts warnt nicht (wie `Setexc`). Nie aufgerufen.
- **Erkennung:** `tos-api/src/bdos/process/hooks.rs:307@check_page0` uebernimmt statt nur zu
  warnen; `$408` (`etv_term`) bleibt wie bisher: Warnung `Hook::EtvTermDirect`, keine Uebernahme.
  Vektoren 0/1 und `etv_xtra` (`$40C`) bleiben ausserhalb (Entscheidung „Umfang“).
- **Zeitpunkt — Abweichung vom Wortlaut „am Ende des Systemaufrufs“, bitte bestaetigen:** nur
  am Trap-Ende liefert das erste `Setexc(n,-1)` nach `Super(0); *(long*)0x80 = x;` noch den alten
  Wert (die Uebernahme kaeme erst nach diesem Aufruf) — die Abnahme waere so nicht erfuellbar.
  Deshalb laeuft die Pruefung fuer einen Prozess im Super-Zustand zusaetzlich **vor** jedem Trap:
  `tos-api/src/bdos/process/hooks.rs:648@before_trap`, vom Kern gerufen
  (`kernel/src/console.rs:456@check_hooks_before_trap` aus `rust_sync_handler`, eine Zeile in
  `kernel/src/arch/aarch64/exceptions.rs:212@check_hooks_before_trap`). Damit sieht auch ein
  `Pexec` aus dem Super-Zustand und das eigene `Pterm` den geschriebenen Wert. Die bisherigen
  Stellen (nach dem Trap, Verlassen des Super-Zustands, Ende im Super-Zustand) bleiben.
- **Kommentare:** `sdk/apps/apitest/apitest.c:516@Setexc` und `tos-api/src/bios/tick.rs:53@etv_timer`
  berichtigt (kein „Schatten“ mehr).
- **`swv_vec` NICHT geaendert (STOP, Befund):** EmuTOS setzt `swv_vec = just_rts`
  (`references/emutos/bios/bios.c:363`), ueberschreibt es aber im selben `bios_init` mit
  `swv_vec = os_header.reseth` (`references/emutos/bios/bios.c:552`, „reset system on monitor
  change“), unbedingt. Gemessen (Sonde V01, tt und falcontc): `swv_vec` = OSHEADER `reseth`,
  nicht `just_rts`, nicht 0. Ein Vorbelegen mit `just_rts` waere eine neue Abweichung zum
  laufenden Orakel. NewTOS bleibt bei 0; Entscheidung beim Lead/Anwender (Vorschlag:
  `RESET_STUB_VA`, der NewTOS-`reseth`, `tos-api/src/bdos/process/loader.rs` `populate_sysvars`,
  Default in `PAGE0_HOOKS` mitziehen).

## Abnahme

- **Sonde** `tosdump/probes/vwprobe.c` (dauerhaft; `make -C tosdump vwprobe-m68k vwprobe-newtos`,
  Lauf `tosdump/probes/run-vwprobe.sh`, `tools/hatari-oracle.sh` kennt `VWPROBE.PRG`): EmuTOS tt =
  falcontc = tt zweiter Lauf, bytegleich. NewTOS nachher: W01–W04 (Vektoren 2, 0x20, 0xFF,
  `etv_critic`, direkt im Super-Zustand geschrieben, `Setexc(n,-1)` liefert den geschriebenen
  Wert), S01 (in einem `Supexec`-Callback geschrieben) und T01 gleich dem Orakel; vorher alle
  `is-written=0`. **T02** (nach dem `Pterm` des Kindes): Orakel behaelt den Wert des Kindes,
  NewTOS gibt den alten zurueck — gewollt, dokumentiert in `docs/COMPATIBILITY.md`. **E01**
  (`$408`): Orakel `is-written=1`, NewTOS `is-old=1` — die dokumentierte Restabweichung. **V01**:
  `swv_vec` (oben). Eingabe per `bios-arg-trace` geprueft (jede `Setexc`-Zeile kommt mit `w:` und
  `l:0xffffffff` an; U1 = das Kind). Warnungen: je Haken und Prozess eine Zeile (Vektoren 2,
  0x20, 0xFF, `etv_critic`, `etv_term ($408, written directly)`; das Kind einmal Vektor 0x20).
- **Keine neue Abweichung:** `bios-core-r1` tt 2/44, falcontc 1/44 und `super-r1` 0/9 · 0/9
  vorher (`b27569f9`) = nachher, tosdiff-Berichte byte-gleich. `sxprobe` nachher byte-gleich mit
  dem Orakel tt/falcontc (BIOS07-Lauf) und mit vorher; `hkprobe` Log und `[hook]`-Zeilen vorher =
  nachher. MEM12 P0PAR: „P0PAR: PASS“, dieselben Zeilen wie im BIOS07-Lauf. qemu-virt bis
  „desktop up“; raspi4b bootet in jedem Lauf.
- **Hosttests:** `cargo xtask test` **1892 / 0 / 1** (Basis 1888/0/1, +4 neu
  `tos-api/tests/bios16_page0_write.rs`: Uebernahme vor dem naechsten Aufruf und in jede Seite 0,
  einmal warnen, `Setexc` nach Direktschreibung liefert diese als alt; Bereichsgrenzen `$008`,
  `$3FC`, `$400`, `$404` ja, `$408`/`$40C`/`$000`/`$004` nein; jede Stelle — vor/nach dem Trap,
  Verlassen, Ende — und keine im Benutzerzustand; Besitz: `Ptermres` behaelt, XBRA-Fall,
  Aushaengen durch Rueckschreiben). Angepasst: `tos-api/tests/bios07_setexc.rs` (der Direkt-Test
  gilt jetzt `$408`), `tos-api/tests/bios15_os_hooks.rs` (VBL-Vektor jetzt uebernommen, am Ende
  per `Pterm` zurueck).
- **Bau/Lint:** rpi4 und qemu-virt release bauen; rustc-Warnungen kernel 93 / tos-api 64 wie Basis
  (159 Warnzeilen je Board); Clippy kernel 121 / tos-api 74 auf beiden Boards, Warnungsmenge je
  Datei identisch zur Basis (`git archive b27569f9`); `clippy -p tos-api --tests`: keine Warnung
  in den neuen/geaenderten Tests.
- **`mapping.tsv` (Ratsche):** neu `setexc` → `adopt_page0_write`, `setexc` →
  `note_shared_write`; `note_write`-Zeile berichtigt. `cargo xtask deps --check-mapping`: 1065
  Zeilen, alle aufgeloest.
- **doc-check:** 0 Fehler in den bearbeiteten Docs; durch die Kern-Zeile verschobener Anker
  `docs/sdk/05-abi-pitfalls.md` (`AesCallOutcome`) nachgezogen; `hatari-oracle.sh` so geaendert,
  dass `TASK/TOOL19.md:9` (`DISKUNIT`) weiter haelt.

## Nachgezogen

`docs/api/bios/05-setexc.md` (Kopfzeile, „Page 0 written directly“ als Tabelle mit Messung, Tabelle
„What NewTOS serves“, Luecken, verschobene Anker), `docs/COMPATIBILITY.md` (OS-Hook-Absatz:
Pruefung vor/nach dem Trap; Besitzer-Absatz; neuer Absatz „A vector cell written directly in page
0“ mit den drei beobachtbaren Unterschieden), `docs/analysis/deps/mapping.tsv`, `docs/STATUS.md`
(Basislinie, Warteschlange, Recent Decisions; aelteste Zeile nach
`docs/archive/status-log-2026-10-26.md`), `tosdump/Makefile`, `tools/hatari-oracle.sh`,
`docs/sdk/05-abi-pitfalls.md` (Anker).

## Offen / nicht geprueft

- **ADR-031 Regel 4** sagt „for warnings only, never to call or act on them“. Die Uebernahme ist
  ein Handeln auf Seite 0 (nicht Aufrufen). Die Anwenderentscheidung deckt es; ein Nachtrag zu
  ADR-031 (wie der vom 2026-10-29) fehlt — ADR-Aenderung beim Lead/Anwender, nicht hier.
- **Zeitpunkt** „vor dem Trap“ zusaetzlich zum Trap-Ende (oben) — bitte bestaetigen.
- **`swv_vec`** nicht geaendert (oben).
- Echte Pi 4.
- Folgeticket nach CLAUDE.md „Testing“ (Sonde → tosdiff): DUMPBIOS braucht eine Fallart
  „Speicher im Super-Zustand schreiben“ (Bericht an den Lead; nicht angelegt).
