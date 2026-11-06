# DESK71 — NewDesk-Dialoge aus Zeichenkoordinaten wie `desk_rsc.c` (unter 400 Zeilen)

Status: **OFFEN** (2026-11-06) · Prioritaet: **P3** · Typ: Abweichung, aus [V32](V32.md)

Seit V32 nimmt NewTOS unter 400 Zeilen den 8x8-Font wie EmuTOS. Mehrere NewDesk-Dialoge haben feste
Pixelhoehen fuer 8x16 und wachsen nicht mit (640x240: About-Dialog ragt unten aus dem Schirm,
`measurements/measurement-2026-11-06-v32-test/RUNINFO.TXT`). In EmuTOS liegen die Desktop-Dialoge in
Zeichenkoordinaten (`desk/desk_rsc.c`) und werden per `rs_obfix` skaliert. Betroffen laut
V32-Developer: `programs/desktop` `insapp.rs` (ganzer Dialog), `ins.rs` (`FLD_H`, `BTN_H`, `IBOX_H`,
`UPDN_H`), `dir.rs` (`FLD_H`), `inf.rs` (`OK_H`, `BTN_H`). Dazu `form.rs` `do_alert`, das schon vorher
von `fm_build` abweicht (`btn_h = ch + 6`, `btn_y`) — eigener Befund, zuerst am Orakel messen.

Auftrag: Dialoge aus den `desk_rsc.c`-Koordinaten aufbauen (ADR-026, EmuDESK-Rebuild), bei 640x240
gegen EmuTOS (Hatari `0x011C`) messen, bei 640x480/1920x1080 unveraendert.

[Gemeinsame Regeln](README.md) gelten.
