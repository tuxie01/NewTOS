# ARCH01 — Wettlauf beim EL0-Eintritt: eine IRQ zwischen `msr elr_el1` und `eret` verwirft das Sprungziel

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified (only docs(task) commit); receipt ARCH01-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 21, 2026-11-11, to PARKED), evidence: owner 2026-11-11 (batch 6 review): only commit 920c49549 is docs(task); receipt TASK/ARCH01-STATUS-DONE.MD names the old hash 7e9b4aa; reopen-derived

Line 3 before the migration:
Status: **ERLEDIGT (2026-09-24)** · Prioritaet: **P1** · Typ: Belegter Defekt / Architektur — gefunden bei DESK19

Abhaengigkeiten: —

## Sachverhalt

`enter_el0_with_resume_capture`
([`kernel/src/arch/aarch64/user_entry.rs`](../kernel/src/arch/aarch64/user_entry.rs))
stellt das Sprungziel in drei Schritten ein und springt dann:

```
msr   elr_el1, {entry}
mov   {tmp}, #0          // SPSR.M = EL0t, DAIF = 0
msr   spsr_el1, {tmp}
eret
```

**Ueber dieses Fenster sind IRQs nicht maskiert.** Trifft eine zwischen dem
Schreiben von `ELR_EL1` und dem von `SPSR_EL1`, ueberschreibt der
Ausnahmeeintritt `ELR_EL1` mit dem unterbrochenen **Kern**-PC. Der Behandler
kehrt in diesen Block zurueck, die naechsten zwei Befehle setzen `SPSR_EL1`
brav auf `0` (EL0t) — und der `eret` springt nach **EL0 auf eine
Kernadresse**.

Genau das zeigt das Protokoll:

```
[launch] EL0: slot 0 entry 0x1000
[EXC] synchronous (non-SVC) exception (Instruction Abort (lower EL)):
      ESR=0x000000008200000e ELR=0xffff0000000c6988 SPSR=0x0000000000000000
[EXC] FAR=0xffff0000000c6988
```

`SPSR = 0` (unser Wert kam durch), `ELR`/`FAR` = Kern-VA, symbolisiert auf
`enter_el0_with_resume_capture + 0x6c`, also **hinter** dem `eret`; IFSC
`0x0e` = Permission Fault Level 2, was EL0 auf einer Kernseite genau ergibt.
Die Registerwerte passen ausschliesslich zu diesem Fenster.

**Haeufigkeit:** einmal in sieben Laeufen desselben Startskripts auf
identischer Kartenkopie (`measurements/measurement-2026-09-23-desk19-desk20/serial/`,
`d19-full-serial.log` gegen `d19-full-repeat1..4-serial.log`, RUNINFO §7).

**Wer betroffen ist:** *jeder* Aufrufer des Helfers — Programmstart per
Doppelklick, `\AUTO\`, `Pexec`, acht Selbsttests. Der Fehler ist aelter als
DESK19; DESK19 hat nur eine weitere Aufrufstelle und damit mehr Gelegenheiten
geschaffen.

## Warum P1

Ein Programmstart, der in etwa jedem siebten Versuch mit einer
Ausnahmemeldung endet, ist genau das, was am Vorfuehrtermin nicht passieren
darf — und es trifft den Weg, ueber den **jede** portierte Anwendung startet.

## Arbeit

Quelle: ARM ARM zu `ERET`, `ELR_EL1`, `SPSR_EL1` und dem Ausnahmeeintritt —
welche Register der Eintritt schreibt, und was `eret` aus `SPSR_EL1` in
`PSTATE.DAIF` zurueckholt. Danach das Fenster schliessen (die uebliche Form
ist `msr daifset, …` vor dem ersten Schreiben; der `eret` stellt DAIF aus
`SPSR_EL1` wieder her, das hier `0` ist, also laeuft EL0 mit offenen IRQs
weiter). **Vor dem Fix belegen, dass es der Wettlauf ist** — z. B. durch eine
kuenstlich eingesetzte Verzoegerung zwischen den beiden `msr`, die ihn
zuverlaessig ausloest; sonst ist der Fix eine Vermutung.

Abnahme: derselbe Startlauf **zwanzigmal** hintereinander ohne `[EXC]`, plus
der kuenstlich ausgeloeste Fall vorher/nachher.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Architekturdatei — Rule 6 der
CLAUDE.md: Registerverhalten nur aus der ARM ARM, nicht aus dem Gedaechtnis.


---

## Abschluss (2026-09-24)

**Die Vermutung haelt — und sie wurde ausgeloest, nicht geglaubt.** In einer
Kopie des Baums steht zwischen `msr elr_el1, {entry}` und dem `eret` eine
Warteschleife ueber 16.777.216 Durchlaeufe, also weit mehr als die 5 ms der
200-Hz-Uhr (`measurements/measurement-2026-09-24-arch01/method/force_race_patch.py`).
Damit faellt **jeder** Start, und zwar mit genau der gemeldeten Signatur:

```
[launch] EL0: slot 0 entry 0x1000
[EXC] synchronous (non-SVC) exception (Instruction Abort (lower EL)):
      ESR=0x000000008200000e ELR=0xffff0000000c4e80 SPSR=0x0000000000000000
[EXC] FAR=0xffff0000000c4e80
```

`ELR`/`FAR` zeigen auf das `subs` **in der eingesetzten Schleife**
(`evidence/forced-before-disasm.txt`) — also auf den Kern-PC, den der
Ausnahmeeintritt nach `ELR_EL1` geschrieben hat und den der `eret` nach EL0
mitgenommen hat. Dieselbe Kopie mit der Maske davor startet durch
(`serial/forced-after-serial.log`).

**Quellenlage.** Die ARM ARM liegt nicht unter `references/`; benutzt wurde
DDI 0487M.c ueber Arms Dokumentendienst, die Abschnitte woertlich:
D1.4.2 „Exception entry" R_DQXFW (der Eintritt schreibt `SPSR_ELx` **und** die
Rueckkehradresse nach `ELR_ELx` — das ist der Defekt) und R_WTXBY
(`PSTATE.{D,A,I,F,SP}` werden auf 1 gesetzt — deshalb ist der Trap-Rueckweg
von Haus aus sicher); D1.4.4 „Exception return" R_BWCFK und C6.2.157 `ERET`
(der Rueckweg setzt `PSTATE` aus `SPSR_EL1`); C6.2.288 `MSR (immediate)`
(`DAIFSet` verodert in `PSTATE.{D,A,I,F}`); C5.2.3 `DAIF`.

**Der Fix** ist eine Zeile in
[`kernel/src/arch/aarch64/user_entry.rs`](../kernel/src/arch/aarch64/user_entry.rs),
`enter_el0_with_resume_capture`: `msr daifset, #0xf` vor der ersten
Zustandsschreibung. Die Maske ueberlebt den `eret` nicht — das geschriebene
`SPSR_EL1` ist `0`, EL0 laeuft also weiter mit offenen Interrupts und nimmt
die angestaute IRQ sofort entgegen. Asynchrone Ausnahmen sind die einzigen,
die in das Fenster gelangen koennen; die Befehle darin ruehren keinen
Speicher an.

**Zwei Korrekturen an der Aufzaehlung im Ticket.**

1. Es sind **elf Aufrufstellen in zehn Dateien**: `launch.rs:122` und **neun**
   Selbsttestdateien (nicht acht; `selftest_sleepwake_switch.rs` ruft
   zweimal). Keiner dieser Selbsttests haengt heute in einem Bootpfad
   (`lib.rs` ruft nur `selftest_emmc::run()`).
2. **`Pexec` ist nicht betroffen.** Der GEMDOS-Trap aus EL0 benutzt den Helfer
   nicht: er schreibt den Ausnahmerahmen um
   (`exceptions.rs:1509-1520`) und kehrt ueber `EXC_BODY` zurueck, wo `DAIF`
   seit dem Ausnahmeeintritt maskiert ist. Im Betrieb ist `launch.rs` der
   einzige Verbraucher — Doppelklick/Ablegen, `\AUTO\`, `#Z`-Autostart.

Ein **zweites Fenster derselben Form gibt es nicht**: der Kernel hat drei
`eret` — den Vektorrueckweg (maskiert; `gic::enable_irqs` hat genau einen
Aufrufer, `lib.rs:299`, einmal beim Booten), den EL2→EL1-Abstieg beim Booten
(`SPSR_EL2 = 0x3c5`, vor Vektortabelle und GIC) und diesen.

Ebenfalls praezisiert: das gemeldete `+ 0x6c` liegt **im** Fenster, nicht
hinter dem `eret` — in HEADs Bau ist das das `mov x11, #0` direkt nach
`msr elr_el1` (`evidence/head-nofix-disasm.txt`).

**Abnahme.** Derselbe Startlauf (DESK19 `d19-full`) **zwanzigmal**, je eigene
frische Kartenkopie: 0 `[EXC]`, 0 `[PANIC]`, 40 EL0-Starts (je `\AUTO\` +
Ziehstart), 20 von 20 mit `CMDLINE: len 13 arg "C:\README.TXT"`. Ein Lauf ist
gegen den DESK19-Referenzmitschnitt zeilengleich bis auf den Uhrzeitstempel.
qemu-virt bootet bis `[desk] desktop up`. Bau und Clippy auf beiden Boards
ohne Delta (64/94 bzw. 80/125), Testsuite 938 / 0 / 1.

Messung: [`measurements/measurement-2026-09-24-arch01/`](../measurements/measurement-2026-09-24-arch01/).
Dokumentiert als **QUIRK-003** in [`docs/MEMORY.md`](../docs/MEMORY.md), plus
die Regel im Prozessmodell ([`docs/ARCHITECTURE.md`](../docs/ARCHITECTURE.md)).
Empfang: [`ARCH01-STATUS-DONE.MD`](ARCH01-STATUS-DONE.MD).
