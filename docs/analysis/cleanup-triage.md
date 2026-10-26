# Cleanup-Triage: STATUS.md, Projekt-Memory, CLAUDE.md

Stand 2026-10-26, NewTOS `main` @ `ea2416c6`. **Nur Triage, nichts geändert.**
Jede Aktion unten ist ein Vorschlag und wartet auf Freigabe.

## Umfang und Methode

Geprüft wurden vier Quellen:

- `CLAUDE.md`
- `docs/STATUS.md` (1055 Zeilen, 1,2 MB, davon 526 Zeilen *Recent Decisions*)
- `docs/MEMORY.md` (ADRs, Quirks, Lessons, Rejected)
- das Auto-Memory `~/.claude/projects/-Volumes-Daten-Development-PiRusTOS/memory/` (90 Memories und der Index)

Sieben Agenten haben nacheinander je einen Abschnitt bearbeitet, alle mit derselben Vorgabe. Jeder Eintrag bekam vier Prüfungen:

1. Zitate lösen auf.
2. Kein Widerspruch zu `docs/analysis/ORACLE.md`, `measurements/` und dem Code.
3. Verhaltensaussagen haben einen Beleg.
4. Dubletten und Ablösungen sind erkannt.

Zitate prüfte zuerst ein mechanischer Prüfer. Er löst Pfade, `file:line`, Markdown-Links und Commit-Hashes auf. Alte Hashes übersetzt er über `docs/commit-map.txt` (Umschreibung vom 2026-10-19), Hashes aus EmuTOS und FreeMiNT schlägt er in den Referenz-Repos nach. Seine Treffer haben die Agenten einzeln beurteilt; reines Rauschen ist nicht als Befund geführt. Ich habe Stichproben gegen Code und Dateien gemacht und die Zählungen aus den Tabellen nachgezählt.

**Grenzen der Prüfung.** Es wurde nur gelesen, nicht gemessen: kein QEMU, kein Hatari, keine Pi. „Widerspricht dem Code“ heißt also: widerspricht dem Quelltext. EmuTOS-Zitate wurden gegen den heutigen Checkout auf dem Oracle-Pin `1a7484ab` geprüft, und zwar nur bei Einträgen mit `keep`.

Klassen: `fact` · `decision` · `trap` · `hypothesis` · `obsolete`.
Aktionen: `keep` · `fix` · `delete` (nur wenn der Inhalt vollständig anderswo steht) · `archive` (Geschichte wandert in eine Archivdatei).

## Zählung

| Quelle | keep | fix | delete | archive | Summe |
|---|---|---|---|---|---|
| CLAUDE.md | 59 | 26 | 0 | 0 | 85 |
| STATUS.md Kopf (1–461) | 9 | 27 | 3 | 27 | 66 |
| STATUS.md Log 466–597 | 0 | 1 | 0 | 131 | 132 |
| STATUS.md Log 598–729 | 0 | 0 | 0 | 132 | 132 |
| STATUS.md Log 730–861 | 0 | 0 | 0 | 132 | 132 |
| STATUS.md Log 862–Ende + Notes | 1 | 0 | 0 | 130 | 131 |
| docs/MEMORY.md | 14 | 22 | 1 | 10 | 47 |
| Auto-Memory | 53 | 24 | 4 | 10 | 91 |
| **Gesamt** | **136** | **100** | **8** | **572** | **816** |

Klassen gesamt: fact=89 · decision=226 · trap=60 · hypothesis=1 · obsolete=440

## Querschnittsbefunde

1. **Das Log in STATUS.md ist Geschichte.** 525 der 526 Zeilen in *Recent Decisions* sind abgeschlossene Arbeit, jede mit DONE-Beleg im Ticket. Ihr dauerhaftes Wissen steht schon in ADRs, `COMPATIBILITY.md`, `docs/api/` oder im Auto-Memory. Ausnahmen: die Liste unter „Vor dem Archivieren umziehen“ und SD1-001.
   Die Sortierung bricht bei Zeile 862. Darüber steht das Neueste oben, ab dort das Älteste oben. Die Zeilen 1047 und 1048 (September) hängen am Ende. Die Archivdatei sollte nach Datum sortiert werden.
2. **Der Kopf von STATUS.md ist veraltet.**
   - Die Basislinien stimmen nicht mehr: Host-Tests 1380 gegen heute 1664, Clippy 76/125 gegen 74/121, SDK 20 gegen 26.
   - Genannt ist Tag `v0.5`, obwohl es `v0.6` und `v0.6.1` gibt.
   - Dort steht „P1: keins“, aber `TASK/USB01.md` ist P1 und offen.
   - Die Phasentabelle ist zu einem zweiten Log geworden (Zeile 13 hat 77 KB) und zeigt den Desktop noch als Boot-Anwendung.
   - Die meisten *Open ADRs* und *Blocked*-Punkte sind erledigt (SH-Teil).
3. **Der Desktop ist umgezogen, die Zitate nicht.** Seit DESK43 U8b (`90b04444`, ADR-026) liegt `kernel/src/desk/` unter `programs/desktop/`. Den alten Pfad nennen noch:
   - `CLAUDE.md` (Quellbaum; dort fehlt `programs/` ganz)
   - etwa 30 Zitate in `docs/MEMORY.md`
   - sieben Memories und Index-Zeile 30
   - viele Log-Zeilen (dort als Geschichte unkritisch)
4. **ARCH03 hat kein ADR, und ADR-025 ist doppelt vergeben.** Der gemeinsame Adressraum (VA = PA) ist umgesetzt, steht aber nur als Entwurf „ADR-025 Proposed“ in `docs/analysis/arch03-implementation-plan.md:28`. In `docs/MEMORY.md:2131` ist ADR-025 die MD-Versorgung (MEM05). Die Folgen:
   - ADR-010 sagt „NOT YET IMPLEMENTED“.
   - ADR-021 (Fenster je Prozess, abgelöst) steht auf „Accepted“.
   - ADR-023 beschreibt einen veralteten Mechanismus.
   - Fünf Memories setzen ILP32-Apps oder das ADR-021-Fenster voraus, obwohl der Loader ELF32 abweist (`tos-api/src/bdos/process/loader.rs:409-426`).
   Erst muss ARCH03 eine freie Nummer bekommen (etwa ADR-028), danach können ADR-010 und ADR-021 als abgelöst markiert werden.
5. **QUIRK-002 ist widerlegt, steht aber noch als gültig da.** INTID 153 wird auf echter Pi zugestellt. Die Korrektur in `docs/MEMORY.md` sagt nur „unbewiesen“, und die beschriebene Auflösung (`pl011_rx_poll`, UART2-IKBD) gibt es nicht mehr. Der Slug des passenden Memories sagt das Gegenteil seines Inhalts.
6. **CLAUDE.md beschreibt die heutige Arbeitsweise nicht.**
   - Der Tech-Stack nennt `cortex-a` und `tock-registers`, die keine Abhängigkeiten sind.
   - Als Grund für Nightly nennt er naked functions; heute ist es `per-package-target`.
   - `cargo xtask qemu-test` gibt es nicht.
   - Phasen-Deliverable-IDs stehen in keinem der letzten 300 Commits.
   - Es fehlen Tickets als Arbeitseinheit, das Oracle (`ORACLE.md`) und FreeMiNT als Quelle.
   - Das Verbot von C, libc und POSIX widerspricht dem SDK (ADR-019) und sollte auf Kernel und tos-api begrenzt werden.
7. **Das Auto-Memory widerspricht sich selbst.**
   - 13 Index-Hooks widersprechen ihrem Body. Am gröbsten: der Hook zu `atari-meeting-demo-october` sagt „sichtbare Fehler vor Konformität“, der Body nimmt genau das zurück.
   - Fünf Memories verdoppeln ein ADR (022, 023, 024, 027, 018).
   - `docs/MEMORY.md` verlinkt 17 Memory-Slugs. Diese Links zeigen aus dem Repo in einen privaten Ordner, und einige zeigen auf Löschkandidaten.
8. **Die ADR-Nummern sind lückenhaft.**
   - ADR-001 bis 009 wurden nie geschrieben, und zwei Reservierungslisten widersprechen sich (`docs/MEMORY.md:448-457`, `docs/ARCHITECTURE.md:155-161`).
   - `docs/api/desktop/_overview.md:4` zitiert ein ADR-003, das es nicht gibt.
   - ADR-Supexec ist angekündigt, fehlt aber.
   - Die Hausregel „ADRs werden nie geändert“ hält die Praxis nicht ein: sieben ADRs wurden an Ort und Stelle ergänzt, ohne dass ihre Statuszeile nachgezogen wurde.
9. **Commit-Hashes**
   - Die meisten alten Hashes lösen über `docs/commit-map.txt` auf; das ist die dokumentierte Konvention.
   - Nicht auflösbar sind 26 Hashes aus dem Juni und Juli (STATUS ab Zeile 928). Sie stammen aus einem früheren Verlauf, den die Map nicht abdeckt. Für Archiv-Geschichte reicht dazu eine Notiz.
   - Weitere BAD-COMMIT-Treffer sind Image-Prüfsummen und Upstream-Pins.
   - Wirklich falsch: drei Hashes in ADR-014 (richtig sind `e8fa10b2`, `af37a4dc`, `e82cf6d3`) und `5d4805e` im Memory zur Altitude-Regel (richtig ist `1983f4a5`).
   - Die Konvention mit `tools/oldhash.sh` steht weder in `docs/MEMORY.md` noch in `CLAUDE.md`.
10. **STATUS.md läuft nicht mehr mit.** Für `cf4d9b33` (WF_OWNER-Tests grün) fehlt die Zeile. Das verletzt die STATUS-Regel aus CLAUDE.md.

## Vor dem Archivieren umziehen

Dieses Wissen steht heute nur in einer Archiv-Kandidatenzeile oder nur im Code. Es muss erst umziehen, sonst geht es mit der Archivierung verloren.

| Herkunft | Wissen | Ziel |
|---|---|---|
| STATUS:582, `arch03-implementation-plan.md:28` | ARCH03-Entscheidung: VA = PA, ein Adressraum | neues ADR in `docs/MEMORY.md` (freie Nummer); ADR-010/021 als abgelöst markieren |
| STATUS:516 | Alte Hashes über `tools/oldhash.sh` und `docs/commit-map.txt` übersetzen | `docs/MEMORY.md` (Lessons) oder `CLAUDE.md` |
| STATUS:586 | Hatari-Messung: TOS 3.06 hat `kbdvec` | `docs/api/xbios/22-kbdvbase.md:219` (dort heute „ABGELEITET“) |
| STATUS:613, `tos-api/src/bdos/fs/cache.rs:237` | Write-back-Abweichung | `docs/COMPATIBILITY.md` |
| STATUS:727 | Ausnahme für das schmale Layout | `docs/COMPATIBILITY.md:1060` |
| V17-STATUS-DONE.MD:227 | Falscher Kopfkommentar in `aes-m07-parameter.tsv` | `tosdump/cases/ERRATA.md` |
| STATUS:702, `thread.rs:95-100` | Guard-Page fehlt auf dem AES-Fiber-Stack | neues Ticket |
| STATUS:612 | BCM2835-Quelle fehlt; Track B ohne SD-Spezifikation | neues Ticket |
| STATUS:802 | Richtung kooperativer Kernel | ADR (heute nur `docs/analysis/coop-kernel/00-effort-comparison.md`) |
| STATUS:780 | EmuTOS-Inkonsistenz `wm_calc` gegen `w_bldactive` (`gemwmlib.c:597-601`, `:1829-1832`) | `docs/api/aes/window-graphics.md` |
| STATUS:755/785 | Tabellendirektiven `!reset` und `+` | `docs/TOS-REFERENCE-DUMPER.md` |
| `rust-toolchain.toml:3` | Verweist für die Nightly-Begründung auf STATUS.md, die dort fehlt | Begründung nach `CLAUDE.md` oder `rust-toolchain.toml` |
| STATUS:950 | Systemressourcen sind in den Kernel kompiliert | Nachtrag zu ADR-013 |
| STATUS:1025 | `#K`-Zeile in NEWDESK.INF und `C:\KEYBOARD.KBD` | `docs/COMPATIBILITY.md` |
| STATUS:1047, `shell_loop.rs:28,305` | Abweichung `DESKTOP_APP` | `docs/COMPATIBILITY.md` |
| 17 Memory-Links in `docs/MEMORY.md`, STATUS:818 | Verweise auf Lösch- und Archivkandidaten im Auto-Memory | erst auf die ADRs umbiegen |

## Nebenbefunde außerhalb der vier Quellen

Nicht Gegenstand der Triage. Hier nur notiert, nicht verfolgt.

- `docs/api/vdi/workstation/100-v_opnvwk.md:423` nennt `HANDLE_COUNT = 16`. Der Code hat 64 (`vwk.rs:22`), EmuTOS hat `NUM_VDI_HANDLES = 128` (steht nur in `docs/api/vdi/_discrepancies.md:22`). Dazu gibt es kein Ticket.
- `docs/api/desktop/deskmain.md:1358-1363` sagt „No build date“. Das ist seit DESK12 falsch.
- `tosdump/reference/FINDINGS-r2.md:202` führt R2-6 noch als „OPEN“.
- `tos-api/src/bdos/mem/traps.rs:660-667` sagt „Phase 06 replaces this“. Die Regel liegt längst in `gemdos/memory.rs:85-106`.
- Sversion `0x2000` → `0x3000` (`5051390e`) und solid drag → `drag_move` wurden im Code abgelöst, ohne STATUS-Zeile.
- `docs/REQUIREMENTS.md:77` sagt zur Lizenz „pending“. Sie ist GPLv2+ (`aec5b730`).

## Vorschlag für die Umsetzung (nach Freigabe)

1. **Sicherung zuerst:** den ganzen Auto-Memory-Ordner als datiertes Tarball in den Scratch kopieren, bevor eine Memory-Datei angefasst wird.
2. Alles aus „Vor dem Archivieren umziehen“ erledigen, zuerst das ARCH03-ADR.
3. Archive anlegen:
   - `docs/archive/status-decisions-2026.md`: alle `archive`-Zeilen aus STATUS.md, nach Datum sortiert, mit einer Notiz zu den nicht auflösbaren Juni/Juli-Hashes.
   - `docs/archive/status-history.md`: der „Historische Text“ aus STATUS.md.
   - `docs/archive/memory-adr-superseded.md`: abgelöste ADR-Abschnitte. Alternative: an Ort und Stelle lassen und nur die Statuszeile ändern; das widerspricht der Regel „nie ändern“ weniger.
   - Archivierte Auto-Memories in einen Unterordner `archive/` des Memory-Ordners, mit Index-Zeile entfernt.
4. Die `fix`-Zeilen abarbeiten.
5. Danach die STATUS-Zeile für diese Bereinigung schreiben.

Offene Entscheidungen für dich:

- Wohin mit Archiven: `docs/archive/` gibt es noch nicht, die Platzierungsregel in CLAUDE.md verlangt Rückfrage.
- Ob STATUS.md ganz aus Kopf und Log besteht oder das Log komplett auslagert.
- Ob abgelöste ADRs in MEMORY.md bleiben.

---

# Einträge je Quelle

## CLAUDE.md (Projektanweisungen)

Quelle: /Volumes/Daten/Development/PiRusTOS/CLAUDE.md Zeilen 1–294. Checker: OK=29, MISSING=6 (alles Rauschen: `kernel8.img` x2 = Build-Artefakt, `method/`/`serial/`/`screendumps/` = Muster fuer Laufverzeichnisse), AMBIGUOUS=1 (`RUNINFO.TXT`, Muster). Keine Commit-Cites.

### Systemische Befunde
- Der Quellbaum-Block (Stand 2026-10-09, `67d81dce`) kennt die DESK43-Umzuege nicht: `kernel/src/desk/` gibt es seit `90b04444` (2026-10-14) nicht mehr, der Desktop liegt in `programs/desktop/` + `programs/newdesk/`; das Wurzelverzeichnis `programs/` (eigener Workspace, `Cargo.toml` `exclude = ["programs"]`) fehlt im Baum ganz.
- Tech-Stack und Namensregeln beschreiben den Plan von Juni, nicht den Code: `cortex-a` und `tock-registers` sind keine Abhaengigkeit (einzige externe: `xhci` in `kernel/Cargo.toml`), `naked_function` braucht kein Nightly mehr (`#[unsafe(naked)]`, kein `#![feature]` im Baum), `BCM2711_*`-Namen nur in `pm.rs`, `TRAP_*`-Konstanten nur in `tos-api/src/aes/shell.rs:111`.
- Die Arbeitsweise ist von Phasen auf Tickets (`TASK/`) und das gepinnte Oracle (`docs/analysis/ORACLE.md`, `ea2416c6`) umgestellt; CLAUDE.md nennt weder Tickets als Arbeitseinheit noch das Oracle als Messreferenz, verlangt aber Phase-Deliverable-IDs in Commits (0 von 300 letzten Commits) und ein `cargo xtask qemu-test`, das es nicht gibt.
- Die "Do not suggest C / libc / POSIX"-Regeln widersprechen dem SDK (`sdk/libc` = mintlib, `sdk/cflib`, `sdk/gemlib`, ADR-019 in docs/MEMORY.md:1176) und der Auto-Memory `port-reuse-dont-rewrite`; sie gelten faktisch nur fuer Kernel/tos-api.
- Mehrere Prozessregeln stehen doppelt in docs/STATUS.md-Kopf und Auto-Memory (Scratch, STATUS-Lockstep, Release-Bau); CLAUDE.md sollte die kanonische Stelle sein.

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| CL-001 | CLAUDE.md:1-4 | Titel + Hinweis "gilt fuer alle Sub-Crates, sofern kein spezifischeres CLAUDE.md" | fact | `git ls-files` zeigt nur ein CLAUDE.md; Aussage stimmt | keep |
| CL-002 | CLAUDE.md:8-10 | NewTOS = Clean-Room-Reimplementierung der TOS-API fuer AArch64/RPi4 | fact | Workspace + `kernel/Cargo.toml` (forced-target aarch64-unknown-none-softfloat) bestaetigen | keep |
| CL-003 | CLAUDE.md:13 | Ziel: natives ARM64-OS, keine Emulation im Kern | decision | Konsistent mit Auto-Memory `altitude-syscall-not-hardware-emulation` | keep |
| CL-004 | CLAUDE.md:14 | Ziel: bit-genaue TOS-API (Trapnummern, ABI, Fehlercodes) | decision | Konsistent mit docs/COMPATIBILITY.md und tosdump/Oracle-Vergleich | keep |
| CL-005 | CLAUDE.md:15 | Ziel: Quellportabilitaet fuer C-Programme | decision | Umgesetzt im SDK (`sdk/`, docs/sdk/), ADR-019 docs/MEMORY.md:1176 | keep |
| CL-006 | CLAUDE.md:16 | Ziel: optionaler m68k-Userland-Emulator spaeter | decision | Gestuetzt durch Auto-Memory `own-extensions-separate-aarch64-from-m68k`; widerspricht docs/STATUS.md:309-312 ("m68k phases removed from scope") – dort korrigieren, nicht hier | keep |
| CL-007 | CLAUDE.md:19 | Nicht-Ziel: Atari-Hardware-Emulation | decision | Konsistent mit Auto-Memory `altitude-syscall-not-hardware-emulation` | keep |
| CL-008 | CLAUDE.md:20 | Nicht-Ziel: Linux, POSIX, "any non-TOS API" | decision | "any non-TOS API" ueberholt: FreeMiNT-`Ssystem` + NewTOS-Modi 2003–2022 (`tos-api/src/gemdos/ssystem.rs`), ADR-027 docs/MEMORY.md:2357, Roadmap 2.0 = FreeMiNT (Auto-Memory `version-roadmap-1-0-2-0`). Auf "TOS- und FreeMiNT-API" erweitern | fix |
| CL-009 | CLAUDE.md:21 | Nicht-Ziel: x86/x86_64 | decision | Keine x86-Targets im Workspace | keep |
| CL-010 | CLAUDE.md:23-26 | Primaere Spezquelle = EmuTOS-Quelltext, extrahiert in Phase 00 nach `docs/api/` | decision | Gilt, aber unvollstaendig: Verhalten wird seit 2026-10-26 gegen das gepinnte Oracle gemessen (docs/analysis/ORACLE.md, EmuTOS `1a7484ab`, `ea2416c6`); FreeMiNT ist laut docs/SOURCE-ANALYSIS-PROTOCOL.md:51 Quelle 2b. Beides hier nennen | fix |
| CL-011 | CLAUDE.md:28-29 | Sekundaerquellen: Compendium, Profibuch, Release Notes, ST Internals | fact | Keine davon liegt in `references/` (vorhanden: arm-arm, circle, datasheets, emutos, emutos-oracle, freemint, gabi, libshare, rpi4-boot, sd-spec, usb, xhci); references/README.md listet sie als "acquire separately". Hinweis ergaenzen, dass sie lokal fehlen | fix |
| CL-012 | CLAUDE.md:31-44 | Begleitdokumente, Lesereihenfolge; `phases/` "PHASE-00 … PHASE-12" | fact | Alle 7 Dateien/Verzeichnisse existieren; `phases/` reicht bis PHASE-16 (+ `_archive/`); docs/analysis/ORACLE.md fehlt in der Liste | fix |
| CL-013 | CLAUDE.md:50-62 | Regel 1: Phase 00 nur Spezextraktion; spaeter gegen `docs/api/` implementieren | decision | Phase-00-Spiegelstrich historisch (Phase 00 abgeschlossen 2026-05-02, docs/STATUS.md:316); "EmuTOS nicht erneut lesen" kollidiert mit Praxis (Auditor misst gegen Oracle, Auto-Memory `no-rationalizing-emutos-deviations`: EmuTOS file:line zitieren). Phase-00-Teil streichen, Oracle aufnehmen | fix |
| CL-014 | CLAUDE.md:64-65 | Compendium/Profibuch nur Querverweis | decision | Konsistent mit SOURCE-ANALYSIS-PROTOCOL | keep |
| CL-015 | CLAUDE.md:67-68 | Hardware: BCM2711-Datenblatt + ARM ARM primaer, Track B | decision | `references/datasheets/RP-008248-DS-1-bcm2711-peripherals.pdf`, `references/arm-arm/DDI0487M…pdf` vorhanden; Track B in docs/SOURCE-ANALYSIS-PROTOCOL.md:68 | keep |
| CL-016 | CLAUDE.md:70-72 | Kein Code/Spec aus dem Gedaechtnis | decision | OK | keep |
| CL-017 | CLAUDE.md:74-92 | Regel 2: keine Stubs; Blocked-Eintrag + Rueckfrage | decision | Gelebt: A45 als Blocked (docs/STATUS.md:357-377, TASK/A45-STATUS-BLOCKED.MD) | keep |
| CL-018 | CLAUDE.md:94-105 | Regel 3: bei Unsicherheit STOP und fragen | decision | OK | keep |
| CL-019 | CLAUDE.md:107-117 | Regel 4: drei Debugversuche, dann Diagnose | decision | OK; ergaenzt durch Auto-Memory `root-cause-before-fixing` | keep |
| CL-020 | CLAUDE.md:119-132 | Regel 5: Spec `docs/api/<group>.md` vor Code | decision | Pfadmuster veraltet: Spezifikationen liegen meist als `docs/api/<group>/<untergruppe>/<nn>-<name>.md` (z. B. docs/api/gemdos/console/01-cconin.md); Einzeldateien wie docs/api/gemdos.md existieren daneben. Muster auf beide Formen erweitern | fix |
| CL-021 | CLAUDE.md:134-141 | Regel 6: keine erfundene Hardware, Linux nur Querverweis | decision | Track B vorhanden; OK | keep |
| CL-022 | CLAUDE.md:147 | Antworten auf Deutsch | decision | OK | keep |
| CL-023 | CLAUDE.md:148 | Code, Kommentare, Commits auf Englisch | decision | Stichprobe Commits/Code englisch; OK | keep |
| CL-024 | CLAUDE.md:149 | Datei-/Pfadnamen englisch, klein, mit Bindestrich | decision | Widerspricht eigener Konvention: `TASK/<ID>-STATUS-DONE.MD`, `RUNINFO.TXT` (CLAUDE.md:201,216), docs/STATUS.md, docs/analysis/ORACLE.md, TOS-Dateien (`NEWDESK.INF`). Ausnahmen benennen | fix |
| CL-025 | CLAUDE.md:150 | Direkt, ohne Fuellwerk | decision | OK | keep |
| CL-026 | CLAUDE.md:156-157 | Rust nightly wegen `#![no_std]` + inline asm + `naked_function`, gepinnt | fact | Pin stimmt (`rust-toolchain.toml` nightly-2026-04-15), Begruendung nicht: naked ist stabil (`#[unsafe(naked)]` kernel/src/lib.rs:183, kein `#![feature]` im Baum). Nightly noetig fuer `per-package-target` (kernel/Cargo.toml:1, .cargo/config.toml `[unstable]`) | fix |
| CL-027 | CLAUDE.md:158 | `#![no_std]` fuer Kernel und Userland | fact | kernel/src/lib.rs:7, tos-api/src/lib.rs:17 (cfg_attr), programs/desktop/src/lib.rs:41 | keep |
| CL-028 | CLAUDE.md:159 | `alloc` erst nach Kernel-Allokator | fact | `#[global_allocator]` kernel/src/lib.rs:14; OK | keep |
| CL-029 | CLAUDE.md:160 | Ein Cargo-Workspace, Sub-Crates je Schicht | fact | Stimmt fuer Kernel-Workspace; `programs/` ist ein zweiter, eigener Workspace (Cargo.toml `exclude`). Erwaehnen | fix |
| CL-030 | CLAUDE.md:161 | `cortex-a`-Crate fuer Registerzugriffe | fact | Nicht im Code: keine Abhaengigkeit in kernel/tos-api/tos-types Cargo.toml, kein `cortex_a` im Quelltext (Treffer nur "cortex-a72" als QEMU-CPU). Streichen oder als nicht genutzt markieren | fix |
| CL-031 | CLAUDE.md:162 | `tock-registers` fuer MMIO | fact | Nicht im Code (keine Abhaengigkeit, kein Treffer); MMIO per `mmio_va()`-Konstanten (z. B. kernel/src/drivers/bcm2711/pl011.rs:27-29) | fix |
| CL-032 | CLAUDE.md:163 | `spin` oder selbstgebaut, kein parking_lot | fact | Selbstgebaut: `SpinMutex` kernel/src/sync.rs; OK | keep |
| CL-033 | CLAUDE.md:164 | QEMU `aarch64-virt` primaeres Testziel, RPi4 sekundaer | fact | Praxis: QEMU `raspi4b` (Board rpi4) ist Hauptziel (Recent Decisions ~99 Nennungen raspi4b vs ~80 virt; Auto-Memory `qemu-window-view-command`, `raspi4b-runs-need-serial-on`), echte Pi = Abnahme-Gate. Anpassen | fix |
| CL-034 | CLAUDE.md:165 | GDB ueber QEMU-gdbstub | fact | tools/xtask/src/main.rs:791-792 (`-S -s`, :1234) | keep |
| CL-035 | CLAUDE.md:168 | Nichts vorschlagen ausser Rust/AArch64-asm | decision | Widerspricht SDK (C-Ports QED, TeraDesk, zView, `sdk/libc` mintlib) und ADR-019 docs/MEMORY.md:1176; auf Kernel/tos-api eingrenzen | fix |
| CL-036 | CLAUDE.md:169 | Keine Linux-APIs, POSIX, libc | decision | `sdk/libc` = mintlib 0.60.1 (POSIX-Schicht) fuer Anwendungen; Auto-Memory `port-reuse-dont-rewrite`. Auf Kernel eingrenzen | fix |
| CL-037 | CLAUDE.md:170 | Kein std/tokio/Heap-Crate vor Allokator | decision | OK, keine solchen Abhaengigkeiten | keep |
| CL-038 | CLAUDE.md:171 | Nur RPi-Firmware-`kernel8.img` als Bootlader | decision | OK (boot/rpi4/config.txt) | keep |
| CL-039 | CLAUDE.md:172 | Kein crossbeam/parking_lot/lazy_static | decision | Keine Treffer im Code | keep |
| CL-040 | CLAUDE.md:173 | Keine x86-Targets | decision | Doppelt zu CLAUDE.md:21, harmlos | keep |
| CL-041 | CLAUDE.md:174 | Kein bindgen auf EmuTOS-C | decision | OK | keep |
| CL-042 | CLAUDE.md:180-205 | Quellbaum-Block als Ganzes | fact | Wurzeleintrag `programs/` fehlt (seit `9f451429`, 2026-10-14; Desktop, newdesk, newtos-gem, hello-gem). Ebenfalls nicht genannt: `README.md`, `LICENSE`, `NOTICE`, `.github/` | fix |
| CL-043 | CLAUDE.md:182-183 | `CLAUDE.md`, `Cargo.toml` (Workspace kernel, tos-api, tos-types, tools/xtask) | fact | Cargo.toml members stimmen exakt | keep |
| CL-044 | CLAUDE.md:184-185 | `kernel/` mit arch/aarch64, drivers/bcm2711, hal, bios, `desk/` = EmuDESK, selftest_*.rs | fact | `kernel/src/desk/` existiert nicht mehr: per `git mv` nach `programs/desktop/` (`90b04444`, DESK43 U8b), ADR-026 docs/MEMORY.md:2240; Rest vorhanden | fix |
| CL-045 | CLAUDE.md:186-188 | `tos-api/` mit gemdos, bdos, bios, xbios, aes, vdi, trap.rs; `tests/` | fact | Alle Verzeichnisse + trap.rs + tests/ vorhanden | keep |
| CL-046 | CLAUDE.md:189 | `tos-types/` = DTA, BPB, MD, Fehlercodes | fact | Nur `MD` (tos-types/src/memory.rs:61) und Fehlercodes; BPB liegt in tos-api/src/bios/bpb.rs:17, ein DTA-Typ existiert nirgends. Dateien: aes, basepage, error, gemdos, memory | fix |
| CL-047 | CLAUDE.md:190 | `tools/` = xtask, make-sd-image.sh, Trace-/Probe-Skripte | fact | Vorhanden; neu `tools/oracle/` + `hatari-oracle.sh` (Oracle) koennte genannt werden | keep |
| CL-048 | CLAUDE.md:191 | `sdk/` = libc, gemlib, cflib, crt0, overlays/, apps/ | fact | Alles vorhanden (crt0 als sdk/crt0.S und sdk/libc/crt0.S) | keep |
| CL-049 | CLAUDE.md:192 | `tosdump/` = cases/, reference/, deploy-* | fact | Vorhanden (deploy-pi4, deploy-tt) | keep |
| CL-050 | CLAUDE.md:193 | `boot/rpi4/` = RPi-Firmwaredateien | fact | Enthaelt nur `config.txt`, keine Firmware (start4.elf o. ae.); Beschreibung auf "config.txt fuer die Bootpartition" aendern | fix |
| CL-051 | CLAUDE.md:194 | `keyboards/` = .kbd-Quellen | fact | german.kbd, us.kbd | keep |
| CL-052 | CLAUDE.md:195-199 | `docs/` mit STATUS, MEMORY, …, api/, analysis/ (<topic>-YYYY-MM-DD.md), hardware/, sdk/, benchmarks/, presentation/ | fact | Alle vorhanden; Namensmuster in docs/analysis/ nur teilweise befolgt (ORACLE.md, arch03-*.md ohne Datum, Unterordner) | keep |
| CL-053 | CLAUDE.md:200 | `phases/` = PHASE-XX-*.md | fact | Vorhanden, PHASE-00 bis PHASE-16 + _archive/ | keep |
| CL-054 | CLAUDE.md:201 | `TASK/` = <ID>.md + -STATUS-DONE.MD / -STATUS-UNFINISHED.MD | fact | Weitere Formen im Bestand: `-STATUS-BLOCKED.MD` (A45), `-AUDIT-BEANSTANDET.MD` (A25), `-PLAN-STATUS-DONE.MD` (DESK43). Ergaenzen | fix |
| CL-055 | CLAUDE.md:202 | `measurements/` = ein Verzeichnis je Lauf | fact | 317 Eintraege; Ausreisser `measurements/hw-round-2026-09-24` (kein `measurement-`-Praefix) | keep |
| CL-056 | CLAUDE.md:203-204 | `references/` gitignored ausser README.md | fact | .gitignore `/references/*` + `!/references/README.md`; `git ls-files references` = nur README.md | keep |
| CL-057 | CLAUDE.md:207-208 | Lokal, gitignored: target, Applications, hardware, graphify-out, .venv, sd-card-backup-* | fact | Alle ignoriert; `.venv/` nur ueber die eigene `.venv/.gitignore` (`*`), nicht ueber die Wurzel-.gitignore | keep |
| CL-058 | CLAUDE.md:212-213 | Nichts Neues in die Wurzel | decision | Regel gilt; `programs/` kam 2026-10-14 neu in die Wurzel (siehe CL-042) | keep |
| CL-059 | CLAUDE.md:214-218 | Laeufe nach `measurements/measurement-YYYY-MM-DD-<ids>/`, RUNINFO.TXT, method/, serial/, screendumps/ | decision | Gelebt; Ausreisser hw-round-2026-09-24, measurement-2026-09-09 ohne IDs; .gitignore schliesst *.img/*.elf je Lauf aus | keep |
| CL-060 | CLAUDE.md:219-221 | Laeufe sind nach Commit unveraenderlich | decision | OK | keep |
| CL-061 | CLAUDE.md:222-223 | Laeufe repo-relativ zitieren | decision | OK | keep |
| CL-062 | CLAUDE.md:224-225 | Scratch nach `$CLAUDE_JOB_DIR/tmp` / Scratchpad, nie `/tmp` | decision | Doppelt in docs/STATUS.md:25-28 und Auto-Memory `subagent-scratch-in-job-tmp`; CLAUDE.md ist die kanonische Stelle | keep |
| CL-063 | CLAUDE.md:226-227 | `kernel8.img` nicht in der Wurzel; `cargo xtask build --release` | decision | Keine kernel8.img in der Wurzel, `build` existiert (tools/xtask/src/main.rs:125); Auto-Memory `flash-release-builds-to-real-hw` ergaenzt | keep |
| CL-064 | CLAUDE.md:233-234 | TOS-Funktionen als kleingeschriebene Rust-Namen | decision | `fopen` tos-api/src/bdos/fs/file.rs:288, `cconws` gemdos/console.rs:1019, `appl_init` aes/appl.rs:405 | keep |
| CL-065 | CLAUDE.md:235-236 | Fehlercodes `E_OK`, `EFILNF = -33` in `tos-types::error` | fact | tos-types/src/error.rs:18,40 (Typ `Long` = i32, tos-types/src/lib.rs:33) | keep |
| CL-066 | CLAUDE.md:237 | Register als `BCM2711_<PERIPHERAL>_<REGISTER>` in `kernel::drivers::bcm2711` | decision | Nur pm.rs folgt (`BCM2711_PM_*`); pl011.rs `UART0_BASE`, emmc.rs `EMMC2_BASE`, mailbox.rs `MBOX_BASE`. Regel an Praxis anpassen oder Code nachziehen (Nutzerentscheidung) | fix |
| CL-067 | CLAUDE.md:238-239 | Trapnummern als `TRAP_GEMDOS = 1` usw. | decision | Dispatcher nutzt Enum mit Literal-Match (tos-api/src/trap.rs:42-48: 1/2/13/14), `TRAP_GEMDOS` nur lokal in tos-api/src/aes/shell.rs:111; Trap 2 (GEM) fehlt in der Regel | fix |
| CL-068 | CLAUDE.md:245-246 | Vor Code: Quellanalyse nach Protokoll | decision | OK | keep |
| CL-069 | CLAUDE.md:247-248 | Aktuelles Phasendokument lesen; ausserhalb der Phase nur mit Freigabe | decision | Es gibt keine "aktuelle Phase" mehr: docs/STATUS.md:8 fuehrt Stand + Ticketliste, Arbeit laeuft ueber `TASK/` (Auto-Memory `finish-the-task-list-no-detours`). Auf "Ticket in TASK/ lesen" umstellen | fix |
| CL-070 | CLAUDE.md:249 | TOS-Spec in `docs/api/` lesen, sonst Regel 5 | decision | OK (Pfadmuster siehe CL-020) | keep |
| CL-071 | CLAUDE.md:250 | docs/MEMORY.md auf ADRs pruefen | decision | OK, ADR-010 bis ADR-027 vorhanden | keep |
| CL-072 | CLAUDE.md:251 | docs/STATUS.md auf laufende Arbeit pruefen | decision | OK; Kopf ist teils veraltet (siehe Teil SH), Auto-Memory `status-md-is-not-evidence` | keep |
| CL-073 | CLAUDE.md:252-253 | Commits nennen Spec-Abschnitt, ADR oder Phase-Deliverable (Bsp. `docs/api/gemdos.md#0x01`) | decision | Beispielanker existiert (docs/api/gemdos.md:13 Cconin); Praxis nennt Ticket-IDs (55 der letzten 300 Betreffzeilen, z. B. "(DESK43 U8b)"). Ticket-IDs aufnehmen | fix |
| CL-074 | CLAUDE.md:259 | Conventional Commits | decision | 300/300 letzte Betreffzeilen konform | keep |
| CL-075 | CLAUDE.md:260 | Scope = Crate/Subsystem | decision | OK | keep |
| CL-076 | CLAUDE.md:261 | Betreff ≤ 72 Zeichen, Imperativ | decision | Regel gilt, in der Praxis 50 der letzten 300 Betreffzeilen laenger | keep |
| CL-077 | CLAUDE.md:262 | Body erklaert warum | decision | OK | keep |
| CL-078 | CLAUDE.md:263 | Phase-Deliverable-IDs referenzieren (`Closes PHASE-02-D3`) | decision | 0 von 300 letzten Commits (Betreff und Body) tragen PHASE-IDs; ersetzt durch Ticket-IDs | fix |
| CL-079 | CLAUDE.md:264 | Keine kaputten Builds, keine roten Tests ohne `#[ignore]` | decision | OK; Basislinie "x/0/1" = ein ignorierter Test | keep |
| CL-080 | CLAUDE.md:265-270 | STATUS.md im Gleichschritt: Phasenzeile anheben + datierter Recent-Decisions-Eintrag, Stash-Verfahren | decision | Recent-Decisions-Teil gelebt; "Phasenzeile anheben" nicht: Zeilen 11/14/15 seit Juli/September eingefroren (docs/STATUS.md:327,330,331). Doppelt in Auto-Memory `update-status-md-after-each-extension`. Phasenzeilen-Pflicht streichen oder auf Stand-Abschnitt umstellen | fix |
| CL-081 | CLAUDE.md:276 | Unit-Tests in-crate `#[cfg(test)]`, auf dem Host | fact | `cargo xtask test` = Hosttests (tools/xtask/src/main.rs:864-876) | keep |
| CL-082 | CLAUDE.md:277 | Integrationstests in QEMU `aarch64-virt` via `cargo xtask qemu-test` | fact | Unterkommando existiert nicht (xtask kennt build, build-programs, emutos-consts, gen-font, gen-fsel-rsc, prgflags, run, test; main.rs:124-132). QEMU-Laeufe laufen ueber `cargo xtask run` + Skripte/Injektor in `measurements/*/method`, Konformitaet ueber tosdump/Oracle (`cf4d9b33`) | fix |
| CL-083 | CLAUDE.md:278 | HW-Tests manuell in docs/HW-TEST-LOG.md | fact | Datei gepflegt, letzter Eintrag 2026-10-23 (`cd60428b`) | keep |
| CL-084 | CLAUDE.md:279-281 | Je TOS-Funktion ein Positiv- und ein Fehlertest | decision | Regel; Erfuellung nicht geprueft (Abdeckung in docs/MEASUREMENT-COVERAGE.md) | keep |
| CL-085 | CLAUDE.md:285-294 | Vorgehen bei Unwissen, sechs Stufen | decision | OK; "Track A or Track B" existiert in SOURCE-ANALYSIS-PROTOCOL | keep |

## docs/STATUS.md Kopf (Stand, Historischer Text, Phase Status, Open ADRs, Blocked)

Quelle: /Volumes/Daten/Development/PiRusTOS/docs/STATUS.md Zeilen 1–461. Checker (805 Treffer): path OK=645, path MISSING=69, path AMBIGUOUS=48, commit OLD-HASH-MAPPED=21, commit OK=12, commit BAD-COMMIT=4, commit EMUTOS-COMMIT=1, link OK=3, link MISSING=2. Echte Defekte darunter: BAD-COMMIT `463cf6a`, `b040bb6` (Z. 332); tote Repo-Pfade `kernel/src/desk/…` (Z. 95, 330, 380, zwei Links in 330), `kernel/src/desktop.rs`, `kernel/src/mm/{frames,heap}.rs` (heute `kernel/src/bios/machine/`), `TASK/A79-`/`A83-STATUS-UNFINISHED.MD` (heute `-STATUS-DONE.MD`). Rauschen: `x0..x7`, `v0..v31`, `v0.5` (Tag), EmuTOS-Kurznamen (`desk_rsc.c:…`, `lists.c`), Laufzeitartefakte (`*.NCP`, `libc.a`). BAD-COMMIT `ce57817`, `2ac3e85` (Z. 331) sind cflib-Upstream-Commits (`sdk/cflib/cflib-21-ce57817.tar.xz`), kein Defekt.

### Systemische Befunde
- Der "aktuelle" Kopf ist nicht aktuell: Ueberschrift "Stand 2026-10-16", Tag `v0.5`, Basislinien vom 16.10. (Hosttests 1380 statt 1664, Clippy 76/125 statt 74/121, SDK 20 statt 26), "P1: keins" trotz offenem P1-Ticket USB01, SDK01 als "Vorrang", obwohl A–J am 21.10. erledigt sind.
- Die Phase-Status-Tabelle ist zum zweiten Aenderungsprotokoll geworden (Zeile 13 = 77 KB, Zeile 14 = 42 KB) und wird nicht mehr gepflegt: Zeilen 11, 14, 15 sind seit Juli/September eingefroren; der Desktop steht noch als "In Arbeit — läuft als Boot-Anwendung", obwohl er seit ADR-026 ein EL0-Programm ist. Vorschlag: je Phase eine Statuszeile, datierte Historie ins Archiv.
- Open ADRs und Blocked sind grossteils erledigt: ADR-Supexec (X01), ADR-Line-A-02 (ADR-011), ADR-010/opaker PD (ARCH03 Stufe 2, `32c6171c`), `graf_mouse` M_SAVE (`984b197f`), EC2, "Kleinere Lücken", Desktop-Rebuild-Verifikation, PORT09. Wirklich offen bleiben nur KEYS02-Restvektoren, A45, `shel_find` Stufe 4, Mehrbank-RAM (kein Blocker) und USB-Maus.
- Der "Historische Text" (Z. 90–305) ist reine Historie mit einem Gliederungsfehler: die Phase-00-Liste (Z. 232–291) steht ohne Kopf unter Phase 03, ihr Kopf steht allein bei Z. 131. Eine Falle darin ist widerlegt (QUIRK-002, Z. 161–172), wird aber noch in docs/MEMORY.md:2538 als gueltig gefuehrt.
- Mehrere Kopf-Regeln stehen doppelt in Auto-Memory und CLAUDE.md (Scratch, Arbeitsweise, SD-Karte, SER01, `cargo clean --release`).

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| SH-001 | docs/STATUS.md:8-17 | Stand 2026-10-16: main, Tag v0.5, EL0-Desktop NEWDESK auf Pi abgenommen, Ziel base-1 offen | fact | Tag veraltet: `v0.6` (STATUS:520) und `v0.6.1` (STATUS:485) existieren (`git tag`); Hashes `03b9025c`/`f73bb075`/`a8b03c81` sind alte, gemappte Hashes (Konvention); Ueberschriftsdatum 16.10. bei Inhalt bis 26.10.; base-1 weiter undefiniert (docs/analysis/inventory-2026-10-08.md vorhanden) | fix |
| SH-002 | docs/STATUS.md:19-23 | DESK43 U10 erledigt; Tester-Lauf und ADR offen; Pi-Restpunkte | obsolete | "Tester-Lauf und ADR offen" erledigt: STATUS:512 (Tester PASS `d0e5afa5`), STATUS:513 (ADR-026); Pi-Restpunkte vollstaendig in TASK/DESK43.md:182-183; Zusammenfassung schon in STATUS:67 | delete |
| SH-003 | docs/STATUS.md:25-28 | Agenten-Scratch im Job-Verzeichnis, QEMU-Sockets relativ, Netzteil, ein QEMU zur Zeit | decision | Scratch-Teil doppelt zu CLAUDE.md:224-225 und Auto-Memory `subagent-scratch-in-job-tmp`; Sockets/Netzteil/ein-QEMU stehen nirgends sonst. Nach CLAUDE.md (Placement rules) verschieben | fix |
| SH-004 | docs/STATUS.md:30-36 | Liste der auf echter Pi 4 abgenommenen Funktionen | fact | Belege vorhanden: Doom 40–75 fps STATUS:588, KEYS02 STATUS:584, Pi-Abnahmen STATUS:504/509/532, docs/HW-TEST-LOG.md | keep |
| SH-005 | docs/STATUS.md:38-40 | ARCH03 umgesetzt und abgenommen; offen MEM05, Stufe 2b, asid=on auf Pi | fact | Laufverzeichnis measurements/measurement-2026-10-13-arch03-acceptance/ existiert; MEM05 ist erledigt (TASK/MEM05-A-, MEM05-B-STATUS-DONE.MD, STATUS:492-493, eigene Liste Z. 74). MEM05 streichen | fix |
| SH-006 | docs/STATUS.md:46 | Basislinie Hosttests 1380/0/1 | fact | Veraltet: zuletzt 1664/0/1 (STATUS:467, A118-Nacharbeit 2026-10-26) | fix |
| SH-007 | docs/STATUS.md:47 | Basislinie Bau-Warnungen tos-api 64, kernel 94 | fact | Veraltet: kernel 94 → 93 (STATUS:474), tos-api 64 | fix |
| SH-008 | docs/STATUS.md:48 | Basislinie Clippy 76/125 | fact | Veraltet: tos-api 74 / kernel 121 (STATUS:468, STATUS:470) | fix |
| SH-009 | docs/STATUS.md:49 | Driftprobe 0 (unsafe 20) | fact | Nicht nachgemessen (read-only); letzter Beleg Tester-Lauf 16.10. (STATUS:512); `tools/desk-drift-probe.sh` existiert. Messdatum dazuschreiben | fix |
| SH-010 | docs/STATUS.md:50 | SDK-Hosttests 20/0/0 | fact | Veraltet: 26/0/0 (STATUS:483-484, 2026-10-22) | fix |
| SH-011 | docs/STATUS.md:52-55 | `cargo clean … --target` braucht `--release`, sonst warmer Release-Cache | trap | Vollstaendig in Auto-Memory `cargo-xtask-board-target-footgun.md:35-39` | delete |
| SH-012 | docs/STATUS.md:57-63 | Desktop nach ADR-022 als Bibliothek `programs/desktop/`, EL0-NEWDESK, APITEST 109/0/5 | fact | Geprueft: programs/desktop/Cargo.toml haengt nur an `newtos-gem` + `tos-types`; kernel/Cargo.toml ohne desktop; APITEST 109/0/5 zuletzt STATUS:467. Doppelt zu Auto-Memory `desktop-1to1-emutos-rebuild` und ADR-026 docs/MEMORY.md:2240, im Kopf aber passend | keep |
| SH-013 | docs/STATUS.md:67 | P1: keins; DESK43 erledigt, offen nur EmuCON-Gegenprobe | fact | Falsch: TASK/USB01.md:3 "OFFEN · P1" seit 2026-10-22; dazu P1-Tickets mit offener HW-Abnahme (A16, A37, DESK34, DESK35, HW03). DESK43-Teil stimmt (TASK/DESK43.md:3) | fix |
| SH-014 | docs/STATUS.md:68 | P2: SDK01 Vorrang; GH1, FS18, PROC03 erledigt | fact | SDK01 A–J erledigt 2026-10-21 (STATUS:487, TASK/SDK01.md:3), offen nur K (Linux-Abnahme, STATUS:481); GH1/FS18/PROC03 gehoeren in die Erledigt-Liste Z. 73-74; offenes P2 USB03 (TASK/USB03.md:3) fehlt | fix |
| SH-015 | docs/STATUS.md:69 | Umfang 1.0: nur FAT16, Ton (SND01, SND02, H04) | decision | Deckt sich mit STATUS:475, Auto-Memory `version-roadmap-1-0-2-0` und Ticketkoepfen (SND01/SND02/H04 P2, "gehoert zu 1.0") | keep |
| SH-016 | docs/STATUS.md:70-72 | P3-/Erweiterungsliste (ARCH03 2b … IO01) | fact | Alle genannten Tickets existieren, keines hat eine DONE-Datei; offenes P3-Ticket USB02 (TASK/USB02.md:3) fehlt | fix |
| SH-017 | docs/STATUS.md:73-74 | Seit 30.09. erledigt, frueher als offen gefuehrt | obsolete | Reine Erledigt-Historie; jeder Punkt hat einen Recent-Decisions-Eintrag | archive |
| SH-018 | docs/STATUS.md:76-78 | SER01: serielle Ausgabe standardmaessig aus, `serial=on`, Shim fuer alte Skripte | fact | kernel/src/bootargs.rs:21-22, tools/qemu-serial-on vorhanden; doppelt zu Auto-Memory `raspi4b-runs-need-serial-on` | keep |
| SH-019 | docs/STATUS.md:80-82 | make-sd-image.sh = QEMU-Abbild, echte Karte nur auf Ansage | decision | Doppelt zu Auto-Memory `card-write-only-what-user-says` und `auto-folder-console-only`; als Kopfregel sinnvoll | keep |
| SH-020 | docs/STATUS.md:84-86 | Ein Task pro Agent, seriell; Lead baut nicht; Worker fassen Karte/Pi nicht an und committen nicht | decision | Doppelt zu Auto-Memory `limit-concurrent-workers-token-budget`, `workers-no-hardware-or-outward-actions`; "committen nicht" zu streng: .claude/agents/developer.md:98 "Do not commit unless the lead asks", Auto-Memory `push-after-worker-report` setzt committende Developer voraus | fix |
| SH-021 | docs/STATUS.md:93-101 | Kurswechsel 2026-07-04 (ADR-016): Desktop neu in `kernel/src/desk/` | obsolete | `kernel/src/desk/`, `kernel/src/desktop.rs` existieren nicht mehr; abgeloest durch ADR-022 (docs/MEMORY.md:1520) und ADR-026 (docs/MEMORY.md:2240); Wissen steht dort | archive |
| SH-022 | docs/STATUS.md:103-118 | Phase 14 alter Desktop (Boot-Anwendung, Menuegruppen A–C, offene Punkte) | obsolete | Selbst als abgeloest markiert (Z. 101); ersetzt durch EL0-NEWDESK | archive |
| SH-023 | docs/STATUS.md:120-124 | Phase 13 AES abgeschlossen, Feinschliff offen (rsrc_saddr …) | obsolete | `rsrc_saddr` inzwischen umgesetzt (tos-api/src/aes/resource.rs:2875); Stand in Phase-Status-Zeile 13 | archive |
| SH-024 | docs/STATUS.md:126-129 | Hinweis: Phasen 00–12 fertig, Folgeblock ist Historie | obsolete | Gliederungstext zur Historie | archive |
| SH-025 | docs/STATUS.md:131, 232-291 | Phase 00 Deliverables D1–D13 (Kopf bei 131, Liste ohne Kopf unter Phase 03) | obsolete | Gliederungsfehler: Liste 232-291 haengt unter "Phase 03"; "Next:" (290-291) nennt bereits erledigte Punkte; Inhalt in docs/api/_phase00-completion.md (Coverage, Open ADRs) | archive |
| SH-026 | docs/STATUS.md:133-160, 173-178 | Phase 02 Konsole D0–D6, Smoke-Test 2026-05-03 | obsolete | "Cconin/Crawcin/Cnecin/Cconrs blockiert auf Phase 06" erledigt mit Phase 07 D3/D4 (STATUS:323) | archive |
| SH-027 | docs/STATUS.md:139-141 | D1-ABI: SVC-Immediate als Trapklasse, Funktion in x8, Argumente x0..x7, Rueckgabe sign-extended i32 | obsolete | Ueberholt: Rueckgabe heute sign-extended i64 (docs/ARCHITECTURE.md:53, tos-api/src/trap.rs:6), Trap 2 (GEM) dazugekommen; kanonisch in docs/ARCHITECTURE.md:53-56 | archive |
| SH-028 | docs/STATUS.md:161-172 | D7/QUIRK-002: PL011-RX-IRQ wird auf echter Pi nicht zugestellt, Loesung RX-Poll | obsolete | Widerlegt 2026-09-04 (Auto-Memory `pl011-rx-irq-not-delivered-on-real-hw`, STATUS:771); `console::pl011_rx_poll` existiert nicht mehr (nur `pl011_rx_drain`, kernel/src/console.rs:221). Vor Archivierung docs/MEMORY.md:2538 (QUIRK-002) als widerlegt markieren | archive |
| SH-029 | docs/STATUS.md:180-230 | Phase 03 Speicher D1–D8, Memory-Map, Frame-Allocator, Heap, Smoke-Test | obsolete | Pfade `kernel/src/mm/frames.rs`, `heap.rs` heute `kernel/src/bios/machine/frames.rs`, `heap.rs`; ST-RAM-Aufteilung ueberholt durch konstanten 10-MiB-Pool (STATUS:319, `STRAM_OS_RESERVE_PHYS` kernel/src/arch/aarch64/link-rpi4.ld:43) | archive |
| SH-030 | docs/STATUS.md:191-196 | D3 MMU: Pfad A (Identity, low-half) gewaehlt, High-Half spaeter | obsolete | Abgeloest durch Phase 05 (STATUS:321) und ADR-014 (docs/MEMORY.md:459, keine Low-Half-Identity) | archive |
| SH-031 | docs/STATUS.md:292-305 | Phase 01 Boot + UART, EC2 (echte Pi) pending | obsolete | EC2 laengst erfuellt (Pi-Boot STATUS:30, Auto-Memory `sd-card-real-hw-layout` 2026-06-18) | archive |
| SH-032 | docs/STATUS.md:309-312 | Phasenplan 2026-05-03 nach EmuTOS-Modulen, alte Nummern in `phases/_archive/`, m68k-Emulator aus dem Umfang | decision | `phases/_archive/` vorhanden; "m68k … removed from scope" widerspricht CLAUDE.md:16 und Auto-Memory `own-extensions-separate-aarch64-from-m68k` (Emulator spaeter fuer .PRG/.TOS/.TTP). Satz anpassen | fix |
| SH-033 | docs/STATUS.md:316 | Phase 00 abgeschlossen 2026-05-02 | fact | docs/api/_phase00-completion.md vorhanden | keep |
| SH-034 | docs/STATUS.md:317 | Phase 01: EC2 pending HW; Boot-Schirm-Warteschleife nach `initinfo` | fact | "EC2 pending HW" ueberholt (siehe SH-031); Cite `bios/initinfo.c:376-442` stimmt am Pin, `INITINFO_DURATION` 15 kernel/src/initinfo.rs:53 | fix |
| SH-035 | docs/STATUS.md:318 | Phase 02: D6/D7 offen; Konsolenprofil je Board; `console_peek_key` ausgenommen | fact | "D7 offen" ueberholt (Konsole auf Pi abgenommen); Ausnahme `console_peek_key` stimmt noch (kernel/src/host_env.rs:87-89 ohne `fill_from_device`) | fix |
| SH-036 | docs/STATUS.md:319 | Phase 03 BIOS-HW-Schicht abgeschlossen + ST-RAM-Pool-Bisect 2026-09-04 | fact | Konstanter Pool stimmt mit link-rpi4.ld:43 (4 MiB membot) ueberein; Zeile ist ueberwiegend datierte Historie (Hash `49c3938` gemappt). Statuskern behalten, Historie ins Archiv | archive |
| SH-037 | docs/STATUS.md:320 | Phase 04 BDOS Memory (`bdos/mem.c`+`umem.c`+`iumem.c`) abgeschlossen | fact | `bdos/mem.c` gibt es in EmuTOS nicht (references/emutos/bdos: umem.c, iumem.c, osmem.c, mem.h). Titel korrigieren | fix |
| SH-038 | docs/STATUS.md:321 | Phase 05 abgeschlossen; D7 Supexec bis ADR-Supexec verschoben | fact | Supexec entschieden mit X01 (STATUS:529, TASK/X01.md:3: Kern EINVFN, Super/Supexec als SDK-Overlay) | fix |
| SH-039 | docs/STATUS.md:322 | Phase 06 komplett; offen: dereferenzierbarer PD, PE_BASEPAGE/GOTHENFREE/BASEPAGEFLAGS, EC4 Pwait | fact | Beides erledigt: `32c6171c` ARCH03 Stufe 2 (tos-api/src/bdos/process/pexec.rs:48-51,191, STATUS:582); Pwait-Scheduler STATUS:992, tos-api/src/bdos/process/sched.rs | fix |
| SH-040 | docs/STATUS.md:323 | Phase 07 komplett, 200-Hz-Tick; offen 50-Hz-Haelfte (etv_timer, Tastenwiederholung, dosound) | fact | etv_timer/dosound weiter nicht im Interrupt (tos-api/src/bios/tick.rs:40-52); Tastenwiederholung aber umgesetzt (kernel/src/key_repeat.rs, Pi STATUS:33). Aus "offen" streichen | fix |
| SH-041 | docs/STATUS.md:324 | Phase 08 Block Devices, BIOS-Konsole, Cookie-Jar, Geraetetabelle (bis 2026-10-13) | fact | 4,3 KB datierte Historie, im Einzelnen nicht geprueft; keine toten Cites laut Checker. Statuskern behalten, Historie ins Archiv | archive |
| SH-042 | docs/STATUS.md:325 | Phase 09 FAT16 komplett; Frename gibt Quellkette noch nicht frei | fact | Frename-Punkt erledigt mit FS18 (STATUS:517, TASK/FS18.md:3); FAT16-only passt zu Umfang 1.0 (STATUS:475) | fix |
| SH-043 | docs/STATUS.md:326 | Phase 10 BIOS Screen, fbcon, VT52 (bis CON04 2026-10-22) | fact | Gepflegt bis 22.10., Laufverzeichnis measurements/measurement-2026-10-22-con04/ existiert; 3,8 KB Historie. Statuskern behalten, Historie ins Archiv | archive |
| SH-044 | docs/STATUS.md:327 | Phase 11 XBIOS substanziell fertig, Re-Audit 2026-07-09 | fact | Seit 2026-07-24 eingefroren: KEYS02 (`Kbdvbase`-Haken, STATUS:587), X01, X04 (STATUS:528) fehlen; Regel CLAUDE.md:266 "Phasenzeile anheben" nicht befolgt | fix |
| SH-045 | docs/STATUS.md:328 | Phase 12 VDI komplett, ~70 Opcodes (8,9 KB, bis 2026-09-21) | fact | Datierte Historie; "Pi-Abnahme offen (HW-Checkliste 31)" nicht geprueft; old hashes `675e311`, `c1f42ff` gemappt. Statuskern behalten, Historie ins Archiv | archive |
| SH-046 | docs/STATUS.md:329 | Phase 13 AES komplett, 11 Manager (77 KB, bis 2026-10-25) | fact | Tote Cites: `TASK/A79-STATUS-UNFINISHED.MD`, `TASK/A83-STATUS-UNFINISHED.MD` (heute `-STATUS-DONE.MD`), 4× `kernel/src/desk`; EmuTOS-Commit `acb7f3b4` gueltig. Statuskern behalten, Historie ins Archiv, Cites dabei korrigieren | archive |
| SH-047 | docs/STATUS.md:330 | Phase 14 Desktop "In Arbeit — läuft als Boot-Anwendung" (42 KB, bis 2026-09-25) | obsolete | Status ueberholt durch DESK43/ADR-026 (STATUS:511-513): EL0-Programm NEWDESK; 10 Pfade + 2 Links auf `kernel/src/desk/` tot. Statuszelle neu schreiben, Historie ins Archiv | fix |
| SH-048 | docs/STATUS.md:331 | Phase 15 First C Source Port "In Arbeit" (11 KB, bis 2026-09-25) | fact | Seit 25.09. eingefroren: PORT09, LDG/zView (STATUS:570-575), Doom, SDK01 (STATUS:487) fehlen; `ce57817`/`2ac3e85` sind cflib-Upstream, kein Defekt | fix |
| SH-049 | docs/STATUS.md:332 | Phase 16 USB-HID "D1–D7 läuft", abgeschlossen 2026-07-02 | fact | BAD-COMMIT `463cf6a`, `b040bb6` (weder Repo noch docs/commit-map.txt noch EmuTOS/FreeMiNT; xHCI-Fix vermutlich in `7ad26fee` 2026-06-26 aufgegangen, pruefen); "Completed" widerspricht offenem P1 USB01 (TASK/USB01.md:3) | fix |
| SH-050 | docs/STATUS.md:336-337 | Einleitung: ADRs blockieren spaetere Phasen | obsolete | Alle genannten Phasen (05, 06, 08, 12) abgeschlossen (STATUS:321-328) | archive |
| SH-051 | docs/STATUS.md:341 | ADR-Supexec blockiert PHASE-06 | obsolete | Entschieden mit X01 2026-10-17 (STATUS:529, TASK/X01.md:3). docs/MEMORY.md:273,292,373 fuehren es noch als offen, dort mitziehen | archive |
| SH-052 | docs/STATUS.md:342 | ADR-BDOS-Lock (SMP-Serialisierung) blockiert PHASE-05 | fact | Phase 05 abgeschlossen 2026-05-07; Thema selbst weiter offen (Mehrkern-Vorbereitung, Auto-Memory `post-2a-roadmap-bios-freemint`), kein ADR in docs/MEMORY.md. Spalte "Blockiert" auf Mehrkern-Ausbau umstellen | fix |
| SH-053 | docs/STATUS.md:343 | ADR-Line-A-01 C-ABI des Line-A-Exports, blockiert PHASE-12 | obsolete | Phase 12 abgeschlossen ohne Line-A-Export; Linie "keine Line-A-Emulation" STATUS:930, Auto-Memory `altitude-syscall-not-hardware-emulation`. Die Entscheidung steht nirgends als ADR: vor Archivierung in docs/MEMORY.md festhalten (Nutzer fragen) | archive |
| SH-054 | docs/STATUS.md:344 | ADR-Line-A-02 Mauszeiger-Eigentuemerschaft, blockiert PHASE-08 | obsolete | Faktisch entschieden durch ADR-011 per-device VDI-Workstation (docs/MEMORY.md:187, STATUS:892); Cursor in tos-api/src/vdi/mouse.rs | archive |
| SH-055 | docs/STATUS.md:348-351 | Bereinigungsnotiz 2026-06-20 | obsolete | Historie der Liste | archive |
| SH-056 | docs/STATUS.md:353 | KEYS02-Rest: IKBD-Vektoren ausser `kbdvec` werden nicht gerufen | fact | Bestaetigt: tos-api/src/xbios/kbdvecs.rs:32-45 (nur `kbdvec` gerufen), Slots :175-178 = `RET_STUB_VA`; Spec docs/api/xbios/22-kbdvbase.md vorhanden. KEYS02 selbst DONE, kein offenes Ticket fuer den Rest | keep |
| SH-057 | docs/STATUS.md:355 | PORT09 (durchgestrichen) erledigt 2026-10-06 | obsolete | Erledigt: STATUS:575, TASK/PORT09-STATUS-DONE.MD; Ursache in TASK/MEM03.md | delete |
| SH-058 | docs/STATUS.md:357-377 | A45: `appl_tplay`/`appl_trecord` blockiert auf AES-Forkqueue | fact | Code tos-api/src/aes/appl.rs:769-807 wie beschrieben; EmuTOS-Cites am Pin geprueft (aes/gemaplib.c:209-213, aes/gemdisp.c:123-146, aes/gemsuper.c:85,109-110 stimmen); TASK/A45-STATUS-BLOCKED.MD | keep |
| SH-059 | docs/STATUS.md:379-394 | Desktop-Rebuild: interaktive Verifikation nur auf Pi, AES-Luecken #11–#13 | obsolete | `kernel/src/desk/` weg (`90b04444`); Injektor seit 2026-09-17 (Auto-Memory `desktop-interactive-verification-real-hw`, `tools/inject-arm.py`); H-Slider umgesetzt (tos-api/src/aes/window.rs:3264, A98). Strategie steht in der Auto-Memory | archive |
| SH-060 | docs/STATUS.md:395-405 | `graf_mouse` M_SAVE/M_RESTORE/M_PREVIOUS als No-Op aufgeschoben | obsolete | Umgesetzt: tos-api/src/aes/graf.rs:637-639 (`vdi::mouse::save_form`/`restore_form`/`previous_form`), `984b197f` 2026-07-23 | archive |
| SH-061 | docs/STATUS.md:406-424 | Scrap/Shell: F7 `shel_read`-Producer fehlt, `shel_find` Stufen 1 und 4 fehlen | fact | F7 erledigt (H05 STATUS:706; `s_cmd`/`s_tail` tos-api/src/aes/shell.rs:139-144); Stufe 1 `p_appdir` umgesetzt (shell.rs:1074); offen nur Stufe 4 `PATH=` (shell.rs:1119) und die `read_file`-Probe (shell.rs:1113). EmuTOS-Cites gemshlib.c:406-472, :645 stimmen. Auf den Rest kuerzen | fix |
| SH-062 | docs/STATUS.md:425-431 | ADR-010 / opaker PD blockiert Source-Porting | obsolete | Geloest durch ARCH03 Stufe 1/2 (VA = PA, `32c6171c` PE_BASEPAGE/GOTHENFREE/BASEPAGEFLAGS), abgenommen STATUS:582; Phase 15 portiert laengst | archive |
| SH-063 | docs/STATUS.md:432-434 | EC2 Real-HW-Rest: FAT16-Datei-I/O auf Pi ungetestet | obsolete | Auf Pi belegt: FS04 Kopieren ueber Partitionsgrenzen (STATUS:32), FS08-Pi-Messung (STATUS:566) | archive |
| SH-064 | docs/STATUS.md:435-443 | Mehrbank-RAM: DTB liest nur erste Bank, 768 MiB TT-RAM | fact | Bestaetigt: kernel/src/arch/aarch64/dtb.rs:6,29 (erste `/memory@`-Bank); kein Blocker (Auto-Memory `framebuffer-outside-st-tt-ram`: "768 MiB ausdruecklich genug"), kein Ticket. Als Ticket/Wunsch fuehren statt unter Blocked | fix |
| SH-065 | docs/STATUS.md:444-449 | Kleinere Luecken: Kbdvbase-Dispatch, Pwait-Round-Robin, Cconrs-E2E-Test; Clippy-Lints behoben | obsolete | Kbdvbase `kbdvec` mit KEYS02 (STATUS:587), Pwait-Scheduler (STATUS:992, A115 STATUS:469), Cconrs-Tests tos-api/tests/con01_console_typeahead.rs | archive |
| SH-066 | docs/STATUS.md:450-460 | USB-Maus nur an einem Port, Abnahme beider Steckkombinationen offen | fact | `4cbd7485` OK, docs/api/usb/hid-mouse-protocols.md und api/usb/pcie-rpi4.md:17 (QEMU ohne VL805) stimmen; laufend in TASK/USB01.md (OFFEN P1) | keep |

## STATUS.md Recent Decisions, Teil 1 (2026-10-01 bis 2026-10-26)

Quelle: docs/STATUS.md Zeilen 462–597 (132 Tabellenzeilen 466–597; 462–465 sind Ueberschrift/Tabellenkopf). Checker: OK=391, OLD-HASH-MAPPED=161, MISSING=52, AMBIGUOUS=19, BAD-COMMIT=7, EMUTOS-COMMIT=4.

### Systemische Befunde
- Alle 7 BAD-COMMIT sind keine Repo-Commits, sondern sha256-Praefixe von Kernel-/DOOM-Images (`2afed1e4`, `ba6172a1`, `1b00a7d5`, `b92e4d58`, `e7636f73`) oder Upstream-Pins (`6f3eeeb4` freemint/hypview, `dcb7a8db` doomgeneric): Checker-Rauschen, kein Defekt.
- MISSING/AMBIGUOUS fast nur Checker-Rauschen: blanke mintlib-Dateien (`crtinit.c`, `setvbuf.c`, `stdio_init.c`, `unix/read.c`, `posix/system.c`, `main.c`) liegen unter `sdk/libc/mintlib/mintlib/`, FreeMiNT-Dateien (`mprot040.c`, `block_IO.c`) unter `references/freemint/sys/`, `desk_rsc.c` ist generiert und ungetrackt in `references/emutos/desk/`; Paket-/Kartendateinamen (`USBSNAP.NCP`, `KERNOFAM.IMG`, `cmdline.txt` …). Einziger echter Pfadbefund: `kernel/src/desk/` (Z. 498, 500, 546–548, 596) ist seit U8b `programs/desktop/` (Z. 500 erklaert das als Historie).
- Folge von Z. 500/511/513 (Desktop ist EL0-Programm in `programs/desktop`): CLAUDE.md:185 nennt weiter `kernel/ … desk/ = EmuDESK rebuild`, acht Auto-Memory-Dateien nennen `kernel/src/desk` (u. a. desktop-1to1-emutos-rebuild.md, desktop-bypasses-app-abi-path.md, desktop-must-run-in-user-scope.md) — gehoert in die Teile CLAUDE.md/Auto-Memory.
- ADR-Luecke ARCH03 (Z. 582/583): die Grundsatzentscheidung „ein gemeinsamer Adressraum VA = PA, PIE, Schutz nach FreeMiNT“ existiert nur als „ADR-025, Status Proposed“ in `docs/analysis/arch03-implementation-plan.md:28-31`; in docs/MEMORY.md ist ADR-025 die MD-Vorrat-Entscheidung (MEM05, MEMORY.md:2131) — Nummernkollision; ADR-021 (MEMORY.md:1408-1411, Fenster je Prozess) steht weiter auf „Accepted“, obwohl ARCH03 es ersetzt; ADR-027 (MEMORY.md:2361) verweist mehrdeutig auf „ADR-025“. Vor Archivierung: ARCH03-ADR mit neuer Nummer nach MEMORY.md, ADR-021 als superseded markieren, ADR-027-Verweis eindeutig machen. Auto-Memory el0-app-memory-must-be-mapped.md beschreibt noch das Vor-ARCH03-Modell (User-VA statt PA).
- Fast alle Zeilen sind abgeschlossene Ticket-Historie mit DONE-Quittung bzw. `Status: ERLEDIGT` im Ticket; dauerhaftes Wissen liegt in docs/api/, COMPATIBILITY.md, MEMORY.md-ADRs oder Auto-Memory. Nur drei Zeilen tragen Wissen, das sonst nirgends steht (Z. 516 Commit-Map-Konvention, Z. 582 ARCH03-ADR, Z. 586 Messbeleg TOS 3.06 `kbdvec`). Die neueste Zeile 466 nennt einen Offen-Punkt, den `cf4d9b33` schon erledigt hat; fuer `cf4d9b33` fehlt der STATUS-Eintrag (CLAUDE.md-Lockstep).

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| SD1-001 | docs/STATUS.md:466 | Oracle-Pin EmuTOS 1a7484ab, Kalibrierung gegen EMUTOS14 | decision | Inhalt vollstaendig in docs/analysis/ORACLE.md §1–§8 und Auto-Memory emutos-oracle-pinned-1a7484ab.md; `aes/gemsuper.c:98` zeigt `ap_3resv = (LONG)&D`. „Offen: 4 WF_OWNER-Tests rot“ ist durch `cf4d9b33` erledigt (DELTAS leer), dafuer kein STATUS-Eintrag | fix |
| SD1-002 | docs/STATUS.md:467 | A118-Nacharbeit: abgelehnter Wiederaufruf gibt Sperre weiter | obsolete | TASK/A118-STATUS-DONE.MD; A118-Bindung in docs/api/aes/application.md, shell-loop.md, window-graphics.md | archive |
| SD1-003 | docs/STATUS.md:468 | A118: restliche `wind_update`-Sperrnehmer nach EmuTOS | obsolete | A118 DONE; Nebenbefunde (`all_run`, `gr_rubwind`/`gr_dragbox`, `MU_M1`-Arm) stehen in TASK/A118-STATUS-DONE.MD:64ff, kein eigenes Ticket | archive |
| SD1-004 | docs/STATUS.md:469 | A115: Scheduler nach `rlr`/`drl` | obsolete | A115 DONE; Spec event.md/accessories.md mit A115-Bindung | archive |
| SD1-005 | docs/STATUS.md:470 | A111 P2: `objc_edit`-Schreibmarke wie `curfld` | obsolete | A111 DONE; Spec docs/api/aes/object-form.md (EDEND) korrigiert | archive |
| SD1-006 | docs/STATUS.md:471 | A117: `MN_SELECTED` an `gl_mnppd` | obsolete | A117 DONE; Spec docs/api/aes/menu.md (2× A117) | archive |
| SD1-007 | docs/STATUS.md:472 | USB02 P10: ErrorRollOver, Anwenderentscheid nach Linux | decision | Entscheidung steht in docs/api/usb/hid-keyboard.md:75 „Status codes 0x01–0x03“; USB02 Sammelticket OFFEN | archive |
| SD1-008 | docs/STATUS.md:473 | USB02 P11 kein F3-Rueckfall nach Timeout; P10 gestoppt | obsolete | „P10 gestoppt“ durch Z. 472 (Anwenderentscheid) ueberholt; P11-Regel in usb-snapshot.md §10 + TASK/USB02.md | archive |
| SD1-009 | docs/STATUS.md:474 | USB01 F3: mehrere HID-Interfaces, Layout v2 | obsolete | Spec hid-mouse-protocols.md §9 (APPROVED 2026-10-24); Pi P1–P5 offen in TASK/USB01.md (OFFEN, P1) | archive |
| SD1-010 | docs/STATUS.md:475 | Umfang 1.0: FAT16 und Ton | decision | Duplikat Auto-Memory version-roadmap-1-0-2-0.md (Update 2026-10-23, inkl. „Messrueckstand keine Schranke“) | archive |
| SD1-011 | docs/STATUS.md:476 | USB01 Schritt 2: `USBSNAP.NCP`, `--usb-extra`, MUTE-Fix | obsolete | Spec docs/api/usb/usb-snapshot.md §4/§9.1; MISSING-Treffer sind Dateinamensmuster; offene Pi-Punkte in TASK/USB01.md | archive |
| SD1-012 | docs/STATUS.md:477 | USB01 Schritt 2: `Ssystem(2022)` Kernelteil | obsolete | Spec usb-snapshot.md (K1–K9); Rest in TASK/USB01.md | archive |
| SD1-013 | docs/STATUS.md:478 | Pi-Abnahme USB01 F1/F2/F8 + CON04 | obsolete | CON04-STATUS-DONE.MD vorhanden; Lauf zitiert | archive |
| SD1-014 | docs/STATUS.md:479 | USB01 F1/F2/F8: Link-TRB, nur Boot-HID, Grenzen geloggt | obsolete | Stand in TASK/USB01.md; xHCI-Quellen unter references/xhci | archive |
| SD1-015 | docs/STATUS.md:480 | CON04: `Bconout(2)` VT52 wie EmuTOS | obsolete | CON04 DONE; Q3-Restabweichung in docs/api/bios/_vt52.md, 03-bconout.md, COMPATIBILITY.md; `Cursconf`-Korrektur in Spec | archive |
| SD1-016 | docs/STATUS.md:481 | SDK01 K: Linux-Abnahme an ragnar76 | decision | sdk/host-report.sh, docs/sdk/17-linux-test.md vorhanden; „Abnahme Linux offen“ in TASK/SDK01.md:3 | archive |
| SD1-017 | docs/STATUS.md:482 | A116: `appl_write` blockiert auf voller Queue | obsolete | TASK/A116.md ERLEDIGT; A75-Eingrenzung in COMPATIBILITY.md | archive |
| SD1-018 | docs/STATUS.md:483 | LIBC-BUF: umgelenktes stdout behaelt 32 KiB | obsolete | TASK/LIBC-BUF.md ERLEDIGT; Overlay sdk/libc/overlay/stdio/vfprintf.c vorhanden; mintlib-Cites Checker-Rauschen | archive |
| SD1-019 | docs/STATUS.md:484 | SDK01 K/L: Rust raus aus dem C-Weg, Linux-Infos | obsolete | Stand in TASK/SDK01.md; `ld.lld` = Werkzeugname | archive |
| SD1-020 | docs/STATUS.md:485 | Release 0.6.1 (Alpha) | obsolete | Tag `v0.6.1` vorhanden, docs/release/v0.6.1/; MISSING = Paketdateinamen | archive |
| SD1-021 | docs/STATUS.md:486 | GH1: `wind_update` als Mutex | obsolete | TASK/GH1.md ERLEDIGT, Pi abgenommen 2026-10-22 („Pi offen“ im Eintrag ueberholt); Restabweichungen in GH1.md | archive |
| SD1-022 | docs/STATUS.md:487 | SDK01 fuer Fremde abgeschlossen, HypView-Abnahme | obsolete | TASK/SDK01.md Zeile H nennt HypView-Folgearbeit; `6f3eeeb4` = Upstream-Pin, kein Defekt | archive |
| SD1-023 | docs/STATUS.md:488 | LIBC-MAIN: crt0 ruft mintlibs `_main` | obsolete | LIBC-MAIN-STATUS-DONE.MD; „alle Programme neu linken“ transient | archive |
| SD1-024 | docs/STATUS.md:489 | FS11: Zeichengeraete-Weg `Fread`/`Fwrite` | obsolete | FS11 DONE; Entscheidungen in docs/api/gemdos/file/3f-fread.md; Typeahead-Verweis auf CON01 durch Z. 521 erledigt | archive |
| SD1-025 | docs/STATUS.md:490 | FS11-Nachtrag: AUX = „Port da, kein Kabel“ | decision | Entscheidung in docs/api/bios/01-bconstat.md:245, 02-bconin.md:279, 03-bconout.md:248 | archive |
| SD1-026 | docs/STATUS.md:491 | LIBC-TTY: mintlibs `isatty` | obsolete | DONE; Entscheidungen docs/analysis/libc-tty-2026-10-13.md; LIBC-IOCTL Ticket vorhanden | archive |
| SD1-027 | docs/STATUS.md:492 | MEM05 A: MD-Vorrat waechst, Grenze je Prozess | decision | ADR-025 (MEMORY.md:2131) + COMPATIBILITY.md (5 Treffer); `mdlimit` in kernel/src/bootargs.rs; MEM10 Ticket vorhanden | archive |
| SD1-028 | docs/STATUS.md:493 | MEM05 B: mintlib-Hunks bis 16 MiB | obsolete | MEM05-B DONE; ADR-025-Status + docs/sdk/07-mintlib-limits.md §10, COMPATIBILITY.md:829 | archive |
| SD1-029 | docs/STATUS.md:494 | DESK43 Neuschnitt + U5a Schutzinstrumente | obsolete | DESK43-U5a DONE; Instrumente (tools/desk-drift-probe.sh, `xtask emutos-consts`, `// NEWTOS:`, `svc-seq-trace`) in docs/api/desktop/_rebuild-plan.md | archive |
| SD1-030 | docs/STATUS.md:495 | DESK43 U6a: erstes Rust-Programm in EL0 | obsolete | DESK43-U6a DONE; PROC05 DONE | archive |
| SD1-031 | docs/STATUS.md:496 | DESK43 U5b: AES-Typen in `tos_types::aes` | obsolete | DONE; DESK59 DONE, SPEC01 Ticket vorhanden | archive |
| SD1-032 | docs/STATUS.md:497 | DESK43 U7: Zellmasse ueber `graf_handle` | obsolete | DONE | archive |
| SD1-033 | docs/STATUS.md:498 | DESK43 U8a: Desktop ohne Kernel-Interna | obsolete | DONE; `kernel/src/desk/` jetzt programs/desktop/ | archive |
| SD1-034 | docs/STATUS.md:499 | DESK43 U6b: Trap-Bindungen in `newtos-gem` | obsolete | DONE | archive |
| SD1-035 | docs/STATUS.md:500 | DESK43 U8b: Desktop wird Bibliothek `desktop` | obsolete | DONE; Hinweis „kernel/src/desk historisch“ kollidiert mit CLAUDE.md:185 und 8 Auto-Memory-Dateien (siehe Systemisch) | archive |
| SD1-036 | docs/STATUS.md:501 | DESK43 U9-0: D5/D6, PROC05 | obsolete | D5 (`desk=kernel`) durch U10 Z. 511 geloescht; D6 in ADR-026 (MEMORY.md:2240) | archive |
| SD1-037 | docs/STATUS.md:502 | DESK43 U9-1/2: NEWDESK eingebettet | obsolete | DONE | archive |
| SD1-038 | docs/STATUS.md:503 | DESK43 U9-3..5: Desktop bootet als EL0 | obsolete | DONE; Rueckfall „eingebauter Desktop“ durch Z. 511 (EmuCON) ersetzt | archive |
| SD1-039 | docs/STATUS.md:504 | Pi-Abnahme U10 + DESK62 | obsolete | DESK62, DESK64 DONE | archive |
| SD1-040 | docs/STATUS.md:505 | DESK59: `act_chg` prueft State | obsolete | DONE; Scratch-Regel in Auto-Memory subagent-scratch-in-job-tmp.md; DESK61 Ticket | archive |
| SD1-041 | docs/STATUS.md:506 | DESK58: `valid_drive` gegen 26 | obsolete | DONE; Folge FS12 OFFEN (TASK/FS12.md) | archive |
| SD1-042 | docs/STATUS.md:507 | T16 + DESK60 | obsolete | beide DONE | archive |
| SD1-043 | docs/STATUS.md:508 | PROC06: Exitcode zero-extended | obsolete | DONE; Spec 4c-pterm.md Exit-Code-Matrix | archive |
| SD1-044 | docs/STATUS.md:509 | DESK43 U9-6 Pi-Abnahme | obsolete | HW-Abnahme, erledigt | archive |
| SD1-045 | docs/STATUS.md:510 | LIBC-FSTAT: `fstat` folgt `isatty` | obsolete | DONE; Entscheidung docs/analysis/libc-fstat-2026-10-17.md; CON02/FS13 erledigt | archive |
| SD1-046 | docs/STATUS.md:511 | DESK43 U10: Kernel-Desktop + `desk=kernel` geloescht | obsolete | ADR-026 haelt die Entscheidung | archive |
| SD1-047 | docs/STATUS.md:512 | DESK43 U10 Tester PASS | obsolete | Hinweis APITEST.NCP-Stand 106 transient, nicht nachgeprueft | archive |
| SD1-048 | docs/STATUS.md:513 | ADR-026 angenommen | decision | Duplikat MEMORY.md:2240 ADR-026 | archive |
| SD1-049 | docs/STATUS.md:514 | `pd::DESKTOP` → `pd::KERNEL_CONTEXT` | fact | Code nutzt `pd::KERNEL_CONTEXT` (tos-api/src/bdos/process/sched.rs:70, aes/resource.rs:597); Begriff im Code dokumentiert | archive |
| SD1-050 | docs/STATUS.md:515 | DESK62: Drop-Start verlaesst Desktop | obsolete | DONE; `fun_desk2desk`-Rest als DESK63 (OFFEN) | archive |
| SD1-051 | docs/STATUS.md:516 | GitHub oeffentlich, filter-repo, alte Hashes ueber Commit-Map | fact | `tools/oldhash.sh` und `docs/commit-map.txt` vorhanden; die Konvention steht sonst nur im Kopfkommentar von tools/oldhash.sh, nicht in MEMORY.md/CLAUDE.md → vor Archivierung als Konvention nach MEMORY.md (oder CLAUDE.md) | archive |
| SD1-052 | docs/STATUS.md:517 | FS18 + FS14: `Frename` wie `xrename` | obsolete | FS14/FS18.md ERLEDIGT; OFD-Abweichung in COMPATIBILITY.md; FS17 Ticket | archive |
| SD1-053 | docs/STATUS.md:518 | PROC03 B5 (2): Accessory sieht `run` | obsolete | PROC03 DONE; Abweichung in COMPATIBILITY.md (B5/`os_run`) | archive |
| SD1-054 | docs/STATUS.md:519 | PROC04 P5: `Fdelete` offener Dateien | obsolete | PROC04 DONE; Abweichung in COMPATIBILITY.md (6× `Fdelete`) | archive |
| SD1-055 | docs/STATUS.md:520 | Release 0.6 (Alpha) | obsolete | Tag `v0.6`, docs/release/v0.6/; „Repository bleibt privat“ durch Z. 516 ueberholt | archive |
| SD1-056 | docs/STATUS.md:521 | CON01: Typeahead, Ctrl-S/Q/X, Tab | obsolete | TASK/CON01.md ERLEDIGT; `Cconrs`-Abweichung in COMPATIBILITY.md | archive |
| SD1-057 | docs/STATUS.md:522 | PROC04 P1 = CON02: Umlenkung | obsolete | CON02 ERLEDIGT; FS16 Ticket | archive |
| SD1-058 | docs/STATUS.md:523 | ADR-027: Kernspeicher nach FreeMiNT | decision | Duplikat MEMORY.md:2357 + Auto-Memory kernel-memory-follows-freemint.md | archive |
| SD1-059 | docs/STATUS.md:524 | PROC04 P4: 75 offene Dateien | obsolete | PROC04 DONE | archive |
| SD1-060 | docs/STATUS.md:525 | Pi RST01; Anwenderentscheide B5 und Fdelete | obsolete | Entscheide umgesetzt in Z. 518/519, benannt in COMPATIBILITY.md | archive |
| SD1-061 | docs/STATUS.md:526 | PROC04 P2: Laufwerk/Verzeichnis je Prozess | obsolete | PROC04 DONE | archive |
| SD1-062 | docs/STATUS.md:527 | PROC03 Stufe 1: Elternkette, `Pterm` | obsolete | Spec docs/api/gemdos/process/_run-ownership-curdir.md | archive |
| SD1-063 | docs/STATUS.md:528 | RST01 + X04: Neustart, OSHEADER-Einsprung | obsolete | beide DONE; RST02, INPUT02 als Tickets offen | archive |
| SD1-064 | docs/STATUS.md:529 | X01 abgenommen | obsolete | X01 DONE (zusaetzlich alte UNFINISHED-Datei) | archive |
| SD1-065 | docs/STATUS.md:530 | M08 erledigt | obsolete | M08 DONE (zusaetzlich alte UNFINISHED-Datei) | archive |
| SD1-066 | docs/STATUS.md:531 | A113 behoben, A114 geklaert | obsolete | beide DONE; Folge A116 erledigt | archive |
| SD1-067 | docs/STATUS.md:532 | Pi-Abnahme A112, DESK48, ARCH02 | obsolete | HW-Abnahme, erledigt | archive |
| SD1-068 | docs/STATUS.md:533 | A112: AES-Weckwege wie EmuTOS | obsolete | A112 DONE; Offenpunkt „Slot statt `rlr`“ durch A115 (Z. 469) erledigt | archive |
| SD1-069 | docs/STATUS.md:534 | A112 Audit; DESK48 Variante (b) | decision | DESK48 (b) in COMPATIBILITY.md, Rest DESK67 Ticket; `event.rs:717` = tos-api/src/aes/event.rs, Zeile historisch | archive |
| SD1-070 | docs/STATUS.md:535 | Pi-Abnahme DESK64 | obsolete | DESK66 Ticket vorhanden | archive |
| SD1-071 | docs/STATUS.md:536 | Sichtung offene Liste | obsolete | Analyse docs/analysis/open-tickets-review-2026-10-17.md | archive |
| SD1-072 | docs/STATUS.md:537 | Sichtung UNFINISHED-Tickets | obsolete | Analyse docs/analysis/unfinished-tickets-review-2026-10-17.md | archive |
| SD1-073 | docs/STATUS.md:538 | DESK64: `ADBKGND` mit Vorschau | obsolete | DONE; `desk_rsc.c` generiert/ungetrackt (2589 Zeilen, Cite im Bereich); Sofort-Speichern-Abweichung in docs/api/desktop/deskinf.md + DESK65 | archive |
| SD1-074 | docs/STATUS.md:539 | MEM02 geschlossen | obsolete | MEM02 DONE; Tests tos-api/tests/mem02_* vorhanden | archive |
| SD1-075 | docs/STATUS.md:540 | ARCH02: FP/SIMD ueber `Pexec` | obsolete | ARCH02 DONE | archive |
| SD1-076 | docs/STATUS.md:541 | DESK48: kein `redraw_desktop()` | obsolete | DONE; offene Anwenderfrage durch Z. 534 (Variante b) entschieden | archive |
| SD1-077 | docs/STATUS.md:542 | A25 geschlossen | obsolete | A25 DONE; Randbeobachtung `MU_MESAG` vermutlich durch A112 F2 erledigt, nicht nachgeprueft | archive |
| SD1-078 | docs/STATUS.md:543 | FS10: `Fseek` auf Geraete-Handles | obsolete | FS10 DONE; Spec korrigiert | archive |
| SD1-079 | docs/STATUS.md:544 | ARCH03 auf `main` abgenommen | obsolete | `asid=on` auf Pi offen steht in TASK/ARCH03.md:3 | archive |
| SD1-080 | docs/STATUS.md:545 | Vorfuehrkarte 0.5 | obsolete | Tag `v0.5`; `2afed1e4` = Image-sha; Cookie-Frage durch Z. 546 + kernel/src/lib.rs:354 beantwortet | archive |
| SD1-081 | docs/STATUS.md:546 | Produktversion 0.5, Versionsschema | decision | Schema + „TOS-sichtbare Versionen nie“ in Auto-Memory version-roadmap-1-0-2-0.md; `NTOS` 0x0100 in kernel/src/lib.rs:354; Version heute 0.6.1 | archive |
| SD1-082 | docs/STATUS.md:547 | About-Box ohne Autorenzeile | decision | kein „Adapted by“ in programs/desktop; Kommentar kernel/src/initinfo.rs:91; Pfad `kernel/src/desk/inf.rs` historisch | archive |
| SD1-083 | docs/STATUS.md:548 | Startbildschirm ohne Autorenzeile | obsolete | Aussage „About-Box behaelt die Zeile“ durch Z. 547 ueberholt | archive |
| SD1-084 | docs/STATUS.md:549 | Startbildschirm im mittleren Drittel | decision | Code kernel/src/initinfo.rs:270 (`sw / 3`) traegt es | archive |
| SD1-085 | docs/STATUS.md:550 | Inventur, RTC01, Vorfuehrung andere Aufloesung | obsolete | RTC01 Ticket + Auto-Memory rtc-ds3231-plan.md; Videoplayer in atari-known-programs-preferred.md; `hdmi_mode` in docs/HARDWARE-PROFILE.md | archive |
| SD1-086 | docs/STATUS.md:551 | DOOM: vier Zusatztasten | obsolete | Code sdk/apps/doom/doomgeneric_gem.c | archive |
| SD1-087 | docs/STATUS.md:552 | `main` traegt den Arbeitsstand | fact | Duplikat Auto-Memory worktree-isolation-orphan-branch.md („Arbeitsbranch seit 2026-10-08 = main“) | archive |
| SD1-088 | docs/STATUS.md:553 | Vorfuehrkarte auf Pi abgenommen | obsolete | `2afed1e4` = Image-sha; BALL01 Ticket | archive |
| SD1-089 | docs/STATUS.md:554 | KEYOWNER; `Ssystem` 2020 | obsolete | KEYOWNER DONE; 2020 in docs/api/gemdos/154-ssystem.md | archive |
| SD1-090 | docs/STATUS.md:555 | SYSINFO02: Startoptionen einstellen | obsolete | DONE; Offenpunkt Tasten durch KEYOWNER (Z. 554) erledigt | archive |
| SD1-091 | docs/STATUS.md:556 | INPUT01: Zeiger bewegt sich waehrend Arbeit | obsolete | DONE; `input=aes` in kernel/src/bootargs.rs:73 | archive |
| SD1-092 | docs/STATUS.md:557 | ZVIEW04: `MN_SELECTED` mit Baum/Eltern | obsolete | DONE | archive |
| SD1-093 | docs/STATUS.md:558 | MEMPROT-FAST | obsolete | DONE; `memprot=verify` in kernel/src/bootargs.rs:39 | archive |
| SD1-094 | docs/STATUS.md:559 | SYSINFO01: `Ssystem` 2003–2019 | obsolete | DONE; Spec docs/api/gemdos/154-ssystem.md | archive |
| SD1-095 | docs/STATUS.md:560 | FS09 Block-Cache; Takt-Ruecknahme | trap | Cache-Abweichung in docs/api/bios/04-rwabs.md; Takt-Lehre in Auto-Memory dont-reverse-on-one-indirect-number.md; `cpuclk=firmware` kernel/src/bootargs.rs:63. „Warum CoreMark 7380 am 04.09.“ bleibt ungeklaerte Hypothese ohne Ticket | archive |
| SD1-096 | docs/STATUS.md:561 | FS08 T4: 32 KiB stdio-Puffer | obsolete | FS08-STDIO DONE; `st_blksize`-Regel in docs/sdk/07-mintlib-limits.md, Overlay quickstat.c | archive |
| SD1-097 | docs/STATUS.md:562 | ZVIEW02: Doppelklick am AES-Tick | obsolete | DONE; A111 DONE | archive |
| SD1-098 | docs/STATUS.md:563 | ZVIEW01: Browser zeigt Namen | obsolete | DONE | archive |
| SD1-099 | docs/STATUS.md:564 | FS08 T3: ARM-Takt auf Firmware-Maximum | decision | FS08-CPUCLK DONE; Quelle docs/hardware/mailbox-framebuffer.md §3f; Standard bestaetigt durch Z. 560 | archive |
| SD1-100 | docs/STATUS.md:565 | FS08 T2: Datei-I/O wie EmuTOS | obsolete | FS08-FAT DONE; `dir_nth_sector`-Rest nur in TASK/FS08-FAT-STATUS-DONE.MD; HW03 DONE | archive |
| SD1-101 | docs/STATUS.md:566 | FS08 T1: EMMC Mehrblock, 4 Bit | obsolete | FS08-EMMC DONE; `emmc=compat` im Code | archive |
| SD1-102 | docs/STATUS.md:567 | Accessory-Titel im Desk-Menue, zView-Absturz | obsolete | erledigt; Folgetickets ZVIEW01/FS08/INPUT01 DONE | archive |
| SD1-103 | docs/STATUS.md:568 | zView M4 abgeschlossen, K1 AES-Workstation | obsolete | ZVIEW-M4 DONE; Offenpunkt weiter wahr: docs/api/vdi/workstation/100-v_opnvwk.md:423 sagt `HANDLE_COUNT = 16`, Code 64 (tos-api/src/vdi/vwk.rs:22), offen in docs/api/vdi/_discrepancies.md D-VDI-3 | archive |
| SD1-104 | docs/STATUS.md:569 | zView M4 teilweise, blockiert durch K1 | obsolete | durch Z. 568 ueberholt | archive |
| SD1-105 | docs/STATUS.md:570 | zView M3: libshare als Modul-Allokator | obsolete | LDG-M3 DONE; Lizenzfrage in TASK/LDG-M3-B2-STATUS-DONE.MD:11; SLB01 Ticket | archive |
| SD1-106 | docs/STATUS.md:571 | AES: Tastenzustand zum Ereigniszeitpunkt | obsolete | LDG-M3-B1 DONE; Rest A111 DONE | archive |
| SD1-107 | docs/STATUS.md:572 | zView M3 teilweise, WinDom | obsolete | B1/B2 durch Z. 571/570 erledigt | archive |
| SD1-108 | docs/STATUS.md:573 | zView M2: Bildbibliotheken | obsolete | LDG-M2 DONE | archive |
| SD1-109 | docs/STATUS.md:574 | LDG M1: `.NLD` nativ | obsolete | LDG-M1 DONE; `.NLD` in ADR-018 Amendment (MEMORY.md:1088) | archive |
| SD1-110 | docs/STATUS.md:575 | PORT09: argv/environ, `ARGV=` | obsolete | PORT09 DONE (alte UNFINISHED-Datei daneben) | archive |
| SD1-111 | docs/STATUS.md:576 | mintlib `<sys/wait.h>` little endian | obsolete | Overlay; `system()`-Hinweis in docs/sdk/07 §8 | archive |
| SD1-112 | docs/STATUS.md:577 | ARCH03-4: ASIDs hinter `asid=on` | decision | Standard aus bestaetigt (kernel/src/lib.rs:256); Plan nach Pi-Lauf in MEMORY.md:2495ff (QUIRK-001-Update) + TASK/ARCH03.md:3 | archive |
| SD1-113 | docs/STATUS.md:578 | PROC02: Kind erbt Standardhandles | obsolete | DONE; Spec 45-fdup.md; Reste via PROC04 erledigt | archive |
| SD1-114 | docs/STATUS.md:579 | PPARENT Weg A | obsolete | DONE; Spec docs/api/gemdos/process/_parent-and-environment.md | archive |
| SD1-115 | docs/STATUS.md:580 | ARCH03 nach E9: crt0 `exit()`, `tfork`-Fix | obsolete | ARCH03-POSTE9 DONE; Analyse vfork-malloc-ownership-2026-10-06.md | archive |
| SD1-116 | docs/STATUS.md:581 | ARCH03 E9: TPA = groesster Block | obsolete | ARCH03-E9 DONE | archive |
| SD1-117 | docs/STATUS.md:582 | ARCH03: Prozess-/Speicherarchitektur nach FreeMiNT | decision | Entscheidung nur als „ADR-025 Proposed“ in docs/analysis/arch03-implementation-plan.md:28-31; MEMORY.md-ADR-025 ist MEM05 (Kollision), ADR-021 (MEMORY.md:1411) weiter „Accepted“ → vor Archivierung ARCH03-ADR mit neuer Nummer nach MEMORY.md, ADR-021 superseded; `ba6172a1` = Image-sha | archive |
| SD1-118 | docs/STATUS.md:583 | zView zurueckgestellt; ARCH03 geplant | obsolete | durch Z. 582 umgesetzt; gleiche ADR-025-Kollision; MEM04 erledigt | archive |
| SD1-119 | docs/STATUS.md:584 | Pi KEYS02 + DESK50; PORT06 fertig | obsolete | `1b00a7d5`/`b92e4d58` = Image-sha; DONE-Quittungen vorhanden | archive |
| SD1-120 | docs/STATUS.md:585 | TT-Doom: keine weitere Optimierung | decision | in TASK/PORT06-STATUS-DONE.MD und PORT06.md | archive |
| SD1-121 | docs/STATUS.md:586 | KEYS02 Doom-Seite, TT-Arbeit | trap | Doom-Optimierungsregel in Auto-Memory port-target-app-unmodified.md:40; Messung „TOS 3.06 hat `kbdvec`“ (measurements/measurement-2026-10-04-keys02-tt) fehlt in docs/api/xbios/22-kbdvbase.md:219 (dort nur „ABGELEITET“) → vorher in die Spec | archive |
| SD1-122 | docs/STATUS.md:587 | KEYS02 Kern: `Kbdvbase`-Haken | obsolete | KEYS02 DONE; Spec 22-kbdvbase.md; Abweichung in COMPATIBILITY.md (`kbdvec`) | archive |
| SD1-123 | docs/STATUS.md:588 | Pi-Abnahme GUI07 + KEYS01 | obsolete | `e7636f73` = Image-sha; K4 durch KEYS02 erledigt | archive |
| SD1-124 | docs/STATUS.md:589 | KEYS01: Tastenwiederholung | obsolete | KEYS01 DONE; Abweichung „Pump statt Timer“ in COMPATIBILITY.md | archive |
| SD1-125 | docs/STATUS.md:590 | GUI07: `vro_cpyfm` zeilenweise | obsolete | GUI07 DONE; `16b_blit.c` = fVDI, nicht im Baum | archive |
| SD1-126 | docs/STATUS.md:591 | Pi-Runde Doom, SER01, Tastatur-Diagnose | obsolete | SER01 DONE; `serial=on` in Auto-Memory raspi4b-runs-need-serial-on.md; Diagnose TASK/PORT06-KEYS-DIAGNOSIS.MD | archive |
| SD1-127 | docs/STATUS.md:592 | PORT06: Doom x2–x4, libc-Endungen, EdDI | obsolete | PORT06 DONE; Endungsregel Auto-Memory own-extensions-separate-aarch64-from-m68k.md; Hatari-Pfad jetzt docs/analysis/ORACLE.md §1 | archive |
| SD1-128 | docs/STATUS.md:593 | PORT10: libc `%f`, `intptr_t`, `system()` | obsolete | DONE; Endungsfrage durch Z. 592 beantwortet; `commit -am`-Lehre in Auto-Memory no-commit-all-while-worker-runs.md; PROC01 durch ARCH03-2 erledigt | archive |
| SD1-129 | docs/STATUS.md:594 | PORT06 Schritt 1: Doom laeuft | obsolete | „nicht auslieferbar“ durch Z. 593 ueberholt; `dcb7a8db` = Upstream-Pin | archive |
| SD1-130 | docs/STATUS.md:595 | PORT09 angehalten, MEM03 neu | obsolete | MEM03 ERLEDIGT (ARCH03-1), PORT09 Z. 575; Quellen-Cites in TASK/PORT09-* | archive |
| SD1-131 | docs/STATUS.md:596 | X03: Maschinenzeile als Bau-Konstante | decision | Code programs/desktop/src/inf.rs:2614-2616 traegt es; Offenpunkt weiter wahr: docs/api/desktop/deskmain.md:1358-1363 („No build date“) seit DESK12 veraltet, nur in TASK/X03-STATUS-DONE.MD:75 notiert | archive |
| SD1-132 | docs/STATUS.md:597 | T14: Mauszeiger im Testbild = Testfehler | trap | T14 DONE; Rest-Riegel in TASK/T15.md (OFFEN) | archive |

## STATUS.md Recent Decisions, Teil 2 (2026-09-13 bis 2026-10-01)

Quelle: docs/STATUS.md Zeilen 598–729 (132 Tabellenzeilen). Checker: OK=1313, OLD-HASH-MAPPED=72, MISSING=209, AMBIGUOUS=119, BAD-COMMIT=8, LINE-OUT-OF-RANGE=3, EMUTOS-COMMIT=2, FREEMINT-COMMIT=1 (2 Zeilen Parser-Rauschen).

### Systemische Befunde
- Alle 8 BAD-COMMIT sind keine Repo-Commits: Kern-Kennungen der Pi-Runden (`2597951a` Z. 602, `92ec0ab7` Z. 611, `14065b06` Z. 614), Upstream-Pins (`6177984e` circle, references/README.md:294; `ce57817`/`2ac3e85` cflib, sdk/cflib/build-cflib.sh:36-38; `5fe39974` OpenLibm-Tag). Checker-Rauschen.
- MISSING/AMBIGUOUS fast nur Rauschen: rund 40 Cites auf `kernel/src/desk/*` (seit U8b, Z. 500, `programs/desktop/`), `desk_rsc.c`/`desk_rsc.h` (generiert, ungetrackt in references/emutos/desk), blanke Dateinamen aus TeraDesk/QED/mintlib/cflib/circle/FreeMiNT (`applik.c`, `startprg.c`, `poslist.c`, `crtinit.c`, `lib/dmachannel.cpp`, `sys/ssystem.c` …), Registerfelder (`CS.INT`, `SPSR.M`, `w.curr`). Die 3 LINE-OUT sind erklaerbar: `deskpro.c:71-121` ist die in Z. 602 selbst widerrufene Fehlangabe, `main.rs:1474` meint den alten Desktop-`main.rs`, `usermem.rs:583` ist seit ARCH03 verschoben (`resolve_kva` jetzt :308). Echte, aber historische Pfadfehler: Z. 655 `measurement-2026-09-21-t04/tera-after-A-idle.png` liegt unter `screendumps/`; Z. 676/677/697 nennen `*-STATUS-UNFINISHED.MD`, die durch DONE-Quittungen ersetzt sind.
- Alle 132 Zeilen sind abgeschlossene Ticket-Historie (Ticket ERLEDIGT/DONE, T13 ZURUECKGESTELLT); offene Folgepunkte haben eigene Tickets (HW05/HW06/HW07, SND01/SND02, REF03, T08, T10, T11, T15, DESK49, ACC08). Der Block Z. 615–651 und 717 beschreibt den EL1-Desktop in `kernel/src/desk/` samt Shell-Loop-Helfern, die U3 (Z. 616) geloescht und U8b (Z. 500) verlegt hat: durchgehend `obsolete`.
- Widerrufe: Z. 682 (A75, `appl_write` auf volle Queue → 0) durch Z. 482 (A116, `b7be9bb1`, COMPATIBILITY.md:500-504); Z. 605 (Ton nach dem OFAM) durch Z. 475 (1.0 = FAT16 + Ton); Z. 661 („Alpha-Tag nicht gesendet“) durch Z. 660; Z. 726 („ADR-022 ist aufgehoben, TeraDesk ist der Desktop“) widerspricht docs/MEMORY.md:1520-1523 (ADR-022 Accepted) und Auto-Memory teradesk-is-the-desktop.md (Korrektur 2026-09-17).
- Wissen, das vor dem Archivieren umziehen muss: (a) HW03-Abweichung „evict haelt `dirty` bei Fehlschlag fest, EmuTOS verwirft“ steht nur in tos-api/src/bdos/fs/cache.rs:237 → COMPATIBILITY.md; (b) AESGLOBAL-Ausnahme (m68k-Ablage auf jeder ABI, Z. 727) steht in docs/api/aes/application.md:289ff/:1174, aber nicht im Abschnitt COMPATIBILITY.md:1060 „Native vs m68k struct variants“ → dort benennen; (c) falscher Kopfkommentar `tosdump/cases/aes-m07-parameter.tsv:39-44` nur in TASK/V17-STATUS-DONE.MD:227 → tosdump/cases/ERRATA.md; (d) AES-Fiber-Stack ohne Guard-Page nur in tos-api/src/aes/thread.rs:95-100 und TASK/A21-STATUS-DONE.MD:139, kein Ticket; (e) REF02-Offenpunkte (BCM2835 ARM Peripherals fehlt, SOURCE-ANALYSIS-PROTOCOL Track B ohne SD-Spezifikationen, dort weiter 0 Treffer) nur in TASK/REF02-STATUS-DONE.MD:336-344, kein Ticket. Die Titelkopie-Abweichung von `mn_register` (Z. 626) steht nur im Code (tos-api/src/aes/menu.rs:1173).

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| SD2-001 | docs/STATUS.md:598 | T12: Riegel fuer `sched_*`-Testdateien | obsolete | T12-STATUS-DONE vorhanden; Nebenfund T14 DONE | archive |
| SD2-002 | docs/STATUS.md:599 | DESK50: Fensteroeffnen ueber `do_wopen`/`do_diropen` | obsolete | DESK50 DONE; Spec docs/api/desktop/desksupp-wopen.md; `kernel/src/desk` = Historie | archive |
| SD2-003 | docs/STATUS.md:600 | DESK41 Pi-Abnahme aus TeraDesk | obsolete | DESK41 DONE; DESK49 OFFEN (eigenes Ticket), PORT09 DONE | archive |
| SD2-004 | docs/STATUS.md:601 | T13 Pause gebaut, verworfen, Lehren | trap | T13 ZURUECKGESTELLT; Lehren in Auto-Memory card-write-only-what-user-says.md, injector-does-not-run-during-cconin.md, auto-folder-console-only.md | archive |
| SD2-005 | docs/STATUS.md:602 | DESK47: `opn_appl`-Dialog fuer `.NTP` | obsolete | DESK47 ERLEDIGT; LINE-OUT `deskpro.c:71-121` ist die im Eintrag widerrufene Angabe; `2597951a` = Kern-Kennung | archive |
| SD2-006 | docs/STATUS.md:603 | REF04: SD-Tabelle umformuliert, Doc-Comments mitsuchen | trap | REF04 ERLEDIGT; Lehrsatz in references/README.md; `cargo clean`-Falle in docs/STATUS.md:52-55; Auto-Memory justification-is-copied-to-several-places.md | archive |
| SD2-007 | docs/STATUS.md:604 | DESK41: `TERADESK.INF` im Abbildbau erzeugt | obsolete | Inhalt in tools/make-sd-image.sh; TeraDesk-Cites (`desk.h`, `prgtype.c`) liegen unter sdk/apps/teradesk (Rauschen) | archive |
| SD2-008 | docs/STATUS.md:605 | Ton erst nach OFAM, circle als Rang 6 | obsolete | Zeitplan durch Z. 475 ueberholt (SND01/SND02 OFFEN, „gehoert zu 1.0“); Rang-6-Bewertung in TASK/SND02.md:72-75; `6177984e` = circle-Pin | archive |
| SD2-009 | docs/STATUS.md:606 | SND01 Track B: PWM ohne Interrupt | obsolete | Inhalt in docs/hardware/pwm-audio.md; `CS.INT`/`TI.INTEN` sind Registerfelder | archive |
| SD2-010 | docs/STATUS.md:607 | SND01: DMA-Sound-Gruppe extrahiert | obsolete | Spec docs/api/xbios/_dmasound-group.md; `sys/xdd/audio/falcon.c` = FreeMiNT-relativ | archive |
| SD2-011 | docs/STATUS.md:608 | SCHED01: `dsptch` nimmt Aufrufer auf | obsolete | Begruendung in tos-api/src/bdos/process/sched.rs:1009; Spec accessories.md | archive |
| SD2-012 | docs/STATUS.md:609 | PORT07: Ball-Demo ueber `vro_cpyfm` | obsolete | Ball-Demo liegt ausserhalb des Repos (Auto-Memory ball-demo-as-benchmark.md), daher MISSING `plat-*.c`; GUI07 erledigt (Z. 590) | archive |
| SD2-013 | docs/STATUS.md:610 | GUI06: Neuzeichen-Boden, Cachepflege | obsolete | DESK48 DONE; HW07 OFFEN, Wissen in TASK/HW07.md | archive |
| SD2-014 | docs/STATUS.md:611 | HW-Runde: DESK46, GUI04, U4 abgenommen | obsolete | `92ec0ab7` = Kern-Kennung; DESK47 erledigt (Z. 602) | archive |
| SD2-015 | docs/STATUS.md:612 | REF02: SD-Spezifikationen gepinnt | decision | Pin in references/README.md:109-148,289-292; Offenpunkte BCM2835/Track B nur in TASK/REF02-STATUS-DONE.MD:336-344, kein Ticket → vorher Ticket oder SOURCE-ANALYSIS-PROTOCOL; REF03 OFFEN | archive |
| SD2-016 | docs/STATUS.md:613 | HW03: Schreibfehler erreicht Anwendung | decision | HW03 DONE; EmuTOS-Abweichung (`evict` haelt `dirty`) nur in cache.rs:237, nicht in COMPATIBILITY.md → vorher dort benennen; QEMU-sdhci-Falle in emmc.rs:992-993 | archive |
| SD2-017 | docs/STATUS.md:614 | DESK46: `Pexec`-Registerrestore fuer EL1 | obsolete | DESK46 DONE; FP/SIMD-Luecke durch ARCH02 (DONE 2026-10-17) geschlossen; `14065b06` = Kern-Kennung | archive |
| SD2-018 | docs/STATUS.md:615 | DESK43 U4 + DESK44 | obsolete | U4 DONE; EL1-Fallen (`Cconws`/`ldtrb`, `dos_trap`-Verengung, `desk/xbiosbind.rs`) seit U8b (Z. 500) gegenstandslos; DESK45 erledigt | archive |
| SD2-019 | docs/STATUS.md:616 | DESK43 U3: Desktop ist Prozess, der endet | obsolete | U3 DONE; Lage „auf EL1“ durch Z. 500 ueberholt | archive |
| SD2-020 | docs/STATUS.md:617 | DESK43 U2: Shell-Puffer-Zustandskanal | obsolete | U2 DONE; THEGLO-Befund in tos-api `shell::reinit` und Spec | archive |
| SD2-021 | docs/STATUS.md:618 | ACC10: `wait_for_accs` als `all_run` | obsolete | ACC10 DONE; Spec docs/api/aes/accessories.md §3a/§9.5 | archive |
| SD2-022 | docs/STATUS.md:619 | DESK43 U1: Tracer-Tor, Driftprobe | obsolete | U1 DONE; tools/desk-drift-probe.sh vorhanden; `desk/dosbind.rs` = Historie | archive |
| SD2-023 | docs/STATUS.md:620 | PORT05: EL0-Elter neu gestartet | obsolete | PORT05 DONE; ACC10 (Z. 618) | archive |
| SD2-024 | docs/STATUS.md:621 | DESK42: Show-info-Baeume nach EmuTOS | obsolete | DESK42 DONE; FS06 DONE | archive |
| SD2-025 | docs/STATUS.md:622 | FS04: Timeout ist eine Dauer | decision | Audit docs/api/hw/emmc-audit.md; HW05/HW06 ZURUECKGESTELLT; HW03 Z. 613 | archive |
| SD2-026 | docs/STATUS.md:623 | PORT03: cflib auf `ce57817` gepinnt | decision | Pin in sdk/cflib/build-cflib.sh:36-38,100 und docs/sdk/09-port-material.md:29,58; Vollkopie-Overlay-Falle in docs/sdk/08/09; BAD-COMMIT = Upstream-Hashes | archive |
| SD2-027 | docs/STATUS.md:624 | DESK31/33/38/32: Desktop-Reste | obsolete | alle DONE; T10 TEILWEISE (eigenes Ticket) | archive |
| SD2-028 | docs/STATUS.md:625 | PORT01: crt0 baute `argv` nie | trap | Auto-Memory lp64-tos-struct-trap.md nennt crt0 argc/argv; cflib-Rest durch Z. 623; mintlib-Dateinamen = Rauschen | archive |
| SD2-029 | docs/STATUS.md:626 | ACC02–ACC05: Accessories, FP-Kontext | decision | `.NAC` = ADR-018 (MEMORY.md:1006); Suchorte in accessories.md; Titelkopie-Abweichung nur in menu.rs:1173, nicht COMPATIBILITY.md; ACC07 DONE, ACC08 OFFEN | archive |
| SD2-030 | docs/STATUS.md:627 | REF01 Nachtrag 2: bash-3.2-Heredoc-Bug | trap | Falle in tools/emutos-config.sh:353-361; Verhaltensregel _emutos-config.md:483; LINE-OUT `main.rs:1474` = alter Desktop | archive |
| SD2-031 | docs/STATUS.md:628 | REF01 Nachtrag 1: `sh` = bash im POSIX-Modus | trap | Schutz in tools/emutos-config.sh:81-85; Klassifikation in _emutos-config.md | archive |
| SD2-032 | docs/STATUS.md:629 | REF01: EmuTOS-Konfigtabelle erzeugt | decision | docs/api/_emutos-config.md, SOURCE-ANALYSIS-PROTOCOL.md:142, Auto-Memory emutos-target-profile-falcon-tos4.md; Pin = ORACLE.md §1 | archive |
| SD2-033 | docs/STATUS.md:630 | ACC01: `.ACC`-Lebenszyklus extrahiert | obsolete | Spec docs/api/aes/accessories.md; Luecken ACC02–05 erledigt (Z. 626) | archive |
| SD2-034 | docs/STATUS.md:631 | ACC06: `Ssystem` 2000/2001 | decision | docs/api/gemdos/154-ssystem.md:21-27, mailbox-framebuffer.md:366 §3d; „nicht committet“ veraltet | archive |
| SD2-035 | docs/STATUS.md:632 | DESK37/36/30 | obsolete | DONE; `sh_gem_show` durch U3 (Z. 616) entfernt | archive |
| SD2-036 | docs/STATUS.md:633 | DESK34/35: Ordner im Fenster, Vollschirm | obsolete | DONE; `sh_gem_enter/leave` durch U3 geloescht | archive |
| SD2-037 | docs/STATUS.md:634 | FS01: LFN-Schlitze beim Loeschen | decision | FreeMiNT-Regel in 41-fdelete.md/3a-ddelete.md; `Frename`-Rest = FS02 ERLEDIGT 2026-10-19 | archive |
| SD2-038 | docs/STATUS.md:635 | DESK22/28 gebaut, DESK23/29 zurueckgezogen | obsolete | `sh_alpha_*` durch U3 geloescht; Lehre in Auto-Memory tickets-from-screendumps-misfire.md | archive |
| SD2-039 | docs/STATUS.md:636 | ARCH01: DAIF vor `eret` maskiert | trap | QUIRK-003 in docs/MEMORY.md:2632, ARCHITECTURE.md:138 | archive |
| SD2-040 | docs/STATUS.md:637 | DESK19/20: Ablegen auf Programm, Funktionstaste | obsolete | DONE; EL0-Eintritts-Nebenbefund durch ARCH01 (Z. 636); `process_funkey` durch DESK33 (Z. 624) | archive |
| SD2-041 | docs/STATUS.md:638 | DESK24/25/26 | obsolete | DONE; fsck-Nebenbefund = FS01 (Z. 634) | archive |
| SD2-042 | docs/STATUS.md:639 | DESK16/17/21 | obsolete | DONE; DESK17(2)-Muster in tickets-from-screendumps-misfire.md | archive |
| SD2-043 | docs/STATUS.md:640 | DESK15: Fenstersymbole | obsolete | DONE; Verknuepfungen = DESK30 (Z. 632) | archive |
| SD2-044 | docs/STATUS.md:641 | DESK18: Arbeitsverzeichnis vor Start | obsolete | DONE; Restpunkt durch DESK19 (Z. 637) | archive |
| SD2-045 | docs/STATUS.md:642 | DESK14: Read .INF, Neustart zurueckgestellt | obsolete | Punkt 4 durch U3 (Z. 616, `restart_desktop`) eingeloest | archive |
| SD2-046 | docs/STATUS.md:643 | DESK13: Rueckfragen, `Supexec` nicht gebaut | decision | Lage in docs/api/xbios/26-supexec.md:12-16; Aufruferseite durch X01 (Z. 719); DONE | archive |
| SD2-047 | docs/STATUS.md:644 | DESK12: Baustempel, `os_date` | obsolete | tos-api/build.rs und tos-api/src/build.rs vorhanden; DONE | archive |
| SD2-048 | docs/STATUS.md:645 | DESK06: Execute EmuCON verschachtelt | obsolete | verschachtelter Lauf durch U3 (Z. 616) ersetzt | archive |
| SD2-049 | docs/STATUS.md:646 | DESK05: Konfigurationsdialog, Kuerzel | obsolete | Folgetickets DESK18–21 erledigt | archive |
| SD2-050 | docs/STATUS.md:647 | DESK03/04: Desktop-Symbole als Zustand | obsolete | DONE; `IG_REMOV`-Rest durch DESK16 (Z. 639) | archive |
| SD2-051 | docs/STATUS.md:648 | DESK01b: 3D-Optik der Desktop-Dialoge | obsolete | DONE; Wunsch in Auto-Memory aes-dialog-3d-look.md | archive |
| SD2-052 | docs/STATUS.md:649 | DESK11/01/02 | obsolete | DONE; `_IDT` durch DESK13 (Z. 643), Baudatum durch DESK12 (Z. 644) | archive |
| SD2-053 | docs/STATUS.md:650 | DESK10: Desktop nur ueber Trap | obsolete | durch U8b (Z. 500, EL0-Programm) ueberholt; Auto-Memory desktop-bypasses-app-abi-path.md nennt noch `kernel/src/desk` (Teil Auto-Memory) | archive |
| SD2-054 | docs/STATUS.md:651 | DESK08/09: Dialogkette, `cnx_put/get` | obsolete | DONE | archive |
| SD2-055 | docs/STATUS.md:652 | A104/A110/A91 | decision | A91 in COMPATIBILITY.md:550-570 und event.md:568; A110 in window-graphics.md:1107 | archive |
| SD2-056 | docs/STATUS.md:653 | A106: AES-Sitzung startet neu | obsolete | A106 DONE; Spec shell-loop | archive |
| SD2-057 | docs/STATUS.md:654 | A107/A109: Sweep fuer Pexec-Kind | decision | A107-Regel in docs/api/aes/menu.md:254; DONE | archive |
| SD2-058 | docs/STATUS.md:655 | T05/T06: Basepage-Segmente, Textrott | obsolete | DONE; Ein-Segment-Rest in T05-STATUS-DONE.MD:60; Cite fehlt `screendumps/` (Historie) | archive |
| SD2-059 | docs/STATUS.md:656 | T07: `setjmp` und OpenLibm | decision | OpenLibm in docs/sdk (04, 05, 09, 16); `5fe39974` = OpenLibm-Tag | archive |
| SD2-060 | docs/STATUS.md:657 | T04: LONG-Vertrag in Wortarrays | decision | COMPATIBILITY.md:423-425, docs/sdk/05-abi-pitfalls.md:559 B16; T08 TEILWEISE | archive |
| SD2-061 | docs/STATUS.md:658 | R02–R05: Raster mit Schirm-Operand | obsolete | Spec _raster.md, D-VDI-23; `EMUTOS14-RASTER-screen-falcontc` = 1.4-Evidenz (ORACLE.md §7) | archive |
| SD2-062 | docs/STATUS.md:659 | V-Serie als Geraeteparameter geschlossen | decision | Regel TT/Falcon-TC in Auto-Memory hatari-falcon-tc-is-the-colour-oracle.md und ORACLE.md §2 | archive |
| SD2-063 | docs/STATUS.md:660 | VideoCore alpha mode = ignored | decision | docs/hardware/mailbox-framebuffer.md:266 §3c | archive |
| SD2-064 | docs/STATUS.md:661 | V11/V12: Overlay-Bit im X-Byte | decision | Vertrag in _raster.md „Pi/BCM2711 binding“; „Tag nicht gesendet“ durch Z. 660 ueberholt | archive |
| SD2-065 | docs/STATUS.md:662 | A103/A105/A108 | obsolete | DONE | archive |
| SD2-066 | docs/STATUS.md:663 | A101: ADR-024 nachgetragen | obsolete | ADR-024 in docs/MEMORY.md:1933 | archive |
| SD2-067 | docs/STATUS.md:664 | A100/A102: Dropdown-Rand, `fm_own` | obsolete | DONE; `kernel/src/desk/menu.rs` = Historie | archive |
| SD2-068 | docs/STATUS.md:665 | A96/A97: Fenster-Sweep, NEWDESK-Owner | obsolete | DONE; Desktop-Sonderweg durch U3/U8b ueberholt | archive |
| SD2-069 | docs/STATUS.md:666 | A48 Tier 1: `SHW_RESCHNG` aufgezeichnet | obsolete | durch A106 (Z. 653) vollendet | archive |
| SD2-070 | docs/STATUS.md:667 | A95: Fensterrahmen gegen Falcon | obsolete | DONE | archive |
| SD2-071 | docs/STATUS.md:668 | A94: altes Top „covered“ | obsolete | DONE | archive |
| SD2-072 | docs/STATUS.md:669 | A93/A98: Slider-Nachrichten, Teilbaum | obsolete | DONE | archive |
| SD2-073 | docs/STATUS.md:670 | A89/A90/A92 | obsolete | DONE | archive |
| SD2-074 | docs/STATUS.md:671 | A87/A88: `gl_ctwait`, `menu_popup` | obsolete | DONE; `mfree_tests`-Pool-Falle durch A89 (Z. 670) entschaerft | archive |
| SD2-075 | docs/STATUS.md:672 | A86: Schirm-MFDB-Doku Truecolor | obsolete | ADR-Platzhalter durch A101 (Z. 663) | archive |
| SD2-076 | docs/STATUS.md:673 | A99: `objc_change` per XOR | obsolete | DONE | archive |
| SD2-077 | docs/STATUS.md:674 | A85: nachlaufender XOR in `just_draw` | obsolete | Kompensation durch A99 (Z. 673) entfernt | archive |
| SD2-078 | docs/STATUS.md:675 | A83: `wind_set` haelt Titelzeiger | obsolete | DONE; LINE-OUT `usermem.rs:583` (jetzt :308); `LiveStr` in window.rs:331 | archive |
| SD2-079 | docs/STATUS.md:676 | A84/A82, A83 STOP | obsolete | STOP durch Z. 675 abgeloest; UNFINISHED-Datei ersetzt (erwartet) | archive |
| SD2-080 | docs/STATUS.md:677 | A79: zwei Looks, Entscheid offen | obsolete | durch Z. 678 entschieden | archive |
| SD2-081 | docs/STATUS.md:678 | A79 Abschluss: Literal-Look einziger | decision | window-graphics.md §WF_COLOR; Auto-Memory hatari-falcon-window-frame-reference.md | archive |
| SD2-082 | docs/STATUS.md:679 | A80: `WF_TOP` auf Handle 0 | obsolete | DONE | archive |
| SD2-083 | docs/STATUS.md:680 | A77: `WM_CLOSED`/`WM_FULLED` mit Rechteck | obsolete | DONE; Nebenbefund durch A93 (Z. 669) | archive |
| SD2-084 | docs/STATUS.md:681 | A76: `Mfree` auf Ressourcenblock | decision | COMPATIBILITY.md:349 | archive |
| SD2-085 | docs/STATUS.md:682 | A75: `appl_read` blockiert, `appl_write` 0 | obsolete | Schreibseite durch Z. 482 (A116) widerrufen, COMPATIBILITY.md:500-504 | archive |
| SD2-086 | docs/STATUS.md:683 | A73: `OUTSIDE_STATE`, Arm-Zeit-Test | obsolete | DONE; Park-Falle in Auto-Memory injector-park-must-not-cross-menu-bar.md | archive |
| SD2-087 | docs/STATUS.md:684 | A72: Spec-Rott bereinigt | obsolete | Nebenbefund durch A86 (Z. 672) | archive |
| SD2-088 | docs/STATUS.md:685 | A68/A69/A70: `just_draw` | obsolete | DONE | archive |
| SD2-089 | docs/STATUS.md:686 | A81: NEWDESK-Root nach Zeigerbreite | decision | COMPATIBILITY.md:364-419, 05-abi-pitfalls.md:424 B10 | archive |
| SD2-090 | docs/STATUS.md:687 | A57+A71: persistenter AES-Kratzpuffer | decision | COMPATIBILITY.md:158-207 | archive |
| SD2-091 | docs/STATUS.md:688 | A56: pid 0 fuer Kette, 1 Screen-Manager | decision | Spec application.md „AES process identity“; `WF_OWNER` (window.rs:3911-3929) folgt dem Pin `gemwmlib.c:1579-1588` (ORACLE.md §5); r2-Faelle bezogen auf EMUTOS14 | archive |
| SD2-092 | docs/STATUS.md:689 | A62 Stufe 1: Identitaet am Slot | obsolete | Werte durch A56 (Z. 688) | archive |
| SD2-093 | docs/STATUS.md:690 | A50: `rsrc_gaddr`-Regel | decision | COMPATIBILITY.md:288-339 | archive |
| SD2-094 | docs/STATUS.md:691 | A78: `w_move`-Blit | obsolete | DONE | archive |
| SD2-095 | docs/STATUS.md:692 | A49/A61: HOTCLOSE, `shel_write`-Wort 3 | obsolete | DONE; Nebenbefund durch A77 (Z. 680) | archive |
| SD2-096 | docs/STATUS.md:693 | A47/A52/A55/A58 | obsolete | DONE | archive |
| SD2-097 | docs/STATUS.md:694 | A46: Menuetrenner als Linien | decision | Versionsgatter in object-form.md:392 (`4b0d9518` = EMUTOS-COMMIT) | archive |
| SD2-098 | docs/STATUS.md:695 | A43/A44: ungueltige Selektoren | obsolete | DONE | archive |
| SD2-099 | docs/STATUS.md:696 | A41 Teil 2: RSHDR vor NRSC | obsolete | DONE; Nebenbefund A76 (Z. 681) | archive |
| SD2-100 | docs/STATUS.md:697 | A41 Teil 1: Farbicons, Blocker | obsolete | durch Z. 696 aufgeloest | archive |
| SD2-101 | docs/STATUS.md:698 | A39 Stufe 2: Selektor aus AESGLOBAL | obsolete | DONE | archive |
| SD2-102 | docs/STATUS.md:699 | A60: TeraDesk schrieb `ap_rscmem` 8 Byte | trap | Overlay sdk/overlays/teradesk; docs/sdk/04-porting-anatomy.md:55 | archive |
| SD2-103 | docs/STATUS.md:700 | A40: `ob_fix` beim Laden | obsolete | DONE | archive |
| SD2-104 | docs/STATUS.md:701 | A27: Nullen des Ausgabepuffers | decision | COMPATIBILITY.md:223; docs/analysis/aes-underwriting-handlers-2026-09-19.md | archive |
| SD2-105 | docs/STATUS.md:702 | A21: Rechteckpool 160 | trap | DONE; Fiber-Stack ohne Guard-Page nur in thread.rs:95-100 und A21-STATUS-DONE.MD:139 → vorher Ticket | archive |
| SD2-106 | docs/STATUS.md:703 | A20: Leistenhoehe aus Fontmetrik | obsolete | DONE | archive |
| SD2-107 | docs/STATUS.md:704 | A01–A04: Desktop-Rechtecke | obsolete | Nebenbefund durch A20 (Z. 703) | archive |
| SD2-108 | docs/STATUS.md:705 | A39 Stufe 1: Ressource je Prozess | obsolete | Stufe 2 Z. 698; DONE | archive |
| SD2-109 | docs/STATUS.md:706 | H05: `shel_read` aus einem Puffer | decision | docs/api/aes/scrap-shell.md:159 „One buffer“; DONE | archive |
| SD2-110 | docs/STATUS.md:707 | A67: 3D-Kasten behaelt Rand | obsolete | Nebenbefunde durch A68–A70 (Z. 685) | archive |
| SD2-111 | docs/STATUS.md:708 | A66: `bb_save` klammert Zeiger | obsolete | Nebenbefunde durch A72/A57 (Z. 684/687) | archive |
| SD2-112 | docs/STATUS.md:709 | H02: Menueklick leckte als `MU_BUTTON` | obsolete | Nebenbefunde durch A73/A88 (Z. 683/671) | archive |
| SD2-113 | docs/STATUS.md:710 | SDK-Entwicklerdoku, AppleDouble-Falle | trap | Falle in docs/sdk/06-image-and-testing.md:78; T04–T07 DONE | archive |
| SD2-114 | docs/STATUS.md:711 | H03: `evnt_timer`-Wortreihenfolge | trap | B16 in 05-abi-pitfalls.md:559; Auto-Memory shell-grep-is-ugrep-wrapper.md | archive |
| SD2-115 | docs/STATUS.md:712 | T01: `_DISKINFO` gepinnt | trap | Erhebung TASK/T01-STATUS-DONE.MD; Auto-Memory lp64-tos-struct-trap.md | archive |
| SD2-116 | docs/STATUS.md:713 | A64/A63: Dateiauswaehler 3D | obsolete | DONE | archive |
| SD2-117 | docs/STATUS.md:714 | A65: Objektzeichnung klammert Zeiger | obsolete | DONE | archive |
| SD2-118 | docs/STATUS.md:715 | T02/T03: Fehlschlagliste, Quellenkette | decision | Regeln in sdk/lib-stale.sh, `expected-failures.txt`; DONE | archive |
| SD2-119 | docs/STATUS.md:716 | Q06: QED-Mengenmodul LP64 | obsolete | Q06 ERLEDIGT 2026-10-17 | archive |
| SD2-120 | docs/STATUS.md:717 | DESK-Rueckbau 1, Luecken G1–G7 | obsolete | Liste docs/analysis/desk-aes-gaps-2026-09-14.md; durch DESK43/U8b ueberholt | archive |
| SD2-121 | docs/STATUS.md:718 | Cookie-Spiegel `$5A0` live | trap | LP64-Schrittweite in docs/api/bios/_cookie-jar.md:268-293; Wirkung von ARCH03 auf den Spiegel nicht geprueft | archive |
| SD2-122 | docs/STATUS.md:719 | X01: `Super`/`Supexec` im Overlay | decision | Overlay `osbind.h`; X01 ERLEDIGT; Offenpunkt (a) in X01-STATUS-DONE.MD:15, (b) durch Z. 718 | archive |
| SD2-123 | docs/STATUS.md:720 | A28: Grenze Desktop/Kind | obsolete | DONE | archive |
| SD2-124 | docs/STATUS.md:721 | A29: Deskriptor nicht freigeben | obsolete | DONE | archive |
| SD2-125 | docs/STATUS.md:722 | A19: `int_in`/`addr_in` geklammert | obsolete | DONE; `SH_ISCR`-Rest durch A61 (Z. 692) | archive |
| SD2-126 | docs/STATUS.md:723 | V17: Mausparkplatz `xres/2` | trap | V17 DONE; falscher Kopfkommentar `aes-m07-parameter.tsv:39-44` steht noch, nur in V17-STATUS-DONE.MD:227 → vorher tosdump/cases/ERRATA.md | archive |
| SD2-127 | docs/STATUS.md:724 | A35: `ap_ptree` als Zeigerfeld | obsolete | „`rs_sglobe` bewusst nicht“ durch A39 (Z. 705/698) widerrufen | archive |
| SD2-128 | docs/STATUS.md:725 | M08: Pufferspalte in `mkrusttests` | obsolete | M08 ERLEDIGT 2026-10-17; Maus-Blocker durch V17 (Z. 723) | archive |
| SD2-129 | docs/STATUS.md:726 | B4/B5: `ap_private`-Selftest, Spec-Blocker | obsolete | „ADR-022 aufgehoben“ widerspricht MEMORY.md:1520-1523 und Auto-Memory teradesk-is-the-desktop.md; Selftest-Lehre in unwired-selftests-rot-silently.md | archive |
| SD2-130 | docs/STATUS.md:727 | AESGLOBAL bleibt m68k-schmal | decision | Entscheidung in docs/api/aes/application.md:289ff/:1174; Ausnahme fehlt in COMPATIBILITY.md:1060 → vorher dort benennen | archive |
| SD2-131 | docs/STATUS.md:728 | A42: `wind_get(WF_NAME)` verweigert | obsolete | Fussnote window-graphics.md:190; DONE | archive |
| SD2-132 | docs/STATUS.md:729 | A36/A35: Substrukturen arrayweise | obsolete | A35-Teil durch Z. 727/724/705 erledigt | archive |

## STATUS.md Recent Decisions, Teil 3 (2026-05-02 bis 2026-09-13)

Quelle: docs/STATUS.md Zeilen 730–861 (132 Tabellenzeilen). Checker: OK=896, OLD-HASH-MAPPED=144, MISSING=170, AMBIGUOUS=63, EMUTOS-COMMIT=16, BAD-COMMIT=2, LINE-OUT-OF-RANGE=1.

### Systemische Befunde
- Beide BAD-COMMIT sind keine Repo-Commits: `b546802` (Z. 768) ist der CoreMark-Upstream-Pin (sdk/apps/coremark-sources.md5:8-10), `2b49c065…` (Z. 792) die md5 von `ETOSREF.IMG`. Der LINE-OUT `main.rs:224` (Z. 734) meint tools/xtask/src/main.rs (Clippy-Fundstelle), nicht kernel/src/main.rs. MISSING/AMBIGUOUS sind Rauschen oder erwartete Historie: Bauprodukte (`*.NCP`, `*.a`, `build/dist/`, `kernel8.img`, `/tmp/sd.img`), blanke Dateinamen aus QED/TeraDesk/mintlib/FreeMiNT/circle/CoreMark (`tasten.c`, `startprg.c`, `sys/filesys.c`, `bcmpropertytags.h`), entfernte ILP32-Skripte (Z. 769), ausgelagerte Ball-Demo (Z. 749), `kernel/src/desk/*` (seit U8b in programs/desktop/), Registerfelder (`CPACR_EL1.FPEN`, `w.full/w.curr/w.prev`).
- Alle 132 Zeilen sind abgeschlossene Historie: Tickets M05–M07, A22/A25/A33/A34/A37/A38/A54, Q07, V16 haben DONE-Quittungen; der Rest sind Messrunden, Audits, Ports und Phase-00/02-Anfangseintraege. Das bleibende Wissen der Entscheidungszeilen steht schon in ADRs (ADR-014/016/017/019/020-Addendum/021/022/023 samt Amendments/026), in COMPATIBILITY.md (Fattrib-Ordner :1102ff, V16-Selektor `-1` :118-145, WF_SCREEN :156-178, vqin_mode :84-100, Low-Memory-Sysvars :1036-1053), in docs/api, im Code oder in der Auto-Memory. Daher durchgehend `archive`.
- Widerrufe/Ueberholungen: Z. 786 (R2-6/R2-7 offen) und Z. 788 (Font-Entscheidung R2-4) durch Z. 1048 (`675e311` → `cdc07fd6`, beide 1:1 nachgebildet, Fonts unschuldig) und tosdump/reference/FINDINGS-r2.md:96; Z. 854 (ST-RAM 14 MiB, Sysvars nie gemappt) durch Z. 765 (10 MiB, layout.rs:83-88) und Z. 803 (COMPATIBILITY.md:1046); Z. 855 (Identity-Mapping) durch ADR-014; Z. 733 (A54) durch A57 (COMPATIBILITY.md:158); Z. 752 (OVERLAY_BIT ohne Platz) durch ADR-024; Z. 820–824 (ILP32-Breitenpfad) durch Z. 818 und Z. 745; Z. 742 nennt `-16892` einen Stapelwert, die Auto-Memory hatari-produces-tt-references.md korrigiert das (2026-10-26: Low-Word von `ap_3resv = &D`); Hatari-Pfad und ROM aus Z. 742/792 sind durch docs/analysis/ORACLE.md §1 abgeloest.
- Wissen, das vor dem Archivieren umziehen muss: (a) Z. 802, Richtungsentscheidung „EmuTOS-kooperative Policy auf FreeMiNT-foermigen Primitiven“ steht nur in docs/analysis/coop-kernel/00-effort-comparison.md, kein ADR → ADR in docs/MEMORY.md oder Verweis in ARCHITECTURE.md; (b) Z. 780, EmuTOS-interne Inkonsistenz `wm_calc` (strip-and-force, gemwmlib.c:1829-1832 mit falschem Kommentar) gegen `w_bldactive` (rohe havevbar/havehbar, :597-601) steht nirgends sonst → docs/api/aes/window-graphics.md; (c) Z. 755/785, Tabellendirektive `!reset` und Vorlauf-Zeilen `+` stehen nur in tosdump/core/cases.h:15,246ff, nicht in docs/TOS-REFERENCE-DUMPER.md → dort nachtragen; (d) Z. 847, rust-toolchain.toml:3 verweist fuer „rationale and update policy“ auf docs/STATUS.md, dort steht aber nur diese Zeile ohne Begruendung → Verweis auf Archiv oder Begruendung in rust-toolchain.toml selbst.
- Beifunde ausserhalb des Chunks: tosdump/reference/FINDINGS-r2.md:202 fuehrt R2-6 noch als „OPEN“, obwohl `cdc07fd6` es entschieden hat; Auto-Memory aes-object-abi-is-tree-abi.md beschreibt ILP32-Apps als laufend, seit Z. 745 lehnt der Lader ELFCLASS32 ab (loader.rs:425); Z. 763 vermerkt `HANDLE_COUNT = 64` statt `NUM_VDI_HANDLES = 128` als offen, Code unveraendert (tos-api/src/vdi/vwk.rs:22), Befund steht in docs/api/vdi/_discrepancies.md:22 (D-VDI-3), aber in keinem Ticket.

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| SD3-001 | docs/STATUS.md:730 | M07: Dumper-Riegel je Parameter statt je Opcode | decision | M07 DONE; Riegel-Aufteilung in docs/TOS-REFERENCE-DUMPER.md:286 und tosdump/core/cases.c; EmuTOS-Cites gemfmlib/gemsclib OK | archive |
| SD3-002 | docs/STATUS.md:731 | M06: Riegel nach gemessener AES-Version, `objc_sysvar` gemessen | decision | M06 DONE; `features`/Versionsfrage in TOS-REFERENCE-DUMPER.md:286; `core/` = tosdump/core (Rauschen) | archive |
| SD3-003 | docs/STATUS.md:732 | M05: `addr_in`-Spalte, Puffer per Nummer | decision | M05 DONE; Format in docs/TOS-REFERENCE-DUMPER.md:208-248 | archive |
| SD3-004 | docs/STATUS.md:733 | A54: `wind_get(WF_SCREEN)` FALSE ohne Puffer | obsolete | A54 DONE; durch A57 eingeengt (COMPATIBILITY.md:156-178, Puffer existiert jetzt in tos-api/src/aes/gsxif.rs) | archive |
| SD3-005 | docs/STATUS.md:734 | A37 Teil 2: mkrsc reserviert volle `te_txtlen` | decision | Vertrag in tools/mkrsc.py:123-129; LINE-OUT `main.rs:224` = tools/xtask/src/main.rs; `qed.rsc`/`build/dist` Bauprodukte | archive |
| SD3-006 | docs/STATUS.md:735 | Q07: `Fattrib` mit Trennzeichen am Ende | decision | Q07 DONE; Abweichung in COMPATIBILITY.md:1102-1118; FreeMiNT-Pfade relativ (Rauschen) | archive |
| SD3-007 | docs/STATUS.md:736 | A38: zu kurzes `.RSC` abgelehnt | obsolete | A38 DONE; `header_is_sound` in tos-api/src/aes/resource.rs | archive |
| SD3-008 | docs/STATUS.md:737 | V16: GEM-Trap laesst `x0` bei unbekanntem Selektor stehen | decision | V16 DONE; F-2 (Selektor `-1` → 0) in COMPATIBILITY.md:118-145; A53 DONE | archive |
| SD3-009 | docs/STATUS.md:738 | A37: `fix_tedinfo_std` nach Relokation | obsolete | A37 DONE; Code resource.rs | archive |
| SD3-010 | docs/STATUS.md:739 | A33: `wm_delete` bestueckt Slot | obsolete | A33 DONE; Lauf measurement-2026-09-12-qemu-r3-a33 vorhanden | archive |
| SD3-011 | docs/STATUS.md:740 | A34: `wind_create` schreibt kein `WS_WORK` | obsolete | A34 DONE; Spec window-graphics.md unveraendert korrekt | archive |
| SD3-012 | docs/STATUS.md:741 | A22: Rechteckliste in z-Reihenfolge | obsolete | A22 DONE; Lauf qemu-r3-a22 vorhanden | archive |
| SD3-013 | docs/STATUS.md:742 | TT-Referenzen per Hatari, bitgleich validiert | decision | Wissen in Auto-Memory hatari-produces-tt-references.md; Hatari-Pfad/ROM durch docs/analysis/ORACLE.md §1 abgeloest; „Stapelwert -16892“ dort korrigiert (= `&D`) | archive |
| SD3-014 | docs/STATUS.md:743 | B2: ungerade Rasterzeiger, `RASTER_ALIGN = 2` | trap | Regel in tools/mkrsc.py:117 + Nachbedingung; `icons.rsc`/`build/dist` Bauprodukte | archive |
| SD3-015 | docs/STATUS.md:744 | B1: `rsrc_load` sucht ueber `sh_find` | obsolete | Code resource.rs; `C:\GEMSYS\RSC\`-Vorsatz entfallen | archive |
| SD3-016 | docs/STATUS.md:745 | ILP32-Programme werden abgewiesen (EPLFMT) | decision | `Ilp32Unsupported` in tos-api/src/bdos/process/loader.rs:110,425; in docs/MEMORY.md (ADR-020) genannt | archive |
| SD3-017 | docs/STATUS.md:746 | A25: `appl_init`-Wiederaufruf verlor Nachrichten | obsolete | „noch nicht deployed/committed“ ueberholt: TASK/A25-STATUS-DONE.MD | archive |
| SD3-018 | docs/STATUS.md:747 | QED-Tippen ohne Zeichen, Hypothese widerlegt | obsolete | Q06 ERLEDIGT 2026-10-17 (Q06-STATUS-DONE.MD), Q03/Q04 gegenstandslos; QED-Dateinamen Rauschen | archive |
| SD3-019 | docs/STATUS.md:748 | USB-Diagnose, vier Treiberdefekte | trap | Defekte (`wait_transfer` u.a.) in TASK/USB01.md (OFFEN) gefuehrt; `sys/usb/...` FreeMiNT-relativ | archive |
| SD3-020 | docs/STATUS.md:749 | Ball-Demo aus dem Baum geloest | decision | Auto-Memory ball-demo-as-benchmark.md; `KNOWN DEFECT` steht in tos-api/src/xbios/screen.rs:220 (Cite :221, eine Zeile verschoben) | archive |
| SD3-021 | docs/STATUS.md:750 | AES-R2 als Dauertest, Reset vor und nach Fall | obsolete | Harness tos-api/tests/aes_reference/ vorhanden; `acb7f3b4` = EmuTOS-Commit | archive |
| SD3-022 | docs/STATUS.md:751 | Fenstermanager gemessen, `MAX_WINDOWS` 15 | obsolete | Fix im Code; Begruendung (NUM_WIN an AES-Version) aus sysconf.h belegt | archive |
| SD3-023 | docs/STATUS.md:752 | `OVERLAY_BIT` ohne Platz, nicht repariert | obsolete | ueberholt durch ADR-024 (docs/MEMORY.md:1986) und docs/api/vdi/raster/_raster.md:607 (Bit im X-Byte); Code nicht geprueft | archive |
| SD3-024 | docs/STATUS.md:753 | AES setzt VDI-Eingabemodus wie EmuTOS | obsolete | Rest-Abweichung in COMPATIBILITY.md:84-100 | archive |
| SD3-025 | docs/STATUS.md:754 | Raster-Messung als Dauertest | obsolete | tos-api/tests/vdi_raster_reference.rs vorhanden | archive |
| SD3-026 | docs/STATUS.md:755 | AES-R2: `!reset wind_new` | decision | Direktive nur in tosdump/core/cases.h:246ff, nicht in docs/TOS-REFERENCE-DUMPER.md → dort nachtragen; Rest Rauschen (`deploy-pi4/`, `a_wind_new.c`) | archive |
| SD3-027 | docs/STATUS.md:756 | Raster-Gruppe gemessen, 57/59 gleich | obsolete | Messhistorie; Referenz `EMUTOS14-RASTER.BIN` | archive |
| SD3-028 | docs/STATUS.md:757 | Raster-Dumper: eigenes Format, CONTRL 15 Woerter | decision | Format im Code tosdump/core/runner_rast.c:39-50, cases.h | archive |
| SD3-029 | docs/STATUS.md:758 | Pi-Gegenprobe VDI, Handle-Verschiebung erklaert | obsolete | Messhistorie; TeraDesk-Handle-Erklaerung nur fuer damaligen #Z-Start gueltig | archive |
| SD3-030 | docs/STATUS.md:759 | `Setscreen` mit zwei Seiten, Tag `0x0004_8009` | decision | ADR-023 Amendment 2 (docs/MEMORY.md:1843ff); circle-Dateinamen relativ (Rauschen) | archive |
| SD3-031 | docs/STATUS.md:760 | `Physbase`/`Logbase` als User-VA, Uncached | decision | ADR-023 Amendment 1 (docs/MEMORY.md:1778ff) | archive |
| SD3-032 | docs/STATUS.md:761 | `Getbpb`/`Getmpb` erreichbar, FreeMiNT `PROT_*` | decision | ADR-023 (docs/MEMORY.md:1627); Auto-Memory adopt-freemint-protection-model.md | archive |
| SD3-033 | docs/STATUS.md:762 | APITEST BIOS-Gruppe, zwei Vertragsluecken | obsolete | Luecken durch Z. 761 (ADR-023) geschlossen | archive |
| SD3-034 | docs/STATUS.md:763 | `MAX_VERTICES` 1024, neun Geraetefelder | decision | Spec korrigiert; offener `HANDLE_COUNT`-Rest steht in docs/api/vdi/_discrepancies.md:22 (D-VDI-3), Code weiter 64 (vwk.rs:22), kein Ticket; `config.h` = EmuTOS | archive |
| SD3-035 | docs/STATUS.md:764 | Keine Phantomtaste nach CoreMark, `Bconstat(2)`-Fix | obsolete | Fix `7201fc7` gemappt; `kernel/src/desk/main.rs` verlegt; TeraDesk-Dateien relativ | archive |
| SD3-036 | docs/STATUS.md:765 | ST-RAM fest 10 MB, OS-Reserve aus Pool | decision | in kernel/src/bios/machine/layout.rs:83-88; Auto-Memory framebuffer-outside-st-tt-ram.md; loest Z. 854 ab | archive |
| SD3-037 | docs/STATUS.md:766 | 2-MiB-BSS-Puffer verschob ST-RAM ueber `ALIGN(2M)` | trap | Ursache in docs/analysis/memory-layout-and-reported-free-memory.md:730ff (§13) | archive |
| SD3-038 | docs/STATUS.md:767 | Konsolen-`ACTIVE`-Gate entfernt | obsolete | Fix im Code; `kernel/src/desk/main.rs` verlegt | archive |
| SD3-039 | docs/STATUS.md:768 | CoreMark nativ, `uintptr_t`-Falle | obsolete | Falle in PORT10 behoben (sdk/overlays/coremark/newtos/core_portme.h:54-56); `b546802` = CoreMark-Pin; docs/benchmarks/coremark.md | archive |
| SD3-040 | docs/STATUS.md:769 | ILP32-Bauweg stillgelegt | decision | ADR-020 Addendum 2026-09-04 (docs/MEMORY.md:1314ff); MISSING = entfernte Skripte (erwartet) | archive |
| SD3-041 | docs/STATUS.md:770 | 200-Hz-Tick, `_hz_200` live, PPI 30 | obsolete | INTID 30 in docs/api/bios/06-tickcal.md, docs/api/bios/_state.md, kernel/src/drivers/timer.rs | archive |
| SD3-042 | docs/STATUS.md:771 | Phantomtaste aus serieller Leitung | trap | Auto-Memory pl011-rx-irq-not-delivered-on-real-hw.md; SER01 DONE | archive |
| SD3-043 | docs/STATUS.md:772 | Boot-Schirm per EmuTOS-`initinfo`-Schleife | obsolete | `INITINFO_DURATION` 15/3 in kernel/src/initinfo.rs:53,56 bestaetigt | archive |
| SD3-044 | docs/STATUS.md:773 | Clipping/Mover: fuenf Stellen wie EmuTOS | obsolete | Fixes im Code, Historie | archive |
| SD3-045 | docs/STATUS.md:774 | TeraDesk laeuft, fuenf Defekte | obsolete | Historie; Bauprodukte MISSING erwartet | archive |
| SD3-046 | docs/STATUS.md:775 | GEMDOS-Lauf 2 auf HW, 0-Byte-Diagnose offen | obsolete | 0-Byte-Diagnose durch HW03 (Z. 613) erledigt | archive |
| SD3-047 | docs/STATUS.md:776 | GEMDOS-Tracer, `Fsfirst("C:\")` → EFILNF | obsolete | Fixes im Code, Historie | archive |
| SD3-048 | docs/STATUS.md:777 | AES-Trap-Tracer, drei Defekte | obsolete | Historie | archive |
| SD3-049 | docs/STATUS.md:778 | `WM_VSLID`/`WM_HSLID` vertauscht, Desktop in User-Space | obsolete | User-Space-Desktop durch ADR-026 (docs/MEMORY.md:2240) umgesetzt | archive |
| SD3-050 | docs/STATUS.md:779 | AES-Runde 1 abgeschlossen, 5 Rest | obsolete | Messhistorie | archive |
| SD3-051 | docs/STATUS.md:780 | `wind_calc` 3D-Zweig, Handle 0 echtes Fenster | trap | Fix im Code; EmuTOS-Inkonsistenz `wm_calc` gegen `w_bldactive` (gemwmlib.c:597-601, :1829-1832 geprueft) nur hier → docs/api/aes/window-graphics.md | archive |
| SD3-052 | docs/STATUS.md:781 | Regel: EmuTOS-konform, keine bewusste Abweichung ausser HW | decision | Auto-Memory no-rationalizing-emutos-deviations.md; CLAUDE.md Regel 1 | archive |
| SD3-053 | docs/STATUS.md:782 | Erste AES-Messung auf HW | obsolete | Messhistorie | archive |
| SD3-054 | docs/STATUS.md:783 | DUMPAES Runde 1, AES-ABI-Unterschiede | obsolete | `d0`-Vertrag in docs/api/aes/_dispatcher.md | archive |
| SD3-055 | docs/STATUS.md:784 | VDI-Ebene abgeschlossen, `VDIDRAW.NAP` | obsolete | Historie | archive |
| SD3-056 | docs/STATUS.md:785 | Runde 3: Vorlaeufe (`+`-Zeilen) | decision | Vorlauf-Format nur in tosdump/core/cases.h:15 → docs/TOS-REFERENCE-DUMPER.md nachtragen | archive |
| SD3-057 | docs/STATUS.md:786 | OFFEN: R2-6/R2-7 EmuTOS-Fehler nachbilden? | obsolete | entschieden in Z. 1048 (`675e311` → `cdc07fd6`): beide 1:1 (tos-api/src/vdi/mod.rs:293-301) | archive |
| SD3-058 | docs/STATUS.md:787 | VDI-R2-Defekte gefixt, 112/8 | obsolete | Historie | archive |
| SD3-059 | docs/STATUS.md:788 | Font-Frage R2-4 offen, kleine Fonts nicht | obsolete | Praemisse korrigiert: FINDINGS-r2.md:96 „CORRECTED“, Z. 1048 (Fonts unschuldig); 6x6-SMALL in AES seit Z. 774 | archive |
| SD3-060 | docs/STATUS.md:789 | `mkrusttests`: Dump als Testorakel | obsolete | Generator tosdump/host/mkrusttests.c vorhanden | archive |
| SD3-061 | docs/STATUS.md:790 | Dumper-Runde 2, tos-api host-testbar | obsolete | Historie; `ETOSDBG.IMG` etc. Kartendateien | archive |
| SD3-062 | docs/STATUS.md:791 | EmuTOS 1.4 vs TOS 3.06 auf derselben TT | obsolete | Messhistorie; Kalibrierung EMUTOS14 gegen Pin in docs/analysis/ORACLE.md | archive |
| SD3-063 | docs/STATUS.md:792 | Orakel EmuTOS @1a7484ab, TOS 3.06 Gegencheck | decision | durch docs/analysis/ORACLE.md §1 umgesetzt (Hatari, eigenes ROM); `2b49c065…` = md5 `ETOSREF.IMG` | archive |
| SD3-064 | docs/STATUS.md:793 | Record-Format v2 mit `d0` | obsolete | Feld in docs/TOS-REFERENCE-DUMPER.md:151 | archive |
| SD3-065 | docs/STATUS.md:794 | `DUMPVDI.PRG` + Smoke-Tabelle | obsolete | Historie | archive |
| SD3-066 | docs/STATUS.md:795 | Dumper AArch64-Backend | obsolete | Historie; mintlib-Dateinamen relativ | archive |
| SD3-067 | docs/STATUS.md:796 | Dumper T2 | obsolete | Historie | archive |
| SD3-068 | docs/STATUS.md:797 | Dumper T1, `tosdiff` | obsolete | Spec docs/TOS-REFERENCE-DUMPER.md | archive |
| SD3-069 | docs/STATUS.md:798 | AES-I/O-Audit Start, `IP_HOLLOW` weiss fuellen | obsolete | Fix im Code (object.rs) | archive |
| SD3-070 | docs/STATUS.md:799 | TeraDesk als GEM-Shell, EL0-`G_USERDEF`-Upcall | obsolete | Historie | archive |
| SD3-071 | docs/STATUS.md:800 | LP64-BASEPAGE-Fix, Flopfmt EUNDEV | obsolete | Auto-Memory lp64-tos-struct-trap.md fortgeschrieben | archive |
| SD3-072 | docs/STATUS.md:801 | `_DTA`-Fix LP64 | obsolete | Auto-Memory lp64-tos-struct-trap.md | archive |
| SD3-073 | docs/STATUS.md:802 | Kooperativer Kern: EmuTOS-Policy, FreeMiNT-Primitiven | decision | Richtung nur in docs/analysis/coop-kernel/00-effort-comparison.md, kein ADR → ADR oder ARCHITECTURE.md vor Archiv | archive |
| SD3-074 | docs/STATUS.md:803 | BIOS-Audit, Sysvars je Prozess in EL0 | decision | COMPATIBILITY.md:1036-1053; docs/analysis/bios-audit/b1-lowmem-feasibility.md | archive |
| SD3-075 | docs/STATUS.md:804 | TeraDesk-Paket, `mkrsc --rshdr` | obsolete | Modus in tools/mkrsc.py:15 | archive |
| SD3-076 | docs/STATUS.md:805 | TeraDesk baut LP64, Schattenbaum-Overlay | trap | Falle (quoted include) in sdk/apps/build-teradesk.sh:20-22 | archive |
| SD3-077 | docs/STATUS.md:806 | `appl_init` fuellt AESGLOBAL wie EmuTOS | obsolete | spaeter fortgeschrieben (Z. 726/727, docs/api/aes/application.md) | archive |
| SD3-078 | docs/STATUS.md:807 | Desktop ruft `appl_init`/`appl_exit` | obsolete | `kernel/src/desk/` verlegt, ADR-026 | archive |
| SD3-079 | docs/STATUS.md:808 | AES-FIFO-Nachrichtenschlange | obsolete | Fix im Code (pd.rs) | archive |
| SD3-080 | docs/STATUS.md:809 | QED P3: `rehilite` ueberspringt DISABLED | obsolete | Fix im Code | archive |
| SD3-081 | docs/STATUS.md:810 | QED P5: `wind_close`/WF_TOP senden WM_REDRAW | obsolete | Fix im Code | archive |
| SD3-082 | docs/STATUS.md:811 | ADR-022 Modul 1: Redraw-Pfad 1:1 | obsolete | ADR-022 (docs/MEMORY.md:1520); `kernel/src/desk/` verlegt | archive |
| SD3-083 | docs/STATUS.md:812 | QED P1+P2: Dropdown-Save/Restore | obsolete | Fix im Code | archive |
| SD3-084 | docs/STATUS.md:813 | D7: `FL3DBAK` flach | obsolete | Fix im Code; Analyse aes-object-form-render-audit.md | archive |
| SD3-085 | docs/STATUS.md:814 | D3/D4/D5: Button-Rand | obsolete | Fix im Code | archive |
| SD3-086 | docs/STATUS.md:815 | D1/D9: Label-Indent | obsolete | Fix im Code | archive |
| SD3-087 | docs/STATUS.md:816 | `Fattrib` TOS4/FreeMiNT-Ordner-GET | decision | COMPATIBILITY.md:1102ff; Auto-Memory cflib-apps-expect-tos4-semantics.md; vervollstaendigt durch Z. 735 | archive |
| SD3-088 | docs/STATUS.md:817 | AES = Kern, Desktop = Client, `WF_NEWDESK` | decision | ADR-022; Auto-Memory desktop-1to1-emutos-rebuild.md | archive |
| SD3-089 | docs/STATUS.md:818 | App-ABI-Pivot auf LP64 | decision | ADR-020 Status Superseded (docs/MEMORY.md:1259); Auto-Memory native-64bit-abi-future.md | archive |
| SD3-090 | docs/STATUS.md:819 | F5: Screen-Handoff, `save_windows` | obsolete | `DESK_PAINTER` durch Z. 817 entfernt, Desktop EL0 (ADR-026) | archive |
| SD3-091 | docs/STATUS.md:820 | ILP32-Breiten-Kampagne | obsolete | ILP32 aufgegeben (Z. 818, Z. 745) | archive |
| SD3-092 | docs/STATUS.md:821 | QED-Start: `rsrc_gaddr` R_FRSTR, NRSC v3/v4 | obsolete | ILP32-Versionen gegenstandslos; Historie | archive |
| SD3-093 | docs/STATUS.md:822 | QED-Blocker diagnostiziert | obsolete | durch Z. 821 behoben | archive |
| SD3-094 | docs/STATUS.md:823 | `ObjTree::lp64` fuer Kernel-Baeume | obsolete | Auto-Memory aes-object-abi-is-tree-abi.md, die ILP32 noch als lebend fuehrt (seit Z. 745 abgewiesen) | archive |
| SD3-095 | docs/STATUS.md:824 | `ObjTree` Stage A, ILP32-Baeume | obsolete | ILP32-Pfad unerreichbar (Z. 745) | archive |
| SD3-096 | docs/STATUS.md:825 | EL0-GEMDOS-Speicherfenster | decision | ADR-021 (docs/MEMORY.md:1408) | archive |
| SD3-097 | docs/STATUS.md:826 | Lader streamt in die TPA | obsolete | Fix im Code | archive |
| SD3-098 | docs/STATUS.md:827 | QED kompiliert unveraendert | obsolete | Historie; Auto-Memory port-target-app-unmodified.md | archive |
| SD3-099 | docs/STATUS.md:828 | cflib-21 portiert | obsolete | Historie | archive |
| SD3-100 | docs/STATUS.md:829 | gemlib-0.44.0 portiert, `svc #2`-ABI | obsolete | ABI im Code exceptions.rs | archive |
| SD3-101 | docs/STATUS.md:830 | mintlib-Port, ADR-019, `CPACR_EL1.FPEN` | decision | ADR-019 (docs/MEMORY.md:1176); Auto-Memory port-reuse-dont-rewrite.md | archive |
| SD3-102 | docs/STATUS.md:831 | Bottom-up-Re-Audit AES komplett | obsolete | Historie; D-Nummern in docs/api/*/_working-list.md | archive |
| SD3-103 | docs/STATUS.md:832 | VDI-Code-Audit abgeschlossen | obsolete | Historie | archive |
| SD3-104 | docs/STATUS.md:833 | GEMDOS-Code-Audit abgeschlossen | obsolete | Historie | archive |
| SD3-105 | docs/STATUS.md:834 | ADR-017 native Aufloesung | decision | ADR-017 (docs/MEMORY.md:682); `kernel/src/desk/global.rs` verlegt | archive |
| SD3-106 | docs/STATUS.md:835 | XBIOS-Re-Audit, Dispatcher-Bug | obsolete | Historie | archive |
| SD3-107 | docs/STATUS.md:836 | BIOS-Pilot-Audit | obsolete | Historie | archive |
| SD3-108 | docs/STATUS.md:837 | Desktop-Rebuild-Kern (#5–#9) | obsolete | durch ADR-022/ADR-026 ueberholt; Pfade verlegt | archive |
| SD3-109 | docs/STATUS.md:838 | ADR-016 Desktop-Neubau | obsolete | ADR-016 (docs/MEMORY.md:586), verfeinert durch ADR-022/026 | archive |
| SD3-110 | docs/STATUS.md:839 | QEMU raspi4b: `probe_read32` fuer PCIe-RC | trap | Falle in kernel/src/drivers/usb/pcie_rpi4.rs:11,459 und exceptions.rs:181-185 | archive |
| SD3-111 | docs/STATUS.md:840 | Icon-View-Ueberlappung behoben | obsolete | Historie | archive |
| SD3-112 | docs/STATUS.md:841 | `objc_find` liefert DISABLED-Objekt | obsolete | Spec docs/api/aes/object-form.md | archive |
| SD3-113 | docs/STATUS.md:842 | XBIOS-Absent-HW-Vertraege | obsolete | Specs docs/api/xbios | archive |
| SD3-114 | docs/STATUS.md:843 | Step-2-BUGs, K1 UART-Poll | obsolete | K1 durch Z. 771/SER01 ueberholt; docs/STEP2-VALIDATION.md | archive |
| SD3-115 | docs/STATUS.md:844 | PL011-Binding in XBIOS-Specs | obsolete | Inhalt in docs/api/xbios/0f-rsconf.md, 2c-bconmap.md, 0c-midiws.md | archive |
| SD3-116 | docs/STATUS.md:845 | Projektstruktur angelegt | obsolete | Historie | archive |
| SD3-117 | docs/STATUS.md:846 | Spec-Vorlage `_template.md` | obsolete | Datei vorhanden | archive |
| SD3-118 | docs/STATUS.md:847 | Toolchain `nightly-2026-04-15` | decision | rust-toolchain.toml:4 bestaetigt; rust-toolchain.toml:3 verweist auf STATUS.md fuer Begruendung, die hier fehlt → Verweis anpassen | archive |
| SD3-119 | docs/STATUS.md:848 | `forced-target` je Paket | decision | kernel/Cargo.toml:1,10 bestaetigt | archive |
| SD3-120 | docs/STATUS.md:849 | `references/emutos/` nur lesend | decision | CLAUDE.md Regel 1; references/README.md | archive |
| SD3-121 | docs/STATUS.md:850 | EmuTOS-Pin `1a7484ab` | decision | references/README.md; docs/analysis/ORACLE.md §1 | archive |
| SD3-122 | docs/STATUS.md:851 | `references/` gitignored ausser README | decision | CLAUDE.md Source Tree; references/README.md | archive |
| SD3-123 | docs/STATUS.md:852 | EmuTOS-Checkout verschoben | obsolete | Historie | archive |
| SD3-124 | docs/STATUS.md:853 | QEMU `virt,gic-version=2`, `-serial stdio` | decision | Begruendung in tools/xtask/src/main.rs:751-785 | archive |
| SD3-125 | docs/STATUS.md:854 | ST/TT-RAM getrennt, ST 14 MiB, Sysvars nicht gemappt | obsolete | ueberholt: Z. 765 (10 MiB, layout.rs:83-88), Z. 803 (COMPATIBILITY.md:1046) | archive |
| SD3-126 | docs/STATUS.md:855 | Identity-Mapping ueber TTBR0 | obsolete | ueberholt durch ADR-014 (docs/MEMORY.md:459) | archive |
| SD3-127 | docs/STATUS.md:856 | Phase 00 gestartet | obsolete | docs/api/_phase00-completion.md | archive |
| SD3-128 | docs/STATUS.md:857 | D5 XBIOS-Triage | obsolete | Specs vorhanden | archive |
| SD3-129 | docs/STATUS.md:858 | D5 XBIOS Batch 2 | obsolete | Specs vorhanden | archive |
| SD3-130 | docs/STATUS.md:859 | D5 XBIOS Batch 3 | obsolete | `_psg-group.md`, `_tt-shifter.md` vorhanden | archive |
| SD3-131 | docs/STATUS.md:860 | D3 GEMDOS Datei/Verzeichnis | obsolete | docs/api/gemdos/file, directory vorhanden (Cite relativ) | archive |
| SD3-132 | docs/STATUS.md:861 | Phase 00 abgeschlossen | obsolete | docs/api/_phase00-completion.md | archive |

## STATUS.md Recent Decisions, Teil 4 (aelteste Zeilen 2026-05-02 bis 2026-07-03, zwei Nachzuegler 2026-09-03/04) + Notes

Quelle: docs/STATUS.md Zeilen 862–1055 (130 Tabellenzeilen + 1 Notes-Absatz). Checker: OK=350, MISSING=37, AMBIGUOUS=27, BAD-COMMIT=26, OLD-HASH-MAPPED=7, EMUTOS-COMMIT=3, FREEMINT-COMMIT=1.

### Systemische Befunde
- **Sortierung:** Ab Z. 862 ist die Tabelle **aufsteigend** sortiert (2026-05-02 → 2026-07-03), darueber absteigend; die Bloecke stossen bei Z. 861/862 (beide 2026-05-02) zusammen. Innerhalb des Blocks Ausreisser (877 = 05-04 nach 05-07, 1009/1010 = 06-24 nach 06-25, 1011 = 06-22 nach 06-26, 1034 = 07-03 zwischen 07-02-Zeilen). Z. 1047 (2026-09-03) und Z. 1048 (2026-09-04) haengen am Tabellenende, gehoeren zwischen Z. 768 (09-04) und Z. 769. Fuer das Archiv: nach Datum neu sortieren.
- **BAD-COMMIT (26):** alle in Z. 928–1034 (2026-06-19 bis 07-02). Die Hashes stehen weder im Repo noch in docs/commit-map.txt noch in einer Referenz; die Arbeit existiert aber unter anderen Hashes mit passendem Betreff, z. B. `7231e9d`→`e8fa10b2`, `463cf6a`→`cb6c4758`, `d3e8714`→`9aece9af`, `794bdc5`→`e3903f64`, `434013f`→`657e35ec`, `aef52f9`→`85cc1e18`, `82952f2`/`4f18412`→`6aa20e6f`, `73823a7`→`dea9244d`, `6471d7f`→`3db0a05d`, `b9451db`→`8b75d0a9`, `f8b344f`→`f6569b23`, `3ed7f89`→`1d90b3f2`, `42b6577`→`11c649f2`, `5d4805e`→`1983f4a5`. Diese Hashes stammen aus einer zwischenzeitlich umgeschriebenen Historie, die commit-map nicht abdeckt (sie deckt `d9841af` 06-16 und `92c85fb` 07-02 ab). Fuer archivierte Historie kein Defekt; im Archiv einmal vermerken. Die 7 OLD-HASH-MAPPED (Z. 881, 1036, 1047, 1048) loesen korrekt auf.
- **MISSING/AMBIGUOUS** sind Historie oder Rauschen: geloeschte Demos/Programme (`programs/lineecho`, `kernel/src/generated/lineecho_elf.rs`, `selftest_lineecho.rs`, `vdi_demo.rs`, alle entfernt mit `7b2e9c18`), der Kernel-Desktop (`kernel/src/desktop.rs`, `desktop_icons.rs`, seit ADR-026 `programs/desktop/`), Diagnosecode `gic_diag.rs` (laut Zeile selbst entfernt), Bauprodukte (`kernel8.img`, `/tmp/sd.img`, `TEST.RSC`, `GEM.RSC`, `icons.rsc`), Registerfelder (`HCR_EL2.RW`, `frame.elr`, `dram.end`), relative EmuTOS-/FreeMiNT-Pfade (`bdos/bdosdefs.h`, `sys/ssystem.c`), Bezeichner (`DIRTREE.txt`, `0..3600`); AMBIGUOUS (`_text.md`, `_fill.md`, `object.rs`, `mod.rs`) sind aus dem Kontext eindeutig. Der Branch `archive/vl805-hub-investigation` (Z. 1015/1016) existiert nicht mehr (kein Ref).
- **Alles ist abgeschlossene Historie** (Phasen 02–16, Phase-00b-Extraktion, Kernel-Desktop-Bau bis zur EmuTOS-Neubauentscheidung). Ueberholt bzw. widerrufen durch neuere Zeilen: Kernel-Desktop (Z. 994–1011, 1017–1043) durch Z. 513 (ADR-026) und Z. 616 (DESK43 U3); Z. 862 (Ziel-Build TT-Shifter/VT52) durch Z. 629 (REF01) + Auto-Memory emutos-target-profile-falcon-tos4.md; Z. 934/936 („`$5A0` nicht gemappt“) durch Z. 803 (B1) + ADR-012-Amendment (docs/MEMORY.md:265) + COMPATIBILITY.md:1045-1047; Z. 976 (`C:\GEMSYS\RSC\`) durch Z. 744; Z. 1014/1022 (Keyboard-XBIOS, Getmpb, Kbshift „out of scope“) durch Z. 835/836; Z. 1006/1010 (Kernel startet Desktop) durch Z. 1047, diese wiederum durch Z. 513/616. Ohne STATUS-Widerruf, aber im Code ueberholt: Z. 884 Sversion `0x2000` → `0x3000` (`5051390e`, tos-api/src/gemdos/version.rs:14); Z. 1039 Solid-Drag → EmuTOS-treuer `drag_move` (tos-api/src/aes/window.rs:5481-5483); Z. 966 prozeduraler Rahmen → `w_bldactive`-Baum (window.rs:313,456).
- **Wissen, das vor dem Archivieren umziehen muss:** (a) Z. 950 Ressourcen-Speichermodell (System-RSC im Kernel eingebettet, App-RSC per `rsrc_load`) gilt weiter (tos-api/src/aes/fsel.rs:98 `include_bytes!`), steht in keinem ADR → Addendum zu ADR-013; (b) Z. 1025 NewTOS-Erweiterung `#K`-Zeile in NEWDESK.INF + `C:\KEYBOARD.KBD` steht nur in kernel/src/keymap.rs:13-14 → COMPATIBILITY.md (NewTOS-Erweiterung); (c) Z. 1047 benannte Abweichung im `DESKTOP_APP`-Arm (Konsole nur bei fehlendem Raster-Backend statt EmuTOS-Bauzeit-Entscheidung `bios/endrom.c`) steht nur in kernel/src/shell_loop.rs:28,305 → COMPATIBILITY.md.
- **Beifunde ausserhalb des Chunks:** tos-api/src/bdos/mem/traps.rs:660-667 behauptet noch „Phase 06 replaces this with PF_TTRAMMEM …“; die Politik sitzt laengst in tos-api/src/gemdos/memory.rs:85-106 (EmuTOS `bdos/umem.c:116-126`) → Kommentar korrigieren. Auto-Memory-Index MEMORY.md nennt references/freemint „not yet present“, die Datei freemint-extension-direction.md selbst und das Repo (`references/freemint` @ `f906416`) sagen vorhanden → Indexzeile korrigieren.

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| SD4-001 | docs/STATUS.md:862 | Ziel-Build TT-Shifter+VT52, kein Falcon | obsolete | Widerrufen durch Z. 629 (REF01, docs/api/_emutos-config.md) und Auto-Memory emutos-target-profile-falcon-tos4.md (Falcon030/TOS4) | archive |
| SD4-002 | docs/STATUS.md:863 | PHASE-02 D1–D3: tos-api, Dispatcher, ABI x8/SVC-Immediate | decision | ABI steht in docs/ARCHITECTURE.md:52-63 (`TrapClass::from_imm`, x8) | archive |
| SD4-003 | docs/STATUS.md:864 | PHASE-04 D9–D11 Malloc/Mfree/Mshrink | obsolete | Phase 04 erledigt (Z. 867) | archive |
| SD4-004 | docs/STATUS.md:865 | D12 Owner-Sentinel; Malloc-Default MX_PREFTTRAM | obsolete | Malloc waehlt jetzt per `PF_TTRAMMEM` (gemdos/memory.rs:85-106 = EmuTOS umem.c:116-126); Kommentar traps.rs:660-667 veraltet (Beifund) | archive |
| SD4-005 | docs/STATUS.md:866 | D13 `pool_stats` | obsolete | Erledigt; ST-RAM-Groesse spaeter geaendert (sd3: Z. 765) | archive |
| SD4-006 | docs/STATUS.md:867 | Phase 04 EC1–EC9, Selftest 10/10 | obsolete | `kernel/src/selftest.rs` vorhanden; abgeschlossen | archive |
| SD4-007 | docs/STATUS.md:868 | Kernel-Clippy aufgeraeumt, `deref_addrof`-False-Positive | trap | Reine Historie; Lint-Allows im Code kommentiert | archive |
| SD4-008 | docs/STATUS.md:869 | PHASE-05 High-Half-Migration, D4 vertagt | obsolete | D4 geloest durch Z. 871, TTBR0-Modell durch ADR-014 (docs/MEMORY.md:459, `EMPTY_L0`) | archive |
| SD4-009 | docs/STATUS.md:870 | PHASE-06 D3 ELF-Loader, 4-KiB-Arena | obsolete | Pfade OK; Loader seither mehrseitig/LP64 | archive |
| SD4-010 | docs/STATUS.md:871 | PHASE-06 D8 Per-Process-Page-Tables | obsolete | mmu.rs vorhanden; abgeschlossen | archive |
| SD4-011 | docs/STATUS.md:872 | D9 `m_own` an RUN | obsolete | traps.rs vorhanden; abgeschlossen | archive |
| SD4-012 | docs/STATUS.md:873 | D2 Prozesstabelle run/nextrun | obsolete | Abgeschlossen | archive |
| SD4-013 | docs/STATUS.md:874 | D1 `BasePage` 256 B bit-genau | obsolete | Assert tos-types/src/basepage.rs:178; `bdos/bdosdefs.h` relativer EmuTOS-Pfad | archive |
| SD4-014 | docs/STATUS.md:875 | Spec-Audit: Cconout/Crawio-Returns, D-009..D-011 | obsolete | D-009/010/011 in docs/api/_discrepancies.md:202,233,266 | archive |
| SD4-015 | docs/STATUS.md:876 | PHASE-05 EC4 EL0-Round-Trip; MRS SP_EL0 UNPREDICTABLE | trap | Falle im Code kernel/src/arch/aarch64/user_entry.rs:254; TLBI-Wedge = QUIRK-001 | archive |
| SD4-016 | docs/STATUS.md:877 | PHASE-05 D5/D6 EL0-Stack, Frame 288 B | obsolete | el0.rs, exception_vectors.s vorhanden; abgeschlossen; Datum ausser Reihe | archive |
| SD4-017 | docs/STATUS.md:878 | Pexec/Pterm Round-Trip, `run=p_parent`, QUIRK-001, ASID-Versuch | trap | QUIRK-001 + ASID-Befund in docs/MEMORY.md:64,125,2431ff; `mmu.rs:245` AMBIGUOUS, Rauschen | archive |
| SD4-018 | docs/STATUS.md:879 | Pexec PE_LOAD/PE_GO opakes Handle | obsolete | Abgeschlossen | archive |
| SD4-019 | docs/STATUS.md:880 | Ptermres, Phase 06 fertig | obsolete | EmuTOS bdos/proc.c:624 OK; abgeschlossen | archive |
| SD4-020 | docs/STATUS.md:881 | Konsolen-Eingabe Cconin..Cconrs | obsolete | `d9841af`→`1b477e1d` (map) | archive |
| SD4-021 | docs/STATUS.md:882 | RPi4-HW-Boot vorbereitet, `init_uart_clock` | trap | Falle in boot/rpi4/config.txt:51 und docs/HW-TEST-LOG.md | archive |
| SD4-022 | docs/STATUS.md:883 | Erstes Programm `lineecho` + Toolchain | obsolete | Programm/Generat/Selftest entfernt (`7b2e9c18`), MISSING erwartet | archive |
| SD4-023 | docs/STATUS.md:884 | PHASE-07 Zeit/Datum, Sversion 0x2000 | obsolete | Sversion jetzt `0x3000` (`5051390e`, gemdos/version.rs:14) | archive |
| SD4-024 | docs/STATUS.md:886 | ramfb-Framebuffer, 28-B-Config-Falle | trap | Falle im Code kernel/src/drivers/qemu_virt/ramfb.rs:40,195 | archive |
| SD4-025 | docs/STATUS.md:888 | PHASE-00b VDI D1 Dispatcher-Spec | obsolete | docs/api/vdi/_dispatcher.md vorhanden | archive |
| SD4-026 | docs/STATUS.md:890 | VDI Batch 1 Workstation-Lifecycle | obsolete | Specs vorhanden | archive |
| SD4-027 | docs/STATUS.md:892 | ADR-011 Multi-Screen-VDI + Batch 1 fertig | decision | ADR-011 in docs/MEMORY.md:187 | archive |
| SD4-028 | docs/STATUS.md:893 | VDI Batch 2 Attribut-Setter | obsolete | Specs vorhanden (AMBIGUOUS aus Kontext eindeutig) | archive |
| SD4-029 | docs/STATUS.md:894 | VDI Batch 3 Output-Primitive | obsolete | Specs vorhanden, spaeter verfeinert (Z. 1034) | archive |
| SD4-030 | docs/STATUS.md:895 | VDI Batch 4 Inquiry, V-1 | obsolete | V-1 = D-VDI-1 in docs/api/vdi/_discrepancies.md:20 | archive |
| SD4-031 | docs/STATUS.md:896 | VDI Batch 5, VDI-Spec komplett | obsolete | Specs vorhanden; „Truecolor out-of-scope“ durch ADR-024 ueberholt | archive |
| SD4-032 | docs/STATUS.md:898 | Mailbox-FB, EL2-Eintritt, BGR | trap | EL2/BGR in docs/hardware/mailbox-framebuffer.md:13-14,161,191 | archive |
| SD4-033 | docs/STATUS.md:900 | FB Normal-NC gemappt | trap | `remap_normal_nc` mmu.rs:608; Cacheability in ADR-023-Amendment docs/MEMORY.md:1778ff | archive |
| SD4-034 | docs/STATUS.md:902 | Erster HW-Boot auf RPi4 | obsolete | docs/HW-TEST-LOG.md:122 | archive |
| SD4-035 | docs/STATUS.md:904 | PHASE-12 D1+D2, `RasterBackend`-Grenze | decision | backend.rs vorhanden; Grenze in Auto-Memory vdi-draw-asm-optimization-later.md | archive |
| SD4-036 | docs/STATUS.md:906 | VDI D3 + Demo | obsolete | `vdi_demo.rs` entfernt (`7b2e9c18`) | archive |
| SD4-037 | docs/STATUS.md:908 | HW-Cache-Bug, `dc cvac`-Clean | trap | `clean_dcache_range` mmu.rs:371; docs/HW-TEST-LOG.md:123; ADR-023-Amendment | archive |
| SD4-038 | docs/STATUS.md:910 | VDI D4 Fill-Pattern | obsolete | Abgeschlossen | archive |
| SD4-039 | docs/STATUS.md:912 | VDI D5 Text, 8×16-Font | obsolete | Ein-Font-Stand durch drei Systemfonts ueberholt (Z. 1048) | archive |
| SD4-040 | docs/STATUS.md:914 | VDI D6 Blit, MFDB 64-bit in CONTRL[7..14] | obsolete | Jetzt ABI-abhaengig `N_PTRINTS` (tos-api/src/vdi/raster.rs:5-12, ADR-020) | archive |
| SD4-041 | docs/STATUS.md:916 | SD-Host-Treiber; QEMU-Karte am Legacy-EMMC | trap | Falle in kernel/src/drivers/bcm2711/emmc.rs:24-27, docs/hardware/emmc-sd.md:21-27; „exFAT spaeter“ vs. Auto-Memory version-roadmap (1.0 nur FAT16) | archive |
| SD4-042 | docs/STATUS.md:918 | BIOS Trap 13 + Rwabs | obsolete | Abgeschlossen | archive |
| SD4-043 | docs/STATUS.md:920 | Phase 08/09 FAT16; FAT12/16 nach Clusterzahl | trap | Regel im Code tos-api/src/bios/bpb.rs:73 (`MAX_FAT12_CLUSTERS`); FAT12-Write in Auto-Memory fat12-floppy-out-of-scope.md | archive |
| SD4-044 | docs/STATUS.md:922 | Phase 09 Unterverzeichnisse/Metadaten | obsolete | Abgeschlossen | archive |
| SD4-045 | docs/STATUS.md:924 | Storage auf echter Pi validiert, 4×500 MB | obsolete | Duplikat Auto-Memory sd-card-real-hw-layout.md | archive |
| SD4-046 | docs/STATUS.md:926 | Phase 10 Screen + EmuCON-Konsole | obsolete | fbcon.rs/shell.rs vorhanden | archive |
| SD4-047 | docs/STATUS.md:928 | Phase 12 VDI komplett | obsolete | `5d4805e` BAD (≈`1983f4a5`); „ein Font“, „Truecolor out-of-scope“ ueberholt (ADR-024) | archive |
| SD4-048 | docs/STATUS.md:930 | IKBD-Maus + AES D1–D3 | obsolete | `5d4805e`/`2cc0078` BAD | archive |
| SD4-049 | docs/STATUS.md:932 | BIOS Bconstat..Bcostat | obsolete | chardev.rs vorhanden | archive |
| SD4-050 | docs/STATUS.md:934 | Cookie-Jar-Analyse, `$5A0` gesperrt | obsolete | `$5A0` seit B1 gemappt: Z. 803, ADR-012-Amendment docs/MEMORY.md:265, COMPATIBILITY.md:1045-1047 | archive |
| SD4-051 | docs/STATUS.md:936 | ADR-012 Accepted (Wert-Query) | decision | ADR-012 docs/MEMORY.md:260 samt Amendment 2026-07-24 | archive |
| SD4-052 | docs/STATUS.md:938 | FreeMiNT sanktionierte Quelle, Ssystem-Spec | decision | Auto-Memory freemint-extension-direction.md; references/freemint @ `f906416` vorhanden; Spec docs/api/gemdos/154-ssystem.md | archive |
| SD4-053 | docs/STATUS.md:940 | Cookie-Jar via Ssystem implementiert | obsolete | tos-api/src/cookie.rs vorhanden | archive |
| SD4-054 | docs/STATUS.md:942 | GIC RX-IRQ Group 0, nicht gefixt; `gpu_mem`-Falle | trap | QUIRK-002 (docs/MEMORY.md:2538, Korrektur :2591) + Auto-Memory pl011-rx-irq…; `gpu_mem` in boot/rpi4/config.txt:25,32-37 | archive |
| SD4-055 | docs/STATUS.md:944 | AES Object Manager, OBJECT 32 B | obsolete | Groesse jetzt ABI-abhaengig (resource.rs:903-904) | archive |
| SD4-056 | docs/STATUS.md:946 | `form_alert` + i18n | obsolete | Abgeschlossen | archive |
| SD4-057 | docs/STATUS.md:948 | G_IMAGE/G_ICON | obsolete | Abgeschlossen | archive |
| SD4-058 | docs/STATUS.md:950 | Ressourcen-Speichermodell: System-RSC eingebettet | decision | Gilt (aes/fsel.rs:98 `include_bytes!`), steht in keinem ADR → vor Archiv als Addendum zu ADR-013 | archive |
| SD4-059 | docs/STATUS.md:952 | TEDINFO-Text | obsolete | Abgeschlossen | archive |
| SD4-060 | docs/STATUS.md:954 | `form_do` + `objc_change` | obsolete | Abgeschlossen | archive |
| SD4-061 | docs/STATUS.md:956 | `objc_edit` | obsolete | Spaeter EmuTOS-treu ersetzt (Z. 1032) | archive |
| SD4-062 | docs/STATUS.md:958 | Nur 768 MiB, Mehrbank-RAM offen | fact | Offener Punkt steht in docs/STATUS.md:435; Auto-Memory framebuffer-outside-st-tt-ram.md („768 MiB genug“) | archive |
| SD4-063 | docs/STATUS.md:960 | Dialog-Story + `form_error` | obsolete | Abgeschlossen | archive |
| SD4-064 | docs/STATUS.md:962 | Menu Manager bar+state | obsolete | Abgeschlossen | archive |
| SD4-065 | docs/STATUS.md:964 | Dropdown-Tracking, MN_SELECTED | obsolete | Abgeschlossen | archive |
| SD4-066 | docs/STATUS.md:966 | Window Manager, prozeduraler Rahmen | obsolete | Durch `w_bldactive`-Baum ersetzt (window.rs:313,456; ADR-022) | archive |
| SD4-067 | docs/STATUS.md:968 | Interaktiver Frame, WM_*-Messages | obsolete | Abgeschlossen | archive |
| SD4-068 | docs/STATUS.md:970 | `graf_*` 70–79 | obsolete | Abgeschlossen | archive |
| SD4-069 | docs/STATUS.md:972 | Multi-Fenster, Rechteckliste, EC1 | obsolete | Abgeschlossen | archive |
| SD4-070 | docs/STATUS.md:974 | Natives `.RSC` (ADR-013) | decision | ADR-013 docs/MEMORY.md:378 | archive |
| SD4-071 | docs/STATUS.md:976 | `rsrc_load` von SD, `C:\GEMSYS\RSC\` | obsolete | Pfadvorsatz entfallen, `sh_find` (Z. 744); `TEST.RSC` Testprodukt | archive |
| SD4-072 | docs/STATUS.md:978 | Klassisch→nativ-Konverter mkrsc | obsolete | tools/mkrsc.py vorhanden | archive |
| SD4-073 | docs/STATUS.md:980 | File Selector Batch 1 | obsolete | `GEM.RSC` Bauprodukt | archive |
| SD4-074 | docs/STATUS.md:982 | File Selector Batch 2 | obsolete | Abgeschlossen | archive |
| SD4-075 | docs/STATUS.md:984 | Scrap + Shell, „AES komplett“ | obsolete | „komplett“ von Z. 1033 korrigiert; shel_write-Modi durch Z. 1046 | archive |
| SD4-076 | docs/STATUS.md:986 | AES-Showcase `aes_demo` | obsolete | Boot startet es nicht mehr (Z. 1006) | archive |
| SD4-077 | docs/STATUS.md:988 | 3D-Objekte | decision | Auto-Memory aes-dialog-3d-look.md | archive |
| SD4-078 | docs/STATUS.md:990 | 3D-Look an EmuTOS angeglichen | obsolete | Abgeschlossen | archive |
| SD4-079 | docs/STATUS.md:992 | Kooperativer Scheduler Fundament | obsolete | Spaeter ausgebaut (sched.rs) | archive |
| SD4-080 | docs/STATUS.md:994 | Desktop Batch 1 (Kernel-`desktop.rs`) | obsolete | Kernel-Desktop entfernt, ADR-026 (Z. 513) | archive |
| SD4-081 | docs/STATUS.md:996 | `desk/`-Extraktion nachgeholt (Regel-1-Verstoss) | obsolete | docs/api/desktop.md vorhanden; Lehre = LESSON-001 docs/MEMORY.md:2762 | archive |
| SD4-082 | docs/STATUS.md:997 | Dropdown-Menues Desktop | obsolete | Ueberholt (Z. 513) | archive |
| SD4-083 | docs/STATUS.md:998 | Verzeichnisfenster WIP, Aliasing-Bug | obsolete | Ueberholt (Z. 513) | archive |
| SD4-084 | docs/STATUS.md:999 | Desktop-Background-Paint-Hook | obsolete | Ueberholt (Z. 513) | archive |
| SD4-085 | docs/STATUS.md:1000 | Desktop-Event-Loop | obsolete | Ueberholt (Z. 513) | archive |
| SD4-086 | docs/STATUS.md:1001 | Menue-Gruppe B Dateioperationen | obsolete | Ueberholt (Z. 513) | archive |
| SD4-087 | docs/STATUS.md:1002 | Menue-Gruppe A | obsolete | Ueberholt (Z. 513) | archive |
| SD4-088 | docs/STATUS.md:1003 | Trash+Printer-Fixtures | obsolete | Printer entfernt (Z. 1036); Desktop ueberholt (Z. 513) | archive |
| SD4-089 | docs/STATUS.md:1004 | Echte Laufwerk-Icons | obsolete | `desktop_icons.rs` weg; `bbd277c` BAD | archive |
| SD4-090 | docs/STATUS.md:1005 | Eject/Blitter/Cache entfernt, CONF-Lehre | trap | Lehre in Auto-Memory check-conf-gating-emutos-extraction.md; `794bdc5`/`434013f` BAD (≈`e3903f64`/`657e35ec`) | archive |
| SD4-091 | docs/STATUS.md:1006 | Desktop ist Boot-Anwendung | obsolete | Durch AES-Shell-Loop (Z. 1047) und ADR-026 (Z. 513); `aef52f9` BAD | archive |
| SD4-092 | docs/STATUS.md:1007 | Erster Desktop-Boot auf HW, 3 MMU-Bugs | decision | ADR-014 docs/MEMORY.md:459; Hashes BAD (≈`e8fa10b2`) | archive |
| SD4-093 | docs/STATUS.md:1008 | Doppelklick, `gl_dcrates`, `#E` | obsolete | Abgeschlossen; Desktop ueberholt (Z. 513) | archive |
| SD4-094 | docs/STATUS.md:1009 | Programme starten + AUTO-Ordner | obsolete | launch.rs vorhanden; AUTO-Regel Auto-Memory auto-folder-console-only.md | archive |
| SD4-095 | docs/STATUS.md:1010 | Shell-Schleife kernel-getrieben | obsolete | Entscheidung „Desktop bleibt Kernel-Modul“ widerrufen durch Z. 513 (ADR-026) | archive |
| SD4-096 | docs/STATUS.md:1011 | `g_patcol`/NEWDESK.INF | obsolete | Lebt in programs/desktop/src/inf.rs; Hashes BAD | archive |
| SD4-097 | docs/STATUS.md:1012 | USB-HID D1–D6, ADR-015 | decision | ADR-015 docs/MEMORY.md:519 | archive |
| SD4-098 | docs/STATUS.md:1013 | USB-A-Bring-up brcmstb/VL805 | obsolete | Spec docs/api/usb/pcie-rpi4.md; DMA-Mapping durch Z. 1016/1015 ueberholt | archive |
| SD4-099 | docs/STATUS.md:1014 | XBIOS-Clock; Keyboard-XBIOS out of scope | obsolete | Keytbl/Kbrate/Bioskeys spaeter implementiert (Z. 835, xbios/mod.rs:99,124); RTC → Auto-Memory rtc-ds3231-plan.md | archive |
| SD4-100 | docs/STATUS.md:1015 | VL805-Hub laeuft, 64-Byte-Context | trap | Falle im Code kernel/src/drivers/usb/xhci/mod.rs:64-71,1361,1401; Branch `archive/vl805-hub-investigation` existiert nicht; Hashes BAD (≈`cb6c4758`) | archive |
| SD4-101 | docs/STATUS.md:1016 | (superseded) Hub abgetrennt | obsolete | Selbst als superseded markiert, durch Z. 1015 | archive |
| SD4-102 | docs/STATUS.md:1017 | API-konformes Objekt-Rendering | obsolete | AES-Teil in object.rs; Desktop-Teil ueberholt (Z. 513) | archive |
| SD4-103 | docs/STATUS.md:1018 | `vq_extnd`, `Cursconf`, `Setexc`-Shadow | obsolete | Shadow dokumentiert tos-api/src/bios/vectors.rs:1-13 + docs/api/bios/05-setexc.md | archive |
| SD4-104 | docs/STATUS.md:1019 | Desktop→AES Schritt 3 | obsolete | Ueberholt (Z. 513) | archive |
| SD4-105 | docs/STATUS.md:1020 | Desktop→AES Schritt 2 | obsolete | Ueberholt (Z. 513) | archive |
| SD4-106 | docs/STATUS.md:1021 | Desktop→AES Schritt 1 | obsolete | Ueberholt (Z. 513) | archive |
| SD4-107 | docs/STATUS.md:1022 | Fdup/Fforce, 15 out-of-scope | obsolete | Out-of-scope-Liste widerrufen (Getmpb/Kbshift Z. 836/762, Keyboard-XBIOS Z. 835) | archive |
| SD4-108 | docs/STATUS.md:1023 | Buttons feuern auf Release | obsolete | Abgeschlossen | archive |
| SD4-109 | docs/STATUS.md:1024 | 3D-Dynamik, `evnt_button`-Doppelklick | obsolete | Abgeschlossen | archive |
| SD4-110 | docs/STATUS.md:1025 | Keyboard-Layouts, `#K`, `C:\KEYBOARD.KBD` | decision | Gilt (kernel/src/keymap.rs:13-14), NewTOS-Erweiterung nicht in COMPATIBILITY.md → dort nennen | archive |
| SD4-111 | docs/STATUS.md:1026 | Scancodes an die AES | obsolete | Abgeschlossen | archive |
| SD4-112 | docs/STATUS.md:1027 | `form_do` SELECTABLE/Radio | obsolete | Abgeschlossen | archive |
| SD4-113 | docs/STATUS.md:1028 | Fenster-Scrolling WM_ARROWED | obsolete | Desktop-Teil ueberholt (Z. 513) | archive |
| SD4-114 | docs/STATUS.md:1029 | Alert-Icons | obsolete | Abgeschlossen | archive |
| SD4-115 | docs/STATUS.md:1030 | `graf_mouse`-Presets | obsolete | Abgeschlossen | archive |
| SD4-116 | docs/STATUS.md:1031 | fsel Pfad-Editierung + Navigation | obsolete | Abgeschlossen | archive |
| SD4-117 | docs/STATUS.md:1032 | `ob_edit` formatierte Felder | obsolete | Abgeschlossen | archive |
| SD4-118 | docs/STATUS.md:1033 | VDI/AES-Audit, 15 AES-Fns nachgezogen | obsolete | 9 Hashes BAD (≈`dea9244d`, `3db0a05d`, `8b75d0a9`, `f6569b23`, `1d90b3f2`, `11c649f2`); Follow-up RSC-Bounds = A38 DONE | archive |
| SD4-119 | docs/STATUS.md:1034 | VDI-Output-Specs Full-Contract, D-VDI-9 | obsolete | D-VDI-9 in docs/api/vdi/_discrepancies.md:28; Datum ausser Reihe | archive |
| SD4-120 | docs/STATUS.md:1035 | Desktop→AES Schritt 4 `objc_find` | obsolete | Ueberholt (Z. 513) | archive |
| SD4-121 | docs/STATUS.md:1036 | Icon-Politur, Printer entfernt | obsolete | `92c85fb`→`66db5d05` (map); ueberholt (Z. 513) | archive |
| SD4-122 | docs/STATUS.md:1037 | Drei HW-Rendering-Bugs, `DESK_PAINTER` | obsolete | Kernel-Desktop-Hook entfallen (Z. 513) | archive |
| SD4-123 | docs/STATUS.md:1039 | Solid Drag + make-sd-image.sh | obsolete | Solid Drag durch EmuTOS-`drag_move` ersetzt (window.rs:5481-5483); SD-Tool-Regel in Auto-Memory auto-folder-console-only.md | archive |
| SD4-124 | docs/STATUS.md:1041 | Desktop-Politur nach HW-Test | obsolete | Ueberholt (Z. 513) | archive |
| SD4-125 | docs/STATUS.md:1043 | Look-Angleichung Titelleiste | obsolete | Rahmen-Soll heute Auto-Memory hatari-falcon-window-frame-reference.md (A79) | archive |
| SD4-126 | docs/STATUS.md:1044 | OUTLINED-Doppelrahmen | obsolete | Abgeschlossen | archive |
| SD4-127 | docs/STATUS.md:1045 | VDI Inquiry/Raster Full-Contract, D-VDI-11..14 | obsolete | D-VDI-11..14 in docs/api/vdi/_discrepancies.md:30-33 | archive |
| SD4-128 | docs/STATUS.md:1046 | AES Resource/fsel/Scrap/Shell Full-Contract, D-054 | obsolete | D-054 in docs/api/_discrepancies.md:1603 | archive |
| SD4-129 | docs/STATUS.md:1047 | AES-Shell-Loop, `#Z` einmalig, DESKTOP_APP-Abweichung | decision | `#Z`-Semantik in docs/api/aes/shell-loop.md:178-212; Rueckfall auf `desk/` ueberholt (Z. 513/616, shell-loop.md:630ff); Abweichung nur in shell_loop.rs:28,305 → COMPATIBILITY.md; `blit_plane`-Befund in TASK/A52 (DONE); falsch einsortiert | archive |
| SD4-130 | docs/STATUS.md:1048 | Referenzdaten fallweise geprueft, `d0`/flip_y, xtask test | obsolete | Hashes map OK; `GLOBAL_EXCLUDED` heute `[11,12]` (tos-api/tests/aes_reference/harness.rs:1268); WF_OWNER/`ap_3resv` deckungsgleich mit docs/analysis/ORACLE.md §4-5; falsch einsortiert | archive |
| SD4-131 | docs/STATUS.md:1050-1055 | Notes: bewusst ueberdokumentiert, Spec-first gegen Drift | decision | Kein Change-Log; Spec-first-Kern auch in LESSON-001 (docs/MEMORY.md:2762) und CLAUDE.md Regel 5, die Rahmung „bewusst ueberdokumentiert“ nur hier | keep |

## docs/MEMORY.md (ADRs, Quirks, Lessons, Rejected)

Quelle: docs/MEMORY.md Zeilen 1–2851. Checker: 431 Zitate, OK=325, OLD-HASH-MAPPED=24, MISSING=68, AMBIGUOUS=7, BAD-COMMIT=3, EMUTOS-COMMIT=3, FREEMINT-COMMIT=1.

### Systemische Befunde
- **ARCH03 hat kein ADR in MEMORY.md.** Der gemeinsame Adressraum (VA = PA, Rechte je Prozess) ist umgesetzt (TASK/ARCH03.md:3, :53-66; tos-api/src/bdos/process/usermem.rs:1-12), sein ADR steht nur als Entwurf „ADR-025, Status: Proposed, Supersedes ADR-010, ADR-021“ in docs/analysis/arch03-implementation-plan.md:28-31. MEMORY.md vergibt ADR-025 am 2026-10-13 an den MD-Vorrat. **Die Nummer ist doppelt belegt.** docs/analysis/ldg-after-arch03-2026-10-06.md:32 und docs/analysis/libshare-2026-10-06.md:51 meinen mit „ADR-025“ den Adressraum. Folge: ADR-010 („NOT YET IMPLEMENTED“) und ADR-021 („Accepted“, Fenster je Prozess) haben einen falschen Status. Der Mechanismus von ADR-023 ist veraltet. ADR-027 (Context) beruft sich auf ARCH03, ohne dass es dafür ein ADR gibt. Vor jeder Archivierung von ADR-010/021 muss das ARCH03-ADR unter einer freien Nummer (ADR-028) in MEMORY.md landen.
- **ADR-001…009 wurden nie geschrieben.** Die Nummerierung beginnt seit dem Initial-Commit `265a43c1` bei ADR-010. Es gibt zwei widersprüchliche Reservierungslisten: MEMORY.md:448-457 (001 = Lizenz) und docs/ARCHITECTURE.md:155-161 (001 = PRG-Format, 002 = Lizenz). Nichts davon liegt in phases/ oder STATUS.md. De facto erledigt: Lizenz = GPLv2+ (`LICENSE`, `aec5b730`, ohne ADR; docs/REQUIREMENTS.md:77 sagt noch „pending, see ADR-001“). Pin `1a7484ab` steht in docs/analysis/ORACLE.md §1, aber docs/api/desktop/_overview.md:4 zitiert ein nicht existierendes „MEMORY.md ADR-003“. Das PRG-Format ist ADR-018. Das offene „ADR-Supexec“ (STATUS.md:341, docs/api/xbios/26-supexec.md:207 „to be filed in docs/MEMORY.md“) fehlt ebenfalls.
- **Desktop-Umzug.** 30 der 68 MISSING sind `kernel/src/desk/*`, `desktop.rs` und `desktop_icons.rs` (nach programs/desktop/ verschoben, DESK43 U8b `90b04444`). In ADR-016/017/022 ist das Geschichte. Zu ändern ist es in ADR-018 (.NLD-Amendment, Change surface).
- **Alte Hashes ohne Konvention.** 24 Zitate sind Hashes von vor dem 2026-10-19 (OLD-HASH-MAPPED). MEMORY.md erwähnt `docs/commit-map.txt` und `tools/oldhash.sh` nirgends (grep ohne Treffer; die Konvention steht nur in STATUS.md:516). Die 3 BAD-COMMIT in ADR-014 waren schon **vor** dem Umschreiben falsch: die echten Commits sind `e8fa10b2`/`af37a4dc`/`e82cf6d3` (alt `2fb1af4`/`b1d5f7b`/`a858961`).
- **Hausregel und Ordnung.** Die Regel „ADRs werden nicht geändert, Ersatz = neues ADR“ (Z.13-14, 20-22) wird von Änderungen an Ort und Stelle in ADR-012/013/017/018/019/020/023 widerlegt. Statuszeilen wurden dabei nicht nachgezogen (ADR-010, 016, 017, 021, 022). ADR-021…027 stehen hinter dem ADR-Template (Z.1365), der Block „Anticipated“ steht zwischen ADR-013 und ADR-014. 28 Wiki-Links `[[…]]` zeigen in das private Auto-Memory außerhalb des Repos (alle 17 Ziele existieren lokal).
- Checker-Rauschen: FreeMiNT-Pfade (`sys/memory.c`, `sys/mint/mem.h`, `sys/dosmem.c`, `sys/arch/mprot030.c`, `sys/k_exec.c`) und Circle-Pfade lösen unter references/freemint/ und references/circle/ auf und stimmen inhaltlich. Die ILP32-Skripte in ADR-020 sind wie beschrieben gelöscht. `ex_lib.c` = sdk/ldg/ldg-r133/demo/libs/ex_lib.c:66-69 stimmt.

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| MD-001 | docs/MEMORY.md:1-16 | Kopf: dauerhaftes Projektwissen, bei Sitzungsbeginn lesen, append-only | decision | „Corrections = superseding entry“ wird durch Änderungen an Ort und Stelle in 7 ADRs widerlegt; Leseregel ist Duplikat von CLAUDE.md (Companion-Liste). Regel an Praxis anpassen (Amendments erlaubt, Statuszeile pflegen) und Hash-Konvention (commit-map/oldhash.sh) ergänzen | fix |
| MD-002 | docs/MEMORY.md:18-36 | „How to use“: ADR/Quirk/Lesson/Rejected, ADRs global nummeriert, nie geändert | decision | Nummer ADR-025 doppelt (arch03-implementation-plan.md:28 vs MEMORY.md:2131); Lücke 001–009 unerklärt; „not modified“ s. MD-001. Hinweis auf reservierte/ungenutzte Nummern ergänzen | fix |
| MD-003 | docs/MEMORY.md:38-130,181-185 | ADR-010: ein gemeinsamer Anwenderadressraum, VA = PA | obsolete | Status „NOT YET IMPLEMENTED, per-process TTBR0 still in force“ falsch: VA = PA umgesetzt mit ARCH03 Stufe 1 (usermem.rs:1-12, TASK/ARCH03.md:59), aber mit TTBR0-Wurzel je Prozess für Rechte statt einer Wurzel. Widerspricht sich selbst (Z.47 „not superseded“ vs Z.184 „superseded“). Entwurf arch03-implementation-plan.md:31 „Supersedes ADR-010“. Kernentscheidung zuerst ins ARCH03-ADR übernehmen | archive |
| MD-004 | docs/MEMORY.md:131-179 | ADR-010 Implementation note: Stage 0 (Heap über High-Half) erledigt, Stage 1 an QEMU-TCG blockiert | obsolete | `55b4390`→`afa3a132` OK. „Wand 3“ (Aliasing hängt QEMU) widerlegt: ARM ARM B2.11 und QEMU 11.0.2 laufen (arch03-implementation-plan.md:19 §0 Nr. 6, arch03-stage0b-spike.md §3). Nur noch Geschichte | archive |
| MD-005 | docs/MEMORY.md:187-256 | ADR-011: physische VDI-Workstation je Gerät, mehrschirmfähig | decision | Status „Accepted“, aber nicht umgesetzt: kein Screen-Device-Register, ein `Host::raster()` (tos-api/src/host.rs:173), Palette gerätelokal-global (kernel/src/vdi_backend.rs:144-153). Zitat `vdi/lineavars.S` falsch → EmuTOS `bios/lineavars.S`. Status auf „accepted, not implemented“ setzen | fix |
| MD-006 | docs/MEMORY.md:260-264,276-374 | ADR-012: kerneleigene Keksdose, Zugriff per Wert über FreeMiNT `Ssystem` | decision | Kern stimmt: fester Vorrat `JAR_CAP = 32` (tos-api/src/cookie.rs:20), Ssystem. Consequences veraltet: `_FLK` wird nicht gesetzt, `EdDI` schon (cookie.rs:221-227); „eigene Getcookie/Setcookie-IDs reservieren“ durch Ssystem-Entscheid (Z.327) überholt. EmuTOS machine.c:665 (`fill_cookie_jar`) OK | fix |
| MD-007 | docs/MEMORY.md:265-274,290-291 | ADR-012 Amendment 2026-07-24 (B1): `$5A0` je Prozess als Spiegel der Kerndose | decision | `3d1c1fe`→`4848540f` = „populate low-memory sysvars + OSHEADER + $5A0 per-process (B1)“; Seite 0 bleibt nach ARCH03 je Prozess privat (arch03-implementation-plan.md Decision 1). b1-lowmem-feasibility.md vorhanden | keep |
| MD-008 | docs/MEMORY.md:378-425 | ADR-013: natives `.RSC`-Format (NRSC, LE, 64-Bit-OBJECT) | decision | Natives Format gilt weiter. Überholt: „Koordinaten schon in Pixeln, `rsrc_obfix` Identität“. Seit NRSC v9 Zeichenraster, Umrechnung beim Laden wie EmuTOS `rs_obfix` (tos-api/src/aes/resource.rs:9-13, :328-345), ab v13 natives RSHDR (:357-380) | fix |
| MD-009 | docs/MEMORY.md:427-444 | ADR-013 Addendum 2026-07-20: Free-String/Free-Image-Tabellen, Versionen 1–4 | fact | Tabellen vorhanden (`NRSC_VER_TABLES = 3`, resource.rs:287-290). Versionsliste endet bei 4, Code kennt v5–v13 (resource.rs:17-30, :292-380). Auf „siehe resource.rs“ verweisen | fix |
| MD-010 | docs/MEMORY.md:448-457 | „Anticipated further ADRs“ 001–006 | obsolete | Nie geschrieben. Widerspricht ARCHITECTURE.md:155-161 (andere Nummern). Erledigt ohne ADR: Lizenz GPLv2+ (`aec5b730`), Pin → ORACLE.md §1, PRG-Format → ADR-018, Mxalloc → ADR-021/023/027. Mitziehen: docs/api/desktop/_overview.md:4 (ADR-003), REQUIREMENTS.md:77 (ADR-001) | archive |
| MD-011 | docs/MEMORY.md:459-517 | ADR-014: alle MMIO über die High-Half, TTBR0 ohne Low-Half-Identität | decision | Gilt: `mmio_va` (kernel/src/drivers/gic.rs:238-253, mailbox.rs:20), `EMPTY_L0_PA` (tos-api/src/bdos/process/mmu.rs:267). BAD-COMMIT `7231e9d`/`c91c9e5`/`3f93fde` existieren nirgends; richtig sind `e8fa10b2` (MMIO high-half), `af37a4dc` (Framebuffer), `e82cf6d3` (Cache/Barrier), alle 2026-06-25 | fix |
| MD-012 | docs/MEMORY.md:519-584 | ADR-015: USB-HID über xHCI (VL805), oberer Stack nach FreeMiNT | decision | Gilt, läuft auf der Pi (STATUS.md:332, kernel/src/drivers/usb/xhci/mod.rs:3-13). Consequence „feste MMIO `0xFE9C0000`, keine PCIe-Enumeration“ widerlegt: kernel/src/drivers/usb/pcie_rpi4.rs:1-21 fährt den RC hoch, enumeriert den VL805, BAR bei `0x6_0000_0000`. UART2-IKBD tatsächlich entfernt (kein ikbd_uart.rs), Auto-Memory keyboard-via-atari-ikbd.md behauptet das Gegenteil | fix |
| MD-013 | docs/MEMORY.md:586-680 | ADR-016: Desktop-Neubau EmuTOS-treu als Modul `kernel/src/desk/` | obsolete | Status noch „Accepted“. Abgelöst durch ADR-022 (Prinzip) und ADR-026 (Kernel ohne Desktop, programs/desktop/). Pfade weg. Rollenaufteilung lebt in .claude/agents/ | archive |
| MD-014 | docs/MEMORY.md:682-690,744-1004 | ADR-017 (2026-07-09): 1920×1080, zur Laufzeit auf 1280×800 umschaltbar, 16 Farben | obsolete | Umschaltung gestrichen (Rev. 2, Z.693-712), Farbmodell durch ADR-024 ersetzt. Context veraltet: statt `const WIDTH 640` jetzt Abfrage per Tag `0x0004_0003` mit Fallback (kernel/src/drivers/bcm2711/framebuffer.rs:29-33,279-310). `kernel/src/desk/{global,app,menu}.rs` weg. EmuTOS `screen.c:968-970` verschoben (`rez = 2` steht bei :971-972). Titel/Status nennen die alte Entscheidung. Bleibendes (Geometrie-Unabhängigkeit als Regel, Getrez-Frage) vorher in Rev. 2 übernehmen | archive |
| MD-015 | docs/MEMORY.md:691-724 | ADR-017 Revision 2 (2026-07-16): Auflösung aus config.txt, Auto-Erkennung, kein Menüwechsel | decision | Umgesetzt: framebuffer.rs:29-33 (`FALLBACK_*` = `BOOT_WIDTH/HEIGHT`), :279-310 (Tag `0x0004_0003`); boot/rpi4/config.txt:14-15. Veraltet: `paint_desktop_bg` gibt es nicht mehr; „Wallpaper erst nach Truecolor“ ist mit ADR-024 erfüllt. Gehört in Titel und Statuszeile | fix |
| MD-016 | docs/MEMORY.md:726-742 | ADR-017 „Superseded draft“: fester 640×480/TT_MEDIUM-Modus | obsolete | Reine Geschichte. Steht auch in docs/api/_discrepancies.md:2660-2760 (D-076/077/078) | archive |
| MD-017 | docs/MEMORY.md:1006-1050,1116-1174 | ADR-018: native Endungen `.NAP`/`.NCP`/`.NTP`, m68k-Endungen reserviert | decision | Endungen gelten (programs/desktop/src/ftype.rs:14,80-96). AUTO nur `.NCP` (kernel/src/launch.rs:369-397). Routing, Change surface und Staged plan (Task #72) sind Geschichte: Start seit ADR-026 nur per `shel_write` (programs/desktop/src/main.rs:1070), `kernel/src/desk/*` und `desktop.rs` weg, HELLO.NCP auf C: (tools/make-sd-image.sh:105). Titel nennt `.NAC`, Tabelle schon | fix |
| MD-018 | docs/MEMORY.md:1052-1086 | ADR-018 Amendment 2026-09-24: `.NAC`, Suche `C:\ACC\` vor der Wurzel, `NUM_ACCS` gemeinsam | decision | `count_accs` kernel/src/accs.rs:130. EmuTOS gemasm.S:73-131, geminit.c:264-284 (`count_accs`), sysconf.h:37-40 am Pin geprüft. ACC02 DONE | keep |
| MD-019 | docs/MEMORY.md:1088-1114 | ADR-018 Amendment 2026-10-06: `.NLD` = natives LDG, Desktop behandelt es als Dokument | decision | LDG-M1 DONE; crt0/newtos.ld/build-ldg-demo.sh vorhanden. `kernel/src/desk/ftype.rs::classify` → programs/desktop/src/ftype.rs:80,96. ex_lib.c:66-69 = sdk/ldg/ldg-r133/demo/libs/ex_lib.c, stimmt | fix |
| MD-020 | docs/MEMORY.md:1176-1230,1241-1254 | ADR-019: Atari-C-Bibliotheken portieren, nicht neu schreiben; SDK getrennt | decision | Gilt. Duplikat: Auto-Memory port-reuse-dont-rewrite.md. `sdk/libc/newtos-arch/crt0.S` existiert nicht → sdk/libc/crt0.S. „App sakrosankt“ (Z.1223-1229) ist durch das Amendment überholt, steht aber unmarkiert. „Verified“ ohne Messlauf, nur sdk/libc/tests/libtest.c | fix |
| MD-021 | docs/MEMORY.md:1231-1239 | ADR-019 Amendment 2026-10-20: SDK-Fixes vs. App-Patches in `sdk/overlays/<prog>/` | decision | sdk/overlays/{qed,teradesk,hypview,coremark} vorhanden, sdk-inventory-2026-10-20.md vorhanden. Duplikat: Auto-Memory port-target-app-unmodified.md | keep |
| MD-022 | docs/MEMORY.md:1256-1312 | ADR-020: App-ABI ILP32 | obsolete | Status korrekt „Superseded“ (LP64 seit 2026-07-21). `14a1f32`→`246c04ca` OK | archive |
| MD-023 | docs/MEMORY.md:1314-1363 | ADR-020 Addendum 2026-09-04: ILP32-Bau entfernt, Laufzeitschalter bleibt | obsolete | Veraltet: der Lader setzt `Abi::Ilp32` nicht mehr, er lehnt ELFCLASS32 ab (`LoadError::Ilp32Unsupported`, tos-api/src/bdos/process/loader.rs:424-426). `relocate_blob_ilp32` heißt `relocate_blob_core_ilp32` (resource.rs:1640). 63 Stellen in 15 Dateien statt ~65/17. Die Begründung für den Schalter steht aktuell in loader.rs:410-423 | archive |
| MD-024 | docs/MEMORY.md:1365-1404 | ADR-Template | decision | Inhalt ok. Steht mitten in der ADR-Folge, ADR-021…027 dahinter. Ans Ende des ADR-Teils verschieben | fix |
| MD-025 | docs/MEMORY.md:1408-1518 | ADR-021: EL0-Speicher über ein gemapptes Fenster je Prozess, Pool bleibt EmuTOS-treu | obsolete | Fenster ist zurückgebaut (usermem.rs:10-12 „window … is gone“; TASK/ARCH03.md:10,59). Aufschub von `F_PROTMODE` überholt durch ARCH03 3b und ADR-027. Status noch „Accepted“. FreeMiNT-Zitate (memory.c:210-211, mem.h:130) stimmen. „Pool-Allokator bleibt EmuTOS-treu“ zitiert ADR-025 (Z.2175-2177, 2236): vorher ins ARCH03-ADR übernehmen | archive |
| MD-026 | docs/MEMORY.md:1520-1623 | ADR-022: Desktop 1:1 nach EmuDesk, gleiche Aufruffolge, alles über den AES-Trap | decision | Prinzip gilt (ADR-026 stützt sich darauf). Endgame erfüllt durch ADR-026, Status nennt das nicht. `kernel/src/desk/` → programs/desktop/. EmuTOS gemgraf.c:369 (`gsx_trans`), deskmain.c:72 (`menu_text`-Makro) geprüft | fix |
| MD-027 | docs/MEMORY.md:1627-1776 | ADR-023: was der Kern herausgibt, muss erreichbar sein; FreeMiNT-`PROT_*` übernommen | decision | Vokabular und Grundsatz gelten (tos-types `ProtMode`; FreeMiNT mem.h:118,120-136, dosmem.c:209-221 geprüft). Duplikat: Auto-Memory adopt-freemint-protection-model.md. Mechanismus veraltet: `expose` trägt jetzt bei VA = PA in die gemeinsamen Tabellen ein und liefert die PA, sichtbar für alle Prozesse (usermem.rs:14-24, :180). `Kind::Shared`, Fenster und Bump-Cursor gibt es nicht mehr. EmuTOS blkdev.c:589 (Funktion beginnt :569) | fix |
| MD-028 | docs/MEMORY.md:1778-1841 | ADR-023 Amendment 1: Physbase/Logbase als `PROT_G`, Normal Non-Cacheable, civac-Umschaltung | fact | Gilt: `MemAttr` (usermem.rs:70), `UserRwNxUncached` (usermem.rs:225), `direct_screen_active` und `clean_invalidate` (kernel/src/vdi_backend.rs:323-333). Veraltet: Messwert „Physbase = 0x00400000 = Fensterbasis“ (seit VA = PA ist es die PA). Kein Messlauf-Verzeichnis zitiert | fix |
| MD-029 | docs/MEMORY.md:1843-1929 | ADR-023 Amendment 2: Setscreen schaltet echt um, doppelte virtuelle Höhe, Tag `0x0004_8009` | fact | Gilt: `set_virtual_offset`/`WANTED_PAGES` (framebuffer.rs:427,457), `resolve_kva` (usermem.rs:308). Circle-Zitate stimmen. Veraltet: `USER_L3_COUNT` 10→16 (Fenster weg, Symbol fehlt). EmuTOS `screen.c:1030` → `atari_physbase` steht bei :917/:1049; `:1071` → `v_bas_ad` bei :1073 | fix |
| MD-030 | docs/MEMORY.md:1933-2127 | ADR-024: VDI-Truecolor, RGB565-Gerätepixel über XRGB8888 | decision | EmuTOS-Zitate am Pin geprüft (config.h:1628-1630,2198-2201; vdi_defs.h:22,153-158; vdi_control.c:206,210; videl.c:320,969-974; gemoblib.c:1327,1336). NewTOS-Zitate verschoben: `NUM_COLORS` :36 statt :33, `PLANES` :43 statt :40; `pal555_to_xrgb` gibt es nicht mehr, `device_to_rgb565` steht bei :479-493. „`docs/MEMORY.md` [[emutos-target-profile-falcon-tos4]]“ zeigt ins Auto-Memory. Hashes OLD-HASH-MAPPED | fix |
| MD-031 | docs/MEMORY.md:2131-2236 | ADR-025: MD-Vorrat wächst seitenweise im Budget, Grenze je Prozess | decision | Code stimmt: `MD_PAGE_BUDGET = 512` (tos-api/src/bdos/mem/pages.rs:47), `DEFAULT_OWNER_MD_LIMIT = 8192` (owner_limit.rs:48), Heap 8 MiB (kernel/src/bios/machine/heap.rs:52-56), `mdlimit=` (bootargs.rs:94). Tests mem05_* und Lauf mem05-kernel vorhanden. EmuTOS osmem.c und tos14fix.txt geprüft. MEM10 offen wie angegeben. Nummer kollidiert mit dem ARCH03-Entwurf (s. Systemik); dieser Eintrag behält sie | keep |
| MD-032 | docs/MEMORY.md:2240-2353 | ADR-026: Desktop ist EL0-Programm NEWDESK, Kern ohne Desktop | decision | Belegt: Test tools/xtask/src/programs_tests.rs:361, `DESKTOP_LOAD_ERROR` kernel/src/shell_loop.rs:157, `KERNEL_DTA` tos-api/src/bdos/fs/find.rs:67, Abhängigkeiten programs/desktop/Cargo.toml, kernel/embed.rs. EmuTOS-Zitate (gemshlib.c:570-612, deskmain.c:2154-2166) geprüft. Offene HW-Punkte decken sich mit TASK/DESK43.md:3,163,183-184 | keep |
| MD-033 | docs/MEMORY.md:2357-2423 | ADR-027: Speicherverwaltung des Kerns nach FreeMiNT, auch wo sichtbar | decision | Gilt. Duplikat: Auto-Memory kernel-memory-follows-freemint.md. Benannt in COMPATIBILITY.md:921. MEM06/07/08/10 und ARCH03 2b ohne DONE (offen wie angegeben). Context stützt sich auf ARCH03 ohne ADR | keep |
| MD-034 | docs/MEMORY.md:2431-2494,2512-2525 | QUIRK-001: QEMU-TCG hängt beim TTBR0-Wechsel; UART-Schreibzeile als Barriere | trap | Barriere steht im Code (kernel/src/arch/aarch64/exceptions.rs:1138,1219,1703-1716,2266). „ASIDs nicht neu versuchen“ (Z.2492-2493) ist durch das Update 2026-10-06 überholt, aber nicht markiert. Ursache „Eltern und Kind bei VA 0x100“ tritt seit ARCH03-0d (PIE-Lader, Bild bei B+0x1000) nicht mehr auf | fix |
| MD-035 | docs/MEMORY.md:2495-2510 | QUIRK-001 Update 2026-10-06: ASIDs hinter `asid=on`, QEMU 11.0.2 ohne Hänger | fact | tos-api/src/bdos/process/asid.rs vorhanden, Lauf measurement-2026-10-06-arch03-4 vorhanden. Pi-Lauf offen, deckt sich mit TASK/ARCH03.md:66 | keep |
| MD-036 | docs/MEMORY.md:2527-2536 | „Other quirks worth recording“: 4 Beispiel-Quirks (PL011 FIFO, Fopen leer, Basisadressen, GPIO-Pull) | hypothesis | Template-Beispiele aus dem Initial-Commit `265a43c1`, nie beobachtet, ohne Beleg (z. B. „Fopen leer → EFILNF“) | delete |
| MD-037 | docs/MEMORY.md:2538-2589,2627-2630 | QUIRK-002: GIC-SPIs in Group 0, PL011-RX-IRQ auf echter Pi nie zugestellt | obsolete | Titel, Symptom und Ursache widerlegt: INTID 153 wird zugestellt (eigene Korrektur Z.2591-2625; STATUS.md:771; Auto-Memory pl011-rx-irq-not-delivered-on-real-hw.md „WIDERLEGT“). Resolution veraltet: `pl011_rx_poll` gelöscht, UART2-IKBD entfernt, Eingabe ist USB-HID (ADR-015). Kopf ohne Date/Severity/Status. Bleibendes (`gpu_mem=256` geht beim Kopieren der config.txt verloren; GICD/PL011-Adressen zur Diagnose) vorher in MD-038 übernehmen. STATUS.md:162-166 trägt die alte Behauptung ebenfalls | archive |
| MD-038 | docs/MEMORY.md:2591-2625 | QUIRK-002 Korrektur: `IGROUPR` ist aus NS RAZ, Lesung beweist nichts; Phantomtaste | trap | Beleg: STATUS.md:771, `c6e4bfb`→`b776ffaa`. Kein Messlauf-Verzeichnis. Formuliert „unproven“, das Auto-Memory „widerlegt“ (zweiter Diagnose-Boot: Flush-Zähler 1, Wert 0x00). Auf „widerlegt“ angleichen und zum eigentlichen Quirk machen (Titel „IGROUPR liest aus NS null“) | fix |
| MD-039 | docs/MEMORY.md:2632-2720 | QUIRK-003: IRQ zwischen `msr elr_el1` und `eret` überschreibt das Sprungziel; `daifset` | trap | Fix steht im Code (kernel/src/arch/aarch64/user_entry.rs:177,250). Läufe und method/force_race_patch.py vorhanden. Nur auf QEMU gemessen (RUNINFO.TXT:10,16). Quelle „ARM ARM M.c, nicht in references/“ veraltet: references/arm-arm/DDI0487M_d ist gepinnt. Deutsch in englischer Datei | fix |
| MD-040 | docs/MEMORY.md:2722-2754 | Quirk-Template | decision | Ok. QUIRK-002 hält das Template nicht ein | keep |
| MD-041 | docs/MEMORY.md:2762-2770 | LESSON-001: Quelle zuerst verhindert Drift | decision | Inhalt deckt CLAUDE.md Regel 1 und Regel 5 ab. Kurz, keine Zitate zu prüfen | keep |
| MD-042 | docs/MEMORY.md:2774-2781 | LESSON-002: `rust-lld` kennt `-nostartfiles` nicht | trap | kernel/build.rs:34 übergibt nur `-T`, `ENTRY(_start_phys)` steht in link-rpi4.ld:46. Checker löst `build.rs` falsch auf (tos-api/src/build.rs); gemeint ist kernel/build.rs | keep |
| MD-043 | docs/MEMORY.md:2785-2792 | LESSON-003: Naked Functions seit Rust 1.88 stabil | fact | Code nutzt `#[unsafe(naked)]` ohne Feature-Gate (kernel/src/arch/aarch64/boot.rs:244, kernel/src/lib.rs:183). Widerspricht CLAUDE.md Tech Stack („nightly required for naked_function … until naked functions stabilize“) und dem Kommentar in rust-toolchain.toml:2. Dort korrigieren | keep |
| MD-044 | docs/MEMORY.md:2800-2813 | REJECTED-001: kein Direktport des EmuTOS-C-Codes | decision | Deckt sich mit CLAUDE.md (clean reimplementation) und ADR-022 („not a C port“) | keep |
| MD-045 | docs/MEMORY.md:2817-2825 | REJECTED-002: kein projekteigener MCP-Server | decision | Kein Widerspruch gefunden | keep |
| MD-046 | docs/MEMORY.md:2829-2839 | REJECTED-003: kein m68k-JIT in Phase 10 | decision | „Phase 10“ ist heute PHASE-10-bios-screen. Die m68k-Emulator-Phase liegt in phases/_archive/PHASE-10-m68k-emulator.md:94,109. Duplikat: ARCHITECTURE.md:159 (ADR-005). Phasenbezug korrigieren | fix |
| MD-047 | docs/MEMORY.md:2843-2851 | REJECTED-004: kein `bindgen` über EmuTOS-Header | decision | Duplikat CLAUDE.md Tech Stack („Do not suggest … bindgen“), selbst so vermerkt | keep |

## Auto-Memory (~/.claude/projects/-Volumes-Daten-Development-PiRusTOS/memory/)

Quelle: 91 Dateien (90 Memories + Index MEMORY.md), zusammen 3376 Zeilen; Ort = Dateiname im Memory-Ordner. Checker (je Datei): 339 Zitate, OK=158, MISSING=135, AMBIGUOUS=26, OLD-HASH-MAPPED=17, EMUTOS-COMMIT=2, FREEMINT-COMMIT=1. Typen laut Frontmatter: project 43, feedback 36, reference 10, ohne Typ 1.

### Systemische Befunde
- **Index (MEMORY.md) ist vollständig, aber 13 Hooks widersprechen ihrem Body.** 90 Zeilen = 90 Dateien, keine fehlende Datei, keine Datei ohne Zeile, keine Doppelzeile. Widersprüche: Z.1 (exFAT-Ziel, Body: FAT16 → FAT32 → exFAT), Z.5 (UART2-Pins prüfen, Weg seit `1ace07a4` aufgegeben), Z.8 („append“, Body: newest-first oben), Z.9 („not yet present“, Body: Klon `f906416` da), Z.14 („twice“, Body: drei Fälle), Z.24 (laufende ILP32-App), Z.27/30/41/42/45 (Desktop in `kernel/src/desk/` bzw. EL1, ADR-016 gilt), Z.43 („Vorführbarkeit vor Konformität“ = Gegenteil des Body). Die 90 MISSING-Links in MEMORY.md sind Checker-Rauschen (Links relativ zum Memory-Ordner).
- **Desktop-Umzug (DESK43 U8b `90b04444`, ADR-026) bestätigt:** sieben Memories plus Index Z.30 nennen `kernel/src/desk/` oder `desk/*.rs` (auto-folder-console-only, desktop-1to1-emutos-rebuild, desktop-bypasses-app-abi-path, desktop-must-run-in-user-scope, own-extensions-separate-aarch64-from-m68k, singletasking-app-owns-full-screen, teradesk-is-the-desktop), dazu version-roadmap (`desk/inf.rs`). Ziel heute: `programs/desktop/src/`. Vier davon sind durch ADR-026 inhaltlich erledigt, nicht nur im Pfad.
- **ILP32/ARCH03-Altlast:** aes-object-abi-is-tree-abi, el0-app-memory-must-be-mapped, lp64-tos-struct-trap, native-64bit-abi-future und singletasking-app-owns-full-screen setzen laufende ILP32-Apps oder das ADR-021-Fenster je Prozess voraus. Stand: Loader lehnt ELFCLASS32 ab (tos-api/src/bdos/process/loader.rs:409-426), ARCH03 Stufe 1 = VA = PA mit gemeinsamen Pools (tos-api/src/bdos/process/usermem.rs:1-12).
- **ADR-Duplikate und Verlinkung aus dem Repo.** adopt-freemint-protection-model (= ADR-023), kernel-memory-follows-freemint (= ADR-027), desktop-1to1-emutos-rebuild (= ADR-022), own-extensions (= ADR-018 + Amendment), vdi-truecolor-workstation (= ADR-024), native-64bit-abi-future (= ADR-020), pl011-… (= QUIRK-002-Korrektur). docs/MEMORY.md verlinkt 17 Memory-Slugs per `[[…]]` (alle Ziele existieren), darunter adopt-freemint-protection-model, el0-app-memory-must-be-mapped, desktop-bypasses-app-abi-path, keyboard-via-atari-ikbd, native-64bit-abi-future, vdi-truecolor-workstation. Vor delete/archive diese Links auf das ADR umbiegen. STATUS.md:818 nennt desktop-bypasses-app-abi-path.
- **Memory gegen Memory:** native-64bit-abi-future:42-43 („lp64-Audit wird gegenstandslos“) gegen lp64-tos-struct-trap (Falle hat 2026-09-13 gebissen); fix-vdi-before-aes („alle Findings, nicht App-triagiert“) gegen das spätere real-apps-are-the-arbiter (Parken ohne App-Wirkung); post-2a-roadmap widerspricht sich selbst (Z.27 gegen Z.67); auto-folder-console-only (QEMU-Start über desk-launch-Hook) gegen qemu-dump-runs (#Z); hatari-falcon-window-frame (ROM `references/emutos/etos512us.img` „gepinnt“) gegen emutos-oracle-pinned. Checker-Lücken: `5d4805e` (altitude) existiert nirgends; `2e3ca46`, `59db9be`, `685cd68`, `a7006f3` ohne Backticks nicht erkannt, per commit-map aufgelöst. Frontmatter: real-apps-are-the-arbiter ohne `type`, check-conf-gating mit abgeschnittener description, `[[TASK/A35]]`/`[[TASK/A63]]`/`[[TASK/A64]]` sind Ticketpfade (existieren), keine Slugs. Übriges MISSING/AMBIGUOUS = Karten-/Hostdateinamen (Rauschen).

### Einträge
| ID | Ort | Inhalt (kurz) | Klasse | Prüfergebnis | Aktion |
|---|---|---|---|---|---|
| AM-001 | MEMORY.md:1-90 | [Index] eine Zeile je Memory | fact | 90/90 deckungsgleich, keine Waisen, keine Doppelzeile. Hooks widersprechen Body: Z.1, 5, 8, 9, 14, 24, 27, 30, 41, 42, 43, 45 (s. Systemische Befunde); Z.33 verkürzt („resume via SendMessage“, Body: nur wenn Zustand gebraucht) | fix |
| AM-002 | adopt-freemint-protection-model.md:1-50 | [project] Speicherschutz nach FreeMiNT PROT_*, kleine Hälfte jetzt | decision | Vollständig in ADR-023 (docs/MEMORY.md:1627ff, gleiches Datum und Zitat, mem.h:120-136) und ADR-027 (große Hälfte). mem.h:120-134 OK. „Für Oktober weder nötig“ veraltet. docs/MEMORY.md-Link auf ADR-023 umbiegen | delete |
| AM-003 | aes-dialog-3d-look.md:1-24 | [feedback] AES-Objekte im EmuTOS-3D-Look | decision | aes-object-form-render-audit.md vorhanden; STATUS Phase 13 „3D-(Falcon-)Design“ umgesetzt; D1–D9-Bezug ist Geschichte, Regel gilt | keep |
| AM-004 | aes-object-abi-is-tree-abi.md:1-33 | [project] Kernel-gebaute Bäume immer `ObjTree::lp64` | trap | Regel im Code samt Doku: object.rs:106 `new`, :126 `lp64`. Prämisse veraltet: keine laufende ILP32-App mehr, Loader lehnt ELF32 ab (loader.rs:409-426); Desktop ist EL0-Prozess (ADR-026), nicht `current()==null`. Auf „Substrat für m68k-Thunk“ umschreiben | fix |
| AM-005 | altitude-syscall-not-hardware-emulation.md:1-40 | [feedback] Syscall-Kompatibilität, dünne Adapter statt ISR-Nachbau | decision | CLAUDE.md:8/13/14/19 stimmen; Kern = CLAUDE.md Goals/Non-goals, Memory ergänzt Anwendungsregel. `5d4805e` existiert nirgends (Repo, commit-map, EmuTOS); Maus-Commit ist `1983f4a5` (2026-06-19, `feed_relative` mouse.rs:189) | fix |
| AM-006 | atari-known-programs-preferred.md:1-12 | [feedback] Portvorschläge aus der Atari-Szene, Quelle vorher prüfen | decision | aniplayer-feasibility-2026-10-09.md vorhanden; kein Widerspruch | keep |
| AM-007 | atari-meeting-demo-october.md:1-52 | [project] Treffen 9.–10.10.2026, Konformität bleibt Maßstab, Vorführkarte | obsolete | Termin vorbei; Tag `ofam-2026` und measurement-2026-10-08-present-res/ vorhanden. Index Z.43 sagt das Gegenteil. Bleibendes (Termin nie Kriterium; Reihenfolge Speicherfehler/Regel 2 → belegte Defekte → Konformität → Kosmetik) vorher nach finish-the-task-list-no-detours | archive |
| AM-008 | auto-folder-console-only.md:1-45 | [project] \AUTO nur .NCP vor AES; make-sd-image = QEMU-Abbild | decision | launch.rs:369-397 (`C:\AUTO\*.NCP`) OK; Kern auch in ADR-018 (AUTO-Absatz). Veraltet: QEMU-Hook in `kernel/src/desk/main.rs` existiert nicht mehr, QEMU-Start = #Z. make-sd-image.sh:513-519 staged weiter nur DUMPVDI + vdi-r2 (Präzisierung stimmt). `1a2efeb`→`49506a2e` | fix |
| AM-009 | auto-folder-park-by-renaming.md:1-22 | [reference] .NCP→.NC parkt AUTO-Programme | trap | TASK/FS03.md „ZURUECKGEZOGEN 2026-09-24“ belegt; Dateinamen = Karteninhalt (Rauschen) | keep |
| AM-010 | ball-demo-as-benchmark.md:1-127 | [project] Ball-Demo als Grafik-Benchmark, Messmethodik | decision | boot.rs:168 MAIR Slot 2 Normal-NC OK, screen.rs:220 KNOWN DEFECT OK, `18d1bde`→`35110fad`. Vorhersagen ausdrücklich als solche markiert. Inhalt nur hier | keep |
| AM-011 | base-emutos-clean-on-pi-before-extension.md:1-50 | [project] erst EmuTOS-Basis sauber auf Pi, dann Weiterentwicklung | decision | Stand 2026-10-17 stimmt (ADR-026, SYSACC); `base-1` Entwurf inventory-2026-10-08.md:166, STATUS.md:17 „Definition offen“; 00-MASTER.md vorhanden | keep |
| AM-012 | block-devices-emmc-qemu.md:1-21 | [reference] QEMU raspi4b emuliert EMMC, SD-Treiber in QEMU debugbar | fact | emmc.rs:1-24 (EMMC2 `0xFE34_0000`, QEMU raspi4b) bestätigt; Planteil Phase 08 erledigt (STATUS Phase 08) | keep |
| AM-013 | brief-agents-with-inputs-not-conclusions.md:1-52 | [feedback] Worker mit Eingaben briefen, nicht mit Ursachen | decision | bdosmain.c:221 = `{ NI, 0, 0 }, /* 0x20 */` OK; kein Widerspruch | keep |
| AM-014 | card-write-only-what-user-says.md:1-40 | [feedback] echte Karte nur auf Ansage | decision | konsistent mit auto-folder-console-only (Präzisierung 2026-09-30); Dateinamen Rauschen | keep |
| AM-015 | cargo-xtask-board-target-footgun.md:1-50 | [reference] beide Boards bauen nach demselben Kernel-ELF | trap | tools/xtask/src/main.rs:552-559 `kernel_elf_path` ohne Board → Falle besteht; docs/MEMORY.md verlinkt | keep |
| AM-016 | cflib-apps-expect-tos4-semantics.md:1-36 | [project] cflib erwartet TOS4-Fattrib auf Ordnern | trap | 43-fattrib.md:11,52-55 und COMPATIBILITY.md:1095 vorhanden; `8af8c21`→`6277ddc7`. Fall dort dupliziert, Regel nur hier | keep |
| AM-017 | check-conf-gating-emutos-extraction.md:1-29 | [feedback] `#if CONF_WITH_*` vor Übernahme prüfen | decision | config.h:2036-2042 (`CONF_WITH_EJECT 0`) OK, deskmain.md:1141 schließt EJCTITEM aus. Frontmatter-description abgeschnitten („…check“) | fix |
| AM-018 | delegate-source-sweeps-to-auditor.md:1-29 | [feedback] Quellen-Sweeps an den Auditor | decision | konsistent mit limit-concurrent-workers; .claude/agents/auditor.md vorhanden | keep |
| AM-019 | desktop-1to1-emutos-rebuild.md:1-38 | [project] ADR-022: Desktop 1:1 durch den AES-Trap | decision | Duplikat ADR-022 (docs/MEMORY.md:1520ff); EL0-Endgame durch ADR-026 erfüllt; Pfad `kernel/src/desk/` weg; Tasks #123/#124 Geschichte. Inbound-Links aus 4 Memories auf ADR-022 umbiegen | delete |
| AM-020 | desktop-bypasses-app-abi-path.md:1-57 | [project] EL1-Desktop zeichnet am AES vorbei, validiert App-Pfad nicht | obsolete | Desktop ist EL0-Programm ohne `tos_api`-Zugriff (programs/desktop/src/lib.rs:9-19, ADR-026); `desk/redraw.rs:89` u.a. weg, render_tree jetzt object.rs:1163; ILP32-Satz überholt. Links in docs/MEMORY.md und STATUS.md:818 | archive |
| AM-021 | desktop-interactive-verification-real-hw.md:1-36 | [project] Pi als Gate, Scratch-Injektor für QEMU | fact | h03/method/ vorhanden; raspi4b ohne Eingabe, qemu-virt ohne Blockgerät (drivers/qemu_virt nur pl011/ramfb) weiter wahr; _hw-test-checklist.md vorhanden | keep |
| AM-022 | desktop-must-run-in-user-scope.md:1-46 | [project] Desktop muss in EL0 laufen | obsolete | erfüllt durch ADR-026 / DESK43 U5–U10 (TASK/DESK43-*-STATUS-DONE.MD, STATUS.md:8-20); `desk/mod.rs:159` weg; gemshlib.c:622-634 OK | archive |
| AM-023 | dont-reverse-on-one-indirect-number.md:1-15 | [feedback] nicht auf eine alte indirekte Zahl hin zurückdrehen | decision | `59db9be`→`635f1127` (manuell, commit-map); kein Widerspruch | keep |
| AM-024 | el0-app-memory-must-be-mapped.md:1-33 | [project] Malloc-Blöcke in EL0-Fenster mappen, User-VA zurück | obsolete | Mechanismus durch ARCH03 ersetzt: VA = PA, Malloc gibt `m_start` (usermem.rs:1-12, „ADR-021 window is gone“). Anforderung lebt in ADR-023. docs/MEMORY.md verlinkt den Slug | archive |
| AM-025 | email-client-whomper-link.md:1-21 | [project] whomper/EMail als Mail-Kandidat, nach Netzwerk | decision | syscalls.master:477-495 (0x160–0x16E) OK; konsistent mit version-roadmap (Netz 1.2) | keep |
| AM-026 | emutos-oracle-pinned-1a7484ab.md:1-34 | [project] Orakel 1a7484ab, tools/oracle, neue Refs EMUTOS-1a7484ab-* | fact | tools/oracle/ (4 Dateien), 28 EMUTOS-1a7484ab-*-Dateien, ORACLE.md §1-4 (acb7f3b4, ap_3resv, SIGKILL, --saveconfig) stimmen. Weitgehend Duplikat ORACLE.md, „LOG zweimal“ nur hier | keep |
| AM-027 | emutos-target-profile-falcon-tos4.md:1-69 | [project] Referenzprofil make-512 als Falcon030/TOS4 | decision | window.rs:61-86 (`gadget_ink`) OK, _emutos-config.md/emutos-config.sh OK. Veraltet: GEMDOS_VERSION „vor dem Setzen prüfen“, gesetzt `0x3000` mit Beleg (gemdos/version.rs:8-14); OS-Header `0x0404` gelandet (B1, STATUS Phase 08) | fix |
| AM-028 | establish-effect-before-fixing.md:1-31 | [feedback] erst Wirkung, dann Fix | decision | kein Widerspruch; verwandt mit noise-filters | keep |
| AM-029 | explain-impact-before-asking-choice.md:1-15 | [feedback] Auswirkungen je Option vor einer Frage | decision | kein Widerspruch | keep |
| AM-030 | fat12-floppy-out-of-scope.md:1-28 | [project] kein FAT12-Schreiben, keine Floppy | decision | fat.rs:256-258 `is_writable = fat16` bestätigt; konsistent mit version-roadmap (1.0 = FAT16) | keep |
| AM-031 | filesystem-target-exfat.md:1-36 | [project] FS-Fahrplan FAT16 → FAT32 → exFAT | decision | Updates 2026-10-19/23 konsistent mit version-roadmap; Kopfabsatz („FAT12/16/32 near-term, Phase 09 entscheiden“) veraltet; Slug und Index Z.1 sagen noch „exFAT-Ziel“ | fix |
| AM-032 | finish-the-task-list-no-detours.md:1-26 | [feedback] TASK/ der Reihe nach, keine Umwege | decision | kein Widerspruch; relativiert fix-vdi-before-aes mit | keep |
| AM-033 | fix-vdi-before-aes-bottom-up.md:1-45 | [feedback] VDI vor AES, alle Findings fixen | decision | 00-GATE.md vorhanden; Statusblock 2026-07-23 ist Geschichte (`796faf1`→`cdfe473f`, `966c46c`→`5ecb0133`). „Alle Findings, nicht App-triagiert“ durch real-apps-are-the-arbiter (2026-09-06) und finish-the-task-list relativiert: Verhältnis klarstellen, Status kürzen | fix |
| AM-034 | flash-release-builds-to-real-hw.md:1-26 | [feedback] Release-Bauten auf die Pi | decision | xtask main.rs:4,243-257 Default dev OK; teilweise Duplikat CLAUDE.md (Source Tree, `--release`) | keep |
| AM-035 | framebuffer-outside-st-tt-ram.md:1-31 | [feedback] Framebuffer nie im ST/TT-Pool | trap | mailbox.rs:254-276 (Tags 0x0001_0005/6), mailbox-framebuffer.md:313 §3a, `c063980`→`f60094e3` OK | keep |
| AM-036 | freemint-extension-direction.md:1-34 | [project] FreeMiNT-kompatible Erweiterungen, FreeMiNT sanktioniert | decision | references/freemint @ `f906416` vorhanden (`xdd/` liegt unter sys/xdd). How-to erledigt: SOURCE-ANALYSIS-PROTOCOL.md:51-54, references/README.md:86ff. Index Z.9 „not yet present“ widerspricht | fix |
| AM-037 | github-issues-in-english.md:1-15 | [feedback] neue GitHub-Issues englisch | decision | kein Widerspruch | keep |
| AM-038 | hatari-falcon-tc-is-the-colour-oracle.md:1-32 | [project] Falcon-TC für Farbe, TT für Geometrie | decision | run-falcon-tc.sh und EMUTOS14-VDI-all-falcontc.BIN vorhanden, aber Lauf heute über tools/hatari-oracle.sh falcontc, Referenz EMUTOS-1a7484ab-VDI-all-falcontc (ORACLE.md §2); emutos-oracle-pinned erklärt ROM/Pfad für überholt | fix |
| AM-039 | hatari-falcon-window-frame-reference.md:1-38 | [project] Hatari-Falcon als Fensterrahmen-Referenz, Rahmen weiß | fact | a79/method/ vorhanden. „PINNED build references/emutos/etos512us.img“ falsch: Datei 2026-08-29 im Referenzbaum gebaut; gepinnt ist tools/oracle/etos512de-1a7484ab.img (ORACLE.md §1, §3 „never built in“) | fix |
| AM-040 | hatari-produces-tt-references.md:1-58 | [project] Hatari reproduziert echte TT-Dumps bitgleich | fact | TT-Validierung nur hier; ap_3resv-Korrektur = ORACLE.md:127-134, WF_OWNER = ORACLE.md:144-145. Aufrufzeile mit ~/Downloads und Release-ROM überholt durch tools/hatari-oracle.sh; deploy-tt/ vorhanden | fix |
| AM-041 | hyp-files-beside-the-app.md:1-15 | [feedback] .HYP in den Programmordner | decision | kein Widerspruch | keep |
| AM-042 | injector-does-not-run-during-cconin.md:1-41 | [reference] Injektor hängt an pump_input, nicht in Cconin | trap | Sachverhalt gilt: read_key_loop (host_env.rs:567-605) eigene Schleife ohne pump_input. Zitate verschoben: pump_input :91, console_read_key_blocking :195, `console.rs:164` jetzt serial_rx_byte :238/:267 | fix |
| AM-043 | injector-park-must-not-cross-menu-bar.md:1-26 | [project] Park-Pfad nie durch die Menüleiste | trap | `27e4328`→`0ec60038` OK; kein Widerspruch | keep |
| AM-044 | justification-is-copied-to-several-places.md:1-32 | [feedback] beim Widerlegen alle Kopien suchen | trap | `d0faaa5`→`517cf4cb` OK | keep |
| AM-045 | kernel-memory-follows-freemint.md:1-20 | [project] Kernel-Speicher nach FreeMiNT (ADR-027) | decision | vollständig in ADR-027 (docs/MEMORY.md:2357-2427: Scope Kernel, AES/VDI und PROC03/04 EmuTOS, COMPATIBILITY-Pflicht, MEM06–10) | delete |
| AM-046 | keyboard-via-atari-ikbd.md:1-50 | [project] Tastatur per ST-IKBD an UART2 | obsolete | Entscheid umgekehrt: `1ace07a4` (2026-06-25) entfernte ikbd_uart; ikbd.rs:8-12 „UART2 shelved“, Quelle USB-HID (ADR-015). UART2-Fakten in docs/hardware/ikbd-uart.md. docs/MEMORY.md verlinkt Slug | archive |
| AM-047 | limit-concurrent-workers-token-budget.md:1-56 | [feedback] ein Task je Agent, seriell | decision | konsistent mit delegate-source-sweeps; Index-Hook verkürzt (s. AM-001) | keep |
| AM-048 | lp64-tos-struct-trap.md:1-86 | [reference] mintlib-Structs 8 statt 4 Byte; AESGLOBAL bleibt schmal | trap | Status „latent, beißt bei DTA“ überholt: packed `_DTA` (sdk/libc/overlay/include/mint/ostruct.h:74-83), basepage.h korrigiert, A35 ERLEDIGT. 32-Bit-Beleg über `USER_VA_BASE=0x0`/4-MiB-TPA seit ARCH03 (VA = PA) falsch begründet, über Poolgrenze < 1 GiB neu belegen. `mt_gem.h:2502` ohne Pfad | fix |
| AM-049 | measure-layer-by-layer-isolated.md:1-15 | [feedback] Performance schichtweise messen | decision | kein Widerspruch | keep |
| AM-050 | mouse-moves-but-nothing-responds-means-app-sleeping.md:1-24 | [project] Maus lebt, UI tot = App parkt im AES-Aufruf | trap | host_env.rs:91 pump_input → usb::poll (:494, :512) OK; H03-Ursache (evnt_timer-Overlay) in STATUS 2026-09-17 belegt | keep |
| AM-051 | native-64bit-abi-future.md:1-55 | [project] App-ABI LP64, Thunk nur für m68k | decision | Kern = ADR-020 Status + Addendum (docs/MEMORY.md:1259-1261, 1314-1366) und STATUS.md:818. Veraltet: `sdk/apps/qed/overlay/memory.c` → sdk/overlays/qed/ (memory.c, set.c); „lp64-Audit gegenstandslos“ falsch (lp64-tos-struct-trap, T01). ADR-020 verlinkt Slug | fix |
| AM-052 | newtosworld-project-site.md:1-22 | [reference] newtosworld.de = Projektseite | fact | nicht nachprüfbar (außen); kein Widerspruch | keep |
| AM-053 | no-commit-all-while-worker-runs.md:1-15 | [feedback] kein `commit -a` während ein Worker läuft | trap | `685cd68`→`db86fd49`, `a7006f3`→`7d19c055` (manuell, commit-map) | keep |
| AM-054 | no-rationalizing-emutos-deviations.md:1-78 | [feedback] Abweichung angleichen, nicht wegerklären; Grenzen des 1:1 | decision | gemsuper.c:95-96 (`ap_private`) und :432 (`int_out`) OK; Abschnitt 2026-10-22 deckt sich mit COMPATIBILITY-Pflicht | keep |
| AM-055 | noise-filters-hide-real-defects.md:1-46 | [feedback] jede gefilterte Zelle braucht Quellenbeleg | trap | vdi_mouse.c:582, vdi_defs.h:94, vdi_control.c:203 OK; V17 behoben (STATUS 2026-09-14) | keep |
| AM-056 | own-extensions-separate-aarch64-from-m68k.md:1-54 | [project] .NAP/.NCP/.NTP/.NAC trennen nativ von m68k | decision | vollständig in ADR-018 + Amendment 2026-09-24 und accs.rs:29-36 (`C:\ACC\*.NAC`, dann `C:\*.NAC`); „ACC/ wäre falsch, offen“ erledigt; ftype.rs jetzt programs/desktop/src/ftype.rs; Frontmatter `node_type: project` | delete |
| AM-057 | pistorm-platform-facts.md:1-47 | [project] PiStorm-Fakten, nur Ball-Demo | obsolete | Anwender 2026-10-08: hat mit NewTOS nichts zu tun → gehört zum Ball-Demo außerhalb des Repos. Commits OLD-HASH-MAPPED; `vro_cpyfm`-Vermutung als unbelegt markiert | archive |
| AM-058 | pl011-rx-irq-not-delivered-on-real-hw.md:1-46 | [project] RX-IRQ wird zugestellt; seriell nie in IKBD-Schlange auf rpi4 | trap | deckt sich mit QUIRK-002-Korrektur (docs/MEMORY.md:2588-2626), console.rs:244-270, pl011.rs:124-159 (IMSC, Init-Flush). `c6e4bfb`→`b776ffaa`, `6a573f9`→`44361630`. Slug sagt Gegenteil des Inhalts → umbenennen; Test-Falle (Bootschirm nur per USB überspringbar) nur hier | fix |
| AM-059 | port-reuse-dont-rewrite.md:1-30 | [feedback] Quellen wiederverwenden, nur Rand anpassen | decision | Duplikat ADR-019 (docs/MEMORY.md:1176ff); ergänzt Anwenderzitat und Vorgehen | keep |
| AM-060 | port-target-app-unmodified.md:1-54 | [feedback] Upstream unverändert, Patches in sdk/overlays/<prog>/ | decision | Korrektur 2026-10-20 = ADR-019-Amendment (docs/MEMORY.md:1231-1239). Z.10-35 („App-Quelle sakrosankt, alles in die Toolchain“) widerspricht der Korrektur im selben File → auf geltende Regel kürzen; Doom-Scope (2026-10-04) nur hier | fix |
| AM-061 | post-2a-roadmap-bios-freemint.md:1-75 | [project] Fahrplan nach 2a: BIOS → ST/TT → koop. Kern → SMP | obsolete | BIOS-Ebene, Upcall (sendsig in sched.rs), koop. Kern, Accessories erledigt. Widerspricht sich (Z.27 FreeMiNT da, Z.67 „NOCH NICHT“); „ST-RAM fest 8 MB“ gegen layout.rs:24 (16-MiB-Slot); Zweig `desk-rebuild` aufgelöst | archive |
| AM-062 | push-after-worker-report.md:1-12 | [feedback] Lead pusht selbst, nie während ein Worker committen kann | decision | `cd60428b`, `be506300` im Repo | keep |
| AM-063 | qemu-dump-runs-via-newdesk-inf-autorun.md:1-36 | [project] Dumperläufe headless in QEMU per #Z | fact | Verfahren gilt; `init.rs:355` → process_inf2 init.rs:437; `sh_main` nicht mehr in kernel/src/lib.rs:407. Rezept ohne `-append serial=on` (raspi4b-runs-need-serial-on) | fix |
| AM-064 | qemu-images-clone-not-copy.md:1-19 | [feedback] ein Basisabbild, Klon je Lauf | decision | kein Widerspruch | keep |
| AM-065 | qemu-window-view-command.md:1-34 | [reference] `cargo xtask run --board rpi4 --window` | fact | xtask main.rs:178-179, 694-701 (`--window`, `--sd`, Default /tmp/sd.img) OK. „Kein IKBD/UART2-Mauspfad“ veraltet: Eingabe ist USB-HID, raspi4b hat kein USB | fix |
| AM-066 | raspi4b-runs-need-serial-on.md:1-15 | [project] seit SER01 `serial=on` nötig | trap | xtask main.rs:829-832, tools/qemu-serial-on/, TASK/SER01-STATUS-DONE.MD OK; `2e3ca46`→`91be77f9` (manuell) | keep |
| AM-067 | real-apps-are-the-arbiter.md:1-35 | [ohne type] Konformitätsfragen ohne App-Wirkung parken | decision | Frontmatter ohne `metadata.type`. Anlass OVERLAY_BIT inzwischen behoben (V11/V12, STATUS 2026-09-21); Begründung „vor dem Atari-Treffen“ veraltet; Verhältnis zu fix-vdi-before-aes benennen | fix |
| AM-068 | reference-dumper-as-benchmark.md:1-50 | [project] Dumper als Benchmark VERWORFEN | obsolete | verworfen 2026-09-25; Messüberlegungen auch in ball-demo-as-benchmark:36-42,77-92; `04aaf16`→`818c6cf3` | archive |
| AM-069 | root-cause-before-fixing.md:1-28 | [feedback] Ursache belegen, dann Fix | decision | kein Widerspruch; konsistent mit CLAUDE.md Regel 3/5 | keep |
| AM-070 | rscview-renders-the-3d-reference.md:1-38 | [reference] rscview = 3D-Soll für Dialoge | fact | fsel-reference-2026-09-17/, tools/hatari-qed.sh, gemfslib.c:66-139 OK; A63/A64 erledigt; `[[TASK/A63]]`/`[[TASK/A64]]` Ticketpfade | keep |
| AM-071 | rtc-ds3231-plan.md:1-8 | [project] DS3231 als BIOS-Uhrtreiber, TASK/RTC01 | decision | TASK/RTC01.md (OFFEN) deckt Entscheid, CR2032/Ladepfad, Datenblattpflicht; Datenblatt fehlt weiter. Pinbelegung nur hier | keep |
| AM-072 | sd-card-real-hw-layout.md:1-48 | [project] Pi-Karte 4×500 MB, FAT16-vs-FAT32 offen | fact | ohne Karte nicht prüfbar; offener Widerspruch korrekt als offen geführt | keep |
| AM-073 | settings-ui-human-readable.md:1-15 | [feedback] Einstellungen als lesbare Schalter | decision | kein Widerspruch | keep |
| AM-074 | shell-grep-is-ugrep-wrapper.md:1-18 | [project] Shell-grep ist ugrep-Wrapper, /usr/bin/grep nutzen | trap | weder in CLAUDE.md noch .claude/agents/; nur hier | keep |
| AM-075 | singletasking-app-owns-full-screen.md:1-32 | [project] gestartete App übernimmt den ganzen Schirm | decision | Modell gilt. `kernel/src/desk` → programs/desktop (EL0); „ADR-021-Fenster als Vorstufe“ überholt (ARCH03, usermem.rs:9-12) | fix |
| AM-076 | start-teradesk-via-real-emutos-mechanism.md:1-23 | [project] TeraDesk über shel_write/#Z starten | obsolete | erledigt: shel_write tos-api/src/aes/shell.rs:483, #Z process_inf2 init.rs:437, Shell-Loop `49c3938`→`47b29a96`; „desk/ zeichnet in-kernel“ überholt (ADR-026) | archive |
| AM-077 | status-md-is-not-evidence.md:1-35 | [feedback] STATUS.md ist Prüfling, nicht Beleg | decision | QUIRK-002-Beispiel deckt docs/MEMORY.md:2588ff | keep |
| AM-078 | subagent-scratch-in-job-tmp.md:1-31 | [feedback] Scratch in $CLAUDE_JOB_DIR/tmp, kein Wildcard-rm | trap | Kern Duplikat CLAUDE.md (Placement rules, Scratch); Wildcard-rm-Verbot nur hier | keep |
| AM-079 | teradesk-is-the-desktop.md:1-91 | [project] EmuDESK Kern, TeraDesk obendrauf, #Z einmalig | decision | gemshlib.c:662-666, :172, geminit.c:769, deskapp.c:832, desksupp.c:56 OK. Veraltet: `kernel/src/desk/` (5×); „ADR-016 gilt“, ADR-016 durch ADR-022/026 abgelöst; „S5–S8 offen“, DESK43 U5–U10 erledigt; Index Z.45 übernimmt das | fix |
| AM-080 | tickets-from-screendumps-misfire.md:1-35 | [feedback] Verhaltenstickets erst nach Quelle | trap | deskmain.c:1282, :957-959, gemoblib.c:889-900 OK; deskmain.c:1034 = Switch-Ende, `menu_tnormal` :1036, trägt | keep |
| AM-081 | tt-card-tests-go-to-tools.md:1-13 | [reference] TT-Karte: Tests nach TOOLS | fact | ohne Karte nicht prüfbar; konsistent mit card-write-only-what-user-says | keep |
| AM-082 | unwired-selftests-rot-silently.md:1-29 | [project] selftest_*.rs in keinem Bootpfad | trap | kernel/src/lib.rs:461-470 bestätigt; 28 Selftest-Dateien, nur `selftest_emmc::run` (lib.rs:413) verdrahtet; `d8b7f66`→`d8471c90`; inventory §3 (6) führt dieselbe offene Entscheidung | keep |
| AM-083 | update-status-md-after-each-extension.md:1-43 | [feedback] STATUS.md nach jeder Erweiterung | decision | Duplikat CLAUDE.md:264-270. Body „newest-first, oben“ stimmt mit STATUS.md:466ff; CLAUDE.md:267 und Index Z.8 sagen „append“ → Wortlaut angleichen. „Current Phase sagt Phase 02“ veraltet (STATUS.md:6-20); „single source of truth“ gegen status-md-is-not-evidence | fix |
| AM-084 | vdi-draw-asm-optimization-later.md:1-26 | [project] VDI-Zeichnen später in Assembler, Trait-Grenze halten | decision | tos-api/src/vdi/backend.rs, kernel/src/vdi_backend.rs vorhanden; docs/MEMORY.md verlinkt | keep |
| AM-085 | vdi-truecolor-workstation.md:1-35 | [project] VDI auf Falcon-Truecolor, erst planen | obsolete | umgesetzt, ADR-024 (docs/MEMORY.md:1933ff, „implemented and shipped“, `09c9ddc`/`11ca57e`); Plan-Dok vorhanden. docs/MEMORY.md verlinkt Slug | archive |
| AM-086 | verify-a-prediction-before-asking-for-it.md:1-29 | [feedback] Vorhersagen erst gegen die Quelle prüfen | decision | gemwmlib.c:1025 OK; `set.c` = sdk/overlays/qed/set.c | keep |
| AM-087 | verify-actual-call-path-before-diagnosing.md:1-33 | [feedback] LR und Erreichbarkeit vor Diagnose | decision | kein Widerspruch | keep |
| AM-088 | verify-visual-output-before-claiming.md:1-34 | [feedback] Pixel genau ansehen vor Urteil | decision | Body nennt drei Fälle, Index Z.14 „twice“ (s. AM-001) | keep |
| AM-089 | version-roadmap-1-0-2-0.md:1-16 | [project] 1.0 = EmuTOS-Parität, FAT16 + Sound; Netz 1.2; 2.0 FreeMiNT | decision | NET01/SND01/SND02 vorhanden, Status konsistent. Veraltet: „NewTOS 0.1“, jetzt 0.6.1 (initinfo.rs:97, shell.rs:123, programs/desktop/src/inf.rs:2579); „Released 0.6 alpha“, Tags v0.6, v0.6.1 | fix |
| AM-090 | workers-no-hardware-or-outward-actions.md:1-26 | [feedback] Worker: kein Flashen/Auswerfen/Veröffentlichen | decision | kein Widerspruch | keep |
| AM-091 | worktree-isolation-orphan-branch.md:1-36 | [reference] Worktrees starteten auf Waisen-Branch | trap | `main` = Arbeitsstand, `archive/restart-2026-07` vorhanden, `dfed003` OK; Bestätigung durch neuen Worktree-Lauf steht laut Body aus | keep |
