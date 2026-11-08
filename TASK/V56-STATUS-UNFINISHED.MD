# V56 — Quittung UNFINISHED (2026-11-08)

Umgesetzt und gemessen, Tester-Abnahme offen. Die drei `vex_*`-Vorgaben `just_rts`, `mov_cur` und das AES-Radtoken waren leere
Funktionen; LLVMs MergeFunctions legte sie im Release-Kern auf eine Adresse (Gegenbau mit `-Z merge-functions=disabled`: drei
Adressen), und `vex_wheelv` verlor den AES-Zweig. Jetzt sind es drei Ein-Byte-`static`s im `.bss` (`token_va`), die weder LLVM noch
ein Linker zusammenlegen darf.

Wirkung: kein Kartenprogramm und kein Port ruft `vex_*` (gesucht in `sdk/` und `programs/`); getroffen waere ein Programm, das das
Rad nimmt und zurueckgibt — danach war das Desktop-Rad tot.

Artefakte: `tos-api/src/vdi/mouse.rs` (`JUST_RTS`, `MOV_CUR`, `AES_WHEEL_TOKEN`, `token_va`); Hosttest
`tos-api/tests/aes_v56_vector_tokens.rs` (2); Probe `tosdump/probes/wvprobe.c`, `tosdump/probes/run-wvprobe.sh`,
`tosdump/probes/wvprobe-mouse.txt`, `tosdump/probes/wvprobe-shots.txt`; `docs/DEVIATIONS.md` (Zeile raus);
`docs/analysis/deps/mapping.tsv` (Notiz `mov_cur`); Lauf `measurements/measurement-2026-11-08-v56/RUNINFO.TXT`. Einzelheiten und
Uebergabe: [V56](V56.md).

Ergebnis: `llvm-nm` Basis eine Adresse, Fix drei; Hosttest optimiert Basis 0/2, Fix 2/2; Szene Basis Rad tot, Fix scrollt (Kontrolle
ohne Probe scrollt auf beiden); WVPROBE E1–E3 Fix = Orakel; wkprobe nur die drei Vektorzeilen anders, jetzt orakelgleich; vdprobe,
`vdi-all`, AES r2/r3/r4, Desktop 640×480/1920×1080 bytegleich. Warnungen unveraendert (64/93, clippy 73/121); `cargo xtask test`
2318/0/6 im Hauptbaum (Basis 2302/0/6; +2 V56, Rest V40).

Offen: Tester-Abnahme; echte Pi nicht noetig ausser im Sammelszenario (USB-Rad nimmt denselben Weg).
