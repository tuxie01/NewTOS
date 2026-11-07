# V50 — Quittung: UMGESETZT, Tester und Pi offen

Datum 2026-11-07 · Zweig `v50`, Basis `ce5e79c2` (V49-Kopf) · Commit `3255df27` (Kern) · Beleg
`measurements/measurement-2026-11-07-v50/RUNINFO.TXT`

**UNFINISHED heisst hier:** Kern 2, Adapter und Hosttests stehen und sind in QEMU und gegen die Basis belegt. Offen
sind die unabhaengige Abnahme durch den Tester und der Pi-Lauf des Anwenders, der nach ADR-036 Punkt 5 entscheidet,
ob der Kern bleibt.

## Was getan wurde

| Datei | was |
|---|---|
| `kernel/src/raster_expand.rs` (neu) | NEON-Kern `expand` (Bytes/Woerter, beliebiges Startbit, vier Schreibmodi), Adapter `glyph`/`mono`, `mono_colors`; skalare Referenzen (nur Host-Bau) |
| `kernel/src/vdi_backend.rs` | Box-Pfad von `draw_glyph_sized` und `blit_mono` ueber die Adapter; die alten Schleifen bleiben an ihrem Platz als Rueckfall |
| `kernel/src/raster_kernel.rs` | `LANE_BITS`, `VSave` `pub(crate)` |
| `kernel/src/raster_prof.rs`, `kernel/src/lib.rs` | `Scope::add_area` (Glyph-Pixel), Moduldeklaration am Dateiende (zitierte Zeilen bleiben stehen) |
| `tos-api/src/vdi/backend.rs` | `MonoBits::as_bytes` |
| `tos-api/tests/raster_kernel_expand.rs` (neu) | 11 Hosttests, Kern und Adapter gegen die skalaren Schleifen bitgleich, Quellen an einer Schutzseite, V-Register-Pruefung im Quelltext |
| `TASK/V50.md` | Umsetzung, Entwurf, Belege, Uebergaben |

## Pruefung, die gelaufen ist

- Hosttest rot mit 14 falschen Kernen/Adaptern, gruen mit dem echten; `cargo xtask test` 2268/0/1 → 2279/0/1.
- Beide Boards: Warnungen und clippy Delta 0 (rustc 64/93, clippy 73/121), auch mit `raster-prof`.
- qemu-virt und raspi4b booten zum Desktop, 0 Exceptions; `vdprobe` Basis = Aenderung bytegleich.
- `vdi-all`, `vdi-v18-t2` Records bytegleich; AES r2/r3/r4 per tosdiff gleich, roh nur die Adresszelle um +0x1000
  verschoben (Kernabbild +4096 Bytes); `shprobe`, `shprobe-desk`, Desktop (640x480, 1920x1080), Bootschirm+Konsole
  bytegleich bis auf das bekannte Zeitpunkt-Bild.

## Offen

- Tester: unabhaengige Abnahme (Uebergabe in `TASK/V50.md`).
- Anwender, echte Pi 4: Fbench vorher/nachher mit den vier Testkernen (sha256 und Ablauf in `TASK/V50.md`); ohne
  messbaren Gewinn kommt `3255df27` nicht hinein (ADR-036 Punkt 5).
- Clip-aus-Pfad der Glyphen ueber den Zeilenrand (`glyph_unclipped`) bleibt skalar (Auftrag).
- `docs/STATUS.md` nicht angefasst (Lead).
