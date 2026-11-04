# PROC08 — Kandidat B: welcher Übersetzungszustand kann veraltet sein? (2026-11-04)

Ticket: `TASK/PROC08.md`, Arbeit Punkt 1 (Researcher), Fortsetzung von
`docs/analysis/proc08-root-cause-2026-11-02.md`. Nur Lesen und Dokumentieren;
kein Fix, keine Produktänderung, keine Karte, keine Pi.

Beleg: `measurements/measurement-2026-11-04-proc08-recur/RUNINFO.TXT` (Foto des
Schirmberichts, Kern `6407f17c…` mit `crash-walk`, serielle Ausgabe aus),
früheres Vorkommen `measurements/measurement-2026-10-31-pitest01-c1/RUNINFO.TXT`.

Kennzeichnung (jede Aussage trägt eine):

- **GEMESSEN** — im Foto/Log der Läufe zu sehen.
- **CODE** — im NewTOS-Quelltext gelesen (Cite `Pfad:Zeile@Symbol`).
- **ARM** — in der ARM ARM (DDI 0487 M.d, `references/arm-arm/DDI0487M_d_a-profile_architecture_reference_manual.pdf`)
  oder im Cortex-A72-TRM (`references/arm-arm/cortex_a72_mpcore_trm_100095_0003_06_en.pdf`)
  gelesen; Regelkennung (z. B. `RDDMVT`) und Seite wie im Modulkopf von
  `kernel/src/arch/aarch64/crash_walk.rs`.
- **SCHLUSS** — aus den obigen gefolgert, nicht selbst belegt.
- **VERMUTUNG** — kein Beleg.

## 1. Kurzfassung

1. **Gemessen ist nur:** Wurzel richtig, Tabellen und Modell sagen „abgebildet“,
   `AT S1E0R` sagt „Übersetzungsfehler Stufe 3“ — im Zeitpunkt des Absturzberichts,
   Mikrosekunden bis wenige Millisekunden nach dem fehlgeschlagenen Abruf (Abschnitt 2).
   Was die Hardware dabei hielt, ist nicht gemessen.
2. **Ein Übersetzungsfehler wird nie zwischengespeichert** (ARM `RXCLRD`, D8-7852).
   Die Hardware hat den Fehler also nicht aus einem veralteten *Blatt*eintrag, sondern
   weil der Walker auf Stufe 3 einen Deskriptor `0` las. Das geht nur, wenn er
   (a) über einen veralteten **Zwischeneintrag** (Walk-Cache; der A72 hat „Intermediate
   table walk caches“, TRM 5.1/5.6, S. 5-276/5-282, mit ASID-Zuordnung) in eine
   **andere L3-Tabelle** als `arena#361` geführt wurde, oder (b) den Inhalt von
   `arena#361[180]` nicht so sah, wie ihn die CPU schrieb (Speicher-/Cache-Sicht des Walkers).
3. **Wahrscheinlichster Mechanismus (Rang 1, SCHLUSS+VERMUTUNG):** ein Zwischeneintrag der
   **Wurzel des soeben beendeten Kindes** (Slot 3) überlebt den Wechsel zurück zum Elternprozess.
   Der Kern räumt beim Kindende die privaten Tabellen des Kindes ab, **solange dessen Wurzel
   noch in `TTBR0_EL1` steht**, und schaltet erst danach um; die Umschaltung selbst
   (`restore_boot_ttbr0`) hat **kein `isb` zwischen `msr ttbr0_el1` und `tlbi`**, anders als
   der Kern es an anderer Stelle selbst nach `RVNRFW` ausführt. Ein Abruf, der in diesem
   Fenster mit der alten Wurzel übersetzt wird, trägt einen Eintrag ein, den keine spätere
   Invalidierung mehr trifft (nach `restore_boot_ttbr0` folgt vor dem `eret` **kein unbedingtes** `tlbi`
   mehr; nur ein `sync_now` mit geändertem gültigem Blatt oder BBM löst eins aus). Ein solcher Eintrag zeigt nach `release_private_tables` auf die **gemeinsame** L3
   des Bereichs, in der zViews eigene Seiten für Nicht-Eigentümer ungültig sind: genau
   „Stufe-3-Fehler auf der ersten Seite des Elternprozesses“. **Offen:** der Auslöser des Abrufs
   (welcher Niederadress-Zugriff läuft im Fenster?) ist nicht gefunden (Abschnitt 5, K1).
4. **Rang 2:** Walker liest `arena#361[180]` aus einer nicht kohärenten Sicht (Cache). Nach
   den Quellen unwahrscheinlich, aber billig zu trennen (Abschnitt 9.1, Stufe `dc`).
5. **Rang 3:** Fehler/Eigenheit des A72 (TLBI trifft Walk-Cache-Eintrag nicht). Die
   Errata-Notiz (SDEN) liegt nicht unter `references/`; nicht bewertbar, Beschaffung nötig.
6. **Entscheidend für die Trennung:** eine kleine **Leiter** im Absturzbericht (Abschnitt 9.1):
   zweiter `AT` (bleibt es?), `AT` auf Nachbarseiten, dann `dc civac` der Deskriptorzeilen,
   `tlbi vaale1is`, `tlbi vaae1is`, `tlbi vmalle1is`, nach jedem Schritt `AT`; Ausgabe „fixed-by …“
   in einer Schirmzeile. Das unterscheidet Rang 1/3 (Zwischeneintrag) von Rang 2 (Speichersicht)
   und von „vorübergehend“.
7. **Fix-Skizze für Rang 1** (Abschnitt 10): `restore_boot_ttbr0` vor `release_slot` stellen
   (so wie der Unwind-Zweig es schon tut) und in `restore_boot_ttbr0` ein `isb` zwischen `msr`
   und `tlbi` setzen. Nicht implementiert.

## 2. Berichtigung zum Auftrag: „AT fällt noch lange nach der Rückkehr“

Der Absturzbericht läuft im Trap des fehlgeschlagenen Abrufs. Zwischen Fehler und `AT`
liegen nur Handler-Prolog, `kill_crashed_program` (FAR lesen, Bericht aufbauen, ein
`kprintln!`) und der Anfang von `crash_walk::report`
(`kernel/src/arch/aarch64/exceptions.rs:1498@report_and_wait`,
`kernel/src/arch/aarch64/crash.rs:96@crash_walk`,
`kernel/src/arch/aarch64/crash_walk.rs:94@report` — der `AT` kommt vor dem Software-Walk).
**CODE.** Der `AT` beweist deshalb **nicht**, dass ein veralteter Zustand lange hält; er
beweist nur, dass die MMU im selben Augenblick wie der Abruf noch genauso antwortet. Eine
rein zeitlich begrenzte Ursache (Speicherordnung, die erst nach einigen µs sichtbar wird)
ist damit **nicht ausgeschlossen**. Die Leiter (Abschnitt 9.1) enthält deshalb als ersten
Schritt einen **zweiten `AT` ohne jede Maßnahme**.

## 3. Der Ablauf, Schritt für Schritt (alles CODE)

Zu `TTBR0_EL1`: Wurzel des Slots = `mmu::l0_pa_for(slot)`. ASIDs sind aus (Standard,
`cmdline.txt` leer): alle Wurzeln und die leere Wurzel laufen mit ASID 0, Blätter sind
global (`nG = 0`, `tos-api/src/bdos/process/mmu.rs:169@D_NG`, `descriptor_bits`);
jeder Wurzelwechsel invalidiert die ganze TLB. Slot 2 = zView (Eltern), Slot 3 = Modul (Kind).
Region R = 2-MiB-Bereich `0x1c00000..0x1dfffff` (enthält zViews TPA `1c85000-1d7d000`,
den Stack, den PD des Moduls). `S` = gemeinsame L3 von R, `P` = `arena#361` = private L3
von Slot 2 für R, `C` = private L3 von Slot 3 für R.

### 3.1 Pexec(3) im Elternprozess (Wurzel Slot 2 aktiv)

| Schritt | Code | Barriere / TLBI | Tabellen |
|---|---|---|---|
| Slot 3 anlegen | `tos-api/src/bdos/process/loader.rs:687@init_slot_tables` → `tos-api/src/bdos/process/mmu.rs:799@init_slot_tables` | **keine** | `release_private_tables(3)` gibt Reste des Vormieters frei (`tos-api/src/bdos/process/mmu.rs:842@release_private_tables`, `arena_free`), danach werden L0/L1/L2/L3 von Slot 3 **an Ort und Stelle** neu geschrieben (kein BBM; Vertrag: Slot nicht aktive Wurzel) |
| Rechte der neuen Wurzel | `loader.rs:707@on_root_built` → `memprot.rs:1372@on_root_built` → `sync` | je nach Änderung `flush_tlb` | `refresh` (`memprot.rs:1118@refresh`): für Slot 3 `make_private` (`tos-api/src/bdos/process/mmu.rs:1229@make_private`): `arena_alloc` (`tos-api/src/bdos/process/mmu.rs:469@arena_alloc`: niedrigste freie Nummer, **wird genullt**), Kopie von `S`, `dsb ish`, dann `replace_bbm` (`tos-api/src/bdos/process/mmu.rs:1193@replace_bbm`): `0` schreiben, `dsb ish`, `tlbi vmalle1is`, `dsb ish`, `isb`, neu schreiben, `dsb ish`, `isb` |
| Ende des Traps | `exceptions.rs:221@sync_now` | `flush_tlb` | normalerweise nichts offen |

### 3.2 Pexec(4) im Elternprozess

| Schritt | Code | Barriere / TLBI |
|---|---|---|
| Besitz/Raum | `pexec.rs:470@set_space` → `memprot.rs:1457@set_space` → `sync()` | `flush_tlb`: bei geänderten gültigen Blättern `dsb ish; tlbi vmalle1is; dsb ish; isb` (`tos-api/src/bdos/process/mmu.rs:1365@flush_tlb`, `tos-api/src/bdos/process/mmu.rs:1428@tlbi_all_barriered`), sonst nur `dsb ish; isb` (`publish_user_mappings`, `tos-api/src/bdos/process/mmu.rs:1739@publish_user_mappings`) |
| Wurzel merken | `exceptions.rs:2318@ttbr0_el1`, `exceptions.rs:2327@parent_l0_pa` | — |
| Umschalten auf das Kind | `exceptions.rs:2412@restore_boot_ttbr0` | `dsb ishst; msr ttbr0_el1, leer; tlbi vmalle1; dsb ish; isb` (`kernel/src/arch/aarch64/user_entry.rs:124-132@tlbi`; **`msr` und `tlbi` ohne `isb` dazwischen**) |
| | `exceptions.rs:2416@install_ttbr0` → `tos-api/src/bdos/process/mmu.rs:1688@install_ttbr0` | `sync_now`, dann `dsb ish; msr ttbr0_el1, Kind; isb` (`tos-api/src/bdos/process/mmu.rs:1710-1717@ttbr0_el1`); **kein `tlbi`** |
| `eret` nach EL0 | — | — |

### 3.3 Kind endet (Pterm), Rückkehr zum Elternprozess

| Schritt | Code | Barriere / TLBI / Tabellen |
|---|---|---|
| Aufräumen unter der Kind-Wurzel | `pterm.rs:360@batch` … `pterm.rs:377@drop` | `drop(batch)` → `sync_now`: Rechte neu, `flush_tlb` (vmalle1is bei geänderten gültigen Blättern); **Wurzel des Kindes aktiv** |
| Slot freigeben | `exceptions.rs:1745@release_slot` → `pterm.rs:419@release_slot` → `process.rs:1301@free_slot` → `memprot.rs:1487@on_slot_released` | **Wurzel des Kindes noch aktiv.** `release_private_tables(3)` (`memprot.rs:1514@release_private_tables`): L2-Einträge von Slot 3 für R **gültig → gültig** auf die gemeinsame Vorlage `S` zurück, `C` geht an die Arena (`arena_free`, **nicht genullt**); **kein BBM** — `tos-api/src/bdos/process/mmu.rs:842@release_private_tables` verlässt sich auf den Aufrufer |
| | `memprot.rs:1517@flush_tlb_slot_released` → `tos-api/src/bdos/process/mmu.rs:1410@flush_tlb_slot_released` → `flush_tlb_always` | `dsb ish; tlbi vmalle1is; dsb ish; isb` — **Wurzel des Kindes noch in `TTBR0_EL1`** |
| (Rest von `free_slot`, `release_client`, …) | `process.rs:1301@free_slot` | Kernel läuft weiter, `TTBR0_EL1` = Kind-Wurzel, deren Tabellen jetzt auf `S` zeigen |
| Auf leere Wurzel | `exceptions.rs:1779@restore_boot_ttbr0` | `dsb ishst; msr ttbr0_el1, leer; tlbi vmalle1; dsb ish; isb` — letzte Invalidierung vor dem Elternprozess; **`msr` und `tlbi` ohne `isb` dazwischen** |
| Eltern-Wurzel | `exceptions.rs:1781@install_ttbr0` → `tos-api/src/bdos/process/mmu.rs:1688@install_ttbr0` | zuerst `sync_now`: die durch `on_slot_released` vorgemerkten Änderungen werden jetzt in die Tabellen geschrieben (`refresh`, ggf. `make_private`/`drop_private` mit `arena_alloc` — **kann `C` sofort wiederverwenden**), `flush_tlb`; dann `dsb ish; msr ttbr0_el1, Eltern; isb`. **Danach kein unbedingtes `tlbi` mehr:** `flush_tlb` invalidiert im `sync_now` nur, wenn `set_leaf` ein gültiges Blatt geändert hat oder `replace_bbm` lief; schreibt `refresh` nur gleiche Werte (`set_leaf` kehrt bei `old == desc` zurück), bleibt es bei `dsb ish; isb`. Ein nach `restore_boot_ttbr0` nachgeladener Eintrag wird dann **nur zufällig** noch getroffen. |
| Trap-Ende | `exceptions.rs:221@sync_now` | normalerweise nichts offen; `eret` |

`sync_now` im Trap-Ende und in `install_ttbr0` ruft `flush_tlb` nur mit `tlbi`, wenn
`set_leaf` einen **gültigen** Blatteintrag geändert hat (`tos-api/src/bdos/process/mmu.rs:1307@set_leaf`,
`TLB_DIRTY`); ungültig→gültig braucht nach `RXCLRD`/`IWZCBG` (D8-7852/7858) keine
Invalidierung. **CODE + ARM.**

## 4. Die Regeln, gegen die geprüft wird

- **Zwischeneinträge werden gespeichert, auch spekulativ** (ARM `RSQBCS`, `IRCGCH`, D8-7852:
  jeder Eintrag, der keinen Übersetzungs-/Adressgrößen-/Access-Flag-Fehler macht, darf „as the
  result of an explicit or speculative access“ in „TLB or intermediate TLB caching structure“
  stehen — zu jeder Zeit, solange die Übersetzung eingeschaltet und die Wurzel nicht
  niedrigerer Stufe ist). Fehler selbst nie (`RXCLRD`).
- **A72-TRM 5.6 (S. 5-282):** eigene „intermediate table walk caches“; Einträge sind einer ASID
  zugeordnet. Das TRM beschreibt dort **genau dieses Fehlerbild** für eine reservierte ASID:
  Spekulation trägt einen Zeiger auf eine Tabelle zweiter Stufe ein, der Prozess endet, das
  Betriebssystem gibt die Tabelle frei und verwendet sie neu, ein späterer Wechsel trifft den
  Eintrag, die neu belegte Tabelle liefert „zufällige“ Daten. Merksatz im TRM: bei reservierter
  ASID **muss die TLB invalidiert werden, um Tabellenspeicher freizugeben**.
  NewTOS mit `asid=off` ist dieser Fall im Ganzen: **jede** Wurzel (auch die leere) trägt
  ASID 0.
- **TLBI VMALLE1(IS)** trifft alle Ebenen, global und nicht-global (ARM `RPMNQC`, D8-7867/7868);
  `TLBI VAAE1(IS)` trifft alle Ebenen für die Adresse, alle ASIDs (`RZSXRQ`, D8-7869);
  `TLBI VAE1(IS)` nur nicht-globale Einträge jeder Ebene **mit angegebener ASID** und globale
  des Endlevels (`RVMSWQ`, D8-7868) — für unseren Fall (ASID aus, globale Blätter, Zwischeneinträge
  ASID 0) wäre `VAE1` mit falscher ASID gefährlich, deshalb ist `VMALLE1` richtig.
- **Fertigstellung:** ein TLBI ist erst nach `DSB` **und** einem Context-Synchronization-Event
  fertig (ARM `IBLDZX`, D8-7875; der A72 hat kein FEAT_ETS2). Bis dahin dürfen spekulative
  Zugriffe die alten Übersetzungsdaten benutzen (`ICPZSM`, D8-7875; erst danach nicht mehr:
  `ILXHLD`).
- **Ein Schreiben von `TTBR0_EL1` wirkt für die Übersetzung erst nach einem Context-Synchronization-Event**
  (ARM `IRBMTM`, D8-7700: „direct writes to TTBR_ELx are not required to be observed by
  indirect reads until completion of a Context synchronization event“). Der Kern befolgt
  das an anderer Stelle selbst: `kernel/src/arch/aarch64/mmu.rs:183@size_asids` führt
  `msr tcr_el1; isb; tlbi vmalle1; dsb ish; isb` aus und zitiert `RVNRFW` (D8-7858).
- **Break-before-make** (`RDDMVT`, D8-7859) für Änderungen des Tabellenzeigers: für die
  L2-Einträge der Slots eingehalten (`replace_bbm`, `tos-api/src/bdos/process/mmu.rs:1193@replace_bbm`); **nicht**
  für `release_private_tables` (gültig→gültig, abhängig von der anschließenden
  Invalidierung durch den Aufrufer).

## 5. Fenster, in denen ein Eintrag der falschen Wurzel überleben kann

Mit ASID 0 für alle Wurzeln gilt: ein Eintrag, der zum Zeitpunkt eines Wurzelwechsels im
Cache steht oder danach unter der alten Wurzel nachgeladen wird, gilt auch unter der neuen.
Alles, was vor dem *letzten* wirksamen Invalidieren steht, ist harmlos. Gesucht ist also
jeder Zugriff, der **nach dem letzten TLBI** noch mit der Kind-Wurzel (oder mit nicht mehr
gültigen Tabellen) übersetzt wird.

### K1 — Nachladen mit der Kind-Wurzel zwischen `tlbi` und `isb` in `restore_boot_ttbr0`

- **Code (CODE):** `kernel/src/arch/aarch64/user_entry.rs:124-132@tlbi`:
  `dsb ishst; msr ttbr0_el1, leer; tlbi vmalle1; dsb ish; isb`. Der gleiche Aufbau in
  `tos-api/src/bdos/process/mmu.rs:1895@tlbi` (`switch_ttbr0_kernel`).
- **Regel (ARM):** `IRBMTM` (D8-7700) — das `msr` ist für Übersetzungen erst nach dem `isb`
  wirksam; `IBLDZX`/`ICPZSM` (D8-7875) — das TLBI ist erst nach `dsb` + `isb` fertig, bis
  dahin sind spekulative Zugriffe mit alten Daten erlaubt. Zwischen `tlbi`-Ausführung und
  `isb` kann also ein Zugriff noch die **alte** `TTBR0_EL1` benutzen und einen
  Zwischeneintrag **nach** dem Invalidieren eintragen. Es gibt in diesem Fenster nichts mehr,
  was ihn entfernt.
- **Was dann im Cache stünde (SCHLUSS):** beim Rückweg ist die alte Wurzel die des Kindes,
  und `release_private_tables` hat deren L2-Eintrag für R bereits auf `S` zurückgesetzt
  (Abschnitt 3.3). Ein Eintrag „R → `S`“ (oder L0/L1-Einträge von Slot 3, die zu Slot 3s L2
  führen, dessen Eintrag R → `S` zeigt) liefert für zViews TPA-Seiten in `S` den
  Deskriptor `0` (Modus `P`, Nicht-Eigentümer: ungültig) → **Übersetzungsfehler Stufe 3**,
  in der gesamten 2-MiB-Region, beim ersten Zugriff (die TLB wurde ja ganz geleert, es gibt
  kein Blatt). Passt zu Foto und C1.
- **Was fehlt (VERMUTUNG):** ein Zugriff auf eine Niederadresse im Fenster. Gefunden ist
  keiner: die Kernpfade dort laufen im Hochhälften-Bereich (TTBR1). Spekulation (falsch
  vorhergesagter indirekter Sprung/Ladeoperation mit einem Benutzerzeiger in einem Register)
  ist architektonisch erlaubt, aber hier nicht belegt. K1 ist damit der **einzige
  gefundene Verstoß gegen eine ARM-Regel auf dem Rückweg**, aber **ohne belegten
  Auslöser**.
- **Fenster-Länge:** `tlbi` + `dsb ish` bis `isb` — auf dem A72 nicht aus Quelle bekannt
  (nicht behaupten). Breadcrumbs (Abschnitt 9.2) messen sie.

### K2 — Wurzel des Kindes bleibt aktiv, während seine Tabellen freigegeben werden

- **Code (CODE):** Abschnitt 3.3, Zeilen „Slot freigeben“ bis „Auf leere Wurzel“:
  `release_private_tables(3)` und `arena_free(C)` laufen mit `TTBR0_EL1` = Wurzel Slot 3;
  die Invalidierung `flush_tlb_slot_released` folgt, aber danach läuft der Kern noch
  (`free_slot`, `release_client`, …) bis `restore_boot_ttbr0`, wieder mit der Kind-Wurzel
  live. Der Unwind-Zweig der gleichen Funktion tut es **anders** und kommentiert den Grund:
  „The boot tables go in first: the slots released here take their page tables with them
  while `TTBR0` still names the dying process's“ (`exceptions.rs:1614-1620@restore_boot_ttbr0`).
  Der Normalweg (`exceptions.rs:1745@release_slot`, `:1779@restore_boot_ttbr0`) hat diese
  Reihenfolge nicht.
- **Regel (ARM/TRM):** `RGPPYH` (D8-7858): wird ein Tabelleneintrag geändert, muss die
  entsprechende TLB-Eintragung invalidiert werden, „including speculative execution“; TRM 5.6:
  Tabellenspeicher darf erst freigegeben werden, wenn keine Wurzel mehr auf ihn zeigen kann
  und die TLB invalidiert ist. Hier steht die Wurzel noch in `TTBR0_EL1`, und das einzige
  Invalidieren dazwischen (`flush_tlb_slot_released`) ist **nicht** das letzte.
- **Wirkung (SCHLUSS):** K2 ist für sich genommen *durch* K1-artige Nachladungen harmlos, solange
  `restore_boot_ttbr0` wirksam ist; K2 **vergrößert** aber den Zeitraum, in dem ein Nachladen
  mit der alten Wurzel *auf freigegebene/umgeschriebene Tabellen* trifft, und ist die
  Voraussetzung dafür, dass das Nachgeladene „R → `S`“ statt „R → `C`“ lautet. (Wäre es
  „R → `C`“, hätte `C` — freigegeben, nicht genullt, Inhalt = Rechte des Kindes inkl. der
  Seiten des Elternprozesses — zufällig *gültige* Seiten; Fehler träte erst auf, wenn `C`
  von `arena_alloc` neu belegt (genullt) wird, was `install_ttbr0`s `sync_now` tun kann.)

### K3 — Änderungen an Tabellen unter der *laufenden* Eltern-Wurzel nach `install_ttbr0`

- **Code (CODE):** `exceptions.rs:221@sync_now` (Trap-Ende) läuft mit der Eltern-Wurzel in
  `TTBR0_EL1`. Hier stehende Änderungen: `set_leaf` gültig→ungültig/gültig→anders löst
  `tlbi vmalle1is` aus; L2-Zeigerwechsel laufen über `replace_bbm`. **Kein Pfad gefunden**, auf
  dem dort ein Zwischeneintrag ohne passendes Invalidieren überlebte. Nicht der Auslöser;
  der Pfad ist nur mit der Leiter (kein Auslöser nötig) und den Breadcrumbs (Zähler „offen
  am Trap-Ende“) abzusichern.

### K4 — `init_slot_tables` ohne Invalidierung (Pexec(3))

- **Code:** `tos-api/src/bdos/process/mmu.rs:799@init_slot_tables` schreibt Slot-Tabellen an Ort und Stelle um, ohne
  `tlbi`, mit der Begründung „Slot nicht aktive Wurzel“. Eintreten kann der Fehler nur,
  wenn ein Zwischeneintrag der Wurzel von Slot 3 im Cache steht. Das ist im Rückweg-Szenario
  nur durch K1/K2 möglich; als eigenständiger Auslöser nicht belegt. (Schwächer als K1/K2;
  nicht isoliert prüfbar.)

### K5 — Kein Auslöser, aber „Verbotene“ Reihenfolge: `msr ttbr0` vor `tlbi` ohne `isb` beim *Hin*weg

- Dieselbe `restore_boot_ttbr0` läuft beim Start des Kindes (`exceptions.rs:2412@restore_boot_ttbr0`).
  Ein dort nachgeladener Eintrag trägt die **Eltern**-Wurzel („R → `P`“). Das Kind
  (Gruppe mit dem Elternprozess, gleiche Blöcke) sieht dann Rechte des Elternprozesses statt
  seiner eigenen, was meist unbemerkt bleibt, solange die Seiten, die das Kind braucht, in
  `P` gültig sind (Inferenz aus `memprot.rs:1457@set_space`; **nicht gemessen**). **Das
  erklärt, warum der Fehler nur beim Rückweg sichtbar wäre**: dort liefert die falsche Wurzel
  *weniger* Rechte (`S`, nichts gültig), beim Hinweg ungefähr *dieselben* (SCHLUSS).

## 6. Ausgeschlossene Erklärungen (jede mit Beleg)

| Erklärung | Beleg |
|---|---|
| Falsche Wurzel in `TTBR0_EL1` | Foto: `ttbr0 2f0000 asid 0 = slot 2, exp 2f0000 rec 2f0000 same` (GEMESSEN) |
| Tabellen ungültig (Kandidat A) | Foto: `L3[180]=20000001cb4743 … mapped L3`, `table==model yes`, `holds yes … pend no` (GEMESSEN) |
| Arena erschöpft | Foto: `short 0` (GEMESSEN) |
| Fehler hängt an DOOM/Vorgeschichte | Anwenderaussage 2026-11-04: Pi eingeschaltet, zView als **erstes** Programm gestartet, nichts lief vorher (Aussage des Anwenders, nicht gemessen). Damit ist DOOM keine Vorbedingung. |
| Leaf-TLB-Eintrag als Ursache | Ein Übersetzungsfehler wird nie als Blatt gespeichert (`RXCLRD`); nach dem Wechsel gibt es zudem kein Blatt (Abschnitt 3.3, volle TLB-Invalidierung) |
| TLB-Conflict-Abort | Der Fehler ist `esr=0x82000007` = Übersetzungsfehler Stufe 3 (GEMESSEN), nicht FST `0x30` |

## 7. Wie gut erklärt jeder Kandidat die Fakten

Fakten: **F1** erster Abruf nach der Rückkehr; **F2** Stufe-3-Übersetzungsfehler;
**F3** Wurzel richtig, Software-Walk und Modell „abgebildet“; **F4** `AT` im selben Moment
ebenfalls Fehler; **F5** gleicher `pc`/`X30`/Layout in zwei Läufen, davon einer ohne
Vorgeschichte; **F6** zweiter zView-Start (C1) und zehn Modulstarts ohne Fehler; **F7**
nicht in QEMU; **F8** nur auf dem Rückweg beobachtet.

| | F1 | F2 | F3 | F4 | F5 | F6 | F7 | F8 | Rang |
|---|---|---|---|---|---|---|---|---|---|
| **R1** Zwischeneintrag der Kind-Wurzel überlebt (K1, verstärkt durch K2) | ja: nach voller TLB-Leerung gibt es kein Blatt, der erste Abruf geht durch den Zwischeneintrag | ja: „R → `S`“ liest `0` auf Stufe 3 | ja | ja: derselbe Cache | ja: Layout deterministisch, Auslöser Spekulation/Timing | ja: seltenes Ereignis | ja: QEMU hat keinen Walk-Cache | ja (K5) | 1 |
| **R2** Walker sieht `arena#361[180]` nicht (Cache/Kohärenz) | ja | ja | ja | ja | möglich | möglich | ja | **nicht erklärt**: warum gerade der Rückweg, warum nie beim ersten Kind | 2 |
| **R3** A72-Eigenheit (TLBI trifft Zwischeneintrag nicht) | ja | ja | ja | ja | ja | ja | ja | nur wenn das Nachladen schon vor dem TLBI geschah | 3 |
| **R4** Nur zeitlich (Ordnung wird in µs sichtbar) | ja | ja | ja | ja (nur wenn es noch hielt) | ja | ja | ja | nicht erklärt | 4 |

Hinweis zu R1: **F8** (Asymmetrie) ist ein *Schluss* aus dem Aufbau der Rechte
(Abschnitt 5, K5), nicht gemessen.

## 8. Zustands-/Zeitabhängigkeit (und die Anwenderaussage)

Anwenderaussage 2026-11-04 (Aussage, nicht Messung): das Auftreten vom 2026-11-04 war der
**erste** zView-Start nach dem Einschalten; es lief nichts davor (kein DOOM).

- Aus dem Foto folgt (GEMESSEN): `pc 1cb497c`, `X30 1cb4964`, Basepage `01c85000`, Text `01c86000`
  sind **mit C1 identisch**, obwohl C1 nach DOOM/DOOMNOKB stand und dieser Lauf nicht. Die
  Speicherbelegung ist also vom Vorlauf unabhängig (dasselbe schon früher aus den
  `entry 0x1c86000`-Zeilen in C1).
- Beide Abstürze traten im **ersten** zView-Start des Boots auf (C1: erster Start nach DOOM;
  Erstes Programm hier). In C1 lief der zweite zView-Start mit derselben Belegung fehlerfrei
  (zehn Modulstarts). Bei gleicher Software-Belegung (gleiche Adressen, vermutlich dieselbe
  niedrigste freie Arena-Nummer) bleibt als Unterschied nur **Mikroarchitektur-/Zeitzustand**
  (Cacheinhalt, TLB-/Walk-Cache-Belegung, Zweigvorhersage, Interrupt-Takt). **SCHLUSS:**
  das spricht für einen Hardwarezustand/Timing-Effekt (R1/R3/R4) und gegen jeden
  softwareseitig deterministischen Fehler; die frühere Frage nach einer DOOM-Verbindung
  (`TASK/DOOM01.md`) bekommt hierdurch keine Stütze.
- **Zusätzliche Zustandsabhängigkeit von R1 (SCHLUSS aus `flush_tlb`):** ein im Fenster K1 nachgeladener Eintrag überlebt nur, wenn das `sync_now` in `install_ttbr0` kein `tlbi` auslöst. Löst die Rechteneuberechnung beim Kindende zufällig eine Änderung eines gültigen Blatts oder einen L2-BBM aus, wird der Eintrag mit invalidiert (und der Fehler bleibt aus). Der Fehler braucht also zwei Zufälle zugleich: Nachladen im Fenster **und** ein „stilles“ Sync. Das passt zu einer Seltenheit von etwa 1 aus 10 bis 20 Modulstarts, ist aber **nicht gemessen**; die Breadcrumb-Zähler (9.2) nehmen „Sync mit tlbi: ja/nein“ auf.
- „Erster Start nach Boot“ kann ein Zufall sein oder ein echter Faktor (kalte Zeilen/Tabellen:
  die Arena-Tabellen jenseits der bisher benutzten sind nach dem Boot zum ersten Mal in
  Benutzung; im Zweitstart wurden sie schon einmal beschrieben). Das ist **VERMUTUNG**;
  prüfbar nur über mehrere Boots mit Zählung (Breadcrumbs: Nummer des Pexec(4)-Rückwegs
  seit dem Boot, Nummer des zView-Starts).
- Ob der Absturz im Rückweg gerade des **zweiten** Kindes liegt (C1: ja, nach zwei
  `[exec] preparing program`): im Foto nicht ablesbar; Breadcrumb „Pexec(4)-Rückwege seit
  Boot“ beantwortet es beim nächsten Vorkommen.

## 9. Messungen, die die Kandidaten trennen

Alle Ergänzungen sind Debug-Funktionen: unter dem bestehenden Feature `crash-walk`
(`kernel/Cargo.toml`) oder einem Unterfeature `crash-walk-crumbs`, das `crash-walk` voraussetzt;
ausgeschaltet muss der Kern bytegleich sein (außer `panic::Location`), geprüft wie bisher
(`TASK/PROC08.md`, „Stand Debug-Feature“, locdiff). **Nichts davon ist implementiert**;
der Entwickler setzt es um.

### 9.1 Stufe 1 — die Leiter im Absturzbericht (nur Absturzzeit, kein heißer Pfad)

Ort: `kernel/src/arch/aarch64/crash_walk.rs:94@report`, **nach** den bisherigen `AT`-Proben und
nach `crash_diag::collect` (der Software-Walk liest nur), vor `write_serial`. Die Leiter verändert
den TLB-Zustand und darf deshalb nichts Bisheriges stören. Ergebnis als Feld in `CrashDiag`
(`tos-api/src/bdos/process/crash_diag.rs:100@CrashDiag`), Ausgabe in
`crash_diag.rs:446@write_screen` (zwei Zeilen, ≤ 80 Spalten) und `write_serial`.

Schritte (je danach `AT S1E0R va`, `isb`, `mrs par_el1`; Abbruch beim ersten Erfolg, „fixed-by“ nennt den Schritt):

| # | Schritt | Zweck |
|---|---|---|
| L0 | `AT` auf `va` (existiert schon) | Ausgangswert |
| L1 | **nochmal `AT`**, keine Maßnahme | Hält der Fehler? (R4: würde jetzt Erfolg zeigen) |
| L2 | `AT` auf die Seite von `SP_EL0` (liegt in R) und auf die private Lese-/Schreibseite `0x7F000` (`PRIV_PAGE_VA`, L2-Index 0, gleiche L0/L1; die Stub-Seite taugt nicht: sie ist für EL0 nur ausführbar, `AT S1E0R` gäbe einen Rechtefehler) | Ist der ganze 2-MiB-Bereich R betroffen (Zwischeneintrag) oder nur die Seite? Private Seite gesund ⇒ L0/L1-Zeiger und die L2-Tabelle in Ordnung |
| L3 | `dsb ish`, `isb`, `AT` | Reine Barriere (Kontrolle) |
| L4 | `dc civac` auf die vier Deskriptorzeilen des Software-Walks (Kernadressen aus `walk`: L0[0], L1[0], L2[14], L3[180]), `dsb ish`, `AT` | R2: Speicher-/Cache-Sicht des Walkers |
| L5 | `tlbi vaale1is, va>>12`, `dsb ish`, `isb`, `AT` | Nur Endlevel (`RDRCZZ`): Kontrolle — Zwischeneinträge bleiben, Fehler bleibt |
| L6 | `tlbi vaae1is, va>>12`, `dsb ish`, `isb`, `AT` | Alle Ebenen für die Adresse, alle ASIDs (`RZSXRQ`, D8-7869): R1 behoben |
| L7 | `tlbi vmalle1is`, `dsb ish`, `isb`, `AT` | Alles (`RPMNQC`): R3 vs R1 |

Schirmzeilen (Beispiel; eine Ziffer „F3“ = Fehler Stufe 3, „ok“ = Erfolg):

```
ladder: L1 F3 sp F3 priv ok | dsb F3 dc F3 vaale1 F3 vaae1 ok => fixed-by vaae1
        (sp = Seite von SP_EL0, priv = 0x7F000)
```

Erwartete Ausgabe je Kandidat (SCHLUSS aus den Regeln, nicht vorab gemessen):

| Kandidat | L1 | sp | priv | dsb | dc | vaale1 | vaae1 | vmalle1 | fixed-by |
|---|---|---|---|---|---|---|---|---|---|
| R1 Zwischeneintrag (L2-Ebene R→`S`) | F3 | F3 | ok | F3 | F3 | **F3** | ok | — | `vaae1` |
| R1, Eintrag auf L0/L1-Ebene (Slot 3s Baum) | F3 | F3 | ok (Slot 3s private Seite ist ebenso gültig) | F3 | F3 | F3 | ok | — | `vaae1` — von L2 nicht unterscheidbar; kein Nachteil für die Entscheidung |
| R2 Speichersicht | F3 | F3 | ok | F3 | **ok** | — | — | — | `dc` |
| R3 TLBI trifft nicht (A72) | F3 | F3 | ok | F3 | F3 | F3 | **F3** | ok | `vmalle1` |
| R4 vorübergehend | **ok** | — | — | — | — | — | — | — | `repeat` |
| nichts hilft | F3 | F3 | ok | F3 | F3 | F3 | F3 | F3 | `none` ⇒ Grundannahme falsch, tiefer analysieren (z. B. TCR/Regime) |

`vaale1` ok wäre ein Widerspruch (ein Blatt kann keinen Übersetzungsfehler erzeugen, `RXCLRD`) und
würde die Lesart des Fehlers selbst in Frage stellen.

### 9.2 Stufe 2 — Breadcrumbs im Rückweg (heißer Pfad, nur mit `crash-walk-crumbs`)

Statische Zähler/Zeitmarken (`CNTVCT_EL0`, ein `mrs` pro Marke), gelesen im Absturzbericht:

- Zähler der Pexec(4)-Rückwege seit Boot und der Pexec(3)/(4)-Aufrufe (beantwortet „zweites
  Kind?“ und die Zustandsfrage, Abschnitt 8).
- Marken: nach `release_slot` (`exceptions.rs:1745@release_slot`), nach dem `tlbi` in
  `restore_boot_ttbr0` (`dsb`+`isb` fertig), nach `install_ttbr0`, am Trap-Ende; die Differenzen
  in µs (Zählertakt aus `CNTFRQ_EL0`). Zeigt die **Länge des Fensters K1/K2**.
- Ob das `sync_now` in `install_ttbr0` und am Trap-Ende ein `tlbi` ausgelöst hat (`TLB_DIRTY` gesetzt/verbraucht, BBM gelaufen): ja/nein je Rückweg.
- Index der von `release_private_tables` zuletzt freigegebenen Arena-Tabelle (`C`) und der von
  `arena_alloc` seitdem vergebenen: zeigt, ob `C` im Rückweg wiederverwendet wurde (K2).
- Schirmzeile: `crumbs: p4ret 2 | rel->rst 31us rst->inst 118us inst->crash 9us | C #362 reused no`.

Eingriff in den heißen Pfad (Marken) ist klein, aber **nicht** nebenwirkungsfrei; deshalb getrennt
vom Absturz-nur-Teil (Stufe 1) und nur mit eigenem Feature.

### 9.3 Stufe 3 (nur wenn 9.1 R1/R3 zeigt und der Auslöser gesucht wird)

Direkt nach `install_ttbr0` im Rückweg ein `AT S1E0R` auf die Seite des Rücksprung-PC
(`frame.elr`), Ergebnis in einen Breadcrumb. Fehler schon dort ⇒ der Zwischeneintrag steht
vor dem `eret` (K1/K2); kein Fehler dort, aber Absturz am Abruf ⇒ es entsteht zwischen
`install_ttbr0` und dem Abruf (K3). **Achtung:** der `AT` selbst lädt Walk-Cache-Einträge
(korrekte, wenn nichts veraltet ist) und kann einen spekulativen Wettlauf überdecken; deshalb
erst nach Stufe 1.

## 10. Minimale Fix-Skizzen (nicht implementiert)

- **R1 (K1+K2):**
  1. `finish_terminate`: `restore_boot_ttbr0()` **vor** `release_slot(...)` aufrufen (wie der
     Unwind-Zweig `exceptions.rs:1620@restore_boot_ttbr0`), die zweite Umschaltung (Eltern-Wurzel)
     bleibt. So wird nie eine Tabelle freigegeben oder umgeschrieben, die eine live-`TTBR0`
     erreichen kann (TRM 5.6).
  2. `restore_boot_ttbr0` (beide Zweige) und `switch_ttbr0_kernel`: `isb` zwischen `msr ttbr0_el1`
     und `tlbi vmalle1`: `dsb ishst; msr ttbr0_el1; isb; tlbi vmalle1; dsb ish; isb`
     (so wie `kernel/src/arch/aarch64/mmu.rs:183@size_asids`).
  3. Prüfen, ob QUIRK-001 (QEMU raspi4b/virt: TLBI nahe `eret`) den Abstand ändert —
     das `isb` liegt *vor* dem `tlbi`, der Abstand `tlbi`→`eret` bleibt gleich; trotzdem auf
     QEMU `raspi4b` und `virt` bootend prüfen.
  4. Abnahme: Leiter/Breadcrumbs zeigen kein „fixed-by …“ mehr in vielen zView-Läufen auf der Pi.
     QEMU kann das nicht zeigen (F7); Host-Test nicht möglich (keine TLB-Nachbildung).
- **R2:** Wenn L4 `dc` zeigt: Schreibzugriffe auf Tabellen so ausführen, dass der Walker sie sieht
  (Zeilen reinigen, `dc cvac`, vor dem `dsb`), oder die Tabellen als Normal Non-Cacheable
  abbilden und `TCR_EL1.IRGN0/ORGN0` entsprechend setzen. Vorher Quelle: ARM `D8.18` /
  A72-TRM 5.4 (Walker liest bei WriteBack-Attribut aus dem L2).
- **R3:** Arm-SDEN für Cortex-A72 beschaffen (liegt nicht unter `references/`), Workaround
  daraus. Bis dahin kein Fix.
- **R4:** Falls L1 `repeat` zeigt: Ursache in der Speicherordnung zu den Deskriptorschreibvorgängen,
  die nach `install_ttbr0` noch ausstehen; Analyse tiefer (Breadcrumbs 9.2).

## 11. Offene Punkte / bewusst nicht beantwortet

1. Welcher Zugriff lädt im Fenster nach? Nicht gefunden (K1 offen).
2. A72-Verhalten von `msr ttbr0_el1` ohne `isb` (Serialisierung, Länge des Fensters) steht nicht
   in den lokalen Quellen; nur die ARM-Regel (`IRBMTM`).
3. Ob Cortex-A72-Errata zu TLBI/Walk-Cache existieren: SDEN fehlt; nicht aus dem Gedächtnis zitiert.
4. `CPUECTLR_EL1.SMPEN` (A72-TRM 5.5, S. 5-281: vor Cache/MMU/TLB-Wartung auf 1 zu setzen) wird im
   Kern nirgends gesetzt (`/usr/bin/grep -rn -i smpen kernel boot` ohne Treffer); es hängt am
   Firmware-Stub. Lesen im Absturzbericht ist **riskant** (implementierungsdefiniertes Register,
   Zugriff von EL1 könnte per `ACTLR_EL2/3` gesperrt sein → Ausnahme im Absturzpfad); deshalb
   nicht in 9.1 aufgenommen.
5. Die Längen der Fenster in µs sind unbekannt (9.2 misst sie).

## Out-of-scope-Beobachtungen

- `init_slot_tables` (`tos-api/src/bdos/process/mmu.rs:799@init_slot_tables`) schreibt den ganzen Baum eines Slots in
  Ort ohne BBM — richtig nur solange der Slot nie live war; nicht Teil von PROC08.
- Die QUIRK-001-Kommentare in `finish_terminate` (`exceptions.rs:1761-1777@QUIRK`) nennen ein
  `kprintln!` zwischen `restore_boot_ttbr0` und `install_ttbr0`, das im Code nicht mehr steht
  (`exceptions.rs:1779-1782@restore_boot_ttbr0`); nur Kommentar-Drift.
