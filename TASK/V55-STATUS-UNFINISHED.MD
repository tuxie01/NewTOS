# V55 — Quittung UNFINISHED (2026-11-08)

Datum 2026-11-08 · `main`, Basis `e160f507` · Commit `9c72d3ec` (Fix, Bildtakt, Hosttests, Spec, Mapping, DEVIATIONS,
verschobene Zitate) und der Folgecommit mit Lauf und Tickets · Beleg `measurements/measurement-2026-11-08-v55/RUNINFO.TXT`

**UNFINISHED heisst hier:** Fix, Hosttest und Messlauf stehen, M1/M2 sind orakelgleich, alles andere bytegleich zur
Basis. Offen sind die unabhaengige Abnahme durch den Tester und die Maus auf der echten Pi (der Weg laeuft im
Timer-Interrupt).

## Was getan wurde

| Datei | was |
|---|---|
| `tos-api/src/vdi/mouse.rs` | `set_state` ersetzt durch `mov_cur` (nur Ort + `draw_flag`) und `vb_draw` (Neuzeichnen je Bild); `MOUSE_FLAG`; EmuTOS-Reihenfolge in `hide`/`dis_cur`; nur Pakete mit Bewegung bewegen |
| `kernel/src/vbl.rs` (neu) | Bildtakt: `vb_draw` einmal je Bild des `Vsync`-Bildzaehlers, aus dem Timer-IRQ ueber EL0 und am Anfang der Trap-Pumpe |
| `kernel/src/drivers/gic.rs`, `kernel/src/host_env.rs`, `kernel/src/lib.rs` | Aufrufe `vbl::irq_tick` / `vbl::pump_tick`, Modul |
| `kernel/src/raster_copy.rs`, `raster_expand.rs`, `raster_kernel.rs`, `input_gate.rs` | Begruendungstexte (FP-Kontext, Zeigerbewegung) auf den Bildtakt nachgezogen |
| `tos-api/tests/vdi_v55_pointer_frame_redraw.rs` (neu) | 6 Hosttests: M1, M2, mehrere Pakete je Bild, verdeckt, hide/show verwerfen, Tastenpaket |
| 4 bestehende Tests | Paket + `vb_draw` statt Paket allein; `set_locator_pos` statt `set_state` |
| `docs/api/vdi/input/_mouse.md` | Entscheidung, EmuTOS-Soll ergaenzt ("nur bei Bewegung", `vdimouse_init`), NewTOS-Abbildung |
| `docs/DEVIATIONS.md` | V55-Zeile entfernt |
| `docs/analysis/deps/mapping.tsv` | `mov_cur`, `vb_draw`, `cur_display`, `int_vbl` (neu gemappt) und Notizen |
| TASK/A145, BIOS15, TOOL32, TOOL33, V56, V59, `docs/api/aes/resource-fileselector.md`, V18-T3-Audit | verschobene Zeilenzitate |
| `measurements/measurement-2026-11-08-v55/` | Beleglauf |
| `TASK/V55.md` | Umsetzung, IRQ-/FP-Sicherheit, Messung, Uebergabe Pi, Uebergabe Tester |

## Pruefung, die gelaufen ist

- Hosttest rot auf der Basis (M1a 194, M2a 353 Abweichungen), gruen danach; `cargo xtask test` 2368/0/6.
- Beide Boards frisch gebaut, Basis und Aenderung: rustc 64/93, clippy 73/121, `--tests` 87 — Delta 0.
- wkprobe gegen das Orakel: M1a, M2a und alle Faelle ausser P0 gleich; Basis gegen Aenderung genau M1a/M2a.
- vdi-all, AES r2/r3/r4, vdprobe, Desktop-Injektor (640x480, 1920x1080), Bootschirm+Konsole: Basis = Aenderung bytegleich.
- qemu-virt: bootet zum Desktop, USB-Maus per QMP bewegt den Zeiger (Screendumps), Basis = Aenderung bytegleich.
- Release-Kern-Disassembly: keine neue Funktion nennt V-Register (ADR-036 Punkt 3).
- Jede QEMU per PID beendet.

## Offen

- Tester: unabhaengige Abnahme (Uebergabe in `TASK/V55.md`).
- Echte Pi mit USB-Maus: Tester-Szenario "Uebergabe Pi" in `TASK/V55.md`.
- `docs/STATUS.md` nicht angefasst (Lead).
