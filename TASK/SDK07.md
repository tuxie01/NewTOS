# SDK07 — TeraDesk: Overscan-Option ohne Atari-Hardware sperren (Bus Error)

Status: **OFFEN** · Prioritaet: **P1** · Typ: Developer (SDK-Overlay) — Anwenderentscheidung 2026-10-29, Fund aus [ARCH06](ARCH06.md)

## Sachverhalt

- Ohne `_VDO` nimmt TeraDesk den ST-Zweig; im Video-Dialog ist Overscan
  freigeschaltet (`long over` mit 0 statt -1 vorbelegt). Overscan ankreuzen +
  OK → Bus Error: Schreibzugriff auf `0xFFFC00` (ACIA) nach `Super(0L)`
  (`sdk/apps/teradesk/video.c:280-294`;
  `measurements/measurement-2026-10-29-arch06/RUNINFO.TXT`).
- Anwender: `_VDO` bleibt weg (Hardware); der Absturz ist ein Portfehler.

## Arbeit

1. Benannte Patch-Stelle in `sdk/overlays/teradesk/`: die Overscan-Option ist
   gesperrt, wenn `_MCH == MCH_NOHARD` (`0xFFFFFFFF`, Cookie per
   `find_cookie`, seit SDK04 C6 korrekt).
2. Abnahme auf QEMU raspi4b mit dem Injektor: Overscan ankreuzen (bzw.
   Versuch) + OK → kein Bus Error, kein Hardware-Schreibzugriff (Trace).

## Abnahme

- Lauf mit Beleg; TeraDesk startet und der Video-Dialog funktioniert sonst
  unveraendert; SDK-Hosttests gruen.
