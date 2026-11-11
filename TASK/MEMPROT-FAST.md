# MEMPROT-FAST — memory-protection resync cheap (brief options 1+2 = analysis options 1 and 3)

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt MEMPROT-FAST-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: only the commit 70a53d26f (perf(memprot): resync only where and for whom righ) that added the receipt itself

Line 3 before the migration:
Status: **ERLEDIGT in QEMU** (Quittung [MEMPROT-FAST-STATUS-DONE.MD](MEMPROT-FAST-STATUS-DONE.MD)), Pi-Lauf offen · Prioritaet: **P1** · Typ: Leistung (Speicherschutz, NewTOS-eigen)

Anwender-Entscheidung 2026-10-07 („Optionen 1+2“ des Auftrags) = in
[`docs/analysis/zview-sluggish-2026-10-07.md`](../docs/analysis/zview-sluggish-2026-10-07.md)
§1 die Fix-Optionen **1** (nur dort neu rechnen, wo sich Rechte ändern) und
**3** (`touch`/`sync` innerhalb eines Traps sammeln). Die 2-MiB-Blockdeskriptoren
(dort Option 2, im Auftrag „Option 3“) sind **nicht** Teil dieses Tickets.

## Sachverhalt

zView-Start in QEMU: 1,38 von 1,61 s Kernzeit sind memprot-Sync, 17,4 Mio.
Seiten neu berechnet (`measurements/measurement-2026-10-07-zview-perf/`).
Pexec 3 58 ms (49 memprot), Pexec 4 51 ms (50), Mshrink 23 ms. Ursache:
`on_alloc`/`on_freed`/`set_mode`/`on_owner_change` rufen `touch` + `sync` über
den ganzen ~916-MB-TPA-Block (`tos-api/src/bdos/process/memprot.rs`), ein
Rebuild rechnet alle Pools für alle Slots neu, `Mshrink` macht einen weiteren
Volldurchgang.

## Arbeit

1. Nur dort neu rechnen/schreiben, wo sich Rechte tatsächlich ändern
   (FreeMiNT-Abkürzung `references/freemint/sys/arch/mprot040.c:1102-1130`):
   gleicher Modus → nichts; `Mshrink` nur der zurückgegebene Rest;
   Besitzerwechsel nur die Slots, deren Besitzermenge sich ändert. Invariante
   „berechnet, nicht fortgeschrieben“ bleibt: das Ergebnis muss einer
   Vollberechnung gleichen.
2. Zusammenfassen: innerhalb eines Traps (z. B. Pexec: alloc → set_mode →
   set_owner → install → launch) schmutzige Bereiche sammeln und einmal am
   Ende / vor `install_ttbr0` synchronisieren. TLB-Pflege (auch ASID-Pfad,
   `asid=on`/`tiny`) für jeden geänderten Eintrag.
3. Sicherheitsnetz (Pflicht): Prüfmodus, der nach jedem Sync die inkrementell
   gepflegten Tabellen gegen eine Vollberechnung vergleicht und bei Abweichung
   meldet/anhält — in Hosttests und in QEMU über die ganze Schutz-Serie;
   per Bootargument abrufbar, Standard aus.

## Abnahme

- Hosttests für die Abkürzungsfälle (alloc/free/shrink/Besitzer/Modus,
  geteilte Blöcke, E10, strict) mit eingeschaltetem Prüfmodus.
- QEMU raspi4b: Schutz-Serie unter `memprot=on`, `strict`, `off` und einmal
  `asid=on`, alle mit Prüfmodus, Ergebnisse wie bisher.
- zView-Profil vorher/nachher (memprot-Anteil).
- Statische Prüfungen unverändert (Hosttests, Release-Builds, Clippy, Drift,
  SDK-Hosttests).
- Pi-Paket: Release-Kernel rpi4 (Prüfmodus aus) +
  `LIESMICH.TXT` in `measurements/measurement-2026-10-07-memprot-fast/bin/`.

Referenzen: `docs/analysis/arch03-implementation-plan.md` (3b), ADR-025,
`references/freemint/sys/arch/mprot040.c:1102-1130`.
