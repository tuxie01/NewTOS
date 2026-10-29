# XBIOS18 — Quittung: das AUX-Warten weicht einem `kbdvec`-Haken aus (qemu-virt)

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-xbios18/](../measurements/measurement-2026-10-29-xbios18/RUNINFO.TXT)
· Ticket [XBIOS18](XBIOS18.md) · Spec `docs/api/xbios/22-kbdvbase.md` (C3 "O1",
Zusatz XBIOS18), `docs/api/bios/02-bconin.md` (Zusatz XBIOS18)

## Gemessen zuerst

- **Sonde (dauerhaft):** `tosdump/probes/axprobe.c` — haengt per `Supexec` einen
  `kbdvec`-Haken ein (zaehlt, merkt Byte und `_hz_200`, kettet unveraendert an
  den alten Vektor; m68k und AArch64 je ein paar Zeilen Assembler), wartet in
  `Bconin(1)`; 0,3 s nach der Bereit-Marke kommt die Taste `a` (1E/9E), 2 s
  spaeter das serielle Byte `Q`. Bau `make -C tosdump axprobe-m68k
  axprobe-newtos` (`tosdump/build-axprobe-newtos.sh`), Lauf
  `tosdump/probes/run-axprobe.sh` (Orakel: Hatari `--cmd-fifo` +
  `--rs232-in`/`--scc-b-in`; qemu-virt: QMP `send-key` an die `usb-kbd` +
  serielles Byte ueber TCP, `tosdump/probes/axprobe-drive.py`; raspi4b: Karte
  aus `tools/mkcard.sh`).
- **Orakel tt und falcontc (je 2 Laeufe, gleich):** `Bconin(1)` 0x51, 2
  Haken-Aufrufe, **beide waehrend des Wartens** (~420/~397 Ticks vor der
  Rueckkehr), Bytes `1e 9e`, verkettete Taste `Bconin(2)` 0x001E0061.
- **NewTOS qemu-virt vorher (daa2973e, 2 Laeufe):** alles gleich bis auf X03:
  **0** Aufrufe waehrend des Wartens, beide Bytes erst mit dem seriellen Byte
  (0 Ticks vor der Rueckkehr). Der Sachverhalt des Tickets stimmt.

## Fix

- `kernel/src/console.rs:267@aux_read_blocking` bekommt `give_way` (Vorbild
  `kernel/src/host_env.rs:578@read_key_loop`): nach dem Pump, wenn kein
  serielles Byte wartet und der laufende Prozess Bytes fuer seinen Haken hat
  (`kbdvecs::pending_for_running`), `None` statt `wfi`. Neues
  `AuxSerial::bconin_restartable` (Trap-Eintrag: `Bconin(1)`, `Cauxin`,
  Ein-Byte-`Fread` auf AUX) nutzt es; `bconin` (Zeilenleser) weicht nie aus.
  `bconin_handle` fordert dann den Neustart an, der Trap-Handler stellt den
  Haken zu und fuehrt das `svc` erneut aus — dasselbe Verfahren wie bei
  `Bconin(2)` (KEYS02). Kein Abgeben an andere Prozesse, kein Spin: das Warten
  bleibt `wfi` wie EmuTOS' `bconin1` (Anwender 2026-10-29, ADR-029 unveraendert).
- Nur `board-qemu-virt`: das rpi4-`kernel8.img` ist vorher/nachher bytegleich
  (a3376c77…); dort hat AUX kein Kabel, `Bconin(1)` wartet nicht.

## Abnahme

- **Sonde beidseitig gleich:** qemu-virt nachher (2 Laeufe) = Orakel tt/falcontc
  in allen verglichenen Zeilen (X01–X08, sha256 der Zeilen a91e3eace2556c21 auf
  allen sechs Seiten); Haken-Aufrufe ~403/~383 Ticks vor dem seriellen Byte.
- **raspi4b (ausserhalb der Abnahme):** `Bconin(1)` 0 sofort, 0 Haken-Aufrufe
  (kein Kabel; QEMU raspi4b hat keine USB-Tastatur).
- **Keine neue Abweichung:** `bios-r1` 41/45 von 147, `xbios-chardev-r1` 12 von 23
  (31, 43–53), `xbios-flush-r1` (Injektor) 0 von 207 — dieselben Faelle wie
  XBIOS15; diese Tabellen laufen auf raspi4b, dessen Kern unveraendert ist.
  `serrx` qemu-virt vorher = nachher = abgenommenes XBIOS15-Ergebnis
  (`SERRX-serial.LOG` ae0d5a59…). Kanarienvogel tt/falcontc OK.
- **Hosttest neu:** `tos-api/tests/xbios18_aux_gives_way.rs` — `Bconin(1)` und
  `Cauxin` fordern bei ausweichendem AUX-Eintrag den Neustart an und liefern
  beim zweiten Lauf das Byte.
- `cargo xtask test` 1824/0/1 (Basis 1823/0/1 + 1), `make -C tosdump host-test`
  gruen, SDK-Hosttests 27/0/0; beide Boards bauen, Warnungen unveraendert
  (rustc 93 kernel / 64 tos-api, clippy 121 / 74, keine in den beruehrten Dateien).
- **doc-check:** 0 Fehler auf den bearbeiteten Doku-Dateien; verschobene Zitate in
  `docs/api/xbios/0e-iorec.md` und `TASK/XBIOS18.md` nachgezogen.

## Nachgezogen

- Doku: `docs/api/xbios/22-kbdvbase.md` (C3 "O1"), `docs/api/bios/02-bconin.md`,
  `docs/api/gemdos/console/03-cauxin.md`, `docs/COMPATIBILITY.md` (Punkt (4) der
  `kbdvec`-Abweichung), Doc-Kommentare `AuxSerial`, `bconin_handle`
  (`tos-api/src/bios/chardev.rs`), `cauxin` (`tos-api/src/gemdos/console.rs`).
- **Ratsche (`docs/analysis/deps/mapping.tsv`):** neue Zeilen `bconin1`/`bconinB`
  → `kernel/src/console.rs` `bconin_restartable`; `bconin_iorec` →
  `aux_read_blocking` (Notiz um das Ausweichen ergaenzt). `cargo xtask deps
  --check-mapping`: 1039 Zeilen, alle aufgeloest.

Nicht gemessen: echte Pi 4 (dort wartet `Bconin(1)` nicht); ein Haken, der nicht
verkettet.
