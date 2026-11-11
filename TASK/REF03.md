# REF03 — Drei falsche Spec-Zitate, zwei davon im Produktcode

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/REF03.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Quellenpflege — aus REF02

Abhaengigkeiten: [REF02](REF02.md) (dort gefunden und gegen die gepinnte Fassung geprueft)

## Die drei

**F1 — Power Control ist §2.2.11, nicht §2.2.1.** (§2.2.1 ist das
SDMA-Adressregister.) Im Audit bereits korrigiert, hier nur der
Vollstaendigkeit halber gefuehrt.

**F2 — Software Reset Register ist §2.2.16, nicht §2.2.15.** (§2.2.15 ist
Timeout Control.) Betroffen: `kernel/src/drivers/bcm2711/emmc.rs:56`, `:809`,
`:846` und `TASK/HW03-STATUS-DONE.MD:216`. **Der Inhalt ist jeweils richtig,
nur die Nummer falsch.**

**F3 — eine Spec-Begruendung traegt nicht.** `emmc.rs:850-855` (`soft_reset`,
aus HW03s Nebenbefund N1) sagt, beide Resets in **einem** Schreibzugriff seien
*„an optimisation the specification never asks for"*. **SD-HC 3.00 §3.8.1
Schritt (2) verlangt woertlich** *„Set **both** Software Reset For DAT Line and
Software Reset For CMD Line to 1"*.

**Die Aenderung selbst bleibt richtig** — QEMUs SDHCI dekodiert das
Zwei-Bit-Byte nicht (HW03s gemessener Befund). Falsch ist nur die Berufung auf
die Spezifikation. Die belegte Fassung steht in `docs/hardware/emmc-sd.md` §5:
§3.8.1 setzt beide Bits, §3.10.1 setzt jedes **bedingt und einzeln** — zwei
Vorschriften nebeneinander, und unsere Wahl folgt der zweiten aus einem
gemessenen Grund.

## Warum das mehr ist als Kosmetik

Ein falsches Zitat ueberlebt jede Durchsicht, weil es **wie** ein Beleg
aussieht. In dieser Ecke hat genau das schon einmal drei Tage gekostet
(`menu.rs`' Modulkopf, 71 Tage im Baum), und es ist die Regel dieses Projekts,
dass eine Begruendung selten nur einmal dasteht.

## Auftrag

Die drei Stellen richtigstellen, mit der Ausgabe **vor** der Abschnittsnummer
(`emmc-sd.md` §8 nennt die Regel und die Umrechnungstabelle 3.00 ↔ 4.20).
Bei F3 die Begruendung durch die belegte ersetzen, nicht die Aenderung.

Dazu ein Sweep: gibt es weitere `§2.2.x`-Zitate im Baum, die aus 4.20 stammen
und gegen unsere gepinnte 3.00 verschoben sind?

## Abnahme

Kein Zitat im Baum, das auf einen anderen Abschnitt zeigt als den gemeinten.
Keine Codeaenderung ausser Kommentaren.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
