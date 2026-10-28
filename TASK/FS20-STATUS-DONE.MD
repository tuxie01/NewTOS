# FS20 — Quittung: GEMDOS meldet ein Laufwerk nach Medienwechsel neu an

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-28-fs20/](../measurements/measurement-2026-10-28-fs20/RUNINFO.TXT)
· Ticket [FS20](FS20.md)

- **Spec zuerst:** `docs/api/gemdos/_state.md` §4a [FS20], aus EmuTOS
  `1a7484ab`: wann das BDOS `Getbpb` ruft (Anmeldung in `ckdrv`, Neuanmeldung
  in `osif`), wo es einen Medienwechsel bemerkt (`ckdrv` nur fuer
  Wechselplatten; `getbcb`-Treffer per `Mediach`; jedes `longjmp_rwabs` als
  logisches `Rwabs` ohne `RW_NOMEDIACH`), was die Neuanmeldung tut (offene
  Dateien frei -> `EIHNDL`, aktuelle Verzeichnisse -> Wurzel, Puffer verworfen
  ohne Rueckschreiben, `Getbpb` + `log_media` mit dem neuen Bootsektor,
  Aufruf neu; `Getbpb` NULL -> `E_CHNG`), und die NewTOS-Abbildung.
  Verweise in `docs/api/gemdos/_dispatcher.md` §8, `docs/api/bios/09-mediach.md`,
  `docs/api/gemdos/process/_run-ownership-curdir.md`.
- **Fix:** `tos-api/src/bdos/fs/cache.rs` — jeder Puffer kennt sein Laufwerk;
  Treffer fragt `Mediach` (`tos-api/src/bdos/fs/cache.rs:255@hit_test`), jeder
  Geraetezugriff macht die Medientests eines logischen `Rwabs`
  (`tos-api/src/bdos/fs/cache.rs:270@rwabs_test`), `mark_bcbs_invalid` =
  `tos-api/src/bdos/fs/cache.rs:284@invalidate_drive`. `osif` =
  `tos-api/src/gemdos/mod.rs:150@dispatch_inner` (Neustart) mit
  `tos-api/src/gemdos/mod.rs:210@media_change_relog`; `offree` =
  `tos-api/src/bdos/fs/file.rs:1326@free_drive_files`, `freetree`s
  `dirtbl`-Teil = `tos-api/src/bdos/fs/curdir.rs:332@free_drive`, Abmelden
  `tos-api/src/bdos/fs/fat.rs:108@log_out`; `Volume` traegt sein Laufwerk.
  `mapping.tsv`: Zeile fuer `mark_bcbs_invalid` (Ratsche), dazu `offree`,
  `freetree`, `osif`, `getbcb`; `deps --check-mapping` 1020 Zeilen gruen.
- **Sonde dauerhaft unter `tosdump/`:** `tosdump/probes/gcprobe.c` (Code
  gleich der BIOS01-Kopie), Bau `make -C tosdump gcprobe-m68k gcprobe-newtos`
  (`tosdump/build-gcprobe-newtos.sh`), Lauf `tosdump/probes/run-gcprobe.sh`
  (Orakelseite ueber `tools/hatari-oracle.sh` Profil `disk`, das den Probe
  `GCPROBE.PRG` jetzt annimmt), Vergleich `gcprobe-compare.py`,
  Eingabe-Pruefung `gcprobe-trace-check.py`.
- **Abnahme (Sondenregel):** Orakel zweimal, NewTOS vorher/nachher mit
  `bios-arg-trace`. A07 2 -> 0, B04 32672 -> 32676, B05 2 -> 0, Markierung
  Sektor 293 -> 277: nachher `GCPROBE.LOG` identisch zum Orakel, Cluster 3 in
  Partitionssektor 277 auf beiden Seiten. Abbildvergleich (Zeitstempel der
  Wurzel genullt): Orakel 1 = 2; NewTOS nachher weicht in **einem Byte** ab —
  Attribut von `GCPROBE.DAT`, EmuTOS 0x20 (`FA_ARCHIVE`), NewTOS 0x00; das war
  vorher genauso (Befund unten). Eingabe-Trace: alle 13 BIOS-Aufrufe der Sonde
  mit den Argumenten der Quelle und den geloggten Ergebnissen; kein
  Kernel-Aufruf von `Rwabs`/`Getbpb`/`Mediach`. `fsck_msdos -n` aller
  Testpartitionen exit 0; Boot bis zum Desktop.
- **Keine neue Abweichung:** bios-r2 3/51, x001 0/8, x004 6/10, x005 1/13,
  x006 5/11, phys 11/12, bios-r1 tt 46/147, falcontc 50/147 — Falllisten wie
  BIOS01 nachher, Endzustaende gleich.
- **Tests:** 7 neue Hosttests (`tos-api/tests/fs20_media_change.rs`,
  `fs20_media_change_geometry.rs`), auf a36ea390 rot, jetzt gruen.
  `cargo xtask test` 1767/0/1 (Basis 1760/0/1), `make -C tosdump host-test`
  gruen. Bau rpi4 release und qemu-virt ohne neue Warnung (tos-api 64 /
  kernel 93 rustc; Clippy tos-api 74 = Basis, kernel 121; keine Meldung in
  den beruehrten Dateien).
- **Folgeticket (Arbeit 4):** der Fall wandert nach DUMPGDOS, sobald es den
  gibt (`docs/TOS-REFERENCE-DUMPER.md` §7 nennt die Sonde als Uebergang).
- **Offen, Entscheidung des Anwenders:** BDOS-Arbeit ausserhalb von
  `gemdos::dispatch` — `Pexec` (Kernelpfad) und die direkten Dateileser des
  Kernels (`bdos::fs::read_file`: AES-Ressourcen, `shel_find`,
  `NEWDESK.INF`, Tastaturtabelle) — scheitert bei offenem Medienwechsel ohne
  Neuanmeldung; der naechste GEMDOS-Aufruf auf dem Laufwerk meldet neu an. In
  EmuTOS sind das GEMDOS-Traps mit Neustart. Optionen im Bericht an den Lead.
- **Befund ausserhalb FS20:** NewTOS' Schliessen einer geschriebenen Datei
  setzt `FA_ARCHIVE` nicht (EmuTOS `ixclose`, `bdos/fsopnclo.c:530-549`).
