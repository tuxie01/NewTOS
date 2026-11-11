# MEM04 — `usermem::free` laesst die Abbildung eines freigegebenen Blocks stehen

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 48ba717ca (feat(kernel): per-process memory protection after ), named by body

Line 3 before the migration:
Status: **ERLEDIGT mit ARCH03 Stufe 3b (2026-10-05) — bei eingeschaltetem Speicherschutz; Tester-Abnahme und Pi offen** · Prioritaet: **P2** · Typ: Isolationsluecke — Nebenfund aus [ARCH03](ARCH03.md) Schritt 2

Abhaengigkeiten: — (wird mit ARCH03 Stufe 1/3 ohnehin neu gebaut; bis dahin eigenstaendig)

## Sachverhalt (Architekt, `docs/analysis/arch03-implementation-plan.md`)

`tos-api/src/bdos/process/usermem.rs:596-604`: nach `Mfree` bleibt die
EL0-Abbildung des Blocks im Adressraum des bisherigen Besitzers bestehen.
Vergibt der Pool den Block danach an einen anderen Prozess, kann der alte
Prozess weiter hineinschreiben. Unter EmuTOS gibt es keinen Schutz; bei uns
widerspricht es dem eigenen Isolationsversprechen von ADR-021.

## Arbeit

Zuerst gegen den Quelltext belegen (Lead hat den Fund noch nicht selbst
nachgelesen), Wirkung zeigen (Test: Prozess A gibt frei, B bekommt den Block,
A schreibt), dann Abbildung beim Freigeben entfernen (TLB-Pflege nach
QUIRK-001 beachten) oder bewusst bis ARCH03 zurueckstellen.

## Abschluss (ARCH03 Stufe 3b, 2026-10-05)

Die Stelle `usermem::free` gibt es seit Stufe 1 nicht mehr (Fenstermodell
zurückgebaut); seit Stufe 1 waren beide Pools für jeden Prozess dauerhaft
abgebildet, die Lücke bestand also in allgemeinerer Form weiter: nach `Mfree`
konnte **jeder** Prozess in den freigegebenen Block schreiben.

Mit Stufe 3b (Speicherschutz an, Standard) wird jeder an den Pool
zurückgegebene Block — `Mfree`, `Mshrink`-Rest, `Pterm`-Ernte, Rest eines
`Ptermres` — **vor der Rückkehr** für alle Prozesse ungültig gemacht, auch
für den bisherigen Besitzer, mit TLB-Invalidierung (`memprot::on_freed`,
FreeMiNT `free_region` → `mark_region(PROT_I)`, `sys/memory.c:932`;
Plan-Invariante I8). Freier Poolspeicher ist ab Boot für niemanden
abgebildet (`mmu::map_pools`).

Belege: Hosttest `tos-api/tests/arch03_3b_memprot.rs`
`malloc_takes_the_prgflags_mode_and_free_memory_is_nobodys` (A gibt frei,
A sieht den Block nicht mehr, B bekommt dieselben Seiten, nur B sieht sie);
QEMU-Sonde `sdk/examples/mprot3b.c` Fall `f` (Kind: `Malloc`, schreiben,
`Mfree`, wieder schreiben → Absturzbericht `Memory violation: free memory,
write (translation fault, level 3)`, Elternteil erhält `0x0000FFFF`) auf
`raspi4b` und `virt`, Messung
`measurements/measurement-2026-10-05-arch03-3b/`.

**Mit `memprot=off`** (Bootargument) bleibt das Verhalten von Stufe 3a: die
Pools sind für alle abgebildet, ein Schreiben nach `Mfree` geht durch (wie
EmuTOS). Quittung: `TASK/ARCH03-3b-STATUS-DONE.MD`.
