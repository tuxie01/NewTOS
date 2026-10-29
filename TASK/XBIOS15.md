# XBIOS15 — qemu-virt: serielle Bytes landen mit Scancode 0 im Tastaturring

Status: **ERLEDIGT** 2026-10-29 ([Quittung](XBIOS15-STATUS-DONE.MD)); eine Verhaltensaenderung zur Bestaetigung beim Anwender · Prioritaet: **P2** · Typ: Developer (Messung zuerst) — Anwenderentscheidung 2026-10-29, Fund aus [XBIOS01](XBIOS01.md)

## Sachverhalt

- Auf `qemu-virt` gehen seriell empfangene Bytes mit Scancode 0 in den
  IKBD-Ring von `Iorec(1)` (`measurements/measurement-2026-10-29-xbios01/RUNINFO.TXT`,
  „Ausserhalb des Auftrags“).
- EmuTOS: der RS-232-Empfang (`mfp_rs232_rx_interrupt_handler`,
  `references/emutos/bios/serport.c:350-359`) legt das Byte mit
  `push_serial_iorec` (`:331`) in den **Eingangsring von Geraet 0**
  (`iorec1.in`, `:333`), nicht in den Tastaturring. Erwartung (zu messen):
  serielles RX landet im Ring von Geraet 0.

## Lokalisierung (Lead, 2026-10-29)

- `push_serial_iorec` → `kernel/src/console.rs@serial_rx_byte`;
  `mfp_rs232_rx_interrupt_handler` → **ohne Gegenstueck**; `incr_tail` ohne
  Gegenstueck. `--up`-Mengen (gesamt / mit Gegenstueck):
  `mfp_rs232_rx_interrupt_handler` 2/0, `push_serial_iorec` 3/1. Ohne
  Gegenstueck in `bios`: `mfp_rs232_rx_interrupt_handler`,
  `mfp_rs232_rx_interrupt`, `incr_tail`; nach TOOL17 neu klassifizieren.
- Retest: `bios-r1`, `xbios-chardev-r1`, `xbios-ptr-r1`, `xbios-flush-r1`
  (beide Profile), Konsole auf `qemu-virt` und `rpi4` (`Bconin(1)`/`Bconstat(1)`
  auf AUX, `Bconin(2)` unbeeinflusst), serielle Eingabe auf `rpi4` mit
  `serial=on` (siehe Erinnerung: RX darf dort nicht in die Tastaturschlange).

## Arbeit

1. **Messen am Orakel:** ein Byte an Hatari seriell einspeisen (Hatari-
   RS-232-Eingabe, Mechanismus im Lauf belegen), danach `Iorec(0)`- und
   `Iorec(1)`-Feld-Dump (TOOL16-Capture) und `Bconstat(1)`/`Bconstat(2)`.
   Erwartet: Geraet-0-Ring, nicht der Tastaturring.
2. Lokalisierung oben pruefen; Spec (`docs/api/xbios/0e-iorec.md`, Geraet 0)
   nachziehen, falls die Messung abweicht.
3. Fix nach Messung: serielles RX in den Ring von Geraet 0; Ratsche
   (`mapping.tsv`-Zeilen).

## Abnahme

- Messfall beidseitig gleich (Geraet-0-Ring gefuellt, Tastaturring leer);
  kein vorher gleicher Fall neu verschieden; Hosttests gruen; beide Boards
  bis zum Desktop; doc-check 0 Fehler; STATUS im selben Commit.

## Stand 2026-10-29 (Schritt 1, Auditor)

- Gemessen (`docs/audits/2026-10-29-xbios15-serial-rx.md`,
  `measurements/measurement-2026-10-29-xbios15-rx/RUNINFO.TXT`): Orakel tt
  (`--rs232-in`, Bconmap 6) und falcontc (`--scc-b-in`, Bconmap 7) legen die
  Bytes in den Ring von Geraet 0 (= Ring des gemappten Ports), Tastaturring
  leer, `Bconstat(1)` -1, `Bconin(1)` liefert das Byte. NewTOS qemu-virt legt
  sie mit Scancode 0 in den Tastaturring (`Bconstat(2)` -1), Geraet-0-Ring
  leer; `Bconmap(-1)` liefert dort noch `0x2c` (XBIOS05). raspi4b verwirft das
  Byte (ACCEPTED nach E5/AUX „no cable“).
- Funde: SERRX-001 (S3, Routing in `serial_rx_byte`), -002 (S3, PLAUSIBLE:
  Scancode-Suche wie `push_ascii_ikbdiorec`, nur mit Zweit-ROM messbar), -003
  (S4: Begruendung „qemu-virt hat kein anderes Eingabegeraet“ widerlegt, USB-
  Tastatur vorhanden), -004 (S4: Falcon-Empfangspfad `scc_rx_interrupt_handler`
  ohne Mapping-Zeile; Lokalisierung oben nennt nur den MFP-Pfad).
- **Entscheidung noetig vor dem Fix** (Auditor-Hinweis): AUX auf qemu-virt ist
  heute „Port vorhanden, kein Kabel“ (`Bconstat(1)` 0). Bytes in den Ring von
  Geraet 0 zu legen, ohne dass `Bconstat(1)`/`Bconin(1)` ihn lesen, waere eine
  zweite Inkonsistenz. Optionen: AUX-Treiber ueber den Ring von Geraet 0 auf
  qemu-virt, oder Serienkonsolen-Profil als Abweichung in COMPATIBILITY.

## Anwenderentscheidung 2026-10-29

- **Option (a):** auf qemu-virt geht serielles RX in den Ring von Geraet 0
  (`Iorec(0)`); ein AUX-Treiber liest daraus; `Bconstat(1)`/`Bconin(1)`
  spiegeln den Ringzustand. Ziel: die Orakeltabelle in
  `docs/audits/2026-10-29-xbios15-serial-rx.md` (Abschnitt 2, SERRX-001).
- **Vor dem Fix — Pruefung (Lead, 2026-10-29), dem Anwender berichtet:**
  - Der Injektor (`tools/inject-arm.py`) speist IKBD-Ereignisse ein, nicht
    seriell — unberuehrt.
  - Automatische Laeufe auf qemu-virt (z. B.
    `measurements/measurement-2026-10-21-gh1-fix2/method/virtrun.py`) nutzen
    `-serial file:` nur fuer die Ausgabe — unberuehrt.
  - **Betroffen:** `cargo xtask run` (qemu-virt) startet mit `-display none
    -serial stdio` (`tools/xtask/src/main.rs:848@serial`); wer dort im Terminal
    tippt, tippt heute in den Tastaturring. Nach dem Fix kommen diese Zeichen
    nur noch auf AUX an; Tastatur ist dann die USB-Tastatur (`-display cocoa`,
    `--window`). Werkzeug wird nicht still geaendert — Entscheidung beim
    Anwender.
- SERRX-002: abgelehnt — kein Zweit-ROM, das Orakel bleibt der eine Pin;
  „won't measure“, im Audit vermerkt.
- SERRX-003: Teil dieses Tickets — die Begruendung „qemu-virt hat kein anderes
  Eingabegeraet“ in `kernel/src/console.rs` (Modulkopf und `serial_rx_byte`)
  und in `docs/api/xbios/0e-iorec.md` (E2) berichtigen, alle Kopien.
- SERRX-004: Lokalisierung ergaenzt — fuer das Falcon-Zielprofil kommt RX ueber
  `scc_rx_interrupt_handler` (`references/emutos/bios/serport.c:647-683`, Push
  in `iorecB.in` `:676`), Gegenstueck ebenfalls
  `kernel/src/console.rs@serial_rx_byte`; die `mapping.tsv`-Zeile kommt mit
  [TOOL17](TOOL17.md).
- **Abnahme:** die Audit-Faelle auf qemu-virt erneut, Ergebnis gleich dem
  Orakel (`Bconstat(1)` -1, `Bconstat(2)` 0, Bytes im Ring von Geraet 0);
  raspi4b unveraendert (verwirft).
- Platz in der Reihe unveraendert (nach BIOS15).

## Anwenderentscheidung 2026-10-29 (Bedienung von `cargo xtask run`)

- Option (b): `cargo xtask run` auf qemu-virt startet standardmaessig mit
  Fenster (`--window`, Tastatur = USB); das Terminal ist AUX (seriell). Ein
  ausdruecklicher Schalter fuer Laeufe ohne Anzeige bleibt (z. B.
  `--headless`). Lauf-Anleitungen in Doku und README nachziehen (CLAUDE.md,
  `docs/`, Erinnerung „QEMU window view command“).

## Stand 2026-10-29

- Umgesetzt: auf qemu-virt geht serielles RX in den Ring von Geraet 0, ein
  AUX-Treiber (`kernel/src/console.rs`, `AuxSerial`) liest daraus; Sonde
  `serrx` gleich dem Orakel (Ring Geraet 0 `61 5a 31`, `Bconstat(1)` -1,
  `Bconstat(2)` 0, `Bconin(1)` 0x61); raspi4b bytegleich wie vorher (verwirft).
  `cargo xtask run` auf qemu-virt: Fenster als Standard, `--headless` ohne
  Anzeige. Beleg `measurements/measurement-2026-10-29-xbios15/RUNINFO.TXT`.
- **Zur Bestaetigung:** `Bconin(1)`/`Cauxin` warten auf qemu-virt jetzt auf
  ein Byte wie EmuTOS' `bconin1` (vorher sofort 0). Kein Aufrufer in
  `programs/`, `sdk/apps`, `sdk/examples` gefunden.
- Unveraendert: `Bconout(1)` verwirft auf qemu-virt weiter; `Bconmap` liefert
  `0x2c` (XBIOS05).

