# TOOL23 — Release-Bau mit `--features aes-trace` linkt nicht

Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: Debug-Feature; wieder aufnehmen, wenn ein AES-Fix den Mitschnitt braucht) · Prioritaet: **P3** · Typ: Developer (Werkzeug) — Fund aus [XBIOS02](XBIOS02.md) (2026-10-29)

## Sachverhalt

`cargo xtask build --board rpi4 --release --features aes-trace` bricht ab:
„kernel image + stack overruns STRAM_OS_RESERVE (membot)“. Auf der Basis vor
XBIOS02 genauso (Bericht des Developers, `measurements/measurement-2026-10-29-xbios02/RUNINFO.TXT`).
Folge: AES-Wirkung eines Programms ist im Release-Bau nicht mitschneidbar;
XBIOS02 musste auf eine Scratch-Logzeile ausweichen.

## Arbeit

1. Groesse messen (mit/ohne Feature), Ursache benennen (Tracepuffer?
   Formatcode?).
2. Vorschlag: Puffer verkleinern/auslagern oder Reserve fuer Debugbauten
   anheben — Debug-Features duerfen den Bau ohne Feature nicht aendern
   (TOOL13-Regel).

## Abnahme

- Release-Bau mit `aes-trace` linkt und bootet bis zum Desktop; Bau ohne
  Feature bytegleich.

[Gemeinsame Regeln](README.md) gelten.
