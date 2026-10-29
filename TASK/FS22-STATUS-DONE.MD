# FS22 — Quittung: Neuanmeldung nach Medienwechsel auch fuer kernel-interne Dateipfade

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-fs22/](../measurements/measurement-2026-10-29-fs22/RUNINFO.TXT)
· Ticket [FS22](FS22.md)

- **Ein Neustart-Helfer (Arbeit 1):** `osif`s Neustart als eine Funktion,
  `tos-api/src/gemdos/mod.rs:201@restartable`: je Lauf frischer
  Fehlerkontext (`setjmp` bei `restrt:`), nach einem Lauf mit Medienwechsel
  dessen Ergebnis verworfen, Neuanmeldung (FS20,
  `tos-api/src/gemdos/mod.rs:254@media_change_relog`), Lauf von vorn.
  `tos-api/src/gemdos/mod.rs:146@dispatch_inner` benutzt ihn, ebenso die
  kernel-internen Pfade: `Pexec`-Laden
  (`tos-api/src/bdos/process/pexec.rs:265@load` — Trap-Handler,
  Kernel-Starter, Accessory-Lader), `bdos::fs::read_file`
  (`tos-api/src/bdos/fs/mod.rs:148@read_file` — Ressourcen, `shel_find`,
  `GEM.RSC`, `NEWDESK.INF`, `KEYBOARD.KBD`), die Verzeichnislisten
  (`tos-api/src/bdos/fs/find.rs:351@list_dir`, `:424` `list_dir_ex` —
  Dateiauswahl, Scrap, AUTO, Accessories, Boot-Abbild) und die direkten
  GEMDOS-Aufrufe der Shell (`tos-api/src/aes/shell.rs:118@dos_call`:
  `Dsetpath`, `Dgetpath`, `Dsetdrv`, `Fdelete`). Die beiden letzten Gruppen
  standen nicht in der Lokalisierung, sind aber dieselbe Klasse (in EmuTOS
  `dos_sfirst`/`dos_snext`/`dos_chdir`/`dos_delete` ueber `trap #1`).
  `programs/desktop` liest als EL0-Programm ueber `svc #1`
  (`programs/newtos-gem/src/gemdos.rs:204@dos_open`), also durch `dispatch`
  — geprueft, nichts zu tun. kernel/ bleibt unveraendert (alle Aufrufer
  laufen ueber die tos-api-Funktionen).
- **Aufraeumen je Pfad (Arbeit 2):** wie EmuTOS' `longjmp` muss ein Lauf mit
  Medienwechsel gescheitert sein und nichts hinterlassen. `Pexec`: Suchen und
  Kopf lesen belegen nichts; der Lader gibt Environment und TPA bei jedem
  spaeteren Fehler frei (`xmfree(p_env); xmfree(p)`, `bdos/proc.c:319-332`),
  neu: ein Dateiladen, das einen Wechsel traf, scheitert in jedem Fall
  (`LoadError::MediaChange`,
  `tos-api/src/bdos/process/loader.rs:598@MediaChange`, mit
  `cache::media_change_pending`), der Slot geht in
  `tos-api/src/bdos/process/pexec.rs:301@load_once` zurueck; `xclose(fh)` hat
  kein Gegenstueck (`FileImage` ist keine offene Datei). `read_file`/Listen:
  nur der Ausgabepuffer, den der naechste Lauf neu schreibt. Die DTA des
  Aufrufers gehoert nicht zu `Pexec` (`ixsfirst(path,0,0L)`,
  `bdos/proc.c:258`; die Analyse nennt `:256`).
- **Grenze (Arbeit 3):** `RESTART_LIMIT` = 3
  (`tos-api/src/gemdos/mod.rs:170@RESTART_LIMIT`), danach Neuanmeldung und
  `E_CHNG` statt eines vierten Neustarts; gilt fuer `dispatch` und alle
  Pfade (`Pexec` → `PexecError::MediaChange` = -14, `read_file` `None`,
  Liste 0). Begruendung: ein Wechsel braucht genau einen Neustart (`Getbpb`
  entschaerft `forcechange`, kein BDOS-Pfad ruft `Rwabs`); die Grenze greift
  nur bei Wechseln von aussen waehrend jedes Neulaufs; 3 = Policy-Wert wie
  `WRITEBACK_ATTEMPTS`. Abweichung in `docs/COMPATIBILITY.md` („A media
  change restarts a call at most three times“).
- **Spec (Arbeit 4):** `docs/api/gemdos/_state.md` §4a — „Open [FS20]“
  aufgeloest (Tabelle der Pfade, Aufraeumen, Grenze, Messung);
  `docs/api/gemdos/_dispatcher.md` §8, `docs/api/gemdos/process/4b-pexec.md`
  (Fehlertabelle, NewTOS-Notiz).
- **Ratsche (Arbeit 5):** `mapping.tsv` `osif`→`restartable`,
  `xexec`→`load`/`load_once`, `kpgmld`→`populate_and_map`, Notiz
  `dispatch_inner`; `deps --check-mapping` 1027 Zeilen gruen.
- **Abnahme (Sondenregel):** neue Sonde `tosdump/probes/pxprobe.c` (Lauf
  `run-pxprobe.sh`, Vergleich `pxprobe-compare.py`, Eingabe-Pruefung
  `pxprobe-trace-check.py`, Bau `make -C tosdump pxprobe-m68k
  pxprobe-newtos`, `tools/hatari-oracle.sh` nimmt `PXPROBE.PRG`). Beide
  Builds liegen auf C: (gleiche Abbild-Bytes beider Seiten), jede Seite
  startet ihren eigenen. Orakel zweimal gleich; NewTOS nachher **identisch**
  zum Orakel: P05 `Pexec` nach Sektor-0-Schreiben 42 (vorher -33 `EFILNF`),
  P06 `Mediach` 0 (vorher 2), P07 DTA des Aufrufers unveraendert, F04
  `Fsfirst` nach dem Neustart dieselben DTA-Felder wie das Orakel, F06/F07
  `Fsnext` ebenso. Testpartition auf allen Seiten unveraendert und gleich;
  `fsck_msdos -n` exit 0; Eingabe-Trace: alle 8 BIOS-Aufrufe mit den
  Argumenten der Quelle, kein Kernel-`Rwabs`/`Getbpb`/`Mediach`.
- **Keine neue Abweichung:** bios-r2 3/51, x001 0/8, x004 6/10, x005 1/13,
  x006 5/11, phys 11/12, bios-r1 tt 41/147, falcontc 45/147 — vorher
  (e4dacd70) und nachher dieselben Falllisten, Endzustaende gleich; gcprobe
  wie FS20 (Log gleich dem Orakel, Partition 36b5a14f.. = FS20, nur das
  FS21-Byte). Desktop: raspi4b in allen Laeufen, qemu-virt „desktop up“.
- **Tests:** 7 neue Hosttests (`tos-api/tests/fs22_media_change_kernel_paths.rs`),
  5 davon auf e4dacd70 rot (2 nutzen die neue API). `cargo xtask test`
  1790/0/1 (Basis 1783/0/1), `make -C tosdump host-test` gruen,
  SDK-Hosttests 27/0/0. Bau rpi4 release und qemu-virt: tos-api 64 / kernel 93
  rustc-Warnungen = Basis, Clippy tos-api 74 = Basis, keine Meldung in den
  beruehrten Dateien.
- **Befunde ausserhalb FS22:** die Tracer (`gemdos-trace`, `aes-trace`,
  `dbg_qedtrace`) schreiben ueber das BDOS ohne Neustart (Debug-Funktionen,
  bewusst nicht beruehrt); `TASK/TOOL15.md` Zeile 11 zitiert das
  Orakel-Skript in Zeile 107 mit Anker `ORACLE_HATARI` — schon vor FS22
  veraltet (dort Zeilen 60/154), doc-check-Fehler;
  die Analyse `docs/analysis/osif-restart-coverage-2026-10-29.md` nennt
  `ixsfirst` bei `bdos/proc.c:256` statt `:258` (datierte Notiz, nicht
  geaendert).
