# UART01 — Unechtes `0x00` auf offener RXD0-Leitung (GPIO 15) landet im AUX-Ring

Status: **OFFEN** · Prioritaet: **P2** · Typ: Messung zuerst (Pi), danach Developer — Anwenderentscheidung 2026-10-30, Folge von [BIOS09](BIOS09.md)

Abhaengigkeiten: [BIOS09](BIOS09.md) (seit dort erreicht das Byte den AUX-Ring); Richtung [IO01](IO01.md) (weitere UARTs, MIDI)

## Sachverhalt

- Ohne UART-Adapter ist GPIO 15 (RXD0) ein Eingang **ohne Pull**: der Kern
  loescht die Pull-Bits selbst (`GPIO_PUP_PDN_CNTRL_REG0` Bits 31:30 = `0b00`,
  `kernel/src/drivers/bcm2711/pl011.rs:93-111@configure_uart0_pins`).
- Gemessen 2026-09-04 (Boot-Schirm, echte Pi): ein `0x00` kam ueber die
  RX-Interrupt-Strecke an (`docs/MEMORY.md` QUIRK-002, "Correction
  2026-09-04"; Kommentar `kernel/src/console.rs:166-181@serial_rx_byte`). Ein
  zweiter Diagnose-Boot zaehlte ein `0x00` im Init-Flush des UART — nur in
  den Notizen des Leads, kein Messlauf.
- Seit BIOS09 (`aac91ba6`) geht jedes Empfangsbyte in den AUX-Ring. Folge auf
  der echten Pi ohne Adapter (PLAUSIBLE, nicht gemessen): das erste
  `Bconin(1)` kehrt einmal sofort mit `0` zurueck, `Bconstat(1)` ist `-1`,
  apitest BC03 (`Bconstat(1) == 0`) kann fehlschlagen.
- EmuTOS-Seite: am Atari liegt hinter dem MFP-Empfaenger ein RS-232-
  Leitungsempfaenger; ohne Kabel kommt kein Byte (`bconstat1` 0,
  `references/emutos/bios/serport.c:255-258`).
- Die Fehlerbits gehen heute verloren: `read_byte` schneidet `UARTDR` auf
  8 Bit ab (`kernel/src/drivers/bcm2711/pl011.rs:176-184@read_byte`); FE/PE/BE/OE
  stehen in den Bits 8–11 von `UARTDR` (Annahme, aus dem PL011 TRM bzw.
  BCM2711-Datenblatt Kap. 11 zu belegen).

## Arbeit

1. **Messung auf der echten Pi** (MEM12-Prueffolge Punkt 11): traegt das
   unechte Byte FE/BE/PE/OE in `UARTDR`? Dafuer braucht es einen Diagnose-Kern
   (Debug-Feature, z. B. `initinfo-diag`), der beim Init-Flush und in
   `pl011_rx_drain` das volle `UARTDR`-Wort (und `RIS`) des ersten Bytes
   festhaelt und anzeigt. Diese Debug-Erweiterung ist der erste
   Developer-Schritt; mit dem Feature aus gilt die Byte-Gleichheit der
   Debug-Features (CLAUDE.md "Testing").
2. **Optionen bewerten, nach der Messung** (Annahmen, aus BCM2711-Datenblatt
   und PL011 TRM zu belegen, Track B):
   - (a) Empfangsbytes mit Fehlerbits verwerfen;
   - (b) internen Pull-up auf RXD0 ueber `GPIO_PUP_PDN_CNTRL_REG0` (BCM2711,
     nicht das alte `GPPUD`-Verfahren) — Leerlaufpegel der UART ist High;
   - (c) beides.
3. **Entwurfsvorgabe:** Pin- und Pull-Konfiguration je UART-Instanz, nicht
   fest auf GPIO 14/15. Ein spaeterer MIDI-Adapter nutzt eine eigene UART
   (UART2–5) auf anderen GPIOs mit 31250 Baud ([IO01](IO01.md)).
4. Optionen mit Messung dem Anwender vorlegen; **kein Fix vor der Vorlage.**

## Abnahme

- Messung auf der Pi mit Lauf unter `measurements/`; Optionen mit Beleg
  (Datenblatt-Abschnitt) vorgelegt.
- Nach dem Fix: MEM12 Punkt 10 ohne einmalige `0`-Rueckkehr; apitest BC03
  gleich `0` auf der Pi ohne Adapter.

[Gemeinsame Regeln](README.md) gelten.
