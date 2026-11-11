# LIBC-IOCTL — mintlib-ioctl: TOS-Fallback fuer TIOCGETP/TIOCSETP/TIOCGWINSZ

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 10, 2026-11-11), evidence: TASK/LIBC-IOCTL.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: SDK / libc — Anwenderentscheidung 2026-10-13 ([LIBC-TTY](LIBC-TTY.md) Punkt 2)

## Sachverhalt

Der NewTOS-ioctl-Shim reicht nur an `Fcntl` weiter. mintlibs TOS-Fallback
(`unix/ioctl.c:176-199`) fuer `TIOCGETP`/`TIOCSETP`/`TIOCGWINSZ` fehlt; RAW/
CBREAK per `ioctl` oder `tcsetattr` ist damit unerreichbar. Upstream nutzt
dort teils `Super()`/`linea0` — ohne diese uebernehmen. Analyse:
`docs/analysis/libc-tty-2026-10-13.md`.

## Arbeit

Researcher: welche Programme das brauchen, was davon ohne Super/LineA geht;
dann Developer. [Gemeinsame Regeln](README.md) gelten.
