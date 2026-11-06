# V32 — Quittung DONE (2026-11-06)

Der Standardfont wird wie EmuTOS `text_init` (`vdi/vdi_text.c:604-623`) aus `V_REZ_VT` abgeleitet:
unter 400 Zeilen 8x8, sonst 8x16; daraus folgen wie in EmuTOS die AES-Masse (`gl_wchar`/`gl_hchar`/
`gl_wbox`/`gl_hbox`, `gemgraf.c:413-415`), IBM-Text, Objektzentrierung, Alert-Zelle, VT52-Konsole und
Bootschirm. Fix `937d6361` (Spec `docs/api/vdi/workstation/01-v_opnwk.md` u. a., 4 Tests). Abnahme
`measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT`: 640x480/1920x1080 bytegleich zur Basis,
640x240 gleich EmuTOS (Font, `graf_handle`, Fensterchrom). Folge [DESK71](DESK71.md).
