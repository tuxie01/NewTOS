# BALL01 — Ball demo: the ball is painted before the background on the first frame

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 17, 2026-11-11, to PARKED), evidence: was comment with a placeholder commit, NOT VERIFIED: no commit evidence

Line 3 before the migration:
Status: **GESCHLOSSEN** 2026-10-17 — Ball-Demo wird komplett ueberarbeitet (Anwender) · Prioritaet: **P3** · Typ: Darstellungsfehler, Ursache unbekannt — Anwender auf der echten Pi 4, 2026-10-08

Kernel `2afed1e4` (desk-rebuild, nach `ofam-2026`), `BALL.NAP` vom 5.10.
(sha256 `014b4a55…`, Quelle ausserhalb des Repos in
`/Volumes/Daten/Development/ball-demo`). Beim Start wird der Ball zuerst vor
dem noch nicht gezeichneten Hintergrund gemalt — ein kurzes, unschoenes
Artefakt; danach laeuft die Demo richtig.

Zu klaeren, ohne Annahme: Reihenfolge im Demo selbst (`ball.c` Kommentar
:26-36 „the very first paint, which has to put the background …"), oder
NewTOS-Seite (Bildwechsel `Setscreen`/`Vsync` nicht atomar — Nebenbeobachtung
in `docs/analysis/arch03-stage3-access-inventory.md`; Framebuffer-Cache-Pflege).
Gegenprobe: dieselbe Binaerdatei auf TT/Hatari.
