# LIBC-TTY — mintlib-`getchar`/`fgets` auf der Konsole: RAW-Weg, doppelt gelesene Tasten

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: a7421716c | Missing: tester acceptance

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/LIBC-TTY-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit a7421716c (fix(libc): build mintlib's own isatty, drop the NewTOS shim (LIBC-TTY)), VERIFIED: diff of a7421716c (fix(libc): build mintlib's own isatty, drop the NewTOS shim ) touches sdk/libc/newtos-arch/isatty.c, sdk/libc/build-libc.sh, sdk/libc/newtos-arch/access.c (+1), names the id by subject, 244 files, legacy line: ERLEDIGT 2026-10-13 ([Quittung](LIBC-TTY-STATUS-DONE.MD))

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-13 ([Quittung](LIBC-TTY-STATUS-DONE.MD)) · Prioritaet: **P2** · Typ: SDK / libc — Nebenfund FS11 (2026-10-13)

Abhaengigkeiten: [FS11](FS11.md) (Zeilenlesen ueber `Fread` auf CON)

## Sachverhalt

`sdk/libc/newtos-arch/isatty.c` meldet die Handles 0–2 immer als Terminal,
setzt aber `__open_stat[].flags` nie. mintlibs `__read` nimmt deshalb fuer
stdin den RAW-Weg (`_console_read_byte` → `Crawcin`, `unix/read.c:173-217`)
statt `Fread`: kein Echo, kein CRMOD; dieser Weg liest zwei Tasten pro Byte
und verwirft die erste (`unix/read.c:176,183`). `fgets` auf stdin endet nie.
Mit den Flags, die mintlibs eigenes `__isatty` setzt (CRMOD|ECHO,
`mintlib/isatty.c:55-57`), laufen `getchar` und `fgets` ueber den
FS11-Zeilenweg korrekt (Beleg: `measurements/measurement-2026-10-13-fs11/`,
Probe FS11DEV Teil C und `serial/smoke-on-rpi4-fs11dev-serial.log`).

Ausserdem meldet der Shim Handles 0–2 auch dann als Terminal, wenn sie per
`Fforce` auf eine Datei umgelenkt sind.

## Arbeit

1. Researcher: belegen, was mintlibs `__isatty` auf TOS tut (ohne MiNT:
   Erkennung per `Fseek`/`Finstat`?) und warum der NewTOS-Shim davon
   abweicht; ob der Shim entfallen kann. Verhalten des RAW-Wegs pruefen.
2. Developer: umsetzen; SDK-Hosttests; QEMU-Sonde `getchar`/`fgets`
   (FS11DEV Teil C ohne Flag-Setzen), EmuCON/APITEST gegentesten.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Entscheidung (Anwender 2026-10-13)

Analyse: `docs/analysis/libc-tty-2026-10-13.md`.

1. `newtos-arch/isatty.c` ersatzlos streichen, mintlibs `mintlib/isatty.c`
   bauen (Fcntl → EINVFN, dann `Fseek`-Probe; seit FS10 korrekt).
2. ioctl-Fallback `TIOCGETP/TIOCSETP/TIOCGWINSZ` → eigenes Ticket
   [LIBC-IOCTL](LIBC-IOCTL.md).
3. Doppellesen im RAW-Weg (`unix/read.c:176-183`) ist mintlib-Verhalten,
   bleibt; in `docs/sdk/07-mintlib-limits.md` vermerken.
4. `do_fstat` meldet fds 0–2 auch umgelenkt als `S_IFCHR` → eigenes Ticket
   [LIBC-FSTAT](LIBC-FSTAT.md).
