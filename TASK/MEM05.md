# MEM05 — viele kleine `malloc` erschoepfen die 254 Speicherdeskriptoren systemweit

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 1a9f1a4fb (feat(libc): malloc hunks double up to 16 MiB (MEM0), named by subject

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-14 (Quittungen [A](MEM05-A-STATUS-DONE.MD), [B](MEM05-B-STATUS-DONE.MD); Pi offen) · Prioritaet: **P2** · Typ: Grenzwert — Nebenfund der E9-Nacharbeit ([ARCH03](ARCH03.md))

## Sachverhalt

Seit E9 holt die libc Speicher per GEMDOS `Malloc` (wie mintlib auf TOS).
MD-Sonde (`measurements/measurement-2026-10-06-post-e9/`): 254 freie MDs am
Start; `malloc(32)` × 10 000 belegt 19 MDs, × 100 000 173; nach 147 605
Bloecken (4,7 MB) liefert `malloc` NULL, alle MDs systemweit weg, ~930 MB frei.

## Arbeit

Zuerst belegen, wie EmuTOS MDs verwaltet (`bdos/osmem.c`, Pool-Groesse,
dynamisches Nachfassen) und wie mintlib unter TOS Bloecke anfordert
(`_heapbase`/MINHUNK); dann angleichen. Nicht umbauen ohne diesen Beleg.

## Entscheidung (Anwender 2026-10-13)

Analyse: `docs/analysis/mem05-md-pool-2026-10-13.md`. Grundsatz des
Anwenders: NewTOS laeuft auf groesserer Hardware mit viel mehr Speicher,
interne Grenzen duerfen sich daran anpassen.

1. **Kernel:** kein fester MD-Pool mehr. Der Vorrat waechst seitenweise nach
   Bedarf; statt einer systemweiten Grenze gilt eine grosszuegige Grenze je
   Prozess (bei Erreichen `Malloc` → 0 wie auf TOS); MD-Freigabe in O(1).
   Die an `MD_BLOCK_COUNT` gekoppelten Schutztabellen (`memprot.rs`,
   `region.rs`) wachsen mit.
2. **SDK/mintlib:** Chunkgroesse verdoppelt weiter bis z. B. 16 MB statt bei
   32 KiB stehenzubleiben (kleine Programme bleiben klein); dafuer eine
   kleine dokumentierte Overlay-Funktion, die nur `MAXHUNK` setzt.
3. `docs/COMPATIBILITY.md`: Eintrag „keine feste MD-Grenze“ als erlaubte
   Verbesserung.
4. Reservierter virtueller Heap mit Zuteilung beim ersten Zugriff → eigenes
   Ticket [MEM06](MEM06.md).

Naechster Schritt: Researcher-Entwurf (Tabellenwachstum, Prozessgrenze und
wo sie eingestellt wird, SDK-Anpassung, Tests) zur Freigabe, dann Developer.

## Freigabe Entwurf (Anwender 2026-10-13)

Entwurf `docs/analysis/mem05-design-2026-10-13.md` freigegeben mit allen
Empfehlungen: E1 Seiten aus dem Kernel-Heap; E2 Heap 2 → 8 MiB plus
Seitenbudget 512 Seiten fuer den MD-Vorrat; E3 Grenze 8192 Bloecke je
Prozess; E4 Boot-Argument `mdlimit=<n>` (`0` = aus); E5 `Malloc(-1)`
unveraendert; E6 Overlay `overlay/mintlib/malloc.c`, `MAXHUNK` 16 MiB,
Aufruf in `__libc_crtinit`; E7/E8 in v1 nein (Schritt 8 entfaellt).
Umsetzung in zwei Developer-Auftraegen: A = Schritte 1–6 + Doku (Kernel),
B = Schritt 7 (SDK). Nebenbefunde: [MEM07](MEM07.md), [MEM08](MEM08.md),
[MEM09](MEM09.md).

## Nachentscheidung (Anwender 2026-10-13) — `SNAP` im Seitenbudget

Teil A hielt vor Schritt 4 an: in `memprot.rs` haengen ausser `RECS`/`KEPT`
auch `SNAP` (`:929-932`, Momentaufnahme aller belegten Bloecke, bricht bei
260 stumm ab) und `VSCRATCH` (`:1641-1644`, `memprot=verify`) an
`MAX_RECORDS`. Entscheidung **B**: alle MEM05-Seiten (MDs, `RECS`, `SNAP`,
`KEPT`) zaehlen gegen das Budget von 512 Seiten; MEM05 belegt nie mehr als
2 MiB Heap. `VSCRATCH` fest 512. Die Kosten einer Synchronisierung
(O(N·64 + N log N) je `Malloc` mit Speicherschutz) werden als bekannte
Grenze dokumentiert; Umbau → [MEM10](MEM10.md).
