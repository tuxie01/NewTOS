# REF04 — Die woertliche Tabelle 4-42 in `emmc-sd.md` gegen die Lizenzbedingungen

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; no receipt | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 17, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 41c9bc1c9 (docs(hw): pin the SD specifications, and HW03's ci), named by body

Line 3 before the migration:
Status: **ERLEDIGT** (2026-09-29) · Prioritaet: **P2** · Typ: Lizenz / Veroeffentlichung — aus REF02

Abhaengigkeiten: [REF02](REF02.md) (dort im Wortlaut gelesen)

## Der Sachverhalt

`docs/hardware/emmc-sd.md` §5 (aus [HW03](HW03.md)) gibt die **komplette
Description-Spalte der Tabelle 4-42** der SD Physical Layer Simplified
Specification wieder — **15 Zeilen Originaltext**.

Die Lizenzbedingungen der SD Association erlauben Nicht-Mitgliedern
(§2) Herunterladen, Ansehen und Drucken fuer den internen Gebrauch, verbieten
aber ausdruecklich:

> *„(a) copy portions of the Simplified Specs (whether by screen scraping,
> screen shots, electronically or otherwise) … or (c) distribute the Simplified
> Specs or any portion thereof."*

§3 — Auszuege in eigene Dokumentation zu uebernehmen — gilt **nur
SDA-Mitgliedern**. Im PDF steht zusaetzlich: *„Distribution of the Simplified
Specification, other than through the Site, is a violation of the Terms."*

**Das Repo ist oeffentlich.**

## Warum es hier steht statt behoben zu sein

Der Inhalt ist **fachlich richtig** und war HW03s Arbeit — es geht nicht um
einen Fehler, sondern um die Form. Und es ist eine Entscheidung ueber die
Veroeffentlichung des eigenen Projekts, also eine des Anwenders, nicht eine,
die ein Agent nebenbei trifft.

Die Lesart stammt aus REF02 und ist **keine Rechtsberatung**.

## Die drei Wege

1. **In eigene Worte fassen.** Jedes Bit behaelt Nummer, Name und unsere
   Beschreibung dessen, was es fuer den Treiber bedeutet; der Originalwortlaut
   entfaellt, die Abschnittsangabe bleibt als Verweis. Sachlich verlustfrei,
   weil unsere Leser den Treiber verstehen muessen, nicht die Spec
   nachschlagen. **Empfehlung.**
2. **Kuerzen** auf Bitnummer + Name + Typspalte (die blosse Zuordnung, die man
   ohnehin aus dem Code liest) und fuer den Rest auf §4.10.1 verweisen.
3. **So lassen** — dann als bewusste Entscheidung mit Begruendung in der Datei
   vermerken, nicht stillschweigend.

## Abnahme

`emmc-sd.md` §5 traegt die gewaehlte Form, und `references/README.md` haelt
fest, was fuer kuenftige Uebernahmen aus dieser Quelle gilt — damit die naechste
Einheit nicht wieder hineinlaeuft.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

---

## Entscheidung 2026-09-29 — Weg 1

Der Anwender hat **Weg 1** gewaehlt: den Originalwortlaut in eigene Worte
fassen. Jedes Bit behaelt **Nummer und Namen** — das sind Bezeichner, keine
Prosa, und sie stehen ohnehin im Treiber — und bekommt eine **eigene**
Beschreibung dessen, was es fuer `kernel/src/drivers/bcm2711/emmc.rs`
bedeutet. Die Abschnittsangabe bleibt als Verweis stehen.

Die Form, an der sich die Umsetzung zu orientieren hat:

```
| 31 | OUT_OF_RANGE | Blockadresse jenseits der Kartenkapazitaet.
                     Unser Treiber meldet EUNDEV. Vgl. SD PLS §4.10.1.
```

Also: **Wirkung fuer unseren Treiber zuerst**, Quellenverweis zuletzt. Wo
ein Bit in unserem Treiber heute keine Wirkung hat, wird **das** gesagt —
nicht die Spec umschrieben, um die Zelle zu fuellen.

`references/README.md` haelt anschliessend fest, was fuer kuenftige
Uebernahmen aus dieser Quelle gilt, damit die naechste Einheit nicht wieder
hineinlaeuft (Abnahme oben).

---

## Abschluss (2026-09-29)

Weg 1 ist umgesetzt — an **zwei** Stellen, nicht an einer.

### `docs/hardware/emmc-sd.md` §5

Die Spalte mit den 15 woertlichen Saetzen ist weg. An ihrer Stelle steht
`Bit | Identifier | For our driver`, davor **ein** Absatz, der die
gemeinsame Wirkung einmal erklaert statt fuenfzehnmal: `check_r1()`
(`emmc.rs:714`) prueft alle Type-`E`-Bits als **eine** Maske, jedes gesetzte
Bit ergibt `Err(BlockError::Card)`, was `host_env.rs::block_error_code`
gleichfoermig auf `EREADF`/`EWRITF` abbildet — wie EmuTOS' `sd.c`.
`check_r1()` wird ausschliesslich nach `CMD17` und `CMD24` gerufen.

**Der einzige Unterschied je Bit ist die Erreichbarkeit, nicht der
Codepfad** — und genau das sagt die Tabelle jetzt:

- **Real erreichbar:** `OUT_OF_RANGE` (Bit 31) und `ADDRESS_ERROR` (Bit 30),
  die beiden, die Tabelle 4-43 fuer `CMD17`/`CMD24` ueberhaupt fuehrt. Bei
  Bit 31 steht die Defektgeschichte dabei: vor `check_r1()` kam ein
  Schreibvorgang hinter das Kartenende als `Ok(())` zurueck, weil niemand
  `RESP0` ansah.
- **Erreichbar, aber von unserer eigenen Anfrageform nicht ausgeloest:**
  `BLOCK_LEN_ERROR` (wir fordern immer 512), `WP_VIOLATION` (bekommt
  bewusst keinen eigenen TOS-Code, wie bei EmuTOS).
- **Unerreichbar, und ehrlich so benannt statt umschrieben:** die
  Erase-Familie, `LOCK_UNLOCK_FAILED`, `CSD_OVERWRITE`, `AKE_SEQ_ERROR` —
  wir senden die betreffenden Kommandos nie. Jede dieser Aussagen ist an
  eine konkrete Codestelle zurueckgebunden, nicht behauptet.

Damit ist die Gefahr von Weg 1 vermieden: keine Zelle ist gefuellt, indem
die Spezifikation umschrieben wurde.

### `kernel/src/drivers/bcm2711/emmc.rs` — die zweite Kopie

**Der Fund, der das Ticket erst vollstaendig gemacht hat.** Beim Schreiben
des Eintrags in `references/README.md` fiel auf, dass `emmc.rs:99-136`
**dieselbe** Beschreibungsspalte ein zweites Mal trug — Satz fuer Satz in
Anfuehrungszeichen, je `R1_*`-Konstante. Der Lead hat es nachgesehen und
bestaetigt.

Das ist genau [„Eine Begruendung steht selten nur einmal"](../docs/MEMORY.md),
nur lag die Kopie diesmal im Quelltext statt in einem zweiten Dokument.
**Haette dieses Ticket nur §5 gefasst, waere es formal geschlossen worden
und die woertliche Wiedergabe im oeffentlichen Repo unveraendert
vorhanden gewesen** — nur in einer anderen Datei. Deshalb wurde sie in
derselben Arbeitseinheit mitgenommen, nicht vertagt.

Die 15 Doc-Kommentare sagen jetzt, was das Bit **fuer diesen Treiber**
bedeutet; die Langform steht in §5, auf die sie verweisen.

**Beweisbar inert:** der Diff enthaelt ausser Kommentarzeilen nichts —
```
git diff -U0 kernel/src/drivers/bcm2711/emmc.rs | grep '^[+-]' \
  | grep -v '^[+-][+-][+-]' | grep -vE '^[+-]\s*(///|//)'
```
ist leer. Vom Lead nachgefahren.

### Sweep

Voller Baum, `/usr/bin/grep -rn -I` ueber 15 charakteristische Satzanfaenge.
**Eine** verbleibende Fundstelle: `measurements/measurement-2026-09-27-hw03/RUNINFO.TXT:47`
zitiert **einen** Satz (Bit 30) mit Quellenangabe.

**Bewusst stehengelassen, mit zwei Gruenden:** Messartefakte werden nach
[Gemeinsame Regeln](README.md) nicht nachtraeglich redigiert — ein
bearbeitetes Artefakt ist als Beleg beschaedigt —, und ein einzelner
zugeschriebener Satz ist der kleinere Fall, den `references/README.md`
Punkt 3 ausdruecklich weiter zulaesst. Das ist eine Entscheidung, kein
Uebersehen.

### Belege

Basislinien **unveraendert**: 962/0/1, 64/94 beide Boards, 80/125 beide
Feature-Saetze — vom Lead nachgefahren, mit `cargo clean … --release`
(siehe unten).

### Herausgefallen als eigenes Ticket

[T12](T12.md) — drei `sched_*`-Testdateien teilen `PROCESS_TABLE` ohne
Riegel, waehrend alle vier `aes_*`-Dateien einen haben. Aufgefallen, weil
ein Testlauf unter Compile-Last einmalig rot wurde.

### Nebenbefund zur Werkzeugkette

`cargo clean -p kernel -p tos-api --target aarch64-unknown-none-softfloat`
**ohne** `--release` raeumt fuer Release-Builds **nichts** weg. Die
Pruefvorschrift des Leads nannte das `--release` bisher nicht; beide
Board-Bauten dieses Tickets sind damit gefahren.
