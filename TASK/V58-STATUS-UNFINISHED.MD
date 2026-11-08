# V58 — Quittung UNFINISHED (2026-11-08)

Datum 2026-11-08 · Zweig `v58`, Basis `6821179c` (`main` mit V49–V51) · Commit `1c1c71a3` (Linien) · Beleg
`measurements/measurement-2026-11-08-v58/RUNINFO.TXT`

**UNFINISHED heisst hier:** Laufform, Speicherseite und Hosttests stehen und sind in QEMU und gegen die Basis
belegt. Offen sind die unabhaengige Abnahme durch den Tester und der Pi-Lauf des Anwenders, der nach ADR-036
Punkt 5 entscheidet, ob die Aenderung bleibt.

## Was getan wurde

| Datei | was |
|---|---|
| `tos-api/src/vdi/line.rs` | `abline_runs` + `LineRuns` + `Touched`: die Pixelfolge von `abline` als Spanne, Spalte oder Einzelpixel samt Beruehrungsbox; `abline` unveraendert; 4 Unit-Tests gegen `abline` |
| `kernel/src/raster_line.rs` (neu) | Speicherseite: je Modus monomorph, Spanne auf Kern 1, Spalte als Stride-Schleife, Einzelpixel; skalare Referenz (nur Host-Bau) |
| `kernel/src/raster_kernel.rs` | `fill_span` (Kern 1 auf einer Zeile mit eigener Phase); Kern-Asm unveraendert in `fill_rows` verschoben |
| `kernel/src/vdi_backend.rs` | `abline` ruft `raster_line::abline`, ein `clean` der gelieferten Box |
| `kernel/src/lib.rs` | Moduldeklaration am Dateiende |
| `tos-api/tests/raster_kernel_line.rs` (neu), `tos-api/tests/raster_kernel_fill.rs` | 6 + 1 Hosttests |
| `measurements/measurement-2026-11-08-v58/` | Beleglauf |
| `TASK/V58.md` | Umsetzung, Entwurf, Belege, Uebergaben |

## Pruefung, die gelaufen ist

- Hosttests rot mit 20 falschen Versionen, gruen mit der echten; `cargo xtask test` 2339/0/6 → 2350/0/6.
- Beide Boards: Warnungen und clippy Delta 0 (rustc 64/93, clippy 73/121); Release-Disassembly ohne Division und
  Aufruf in den Pixelschleifen.
- qemu-virt und raspi4b booten zum Desktop, 0 Exceptions in 33 Logs; jede QEMU per PID beendet, keine blieb liegen.
- `vdi-all`, `vdi-v18`, `vdi-v18-t2`, `vdi-v18-t3` bytegleich; AES r2/r3/r4 tosdiff 0 Differenzen (roh nur die
  Adresszelle, Abbild +4096 Bytes); `vdprobe` 986 Faelle bytegleich; `shprobe`, `shprobe-desk`, Desktop mit Injektor
  (640x480, 1920x1080), Bootschirm+Konsole bytegleich.

## Offen

- Tester: unabhaengige Abnahme (Uebergabe in `TASK/V58.md`).
- Anwender, echte Pi 4: Fbench vorher/nachher mit den vier Testkernen (sha256 und Ablauf in `TASK/V58.md`); ohne
  messbaren Gewinn kommt `1c1c71a3` nicht hinein (ADR-036 Punkt 5). Informell auf dem Mac war senkrechtes XOR
  langsamer als die Referenz (RUNINFO); die Profilkerne trennen Kern und `clean`.
- `docs/STATUS.md` nicht angefasst (Lead).
