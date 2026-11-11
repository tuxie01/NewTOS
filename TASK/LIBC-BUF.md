# LIBC-BUF — erster `printf` auf ungepuffertes stdout legt nur 1 KiB Puffer an

Status: IMPLEMENTED (2026-11-11) | Priority: P3 | Commit: 41ba8f966 | Missing: real-Pi test

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: commit 41ba8f966 (fix(sdk): redirected stdout keeps its 32 KiB buffer (LIBC-BUF)), VERIFIED: diff of 41ba8f966 (fix(sdk): redirected stdout keeps its 32 KiB buffer (LIBC-BU) touches sdk/libc/overlay/, names the id by subject, 28 files, legacy line: ERLEDIGT 2026-10-22 (QEMU, `41ba8f96`, Sonde LBUF rot/gruen, `measurements/measu

Line 3 before the migration:
Status: **ERLEDIGT 2026-10-22** (QEMU, `41ba8f96`; Sonde LBUF rot/gruen, `measurements/measurement-2026-10-22-libc-buf/`; Pi offen) · Prioritaet: **P3** · Typ: SDK / libc — Nebenfund LIBC-FSTAT (2026-10-17)

Beobachtet in der FSTATP-Sonde: vor der ersten Ausgabe ist
`stdout->__bufsize` 32 KiB; der erste `printf` auf den noch ungepufferten
Strom setzt `__bufsize` auf 0, der danach angelegte Puffer ist 1 KiB — auf der
Konsole wie bei einer Datei. Upstream-stdio-Pfad. Pruefen, ob dasselbe auch
`fopen`-Dateien trifft und damit die FS08-Regel (32 KiB fuer Dateien,
`0abc634`) unterlaeuft; dann entscheiden.

[Gemeinsame Regeln](README.md) gelten.

## Anwenderentscheid 2026-10-22

Fix einbauen: umgelenktes `stdout` soll nach dem ersten `printf` denselben
Puffer behalten wie eine `fopen`-Datei (32 KiB, FS08-Regel), als Patch in
`sdk/overlays/` bzw. `sdk/libc/overlay/` (Upstream bleibt unveraendert).
Befund dazu: `docs/analysis/open-tickets-review-2026-10-19.md` §LIBC-BUF.
