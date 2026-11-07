# V49 — Quittung: UMGESETZT, Tester und Pi offen

Datum 2026-11-07 · Zweig `v49`, Basis `3a9221eb` · Commits `87b2a9fd` (Kern), `8403f75b` (`raster-prof`)
· Beleg `measurements/measurement-2026-11-07-v49/RUNINFO.TXT`

**UNFINISHED heisst hier:** Kern, Rahmen, Hosttest und Debug-Feature stehen und sind in QEMU und gegen die Basis
belegt. Offen sind die unabhaengige Abnahme durch den Tester und der Pi-Lauf des Anwenders, der nach ADR-036
Punkt 5 entscheidet, ob der Kern bleibt.

## Was getan wurde

| Datei | was |
|---|---|
| `kernel/src/raster_kernel.rs` (neu) | Schnittstelle `Surface`/`Area`/`Fill`; NEON-Kern `fill_pattern` (vier Schreibmodi), `fill_solid`; skalare Referenz (nur Host-Bau) |
| `kernel/src/vdi_backend.rs` | `fill_rect` und `fill_pattern` rufen den Kern; Hilfen `kernel_surface`/`kernel_area` am Dateiende; je eine `cfg`-Zeile `raster-prof` in sieben Methoden und `clean` |
| `kernel/src/raster_prof.rs` (neu), `kernel/Cargo.toml`, `kernel/src/lib.rs`, `kernel/src/ikbd.rs` | Debug-Feature `raster-prof`, Strg+Alt+F8 (der `sd-diag`-Hotkey-Block in `ikbd.rs` behaelt seine Zeilenzahl, damit Zitate stehen bleiben) |
| `tos-api/tests/raster_kernel_fill.rs` (neu) | 7 Hosttests, Kern gegen Referenz bitgleich, plus Pruefung der V-Register-Sicherung im Quelltext |
| `TASK/V49.md` | Umsetzung, FP-Kontext, Entwurf, Belege, Uebergaben |

`clean` unveraendert; HAL `Framebuffer::fill_rect`/`clear` unveraendert (skalar).

## Pruefung, die gelaufen ist

- Hosttest rot mit acht falschen Kernen, gruen mit dem echten; `cargo xtask test` 2261/0/1 → 2268/0/1.
- Beide Boards: Warnungen und clippy Delta 0 (rustc 64/93, clippy 73/121), auch mit `raster-prof`.
- Feature aus bytegleich zum Bau ohne Feature bis auf `Location`-Zeilen (0 unerklaert).
- qemu-virt und raspi4b booten zum Desktop, 0 Exceptions; `vdprobe` Basis = Aenderung bytegleich.
- `vdi-all`, `vdi-v18-t2`, AES r2/r3/r4: Records Basis = Aenderung bytegleich; `shprobe`, `shprobe-desk`, Desktop
  (640x480, 1920x1080), Bootschirm+Konsole bytegleich bis auf ein Zeitpunkt-Bild (gleicher Text, weiter gescrollt).

## Offen

- Tester: unabhaengige Abnahme (Uebergabe in `TASK/V49.md`).
- Anwender, echte Pi 4: Fbench vorher/nachher mit den vier Testkernen (sha256 und Ablauf in `TASK/V49.md`);
  ohne messbaren Gewinn kommt `87b2a9fd` nicht hinein (ADR-036 Punkt 5).
- `references/tos4x` (Tag `build/tos404`) und der fVDI-SAGA-Treiber nicht gelesen (siehe Ticket).
- `docs/STATUS.md` nicht angefasst (Lead).
