# NET06 — WLAN (CYW43455 ueber SDIO)

Status: PARKED (2026-11-11) | Priority: P4 | Reason: backlog, large hardware driver (WLAN CYW43455), after Ethernet | Reopen: NET01, NET02 and the Ethernet work are done

Migrated (owner batch 11, 2026-11-11), evidence: TASK/NET06.md status line: BACKLOG

Line 3 before the migration:
Status: **BACKLOG** · Prioritaet: **P4** · Typ: Hardwaretreiber (gross)

Abhaengigkeiten: [NET01](NET01.md), [NET02](NET02.md)

## Sachverhalt

Das WLAN der Pi 4 (Cypress/Infineon CYW43455 am SDIO-Bus) braucht:
SDIO-Hostcontroller (zweiter, nicht der EMMC-Pfad der SD-Karte),
Firmware-Upload aus einer Datei, das proprietaere `brcmfmac`-Protokoll
(Circle: `addon/wlan/`) und einen WPA2-Supplicant. Das ist die mit Abstand
groesste Einzelaufgabe im Netzbereich — nach Ethernet, nicht davor.

## Arbeit

Track-B-Vorlage (Circle `addon/wlan/`, Firmware-Lizenz, SDIO-Register aus
dem Datenblatt), dann Entscheidung des Leads.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
