# PROC02 — Kind erbt umgelenkte Standardhandles nicht; `Fforce` zaehlt keine Benutzung

Status: DONE (2026-11-11) | Priority: P2 | Commit: 88bf8192b | Receipt: PROC02-STATUS-DONE.MD

Migrated (owner batch 4, 2026-11-11), evidence: receipt TASK/PROC02-STATUS-DONE.MD exists, commit 88bf8192b, TASK/PROC02-STATUS-DONE.MD, commit 88bf8192b (fix(bdos): children inherit forced handles, Fforce counts uses (PROC02), fix/feat commits naming the id: 88bf8192b (n=1), measurements/measurement-2026-10-02-port10b/, the receipt names no hash for this commit, the Migrated line names it

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-06 ([Quittung](PROC02-STATUS-DONE.MD)) · Prioritaet: **P2** (nach dem OFAM-Termin) · Typ: Abweichung von EmuTOS — aus [PORT10](PORT10.md)

Abhaengigkeiten: —

## Sachverhalt (vom Lead gegen die Quelle geprueft 2026-10-02)

1. **Vererbung:** `tos-api/src/bdos/process/loader.rs:676` setzt
   `bp.p_uft = [-1; NUMSTD]`. EmuTOS vererbt die Standardhandles des Aufrufers
   (`references/emutos/bdos/proc.c:384-397`, `init_pd_files` → `ixforce`).
2. **Benutzungszaehler:** EmuTOS erhoeht beim Umlenken auf einen
   Nicht-Standard-Handle `sft[h-NUMSTD].f_use` (`bdos/fshand.c:66-74`). Bei uns
   liefert `Fwrite(1)` nach `Fforce(1, dup)` + `Fclose(dup)` `EIHNDL` (-37).

Folge: `system("prog >datei")` — die Datei bleibt leer, das Kind schreibt auf
die Konsole, und der Aufrufer verliert danach seinen Handle 1.
Belegt in `measurements/measurement-2026-10-02-port10b/` (`port10.c`,
2 von 29 Pruefungen FAIL). Doom ist nicht betroffen.

## Arbeit

Spec in `docs/api/gemdos/` pruefen/ergaenzen, beides 1:1 nach EmuTOS
umsetzen, Hosttests; Abnahme: `port10.c` 29/29 in QEMU.
