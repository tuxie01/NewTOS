# CON04 — Quittung

**Status: IMPLEMENTIERT · QEMU raspi4b und Hatari-Falcon belegt · Tester-Abnahme
PASS (`5ca5fa30`, `measurements/measurement-2026-10-22-con04-test/RUNINFO.TXT`) · echte Pi abgenommen 2026-10-23 (`measurements/measurement-2026-10-23-usb01-con04-pi/RUNINFO.TXT`)** · 2026-10-22 · Basis `main` @ `1dec9058` · Commits
`9409f35a` (Code), `deb6e253` (Hosttests), `b31a0a76` (Doku), `373ff2a9`
(Sonde + Beleg) · Beleg
[measurements/measurement-2026-10-22-con04/](../measurements/measurement-2026-10-22-con04/RUNINFO.TXT)

Echte Pi 2026-10-23 (VT52PI.NAP, 240x67): T1–T5 und Ruecklesen stimmen, T6 blinkt,
Taste fuehrt sofort zum Desktop zurueck. Offen bleiben nur die Folgetickets CON05
und der kleine 8-Pixel-Streifen unten.

## Was getan ist

* **Automat** nach `bios/vt52.c`/`conout.c` in `tos-api/src/vdi/escape.rs` auf
  dem bestehenden Alpha-Zustand (derselbe Cursor wie `v_escape`): alle Folgen
  der Spec `docs/api/bios/_vt52.md`, Steuerzeichen, Umbruch, Scrollen.
* **Cursor** als invertierte Zelle, `disab_cnt`, `ESC e/f`, Blinken aus dem
  200-Hz-Interrupt (`kernel/src/drivers/gic.rs` → `fbcon::blink_tick`),
  `Cursconf` auf demselben Zustand.
* **D1** Farben Register → Stift (`REV_MAP_COL`) → Palette der physischen
  Workstation, ohne Workstation `falcon_default_palette`. **D3** Umbruch aus,
  BS nur links, HT klemmt, LF/VT/FF ohne CR. **D4** BEL wertet `conterm&4`,
  kein Ton. **D5** `Bconout(5)` eigener roher Pfad.
* **`v_escape`**: Strings in den Automaten, `v_curtext` ueber `Crawio`,
  `vs_curaddress` direkt mit Klemmen (Q6). `v_opnwk`/`v_clswk` →
  `esc_init`/`esc_exit`; AES `gsx_graphic` (`gl_graphic`) in
  `sh_tographic`/`sh_toalpha`.
* **Q3** zuerst auf Hatari Falcon geprueft: Eigenheit bestaetigt (eine Zelle in
  Vordergrundfarbe bei `ESC p` + `ESC K` an ungerader/letzter, `ESC o` an
  gerader Spalte). Umgesetzt nach Regel: einheitlich mit Hintergrund,
  COMPATIBILITY-Eintrag mit Beleg. **Q5/Q6/D4** ebenfalls in COMPATIBILITY.
* **Kernel-Ausgaben mit nacktem `\n`:** gesucht in `kernel/src` und
  `tos-api/src` (alle Wege zu `console_write_byte`/`Cconout`/`Bconout(2)`).
  Umgestellt: `kernel/src/selftest.rs` (RX-Echo: `\r\n`). Schon richtig:
  Kernel-Konsole `shell.rs`, `bench.rs`, `aes_demo.rs`, `selftest_fbcon.rs`
  (alle `\r\n`), Absturzbericht (`crash.rs` setzt `\r` vor jedes `\n`); dort
  jetzt zusaetzlich EmuTOS' `ESC f ESC v` am Anfang (`kprint.c:312`).
  `kprintln!` geht nur auf die UART. mintlib-Programme schreiben die Konsole
  im Textmodus (`\r\n`, `mintlib/main.c` + `stdio/sysd-stdio.c`).
* **Spec berichtigt:** `docs/api/xbios/15-cursconf.md` — `Cursconf(2)` = ruhig,
  `(3)` = blinkend (Hatari belegt). `_vt52.md` §15, `03-bconout.md`,
  `vdi/_escape.md` nachgefuehrt.

## Pruefungen

* Hosttests **1557 / 0 / 1** (vorher 1512 / 0 / 1, +45 in
  `tos-api/tests/vt52_console.rs` und `vt52_console_default_palette.rs`).
  Angepasst, weil sich das Verhalten absichtlich aendert: `fs11_device_rw`
  (`Bconout(5)` laeuft nicht mehr ueber das Konsolengeraet), die
  zeichnenden-Panik-Attrappen von `aes_rsrc_obfix_at_load` und
  `aes_wind_get_desktop_geometry` (dulden das Loeschen der Konsole in
  `gsx_init` → `v_opnwk` → `esc_init`), der VDI-Referenz-Host (Konsole wie im
  Kernel, sonst bewegt `v_curtext` den Cursor nicht), `crash.rs`-Test (erste
  Zeile `ESC f ESC v`).
* Bau beide Boards: Warnungen 160 / 160 = Basis; Clippy tos-api 76, kernel 125
  (beide Boards) = Basis.
* QEMU raspi4b (Release): Sondenschirm 1 gegen EmuTOS 4/2400 Zellen
  verschieden (genau Q3), Schirm 2 0; Cursor ruhig nach `Cursconf(2)`,
  blinkt mit 0,5 s nach `Cursconf(3)`. NEWDESK, TeraDesk (#Z) und
  AUTO-Programme (Cconws, mintlib-`printf`) zellgleich zum Vorstand.

## Offen

* Tester: TOS-Programm per Doppelklick aus NEWDESK und aus TeraDesk (Cursor
  sichtbar und blinkend waehrend des Programms, danach auf dem Desktop
  verschwunden), EmuCON-Stellvertreter, Absturzbericht, `qemu-virt`,
  APITEST-Basislinie.
* Echte Pi 4 (Blinken aus dem Timer-Interrupt, 1920x1080 = 240 Spalten,
  `vs_curaddress` jenseits Spalte 223).
* Nicht angefasst: `bell_hook` ($5AC) gibt es in NewTOS nicht (in COMPATIBILITY
  unter D4 genannt); `docs/api/bios/_state.md` §9 nennt noch die falschen Namen
  (`_vt52.md` §0).
