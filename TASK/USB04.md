# USB04 — Uebrige USB-Maus-Ausbauten aus der Fix-Liste (F5-Rest, F6, F7, F9)

Status: **OFFEN** · Prioritaet: **P3** (nach 1.0) · Typ: Sammelticket — Rest aus [USB01](USB01.md) (Anwender 2026-11-04)

## Inhalt

Aus `docs/api/usb/hid-mouse-protocols.md` §6, nichts davon gebaut:

- **F5 Rest:** Tasten 4/5 und horizontales Rad (AC Pan). Taste 3 ist in
  [USB03](USB03.md).
- **F6 Hotplug:** Umstecken ohne Neustart (Port-Status-Events, Hub-Status,
  Abbau des Slots, Neuenumeration ausserhalb des Timer-IRQ). Braucht die
  xHCI-Spec lokal (`references/`).
- **F7 Absolute Zeiger** (Tablets, KVM, QEMU `usb-tablet`): braucht vorher
  eine Produktentscheidung (Relativ-Umrechnung wie FreeMiNT `tablet` oder
  echter Absolutweg zur VDI).
- **F9 Root-Port-Reset mit Wiederholung, SuperSpeed-Hub/Intervall:** nur,
  wenn eine Messung es zeigt.

Je Punkt Spec in `docs/api/usb/` vor Code; F7 erst nach Anwenderentscheid.

[Gemeinsame Regeln](README.md) gelten.
