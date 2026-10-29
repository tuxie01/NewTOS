# BIOS10 — Quittung: `Kbshift` haelt das ganze Schaltbyte wie EmuTOS' `shifty`

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-bios10/](../measurements/measurement-2026-10-29-bios10/RUNINFO.TXT)
· Ticket [BIOS10](BIOS10.md) (Anwenderentscheidung 2026-10-28 D1 = 1) · Spec
`docs/api/bios/0b-kbshift.md` · EmuTOS `references/emutos/bios/ikbd.c:145-153`, `:776-883`

## Fix

- Neu `tos-api/src/bios/shifty.rs`: `Shifty` (`tos-api/src/bios/shifty.rs:119@Shifty`, ein
  `AtomicU8`: `get`/`set` aller acht Bits, `key` als eine atomare Aktualisierung) und
  `tos-api/src/bios/shifty.rs:89@key_event` — die Aenderungen von `kbd_int` an `shifty` fuer ein
  IKBD-Tastenbyte: Shift/Ctrl/Alt setzen/loeschen ihr Bit, CapsLock kippt beim Druecken, Clr/Home
  bzw. Insert setzen `MODE_HOME`/`MODE_INSERT` nur bei gedruecktem Alt (`ikbd.c:876-883`), ihr
  Loslassen loescht das Bit immer (`:807-814`); alle anderen Bits (Bit 7, von einem Programm
  gesetzte) bleiben. Dazu `is_modifier` und die `KEY_*`/`MODE_*`-Konstanten.
- `kernel/src/ikbd.rs`: die fuenf Atomics (`LSHIFT`, `RSHIFT`, `CTRL`, `CAPS`, `ALT`, nur Bits 0–4)
  und die `SC_*`-Konstanten sind durch ein Byte ersetzt (`kernel/src/ikbd.rs:43@SHIFTY`).
  `kernel/src/ikbd.rs:358@shift_state` liefert, `kernel/src/ikbd.rs:366@set_shift_state` speichert
  das ganze Byte; `decode_key` wendet das Tastenbyte vor ASCII-Umsetzung und Schlangeneintrag an
  (`kernel/src/ikbd.rs:272@SHIFTY`), wie `kbd_int` (`kb_last.k.shifty = shifty`, `:898`);
  `to_ascii` liest die Bits aus dem Byte. Ablauf sonst unveraendert (Modifikatoren ohne
  Schlangeneintrag, CapsLock-Loslassen und andere Breaks stoppen die Wiederholung wie bisher).
- Doku-Kommentare: `tos-api/src/host.rs` (`keyboard_shift_state`, `set_keyboard_shift_state`),
  `tos-api/src/bios/mod.rs` (`kbshift`).

## Abnahme

- **Sonde K01–K13** (neue dauerhafte Sonde `tosdump/probes/ksprobe.c`, Teil K = die Audit-Zeilen):
  nachher gleich dem Orakel tt und falcontc (`00 00 10 10 ff ff ff ff 81 81 81 81 00`), auch mit
  dem Produktkern ohne Injektor; vorher K05 `0x1f`, K09 `0x01`.
- **Tastenpfad (Arbeit 2, PLAUSIBLE → gemessen):** Teil H der Sonde, Tasten aus
  `tosdump/probes/ksprobe-keys.txt` (NewTOS IKBD-Injektor, Orakel Hatari `--cmd-fifo`):
  T01–T24 und H01–H03 nachher gleich dem Orakel; vorher setzte der Tastaturpfad Bit 5/6 nie und
  `Kbshift(0x80)` ging verloren. Tabelle im Ticket („Messung Tastenpfad“). Eingaben per
  `bios-arg-trace` geprueft (jeder Setz-Aufruf kam wie im Quelltext an).
- **Tabellen** (vorher = `git archive 8cc34fa0`, nachher = dieser Baum; tt / falcontc):
  `bios-r1` 37→37 / 41→41, `bios-core-r1` 2→2 / 1→1, `xbios-chardev-r1` 12→12 / 12→12,
  `xbios-system-r1` 9→9 / 9→9; kein Fall neu verschieden, Fall-Listen identisch; `bios-r1` 77
  (`Kbshift(-1)`) EQUAL vorher und nachher.
- **Hosttests:** `cargo xtask test` **1872 / 0 / 1** (Basis 1859/0/1, +13): 8 Unit-Tests in
  `tos-api/src/bios/shifty.rs` (Setzen/Abfragen aller acht Bits, Modifikatoren, CapsLock,
  Home/Insert mit und ohne Alt, Alt zuerst losgelassen, gespeichertes Bit 7 bleibt, Modifikatormenge)
  und `tos-api/tests/bios10_kbshift.rs` (5 Tests ueber den Trap: K01–K13 gegen die Orakelwerte,
  jeder Bytewert 0..=0xFF hin und zurueck, Abfrage aendert nichts, alter Wert null-erweitert,
  Tastenpfad T01–T24).
- **Bau/Lint:** rpi4 und qemu-virt release bauen; rustc-Warnungen kernel 93 / tos-api 64 wie Basis;
  Clippy kernel 121 / tos-api 74 auf beiden Boards wie Basis (BIOS07-Stand, seither keine
  Produktaenderung); keine Warnung in den beruehrten Dateien und Tests. Boot qemu-virt bis
  „desktop up“; raspi4b bootet in jedem Lauf.
- **Leser des Zustands (Wiedertest laut Lokalisierung):** VDI `vq_key_s` maskiert weiter auf `0x0f`
  wie EmuTOS (`tos-api/src/vdi/inquiry.rs:208@vq_key_s`). Der Desktop testet `graf_mkstate`s
  `kstate` nur maskiert (`0x03`, `MODE_SCA`, `MODE_CTRL` in `programs/desktop/src/main.rs`,
  `programs/desktop/src/dir.rs`), die Bits 0–3 entstehen aus denselben Tastenbytes wie vorher —
  Shift-/Alt-/Ctrl-Klick per Quelltext unveraendert, nicht gefahren. Boot-Bildschirm
  (`kernel/src/initinfo.rs`) maskiert ebenfalls (`MODE_SCA`, `MODE_SHIFT`).
- **doc-check:** 0 Fehler in den geaenderten Dokumenten ausser den vorbestehenden; verschobene
  Anker nachgezogen in `docs/api/xbios/0e-iorec.md` (`push_rx`), `docs/audits/2026-10-28-bios-core.md`
  und `TASK/BIOS10.md` (`set_shift_state`/`shift_state`).
- **`mapping.tsv` (Ratsche):** neu `kbd_int` → `tos-api/src/bios/shifty.rs@key_event`,
  `kbshift` → `kernel/src/ikbd.rs@shift_state` und `@set_shift_state`. `cargo xtask deps
  --check-mapping`: 1054 Zeilen, alle aufgeloest; `deps kbshift --down` vollstaendig.

## Nachgezogen

`docs/api/bios/0b-kbshift.md` (Quellzusammenfassung mit der Tastenregel und den Orakelwerten,
Bit-Tabelle, Absatz zur Tastatur-Mausemulation, Pruefkriterien, Verwender VDI/AES mit der
`0x0f`-Maske, NewTOS-Notizen und Pi-Binding auf den neuen Stand, D1 als umgesetzt markiert,
Zeilenbereiche berichtigt), `TASK/BIOS10.md` (Status, Messung Tastenpfad), `docs/STATUS.md`
(Recent Decisions, Warteschlange; aelteste Zeile nach `docs/archive/status-log-2026-10-26.md`),
`docs/analysis/deps/mapping.tsv`, `tosdump/Makefile` (Ziele `ksprobe-m68k`/`ksprobe-newtos`).

## Neue dauerhafte Sonde

`tosdump/probes/ksprobe.c`, `tosdump/probes/ksprobe-keys.txt`, `tosdump/probes/run-ksprobe.sh`
(`emutos tt|falcontc`, `kernel <tree> <out> [features]` baut den Injektor-Kern in einer Scratch-Kopie,
`newtos <out> <kernel>`), `tosdump/build-ksprobe-newtos.sh`. Folgeticket fuer tosdiff: die
Setz-Zeilen `13:11 w:0x1FF` / `w:0x7F81` werden mit [BIOS13](BIOS13.md) tabellenfaehig; der
Tastenpfad braucht Tasteneingabe und bleibt Sonde.

## Offen / nicht geprueft

- Befunde ausserhalb des Tickets (gemeldet, nicht behoben; Spec „Residual deviations“):
  O1 keine Tastatur-Mausemulation (Alt+Clr/Home, Alt+Insert landen als Tasten in der Schlange;
  `handle_mouse_mode` `unmapped`), O2 AES-Tastenzustand (`graf_mkstate`, `EV_KS`) nicht auf `0x0f`
  maskiert wie EmuTOS ueber `vq_key_s` (CapsLock schon vorher, jetzt auch Bits 5–7), O3 das Byte
  folgt der Eingabepumpe (`Kbshift` pumpt nicht, `Bconstat(2)` nur bei leerer Schlange).
- Echte Pi 4 (USB-HID-Tastatur mit Home/Insert), Desktop-Klicks mit Modifikatoren nicht gefahren.
