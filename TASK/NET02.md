# NET02 — `netif`-Schicht: FreeMiNTs Treiberschnittstelle nachbauen

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/NET02.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Portierung / Schnittstelle

Abhaengigkeiten: [NET01](NET01.md), [MOD01](MOD01.md) (Vertrag verstanden)

## Sachverhalt

FreeMiNTs Stack haengt an genau einer Struktur: `struct netif`
(`references/freemint/sys/sockets/inet4/if.h:100-150`) mit
`open/close/output/ioctl/timeout`, Sende-/Empfangsqueues (`struct ifq`),
Hardwareadresse, MTU, Zaehler. Alle mitgelieferten Treiber (`sys/sockets/xif/`)
sind m68k-Bus und fuer uns irrelevant — wir schreiben genau **ein** `netif`
fuer GENET.

## Arbeit

1. `netif` und `ifq` nach Rust uebersetzen (Felder 1:1, Zeiger → Traits oder
   Funktionszeiger — Entscheidung im Ticket begruenden).
2. GENET-Treiber aus NET01 als `netif` anbinden: `output` schiebt Rahmen in
   den Sendering, Empfang fuellt `rcv` und weckt den Stack.
3. `if_register`/`if_input`-Aequivalente (`inet4/if.c`).
4. Hosttest mit einem Dummy-`netif` (Rahmen rein/raus, Zaehler).

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.
