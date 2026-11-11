# SLB01 — FreeMiNT shared libraries (SLB) and th-otto's zView

Status: PARKED (2026-11-11) | Priority: P3 | Reason: future plan (owner 2026-10-06): leave as is for now | Reopen: the owner takes up shared libraries

Migrated (owner batch 11, 2026-11-11), evidence: TASK/SLB01.md status line: ZUKUNFT (Anwender 2026-10-06: „erstmal so belassen, das ist ein Zukunftsplan")

Line 3 before the migration:
Status: **ZUKUNFT** (Anwender 2026-10-06: „erstmal so belassen, das ist ein Zukunftsplan") · Prioritaet: **P3** · Typ: FreeMiNT-Erweiterung

Abhaengigkeiten: [ARCH03](ARCH03.md) (gemeinsamer Adressraum, PROT_G) · zView-LDG-Weg (M1–M4) laeuft vorher fertig

## Warum

th-ottos gepflegte zView-Fassung (github.com/th-otto/zview) laedt ueber 100
Codecs und die Fremdbibliotheken (zlib, libpng, libjpeg …) als SLBs: vom
Kernel verwaltet, einmal geladen, von mehreren Programmen geteilt. Die LDG-
Fassung in `Applications/z-tools` hat 11 Codecs und linkt jede Bibliothek in
jeden Codec. Der Anwender haelt den th-otto-Weg fuer den sinnvolleren, sobald
die Basis steht.

## Arbeit (erst Recherche, dann Entscheidung mit Folgen je Option)

1. Spec aus FreeMiNT: `references/freemint/sys/slb.c`, `sys/mint/slb.h`,
   `Slbopen/Slbclose/Slbexec`, `Pexec 106`, Aufruf-Trampolin
   (`arch/user_things.S`), Rechte `PROT_G` fuer Nutzer der Bibliothek.
   ARCH03-Plan, Tabelle „Erweiterungen nach Stufe 3" (Schaetzung 4–6 Tage + Spec).
2. th-otto-zView auswerten: Quelle pinnen, Voraussetzungen (windom oder nicht,
   Bibliotheken als SLB, Codec-Liste), was von LDG-M1–M4 wiederverwendbar ist
   (M2-Bibliotheken, B1-AES-Fix, windom).
3. Erst danach Kernel-SLB, SLB-Verpackung der Bibliotheken, zView-Port.

Analysen: `docs/analysis/zview-ldg-2026-10-04.md` §1.4,
`docs/analysis/ldg-after-arch03-2026-10-06.md`.
