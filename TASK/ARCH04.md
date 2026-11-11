# ARCH04 — CPU-Eigenschaften zur Laufzeit aus `ID_AA64*_EL1` lesen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 9, 2026-11-11), evidence: TASK/ARCH04.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Robustheit — uebertragen aus dem alten GitHub-Issue #10 (2026-07-29, Repository NewTOS-OLD)

Statt Annahmen ueber den Cortex-A72 als Konstanten im Bootpfad zu halten,
die Feature-Register lesen. Anlass damals: `SCTLR_EL1_INIT` Bits `SED`/`ITD`
(RES1 nur ohne `FEAT_AA32EL0`, ARM ARM D24.2.174) und Break-before-make beim
Blockgroessenwechsel (`FEAT_BBML1/2`, D8.17.1) waren aus den Primaerquellen
nicht entscheidbar.

Stand 2026-10-19: `ID_AA64MMFR0_EL1.ASIDBits` wird schon gelesen
(`kernel/src/arch/aarch64/mmu.rs:167-185`). Der Cortex-A72-TRM liegt jetzt vor
(`references/arm-arm/cortex_a72_mpcore_trm_100095_0003_06_en.pdf`) — damit die
beiden Fragen zuerst aus der Quelle beantworten, dann entscheiden, was zur
Laufzeit gelesen werden muss.

[Gemeinsame Regeln](README.md) gelten.
