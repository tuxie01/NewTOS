# V54 — Quittung UNFINISHED (2026-11-08)

Datum 2026-11-08 · `main`, Basis `67874e27` · Commits `61989b6d` (Probe G1/G2), `721e0441` (Fix, Hosttest, Spec,
DEVIATIONS) und der Folgecommit mit Lauf und Tickets · Beleg `measurements/measurement-2026-11-08-v54/RUNINFO.TXT`

**UNFINISHED heisst hier:** Fix, Hosttest und Messlauf stehen, F1/F2 sind orakelgleich, der AES-Weg ist gegen das Orakel
gemessen gleich. Offen ist nur die unabhaengige Abnahme durch den Tester.

## Was getan wurde

| Datei | was |
|---|---|
| `tos-api/src/vdi/mouse.rs` | `vsc_form` speichert nur, kein `erase`/`draw` mehr (EmuTOS `set_mouse_form`, `vdi/vdi_mouse.c:450-492`) |
| `tos-api/tests/vdi_v54_vsc_form_no_redraw.rs` (neu) | 4 Hosttests: F1, F2, Paket danach, AES-Weg sofort |
| `tosdump/probes/wkprobe.c` | Version 2: G1/G2 = F1/F2 ueber `graf_mouse(USER_DEF)` |
| `docs/api/vdi/input/_mouse.md` | `vsc_form` Behaviour Schritt 6 "No redraw" mit Quellzitaten |
| `docs/DEVIATIONS.md` | V54-Zeile entfernt |
| `measurements/measurement-2026-11-08-v54/` | Beleglauf |
| `TASK/V54.md` | Umsetzung, Messung, Uebergabe Tester, Nebenfunde |

## Pruefung, die gelaufen ist

- Hosttest rot auf dem alten Code (F1b 243, F2b 180 Abweichungen), gruen danach; `cargo xtask test` 2362/0/6.
- Beide Boards frisch gebaut, Basis und Aenderung: rustc 64/93, clippy 73/121, `--tests` 87 — Delta 0.
- wkprobe gegen das Orakel: F1a–F1d, F2b/F2c, G1b–G1d, G2b/G2c gleich; Basis gegen Aenderung genau F1b/F2b.
- vdi-all, AES r2/r3/r4, vdprobe, Desktop-Injektor (640x480, 1920x1080), Bootschirm+Konsole: Basis = Aenderung bytegleich.
- qemu-virt bootet zum Desktop; jede QEMU per PID beendet.

## Offen

- Tester: unabhaengige Abnahme (Uebergabe in `TASK/V54.md`).
- Nebenfund `graf_mouse(USER_DEF)` Farben/Hotspot (gelesen) braucht ein eigenes Ticket (Lead).
- `docs/STATUS.md` nicht angefasst (Lead).
