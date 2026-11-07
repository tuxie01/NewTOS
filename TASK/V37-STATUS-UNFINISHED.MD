# V37 — Quittung UNFINISHED (2026-11-07)

Umgesetzt und gegen das Orakel gemessen; **offen ist nur die Abnahme des Testers** (UNFINISHED heisst hier nicht:
Implementierung fehlt).

Der waagrechte `v_justified` richtet jetzt wie EmuTOS gegen `max_x` aus (`output_text(vwk, cnt, str, max_x, &just)`,
`vdi/vdi_text.c:1288`; `delh` aus `h_align`, Outline und Skew-Additiv je `v_align`, `:332-390`; Start
`DESTX = x - delh - outline`, `:410-412`): `tos-api/src/vdi/text.rs` `v_justified` berechnet `delh` vor der `chup`-Weiche
fuer alle Winkel und beginnt Lauf und Unterstreichung bei `x - delh`. Vorher fehlte `delh` im waagrechten Weg ganz;
jede gemessene Abweichung war eine reine Verschiebung um EmuTOS' `delh` (`d1` 1 px im 8×16-Font, `h_align` 100/200 px).

Artefakte: Code oben; Test `tos-api/tests/vdi_reference/cases_v37.rs` (15, davon 4 ZG mit Grund V41 ignoriert); Probe
`tosdump/probes/vdprobe.c` `t37` (T3700–T3741); Spec `docs/api/vdi/output/_gdp.md` ergaenzt (keine Korrektur);
`docs/analysis/deps/mapping.tsv` Zeile `output_text` → `v_justified`; Lauf `measurements/measurement-2026-11-07-v37/RUNINFO.TXT`.
Zahlen, AES-Gegenprobe und Uebergabeliste: [V37](V37.md).

Ergebnis: TVO, TVP und alle 42 T37-Faelle orakelgleich; gegen die Basis genau 40 Probefaelle anders; `vdi-v18-t2` 0/32,
`vdi-all` 0/182; AES r2/r3/r4 und Desktop/Bootschirm bytegleich zur Basis.

Offen: Tester-Abnahme. ZG002/003/008/009 bleiben bis [V41](V41.md) verschieden (Replace-Modus mit Skew; mit V41s
Scratch-Kandidat plus V37 orakelgleich, Hosttest).
