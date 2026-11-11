# LIBC-FSTAT — `fstat` meldet fds 0–2 auch bei Umlenkung als Zeichengeraet

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: 67878ccd4 | Missing: tester acceptance

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: receipt TASK/LIBC-FSTAT-STATUS-DONE.MD exists (legacy DONE receipt, IMPLEMENTED accepted by TOOL55), commit 67878ccd4 (fix(libc): fstat follows isatty for standard handles (LIBC-FSTAT)), VERIFIED: diff of 67878ccd4 (fix(libc): fstat follows isatty for standard handles (LIBC-F) touches sdk/libc/newtos-arch/do_fstat.c, names the id by subject, 282 files, legacy line: ERLEDIGT ([Quittung](LIBC-FSTAT-STATUS-DONE.MD), 2026-10-17, Tester-Abnahme offe

Line 3 before the migration:
Status: **ERLEDIGT** ([Quittung](LIBC-FSTAT-STATUS-DONE.MD), 2026-10-17, Tester-Abnahme offen) · Prioritaet: **P3** · Typ: SDK / libc — Anwenderentscheidung 2026-10-13 ([LIBC-TTY](LIBC-TTY.md) Punkt 4)

## Sachverhalt

`sdk/libc/newtos-arch/do_fstat.c:76-79` setzt fuer fds 0–2 immer `S_IFCHR`,
auch wenn sie per `Fforce` auf eine Datei zeigen. Upstream-Verhalten auf TOS
und eine passende Erkennung (z. B. ueber den `isatty`-Status) belegen, dann
korrigieren. Analyse: `docs/analysis/libc-tty-2026-10-13.md`.

[Gemeinsame Regeln](README.md) gelten.

## Entscheidung (Anwender 2026-10-17)

Analyse `docs/analysis/libc-fstat-2026-10-17.md`. Fix nur in
`sdk/libc/newtos-arch/do_fstat.c`: fd 0–5 sind Geraet genau dann, wenn
`__isatty(fd)` es sagt; 32 KiB `st_blksize` fuer jede seekbare Datei mit
`fd >= 0`. Umgelenktes stdout/stdin bekommt den Textmodus wie mintlib auf TOS
(CR/LF; `UNIXMODE=b` fuer binaer). fd 3–5 als Standardgeraet → `S_IFCHR`.
Ungueltiger fd und Geraete-Handles ≥ 6 bleiben wie heute. Die zwei
veralteten Stellen in `docs/sdk/07-mintlib-limits.md` (Vererbung ans Kind,
Zeile ~188) werden mitkorrigiert.

## Datumsnotiz (2026-10-16)

Die Arbeit an diesem Ticket ist vom **2026-10-16** (Commit `6ee90541`).
Messlauf `measurements/measurement-2026-10-17-libc-fstat/`, Analyse
`docs/analysis/libc-fstat-2026-10-17.md` und die Datumsangaben oben tragen
versehentlich den 17.10. Die Pfade bleiben, weil Messlaeufe unveraenderlich
sind und Verweise sonst brechen.
