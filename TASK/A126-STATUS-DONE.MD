# A126 — Quittung

Erledigt 2026-10-30. Die AES meldet `kstate` nur mit den Bits 0–3 wie EmuTOS
(`gsx_kstate` → `vq_key_s` → `Kbshift(-1) & 0x0f`). Sonde `akprobe`
(CapsLock + Alt) gleich dem Orakel tt und falcontc; Hosttests 1899/0/1.
Beleg `measurements/measurement-2026-10-30-a126/RUNINFO.TXT`. Folgeticket
[TOOL30](TOOL30.md) (tosdiff).
