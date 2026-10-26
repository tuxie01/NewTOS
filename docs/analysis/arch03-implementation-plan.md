# ARCH03 Schritt 2 — Implementierungsplan: ein gemeinsamer Adressraum mit Schutzrechten je Prozess

Stand 2026-10-04 · Architekten-Auftrag zu `TASK/ARCH03.md` Schritt 2 · **Kein Code, keine Commits.** Eingabe: `docs/analysis/arch03-freemint-memory-model.md` (Schritt 1, im Folgenden **S1**). Umsetzung erst nach der OFAM (9./10. Oktober 2026) und nach Freigabe durch den Anwender.

Pins: FreeMiNT `f906416` (`references/freemint`), EmuTOS `1a7484ab` (`references/emutos`), NewTOS-Zweig `desk-rebuild`.

**Kennzeichnung** (wie S1): **[gelesen]** = in dieser Sitzung `file:line` gelesen · **[S1]** = Beleg aus der Schritt-1-Analyse übernommen, dort `[gelesen]` · **[Schluss]** = gefolgert · **[Track B offen]** = AArch64-Architekturaussage, die vor der Umsetzung gegen das ARM ARM zu prüfen ist (das ARM ARM lag bei Erstellung nicht in `references/`; **0a:** inzwischen gepinnt in `references/arm-arm/` — ARM ARM DDI 0487 **M.d**, Cortex-A72-TRM r0p3, AAELF64 2025Q4; Ergebnis je Aussage in `docs/analysis/arch03-stage0a-armarm-verification.md` (**V0a**). Wo unten „0a:“ steht, ist die Aussage dort belegt oder korrigiert; Seitenangaben beziehen sich auf diese Ausgaben) · **[Schätzung]** = Aufwand, kein Beleg.

Pfadkürzel: `FM` = `references/freemint/sys`, `XA` = `references/freemint/xaaes/src.km`, `ET` = `references/emutos`, `NT` = `tos-api/src/bdos/process`, `KA` = `kernel/src/arch/aarch64`.

---

## 0. Kurzfassung

1. **Zielmodell:** Ein Anwenderadressraum, in dem **jede Anwenderadresse gleich ihrer physischen Adresse ist (VA = PA)** — mit genau einer benannten Ausnahme, der prozesseigenen Seite 0 (Basepage-freie Systemvariablen-Seite, Abschnitt 3.1). Jeder Prozess (genauer: jeder `memspace`) hat eine **eigene Wurzeltabelle in `TTBR0_EL1`**, die über **denselben** Abbildungen liegt und sich nur in den **Rechten** unterscheidet — FreeMiNTs Modell (`FM/arch/mprot030.c:18`, `:76-80` [S1]), auf AArch64 übertragen.
2. **Gestuft wie FreeMiNT mit `no_mem_prot`:** Stufe 1 baut den gemeinsamen Adressraum **ohne** Rechteunterschiede (alle Prozesse teilen die Tabellen unterhalb der Wurzel), Stufe 3 führt die Rechte `PROT_P/G/S/PR/I` je Adressraum ein. Dazwischen liegen die `Pexec`-Vollständigkeit (Stufe 2) und davor die Vorarbeiten (Stufe 0: Track-B-Nachweis, QEMU/Pi-Spike, Fehlerpfad, PIE-Lader). Jede Stufe ist für sich auslieferbar.
3. **Programme werden positionsunabhängig (ELF `ET_DYN`, nur `R_AARCH64_RELATIVE`)** und vom Kernel beim Laden reloziert — wie GEMDOS-`.PRG` mit Relokationstabelle bei EmuTOS (`ET/bdos/kpgmld.c`) und FreeMiNT (`FM/memory.c:1699-1815` [S1]). Das ist die Voraussetzung, ohne die ein gemeinsamer Raum nicht geht (zwei Programme können nicht beide bei `0x1000` liegen, S1 C.9 Nr. 3).
4. **Die AES behält ihren Zugriff ohne `TTBR0`-Akrobatik:** Weil VA = PA gilt, erreicht der Kernel jeden Anwenderzeiger über seinen eigenen High-Half-Alias (`va + 0xFFFF_0000_0000_0000`). Das ist das AArch64-Gegenstück zu FreeMiNTs `prot_temp`/`F_OS_SPECIAL` — ohne TLB-Pflege. Ausgeführt wird Anwendercode (Userdef, `kbdvec`) dagegen nur mit den Rechten des Eigentümers (XaAES-Muster, Abschnitt 2 Frage 9).
5. **Aufwand [Schätzung]:** Stufe 0 ≈ 9–13 Tage, Stufe 1 ≈ 8–12, Stufe 2 ≈ 8–11, Stufe 3 ≈ 10–15, Stufe 4 ≈ 3–5; Erweiterungen (SLB, `kbdvec` global, `Mvalidate`) danach. **Bis einschließlich Stufe 3 rund 38–56 Arbeitstage**, seriell.
6. **Wichtigste offene Risiken:** (a) ADR-010s „Wand 3“ (Kernel/Anwender-Aliasing hängt QEMU) — **wird vom laufenden System widerlegt**, denn der Lader bildet heute jede TPA-Seite mit abweichenden Rechten neben dem Kernel-Alias ab (Frage 2); vor Stufe 1 trotzdem per Spike nachmessen. **0a:** Wand 3 ist keine Architekturregel — nach ARM ARM B2.11 (S. B2-331/332) gelten nur Speichertyp, Shareability und Cacheability als „mismatched attributes“, Rechte-Unterschiede (AP/XN) zwischen Aliasen nicht; wenn überhaupt ein QEMU-Effekt (V0a §9 I6). **0b:** auf QEMU 11.0.2 (`raspi4b` und `virt`) nicht reproduzierbar — sechs Abbildungsvarianten und die exakte Nachstellung des Versuchs vom 2026-06-16 laufen, je dreimal (S0b = `docs/analysis/arch03-stage0b-spike.md` §3); Pi-Lauf offen. (b) ASIDs auf QEMU (QUIRK-001) — das Modell braucht sie nicht; sie sind eine spätere, optionale Beschleunigung. **0b:** auf QEMU 11.0.2 funktionieren sie (16-Bit-ASIDs gemeldet, getaggter `msr` hängt nicht, Wechsel ohne TLBI korrekt; S0b §4). (c) ~~Alle AArch64-Bitaussagen sind [Track B offen].~~ **0a:** gegen ARM ARM M.d und A72-TRM geprüft (V0a); offen bleiben nur das QEMU-Verhalten (→ 0b) und die ELF-Dynamic-Section (SYSVABI64 nicht gepinnt, → 0d).

---

## 1. ADR-Entwurf

> Nächste freie Nummer: ADR-024 ist vergeben (`docs/MEMORY.md:1885`, VDI-Truecolor), `ADR-025` kommt in keinem Dokument vor (`grep -rn "ADR-02[5-9]" docs TASK`, 2026-10-04, ohne Treffer). Der Text bleibt hier, bis der Anwender ihn freigibt.

## ADR-025: Ein gemeinsamer Anwenderadressraum (VA = PA) mit Schutzrechten je Adressraum — FreeMiNTs Modell auf AArch64

> **Übernommen als ADR-028** in `docs/MEMORY.md` (2026-10-26). Die Nummer 025 ging an die MD-Versorgung (MEM05); der Entwurf unten bleibt als Stand vom 2026-10-04 stehen.

**Date:** 2026-10-04
**Status:** Proposed
**Phase:** cross-cutting (Prozessmodell, Lader, BDOS-Speicher, MMU, AES)
**Supersedes:** ADR-010 (Entscheidung übernommen, Umsetzungsweg ersetzt), ADR-021 (Fenster-Mechanismus; Pool-Teil bleibt). **Extends:** ADR-023 (`PROT_*`-Vokabular wird wirksam).

### Context

NewTOS gibt heute jedem Prozess einen eigenen EL0-Adressraum: Basepage bei VA 0 (`NT/loader.rs:170`), Programmbild fest bei `0x1000` (`NT/elf.rs:341-343`, `sdk/newtos.ld`), Speicherblöcke in ein prozesseigenes Fenster mit VA ≠ PA (`NT/usermem.rs:26-31,308-363`). Ein Zeiger gilt nur in dem Prozess, der ihn bekam. Damit sind `Pexec` 3/4/5/6/7 nur als Handle-Krücke oder gar nicht umsetzbar (`NT/pexec.rs:75-81,177-179`), LDG geht nicht (Modul schreibt über `OFFSETLDG` in den Stapel des Aufrufers, `Applications/ldg-src/ldg/src/devel/ldglib.c:134-137` [S1]), residente Programme verlieren ihre Abbildung (S1 C.9 Nr. 21), MEM03 (`p_env` ist eine nicht abgebildete PA) und der Userdef-Fall mit fremdem Baumbesitzer (`docs/analysis/coop-kernel/30-aes-thread-design.md:693-705`) sind Folgen desselben Entwurfs.

EmuTOS hat einen Adressraum ohne Schutz (`ET/bdos/proc.c:179-361`, S1 B). FreeMiNT hat einen Adressraum mit per-Prozess-Seitentabellen, die nur die Rechte unterscheiden (`FM/arch/mprot030.c:18`: „initially, the logical address space is the physical address space“; `:76-80`: „You have one of these per process“ [gelesen]); der Schutz ist abschaltbar, ohne dass sich Regionen, Besitz oder `Pexec` ändern (`FM/arch/mprot030.c:242-246` u. a., S1 A.13).

Anwenderentscheidung 2026-10-04: „es so wie bei FreeMiNT bauen“.

### Decision

1. **Ein gemeinsamer Anwenderadressraum, VA = PA** für alles, was Prozessen gehört oder ihnen übergeben wird: beide BDOS-Pools, der Bildschirmspeicher, veröffentlichte Kernelstrukturen (ADR-023). **Einzige Ausnahme:** die Seite VA `[0, 0x1000)` bleibt je Prozess privat (klassische Systemvariablen, OSHEADER, `$5A0`, Trampoline — B1, KEYS02), Abschnitt 3.1.
2. **Eine `TTBR0`-Wurzel je Adressraum (`memspace`).** In Stufe 1 teilen alle Wurzeln die Tabellen unterhalb der Wurzel (außer der Tabelle, die Seite 0 enthält): kein Schutz zwischen Prozessen — FreeMiNT mit `no_mem_prot`. Ab Stufe 3 bekommt jeder `memspace` eigene Blatttabellen, wo seine Rechte von denen anderer abweichen: **FreeMiNTs `PROT_P/G/S/PR/I`** mit FreeMiNTs Bedeutung (`FM/mint/mem.h:120-136` [gelesen]).
3. **Programme sind positionsunabhängig** (`ET_DYN`, ausschließlich `R_AARCH64_RELATIVE`), der Kernel lädt sie in einen Pool-Block und reloziert sie dort, wie GEMDOS es mit `.PRG` tut. `ET_EXEC` wird nach Stufe 1 mit `EPLFMT` abgelehnt.
4. **Besitz nach FreeMiNT (`MEMREGION`, mehrere Besitzer, `links`)**, Freigabe-Etikett weiter nach EmuTOS (`m_own` = PD-Zeiger), Abschnitt 3.4.
5. **Kernelzugriff auf Anwenderspeicher** über zwei Primitive: mit den Rechten des Aufrufers (unprivilegierte Lade-/Speicherbefehle über dessen `TTBR0`, mit Fehlerabfang) für Syscall-Argumente; über den Kernel-Alias (`va + VA_OFFSET`, nach Prüfung in der Regionstabelle) für kernel-initiierte Zugriffe auf fremde Prozesse (AES). Anwendercode läuft nur mit den Rechten seines Eigentümers.
6. **Schutzverletzung = Programmabbruch mit Meldung** (EmuTOS: Meldung + `Pterm(-1)`, `ET/bios/kprint.c:587-595`, `ET/bios/panicasm.S:96-99` [gelesen]); FreeMiNTs `SIGBUS`-Handler später.

### Rationale

- **Zeiger müssen über Prozessgrenzen dieselbe Bedeutung haben** — das ist die gemeinsame Voraussetzung von `Pexec` 3/4/5/6/7, LDG, SLB, TSR, Accessories, AES-Nachrichten mit Zeigern, globalem `Mxalloc` und `shm` (S1 §0 Nr. 2, Lückentabelle D). Ein Fenster je Prozess kann das prinzipiell nicht, ein gemeinsamer Raum kann es trivial.
- **VA = PA statt fester Verschiebung:** `Malloc` liefert dann wieder genau `m_start` (EmuTOS-treu, keine Seiten-Übervergabe mehr wie heute in `tos-api/src/gemdos/memory.rs:90-119`); die MDs, die `Getmpb` ausliefert, enthalten direkt benutzbare Adressen; der ST-RAM-Pool liegt bei `0x0040_0000..0x00E0_0000` (Bootprotokoll `measurements/measurement-2026-09-14-x01/before/qed-BOOT-SERIAL.txt:29` [gelesen]) — also unterhalb 16 MiB wie auf einem ST, was dem späteren m68k-Emulator gemeinsame Zeiger erlaubt. Eine feste Verschiebung brächte keinen Vorteil: das Aliasing mit der Kernel-Hälfte bleibt dasselbe.
- **Alle Anwenderadressen bleiben positive 32-Bit-`Long`:** Pool-Ende ist durch `TOS_MD_MAX_END = 0x7FFF_F000` begrenzt (`kernel/src/bios/machine/layout.rs:120` [gelesen]) und `seed_md` lehnt alles Breitere ab (`tos-api/src/bdos/mem/init.rs:224-236` [gelesen]). Damit passen Basepage (`tos-types/src/basepage.rs:95-135`), DTA und Cookie-Werte ohne Formatänderung.
- **Erst Raum, dann Rechte:** FreeMiNT zeigt, dass alles außer den Seitentabellen vom Schutz unabhängig ist (S1 A.13). Stufe 1 liefert damit sofort MEM03, echte PD-Zeiger und `Pexec 3/4`; die Rechte sind ein späterer, eigenständiger Schritt.
- **Ein Mechanismus für Prozesse, AES und Kernel:** Mit VA = PA ist „Anwenderzeiger → Kernelzeiger“ eine Addition; FreeMiNT braucht dafür `prot_temp` mit Seitentabellen-Umbau (`FM/arch/mprot030.c:775-805` [gelesen]), NewTOS nicht.

### Consequences

- Positiv: echte PD-Zeiger (`p_parent`, `Pexec(3)`-Rückgabe), `Pexec` alle EmuTOS-Modi und die FreeMiNT-Modi, LDG unverändert lauffähig, TSR, globaler Speicher, MEM03 gelöst, keine Fenstererschöpfung mehr (S1 C.4; `zview-ldg-2026-10-04.md` L5), Userdef fremder Baumbesitzer lesbar.
- Negativ: **jedes** native Programm muss neu gebaut werden (PIE, neues `crt0`, neues Linkskript); die neun Selbsttests mit handgebauten ELF-Blobs ebenso. Stufe 1 hebt die heutige Isolation zwischen Prozessen auf (ein fehlerhaftes Programm kann ein anderes beschädigen — wie in EmuTOS, bis Stufe 3). Ab Stufe 3 Pool-Allokationen auf Seitengröße gerundet (FreeMiNT-`QUANTUM`, `FM/mint/mem.h:152,200-205` [S1]) — beobachtbar in `Malloc(-1)`/`Getmpb`.
- Neutral: Seite 0 bleibt prozesseigen (sanktionierte Abweichung, schon heute). Der `TTBR0`-Wechsel samt QUIRK-001-Klammer bleibt (er wird ab Stufe 3 auch gebraucht, um Rechte zu wechseln).

### Alternatives considered

- **ADR-010 wörtlich (ein einziges `TTBR0`, nie gewechselt, kein Schutz):** einfachster Endzustand, aber Seite 0 könnte nicht prozesseigen sein (B1-Spiegel, KEYS02-Tabelle, Trampolin-Scratch `0x730/0x760`), und es gäbe keinen Weg zu Rechten ohne zweiten Umbau. Als Zwischenstand verworfen zugunsten „eigene Wurzel, gemeinsame Tabellen“ — gleiche Wirkung, aber der Weg zu Stufe 3 ist bereits angelegt.
- **ADR-021 weiterführen (Fenster je Prozess) und Zeiger beim Übergeben umrechnen:** unmöglich für Zeiger, die der Kernel nicht sieht (LDG `OFFSETLDG` steht als Text in der Umgebung, `ldgload.c:107-113` [S1]; Strukturen mit eingebetteten Zeigern in Nachrichten).
- **Gemeinsame Abbildung derselben Blöcke an verschiedenen VAs je Prozess** (`docs/analysis/emutos-memory-model.md` §4.6 letzter Punkt): löst Sichtbarkeit, aber nicht Zeigergleichheit — LDG und AES-Zeiger brauchen dieselbe Adresse.
- **Userspace-LDG-Lader** (`zview-ldg-2026-10-04.md` §4.2): umgeht das Problem nur für LDG; `Pexec 5/6/7`, TSR, SLB, `p_env` bleiben offen. Mit ADR-025 überflüssig.
- **Volle Prozessisolation ohne gemeinsame Adressen (eigene Räume + IPC):** widerspricht dem TOS-Vertrag (PD-Zeiger, `Pexec(3)`), verworfen schon in ADR-010.
- **Sofort Rechte (ohne Stufe 1):** größerer Einzelschritt, zwei Fehlerquellen (Adressraum und Rechte) gleichzeitig; FreeMiNT selbst trennt beides. Verworfen.
- **Feste Ladeadresse je Programm (Adressvergabe statt Relokation):** braucht eine globale Adressverwaltung für Programmbilder, verträgt keine zwei Instanzen desselben Programms (zwei `ex_lib`-Module, LDG lädt je Client neu, `ldgapp.c:37-38` [S1]). Verworfen.

### References

- S1 (`docs/analysis/arch03-freemint-memory-model.md`), dieser Plan.
- FreeMiNT `FM/arch/mprot030.c`, `FM/memory.c`, `FM/k_exec.c`, `FM/k_exit.c`, `FM/dosmem.c`, `FM/mint/mem.h`; XaAES `XA/k_main.c`, `XA/messages.c`, `XA/draw_obj.c`.
- EmuTOS `ET/bdos/proc.c`, `ET/bdos/umem.c`, `ET/bios/kprint.c`, `ET/bios/panicasm.S`.
- ADR-010, ADR-014, ADR-020 (Nachtrag), ADR-021, ADR-023, QUIRK-001, QUIRK-003.

---

## 2. Antworten auf die 20 offenen Fragen aus S1 §F

### Frage 1 — Ein `TTBR0` für alle oder eines je Prozess? Stufung?

**Antwort:** Eine Wurzel je `memspace` über identischer VA→PA-Abbildung (FreeMiNT), gestuft: Stufe 1 teilt alle Tabellen unterhalb der Wurzel außer der Seite-0-Tabelle (Wirkung = ein gemeinsamer Raum ohne Schutz, FreeMiNT `no_mem_prot`), Stufe 3 trennt die Blatttabellen dort, wo Rechte abweichen.

**Begründung/Belege:**
- FreeMiNT: eine Tabelle je Prozess über derselben Identität, nur Rechte verschieden (`FM/arch/mprot030.c:18,76-80` [gelesen]); abschaltbar ohne Semantikänderung (S1 A.13).
- Der Wurzelwechsel ist in NewTOS **bereits produktiv** und auf QEMU und Pi erprobt: `restore_boot_ttbr0` (TLBI) → UART-Ausgabe → `install_ttbr0` (`KA/exceptions.rs:1056-1066`, `:1377-1382`, `:1910-1920` [gelesen]). Er bleibt in Stufe 1 nötig, weil Seite 0 je Prozess verschieden ist; die Klammer ist also kein neues Risiko.
- QUIRK-001 entsteht durch „gleiche VA, verschiedene PA, ASID 0“ (`docs/MEMORY.md:2103-2117` [gelesen]). Im gemeinsamen Raum gilt das nur noch für Seite 0; für alles andere ist dieselbe VA dieselbe PA, ein vom TCG wiederverwendeter Übersetzungsblock ist dort **richtig**. **[Schluss]** Die Klammer bleibt trotzdem, bis der Spike (Stufe 0b) zeigt, dass sie entfallen kann. **0a:** Architektonisch nötig ist nur die TLB-Pflege (leere Wurzel → `tlbi vmalle1` → `dsb ish` → `isb` → neue Wurzel; ARM ARM `IRCGCH` D8-7852, `RXCLRD` D8-7852, `IBLDZX` D8-7875; Repo `kernel/src/arch/aarch64/user_entry.rs:101-115` [gelesen]); die UART-Klammer ist ein reiner QEMU-Workaround, und die QUIRK-001-Begründung „PIPT ⇒ Reihenfolge unerheblich“ ist unvollständig (V0a §9 I3/I4). **0b:** Auf QEMU 11.0.2 ist die Klammer entbehrlich: die Folge „leere Wurzel → `tlbi vmalle1` → `dsb ish` → `isb` → neue Wurzel → `isb` → `eret`“ ohne jeden Gerätezugriff wechselt zwischen zwei Wurzeln mit Code an derselben VA (Seite 0) 32/32-mal richtig, auf `raspi4b` und `virt` (S0b §4, `sw-arch`). Die Klammer kann nach Bestätigung auf der Pi entfallen.

### Frage 2 — Ist VA = PA überhaupt möglich?

**Antwort: Ja, auf beiden Boards.** Bedingungen und Prüfung:

| Bedingung | Befund | Beleg |
|---|---|---|
| Pool in 31 Bit | ST-RAM `0x0040_0000+10 MiB`, TT-RAM `0x0100_0000+942 MiB` (rpi4, QEMU `raspi4b` und Pi); `qemu-virt`: `0x4040_0000`, `0x4100_0000+246 MiB`. Obergrenze `TOS_MD_MAX_END = 0x7FFF_F000`, Pool-Kappung `POOL_MAX_BYTES = 1 GiB`. | Bootprotokolle `measurements/measurement-2026-09-14-x01/*/qed-BOOT-SERIAL.txt:29`; `layout.rs:105,120` [gelesen] |
| Keine Kollision mit Seite 0 | Pool beginnt bei ≥ 4 MiB; VA `[0, 0x0040_0000)` ist kein Pool (Kernel-Reserve `STRAM_OS_RESERVE = 4 MiB`, `layout.rs:83` [gelesen]). Die prozesseigene Tabelle für `[0, 2 MiB)` stößt an nichts. | — |
| Bildschirmspeicher | liegt in VideoCore-Speicher oberhalb `dram.end` (ADR-014 Folgen, `docs/MEMORY.md:502-506`) — auf dem Pi 4 unterhalb 1 GiB **[Schluss, nicht gemessen]** → VA = PA < 2 GiB. Vor Stufe 1 aus dem Bootprotokoll (`Physbase`-PA) belegen. | Prüfpunkt Stufe 0b. **0b:** QEMU `raspi4b` `0x3C10_0000..0x3C35_8000` (VideoCore-Speicher oberhalb `dram.end`, nur 1-MiB-ausgerichtet → in TTBR0 als 4-KiB-Seiten abbilden), `virt` `0x5081_7000..` (OS-Reserve) — beide < 2 GiB; Pi offen (S0b §6) |
| PXN für EL1 auf Anwenderseiten | unverändert: jede Anwendervorgabe setzt PXN (`NT/mmu.rs:156-165,202-203` [gelesen]). **0a:** bei `AP=01` ist EL1-Ausführung ohnehin entfernt (`RVWLLR` D8-7758, Table D8-65); PXN wirkt real bei `AP=11` (V0a A3). Der Kernel führt nie Anwendercode aus — auch nicht über den Alias, denn der High-Half-Alias ist Kernel-Normalspeicher. | — |
| **QEMU-Aliasing-Wand (ADR-010 „Wand 3“)** | ADR-010 meldet einen Hänger beim ersten Kernel-Store nach Installieren einer Identitätsabbildung mit EL0-Rechten (`docs/MEMORY.md:155-166`). **Das laufende System widerlegt die Diagnose „Aliasing mit abweichenden Rechten“:** Der Lader bildet jede TPA-Seite als `UserRwExec` bei VA 0 ab (`NT/loader.rs:546-560`), während der Kernel dieselben Seiten über `base_kva` beschreibt (`NT/loader.rs:463-470`; `refresh_cookie_mirrors` `:1129-1138`; Keytbl/Kbdvec über `basepage_kva + offset`, S1 C.8) — auf QEMU `raspi4b` und auf der Pi, seit Monaten. Unterschiede zum Versuch von 2026-06-16: 2-MiB-Blöcke, ganzes DRAM inklusive Kernel-Heap, Installation beim Booten **mit** TLBI, Board `virt`, QEMU 11.0.0 (installiert ist jetzt 11.0.2, `qemu-system-aarch64 --version` [gelesen]). **0a:** Die Architektur erlaubt Aliase mit abweichenden Rechten bei gleichem Typ/Shareability/Cacheability (B2.11, S. B2-331/332; Kernel-Alias und Anwender-Alias sind beide Normal/Inner-Shareable) — die Wand ist nicht architektonisch begründbar (V0a §9 I6). | **Spike Stufe 0b** klärt, welcher dieser Unterschiede der Auslöser war. **0b:** keiner davon löst auf QEMU 11.0.2 noch etwas aus: Pools in 2-MiB-Blöcken und 4-KiB-Seiten, ganzes DRAM inkl. Kernel-Heap, mit/ohne TLBI, `raspi4b` und `virt`, sowie der Austausch der gefüllten Boot-Identität gegen die EL0-Identität vor `heap::init` — alles läuft, Kernel- und EL0-Sicht kohärent (S0b §3). Der damalige Hänger ist mit 11.0.2 nicht mehr nachstellbar [Schluss: QEMU 11.0.0 oder der damalige PA-dereferenzierende Heap] |

**Rückfallebene**, falls der Spike eine echte Wand findet: Pool-Seiten werden nicht beim Booten, sondern **bei der Allokation** in die gemeinsamen Tabellen eingetragen (4-KiB-Seiten, frische VAs — exakt das heutige `usermem::map`-Muster, nur mit VA = PA). Das ist bewiesenermaßen QEMU-tauglich. **0b:** Auf QEMU wird die Rückfallebene nicht gebraucht (Boot-Abbildung funktioniert, S0b §3); sie bleibt nur für den Fall, dass der Pi-Lauf widerspricht.

### Frage 3 — Ladeadressen, PIE, vorhandene `.NAP`/`.NCP`, SDK-Linkskript

**Antwort:** PIE (`ET_DYN`), Relokation im Kernel, harter Schnitt.

- **Format:** ELF64 `ET_DYN`, `EM_AARCH64`; `PT_DYNAMIC` mit `DT_RELA`/`DT_RELASZ`/`DT_RELAENT`; erlaubt sind nur `R_AARCH64_RELATIVE` (Wert = Ladebasis + Addend, 64 Bit). `DT_TEXTREL` und jede andere Relokationsart → `EPLFMT`. Keine dynamischen Symbole, kein Interpreter (`PT_INTERP` → `EPLFMT`). Reloziert wird nach dem Streamen der Segmente in den TPA-Block und **vor** `sync_code_cache` (`NT/loader.rs:540-544`), damit I-Cache-Pflege das Endbild sieht. **0a (V0a §8):** `R_AARCH64_RELATIVE` = **1027**, Operation `Delta(S)+A`, Platz = 8-Byte-ausgerichteter 64-Bit-Wert, Ergebnis = Addend + (Lade- − Linkadresse) (AAELF64 5.7.15, S. 40; Prosa S. 41) — **belegt**. Als Ladeprüfung zusätzlich: `e_machine = EM_AARCH64 (183)`, `e_flags = 0`, `ELFCLASS64`, Little-Endian (5.2, S. 13); `R_AARCH64_COPY` nur in `ET_EXEC` (S. 41) ⇒ in `ET_DYN` unzulässig. **Weiterhin offen — nicht in AAELF64:** Aufbau `Elf64_Rela`/`ELF64_R_TYPE`, `PT_DYNAMIC`, `DT_RELA/RELASZ/RELAENT`, `DT_TEXTREL`, `ET_DYN`-Anforderungen — AAELF64 6.3–6.5 (S. 43/44) verweist auf SYSVABI64/gABI (nicht gepinnt, V0a §11 Nr. 5); `DT_RELACOUNT` ist in keiner gepinnten Quelle belegt, der Lader darf es nicht voraussetzen. Ob ein `-pie`-Image außer `RELATIVE` auch `ABS64`/`IRELATIVE` enthält, ist Linkerverhalten → Toolchain-Spike 0d.
- **Warum im Kernel und nicht im `crt0`:** EmuTOS und FreeMiNT relozieren in `Pexec` (S1 A.7 letzter Punkt; `ET/bdos/kpgmld.c`); `Pexec(3)` muss ein fertiges, aufrufbares Bild liefern (LDG ruft danach nur `Pexec(4)` — der Modulcode würde sich sonst erst beim Start selbst relozieren, was hier zufällig auch ginge, für `p_tbase`-lesende Shells aber nicht).
- **Layout im TPA-Block** (Basis `B` = seitenausgerichteter Pool-Block): `B+0x000` Basepage, `B+0x100..B+0x1000` frei (Platz für spätere `user_things`), `B+0x1000` Programmbild, darüber BSS, Heap, Stapel bis `p_hitpa`. **Das Linkskript bleibt bei Basis `0x1000`** (`sdk/newtos.ld`), nur als PIE: `-pie --no-dynamic-linker -z text`; `.dynamic`, `.rela.dyn`, `.got` werden **nicht** mehr verworfen (heute `/DISCARD/ … *(.got .got.plt)`, `sdk/newtos.ld` Ende [gelesen]). `p_tbase = B + 0x1000` statt EmuTOS `B + 0x100` — für kein bekanntes Programm beobachtbar **[Schluss]**.
- **`crt0`:** liest die Basepage aus `x0` statt aus VA 0 (`sdk/libc/crt0.S`: `mov x19, #0` → `mov x19, x0` [gelesen]; ebenso `sdk/crt0.S`). Der Kernel übergibt `x0 = basepage_va` bereits heute (`KA/exceptions.rs:1072,1925`).
- **Vorhandene Binärdateien:** alle stammen aus diesem Repo (`sdk/apps/{qed,teradesk,doom,coremark,apitest,…}`, `sdk/examples/*.c`). **Harter Schnitt:** in Stufe 0d werden alle als PIE neu gebaut; diese PIE-Bauten laufen auf dem Stufe-0-Kern (Ladebasis 0) **und** auf dem Stufe-1-Kern (Ladebasis = Block), damit jede Stufe rückrollbar ist. Ab Stufe 1 lehnt der Lader `ET_EXEC` mit `EPLFMT` ab (Entscheidung E7).
- **Selbsttests:** neun Dateien bauen ELF-Blobs von Hand (`kernel/src/selftest_{d3,d6,d7,pexec,pexec_d4d5,sysvars,upcall,etv_term,sleepwake_switch}.rs`, `grep registry::register|e_entry` [gelesen]). Umstellen auf `ET_DYN` ohne Relokationen; jeder Blob ist auf Absolutadressen zu prüfen (insbesondere Lesen der Basepage bei VA 0). Merkregel „unverdrahtete Selbsttests verrotten still“ beachten.

### Frage 4 — TPA-Größe, `p_hitpa`, `Malloc(-1)`, Fenster-/Poolgröße

**Antwort:**
- **TPA:** bleibt zunächst ein Block fester Standardgröße (4 MiB, `NT/arena.rs:52`) — aber jetzt **bei VA = PA**, und `p_lowtpa`/`p_hitpa`/Segmentfelder sind echte Adressen. EmuTOS gibt den **größten freien Block** (`ET/bdos/proc.c:460-502` [S1]); in einem Mehrprozesssystem verhungern damit alle anderen — FreeMiNT begrenzt deshalb mit `initialmem` (Standard 1 MiB) und `F_SMALLTPA` (`FM/memory.c:1494-1500` [S1]). Unsere libc legt ihren Heap in das freie TPA (`sdk/libc/newtos-arch/heap_init.c:1-50` [S1]) und ruft kein `Mshrink`; „größter Block“ würde den Pool für Accessories und Kinder leeren. Empfehlung E9: feste Standardgröße, übersteuerbar über PRGFLAGS-Notiz (Frage 8) und `Pexec`-Modus 7.
- **`Mshrink(basepage, n)`** wird echt: verkleinert den TPA-Poolblock (heute über Block 0 der Fenstertabelle, `NT/usermem.rs:266-283`).
- **`Malloc(-1)`/`Mxalloc(-1)`:** ab Stufe 1 wieder EmuTOS: größter freier Block des gewählten Pools (`ET/bdos/umem.c`). Die Fenstergrenze (`tos-api/src/gemdos/memory.rs:63-71`) entfällt. Ab Stufe 3 auf Seitengröße abgerundet (Quantum).
- **Fenstergröße:** entfällt (es gibt kein Fenster). Begrenzt ist nur noch der Pool.

### Frage 5 — Was wird aus `usermem.rs`?

**Antwort:** Das Modul wird in Stufe 1 auf eine **Regionstabelle** zurückgebaut; das Fenster verschwindet.

| Heute | Ab Stufe 1 | Ab Stufe 3 |
|---|---|---|
| `map` (Fenster, Bump-Cursor, `NT/usermem.rs:308-363`) | entfällt: Pool ist im gemeinsamen Raum bei VA = PA abgebildet; `Malloc` liefert `m_start` | `attach`: Rechte des Besitzers in seinem `memspace` setzen |
| `seed_tpa` (Block 0 = TPA) | entfällt (TPA ist ein gewöhnlicher Poolblock bei seiner Adresse) | — |
| `translate`/`free`/`free_by_mstart` | entfällt: `Mfree`/`Mshrink` gehen direkt an die EmuTOS-Traps mit der Anwenderadresse | Regionen-Detach + Rechte entziehen + TLBI |
| `expose`/`expose_attr` (ADR-023) | bildet die Kernelseite **bei VA = PA** in die gemeinsame Tabelle ab, `PROT_PR`/`PROT_G` wie heute; liefert die PA als Zeiger | Rechte je Prozess aus dem Modus |
| `Kind::Shared`, `MemAttr::Uncached`, `UNCACHED_SLOTS` (`:95-218`) | bleiben (Bildschirmspeicher Normal-NC; Mismatched-Attribute-Regel unverändert, ADR-023 Nachtrag) | unverändert |
| `resolve_kva` (`:583-594`) | `va + VA_OFFSET` nach Bereichsprüfung in der Regionstabelle | unverändert |
| `window_remaining` | entfällt | — |

- **Doku-Folgen** (Merkregel „eine Begründung steht selten nur einmal“): Die Fenster-Begründung steht in `docs/COMPATIBILITY.md:216` („per-process window VA“), `:327` (Ressource über Kernel-Alias), ADR-021, ADR-023 (+Nachträge), `docs/ARCHITECTURE.md:97-112`, `NT/usermem.rs`-Kopf, `NT/mmu.rs`-Kopf, `sdk/newtos.ld`, `sdk/libc/crt0.S`, `sdk/crt0.S` und in `docs/api/{bios/07-getbpb.md,bios/00-getmpb.md,bios/_cookie-jar.md,xbios/02-physbase.md,xbios/03-logbase.md,xbios/10-keytbl.md,xbios/22-kbdvbase.md,gemdos/memory/48-malloc.md,gemdos/process/4b-pexec.md,structures/basepage.md,aes/window-graphics.md,aes/resource-fileselector.md}` (`grep "per-process window|heap window|USER_VA_BASE|VA 0"` [gelesen]). Alle Stellen gehen in Stufe 1 mit.

### Frage 6 — Besitzmodell

**Antwort:** FreeMiNT-`MEMREGION`-artig mit mehreren Besitzern; EmuTOS' `m_own` bleibt das Freigabe-Etikett des Pools.

- **Regionstabelle** (kernelintern, nicht ABI — `docs/COMPATIBILITY.md` „Free implementation“): je Poolblock ein Eintrag `{ loc (= m_start = VA = PA), len, owners: Bitmenge über memspaces, links, prot: ProtMode, flags: KEEP | SHARED | TSR | KERNEL, attr: MemAttr }`. Entspricht `MEMREGION` (`FM/mint/mem.h:16-27` [S1]) plus `memspace.mem[]` als Bitmenge (`MAX_PROCESSES = 16 ≤ 32`, `tos-api/src/bdos/process.rs:63` [gelesen]).
- **`attach`/`detach`** wie FreeMiNT (`FM/memory.c:512-660` [S1]): `links` zählt Besitzer; Freigabe an den Pool erst bei `links == 0`. `M_KEEP` → bei Prozessende an den Systembesitzer statt frei (`FM/proc_help.c:243-258` [S1]).
- **`m_own`** bleibt EmuTOS-Semantik („wessen Blöcke bei `Pterm` ernten“, `ET/bdos/proc.c:92-101,110-141` [S1]) und wird **der echte PD-Zeiger** (VA der Basepage) statt der Kernel-VA (`NT/loader.rs:425` heute). `free_all_owned` wird zu „detach aller Regionen dieses Besitzers“; frei wird, was danach niemandem mehr gehört.
- **`Pexec(3)`:** Basepage, Umgebung und TPA gehören dem **Aufrufer** (EmuTOS `owner = run`, `ET/bdos/proc.c:302-307` [S1]; FreeMiNT `create_env`/`create_base`/`load_region` an `curproc`, S1 A.7). Das Kind darf (Stufe 3) mit Besitzerrechten darauf zugreifen, weil es beim Start Mitbesitzer wird (FreeMiNT `attach_region(p, env); attach_region(p, base)`, `FM/k_exec.c:518-520` [S1]).

### Frage 7 — Standard Kind → Eltern; `Pexec 4/104` als Thread?

**Antwort:**
- **Standard nicht strikt** (FreeMiNT-Standard `MPF_STRICT = 0`, `FM/k_exec.c:799-806` [gelesen], `doc/examples/mint.cnf:132-137` [S1]): ein per `Pexec` 0/6/100/106 gestartetes Kind behält Zugriff auf die Regionen des Elternprozesses, die beim Start bestanden. Konfigurationsschalter „strikt“ (Boot-Option) für später. Empfehlung E5.
- **`Pexec 4/104` = Thread im `memspace` des Aufrufers** (FreeMiNT `FORK_SHAREVM`, `FM/k_exec.c:224-231,421` [S1]): das Kind erhält dieselbe `TTBR0`-Wurzel wie der Aufrufer. **Besitz** für Allokationen, die das Kind macht, folgt dagegen EmuTOS: Besitzer ist das Kind (`run` = Kind, `ET/bdos/proc.c:244-248,536-578` [S1]), so dass `ldg_init`-Allokationen beim `Pterm` des Moduls frei werden, wie der LDG-Code es erwartet (`zview-ldg-2026-10-04.md` §3.6). Empfehlung E6.
- Damit läuft LDG unverändert: `Pexec(5)` → `Pexec(3)` (Bild in Aufrufer-Speicher) → `Pexec(4)` (Thread schreibt über `OFFSETLDG` in den Aufruferstapel) → `Pwait` (`ldgload.c:73-158` [S1]).

### Frage 8 — `PROT_*` auf `Mxalloc`, PRGFLAGS im ELF

**Antwort:**
- **Kodierung exakt nach FreeMiNT, mit dem „+1“:** im `Mxalloc`-Modus bedeutet das Nibble `0x10` = `PROT_P`, `0x20` = `PROT_G`, `0x30` = `PROT_S`, `0x40` = `PROT_PR`, `0x50` = `PROT_I`; `0x00` = „aus den PRGFLAGS des Aufrufers“; `> PROT_MAX_MODE` → `PROT_P` (`FM/dosmem.c:209-221` [gelesen]; Kommentar `FM/mint/mem.h:118`: „when users call Mxalloc, they add 0x10 to what you see here“ [gelesen]). Im PRGFLAGS-Wort dagegen **ohne** +1 (`F_PROT_G = 0x10` = `PROT_G`). **Achtung:** Der Kommentar in `tos-types/src/memory.rs:155-168` stellt beide Kodierungen als „bezogen durch `(mode & F_PROTMODE) >> F_PROTSHIFT`“ dar — für `Mxalloc` stimmt das nicht. Vor der Umsetzung korrigieren; Hosttest für beide Wege. Bit `0x08` wird ignoriert (in FreeMiNT auskommentiert, `FM/dosmem.c:226-236` [gelesen]); `F_KEEP` (`0x4000`, `FM/mint/mem.h:105` [gelesen]) wird ausgewertet.
- **Wann:** Stufe 2 dekodiert und speichert den Modus in der Region (keine Wirkung, wie FreeMiNT mit `no_mem_prot`); Stufe 3 macht ihn wirksam. Damit endet ADR-21s „`mode &= MX_MODEMASK`“ (`tos-types/src/memory.rs:142`).
- **PRGFLAGS im ELF:** NewTOS-ELF hat keinen PRG-Kopf. Vorschlag: ein `PT_NOTE`-Segment mit Notiz-Name `"NewTOS"`, Typ 1, Inhalt `u32 prgflags` (Linker-Abschnitt `.note.newtos.prgflags`, vom SDK als Standard `PF_STANDARD` erzeugt; Werkzeug zum Ändern wie `prgflags.ttp`). Fehlt die Notiz → `PF_STANDARD` wie heute (`NT/loader.rs:674`). `Pexec(7)` liefert die Flags direkt. Der Lader trägt sie in `p_flags` ein und in die Speicherflags des `memspace` (Standardschutz). Entscheidung E8b (Notizformat ist **unsere** Erfindung, keine Extraktion — deshalb Anwenderfreigabe).

### Frage 9 — Wie greift XaAES auf Client-Speicher zu? (gezielt gelesen)

**Befund (alles [gelesen]):**
1. **AES-Aufrufe laufen im Kontext des aufrufenden Prozesses** — `XA_handler` ruft die Funktion direkt im Trap des Aufrufers (`XA/handler.c:479-739`, Zusammenfassung in `docs/analysis/coop-kernel/freemint-multitasking-model.md:449-463`). Die Seitentabelle ist die des Aufrufers; seine Zeiger (Parameterblock, Bäume, Puffer) sind direkt gültig.
2. **Daten, die ein Client sehen muss, legt XaAES in den Speicher des Clients:** `umalloc` hängt eine `PROT_P`-Region an `curproc` (`FM/umemory.c:243-290`: `get_region(alt/core, size, PROT_P)` + `attach_region(p, m)`); benutzt z. B. für den per-Client-Block `client->ut` mit Userdef-Parameterblock und Rücksprungadresse (`XA/xa_appl.c:185`), Ressourcendaten (`XA/xa_rsrc.c:153,218,985`), Callout-Blöcke (`XA/xa_wdlg.c:78`, `XA/xa_lbox.c:70`).
3. **Nachrichten werden nie in fremden Speicher geschrieben.** „If the caller of send_a_message is NOT the receiver of the message, ALWAYS queue it on behalf of the receiver. Then Unblock() receiver. The receiver, when it is woken up, will then call check_queued_events()“ (`XA/messages.c:649-660`); die Warteschlange liegt im Kernelspeicher (`queue_message`, `:591-640`), kopiert wird im Empfängerkontext (`XA/xa_evnt.c:321`, `check_queued_events`).
4. **Arbeit, die fremden Client-Speicher braucht, wird als „Client-Event“ zum Eigentümer verschoben** (`post_cevent`, `XA/k_main.c:213-262`; ausgeführt in `cBlock`/`dispatch_cevent`, `:305-330,369-380`, also wenn der Client selbst im AES blockiert).
5. **Userdefs (`G_PROGDEF`) laufen nur im Eigentümerkontext:** `client = lookup_extension(NULL, XAAES_MAGIC)` = Client des **laufenden** Prozesses (`XA/draw_obj.c:371`), Aufruf direkt per `jsr` im Supervisor-Modus (`do_callout`, `:334-351`, `:458`), Parameterblock in `client->ut` (`:418-426`). `PROGDEF_BY_SIGNAL` ist 0 (`:46`).
6. XaAES setzt **kein** `F_OS_SPECIAL` (`grep F_OS_SPECIAL XA/*.c` ohne Treffer [gelesen]); `F_OS_SPECIAL` ist für ein per `GEM=` geladenes externes AES gedacht (`FM/init.c:220,1237,1260` [S1]).

**Abbildung auf NewTOS:**
- Die NewTOS-AES läuft in EL1. **Lesen/Schreiben** von Client-Speicher (Parameterblock, Bäume, Menüs, `WF_NEWDESK`-Baum, `menu_register`-Titel, Nachrichtenpuffer) geht über den Kernel-Alias `va + VA_OFFSET` nach Regionsprüfung — unabhängig davon, welcher Prozess gerade `TTBR0` hält. Damit ist die „echte Lücke“ von `30-aes-thread-design.md:693-705` für **Lesezugriffe** geschlossen (Bäume eines dritten Prozesses zeichnen). Das ist strenger kontrolliert als FreeMiNT (dort entscheidet die Tabelle des Aufrufers), aber gleiches Ergebnis für korrekte Programme.
- **Ausführen** von Client-Code (Userdef, `kbdvec`, `etv_term`) nur mit den Rechten des Eigentümers. Stufe 1/2: ohne Schutz trivial erfüllt (jeder Prozess sieht alles; der Upcall darf auf dem Stapel des laufenden Prozesses laufen, wie heute). Ab Stufe 3 zwei Wege — Entscheidung E10:
  - **(a) zeitweise Freigabe** (FreeMiNTs eigenes Mittel: `prot_temp`, `FM/arch/mprot030.c:775-805` [gelesen]; SLB-Muster `mark_proc_region(…, PROT_G, …)`, `FM/slb.c:581-591` [S1]): Während des Upcalls erhält der laufende `memspace` Besitzerrechte auf die Regionen des Baumeigentümers; nach `Psigreturn` zurück + TLBI. Erhält EmuTOS' sofortiges Zeichnen. **Empfohlen.**
  - **(b) Verschieben zum Eigentümer** (XaAES `post_cevent`): Redraw-Auftrag in die Warteschlange des Eigentümers. Zeitverhalten wie XaAES, nicht wie EmuTOS.
- Nachrichten bleiben wie heute im Kernel (FIFO je AES-Identität, geweckt über `crate::aes::pd::post_msg`, `NT/sched.rs:45-51` [gelesen]) — passt bereits zu XaAES.
- Zeiger **in** Nachrichten (AV-Protokoll `VA_START` mit Stringzeiger) sind ab Stufe 3 nur lesbar, wenn der Sender globalen Speicher benutzt — so verlangt es FreeMiNT, und LDG tut es (`ldg_Malloc` = `Mxalloc(size, 3+8+32)` → Schutznibble `0x20` → `PROT_G`, `ldgutil.c:120-158` [S1] mit der Kodierung aus Frage 8).

### Frage 10 — Kernelzugriff auf Anwenderzeiger

**Antwort:** Zwei Primitive, klar getrennt (Abschnitt 3.6):
1. **`uaccess` — mit den Rechten des Aufrufers:** für Syscall-Argumente (Pfade, Puffer von `Fread`/`Fwrite`, `Cconws`, DTA). Unprivilegierte Lade-/Speicherbefehle `ldtr*`/`sttr*` über das aktive `TTBR0` des Aufrufers (heute schon an mehreren Stellen: `KA/exceptions.rs:1597-1650`, `tos-api/src/gemdos/console.rs:292`, `bios/vectors.rs:133`, `bios/chardev.rs:156` [gelesen]). Neu: ein **Fehlerabfang** wie der vorhandene MMIO-Probe (`PROBE_ACTIVE`/`PROBE_FAULTED`, `KA/exceptions.rs:171-220` [gelesen]) — ein EL1-Abbruch auf eine Anwenderadresse wird zu `EFAULT`-artigem Fehler bzw. Programmabbruch statt Maschinenhalt (heute: `unhandled` → `halt`, `KA/exceptions.rs:1946-1998` [gelesen]). **0a (V0a §4):** Der Cortex-A72 hat **kein PAN** (FEAT_PAN: optional in 8.0, verpflichtend ab 8.1, ARM ARM A2.2 S. A2-101; A72 = ARMv8.0, TRM S. 1-15; `ID_AA64MMFR1_EL1` laut TRM `RES0`, S. 4-364 — zur Laufzeit gegenlesen, Feld `PAN` [23:20], D24-9253) und **kein UAO** (optional ab 8.1, verpflichtend ab 8.2, S. A2-112; `ID_AA64MMFR2_EL1` im TRM nicht aufgeführt). `LDTR/STTR` verhalten sich an EL1 (UAO=0) „as if executed at EL0“ (C6.2.245, S. C6-2374): geprüft werden die **EL0-Rechte** (`AP[1]=1`, Table D8-63); PAN betrifft nur *privilegierte* Zugriffe (`RXTSQH`, D8-7775/7776), `ldtr/sttr` gehen auch mit PAN durch. Kernelzeiger (`AP=00`) und Adressen außerhalb des TTBR0-Bereichs fallen von selbst durch (Permission- bzw. Level-0-Translation-Fault). Da PAN fehlt, erzwingt **nur Disziplin** (jeder Anwenderzugriff über `ldtr/sttr` oder `kalias`), dass der Kernel nie direkt dereferenziert. Befehlsform: unskalierter `imm9` (−256…255), kein Writeback/Registeroffset (C6-2374). Fehlerbild eines `ldtr/sttr`-Aborts: `EC 0x25`, `FAR_EL1` = VA, `DFSC`/`WnR` in `ESR_EL1`; von einem Kernelfehler nur über den PC (Fixup-Tabelle) unterscheidbar. **0b:** auf QEMU genau so gemessen — `sttr` auf EL0-RO → DFSC `0x0F`, `WnR` 1; `ldtr` auf `AP=00` → `0x0F`; ungültig auf L3/L2/L1/L0 → `0x07/0x06/0x05/0x04`; `ldtr` auf eine Kernel-VA (TTBR1-Block) → `0x0E`; `ELR` = der `ldtr`/`sttr` selbst; `MMFR1.PAN = 0`, privilegiertes `ldr` auf Anwenderseiten gelingt (S0b §5).
2. **`kalias` — kernel-initiiert, über fremde Prozesse hinweg:** `va + VA_OFFSET` nach Prüfung „liegt vollständig in einer lebenden Region (oder Seite-0-Sonderfall)“. Das ist das Gegenstück zu FreeMiNTs `prot_temp(loc,len,-1)` (`FM/arch/mprot030.c:775-805` [gelesen]), aber ohne Seitentabellen-Umbau, weil der High-Half-Alias das ganze DRAM bereits enthält (`KA/mmu.rs:1-60` [S1]). Benutzer: AES, Lader (Kind-Basepage schreiben), `Pexec`-Umgebung, Cookie-Spiegel.
- **Seite 0 ist der einzige Ort, wo VA ≠ PA:** `kalias(slot, va < 0x1000)` muss den Slot kennen (Basis = PA der prozesseigenen Seite 0).

### Frage 11 — Fehlerbehandlung bei Schutzverletzung

**Antwort:**
- **Heute:** jeder synchrone Nicht-SVC-Abbruch aus EL0 hält die Maschine an (`rust_sync_handler` → `unhandled` → `halt`, `KA/exceptions.rs:210-223,1946-1998` [gelesen]). Mit Schutz würde aus jeder Verletzung ein Systemstillstand — das muss **vor** Stufe 3 weg (Stufe 0c).
- **Ziel:** Instruction/Data Abort aus niedrigerer EL (`EC_IABT_LOWER`, `EC_DABT_LOWER`, `KA/exceptions.rs:87-93` [gelesen]) → Klassifizieren → Meldung → `Pterm(-1)`:
  - **Klassifikation** wie FreeMiNTs `report_buserr` (`FM/arch/mprot.x:71-150` [S1]): aus der Regionstabelle für `FAR_EL1`: „free“ (keine Region), „private/global/super/readable“ (Modus der Region), sonst „hardware“. Zusätzlich die Fehlerart aus `ESR_EL1.ISS` (Übersetzungs- vs. Berechtigungsfehler, Ebene, Schreib-/Lesezugriff `WnR`). **0a (V0a §6):** belegt — `DFSC/IFSC`: Translation `0b0001LL`, Access-flag `0b0010LL`, Permission `0b0011LL` (LL = Ebene 0–3; die Level-0-Variante von Access-flag/Permission nur mit LPA2), Address-size `0b0000LL`, sync. External Abort `0b010000`, TLB-Konflikt `0b110000` (ARM ARM D24-8672/8673 IFSC, D24-8683…8685 DFSC); QUIRK-003 `0x0e` = Permission Level 2 ✓. `ESR_EL1.WnR` (Bit 6, nur Data Abort): `1` = Schreibzugriff (D24-8682); `FAR_EL1` ist bei allen MMU-Aborts gültig, `FnV` nur bei sync. External Abort (D24-8681, D1-7219); Ebene bei Permission-Fault = Ebene des Block-/Seitendeskriptors (`RCSRQR` D8-7841). EC `0x20/0x21/0x24/0x25` (D24-8643). **Wichtig:** Nicht-Besitzer-Einträge von `PROT_P/S/I` sind *ungültig* (Abschnitt 3.3) und melden daher **Translation-Faults**, nicht Permission-Faults — „geschützt“ vs. „nicht abgebildet“ ist nur über die Regionstabelle (FAR) unterscheidbar; echte Permission-Faults gibt es nur bei `PROT_PR` (Schreiben). `DFSC = 0b110000` (TLB-Konflikt) ist ein Kernelfehler (BBM verletzt, `RWJMZJ` D8-7842).
  - **Meldung:** serielle Zeile (wie heute) **und** — wenn die AES läuft — `form_alert` nach dem Muster „PROCESS … KILLED: MEMORY VIOLATION“ (`FM/arch/mprot.x:89-150` [S1]); EmuTOS zeigt Register und „Crash at text+…“ (`ET/bios/kprint.c:560-585` [gelesen]). Text: Entscheidung E11.
  - **Ende:** `Pterm(-1)` über den vorhandenen Terminierungsweg (`terminate_and_redirect`, `KA/exceptions.rs:1229`), wie EmuTOS `kill_program` (`ET/bios/panicasm.S:96-99` [gelesen]: `Pterm(-1)`).
  - Später (FreeMiNT-Erweiterung): ist ein `SIGBUS`/`SIGSEGV`-Handler installiert, wird er per Upcall gerufen (`FM/arch/sig_mach.c:493-502` [S1]); die Upcall-Infrastruktur (`deliver_upcall`, `KA/exceptions.rs:1395-1473`) existiert.
- **Kernel-Abbrüche auf Anwenderadressen** (EL1, `EC_DABT_SAME`) während `uaccess` → Fehlerabfang (Frage 10); außerhalb von `uaccess` bleiben sie Kernelfehler (Halt mit Bericht).

### Frage 12 — Cookie-Jar, Systemvariablen, `kbdvec`, OSHEADER

**Antwort:**
- **Seite 0 bleibt je Prozess** (E3), mit allen festen Adressen (`$5A0` → Jar `0x5F0`, OSHEADER `0x5C0`, Trampolin `0x700`, USERDEF `0x714`, PARMBLK `0x730`, Rücksprung `0x760`, Keytbl `0x768`, Kbdvecs `0x9A0`, Stubs `0xA00`, `NT/loader.rs:710-830` [gelesen]). `_p_cookies` = `0x5A0` ist ABI und bleibt.
- **Geändert werden muss `os_run`:** heute zeigt es auf VA 0, weil dort die Basepage liegt und `p_lowtpa == 0` (`NT/loader.rs:935-940` [gelesen]). Neu: eine Zelle in Seite 0 (z. B. im freien Bereich unterhalb `0x5C0`), die die Basepage-VA des Prozesses hält; `os_run` zeigt darauf. Für den Prozess selbst ist „aktueller PD“ immer er selbst — er kann die Zelle nur lesen, während er läuft **[Schluss]**.
- **Cookie-Werte, die auf Daten zeigen** (z. B. LDG `LDGM` → `LDG_INFOS` im TSR, S1 / `zview-ldg` §3.5), sind im gemeinsamen Raum gültig; Stufe 3: Ziel muss `PROT_G`/TSR sein (FreeMiNT macht TSR-Speicher automatisch global, Frage 16).
- **`kbdvec`:** siehe Frage-12-Zusatz und E4 — zunächst unverändert prozesseigen (KEYS02 D1, `docs/COMPATIBILITY.md:640-662` [gelesen]); Wiederaufnahme nach Stufe 3 (Abschnitt 3.14).

### Frage 13 — `p_parent`, `Mshrink`, PD-Zeiger, `RUN`, `PROCESS_TABLE`

**Antwort:**
- `p_parent` = Basepage-VA des Elternprozesses (EmuTOS `p->p_parent = run`, `ET/bdos/proc.c:541` [S1]; FreeMiNT `FM/k_exec.c:420` [S1]); heute `0` (`NT/loader.rs:677` [gelesen]). Kernel-gestartete Programme: `0` wie bisher.
- `Pexec(3)`/`(5)`/`(7)` liefern den **echten PD-Zeiger**; `Pexec(4)`/`(6)` nehmen ihn (Handle-Auflösung über `basepage_pa`, `NT/pexec.rs:297-311`, wird zu „Basepage-VA → Slot“).
- `RUN` hält künftig die **Anwender-VA** der Basepage (heute die Kernel-VA, `tos-api/src/bdos/process.rs:771-777` [gelesen]); Kernelzugriff über `kalias`. `m_own` = derselbe Wert (Frage 6).
- `PROCESS_TABLE` bleibt Kernel-Buchhaltung (Slot ↔ PD), ergänzt um `memspace`-Index; `l0_pa` wandert in den `memspace`.

### Frage 14 — Slot-Zahl und Seitentabellenbudget

**Antwort:**
- `MAX_PROCESSES = 16` bleibt vorerst (Erhöhung ist unabhängig).
- **Stufe 1:** je `memspace` 4 Tabellenseiten (L0, L1, das L2 für das erste GiB, das L3 für `[0, 2 MiB)`) = 16 KiB; geteilt: die L2-Tabellen der übrigen GiB-Bereiche und die L3-Tabellen des Pools (Pi: 952 MiB → ≤ 477 L3 ≈ 1,9 MiB, einmalig; mit 2-MiB-Blöcken an ausgerichteten Stellen nur die Ränder als L3 — Spike-Frage). **0b:** gemessen auf `raspi4b` (Pi-Layout): Pools mit 2-MiB-Blöcken **3** Tabellen insgesamt, nur 4-KiB-Seiten **479** Tabellen (≈ 1,9 MiB); `virt` 4 bzw. 132 (S0b §3). Heute: `16 × (3 + 16) × 4 KiB = 1216 KiB` statisch (`NT/mmu.rs:66-70` [gelesen]). Netto etwa gleich.
- **Stufe 3:** je `memspace` eigene L3 nur für 2-MiB-Bereiche, in denen seine Rechte vom gemeinsamen Muster abweichen; Bereiche mit „alle gleich“ (z. B. `PROT_G`-Bildschirm, global) zeigen auf geteilte L3. Grobe Rechnung je Prozess: TPA 2–3 L3, Heap-Blöcke ~5, Bildschirm 0 (geteilt) → ≈ 10 L3 = 40 KiB; 16 Prozesse ≈ 0,7 MiB **[Schätzung]**. Herkunft: ein eigener **Seitentabellen-Vorrat**, vor `bring_up_bdos_pools` reserviert (`kernel/src/lib.rs:742-775` [gelesen] verteilt sonst alle Rahmen an BDOS), Obergrenze z. B. 4 MiB; Erschöpfung → `ENSMEM` der auslösenden Allokation.
- FreeMiNT zahlt ~1 KiB je MiB RAM je Prozess (`FM/arch/mprot030.c:229-231` [S1]); wir sind durch Teilen darunter.

### Frage 15 — ASIDs auf QEMU nutzbar?

**Antwort:** Das Modell **braucht keine ASIDs**. Grundpfad auf beiden Plattformen bleibt: Wurzelwechsel mit vorausgehendem TLBI im Kernelkontext + QUIRK-001-Klammer (heute produktiv). ASIDs sind eine **optionale Beschleunigung (Stufe 4)**, hinter einem Fähigkeitsflag, das beim Booten gesetzt wird — Standard aus, an auf der Pi nach Nachweis. Damit gibt es zwei Pfade, von denen einer der heutige, erprobte ist.
- Belege: ASID-getaggtes `TTBR0` hängt QEMU 11.0.0 TCG (`docs/MEMORY.md:2125-2142`, `NT/mmu.rs:366-374` [gelesen]). Installiert ist jetzt 11.0.2 → im Spike 0b neu messen.
- **0a (V0a §2/§3): belegt.** ASID = `TTBR0_EL1[63:48]` bei `TCR_EL1.A1 = 0` (ARM ARM D24-9921, D24-10042); 8 Bit bei `TCR_EL1.AS = 0`, 16 Bit bei `AS = 1` (Bit 36, D24-9917; `IBZXYN` D8-7855). **Cortex-A72: `ID_AA64MMFR0_EL1.ASIDBits = 0x2` = 16 Bit** (TRM Table 4-35, S. 4-116); `AS = 0` genügt für 16 Prozesse. `nG` (Bit 11) auf allen Anwendereinträgen (`RQGKGF` D8-7854); Einträge nicht-finaler Ebenen gelten ohnehin immer als nicht-global (`RCHVGJ`). Wechsel bei `A1 = 0` mit einem `msr ttbr0_el1` + `isb`, weil die Kernel-Einträge global sind (Example D8-1, D8-7700). TLBI (alle im A72 vorhanden, TRM Table 4-6 S. 4-80): `ASIDE1(IS)` löscht nur **nicht-globale** Einträge der ASID (`RFKSVM` D8-7868), `VAE1(IS)` VA+ASID (+ globale der letzten Ebene, `RVMSWQ`), `VAAE1(IS)` VA für alle ASIDs, `VMALLE1(IS)` alles; Operand `Xt`: ASID [63:48] (obere 8 Bit 0 bei 8-Bit-ASID), `VA[55:12]` in [43:0] (`IZVNJG`, `ILHKCP` D8-7863/7864); `…L`-Varianten betreffen nur die letzte Ebene. Beim ASID-Überlauf sicherheitshalber `VMALLE1(IS)` (der Wortlaut von `RFKSVM` „final lookup level“ ist neben `RCHVGJ` nicht eindeutig). **Offen → 0b:** was QEMU `raspi4b`/`virt` als `ASIDBits` melden und ob QEMU 11.0.2 den ASID-getaggten `msr` noch hängt. **0b:** beide melden `ASIDBits = 2` (16 Bit, `ID_AA64MMFR0_EL1 = 0x1124` wie TRM); der getaggte `msr ttbr0_el1` hängt nicht; zwei Wurzeln mit ASID 1/2, `nG = 1`, `TCR.AS = 0` wechseln nur mit `msr; isb; eret` 32/32-mal richtig, ohne TLBI (S0b §4). Ohne TLBI **und** ohne ASID liefert QEMU dagegen 16/32-mal die Seite 0 der vorigen Wurzel — TLBI oder ASID ist Pflicht. Damit ist Stufe 4 auch auf QEMU entwickelbar; das Fähigkeitsflag bleibt, bis die Pi bestätigt.

### Frage 16 — `Ptermres`/Residenz

**Antwort:** Ja, die Seiten bleiben erreichbar.
- EmuTOS: TPA auf `blkln` verkleinern (`xsetblk(0, run, blkln)`), eigene Blöcke aus `mp_mal` austragen, `xterm` (`ET/bdos/proc.c:624-634` [S1]). NewTOS ignoriert `blkln` heute (`NT/pterm.rs:116-135` [gelesen]) — Stufe 2 setzt die Verkleinerung um.
- FreeMiNT: Basepage-Region auf `save` verkleinern; jede Region mit `links == 1` und `PROT_P` → `PROT_G` (`FM/k_exit.c:320-336` [gelesen]); Prozess in `TSR_Q`, Speicher bleibt.
- **NewTOS:** Slot wird frei (ein PD ohne laufenden Code braucht keinen Slot — EmuTOS kennt danach auch keinen Prozess mehr); die Regionen wechseln zum **Systembesitzer** mit Flag `TSR` und Modus `PROT_G` (ab Stufe 3 wirksam). Stufe 1: ohnehin sichtbar, weil gemeinsam abgebildet.

### Frage 17 — Signalpfad/Upcall, Trampolin

**Antwort:** Trampolin, USERDEF-Stub, PARMBLK-Scratch und Rücksprungzelle bleiben **in der prozesseigenen Seite 0** bei den festen VAs (`NT/loader.rs:740-783` [gelesen]). Das ist funktional FreeMiNTs `tp_reg`/`user_things` je Prozess (`FM/proc_help.c:147-205` [S1]), nur an fester statt variabler Adresse. `deliver_upcall` bleibt reine Rahmenmanipulation (`KA/exceptions.rs:1395-1473` [gelesen]). Die `TTBR0`-Lücke betrifft nur noch das **Ausführen** fremden Codes ab Stufe 3 (Frage 9, E10).

### Frage 18 — m68k-Thunk

**Antwort:** Das Modell ist kompatibel und bereitet ihn sogar vor: alle gemeinsamen Adressen < 2 GiB (32-Bit-tauglich), ST-RAM-Pool bei `0x0040_0000..0x00E0_0000` (24-Bit-sauber). Ein m68k-Prozess bräuchte zusätzlich eine eigene Sicht auf `0..0x800` (Vektoren, Systemvariablen) — das ist genau die prozesseigene Seite 0. Big-Endian-Daten und die Übersetzung bleiben Sache des Emulators (`docs/ARCHITECTURE.md:72-86`). Nicht Teil von ARCH03; keine Entscheidung nötig.

### Frage 19 — `Pfork`

**Antwort:** **Nicht umsetzen.** `Pfork` ist keine Atari-Pflicht (EmuTOS hat es nicht, S1 B); FreeMiNTs Inhaltstausch bei jedem Kontextwechsel (`swap_in_curproc`, `FM/proc.c:376-410` [S1]) ist ein Notbehelf für MMU-lose 68000 (`docs/analysis/coop-kernel/freemint-multitasking-model.md:233-238`) und widerspricht im gemeinsamen Raum der Regel „gleiche VA = gleiche PA“. `Pfork` antwortet weiter mit dem heutigen Fehler für unbekannte Funktionen (`EINVFN`). `Pvfork` (Eltern schläft, Kind teilt den Speicher, `FM/k_fork.c:252-310` [S1]) ist dagegen im Modell billig und kann mit Stufe 2b kommen, falls mintlib es benutzt (mintlibs TOS-Zweig nutzt `Pexec 5`+`4/6`, PROC01).

### Frage 20 — `F_OS_SPECIAL`-Äquivalent

**Antwort:**
- AES und Desktop (`kernel/src/desk/`) laufen in EL1 und sind über `kalias` ohnehin „besonders“ — kein Flag nötig.
- Ab Stufe 3 wird `F_OS_SPECIAL` (`0x8000`, `FM/mint/mem.h:107` [gelesen]) als `memspace`-Eigenschaft umgesetzt (Besitzerrechte auf alles, eigene `PROT_P` für andere `PROT_S`, `FM/arch/mprot030.c:1122-1160` [S1]), **aber für niemanden gesetzt**. Grund: Auch unter FreeMiNT bekommt es nur ein per `GEM=` gestartetes externes AES (`FM/init.c:220` [S1]); XaAES und normale Desktops (TeraDesk) laufen ohne (Frage 9 Nr. 6).
- Der spätere Umzug des Desktops nach EL0 (Merkzettel „desktop-must-run-in-user-scope“) braucht es nach diesem Befund nicht: EmuDesk spricht mit Programmen über AES-Aufrufe und `shel_write`, nicht über deren Speicher **[Schluss]** — beim Umzug prüfen.

---

## 3. Zielentwurf im Detail

### 3.1 Adressraumaufteilung

```text
TTBR1 (Kernel, unverändert):  0xFFFF_0000_0000_0000 + PA   ganzes DRAM Normal-WB, MMIO Device   (KA/mmu.rs)

TTBR0 (je memspace eine Wurzel):
  VA [0x0000_0000, 0x0000_1000)   Seite 0, PRIVAT je Prozess (VA ≠ PA):   Systemvariablen $400.., OSHEADER, $5A0-Jar,
                                   Trampoline 0x700.., Keytbl 0x768, Kbdvecs 0x9A0, Stubs 0xA00, os_run-Zelle
  VA [0x0000_1000, Pool-Anfang)    nicht abgebildet, außer veröffentlichten Kernelseiten bei VA = PA (ADR-023, PROT_PR/G)
  VA = PA  ST-RAM-Pool            Pi: 0x0040_0000..0x00E0_0000   virt: 0x4040_0000..
  VA = PA  TT-RAM-Pool            Pi: 0x0100_0000..≈0x3AE0_0000  virt: 0x4100_0000..
  VA = PA  Bildschirmspeicher      Normal-NC, PROT_G (ADR-023 Nachtrag)
  alles andere                     ungültig (Übersetzungsfehler → Abbruch)
  Obergrenze aller Anwender-VAs   < 0x8000_0000 (TOS_MD_MAX_END)
```

- **Warum Seite 0 privat bleibt (E3):** Inhalte sind teils prozessspezifisch (PARMBLK-Scratch und Rücksprungzelle für den USERDEF-Upcall, `NT/loader.rs:757-783`; KEYS02-Tabelle; Keytbl-Kopie) und an feste ABI-Adressen gebunden. EmuTOS hat sie global, aber Supervisor-only; NewTOS gibt sie seit B1 an EL0 (`docs/COMPATIBILITY.md:684-708`). Die Abweichung „Zeiger in Seite 0 sind prozessrelativ“ besteht schon heute und wird in `COMPATIBILITY.md` als einzige Ausnahme vom gemeinsamen Raum benannt.
- **Basepage wandert** von VA 0 an den Anfang des TPA-Blocks (VA = PA). Seite 0 enthält danach keine Basepage mehr.
- **QEMU-Aliasing:** jede Anwender-VA hat einen zweiten Weg über TTBR1. Das ist heute schon so (Frage 2) und wird durch Spike 0b bestätigt. **0b:** auf QEMU 11.0.2 bestätigt (S0b §3). Mismatched **Memory-Type**-Aliase gibt es nur beim Bildschirm, mit der vorhandenen Clean+Invalidate-Regel (`NT/usermem.rs:433-534`, `NT/mmu.rs:491-542` [gelesen]).

### 3.2 Seitentabellen je `memspace`

- **Wurzel:** L0 je `memspace`; L0[0] → L1 je `memspace`; L1[0] → L2₀ je `memspace` (erstes GiB, enthält die Seite-0-Tabelle); L1[1..] → **geteilte** L2; L2₀[0] → L3 für `[0, 2 MiB)` je `memspace` (Seite 0 + ggf. Kernel-Veröffentlichungen); L2₀[1..] → geteilte L3 (Kopie der Zeiger aus der geteilten L2₀-Vorlage).
- **Stufe 1:** alle L3/L2 außer den genannten sind **eine** geteilte Menge. Pool-Einträge werden beim Booten (oder, Rückfall, bei Allokation) eingetragen: Normal-WB, Inner-Shareable, `AP=01`, `PXN=1`, `UXN=0` (wie `UserRwExec` heute, `NT/mmu.rs:202-205` [gelesen]).
- **Stufe 3:** „Copy-on-differ“ je 2-MiB-Bereich: Weicht in einem Bereich mindestens eine Seite der Rechte dieses `memspace` vom geteilten Muster ab, bekommt der `memspace` ein eigenes L3 (Kopie der geteilten, dann Änderung) und sein L2-Eintrag zeigt darauf. Das geteilte Muster ist „Nicht-Besitzer-Rechte“ (FreeMiNTs `init_page_table` füllt ebenso zuerst als Nicht-Besitzer, `FM/arch/mprot030.c:208-216,915,976` [S1]).
- **Änderungsregel:** Tabellenschreibzugriffe nur unter der BDOS-Sperre; `dsb ish` vor der Nutzung (`NT/mmu.rs:382-392` [gelesen]); bei **Verengung** (gültig → ungültig, RW → RO) TLBI nach dem Schreiben; bei Erweiterung ungültig → gültig kein TLBI — weil ungültige Einträge nicht im TLB stehen (S1 E.4 Nr. 3; belegt: `RXCLRD` D8-7852, `IWZCBG` D8-7858), aber `dsb` vor der Nutzung (`RSPVBD` D8-7704) und eine Kontext-Synchronisation (ISB/`eret`) für Befehlsabrufe. **0a: korrigiert** — bei Erweiterung *gültig → gültig* (RO→RW, `UXN` 1→0) ist **doch ein TLBI nötig**: gültige Einträge sind gecacht und selbst Permission-Fault-Einträge dürfen gecacht sein (`RSQBCS` D8-7852, `RJVXRH`/`IWJLXV` D8-7841, `RGPPYH` D8-7858). **0a: korrigiert/belegt (V0a C3/C4).** Break-before-make verlangt die Architektur bei (i) Änderung von Speichertyp/Shareability/Cacheability, (ii) Änderung der Ausgabeadresse, wenn alt oder neu Schreibrecht gewährt oder der Inhalt am neuen OA abweicht, (iii) Änderung der Blockgröße (Block↔Tabelle) ohne FEAT_BBML1/2 — **der A72 hat beides nicht** [Schluss: ARMv8.0, kein `ID_AA64MMFR2_EL1` im TRM], (iv) Anlegen eines globalen Eintrags, der nicht-globale überlagert (`RWHZWS`, D8-7858/7859). Reine `AP/UXN/PXN`-Änderungen im Blattdeskriptor stehen nicht in der Liste (kein BBM, aber TLBI s. o.); **Änderungen von `APTable/UXNTable/PXNTable` verlangen BBM samt TLBI aller Ebenen** (`IYQWJF` D8-7757, `IZGSNV` D8-7759). Folge „Copy-on-differ“: Block→Tabelle ist BBM; das Umhängen eines L2-Tabelleneintrags von geteiltem auf eigenes L3 steht nicht ausdrücklich in der Liste — **konservativ ebenfalls BBM** [Schluss]. Folge nach `RDDMVT` (D8-7859): Eintrag ungültig → `dsb` → TLBI (Broadcast-Form) → `dsb` → neuer Eintrag → `dsb`. Verletzte BBM kann als TLB-Konflikt-Abort (`DFSC 0b110000`) auftauchen (D8-7842).
  - **0a: Invarianten für Tabellen.** (1) Bits [63:59] aller Tabellendeskriptoren sind **0**: der A72 kann die Hierarchiebits nicht abschalten (`TCR_EL1.HPD0/HPD1` im TRM RES0, Table 4-53 S. 4-147; ARM ARM Table D8-50, D8-7737/7738). (2) `AF = 1` in jedem gültigen Block-/Seitendeskriptor — der A72 hat keine HW-AF-Verwaltung (TRM Table 4-24 `HWAccFlg = 0`, S. 4-100; ARM ARM D8-7845 Schritt 9). (3) `TCR_EL1.EPD0 = 1` ersetzt kein TLBI (wirkt nur bei TLB-Miss, D24-9923). (4) Konsistenzhinweis (kein Track-B-Fund): „sein L2-Eintrag zeigt auf eigenes L3“ setzt ein **eigenes L2** je abweichendem GiB voraus; im ersten GiB (Pi: Pool `0x0040_0000…≈0x3AE0_0000`, L2₀ ist ohnehin je `memspace`) erfüllt, auf `qemu-virt` (Pool ab `0x4000_0000`, zweites GiB mit geteiltem L2) **nicht** — dort braucht ein abweichender `memspace` zusätzlich ein eigenes L2.
- **TLBI und QUIRK-001:** Verengungen passieren in `Mfree`/`Mshrink`/`Pterm`/Rechte-Rücknahme, also im Trap, kurz vor dem `eret`. Regel: TLBI wird **nicht** direkt vor `eret` ausgeführt, sondern durch denselben Weg wie heute geschleust (`restore_boot_ttbr0` + Device-Schreibverkehr + `install_ttbr0`), ab Stufe 4 mit ASID per `TLBI VAE1`/`ASIDE1`. Spike 0b misst, ob TLBI-vor-`eret` auf 11.0.2 noch hängt. **0b:** hängt nicht — `msr neu; isb; tlbi vmalle1; dsb ish; eret` 32/32 richtig auf `raspi4b` und `virt` (S0b §4, `sw-tlbi-eret`); die Regel kann nach dem Pi-Lauf entfallen. **0a:** Die Regel „TLBI nicht direkt vor `eret`“ ist **keine Architekturregel**, sondern QEMU-Workaround (QUIRK-001). Architektur: `TLBI; DSB ISH` beendet die Pflege, die Kontext-Synchronisation liefert der `eret` selbst (`IBLDZX` D8-7875; Ausnahmerückkehr ist beim A72 ohne FEAT_ExS stets eine CSE, D24-8470; TRM Table 4-44 Bits 11/22 RES1, S. 4-126/4-127). TLBI-Wahl nach Änderung von Tabellen-/Zwischenebeneneinträgen: `VAE1`/`VAAE1`/`VMALLE1`, **nicht** `…L` (`RDRCZZ` D8-7869).

### 3.3 Schutzmodell — `PROT_*` auf AArch64

Deskriptorbits nach `NT/mmu.rs:91-111` [gelesen]. **0a:** Bitlagen jetzt gegen die Primärquelle bestätigt (ARM ARM Figure D8-16/Table D8-52, D8-7741…7744: `AP[2]`=7, `AP[1]`=6, `SH`=[9:8], `AF`=10, `nG`=11, `AttrIndx`=[4:2], `PXN`=53, `UXN`=54; Tabellendeskriptor `APTable`=[62:61], `UXNTable`=60, `PXNTable`=59, Table D8-50). `AP[2:1]`: `01` = EL1+EL0 RW, `11` = EL1+EL0 RO, `00` = nur EL1 RW (Table D8-63, D8-7756; `10` = nur EL1 RO).

| Modus | Besitzer (Region in `memspace`) | Nicht-Besitzer | FreeMiNT-Beleg |
|---|---|---|---|
| `PROT_P` | `AP=01`, `UXN=0` | **ungültig** | `mprot030.c:400-402,653-680` [S1] |
| `PROT_G` | `AP=01`, `UXN=0` | `AP=01`, `UXN=0` (für alle gleich → geteiltes L3 möglich) | `:656-659` [S1] |
| `PROT_S` | `AP=01`, `UXN=0` | **ungültig** (EL1 erreicht die Seite über den Kernel-Alias; ein EL1-only-Eintrag in `TTBR0` hätte keinen Nutzen) | `:400-402` [S1] |
| `PROT_PR` | `AP=01`, `UXN=0` | `AP=11`, `UXN=0` (68k kennt kein NX; Ausführen bleibt erlaubt) | `:400-402` [S1] |
| `PROT_I` | ungültig | ungültig | `:656-659` [S1] |
| Kernel-veröffentlicht `PROT_PR`/`PROT_G` (ADR-023) | — (Besitzer ist der Kernel) | wie heute `UserRoNx`/`UserRwNx` (+`Uncached`) | `NT/usermem.rs:462-468` [gelesen] |

- **`PXN = 1` überall** (unverändert). **`UXN`** wird für Anwenderspeicher nicht gesetzt: Heap-Blöcke müssen ausführbar sein (LDG lädt Code in `Mxalloc`-Speicher bzw. per `Pexec(3)` in einen Poolblock); W^X ist eine spätere, getrennte Härtung (heute schon so: `NT/loader.rs:29-34` [gelesen]).
- **0a: PXN/UXN-Kombinationen geprüft.** `PROT_PR`-Nicht-Besitzer (`AP=11`, `UXN=0`, `PXN=1`) = „PrivRead, UnprivRead, UnprivExecute“ (Table D8-65, D8-7759/7760) ✓; Besitzer/`PROT_G` (`AP=01`, `UXN=0`) = „PrivRead, PrivWrite, UnprivRead, UnprivWrite, UnprivExecute“ ✓ (EL1-Ausführung bei `AP=01` entfällt ohnehin, `RVWLLR` D8-7758). Wirklich nötig ist `PXN=1` nur bei `AP=11`.
- **Seitengranularität:** ab Stufe 3 werden Poolblöcke auf 4 KiB gerundet und ausgerichtet (FreeMiNT-`QUANTUM`, dort 8 KiB, `FM/mint/mem.h:152` [S1]); sonst teilen sich zwei Besitzer eine Seite. Das ändert die beobachtbare Granularität von `Malloc` → sanktionierte FreeMiNT-Abweichung (E16).
- **Standardmodus** einer Allokation = PRGFLAGS-Schutz des `memspace` (Frage 8), Kernel-eigene Poolblöcke (AES-Puffer `gsxif`, Umgebungen, Ressourcen) erhalten den Modus, den ihr Verwendungszweck verlangt: Ressourcenblob = Besitz des Programms (EmuTOS: `Mxalloc` des Programms, `NT/usermem.rs:606-616` [gelesen]), Bildschirmpuffer `WF_SCREEN` = `PROT_G` (wird an alle ausgegeben, `docs/COMPATIBILITY.md:170-176`).

### 3.4 Besitzmodell

Siehe Frage 6. Ergänzend:
- **Lebenszyklus:** `Mxalloc` → `get` aus EmuTOS-Pool (unverändert, ADR-021 bleibt hier gültig) → Region anlegen (`links = 1`, Besitzer = aktueller `memspace`, Modus) → Stufe 3: Rechte eintragen. `Mfree` → `detach` dieses Besitzers → bei `links == 0` EmuTOS-`mfree` + Stufe 3: Seiten für alle ungültig + TLBI (`FM/memory.c:932` [S1]).
- **Wer darf `Mfree`?** EmuTOS: jeder, ohne Besitzprüfung (`ET/bdos/umem.c:135-159`, zitiert in `NT/usermem.rs:606-612` [gelesen]); FreeMiNT: nur Besitzer, `M_KEEP`-Blöcke jeder (`FM/dosmem.c:365-398` [S1]). Empfehlung: FreeMiNT-Regel ab Stufe 2 (sonst kann ein Prozess die Basepage eines Geschwisters freigeben); `EIMBA` für Nicht-Besitzer — beobachtbar nur für Programme, die fremde Blöcke freigeben (TeraDesks `xrsrc_free` gibt **eigene** frei, `NT/usermem.rs:610-612`).

### 3.5 Lader

1. Kopf lesen (`ET_DYN`, Relokationstabelle finden; `PT_NOTE` PRGFLAGS).
2. TPA-Block aus dem Pool (`Mxalloc` nach PRGFLAGS ST/TT, `FM/memory.c:1442-1472` [S1] / `ET/bdos/proc.c:481-485` [S1]); Basis `B` seitenausgerichtet.
3. Segmente nach `B + p_vaddr` streamen (heute `NT/loader.rs:474-494`, nur mit Bias), BSS/TPA nullen.
4. `R_AARCH64_RELATIVE` anwenden: `*(u64*)(B + r_offset) = B + r_addend`; Zieladresse muss in einem beschreibbaren Segment liegen.
5. Basepage bei `B` schreiben (`p_lowtpa = B`, `p_hitpa = B + size`, Segmente + `B`, `p_parent`, `p_env` = VA der Umgebung, `p_xdta = B + 0x80`).
6. Seite 0 des Prozesses schreiben (B1-Systemvariablen, Jar, Trampoline, Kbdvec-Standard) — **in die prozesseigene Seite-0-PA**, nicht in den TPA.
7. `sync_code_cache` über das Bild (`NT/mmu.rs:445-489`).
8. `memspace` anlegen bzw. teilen (Pexec 4), Regionen eintragen.
9. Eintritt: `x0 = B`, `ELR = B + e_entry`, `SP_EL0 = p_hitpa`.

`ET_EXEC` → `EPLFMT` (nach Stufe 1). Fehlerpfade geben TPA und Umgebung frei (heute `NT/loader.rs:430-441`).

### 3.6 Kernelzugriff auf Anwenderspeicher

Zwei Module (Namen vorläufig): `uaccess` (Aufruferrechte, `ldtr/sttr`, Fehlerabfang) und `kalias` (Regionsprüfung + `VA_OFFSET`). Regel für die Umstellung: **jede** EL1-Stelle, die heute einen Anwenderzeiger direkt dereferenziert, wird einer der beiden zugeordnet. Bekannte Gruppen: AES-Parameterblock und Bäume (`tos-api/src/aes/mod.rs:299-320` liest mit normalen Ladebefehlen über das aktive `TTBR0` [gelesen]) → `kalias` (AES ist privilegiert); GEMDOS-Puffer/Pfade → `uaccess`. Die Inventur (alle Stellen) ist Auftrag für den Auditor vor Stufe 3 (Merkregel „Quellen-Sweeps an den Auditor“).

### 3.7 `Pexec` — alle Modi

Spezifikation EmuTOS-Modi liegt vor (`docs/api/gemdos/process/4b-pexec.md:54,91-126,178-179` [gelesen]); FreeMiNT-Modi 100+ sind **noch nicht** spezifiziert (Regel 5: zuerst Spec, Stufe 2b).

| Modus | Laden/Anlegen | Basepage/Env/TPA gehören | `memspace` des Kindes | Rückgabe | Wer sieht was (ab Stufe 3) | Stufe |
|---|---|---|---|---|---|---|
| 0 `PE_LOADGO` | laden + starten, Eltern schläft | Kind (EmuTOS `owner = p`, `ET/bdos/proc.c:305-307` [S1]) | neu; nicht strikt: Eltern-Regionen sichtbar | Exitcode | Eltern: Kind-TPA nicht (Kind besitzt sie); Kind: Eltern-Regionen (nicht strikt) | 1 |
| 3 `PE_LOAD` | laden, nicht starten | **Aufrufer** | — (noch keiner) | **PD-Zeiger** | Aufrufer: alles (Besitzer) | 1 |
| 4 `PE_GO` | starten (PD vom Aufrufer, muss ihm gehören: FreeMiNT `EFAULT` sonst, `FM/k_exec.c:378-387` [S1]) | unverändert (Aufrufer) | **geteilt mit Aufrufer** (Thread) | Exitcode | identisch mit Aufrufer | 1 (Teilen) |
| 5 `PE_BASEPAGE` | Env + Basepage, TPA = Block (EmuTOS größter Block, `:210-237,460-502` [S1]; NewTOS: Standardgröße, E9), `p_flags = 0` | Aufrufer | — | PD-Zeiger | Aufrufer | 2 |
| 6 `PE_GOTHENFREE` | Besitz auf Kind (`set_owner`, `ET/bdos/proc.c:238-248` [S1]), dann starten | **Kind** | neu (Kopie-Sicht, nicht strikt) | Exitcode | wie Modus 0 | 2 |
| 7 `PE_BASEPAGEFLAGS` | wie 5, `p_flags = path` | Aufrufer | — | PD-Zeiger | Aufrufer | 2 |
| 100 | wie 0 ohne Warten (`FM/k_exec.c:210-213` [S1]) | Kind | neu | PID | wie 0 | 2b |
| 104 | wie 4 ohne Warten (`:232-240` [S1]) | Aufrufer | geteilt | PID | wie 4 | 2b |
| 106 | wie 6 ohne Warten; SLB-Pfad (`:232-234` [S1], `FM/slb.c:331` [S1]) | Kind | neu | PID | wie 6 | 2b |
| 200 | Overlay: Aufrufer durch Programm ersetzen (`:214-217,411` [S1]) | Prozess selbst | bisheriger, Regionen des alten Bildes werden gelöst | — (kehrt nicht zurück) | — | 2b |
| 204 / 206 | Overlay einer vorbereiteten Basepage (`:243-259` [S1]) | wie 200 | — | — | — | 2b |
| 50 `PE_RELOCATE` | EmuTOS: nur NatFeats-Bootstrap (`ET/bdos/proc.c:193-209` [S1]) | — | — | `EINVFN` | — | bleibt `EINVFN` (Nicht-Ziel) |
| sonst | — | — | — | `EINVFN` (EmuTOS) | — | — |

- **Nicht-wartende Modi (100/104/106)** setzen einen Scheduler voraus, in dem Eltern und Kind gleichzeitig laufbereit sind. Der kooperative Scheduler kann das (`NT/sched.rs` Run-Set, `admit`, `:351`), aber `handle_pexec` nimmt heute den Eltern grundsätzlich aus dem Run-Set (`KA/exceptions.rs:1866-1877` [gelesen]) — für 100/104/106 entfällt das. `Pwait`/`Pwaitpid` (GEMDOS `0x4D` existiert, `KA/exceptions.rs:1011`) liefert später den Exitcode.
- **Accessories** bleiben eigene Prozesse (Slot-Peers, `kernel/src/accs.rs:201-206` lädt per `pexec::load(…, false)` [gelesen]) — FreeMiNT-Weg (`create_process`, S1 A.11), nicht EmuTOS' „alle ACCs im AES-Speicher“ (S1 B). Nach Stufe 1 sind ihre Zeiger (z. B. `menu_register`-Titel) für die AES über `kalias` lesbar; ab Stufe 3 gehört ihr Speicher ihnen.

### 3.8 `Ptermres`, Accessories, TSR

Siehe Frage 16 und 3.7. `Ptermres(blkln)`: TPA-Region auf `blkln` verkleinern (`Mshrink`-Logik), Regionen → Systembesitzer, `TSR`-Flag, `PROT_G`; Slot frei; Eltern fortsetzen wie heute (`finish_terminate`, `KA/exceptions.rs:1239-1390`).

### 3.9 AES: Nachrichtenzeiger, Userdef-Callbacks

- Nachrichten bleiben Kernel-Warteschlangen (Frage 9 Nr. 3).
- Bäume/Menüs/Strings fremder Clients: Lesen über `kalias`.
- Userdef/`G_PROGDEF` mit Baumeigentümer ≠ laufender Prozess: Stufe 1/2 direkt (wie heute); ab Stufe 3 zeitweise Freigabe (E10a) bzw. Verschieben (E10b).
- `rsrc_load`: Blob ist ein Poolblock des Programms bei VA = PA; `rsrc_gaddr` liefert dessen Adresse (heute Fensteradresse, `tos-api/src/aes/resource.rs:2365,2395` [S1]).

### 3.10 Globaler `Mxalloc`, `M_KEEP`, `shm`

- `Mxalloc(…, 0x20 | mode)` → `PROT_G` (Frage 8), sichtbar für alle. `F_KEEP` → Region überlebt das Programmende beim Systembesitzer; jeder darf sie freigeben (FreeMiNT `FM/dosmem.c:365-398` [S1]).
- **`shm`** (`u:\shm`, `Fcntl(SHMSETBLK/SHMGETBLK)`, `FM/shmfs.c:658-720` [S1]): im Modell trivial (Mitbesitz = `attach`, gleiche Adresse), **aber NewTOS hat kein `u:`-Laufwerk** — das ist ein eigenes Projekt (Pseudodateisystem). Nicht Teil von ARCH03; als Folgeticket nennen (E17).

### 3.11 Umgebungsblöcke (MEM03)

Mit Stufe 1 gelöst: `alloc_env_for` liefert einen Poolblock; `p_env` = seine Adresse = VA (heute PA, nicht abgebildet, `NT/loader.rs:498-507,675` [gelesen]). `EnvSpec::Inherit` liest `(*run).p_env` über `kalias` (heute als PA, `NT/loader.rs:1244-1256` [gelesen]). Ab Stufe 3: Env-Region nach dem Laden auf den PRGFLAGS-Modus des Kindes (FreeMiNT `mark_region(env, …)`, `FM/k_exec.c:398` [S1]); Kind ist Mitbesitzer.

### 3.12 `kbdvec` (KEYS02 D1 neu betrachtet)

- **Heute:** je Prozess eine Tabelle in Seite 0 bei `0x9A0`, Haken endet mit dem Prozess, TSR-Haken gehen nicht (`docs/COMPATIBILITY.md:640-662` [gelesen], Beobachtbares (3)).
- **Im neuen Modell möglich:** eine **globale** Tabelle (EmuTOS: eine Tabelle in BIOS-BSS, `ET/bios/aciavecs.S:126-141`, laut `COMPATIBILITY.md:644-646`; FreeMiNT: globale `PROT_G`-Region, `FM/keyboard.c:1524` [S1]) auf einer globalen Kernelseite bei VA = PA, `PROT_G`. Haken eines TSR werden dann in jedem Kontext aufgerufen; Schutz gegen hängende Haken: beim Freigeben einer Region setzt der Kernel jeden Vektor, der in sie zeigt, auf den Standard zurück (sicherer als EmuTOS, das genau daran stirbt, Spec A8).
- **Empfehlung E4:** KEYS02 D1 bis einschließlich Stufe 3 **unverändert** lassen (frisch abgenommen 2026-10-04, Doom läuft); Wechsel zur globalen Tabelle als Erweiterung nach Stufe 3, sobald ein TSR mit `kbdvec`-Haken als Anwendungsfall vorliegt (Merkregel „echte Anwendungen sind der Schiedsrichter“).

### 3.13 `vfork`/`system()` (PROC01)

mintlibs TOS-Zweig: `tfork` = `Pexec(5)` + `Pexec(4/6)` auf Code im eigenen Adressraum (`TASK/PROC01.md`). Mit Stufe 1 (gemeinsamer Raum, `Pexec 4` als Thread) und Stufe 2 (Modi 5/6/7) geht das ohne Sonderweg; das `system.c`-Overlay kann danach entfallen (Abnahme PROC01). `Pvfork` optional (Frage 19).

### 3.14 Upcalls

Unverändert: Trampoline in Seite 0; `deliver_upcall`/`Psigreturn` (`KA/exceptions.rs:1395-1580`). Ab Stufe 3 beim Upcall in fremden Code die Freigaberegel aus E10 anwenden; `etv_term`-Handler gehören immer dem sterbenden Prozess selbst (kein Problem).

### 3.15 Schutzverletzungen

Siehe Frage 11. Umsetzung in Stufe 0c (Abbruch statt Halt), Klassifikation in Stufe 3.

### 3.16 Kontextwechsel

- **Stufe 1–3:** `install_ttbr0(memspace.l0_pa)` mit vorausgehendem `restore_boot_ttbr0` (TLBI) und UART-Klammer an den vier heutigen Stellen (`KA/exceptions.rs:1056-1066,1377-1382,1910-1920`, `KA/user_entry.rs:101-117` [S1]). Wechsel zwischen zwei Prozessen **desselben** `memspace` (Pexec-4-Thread) braucht keinen Wechsel. **0b:** Auf QEMU 11.0.2 genügt die Architekturfolge ohne UART-Klammer (S0b §4); die Klammer bleibt nur bis zum Pi-Lauf.
- **Stufe 4 (optional, nur Hardware):** ASID je `memspace`, `nG = 1` auf allen Anwendereinträgen, Wechsel ohne TLBI; ASID-Generationen mit `TLBI ASIDE1IS` bzw. `VMALLE1IS` beim Überlauf. **0a:** belegt (Frage 15; A72 16-Bit-ASID, `ASIDE1` lässt globale Einträge stehen).

### 3.17 Mehrkern-Vorbereitung (nur Notizen, kein Ziel von ARCH03)

- Kerne 1–3 sind geparkt (`docs/ARCHITECTURE.md:116-117`).
- Im Entwurf schon richtig gewählt: `dsb ish`/Inner-Shareable-Tabellenwalks; TLBI-Aufrufe sollten von Anfang an die `…IS`-Varianten benutzen (auf einem Kern gleichwertig — **0a:** belegt: `…IS` wirkt auf alle TLBs der Inner-Shareable-Domäne, `RQRNJC` D8-7874, `IXMWNR` D8-7861; A72 `MaintBcst = 0x2`, TRM S. 4-102).
- Später nötig: echte Sperre statt „BDOS-Giant-Lock = ein Kern“ für Regionstabelle und Seitentabellen; `TTBR0` je Kern; ASID-Zuteilung global; ein `memspace`, der auf zwei Kernen läuft (Pexec-4-Thread), braucht Broadcast-TLBI bei Verengung.

---

## 4. Migrationsplan in auslieferbaren Stufen

Jede Stufe: eigener Zweig von `desk-rebuild` (Merkregel „Agent-Worktrees starten auf einem Waisen-Branch“), Developer → Tester → Auditor → Lead-Commit, STATUS-Eintrag je Stufe. Pi-Abnahme macht der Anwender (Merkregel „Workers take no hardware actions“).

### Stufe 0a — Track-B-Nachweis (Dokument, kein Code) — **erledigt 2026-10-04:** `docs/analysis/arch03-stage0a-armarm-verification.md`

- **Inhalt:** Für jede Stelle „[Track B offen]“ in diesem Plan die ARM-ARM-Fundstelle (DDI 0487, Ausgabe **M.d** — 0a: so gepinnt in `references/arm-arm/`, nicht M.c) bzw. AAELF64 eintragen: Deskriptorbits (AP, UXN, PXN, nG, AF, SH), `TCR_EL1.A1/AS`, `ID_AA64MMFR0_EL1.ASIDBits`, `ID_AA64MMFR1_EL1.PAN`, `SCTLR_EL1.UCI/UCT`, TLBI-Varianten und ihre Barrieren, Break-before-make-Regeln, `ESR_EL1` DFSC/IFSC/`WnR`, `LDTR/STTR`, `R_AARCH64_RELATIVE`. Cortex-A72-Werte aus dem A72-TRM.
- **Wer:** Researcher mit Zugang zu den Arm-Dokumenten (laut S1 liefern die Arm-Seiten ohne JavaScript keinen Text; QUIRK-003 nennt einen „Dokumentendienst“) — ggf. legt der Anwender die PDFs nach `references/arm-arm/`. **0a:** Die PDFs liegen dort; `pdftotext -layout` genügt als Extraktion. **Rest-offen** (nicht aus den drei Quellen entscheidbar): QEMU-Verhalten und Identifikationsregister unter QEMU (→ 0b); `Elf64_Rela`/`DT_*`/`ET_DYN` (SYSVABI64/gABI nicht gepinnt, → Pinnen + 0d); A72-Errata (nicht gepinnt).
- **Aufwand:** 1 Tag. **Risiko:** Zugang. **Rückrollen:** entfällt.

### Stufe 0b — Spike auf QEMU 11.0.2 und Pi (Kopie des Baums, nichts wird eingecheckt außer der Messung)

- **Messungen:** (1) Pool bei VA = PA in geteilten L3 mit 4-KiB-Seiten, Kernel schreibt über den Alias — Boot + Programmstart; (2) dasselbe mit 2-MiB-Blöcken; (3) Installation beim Booten mit und ohne TLBI (ADR-010-Versuch nachstellen, Board `raspi4b` **und** `virt`); (4) ASID-getaggtes `TTBR0` (QUIRK-001-Wand 2); (5) TLBI direkt vor `eret` ohne UART-Klammer (Wand 1); (6) Bildschirm-PA aus dem Bootprotokoll. **0a-Ergänzung:** (7) Identifikationsregister unter QEMU `raspi4b`/`virt` und auf der Pi auslesen und protokollieren (`ID_AA64MMFR0/1/2_EL1`, `CTR_EL0`, `CCSIDR_EL1`; erwartet laut A72-TRM: ASIDBits 16, PAN/UAO/BBM/HPDS/HAFDBS nicht vorhanden, 64-Byte-Zeilen — QEMU kann abweichen); (8) pro QEMU-Workaround (UART-Klammer, ASID-`msr`, Aliasing, TLBI vor `eret`) ausdrücklich festhalten, dass er **keine** Architekturregel ist (V0a §9), damit er bei Fortfall sauber entfernt werden kann.
- **Ergebnis:** Messprotokoll `measurements/measurement-…-arch03-spike/`, QUIRK-001/ADR-010-Nachtrag (Vorschlag), Entscheidung Boot-Abbildung vs. Abbildung bei Allokation.
- **0b: QEMU-Teil erledigt 2026-10-05** — `docs/analysis/arch03-stage0b-spike.md`, Messprotokoll `measurements/measurement-2026-10-05-arch03-0b/`. Auf QEMU 11.0.2 (`raspi4b` + `virt`, je 3 Läufe + 4 Boot-Nachstellungen): (1)/(2) Pool VA = PA in 4 KiB und 2 MiB mit Kernel-Store über den Alias → OK; (3) Einbau mit und ohne TLBI, auch beim Booten mit Austausch der Boot-Identität (ADR-010) → OK; (4) ASID-getaggtes `TTBR0` → OK; (5) TLBI direkt vor `eret` ohne Klammer → OK; (6) Bildschirm-PA `0x3C10_0000` (`raspi4b`) / `0x5081_7000` (`virt`); (7) ID-Register = A72-TRM außer L1D-Assoziativität (QEMU 4-fach, TRM 2-fach); (8) alle vier Workarounds sind auf QEMU 11.0.2 entbehrlich. Entscheidung: **Boot-Abbildung** (Rückfall nur bei Pi-Widerspruch). **Offen:** Pi-Lauf (Abbild + Anleitung S0b §7), danach QUIRK-001/ADR-010-Nachtrag.
- **Aufwand:** 2–3 Tage. **Risiko:** mittel (Pi-Läufe nur durch den Anwender). **Rückrollen:** entfällt (Wegwerfkopie).

### Stufe 0c — Programmabbruch statt Maschinenhalt; Fehlerabfang für Anwenderzeiger

- **Module:** `KA/exceptions.rs` (`rust_sync_handler`, neuer Zweig für `EC_IABT_LOWER`/`EC_DABT_LOWER` → Meldung + `terminate_and_redirect(frame, -1)`), Fehlerabfang-Rahmen nach dem `probe_read32`-Muster, `uaccess`-Hilfen in `tos-api` (erste Nutzer: `read_user_cstr`, `read_env_block`).
- **Invarianten:** Ein EL0-Abbruch hält nie mehr die Maschine an; der Eltern- bzw. Desktopprozess läuft weiter; Kernel-Abbrüche außerhalb `uaccess` halten weiter (mit Bericht).
- **Tests:** Hosttest der Klassifikation (ESR-Dekodierung); QEMU: Testprogramm `sdk/examples/` „schreibt nach 0xDEAD0000“ → Alert/Serienzeile, Desktop lebt; `Fread` mit ungültigem Puffer → Fehlercode statt Halt; Pi-Lauf.
- **Risiko:** gering–mittel (Terminierung aus einem Nicht-SVC-Rahmen; `finish_terminate` erwartet `parent_frame`, `NT/pterm.rs:163-166`). **Rückrollen:** Revert, unabhängig.
- **Aufwand:** 2–3 Tage. **Nebennutzen:** auch ohne ARCH03 wertvoll (heute hält jeder Programmabsturz die Pi an).

### Stufe 0d — PIE-Lader bei Ladebasis 0 + Neubau aller Programme

- **Module:** `NT/elf.rs` (`ET_DYN`, `PT_DYNAMIC`, Relocs, `PT_NOTE`), `NT/loader.rs` (Relokationsschritt mit Bias = 0 heute), `sdk/newtos.ld` (PIE, `.dynamic/.rela.dyn/.got` behalten), `sdk/crt0.S` + `sdk/libc/crt0.S` (Basepage aus `x0`), `sdk/build-*.sh` / `sdk/apps/build-*.sh` (`-fPIE -pie`), neun Selbsttest-Blobs → `ET_DYN`.
- **Invarianten:** Verhalten für Programme unverändert (Bias 0 ⇒ identisches Bild); `ET_EXEC` wird **noch** angenommen (Übergang); jedes ausgelieferte Programm ist PIE.
- **Tests:** Hosttests: Relokationsanwendung (synthetisches ELF), Ablehnung `DT_TEXTREL`/fremder Relocs/`PT_INTERP`; `cargo xtask` beide Boards frisch gebaut (Merkregel „Board-Target-Falle“); Selbsttests; apitest (Basis: 82 PASS / 0 FAIL / 5 SKIP, ADR-023), QED/TeraDesk/Doom/CoreMark starten + Screendump; Pi-Lauf.
- **0a-Ergänzung:** Lader prüft den ELF-Kopf nach AAELF64 5.2 (`EM_AARCH64`=183, `e_flags`=0, `ELFCLASS64`, Little-Endian); im Toolchain-Spike zuerst `llvm-readelf -r` auf `hello.c`: welche Relokationstypen entstehen tatsächlich (nur `R_AARCH64_RELATIVE`=1027? auch `ABS64`/`IRELATIVE`/`GLOB_DAT`?) — das ist Werkzeugverhalten, nicht aus AAELF64 ableitbar (V0a H4); `Elf64_Rela`/`DT_*` sind gegen die zu pinnende SYSVABI64/gABI zu belegen (V0a H5/H6).
- **Risiken:** Clang/lld-PIE-Eigenheiten (GOT-Relaxierung, Relocs gegen schwache Symbole) — Toolchain-Spike zuerst mit `hello.c`, dann QED; ported Apps unverändert (Merkregel „Ported app stays unmodified“) — Fehler nur in Toolchain/Overlays beheben. **Rückrollen:** Revert; alte Binärdateien liegen auf der Karte bzw. im QEMU-Abbild.
- **Aufwand:** 4–6 Tage.

### Stufe 1 — Gemeinsamer Adressraum ohne Schutz (FreeMiNT `no_mem_prot`, ADR-010 umgesetzt)

- **Module:** `NT/mmu.rs` (geteilte Tabellen, `memspace`-Wurzeln, Seite-0-Tabelle je `memspace`; `USER_L3_COUNT`-Statik entfällt), `NT/arena.rs` (TPA-Block bei VA = PA), `NT/loader.rs` (Bias = Blockbasis, Basepage bei `B`, Seite 0 getrennt, `os_run`-Zelle, `p_parent`, `p_env`), `NT/pexec.rs` (echte PD-Zeiger, `Pexec 4` teilt `memspace`), `NT/usermem.rs` (Rückbau, Frage 5), `tos-api/src/gemdos/memory.rs` (EmuTOS-Traps direkt), `tos-api/src/bios/{bpb,getmpb}.rs`, `tos-api/src/xbios/{screen,keytbl,kbdvecs}.rs`, `tos-api/src/aes/{resource,object,window,gsxif}.rs` (`kalias`), `NT/process.rs` (`RUN` = Anwender-VA), `KA/exceptions.rs` (`handle_pexec`, `apply_switch`, `finish_terminate`), `kernel/src/launch.rs`, `kernel/src/desk/dosbind.rs`; alle Doku-Stellen aus Frage 5.
- **Invarianten:** (I1) jede Anwender-VA ≥ `0x1000` ist gleich ihrer PA; (I2) jede Anwender-VA < `0x8000_0000`; (I3) Seite 0 ist die einzige prozesseigene Übersetzung; (I4) jeder Zeiger, den ein GEMDOS/BIOS/XBIOS/AES-Aufruf ausgibt (außer in Seite 0), ist in jedem Prozess dieselbe Adresse; (I5) PXN auf allen Anwenderseiten; (I6) Kernel-Image, Heap, MD-Arena, Seitentabellen sind nie in `TTBR0`.
- **Tests:** Hosttests (Adressrechnung, `os_run`, Basepage-Felder); QEMU: Selbsttests, apitest (gleiche Zahlen, BS/BB-Prüfungen jetzt mit VA = PA), `envkid` aus PORT09 (MEM03-Abnahme), neuer Test „Shell-Idiom“ (`Pexec 3` → `p_tlen` lesen → `Mshrink` → `Pexec 4`), `port10.c`, QED/TeraDesk/Doom/Accessories, Screendumps; Pi-Lauf.
- **Risiken:** Wand 3 (→ Rückfall „Abbildung bei Allokation“, Frage 2); vergessene Stelle mit VA-0-Annahme (Liste S1 C.9, 22 Punkte, als Prüfliste); Isolation weg — ein Absturz kann andere Prozesse beschädigen (bewusst, wie EmuTOS; Stufe 0c begrenzt die Folgen).
- **Rückrollen:** Revert der Stufe; Programme aus 0d laufen auf beiden Kernen (PIE + `x0`).
- **Aufwand:** 8–12 Tage.

### Stufe 2 — `Pexec`-Vollstand (EmuTOS 5/6/7), `Ptermres` echt, Regionen mit Mehrfachbesitz, PRGFLAGS

- **Module:** `NT/pexec.rs`, `NT/pterm.rs` (`blkln`, TSR-Übergabe), neue Regionstabelle (ersetzt Rest von `usermem`), `tos-api/src/bdos/mem/free_all_owned.rs` (Detach statt Free), `Mxalloc`-Dekodierung (ohne Wirkung), `PT_NOTE`-PRGFLAGS, `Mfree`-Besitzregel (3.4).
- **Invarianten:** Regionen werden genau bei `links == 0` frei; EmuTOS-Besitz (`m_own`) und Regionen-Besitz stimmen überein; `Pexec 5/7` → echte PD-Zeiger.
- **Tests:** Hosttests der Regionstabelle (Attach/Detach/Keep/TSR), Mxalloc-Modusdekodierung (beide Kodierungen, Frage 8); QEMU: PROC01-Abnahme (`Pexec(5)+Pexec(6)`), mintlib `system()` ohne Overlay, LDG-Vertikalschnitt (`ex_lib` + Demo-Client aus `Applications/ldg-src`, `ldg_cpush` als m68k-Asm-Retarget = Leeroperation, weil der Kernel beim Laden synchronisiert), TSR-Test (Programm `Ptermres`, zweites Programm ruft über Cookie-Zeiger in den TSR); Pi-Lauf.
- **Risiken:** Ownership-Umstellung von `free_all_owned` (heute EmuTOS-1:1) — Regression bei Programmende; Auditor gegen `ET/bdos/proc.c`.
- **Rückrollen:** Revert; Stufe 1 bleibt funktionsfähig.
- **Aufwand:** 5–7 Tage.

### Stufe 2b — FreeMiNT-Modi 100/104/106/200/204/206 (+ optional `Pvfork`)

- **Erst Spec** (Researcher, Regel 5): `docs/api/gemdos/process/4b-pexec.md` um die FreeMiNT-Modi aus `FM/k_exec.c:168-660` erweitern; Rückgabe-PID, `Pwaitpid`-Bezug.
- **Module:** `NT/pexec.rs`, `KA/exceptions.rs::handle_pexec` (Eltern bleibt laufbereit), Scheduler.
- **Tests:** QEMU: Programm startet Kind mit 100, beide laufen kooperativ, `Pwait` liefert Exitcode; Overlay-Test.
- **Aufwand:** 3–4 Tage (inkl. 1–2 Tage Spec). **Rückrollen:** Revert, unabhängig von Stufe 3.

### Stufe 3 — Rechte je Adressraum

- **Module:** `NT/mmu.rs` (Copy-on-differ-L3, Seitentabellen-Vorrat, Verengung + TLBI), Regionstabelle (Rechte berechnen aus Besitz/Modus/nicht-strikt/Freigaben), BDOS-Pool-Rundung auf 4 KiB (`tos-api/src/bdos/mem/pool.rs`), `Mxalloc` wirksam, `Ptermres` → `PROT_G`, Env-Modus, `uaccess`/`kalias`-Inventur abgeschlossen (Auditor-Liste), Fehlerklassifikation + Alert, E10-Mechanismus für fremde Userdefs, `F_OS_SPECIAL` (ungenutzt), `Mvalidate`/`Maccess` optional.
- **Invarianten:** (I7) ein Prozess erreicht `PROT_P`-Speicher anderer nur über nicht-strikte Elternschaft oder ausdrückliche Freigabe; (I8) freigegebener Speicher ist für alle ungültig, bevor er neu vergeben wird; (I9) kein TLBI unmittelbar vor `eret` ohne die geprüfte Klammer (**0b:** auf QEMU 11.0.2 hinfällig, S0b §4 — streichen, sobald die Pi bestätigt); (I10) keine Seite mit zwei Besitzern verschiedener Rechte (Quantum).
- **Tests:** Hosttests: Rechtematrix → Deskriptorbits (Tabelle 3.3), Copy-on-differ, Rundung; QEMU: Schutztest-Programme (Kind schreibt in Eltern-`PROT_P` strikt → Abbruch mit Meldung; nicht strikt → geht; `PROT_PR`-Block eines anderen lesen ja/schreiben nein; Zugriff nach `Mfree` → Abbruch), alle Anwendungen inkl. Accessories und TeraDesk mit Userdef-Hintergrund (`WF_NEWDESK`), LDG, TSR; Pi-Lauf mit Doom/QED/TeraDesk.
- **Risiken:** Programme, die stillschweigend fremden Speicher lesen (FreeMiNT-Erfahrung: „Some programs may refuse to run“, `mint.cnf:132-137` [S1]) — deshalb nicht strikt als Standard; TLBI-Häufigkeit (Leistung) — erst messen; QEMU-TLB-Verhalten (Spike-Ergebnis).
- **Rückrollen:** Schalter „Schutz aus“ zur Bootzeit (wie FreeMiNT `MEM_PROT=NO`, `FM/bootmenu.c:207-219` [S1]): alle `memspace` zeigen wieder auf die geteilten Tabellen — damit ist Stufe 3 zur Laufzeit abschaltbar, zusätzlich Revert.
- **Aufwand:** 10–15 Tage.

#### Stufe 3 wie gebaut (3a–3c, Stand 2026-10-05)

Quittungen: `TASK/ARCH03-3a-STATUS-DONE.MD`, `TASK/ARCH03-3b-STATUS-UNFINISHED.MD`,
`TASK/ARCH03-3c-STATUS-UNFINISHED.MD`; Belege `measurements/measurement-2026-10-05-arch03-3{a,b,c}/`.

- **3a — Kernelzugriff:** jede Stelle der Inventur (`docs/analysis/arch03-stage3-access-inventory.md`) läuft über `uaccess` (Aufruferrechte, `ldtr/sttr`, Fehlerabfang) oder prüft mit Aufruferrechten und liest über den Kernel-Alias (`kalias`); ein schlechter Zeiger beendet das Programm statt den Kernel.
- **3b — Rechte je Adressraum:** `tos-api/src/bdos/process/memprot.rs`. Rechte werden **berechnet, nicht fortgeschrieben**: Änderungen markieren Bereiche/Slots als schmutzig, `sync()` (Ende jeder mutierenden BDOS-Operation, vor jedem `install_ttbr0`, Ende jedes Traps) rechnet die 2-MiB-Regionen aus Pool-Listen, Besitzern (`m_own` + weitere Besitzer), Blocksätzen (Modus, E5-Erbe) und E10-Freigaben neu. Geteilte L3 = Nicht-Besitzer-Rechte, private L3 nur wo ein Slot abweicht (Copy-on-differ, §3.2); jeder Slot hat eigene L2 für alle vier GiB. Tabellen aus einem Vorrat, den der Kernel beim Booten vor den Pools nimmt. Seitenraster (E16) nur bei Schutz an. Bootschalter `memprot=off` (= 3a) / `memprot=strict` (E5 strikt). Schutzverletzung → EmuTOS-Absturzbericht + Zeile `Memory violation: …` (E11, kein Alert). Keine ASIDs (Stufe 4).
  - **MEMPROT-FAST (2026-10-07, Anwenderentscheidung: Fix-Optionen 1 und 3 aus `docs/analysis/zview-sluggish-2026-10-07.md` §1 — nur ändern, wo sich Rechte ändern; sammeln je Trap; 2-MiB-Blockdeskriptoren (dort Option 2) nicht):** die Invariante „berechnet, nicht fortgeschrieben“ bleibt, aber eine Änderung markiert nur noch, **wo und für wen** sich Deskriptoren ändern können (FreeMiNT-Abkürzung `FM/arch/mprot040.c:1102-1130`): gleicher oder gleichwertiger Modus (`PROT_P`↔`PROT_S`) → nichts; Nicht-Besitzer-Muster nur bei Modi mit Fremdzugriff (`PROT_G`/`PROT_PR`); Besitzerwechsel nur die Slots, die den Block über den hinzugekommenen/weggefallenen Besitzer erreichen (`on_owner_change_by`); Freigabe/`Mshrink` nur der zurückgegebene Teil, gesehen von den Slots mit privater Kopie; Strukturänderungen (Wurzel gebaut/freigegeben, `Pexec`-4-Raum `set_space`, E10) als Differenz je Block vorher/nachher, Vollneubau nur für eine frische Wurzel. Zusammengesetzte Operationen (`Pexec`-Laden/-Start, `Pterm`-Kehraus, ausgerichtete Allokation) laufen in einem `memprot::batch()` — ein Sync am Ende, `install_ttbr0` und Trap-Ende synchronisieren immer (`sync_now`). Prüfmodus `memprot=verify` (Hosttests: `set_verify`) vergleicht nach jedem Sync jede Poolseite der geteilten Tabellen und jeder privaten Kopie mit einer unabhängigen Vollberechnung (Algorithmus 3b); unangekündigte Strukturänderungen fallen auf Vollneubau zurück und werden gezählt. Lauf: `measurements/measurement-2026-10-07-memprot-fast/`.
- **3c — Abschluss:**
  - **E10 (a) verdrahtet, in allen Modi:** `aes/object.rs` `ob_user` führt die `G_USERDEF`-Routine eines **fremden** Baumeigentümers im laufenden Prozess aus wie EmuTOS (`aes/gemoblib.c:411-426` `call_usercode` = `jsr` auf dem laufenden Stapel, aus `ob_user` `:433-448`, `just_draw` `:792-793`); bei Schutz an mit `memprot::grant_temp` vor und `revoke_temp` nach dem Callout (FreeMiNT `prot_temp`, `FM/arch/mprot030.c:775-805`). Die A97-Zurückstellung ist aufgehoben (sie bestand, weil vor Stufe 1 die fremde VA im laufenden Prozess nichts bedeutete). Kein Callout nur ohne EL0-Prozess (Kernel-Aufrufer). Sonde `sdk/examples/e10usr.c` + `e10kid.c`.
  - **Lader-Reihenfolge wie EmuTOS:** Umgebung vor TPA (`ET/bdos/proc.c:281-299`); beide als Aufrufer angelegt, bei `PE_LOADGO` dem Kind übergeben (`:302-307`).
  - **E6 vs. strict:** `memprot=strict` betrifft nur Kinder mit eigenem Raum (`Pexec` 0/6); ein `Pexec`-4-Kind teilt den Raum des Aufrufers in jedem Modus (dokumentiert in `4b-pexec.md`, `COMPATIBILITY.md`).
  - **`F_KEEP`** (`0x4000`) nach FreeMiNT (`FM/dosmem.c:131-135,161-165,365-398`, `FM/proc_help.c:242-258`): Block überlebt den letzten Besitzer, geht an den Kernel (= `rootproc`), danach darf jeder ihn freigeben; Modus unverändert; in jedem Schutzmodus.
  - **Tabellenvorrat erschöpft:** **Abweichung von Frage 14** („Erschöpfung → `ENSMEM` der auslösenden Allokation“): gebaut ist „der Prozess behält dort die Nicht-Besitzer-Rechte (verliert Zugriff, gewinnt nie welchen)“, gezählt (`table_shortfalls`, `table_shortfalls_of`) und einmal je Prozess seriell gemeldet (`[memprot] page-table arena exhausted …`) — Leitvorgabe 2026-10-05.
- **E9 (nach 3c, 2026-10-05, Leitentscheidung):** die TPA ist der größte freie Block wie EmuTOS `alloc_tpa` (`ET/bdos/proc.c:460-502`, Poolwahl mit Bits-31-28-Gleichstand, `p_hitpa = p + max` `:374`), `TPA_SIZE` entfällt (`NT/arena.rs` `alloc_tpa(flags, needed)`, `choose_pool`; `traps::mxalloc_largest`); Nullen nach `kpgmld` (`PF_FASTLOAD`: nur das Bild, `ET/bdos/kpgmld.c:212-221`); SDK-Start = mintlib `_crtinit`/`_acc_main` (`sdk/libc/newtos-arch/heap_init.c`, `sdk/crt0.S`: `Mshrink(PD, Programm + Stapel)`, malloc über GEMDOS `Malloc`); Accessory-Lader schrumpft wie `pgmld` (`ET/util/gemdos.c:29-47`, `kernel/src/accs.rs`); Tabellenvorrat + 2 × Poolregionen für zwei noch nicht geschrumpfte TPAs (`kernel/src/lib.rs`, `ARENA_MAX_TABLES` 8192).
- **Offen nach 3c:** S2ALL/TSRPAR-Prüfung „TPA beyond blkln back in the pool“ unter Schutz an/strict (Ursache E9 + E16, nicht die Lader-Reihenfolge — siehe 3c-Quittung); FreeMiNT-Modi 100+ (2b), `Mvalidate`/`Maccess`, `F_OS_SPECIAL` (nicht gebaut), Pi-Lauf.

### Stufe 4 — ASIDs und TLB-Feinpflege (optional, Hardware)

- **Module:** `NT/mmu.rs` (`nG`, ASID-Zuteilung), `KA/boot.rs` (`TCR_EL1.AS` je nach `ASIDBits`), Wechselpfade.
- **Invarianten:** QEMU-Pfad unverändert (Flag aus); auf der Pi kein TLBI beim Wechsel.
- **Tests:** Pi-Messung Wechselkosten vorher/nachher (`hz_200`-basierter Zähler), Stresstest viele Prozesswechsel; QEMU unverändert grün.
- **Aufwand:** 3–5 Tage. **Rückrollen:** Flag aus.

#### Stufe 4 wie gebaut (Stand 2026-10-06)

Quittung `TASK/ARCH03-4-STATUS-*.MD`; Belege `measurements/measurement-2026-10-06-arch03-4/`.

- **Schalter:** Bootargument `asid=on` (Standard **aus**); `asid=tiny` = an mit nur 3 ASIDs je Generation (Prüfhaken für den Überlauf). Aus: der bisherige Pfad unverändert (ASID 0, `restore_boot_ttbr0` mit `tlbi vmalle1` bei jedem Wechsel, Blätter ohne `nG`, `TCR_EL1` wie gebootet).
- **Kern (`kernel/src/lib.rs`, `kernel/src/arch/aarch64/mmu.rs` `size_asids`):** liest `ID_AA64MMFR0_EL1.ASIDBits` ([7:4]); bei `0b0010` setzt er `TCR_EL1.AS` (16 Bit) mit `msr; isb; tlbi vmalle1; dsb ish; isb` — `AS` darf im TLB gecacht sein (D24.2.193), danach Invalidierung nach `RVNRFW` (D8-7858). QEMU `raspi4b`/`virt` melden `0x2` (wie TRM Table 4-35) → 65 535 ASIDs je Generation.
- **Zuteilung (`tos-api/src/bdos/process/asid.rs`):** ASID je **Belegung** eines Slots (nicht je Slot): `init_slot_tables` (neuer Mieter) und das Freigeben eines Slots ziehen die ASID ein (`TLBI ASIDE1IS` als Aufräumen; die Korrektheit kommt daher, dass eine eingezogene Nummer in derselben Generation nie neu vergeben wird). Nummern aufsteigend, ASID 0 = leere Wurzel. Erschöpft → neue Generation: `TTBR0` auf die leere Wurzel mit ASID 0, `tlbi vmalle1is; dsb ish; isb`, dann die neue Wurzel — so kann zwischen Flush und Wechsel kein Eintrag einer veralteten ASID nachgeladen werden. `ASIDE1` nicht für den Überlauf (Wortlaut `RFKSVM` D8-7868 „final lookup level“, V0a C6).
- **Wechsel (`mmu::install_ttbr0`/`switch_ttbr0_kernel` → `install_tagged`):** `dsb ish; msr ttbr0_el1, L0|ASID<<48; isb`, **kein TLBI** (Kernel-Einträge global, Example D8-1, V0a B8); `restore_boot_ttbr0` setzt nur die leere Wurzel (ASID 0). `handle_pexec` speichert `parent_l0_pa` ohne die ASID-Bits. Die QUIRK-001-Druckzeilen bleiben (Diagnose; Entfernen ist ein eigener Schritt nach der Pi).
- **`nG = 1`** auf jedem Anwenderblatt (`PageFlags::descriptor_bits`), nur bei `asid=on`.
- **Feinpflege (`mmu::set_leaf`/`flush_tlb`):** Verengung eines Blatts → `TLBI VAAE1IS` (geteilte Tabelle, alle ASIDs) bzw. `TLBI VAE1IS` mit der ASID des Besitzers (private Tabelle); gesammelt, Duplikate einmal, > 64 → `vmalle1is`; ein Slot ohne aktuelle ASID schuldet nichts. BBM eines Slot-L2-Eintrags (`make_private`/`drop_private`) → `TLBI VAE1IS` für die Region mit der Slot-ASID (alle Ebenen, `RVMSWQ`); `drop_private` schuldet zusätzlich jede Seite, in der die private Kopie von der geteilten abwich. Nicht `…L` (`RDRCZZ`).
- **QEMU-Befund (gemessen, Ursache [Schluss]):** QEMU 11.0.2 hält Einträge einer **inaktiven** ASID nicht beobachtbar — ein absichtlich kaputter Kern ohne die Feinpflege besteht den Inaktiv-ASID-Fall (ASIDSTR Teil C) trotzdem, nur der Fall derselben ASID (MPROT `f`) fällt durch; und ein ASID-Wechsel kostet dort so viel wie ein TLBI (Wechselkosten mit `asid=on` auf QEMU **höher**: Installation 1,7 → 5,8 µs bei Schutz an). Das Behalten über ASIDs und der Nutzen sind nur auf der Pi prüfbar.
- **Offen:** Pi-Lauf (`asid=on` und `asid=tiny`: ASIDSTR/ASIDPERF/MPROT/S2ALL/Desktop/TeraDesk/QED, Wechselkosten mit dem Zählerkern); danach Standard umstellen und die QUIRK-001-Klammer/Invariante I9 streichen.

### Erweiterungen nach Stufe 3 (eigene Tickets)

| Erweiterung | Inhalt | Aufwand [Schätzung] |
|---|---|---|
| SLB | `Slbopen/Slbclose/Slbexec`, `Pexec 106`, Freigabe der Bibliotheksregionen `PROT_G` für den Aufrufer (`FM/slb.c:330-401,581-629` [S1]), Aufruf-Trampolin in Seite 0 statt `user_things` (`FM/arch/user_things.S:99-160` [S1]) | 4–6 Tage + Spec |
| `kbdvec` global | E4, Abschnitt 3.12 | 1–2 Tage + Spec-Nachtrag KEYS02 |
| `Mvalidate`/`Maccess` | `FM/dosmem.c:458-540` [S1] | 1 Tag + Spec |
| `shm` | braucht `u:`-Pseudolaufwerk | eigenes Projekt |
| `Psetlimit`/`maxmem` | `FM/memory.c:1858-1875`, `FM/dosmem.c:80-107` [S1] | 1–2 Tage |

### Gesamtaufwand [Schätzung]

| Stufe | Tage |
|---|---|
| 0a Track B | 1 |
| 0b Spike | 2–3 |
| 0c Abbruch statt Halt | 2–3 |
| 0d PIE + Neubau | 4–6 |
| 1 Gemeinsamer Raum | 8–12 |
| 2 Pexec 5/6/7, Ptermres, Regionen | 5–7 |
| 2b FreeMiNT-Modi | 3–4 |
| 3 Rechte | 10–15 |
| **Summe 0–3** | **35–51** (+ Puffer ≈ 10 % → rund 38–56) |
| 4 ASIDs (optional) | 3–5 |

---

## 5. Was nach jeder Stufe möglich wird

| Nach Stufe | Neu möglich | Tickets |
|---|---|---|
| 0c | Programmabsturz beendet nur das Programm; ungültige Puffer liefern Fehler statt Halt | (Robustheit, OFAM-Nachlauf) |
| 0d | PIE-Toolchain; Relokationslader (auch Vorlage für MOD01-Kernmodule, `zview-ldg` §4.3) | — |
| 1 | **MEM03** (Umgebung lesbar) → **PORT09** entblockt; echte PD-Zeiger, `p_parent`; Shell-Idiom `Pexec 3`→`Mshrink`→`Pexec 4`; `Malloc(-1)` wieder EmuTOS-ehrlich; keine Fenstererschöpfung (zView-Bildfolge, `zview-ldg` L5); fremde Bäume zeichenbar (Lesen); `Getmpb`-MDs direkt benutzbar | MEM03, PORT09 |
| 2 | **PROC01** (`Pexec 5/6/7`, mintlib `system()` ohne Overlay); **LDG unverändert** (`ldg_load` über `Pexec 5/3/4`) → zView-Weg „Option A“ ohne Userspace-Lader; **TSR** (`Ptermres` mit `blkln`, erreichbarer residenter Code); globaler `Mxalloc` angenommen (Modus gespeichert) | PROC01, PORT02/zView, LDG |
| 2b | FreeMiNT-Programme mit `Pexec 100/104/106/200`; Voraussetzung für SLB | (SLB) |
| 3 | Prozessschutz wie FreeMiNT; `PROT_G`/`PROT_PR` wirksam; Meldungen „MEMORY VIOLATION“; Accessories und Apps können sich nicht mehr gegenseitig beschädigen | — |
| 4 | schnellere Prozesswechsel auf der Pi; Grundlage Mehrkern | — |
| Erweiterungen | SLB (th-otto-zView mit 100+ Codecs, `zview-ldg` §1.4), TSR-`kbdvec`-Haken, `shm` | — |

**PROC02** (Handle-Vererbung) ist vom Adressraum unabhängig und kann jederzeit kommen.

---

## 6. Offene Entscheidungen für den Anwender (mit Empfehlung)

| Nr. | Frage | Empfehlung | Begründung kurz |
|---|---|---|---|
| E1 | VA = PA oder feste Verschiebung? | **VA = PA** | `Malloc` = `m_start`, `Getmpb` direkt nutzbar, ST-RAM < 16 MiB; Verschiebung bringt nichts (Frage 2) |
| E2 | Erst gemeinsamer Raum ohne Schutz, dann Rechte? | **Ja, gestuft** | FreeMiNT trennt beides selbst; jede Stufe auslieferbar |
| E3 | Seite 0 je Prozess behalten oder global machen? | **Je Prozess behalten** | B1/KEYS02/Trampolin-Scratch bleiben unverändert; einzige benannte Ausnahme |
| E4 | `kbdvec`: KEYS02 D1 (je Prozess) behalten? | **Behalten bis nach Stufe 3**, dann global mit automatischem Aushängen, wenn ein TSR-Fall vorliegt | frisch abgenommen; kein heutiger Anwendungsfall |
| E5 | Kind → Eltern: nicht strikt (FreeMiNT-Standard) oder strikt? | **Nicht strikt**, Schalter für strikt | FreeMiNT-Erfahrung, Kompatibilität |
| E6 | `Pexec 4`: Thread im `memspace` des Aufrufers, Besitz EmuTOS? | **Ja (Mischform)** | LDG braucht beides (Schreiben in Aufruferstapel; Modul-Allokationen frei bei Modulende) |
| E7 | Alte `ET_EXEC`-Programme weiter laden? | **Nein, harter Schnitt nach Stufe 1** (Übergang nur in 0d) | alle Programme stammen aus dem Repo; ein Altpfad wäre ein zweiter Lader |
| E8 | Relokation im Kernel oder im `crt0`? | **Kernel** | EmuTOS/FreeMiNT-treu; `Pexec(3)` liefert ein fertiges Bild |
| E8b | PRGFLAGS als ELF-Notiz `"NewTOS"`/Typ 1? | **Ja** (Format ist unsere Festlegung) | Pexec-7 und Speicherschutz-Standard brauchen eine Quelle |
| E9 | TPA-Größe: fest (4 MiB, übersteuerbar) oder EmuTOS „größter Block“? | ~~**Fest + übersteuerbar**~~ → **Leitentscheidung 2026-10-05: EmuTOS „größter Block“** (`alloc_tpa`), Programme schrumpfen selbst (mintlib `_crtinit`), Accessories der Lader (`pgmld`) | ursprünglich: Mehrprozessbetrieb; FreeMiNT begrenzt ebenso (`initialmem`). Umgesetzt: `TASK/ARCH03-E9-STATUS-*.MD` |
| E10 | Fremder Userdef ab Stufe 3: zeitweise Freigabe (a) oder Verschieben zum Eigentümer (b)? | **(a)** | erhält EmuTOS' Zeichenverhalten; FreeMiNT-eigenes Mittel (`prot_temp`, SLB) |
| E11 | Schutzverletzung: Meldung + `Pterm(-1)` (EmuTOS) jetzt, `SIGBUS`-Handler (FreeMiNT) später? | **Ja**; Alerttext nach FreeMiNT „MEMORY VIOLATION“ | EmuTOS-treu, Upcall-Weg existiert für später |
| E12 | ASIDs | **Optional, nur Hardware, Stufe 4**; TLBI-Pfad bleibt Standard | QUIRK-001; Modell braucht sie nicht. **0b:** QUIRK-001-Teil entfällt auf QEMU 11.0.2 (ASIDs laufen dort, S0b §4); „nur Hardware“ ist damit keine Notwendigkeit mehr **4 (2026-10-06):** gebaut hinter `asid=on`, Standard aus bis zum Pi-Lauf (Abschnitt „Stufe 4 wie gebaut“) |
| E13 | EL0-Cache-Pflege (`SCTLR_EL1.UCI`) freigeben? | **Nein, vorerst**; Kernel synchronisiert beim Laden; Systemaufruf erst, wenn ein Programm zur Laufzeit Code erzeugt | kein heutiger Bedarf; **0a:** Bitbedeutung belegt (`UCI` Bit 26: bei 0 trappen `DC CVAU/CIVAC/CVAC` und `IC IVAU` an EL0 mit EC `0x18`; bei 1 braucht EL0 Leserecht auf die Adresse — ARM ARM D24-9704, D7-7659; TRM S. 4-126; A72 `CTR_EL0.IDC/DIC` = 0 ⇒ Wartung nach Codeschreiben bleibt nötig, TRM S. 4-122) |
| E14 | `Pfork` | **Nicht umsetzen** (`EINVFN`) | keine TOS-Pflicht, widerspricht VA = PA |
| E15 | `Mfree` nur durch Besitzer (FreeMiNT) statt jeder (EmuTOS)? | **FreeMiNT-Regel ab Stufe 2** | Mehrprozessbetrieb; korrekte Programme unberührt |
| E16 | Pool-Rundung auf 4 KiB ab Stufe 3 (beobachtbar)? | **Ja**, als sanktionierte FreeMiNT-Abweichung in `COMPATIBILITY.md` | ohne Rundung kein seitengenauer Schutz |
| E17 | `shm`/`u:`-Laufwerk | **Eigenes Ticket, nicht ARCH03** | braucht ein Pseudodateisystem |
| E18 | Rückfall, falls Spike 0b eine echte Aliasing-Wand findet | **Abbildung bei Allokation** (4-KiB, frische VAs) | bewährtes `usermem`-Muster. **0b:** auf QEMU keine Wand (S0b §3) — Rückfall nur, wenn die Pi widerspricht |

---

## 7. Abgleich mit den Vorgänger-Analysen (vom Schritt-1-Agenten nicht gelesen)

1. **`docs/analysis/coop-kernel/freemint-multitasking-model.md` §5.2/§7.1** stellt NewTOS' per-Prozess-`TTBR0` als „architektonisch das, was … FreeMiNTs MMU-Zweig“ ist dar (`:424-430`, `:523-530`). **Teilweise falsch:** FreeMiNT wechselt die Tabelle, aber über **derselben** Abbildung (`FM/arch/mprot030.c:18,76-80` [gelesen]); NewTOS wechselt die **Abbildung**. Genau dieser Unterschied ist ARCH03. Ebenso überholt: „kein Upcall-Mechanismus“ (`:549-556`) — `deliver_upcall`/`Psigreturn` existieren (`KA/exceptions.rs:1395-1580` [gelesen]). Seine Beobachtung, dass XaAES im Aufruferkontext läuft und per `sleep`/`wake` blockiert (`:447-482`), ist richtig und hier in Frage 9 vertieft.
2. **`docs/analysis/emutos-memory-model.md` §4.4a** nennt das Fenster je Prozess die „ISA-appropriate generalisation“ von FreeMiNTs Mechanismus und hält gemeinsamen Speicher „at (not necessarily identical) VAs“ für ausreichend (§4.6). **Widerlegt** durch LDG (Zeiger als Text in der Umgebung, Schreiben in den Aufruferstapel) und AES-/Cookie-Zeiger, die dieselbe Adresse brauchen. Dieselbe Analyse nennt aber Identität als **v2-Ziel**, sobald ein relozierender Lader existiert (§4.6, „identity-map … the maximally faithful end-state“) — ARCH03 ist dieses v2. Die gleiche Begründung steht in ADR-021 (`docs/MEMORY.md:1428-1436`): sie wird durch ADR-025 ersetzt.
3. **ADR-010** (`docs/MEMORY.md:38-182`): Entscheidung wird übernommen; die Schlussfolgerung „kann auf diesem QEMU nicht entwickelt werden“ (`:168-176`) ist durch den laufenden Aliasing-Betrieb in Frage gestellt (Frage 2) und wird mit Spike 0b überprüft.
4. **`docs/analysis/zview-ldg-2026-10-04.md` §4.2 „Verworfen“** (Kernel-`Pexec(3)` umbauen sei mehr Arbeit und riskanter als ein Userspace-Lader): durch die Anwenderentscheidung überholt. Mit Stufe 2 läuft das Original-`ldg_load`; `ldg_cpush` (m68k-Asm, `ldgutil.c:205-225`) wird im Rahmen der Portierregel retargetiert (Leeroperation). Die Frage „Modul-Endung `.ldg` vs. eigene“ (dort §6 Nr. 2) bleibt offen und ist unabhängig von ARCH03.
5. **S1-Randbeobachtungen:** `NT/mmu.rs:140-142` nennt „384 KiB bei `USER_L3_COUNT = 3`“, `:66-70` „1216 KiB“ (bestätigt [gelesen]) — entfällt mit Stufe 1. `p_parent = 0` — in Stufe 1 behoben.
6. **Neu gefunden (nicht in S1):** `usermem::free` lässt die Abbildung eines freigegebenen Blocks stehen (`NT/usermem.rs:596-604` [gelesen]). Wird der Poolblock danach an einen anderen Prozess vergeben, kann der alte Prozess über seine Fensteradresse weiter hineinschreiben — eine bestehende Isolationslücke des heutigen Modells, die Stufe 3 (I8) schließt. Ebenso: `tos-types/src/memory.rs:155-168` verwechselt die `Mxalloc`- und die PRGFLAGS-Kodierung (Frage 8).

---

## 8. Nicht verifiziert / Vorbehalte

- **0a: erledigt** — die AArch64-Architekturaussagen (Deskriptorbits, ASID-Breite/`TCR_EL1.AS`, `nG`, `UCI`, PAN beim A72, TLBI-Varianten und Barrieren, Break-before-make, DFSC/IFSC-Kodierungen, `LDTR/STTR`) sind in V0a gegen ARM ARM M.d und A72-TRM belegt oder korrigiert (Änderungen oben mit „0a:“). Ursprünglich: **[Track B offen]** → Stufe 0a. Bitpositionen stammen aus dem Repo-Code (`NT/mmu.rs:91-111`), dessen Quelle laut S1 ein Linux-Querverweis ist.
- **0a:** `R_AARCH64_RELATIVE` = 1027 und Berechnung aus der AAELF64 belegt. **Weiter nicht belegt:** `Elf64_Rela`-Aufbau und `DT_*`-Tags (AAELF64 verweist auf SYSVABI64/gABI, nicht gepinnt).
- **Wand 3** nur logisch widerlegt (laufendes Aliasing), nicht gemessen → Spike 0b. **0a:** zusätzlich architektonisch nicht begründbar (B2.11); verbleibt als QEMU-Frage. **0b:** auf QEMU 11.0.2 gemessen und nicht vorhanden (S0b §3); Pi offen.
- **Bildschirmspeicher-PA < 2 GiB auf der Pi** gefolgert, nicht gemessen. **0b:** auf QEMU `raspi4b` gemessen (`0x3C10_0000`), Pi offen (S0b §6).
- **XaAES:** `handler.c` selbst nicht erneut gelesen (Beleg aus `freemint-multitasking-model.md:449-463`); `check_queued_events` nur am Kopf gelesen (`XA/xa_evnt.c:321-333`); Menüzeichnen fremder Clients in XaAES nicht nachverfolgt.
- **FreeMiNT `keyboard.c:1524`, `slb.c`, `shmfs.c`, `mprot.x`, `k_fork.c`, `proc_help.c`** nur über S1 zitiert.
- **Aufwände** sind Schätzungen ohne Messgrundlage; die Spannen setzen serielle Arbeit voraus (Merkregel „ein Task pro Agent“).
- **Clang/lld-PIE-Verhalten** (GOT-Relaxierung, Relocs für schwache Symbole, `-z text`) nicht ausprobiert → erster Schritt von 0d.
- **Ob mintlib auf NewTOS den MiNT-Zweig (`Pvfork`) oder den TOS-Zweig (`tfork`) nimmt**, nicht geprüft; PROC01 nennt den TOS-Zweig.
