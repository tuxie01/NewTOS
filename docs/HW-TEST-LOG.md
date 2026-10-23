# HW-TEST-LOG — Real Raspberry Pi 4 bring-up

> Manual hardware verification log + setup guide for booting NewTOS on
> a physical Raspberry Pi 4 (BCM2711) over a serial console. Closes
> PHASE-01 **EC2** once the boot banner appears on real hardware.
>
> **First metal boot succeeded 2026-06-17** (graphics path: VideoCore
> framebuffer test pattern on HDMI; serial console not yet wired). The
> boot/MMU/heap path and the BCM2711 mailbox framebuffer run on real
> silicon. Serial-console verification (boot banner + self-tests over
> PL011) still pending the USB-TTL adapter — see §7.

---

## 1. What you need

- A Raspberry Pi 4 (any RAM size — the kernel currently uses the first
  1 GiB unless the firmware-supplied DTB reports more).
- A micro-SD card (any size; we only use a small FAT32 boot partition).
- A **3.3 V** USB-to-TTL serial adapter (e.g. CP2102 / FT232 / CH340).
  **Do not** drive the Pi's RX with 5 V signalling — the GPIO is 3.3 V
  and 5 V will damage it. If the adapter is level-selectable
  (e.g. a "3.3 / 5 V" cable), select **3.3 V**.
  - **In use here:** Waveshare USB-TO-TTL (D), **FT232RNL**, USB-A,
    SH1.0 3-pin (TXD / RXD / GND), 3.3 / 5 V selectable — use the
    **3.3 V** setting. FTDI wire convention: black = GND, yellow = the
    adapter's RXD, orange = the adapter's TXD (verify against the
    cable's own labels).
- A host terminal at **115200 baud, 8N1, no flow control**.
  - **macOS** (FT232R works with the built-in driver — no install on
    recent macOS):
    ```sh
    ls /dev/tty.usbserial-*                  # find the device
    screen /dev/tty.usbserial-XXXX 115200    # exit: Ctrl-A then K
    ```
  - Alternatives: `minicom`, `picocom -b 115200 /dev/tty.usbserial-XXXX`,
    or PuTTY.

## 2. Wiring (USB-TTL ↔ Pi 4 GPIO header)

The kernel drives **PL011 / UART0** on GPIO 14/15. Connect three wires
(TX↔RX crossed), sharing a common ground. **Leave the adapter's 5 V /
3.3 V power pin disconnected** — the Pi is powered separately.

| USB-TTL pin | Pi 4 GPIO (BCM) | Pi 4 header pin |
|-------------|-----------------|-----------------|
| RX  (adapter in)  | GPIO 14 — TXD0 | pin 8  |
| TX  (adapter out) | GPIO 15 — RXD0 | pin 10 |
| GND               | GND            | pin 6  |

(Adapter RX ← Pi TX, adapter TX → Pi RX. If you only see output but
typing does nothing, you likely swapped TX/RX or left GND off.)

## 3. SD-card layout (FAT32 boot partition)

Format the card with a single FAT32 partition and copy:

1. **Raspberry Pi 4 firmware** (from the official `raspberrypi/firmware`
   `boot/` folder): `start4.elf` and `fixup4.dat` (the minimum for a
   bare-metal 64-bit kernel; `bootcode.bin` is **not** used on the Pi 4).
2. **`config.txt`** — use [`boot/rpi4/config.txt`](../boot/rpi4/config.txt)
   from this repo. The `init_uart_clock=48000000` line is mandatory.
3. **`kernel8.img`** — built below.

That's it. No `cmdline.txt` is required for a bare-metal kernel.

## 4. Build `kernel8.img`

```sh
cargo xtask build --board rpi4 --release
# → produces ./kernel8.img at the workspace root (raw binary via objcopy)
```

Copy that `kernel8.img` to the SD card's boot partition.

## 5. Boot

1. Insert the SD card, connect the USB-TTL wires, open the host terminal
   at 115200 8N1.
2. Power the Pi.
3. **Expected first output** (within a second or two):

   ```
   RB
   NewTOS booting on rpi4
   Phase 02 reached.
   [mm] ...
   ```

   `RB` is the **pre-MMU boot heartbeat** — the boot stub writes it to
   the PL011 data register as its very first action, before any MMU /
   stack / BSS setup. Seeing `RB` confirms the firmware loaded
   `kernel8.img`, the CPU reached our entry point, and the serial path
   + baud are correct. The banner and self-test sequence follow (the
   same one QEMU prints: `[selftest p04] ... 10 passed`, `EC4/D3/D6/D7
   PASS`, `D3/D4D5 PEXEC PASS`, `EINVFN OK`, then the `[cconin]` and
   `RX echo` prompts — type a key to see it echoed).

## 6. Troubleshooting

| Symptom | Likely cause |
|---|---|
| **Nothing at all — not even `RB`** | The CPU never reached our entry, or serial is dead: wrong baud (must be 115200); TX/GND not connected or TX/RX swapped; `config.txt` missing (`enable_uart=1` / `arm_64bit=1`); firmware files (`start4.elf`/`fixup4.dat`) or `kernel8.img` missing/misnamed. |
| **`RB` then nothing** | Serial + firmware + entry are **fine** — a boot stage *after* the heartbeat faulted (MMU stage-1, BSS, stack, or the high-half jump). This narrows it to `boot.rs` / `mmu.rs`; capture exactly how far it got. |
| **Garbage / random bytes** | UART clock mismatch — confirm `init_uart_clock=48000000` is in `config.txt`. This is the #1 Pi 4 cause. |
| **Output but typing does nothing** | Adapter TX → Pi RX (GPIO 15) not connected, or TX/RX swapped. |
| **Banner then hangs / faults** | A real-hardware MMU / cache / boot-stage difference vs QEMU — capture the full log and the `[EXC]` line if any, and record it below. |

The pre-MMU `RB` heartbeat (boot stub) makes the silent-early-failure
case diagnosable: **no `RB`** = serial/firmware/entry problem; **`RB`
then silence** = a fault in our own early boot code.

---

## 7. Test log

Record each metal boot attempt here (date, firmware version, RAM size,
what appeared, any fault).

| Date | Pi model / RAM | Firmware | Result | Notes |
|------|----------------|----------|--------|-------|
| 2026-06-17 | RPi 4 Model B | `start4.elf`/`fixup4.dat` (card, 2026-04-17) | ✅ **First metal boot — VideoCore framebuffer test pattern visible on HDMI** (red / green+white-box / blue, 640×480 on a 1080p screen) | **Graphics path only** (no serial yet). Confirms on real silicon: EL2→EL1 drop (firmware enters at EL2), mailbox property-interface FB allocation against real VideoCore firmware, **Non-Cacheable FB mapping** (without it the display DMA would not see the cached pixels — the exact failure QEMU's TCG cannot reproduce), BGR pixel order, and the whole MMU/heap boot path. SD card written from this repo (`kernel8.img` + `init_uart_clock=48000000` added to an existing RPi4 `config.txt`). |
| 2026-06-17 | RPi 4 Model B | as above | ✅ **First full VDI frame on metal** — the Phase-12 VDI demo (16 GEM palette tiles via `vr_recfl` + line grid/border via `v_pline`, every pixel through `svc #2`) renders correctly on HDMI. | **Found + fixed a real HW-only cache bug en route:** the first attempt showed the filled tiles sharp but the **lines as dashes / missing** — dense `fill_rect` writes evict whole cache lines (visible), sparse single-pixel `put_pixel` writes stay stranded in untouched lines (invisible). The Non-Cacheable FB mapping was *not* effective on real silicon for these writes; QEMU's TCG has no cache so it could never surface there. Fix: `mmu::clean_dcache_range` + clean each drawn region (`dc cvac` to PoC) after `fill_rect`/`draw_line`. After the fix the grid lines are solid. The diagnostic signature (dense OK, sparse broken) pinpointed it without a serial console. |
| 2026-09-05 | RPi 4 Model B | as above | ✅ **BIOS group proven from EL0** — `APITEST.NCP` (kernel `2908bbe`): **86 PASS / 0 FAIL / 1 SKIP**. All twelve trap-13 functions, 80 checks, documented return *and* documented error case each. Better than QEMU's 82/0/5: the five interactive `Bconin(2)` checks ran (BIOS-CHR 25 → 30) and `Drvmap` saw four drives. The one skip is `T29.3` (needs a child program that is not on the card). Yesterday this group stood at **zero** proven functions of twelve. |
| 2026-09-05 | RPi 4 Model B | as above | ✅ **An application can draw to the screen itself** — `PHYSBASE.NCP`: asks `Physbase()` over the TOS API, writes three colour bands with a plain store loop, **no cache maintenance**. Bands appear. This is ADR-023 (`2cf4fac`) plus the uncached exposure (`92e6276`) working on real caches, which QEMU cannot test — it has no cache model. |
| 2026-09-05 | RPi 4 Model B | as above | ✅ **Page flipping on real VideoCore firmware** — `SCRFLIP.NCP` (`2908bbe`): painted the hidden page, screen showed **nothing** (`HIDDEN PAGE PAINTED — screendump now must NOT show the bands`), then `Setscreen` flipped and the image stood full-screen. Nothing copied, only the scan-out moved. Mailbox tag `0x0004_8009`, sourced from `references/circle`. The last piece the October demo needed. |
| 2026-09-09 | RPi 4 Model B | 1920x1080, `#Z` autorun | ⚠️ **`Setscreen` rastet die physische Basis nicht auf den Ruecklauf** | Beobachtet mit einer externen Anwendung, die eine feste Bildregion aendert: der Seitenwechsel wird sofort wirksam (synchroner VideoCore-Mailbox-Write `0x0004_8009`), es gibt kein Pending-Register und keinen Retrace-Interrupt, der ihn einbaut. Die Naht steht still und ist unuebersehbar. Kein Anwendungsprogramm kann das umgehen. Beschrieben mit Aufrufkette, Belegen und zwei offenen Referenzfragen als `# KNOWN DEFECT` an `tos-api/src/xbios/screen.rs:221`. Kein Verhalten geaendert. |
| 2026-09-11 | RPi 4 Model B | 1920x1080 | ✅ **Ueberlappende Fenster: kein Rest, kein Fehler** — die Rechtecklisten-Reparatur FIX-01…06 interaktiv abgenommen | Vom Anwender auf echter Hardware gefahren und bestaetigt: Fenster ueberlappen lassen, den Fokus zwischen ihnen wechseln, sie uebereinander verschieben — **alles ohne Fehler**, keine stehengebliebenen Baender, kein Ueberzeichnen ueber den Rahmen. Das ist der D2/R2-Residue-Fall, das Leitsymptom des ganzen Durchgangs, und er ist strukturell weg. Deckt Checkliste D2 (`docs/api/desktop/_hw-test-checklist.md`) und [A32](../TASK/A32.md) Punkt 1 und 2. QEMU kann diesen Nachweis nicht fuehren: der Dumper oeffnet und schliesst die Fenster in Millisekunden, und `raspi4b` hat kein Eingabegeraet (unser USB laeuft nur ueber PCIe+xHCI). **Nachgetragen am 2026-09-12** — der Test lief am 11.09., wurde damals aber nicht protokolliert; die leere Checkbox hatte ihn faelschlich als offen gefuehrt. |
| 2026-09-11 | RPi 4 Model B | 1920x1080, `#Z` autorun | ✅ **TeraDesk startet wieder** — Abnahme des Ausrichtungsfixes (Befund B2) auf echter Hardware | Kernel `39b293a4…` (Release, derselbe, der in QEMU in beiden Bauprofilen geprueft wurde) plus neu erzeugte `icons.rsc` (13284 B, 0 von 76 Rasterzeigern ungerade; vorher 36). Vorher brach TeraDesk im Debugbau sofort mit `slice::from_raw_parts requires the pointer to be aligned` ab, im Releasebau lief derselbe Zugriff still als UB durch. Die Karte trug zuvor den Dev-Kernel `3c6d7a4a…`. Offen geblieben: QED auf echter Hardware (Befund B1, in QEMU belegt) ist noch nicht gefahren. |
| 2026-09-11 | RPi 4 Model B | 1920x1080, `#Z` autorun | ✅ **AES-Runde 3 auf echter Hardware gemessen** — 39 Faelle, `tosdiff --validate` accepted | Falltabelle `aes-r3-wind-state.tsv` (CRC `979FAC60`), Dump `measurements/measurement-2026-09-11-pi4-r3/`. Gegen denselben Lauf in QEMU `raspi4b`: **38 von 39 Faellen bitgleich**; der eine Unterschied ist Fall 603, `int_out[3]/[4]` = 1920x1080 gegen 640x480 — Bildschirmgeometrie, kein Defekt, und beide liefern korrekt den ganzen Schirm (`0,0,0,0` haette bedeutet, dass die fruehe Rueckkehr versagt). Bestaetigt erneut, dass die Emulation die Hardware trifft. Der inhaltliche Vergleich laeuft gegen den QEMU-Lauf, weil der auf dem 640x480-Profil der TT liegt. |
| 2026-09-12 | RPi 4 Model B | 1920x1080, Doppelklick im Desktop | ✅ **QED laedt seine Resource wieder** — Abnahme des Pfadaufloesungsfixes (Befund B1) auf echter Hardware · ⚠️ **zwei andere, bekannte Fehler dahinter** | Kernel `39b293a4…`. Der Alarm „Resource nicht gefunden!" ist **weg**: `rsrc_load` spiegelt jetzt EmuTOS' `rs_load` (`gemrslib.c:835-837`) und loest einen qualifizierten Pfad ueber `sh_find` auf. Beachte den Startweg: **per Doppelklick im Desktop**, nicht ueber den `#Z`-Autostart, den QEMU geprueft hatte — der Fix traegt also auf beiden Wegen. **Dahinter sichtbar geworden, beide schon bekannt und ticketiert, vom Anwender bewusst zurueckgestellt:** (1) QED meldet, es finde den `CLIPBOARD`-Ordner nicht, obwohl er existiert — das ist der TOS1-3-gegen-TOS4-Fork bei `Fattrib` auf Verzeichnisse (Projektnotiz `cflib-apps-expect-tos4-semantics`); (2) getippter Text in einer neuen Datei erzeugt **keine gezeichneten Zeichen** — [Q06](../TASK/Q06.md) und `docs/analysis/qed-typing-redraw/`, wo die urspruengliche Hypothese bereits widerlegt ist. Keiner der beiden ist eine Regression dieses Fixes. |
| 2026-09-18 | RPi 4 Model B | 1920x1080, `#Z` autorun TeraDesk | ✅ **H02 + H03 auf echter Hardware abgenommen** — Dateiinfo bei markierter Datei, TeraDesk beendet sich regulaer | Kernel `cd1b1af9…` (Release aus `b186d6b`), `DESKTOP.NAP` `db31aae1…` (gemlib `a_evnt_timer`-Overlay). Vom Anwender gefahren: eine Datei markieren, *File → Info* → die Markierung bleibt, der Dateidialog kommt und laesst sich editieren (H02); *File → Quit* → Dialog → eingebauter Desktop (H03). Anwender: „das ist schon ein erfolgreicher Meilenstein". Weitere Befunde beobachtet, auf Wunsch des Anwenders **nicht** benannt — die Taskliste hat Vorrang. Die uebrigen Punkte der Liste `docs/api/desktop/_hw-test-2026-09-17.md` (A63–A65, T01, A66, A67) wurden nicht einzeln zurueckgemeldet und bleiben *HW offen*. Alter Kernel liegt als `kernel8.914b86e.img` auf der Karte. |
| 2026-09-20 | RPi 4 Model B | 1920x1080, `#Z` autorun TeraDesk | ✅ **Karte auf `4aa3bf2` abgenommen: Farbicons, Menuetrenner, volle Leisten, Start-und-Rueckkehr** | Kernel `0d959619…` (Release), `DESKTOP.NAP` `fb37bb3b…` (A60), `desktop.rsc` `409f69a0…`/`icons.rsc` `8d4bc8a2…`/`cicons.rsc` `ee7de7f3…`/`qed.rsc` `30ac6bf2…` (v13, A40/A41). Anwender: „perfekt, funktioniert" auf die angekuendigte Erwartung — Farbicons fuer Laufwerke/Papierkorb und gelbe Ordner im Fenster (A41), Menuetrenner als gepunktete Linien (A46), Menueleisten ueber die volle Breite (A40), Programm starten und zurueck mit intakten Fenstern und Infozeile (H05, A39). Damit auf Hardware bestaetigt: A39, A40, A41, A46, H05; A60 (TeraDesk startet/laeuft mit dem Overlay) implizit. Nicht einzeln zurueckgemeldet: A43/A44 (nicht sichtbar), A63–A65, T01, A66, A67. |
| 2026-10-17 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **A112 + DESK48 + ARCH02 abgenommen, keine Rueckschritte** | Kernel `bff063c5…` (Release aus `f2c68a31`; Vorgaenger `715462e5…` als `KERN17B.IMG`). Anwender: Desktop (Fenster, Ordner, Menues, Dialoge), SYSACC, TeraDesk, QED wie vorher, Menue offen haelt alles an (A91); DESK48 Markierung verschwindet, mehrere Desktopsymbole werden alle verarbeitet, zwei Fenster auf demselben Verzeichnis aktualisieren beide. *„soweit alles bestätigt, keine bugs“*. Nicht direkt gezeigt: Accessory mit eigenem Timer (kein solches auf der Karte), ARCH02-Sonde `FPPEXEC` nicht gefahren. |
| 2026-10-18 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **RST01 abgenommen — Neustart wie geplant** | Kernel `f000303a…` (Release aus `3717b49e`; Vorgaenger `bff063c5…` als `KERN1018.IMG`), TeraDesk `DESKTOP.NAP` `fdaada9e…` (Overlay, alt als `DESKTOP.BAK`). Anwender: *„reset funktioniert, wie geplant“* — PM-Watchdog-Neustart auf echter Hardware bestaetigt (Quelle war nur Circle, nicht Datenblatt). Einzelne Punkte der Testliste (Ctrl+Alt+Del / +RShift, Negativfaelle, TeraDesk „Reboot“, Shift L/R, SYSACC, Ordner) nicht einzeln zurueckgemeldet. |
| 2026-10-18 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **Release-Kandidat NewTOS 0.6 abgenommen** | Kernel `4bc49cf3…` (Release aus `f7cb0fd9`; vorher `f000303a…` als `KERN18B.IMG`). Anwender-Kurztest (Start/Version, EmuCON `type` mit Ctrl-S/Q und Typeahead — Tempo, QED, Doom, TeraDesk, Ctrl+Alt+Del): *„laeuft wie erwartet“*. Damit auf echter Pi: PROC04 Punkt 1/4, CON01 (USB-Poll je Ausgabebyte ohne sichtbare Verlangsamung). |
| 2026-10-22 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **GH1 abgenommen, HypView erstmals auf der Pi** | Kernel `1a86e475…` (Release aus `d40f4f16`, GH1-Fix `7b295ab3`; Vorgaenger als `KERN18C.IMG`), `HYP_VIEW.NAP` `200a7d78…` mit `QED.HYP`/`ZVIEW.HYP`/`TERADESK.HYP` je im Programmordner. Anwender-Kurztest nach Prueflist (SYSACC-Popup ueber `C:\ACC\`, QED Menue + Klick in den Text, Desktop/Fenster/Accessory, TeraDesk, HypView, zView): *„scheint all das zu funktionieren, auch das NAC funktioniert“*. zView-Laden (FS08): *„das Laden ist ok“* — SD am Pi liefert rund 5–10 MB/s, gemessen 10 MB in 975 ms. TeraDesk-Info (MEM01): rund 10 MB ST-RAM, rund 730 MB TT-RAM, passend zum Bootschirm. Nicht gefahren: Werte in SYSACC aendern; Umbenennen mit langem Namen (die 8.3-Eingabemasken lassen keinen langen Namen zu, keine Prio). |
| 2026-10-22 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **Release NewTOS 0.6.1 abgenommen** | Kernel `eaba7c5e…` (Release aus `26193203`, nur Versionstext gegenueber `d40f4f16`). Anwender: Bootschirm und About zeigen 0.6.1, Desktop/SYSACC/HypView wie zuvor — *„passt, stimmt alles“*. |
| 2026-10-23 | RPi 4 Model B | 1920x1080, NEWDESK | ✅ **USB01 F1/F2/F8 + CON04 VT52 abgenommen** | Kernel `86205b81…` (usb-diag-Release aus `7c1f621d`; Vorgaenger `eaba7c5e…` als `KERN1022.IMG`), `VT52PI.NAP` `d7861cfa…`. Apple-Tastatur (mit eigenem Hub) + ROCCAT-Maus in zwei Steckpositionen (Maus an Port 2 bzw. 4 des VL805-internen Hubs): je 4 Geraete, Tastatur + Maus konfiguriert, 13 Befehle, kein Fehler; Zeiger, Tasten, Tippen gingen beide Male, `C:\USBDIAG.LOG` geschrieben und am Desktop umbenannt. VT52PI bei 240x67: T1–T5 und Ruecklesen stimmen, T6 blinkt, Taste -> Desktop sofort zurueck. Nicht abgedeckt: >15 Befehle, Report-only-Maeuse. Beleg `measurements/measurement-2026-10-23-usb01-con04-pi/RUNINFO.TXT`. |
| _pending_ | | | _serial console_ | Boot banner + self-tests over PL011 once the Waveshare FT232RNL USB-TTL (D) adapter is wired (ordered 2026-06-16). Heartbeat will read `RB2` (EL2 entry digit). |
