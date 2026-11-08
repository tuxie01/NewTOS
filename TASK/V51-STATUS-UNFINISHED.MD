# V51 — Quittung: UMGESETZT, Tester und Pi offen

Datum 2026-11-07 · Zweig `v51`, Basis `abe8dae1` (V50-Kopf) · Commits `1197fed7` (Kern), `8940791d` (Lauf) · Beleg
`measurements/measurement-2026-11-07-v51/RUNINFO.TXT`

**UNFINISHED heisst hier:** Kern 3, Adapter und Hosttests stehen und sind in QEMU und gegen die Basis belegt. Offen
sind die unabhaengige Abnahme durch den Tester und der Pi-Lauf des Anwenders, der nach ADR-036 Punkt 5 entscheidet,
ob der Kern bleibt.

## Was getan wurde

| Datei | was |
|---|---|
| `kernel/src/raster_copy.rs` (neu) | NEON-Kern `copy_area` (Kopierkern fuer `S_ONLY`, Op-Kern fuer die anderen 15, vorwaerts/rueckwaerts, Zeilen von oben/unten), Adapter `copy_rect`; `raster_op` (unveraendert hierher verschoben); skalare Referenzen (nur Host-Bau) |
| `kernel/src/vdi_backend.rs` | `copy_rect` ruft den Adapter (Op-5-Ruecksprung, Zielbox, `raster-prof`, `clean` wie bisher); `raster_op` reexportiert |
| `kernel/src/lib.rs` | Moduldeklaration am Dateiende |
| `tos-api/tests/raster_kernel_copy.rs` (neu) | 10 Hosttests, Kern und Adapter gegen die alte Schleife bitgleich, Speicher zwischen zwei Schutzseiten, V-Register-Pruefung im Quelltext |
| `measurements/measurement-2026-11-07-v51/` | Beleglauf |
| `TASK/V51.md` | Umsetzung, Entwurf, Belege, Uebergaben |

## Pruefung, die gelaufen ist

- Hosttest rot mit 14 falschen Kernen/Adaptern, gruen mit dem echten; `cargo xtask test` 2279/0/1 → 2289/0/1.
- Beide Boards: Warnungen und clippy Delta 0 (rustc 64/93, clippy 73/121), auch mit `raster-prof`.
- qemu-virt und raspi4b booten zum Desktop, 0 Exceptions; keine QEMU blieb liegen.
- `vdprobe` (TRJ–TRM Schirm->Schirm) Basis = Aenderung bytegleich; `vdi-all`, `vdi-v18-t2`, AES r2/r3/r4 Records
  bytegleich (gleiche Kerngroesse); `slprobe` nur `held1`-Zeitstreuung wie Basis gegen Basis; `shprobe`,
  `shprobe-desk`, Desktop (640x480, 1920x1080), Bootschirm+Konsole bytegleich (zwei QMP-Fehlstarts des Messgeschirrs
  neu gelaufen, belegt).

## Offen

- Tester: unabhaengige Abnahme (Uebergabe in `TASK/V51.md`).
- Anwender, echte Pi 4: Fbench vorher/nachher mit den vier Testkernen (sha256 und Ablauf in `TASK/V51.md`); ohne
  messbaren Gewinn kommt `1197fed7` nicht hinein (ADR-036 Punkt 5).
- `docs/STATUS.md` nicht angefasst (Lead).
