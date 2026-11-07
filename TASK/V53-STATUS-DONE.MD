# V53 — Quittung DONE (2026-11-08)

Umgesetzt, gemessen und vom Tester abgenommen (PASS, `c013f4b2`, `measurements/measurement-2026-11-07-v53-test/RUNINFO.TXT`): W1/W2/C1 orakelgleich (C1 mit spaetem Paket nach `v_clswk` belegt), Basis gegen Fix nur die sechs Records, Tabellen und AES bytegleich, Programmwechsel per `#Z` mit Menue, Dialog und Ziehen danach. Nicht gefahren: TeraDesk-Start/Ende (kein Injektor-Szenario), echte Pi (USB-Maus nach Programmwechsel ins Testszenario). Nebenbefund gefaltete Sentinels: [V56](V56.md).

`v_opnwk` setzt die VDI-Maus jetzt bei jedem Aufruf zurueck wie EmuTOS `timer_init` + `vdimouse_init`
(`references/emutos/vdi/vdi_control.c:562-563`, `vdi/vdi_mouse.c:565-615`, `vdi/vdi_misc.c:128`): Versteckzaehler 1 ohne
Restaurieren, Mitte, Tasten und `cur_ms_stat` 0, Standardpfeil, `vex_*`-Vorgaben, Pakete an. `v_clswk` schaltet sie ab wie
`vdimouse_exit` (`vdi/vdi_mouse.c:622-645`): vier Mausvektoren `just_rts`, kein Paket bis zum naechsten `v_opnwk`, auch kein
direkter AES-Aufruf (`user_but`/`user_mot`). `v_opnvwk` beruehrt die Maus nicht mehr; die AES holt sich den Radvektor am Ende
jedes `gsx_init`.

Artefakte: `tos-api/src/vdi/mouse.rs` (`vdimouse_init`, `vdimouse_exit`, `timer_init`, `MOUSE_STATE`), `tos-api/src/vdi/workstation.rs`
(`v_opnwk`, `v_clswk`, `init_wk`), `tos-api/src/aes/gsxif.rs` (`gsx_init`), `tos-api/src/aes/init.rs`; Tests
`tos-api/tests/vdi_v53_opnwk_clswk_mouse.rs`, `tos-api/tests/aes_v53_session_mouse.rs` (6, auf altem Code rot); Spec
`docs/api/vdi/workstation/01-v_opnwk.md`, `docs/api/vdi/workstation/101-v_clsvwk.md`, `docs/api/vdi/input/_mouse.md`,
`docs/api/aes/window-graphics.md`; `docs/analysis/deps/mapping.tsv`; `docs/DEVIATIONS.md` (V53-Zeilen raus, zwei Nebenfunde
rein); Lauf `measurements/measurement-2026-11-07-v53/RUNINFO.TXT`. Einzelheiten und Uebergabeliste: [V53](V53.md).

Ergebnis: wkprobe W1a–c, W2a–c, C1a–d orakelgleich (C1 mit spaeterem Paket nachgemessen); gegen die Basis genau W1b/W1c/W2b/W2c/
C1b/C1c anders. vdprobe 0/954, `vdi-all` 0/182, `vdi-v18-t2` 0/32, `vdi-v18-t3` 1/2 (Fall 13 wie vorher), AES r2/r3/r4 und Desktop
(640×480, 1920×1080, Programmwechsel nach WKPROBE) unveraendert benutzbar. `cargo xtask test` 2297/0/6 fuer V53 (Hauptbaum mit
V38: 2305/0/6); Warnungen beide Boards unveraendert (64/93, clippy 73/121).

Offen: Tester-Abnahme. Im LOG bleiben die "vector before"-Zeilen verschieden (AES nicht in den Vektoren; gefaltete Sentinels im
Release-Kernel) — Nebenfunde, Ticket fuer die Sentinels vorgeschlagen.
