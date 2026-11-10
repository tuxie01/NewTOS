# Ticket inventory — state of every file in TASK/ (2026-11-10)

Dated snapshot (read-only inventory, no decisions, no recommendations). Historical, not authoritative after its date.

## 1. Header and method

- Date: 2026-11-10. HEAD: `ad58e69e0` (branch main, working tree clean at start).
- Scope: every entry of `TASK/` (1088 directory entries): 663 `*.md` files (657 with a `Status:` line, 6 without), 423 `*.MD` files (420 `*-STATUS-*.MD` receipts, 3 other `.MD` files), the directory `TASK/evidence` (12 entries) and `TASK/tasks.json`. One untracked macOS artefact (`.DS_Store`) exists in the directory and is not a row.
- Rows: 687 = 663 ticket/`.md` rows (657 with status line + 6 non-ticket `.md`), 19 orphan-receipt ids (receipt id with no `.md` file and no ancestor ticket), 5 non-`.md` artefacts (3 `.MD` diagnoses, `evidence`, `tasks.json`). Receipts are not separate rows; they are columns of their ticket row.
- Status line: first line matching `^Status:` within the first 40 lines; its number is given as `@L<n>` (always 3 for the 657 that have one). Priority is taken from the `Prioritaet`/`Priorität`/`Priority` field of that line. Cells are truncated to ~120 characters with `…`; `**` bold markers are removed; `|` is replaced by `/`.
- Receipt mapping: id = text before `-STATUS-`. Exact ticket id = direct receipt; else strip trailing `-segment`s until a ticket exists (sub-step, e.g. `ARCH03-3b` -> `ARCH03`, `DESK43-U5a` -> `DESK43`); a range id `R02-R05` maps to both `R02` and `R05` (flagged C6); no ancestor = orphan row. Receipts column: `DONE` = direct receipt `TASK/<id>-STATUS-DONE.MD`; `<sub>:<STATE>` = sub-step receipt `TASK/<sub>-STATUS-<STATE>.MD`.
- Commits: `git log` of the current branch only (%d commits), regex `(^|[^A-Za-z0-9-])<ID>([^A-Za-z0-9-]|$)` over subject+body (evaluated in Python with the same pattern, multiline). Verified against the git command `git log -E --grep=<regex>` on the current branch: ticket ids A10=0, V55=14, FS20=8, A25=8 agree with the script; the non-ticket substrings show the regex separates prefixes (git: A1=1 vs A10=0; V5=2 vs V55=14; FS2=0 vs FS20=8; FS20-B=0). Consequence of the hyphen exclusion: `A25-STATUS` or `DESK43-U5a` in a message does not count for `A25`/`DESK43`. Hash/date = newest matching commit in log order (author date). "fix-n" = commits among them whose subject starts with `fix`/`feat` (optionally with scope/`!`) and names the id (case-insensitive prefix).
- STATUS.md mentions: every line of `docs/STATUS.md` (296 lines) matching `(?<![A-Za-z0-9-])<ID>(?![A-Za-z0-9-])` (covers links `TASK/<ID>.md` and bare ids); additionally, for ids of form `LETTERS+digits`, an id strictly inside a same-prefix range `X01–X09` counts as an implicit mention (`range`). Notation `L<line>:<section>:<claim>`; sections: OA = "Offen beim Anwender" (2026-11-08 and 2026-11-07 items), WQ = Warteschlange (sub-list when identifiable: Geparkt = "Geparkt 2026-10-31 (Anwender)", EbGeparkt = "Ebenfalls geparkt", Zurueckgest, geparktMIDI, DevReihe, VorStart; `/?` = sub-marker not mapped), P1, P1u = "P1, umgesetzt, nur Pi- bzw. Anwendungsabnahme offen", P2, P34 = "P3 / P4", ERLoQ = "ERLEDIGT ohne DONE-Quittung" list, U10 = Umfang 1.0, BLK = Blocked, RD = Recent Decisions, PH = Phase table, CS = Current state prose. All quotes are in Appendix B. **Line 121 is the queue line; its per-id statement is the quote in Appendix B.**
- Claim (heuristic, keyword based, window = up to 110 characters after the mention, cut at the next ticket id): `done` (erledigt/umgesetzt/bestanden/veroeffentlicht/abgenommen/PASS), `parked` (geparkt/zurueckgestellt/ruht, or the mention stands in a "Geparkt"/"Zurueckgestellt" sub-list), `open` (mention in an OA/P1/P2/P34 list without a done/parked word), `done-no-receipt` (ERLoQ list), `impl-open` (P1u list), `blocked` (BLK), `none`. This extraction is mechanical; the rule outcomes of C3 are candidates to be read with their quote, not verdicts.
- Owner decision: last (by date, ties by later line) occurrence in the ticket text of `Anwender…<date>` or `<date>…Anwender` (regex `Anwender\w*[^\n]{0,60}?(20\d\d-\d\d-\d\d)|(20\d\d-\d\d-\d\d)[^\n]{0,25}Anwender`); the line is quoted (≤110 characters). Mechanical: it can quote a non-decision sentence and can miss one phrased differently.
- Parked class = leading status word GEPARKT, ZURUECKGESTELLT/ZURÜCKGESTELLT, BACKLOG, ZUKUNFT (IDEE is listed in section 4 but not in the parked class). Reason "yes (status line)" = text beyond the word and a bare date on the status line before ` · `; "body only" = no such text, but a line in lines 4–60 of the ticket matches `geparkt|zurueckgestellt|Grund:|wartet auf|ruht` (quoted; not verified to be a reason); "no" = neither.

### Closed-state mapping (assumption)

`TASK/README.md` (lines 33–34) defines receipts (`DONE`, `FAILURE`, `UNFINISHED`) and says that closure is recorded in the task file ("Status, Commit/Artefakt, …, offene Hardwareabnahme"); it does **not** define which status words mean closed (README.md:38–102 table uses ERLEDIGT, OFFEN, IMPLEMENTIERT — HW OFFEN, ZURÜCKGESTELLT). The mapping used here is therefore an explicit assumption derived from the words in use:

- CLOSED: status line (before ` · `) starts with ERLEDIGT, BEHOBEN, UMGESETZT, GESCHLOSSEN, GEGENSTANDSLOS, ZURUECKGEZOGEN, ABGESCHLOSSEN (ALS BLOCKED), GEKLAERT, ACCEPTED(-Abweichung), URSACHE GEFUNDEN UND BEHOBEN, and carries no `offen`/`blockiert` after the word.
- CLOSED-OPENACC: same leading words, but the remainder contains `offen` or `blockiert` (e.g. "ERLEDIGT — Pi-Sichtabnahme offen", "BEHOBEN — HW-Gegenprobe offen"). Treated as closed for C1/C2 (the leading word is the ticket's own state); shown separately in section 4.
- IMPL-OPEN: leading IMPLEMENTIERT (README.md:59 "IMPLEMENTIERT — HW OFFEN"): not closed. PARTIAL: TEILWEISE, "TEIL 1 …", "Teil 1 …", "Stufe 1 …", "A–F, H, I, J ERLEDIGT", "BEFUND ERLEDIGT, ABNAHME OFFEN": not closed. OPEN: OFFEN…, URSACHE BELEGT, Fix offen. PARKED: GEPARKT, ZURUECKGESTELLT. BACKLOG, IDEE, ZUKUNFT: own states, not closed. UMGESETZT is mapped closed while IMPLEMENTIERT is not — this split is an assumption taken from the words' use (UMGESETZT tickets in this set state the tester acceptance; IMPLEMENTIERT tickets state a pending acceptance).

### Class rules (exact)

- **C1** done but not closed: ticket row, status class not CLOSED/CLOSED-OPENACC, and (a **direct** DONE receipt `<id>-STATUS-DONE.MD` exists OR at least one commit with subject starting `fix`/`feat` names the id). Sub-step DONE receipts alone do not trigger C1.
- **C2** closed but no evidence: status class CLOSED or CLOSED-OPENACC, no DONE receipt of any kind (direct or sub-step) AND zero commits matching the id regex. (Applied to GEGENSTANDSLOS/ZURUECKGEZOGEN too, as briefed.)
- **C3** STATUS.md vs ticket: any mention whose claim conflicts with the ticket class: `done` vs OPEN/PARKED/BACKLOG/IDEE/ZUKUNFT; `parked` vs OPEN/IMPL-OPEN/CLOSED; `open` vs CLOSED; `done-no-receipt` vs anything not CLOSED/CLOSED-OPENACC/PARTIAL; `impl-open` vs anything not IMPL-OPEN/CLOSED-OPENACC. Phase-table, current-state prose and `none` claims never trigger. Candidates by keyword heuristic.
- **C4** parked without reason: parked class and reason = "no" (rule above). "body only" counts as a reason present.
- **C5** status text not a recognizable state word: ticket `.md` whose line-3 status does not start with one of the words of the mapping, or has no `Status:` line; and every non-ticket artefact (README, COMPARISON, …, `evidence`, `tasks.json`, other `.MD`) as "no state model applies".
- **C6** duplicate/renamed/superseded ids: (a) orphan receipt (id without ticket file); (b) range receipt (`R02-R05`); (c) one of the words `superseded|Duplikat|umbenannt|ersetzt durch|zusammengef…|aufgegangen|Dublette|renamed|abgeloest durch` plus another id in lines 1–12 of the ticket. No ticket text mentions `FS20-B`; no FS20-B -> FS22 rename is visible in `TASK/` (git: FS20-B=0 commits).
- **OK**: none of the above. A row may carry several classes (all listed); sorting is by first class (C1..C6, OK), then id.

## 2. Summary counts

Total rows: 687.

| Class | rows with this class (any position) | rows whose first class it is |
|---|---|---|
| C1 | 51 | 51 |
| C2 | 23 | 23 |
| C3 | 23 | 14 |
| C4 | 81 | 81 |
| C5 | 11 | 9 |
| C6 | 23 | 23 |
| OK | 486 | 486 |

Of C4, 80 are the `G-*` BACKLOG tickets (status `BACKLOG` with no reason on the status line; only the README.md:104–106 section text describes them).
Row kinds: ticket-with-status-line 657, non-ticket `.md` 6, orphan receipts 19, other artefacts 5.

Status classes of the 657 status lines: BACKLOG 81, CLOSED 307, CLOSED-OPENACC 82, IDEE 4, IMPL-OPEN 8, OPEN 105, PARKED 60, PARTIAL 8, ZUKUNFT 2.

Check of the lead's hint list (facts from this run): 
A16: IMPL-OPEN / direct DONE yes / fix-n 1; A34: IMPL-OPEN / direct DONE yes / fix-n 1; A37: IMPL-OPEN / direct DONE yes / fix-n 2; A78: IMPL-OPEN / direct DONE yes / fix-n 2; ACC06: PARTIAL / direct DONE yes / fix-n 1; BIOS16: OPEN / direct DONE yes / fix-n 1; BIOS17: OPEN / direct DONE yes / fix-n 1; DESK34: IMPL-OPEN / direct DONE yes / fix-n 1; DESK35: IMPL-OPEN / direct DONE yes / fix-n 2; DESK42: IMPL-OPEN / direct DONE yes / fix-n 2; FS09: OPEN / direct DONE yes / fix-n 1; FS23: OPEN / direct DONE yes / fix-n 1; INPUT01: OPEN / direct DONE yes / fix-n 2; PORT01: CLOSED-OPENACC / direct DONE yes / fix-n 1; V17: IMPL-OPEN / direct DONE yes / fix-n 1; ZVIEW01: OPEN / direct DONE yes / fix-n 1; ZVIEW02: OPEN / direct DONE yes / fix-n 2; ZVIEW04: OPEN / direct DONE yes / fix-n 1; A132: OPEN / direct DONE no / fix-n 1; A133: OPEN / direct DONE no / fix-n 2.

## 3. Full table (sorted by first class, then id)

Columns: id | classes (C3 detail) | status line @line (verbatim, truncated) | prio | receipts | commits n / newest / fix-n | STATUS.md mentions | last owner decision | parked reason. Kind of non-ticket rows is given in the status cell.

| id | class | status line | prio | receipts | commits | STATUS.md | owner decision | parked reason |
|---|---|---|---|---|---|---|---|---|
| A127 | C1 | @L3 OFFEN · Prioritaet: P1 · Typ: Fehler, Ursache zuerst — Fremdmeldung auf echter Pi 4 (Anwender 2026-11-04) | P1 | - | 13 / 3e825c135 2026-11-07 / 1 | L121:WQ:none, L145:P1:open | 2026-11-05: Eigenes Issue fuer den Menuebefund: GitHub #19 (Anwender 2026-11-05; #3 war als USB-Thema schon | n/a |
| A132 | C1 C3 [C3: L121 WQ/done] | @L3 OFFEN · Prioritaet: P2 · Typ: Abweichung von EmuTOS — Nebenbefund der A129-Messung (2026-11-04) | P2 | - | 4 / 3e825c135 2026-11-07 / 1 | L121:WQ:done, L121:WQ:none | none found | n/a |
| A133 | C1 C3 [C3: L121 WQ/done] | @L3 OFFEN · Prioritaet: P2 · Typ: Abweichung von EmuTOS — Nebenbefund aus A132 (2026-11-04) | P2 | - | 3 / 4d44a6168 2026-11-04 / 2 | L121:WQ:done | none found | n/a |
| A16 | C1 | @L3 IMPLEMENTIERT — HW OFFEN · Priorität: P1 · Typ: Belegter Defekt / bedingte Reparatur | P1 | DONE | 2 / ce8e532ed 2026-09-14 / 1 | L121:WQ:none, L151:P1u:impl-open | none found | n/a |
| A34 | C1 | @L3 IMPLEMENTIERT — ABNAHME OFFEN (A33) · Prioritaet: P2 · Typ: Belegter Defekt / gemessen | P2 | DONE | 1 / 4cbd74855 2026-09-13 / 1 | - | none found | n/a |
| A37 | C1 | @L3 IMPLEMENTIERT (2026-09-12) — anwendungsseitige Abnahme offen · Prioritaet: P1 · Typ: Belegter Defekt / SPEICHERFEHL… | P1 | DONE | 3 / 237e35700 2026-09-17 / 2 | L121:WQ:none, L151:P1u:impl-open | none found | n/a |
| A78 | C1 | @L3 IMPLEMENTIERT (2026-09-20) — Tester-/Hardwareabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt / Konformitaet (… | P2 | DONE | 3 / 83a403cd1 2026-09-20 / 2 | - | none found | n/a |
| ACC06 | C1 | @L3 TEIL 1 ERLEDIGT (2026-09-24), TEIL 2 ALS QUELLTEXT · Prioritaet: P2 · Typ: Abnahmefall + Kernelbindung | P2 | DONE | 5 / 8171511dd 2026-10-26 / 1 | - | none found | n/a |
| BIOS16 | C1 C3 [C3: L113 WQ/EbGeparkt/done, L134 WQ/done] | @L3 OFFEN (entschieden 2026-10-30) · Prioritaet: P2 · Typ: Developer — Fund aus BIOS07<BIOS07> (2026-10-29) | P2 | DONE | 7 / 3ef688fd4 2026-10-30 / 1 | L44:CS:none, L113:WQ/EbGeparkt:done, L134:WQ:done | 2026-10-30: ## Nachentscheidungen (Anwender, 2026-10-30) | n/a |
| BIOS17 | C1 C3 [C3: L134 WQ/done] | @L3 OFFEN · Prioritaet: P3 · Typ: Developer — Anwenderentscheidung 2026-10-30, Nachlauf aus BIOS16<BIOS16> | P3 | DONE | 3 / f065a7050 2026-10-30 / 1 | L44:CS:none, L134:WQ:done | 2026-10-30: ## Anwenderentscheidung 2026-10-30 | n/a |
| COOK01 | C1 | @L3 ZURUECKGESTELLT (Anwender 2026-10-20: aktuell vermutlich ohne Wirkung; Varianten 0xFFFFFFFF / Familie+Kennung / 0xF… | P3 | - | 4 / 86f39d28e 2026-10-29 / 1 | L179:P34:open | 2026-10-20: ## Quelle tos.hyp (2026-10-20, vom Anwender genannt) | yes (status line): Anwender 2026-10-20: aktuell vermutlich ohne Wirkung; Varianten 0xFFFFFFFF / Familie+Kennu… |
| COOK02 | C1 C3 [C3: L132 WQ/DevReihe/done] | @L3 GEPARKT 2026-10-30 (Anwender; wieder aufnehmen, wenn ein Port es braucht) · Prioritaet: P3 · Typ: Developer — Anwen… | P3 | - | 4 / f8fb280e2 2026-10-30 / 1 | L44:CS:none, L132:WQ/DevReihe:done | 2026-10-30: ## Anwenderentscheidung 2026-10-30 (aus `docs/analysis/cook02-country-facts-2026-10-30.md`) | yes (status line): (Anwender; wieder aufnehmen, wenn ein Port es braucht) |
| COVERAGE | C1 C5 | none (non-ticket .md) | none | - | 2 / c173cddcf 2026-10-28 / 1 | - | none found | n/a |
| DESK34 | C1 | @L3 IMPLEMENTIERT — HW OFFEN · Prioritaet: P1 · Typ: Belegter Defekt — Pi-Runde 2026-09-24 | P1 | DONE | 5 / af0bc92d3 2026-09-24 / 1 | L121:WQ:none, L151:P1u:impl-open | none found | n/a |
| DESK35 | C1 | @L3 IMPLEMENTIERT — HW OFFEN · Prioritaet: P1 · Typ: Belegter Defekt — Pi-Runde 2026-09-24 | P1 | DONE | 5 / 7308e1c1f 2026-09-26 / 2 | L121:WQ:none, L151:P1u:impl-open | none found | n/a |
| DESK42 | C1 | @L3 IMPLEMENTIERT — HW-Abnahme offen (2026-09-25) · Prioritaet: P3 · Typ: Konformitaet — Pi-Runde 2026-09-25 | P3 | DONE | 5 / 4b3d73828 2026-09-30 / 2 | - | none found | n/a |
| DESK49 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Werkzeugkette — Nebenfund aus DESK41<DESK41> | P3 | - | 1 / 337f3553c 2026-09-29 / 1 | L176:P34:open | none found | n/a |
| DESK61 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Konformitaet — Nebenfund DESK59 (2026-10-16) | P3 | - | 1 / d890e322f 2026-10-16 / 1 | L176:P34:open | none found | n/a |
| DESK65 | C1 | @L3 OFFEN (Erweiterung) · Prioritaet: P3 · Typ: Weiterentwicklung ueber EmuTOS hinaus — Anwenderentscheidung 2026-10-17 | P3 | - | 2 / 0e95dcedc 2026-10-17 / 1 | L176:P34:open | 2026-10-17: Anwender 2026-10-17 (bei DESK64): *„ich empfand das sofortige speichern gar | n/a |
| FS09 | C1 | @L3 OFFEN · Prioritaet: P2 · Typ: FreeMiNT-Erweiterung ueber EmuTOS hinaus — Anwender 2026-10-07 („Freemint hat sowas",… | P2 | DONE | 3 / 29a0faa65 2026-10-07 / 1 | - | 2026-10-07: Status: OFFEN · Prioritaet: P2 · Typ: FreeMiNT-Erweiterung ueber EmuTOS hinaus — Anwender 2026-10-07 ( | n/a |
| FS12 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Konformitaet — Nebenfund DESK58 (2026-10-16) | P3 | - | 1 / 3b8098415 2026-10-16 / 1 | L121:WQ:none, L176:P34:open | none found | n/a |
| FS23 | C1 C3 [C3: L125 WQ/DevReihe/done] | @L3 OFFEN · Prioritaet: P2 · Typ: Developer — Anwenderentscheidung 2026-10-29, Fund F1 aus TOOL20<TOOL20> | P2 | DONE | 4 / 40e06519c 2026-10-29 / 1 | L44:CS:none, L125:WQ/DevReihe:done | 2026-10-29: Status: OFFEN · Prioritaet: P2 · Typ: Developer — Anwenderentscheidung 2026-10-29, Fund F1 aus [TOOL20 | n/a |
| FS32 | C1 C3 [C3: L121 WQ/parked] | @L3 OFFEN (2026-11-06) · Prioritaet: P3 · Typ: Werkzeug — Folge-Ticket nach CLAUDE.md "Testing" aus FS31<FS31> | P3 | - | 1 / 20f87d1d3 2026-11-06 / 1 | L121:WQ:parked | 2026-10-30: Vor-Start-Pruefung (Anwender 2026-10-30): Blockiert keine Abnahme eines Produkt-Fixes, solange die Sonde laeuf | n/a |
| HW05 | C1 | @L3 BEFUND ERLEDIGT, ABNAHME OFFEN — Pi-Lauf nach HW08<HW08> (2026-11-06; vorher zurueckgestellt, Anwender 2026-09-25) … | P2 | - | 6 / b1cfa8be4 2026-11-07 / 1 | L121:WQ:none, L160:P2:done | 2026-09-25: Status: BEFUND ERLEDIGT, ABNAHME OFFEN — Pi-Lauf nach HW08<HW08> (2026-11-06; vorher zurueckgestellt, | n/a |
| INPUT01 | C1 | @L3 OFFEN · Prioritaet: P2 · Typ: Abweichung, Ursache unbekannt — Anwender auf der echten Pi 4, 2026-10-07 | P2 | DONE | 7 / ae22f4fc4 2026-11-08 / 2 | - | 2026-10-07: Status: OFFEN · Prioritaet: P2 · Typ: Abweichung, Ursache unbekannt — Anwender auf der echten Pi 4, 20 | n/a |
| IO01 | C1 | @L3 IDEE (Anwender 2026-10-13, Fragen vom Treffen in Muenchberg) · Prioritaet: P3 · Typ: Hardware / BIOS-Zeichengeraete… | P3 | - | 3 / 04cdc0935 2026-10-30 / 1 | L136:WQ/geparktMIDI:parked, L180:P34:open | 2026-10-13: Status: IDEE (Anwender 2026-10-13, Fragen vom Treffen in Muenchberg) · Prioritaet: P3 · Typ: Hardware  | yes (status line): Anwender 2026-10-13, Fragen vom Treffen in Muenchberg) |
| MEM10 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Laufzeit — Anwenderentscheidung 2026-10-13 (MEM05 Teil A) | P3 | - | 2 / c4e445e46 2026-10-13 / 1 | L178:P34:open | 2026-10-13: Status: OFFEN · Prioritaet: P3 · Typ: Laufzeit — Anwenderentscheidung 2026-10-13 (MEM05 Teil A) | n/a |
| MEM11 | C1 C3 [C3: L131 WQ/DevReihe/done] | @L3 GEPARKT 2026-10-31 (Anwender: heute nicht erreichbar; der Hosttest, der die Speicheraufteilung unter 2 GiB festhael… | P2 | - | 10 / d92eb6bdd 2026-10-31 / 4 | L44:CS:none, L89:WQ/Geparkt:parked, L131:WQ/DevReihe:done, L154:P2:open | 2026-10-31: ## Anwenderentscheidung 2026-10-31 | yes (status line): (Anwender: heute nicht erreichbar; der Hosttest, der die Speicheraufteilung unter 2 GiB fe… |
| PORT11 | C1 | @L3 TEILWEISE (`docs/sdk/07` `malloc`-Zeile und libc-crt0-Absatz in `docs/sdk/03` nachgezogen, `861ad038`/`57a71b80`; o… | P3 | - | 2 / d40f4f16d 2026-10-21 / 1 | - | none found | n/a |
| PROC08 | C1 | @L3 ZURUECKGESTELLT — to be tested 2026-11-04 (Anwender: Fix `01eb7709` auf der Karte, 60 zView-Starts ohne Absturz; ni… | P1 | - | 17 / aaf6b4e74 2026-11-04 / 4 | L44:CS:none, L44:CS:none, L119:WQ:none, L120:WQ:none, L121:WQ:none, L139:WQ/Zurueckgest:parked | 2026-11-04: ## Stand 2026-11-04 — Pi mit Fix (Anwender) | yes (status line): to be tested 2026-11-04 (Anwender: Fix `01eb7709` auf der Karte, 60 zView-Starts ohne Abst… |
| README | C1 C5 | none (non-ticket .md) | none | - | 33 / ae22f4fc4 2026-11-08 / 13 | - | none found | n/a |
| RST02 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Rest aus RST01<RST01> (Anwenderentscheidung 2026-10-17) | P3 | - | 2 / 95be2e0dd 2026-10-17 / 1 | L121:WQ:none, L177:P34:open | 2026-10-17: Status: OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Rest aus RST01<RST01> (Anwenderents | n/a |
| RTC01 | C1 C3 [C3: L118 WQ/parked] | @L3 OFFEN (after the Atari meeting) · Priority: P3 · Type: new hardware driver | P3 | - | 4 / 6796787ce 2026-10-31 / 1 | L118:WQ:parked, L175:P34:open | none found | n/a |
| SDK01 | C1 | @L3 A–F, H, I, J ERLEDIGT 2026-10-21 (G gestrichen; L erledigt 2026-10-22; K umgesetzt, Abnahme Linux/Homebrew offen) ·… | P2 | - | 29 / 8171511dd 2026-10-26 / 8 | L155:P2:done, L241:PH:none | 2026-10-22: / K / Rust raus aus dem C-Weg: `sdk/toolchain.sh`/`newtos-cc` nehmen zuerst `ld.lld`, `llvm-ar`, `llvm-nm` | n/a |
| SPEC03 | C1 | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Aufraeumarbeit; der Hook blockiert die Altlast nicht) · Pri… | P3 | - | 2 / 46bb1c0ca 2026-10-30 / 1 | L108:WQ/EbGeparkt:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Aufraeumarbeit; der Hook blockiert die Alt | yes (status line): (Anwender, Vor-Start-Pruefung: reine Aufraeumarbeit; der Hook blockiert die Altlast nicht) |
| T08 | C1 | @L3 TEILWEISE (Kern-Fix `swap_vector` auf Zeigerregel `contrl_ptr`/`set_contrl_ptr` und Spec `docs/api/vdi/input/_mouse… | P4 | - | 2 / d40f4f16d 2026-10-21 / 1 | - | none found | n/a |
| T09 | C1 | @L3 OFFEN · Prioritaet: P4 · Typ: Lader / Startcode / Selftest — Reste aus T05/T06 (Developer) | P4 | - | 1 / 750177db4 2026-09-21 / 1 | - | none found | n/a |
| T10 | C1 | @L3 TEILWEISE (`tools/inject-arm.py` mit Programmidentitaets-Ausloesern, DESK43 U9-0d `acb0ff45`; Abnahme „alter a78-La… | P3 | - | 5 / d40f4f16d 2026-10-21 / 1 | - | none found | n/a |
| T11 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Unzuverlaessiger Test — beobachtet 2026-09-26 | P3 | - | 1 / 4d16ace00 2026-09-26 / 1 | - | none found | n/a |
| T13 | C1 | @L3 ZURUECKGESTELLT (2026-09-30, Anwenderentscheidung) · Prioritaet: P2 · Typ: SDK-Beispiele — Anwenderwunsch 2026-09-30 | P2 | - | 1 / f4699fa5f 2026-09-30 / 1 | L161:P2:open | 2026-09-30: ### `\AUTO\HELLO.NCP` entfernt (Anwender 2026-09-30) | yes (status line): , Anwenderentscheidung) |
| T18 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Housekeeping — Kommentare, kein Laufzeitdefekt (wie T06<T06>) | P3 | - | 3 / 3857c9c3d 2026-10-26 / 1 | L179:P34:open | none found | n/a |
| TICK01 | C1 | @L3 GEPARKT 2026-10-31 (Anwender: wieder aufnehmen, wenn die Tester Fehlerbild K6 aus DOOM01 sehen) · Prioritaet: P3 · … | P3 | - | 2 / db112d1eb 2026-10-31 / 1 | L118:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender: wieder aufnehmen, wenn die Tester Fehlerbild K6 aus DOOM01 sehen) ·  | yes (status line): (Anwender: wieder aufnehmen, wenn die Tester Fehlerbild K6 aus DOOM01 sehen) |
| TOOL30 | C1 | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (A126) per Sonde abgenommen) · Prioritaet: P3 · … | P3 | - | 1 / 6ea5f8835 2026-10-30 / 1 | L115:WQ/EbGeparkt:parked, L138:WQ/geparktMIDI:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (A126) per Sonde abgenommen) ·  | yes (status line): (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (A126) per Sonde abgenommen) |
| TOOL34 | C1 | @L3 GEPARKT 2026-11-06 (zugehoeriger Fix V23<V23> per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) · Priori… | P3 | - | 1 / ee67903f7 2026-11-06 / 1 | L121:WQ:parked | none found | yes (status line): (zugehoeriger Fix V23<V23> per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) |
| USB02 | C1 | @L3 OFFEN · Prioritaet: P3 · Typ: Sammelticket — Nebenbefunde Developer/Tester 2026-10-23 | P3 | - | 13 / 19ff79f66 2026-10-24 / 2 | L178:P34:open, L242:PH:none | 2026-10-24: #### D. Anwenderentscheidung 2026-10-24 (= Freigabe der Spec-Regel) | n/a |
| V17 | C1 | @L3 IMPLEMENTIERT (2026-09-14) · Abnahme durch den Tester offen · | none | DONE | 3 / bff78f6fd 2026-09-14 / 1 | - | none found | n/a |
| V18 | C1 | @L3 Teil 1 BEHOBEN (V19–V26), Teil 2 GEMESSEN (2026-11-06, `eba4122a`), Teil 3 GEMESSEN (2026-11-07) · Prioritaet: P2 ·… | P2 | - | 23 / 74ad8d3ed 2026-11-07 / 11 | L64:OA:open, L121:WQ:none, L121:WQ:none, L121:WQ:none, L121:WQ:none, L121:WQ:none, L282:RD:none, L287:RD:none | 2026-11-05: Anwender 2026-11-05: *"es kann durchaus sein das danach einige AES funktionen nicht mehr so | n/a |
| V41 | C1 | @L3 URSACHE BELEGT, Fix offen, ans Listenende (Anwender 2026-11-06, zugunsten HW08<HW08>); D1–D4 entschieden (Anwender … | P3 | - | 7 / 313a0674d 2026-11-08 / 1 | L44:CS:none, L59:OA:open, L121:WQ:none, L121:WQ:none, L289:RD:none | 2026-11-08: Entscheidungen (Anwender 2026-11-08): D1 nachbilden: NewTOS benutzt den Scratch-Puffer des Aufrufers w | n/a |
| ZVIEW01 | C1 | @L3 OFFEN · Prioritaet: P1 (Vorfuehrung) · Typ: Defekt, Ursache unbekannt — Anwender auf der echten Pi 4, 2026-10-07 | P1 | DONE | 3 / e77732416 2026-10-07 / 1 | - | 2026-10-07: Status: OFFEN · Prioritaet: P1 (Vorfuehrung) · Typ: Defekt, Ursache unbekannt — Anwender auf der echte | n/a |
| ZVIEW02 | C1 | @L3 OFFEN · Prioritaet: P2 · Typ: Defekt, Ursache unbekannt — aus ZVIEW01<ZVIEW01-STATUS-DONE> | P2 | DONE | 3 / 2254caf46 2026-10-07 / 2 | - | none found | n/a |
| ZVIEW04 | C1 | @L3 OFFEN · Prioritaet: P2 · Typ: Defekt, Ursache unbekannt | P2 | DONE | 2 / 374aec8c2 2026-10-08 / 1 | - | none found | n/a |
| A06 | C2 | @L3 ERLEDIGT · Priorität: P2 · Typ: Global-Feldprüfung | P2 | - | 0 / - / 0 | L163:ERLoQ:done-no-receipt | none found | n/a |
| A08 | C2 | @L3 ERLEDIGT · Priorität: P2 · Typ: Global-Feldprüfung | P2 | - | 0 / - / 0 | L163:ERLoQ:done-no-receipt | none found | n/a |
| A09 | C2 | @L3 ERLEDIGT · Priorität: P2 · Typ: Global-Feldprüfung | P2 | - | 0 / - / 0 | L163:ERLoQ:done-no-receipt | none found | n/a |
| BIOS05 | C2 | @L3 ACCEPTED-Abweichung 2026-10-28 (ADR-031), offen nur die Warnung in BIOS15<BIOS15> · Prioritaet: P1 · Typ: Developer… | P1 | - | 0 / - / 0 | L127:WQ/DevReihe:none | 2026-10-28: ## Anwenderentscheidung 2026-10-28 | n/a |
| Q01 | C2 | @L3 GEGENSTANDSLOS (2026-09-21) · Priorität: P1 · Typ: Pixel-Messlücke | P1 | - | 0 / - / 0 | - | none found | n/a |
| Q02 | C2 | @L3 GEGENSTANDSLOS (2026-09-21) · Priorität: P1 · Typ: Pixel-Messlücke | P1 | - | 0 / - / 0 | - | none found | n/a |
| Q03 | C2 | @L3 GEGENSTANDSLOS (2026-09-21) · Priorität: P1 · Typ: Pixel-Messlücke | P1 | - | 0 / - / 0 | - | none found | n/a |
| Q04 | C2 | @L3 GEGENSTANDSLOS (2026-09-21) · Priorität: P1 · Typ: Interaktive Messlücke | P1 | - | 0 / - / 0 | - | none found | n/a |
| Q05 | C2 | @L3 GEGENSTANDSLOS (2026-09-21) · Priorität: P1 · Typ: Interaktive Pixel-Messlücke | P1 | - | 0 / - / 0 | - | none found | n/a |
| R03 | C2 | @L3 ERLEDIGT (2026-09-21) — Pi-Abnahme offen · Priorität: P1 · Typ: Messlücke / Messdesign | P1 | - | 0 / - / 0 | L121:WQ:none(range), L152:P1u:impl-open(range), L168:ERLoQ:done-no-receipt(range), L238:PH:none(range) | none found | n/a |
| R04 | C2 | @L3 ERLEDIGT (2026-09-21) — Pi-Abnahme offen · Priorität: P1 · Typ: Messlücke / Messdesign | P1 | - | 0 / - / 0 | L121:WQ:none(range), L152:P1u:impl-open(range), L168:ERLoQ:done-no-receipt(range), L238:PH:none(range) | none found | n/a |
| V02 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L165:ERLoQ:done-no-receipt | none found | n/a |
| V04 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt | none found | n/a |
| V05 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt(range) | none found | n/a |
| V06 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P1 · Typ: Geräteunterschied / Test | P1 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt(range) | none found | n/a |
| V07 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt(range) | none found | n/a |
| V08 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt(range) | none found | n/a |
| V09 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt(range) | none found | n/a |
| V10 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P1 · Typ: Geräteunterschied / Test | P1 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt | none found | n/a |
| V13 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt | none found | n/a |
| V14 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt(range) | none found | n/a |
| V15 | C2 | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Messauswertung | P2 | - | 0 / - / 0 | L166:ERLoQ:done-no-receipt | none found | n/a |
| XBIOS03 | C2 | @L3 ACCEPTED-Abweichung 2026-10-28 (ADR-031), offen nur die Warnung in BIOS15<BIOS15> · Prioritaet: P1 · Typ: Developer… | P1 | - | 0 / - / 0 | L127:WQ/DevReihe:none | 2026-10-28: ## Anwenderentscheidung 2026-10-28 | n/a |
| A139 | C3 [C3: L121 WQ/parked] | @L3 OFFEN (2026-11-06) · Prioritaet: P3 · Typ: Kandidat aus dem Lesen, ungemessen — aus V30<V30> | P3 | - | 0 / - / 0 | L121:WQ:parked | none found | n/a |
| A45 | C3 [C3: L148 P1/open] | @L3 ABGESCHLOSSEN ALS BLOCKED (2026-09-13) · Prioritaet: P1 | P1 | BLOCKED | 3 / 8171511dd 2026-10-26 / 0 | L148:P1:open, L254:BLK:blocked, L274:BLK:blocked | none found | n/a |
| ARCH03 | C3 [C3: L148 P1/open, L174 P34/open] | @L3 UMGESETZT, abgenommen (Tester auf `main` 2026-10-13; Pi mit der Vorfuehrkarte 2026-10-08) · offen nur Stufe 2b, `as… | P1 | ARCH03-0c:DONE, ARCH03-0d:DONE, ARCH03-1:DONE, ARCH03-2:DONE, ARCH03-3a:DONE, ARCH03-3b:DONE, ARCH03-3b:UNFINISHED, ARCH03-3c:DONE, ARCH03-3c:UNFINIS… | 34 / def5d3e20 2026-10-26 / 11 | L30:CS:none, L35:CS:done, L38:CS:none, L121:WQ:none, L148:P1:open, L174:P34:open, L232:PH:none | 2026-10-05: ## Entscheidungen 2026-10-05 (Anwender: „mach einfach weiter“ = Empfehlungen angenommen) | n/a |
| FS28 | C3 [C3: L134 WQ/parked] | @L3 ERLEDIGT 2026-10-30 (Quittung FS28-STATUS-DONE.MD<FS28-STATUS-DONE>, Beleg `measurements/measurement-2026-10-30-fs2… | P2 | DONE | 5 / 10d8e135f 2026-10-30 / 1 | L44:CS:none, L134:WQ:parked, L134:WQ:done, L134:WQ:none | 2026-10-30: - Erreichbarkeit auf rpi4 (Anwenderfrage, Lead 2026-10-30): physisch kein | n/a |
| INPUT04 | C3 [C3: L136 WQ/geparktMIDI/parked] | @L3 OFFEN · Prioritaet: P3 · Typ: Developer — Anwenderentscheidung 2026-10-30, Fund aus XBIOS04<XBIOS04> (`docs/analysi… | P3 | - | 1 / 9f71ded8e 2026-10-30 / 0 | L136:WQ/geparktMIDI:parked | 2026-10-30: ## Vorgabe (Anwender, 2026-10-30) | n/a |
| REL071 | C3 [C3: L69 OA/open] | @L3 ERLEDIGT 2026-11-07 (Quittung<REL071-STATUS-DONE>) · Prioritaet: P1 · Typ: Release — Anwender 2026-11-06: "was wir … | P1 | DONE | 13 / 7e48f7395 2026-11-07 / 0 | L69:OA:open, L121:WQ:none | 2026-11-07: Pi des Anwenders mit dem 0.7.1-ZIP bestanden: `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`. Ver | n/a |
| TOOL04 | C3 [C3: L159 P2/open] | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL04-STATUS-DONE>) · Prioritaet: P2 · Typ: Werkzeug (Testbarkeit) — Anwenderentsche… | P2 | DONE | 10 / c3db3747d 2026-10-28 / 1 | L159:P2:open | 2026-10-27: - Groesse variieren (Anwender 2026-10-27): dasselbe Testbinary frisch | n/a |
| TOOL05 | C3 [C3: L159 P2/parked] | @L3 OFFEN · Prioritaet: P2 · Typ: Werkzeug (Zitierfestigkeit) — Anwenderentscheidung 2026-10-28 | P2 | - | 2 / c3db3747d 2026-10-28 / 0 | L159:P2:parked | 2026-10-28: Status: OFFEN · Prioritaet: P2 · Typ: Werkzeug (Zitierfestigkeit) — Anwenderentscheidung 2026-10-28 | n/a |
| TOOL33 | C3 [C3: L63 OA/open] | @L3 ERLEDIGT 2026-11-05 (Quittung<TOOL33-STATUS-DONE>) · Prioritaet: P1 (Anwender 2026-11-05: nach TOOL32<TOOL32> VDI, … | P1 | DONE | 6 / d352f9ff8 2026-11-05 / 1 | L63:OA:open, L121:WQ:done | 2026-11-05: Status: ERLEDIGT 2026-11-05 (Quittung<TOOL33-STATUS-DONE>) · Prioritaet: P1 (Anwender 2026-11-05: | n/a |
| USB03 | C3 [C3: L121 WQ/parked] | @L3 OFFEN · Prioritaet: P1 (Anwender 2026-11-04, vorher P2; der USB-Rest aus USB01<USB01>) · Typ: Treiberluecke — aus R… | P1 | - | 4 / a06db76c1 2026-11-05 / 0 | L121:WQ:parked, L143:P1:open, L242:PH:none, L242:PH:none | 2026-11-04: Dazu (Anwender 2026-11-04, aus F5 in `docs/api/usb/hid-mouse-protocols.md` §6): | n/a |
| V35 | C3 [C3: L121 WQ/parked] | @L3 ERLEDIGT 2026-11-07 (Quittung<V35-STATUS-DONE>; Fix `788a8aa9`, Tester PASS `44558a7a`; Folge V52<V52> geparkt, TOO… | P3 | DONE | 8 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L121:WQ:done, L121:WQ:parked, L280:RD:none(range) | 2026-11-06: Anwenderentscheidung 2026-11-06 (Auswahl "Alle wie empfohlen"): nur die lokale Kopie nachbauen (`GSX_ENTRY` ko | n/a |
| V38 | C3 [C3: L58 OA/open] | @L3 ERLEDIGT 2026-11-08 (Quittung<V38-STATUS-DONE>; Fix `e6ffdedc`, Tester PASS `f6ae5160`) · Prioritaet: P3 · Typ: Rob… | P3 | DONE | 7 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L58:OA:open, L121:WQ:done, L280:RD:none(range), L288:RD:done | 2026-11-08: Anwender 2026-11-08: Die Grenze soll hoeher. Zuerst die Dauer des 5x3-Endlosfalls mit der heutigen Grenze  | n/a |
| V39 | C3 [C3: L60 OA/done] | @L3 OFFEN (2026-11-06; Anwender 2026-11-08: wird mit V41<V41> gefixt, D4) · Prioritaet: P3 · Typ: Abweichung, gemessen … | P3 | - | 1 / 313a0674d 2026-11-08 / 0 | L60:OA:done, L121:WQ:none, L280:RD:none(range) | 2026-11-06: Status: OFFEN (2026-11-06; Anwender 2026-11-08: wird mit V41<V41> gefixt, D4) · Prioritaet: P3 ·  | n/a |
| V58 | C3 [C3: L55 OA/open] | @L3 ERLEDIGT 2026-11-08 (Quittung<V58-STATUS-DONE>; Commits `1c1c71a3`, `ea4a8996`, Tester PASS `3711dd00`, Pi `measure… | P2 | DONE | 10 / ae22f4fc4 2026-11-08 / 0 | L44:CS:none, L44:CS:none, L55:OA:open, L121:WQ:none, L280:RD:none, L283:RD:none, L283:RD:none | 2026-11-08: Status: ERLEDIGT 2026-11-08 (Quittung<V58-STATUS-DONE>; Commits `1c1c71a3`, `ea4a8996`, Tester PASS ` | n/a |
| G-AES-011 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-012 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-013 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-014 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-015 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-017 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-019 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-020 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-021 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-022 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-023 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-026 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-030 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-031 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-032 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-033 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-034 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-035 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-036 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-037 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-038 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-039 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-040 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-041 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-042 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-043 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-044 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-045 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-046 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-047 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-048 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-050 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-051 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-052 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-053 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-054 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-055 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-056 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-070 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-071 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-072 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-073 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-074 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-075 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-076 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-080 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-081 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-082 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-090 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-091 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-107 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-110 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-111 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-112 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-113 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-114 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-120 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-121 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-122 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-123 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-124 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-125 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-126 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-AES-127 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-001 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-002 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-014 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-028 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-031 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-101 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-112 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-118 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-119 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-120 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-124 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-125 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-126 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-127 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-128 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| G-VDI-134 | C4 | @L3 BACKLOG · Priorität: P3 · Typ: Funktionsweise ungemessen / Messdesign | P3 | - | 0 / - / 0 | - | none found | no |
| NET06 | C4 | @L3 BACKLOG · Prioritaet: P4 · Typ: Hardwaretreiber (gross) | P4 | - | 0 / - / 0 | - | none found | no |
| A25-AUDIT-BEANSTANDET.MD | C5 | non-ticket artefact, no status line | none | - | - | - | none found | n/a |
| COMPARISON | C5 | none (non-ticket .md) | none | - | 0 / - / 0 | - | none found | n/a |
| GAPS-INDEX-2026-09-12 | C5 | none (non-ticket .md) | none | - | 0 / - / 0 | - | none found | n/a |
| HANDOVER-2026-09-11 | C5 | none (non-ticket .md) | none | - | 0 / - / 0 | - | none found | n/a |
| KEYS01-SEQUENCE-CHECK.MD | C5 | non-ticket artefact, no status line | none | - | - | - | none found | n/a |
| PORT06-KEYS-DIAGNOSIS.MD | C5 | non-ticket artefact, no status line | none | - | - | - | none found | n/a |
| RSC-AUDIT-INDEX | C5 | none (non-ticket .md) | none | - | 0 / - / 0 | - | none found | n/a |
| evidence | C5 | non-ticket artefact, no status line | none | - | - | - | none found | n/a |
| tasks.json | C5 | non-ticket artefact, no status line | none | - | - | - | none found | n/a |
| ACCTITLE | C6 | no ticket file (orphan receipt) | none | TASK/ACCTITLE-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| DESK57 | C6 [C6: L3: Status: ERLEDIGT 2026-10-17 — superseded by DESK62<DESK62> (Quitt…] | @L3 ERLEDIGT 2026-10-17 — superseded by DESK62<DESK62> (Quittung<DESK57-STATUS-DONE>) · Priority: P2 · Type: defect, re… | P2 | DONE | 2 / b50827859 2026-10-17 / 0 | - | none found | n/a |
| FIX-01 | C6 | no ticket file (orphan receipt) | none | TASK/FIX-01-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| FIX-02 | C6 | no ticket file (orphan receipt) | none | TASK/FIX-02-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| FIX-03 | C6 | no ticket file (orphan receipt) | none | TASK/FIX-03-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| FIX-04 | C6 | no ticket file (orphan receipt) | none | TASK/FIX-04-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| FIX-05 | C6 | no ticket file (orphan receipt) | none | TASK/FIX-05-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| FIX-06 | C6 | no ticket file (orphan receipt) | none | TASK/FIX-06-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| FS07 | C6 [C6: L3: Status: ERLEDIGT 2026-10-13 (Duplikat von FS10<FS10>, `f6049bf8`:…] | @L3 ERLEDIGT 2026-10-13 (Duplikat von FS10<FS10>, `f6049bf8`: `Fseek` auf Zeichengeraete-Handle liefert 0; Pi offen) · … | P2 | - | 2 / d40f4f16d 2026-10-21 / 1 | L164:ERLoQ:done-no-receipt | none found | n/a |
| LDG-M1 | C6 | no ticket file (orphan receipt) | none | TASK/LDG-M1-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| LDG-M2 | C6 | no ticket file (orphan receipt) | none | TASK/LDG-M2-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| LDG-M3 | C6 | no ticket file (orphan receipt) | none | TASK/LDG-M3-STATUS-DONE.MD; TASK/LDG-M3-STATUS-UNFINISHED.MD | 2 / 033da16fb 2026-10-25 / 1 | - | none found | n/a |
| LDG-M3-B1 | C6 | no ticket file (orphan receipt) | none | TASK/LDG-M3-B1-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| LDG-M3-B2 | C6 | no ticket file (orphan receipt) | none | TASK/LDG-M3-B2-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| LIBC-MAIN | C6 | no ticket file (orphan receipt) | none | TASK/LIBC-MAIN-STATUS-DONE.MD | 4 / cc9d995ab 2026-10-13 / 0 | - | none found | n/a |
| PORT-WAITH | C6 | no ticket file (orphan receipt) | none | TASK/PORT-WAITH-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| PPARENT | C6 | no ticket file (orphan receipt) | none | TASK/PPARENT-STATUS-DONE.MD | 3 / 861ad0380 2026-10-06 / 2 | - | none found | n/a |
| R02 | C6 | @L3 ERLEDIGT (2026-09-21) — Pi-Abnahme offen · Priorität: P1 · Typ: Messlücke / Messdesign | P1 | R02-R05:DONE(range) | 0 / - / 0 | L121:WQ:none, L152:P1u:impl-open, L168:ERLoQ:done-no-receipt, L238:PH:none | none found | n/a |
| R05 | C6 | @L3 ERLEDIGT (2026-09-21) — Pi-Abnahme offen · Priorität: P1 · Typ: Messlücke / Messdesign | P1 | R02-R05:DONE(range) | 1 / fe310b133 2026-09-21 / 1 | L121:WQ:none, L152:P1u:impl-open, L168:ERLoQ:done-no-receipt, L238:PH:none | none found | n/a |
| R3-TABELLE | C6 | no ticket file (orphan receipt) | none | TASK/R3-TABELLE-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| V-SERIES | C6 | no ticket file (orphan receipt) | none | TASK/V-SERIES-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| ZVIEW-M4 | C6 | no ticket file (orphan receipt) | none | TASK/ZVIEW-M4-STATUS-DONE.MD; TASK/ZVIEW-M4-STATUS-UNFINISHED.MD | 1 / feb76357f 2026-10-17 / 0 | - | none found | n/a |
| ZVIEW-M4-K1 | C6 | no ticket file (orphan receipt) | none | TASK/ZVIEW-M4-K1-STATUS-DONE.MD | 0 / - / 0 | - | none found | n/a |
| A01 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| A02 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| A03 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| A04 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| A05 | OK | @L3 ERLEDIGT (2026-09-19) — Versionsunterschied · Priorität: P2 · Typ: Versionsprüfung | P2 | DONE | 2 / 6f3883ce7 2026-09-20 / 0 | - | none found | n/a |
| A07 | OK | @L3 ERLEDIGT (2026-09-19) — Konfigurationsunterschied belegt · Priorität: P2 · Typ: Global-Feldprüfung | P2 | DONE | 2 / d6ff251ba 2026-10-29 / 1 | - | none found | n/a |
| A10 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Nachmessung auswerten | P1 | DONE | 0 / - / 0 | - | none found | n/a |
| A100 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet desk/ — Nebenbefund A99 (Develop… | P3 | DONE | 2 / 1b3ce18cf 2026-09-21 / 2 | - | none found | n/a |
| A101 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P3 · Typ: Dokumentationskorrektur — Nebenbefund A86 (Researcher) | P3 | DONE | 2 / 52c4362b6 2026-09-21 / 0 | - | none found | n/a |
| A102 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet desk/ + AES — Nebenbefunde A87/A… | P3 | DONE | 2 / 1b3ce18cf 2026-09-21 / 2 | - | none found | n/a |
| A103 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P4 · Typ: Konformitaet — Nebenbefunde A89 (Developer) | P4 | DONE | 2 / cf31f1d14 2026-09-21 / 2 | - | none found | n/a |
| A104 | OK | @L3 ERLEDIGT (2026-09-23) — Fix im Top-Request-Zweig, Rest als Teilmenge dokumentiert · Prioritaet: P4 · Typ: Konformit… | P4 | DONE | 1 / 7a250028e 2026-09-23 / 1 | - | none found | n/a |
| A105 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen (Punkt 1, Infozeile) · Prioritaet: P4 · Typ: Konformitaet — Nebenbefu… | P4 | DONE | 2 / cf31f1d14 2026-09-21 / 2 | - | none found | n/a |
| A106 | OK | @L3 ERLEDIGT (2026-09-22) — Pi-Abnahme offen · Prioritaet: P3 · Typ: Funktionsluecke / Protokoll — Tier 2 aus A48 (Desi… | P3 | DONE | 6 / f6f902027 2026-10-16 / 4 | - | none found | n/a |
| A107 | OK | @L3 ERLEDIGT (2026-09-22) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Robustheit — Rest aus A96 (Developer) | P3 | DONE | 3 / 7a250028e 2026-09-23 / 3 | - | none found | n/a |
| A108 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet desk/ — Nebenbefund A100/A102 (D… | P3 | DONE | 2 / cf31f1d14 2026-09-21 / 2 | - | none found | n/a |
| A109 | OK | @L3 ERLEDIGT (2026-09-22) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet desk/ + AES — Residuen A103/A105… | P3 | DONE | 2 / 90671b304 2026-09-22 / 2 | - | none found | n/a |
| A11 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Nachmessung auswerten | P1 | DONE | 0 / - / 0 | - | none found | n/a |
| A110 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Konformitaet — Reste aus A107/A109 (Developer) | P4 | DONE | 3 / b7356cf8b 2026-09-23 / 3 | - | none found | n/a |
| A111 | OK | @L3 ERLEDIGT (QEMU; Pi offen) — Punkt 1 mit ZVIEW02<ZVIEW02-STATUS-DONE>, `388c6bcd`; Punkt 2 mit `033da16f`, siehe A11… | P3 | DONE | 7 / 587e0f688 2026-10-25 / 3 | - | none found | n/a |
| A112 | OK | @L3 ERLEDIGT 2026-10-17 (QEMU, `1762e837`; Pi abgenommen 2026-10-17, `docs/HW-TEST-LOG.md`) — Quittung<A112-STATUS-DONE… | P2 | DONE | 13 / e7754b16e 2026-10-25 / 2 | - | 2026-10-17: ## Freigabe und Umfang (Anwender 2026-10-17) | n/a |
| A113 | OK | @L3 ERLEDIGT 2026-10-17 (QEMU, `79383cd0`; echte Pi offen) — Quittung<A113-STATUS-DONE> · Prioritaet: P2 · Typ: Abweich… | P2 | DONE | 7 / 1a7e2c828 2026-10-19 / 1 | - | none found | n/a |
| A114 | OK | @L3 ERLEDIGT 2026-10-17 (`79383cd0`) — Quittung<A114-STATUS-DONE> · Prioritaet: P2 · Typ: moegliche Abweichung von EmuT… | P2 | DONE | 4 / a46910912 2026-10-17 / 1 | - | none found | n/a |
| A115 | OK | @L3 ERLEDIGT (QEMU, 2026-10-25, `e7754b16`) — Pi offen · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Rest aus A112<A1… | P3 | DONE | 5 / 17cab8b65 2026-10-25 / 1 | L232:PH:none | none found | n/a |
| A116 | OK | @L3 ERLEDIGT 2026-10-22 (QEMU, `b7be9bb1`; LIFO und Selbstschreiber blockieren wie EmuTOS; `measurements/measurement-20… | P3 | - | 5 / 02fd1b8dc 2026-10-22 / 1 | L163:ERLoQ:done-no-receipt | 2026-10-22: ## Anwenderentscheid 2026-10-22 | n/a |
| A117 | OK | @L3 ERLEDIGT 2026-10-25 (QEMU, `eaf32b04`; echte Pi offen) — Quittung<A117-STATUS-DONE> · Prioritaet: P2 · Typ: Abweich… | P2 | DONE | 7 / 582a31ffc 2026-10-26 / 1 | - | none found | n/a |
| A118 | OK | @L3 ERLEDIGT (QEMU) 2026-10-26 (`ded51d9e`, Nacharbeit `3cb21e31`; Pi offen) · Prioritaet: P3 · Typ: Abweichung von Emu… | P3 | DONE | 7 / e72590886 2026-10-26 / 1 | L239:PH:none | 2026-10-26: - Entscheidung Anwender 2026-10-26: 1:1 wie EmuTOS. Ein nach dem Warten | n/a |
| A119 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS (Optik) — F4 aus GH1<GH1> (2026-10-19), bei GH1 bewusst nicht u… | P3 | - | 1 / 22325b9a6 2026-10-22 / 0 | L121:WQ:none, L177:P34:open | none found | n/a |
| A12 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Nachmessung auswerten | P1 | DONE | 0 / - / 0 | - | none found | n/a |
| A120 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenbefunde aus A116<A116> (2026-10-22, `b7be9bb1`) | P3 | - | 1 / 02fd1b8dc 2026-10-22 / 0 | L121:WQ:none(range), L177:P34:open(range) | none found | n/a |
| A121 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS (ungeprueft) — Nebenbefund aus A117<A117> (2026-10-25, `eaf32b0… | P3 | - | 1 / 8da6e8601 2026-10-25 / 0 | L121:WQ:none(range), L177:P34:open(range) | none found | n/a |
| A122 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS (ungeprueft) — Nebenbefund aus A111<A111> (2026-10-25, `033da16… | P3 | - | 2 / 9ef69b0f0 2026-11-05 / 0 | L121:WQ:none(range), L177:P34:open(range) | none found | n/a |
| A123 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: moeglicher Defekt (ungeprueft) — Nebenbefund aus A111<A111> (2026-10-25, `033da16f`) | P3 | - | 1 / ad9b023da 2026-10-25 / 0 | L121:WQ:none(range), L177:P34:open(range) | none found | n/a |
| A124 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Testluecke — Nebenbefund aus A115<A115> (2026-10-25, Tester `74505b22`) | P3 | - | 1 / 17cab8b65 2026-10-25 / 0 | L121:WQ:none(range), L177:P34:open(range) | none found | n/a |
| A125 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Robustheit (Kernel) — Nebenbefund aus A21<A21> (2026-09-19), aus dem STATUS-Log ueber… | P3 | - | 1 / f242696e8 2026-10-26 / 0 | L121:WQ:none, L177:P34:open | none found | n/a |
| A126 | OK | @L3 ERLEDIGT 2026-10-30 (Quittung<A126-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer — Fund O2 aus BIOS10<BIOS10> (20… | P2 | DONE | 4 / 6ea5f8835 2026-10-30 / 1 | L44:CS:none, L115:WQ/EbGeparkt:done, L134:WQ:done, L134:WQ:none, L134:WQ:none | 2026-10-30: ## Abnahme (Anwender, 2026-10-30) | n/a |
| A128 | OK | @L3 UMGESETZT 2026-11-04 (`df2c7501`; Tester PASS `measurements/measurement-2026-11-04-a128-test/RUNINFO.TXT`; offen nu… | P1 | - | 11 / 8040abebb 2026-11-05 / 2 | L121:WQ:done | 2026-11-04: Entscheidungen dort unter "Entscheidungen (Anwender 2026-11-04)": Rad nur in der | n/a |
| A129 | OK | @L3 ERLEDIGT 2026-11-04 — konform zu EmuTOS, kein Fehler (Anwender einverstanden; Quittung<A129-STATUS-DONE>) · Priorit… | P2 | DONE | 3 / 3b1fd0668 2026-11-04 / 0 | L121:WQ:done | 2026-11-04: Status: ERLEDIGT 2026-11-04 — konform zu EmuTOS, kein Fehler (Anwender einverstanden; [Quittung](A129-STAT | n/a |
| A13 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Messlücke / bedingte Reparatur | P1 | DONE, UNFINISHED | 0 / - / 0 | - | none found | n/a |
| A130 | OK | @L3 ERLEDIGT 2026-11-05 (Quittung<A130-STATUS-DONE>); #2 beantwortet und geschlossen 2026-11-05 · Prioritaet: P2 · Typ:… | P2 | DONE | 12 / 3e825c135 2026-11-07 / 1 | L121:WQ:none | 2026-11-05: geschlossen 2026-11-05 (Text vom Anwender freigegeben). | n/a |
| A131 | OK | @L3 ERLEDIGT 2026-11-05 (Quittung<A131-STATUS-DONE>); #7 beantwortet und geschlossen 2026-11-05 · Prioritaet: P3 · Typ:… | P3 | DONE | 9 / 3e825c135 2026-11-07 / 1 | L121:WQ:none | 2026-11-05: 2026-11-05 (Text vom Anwender freigegeben). | n/a |
| A134 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichungen von EmuTOS — Nebenbefunde aus A132 (2026-11-04, Developer, per Lesen) | P3 | - | 0 / - / 0 | L121:WQ:none | none found | n/a |
| A135 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenbefund aus A133 (2026-11-04, gelesen) | P3 | - | 1 / 9ef69b0f0 2026-11-05 / 0 | L121:WQ:none | none found | n/a |
| A136 | OK | @L3 OFFEN · Prioritaet: P3 (Desktop-Menues haben vermutlich keine Luecken, nicht gemessen) · Typ: Konformitaet — aus de… | P3 | - | 1 / eae13cecd 2026-11-05 / 0 | L121:WQ:none | none found | n/a |
| A137 | OK | @L3 OFFEN (2026-11-06) · Prioritaet: P3 · Typ: Kandidat aus dem Lesen, ungemessen — aus V30<V30> | P3 | - | 0 / - / 0 | L121:WQ:none | none found | n/a |
| A138 | OK | @L3 OFFEN (2026-11-06) · Prioritaet: P3 · Typ: Kandidat aus dem Lesen, ungemessen — aus V30<V30> | P3 | - | 0 / - / 0 | L121:WQ:none | none found | n/a |
| A14 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Messlücke / bedingte Reparatur | P1 | DONE, UNFINISHED | 0 / - / 0 | - | none found | n/a |
| A140 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<A140-STATUS-DONE>; `fba96623`, Pi ok, #22 geschlossen) · Prioritaet: P2 · Typ: Tester… | P2 | DONE | 4 / 72624def3 2026-11-07 / 1 | L45:CS:none, L121:WQ:done | 2026-11-07: Pi des Anwenders mit dem 0.7.1-ZIP bestanden: `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`. Ver | n/a |
| A141 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<A141-STATUS-DONE>; B2 `857cf10d`, Tester PASS, Pi ok, #23 geschlossen) · Prioritaet: … | P2 | DONE | 8 / 7e48f7395 2026-11-07 / 0 | L44:CS:none, L121:WQ:none | 2026-11-07: Pi des Anwenders mit dem 0.7.1-ZIP bestanden: `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`. Ver | n/a |
| A142 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<A142-STATUS-DONE>; `2f6ea7eb`, `56fd81a1`, Pi ok, #21 geschlossen) · Prioritaet: P3 ·… | P3 | DONE | 7 / 72624def3 2026-11-07 / 2 | L45:CS:none, L121:WQ:none | 2026-11-07: Pi des Anwenders mit dem 0.7.1-ZIP bestanden: `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`. Ver | n/a |
| A143 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P3 · Typ: Abweichung, gelesen, nicht gemessen — aus `docs/analysis/fbench-function… | P3 | - | 3 / b0d6457dd 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| A144 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P3 · Typ: Abweichung, am Orakel gesehen, Ursache offen — aus A142<A142> | P3 | - | 2 / b0d6457dd 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| A145 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P3 · Typ: Messluecke und Abweichung, gelesen — aus `docs/analysis/fbench-functions… | P3 | - | 1 / b0d6457dd 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| A146 | OK | @L3 GEPARKT (2026-11-07, Anwender: "auf eine spaetere Research packen") · Prioritaet: P4 · Typ: Frage, Recherche — aus … | P4 | - | 1 / ec3403aa3 2026-11-07 / 0 | L121:WQ:none | 2026-11-07: Status: GEPARKT (2026-11-07, Anwender: "auf eine spaetere Research packen") · Prioritaet: P4 · Typ: Fr | yes (status line): , Anwender: "auf eine spaetere Research packen") |
| A147 | OK | @L3 OFFEN (2026-11-08) · Prioritaet: P3 · Typ: Abweichung, gelesen (nicht gemessen) — Nebenfund aus V54<V54> | P3 | - | 1 / 7774bef5c 2026-11-08 / 0 | L121:WQ:none, L121:WQ:none, L282:RD:none | none found | n/a |
| A148 | OK | @L3 GEPARKT — Frage (2026-11-08, Anwender: "so irrelevant erstmal, nur wenn man ganz schnell hin und her scrollt") · Pr… | P4 | - | 5 / ad58e69e0 2026-11-10 / 0 | L121:WQ:none | 2026-11-08: ## Antworten (Anwender, 2026-11-08) | yes (status line): Frage (2026-11-08, Anwender: "so irrelevant erstmal, nur wenn man ganz schnell hin und her… |
| A15 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Messlücke / bedingte Reparatur | P1 | DONE, UNFINISHED | 0 / - / 0 | - | none found | n/a |
| A17 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Spec-/Belegkorrektur | P2 | DONE | 1 / 1be3fd942 2026-09-19 / 0 | - | none found | n/a |
| A18 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P1 · Typ: Spec-Korrektur | P1 | DONE | 1 / 6ce6111f0 2026-09-19 / 0 | - | none found | n/a |
| A19 | OK | @L3 ERLEDIGT (2026-09-14) · Priorität: P1 · Typ: Belegter Defekt / ABI | P1 | DONE | 3 / 021e47b39 2026-09-14 / 1 | - | none found | n/a |
| A20 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Belegter Defekt / Geräteableitung | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| A21 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Belegter Defekt / Robustheit | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| A22 | OK | @L3 ERLEDIGT (2026-09-12) · Priorität: P1 · Typ: Messlücke / Messdesign | P1 | DONE | 2 / 134e0b9f3 2026-09-14 / 1 | - | none found | n/a |
| A23 | OK | @L3 ERLEDIGT (2026-09-10, siehe A23-STATUS-DONE.MD<A23-STATUS-DONE>) · Priorität: P1 · Typ: Gate / Analyse vor Reparatur | P1 | DONE | 1 / 68afeadd4 2026-09-19 / 0 | - | 2026-09-10: ## Anwenderentscheidung 2026-09-10 — Schattengröße für Stufe 4 | n/a |
| A24 | OK | @L3 ERLEDIGT (2026-09-12) · Priorität: P1 · Typ: Messlücke / Messdesign | P1 | DONE | 1 / 134e0b9f3 2026-09-14 / 0 | - | none found | n/a |
| A25 | OK | @L3 ERLEDIGT (2026-10-17, QEMU-Lauf) · Priorität: P1 · Typ: bestätigter Zustandsfehler | P1 | DONE, UNFINISHED | 8 / fe5ba8f97 2026-10-17 / 1 | - | none found | n/a |
| A26 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Erratum / Dokumentation | P2 | - | 1 / 99e625dcc 2026-09-19 / 0 | L163:ERLoQ:done-no-receipt | none found | n/a |
| A27 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P2 · Typ: Bewertung einer bewussten Abweichung | P2 | DONE | 2 / 0b6d91a5b 2026-09-20 / 1 | - | none found | n/a |
| A28 | OK | @L3 ERLEDIGT (2026-09-14) · Priorität: P1 · Typ: Belegter Defekt / Startpfad | P1 | DONE, UNFINISHED | 5 / feb76357f 2026-10-17 / 1 | - | none found | n/a |
| A29 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<A29-STATUS-DONE>) · Priorität: P1 · Typ: Belegter Defekt / Startpfad | P1 | DONE, UNFINISHED | 5 / f6f902027 2026-10-16 / 1 | - | none found | n/a |
| A30 | OK | @L3 ERLEDIGT (2026-09-19) — Entwurf; Ausfuehrung als Folgeauftrag · Priorität: P1 · Typ: Messlücke / Messdesign | P1 | DONE | 1 / 54a3e7551 2026-09-19 / 0 | - | none found | n/a |
| A31 | OK | @L3 GEGENSTANDSLOS (2026-09-19) · Priorität: P2 · Typ: Testlücke | P2 | DONE | 1 / 3864bc320 2026-09-19 / 0 | - | none found | n/a |
| A32 | OK | @L3 ERLEDIGT (2026-09-19) · Priorität: P1 · Typ: Hardwareabnahme | P1 | DONE, UNFINISHED | 1 / 68afeadd4 2026-09-19 / 0 | - | none found | n/a |
| A33 | OK | @L3 ERLEDIGT (2026-09-12) · Priorität: P2 · Typ: Belegter Defekt | P2 | DONE | 2 / 134e0b9f3 2026-09-14 / 1 | - | none found | n/a |
| A35 | OK | @L3 ERLEDIGT (2026-09-14) · Prioritaet: P1 | P1 | DONE, UNFINISHED | 8 / feb76357f 2026-10-17 / 2 | - | none found | n/a |
| A36 | OK | @L3 ERLEDIGT (2026-09-13) · Prioritaet: P1 · Typ: Belegter Defekt / Relokation | P1 | DONE | 1 / 4cbd74855 2026-09-13 / 1 | - | none found | n/a |
| A38 | OK | @L3 ERLEDIGT (2026-09-12) · Prioritaet: P1 · Typ: Belegter Defekt / SPEICHERFEHLER | P1 | DONE | 2 / 134e0b9f3 2026-09-14 / 1 | - | none found | n/a |
| A39 | OK | @L3 ERLEDIGT (2026-09-19, beide Stufen), auf echter Pi 4 bestaetigt 2026-09-20 · Prioritaet: P1 (hochgestuft 2026-09-18… | P1 | DONE | 10 / 3766c6ca1 2026-09-20 / 5 | - | none found | n/a |
| A40 | OK | @L3 ERLEDIGT (2026-09-19), auf echter Pi 4 bestaetigt 2026-09-20 · Prioritaet: P2 · Typ: Latente Abweichung | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| A41 | OK | @L3 ERLEDIGT (2026-09-19), auf echter Pi 4 bestaetigt 2026-09-20 · Prioritaet: P2 · Typ: Funktionsluecke | P2 | DONE | 4 / fa72363b6 2026-09-19 / 3 | - | none found | n/a |
| A42 | OK | @L3 ERLEDIGT (2026-09-13) · Prioritaet: P1 · Typ: Belegter Defekt / gemessen | P1 | DONE | 6 / 77b7e2963 2026-09-20 / 3 | - | none found | n/a |
| A43 | OK | @L3 ERLEDIGT (2026-09-19) · Prioritaet: P2 · Typ: Belegter Defekt / gemessen | P2 | DONE | 1 / d89a84579 2026-09-19 / 1 | - | none found | n/a |
| A44 | OK | @L3 ERLEDIGT (2026-09-19) · Prioritaet: P2 · Typ: Belegter Defekt / optisch | P2 | DONE | 1 / d89a84579 2026-09-19 / 1 | - | none found | n/a |
| A46 | OK | @L3 ERLEDIGT (2026-09-19), auf echter Pi 4 bestaetigt 2026-09-20 · Prioritaet: P2 · Typ: Belegter Defekt / optisch | P2 | DONE | 2 / 6f3883ce7 2026-09-20 / 1 | - | none found | n/a |
| A47 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P2 · Typ: Belegter Defekt / gemessen | P2 | DONE | 1 / 0b6d91a5b 2026-09-20 / 1 | - | none found | n/a |
| A48 | OK | @L3 ERLEDIGT (2026-09-21) — Tier 2 in A106 · Prioritaet: P2 · Typ: Funktionsluecke | P2 | DONE | 3 / ba6194d50 2026-09-23 / 3 | - | none found | n/a |
| A49 | OK | @L3 ERLEDIGT (2026-09-20) — HW-Sichtpunkt offen · Prioritaet: P3 · Typ: Funktionsluecke | P3 | DONE | 1 / 2a4636c06 2026-09-20 / 1 | - | none found | n/a |
| A50 | OK | @L3 ERLEDIGT (2026-09-20, Option (a)) · Prioritaet: P2 · Typ: Abweichung / zu bewerten | P2 | DONE | 1 / 3766c6ca1 2026-09-20 / 1 | - | none found | n/a |
| A51 | OK | @L3 GEGENSTANDSLOS (2026-09-20) · Prioritaet: P2 · Typ: Belegter Defekt / Startpfad | P2 | DONE | 1 / 84b39a023 2026-09-20 / 0 | - | none found | n/a |
| A52 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P3 · Typ: Robustheit | P3 | DONE | 1 / 0b6d91a5b 2026-09-20 / 1 | - | none found | n/a |
| A53 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P2 · Typ: Doku / Erratum | P2 | DONE | 1 / 6f3883ce7 2026-09-20 / 0 | - | none found | n/a |
| A54 | OK | @L3 ERLEDIGT (2026-09-12) · Priorität: P1 · Typ: Belegter Defekt / gemessen | P1 | DONE | 4 / 6154e982b 2026-09-20 / 1 | - | none found | n/a |
| A55 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P2 · Typ: Belegter Defekt / gemessen | P2 | DONE | 1 / 0b6d91a5b 2026-09-20 / 1 | - | none found | n/a |
| A56 | OK | @L3 ERLEDIGT (2026-09-20) — Hardwareabnahme offen · Priorität: P2 · Typ: Belegter Defekt / gemessen | P2 | DONE | 4 / 83a403cd1 2026-09-20 / 2 | - | none found | n/a |
| A57 | OK | @L3 ERLEDIGT (2026-09-20) — Hardwareabnahme offen · Priorität: P2 · Typ: Fehlende Infrastruktur / Entwurf vor Code | P2 | DONE | 4 / 1794be822 2026-09-20 / 2 | - | none found | n/a |
| A58 | OK | @L3 GEGENSTANDSLOS (2026-09-20, durch A27) · Prioritaet: P3 · Typ: Konformitaet | P3 | DONE | 1 / 0b6d91a5b 2026-09-20 / 1 | - | none found | n/a |
| A59 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P2 · Typ: Pruefluecke (nicht: belegter Defekt) | P2 | DONE | 2 / 77b7e2963 2026-09-20 / 0 | - | none found | n/a |
| A60 | OK | @L3 ERLEDIGT (2026-09-19) — Hardwareabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt in einer Zielanwendung | P2 | DONE | 7 / 65bde94ed 2026-09-19 / 3 | - | none found | n/a |
| A61 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P3 · Typ: Belegter Defekt / ABI | P3 | DONE | 2 / 2a4636c06 2026-09-20 / 1 | - | none found | n/a |
| A62 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<A62-STATUS-DONE>) · Prioritaet: P2 · Typ: Folgefrage aus einem Fix | P2 | DONE, UNFINISHED | 5 / 83a403cd1 2026-09-20 / 2 | - | none found | n/a |
| A63 | OK | @L3 ERLEDIGT (2026-09-17) · Prioritaet: P2 · Typ: Belegter Defekt / bewusste Abkuerzung | P2 | DONE | 7 / 00b1e2659 2026-09-17 / 1 | - | 2026-09-17: Beobachtung des Anwenders auf echter Pi 4 (2026-09-17): *„es fehlen z.B. Rahmen | n/a |
| A64 | OK | @L3 ERLEDIGT (2026-09-17) · Prioritaet: P2 · Typ: Belegter Defekt / 3D-Optik | P2 | DONE | 9 / 800781b49 2026-09-20 / 2 | - | 2026-09-17: Anwender, 2026-09-17, nach dem Hatari-Referenzvergleich: *„der 3D-Look ist | n/a |
| A65 | OK | @L3 ERLEDIGT (2026-09-17) · Prioritaet: P1 · Typ: Belegter Defekt / gemessen und nachgestellt | P1 | DONE | 8 / 88acb5972 2026-09-18 / 2 | - | 2026-09-17: Der Anwender hat am 2026-09-17 einen Zeichenfehler im AES-Dateiauswaehler auf | n/a |
| A66 | OK | @L3 ERLEDIGT (2026-09-18) — Hardwareabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt / gleiche Klasse wie A65 | P2 | DONE | 5 / 800781b49 2026-09-20 / 1 | - | none found | n/a |
| A67 | OK | @L3 ERLEDIGT (2026-09-18) — Sichtabnahme auf der Pi offen · Prioritaet: P2 · Typ: Bewusste Abweichung / gemessen | P2 | DONE | 5 / 800781b49 2026-09-20 / 1 | - | none found | n/a |
| A68 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Belegter Defekt / Nebenbefund A67 | P3 | DONE | 1 / 8b7dcdd16 2026-09-20 / 1 | - | none found | n/a |
| A69 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Belegter Defekt / Nebenbefund A67 | P3 | DONE | 1 / 8b7dcdd16 2026-09-20 / 1 | - | none found | n/a |
| A70 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Belegter Defekt / Nebenbefund A67 | P3 | DONE | 1 / 8b7dcdd16 2026-09-20 / 1 | - | none found | n/a |
| A71 | OK | @L3 ERLEDIGT (2026-09-20, mit A57) · Prioritaet: P3 · Typ: Belegter Defekt / Nebenbefund A66 | P3 | DONE | 1 / 1794be822 2026-09-20 / 1 | - | none found | n/a |
| A72 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P3 · Typ: Dokumentationskorrektur | P3 | DONE | 7 / 371c50fdc 2026-11-02 / 3 | - | none found | n/a |
| A73 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Abnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefunde H02 | P3 | DONE | 3 / c15baedba 2026-09-21 / 2 | - | none found | n/a |
| A75 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Abnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A30 | P3 | DONE | 5 / b7be9bb1d 2026-10-22 / 3 | - | none found | n/a |
| A76 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Abnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A41 | P3 | DONE | 3 / 28b1b5693 2026-09-20 / 2 | - | none found | n/a |
| A77 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A49 | P3 | DONE | 2 / 4f25dd03a 2026-09-20 / 2 | - | none found | n/a |
| A79 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Belegter Defekt / Konformitaet (A59, Bauart 3… | P3 | DONE | 5 / a46e5873b 2026-09-20 / 3 | - | none found | n/a |
| A80 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P3 · Typ: Belegter Defekt / Robustheit (A59, Bauart 3) | P3 | DONE | 2 / d529a7068 2026-09-20 / 1 | - | none found | n/a |
| A81 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt / ABI (A59, Bauart 3) + Spec … | P2 | DONE | 4 / 35cb5012f 2026-09-21 / 2 | - | none found | n/a |
| A82 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P3 · Typ: Spec-Korrektur (Researcher) — A59 | P3 | DONE | 2 / 1e0f4e016 2026-09-20 / 1 | - | none found | n/a |
| A83 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet (A59, Bauart 3) | P3 | DONE | 3 / a46e5873b 2026-09-20 / 2 | - | none found | n/a |
| A84 | OK | @L3 ERLEDIGT (2026-09-20) · Prioritaet: P4 · Typ: Konformitaet, in Pixeln unbeobachtbar (A59, Bauart 3) | P4 | DONE | 2 / 1e0f4e016 2026-09-20 / 1 | - | none found | n/a |
| A85 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A68–A70 (Researche… | P3 | DONE | 3 / 92299c0f1 2026-09-21 / 3 | - | none found | n/a |
| A86 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P3 · Typ: Dokumentationskorrektur / Nebenbefund A72 | P3 | DONE | 2 / dc2481fca 2026-09-21 / 0 | - | none found | n/a |
| A87 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A73 (Auditor) | P3 | DONE | 2 / 1b3ce18cf 2026-09-21 / 2 | - | none found | n/a |
| A88 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A73 (Auditor) | P3 | DONE | 1 / 49a117239 2026-09-21 / 1 | - | none found | n/a |
| A89 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A73 (Developer) | P3 | DONE | 1 / c15baedba 2026-09-21 / 1 | - | none found | n/a |
| A90 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A73 (Developer) | P3 | DONE | 1 / c15baedba 2026-09-21 / 1 | - | none found | n/a |
| A91 | OK | @L3 ERLEDIGT (2026-09-23) — dokumentiert · Prioritaet: P4 · Typ: Struktur / Nebenbefund A73 (Developer) — haengt am koo… | P4 | DONE | 3 / 4f3e8bfb2 2026-10-17 / 2 | - | none found | n/a |
| A92 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Konformitaet / Nebenbefund A73-Ursachenaudit … | P3 | DONE | 2 / c15baedba 2026-09-21 / 1 | - | none found | n/a |
| A93 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P4 · Typ: Konformitaet / Nebenbefund A77 (Developer) | P4 | DONE | 2 / 1babcf9bb 2026-09-21 / 2 | - | none found | n/a |
| A94 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt / Nebenbefund A79 (Hatari-Fal… | P2 | DONE | 4 / 7a250028e 2026-09-23 / 3 | - | none found | n/a |
| A95 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt / Nebenbefunde A79 (Referenzv… | P2 | DONE | 4 / 4eb910058 2026-09-23 / 4 | - | none found | n/a |
| A96 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt / Robustheit — Nebenbefund A8… | P2 | DONE | 4 / f6f902027 2026-10-16 / 3 | - | none found | n/a |
| A97 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Belegter Defekt / Speichermodell — Nebenbefun… | P2 | DONE | 3 / f2af53ea8 2026-10-05 / 3 | - | none found | n/a |
| A98 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P4 · Typ: Konformitaet, in Pixeln unbeobachtbar — Rest aus A84 | P4 | DONE | 2 / 1babcf9bb 2026-09-21 / 2 | - | none found | n/a |
| A99 | OK | @L3 ERLEDIGT (2026-09-20) — Pi-Sichtabnahme offen · Prioritaet: P2 (hochgestuft 2026-09-20, sichtbare Folge von A85) · … | P2 | DONE | 2 / 92299c0f1 2026-09-21 / 2 | - | none found | n/a |
| ACC01 | OK | @L3 ERLEDIGT (2026-09-24) · Prioritaet: P2 · Typ: Quellenanalyse (Researcher) — Voraussetzung fuer ACC02–ACC05 | P2 | DONE | 2 / a0eeebb3c 2026-09-24 / 0 | - | 2026-09-22: `.ACC`-Unterstuetzung fehlt bei uns fast vollstaendig (Anwender 2026-09-22). | n/a |
| ACC02 | OK | @L3 ERLEDIGT (2026-09-25) · Prioritaet: P2 · Typ: Funktionsluecke | P2 | DONE | 5 / 337f3553c 2026-09-29 / 4 | - | none found | n/a |
| ACC03 | OK | @L3 ERLEDIGT (2026-09-25) · Prioritaet: P2 · Typ: Funktionsluecke / Kern | P2 | DONE | 7 / 4e668641a 2026-10-17 / 5 | - | none found | n/a |
| ACC04 | OK | @L3 ERLEDIGT (2026-09-25) · Prioritaet: P2 · Typ: Funktionsluecke | P2 | DONE | 1 / 96f178544 2026-09-24 / 1 | - | none found | n/a |
| ACC05 | OK | @L3 ERLEDIGT (2026-09-25) · Prioritaet: P2 · Typ: Entscheidung + Werkzeugkette | P2 | DONE | 1 / 96f178544 2026-09-24 / 1 | - | none found | n/a |
| ACC07 | OK | @L3 ERLEDIGT 2026-10-07 (QEMU, `05b3376d`: Titelkopien auf gemeinsamer read-only Seite, ADR-023; TeraDesk und QED zeige… | P3 | - | 2 / d40f4f16d 2026-10-21 / 0 | L163:ERLoQ:done-no-receipt | none found | n/a |
| ACC08 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Pruefung / Funktionsluecke — Nebenbefund ACC04 | P3 | - | 1 / 7fecacd88 2026-09-24 / 0 | - | none found | n/a |
| ACC09 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Werkzeug (Accessory) — Anwenderwunsch 2026-09-25 | P3 | - | 1 / aa405b249 2026-09-24 / 0 | - | 2026-09-25: Status: OFFEN · Prioritaet: P3 · Typ: Werkzeug (Accessory) — Anwenderwunsch 2026-09-25 | n/a |
| ACC10 | OK | @L3 BEHOBEN (2026-09-26) · Prioritaet: P2 · Typ: Belegter Defekt — aus PORT05 herausgefallen | P2 | DONE | 5 / 39d8c53e2 2026-09-29 / 2 | - | none found | n/a |
| ARCH01 | OK | @L3 ERLEDIGT (2026-09-24) · Prioritaet: P1 · Typ: Belegter Defekt / Architektur — gefunden bei DESK19 | P1 | DONE | 3 / 8e1871a47 2026-09-24 / 1 | - | none found | n/a |
| ARCH02 | OK | @L3 ERLEDIGT (2026-10-17, QEMU; Pi abgenommen 2026-10-17, `docs/HW-TEST-LOG.md`) · Prioritaet: P2 · Typ: Belegter Defek… | P2 | DONE | 8 / 22325b9a6 2026-10-22 / 3 | - | none found | n/a |
| ARCH04 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Robustheit — uebertragen aus dem alten GitHub-Issue #10 (2026-07-29, Repository NewTO… | P3 | - | 1 / 2b559b3ec 2026-10-19 / 0 | L178:P34:open | none found | n/a |
| ARCH05 | OK | @L3 ERLEDIGT 2026-10-28 — Nachtrag ADR-012 in `docs/MEMORY.md`; Umsetzung ARCH06<ARCH06>, SDK03<SDK03>, TOOL18<TOOL18> … | P2 | - | 4 / a9f4d564c 2026-10-28 / 0 | L128:WQ/DevReihe:none | 2026-10-28: ## Entscheidungen des Anwenders 2026-10-28 | n/a |
| ARCH06 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<ARCH06-STATUS-DONE>); `_VDO`-Entscheidung beim Anwender · Prioritaet: P2 · Typ: Devel… | P2 | DONE | 6 / 86f39d28e 2026-10-29 / 1 | L44:CS:none | 2026-10-29: ## Anwenderentscheidung 2026-10-29 (`_VDO`) | n/a |
| BALL01 | OK | @L3 GESCHLOSSEN 2026-10-17 — Ball-Demo wird komplett ueberarbeitet (Anwender) · Prioritaet: P3 · Typ: Darstellungsfehle… | P3 | - | 2 / 8658d005e 2026-10-17 / 0 | - | 2026-10-08: Status: GESCHLOSSEN 2026-10-17 — Ball-Demo wird komplett ueberarbeitet (Anwender) · Prioritaet: P3 · T | n/a |
| BIOS01 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<BIOS01-STATUS-DONE>, S2: BIOS01-S2-STATUS-DONE.MD<BIOS01-S2-STATUS-DONE>); 004-Laufze… | P1 | BIOS01-S2:DONE, DONE | 14 / 6ddd4496a 2026-10-29 / 4 | L44:CS:none, L44:CS:none | 2026-10-29: Status: ERLEDIGT 2026-10-29 (Quittung<BIOS01-STATUS-DONE>, S2: [BIOS01-S2-STATUS-DONE.MD](BIOS01-S2-S | n/a |
| BIOS02 | OK | @L3 ERLEDIGT 2026-10-28 (Spec freigegeben) · Prioritaet: P1 · Typ: Researcher — Anwenderentscheidung 2026-10-28 | P1 | - | 3 / 40efc473b 2026-10-28 / 0 | - | 2026-10-28: ## Entscheidungen des Anwenders 2026-10-28 | n/a |
| BIOS03 | OK | @L3 ERLEDIGT 2026-10-28 — ADR-031 in `docs/MEMORY.md` (Vorschlag `docs/analysis/os-hooks-adr-proposal-2026-10-28.md`); … | P1 | - | 5 / 99ad54a78 2026-10-28 / 0 | - | 2026-10-28: ## Entscheidungen des Anwenders 2026-10-28 (Q1–Q6) | n/a |
| BIOS04 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<BIOS04-STATUS-DONE>) · Prioritaet: P1 · Typ: Researcher (FS11-Entscheidung neu treffe… | P1 | DONE | 3 / 4039f1d64 2026-10-29 / 1 | L44:CS:none, L124:WQ/DevReihe:done | 2026-10-29: ## Entscheidung (Anwender, 2026-10-29) | n/a |
| BIOS06 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<BIOS06-STATUS-DONE>); O3 gemessen (EmuTOS: Busfehler + Panic), stiller Erfolg bleibt … | P1 | DONE | 5 / f77ba8bac 2026-10-29 / 1 | L44:CS:none | 2026-10-28: ## Entscheidungen des Anwenders 2026-10-28 | n/a |
| BIOS07 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<BIOS07-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (004 ggf. Researcher, falls al… | P2 | DONE | 6 / db112d1eb 2026-10-31 / 1 | L44:CS:none, L112:WQ/EbGeparkt:done, L126:WQ/DevReihe:done | 2026-10-28: ## Entscheidungen des Anwenders 2026-10-28 (BIOS12) | n/a |
| BIOS08 | OK | @L3 ERLEDIGT 2026-10-30 (Quittung<BIOS08-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (Profil folgt aus XBIOS05 Opti… | P2 | DONE | 2 / 5b059a0a0 2026-10-30 / 1 | L44:CS:none, L127:WQ/DevReihe:none, L127:WQ/DevReihe:done | 2026-10-28: Status: ERLEDIGT 2026-10-30 (Quittung<BIOS08-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (Pro | n/a |
| BIOS09 | OK | @L3 UMGESETZT 2026-10-30, Pi-Messung im spaeteren Testszenario (MEM12-Schritt entfernt, Anwender 2026-10-30) · Priorita… | P2 | - | 3 / 5ad25f644 2026-10-30 / 1 | L44:CS:none, L121:WQ:none, L127:WQ/DevReihe:none(range), L134:WQ:done | 2026-10-30: ## Abnahme (Anwender, 2026-10-30) | n/a |
| BIOS10 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<BIOS10-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer — Anwenderentscheidung 2026-10… | P2 | DONE | 5 / db112d1eb 2026-10-31 / 1 | L127:WQ/DevReihe:none(range), L127:WQ/DevReihe:done | 2026-10-28: ## Entscheidung des Anwenders 2026-10-28 (BIOS12) | n/a |
| BIOS11 | OK | @L3 ERLEDIGT 2026-10-29 mit XBIOS01<XBIOS01> (`a32921d5`), vom Anwender abgenommen · Prioritaet: P2 · Typ: Developer — … | P2 | - | 3 / 193197bed 2026-10-29 / 1 | L127:WQ/DevReihe:none(range) | 2026-10-28: ## Anwenderentscheidung 2026-10-28 | n/a |
| BIOS12 | OK | @L3 ERLEDIGT 2026-10-31 (Quittung<BIOS12-STATUS-DONE>) · Prioritaet: P3 · Typ: Researcher — Anwenderentscheidung 2026-1… | P3 | DONE | 5 / db112d1eb 2026-10-31 / 1 | L118:WQ:done, L127:WQ/DevReihe:none(range) | 2026-10-28: Status: ERLEDIGT 2026-10-31 (Quittung<BIOS12-STATUS-DONE>) · Prioritaet: P3 · Typ: Researcher — A | n/a |
| BIOS13 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Doku-/Ledger-Pflege; kein Programm, kein offener Fix) · Pri… | P3 | - | 1 / c173cddcf 2026-10-28 / 0 | L94:WQ/EbGeparkt:parked, L127:WQ/DevReihe:none(range) | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Doku-/Ledger-Pflege; kein Programm, kein o | yes (status line): (Anwender, Vor-Start-Pruefung: reine Doku-/Ledger-Pflege; kein Programm, kein offener Fix) |
| BIOS14 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Werkzeugluecke; blockiert keinen offenen Fix) · Prioritaet: P3 · … | P3 | - | 0 / - / 0 | L95:WQ/EbGeparkt:parked, L127:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Werkzeugluecke; blockiert keinen offenen Fix) ·  | yes (status line): (Anwender, Vor-Start-Pruefung: Werkzeugluecke; blockiert keinen offenen Fix) |
| BIOS15 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<BIOS15-STATUS-DONE>), vom Anwender abgenommen · Prioritaet: P1 · Typ: Developer — Anw… | P1 | DONE | 9 / 46947fab1 2026-10-30 / 3 | L44:CS:none | 2026-10-29: - Bestaetigt (Anwender 2026-10-29, Nachtrag zu ADR-031): zusaetzliche Erkennungsstelle „nach jedem | n/a |
| BOOT01 | OK | @L3 OFFEN (Punkt 1 zentrierte Box erledigt `2ace2639`, `measurements/measurement-2026-10-09-initinfo/`; Punkte 2–5 offe… | P3 | - | 3 / d40f4f16d 2026-10-21 / 0 | - | 2026-09-23: Entscheid des Anwenders (2026-09-23): eine zentrierte Box. Statt den | n/a |
| CON01 | OK | @L3 ERLEDIGT 2026-10-18 (QEMU, `5d2a3812`; Pi offen) · Prioritaet: P3 · Typ: Konformitaet — Anwenderentscheidung 2026-1… | P3 | - | 5 / 86eac1a48 2026-10-18 / 1 | L164:ERLoQ:done-no-receipt | 2026-10-18: ## Anwenderentscheidung (2026-10-18) | n/a |
| CON02 | OK | @L3 ERLEDIGT 2026-10-18 (mit PROC04 Punkt 1, `1a306120`) · Prioritaet: P3 · Typ: Konformitaet — Nebenfund LIBC-FSTAT (2… | P3 | - | 4 / 765425fb9 2026-10-18 / 2 | L164:ERLoQ:done-no-receipt | none found | n/a |
| CON03 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Befund 2026-10-18 (Frage des Anwenders nach einem Textbetrach… | P3 | - | 4 / 1d7a4c0b8 2026-10-22 / 0 | L178:P34:open | 2026-10-22: ## Spec und Anwenderentscheide 2026-10-22 | n/a |
| CON04 | OK | @L3 ERLEDIGT 2026-10-23 (implementiert 2026-10-22, QEMU raspi4b + Hatari-Falcon-Gegenprobe; Tester-Abnahme PASS `5ca5fa… | P2 | DONE | 11 / cd60428b9 2026-10-23 / 1 | L32:CS:none, L236:PH:none, L236:PH:none | 2026-10-22: ## Entscheidungen (Anwender, 2026-10-22) | n/a |
| CON05 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Nebenbefund CON04-Abnahme (2026-10-22) | P3 | - | 1 / f9fdd32c3 2026-10-22 / 0 | - | none found | n/a |
| DESK01 | OK | @L3 ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Funktionsluecke desk/ | P2 | DONE | 1 / 0ffb333ae 2026-09-23 / 1 | - | none found | n/a |
| DESK02 | OK | @L3 ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Funktionsluecke desk/ | P2 | DONE | 2 / 43e257776 2026-09-23 / 2 | - | none found | n/a |
| DESK03 | OK | @L3 ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Funktionsluecke desk/ | P3 | DONE | 5 / 4dcff4dfb 2026-09-25 / 2 | - | none found | n/a |
| DESK04 | OK | @L3 ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Funktionsluecke desk/ | P3 | DONE | 4 / 7dd743e1f 2026-09-23 / 2 | - | none found | n/a |
| DESK05 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P3 · Typ: Funktionsluecke desk/ | P3 | DONE | 5 / 4dcff4dfb 2026-09-25 / 1 | - | none found | n/a |
| DESK06 | OK | @L3 ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Funktionsluecke desk/ | P3 | DONE | 8 / c0906fe4f 2026-09-24 / 4 | - | none found | n/a |
| DESK07 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Pruefung / ggf. gegenstandslos | P4 | DONE | 1 / 5336331a3 2026-09-23 / 0 | - | none found | n/a |
| DESK08 | OK | @L3 ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Strukturluecke desk/ (G7 der Lueckenliste) | P2 | DONE | 2 / 0ffb333ae 2026-09-23 / 2 | - | none found | n/a |
| DESK09 | OK | @L3 ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen · Prioritaet: P2 · Typ: Strukturluecke desk/ (G2 der Lueckenliste) | P2 | DONE | 3 / c0906fe4f 2026-09-24 / 3 | - | none found | n/a |
| DESK10 | OK | @L3 ERLEDIGT (2026-09-23) — Pi-Sichtabnahme offen · Prioritaet: P3 · Typ: Strukturluecke desk/ (G6 der Lueckenliste) | P3 | DONE | 3 / 135bcbce7 2026-09-26 / 2 | - | none found | n/a |
| DESK11 | OK | @L3 ERLEDIGT (2026-09-24) — Pi-Sichtabnahme offen · Prioritaet: P4 · Typ: Konformitaet desk/ — Nebenbefund DESK10 | P4 | DONE | 2 / 0ffb333ae 2026-09-23 / 2 | - | none found | n/a |
| DESK12 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P3 · Typ: Werkzeugkette — Nebenbefund DESK01 | P3 | DONE | 2 / 8a32d0302 2026-09-23 / 2 | - | none found | n/a |
| DESK13 | OK | @L3 ERLEDIGT — HW-Abnahme offen · Prioritaet: P3 · Typ: Funktionsluecken — Nebenbefunde DESK02 | P3 | DONE | 6 / f497ec8d7 2026-09-23 / 3 | - | none found | n/a |
| DESK14 | OK | @L3 ERLEDIGT (Punkte 1-3; Punkt 4 zurueckgestellt, siehe unten) · | none | DONE | 9 / 7308e1c1f 2026-09-26 / 4 | - | none found | n/a |
| DESK15 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P3 · Typ: Funktionsluecke — gemeldet in DESK03 | P3 | DONE | 5 / a0eeebb3c 2026-09-24 / 2 | - | none found | n/a |
| DESK16 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Ressourcenluecke — gemeldet in DESK03 | P4 | DONE | 3 / f68517cd8 2026-09-23 / 1 | - | none found | n/a |
| DESK17 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Konformitaet — Nebenbefunde DESK03 | P4 | DONE | 3 / f68517cd8 2026-09-23 / 1 | - | none found | n/a |
| DESK18 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P2 · Typ: Kompatibilitaetsdefekt — Nebenbefund DESK05 | P2 | DONE | 7 / 89f012b21 2026-09-24 / 2 | - | none found | n/a |
| DESK19 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P3 · Typ: Funktionsluecke — Nebenbefund DESK05 | P3 | DONE | 8 / 29b4e6087 2026-09-24 / 3 | - | none found | n/a |
| DESK20 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Funktionsluecke — Nebenbefund DESK05 | P4 | DONE | 4 / 920c49549 2026-09-24 / 1 | - | none found | n/a |
| DESK21 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Konformitaet — Nebenbefund DESK05 | P4 | DONE | 4 / 5a3168a49 2026-09-23 / 1 | - | none found | n/a |
| DESK22 | OK | @L3 ERLEDIGT (2026-09-24) · Prioritaet: P3 · Typ: Defekt — Nebenbefund DESK06 | P3 | DONE | 3 / c35b5e9cf 2026-09-24 / 1 | - | none found | n/a |
| DESK23 | OK | @L3 ZURUECKGEZOGEN (2026-09-24, EmuTOS-konform) · Prioritaet: P4 · Typ: Defekt — Nebenbefund DESK06 | P4 | DONE | 3 / c35b5e9cf 2026-09-24 / 1 | - | none found | n/a |
| DESK24 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P3 · Typ: Funktionsluecke — Nebenbefund DESK13 | P3 | DONE | 4 / 5a3168a49 2026-09-23 / 1 | - | none found | n/a |
| DESK25 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Funktionsluecke — Nebenbefund DESK13 | P4 | DONE | 4 / 5a3168a49 2026-09-23 / 1 | - | none found | n/a |
| DESK26 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Defekt — Nebenbefund DESK13 | P4 | DONE | 4 / 5a3168a49 2026-09-23 / 1 | - | none found | n/a |
| DESK27 | OK | @L3 ERLEDIGT (2026-09-23) · Prioritaet: P4 · Typ: Dokumentationsfehler — Nebenbefund DESK13 | P4 | DONE | 2 / f497ec8d7 2026-09-23 / 0 | - | none found | n/a |
| DESK28 | OK | @L3 ERLEDIGT (2026-09-24, teils zurueckgezogen) · Prioritaet: P2 · Typ: Kompatibilitaetsdefekt — Nebenbefund DESK18 | P2 | DONE | 3 / c35b5e9cf 2026-09-24 / 1 | - | none found | n/a |
| DESK29 | OK | @L3 ZURUECKGEZOGEN (2026-09-24, EmuTOS-konform) · Prioritaet: P3 · Typ: Verhaltensabweichung — Nebenbefund DESK18 | P3 | DONE | 3 / c35b5e9cf 2026-09-24 / 1 | - | none found | n/a |
| DESK30 | OK | @L3 ERLEDIGT · Prioritaet: P2 · Typ: Funktionsluecke — gemeldet in DESK15 | P2 | DONE | 8 / 29b4e6087 2026-09-24 / 2 | - | none found | n/a |
| DESK31 | OK | @L3 ERLEDIGT · Prioritaet: P3 · Typ: Funktionsluecken — Nebenbefunde DESK24/DESK25/DESK26 | P3 | DONE | 2 / aa76e41f3 2026-09-25 / 1 | - | none found | n/a |
| DESK32 | OK | @L3 ERLEDIGT · Prioritaet: P4 · Typ: Kosmetik — Nebenbefund DESK21 | P4 | DONE | 2 / aa76e41f3 2026-09-25 / 1 | - | none found | n/a |
| DESK33 | OK | @L3 ERLEDIGT · Prioritaet: P3 · Typ: Funktionsluecke — Nebenbefund DESK20 | P3 | DONE | 2 / aa76e41f3 2026-09-25 / 1 | - | none found | n/a |
| DESK36 | OK | @L3 ERLEDIGT · Prioritaet: P2 · Typ: Belegter Defekt — Pi-Runde 2026-09-24 | P2 | DONE | 4 / af0bc92d3 2026-09-24 / 1 | - | none found | n/a |
| DESK37 | OK | @L3 ERLEDIGT · Prioritaet: P2 · Typ: Belegter Defekt — Pi-Runde + Hatari-Gegenprobe 2026-09-24 | P2 | DONE | 4 / 7308e1c1f 2026-09-26 / 2 | - | none found | n/a |
| DESK38 | OK | @L3 ERLEDIGT · Prioritaet: P3 · Typ: Funktionsluecke — Nebenbefund DESK30 | P3 | DONE | 5 / f7c61f863 2026-09-25 / 2 | - | none found | n/a |
| DESK39 | OK | @L3 ERLEDIGT 2026-09-25 (`f7c61f86`, ADFFINFO-Neubau: `STRENAME`-Retry-Schleife und `dos_chmod`-Rueckschreiben in `inf_… | P3 | - | 2 / d0bf0bf57 2026-10-22 / 0 | L164:ERLoQ:done-no-receipt | none found | n/a |
| DESK40 | OK | @L3 OFFEN · Prioritaet: P4 · Typ: Konformitaet — Nebenbefunde DESK31/DESK38 | P4 | - | 1 / 169dab47e 2026-09-25 / 0 | L121:WQ:none | none found | n/a |
| DESK41 | OK | @L3 ERLEDIGT (HW-Abnahme aus TeraDesk 2026-10-01, Statusdatei<DESK41-STATUS-DONE>) — implementiert 2026-09-29 · Priorit… | P2 | DONE | 5 / fbecd68a1 2026-10-01 / 1 | L29:CS:none | none found | n/a |
| DESK43 | OK | @L3 ERLEDIGT 2026-10-16 (U1–U10; U10 `f6f90202`, Tester-Lauf `87436d52` PASS, ADR-026; Pi 2026-10-16 gebootet; offen nu… | P1 | DESK43-PLAN:DONE, DESK43-U1:DONE, DESK43-U2:DONE, DESK43-U3:DONE, DESK43-U4:DONE, DESK43-U5a:DONE, DESK43-U5b:DONE, DESK43-U6a:DONE, DESK43-U6b:DONE,… | 48 / d40f4f16d 2026-10-21 / 7 | L18:CS:done, L121:WQ:none, L152:P1u:impl-open, L167:ERLoQ:done-no-receipt, L240:PH:none | 2026-10-16: - Erledigt nachgezogen (2026-10-16, Anwender ja): ADR-026 in | n/a |
| DESK44 | OK | @L3 BEHOBEN — HW-Gegenprobe offen (2026-09-26) · Prioritaet: P3 · Typ: Belegter Widerspruch im Baum — aus DESK43 U3, An… | P3 | DONE | 4 / 4b3d73828 2026-09-30 / 2 | - | 2026-09-26: ## Die Entscheidung (Anwender 2026-09-26) | n/a |
| DESK45 | OK | @L3 ERLEDIGT als Form A (DESK43 U8a `09eaca14`: Feature `selftest-desk`, ein Kommando, Gegenprobe Karte unberuehrt; Tes… | P3 | - | 3 / d40f4f16d 2026-10-21 / 1 | L164:ERLoQ:done-no-receipt | none found | n/a |
| DESK46 | OK | @L3 BEHOBEN — auf echter Pi 4 abgenommen (2026-09-28) · Prioritaet: P1 · Typ: Belegter Defekt — Pi-Runde 2026-09-26 | P1 | DONE | 4 / ab23a69d8 2026-09-28 / 1 | - | 2026-09-26: * `console_pause_for_key` ist entfallen (Anwenderentscheidung 2026-09-26). Das | n/a |
| DESK47 | OK | @L3 ERLEDIGT (2026-09-30, auf der Pi abgenommen) · Prioritaet: P2 · Typ: Fehlende EmuTOS-Funktion — Anwenderbefund Pi-R… | P2 | - | 3 / 4b3d73828 2026-09-30 / 1 | L28:CS:none, L164:ERLoQ:done-no-receipt | 2026-09-28: Status: ERLEDIGT (2026-09-30, auf der Pi abgenommen) · Prioritaet: P2 · Typ: Fehlende EmuTOS-Funktion  | n/a |
| DESK48 | OK | @L3 ERLEDIGT 2026-10-17 (QEMU, `f2f127a7`; Pi abgenommen 2026-10-17, `docs/HW-TEST-LOG.md`) — Quittung<DESK48-STATUS-DO… | P2 | DONE | 6 / 22325b9a6 2026-10-22 / 1 | - | none found | n/a |
| DESK50 | OK | @L3 ERLEDIGT — Pi-Abnahme 2026-10-04 (Statusdatei<DESK50-STATUS-DONE>) · Prioritaet: P2 · Typ: Fehlende EmuTOS-Funktion… | P2 | DONE | 5 / 98d935a1c 2026-10-04 / 2 | L29:CS:none | none found | n/a |
| DESK51 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Fehlende EmuTOS-Funktion — Nebenfund aus DESK50<DESK50> | P3 | - | 0 / - / 0 | L176:P34:open | none found | n/a |
| DESK52 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Fehlende EmuTOS-Funktion — Nebenfund aus DESK50<DESK50> | P3 | - | 0 / - / 0 | L176:P34:open(range) | none found | n/a |
| DESK53 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenfund aus DESK50<DESK50> | P3 | - | 0 / - / 0 | L176:P34:open(range) | none found | n/a |
| DESK54 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenfund aus DESK50<DESK50> | P3 | - | 0 / - / 0 | L176:P34:open(range) | none found | n/a |
| DESK55 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenfund aus DESK50<DESK50> | P3 | - | 0 / - / 0 | L176:P34:open | none found | n/a |
| DESK56 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<DESK56-STATUS-DONE>) · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenfund aus DE… | P3 | DONE | 1 / 526ad8ac0 2026-10-17 / 0 | - | none found | n/a |
| DESK58 | OK | @L3 ERLEDIGT 2026-10-16 (Quittung<DESK58-STATUS-DONE>) · Prioritaet: P3 · Typ: Konformitaet — gefunden in der DESK43-Pr… | P3 | DONE | 4 / a217b1aa0 2026-10-16 / 2 | - | none found | n/a |
| DESK59 | OK | @L3 ERLEDIGT 2026-10-16 (Quittung<DESK59-STATUS-DONE>) · Prioritaet: P3 · Typ: Konformitaet — Nebenfund DESK43 U5b (202… | P3 | DONE | 5 / 3b8098415 2026-10-16 / 2 | - | none found | n/a |
| DESK60 | OK | @L3 ERLEDIGT 2026-10-16 (Quittung<DESK60-STATUS-DONE>) · Prioritaet: P3 · Typ: Diagnose/Log — Nebenfund DESK43-Abnahme … | P3 | DONE | 3 / e52bc0d38 2026-10-16 / 1 | - | none found | n/a |
| DESK62 | OK | @L3 ERLEDIGT 2026-10-16 (Quittung<DESK62-STATUS-DONE>) · Prioritaet: P2 · Typ: Fehler — Befund des Anwenders auf echter… | P2 | DONE | 9 / 526ad8ac0 2026-10-17 / 1 | - | 2026-10-16: (Korrigiert 2026-10-16: der Anwender hat auf der Pi bestaetigt, dass QED | n/a |
| DESK63 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: fehlender Zweig — Nebenfund DESK62 (2026-10-16) | P3 | - | 2 / 0c6108540 2026-10-16 / 0 | L176:P34:open | none found | n/a |
| DESK64 | OK | @L3 ERLEDIGT 2026-10-17, auf echter Pi 4 abgenommen · Prioritaet: P3 · Typ: Konformitaetsfrage — Befund des Anwenders a… | P3 | DONE | 7 / 3930468be 2026-10-17 / 1 | - | 2026-10-17: ## Anwenderentscheidungen zu den Restpunkten (2026-10-17) | n/a |
| DESK66 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenfund DESK64, Anwender 2026-10-17: Weg (a) | P3 | - | 1 / 1e20f80cc 2026-10-17 / 0 | L176:P34:open | 2026-10-17: Status: OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenfund DESK64, Anwender 2026-10-17: W | n/a |
| DESK67 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Konformitaet / Laufzeit — Rest aus DESK48<DESK48-STATUS-DONE> (2026-10-17) | P3 | - | 0 / - / 0 | L176:P34:open | 2026-10-17: verarbeiten“ (Anwender 2026-10-17, `docs/COMPATIBILITY.md`). | n/a |
| DESK68 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS (ungeprueft) — Nebenbefund aus A111<A111> (2026-10-25) | P3 | - | 1 / ad9b023da 2026-10-25 / 0 | - | none found | n/a |
| DESK69 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — ADR-030 Q3 (Anwender 2026-10-26: „NEWDESK nimmt die VDI-Werte… | P3 | - | 2 / b63a58029 2026-10-26 / 0 | L176:P34:open | 2026-10-26: Status: OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — ADR-030 Q3 (Anwender 2026-10-26: „NEWDES | n/a |
| DESK70 | OK | @L3 ERLEDIGT 2026-11-07 (Tester PASS, Pi ok, #12 beantwortet und geschlossen) · Prioritaet: P3 · Typ: Frage, erst EmuTO… | P3 | DONE | 9 / 72624def3 2026-11-07 / 2 | L121:WQ:none | 2026-11-07: Pi des Anwenders mit dem 0.7.1-ZIP bestanden: `measurements/measurement-2026-11-07-rel071-pi/RUNINFO.TXT`. Ver | n/a |
| DESK71 | OK | @L3 OFFEN (2026-11-06) · Prioritaet: P3 · Typ: Abweichung, aus V32<V32> | P3 | - | 1 / cc5f476ba 2026-11-06 / 0 | L121:WQ:none | none found | n/a |
| DOOM01 | OK | @L3 ZURUECKGESTELLT 2026-11-02 (Anwender: erst PROC08<PROC08>; der naechste Schritt unten bleibt der Wiedereinstieg; vo… | P2 | - | 15 / 468b3b531 2026-11-02 / 0 | L118:WQ:none, L119:WQ:none, L120:WQ:none, L139:WQ/Zurueckgest:parked, L139:WQ/Zurueckgest:parked, L141:P1:open | 2026-11-02: Status: ZURUECKGESTELLT 2026-11-02 (Anwender: erst PROC08<PROC08>; der naechste Schritt unten bleibt  | yes (status line): (Anwender: erst PROC08<PROC08>; der naechste Schritt unten bleibt der Wiedereinstieg; vorh… |
| EMU01 | OK | @L3 IDEE (2026-11-07, Anwender: Line-A-Ueberlegungen festhalten) · Prioritaet: P4 · Typ: Richtungsnotiz — kein Auftrag,… | P4 | - | 1 / 3380004fb 2026-11-07 / 0 | L121:WQ:none | 2026-11-07: Fuer den Emulator gilt (Anwender 2026-11-07, im Gespraech zu V18 Teil 3 K26): | yes (status line): , Anwender: Line-A-Ueberlegungen festhalten) |
| FS01 | OK | @L3 ERLEDIGT · Prioritaet: P2 · Typ: Dateisystemdefekt — Nebenbefund DESK24/DESK25 | P2 | DONE | 4 / 269b747bd 2026-09-24 / 1 | - | none found | n/a |
| FS02 | OK | @L3 ERLEDIGT 2026-10-19 (QEMU/Host, `5faf62de` mit FS18: `Frename` erase VFAT-Kette des alten Namens, Test `fs18_frenam… | P3 | - | 2 / d40f4f16d 2026-10-21 / 0 | L164:ERLoQ:done-no-receipt | none found | n/a |
| FS03 | OK | @L3 ZURUECKGEZOGEN (2026-09-24, kein Defekt) · Prioritaet: P3 · Typ: Frage an die Quelle — Beobachtung beim Kartenaufra… | P3 | - | 2 / f2f55e2e1 2026-09-24 / 0 | - | none found | n/a |
| FS04 | OK | @L3 BEHOBEN (2026-09-25, auf der Pi gemessen) · Prioritaet: P2 · Typ: Belegter Defekt / Laufzeit — Pi-Runde 2026-09-25 | P2 | FS04-EMMC:DONE, FS04-FIX:DONE, DONE, UNFINISHED | 20 / 0cd8470f7 2026-09-28 / 2 | L27:CS:none | 2026-09-25: 4. Der EMMC-Treiber selbst. Anwender 2026-09-25: *„im uebrigen kann es | n/a |
| FS05 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<FS05-STATUS-DONE>) · Prioritaet: P3 · Typ: Belegter Defekt / Laufzeit — aus der FS04-… | P3 | DONE | 2 / 526ad8ac0 2026-10-17 / 0 | - | none found | n/a |
| FS06 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<FS06-STATUS-DONE>) · Prioritaet: P2 · Typ: Belegter Defekt / Dateisystem — aus DESK42… | P2 | DONE | 2 / 526ad8ac0 2026-10-17 / 1 | - | none found | n/a |
| FS08 | OK | @L3 ERLEDIGT 2026-10-22 (Teile 1–4 `e8fe4137`, `9fca9876`, `1b5409d1`/`f89401ce`, `31a7fbce`; Pi-Messung 2026-10-07: BI… | P1 | FS08-CPUCLK:DONE, FS08-EMMC:DONE, FS08-FAT:DONE, FS08-STDIO:DONE | 19 / 99d35b242 2026-10-22 / 6 | L167:ERLoQ:done-no-receipt | 2026-10-22: Status: ERLEDIGT 2026-10-22 (Teile 1–4 `e8fe4137`, `9fca9876`, `1b5409d1`/`f89401ce`, `31a7fbce`; Pi-Messu | n/a |
| FS10 | OK | @L3 ERLEDIGT 2026-10-13 (Quittung<FS10-STATUS-DONE>) · Prioritaet: P2 · Typ: Konformitaet — Nebenfund ARCH03 (Post-E9 P… | P2 | DONE | 7 / cbad7659b 2026-10-16 / 3 | L234:PH:none | none found | n/a |
| FS11 | OK | @L3 ERLEDIGT 2026-10-13 (Quittung<FS11-STATUS-DONE>, AUX-Nachtrag<FS11-AUX-STATUS-DONE>; Pi offen) · Prioritaet: P3 · T… | P3 | FS11-AUX:DONE, DONE | 13 / 6fde3669f 2026-10-18 / 3 | L234:PH:none | none found | n/a |
| FS13 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<FS13-STATUS-DONE>) · Prioritaet: P3 · Typ: Konformitaet — Nebenfund LIBC-FSTAT (2026-… | P3 | DONE | 3 / 526ad8ac0 2026-10-17 / 1 | - | none found | n/a |
| FS14 | OK | @L3 ERLEDIGT 2026-10-19 (QEMU, `ba40722f`; Pi offen) · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Rest aus FS06<FS06… | P3 | - | 3 / 54977b5ad 2026-10-19 / 1 | L164:ERLoQ:done-no-receipt | none found | n/a |
| FS15 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: fehlende Funktion — Nebenfund PROC04-Re-Validierung (2026-10-18) | P3 | - | 1 / 097b7f99f 2026-10-18 / 0 | L121:WQ:none, L177:P34:open | none found | n/a |
| FS16 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Laufzeit / Abweichung von EmuTOS — Befund PROC04 Punkt 1 (2026-10-18), Anwender: als … | P3 | - | 1 / aa0443d92 2026-10-18 / 0 | L121:WQ:none(range), L177:P34:open | 2026-10-18: Status: OFFEN · Prioritaet: P3 · Typ: Laufzeit / Abweichung von EmuTOS — Befund PROC04 Punkt 1 (2026-1 | n/a |
| FS17 | OK | @L3 OFFEN (Punkt 3 erledigt → FS18<FS18> `5faf62de`; Punkte 1, 2, 4, 5, 6 offen, Review 2026-10-19; Punkt 7 ungeprueft)… | P3 | - | 3 / d40f4f16d 2026-10-21 / 0 | L121:WQ:none, L177:P34:open | none found | n/a |
| FS18 | OK | @L3 ERLEDIGT 2026-10-19 (QEMU, `ba40722f`; Pi offen) · Prioritaet: P2 · Typ: Fehler (Datenverlust) — aus FS17 Punkt 3, … | P2 | - | 4 / 54977b5ad 2026-10-19 / 1 | L164:ERLoQ:done-no-receipt, L235:PH:none | none found | n/a |
| FS19 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: nur Programme mit DTA in unabgebildetem Speicher) · Prioritaet: P… | P3 | - | 3 / 9ef372a56 2026-10-31 / 0 | L86:WQ/Geparkt:parked, L88:WQ/Geparkt:parked, L179:P34:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: nur Programme mit DTA in unabgebildetem Speicher | yes (status line): (Anwender, Vor-Start-Pruefung: nur Programme mit DTA in unabgebildetem Speicher) |
| FS20 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<FS20-STATUS-DONE>); Option B als FS22<FS22> · Prioritaet: P1 · Typ: Developer — Fund … | P1 | DONE | 8 / 2e183f025 2026-10-29 / 5 | L44:CS:none | 2026-10-29: ## Anwenderentscheidung 2026-10-29 | n/a |
| FS21 | OK | @L3 ERLEDIGT 2026-10-31 (Quittung<FS21-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer — Fund aus FS20<FS20> (2026-10-2… | P2 | DONE | 4 / 43b5789a1 2026-10-31 / 2 | L44:CS:none, L118:WQ:none | 2026-10-31: ## Anwenderentscheidung 2026-10-31 | n/a |
| FS22 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<FS22-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer — Anwenderentscheidung 2026-10-2… | P2 | DONE | 7 / 2e183f025 2026-10-29 / 3 | L44:CS:none | 2026-10-29: - Anwenderentscheidung 2026-10-29: Option B. | n/a |
| FS24 | OK | @L3 ACCEPTED-Abweichung 2026-10-30 (ADR-033, Anwender; Option B gemessen und verworfen, Residuum V2/V3) · Prioritaet: P… | P2 | - | 8 / 799b7dfd9 2026-10-30 / 1 | L125:WQ/DevReihe:accepted | 2026-10-30: ## Entscheidung (Anwender, 2026-10-30) | n/a |
| FS25 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Frage nur ans Orakel) · Prioritaet: P3 · Typ: Auditor/Developer (… | P3 | - | 1 / 9ef372a56 2026-10-31 / 0 | L92:WQ/Geparkt:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Frage nur ans Orakel) · Prioritaet: P3 · Typ | yes (status line): (Anwender, Vor-Start-Pruefung: Frage nur ans Orakel) |
| FS26 | OK | @L3 OFFEN (QEMU-Messung 2026-10-31 erledigt; Pi-Messung T1–T3 ins Testszenario, Fix-Entscheidung danach) · Prioritaet: … | P3 | - | 5 / a9963c00a 2026-10-31 / 0 | L121:WQ:none, L129:WQ/DevReihe:none | 2026-10-31: ## Anwenderentscheidung 2026-10-31 (nach der QEMU-Messung) | n/a |
| FS27 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: nur beschaedigte Medien) · Prioritaet: P3 · Typ: Developer (erst … | P3 | - | 1 / 40e06519c 2026-10-29 / 0 | L106:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: nur beschaedigte Medien) · Prioritaet: P3 ·  | yes (status line): (Anwender, Vor-Start-Pruefung: nur beschaedigte Medien) |
| FS29 | OK | @L3 GEPARKT 2026-10-31 (Anwender: braucht ein Werkzeug, das per `Rwabs` Sektor 0 schreibt; keins auf der Karte oder gep… | P3 | - | 2 / 37d8e2a73 2026-10-31 / 0 | L131:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender: braucht ein Werkzeug, das per `Rwabs` Sektor 0 schreibt; keins auf d | yes (status line): (Anwender: braucht ein Werkzeug, das per `Rwabs` Sektor 0 schreibt; keins auf der Karte od… |
| FS30 | OK | @L3 GEPARKT 2026-10-31 (Anwender: kein Programm liest es) · Prioritaet: P3 · Typ: Developer — Fund aus FS26<FS26> (Mess… | P3 | - | 1 / 37d8e2a73 2026-10-31 / 0 | L131:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender: kein Programm liest es) · Prioritaet: P3 · Typ: Developer — Fund | yes (status line): (Anwender: kein Programm liest es) |
| FS31 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<FS31-STATUS-DONE>) · Prioritaet: P1 · Typ: Belegter Defekt, Datenverlust — aus HW08<H… | P1 | DONE | 9 / 3e825c135 2026-11-07 / 1 | L44:CS:none, L121:WQ:done | none found | n/a |
| FS33 | OK | @L3 OFFEN (2026-11-06) · Prioritaet: P3 · Typ: Abweichung, am Orakel gemessen — aus HW08<HW08> | P3 | - | 1 / 5fb5f5034 2026-11-06 / 0 | L121:WQ:none | none found | n/a |
| FS34 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P3 · Typ: Abweichung, in QEMU gemessen — aus HW08<HW08> | P3 | - | 1 / deaad5dfa 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| GH1 | OK | @L3 ERLEDIGT (QEMU) 2026-10-21 (`7b295ab3`; Pi abgenommen 2026-10-22, `docs/HW-TEST-LOG.md`) · Prioritaet: P2 · Typ: Fe… | P2 | - | 10 / 582a31ffc 2026-10-26 / 1 | L31:CS:none, L164:ERLoQ:done-no-receipt | 2026-10-22: - ~~Echte Pi 4~~ abgenommen 2026-10-22 (Anwender-Kurztest, `docs/HW-TEST-LOG.md`). | n/a |
| GUI01 | OK | @L3 GESCHLOSSEN 2026-10-22 (Anwenderentscheid 2026-10-18: Betrachter als Anwendung + Zuordnung, kein eingebautes `show_… | P4 | - | 5 / 37b42d622 2026-10-22 / 0 | - | 2026-10-22: Status: GESCHLOSSEN 2026-10-22 (Anwenderentscheid 2026-10-18: Betrachter als Anwendung + Zuordnung, kein e | n/a |
| GUI02 | OK | @L3 ZURUECKGESTELLT (Lead 2026-09-24) · Prioritaet: P4 · Typ: Funktionsluecke AES/Desktop — aus der Schaltertabelle (RE… | P4 | - | 2 / efc778861 2026-09-24 / 0 | - | none found | yes (status line): Lead 2026-09-24) |
| GUI03 | OK | @L3 OFFEN · Prioritaet: P4 · Typ: Funktionsluecke AES — aus der Schaltertabelle (REF01) | P4 | - | 1 / e3234e6d4 2026-09-24 / 0 | - | none found | n/a |
| GUI04 | OK | @L3 ERLEDIGT — auf echter Pi 4 abgenommen (2026-09-28) · Prioritaet: P2 · Typ: Funktionsluecke USB/AES + benannte Erwei… | P2 | DONE | 6 / 3e30fcda4 2026-09-28 / 1 | - | 2026-09-25: Lead-Entscheidung des Anwenders 2026-09-25: das Rad soll scrollen. Also | n/a |
| GUI05 | OK | @L3 OFFEN · Prioritaet: P4 · Typ: Konformitaet AES/Desktop — aus der Schaltertabelle (REF01) | P4 | - | 1 / e3234e6d4 2026-09-24 / 0 | - | none found | n/a |
| GUI06 | OK | @L3 BEHOBEN — Pi-Abnahme offen (2026-09-28) · Prioritaet: P2 · Typ: Belegter Defekt / Laufzeit — aus der FS04-Messung h… | P2 | DONE | 2 / 87b34ead1 2026-09-29 / 0 | - | none found | n/a |
| GUI07 | OK | @L3 ERLEDIGT (2026-10-03, Statusdatei<GUI07-STATUS-DONE>; vorgezogen; Pi 2026-10-03: Doom x3/x4 40–75 fps statt 7,6/4,2… | P2 | DONE | 6 / 6190bdbc5 2026-10-03 / 0 | - | 2026-09-28: ## Die Staffelung bis zum Assembler — Anwenderfrage 2026-09-28 | n/a |
| H01 | OK | @L3 GEPARKT — wartet auf Hardware · Prioritaet: P2 · Typ: Treiber / Hardware | P2 | - | 1 / a10ae969a 2026-09-29 / 0 | L160:P2:open | 2026-09-12: Das 24C32-EEPROM als NVRAM. Anwenderentscheid 2026-09-12: „mit dem NVRAM | yes (status line): wartet auf Hardware |
| H02 | OK | @L3 ERLEDIGT (2026-09-17), auf echter Pi 4 bestaetigt 2026-09-18 · Prioritaet: P1 · Typ: Beobachtung auf echter Hardware | P1 | DONE | 11 / c15baedba 2026-09-21 / 4 | - | 2026-09-17: Beobachtet vom Anwender auf echter Pi 4 am 2026-09-17, Kartenstand: | n/a |
| H03 | OK | @L3 ERLEDIGT (2026-09-17), auf echter Pi 4 bestaetigt 2026-09-18 · Prioritaet: P1 · Typ: Beobachtung auf echter Hardware | P1 | DONE | 11 / cb6dd5a60 2026-09-21 / 2 | - | 2026-09-17: Beobachtet vom Anwender auf echter Pi 4 am 2026-09-17, gleicher | n/a |
| H04 | OK | @L3 GEPARKT — nach H02/A66/A67, vor dem Oktober-Termin nur, wenn Zeit bleibt · Prioritaet: P2, Demo-Wert hoch; gehoert … | P2, | - | 2 / 9ee879def 2026-10-23 / 0 | L121:WQ:none, L159:P2:open, L172:U10:none | 2026-10-23: Status: GEPARKT — nach H02/A66/A67, vor dem Oktober-Termin nur, wenn Zeit bleibt · Prioritaet: P2, Demo- | yes (status line): nach H02/A66/A67, vor dem Oktober-Termin nur, wenn Zeit bleibt |
| H05 | OK | @L3 ERLEDIGT (2026-09-18), auf echter Pi 4 bestaetigt 2026-09-20 · Prioritaet: P1 · Typ: Beobachtung auf echter Hardwar… | P1 | DONE | 9 / 83a403cd1 2026-09-20 / 4 | - | none found | n/a |
| HW02 | OK | @L3 ZURUECKGESTELLT (Lead 2026-09-25) · Prioritaet: P4 · Typ: Genauigkeitsfrage — Nebenbefund ACC06 | P4 | - | 4 / d8b260035 2026-09-25 / 0 | - | 2026-09-25: ## Die eigentliche Frage (Anwender, 2026-09-25): taktet die Pi von selbst? | yes (status line): Lead 2026-09-25) |
| HW03 | OK | @L3 BEHOBEN — drei Pi-Gegenproben offen (2026-09-27) · Prioritaet: P1 · Typ: Belegter Defekt / Datenintegritaet — aus d… | P1 | DONE | 7 / e421bd5d6 2026-11-06 / 4 | L121:WQ:none, L151:P1u:impl-open | none found | n/a |
| HW04 | OK | @L3 OFFEN (teilweise: Durchsatzpunkte 1-Bit-Bus und Nur-Einzelblock erledigt mit FS08 Teil 1 `e8fe4137`, Pi-Messung `me… | P2 | - | 3 / d40f4f16d 2026-10-21 / 0 | L158:P2:open | none found | n/a |
| HW06 | OK | @L3 ERLEDIGT — Eigenschaft der Karte, kein Muster in NewTOS (2026-11-06, HW08<HW08>; vorher zurueckgestellt, Anwender 2… | P3 | - | 2 / e421bd5d6 2026-11-06 / 1 | L121:WQ:done | 2026-09-25: Status: ERLEDIGT — Eigenschaft der Karte, kein Muster in NewTOS (2026-11-06, HW08<HW08>; vorher zurue | n/a |
| HW07 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Auffaelligkeit, keine Feststellung — aus GUI06<GUI06> | P3 | - | 1 / 4ef536366 2026-09-28 / 0 | - | none found | n/a |
| HW08 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<HW08-STATUS-DONE>; Pi-Laeufe `measurements/measurement-2026-11-07-hw08-pi5/RUNINFO.TX… | P1 | DONE | 27 / deaad5dfa 2026-11-07 / 3 | L44:CS:none, L44:CS:none, L121:WQ:none, L121:WQ:none, L121:WQ:none, L160:P2:done | 2026-11-06: ## Entscheidungen fuer den Fix (Anwender 2026-11-06, traegt die Empfehlungen des Leads mit) | n/a |
| IDEA01 | OK | @L3 IDEE (Anwender 2026-09-25, ausdruecklich „nicht jetzt") · Prioritaet: — · Typ: Richtungsnotiz | — | - | 2 / 849961c86 2026-09-25 / 0 | - | 2026-09-25: Status: IDEE (Anwender 2026-09-25, ausdruecklich „nicht jetzt") · Prioritaet: — · Typ: Richtungsnotiz | yes (status line): Anwender 2026-09-25, ausdruecklich „nicht jetzt") |
| INPUT02 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Abweichung von EmuTOS — Nebenfund RST01 (2026-10-17) | P3 | - | 1 / 51504c242 2026-10-17 / 0 | L121:WQ:none, L177:P34:open | none found | n/a |
| INPUT03 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm braucht die Maus-Tastatur-Emulation) · Prioritaet: … | P3 | - | 1 / 6d2bc1d1b 2026-10-29 / 0 | L104:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm braucht die Maus-Tastatur-Emulatio | yes (status line): (Anwender, Vor-Start-Pruefung: kein Programm braucht die Maus-Tastatur-Emulation) |
| KEYOWNER | OK | @L3 ERLEDIGT (QEMU), 2026-10-08 — Quittung<KEYOWNER-STATUS-DONE> · Prioritaet: P1 (sichtbar im Demo-Stand: Esc/Return/P… | P1 | DONE | 1 / c22c952b0 2026-10-08 / 0 | - | none found | n/a |
| KEYS01 | OK | @L3 ERLEDIGT — Pi-Abnahme 2026-10-03 · Prioritaet: P1 (vor dem Termin) · Typ: Kernel / BIOS-Tastatur — aus PORT06-KEYS-… | P1 | DONE | 4 / f557802bb 2026-10-03 / 1 | L233:PH:none | 2026-10-02: ## Entscheidung (Anwender 2026-10-02) | n/a |
| KEYS02 | OK | @L3 ERLEDIGT — Pi-Abnahme 2026-10-04 (Statusdatei<KEYS02-STATUS-DONE>; TT in Hatari bestanden) · Prioritaet: P1 (Anwend… | P1 | DONE | 9 / 98d935a1c 2026-10-04 / 2 | L28:CS:none, L237:PH:none, L252:BLK:blocked | 2026-10-04: Status: ERLEDIGT — Pi-Abnahme 2026-10-04 (Statusdatei<KEYS02-STATUS-DONE>; TT in Hatari bestanden) ·  | n/a |
| KEYS03 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Kernel / Desktop-INF — offener Konflikt aus `docs/COMPATIBILITY.md` (Eintrag „The boo… | P3 | - | 2 / 3857c9c3d 2026-10-26 / 0 | L178:P34:open | none found | n/a |
| KEYS04 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm pollt Kbshift (nur apitest)) · Prioritaet: P3… | P3 | - | 1 / 6d2bc1d1b 2026-10-29 / 0 | L103:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm pollt Kbshift (nur apitest)) | yes (status line): (Anwender, Vor-Start-Pruefung: kein Kartenprogramm pollt Kbshift (nur apitest)) |
| KEYS05 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P4 · Typ: Abweichung, gelesen, nicht gemessen — Nebenbefund aus V34<V34> | P4 | - | 1 / 35eeb6b51 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| LIBC-BUF | OK | @L3 ERLEDIGT 2026-10-22 (QEMU, `41ba8f96`; Sonde LBUF rot/gruen, `measurements/measurement-2026-10-22-libc-buf/`; Pi of… | P3 | - | 6 / 58abf82ad 2026-10-22 / 2 | L165:ERLoQ:done-no-receipt | 2026-10-22: ## Anwenderentscheid 2026-10-22 | n/a |
| LIBC-FSTAT | OK | @L3 ERLEDIGT (Quittung<LIBC-FSTAT-STATUS-DONE>, 2026-10-17, Tester-Abnahme offen) · Prioritaet: P3 · Typ: SDK / libc — … | P3 | DONE | 6 / 1842d0bdf 2026-10-16 / 1 | - | 2026-10-17: ## Entscheidung (Anwender 2026-10-17) | n/a |
| LIBC-IOCTL | OK | @L3 OFFEN · Prioritaet: P3 · Typ: SDK / libc — Anwenderentscheidung 2026-10-13 (LIBC-TTY<LIBC-TTY> Punkt 2) | P3 | - | 1 / 4d4a964d2 2026-10-13 / 0 | L177:P34:open | 2026-10-13: Status: OFFEN · Prioritaet: P3 · Typ: SDK / libc — Anwenderentscheidung 2026-10-13 ([LIBC-TTY](LIBC-TT | n/a |
| LIBC-TTY | OK | @L3 ERLEDIGT 2026-10-13 (Quittung<LIBC-TTY-STATUS-DONE>) · Prioritaet: P2 · Typ: SDK / libc — Nebenfund FS11 (2026-10-1… | P2 | DONE | 7 / 9c57edc4c 2026-10-13 / 1 | - | 2026-10-13: ## Entscheidung (Anwender 2026-10-13) | n/a |
| M01 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Hardwaremessung | P1 | DONE | 0 / - / 0 | - | none found | n/a |
| M02 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Hardwaremessung | P1 | DONE | 0 / - / 0 | - | none found | n/a |
| M03 | OK | @L3 ERLEDIGT · Priorität: P1 · Typ: Hardwaremessung | P1 | DONE | 0 / - / 0 | - | none found | n/a |
| M04 | OK | @L3 ERLEDIGT · Priorität: P2 · Typ: Messverfahren | P2 | DONE | 0 / - / 0 | - | none found | n/a |
| M05 | OK | @L3 ERLEDIGT (2026-09-12) · Priorität: P1 · Typ: Messverfahren / Werkzeuggrenze | P1 | DONE | 2 / 134e0b9f3 2026-09-14 / 1 | - | none found | n/a |
| M06 | OK | @L3 ERLEDIGT (2026-09-13) · Priorität: P1 · Typ: Messverfahren / Werkzeugfehler | P1 | DONE | 2 / 134e0b9f3 2026-09-14 / 1 | - | none found | n/a |
| M07 | OK | @L3 ERLEDIGT (2026-09-13) · Priorität: P2 · Typ: Messverfahren / Werkzeuggenauigkeit | P2 | DONE | 2 / 134e0b9f3 2026-09-14 / 1 | - | none found | n/a |
| M08 | OK | @L3 ERLEDIGT 2026-10-17 — Quittung<M08-STATUS-DONE> · Priorität: P2 · Typ: Messverfahren / Dauertest | P2 | DONE, UNFINISHED | 6 / ce68321b4 2026-10-17 / 2 | - | none found | n/a |
| M09 | OK | @L3 OFFEN · Priorität: P3 · Typ: Messverfahren / Entwurf vor Code | P3 | - | 0 / - / 0 | - | none found | n/a |
| M10 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Messverfahren / Entwurf vor Code | P3 | - | 1 / bff78f6fd 2026-09-14 / 0 | - | none found | n/a |
| MEM01 | OK | @L3 ERLEDIGT 2026-10-22 (behoben mit ARCH03 Stufe 1 `c6280342`, ST/TT getrennt; Hosttest `mem02_mxalloc_modes`, `measur… | P2 | - | 2 / 2d28bea65 2026-10-22 / 0 | L165:ERLoQ:done-no-receipt | 2026-10-17: Status: ERLEDIGT 2026-10-22 (behoben mit ARCH03 Stufe 1 `c6280342`, ST/TT getrennt; Hosttest `mem02_mxallo | n/a |
| MEM02 | OK | @L3 ERLEDIGT 2026-10-17 (Statusdatei<MEM02-STATUS-DONE>; behoben durch ARCH03 Stufe 1 `a9279fe9`) · Prioritaet: P2 · Ty… | P2 | DONE | 5 / 2dbc58ead 2026-10-17 / 0 | - | 2026-10-17: Abnahme neu (Anwender 2026-10-17): die Ball-Demo ist kein Messwert und | n/a |
| MEM03 | OK | @L3 ERLEDIGT 2026-10-05 (mit ARCH03 Stufe 1 `c6280342`; Sonde ENVKID/MEM03 PASS in QEMU, siehe ARCH03-1-Quittung<ARCH03… | P2 | - | 7 / d40f4f16d 2026-10-21 / 4 | L165:ERLoQ:done-no-receipt | none found | n/a |
| MEM04 | OK | @L3 ERLEDIGT mit ARCH03 Stufe 3b (2026-10-05) — bei eingeschaltetem Speicherschutz; Tester-Abnahme und Pi offen · Prior… | P2 | - | 2 / 48ba717ca 2026-10-05 / 1 | L165:ERLoQ:done-no-receipt | none found | n/a |
| MEM05 | OK | @L3 ERLEDIGT 2026-10-14 (Quittungen A<MEM05-A-STATUS-DONE>, B<MEM05-B-STATUS-DONE>; Pi offen) · Prioritaet: P2 · Typ: G… | P2 | MEM05-A:DONE, MEM05-B:DONE | 16 / 526ad8ac0 2026-10-17 / 4 | L167:ERLoQ:done-no-receipt | 2026-10-13: ## Nachentscheidung (Anwender 2026-10-13) — `SNAP` im Seitenbudget | n/a |
| MEM06 | OK | @L3 IDEE (Anwender 2026-10-13, aus MEM05) · Prioritaet: P3 · Typ: Speicherverwaltung — Richtungsnotiz | P3 | - | 1 / 9d2e6f2f2 2026-10-13 / 0 | L179:P34:open | 2026-10-13: Status: IDEE (Anwender 2026-10-13, aus MEM05) · Prioritaet: P3 · Typ: Speicherverwaltung — Richtungsno | yes (status line): Anwender 2026-10-13, aus MEM05) |
| MEM07 | OK | @L3 ERLEDIGT (geschlossen nach Review 2026-10-19) · Prioritaet: P3 · Typ: Grenzwert — Nebenfund MEM05-Entwurf (2026-10-… | P3 | - | 2 / 22884fcfb 2026-10-19 / 0 | L165:ERLoQ:done-no-receipt | none found | n/a |
| MEM08 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Laufzeit — Nebenfund MEM05-Entwurf (2026-10-13) | P3 | - | 0 / - / 0 | L178:P34:open | none found | n/a |
| MEM09 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<MEM09-STATUS-DONE>) · Prioritaet: P3 · Typ: Robustheit — Nebenfund MEM05-Entwurf (202… | P3 | DONE | 1 / 526ad8ac0 2026-10-17 / 0 | - | none found | n/a |
| MEM12 | OK | @L3 ERLEDIGT 2026-10-30 (Quittung<MEM12-STATUS-DONE>; Pi-Lauf des Anwenders 2026-10-29, Kern `58e35ac8`) · Prioritaet: … | P1 | DONE | 25 / a9963c00a 2026-10-31 / 5 | L141:P1:done | 2026-10-30: ## Abschluss (Anwender, 2026-10-30) | n/a |
| MEMPROT-FAST | OK | @L3 ERLEDIGT in QEMU (Quittung MEMPROT-FAST-STATUS-DONE.MD<MEMPROT-FAST-STATUS-DONE>), Pi-Lauf offen · Prioritaet: P1 ·… | P1 | DONE | 1 / 15e51665c 2026-10-07 / 0 | - | 2026-10-07: Anwender-Entscheidung 2026-10-07 („Optionen 1+2“ des Auftrags) = in | n/a |
| MOD01 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Quellenanalyse (Researcher) — Entwurfsgrundlage, kein Code | P3 | - | 3 / 64bee0ddf 2026-09-25 / 0 | - | none found | n/a |
| NET01 | OK | @L3 OFFEN — Vorrecherche · Prioritaet: P2 (Ziel 1.2, Pflicht) · Typ: Erweiterung — Anwender 2026-10-20 | P2 | - | 1 / 062f33819 2026-10-20 / 0 | L158:P2:open | 2026-10-20: Status: OFFEN — Vorrecherche · Prioritaet: P2 (Ziel 1.2, Pflicht) · Typ: Erweiterung — Anwender 2026-1 | n/a |
| NET02 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Portierung / Schnittstelle | P3 | - | 0 / - / 0 | - | none found | n/a |
| NET03 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Portierung (gross) | P3 | - | 0 / - / 0 | - | none found | n/a |
| NET04 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: API-Schicht | P3 | - | 0 / - / 0 | - | none found | n/a |
| NET05 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Anwendungsport | P3 | - | 0 / - / 0 | - | none found | n/a |
| PITEST01 | OK | @L3 ERLEDIGT 2026-11-02 (Quittung PITEST01-STATUS-DONE.MD<PITEST01-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (Son… | P2 | DONE | 11 / 468b3b531 2026-11-02 / 3 | L119:WQ:none, L119:WQ:none, L119:WQ:none | 2026-11-02: ## Abschluss (Anwender 2026-11-02) | n/a |
| PORT01 | OK | @L3 URSACHE GEFUNDEN UND BEHOBEN (2026-09-25) — Abnahme blockiert durch einen zweiten, eigenen Defekt (siehe Abschluss … | P2 | DONE | 5 / 659f1e7ee 2026-09-25 / 1 | - | none found | n/a |
| PORT02 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<PORT02-STATUS-DONE>) · Prioritaet: P2 · Typ: Bewertung / Portierbarkeit — Anwenderwun… | P2 | DONE | 12 / 526ad8ac0 2026-10-17 / 0 | - | 2026-09-28: Anwender hat ihn am 2026-09-28 selbst als „wichtiges Werkzeug, fuer eine | n/a |
| PORT03 | OK | @L3 ERLEDIGT (2026-09-25) · Prioritaet: P2 · Typ: Versionswechsel — Nebenbefund PORT01 | P2 | DONE | 7 / f974b3649 2026-09-25 / 1 | - | none found | n/a |
| PORT04 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Quellenpflege (Researcher, dann ggf. Developer) | P3 | - | 1 / f974b3649 2026-09-25 / 0 | - | none found | n/a |
| PORT05 | OK | @L3 BEHOBEN — auf echter Pi 4 abgenommen (2026-09-26) · Prioritaet: P1 · Typ: Belegter Defekt — Anwender 2026-09-25 | P1 | DONE | 14 / ec94e7b94 2026-09-29 / 5 | - | 2026-09-25: ## Beide Ausgaenge, im Wortlaut (Anwender 2026-09-25) | n/a |
| PORT06 | OK | @L3 ERLEDIGT — Pi-Abnahme 2026-10-04 (Statusdatei<PORT06-STATUS-DONE>; TT als Machbarkeitsbeweis) · Prioritaet: P1 (vor… | P1 | PORT06-EDDI:DONE, DONE | 27 / 2dbc58ead 2026-10-17 / 8 | - | 2026-10-02: ## Entscheidungen 2026-10-02 (Anwender) | n/a |
| PORT07 | OK | @L3 ERLEDIGT — ein Pi-Lauf offen (2026-09-28) · Prioritaet: P2 · Typ: Portierung / Konformitaet — Anwenderwunsch 2026-0… | P2 | DONE | 3 / 87b34ead1 2026-09-29 / 0 | - | 2026-09-28: Status: ERLEDIGT — ein Pi-Lauf offen (2026-09-28) · Prioritaet: P2 · Typ: Portierung / Konformitaet —  | n/a |
| PORT08 | OK | @L3 GEKLAERT — Ursache getrennt (2026-09-29) · Prioritaet: P2 · Typ: Belegter Defekt — Pi-Lauf 2026-09-28 | P2 | DONE | 4 / 87b34ead1 2026-09-29 / 0 | - | none found | n/a |
| PORT09 | OK | @L3 ERLEDIGT 2026-10-06 (Quittung<PORT09-STATUS-DONE>; vorher blockiert durch MEM03<MEM03>, Statusdatei<PORT09-STATUS-U… | P3 | DONE, UNFINISHED | 8 / feb76357f 2026-10-17 / 3 | - | none found | n/a |
| PORT10 | OK | @L3 ERLEDIGT (2026-10-02, Statusdatei<PORT10-STATUS-DONE>; `system()` lief ueber ein SDK-Overlay bis PROC01<PROC01> — s… | P1 | DONE | 8 / 861ad0380 2026-10-06 / 6 | - | none found | n/a |
| PORT12 | OK | @L3 Stufe 1 erledigt, ADR-Entscheidung offen (2026-11-06) · Prioritaet: P2 · Typ: Port, Erweiterung; Anwender 2026-11-0… | P2 | PORT12-1:DONE | 3 / 5fb5f5034 2026-11-06 / 0 | L70:OA:open, L121:WQ:done | 2026-11-06: Status: Stufe 1 erledigt, ADR-Entscheidung offen (2026-11-06) · Prioritaet: P2 · Typ: Port, Erweiterun | n/a |
| PRESENT01 | OK | @L3 ERLEDIGT (QEMU), 2026-10-08, ein offener Punkt (Bild auf/ab, Kernel-Tastaturtabelle) — Quittung<PRESENT01-STATUS-DO… | P1 | DONE | 0 / - / 0 | - | 2026-10-08: ## Anforderungen des Anwenders (2026-10-08) | n/a |
| PROC01 | OK | @L3 ERLEDIGT (2026-10-05, mit ARCH03 Stufe 2, Statusdatei<ARCH03-2-STATUS-DONE>) · Prioritaet: P2 (nach dem OFAM-Termin… | P2 | - | 4 / 32c6171ca 2026-10-05 / 2 | L165:ERLoQ:done-no-receipt | 2026-10-02: diesen Zweig (Entscheidung Anwender 2026-10-02). | n/a |
| PROC02 | OK | @L3 ERLEDIGT 2026-10-06 (Quittung<PROC02-STATUS-DONE>) · Prioritaet: P2 (nach dem OFAM-Termin) · Typ: Abweichung von Em… | P2 | DONE | 4 / e0ce232f9 2026-10-06 / 2 | - | none found | n/a |
| PROC03 | OK | @L3 ERLEDIGT 2026-10-19 (QEMU) — Quittung<PROC03-STATUS-DONE> · Prioritaet: P2 · Typ: Abweichung von EmuTOS — aus ARCH0… | P2 | DONE | 10 / d41789b50 2026-10-19 / 3 | - | 2026-10-18: ## Anwenderentscheidung B5 (2026-10-18) | n/a |
| PROC04 | OK | @L3 ERLEDIGT 2026-10-19 (QEMU) — Quittung<PROC04-STATUS-DONE> · Prioritaet: P3 · Typ: Abweichungen von EmuTOS — aus PRO… | P3 | DONE | 17 / 3bba05338 2026-10-19 / 5 | - | 2026-10-18: ## Anwenderentscheidungen zu Punkt 1 (2026-10-18) | n/a |
| PROC05 | OK | @L3 ERLEDIGT (2026-10-14, DESK43 U9-0b, uncommitted) · Prioritaet: P3 · Typ: Konformitaet — Nebenfund DESK43 U6a (2026-… | P3 | DONE | 7 / cbad7659b 2026-10-16 / 2 | - | none found | n/a |
| PROC06 | OK | @L3 ERLEDIGT (2026-10-16, uncommitted) · Prioritaet: P3 · Typ: Konformitaet — Nebenfund DESK43 U9-0 (2026-10-14) · Quit… | P3 | DONE | 4 / 16970fa16 2026-10-16 / 2 | - | none found | n/a |
| PROC07 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: undokumentierte CP/M-Reliquie; gemlib sendet nur Opcodes != 0) · … | P3 | - | 1 / 6ddd4496a 2026-10-29 / 0 | L105:WQ/EbGeparkt:parked, L109:WQ/EbGeparkt:parked, L132:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: undokumentierte CP/M-Reliquie; gemlib sendet nur | yes (status line): (Anwender, Vor-Start-Pruefung: undokumentierte CP/M-Reliquie; gemlib sendet nur Opcodes !=… |
| Q06 | OK | @L3 ERLEDIGT 2026-10-17 (Quittung<Q06-STATUS-DONE>) · Priorität: P1 · Typ: Anwendungsdiagnose → belegter Defekt | P1 | DONE, UNFINISHED | 1 / feb76357f 2026-10-17 / 0 | - | none found | n/a |
| Q07 | OK | @L3 ERLEDIGT (2026-09-12) · Prioritaet: P1 · Typ: Anwendungsdiagnose / Funktionsfehler | P1 | DONE | 4 / 6f3883ce7 2026-09-20 / 1 | - | 2026-09-12: Vom Anwender auf echter Pi 4 gemeldet (2026-09-12): QED zeigt beim Start, es | n/a |
| R01 | OK | @L3 ZURÜCKGESTELLT · Priorität: P3 · Typ: Zurückgestellte Vertragsabweichung | P3 | - | 0 / - / 0 | - | none found | yes (status line): ZURÜCKGESTELLT |
| REF01 | OK | @L3 ERLEDIGT (2026-09-24) · Prioritaet: P2 · Typ: Werkzeugkette / Quellenanalyse | P2 | DONE | 4 / 60db9fcec 2026-09-24 / 0 | - | none found | n/a |
| REF02 | OK | @L3 ERLEDIGT (2026-09-28) · Prioritaet: P2 · Typ: Quellenlage (Researcher) — aus dem EMMC-Audit | P2 | DONE | 2 / f3a468fb2 2026-09-27 / 1 | - | none found | n/a |
| REF03 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Quellenpflege — aus REF02 | P3 | - | 1 / 41c9bc1c9 2026-09-28 / 0 | L175:P34:open | none found | n/a |
| REF04 | OK | @L3 ERLEDIGT (2026-09-29) · Prioritaet: P2 · Typ: Lizenz / Veroeffentlichung — aus REF02 | P2 | - | 5 / 22325b9a6 2026-10-22 / 0 | L165:ERLoQ:done-no-receipt | none found | n/a |
| REF05 | OK | @L3 OFFEN — braucht den Anwender · Prioritaet: P3 · Typ: Quellenlage — uebertragen aus dem alten GitHub-Issue #11 (2026… | P3 | - | 1 / 22325b9a6 2026-10-22 / 0 | L175:P34:open | none found | n/a |
| REF06 | OK | @L3 OFFEN — braucht den Lead · Prioritaet: P3 · Typ: Quellenlage — Restpunkte aus REF02<REF02> (2026-09-28), aus dem ST… | P3 | - | 1 / f242696e8 2026-10-26 / 0 | L175:P34:open | none found | n/a |
| REL07 | OK | @L3 ERLEDIGT 2026-11-04 (Quittung<REL07-STATUS-DONE>) · Prioritaet: P1 · Typ: Release — Anwenderentscheidung 2026-11-04 | P1 | DONE | 13 / 123315777 2026-11-04 / 2 | L121:WQ:done, L121:WQ:done | 2026-11-04: ## Stand Schritt 5 (veroeffentlicht 2026-11-04, Anwender-Go) | n/a |
| REL072 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<REL072-STATUS-DONE>; veroeffentlicht: Tag `v0.7.2` auf `d35670d6`, GitHub-Pre-Release… | P1 | DONE | 8 / ad58e69e0 2026-11-10 / 0 | L60:OA:done, L280:RD:done | 2026-11-08: den der Anwender am 2026-11-08 auf der Pi gefahren hat. Vorher der unbelegte Satz zum Zeiger beim Rechnen aus  | n/a |
| RST01 | OK | @L3 ERLEDIGT 2026-10-17 (QEMU, `f63c39c1`; Pi abgenommen 2026-10-18, `docs/HW-TEST-LOG.md`) — Quittung<RST01-STATUS-DON… | P2 | DONE | 10 / 46947fab1 2026-10-30 / 2 | - | 2026-10-17: ## Anwenderentscheidungen (2026-10-17) | n/a |
| SCHED01 | OK | @L3 BEHOBEN — auf echter Pi 4 abgenommen (2026-09-29) · Prioritaet: P1 · Typ: Belegter Defekt / Haenger — aus PORT08<PO… | P1 | DONE | 6 / 7c5c6e2e2 2026-09-29 / 1 | - | 2026-09-29: Sichtbar bestaetigt (Anwender, 2026-09-29): *„der Desktop kam wieder | n/a |
| SDK02 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Nutzer der Konstanten) · Prioritaet: P3 · Typ: Developer (SD… | P3 | - | 2 / a5e8a8762 2026-10-28 / 0 | L116:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Nutzer der Konstanten) · Prioritaet: P3 | yes (status line): (Anwender, Vor-Start-Pruefung: kein Nutzer der Konstanten) |
| SDK03 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<SDK03-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (SDK) — Anwenderentscheidung 20… | P2 | DONE | 3 / 86f39d28e 2026-10-29 / 2 | L45:CS:none | 2026-10-28: Status: ERLEDIGT 2026-10-29 (Quittung<SDK03-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (SDK) | n/a |
| SDK04 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<SDK04-STATUS-DONE>) · Prioritaet: P1 · Typ: Researcher (Erhebung) + Developer (SDK-Ty… | P1 | SDK04-C6:DONE, DONE | 7 / 9b882e8d4 2026-10-29 / 3 | L45:CS:none, L45:CS:none | 2026-10-28: ## Erweiterung 2026-10-28 (Anwender) | n/a |
| SDK05 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<SDK05-STATUS-DONE>); QED-Anzeige unveraendert (siehe Quittung) · Prioritaet: P1 · Typ… | P1 | DONE | 3 / 9b882e8d4 2026-10-29 / 2 | L45:CS:none | 2026-10-28: Status: ERLEDIGT 2026-10-29 (Quittung<SDK05-STATUS-DONE>); QED-Anzeige unveraendert (siehe Quittung)  | n/a |
| SDK06 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm liest einen Cookie mit gesetztem Bit 31) · Priorita… | P3 | - | 1 / 2de6909ab 2026-10-29 / 0 | L117:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm liest einen Cookie mit gesetztem B | yes (status line): (Anwender, Vor-Start-Pruefung: kein Programm liest einen Cookie mit gesetztem Bit 31) |
| SDK07 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<SDK07-STATUS-DONE>) · Prioritaet: P1 · Typ: Developer (SDK-Overlay) — Anwenderentsche… | P1 | DONE | 2 / 58e35ac87 2026-10-29 / 1 | L45:CS:none | 2026-10-29: Status: ERLEDIGT 2026-10-29 (Quittung<SDK07-STATUS-DONE>) · Prioritaet: P1 · Typ: Developer (SDK- | n/a |
| SDK08 | OK | @L3 ERLEDIGT 2026-10-30 (Quittung<SDK08-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (Werkzeug/SDK) — Anwenderentsch… | P2 | DONE | 3 / d3d67c671 2026-10-30 / 1 | L44:CS:none, L45:CS:none, L134:WQ:done, L135:WQ/DevReihe:none | 2026-10-30: ## Vorgabe (Anwender, 2026-10-30) | n/a |
| SDL01 | OK | @L3 OFFEN · Prioritaet: P3 (nach dem OFAM-Termin) · Typ: Portierung / Unterbau — Anwenderidee 2026-09-28 | P3 | - | 1 / dcaee1bf9 2026-09-28 / 0 | L174:P34:open | 2026-09-28: Status: OFFEN · Prioritaet: P3 (nach dem OFAM-Termin) · Typ: Portierung / Unterbau — Anwenderidee 2026 | n/a |
| SER01 | OK | @L3 ERLEDIGT (2026-10-02, Statusdatei<SER01-STATUS-DONE>; Pi-Abnahme offen) · Prioritaet: P1 (vor dem Termin) · Typ: Ke… | P1 | DONE | 4 / cdb774d33 2026-10-03 / 1 | L121:WQ:none, L152:P1u:impl-open, L195:CS:none | 2026-10-02: ## Entscheidung (Anwender 2026-10-02) | n/a |
| SLB01 | OK | @L3 ZUKUNFT (Anwender 2026-10-06: „erstmal so belassen, das ist ein Zukunftsplan") · Prioritaet: P3 · Typ: FreeMiNT-Erw… | P3 | - | 1 / 29f67a852 2026-10-06 / 0 | - | 2026-10-06: Status: ZUKUNFT (Anwender 2026-10-06: „erstmal so belassen, das ist ein Zukunftsplan") · Prioritaet: P3* | yes (status line): Anwender 2026-10-06: „erstmal so belassen, das ist ein Zukunftsplan") |
| SND01 | OK | @L3 OFFEN · Prioritaet: P2 (gehoert zu 1.0, Anwender 2026-10-23; vorher P3) · Typ: Treiber + API-Ebene — Anwenderwunsch… | P2 | SND01-SPEC:DONE, SND01-TRACKB:DONE | 10 / 73a66ceb8 2026-10-23 / 0 | L121:WQ:none, L157:P2:open, L171:U10:none | 2026-10-23: Status: OFFEN · Prioritaet: P2 (gehoert zu 1.0, Anwender 2026-10-23; vorher P3) · Typ: Treiber + API-E | n/a |
| SND02 | OK | @L3 OFFEN · Prioritaet: P2 (gehoert zu 1.0, Anwender 2026-10-23; vorher P3) · Typ: Track-B-Recherche — aus SND01<SND01>… | P2 | - | 3 / 9ee879def 2026-10-23 / 0 | L121:WQ:none, L157:P2:open, L172:U10:none | 2026-10-23: Status: OFFEN · Prioritaet: P2 (gehoert zu 1.0, Anwender 2026-10-23; vorher P3) · Typ: Track-B-Recherc | n/a |
| SPEC01 | OK | @L3 ERLEDIGT 2026-10-16 (`43ae3553`) · Prioritaet: P3 · Typ: Spec-Korrektur — Nebenfund DESK43 U5b (2026-10-14) | P3 | - | 3 / 16970fa16 2026-10-16 / 0 | L165:ERLoQ:done-no-receipt | none found | n/a |
| SPEC02 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Nachpflege; offene Arbeit liest die Stellen nicht) · Priori… | P3 | - | 2 / a8d875a73 2026-10-30 / 0 | L107:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Nachpflege; offene Arbeit liest die Stelle | yes (status line): (Anwender, Vor-Start-Pruefung: reine Nachpflege; offene Arbeit liest die Stellen nicht) |
| SPEC04 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<SPEC04-STATUS-DONE>); offen fuer den Developer: Punkt 3 (Generator), Punkt 5 (Messfal… | P4 | DONE | 3 / ad201a574 2026-11-08 / 0 | L121:WQ:none | none found | n/a |
| SYSINFO01 | OK | @L3 ERLEDIGT (QEMU), 2026-10-07 — Quittung<SYSINFO01-STATUS-DONE> · Prioritaet: P2 · Typ: NewTOS-Erweiterung (Accessory… | P2 | DONE | 2 / 060f6e3b4 2026-10-07 / 1 | - | 2026-10-07: ## Anforderungen des Anwenders (2026-10-07) | n/a |
| SYSINFO02 | OK | @L3 ERLEDIGT (QEMU), 2026-10-08, ein offener Punkt (Esc, Kernel-AES) — Quittung<SYSINFO02-STATUS-DONE> · Prioritaet: P2… | P2 | DONE | 2 / 1fc6a03d3 2026-10-08 / 1 | - | 2026-10-07: ## Anforderungen des Anwenders (2026-10-07/08) | n/a |
| T01 | OK | @L3 ERLEDIGT (2026-09-17) · Prioritaet: P1 · Typ: Belegter Defekt / LP64-Struktur | P1 | DONE | 4 / 00b1e2659 2026-09-17 / 1 | - | none found | n/a |
| T02 | OK | @L3 ERLEDIGT (2026-09-16) · Prioritaet: P1 · Typ: Werkzeugkette / stille Luecke | P1 | DONE | 5 / 3e63243b4 2026-09-16 / 1 | - | none found | n/a |
| T03 | OK | @L3 ERLEDIGT (2026-09-16) · Prioritaet: P1 · Typ: Werkzeugkette / Bauabhaengigkeit | P1 | DONE | 5 / 3e63243b4 2026-09-16 / 1 | - | none found | n/a |
| T04 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Sichtabnahme offen (TeraDesk-Union) · Prioritaet: P2 · Typ: Werkzeugkette / LP64-ABI-Fra… | P2 | DONE | 4 / c5fff3be4 2026-11-06 / 2 | - | none found | n/a |
| T05 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P3 (latent) · Typ: Lader / Linkerskript / Basepage-Vertrag | P3 | DONE | 3 / fe30bfc17 2026-10-23 / 1 | - | none found | n/a |
| T06 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P3 · Typ: Housekeeping / Werkzeugkette | P3 | DONE | 3 / 99cbcdab6 2026-10-08 / 1 | - | none found | n/a |
| T07 | OK | @L3 ERLEDIGT (2026-09-21) · Prioritaet: P2 · Typ: Werkzeugkette / Bibliotheksluecke | P2 | DONE | 2 / f6382b246 2026-09-21 / 1 | - | none found | n/a |
| T12 | OK | @L3 ERLEDIGT (2026-10-01, Statusdatei<T12-STATUS-DONE>) · Prioritaet: P2 · Typ: Pruefstand — Nebenfund aus REF04<REF04> | P2 | DONE | 3 / 17bbd8aac 2026-10-01 / 0 | - | none found | n/a |
| T14 | OK | @L3 ERLEDIGT (2026-10-01, Statusdatei<T14-STATUS-DONE>) · Prioritaet: P2 · Typ: Pruefstand oder Produktfehler (ungeklae… | P2 | DONE | 3 / fa95e0c0d 2026-10-01 / 0 | - | none found | n/a |
| T15 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Pruefstand — Nebenfund aus T14<T14> | P3 | - | 1 / fa95e0c0d 2026-10-01 / 0 | L175:P34:open | none found | n/a |
| T16 | OK | @L3 ERLEDIGT 2026-10-16 (Quittung<T16-STATUS-DONE>) · Prioritaet: P3 · Typ: Messwerkzeug — Nebenfund DESK43 U9-3..5 (20… | P3 | DONE | 3 / e52bc0d38 2026-10-16 / 1 | - | none found | n/a |
| T17 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Testharness — Nebenbefund CON04 (2026-10-22) | P3 | - | 1 / 49249a2d3 2026-10-22 / 0 | - | none found | n/a |
| TOOL01 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: blockiert keine Abnahme) · Prioritaet: P2 · Typ: Werkzeug (Reprod… | P2 | - | 2 / 9ef372a56 2026-10-31 / 0 | L91:WQ/Geparkt:parked, L159:P2:open | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: blockiert keine Abnahme) · Prioritaet: P2 ·  | yes (status line): (Anwender, Vor-Start-Pruefung: blockiert keine Abnahme) |
| TOOL02 | OK | @L3 ERLEDIGT 2026-10-27 — Quittung<TOOL02-STATUS-DONE> · Prioritaet: P2 · Typ: Werkzeug (Reproduzierbarkeit) — Anwender… | P2 | DONE | 3 / c4f9771b9 2026-10-27 / 1 | - | 2026-10-27: - Anwenderentscheidung 2026-10-27: „mkcard.sh: must write the same | n/a |
| TOOL03 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: nur fuer DUMPGDOS/FS19 noetig) · Prioritaet: P2 · Typ: Werkzeug (… | P2 | - | 3 / 9ef372a56 2026-10-31 / 0 | L86:WQ/Geparkt:parked, L159:P2:open | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: nur fuer DUMPGDOS/FS19 noetig) · Prioritaet: P | yes (status line): (Anwender, Vor-Start-Pruefung: nur fuer DUMPGDOS/FS19 noetig) |
| TOOL06 | OK | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL06-STATUS-DONE>) · Prioritaet: P1 · Typ: Werkzeug (Orakel) — Anwenderentscheidung… | P1 | DONE | 4 / b81d0ddcd 2026-10-28 / 1 | - | 2026-10-28: - Anwenderentscheidung 2026-10-28: ein zweites, eigenes Orakelprofil fuer | n/a |
| TOOL07 | OK | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL07-STATUS-DONE>) · Prioritaet: P1 · Typ: Werkzeug (Messung) — Anwenderentscheidun… | P1 | DONE | 5 / 5f105c39c 2026-10-28 / 2 | L44:CS:none | 2026-10-28: ## Entscheidungen des Anwenders 2026-10-28 (Rest) | n/a |
| TOOL08 | OK | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL08-STATUS-DONE>) · Prioritaet: P1 · Typ: Werkzeug (Eingabepruefung) — Anwenderent… | P1 | DONE | 4 / 40efc473b 2026-10-28 / 1 | - | 2026-10-28: ## Entscheidung des Anwenders 2026-10-28 | n/a |
| TOOL09 | OK | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL09-STATUS-DONE>) · Prioritaet: P2 · Typ: Werkzeug (Messablauf) — Anwenderentschei… | P2 | DONE | 4 / 51b242864 2026-10-28 / 1 | L44:CS:none | 2026-10-28: - Anwenderentscheidung 2026-10-28: Pre-flight 2 nennt den tatsaechlichen | n/a |
| TOOL10 | OK | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL10-STATUS-DONE>) · Prioritaet: P2 · Typ: Werkzeug — Anwenderentscheidung 2026-10-… | P2 | DONE | 3 / a5dc93a14 2026-10-28 / 1 | L44:CS:none | 2026-10-28: Status: ERLEDIGT 2026-10-28 (Quittung<TOOL10-STATUS-DONE>) · Prioritaet: P2 · Typ: Werkzeug — Anw | n/a |
| TOOL11 | OK | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL11-STATUS-DONE>) · Prioritaet: P1 · Typ: Werkzeug (Vergleichsregel) — Befund des … | P1 | DONE | 5 / 16a15bc40 2026-10-28 / 1 | - | none found | n/a |
| TOOL12 | OK | @L3 ERLEDIGT 2026-10-28 (Quittung<TOOL12-STATUS-DONE>) · Prioritaet: P1 · Typ: Developer (Werkzeug) — Anwenderentscheid… | P1 | DONE | 2 / e70700d69 2026-10-28 / 1 | L44:CS:none | 2026-10-28: Status: ERLEDIGT 2026-10-28 (Quittung<TOOL12-STATUS-DONE>) · Prioritaet: P1 · Typ: Developer (Wer | n/a |
| TOOL13 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein anstehender Fix bringt ein Debug-Feature) · Prioritaet: P2 ·… | P2 | - | 2 / 9ef372a56 2026-10-31 / 0 | L82:WQ/Geparkt:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein anstehender Fix bringt ein Debug-Feature) · | yes (status line): (Anwender, Vor-Start-Pruefung: kein anstehender Fix bringt ein Debug-Feature) |
| TOOL14 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Sonden laufen ohne Trace-Betriebsart) · Prioritaet: P2 · Typ: Dev… | P2 | - | 2 / 9ef372a56 2026-10-31 / 0 | L83:WQ/Geparkt:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Sonden laufen ohne Trace-Betriebsart) · Priorita | yes (status line): (Anwender, Vor-Start-Pruefung: Sonden laufen ohne Trace-Betriebsart) |
| TOOL15 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: blockiert keine Abnahme) · Prioritaet: P2 · Typ: Developer (Werkz… | P2 | - | 2 / 9ef372a56 2026-10-31 / 0 | L84:WQ/Geparkt:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: blockiert keine Abnahme) · Prioritaet: P2 ·  | yes (status line): (Anwender, Vor-Start-Pruefung: blockiert keine Abnahme) |
| TOOL16 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<TOOL16-STATUS-DONE>) · Prioritaet: P1 · Typ: Developer (Werkzeug) — Anwenderentscheid… | P1 | DONE | 4 / 47d81a210 2026-10-29 / 1 | - | 2026-10-28: ## Nachtrag Anwender 2026-10-28 | n/a |
| TOOL17 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<TOOL17-STATUS-DONE>); Ermessensfaelle (Gruppe G) zur Bestaetigung beim Anwender · Pri… | P1 | DONE | 11 / 6ddd4496a 2026-10-29 / 2 | L44:CS:none | 2026-10-28: ## Nachtrag Anwender 2026-10-28 | n/a |
| TOOL18 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<TOOL18-STATUS-DONE>); 7 Cookies OPEN beim Anwender · Prioritaet: P2 · Typ: Developer … | P2 | DONE | 3 / b71175ef2 2026-10-29 / 1 | - | 2026-10-28: - Anwender 2026-10-28: messen auf dem gepinnten ROM, Profile `tt`, | n/a |
| TOOL19 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<TOOL19-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (Werkzeug) — Anwenderentscheid… | P2 | DONE | 2 / 6f38e9b9f 2026-10-29 / 1 | L44:CS:none | 2026-10-28: - Anwender 2026-10-28: in Ordnung, wenn (a) die Datei immer aus der | n/a |
| TOOL20 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<TOOL20-STATUS-DONE>); drei Funde zur Triage beim Anwender · Prioritaet: P2 · Typ: Dev… | P2 | DONE | 4 / 31fa7f7d6 2026-10-29 / 2 | - | 2026-10-29: ## Anwenderentscheidung 2026-10-29 | n/a |
| TOOL21 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<TOOL21-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (Werkzeug + Zuordnung) — Anwen… | P2 | DONE | 2 / daa2973ea 2026-10-29 / 1 | L44:CS:none | 2026-10-29: Status: ERLEDIGT 2026-10-29 (Quittung<TOOL21-STATUS-DONE>) · Prioritaet: P2 · Typ: Developer (Wer | n/a |
| TOOL22 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Tabellenpflege, haengt an PROC07) · Prioritaet: P3 · Typ: Develop… | P3 | - | 1 / 6ddd4496a 2026-10-29 / 0 | L109:WQ/EbGeparkt:parked, L133:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Tabellenpflege, haengt an PROC07) · Prioritaet:  | yes (status line): (Anwender, Vor-Start-Pruefung: Tabellenpflege, haengt an PROC07) |
| TOOL23 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Debug-Feature; wieder aufnehmen, wenn ein AES-Fix den Mitschnitt … | P3 | - | 1 / 679c505e6 2026-10-29 / 0 | L110:WQ/EbGeparkt:parked, L133:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Debug-Feature; wieder aufnehmen, wenn ein AES-Fi | yes (status line): (Anwender, Vor-Start-Pruefung: Debug-Feature; wieder aufnehmen, wenn ein AES-Fix den Mitsc… |
| TOOL24 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Debug-Feature, blockiert nichts) · Prioritaet: P3 · Typ: Develope… | P3 | - | 1 / 40e06519c 2026-10-29 / 0 | L111:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Debug-Feature, blockiert nichts) · Prioritaet: * | yes (status line): (Anwender, Vor-Start-Pruefung: Debug-Feature, blockiert nichts) |
| TOOL25 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS07) per Sonde abgenommen) · Prioritaet: P3 … | P3 | - | 1 / 8cc34fa0e 2026-10-29 / 0 | L112:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS07) per Sonde abgenommen)  | yes (status line): (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS07) per Sonde abgenommen) |
| TOOL26 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS16) per Sonde abgenommen) · Prioritaet: P3 … | P3 | - | 1 / f2652440e 2026-10-30 / 0 | L113:WQ/EbGeparkt:parked, L134:WQ:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS16) per Sonde abgenommen)  | yes (status line): (Anwender, Vor-Start-Pruefung: zugehoeriger Fix (BIOS16) per Sonde abgenommen) |
| TOOL27 | OK | @L3 GEPARKT 2026-10-30 (Anwender: wieder aufnehmen mit MIDI/IO01<IO01>; geparkt) · Prioritaet: P3 · Typ: Developer (Wer… | P3 | - | 2 / 04cdc0935 2026-10-30 / 0 | L137:WQ/geparktMIDI:parked | 2026-10-30: ## Geparkt (Anwender, 2026-10-30) | yes (status line): (Anwender: wieder aufnehmen mit MIDI/IO01<IO01>; geparkt) |
| TOOL28 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Handregel in CLAUDE.md deckt es ab) · Prioritaet: P3 · Typ: Devel… | P3 | - | 1 / 0a07db388 2026-10-30 / 0 | L114:WQ/EbGeparkt:parked, L138:WQ/geparktMIDI:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Handregel in CLAUDE.md deckt es ab) · Prioritaet | yes (status line): (Anwender, Vor-Start-Pruefung: Handregel in CLAUDE.md deckt es ab) |
| TOOL29 | OK | @L3 ERLEDIGT 2026-10-30 (Quittung<TOOL29-STATUS-DONE>) · Prioritaet: P2 (klein) · Typ: Developer (Werkzeug) — Anwendere… | P2 | DONE | 4 / 9ef372a56 2026-10-31 / 1 | L44:CS:none, L134:WQ:done | 2026-10-30: ## Anwenderentscheidungen waehrend der Umsetzung (2026-10-30) | n/a |
| TOOL31 | OK | @L3 GEPARKT 2026-11-04 (zugehoeriger Fix A128 per Sonde abgenommen; Werkzeug-Ticket hinter den Produkt-Fixes) · Priorit… | P3 | - | 0 / - / 0 | L121:WQ:none | none found | yes (status line): (zugehoeriger Fix A128 per Sonde abgenommen; Werkzeug-Ticket hinter den Produkt-Fixes) |
| TOOL32 | OK | @L3 ERLEDIGT 2026-11-05 (Quittung<TOOL32-STATUS-DONE>) · Prioritaet: P1 (Anwender 2026-11-05: vor A119<A119>, VDI vor A… | P1 | DONE | 8 / 020bfd16d 2026-11-07 / 2 | L121:WQ:none, L121:WQ:done | 2026-11-05: Status: ERLEDIGT 2026-11-05 (Quittung<TOOL32-STATUS-DONE>) · Prioritaet: P1 (Anwender 2026-11-05: | n/a |
| TOOL35 | OK | @L3 GEPARKT 2026-11-07 (zugehoeriger Fix V34<V34> per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) · Priori… | P3 | - | 1 / 35eeb6b51 2026-11-07 / 0 | L121:WQ:parked | none found | yes (status line): (zugehoeriger Fix V34<V34> per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) |
| TOOL36 | OK | @L3 GEPARKT 2026-11-07 (zugehoeriger Fix V35<V35> per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) · Priori… | P3 | - | 1 / a862f7647 2026-11-07 / 0 | L121:WQ:parked | none found | yes (status line): (zugehoeriger Fix V35<V35> per Sonde abgenommen; Werkzeug-Ticket hinter der VDI-Arbeit) |
| TOOL37 | OK | @L3 GEPARKT 2026-11-08 (Werkzeug-Ticket, blockiert keine Fix-Abnahme; hinter die VDI-Arbeit) · Prioritaet: P3 · Typ: De… | P3 | - | 1 / ad201a574 2026-11-08 / 0 | L121:WQ:parked | none found | yes (status line): (Werkzeug-Ticket, blockiert keine Fix-Abnahme; hinter die VDI-Arbeit) |
| TOOL38 | OK | @L3 GEPARKT (2026-11-08; Laeufe sind durch Wiederholen abnehmbar, blockiert keinen Fix) · Prioritaet: P3 · Typ: Werkzeu… | P3 | - | 1 / 7774bef5c 2026-11-08 / 0 | L121:WQ:none, L121:WQ:parked, L282:RD:parked | none found | yes (status line): ; Laeufe sind durch Wiederholen abnehmbar, blockiert keinen Fix) |
| UART01 | OK | @L3 GEPARKT 2026-10-30 (Anwender: wieder aufnehmen mit MIDI/IO01<IO01>; geparkt) · Prioritaet: P2 · Typ: Messung zuerst… | P2 | - | 2 / 04cdc0935 2026-10-30 / 0 | L136:WQ/geparktMIDI:parked | 2026-10-30: ## Geparkt (Anwender, 2026-10-30) | yes (status line): (Anwender: wieder aufnehmen mit MIDI/IO01<IO01>; geparkt) |
| USB01 | OK | @L3 ERLEDIGT 2026-11-04 (Quittung<USB01-STATUS-DONE>; Anwender 2026-11-04: Rest auf USB03, A127, USB04 und das Testszen… | P1 | DONE | 26 / b0eeee2ee 2026-11-04 / 8 | L32:CS:none, L142:P1:done, L242:PH:none | 2026-11-04: ## Abschluss (Lead-Abgleich, Anwender-Freigabe 2026-11-04) | n/a |
| USB04 | OK | @L3 OFFEN · Prioritaet: P3 (nach 1.0) · Typ: Sammelticket — Rest aus USB01<USB01> (Anwender 2026-11-04) | P3 | - | 1 / b0eeee2ee 2026-11-04 / 0 | L147:P1:open, L178:P34:open, L242:PH:none | 2026-11-04: Status: OFFEN · Prioritaet: P3 (nach 1.0) · Typ: Sammelticket — Rest aus USB01<USB01> (Anwender 2 | n/a |
| V01 | OK | @L3 ERLEDIGT (2026-09-21) — Geraeteparameter, per EmuTOS bewiesen · Priorität: P2 · Typ: Geräteunterschied / Test | P2 | - | 1 / 46947fab1 2026-10-30 / 1 | L165:ERLoQ:done-no-receipt | none found | n/a |
| V03 | OK | @L3 ERLEDIGT (2026-09-21) · Priorität: P2 · Typ: Geräteunterschied / Quellenprüfung | P2 | DONE | 1 / 83d3003e2 2026-09-21 / 1 | - | none found | n/a |
| V11 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Abnahme offen · Priorität: P1 · Typ: Diagnose eines bekannten Defekts | P1 | DONE | 1 / 83d3003e2 2026-09-21 / 1 | L121:WQ:none, L152:P1u:impl-open | none found | n/a |
| V12 | OK | @L3 ERLEDIGT (2026-09-21) — Pi-Abnahme offen · Priorität: P1 · Typ: Bedingte Implementierung | P1 | DONE | 1 / 83d3003e2 2026-09-21 / 1 | L121:WQ:none, L152:P1u:impl-open | none found | n/a |
| V16 | OK | @L3 ERLEDIGT (2026-09-12) · Prioritaet: P1 · Typ: Belegter Defekt / Contract | P1 | DONE | 3 / 6f3883ce7 2026-09-20 / 1 | - | none found | n/a |
| V19 | OK | @L3 ERLEDIGT 2026-11-05 (Quittung<V19-STATUS-DONE>) · Prioritaet: P1 · Typ: Belegter Defekt, gemessen (VDI-DRAW-007, S1… | P1 | DONE | 4 / 87f1684e5 2026-11-05 / 3 | L44:CS:none, L121:WQ:done, L121:WQ:none | none found | n/a |
| V20 | OK | @L3 ERLEDIGT 2026-11-05 (Quittung<V20-STATUS-DONE>) · Prioritaet: P2 · Typ: Belegter Defekt, gemessen (VDI-DRAW-001/002… | P2 | DONE | 4 / 633e81e33 2026-11-06 / 2 | L44:CS:none, L121:WQ:done | none found | n/a |
| V21 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V21-STATUS-DONE>) · Prioritaet: P2 · Typ: Belegter Defekt, gemessen (VDI-DRAW-008) — … | P2 | DONE | 4 / e6ffdedce 2026-11-07 / 3 | L121:WQ:done, L288:RD:none | none found | n/a |
| V22 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V22-STATUS-DONE>) · Prioritaet: P3 · Typ: Belegter Defekt, gemessen (VDI-DRAW-004, 00… | P3 | DONE | 2 / f19a4b898 2026-11-06 / 1 | L121:WQ:done | none found | n/a |
| V23 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V23-STATUS-DONE>) · Prioritaet: P3 · Typ: Belegter Defekt, per Probe gemessen (VDI-DR… | P3 | DONE | 4 / ef3c0b5c7 2026-11-06 / 1 | L121:WQ:done | none found | n/a |
| V24 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V24-STATUS-DONE>) · Prioritaet: P3 · Typ: Belegter Defekt, gemessen (VDI-DRAW-009) — … | P3 | DONE | 3 / 49e1a6f8d 2026-11-06 / 2 | L121:WQ:done | none found | n/a |
| V25 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V25-STATUS-DONE>) · Prioritaet: P3 · Typ: Belegter Defekt, gemessen (VDI-DRAW-010; 01… | P3 | DONE | 3 / 4a8e1ae3e 2026-11-06 / 1 | L44:CS:none, L121:WQ:done | none found | n/a |
| V26 | OK | @L3 ERLEDIGT 2026-11-05 (Quittung<V26-STATUS-DONE>) · Prioritaet: P2 · Typ: Robustheit/Speicherschutz, gelesen, Wirkung… | P2 | DONE | 6 / 7cda128b4 2026-11-06 / 3 | L44:CS:none, L121:WQ:none | 2026-11-05: ## Entscheidung (Anwender, 2026-11-05) | n/a |
| V27 | OK | @L3 OFFEN (2026-11-05) · Prioritaet: P4 · Typ: Robustheit, gelesen — Nebenbefund aus V19<V19> | P4 | - | 2 / f5c3fa903 2026-11-06 / 0 | L121:WQ:none | none found | n/a |
| V28 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V28-STATUS-DONE>) · Prioritaet: P1 · Typ: Belegter Defekt, gemessen (VDI-TEXT-001/002… | P1 | DONE | 2 / 7cda128b4 2026-11-06 / 1 | L44:CS:none, L121:WQ:done | none found | n/a |
| V29 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V29-STATUS-DONE>) · Prioritaet: P2 · Typ: Belegter Defekt, gemessen (VDI-TEXT-003, S2… | P2 | DONE | 2 / f5c3fa903 2026-11-06 / 1 | L44:CS:none, L121:WQ:done | none found | n/a |
| V30 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V30-STATUS-DONE>) · Prioritaet: P2 · Typ: Belegter Defekt, gemessen (VDI-TEXT-004 S2,… | P2 | DONE | 3 / 587ddbd55 2026-11-06 / 1 | L44:CS:none, L121:WQ:done | 2026-11-06: Anwenderentscheidung 2026-11-06 (Auswahl "Alle wie empfohlen"): Zeichen und Unterstreichung bei `vs_clip(0)` l | n/a |
| V31 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<V31-STATUS-DONE>; Fix `17bcaf56`, Tester PASS `b1c35c9b`) · Prioritaet: P3 · Typ: Bel… | P3 | DONE | 7 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L121:WQ:done, L280:RD:none | 2026-11-06: Anwenderentscheidung 2026-11-06 (Auswahl "Alle wie empfohlen"): K14 wie EmuTOS nachbauen — die `XDDA`-Drift be | n/a |
| V32 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V32-STATUS-DONE>) · Prioritaet: P3 · Typ: Belegter Defekt, gemessen ausserhalb der Or… | P3 | DONE | 2 / cc5f476ba 2026-11-06 / 1 | L44:CS:none, L121:WQ:done, L280:RD:none(range) | none found | n/a |
| V33 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V33-STATUS-DONE>) · Prioritaet: P2 · Typ: Stub (CLAUDE.md Regel 2), gemessen (VDI-TEX… | P2 | DONE | 10 / 7cb9d9d17 2026-11-06 / 2 | L44:CS:none, L121:WQ:done, L280:RD:none(range) | 2026-11-06: - Zusatzpunkte des Developers (Anwender 2026-11-06, "alles so lassen"): Scratch-Puffer des Aufrufers ungen | n/a |
| V34 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<V34-STATUS-DONE>; Fix `5f299bf8`, Tester PASS `038463fe`; Folge TOOL35<TOOL35>, KEYS0… | P3 | DONE | 6 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L121:WQ:done, L121:WQ:none, L280:RD:none(range) | none found | n/a |
| V36 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V28-STATUS-DONE>) · Prioritaet: P3 · Typ: Belegter Defekt, gemessen (VDI-TEXT-014) — … | P3 | - | 1 / 7cda128b4 2026-11-06 / 0 | L121:WQ:none, L280:RD:none(range) | none found | n/a |
| V37 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<V37-STATUS-DONE>; Fix `2bf28d71`, Tester PASS `a7a84391`) · Prioritaet: P3 · Typ: Kan… | P3 | DONE | 7 / ae22f4fc4 2026-11-08 / 2 | L44:CS:none, L121:WQ:done, L280:RD:none(range), L289:RD:none | none found | n/a |
| V40 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<V40-STATUS-DONE>; Fix `5be660c2`, Tester PASS `da2aad51`) · Prioritaet: P3 · Typ: Abw… | P3 | DONE | 4 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L121:WQ:done, L280:RD:none, L286:RD:done | none found | n/a |
| V42 | OK | @L3 OFFEN (2026-11-06) · Prioritaet: P4 · Typ: Abweichung, gemessen — aus V30<V30> | P4 | - | 1 / b0d6457dd 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| V43 | OK | @L3 OFFEN (2026-11-06) · Prioritaet: P4 · Typ: Kandidat aus dem Lesen, ungemessen — aus V30<V30> | P4 | - | 0 / - / 0 | L121:WQ:none | none found | n/a |
| V44 | OK | @L3 ERLEDIGT 2026-11-06 (Quittung<V44-STATUS-DONE>) · Prioritaet: P2 · Typ: Regression, gemessen in QEMU — aus V33<V33> | P2 | DONE | 5 / 7cb9d9d17 2026-11-06 / 1 | L44:CS:none, L121:WQ:none, L121:WQ:done | none found | n/a |
| V45 | OK | @L3 OFFEN (2026-11-06) · Prioritaet: P3 · Typ: Abweichung, gemessen, Ursache offen — aus V41<V41> | P3 | - | 0 / - / 0 | L121:WQ:none, L121:WQ:none | none found | n/a |
| V46 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P3 · Typ: Abweichung, gelesen, nicht gemessen — aus `docs/analysis/fbench-function… | P3 | - | 1 / b0d6457dd 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| V47 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P3 · Typ: Messluecke — aus `docs/analysis/fbench-functions-2026-11-07.md` | P3 | - | 1 / b0d6457dd 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| V48 | OK | @L3 OFFEN (2026-11-07) · Prioritaet: P4 · Typ: Messluecke — aus `docs/analysis/fbench-functions-2026-11-07.md` | P4 | - | 1 / b0d6457dd 2026-11-07 / 0 | L121:WQ:none | none found | n/a |
| V49 | OK | @L3 ERLEDIGT 2026-11-07 (Quittung<V49-STATUS-DONE>; Commits `87b2a9fd`, `8403f75b`, Tester PASS `c723f24b`, Pi Fbench `… | P2 | DONE | 12 / df3b95426 2026-11-08 / 1 | L44:CS:none, L121:WQ:done, L280:RD:none, L283:RD:none, L284:RD:none, L284:RD:none | 2026-11-07: Status: ERLEDIGT 2026-11-07 (Quittung<V49-STATUS-DONE>; Commits `87b2a9fd`, `8403f75b`, Tester PASS ` | n/a |
| V50 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<V50-STATUS-DONE>; Commit `3255df27`, Tester PASS `e6763ca4`, Pi `measurements/measure… | P2 | DONE | 10 / 281b078dd 2026-11-08 / 0 | L44:CS:none, L44:CS:none, L121:WQ:none, L280:RD:none(range), L283:RD:none, L284:RD:none | none found | n/a |
| V51 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<V51-STATUS-DONE>; Commits `1197fed7`, `8940791d`, Tester PASS `fe527625`, Pi `measure… | P2 | DONE | 9 / 281b078dd 2026-11-08 / 0 | L44:CS:none, L44:CS:none, L121:WQ:done, L280:RD:none, L284:RD:none | none found | n/a |
| V52 | OK | @L3 GEPARKT 2026-11-07 (Anwender: kein Kartenprogramm und kein Port betroffen, gemlib uebergibt immer den vollen Zaehle… | P4 | - | 2 / 9f7b75577 2026-11-07 / 0 | L44:CS:none, L121:WQ:none, L121:WQ:none | 2026-11-07: Status: GEPARKT 2026-11-07 (Anwender: kein Kartenprogramm und kein Port betroffen, gemlib uebergibt immer  | yes (status line): (Anwender: kein Kartenprogramm und kein Port betroffen, gemlib uebergibt immer den vollen … |
| V53 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<V53-STATUS-DONE>; Fix `4cc7005c`, Tester PASS `c013f4b2`; Folge V56<V56>) · Prioritae… | P3 | DONE | 4 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L121:WQ:done, L280:RD:none, L287:RD:none | none found | n/a |
| V54 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<V54-STATUS-DONE>; Fix `721e0441`, Probe `61989b6d`, Tester PASS `2869dcfc`) · Priorit… | P3 | DONE | 9 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L44:CS:none, L121:WQ:done, L121:WQ:none, L121:WQ:none, L280:RD:none(range), L282:RD:none | 2026-11-08: Kartenprogramme erreichen es ueber AES `graf_mouse`. Anwender 2026-11-08: V54 ist das naechste Ticket. | n/a |
| V55 | OK | @L3 ERLEDIGT 2026-11-10 (Quittung<V55-STATUS-DONE>; Fix `9c72d3ec`, Nachtrag `8ef6cf96` (Bildtakt im IRQ mit dem Kontex… | P4 | DONE | 14 / ad58e69e0 2026-11-10 / 2 | L44:CS:none, L44:CS:none, L56:OA:done, L121:WQ:none, L121:WQ:none, L280:RD:none(range), L281:RD:none, L282:RD:none | 2026-11-08: Entscheidung (Anwender 2026-11-08): (a). Der Zeiger wird wie EmuTOS erst im naechsten Bildtakt gezeichnet  | n/a |
| V56 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<V56-STATUS-DONE>; Fix `559614ed`, Tester PASS `f064a5cf`) · Prioritaet: P2 · Typ: Bel… | P2 | DONE | 5 / ae22f4fc4 2026-11-08 / 1 | L44:CS:none, L121:WQ:none, L280:RD:none, L285:RD:none, L287:RD:none | none found | n/a |
| V57 | OK | @L3 GEPARKT 2026-11-08 (Vor-Start-Pruefung: kein AUTO-Programm der Karte oeffnet eine VDI-Workstation) · Prioritaet: P4… | P4 | - | 1 / ad201a574 2026-11-08 / 0 | L121:WQ:none | none found | yes (status line): (Vor-Start-Pruefung: kein AUTO-Programm der Karte oeffnet eine VDI-Workstation) |
| V59 | OK | @L3 ERLEDIGT 2026-11-08 (Quittung<V59-STATUS-DONE>; Commits `5a3052ec`, `d86b55fa`, Tester PASS `2d58115d`, Pi `measure… | P2 | DONE | 10 / ae22f4fc4 2026-11-08 / 0 | L44:CS:none, L44:CS:none, L55:OA:done, L121:WQ:done, L280:RD:none, L283:RD:none, L283:RD:none | 2026-11-08: Status: ERLEDIGT 2026-11-08 (Quittung<V59-STATUS-DONE>; Commits `5a3052ec`, `d86b55fa`, Tester PASS ` | n/a |
| W01 | OK | @L3 OFFEN · Priorität: P2 · Typ: Werkzeug / GEM-Anwendung | P2 | - | 0 / - / 0 | L159:P2:open | 2026-09-12: Herkunft: Anwenderidee 2026-09-12. Zunächst reiner Betrachter, kein Editor. | n/a |
| X01 | OK | @L3 ERLEDIGT 2026-10-17 — Quittung<X01-STATUS-DONE> · Prioritaet: P2 | P2 | DONE, UNFINISHED | 10 / 7bc047a63 2026-10-29 / 1 | L231:PH:none, L231:PH:none, L237:PH:none | 2026-10-17: Knoepfe. Anwender-Korrektur 2026-10-17: „Quit“ beendet nur TeraDesk | n/a |
| X02 | OK | @L3 OFFEN · Prioritaet: P3 · Typ: Entscheidungsvorlage / Hardware-Recherche (Track B) — Anwenderwunsch 2026-09-21 | P3 | - | 3 / 972455284 2026-09-28 / 0 | - | 2026-09-21: Status: OFFEN · Prioritaet: P3 · Typ: Entscheidungsvorlage / Hardware-Recherche (Track B) — Anwenderwu | n/a |
| X03 | OK | @L3 ERLEDIGT (2026-10-01, Quittung<X03-STATUS-DONE>) · Abnahme vom Lead nachgeprueft (Screendumps beider Boards, Bau, C… | P2 | DONE | 8 / 7bc047a63 2026-10-29 / 3 | - | 2026-10-01: ## Entscheidung (Anwender 2026-10-01) — Bau-Konstante je Board | n/a |
| X04 | OK | @L3 ERLEDIGT 2026-10-17 (QEMU, `f63c39c1`; echte Pi offen) — Quittung<X04-STATUS-DONE> · Prioritaet: P3 · Typ: Fehler —… | P3 | DONE | 6 / 51504c242 2026-10-17 / 1 | L237:PH:none | 2026-10-17: Anwender 2026-10-17: Neustart gibt es auf NewTOS noch gar nicht (auch kein | n/a |
| XBIOS01 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<XBIOS01-STATUS-DONE>), vom Anwender abgenommen (`a32921d5`) · Prioritaet: P1 · Typ: D… | P1 | DONE | 12 / 639c3aa02 2026-10-29 / 3 | L44:CS:none, L45:CS:none | 2026-10-28: ## Entscheidungen des Anwenders 2026-10-28 (E1-E6) | n/a |
| XBIOS02 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<XBIOS02-STATUS-DONE>) · Prioritaet: P1 · Typ: Developer — Anwenderentscheidung 2026-1… | P1 | DONE | 3 / 679c505e6 2026-10-29 / 1 | L44:CS:none, L123:WQ/DevReihe:done | 2026-10-28: Status: ERLEDIGT 2026-10-29 (Quittung<XBIOS02-STATUS-DONE>) · Prioritaet: P1 · Typ: Developer — A | n/a |
| XBIOS04 | OK | @L3 ERLEDIGT 2026-10-30 (Option 1 + Warnung; W01–W04 ACCEPTED) · Prioritaet: P2 · Typ: Developer (nach Researcher-Note)… | P2 | - | 5 / 67e413d67 2026-10-30 / 1 | L44:CS:none, L127:WQ/DevReihe:none, L134:WQ:done, L252:BLK:blocked | 2026-10-30: ## Abnahme (Anwender, 2026-10-30) | n/a |
| XBIOS05 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<XBIOS05-STATUS-DONE>) · Prioritaet: P2 · Typ: Researcher (Profil „ein Port, `maptabsi… | P2 | DONE | 8 / d0d28fda6 2026-10-30 / 2 | L44:CS:none, L45:CS:none, L127:WQ/DevReihe:none(range), L127:WQ/DevReihe:done | 2026-10-30: ## Entscheidung (Anwender, 2026-10-30) | n/a |
| XBIOS06 | OK | @L3 GEPARKT 2026-10-30 (Anwender: wieder aufnehmen mit MIDI/IO01<IO01>; Fix geparkt; Messung `measurements/measurement-… | P2 | - | 5 / 04cdc0935 2026-10-30 / 0 | L127:WQ/DevReihe:none(range), L134:WQ:none | 2026-10-30: ## Geparkt (Anwender, 2026-10-30) | yes (status line): (Anwender: wieder aufnehmen mit MIDI/IO01<IO01>; Fix geparkt; Messung `measurements/measur… |
| XBIOS07 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Doku-Korrektur ohne Nutzer) · Prioritaet: P3 · Typ: Researcher — … | P3 | - | 1 / eb46d6591 2026-10-28 / 0 | L96:WQ/EbGeparkt:parked, L127:WQ/DevReihe:none(range) | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Doku-Korrektur ohne Nutzer) · Prioritaet: P3 | yes (status line): (Anwender, Vor-Start-Pruefung: Doku-Korrektur ohne Nutzer) |
| XBIOS08 | OK | @L3 GEPARKT 2026-10-30 (Anwender: bis ein Port es braucht) · Prioritaet: P2 · Typ: Developer (rohe Felder halten wie Em… | P2 | - | 1 / abc6276aa 2026-10-30 / 0 | L127:WQ/DevReihe:none(range), L134:WQ:none | 2026-10-30: ## Geparkt (Anwender, 2026-10-30) | yes (status line): (Anwender: bis ein Port es braucht) |
| XBIOS09 | OK | @L3 GEPARKT 2026-10-30 (Anwender: bis ein Port es braucht) · Prioritaet: P2 · Typ: Developer — Anwenderentscheidung 202… | P2 | - | 1 / abc6276aa 2026-10-30 / 0 | L127:WQ/DevReihe:none(range), L134:WQ:none | 2026-10-30: ## Geparkt (Anwender, 2026-10-30) | yes (status line): (Anwender: bis ein Port es braucht) |
| XBIOS10 | OK | @L3 GEPARKT 2026-10-30 (Anwender: bis ein Port es braucht) · Prioritaet: P2 · Typ: Developer — Anwenderentscheidung 202… | P2 | - | 4 / ebda3d727 2026-10-30 / 0 | L127:WQ/DevReihe:none(range), L128:WQ/DevReihe:none, L134:WQ:none | 2026-10-30: ## Geparkt (Anwender, 2026-10-30) | yes (status line): (Anwender: bis ein Port es braucht) |
| XBIOS11 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm haengt am Seed-Ursprung (einziger Nutzer Tera… | P3 | - | 0 / - / 0 | L97:WQ/EbGeparkt:parked, L127:WQ/DevReihe:none(range) | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Kartenprogramm haengt am Seed-Ursprung (ein | yes (status line): (Anwender, Vor-Start-Pruefung: kein Kartenprogramm haengt am Seed-Ursprung (einziger Nutze… |
| XBIOS12 | OK | @L3 ERLEDIGT 2026-10-31 (Quittung<XBIOS12-STATUS-DONE>) · Prioritaet: P3 · Typ: Researcher — Anwenderentscheidung 2026-… | P3 | DONE | 2 / 6796787ce 2026-10-31 / 1 | L44:CS:none, L118:WQ:done, L127:WQ/DevReihe:none(range) | 2026-10-31: ## Anwenderentscheidung 2026-10-31 (wieder aufgenommen) | n/a |
| XBIOS13 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Spec-Pflege) · Prioritaet: P3 · Typ: Researcher — Anwendere… | P3 | - | 0 / - / 0 | L99:WQ/EbGeparkt:parked, L127:WQ/DevReihe:none(range) | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: reine Spec-Pflege) · Prioritaet: P3 · Typ: R | yes (status line): (Anwender, Vor-Start-Pruefung: reine Spec-Pflege) |
| XBIOS14 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Orakelgrenze; blockiert keinen Fix) · Prioritaet: P3 · Typ: Resea… | P3 | - | 0 / - / 0 | L100:WQ/EbGeparkt:parked, L127:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: Orakelgrenze; blockiert keinen Fix) · Prioritaet | yes (status line): (Anwender, Vor-Start-Pruefung: Orakelgrenze; blockiert keinen Fix) |
| XBIOS15 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<XBIOS15-STATUS-DONE>), vom Anwender abgenommen (`532cb52e`, wartendes `Bconin(1)`/`Ca… | P2 | DONE | 10 / aac91ba69 2026-10-30 / 4 | L44:CS:none | 2026-10-29: ## Anwenderentscheidung 2026-10-29 (Bedienung von `cargo xtask run`) | n/a |
| XBIOS16 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm ruft Jdisint/Jenabint/Mfpint) · Prioritaet: P3 · Ty… | P3 | - | 1 / 18e8c1152 2026-10-29 / 0 | L101:WQ/EbGeparkt:parked | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm ruft Jdisint/Jenabint/Mfpint) · Pr | yes (status line): (Anwender, Vor-Start-Pruefung: kein Programm ruft Jdisint/Jenabint/Mfpint) |
| XBIOS17 | OK | @L3 GESCHLOSSEN 2026-10-29 (ACCEPTED-Abweichung, Anwender) · Prioritaet: P3 · Typ: Developer — Anwenderentscheidung 202… | P3 | - | 4 / b231947db 2026-10-29 / 1 | - | 2026-10-29: ## Anwenderentscheidung 2026-10-29 | n/a |
| XBIOS18 | OK | @L3 ERLEDIGT 2026-10-29 (Quittung<XBIOS18-STATUS-DONE>) · Prioritaet: P3 · Typ: Developer — Anwenderentscheidung 2026-1… | P3 | DONE | 5 / 5ad25f644 2026-10-30 / 3 | L44:CS:none | 2026-10-29: (Anwender 2026-10-29, keine Erweiterung von ADR-029). | n/a |
| XBIOS19 | OK | @L3 GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm ruft Bioskeys) · Prioritaet: P3 · Typ: Developer — … | P3 | - | 1 / 679c505e6 2026-10-29 / 0 | L102:WQ/EbGeparkt:parked, L133:WQ/DevReihe:none | 2026-10-31: Status: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm ruft Bioskeys) · Prioritaet: P3* | yes (status line): (Anwender, Vor-Start-Pruefung: kein Programm ruft Bioskeys) |
| ZVIEW03 | OK | @L3 ZUKUNFT (Anwender 2026-10-07: „perspektivisch mit einem Cache im jeweiligen Verzeichnis, aber nicht jetzt") · Prior… | P3 | - | 1 / 1bea9039f 2026-10-07 / 0 | - | 2026-10-07: Status: ZUKUNFT (Anwender 2026-10-07: „perspektivisch mit einem Cache im jeweiligen Verzeichnis, aber nich | yes (status line): Anwender 2026-10-07: „perspektivisch mit einem Cache im jeweiligen Verzeichnis, aber nicht… |

## 4. Distinct status words in use today

Leading token of the status line (text after `Status:` and `**`, first word, as written), 657 status lines:

| leading token (as written) | count |
|---|---|
| ERLEDIGT | 358 |
| OFFEN | 104 |
| BACKLOG | 81 |
| GEPARKT | 53 |
| BEHOBEN | 8 |
| GEGENSTANDSLOS | 8 |
| IMPLEMENTIERT | 8 |
| ZURUECKGESTELLT | 6 |
| IDEE | 4 |
| ACCEPTED-Abweichung | 3 |
| GESCHLOSSEN | 3 |
| TEILWEISE | 3 |
| UMGESETZT | 3 |
| ZURUECKGEZOGEN | 3 |
| URSACHE | 2 |
| ZUKUNFT | 2 |
| ABGESCHLOSSEN | 1 |
| A–F | 1 |
| BEFUND | 1 |
| GEKLAERT | 1 |
| Stufe | 1 |
| TEIL | 1 |
| Teil | 1 |
| ZURÜCKGESTELLT | 1 |

Mapped state word (longest-prefix match used by the rules above, after normalising Ü->UE) with count and suffix variants:

| mapped word | count | class | notes |
|---|---|---|---|
| ERLEDIGT | 358 | CLOSED 283, CLOSED-OPENACC 75 | |
| OFFEN | 104 | OPEN 104 | |
| BACKLOG | 81 | BACKLOG 81 | |
| GEPARKT | 53 | PARKED 53 | |
| IMPLEMENTIERT | 8 | IMPL-OPEN 8 | |
| GEGENSTANDSLOS | 8 | CLOSED 8 | |
| BEHOBEN | 8 | CLOSED 5, CLOSED-OPENACC 3 | |
| ZURUECKGESTELLT | 7 | PARKED 7 | |
| IDEE | 4 | IDEE 4 | |
| TEILWEISE | 3 | PARTIAL 3 | |
| ACCEPTED | 3 | CLOSED-OPENACC 2, CLOSED 1 | |
| UMGESETZT | 3 | CLOSED 2, CLOSED-OPENACC 1 | |
| GESCHLOSSEN | 3 | CLOSED 3 | |
| ZURUECKGEZOGEN | 3 | CLOSED 3 | |
| TEIL 1 | 2 | PARTIAL 2 | |
| ZUKUNFT | 2 | ZUKUNFT 2 | |
| BEFUND ERLEDIGT | 1 | PARTIAL 1 | |
| A–F | 1 | PARTIAL 1 | |
| URSACHE BELEGT | 1 | OPEN 1 | |
| ABGESCHLOSSEN | 1 | CLOSED 1 | |
| URSACHE GEFUNDEN UND BEHOBEN | 1 | CLOSED-OPENACC 1 | |
| GEKLAERT | 1 | CLOSED 1 | |
| STUFE 1 | 1 | PARTIAL 1 | |

Spelling variants and qualifiers observed:
- ZURUECKGESTELLT (ASCII) 6; ZURÜCKGESTELLT (umlaut) 1 (R01 only is the umlaut form, as in README.md:91).
- Date placement: `ERLEDIGT (YYYY-MM-DD)` 139; `ERLEDIGT YYYY-MM-DD` (no brackets) 11; bare `ERLEDIGT` 189; `ERLEDIGT (QEMU…)` 7.
- Acceptance qualifiers after a closed word (CLOSED-OPENACC, 82 rows): `— Pi-Sichtabnahme offen`, `— Pi-Abnahme offen`, `— Hardwareabnahme offen`, `— HW OFFEN`, `— HW-Abnahme offen`, `— drei Pi-Gegenproben offen` …
- `IMPLEMENTIERT` forms: `— HW OFFEN`, `— HW-Abnahme offen`, `— ABNAHME OFFEN (A33)`, `(date) — anwendungsseitige Abnahme offen`, `(date) — Tester-/Hardwareabnahme offen`, bare.
- Multi-word / composite lines (class PARTIAL or special): `TEIL 1 ERLEDIGT … TEIL 2 …`, `Teil 1 BEHOBEN … Teil 2 GEMESSEN …`, `Stufe 1 erledigt, ADR-Entscheidung offen`, `A–F, H, I, J ERLEDIGT`, `BEFUND ERLEDIGT, ABNAHME OFFEN`, `URSACHE GEFUNDEN UND BEHOBEN … Abnahme blockiert`, `URSACHE BELEGT, Fix offen`, `ABGESCHLOSSEN ALS BLOCKED`, `GEKLAERT — Ursache getrennt`, `ZURUECKGESTELLT — to be tested`, `OFFEN (Erweiterung)`, `OFFEN — braucht den Lead / den Anwender / Vorrecherche`, `GEPARKT — …`.
- Mixed language: status line prose is German; the `Priority:`/`Type:` field names are English in DESK57 (`Priority: P2`) and German elsewhere (`Priorität` with umlaut vs `Prioritaet` ASCII).
- Priority values as written: P1 108, P2 198, P2, 1, P3 307, P4 40, none 2, — 1.
- Receipt states present: DONE 400, UNFINISHED 19, BLOCKED 1 (FAILURE: 0; the sub-step receipts include `-PLAN-STATUS-DONE`: `DESK43-PLAN`).

## 5. Could not determine

- Meaning of "closed" is not defined by `TASK/README.md`; the mapping in section 1 is an assumption. Whether UMGESETZT/ACCEPTED-Abweichung/GEGENSTANDSLOS/ZURUECKGEZOGEN/GEKLAERT count as closed for the owner is not determinable from the files.
- Whether a sub-step receipt (`DESK43-U*`, `ARCH03-*`, `FS08-*`, `MEM05-A/B`, `LDG-*`, `ZVIEW-M4-*`, `FIX-0*`…) means the parent ticket is complete: not determinable; the rules do not count it for C1 (it does count as evidence for C2).
- Commit match limits: ids written with a trailing hyphen segment (`A25-…`, `DESK43-U5a`) or abbreviated ranges (`A119–A125` counts only the written ids) are not matched; history was rewritten on 2026-10-19 (`git filter-repo`), hashes cited inside tickets/docs before that date are old hashes (map `docs/commit-map.txt`, `tools/oldhash.sh`) — only current `git log` was used, no hash cited in a ticket was translated. Commit date is the author date.
- Whether a commit that names an id actually fixes it: only the subject prefix `fix`/`feat` was checked (not the diff); commits with prefix `docs`/`test`/`chore` that close a ticket are not counted as fix commits (they count in the commits column).
- C3 is keyword-based on a ≤110-character window; claims with several ids per window, negations ("noch nicht erledigt") and tables are not parsed. Section assignment of a mention inside the long lines 121 (queue) and 53–61 is by the nearest preceding sub-marker; a `/?` or missing sub-list means not mapped. The `claim` for each mention is in Appendix B; no manual spot-check of all rows was made.
- Last owner decision: regex-based (section 1). 432 status-line tickets have no match ("none found"); for tickets whose decision is phrased without the word `Anwender` (e.g. `Lead`, `user`, `Benutzerentscheidung`) the cell says "none found" although a decision may exist.
- Parked reason "body only" rows: the matched body line is a keyword hit, not verified to be the reason.
- C6 (c) covers only ids in the first 12 lines of a ticket with the listed keywords; renames described elsewhere in prose, or without those words, are not detected. No FS20-B ticket, receipt or message exists, so the example rename FS20-B -> FS22 cannot be confirmed from `TASK/` or git.
- Receipt content (what a DONE receipt says, whether it really covers the ticket) was not read; only file existence and name.
- `TASK/evidence` (12 entries) and `TASK/tasks.json` were listed, not analysed; `tasks.json` is the machine-readable task list of the original 2026-09-09 package (README.md:11) and was not compared with the tickets.
- `.DS_Store` in `TASK/` is untracked (git ls-files: 0) and not a row.

## Appendix A. Receipt anomalies

Orphan receipts (receipt id with no ticket file and no ancestor ticket): 19 ids.

- `ACCTITLE`: TASK/ACCTITLE-STATUS-DONE.MD
- `FIX-01`: TASK/FIX-01-STATUS-DONE.MD
- `FIX-02`: TASK/FIX-02-STATUS-DONE.MD
- `FIX-03`: TASK/FIX-03-STATUS-DONE.MD
- `FIX-04`: TASK/FIX-04-STATUS-DONE.MD
- `FIX-05`: TASK/FIX-05-STATUS-DONE.MD
- `FIX-06`: TASK/FIX-06-STATUS-DONE.MD
- `LDG-M1`: TASK/LDG-M1-STATUS-DONE.MD
- `LDG-M2`: TASK/LDG-M2-STATUS-DONE.MD
- `LDG-M3`: TASK/LDG-M3-STATUS-DONE.MD, TASK/LDG-M3-STATUS-UNFINISHED.MD
- `LDG-M3-B1`: TASK/LDG-M3-B1-STATUS-DONE.MD
- `LDG-M3-B2`: TASK/LDG-M3-B2-STATUS-DONE.MD
- `LIBC-MAIN`: TASK/LIBC-MAIN-STATUS-DONE.MD
- `PORT-WAITH`: TASK/PORT-WAITH-STATUS-DONE.MD
- `PPARENT`: TASK/PPARENT-STATUS-DONE.MD
- `R3-TABELLE`: TASK/R3-TABELLE-STATUS-DONE.MD
- `V-SERIES`: TASK/V-SERIES-STATUS-DONE.MD
- `ZVIEW-M4`: TASK/ZVIEW-M4-STATUS-DONE.MD, TASK/ZVIEW-M4-STATUS-UNFINISHED.MD
- `ZVIEW-M4-K1`: TASK/ZVIEW-M4-K1-STATUS-DONE.MD

Range / sub-step receipts per parent (parent exists):

- ARCH03: ARCH03-0c:DONE, ARCH03-0d:DONE, ARCH03-1:DONE, ARCH03-2:DONE, ARCH03-3a:DONE, ARCH03-3b:DONE, ARCH03-3b:UNFINISHED, ARCH03-3c:DONE, ARCH03-3c:UNFINISHED, ARCH03-4:DONE, ARCH03-E9:DONE, ARCH03-POSTE9:DONE, ARCH03-POSTE9:UNFINISHED
- BIOS01: BIOS01-S2:DONE
- DESK43: DESK43-PLAN:DONE, DESK43-U1:DONE, DESK43-U2:DONE, DESK43-U3:DONE, DESK43-U4:DONE, DESK43-U5a:DONE, DESK43-U5b:DONE, DESK43-U6a:DONE, DESK43-U6b:DONE, DESK43-U7:DONE, DESK43-U8a:DONE, DESK43-U8b:DONE, DESK43-U9-0:DONE, DESK43-U9-1-2:DONE, DESK43-U9-3-5:DONE
- FS04: FS04-EMMC:DONE, FS04-FIX:DONE
- FS08: FS08-CPUCLK:DONE, FS08-EMMC:DONE, FS08-FAT:DONE, FS08-STDIO:DONE
- FS11: FS11-AUX:DONE
- MEM05: MEM05-A:DONE, MEM05-B:DONE
- PORT06: PORT06-EDDI:DONE
- PORT12: PORT12-1:DONE
- R02: R02-R05:DONE(range)
- R05: R02-R05:DONE(range)
- SDK04: SDK04-C6:DONE
- SND01: SND01-SPEC:DONE, SND01-TRACKB:DONE

## Appendix B. STATUS.md mentions with quotes

Format: `id` — `L<line>` section/sub, claim, quote (about 25 characters before the id, about 70 after). Only ids with at least one mention.

**A06** (ticket class CLOSED)
- L163 ERLoQ, done-no-receipt: ` Anwender 2026-10-26): A06, `
**A08** (ticket class CLOSED)
- L163 ERLoQ, done-no-receipt: `nder 2026-10-26): A06, A08, `
**A09** (ticket class CLOSED)
- L163 ERLoQ, done-no-receipt: `2026-10-26): A06, A08, A09, `
**A115** (ticket class CLOSED-OPENACC)
- L232 PH, none: `h EmuTOS 'rlr'/'drl' mit A115 ('e7754b16') / 2026-10-25 / 'phases/PHASE-06-bdos-process.md', ADR-02`
**A116** (ticket class CLOSED-OPENACC)
- L163 ERLoQ, done-no-receipt: `): A06, A08, A09, A26, A116, `
**A118** (ticket class CLOSED-OPENACC)
- L239 PH, none: `ng in A-Tickets, zuletzt A118 / 2026-10-26 / 'phases/PHASE-13-aes.md' /`
**A119** (ticket class OPEN)
- L121 WQ, none: `ann EmuTOS-Konformitaet (A119–`
- L177 P34, open: ` FS16, FS17, LIBC-IOCTL, A119–`
**A120** (ticket class OPEN)
- L121 WQ, none (range): `ann EmuTOS-Konformitaet (A119–A125, `
- L177 P34, open (range): ` FS16, FS17, LIBC-IOCTL, A119–A125, `
**A121** (ticket class OPEN)
- L121 WQ, none (range): `ann EmuTOS-Konformitaet (A119–A125, `
- L177 P34, open (range): ` FS16, FS17, LIBC-IOCTL, A119–A125, `
**A122** (ticket class OPEN)
- L121 WQ, none (range): `ann EmuTOS-Konformitaet (A119–A125, `
- L177 P34, open (range): ` FS16, FS17, LIBC-IOCTL, A119–A125, `
**A123** (ticket class OPEN)
- L121 WQ, none (range): `ann EmuTOS-Konformitaet (A119–A125, `
- L177 P34, open (range): ` FS16, FS17, LIBC-IOCTL, A119–A125, `
**A124** (ticket class OPEN)
- L121 WQ, none (range): `ann EmuTOS-Konformitaet (A119–A125, `
- L177 P34, open (range): ` FS16, FS17, LIBC-IOCTL, A119–A125, `
**A125** (ticket class OPEN)
- L121 WQ, none: `muTOS-Konformitaet (A119–A125, `
- L177 P34, open: `, FS17, LIBC-IOCTL, A119–A125, `
**A126** (ticket class CLOSED)
- L44 CS, none: `sk-Tests; davor 1899 mit A126, +4 Tests; davor 1895 mit `
- L115 WQ/EbGeparkt, done: `0.md) (zugehoeriger Fix (A126) per Sonde abgenommen),`
- L134 WQ, done: ` Commit der Quittung) → [A126 (umgesetzt 2026-10-30, Commit dieser Zeile) → `
- L134 WQ, none: `, Commit dieser Zeile), [A126; `
- L134 WQ, none: `FS28](../TASK/FS28.md), [A126 (`
**A127** (ticket class OPEN)
- L121 WQ, none: `Anwender 2026-11-04): [A127 (Ursachenforschung 2026-11-04 ohne Nachstellung, 'docs/analysis/a127-`
- L145 P1, open: `  [A127 (neu: Menueeintraege wirken erst nach mehreren`
**A128** (ticket class CLOSED-OPENACC)
- L121 WQ, done: `Ragnars Rueckmeldung) → [A128 (Dateiauswahl: Schieber wie EmuTOS, Rad als Ergaenzung; GitHub #9; um`
**A129** (ticket class CLOSED)
- L121 WQ, done: `L31.md), P3, geparkt) → [A129 (erledigt 2026-11-04: Pixel EmuTOS-gleich, kein Fehler) → `
**A130** (ticket class CLOSED)
- L121 WQ, none: `ht 2026-11-04 ans Ende: [A130 (#2 Fenster-Rollbalken, `
**A131** (ticket class CLOSED)
- L121 WQ, none: `130-card/RUNINFO.TXT'), [A131 (#7 Fensterschatten, `
**A132** (ticket class OPEN)
- L121 WQ, done: `-gleich, kein Fehler) → [A132 (umgesetzt 2026-11-04, 'be3c06a9', Tester PASS; Maske im Titel der Da`
- L121 WQ, none: `Tester PASS zusammen mit A132 'measurements/measurement-2026-11-04-a133-test/RUNINFO.TXT'; TEDINFO-`
**A133** (ticket class OPEN)
- L121 WQ, done: `tel der Dateiauswahl) → [A133 (umgesetzt 2026-11-04, 'fd6c7e3f', Tester PASS zusammen mit `
**A134** (ticket class OPEN)
- L121 WQ, none: `-down-Box wie EmuTOS) → [A134 (neu, `
**A135** (ticket class OPEN)
- L121 WQ, none: `bmodus des Farbworts) → [A135 (neu, `
**A136** (ticket class OPEN)
- L121 WQ, none: `: 'G_BOXCHAR' ebenso) → [A136 (neu, `
**A137** (ticket class OPEN)
- L121 WQ, none: `](../TASK/V43.md) (P4), [A137, `
**A138** (ticket class OPEN)
- L121 WQ, none: `A137](../TASK/A137.md), [A138, `
**A139** (ticket class OPEN)
- L121 WQ, parked: `A138](../TASK/A138.md), [A139, zuerst messen. Aus der Pi-Abnahme 0.7.1 (2026-11-07) geparkt: `
**A140** (ticket class CLOSED)
- L45 CS, none: `vor 36 am 2026-11-07 mit A140, Developer, +1 'hypview-win-host'; davor 35 am 2026-10-30 mit `
- L121 WQ, done: `71](../TASK/REL071.md): [A140 (#22; umgesetzt 2026-11-07, Patch 'sdk/overlays/hypview/dl_win.c', 'm`
**A141** (ticket class CLOSED)
- L44 CS, none: `r 2241 am 2026-11-07 mit A141, Developer und Tester, +2 'rsc_recolour_tests'; davor 2239 am 2026-11`
- L121 WQ, none: `; Tester und Pi offen), [A141 (#23; gemessen 2026-11-07: EmuTOS-gleich, Ursache in der Ressource, E`
**A142** (ticket class CLOSED)
- L45 CS, none: `(gemessen 2026-11-07 mit A142, Developer, +2 'inline-rsh-host', 'hypview-prepare-host'; davor 36 am`
- L121 WQ, none: `B1/B2/C beim Anwender), [A142 (#21; Bytefolge in cflibs Bildern, andere Wurzel, offen), Paket und P`
**A143** (ticket class OPEN)
- L121 WQ, none: `d) (P4, 4096+ Zeichen), [A143, `
**A144** (ticket class OPEN)
- L121 WQ, none: `A143](../TASK/A143.md), [A144; aus dem Fbench-Bericht 'docs/analysis/fbench-functions-2026-11-07.md`
**A145** (ticket class OPEN)
- L121 WQ, none: ` V47<../TASK/V47>, [A145, `
**A146** (ticket class PARKED)
- L121 WQ, none: `1 (2026-11-07) geparkt: [A146 (Frage, Recherche: Abstand zwischen Fensterrahmen und Schatten). Aus `
**A147** (ticket class OPEN)
- L121 WQ, none: ` '2869dcfc'; Nebenfunde [A147, `
- L121 WQ, none: `. Aus V54 (2026-11-08): [A147 ('graf_mouse(USER_DEF)' Farben und Hotspot, gelesen, zuerst messen), `
- L282 RD, none: `1, Delta 0. Nebenfunde: [A147 ('graf_mouse(USER_DEF)' Farben/Hotspot, gelesen), `
**A148** (ticket class PARKED)
- L121 WQ, none: `t V54/V55 (2026-11-08): [A148 (`
**A16** (ticket class IMPL-OPEN)
- L121 WQ, none: `der Pi offenen Abnahmen (A16, `
- L151 P1u, impl-open: `  A16, `
**A26** (ticket class CLOSED)
- L163 ERLoQ, done-no-receipt: `10-26): A06, A08, A09, A26, `
**A37** (ticket class IMPL-OPEN)
- L121 WQ, none: `i offenen Abnahmen (A16, A37, `
- L151 P1u, impl-open: `  A16, A37 (anwendungsseitig), `
**A45** (ticket class CLOSED)
- L148 P1, open: `.md) Rest (siehe oben). [A45 ist`
- L254 BLK, blocked: `- A45: 'appl_tplay' und 'appl_trecord' sind ohne die AES-Forkqueue nicht`
- L274 BLK, blocked: `ein Aufruf. Ticket 'TASK/A45.md'.`
**ACC07** (ticket class CLOSED)
- L163 ERLoQ, done-no-receipt: `06, A08, A09, A26, A116, ACC07,`
**ARCH03** (ticket class CLOSED)
- L30 CS, none: `8 mit Speicherschutz an (ARCH03 bis zu den`
- L35 CS, done: `ARCH03 ist umgesetzt und abgenommen (ADR-028, VA = PA; Tester auf 'main'`
- L38 CS, none: `[ARCH03: Stufe 2b ('Pexec' 100+), 'asid=on' auf echter Pi.`
- L121 WQ, none: `eils aufeinander auf) → [ARCH03-Rest (Vorschlag 2026-11-05, nicht bestaetigt: Stufe 2b nach 1.0, 'asi`
- L148 P1, open: `  [ARCH03 Rest (siehe oben). `
- L174 P34, open: `WEISE, 3 IDEE. Darunter: ARCH03 Stufe 2b, `
- L232 PH, none: `REE'/'BASEPAGEFLAGS' mit ARCH03 Stufe 2 ('32c6171c'), Scheduler nach EmuTOS 'rlr'/'drl' mit `
**ARCH04** (ticket class OPEN)
- L178 P34, open: `  CON03, ARCH04, `
**ARCH05** (ticket class CLOSED)
- L128 WQ/DevReihe, none: `ch ADR-031, XBIOS10 nach ARCH05) →`
**ARCH06** (ticket class CLOSED)
- L44 CS, none: `+5 Tests; davor 1828 mit ARCH06, +4 Tests; davor 1824 mit `
**BIOS01** (ticket class CLOSED)
- L44 CS, none: `+2 Tests; davor 1840 mit BIOS01 Rest, +7 Tests; davor 1833 mit `
- L44 CS, none: `+7 Tests; davor 1760 mit BIOS01 `
**BIOS04** (ticket class CLOSED)
- L44 CS, none: `+3 Tests; davor 1847 mit BIOS04, +4 Tests; davor 1843 mit `
- L124 WQ/DevReihe, done: `  [BIOS04 (erledigt 2026-10-29, Commit der Quittung) →`
**BIOS05** (ticket class CLOSED-OPENACC)
- L127 WQ/DevReihe, none: `29, Commit der Quittung; BIOS05 und `
**BIOS06** (ticket class CLOSED)
- L44 CS, none: ` +1 Test; davor 1842 mit BIOS06, +2 Tests; davor 1840 mit `
**BIOS07** (ticket class CLOSED)
- L44 CS, none: `ead 1872; davor 1859 mit BIOS07, +9 Tests; davor 1850 mit `
- L112 WQ/EbGeparkt, done: `5.md) (zugehoeriger Fix (BIOS07) per Sonde abgenommen),`
- L126 WQ/DevReihe, done: ` uebrigen Audit-Tickets [BIOS07 (erledigt 2026-10-29, Commit der Quittung),`
**BIOS08** (ticket class CLOSED)
- L44 CS, none: `+4 Tests; davor 1888 mit BIOS08, +7 Tests; davor 1881 mit `
- L127 WQ/DevReihe, none: `  BIOS08–`
- L127 WQ/DevReihe, done: `9, Commit der Quittung; [BIOS08 erledigt 2026-10-30, Commit der Quittung), `
**BIOS09** (ticket class CLOSED)
- L44 CS, none: ` +1 Test; davor 1894 mit BIOS09, +1 Test; davor 1893 mit `
- L121 WQ, none: `SER01, V11, V12, DESK43, BIOS09, `
- L127 WQ/DevReihe, none (range): `  BIOS08–BIOS14 (`
- L134 WQ, done: ` Commit der Quittung) → [BIOS09 (umgesetzt 2026-10-30, Commit der Quittung; offen der Pi-Schritt in d`
**BIOS10** (ticket class CLOSED)
- L127 WQ/DevReihe, none (range): `  BIOS08–BIOS14 (`
- L127 WQ/DevReihe, done: `  BIOS08–BIOS14 ([BIOS10 erledigt 2026-10-29, Commit der Quittung; `
**BIOS11** (ticket class CLOSED)
- L127 WQ/DevReihe, none (range): `  BIOS08–BIOS14 (`
**BIOS12** (ticket class CLOSED)
- L118 WQ, done: `). Wartet auf Anwender: [BIOS12 erledigt 2026-10-31 (Quittung). `
- L127 WQ/DevReihe, none (range): `  BIOS08–BIOS14 (`
**BIOS13** (ticket class PARKED)
- L94 WQ/EbGeparkt, parked: `  [BIOS13 (reine Doku-/Ledger-Pflege; kein Programm, kein offener Fix),`
- L127 WQ/DevReihe, none (range): `  BIOS08–BIOS14 (`
**BIOS14** (ticket class PARKED)
- L95 WQ/EbGeparkt, parked: `  [BIOS14 (Werkzeugluecke; blockiert keinen offenen Fix),`
- L127 WQ/DevReihe, none: `  BIOS08–BIOS14 (`
**BIOS15** (ticket class CLOSED)
- L44 CS, none: `+8 Tests; davor 1808 mit BIOS15, +18 Tests; davor 1790 mit `
**BIOS16** (ticket class OPEN)
- L44 CS, none: ` +1 Test; davor 1892 mit BIOS16, +4 Tests; davor 1888 mit `
- L113 WQ/EbGeparkt, done: `6.md) (zugehoeriger Fix (BIOS16) per Sonde abgenommen),`
- L134 WQ, done: `./TASK/TOOL25.md) (P3); [BIOS16 (erledigt 2026-10-30, '9c4c29c5') → `
**BIOS17** (ticket class OPEN)
- L44 CS, none: ` +1 Test; davor 1893 mit BIOS17, +1 Test; davor 1892 mit `
- L134 WQ, done: `26-10-30, '9c4c29c5') → [BIOS17 (erledigt 2026-10-30, Commit der Quittung) → `
**CON01** (ticket class CLOSED-OPENACC)
- L164 ERLoQ, done-no-receipt: `  CON01, `
**CON02** (ticket class CLOSED)
- L164 ERLoQ, done-no-receipt: `  CON01, CON02, `
**CON03** (ticket class OPEN)
- L178 P34, open: `  CON03, `
**CON04** (ticket class CLOSED)
- L32 CS, none: `.6.1, USB01 F1/F2/F8 und CON04 (`
- L236 PH, none: `tung in 'Bconout(2)' mit CON04 / 2026-10-23 / 'phases/PHASE-10-bios-screen.md', `
- L236 PH, none: `ASE-10-bios-screen.md', [CON04 /`
**COOK01** (ticket class PARKED)
- L179 P34, open: `messen); zurueckgestellt COOK01; Ideen `
**COOK02** (ticket class PARKED)
- L44 CS, none: `uer Test; davor 1895 mit COOK02 '_IDT', kein neuer Test; davor 1895 mit `
- L132 WQ/DevReihe, done: `07](../TASK/PROC07.md), [COOK02 (geparkt; Punkt 3 '_IDT' = Orakel umgesetzt 2026-10-30, Commit der Qu`
**DESK34** (ticket class IMPL-OPEN)
- L121 WQ, none: `enen Abnahmen (A16, A37, DESK34, `
- L151 P1u, impl-open: ` A37 (anwendungsseitig), DESK34, `
**DESK35** (ticket class IMPL-OPEN)
- L121 WQ, none: `ahmen (A16, A37, DESK34, DESK35, `
- L151 P1u, impl-open: `wendungsseitig), DESK34, DESK35, `
**DESK39** (ticket class CLOSED)
- L164 ERLoQ, done-no-receipt: `  CON01, CON02, DESK39, `
**DESK40** (ticket class OPEN)
- L121 WQ, none: `Konformitaet (A119–A125, DESK40/49/51–55/61/63/67–69, `
**DESK41** (ticket class CLOSED)
- L29 CS, none: `n Endungen aus TeraDesk (DESK41), Fensteroeffnen (`
**DESK43** (ticket class CLOSED-OPENACC)
- L18 CS, done: `e, ohne Umgehungen. Seit DESK43 (erledigt 2026-10-16, `
- L121 WQ, none: `02–R05, SER01, V11, V12, DESK43, `
- L152 P1u, impl-open: `02–R05, SER01, V11, V12, DESK43 (Pi-Gegenprobe des EmuCON-Rueckfalls bei`
- L167 ERLoQ, done-no-receipt: `  DESK43 (Quittungen bis U9-3-5), `
- L240 PH, none: `s/PHASE-14-desktop.md', [DESK43 /`
**DESK45** (ticket class CLOSED-OPENACC)
- L164 ERLoQ, done-no-receipt: `  CON01, CON02, DESK39, DESK45, `
**DESK47** (ticket class CLOSED)
- L28 CS, none: ` '.NTP'-Parameterdialog (DESK47), die vier`
- L164 ERLoQ, done-no-receipt: `, CON02, DESK39, DESK45, DESK47, `
**DESK49** (ticket class OPEN)
- L176 P34, open: `  DESK49, `
**DESK50** (ticket class CLOSED)
- L29 CS, none: `DESK41), Fensteroeffnen (DESK50), zView;`
**DESK51** (ticket class OPEN)
- L176 P34, open: `  DESK49, DESK51–`
**DESK52** (ticket class OPEN)
- L176 P34, open (range): `  DESK49, DESK51–DESK55, `
**DESK53** (ticket class OPEN)
- L176 P34, open (range): `  DESK49, DESK51–DESK55, `
**DESK54** (ticket class OPEN)
- L176 P34, open (range): `  DESK49, DESK51–DESK55, `
**DESK55** (ticket class OPEN)
- L176 P34, open: `  DESK49, DESK51–DESK55, `
**DESK61** (ticket class OPEN)
- L176 P34, open: `  DESK49, DESK51–DESK55, DESK61, `
**DESK63** (ticket class OPEN)
- L176 P34, open: `, DESK51–DESK55, DESK61, DESK63, `
**DESK65** (ticket class OPEN)
- L176 P34, open: `–DESK55, DESK61, DESK63, DESK65, `
**DESK66** (ticket class OPEN)
- L176 P34, open: ` DESK61, DESK63, DESK65, DESK66, `
**DESK67** (ticket class OPEN)
- L176 P34, open: ` DESK63, DESK65, DESK66, DESK67, `
**DESK69** (ticket class OPEN)
- L176 P34, open: ` DESK65, DESK66, DESK67, DESK69, `
**DESK70** (ticket class CLOSED)
- L121 WQ, none: `131-card/RUNINFO.TXT'), [DESK70 (#12 Textprogramm-Ausgabe, `
**DESK71** (ticket class OPEN)
- L121 WQ, none: `est/RUNINFO.TXT'; Folge [DESK71) → `
**DOOM01** (ticket class PARKED)
- L118 WQ, none: `1.md) (bis Tester K6 aus DOOM01 sehen).`
- L119 WQ, none: `1/RUNINFO.TXT'); Funde: [DOOM01, `
- L120 WQ, none: `ender 2026-11-02), vor DOOM01. Ursachenforschung 2026-11-02 ohne Beleg ('docs/analysis/proc08-root-`
- L139 WQ/Zurueckgest, parked: `  [DOOM01 (DOOM-Freeze im Menue, nur auf der Pi) — erneut zurueckgestellt 2026-`
- L139 WQ/Zurueckgest, parked: `en; naechster Schritt in DOOM01 "Stand 2026-11-02".`
- L141 P1, open: `' ist erster Schritt in [DOOM01 und im spaeteren Testszenario.`
**EMU01** (ticket class IDEE)
- L121 WQ, none: `68k-Emulator mit Line-A [EMU01 (IDEE, nach 1.0), `
**FS02** (ticket class CLOSED-OPENACC)
- L164 ERLoQ, done-no-receipt: ` DESK39, DESK45, DESK47, FS02, `
**FS04** (ticket class CLOSED)
- L27 CS, none: `ueber Partitionsgrenzen (FS04), Doom (40–75 fps, 'kbdvec'`
**FS07** (ticket class CLOSED-OPENACC)
- L164 ERLoQ, done-no-receipt: `9, DESK45, DESK47, FS02, FS07, `
**FS08** (ticket class CLOSED)
- L167 ERLoQ, done-no-receipt: `(Quittungen bis U9-3-5), FS08 (je Teilschritt), `
**FS10** (ticket class CLOSED)
- L234 PH, none: `ssen; Geraetetabelle mit FS10/`
**FS11** (ticket class CLOSED-OPENACC)
- L234 PH, none: ` Geraetetabelle mit FS10/FS11 / 2026-10-13 / 'phases/PHASE-08-bios-block-devices.md' /`
**FS12** (ticket class OPEN)
- L121 WQ, none: `40/49/51–55/61/63/67–69, FS12, `
- L176 P34, open: ` DESK66, DESK67, DESK69, FS12`
**FS14** (ticket class CLOSED-OPENACC)
- L164 ERLoQ, done-no-receipt: `K45, DESK47, FS02, FS07, FS14, `
**FS15** (ticket class OPEN)
- L121 WQ, none: `51–55/61/63/67–69, FS12, FS15–`
- L177 P34, open: `  (26 Laufwerke), FS15, `
**FS16** (ticket class OPEN)
- L121 WQ, none (range): `51–55/61/63/67–69, FS12, FS15–FS17, `
- L177 P34, open: `  (26 Laufwerke), FS15, FS16, `
**FS17** (ticket class OPEN)
- L121 WQ, none: `/61/63/67–69, FS12, FS15–FS17, `
- L177 P34, open: ` Laufwerke), FS15, FS16, FS17, LIBC-IOCTL, `
**FS18** (ticket class CLOSED-OPENACC)
- L164 ERLoQ, done-no-receipt: `ESK47, FS02, FS07, FS14, FS18, `
- L235 PH, none: `09-bdos-filesystem.md', [FS18 /`
**FS19** (ticket class PARKED)
- L86 WQ/Geparkt, parked: `3.md) (nur fuer DUMPGDOS/FS19 noetig), DUMPGDOS (kein`
- L88 WQ/Geparkt, parked: `  [FS19 (nur Programme mit DTA in unabgebildetem Speicher),`
- L179 P34, parked: ` T18<../TASK/T18>, [FS19 ('Fsfirst'-Reihenfolge, erst messen); zurueckgestellt `
**FS20** (ticket class CLOSED)
- L44 CS, none: `16 Tests; davor 1767 mit FS20, +7 Tests; davor 1760 mit `
**FS21** (ticket class CLOSED)
- L44 CS, none: `+4 Tests; davor 1918 mit FS21, +8 Tests; davor 1910 mit `
- L118 WQ, none: `  [FS21 (`
**FS22** (ticket class CLOSED)
- L44 CS, none: `18 Tests; davor 1790 mit FS22, +7 Tests; davor 1783 mit `
**FS23** (ticket class OPEN)
- L44 CS, none: `+9 Tests; davor 1850 mit FS23, +3 Tests; davor 1847 mit `
- L125 WQ/DevReihe, done: `  [FS23 (erledigt 2026-10-29, Commit der Quittung), `
**FS24** (ticket class CLOSED)
- L125 WQ/DevReihe, accepted: `, Commit der Quittung), [FS24 (ACCEPTED unter ADR-033, Anwender 2026-10-30) →`
**FS25** (ticket class PARKED)
- L92 WQ/Geparkt, parked: `  [FS25 (Frage nur ans Orakel).`
**FS26** (ticket class OPEN)
- L121 WQ, none: `11, V12, DESK43, BIOS09, FS26 `
- L129 WQ/DevReihe, none: `  [FS26 (Ordner anlegen ~3 s auf der Pi; Schichtmessung in`
**FS27** (ticket class PARKED)
- L106 WQ/EbGeparkt, parked: `  [FS27 (nur beschaedigte Medien),`
- L134 WQ, none: `24](../TASK/TOOL24.md), [FS27, `
**FS28** (ticket class CLOSED)
- L44 CS, none: `+3 Tests; davor 1907 mit FS28, +2 Tests; davor 1905 mit `
- L134 WQ, parked: `ation durch den Lead) → [FS28 (Anwender 2026-10-30; geparkt bis ein Port sie braucht, weil kein Pro`
- L134 WQ, done: `(../TASK/XBIOS10.md)) → [FS28 (erledigt 2026-10-30, Commit dieser Zeile), `
- L134 WQ, none: `./TASK/TOOL26.md) (P3); [FS28, `
**FS29** (ticket class PARKED)
- L131 WQ/DevReihe, none: ` Commit dieser Zeile) → [FS29 (`
**FS30** (ticket class PARKED)
- L131 WQ/DevReihe, none: `bs'-Sektor-0-Werkzeug), [FS30 (`
**FS31** (ticket class CLOSED)
- L44 CS, none: `r 2217 am 2026-11-06 mit FS31, Developer, +17 Tests; davor 2200 am 2026-11-06 mit `
- L121 WQ, done: `lt, Anwender: defekt) → [FS31 (erledigt 2026-11-06, '20f87d1d', Tester PASS 'measurements/measureme`
**FS32** (ticket class OPEN)
- L121 WQ, parked: `en beim Anwender; Folge [FS32, geparkt) → fuer Release `
**FS33** (ticket class OPEN)
- L121 WQ, none: `TASK/V45.md) (aus V41), [FS33 (aus `
**FS34** (ticket class OPEN)
- L121 WQ, none: ` HW08, zweiter Handle), [FS34 (aus `
**GH1** (ticket class CLOSED)
- L31 CS, none: `cs/HW-TEST-LOG.md' dazu: GH1`
- L164 ERLoQ, done-no-receipt: ` FS02, FS07, FS14, FS18, GH1,`
**H01** (ticket class PARKED)
- L160 P2, open: `  [H01 (wartet auf Hardware); `
**H04** (ticket class PARKED)
- L121 WQ, none: ` fuer 1.0 (SND01, SND02, H04). Daneben: die nur auf der Pi offenen Abnahmen (`
- L159 P2, open: `atei zitiert); geparkt: [H04,`
- L172 U10, none: `  SND02 (HDMI), H04 (Klingel, Tastenklick, 'Dosound'), alle `
**HW03** (ticket class CLOSED-OPENACC)
- L121 WQ, none: `16, A37, DESK34, DESK35, HW03, `
- L151 P1u, impl-open: `seitig), DESK34, DESK35, HW03 (drei Pi-Gegenproben),`
**HW04** (ticket class OPEN)
- L158 P2, open: `orrecherche, Ziel 1.2), [HW04`
**HW05** (ticket class PARTIAL)
- L121 WQ, none: `out der Karte; schliesst HW05, `
- L160 P2, done: ` (wartet auf Hardware); [HW05 (Befund erledigt, Abnahme offen: dieselbe 'QED.NAP'-Kopie mit Stoppuh`
**HW06** (ticket class CLOSED)
- L121 WQ, done: `r Karte; schliesst HW05, HW06 ein; Punkte 1+2 erledigt 2026-11-06, 'e421bd5d', 'measurements/measur`
**HW08** (ticket class CLOSED)
- L44 CS, none: `r 2239 am 2026-11-06 mit HW08 Fix, Developer, +22 Tests; davor 2217 am 2026-11-06 mit `
- L44 CS, none: `r 2200 am 2026-11-06 mit HW08 Punkte 1+2, Developer, +5 Tests; davor 2195 am 2026-11-06 mit `
- L121 WQ, none: `nel gegen 'e56021e7') → [HW08 (`
- L121 WQ, none: `3](../TASK/FS33.md) (aus HW08, zweiter Handle), `
- L121 WQ, none: `4](../TASK/FS34.md) (aus HW08, abgelehnter Datensektor), `
- L160 P2, done: `rher, ins Testszenario; [HW08 erledigt 2026-11-07); zurueckgestellt:`
**INPUT02** (ticket class OPEN)
- L121 WQ, none: `/67–69, FS12, FS15–FS17, INPUT02, `
- L177 P34, open: `IOCTL, A119–A125, RST02, INPUT02,`
**INPUT03** (ticket class PARKED)
- L104 WQ/EbGeparkt, parked: `  [INPUT03 (kein Programm braucht die Maus-Tastatur-Emulation),`
- L134 WQ, none: `04](../TASK/KEYS04.md), [INPUT03 (`
**INPUT04** (ticket class OPEN)
- L136 WQ/geparktMIDI, parked: `01](../TASK/UART01.md); [INPUT04 (`
**IO01** (ticket class IDEE)
- L136 WQ/geparktMIDI, parked: `  geparkt bis MIDI/IO01: `
- L180 P34, open: `  IO01.`
**KEYS01** (ticket class CLOSED)
- L233 PH, none: `tenwiederholung gepollt (KEYS01); 'etv_timer'-Aufruf und 'dosound' ohne Gegenstueck ('tos-api/src/bio`
**KEYS02** (ticket class CLOSED)
- L28 CS, none: `KEYS02), Tastenwiederholung, '.NTP'-Parameterdialog (`
- L237 PH, none: `ubstanziell fertig; dazu KEYS02 ('kbdvec'-Haken, Pi 2026-10-04), `
- L252 BLK, blocked: `- KEYS02 / D7-Luecke: IKBD-Vektoren ausser 'kbdvec' werden nicht gerufen (2026`
**KEYS03** (ticket class OPEN)
- L178 P34, open: `SB04.md), MEM08, MEM10, [KEYS03,`
**KEYS04** (ticket class PARKED)
- L103 WQ/EbGeparkt, parked: `  [KEYS04 (kein Kartenprogramm pollt Kbshift (nur apitest)),`
- L134 WQ, none: `(../TASK/A126.md) (P2), [KEYS04, `
**KEYS05** (ticket class OPEN)
- L121 WQ, none: `K/V48.md) (P4); aus V34 [KEYS05 (`
**LIBC-BUF** (ticket class CLOSED-OPENACC)
- L165 ERLoQ, done-no-receipt: `  LIBC-BUF, `
**LIBC-IOCTL** (ticket class OPEN)
- L177 P34, open: `erke), FS15, FS16, FS17, LIBC-IOCTL, `
**MEM01** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `  LIBC-BUF, MEM01, `
**MEM03** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `  LIBC-BUF, MEM01, MEM03, `
**MEM04** (ticket class CLOSED-OPENACC)
- L165 ERLoQ, done-no-receipt: ` LIBC-BUF, MEM01, MEM03, MEM04, `
**MEM05** (ticket class CLOSED-OPENACC)
- L167 ERLoQ, done-no-receipt: `, FS08 (je Teilschritt), MEM05 (A und B),`
**MEM06** (ticket class IDEE)
- L179 P34, open: `ckgestellt COOK01; Ideen MEM06,`
**MEM07** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `UF, MEM01, MEM03, MEM04, MEM07, `
**MEM08** (ticket class OPEN)
- L178 P34, open: `SB04](../TASK/USB04.md), MEM08, `
**MEM10** (ticket class OPEN)
- L178 P34, open: `./TASK/USB04.md), MEM08, MEM10, `
**MEM11** (ticket class PARKED)
- L44 CS, none: `+8 Tests; davor 1910 mit MEM11, +3 Tests; davor 1907 mit `
- L89 WQ/Geparkt, parked: `  [MEM11 (heute nicht erreichbar; ein Hosttest haelt die`
- L131 WQ/DevReihe, done: `  [MEM11-Hosttest (Speicheraufteilung < 2 GiB; erledigt 2026-10-31, Commit die`
- L154 P2, open: `- P2: [MEM11 (Anwenderadressen nur zufaellig < 2 GiB;`
**MEM12** (ticket class CLOSED)
- L141 P1, done: `- P1: [MEM12 — erledigt 2026-10-30 (Anwender: Pi-Lauf 2026-10-29, Kern '58e35ac8')`
**NET01** (ticket class OPEN)
- L158 P2, open: `  [NET01 (Vorrecherche, Ziel 1.2), `
**PITEST01** (ticket class CLOSED)
- L119 WQ, none: `  [PITEST01 (`
- L119 WQ, none: `st01-part3/RUNINFO.TXT', PITEST01 "Stand Teil 3"); Karte gefuellt und Pi-Laeufe A, B, `
- L119 WQ, none: ` Ticket); Kartenstand in PITEST01 "Stand nach den Pi-Laeufen".`
**PORT12** (ticket class PARTIAL)
- L70 OA, open: `cket der Warteschlange: [PORT12 (ADR-Entscheidung`
- L121 WQ, done: `rtet und geschlossen) → [PORT12 (XGDOS, neu 2026-11-06, Anwender: frueher einplanen; Stufe 1 erledigt`
**PROC01** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `01, MEM03, MEM04, MEM07, PROC01, `
**PROC07** (ticket class PARKED)
- L105 WQ/EbGeparkt, parked: `  [PROC07 (undokumentierte CP/M-Reliquie; gemlib sendet nur Opcodes != 0),`
- L109 WQ/EbGeparkt, parked: `abellenpflege, haengt an PROC07),`
- L132 WQ/DevReihe, none: `  [PROC07, `
**PROC08** (ticket class PARKED)
- L44 CS, none: `r 1974 am 2026-11-04 mit PROC08 Schirmbericht, +6 Tests; davor 1929 mit `
- L44 CS, none: `+6 Tests; davor 1929 mit PROC08 'crash-walk', +7 Tests; davor 1922 mit `
- L119 WQ, none: `01](../TASK/DOOM01.md), [PROC08 (neu, `
- L120 WQ, none: `  [PROC08 (`
- L121 WQ, none: `n Ragnar informieren). PROC08 wieder aufgetreten 2026-11-04 (Foto, 'measurements/measurement-2026`
- L139 WQ/Zurueckgest, parked: `26-11-02 (Anwender: erst PROC08); Diagnosestand 2026-10-31: Freeze nur mit dem Accessory X-Shot (wart`
**R02** (ticket class CLOSED-OPENACC)
- L121 WQ, none: `7, DESK34, DESK35, HW03, R02–`
- L152 P1u, impl-open: `  R02–`
- L168 ERLoQ, done-no-receipt: `  R02–`
- L238 PH, none: `~70 Opcodes); Pi-Abnahme R02–`
**R03** (ticket class CLOSED-OPENACC)
- L121 WQ, none (range): `7, DESK34, DESK35, HW03, R02–R05, `
- L152 P1u, impl-open (range): `  R02–R05, `
- L168 ERLoQ, done-no-receipt (range): `  R02–R05 ('R02-R05-STATUS-DONE.MD'). Liste per Skript aus der Statuszeile`
- L238 PH, none (range): `~70 Opcodes); Pi-Abnahme R02–R05 offen / 2026-09-21 / 'phases/PHASE-12-vdi.md' /`
**R04** (ticket class CLOSED-OPENACC)
- L121 WQ, none (range): `7, DESK34, DESK35, HW03, R02–R05, `
- L152 P1u, impl-open (range): `  R02–R05, `
- L168 ERLoQ, done-no-receipt (range): `  R02–R05 ('R02-R05-STATUS-DONE.MD'). Liste per Skript aus der Statuszeile`
- L238 PH, none (range): `~70 Opcodes); Pi-Abnahme R02–R05 offen / 2026-09-21 / 'phases/PHASE-12-vdi.md' /`
**R05** (ticket class CLOSED-OPENACC)
- L121 WQ, none: `ESK34, DESK35, HW03, R02–R05, `
- L152 P1u, impl-open: `  R02–R05, `
- L168 ERLoQ, done-no-receipt: `  R02–R05 ('R02-R05-STATUS-DONE.MD'). Liste per Skript aus der Statuszeile`
- L238 PH, none: `Opcodes); Pi-Abnahme R02–R05 offen / 2026-09-21 / 'phases/PHASE-12-vdi.md' /`
**REF03** (ticket class OPEN)
- L175 P34, open: `  RTC01, REF03, `
**REF04** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `3, MEM04, MEM07, PROC01, REF04, `
**REF05** (ticket class OPEN)
- L175 P34, open: `  RTC01, REF03, REF05 (GIC-Quellen), `
**REF06** (ticket class OPEN)
- L175 P34, open: `03, REF05 (GIC-Quellen), REF06 (fehlende Track-B-Quellen), `
**REL07** (ticket class CLOSED)
- L121 WQ, done: `  [REL07 Release 0.7 (veroeffentlicht 2026-11-04: Tag 'v0.7' auf 'd7711c`
- L121 WQ, done: `#3, #8), #9 geschlossen, REL07 erledigt; Paket mit 'tools/release/package-release.sh', 'measurements`
**REL071** (ticket class CLOSED)
- L69 OA, open: `  REL071 nur vom Anwender als frueher getestet gemeldet, kein Beleg im Repo.`
- L121 WQ, none: `geparkt) → fuer Release [REL071: `
**REL072** (ticket class CLOSED)
- L60 OA, done: `11-08 ('cb33f3fc'). (f) [REL072 0.7.2 veroeffentlicht 2026-11-08 (Tag 'v0.7.2');`
- L280 RD, done: `/ 2026-11-08 / [REL072: NewTOS 0.7.2 veroeffentlicht (Tag 'v0.7.2' auf 'd35670d6', GitHub-Pr`
**RST02** (ticket class OPEN)
- L121 WQ, none: `S12, FS15–FS17, INPUT02, RST02) → Ton fuer 1.0 (`
- L177 P34, open: `, LIBC-IOCTL, A119–A125, RST02, `
**RTC01** (ticket class OPEN)
- L118 WQ, parked: ` 'DEFAULT_DATETIME', bis RTC01; Commit dieser Zeile). Geparkt: `
- L175 P34, open: `  RTC01, `
**SDK01** (ticket class PARTIAL)
- L155 P2, done: `auf tosdump-Format v3), [SDK01 (A–L erledigt bzw. umgesetzt, G gestrichen;`
- L241 PH, none: `ypView; SDK fuer Fremde (SDK01) / 2026-10-22 / 'phases/PHASE-15-first-source-port.md', 'docs/sdk/' /`
**SDK02** (ticket class PARKED)
- L116 WQ/EbGeparkt, parked: `  [SDK02 (kein Nutzer der Konstanten),`
- L134 WQ, none: `/TASK/INPUT03.md) (P3); [SDK02, `
**SDK03** (ticket class CLOSED)
- L45 CS, none: `okie-host'; davor 28 mit SDK03, +1 'sysinfo-host'; davor 27 mit `
**SDK04** (ticket class CLOSED)
- L45 CS, none: `hard-host'; davor 33 mit SDK04, +3 'sysvars-layout', 'hypview-ref-host', 'zview-color-host'; davor 3`
- L45 CS, none: `olor-host'; davor 30 mit SDK04 `
**SDK05** (ticket class CLOSED)
- L45 CS, none: `okie-host'; davor 29 mit SDK05, +1 'cookie-host'; davor 28 mit `
**SDK06** (ticket class PARKED)
- L117 WQ/EbGeparkt, parked: `  [SDK06 (kein Programm liest einen Cookie mit gesetztem Bit 31).`
- L134 WQ, none: `K02](../TASK/SDK02.md), [SDK06`
**SDK07** (ticket class CLOSED)
- L45 CS, none: `ap-layout'; davor 34 mit SDK07, +1 'td-nohard-host'; davor 33 mit `
**SDK08** (ticket class CLOSED)
- L44 CS, none: `+4 Tests; davor 1895 mit SDK08, kein neuer Test; davor 1895 mit `
- L45 CS, none: `vor 35 am 2026-10-30 mit SDK08, kein neuer Test: Rebuild-Paritaet von 'rt-host'/'ldbl-host' blendet `
- L134 WQ, done: ` Commit der Quittung) → [SDK08 (erledigt 2026-10-30, Commit der Quittung) → `
- L135 WQ/DevReihe, none: `  (P3) bei Gelegenheit; [SDK08 (`
**SDL01** (ticket class OPEN)
- L174 P34, open: `runter: ARCH03 Stufe 2b, SDL01,`
**SER01** (ticket class CLOSED-OPENACC)
- L121 WQ, none: `, DESK35, HW03, R02–R05, SER01, `
- L152 P1u, impl-open: `  R02–R05, SER01, `
- L195 CS, none: `usgabe, seit 2026-10-02 (SER01): auf der Pi standardmaessig`
**SND01** (ticket class OPEN)
- L121 WQ, none: `, RST02) → Ton fuer 1.0 (SND01, `
- L157 P2, open: `  [SND01, `
- L171 U10, none: `; Ton gehoert dazu — SND01 (Falcon-DMA-Sound),`
**SND02** (ticket class OPEN)
- L121 WQ, none: `) → Ton fuer 1.0 (SND01, SND02, `
- L157 P2, open: `D01](../TASK/SND01.md), [SND02,`
- L172 U10, none: `  SND02 (HDMI), `
**SPEC01** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `4, MEM07, PROC01, REF04, SPEC01, `
**SPEC02** (ticket class PARKED)
- L107 WQ/EbGeparkt, parked: `  [SPEC02 (reine Nachpflege; offene Arbeit liest die Stellen nicht),`
- L134 WQ, none: `  [SPEC02, `
**SPEC03** (ticket class PARKED)
- L108 WQ/EbGeparkt, parked: `  [SPEC03 (reine Aufraeumarbeit; der Hook blockiert die Altlast nicht),`
**SPEC04** (ticket class CLOSED-OPENACC)
- L121 WQ, none: ` erledigt 2026-11-10) → [SPEC04 (`
**T13** (ticket class PARKED)
- L161 P2, open: `  [T13.`
**T15** (ticket class OPEN)
- L175 P34, open: `hlende Track-B-Quellen), T15,`
**T18** (ticket class OPEN)
- L179 P34, open: `  [T18, `
**TICK01** (ticket class PARKED)
- L118 WQ, none: `dieser Zeile). Geparkt: [TICK01 (bis Tester `
**TOOL01** (ticket class PARKED)
- L91 WQ/Geparkt, parked: `  [TOOL01 (blockiert keine Abnahme),`
- L159 P2, open: ` W01<../TASK/W01>, [TOOL01 ('cargo xtask doctor', Projekt-venv), `
**TOOL03** (ticket class PARKED)
- L86 WQ/Geparkt, parked: `  [TOOL03 (nur fuer DUMPGDOS/`
- L159 P2, open: `doctor', Projekt-venv), [TOOL03 (reproduzierbare Messabbilder und Dumper-Bauten, vor dem DUMPGDOS-Abb`
**TOOL04** (ticket class CLOSED)
- L159 P2, open: `PGDOS-Abbildvergleich), [TOOL04 (Testlaufzeit: schnelle und volle Stufe), `
**TOOL05** (ticket class OPEN)
- L159 P2, parked: `nelle und volle Stufe), [TOOL05 (doc-check auch fuer Doku, die eine geaenderte Datei zitiert); gepark`
**TOOL07** (ticket class CLOSED)
- L44 CS, none: `+9 Tests; davor 1751 mit TOOL07 Rest, +3 xtask-Tests; davor 1748 mit `
**TOOL09** (ticket class CLOSED)
- L44 CS, none: `; davor 1745 mit TOOL10; TOOL09 +16, `
**TOOL10** (ticket class CLOSED)
- L44 CS, none: `ps-Tests; davor 1745 mit TOOL10; `
**TOOL12** (ticket class CLOSED)
- L44 CS, none: `sk-Tests; davor 1748 mit TOOL12, +3 deps-Tests; davor 1745 mit `
**TOOL13** (ticket class PARKED)
- L82 WQ/Geparkt, parked: `  [TOOL13 (kein anstehender Fix bringt ein Debug-Feature;`
**TOOL14** (ticket class PARKED)
- L83 WQ/Geparkt, parked: `isher von Hand belegt), [TOOL14 (Sonden laufen`
**TOOL15** (ticket class PARKED)
- L84 WQ/Geparkt, parked: `hne Trace-Betriebsart), [TOOL15 (Echtzeit-Sonden`
**TOOL17** (ticket class CLOSED)
- L44 CS, none: `+2 Tests; davor 1821 mit TOOL17, +5 Tests; davor 1816 mit `
**TOOL19** (ticket class CLOSED)
- L44 CS, none: `+7 Tests; davor 1833 mit TOOL19, +5 Tests; davor 1828 mit `
**TOOL21** (ticket class CLOSED)
- L44 CS, none: ` +1 Test; davor 1823 mit TOOL21, +2 Tests; davor 1821 mit `
**TOOL22** (ticket class PARKED)
- L109 WQ/EbGeparkt, parked: `  [TOOL22 (Tabellenpflege, haengt an `
- L133 WQ/DevReihe, none: `  [TOOL22, `
**TOOL23** (ticket class PARKED)
- L110 WQ/EbGeparkt, parked: `  [TOOL23 (Debug-Feature; wieder aufnehmen, wenn ein AES-Fix den Mitschnitt bra`
- L133 WQ/DevReihe, none: `9](../TASK/XBIOS19.md), [TOOL23,`
**TOOL24** (ticket class PARKED)
- L111 WQ/EbGeparkt, parked: `  [TOOL24 (Debug-Feature, blockiert nichts),`
- L134 WQ, none: `02](../TASK/SPEC02.md), [TOOL24, `
**TOOL25** (ticket class PARKED)
- L112 WQ/EbGeparkt, parked: `  [TOOL25 (zugehoeriger Fix (`
- L134 WQ, none: `FS27](../TASK/FS27.md), [TOOL25 (`
**TOOL26** (ticket class PARKED)
- L113 WQ/EbGeparkt, parked: `  [TOOL26 (zugehoeriger Fix (`
- L134 WQ, none: `A126](../TASK/A126.md); [TOOL26 (`
**TOOL27** (ticket class PARKED)
- L137 WQ/geparktMIDI, parked: `edem IKBD-Interpreter); [TOOL27 (`
**TOOL28** (ticket class PARKED)
- L114 WQ/EbGeparkt, parked: `  [TOOL28 (Handregel in CLAUDE.md deckt es ab),`
- L138 WQ/geparktMIDI, parked: `nach tosdiff, geparkt); [TOOL28 (`
**TOOL29** (ticket class CLOSED)
- L44 CS, none: `+2 Tests; davor 1905 mit TOOL29, +6 xtask-Tests; davor 1899 mit `
- L134 WQ, done: ` Commit dieser Zeile) → [TOOL29 (umgesetzt 2026-10-30, Commit dieser Zeile; Hook-Installation durch d`
**TOOL30** (ticket class PARKED)
- L115 WQ/EbGeparkt, parked: `  [TOOL30 (zugehoeriger Fix (`
- L138 WQ/geparktMIDI, parked: `ost_clang ins RUNINFO); [TOOL30 (`
**TOOL31** (ticket class PARKED)
- L121 WQ, none: `stszenario; Folgeticket [TOOL31, `
**TOOL32** (ticket class CLOSED)
- L121 WQ, none: `ie 26 VDI-Kandidaten aus TOOL32 am Orakel messen, drei Teile, dann Fix-Tickets je Befund; Teil 1 geme`
- L121 WQ, done: `on' ins Testszenario) → [TOOL32 (VDI-Zuordnung in 'mapping.tsv', erledigt 2026-11-05; 26 Abweichungs-`
**TOOL33** (ticket class CLOSED)
- L63 OA, open: `  47 AES-Kandidaten aus [TOOL33 (Messung am Orakel wie`
- L121 WQ, done: `m Ticket, ungemessen) → [TOOL33 (AES-Zuordnung, erledigt 2026-11-05; 47 Kandidaten im Ticket, ungemes`
**TOOL34** (ticket class PARKED)
- L121 WQ, parked: `NINFO.TXT'; Folgeticket [TOOL34 geparkt) → `
**TOOL35** (ticket class PARKED)
- L121 WQ, parked: `est/RUNINFO.TXT'; Folge [TOOL35, geparkt) → `
**TOOL36** (ticket class PARKED)
- L121 WQ, parked: `1.md) (IDEE, nach 1.0), [TOOL36 (geparkt); bekannte Abweichungen nach Ausloeser: 'docs/DEVIATIONS.md'`
**TOOL37** (ticket class PARKED)
- L121 WQ, parked: `1-08, 'e197b9cd'; Folge [TOOL37 Graph, geparkt, `
**TOOL38** (ticket class PARKED)
- L121 WQ, none: `A147](../TASK/A147.md), [TOOL38) → `
- L121 WQ, parked: `elesen, zuerst messen), [TOOL38 (geparkt: wkprobe-Wartegrenze; Laeufe durch Wiederholen abnehmbar). A`
- L282 RD, parked: `rben/Hotspot, gelesen), [TOOL38 (wkprobe-Wartegrenze, geparkt). /`
**UART01** (ticket class PARKED)
- L136 WQ/geparktMIDI, parked: ` geparkt bis MIDI/IO01: [UART01; `
**USB01** (ticket class CLOSED)
- L32 CS, none: `er Release-Kernel 0.6.1, USB01 `
- L142 P1, done: `  [USB01 — erledigt 2026-11-04 (Quittung; `
- L242 PH, none: ` laeuft auf echter Pi 4; USB01 erledigt 2026-11-04; offen `
**USB02** (ticket class OPEN)
- L178 P34, open: `  CON03, ARCH04, USB02, `
- L242 PH, none: `11-04; offen USB03 (P1), USB02 (`
**USB03** (ticket class OPEN)
- L121 WQ, parked: `eig der Dateiauswahl) → [USB03 (geparkt 2026-11-05 bis Ragnars Rueckmeldung zu #19 und zum USB-Stand`
- L143 P1, open: `ware bestaetigt). Rest: [USB03`
- L242 PH, none: `ledigt 2026-11-04; offen USB03 (`
- L242 PH, none: `E-16-usb-hid-input.md', [USB03 /`
**USB04** (ticket class OPEN)
- L147 P1, open: `VL805 ins Testszenario, [USB04 (`
- L178 P34, open: `  CON03, ARCH04, USB02, [USB04, `
- L242 PH, none: ` USB03 (P1), USB02 (P3), USB04 (`
**V01** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `, PROC01, REF04, SPEC01, V01, `
**V02** (ticket class CLOSED)
- L165 ERLoQ, done-no-receipt: `C01, REF04, SPEC01, V01, V02,`
**V04** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt: `  V04–`
**V05** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt (range): `  V04–V10, `
**V06** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt (range): `  V04–V10, `
**V07** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt (range): `  V04–V10, `
**V08** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt (range): `  V04–V10, `
**V09** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt (range): `  V04–V10, `
**V10** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt: `  V04–V10, `
**V11** (ticket class CLOSED-OPENACC)
- L121 WQ, none: `5, HW03, R02–R05, SER01, V11, `
- L152 P1u, impl-open: `  R02–R05, SER01, V11, `
**V12** (ticket class CLOSED-OPENACC)
- L121 WQ, none: `03, R02–R05, SER01, V11, V12, `
- L152 P1u, impl-open: `  R02–R05, SER01, V11, V12, `
**V13** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt: `  V04–V10, V13–`
**V14** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt (range): `  V04–V10, V13–V15 (36); dazu sieben mit nur Teil- oder Sammelquittung:`
**V15** (ticket class CLOSED)
- L166 ERLoQ, done-no-receipt: `  V04–V10, V13–V15 (36); dazu sieben mit nur Teil- oder Sammelquittung:`
**V18** (ticket class PARTIAL)
- L64 OA, open: `  [V18), ans Ende der Warteschlange: ja oder nein. (2) Namen der`
- L121 WQ, none: `Anwender 2026-11-05): [V18 (die 26 VDI-Kandidaten aus `
- L121 WQ, none: `eidung C orakelgleich) → V18 Teil 2 gemessen 2026-11-06 'eba4122a': `
- L121 WQ, none: `7-v35-test/RUNINFO.TXT'; V18 Teil 2 damit orakelgleich bis auf `
- L121 WQ, none: `Orakel, NewTOS bremst) → V18 Teil 3 gemessen 2026-11-07 ('020bfd16', 'docs/audits/2026-11-07-v18-t`
- L121 WQ, none: `rentscheidungen A/B/C in V18; Teil 2/3 danach). Reihenfolge nach der Prioritaetenrunde (Anwender`
- L282 RD, none: `/RUNINFO.TXT'). Warum: V18 Teil 3, VDI-WK-003 am Orakel gemessen (wkprobe `
- L287 RD, none: `/RUNINFO.TXT'). Warum: V18 Teil 3 (VDI-WK-001/002) am Orakel gemessen. EmuTOS 'timer_init' + 'vd`
**V19** (ticket class CLOSED)
- L44 CS, none: `r 2004 am 2026-11-05 mit V19, Tester; davor 1974 am 2026-11-04 mit `
- L121 WQ, done: ` 9 Befunde, Fix-Tickets [V19 (erledigt 2026-11-05, '506643fb', Tester PASS 'measurements/measureme`
- L121 WQ, none: `ASK/V26.md) (Nebenbefund V19, Speicherweg griff in ungepruefte Zeile, `
**V20** (ticket class CLOSED)
- L44 CS, none: `r 2033 am 2026-11-05 mit V20, Developer, +20 Tests; davor 2013 am 2026-11-05 mit `
- L121 WQ, done: `26-test/RUNINFO.TXT') → [V20 ('abline', erledigt 2026-11-05, 'bec04bab'; Anwenderentscheidung A: U`
**V21** (ticket class CLOSED)
- L121 WQ, done: `and ins Testszenario) → [V21 (erledigt 2026-11-06, '497773f9', Saat-Warteschlange wie EmuTOS; Test`
- L288 RD, none: `ange (EmuTOS-gleich seit V21) kann bei echten Eingaben endlos kreisen; kleinster Fall 5x3-Clip mit`
**V22** (ticket class CLOSED)
- L121 WQ, done: ` offen beim Anwender) → [V22 (erledigt 2026-11-06, '7108bdff', Tester PASS 'measurements/measureme`
**V23** (ticket class CLOSED)
- L121 WQ, done: `22-test/RUNINFO.TXT') → [V23 (erledigt 2026-11-06, 'ee67903f', Kappung zentral in 'dispatch_in'; T`
**V24** (ticket class CLOSED)
- L121 WQ, done: `K/TOOL34.md) geparkt) → [V24 (erledigt 2026-11-06, 'caf77fab', Anwenderentscheidung B; Tester PASS`
**V25** (ticket class CLOSED)
- L44 CS, none: `r 2066 am 2026-11-06 mit V25 laut Recent Decisions; davor 2033 am 2026-11-05 mit `
- L121 WQ, done: `24-test/RUNINFO.TXT') → [V25 (erledigt 2026-11-06, '294272e2', frueher Ausstieg von 'curve'; Teste`
**V26** (ticket class CLOSED)
- L44 CS, none: `r 2013 am 2026-11-05 mit V26, Developer, +9 Tests; davor 2004 am 2026-11-05 mit `
- L121 WQ, none: `19-test/RUNINFO.TXT') → [V26 (Nebenbefund `
**V27** (ticket class OPEN)
- L121 WQ, none: `/V57.md) P4, geparkt) → [V27 (`
**V28** (ticket class CLOSED)
- L44 CS, none: `r 2096 am 2026-11-06 mit V28, Developer, +30 Tests; davor 2066 am 2026-11-06 mit `
- L121 WQ, done: ` 2026-11-06 'eba4122a': [V28 (erledigt 2026-11-06, 'f1006588', Tester PASS 'measurements/measureme`
**V29** (ticket class CLOSED)
- L44 CS, none: `r 2116 am 2026-11-06 mit V29, Developer, +20 Tests; davor 2096 am 2026-11-06 mit `
- L121 WQ, done: `V36](../TASK/V36.md)) → [V29 (erledigt 2026-11-06, 'b4e419e5', Tester PASS 'measurements/measureme`
**V30** (ticket class CLOSED)
- L44 CS, none: `r 2160 am 2026-11-06 mit V30, Developer, +40 Tests; davor 2120 am 2026-11-06 mit `
- L121 WQ, done: `29-test/RUNINFO.TXT') → [V30 (erledigt 2026-11-06, '858e5f68', Tester PASS 'measurements/measureme`
**V31** (ticket class CLOSED)
- L44 CS, none: `r 2258 am 2026-11-07 mit V31, Developer und Tester, +17 Tests; davor 2241 am 2026-11-07 mit `
- L121 WQ, done: `ein Kernelcode davor) → [V31 (erledigt 2026-11-07, '17bcaf56', Tester PASS 'measurements/measureme`
- L280 RD, none: ` und den VDI-Treue-Fixes V31–`
**V32** (ticket class CLOSED)
- L44 CS, none: `r 2120 am 2026-11-06 mit V32, Developer, +4 Tests; davor 2116 am 2026-11-06 mit `
- L121 WQ, done: `5 dabei orakelgleich) → [V32 (erledigt 2026-11-06, '937d6361', Tester PASS 'measurements/measureme`
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
**V33** (ticket class CLOSED)
- L44 CS, none: `r 2190 am 2026-11-06 mit V33, Developer, +30 Tests; davor 2160 am 2026-11-06 mit `
- L121 WQ, done: `fad ins Testszenario) → [V33 (erledigt 2026-11-06, '877a14b9', Tester PASS 'measurements/measureme`
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
**V34** (ticket class CLOSED)
- L44 CS, none: `r 2261 am 2026-11-07 mit V34, Developer und Tester, +3 Tests; davor 2258 am 2026-11-07 mit `
- L121 WQ, done: `](../TASK/DESK71.md)) → [V34 (erledigt 2026-11-07, '5f299bf8', Tester PASS 'measurements/measureme`
- L121 WQ, none: `./TASK/V48.md) (P4); aus V34 `
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
**V35** (ticket class CLOSED)
- L44 CS, none: `r 2273 am 2026-11-07 mit V35, Developer und Tester, +12 Tests, 1 ignoriert mit Grund (TPH, `
- L121 WQ, done: `/TOOL35.md), geparkt) → [V35 (erledigt 2026-11-07, '788a8aa9', Tester PASS 'measurements/measureme`
- L121 WQ, parked: `Alt+Help 'dumpflg'); aus V35 geparkt `
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
**V36** (ticket class CLOSED)
- L121 WQ, none: `T'; erledigt damit auch [V36) → `
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
**V37** (ticket class CLOSED)
- L44 CS, none: `r 2291 am 2026-11-07 mit V37, Developer und Tester, +11, 4 ignoriert mit Grund `
- L121 WQ, done: `8-v59-pi/RUNINFO.TXT' → [V37 (erledigt 2026-11-07, '2bf28d71', Tester PASS 'measurements/measureme`
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
- L289 RD, none: `/ 2026-11-07 / [V37: Waagrechtes 'v_justified' richtet wie EmuTOS gegen 'max_x' aus, mit `
**V38** (ticket class CLOSED)
- L44 CS, none: `f 'c013f4b2' mit V53 und V38, Lead, +6 `
- L58 OA, open: `  (c) [V38 Anwender 2026-11-08: Grenze hoeher, Wert erst nach Pi-Messung des 5x3`
- L121 WQ, done: `37-test/RUNINFO.TXT') → [V38 (erledigt 2026-11-08, 'e6ffdedc', Tester PASS; EmuTOS haengt im 5x3-F`
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
- L288 RD, done: `/ 2026-11-08 / [V38: 'v_contourfill' hat eine Notbremse bei 16 Spannen je Schirmpixel ('e`
**V39** (ticket class OPEN)
- L60 OA, done: `lippen, D3 Toleranz, D4 [V39 mit aufnehmen). (e) Push erledigt 2026-11-08 ('cb33f3fc'). (f) `
- L121 WQ, none: ` (2026-11-06): gemessen [V39 ('F_LIGHT'), `
- L280 RD, none (range): ` und den VDI-Treue-Fixes V31–V40, `
**V40** (ticket class CLOSED)
- L44 CS, none: `f '5be660c2' mit V56 und V40, Tester, +2 `
- L121 WQ, done: `SK/V39.md) ('F_LIGHT'), [V40 ('LSTLIN'; erledigt 2026-11-08, '5be660c2', Tester PASS), `
- L280 RD, none: ` den VDI-Treue-Fixes V31–V40, `
- L286 RD, done: `/ 2026-11-08 / [V40: 'LSTLIN' wie EmuTOS gefuehrt, die XOR-Unterstreichung liest es ('5be`
**V41** (ticket class OPEN)
- L44 CS, none: `1, 4 ignoriert mit Grund V41; davor 2280 am 2026-11-07 auf dem Merge von `
- L59 OA, open: ` (Tester-Szenario). (d) [V41 `
- L121 WQ, none: `be660c2', Tester PASS), [V41 (Ursache belegt 2026-11-06, 'de1d12dd', neun Stellen; keine App zeich`
- L121 WQ, none: `45](../TASK/V45.md) (aus V41), `
- L289 RD, none: `G002/003/008/009 Rest = [V41. Hosttests 2280 -> 2291/0/6; Warnungen unveraendert. /`
**V42** (ticket class OPEN)
- L121 WQ, none: `gelehnter Datensektor), [V42 (`
**V43** (ticket class OPEN)
- L121 WQ, none: `026-11-07); nur gelesen [V43 (`
**V44** (ticket class CLOSED)
- L44 CS, none: `r 2195 am 2026-11-06 mit V44, Developer, +5 Tests; davor 2190 am 2026-11-06 mit `
- L121 WQ, none: `est/RUNINFO.TXT'; Folge [V44; vor der naechsten Karte QED, TeraDesk, HypView, zView neu linken; Pi`
- L121 WQ, done: `ext ins Testszenario) → [V44 (erledigt 2026-11-06, '6e96e91c', Tester PASS 'measurements/measureme`
**V45** (ticket class OPEN)
- L121 WQ, none: `/RUNINFO.TXT'; ZG012 aus V45 dabei orakelgleich) → `
- L121 WQ, none: `ix ans Ende, Anwender), [V45 (aus `
**V46** (ticket class OPEN)
- L121 WQ, none: `unctions-2026-11-07.md' [V46, `
**V47** (ticket class OPEN)
- L121 WQ, none: ` V46<../TASK/V46>, [V47, `
**V48** (ticket class OPEN)
- L121 WQ, none: `A145](../TASK/A145.md), [V48 (`
**V49** (ticket class CLOSED)
- L44 CS, none: `-11-07 auf dem Merge von V49 '2795e432', Lead, +7 'raster_kernel_fill'; davor 2273 am 2026-11-07 m`
- L121 WQ, done: `Anwender 2026-11-07): [V49 (erledigt 2026-11-07, Merge '2795e432', Tester PASS, Pi Fbench 'measu`
- L280 RD, none: `ase nach der NEON-Reihe (V49–`
- L283 RD, none: `ion. Gegen den Stand vor V49 (Fbench-Vergleich): VDI 234 %, AES 239 %, Vollbildfenster 442 %. Host`
- L284 RD, none: `036, gleicher Befund wie V49 (pixelweise 'combine'/'read_volatile'). Pixel unveraendert: alle Tabe`
- L284 RD, none: `. Pi des Anwenders gegen V49: Mono blit 13 460 -> 45 721/s, Text x1,8-2,6, Vollbild-Scrollen 81,6 `
**V50** (ticket class CLOSED)
- L44 CS, none: `-11-08 auf dem Merge von V50/`
- L44 CS, none: `51 '20c23349', Lead, +11 V50, +10 `
- L121 WQ, none: `-v49-pi/RUNINFO.TXT') → [V50 → `
- L280 RD, none (range): `ase nach der NEON-Reihe (V49–V51, `
- L283 RD, none: `in Fbench-Lauf gegen den V50/V51-Lauf: Hide/show mouse 20 377 -> 111 346/s (x5,46), graf_mouse off`
- L284 RD, none: `/ 2026-11-08 / [V50 und `
**V51** (ticket class CLOSED)
- L44 CS, none: `08 auf dem Merge von V50/V51 '20c23349', Lead, +11 `
- L44 CS, none: `349', Lead, +11 V50, +10 V51; davor 2318 am 2026-11-08 auf '5be660c2' mit `
- L121 WQ, done: `V50<../TASK/V50> → [V51 (beide erledigt 2026-11-08, Merge '20c23349', Tester PASS, Pi 'measur`
- L280 RD, none: `nach der NEON-Reihe (V49–V51, `
- L284 RD, none: `50](../TASK/V50.md) und [V51: NEON-Kerne fuer Expansion (Text, 'vrt_cpyfm') und Rechteck-Kopie (Sc`
**V52** (ticket class PARKED)
- L44 CS, none: `gnoriert mit Grund (TPH, V52); davor 2261 am 2026-11-07 mit `
- L121 WQ, none: `mit orakelgleich bis auf V52) → eingeschoben (Anwender 2026-11-07): `
- L121 WQ, none: `pflg'); aus V35 geparkt [V52 (`
**V53** (ticket class CLOSED)
- L44 CS, none: `11-08 auf 'c013f4b2' mit V53 und `
- L121 WQ, done: `3-vdi-workstation.md'): [V53 ('v_opnwk'/'v_clswk' Maus; erledigt 2026-11-08, '4cc7005c', Tester PA`
- L280 RD, none: `VDI-Treue-Fixes V31–V40, V53–`
- L287 RD, none: `/ 2026-11-08 / [V53: 'v_opnwk' setzt die Maus jedes Mal zurueck, 'v_clswk' schaltet sie a`
**V54** (ticket class CLOSED)
- L44 CS, none: `r 2362 am 2026-11-08 mit V54 '2869dcfc', Tester, +4 `
- L44 CS, none: `4 '2869dcfc', Tester, +4 V54; davor 2358 am 2026-11-08 auf dem Merge von `
- L121 WQ, done: `9614ed', Tester PASS) → [V54 ('vsc_form'; erledigt 2026-11-08, '721e0441', Tester PASS '2869dcfc';`
- L121 WQ, none: `ahmen und Schatten). Aus V54 (2026-11-08): `
- L121 WQ, none: `ehmbar). Aus dem Pi-Test V54/`
- L280 RD, none (range): `VDI-Treue-Fixes V31–V40, V53–V56. Nur der Kern geaendert, Programme wie 0.7.1 (ihre 'CHANGES.TXT' entf`
- L282 RD, none: `/ 2026-11-08 / [V54: 'vsc_form' speichert die Zeigerform nur und zeichnet nicht neu, wie `
**V55** (ticket class CLOSED)
- L44 CS, none: `(gemessen 2026-11-08 mit V55 'b744febc', Tester, +6 `
- L44 CS, none: `5 'b744febc', Tester, +6 V55, +1 Nachtrag; davor 2362 am 2026-11-08 mit `
- L56 OA, done: `9-pi/RUNINFO.TXT'). (b) [V55 entschieden 2026-11-08: (a), umgesetzt, Tester PASS; Pi-Test mit Rele`
- L121 WQ, none: `](../TASK/TOOL38.md)) → [V55 (`
- L121 WQ, none: `ar). Aus dem Pi-Test V54/V55 (2026-11-08): `
- L280 RD, none (range): `VDI-Treue-Fixes V31–V40, V53–V56. Nur der Kern geaendert, Programme wie 0.7.1 (ihre 'CHANGES.TXT' entf`
- L281 RD, none: `/ 2026-11-08 / [V55: Der Mauszeiger wird im naechsten Bildtakt gezeichnet, wie EmuTOS 'vb`
- L282 RD, none: `nur noch P0 und M1a/M2a (V55) verschieden; vdi-all, vdprobe, AES r2/r3/r4, Desktop- und Bootbilder`
**V56** (ticket class CLOSED)
- L44 CS, none: `11-08 auf '5be660c2' mit V56 und `
- L121 WQ, none: `5c', Tester PASS; Folge [V56 `
- L280 RD, none: `Treue-Fixes V31–V40, V53–V56. Nur der Kern geaendert, Programme wie 0.7.1 (ihre 'CHANGES.TXT' entf`
- L285 RD, none: `/ 2026-11-08 / [V56: Die 'vex_*'-Vorgabemarken sind getrennte 'static AtomicU8' statt lee`
- L287 RD, none: `en Radvektors tot sein: [V56 (`
**V57** (ticket class PARKED)
- L121 WQ, none: `L37.md) Graph, geparkt, [V57 `
**V58** (ticket class CLOSED)
- L44 CS, none: `-11-08 auf dem Merge von V58/`
- L44 CS, none: `59 '64bf181e', Lead, +11 V58, +8 `
- L55 OA, open: `digt 2026-11-08: Pi-Lauf V58 + `
- L121 WQ, none: `ee neben der V-Reihe) → [V58 (Linien) und `
- L280 RD, none: `der NEON-Reihe (V49–V51, V58, `
- L283 RD, none: `/ 2026-11-08 / [V58 und `
- L283 RD, none: ` 'raster-prof'-Kerne von V58, EL0-Test mit lebenden V-Registern (`
**V59** (ticket class CLOSED)
- L44 CS, none: `08 auf dem Merge von V58/V59 '64bf181e', Lead, +11 `
- L44 CS, none: `181e', Lead, +11 V58, +8 V59; davor 2339 am 2026-11-08 auf dem Merge von `
- L55 OA, done: `026-11-08: Pi-Lauf V58 + V59 bestanden, beide in 'main'`
- L121 WQ, done: `SK/V58.md) (Linien) und [V59 (Mauszeiger), Anwender 2026-11-08: NEON-Reihe fertig machen; beide er`
- L280 RD, none: `EON-Reihe (V49–V51, V58, V59) und den VDI-Treue-Fixes `
- L283 RD, none: `58](../TASK/V58.md) und [V59: Linien als Spanne, Spalte oder Schrittpixel, Mauszeiger je Zelle ges`
- L283 RD, none: `it lebenden V-Registern (V59). /`
**W01** (ticket class OPEN)
- L159 P2, open: `  (teilweise), [W01, `
**X01** (ticket class CLOSED)
- L231 PH, none: `lossen; D7 'Supexec' mit X01 entschieden (Super/Supexec als SDK-Overlay) / 2026-10-17 / 'phases/PH`
- L231 PH, none: `igh-half-migration.md', [X01 /`
- L237 PH, none: `'-Haken, Pi 2026-10-04), X01, `
**X04** (ticket class CLOSED-OPENACC)
- L237 PH, none: `en, Pi 2026-10-04), X01, X04; Rest-Vektoren unter *Blocked* / 2026-10-17 / 'phases/PHASE-11-xbios.`
**XBIOS01** (ticket class CLOSED)
- L44 CS, none: `+7 Tests; davor 1783 mit XBIOS01, +16 Tests; davor 1767 mit `
- L45 CS, none: `info-host'; davor 27 mit XBIOS01, +1 'iorec-layout') /`
**XBIOS02** (ticket class CLOSED)
- L44 CS, none: `+4 Tests; davor 1843 mit XBIOS02, +1 Test; davor 1842 mit `
- L123 WQ/DevReihe, done: `  die S2-Audit-Tickets [XBIOS02 (erledigt 2026-10-29, '7bc047a6'),`
**XBIOS03** (ticket class CLOSED-OPENACC)
- L127 WQ/DevReihe, none: `der Quittung; BIOS05 und XBIOS03`
**XBIOS04** (ticket class CLOSED)
- L44 CS, none: `uer Test; davor 1895 mit XBIOS04, +1 Test; davor 1894 mit `
- L127 WQ/DevReihe, none: `0, Commit der Quittung), XBIOS04–`
- L134 WQ, done: `Prueffolge, Punkt 10) → [XBIOS04 (umgesetzt 2026-10-30, Commit der Quittung) → `
- L252 BLK, blocked: ` IKBD-Interpreter; nach [XBIOS04 Option 1 gibt es keinen, 'Ikbdws'/'Bconout(4)' warnen nur), MIDI (kei`
**XBIOS05** (ticket class CLOSED)
- L44 CS, none: `+7 Tests; davor 1881 mit XBIOS05, +9 Tests, Basis laut Lead 1872; davor 1859 mit `
- L45 CS, none: `ang-Update; davor 35 mit XBIOS05, +1 'bconmap-layout'; davor 34 mit `
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
- L127 WQ/DevReihe, done: `tung), XBIOS04–XBIOS14 ([XBIOS05 erledigt 2026-10-29, Commit der Quittung; `
**XBIOS06** (ticket class PARKED)
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
- L134 WQ, none: `er Karte betroffen ist: [XBIOS06-Fix, `
**XBIOS07** (ticket class PARKED)
- L96 WQ/EbGeparkt, parked: `  [XBIOS07 (Doku-Korrektur ohne Nutzer),`
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
**XBIOS08** (ticket class PARKED)
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
- L134 WQ, none: `./TASK/XBIOS06.md)-Fix, [XBIOS08, `
**XBIOS09** (ticket class PARKED)
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
- L134 WQ, none: `8](../TASK/XBIOS08.md), [XBIOS09, `
**XBIOS10** (ticket class PARKED)
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
- L128 WQ/DevReihe, none: `d ACCEPTED nach ADR-031, XBIOS10 nach `
- L134 WQ, none: `9](../TASK/XBIOS09.md), [XBIOS10) → `
**XBIOS11** (ticket class PARKED)
- L97 WQ/EbGeparkt, parked: `  [XBIOS11 (kein Kartenprogramm haengt am Seed-Ursprung (einziger Nutzer TeraDes`
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
**XBIOS12** (ticket class CLOSED)
- L44 CS, none: `+7 Tests; davor 1922 mit XBIOS12, +4 Tests; davor 1918 mit `
- L118 WQ, done: ` 2026-10-31 (Quittung). [XBIOS12 umgesetzt 2026-10-31 (ohne RTC startet die Uhr beim Baudatum des Kern`
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
**XBIOS13** (ticket class PARKED)
- L99 WQ/EbGeparkt, parked: `  [XBIOS13 (reine Spec-Pflege),`
- L127 WQ/DevReihe, none (range): `0, Commit der Quittung), XBIOS04–XBIOS14 (`
**XBIOS14** (ticket class PARKED)
- L100 WQ/EbGeparkt, parked: `  [XBIOS14 (Orakelgrenze; blockiert keinen Fix),`
- L127 WQ/DevReihe, none: `t der Quittung), XBIOS04–XBIOS14 (`
**XBIOS15** (ticket class CLOSED)
- L44 CS, none: `+5 Tests; davor 1816 mit XBIOS15, +8 Tests; davor 1808 mit `
**XBIOS16** (ticket class PARKED)
- L101 WQ/EbGeparkt, parked: `  [XBIOS16 (kein Programm ruft Jdisint/Jenabint/Mfpint),`
**XBIOS18** (ticket class CLOSED)
- L44 CS, none: `+4 Tests; davor 1824 mit XBIOS18, +1 Test; davor 1823 mit `
**XBIOS19** (ticket class PARKED)
- L102 WQ/EbGeparkt, parked: `  [XBIOS19 (kein Programm ruft Bioskeys),`
- L133 WQ/DevReihe, none: `22](../TASK/TOOL22.md), [XBIOS19, `
