# PORT12 Stufe 1 — Quittung

Stand 2026-11-06. Basis `446dd196`. Nichts an Kernel, tos-api, tos-types. Keine Hardware, keine Karte, kein Push.

**Getan:** XGDOS `f6c5aeb7` nach `references/xgdos/` geklont und in `references/README.md` gepinnt, unveraendert. Host-Tests auf dem Mac gebaut und gelaufen, mit dem SDK gebaut, in QEMU raspi4b gelaufen. Testprogramm fuer Speedo-Fonts ueber das echte VDI und PDF auf der Karte. Sonde fuer Vektor 34, `Ptermres`, Rechte, Cookie, Aufrufkosten in `memprot` an, strict, aus.

**Artefakte:** `measurements/measurement-2026-11-06-port12/RUNINFO.TXT` (Fragen 1-6 mit Stellen), Programme und Skripte unter `measurements/measurement-2026-11-06-port12/method/`, `sdk/overlays/xgdos/lp64-wire.patch`, `sdk/overlays/xgdos/xos_newtos.c`. Ticket [PORT12](PORT12.md) traegt Ergebnis, Aufwand, offene Entscheidung.

**Geprueft, gemessen:** Ausgaben, PGM, PDF, Metafile der Upstream-Tests Mac gegen NewTOS bytegleich. Mit Patch: Schirm 0 Pixel Abweichung bei y < 455, PDF bytegleich, Bilder angesehen. Vektor 34 gerufen: 0 mal.

**Nicht geprueft:** `.FNT`-Kette, Speicher- und Metafile-Geraet unter dem Patch, Mehrebenen-Bilder, zweiter Prozess in XGDOS, AES, reale Pi, Speedo-Tempo, XGDOS selbst auf Hatari. `txbench` fuer Weg A nur geschaetzt.

**Offen:** Entscheidung Weg A oder B beim Anwender, danach ADR. Nebenbefund `Fcreate` ueber 8.3 (kein Ticket angelegt).

Kein Teil davon ist eine Hardwareabnahme.
