# NET01 — Netzwerk fuer NewTOS (Ziel Version 1.2): STinG/STiK-kompatibel

Status: OPEN (2026-11-11) | Priority: P2

Migrated (owner batch 13, 2026-11-11), evidence: TASK/NET01.md status line: OFFEN — Vorrecherche, no DONE receipt, no fix/feat commit names the id, owner 2026-11-11: stays OPEN (target 1.2, preliminary research (owner 2026-10-20))

Line 3 before the migration:
Status: **OFFEN — Vorrecherche** · Prioritaet: **P2 (Ziel 1.2, Pflicht)** · Typ: Erweiterung — Anwender 2026-10-20

Anwender: Netzwerk ist fuer **1.2 absolut Pflicht**; WLAN waere lieber, aber fuer
den Anfang sehr anspruchsvoll → zuerst kabelgebunden.

## Richtung

- Schnittstelle nach oben: **STinG/STiK** (Cookie `STiK`, Funktionstabelle) —
  damit laufen vorhandene TOS-Netzprogramme nach Neuuebersetzung ohne MiNT
  (Teli, Chatter, Fiffi, iFusion, Emailer/IConnect …).
- Spaeter (2.0, FreeMiNT-Richtung): MiNT-Sockets; die STiK-Schnittstelle bleibt
  ueber eine Bruecke wie FreeMiNTs GlueSTiK erhalten — keine Doppelarbeit.

## Quellen (docs.dev-docs.org, Kategorie Source Code)

STinG v1.20 Quelltext (2019), STiK 2 Quelltext, STiK 'C' devkit v1.11,
STiK 68K devkit, Draconis Socket-Library, EtherNE-Treiber (Beispiel fuer einen
STinG-Treiber).

## Vorrecherche (Researcher, kein Bau)

1. Lizenz von STinG und STiK (darf eine portierte Fassung verteilt werden?).
2. Aufbau: Anteil C / 68000-Assembler, Modulstruktur (Kern, STX-Module,
   Treiberschnittstelle nach unten), was davon NewTOS-seitig im Kern vs. als
   Programm laufen soll.
3. Hardware Track B: Ethernet der Pi 4 (GENET + BCM54213PE-PHY) — im
   BCM2711-Datenblatt beschrieben? Sonst nur Circle/Linux als Querreferenz
   (wie beim PM-Watchdog). Alternative USB-Ethernet-Adapter (USB ist vorhanden).
4. WLAN (CYW43455 ueber SDIO, Firmware-Blob, WPA) — Aufwand grob einordnen, fuer spaeter.
5. QEMU: welche Netzwerkgeraete bildet raspi4b nach (fuer Tests ohne Pi)?

[Gemeinsame Regeln](README.md) gelten.
