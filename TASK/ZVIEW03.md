# ZVIEW03 — thumbnail cache per directory (future)

Status: PARKED (2026-11-11) | Priority: P3 | Reason: future (owner 2026-10-07): perhaps a per-directory cache, not now | Reopen: the owner takes up the thumbnail cache

Migrated (owner batch 12, 2026-11-11), evidence: TASK/ZVIEW03.md status line: ZUKUNFT (Anwender 2026-10-07: „perspektivisch mit einem Cache im jeweiligen Verzeichnis, aber nicht jetzt")

Line 3 before the migration:
Status: **ZUKUNFT** (Anwender 2026-10-07: „perspektivisch mit einem Cache im jeweiligen Verzeichnis, aber nicht jetzt") · Prioritaet: **P3**

zView dekodiert bei jedem Ordnerbesuch alle Vorschaubilder neu
(`docs/analysis/zview-sluggish-2026-10-07.md` §3: ~63 ms Rechenzeit je Foto in
QEMU, zweiter Besuch 513 ms trotz FS09-Cache). Idee des Anwenders: Vorschaubilder
in einer Cache-Datei im jeweiligen Verzeichnis ablegen. Offen: zView ist
Atari-Software und bleibt unveraendert — ob das ueber die Toolchain (Overlay),
ueber th-ottos gepflegte Fassung (SLB01) oder gar nicht geht, ist zu klaeren.
