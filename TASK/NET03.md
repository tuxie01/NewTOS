# NET03 — `inet4` portieren: ARP, IP, ICMP, UDP, TCP, Routing

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner decision 2026-11-11 at the legacy migration: not on the work queue | Reopen: a card program or port is affected, or the owner schedules it

Migrated (owner batch 11, 2026-11-11), evidence: TASK/NET03.md status line: OFFEN, no DONE receipt, no fix/feat commit names the id

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P3** · Typ: Portierung (gross)

Abhaengigkeiten: [NET02](NET02.md)

## Sachverhalt

`references/freemint/sys/sockets/inet4/` = 27 `.c`, ~14 000 Zeilen, reines C
ohne m68k-Assembler: `arp.c`, `ip.c`, `icmp.c`, `igmp.c`, `tcp*.c`, `udp.c`,
`route.c`, `if.c`, `ifeth.c`, `bpf.c`. Das ist der Stack, den HighWire
(und jede andere Netzanwendung) sehen wird.

## Arbeit

1. Entscheiden: C-Port (Tarball+Overlay wie libc/gemlib, im Kernel gelinkt)
   oder Rust-Uebersetzung. **Memory-Regel „Port: reuse, don't rewrite"
   spricht fuer den C-Port**, aber der Kernel ist `no_std`-Rust — Machbarkeit
   (Buildweg, `extern "C"`-Grenze, Allokator, Locking) zuerst pruefen und
   dem Lead vorlegen.
2. Portieren, Schicht fuer Schicht, je mit Test gegen eine Gegenstelle.
3. Abnahme: `ping` von aussen beantwortet; TCP-Verbindung zu einem
   Host-Webserver steht (`curl` gegen uns bzw. wir gegen ihn).

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Erst Machbarkeitsvorlage, dann bauen.
