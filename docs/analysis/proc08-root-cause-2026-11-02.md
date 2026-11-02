# PROC08 — Ursachenforschung: fehlende Stufe-3-Abbildung nach `Pexec(4)` (2026-11-02)

Ticket: `TASK/PROC08.md`, Arbeit Punkt 1 (Researcher). Stand: **Ursache nicht
belegt.** Dieser Bericht trennt, was gemessen ist, was aus dem Code folgt und
was Vermutung bleibt. Es gibt keinen Fix und keine Produktänderung; die
Entscheidung über das weitere Vorgehen liegt beim Anwender.

Kennzeichnung: **GEMESSEN** = im Pi-Log C1 oder in den QEMU-Läufen dieses
Auftrags zu sehen. **AUS DEM CODE** = gelesen, nicht gelaufen. **VERMUTUNG** =
Schluss ohne eigenen Beleg.

## 1. Kurzfassung

- Der Fehlerzeitpunkt ist eng eingegrenzt (Abschnitt 2): zView (Slot 2) kehrt aus
  dem **zweiten** `Pexec(4)` eines LDG-Moduls zurück; die Rückkehrregister sind
  korrekt, die Seite, von der der Befehl geholt wird, ist dieselbe Seite wie der
  eben ausgeführte `svc`. Zwischen `svc` und Rückkehr ist der Eintrag dieser
  Seite in der Tabelle des Elternprozesses ungültig geworden (oder die
  Hardware hat ihn so gesehen).
- Drei Erklärungen sind **widerlegt** (Tabellen-Arena erschöpft,
  Momentaufnahme der Blöcke zu klein, Prozesswechsel/Interrupt im
  Rückkehrpfad).
- Zwei Kandidaten bleiben, **keiner belegt**: (A) die Rechteverwaltung
  (`memprot`) hat dem Elternprozess das Eigentumsrecht an der eigenen Code-Seite
  in einem Zustand entzogen, den QEMU nicht erreicht; (B) die Tabellen waren
  richtig, der Cortex-A72 hat sie nicht so gesehen (TLB, Walk-Cache,
  Schreibreihenfolge). Reihenfolge nach Beleglage: A und B gleichauf, siehe
  Abschnitt 5.
- **Reproduktion in QEMU: negativ** (3 Versuche, 810 Modulstarts, 0 Fehler,
  `memprot=verify` ohne Abweichung). Das ist kein Entlastungsbeleg: QEMU bildet
  TLB, Walk-Cache und Caches des A72 nicht nach.
- Es gibt **ohne Codeänderung** einen trennenden Pi-Lauf: die Startoption
  `memprot=verify` (Abschnitt 6). Er trennt A von B, sofern der Fehler mit der
  Option noch auftritt.

## 2. Was das Pi-Log C1 hergibt (GEMESSEN)

Quelle: `measurements/measurement-2026-10-31-pitest01-c1/serial/screenlog-c1.txt`.

1. Ablauf vor dem Absturz, Zeilen 6661–6666: `[launch] EL0: slot 2 entry
   0x1c86000` (zView), danach genau zwei Paare `[exec] preparing program (slot
   3)` / `[exec] launching`, danach `[crash] slot 2 Bus Error esr=0x82000007
   pc=0x1cb497c`. Das erste Kind ist also normal zurückgekehrt (der Elternprozess
   hat danach ein zweites gestartet); der Absturz folgt dem **zweiten** Start.
2. Zwischen dem letzten `launching` und `[crash]` steht keine `[sched]`-Zeile: kein
   Prozesswechsel in diesem Fenster.
3. Rückkehrzustand (Registerauszug Zeilen 6670–6682): `x0 = 0` (Exit-Code des
   Kindes), `x1 = 0`, `x2 = 0x1dc6000` (der PD-Zeiger des Moduls, Argument von
   `Pexec(4)`), `x3 = 0`, `x8 = 0x4b`, `x30 = 0x1cb4964`. Das ist genau der Zustand
   aus dem Pexec-Snapshot mit überschriebenem `x0`; die Rückkehr selbst ist
   korrekt (`finish_terminate` hat Register und `ELR` richtig zurückgeschrieben).
4. `pc = 0x1cb497c` ist der Befehl nach dem `svc` (`bl ldg_cpush`, laut
   Ticket ELF `0x2f97c`); `x30 = 0x1cb4964` ist der Rücksprung
   des davor ausgeführten `ldg_cpush`-Aufrufs bei `0x1cb4960`. `0x1cb4960`,
   `0x1cb4964` und `0x1cb4978` (`svc`) liegen in **derselben 4-KiB-Seite
   `0x1cb4000`** wie `pc`. **Folgerung:** vor dem `svc` war die Seite abgebildet
   (sonst wäre der Befehl nie ausgeführt worden); danach nicht mehr.
5. Seite `0x1cb4000` liegt im 2-MiB-Bereich `0x1c00000..0x1dfffff`. Dort liegen
   auch der PD des Moduls (`0x1dc6000`), das TPA von zView (`basepage 0x1c85000`)
   und das Stack-Ende (`USP 0x1d7c1d0`). Alle Programme dieses Bootlaufs starten
   bei Eintritt `0x1c86000` (Zeilen 178, 3115, 4405, …): die Pool-Belegung ist
   deterministisch; der Unterschied zwischen dem Absturzlauf und dem
   fehlerfreien zweiten zView-Start liegt nicht in den Adressen.
6. Keine Meldung der beiden Prüfpfade, die der Kern am Ende jedes Traps ausgibt
   (`[memprot] page-table arena exhausted …`, `[memprot] VERIFY MISMATCH …`):
   `/usr/bin/grep -n -i "shortfall\|exhausted\|mismatch\|\[memprot\]"` auf dem Log ist leer.
   `MEM05 after program` in Zeile 6692 nennt `snapshot_short=0`.
7. Boot: Tabellen-Arena `0x01000000+2127 tables`, 357 gemeinsame L3-Tabellen
   (Zeilen 93, 95).

## 3. Der Codepfad (AUS DEM CODE)

### 3.1 Wie ein LDG-Modul gestartet wird

`ldg_load` (`sdk/ldg/ldg-r133/src/devel/ldgload.c:93`) macht der Reihe nach
`Pexec(5)` (Platzhalter-Basepage, um die Umgebung zu holen), `Malloc`,
`Mfree(env)`, `Mfree(module)`, `Pexec(3)` (Laden), `ldg_cpush`, `Pexec(4)`
(`ldgload.c:171@Pexec`), `ldg_cpush` (`ldgload.c:176@ldg_cpush`). Das Modul beendet sich nach
`ldg_init` über `exit`/`Pterm` (`sdk/ldg/build-ldg-demo.sh:21@Pterm`, Kopfkommentar zu
"its own Pterm"); ob das im Fehlerfall `Pterm` oder `Ptermres` war, ist aus dem Log
nicht unterscheidbar und für die folgende Analyse nicht tragend.

### 3.2 Speichermodell dahinter

- Jedes TPA ist beim Start der **größte freie Block** (E9, `alloc_tpa`), das
  Programm gibt in `crt0` per `Mshrink` zurück, was es nicht braucht
  (`sdk/crt0.S:25@Mshrink`). Ein per `Pexec(3)`/`Pexec(5)` geladenes Modul-TPA umfasst also
  bis zu seinem eigenen `Mshrink` fast den ganzen Pool (hier ~740 MB, 357
  2-MiB-Bereiche) und gehört dem Aufrufer.
- Mit Speicherschutz (Standard) hält jeder Prozess für jeden 2-MiB-Bereich, in
  dem seine Rechte vom allgemeinen Muster abweichen, eine **private L3-Tabelle**
  aus der Arena. Die Arena ist für "gemeinsame + zwei ungeschrumpfte TPAs + 64 je
  Slot" bemessen (`kernel/src/lib.rs:1050@PRIVATE_L3_BUDGET`,
  `kernel/src/lib.rs:981@count`).
- Ein `Pexec(4)`-Kind läuft im **Speicherraum seines Aufrufers**
  (`tos-api/src/bdos/process/pexec.rs:470@set_space`): Eltern- und Kind-Slot
  gehören zur selben Gruppe und halten dieselben Blöcke
  (`tos-api/src/bdos/process/memprot.rs:1457@set_space`).
- Rechte werden **berechnet, nicht fortgeschrieben**: Operationen markieren
  Bereiche (`memprot.rs:640@add_dirty`), `sync_now` schreibt sie aus Block-Liste,
  Eigentümern und Gruppen neu (`memprot.rs:772@sync_now`,
  `memprot.rs:1118@refresh`). Läuft ein Slot aus Tabellen (Arena leer), behält er
  das Muster der Nicht-Eigentümer — "it can lose access, never gain any"
  (Moduldoku, `memprot.rs:1235@note_shortfall`).

### 3.3 Rückkehr aus dem Kind

1. `handle_pexec` schreibt vor dem Umschalten `parent_l0_pa` = `TTBR0_EL1` des
   Aufrufers in den Kind-Slot (`kernel/src/arch/aarch64/exceptions.rs:2318@ttbr0_el1`,
   `exceptions.rs:2328@parent_l0_pa`) und schaltet über
   `restore_boot_ttbr0` + `install_ttbr0(child)` um
   (`exceptions.rs:2412@restore_boot_ttbr0`, `exceptions.rs:2416@install_ttbr0`).
2. `Pterm` des Kindes: `terminate_inner` räumt in einem `batch` auf
   (`tos-api/src/bdos/process/pterm.rs:377@batch`: Sweep über `free_all_owned`,
   `release_unstarted_of`); das `drop(batch)` führt `sync` aus, **während die
   Wurzel des Kindes noch aktiv ist**.
3. `finish_terminate` kopiert den Snapshot zurück, gibt den Kind-Slot frei
   (`exceptions.rs:1745@release_slot` → `tos-api/src/bdos/process.rs:1301@free_slot`
   → `memprot.rs:1487@on_slot_released` → `tos-api/src/bdos/process/mmu.rs:842@release_private_tables`
   + `flush_tlb_slot_released`), schaltet dann auf die leere Wurzel
   (`exceptions.rs:1779@restore_boot_ttbr0`: `msr ttbr0_el1` + `tlbi vmalle1`) und
   installiert die Wurzel des Elternprozesses (`exceptions.rs:1781@install_ttbr0`).
4. `install_ttbr0` ruft **zuerst** `memprot::sync_now` (`tos-api/src/bdos/process/mmu.rs:1688@install_ttbr0`);
   die dort verarbeiteten Änderungen, darunter die durch `on_slot_released`
   markierten, landen also in den Tabellen, bevor `TTBR0` geschrieben wird.
5. `rust_sync_handler` ruft am Trap-Ende erneut `sync_now`
   (`exceptions.rs:221@sync_now`) und gibt Verify- und Arena-Meldungen aus
   (`exceptions.rs:230@take_shortfall_report`).

Zwischen Schritt 3 und dem `eret` findet kein Prozesswechsel statt; IRQs sind im
SVC-Handler maskiert und werden nur in den Warteschleifen `pump_input` und den
blockierenden Lesepfaden kurz freigegeben
(`kernel/src/host_env.rs:90@pump_input`), die auf diesem Pfad nicht laufen.

## 4. Widerlegte Erklärungen

| Erklärung | Beleg gegen sie | Art |
|---|---|---|
| Tabellen-Arena erschöpft (Elternprozess behält Nicht-Eigentümer-Muster, also ungültig auf eigenen Seiten) | Der Kern meldet das am Trap-Ende (`exceptions.rs:230@take_shortfall_report`); im Log keine Meldung. Der Bedarf liegt weit unter der Arena: QEMU-Messung, Abschnitt 7: 6 private Tabellen belegt, 454 gemeinsame (Arena 2415), Pi: 2127 Tabellen, 357 gemeinsame. Bedarfsspitze beim ungeschrumpften Modul-TPA etwa 2 × 357. | GEMESSEN + AUS DEM CODE |
| Momentaufnahme der Blöcke zu klein, Blöcke fehlen, Eltern-TPA erscheint frei | `snapshot_short=0` (Log Zeile 6692, `memprot.rs:1009@snapshot` zählt jede Auslassung in `SNAP_SHORT`). | GEMESSEN |
| Prozesswechsel oder Interrupt zwischen Kindende und Rückkehr | Keine `[sched]`-Zeile im Fenster; Pfad läuft mit maskierten IRQs (Abschnitt 3.3). | GEMESSEN + AUS DEM CODE |
| Falsche Wurzel für den Eltern-Slot (`parent_l0_pa` falsch) | Der Fehler ist Stufe 3, nicht Stufe 0–2: Die Wurzel führt bis zur L3-Tabelle des Bereichs; mit leerer oder falscher Wurzel wäre das Level ein anderes. Außerdem lief derselbe Elternprozess vor dem `svc` unter derselben Wurzel. | AUS DEM CODE (Level der Meldung: Kern-Dekodierung im Log "translation fault, level 3") |

## 5. Kandidaten, nach Beleglage geordnet

### A. Rechteverwaltung entzieht dem Eltern-Slot seine eigene Seite (AUS DEM CODE möglich, nicht belegt)

Der Eintrag in der privaten L3 des Elternprozesses für `0x1cb4000` wird ungültig,
wenn `refresh` für diesen Slot und Bereich entweder `drop_private` wählt
(`needs == false`, d. h. der Slot "hält" in `0x1c00000..` keinen Block mit
abweichenden Rechten) oder die Seite als frei schreibt
(`write_pages`: kein Snapshot-Block deckt die Seite). Beides setzt voraus, dass
Eigentümer-Auflösung, Gruppen oder Snapshot für den Eltern-Slot in diesem Sync
falsch waren.

- **Dafür:** Das Fehlerbild (gültig vor dem `svc`, ungültig danach, im
  Bereich mit PD des Kindes) passt zu genau dieser Maschinerie; der
  Elternzustand ändert sich nur durch `Pexec`/`Pterm`-Bewegungen in diesem
  Bereich. Die Logik ist stark zustandsabhängig (Dirty-Liste mit Zusammenlegung
  bei `MAX_DIRTY = 32`, `check_structure`-Rückfall, Gruppen über `space_pd`,
  weitere Eigentümer in `region`), und der Fehler tritt nur nach einer
  bestimmten Vorgeschichte auf.
- **Dagegen:** `memprot=verify` (vollständige, unabhängige Neuberechnung nach
  jedem Sync) fand in QEMU über 155 Prüfungen und 50,9 Mio. Seitenvergleiche
  **keine** Abweichung; 800 Modulstarts in Schleife ohne Fehler; kein Zähler
  (`fallbacks 0`) deutet auf unangemeldete Strukturänderungen. Ich habe im Code
  keinen konkreten Pfad gefunden, der dem Eltern-Slot die eigene Seite entzieht.
  Wichtig: `verify` prüft nur Übereinstimmung **Tabelle gegen Modell**; ein Fehler
  **im Modell** (z. B. Eigentümer-Auflösung über `basepage_kva` in
  `Structure::slot_of`) bleibt dabei unsichtbar.
- **Nicht untersucht:** Vorgeschichte DOOM/DOOMNOKB (Abschnitt 8).

### B. Tabellen richtig, A72 sieht sie anders (VERMUTUNG, nicht belegt)

Eine Stufe-3-Translation-Fault verlangt, dass der Walker einen ungültigen
L3-Eintrag gelesen hat. Das geht entweder, weil der Eintrag im Speicher
ungültig war (= A), oder weil die Hardware einen veralteten Zwischeneintrag
(L2-Beschreiber im Walk-Cache zeigt auf eine inzwischen **freigegebene und
neu genullte** Arena-Tabelle) oder einen noch nicht sichtbaren Schreibzustand
benutzt hat.

- **Dafür:** Nicht in QEMU reproduzierbar (QEMU hat weder Walk-Cache noch
  A72-TLB; so auch die Richtung des Anwenders in `TASK/DOOM01.md`,
  Abschnitt "Richtung des Anwenders"). Der Pfad schreibt mitten in einem Trap
  viele Tabellenbeschreiber um und verwendet Arena-Tabellen sofort wieder
  (`tos-api/src/bdos/process/mmu.rs:469@arena_alloc` nimmt die niedrigste freie Nummer und nullt sie, nach
  `drop_private`/`release_private_tables`).
- **Dagegen:** Jede Änderung, die einen gültigen Eintrag schmälert, wird im Code
  von einer `tlbi vmalle1is`-Folge begleitet (`tos-api/src/bdos/process/mmu.rs:1193@replace_bbm`,
  `tos-api/src/bdos/process/mmu.rs:1365@flush_tlb`, `exceptions.rs:1779@restore_boot_ttbr0`), und das
  Umschalten der Wurzel verwirft die ganze TLB (ASIDs aus). Ich habe keinen
  konkreten Pfad gefunden, auf dem ein freigegebener Walk-Cache-Eintrag
  überlebt. Ob die Ordnung der Barrieren auf dem A72 genügt, ist eine Frage an
  die ARM ARM (Track B, Abschnitt "Offene Fragen"), die ich hier nicht aus dem
  Gedächtnis beantworte.
- **Auffälligkeit (AUS DEM CODE, nicht als Ursache belegt):** `finish_terminate`
  gibt den Kind-Slot und damit dessen private Tabellen frei
  (`exceptions.rs:1745@release_slot`), **solange die Wurzel des Kindes noch in
  `TTBR0_EL1` steht**; erst danach wird auf die leere Wurzel umgeschaltet
  (`exceptions.rs:1779@restore_boot_ttbr0`). Ebenso läuft das `sync` am Ende von
  `terminate_inner` (`pterm.rs:377@batch`) unter der Wurzel des Kindes. Der
  `tlbi` danach verwirft alles; ein Fehler entsteht daraus nur, wenn zwischen
  Freigabe und `tlbi` auf dem Kern ein Walk über den Bereich stattfindet. Das ist
  weder ausgeschlossen noch belegt.

### C. Verworfen, aber der Vollständigkeit halber

Hardwarefehler/Speicherfehler: kein Hinweis, der Fehler ist wiederholbar
unauffällig (zweiter Start fehlerfrei).

## 6. Trennender Pi-Lauf mit vorhandenen Mitteln

`cmdline.txt` mit `serial=on memprot=verify` (Startoption siehe
`kernel/src/lib.rs:1057@memprot_switch`). Danach den Ablauf des Absturzlaufs
wiederholen: DOOM, DOOMNOKB, dann zView; mehrere zView-Starts in einem Boot.
Ergebnis liest sich so:

| Beobachtung | Folgerung |
|---|---|
| `[memprot] VERIFY MISMATCH <kind> slot <s> va 0x… expected … table …` vor oder statt des Absturzes | A, und zwar im fortschreibenden Teil von `memprot`; Slot, Adresse und Art stehen in der Zeile. |
| Absturz bei `esr=0x82000007` ohne Abweichung | Tabelle = Modell. Dann entweder Modellfehler (A im Modell) oder B. Trennt nicht weiter — dafür der nächste Schritt. |
| Kein Absturz in vielen Starts | Verify verschiebt das Zeitverhalten (jede Prüfung ~300 k Seitenvergleiche); nicht als Entlastung werten. |

Wenn die Option nicht trennt: eine **merkmalgesteuerte** Erweiterung des
Absturzberichts für Befehlsabbrüche im Elternprozess (Produktänderung, nur nach
Freigabe, Standard aus, Bytegleichheit ohne Merkmal wie in `CLAUDE.md`
verlangt): ausgeben, was der Software-Walk `mmu::translate(slot, far)`
(`tos-api/src/bdos/process/mmu.rs:1593@translate`) für die Fehleradresse sagt, den gelesenen `TTBR0_EL1`
gegen `mmu::l0_pa_for(slot)` und den L2-Eintrag des Bereichs. Gültig im
Software-Walk, ungültig in der Hardware = B; ungültig im Software-Walk = A im
Modell.

## 7. QEMU-Reproduktion (GEMESSEN)

Lauf: `measurements/measurement-2026-11-02-proc08/RUNINFO.TXT`. Kern: Release,
Quelle unverändert gegenüber dem Pi-Kern (kein Diff in `kernel/ tos-api/
tos-types/ sdk/ programs/` seit `6796787c`), QEMU 11.0.2 `raspi4b`,
`serial=on`.

| Versuch | Inhalt | Ergebnis |
|---|---|---|
| a1 | zView per `#Z`, `memprot=verify` | 10 Modulstarts, kein Absturz, 155 Prüfungen, 0 Abweichungen |
| a2 | `LDGTEST.NCP` (69 Prüfungen mit `ldg_open`/`ldg_close`, zwei gleichzeitig) | ALL PASS, 0 Abweichungen |
| a4 | Sonde `ldgloop.c`: 400 Wiederholungen von 2 × `ldg_open`, 2 × `ldg_close` mit dem echten `libldg`-Pfad `Pexec 5/3/4` | 800 Modulstarts, `bad=0`, größter freier Block wie am Anfang, kein Absturz |
| a3 | Zustandsprüfung, kein Versuch: zView, Speicherauszug bei t = 40 s und 80 s | Arena-Invarianten halten (keine Tabelle doppelt referenziert, keine als frei markiert, keine Leichen); je Slot 0, 1, 2 zwei private Tabellen |

**Nicht reproduziert.** Das ist nur ein Nachweis, dass der Fehler im Normalbetrieb
von QEMU nicht auftritt. Die Vorgeschichte des Pi-Laufs (DOOM, DOOMNOKB, Absturz-
und Neustartfolge) wurde nicht nachgespielt: `DOOM1.WAD` liegt nur auf den
Karten-Backups, und mehrere zView-Starts in einem Boot brauchen
Tasteneingabe (Injektor) — außerhalb der drei Versuche.

## 8. Offene Punkte

- Verbindung zu DOOM01 (Ticket: "nicht belegt"): bleibt unbelegt. Beide Läufe
  teilen nur die Vorgeschichte; ein gemeinsamer Mechanismus (z. B. über
  `usermem` und die ungepufferten Bildschirmseiten) ist nicht untersucht.
- Eigentümer-Auflösung `Structure::slot_of`
  (`memprot.rs:883@slot_of`) ist reiner Cookie-Vergleich
  über `basepage_kva`; ob ein Fall existiert, in dem der Cookie des
  Elternprozesses nach einer Slot-Wiederverwendung (Slot 2 hostet jedes Programm
  des Laufs) kurz keinem lebenden Slot zugeordnet ist, habe ich nicht
  ausgeschlossen. Hier würde `verify` nichts sehen (Modellfehler).
- ARM-ARM-Frage (Track B), nicht beantwortet: Genügt auf dem A72 die Folge
  "Eintrag ungültig, `dsb`, `tlbi vmalle1is`, `dsb`, neuer Eintrag" auch für
  zwischengespeicherte Zwischenebenen, wenn der Walk durch spekulative Zugriffe
  ausgelöst werden kann, solange `TTBR0` eine Wurzel mit Einträgen trägt?

## 9. Vorschlag (Anwender entscheidet)

1. **Zuerst** der Pi-Lauf mit `memprot=verify` (Abschnitt 6): kostet nur
   `cmdline.txt` und Zeit, keine Codeänderung.
2. **Wenn nicht trennend:** merkmalgesteuerte Absturzdiagnose (Abschnitt 6), eine
   kleine Produktänderung, Freigabe nötig.
3. **Fix-Richtung erst danach.** Heute eine zu nennen wäre geraten:
   - verifiziert eine Abweichung: Korrektur im fortschreibenden Teil von
     `memprot` an der gemeldeten Stelle;
   - Modellfehler: Eigentümer-/Gruppenauflösung korrigieren, mit Host-Test, der
     die gemeldete Folge nachspielt;
   - B: Reihenfolge in `finish_terminate` und `install_ttbr0` gegen die ARM ARM
     prüfen (Track B), nicht nach Gefühl umstellen.
4. **Abnahme eines späteren Fixes:** (a) Host-Test, der die gefundene Folge
   nachspielt; (b) QEMU: Sonde `ldgloop` mit mindestens 800 Modulstarts und
   `memprot=verify` ohne Abweichung; (c) Pi: Vorgeschichte des Absturzlaufs
   (DOOM, DOOMNOKB) und danach mindestens 20 aufeinanderfolgende zView-Starts
   (je ~10 Modulstarts) ohne Absturz, auf demselben Kartenstand. Die Rate des
   Fehlers ist nur aus einem Ereignis in zwölf Modulstarts bekannt; (c) ist
   deshalb statistisch schwach und ersetzt die Diagnose nicht.

## Out-of-scope-Beobachtungen

- `exceptions.rs:2160@handle_pexec` und `finish_terminate` tragen die QUIRK-001-
  Klammern (UART-Ausgaben zwischen `tlbi` und `TTBR0`-Schreiben); auf dem Pi
  verschieben sie das Zeitverhalten des Pfades (relevant für Abschnitt 6).
- `docs/MEMORY.md` QUIRK-001 beschreibt die Gleichadress-Ursache als erledigt
  (ADR-028); die Klammern bleiben im Code. Kein Befund zu PROC08.
