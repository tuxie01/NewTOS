# MEM01 — `Mxalloc(-1)` meldet fuer ST-RAM und TT-RAM denselben Wert

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 17, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by c6280342e (feat(kernel): one shared address space for all pro), named by status-line-named

Line 3 before the migration:
Status: **ERLEDIGT 2026-10-22** (behoben mit ARCH03 Stufe 1 `c6280342`, ST/TT getrennt; Hosttest `mem02_mxalloc_modes`, `measurements/measurement-2026-10-17-mem02-tests/`; Anwender 2026-10-22 auf der Pi: TeraDesk-Info zeigt rund 10 MB ST-RAM und rund 730 MB TT-RAM, passend zum Bootschirm minus Belegtes) · Prioritaet: **P2** · Typ: Belegter Defekt / Speicherverwaltung — Anwenderbefund am Geraet (2026-09-23)

Abhaengigkeiten: —

## Der Befund

TeraDesks Info-Dialog auf der echten Pi 4 zeigt

```
Free memory:   8257536 bytes ST-RAM
               8257536 bytes TT-RAM
```

TeraDesk holt die Werte getrennt: `Mxalloc(-1L, 0)` (ST) und `Mxalloc(-1L, 1)`
(TT), `sdk/apps/teradesk/main.c:326-327`. Der Bootschirm derselben Maschine
meldet **ST-RAM 10 MB, TT-RAM 750 MB** (`kernel/src/initinfo.rs`). 8 257 536
Bytes = 7,875 MiB — fuer TT-RAM zwei Groessenordnungen zu wenig, und dass
beide Zahlen **exakt gleich** sind, sieht nicht nach Zufall aus.

## Die Kette, die zu pruefen ist

- `tos-api/src/bdos/mem/traps.rs` `mxalloc`, Zweig `amount == -1` (`:268-293`):
  liefert `pool::largest_free(pmd)` bzw. `(pmdalt)` je nach Modus — der Modus
  wird also unterschieden.
- `tos-api/src/bdos/mem/pool.rs` `largest_free` (`:150-165`): laeuft die
  Freiliste `mp_mfl` ab und nimmt das groesste `m_length`.
- `tos-api/src/bdos/mem/init.rs` `init` (`:165-215`): seedet je Pool **eine**
  MD ueber `seed_md(start, len)`.
- `kernel/src/lib.rs` (`:717-734`): `claim(Pool::Stram)` / `claim(Pool::Ttram)`
  fordern `stats(pool).free()` **zusammenhaengende** Frames an
  (`alloc_frames_contig`) und uebergeben Start/Laenge an `init`.

Verdachtsrichtung (nicht geraten, zu belegen): entweder liefert
`alloc_frames_contig` fuer TT-RAM nur einen kleinen zusammenhaengenden Block,
oder `stats(Ttram).free()` ist klein, oder beide Pools bekommen denselben
Bereich. Die Gleichheit der beiden Zahlen ist der beste Hinweis.

## Arbeit (Auditor)

1. Den Weg vom Bootschirm-Wert (750 MB) bis `largest_free(pmdalt)` mit
   Messpunkten belegen — Serial-Trace auf einer **Scratch**-Kopie, echte Pi
   oder QEMU raspi4b (dort meldet der Bootschirm andere Zahlen, der Weg ist
   derselbe).
2. Ursache benennen, Hosttest schreiben, der sie festhaelt (rot).
3. Erst danach Fix (Developer).

## Abnahme

`Mxalloc(-1, 0)` und `(-1, 1)` melden die tatsaechlichen groessten freien
Bloecke ihres Pools; TeraDesks Info-Dialog zeigt fuer TT-RAM eine Zahl in der
Groessenordnung der Poolgroesse. Hosttest + Beleg am Geraet.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Erst Ursache mit Beleg, dann Fix.
