# REF06 — BCM2835 ARM Peripherals fehlt in `references/`; Track B nennt die SD-Spezifikationen nicht

Status: **OFFEN — braucht den Lead** · Prioritaet: **P3** · Typ: Quellenlage — Restpunkte aus [REF02](REF02.md) (2026-09-28), aus dem STATUS-Log uebertragen 2026-10-26

Abhaengigkeiten: [REF02](REF02.md) (dort gefunden und gemeldet)

## Die zwei Punkte

**1. Das BCM2835-Datenblatt fehlt.** Das EMMC-Audit stuetzt sich auf
*BCM2835 ARM Peripherals*, Kap. 5 „External Mass Media Controller",
S. 65–81, als Quelle 2 (`docs/api/hw/emmc-audit.md:25`, Zustaendigkeit
`:65-69`) — die einzige Quelle fuer die 32-Bit-Gruppierung der Register.
In `references/` liegt nur das BCM2711-Datenblatt
(`references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`); REF02 hat
das BCM2835-Dokument ausdruecklich nicht geholt, weil sein Auftrag nur die
SD-Spezifikationen ins Netz liess (`TASK/REF02-STATUS-DONE.MD:92-95`,
`:336-338`; `TASK/REF02.md:131-134`).

**2. Track B fuehrt die SD-Spezifikationen nicht.** Die Quellenhierarchie
in `docs/SOURCE-ANALYSIS-PROTOCOL.md:73-82` nennt BCM2711-Datenblatt,
ARM ARM, Pi-4-Bootdoku, Richter-Tutorials, Linux, U-Boot/circle — aber nicht
die seit REF02 gepinnten SD Simplified Specifications (Physical Layer 9.10,
Host Controller 3.00; `references/README.md`, Tabelle unter „sd-spec"),
obwohl sie fuer SD/SDHCI-Arbeit primaer sind: das BCM2711-Datenblatt hat
kein EMMC-Kapitel. REF02s Vorschlag: eine Zeile „SD Simplified
Specifications — primaer fuer SD/SDHCI" zwischen Rang 3 und 4
(`TASK/REF02-STATUS-DONE.MD:344-348`).

## Auftrag

1. BCM2835 ARM Peripherals beschaffen und wie die anderen Quellen pinnen
   (Ausgabe, Datum, kanonische Quelle, SHA-256, Bytegroesse in
   `references/README.md`); Lizenz- und Bezugsfrage vorher klaeren.
2. Die Track-B-Hierarchie um die SD-Spezifikationen ergaenzen — eine
   Aenderung am Protokoll, deshalb Entscheidung des Leads.

[Gemeinsame Regeln](README.md) gelten.
