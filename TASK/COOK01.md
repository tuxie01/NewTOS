# COOK01 — `_MCH` = 0x00050000 ist die ARAnyM-Kennung

Status: **ZURUECKGESTELLT** (Anwender 2026-10-20: aktuell vermutlich ohne Wirkung; Varianten 0xFFFFFFFF / Familie+Kennung / 0xFFFF+Kennung, Empfehlung 0xFFFFFFFF) · Prioritaet: **P3** · Typ: Konformitaet — Hinweis von aussen (2026-10-19)

## Befund

`tos-api/src/cookie.rs:223` setzt `_MCH = 0x0005_0000` als vermeintlich
NewTOS-eigene Kennung (Spec `docs/api/bios/_cookie-jar.md:53,136,148`). Der Wert
ist aber belegt: `MCH_ARANYM` (`references/emutos/include/cookie.h:57`;
https://freemint.github.io/tos.hyp/de/bios_cookiejar.html#Cookie_2C_20_MCH).
Ein Programm, das `_MCH` auswertet, haelt NewTOS fuer ARAnyM.

## Moegliche Werte (zu klaeren)

- EmuTOS ohne Atari-Hardware (`CONF_ATARI_HARDWARE` aus): `MCH_NOHARD =
  0xFFFFFFFF` „No ST-compatible hardware“ (`bios/machine.c:400-401`,
  `include/cookie.h:50`) — passt zu NewTOS (kein Shifter/MFP/Videl).
- Was setzen FireBee, Milan (`0x00040000`), Hades, CT60, MiSTer/andere
  Nachbauten? Liste der vergebenen Werte (tos.hyp, FreeMiNT `sys/cookie.h`,
  FireBee-TOS/EmuTOS-ColdFire) zusammentragen; gibt es eine Vergabestelle
  oder Konvention fuer neue Maschinen?
- Wer liest `_MCH` ueberhaupt (SDK-Programme, mintlib, cflib, gemlib, TeraDesk,
  QED, zView)? Anwender: fuer die meisten Anwendungen vermutlich egal.

Nebenbei: `_AKP` steht auf US (`cookie.rs:225`), die Karte ist deutsch
voreingestellt — pruefen, ob `_AKP` der geladenen Tastaturbelegung folgen muss.

## Auftrag

Recherche (Researcher), dann Anwenderentscheidung, dann Spec + Code + Hosttest.

[Gemeinsame Regeln](README.md) gelten.

## Quelle tos.hyp (2026-10-20, vom Anwender genannt)

https://freemint.github.io/tos.hyp/de/bios_cookiejar.html#Cookie_2C_20_MCH —
vergebene `_MCH`-Werte (High-/Low-Word): 0/0 ST, 0/0x4D34 Medusa T40 ohne SCSI,
1/0 STE (Low-Word: 1/2/3 ST Book/STylus, 8 STE mit IDE), 1/0x10 Mega STE,
1/0x100 Sparrow, 2/0 TT oder Hades, 2/0x4D34 Medusa T40 mit SCSI, 3/0 Falcon030,
4/0 Milan, 5/0 ARAnyM. **-1 (`0xFFFFFFFF`) = „nicht-ST-kompatible Hardware“, alle
Hardwarezugriffe sind dann zu unterlassen.** Fuer FireBee, MiSTer, Hatari oder
neue Maschinen ist dort kein Wert definiert, eine Vergaberegel fehlt.

→ `0xFFFFFFFF` ist sowohl der EmuTOS-Wert ohne Atari-Hardware als auch die
dokumentierte Bedeutung fuer NewTOS' Lage (kein Shifter/MFP/Videl). Offen nur:
was FireBee-Firmware tatsaechlich setzt (Quelle noch nicht gesehen).

`_AKP`: Bits 0–7 Tastaturlayout, 8–15 Systemsprache (0 USA, 1 Deutschland, …);
NewTOS setzt fest 0 (US/US) — sollte der geladenen Belegung folgen
(`KEYBOARD.KBD` deutsch → Low-Byte 1), Sprache 0 (Desktop-Texte englisch).

## Nachtrag 2026-10-29 (ARCH06)

Umgesetzt mit [ARCH06](ARCH06.md) nach dem Nachtrag zu ADR-012 vom
2026-10-28 (`docs/MEMORY.md`): `_MCH` = `0xFFFFFFFF` (`MCH_NOHARD`),
`tos-api/src/cookie.rs:225@MCH_NOHARD`, Konstante
`tos-api/src/cookie.rs:250@MCH_NOHARD`. Die Zeilenangaben im Befund oben
beschreiben den Stand vom 2026-10-19. `_AKP` bleibt hier unberuehrt.
