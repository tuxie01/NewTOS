# A118 — STATUS: DONE (2026-10-25, Nacharbeit 2026-10-26, QEMU) — restliche Nehmer der `wind_update`-Sperre

**Auftrag:** die in GH1 benannten Restabweichungen je Stelle gegen EmuTOS
belegen, Wirkung zeigen, nach EmuTOS angleichen.

## Wirkung (vor dem Fix, HEAD `17cab8b6`)

Hosttest `tos-api/tests/aes_a118_lock_takers.rs`: 3/13 gruen (die drei
Gegenproben), 10 rot — `wind_open`/`wind_close`/`wind_delete`/`wind_set`/
`graf_rubberbox`/`graf_dragbox`/`fsel_input` eines zweiten Prozesses liefen
an einer fremden Sperre vorbei; `appl_exit` hielt die Sperre nicht ueber
`wait_for_accs`; ein Klick in der Dateiauswahl eines Accessorys erfuellte das
geparkte `MU_BUTTON` des Desktops; der Quick-Check eines Accessorys feuerte auf
den Druck des Desktops. QEMU (Accessory `A118ACC.NAC` ruft `fsel_input`,
Doppelklick aufs C:-Icon neben dem Dialog): vorher ist das Icon danach
selektiert, nachher nicht (4418 px).

`w_redraw_desktop` aus der Liste entfaellt: die Funktion fehlt in NewTOS ganz
(Antwort des Screen Managers auf `WM_REDRAW` in seiner eigenen Queue), es ist
kein fehlender Sperr-Nehmer.

## Fix (`ded51d9e`)

Dispatcher-Tor `update_block` vor 19, 70, 71, 90/91 und
`window::opcode_takes_lock` (101/102/103/105, genau wenn EmuTOS `BEG_UPDATE`
erreicht); die Rumpfe klammern mit `update_beg`/`update_end`, `fsel` mit
`fm_own`; `appl_exit` einmal je `AC_CLOSE`-Runde; `input::owns_mouse` fuer
Quick-Check und geparkte Rechteck-Waits; Shell-Schleife ruft `form_error`
ueber den GEM-Trap aus EL1. Spec vorher/mit dem Fix:
`docs/api/aes/window-graphics.md` „The AES's own takers (A118)“, `event.md`,
`application.md`, `resource-fileselector.md`, `shell-loop.md`.
Benannte Abweichungen: `fsel` wartet vor dem Zeichnen (EmuTOS zeichnet und
wartet dann in `fm_do`); der `Pexec`-Kind-Sweep zaehlt bei fremdem Halter
weiter oben drauf (kein EmuTOS-Gegenstueck).

## Pruefung

Bau beide Boards dev+release rc 0, 159 = 159 Warnzeilen; Clippy tos-api 74,
kernel 121 = unveraendert; `cargo xtask test` 161 bin 1644/0/1 → 162 bin
1657/0/1 (+13). QEMU raspi4b: GH1-Serie 41/41 + std 26/26 pixelgleich, A117
7/7, TeraDesk Start+Beenden 3/3, QED Start+Beenden (in der GH1-Serie),
APITEST on/off/strict 109/0/5 = 109/0/5, `fm_error`-Alert ueber den Trap
pixelgleich; 0 EXC/PANIC, fsck rc 0. Belege
`measurements/measurement-2026-10-25-a118/RUNINFO.TXT`.

## Nacharbeit nach dem Tester (2026-10-26, `3cb21e31`)

Defekt: ein geparkter Fensteraufruf (101/102/103/105), der beim
Wiederausfuehren ablehnt, liess die ihm uebergebene Sperre stehen (Lock
`(1, B)` statt `(0, None)`; Tester-Fall auf `f42f3719` rot). Fix
`window::end_unused_grant`: Uebergabe abnehmen und weitergeben wie das
`END_UPDATE` des Geweckten in EmuTOS. Neue Tests toeten die vier
Mutations-Ueberlebenden; Mutationsprobe 5/5 rot. Zahlen: Warnzeilen 159 = 159,
Clippy 74/121 unveraendert, `cargo xtask test` 1657/0/1 → 1664/0/1; QEMU
GH1-Serie 41/41, std/A117/TeraDesk-Ende/a118-Probe/APITEST-Bilder 41/41
pixelgleich, APITEST on/off/strict 109/0/5 = 109/0/5. Belege
`measurements/measurement-2026-10-26-a118-fix/RUNINFO.TXT`.

## Offen

- Echte Pi 4.
- Das Parken des Shell-Schleifen-Alerts hinter einem Accessory ist nirgends
  provoziert (nur der Weg ohne Halter in QEMU).
- Nebenbefunde (nicht angefasst): `ap_exit`s abschliessendes `all_run()`
  (`aes/gemaplib.c:259`) fehlt; `graf_rubberbox`/`graf_dragbox` laufen ueber
  eigene Schleifen statt `gr_rubwind`/`gr_dragbox` (Begrenzung von
  `graf_dragbox` gegen `rc_constrain`/`gr_clamp` pruefen); `evnt_multi`
  testet `MU_M1`/`MU_M2` zur Arm-Zeit auch dann, wenn ein Quick-Check schon
  gefeuert hat (EmuTOS armt nur bei `what == 0`, `aes/gemevlib.c:231`);
  `w_redraw_desktop`/`hctl_msg` fehlen; `ACCLOSE_ROUND` ist ein globales
  Flag, nicht je Identitaet.
