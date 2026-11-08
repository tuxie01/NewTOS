# V59 — Quittung UNFINISHED (2026-11-08)

Datum 2026-11-08 · Zweig `v59`, Basis `6821179c` (`main` mit V49–V51) · Commits `5a3052ec` (Code und Tests) und der
Folgecommit mit Lauf und Tickets · Beleg `measurements/measurement-2026-11-08-v59/RUNINFO.TXT`

**UNFINISHED heisst hier:** Zellenweg, Hosttests und Messlauf stehen; Pixel und Zeigerverhalten sind auf dem Host und in
QEMU gegen die Basis gleich belegt. Offen sind die unabhaengige Abnahme durch den Tester und der Pi-Lauf des Anwenders,
der nach ADR-036 Punkt 5 entscheidet, ob die Aenderung bleibt.

## Was getan wurde

| Datei | was |
|---|---|
| `tos-api/src/vdi/backend.rs` | drei neue Trait-Methoden `read_device_rect`, `write_device_rect`, `draw_sprite`, jede mit der alten Einzelpixel-Definition als Vorgabe |
| `tos-api/src/vdi/mouse.rs` | `draw` = Zelle lesen + `draw_sprite`, `erase` = Zelle schreiben + das unveraenderte `flush_rect`; `full_clip` entfiel |
| `kernel/src/raster_copy.rs` | Uebertragungskern `copy_rows` (NEON, v0–v3), Adapter `read_rect`/`write_rect`, skalare Referenzen; FP-Kontext-Notiz (Zeiger auch im Timer-IRQ aus EL0) |
| `kernel/src/raster_expand.rs` | `sprite` (zwei `Transparent`-Expansionen des V50-Kerns), `sprite_reference` |
| `kernel/src/vdi_backend.rs` | die drei Methoden direkt nach `copy_rect`, ein `clean` je Zeichnen |
| `tos-api/tests/raster_kernel_cell.rs` (neu) | 6 Hosttests, Kernteile gegen die Einzelpixel-Referenzen, Schutzseiten |
| `tos-api/tests/vdi_mouse_v59_cell.rs` (neu) | 2 Hosttests, altes `draw`/`erase` woertlich gegen Vorgabe- und NEON-Weg, Zeiger-Zustandsmaschine auf beiden Wegen und gegen das alte Malen |
| `tos-api/tests/raster_kernel_copy.rs`, `raster_kernel_expand.rs` | Registerpruefung zaehlt drei Kerne; `allow(dead_code)` fuer die nur im neuen Test benutzten Funktionen |
| `TASK/A145.md`, `TASK/BIOS15.md`, `TASK/TOOL32.md`, `TASK/V24.md`, `TASK/V56.md`, `docs/api/vdi/input/_mouse.md`, `docs/audits/2026-11-05-v18-t1-vdi-draw.md`, `docs/audits/2026-11-07-v18-t3-vdi-workstation.md` | je eine verschobene Zeilenangabe nachgezogen (doc-check) |
| `measurements/measurement-2026-11-08-v59/` | Beleglauf |
| `TASK/V59.md` | Umsetzung, FP-Kontext, Entwurf, Belege, Uebergaben |

## Pruefung, die gelaufen ist

- Hosttests rot mit 16 falschen Varianten (Kern, Adapter, Trait-Vorgaben, `mouse.rs`), gruen mit dem echten;
  `cargo xtask test` 2339/0/6 → 2347/0/6.
- Beide Boards frisch gebaut: Warnungen und clippy Delta 0 (rustc 64/93, clippy 73/121, Testziele 87), auch mit
  `raster-prof` uebersetzbar; Disassembly: Uebertragungskern nur q0–q3 in Lade-/Speicherbefehlen.
- qemu-virt und raspi4b booten zum Desktop, 0 Exceptions; jede QEMU per PID beendet und als beendet geprueft.
- `wkprobe` (Zeiger-Faelle) und `vdprobe` Basis = Aenderung bytegleich; `vdi-all` bytegleich; AES r2/r3/r4 tosdiff
  0 Differenzen (roh nur die Adresszelle, Kernabbild +4096 Bytes); Desktop mit Injektor 11/11 bei 640x480 und
  1920x1080, Bootschirm+Konsole 6/6 bei beiden Groessen bytegleich.

## Offen

- Tester: unabhaengige Abnahme (Uebergabe in `TASK/V59.md`).
- Anwender, echte Pi 4: Fbench vorher/nachher mit den zwei Testkernen (sha256 und Ablauf in `TASK/V59.md`); ohne
  messbaren Gewinn kommt `5a3052ec` nicht hinein (ADR-036 Punkt 5).
- `docs/STATUS.md` nicht angefasst (Lead).
