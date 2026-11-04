# PROC08 — Rueckkehr aus `Pexec(4)` findet die Code-Seite des Elternprozesses nicht

Status: **ZURUECKGESTELLT — to be tested** 2026-11-04 (Anwender: Fix `01eb7709` auf der Karte, 60 zView-Starts ohne Absturz; nicht schliessen, im Alltag und bei den Testern weiter beobachten; Kartenkern mit `crash-walk` samt Leiter) · Prioritaet: **P1** · Typ: Kernfehler — Ursachenforschung zuerst, Fix erst nach Vorlage der Ursache — Fund C1-2 aus [PITEST01](PITEST01.md), Lauf C1

Vor-Start-Pruefung: Produkt-Ticket, Kartenprogramm betroffen (zView laedt
seine Bildformat-Module so); jedes Programm, das per `Pexec` 3+4 Module oder
Kinder startet, kann es treffen.

## Sachverhalt (gemessen, echte Pi 4)

- Lauf: `measurements/measurement-2026-10-31-pitest01-c1/RUNINFO.TXT`
  (Release-Kern `e0fe680b...` aus `6796787c`, `serial=on`), serielle
  Mitschrift
  `measurements/measurement-2026-10-31-pitest01-c1/serial/screenlog-c1.txt` Zeilen 6660–6691.
- Erster zView-Start nach DOOM und DOOMNOKB: zView startet zwei Kinder
  (`[exec] preparing program (slot 3)` zweimal), dann
  `[crash] slot 2 Bus Error esr=0x82000007 pc=0x1cb497c far=Some(30099836)`,
  "private memory, execute (translation fault, level 3)",
  basepage `01c85000`, text `01c86000`, "Crash at text+0002e97c",
  X30 = `0x1cb4964`.
- Die Adresse liegt in zViews eigenem Code. `ZVIEW.NAP` auf der Karte ist
  bytegleich mit `sdk/zview/build/out/ZVIEW.NAP` (sha256 `efb1aebbd303ba57...`,
  nicht gestrippt); Lastbasis = basepage (ELF-Eintrag `0x1000` -> `0x1c86000`).
  pc `0x1cb497c` = ELF `0x2f97c` in `ldg_load`:

  ```
  2f944  svc #1          ; Pexec(3, ...)  Modul laden
  2f960  bl  ldg_cpush
  2f964  mov w0,#4 ; mov w8,#0x4b ; x1=0, x2=basepage, x3=0
  2f978  svc #1          ; Pexec(4, 0, basepage, 0)  Modul starten
  2f97c  bl  ldg_cpush   ; <- Befehlsabruf scheitert
  ```

  Der erste Befehl nach der Rueckkehr aus `Pexec(4)` in den Elternprozess
  hatte keine Uebersetzung auf Stufe 3.
- Zweiter zView-Start im selben Boot: zehn Modulstarts, kein Absturz,
  normales Ende. Der Fehler haengt also von Zustand oder Zeitpunkt ab.
- Seitenschutz-Fehlerbild "Bus Error" mit Tastendruck-Abbruch: Release-Kern
  ohne `crash-nowait`.

## Bezug

- Kernstellen, an denen der Elternprozess nach dem Ende des Kindes wieder
  eingeschaltet wird bzw. das Kind startet (Ausgangspunkt, nicht Befund):
  `kernel/src/arch/aarch64/exceptions.rs:1781@install_ttbr0`,
  `kernel/src/arch/aarch64/exceptions.rs:2172@handle_pexec`,
  `kernel/src/arch/aarch64/exceptions.rs:2416@install_ttbr0`,
  `tos-api/src/bdos/process/mmu.rs:1688@install_ttbr0`.
- [PROC01](PROC01.md) (Pexec-Modi), [MEM12](MEM12.md) (Seite-0-Schutz,
  erledigt), QUIRK-001 in `docs/MEMORY.md`.
- [DOOM01](DOOM01.md): Verbindung nicht belegt.

## Arbeit

1. Researcher: Ursache belegen — welcher Zustand fuehrt dazu, dass beim
   Ruecksprung aus `Pexec(4)` die Code-Seite des Elternprozesses nicht
   abgebildet ist (falsche Wurzel aktiv, Eintrag fehlt/entfernt, Reihenfolge
   von Tabellenpflege und Umschalten, Prozesswechsel waehrend des Kindes, ...).
   Reproduktion in QEMU versuchen (zView-Modulstarts, auch nach DOOM-Lauf);
   Bericht an den Lead vor jedem Fix.
2. Developer erst nach Entscheid des Anwenders; Abnahme: Wiederholungslauf
   mit vielen `Pexec` 3+4 (Sonde oder zView) in QEMU und auf der echten Pi.

## Stand 2026-11-02 (Schritt 1 erledigt, Ursache nicht belegt)

Bericht `docs/analysis/proc08-root-cause-2026-11-02.md`, QEMU-Lauf
`measurements/measurement-2026-11-02-proc08/RUNINFO.TXT`.

- Gemessen (C1-Log): Die Code-Seite war vor dem zweiten `Pexec(4)`
  abgebildet (vorige Befehle in derselben Seite), danach nicht; kein
  Prozesswechsel dazwischen; keine Arena- oder Snapshot-Meldung.
- Widerlegt: Arena erschoepft, Snapshot zu kurz, Prozesswechsel im
  Rueckweg; falsche Wurzel nur schwach widerlegt.
- Kandidaten, keiner belegt: A — `memprot` entzieht dem Eltern-Slot die
  Seite (Modellfehler waere fuer `memprot=verify` unsichtbar); B — Tabellen
  richtig, Cortex-A72 sieht sie anders (TLB/Walk-Cache/Reihenfolge; Kind-Slot
  wird freigegeben, solange seine Wurzel in `TTBR0_EL1` steht).
- QEMU raspi4b: 3 Versuche, 810 Modulstarts, 0 Fehler (QEMU modelliert
  A72-TLB und Walk-Cache nicht).
- Vorschlag des Researchers: Pi-Lauf mit `serial=on memprot=verify`; trennt
  das nicht, Debug-Feature (Standard aus) fuer eine Absturzdiagnose
  (Software-Walk von `far`, `TTBR0_EL1` gegen die Slot-Wurzel, L2-Eintrag).
  Entscheidung beim Anwender.

## Stand Debug-Feature (2026-11-02, Developer; Anwenderentscheid 2026-11-02)

Zuerst eine Absturzdiagnose als Debug-Feature, dann ein Pi-Lauf durch den
Anwender. Lauf und Nachweise:
`measurements/measurement-2026-11-02-proc08-diag/RUNINFO.TXT`.

- **Feature:** `crash-walk` (Kern, `kernel/Cargo.toml`; reicht
  `tos-api/crash-walk` durch). Standard aus. Ohne Feature sind Code und
  Daten des Kerns bytegleich mit dem Stand ohne Änderung (rpi4 und
  qemu-virt, release und dev, locdiff „0 differing bytes“).
- **Was es tut:** Nach der `[crash]`-Zeile eines Programmabsturzes
  (`kernel/src/arch/aarch64/crash.rs:96@crash_walk`) schreibt
  `kernel/src/arch/aarch64/crash_walk.rs:138@report` Zeilen `[crashwalk] …`
  auf die serielle Leitung (nicht auf den Bildschirm), vor dem Bericht und
  vor dem Tastenwarten. Der Bericht selbst und das Verhalten bleiben gleich.
  Für die Fehleradresse und, bei einem EL0-Absturz, für die Seite des PC
  (wenn sie eine andere ist): `AT S1E0R` + `PAR_EL1` (was die MMU jetzt
  antwortet; ARM ARM DDI 0487 M.d C5.4.5, D8.2.13, D24.2.141 — Zitate im
  Modulkopf), `TTBR0_EL1` gegen die erwartete Wurzel, ein geprüfter
  Software-Walk durch die Tabellen des Slots
  (`tos-api/src/bdos/process/mmu.rs:2097@walk`: jeder Tabellenzeiger wird
  vor dem Lesen gegen die eigenen Tabellen geprüft) und die Sicht des
  Rechtemodells auf die Seite
  (`tos-api/src/bdos/process/memprot.rs:2103@page_view`, gerechnet wie
  `memprot=verify`, aus Blocklisten, Blockeinträgen und Prozesstabelle).
  Keine Allokation.
- **Bauen (Pi-Kern):** `cargo xtask build --board rpi4 --release --features
  crash-walk` → `target/aarch64-unknown-none-softfloat/release/kernel8.img`
  (sha256 im RUNINFO). Mit `crash-nowait` kombinierbar; für den Pi-Lauf
  **ohne** `crash-nowait` (der Bericht wartet auf eine Taste wie bisher).
- **Kommandozeile (`cmdline.txt`) für den Pi-Lauf:** `serial=on memprot=verify`.
  Ablauf wie im Absturzlauf C1 (DOOM, DOOMNOKB, dann zView, mehrere
  zView-Starts in einem Boot); die serielle Mitschrift ist das Ergebnis.

### Was jede Zeile für A und B bedeutet

Bei PROC08 ist ein Befehlsabbruch zu erwarten (`esr=0x82000007`, `far` =
`pc`): dann gibt es nur die `far`-Zeilen.

| Zeile | Inhalt | Lesart |
|---|---|---|
| `[crashwalk] slot S src=el0 esr=… pc=… far=…` | Kopf | `S` = Slot des abgestürzten Prozesses (bei PROC08 zView). |
| `[crashwalk] ttbr0=… asid=… root=slot K; slot S expects … records … -> same` | aktive Wurzel gegen erwartete (`l0_pa_for`) und eingetragene (`Process::l0_pa`) | `DIFFERENT` oder `root` ≠ `slot S`: falsche Wurzel aktiv — weder A noch B, die im Bericht nur schwach widerlegte Erklärung. `same`: Wurzel richtig. |
| `[crashwalk] far va=… walk slot S root …: L0[i]=…(s2.L0) … L3[j]=…(arena#N used) -> …` | Software-Walk: Deskriptor und Tabellenart je Stufe | `-> mapped L3`: im Speicher ist die Seite abgebildet → spricht für **B**. `-> INVALID at L3`: in den Tabellen nicht abgebildet → **A** (welcher Teil: Modellzeilen). `arena#N FREE` oder `BAD TABLE`: der Slot zeigt auf eine freigegebene oder fremde Tabelle — Fehler der Tabellenpflege (A-Seite, nicht Hardware). Steht `root` auf einem anderen Slot, folgt eine zweite Walk-Zeile durch dessen Tabellen. |
| `[crashwalk] far l2 template=… l3 also named by slots=0x… template=yes/no` | wer die L3-Tabelle des 2-MiB-Bereichs sonst noch benennt | Private L3: `slots=0x0 template=no` erwartet. Ein weiterer Slot benennt dieselbe Tabelle: doppelt vergebene Arena-Tabelle (Tabellenpflege, A-Seite). `template=yes`: der Slot läuft im Bereich über die gemeinsame Tabelle (kein Eigentum dort). |
| `[crashwalk] far AT S1E0R par=…: ok pa=… / fault fst=0x07 (translation L3)` | was die MMU **jetzt** für die Adresse antwortet (TLB oder Walk, IMPLEMENTATION DEFINED, `RNPZWX`) | Walk `mapped` + `ok`: Speicher und MMU stimmen jetzt überein, zur Fehlerzeit sah die Hardware anderes → **B** (vorübergehend). Walk `mapped` + `fault`: die MMU antwortet noch mit dem alten Zustand, obwohl im Speicher ein gültiger Eintrag steht → **B** (veralteter TLB-/Walk-Cache-Eintrag). Walk `INVALID` + `fault`: Hardware und Tabellen einig → nicht B. |
| `[crashwalk] far model slot S: pool=… block … mode=… own=… further=… owners=… inherit=…` | Block der Seite, Modus, Eigentümer-Cookies, die Slots dahinter | Bei PROC08 muss die Code-Seite in zViews TPA-Block liegen und `owners` zViews Slot enthalten. Fehlt er, obwohl `own` zViews Basepage ist: Eigentümerauflösung falsch (A im Modell). |
| `[crashwalk] far model slot S: live=… slot_live=… slot_kva=… space=… group=… temp=… holds=… expect=… shared=… private needed=… has=… shortfalls=… pending=…` | was das Modell dem Slot für die Seite gibt | `holds=yes`, `expect` gültig, Walk `INVALID`: Tabelle weicht vom Modell ab → A im fortschreibenden Teil (`memprot=verify` meldet dann auch `VERIFY MISMATCH`). `holds=no`/`expect=0x0` für die eigene Code-Seite: A im Modell (verify sieht das nicht). `pending=yes`: zur Fehlerzeit lagen Änderungen noch nicht in den Tabellen. `shortfalls>0`: Arena erschöpft (widerlegt, aber hier sichtbar). |
| `[crashwalk] far summary: model=… table=… at=… table==model …` | die drei Antworten nebeneinander | `valid/valid/ok` oder `valid/valid/fault` → **B**. `valid/invalid/…` → **A**, Tabellenpflege. `invalid/invalid/fault` → **A**, Modell. `table==model NO` = L3-Deskriptor ≠ Modell. |

Gemessen in QEMU raspi4b (`serial=on` und `serial=on memprot=verify`):
nicht abgebildete Fälle (Sprung auf und Lesen von frisch freigegebenem
Pool-Speicher → `INVALID at L3`, `fst=0x07`; Lesen in GiB 2 → `INVALID at
L2`, `fst=0x06`) und abgebildete (Schreiben/Ausführen auf Seite 0 und die
PC-Seite → `mapped L3`, `AT ok`), jeweils mit der Ausnahme-Kennung der
`[crash]`-Zeile übereinstimmend; 0 Abweichungen im verify-Lauf. Einen Fall
„Tabelle gültig, Hardware Fehler“ (B) kann QEMU nicht erzeugen; das ist der
Zweck des Pi-Laufs.

## Stand Schirmbericht (2026-11-04, Developer; Anwenderentscheid 2026-11-04)

Der Absturz ist selten. Statt ihn mit serieller Mitschrift zu jagen, läuft
auf der Karte ein `crash-walk`-Kern als Alltagskern mit **leerer**
`cmdline.txt` (serielle Leitung aus); tritt der Absturz beim normalen Testen
auf, fotografiert der Anwender den Schirm. Deshalb steht die Diagnose jetzt
auch im Absturzbericht auf dem Schirm, unabhängig von `serial=on`. Lauf:
`measurements/measurement-2026-11-04-proc08-screen/RUNINFO.TXT`.

- **Einmal erheben, zweimal ausgeben:**
  `kernel/src/arch/aarch64/crash_walk.rs:138@report` liest zuerst
  `TTBR0_EL1` und die `AT S1E0R`-Proben, dann sammelt
  `tos-api/src/bdos/process/crash_diag.rs:175@collect` Walk(s) und
  Modellsicht in einen `CrashDiag`. Daraus entstehen die seriellen Zeilen
  (`tos-api/src/bdos/process/crash_diag.rs:574@write_serial`, Text
  unverändert; Host-Test gegen die Zeilen des Laufs vom 2026-11-02) und der
  Schirmblock (`tos-api/src/bdos/process/crash_diag.rs:722@write_screen`),
  den `kernel/src/arch/aarch64/crash.rs:100@screen` nach dem Berichtstext
  und vor „Press any key" ausgibt. Kein zweiter Walk.
- **Bauen:** wie oben, `cargo xtask build --board rpi4 --release --features
  crash-walk`, ohne `crash-nowait`. Ohne Feature ist der Kern weiterhin
  bytegleich (Nachweis im Lauf).
- **Form:** Zahlen hex ohne `0x` (Adressen, Deskriptoren, Masken, `par`);
  Slot, Tabellenindex `[i]` und Stufe `L<n>` dezimal. Eine Leerzeile, eine
  Kopfzeile, je untersuchter Adresse sieben Zeilen (zwei mehr, wenn
  `TTBR0_EL1` auf einen anderen Slot zeigt; eine Modellzeile statt drei bei
  `memprot=off`). Bei PROC08 (Befehlsabbruch, `far` = `pc`) also 9 Zeilen,
  bei einem Datenabbruch aus einer anderen Seite 16. Zeilen bei
  realistischen Werten ≤ 80 Spalten; bei 1920×1080 (240×67 Zeichen) passt
  der ganze Bericht samt Block und Aufforderung auf den Schirm.

Beispiel (QEMU raspi4b, 1920×1080, leere Kommandozeile, Sprung auf eine
frisch freigegebene Pool-Seite, wie der PROC08-Befehlsabbruch;
`measurements/measurement-2026-11-04-proc08-screen/screendumps/s1-j-1080.png`):

```
crash-walk slot 1 el0: ttbr0 2e9000 asid 0 = slot 1, exp 2e9000 rec 2e9000 same
far 1d8b000 s1: L0[0]=2ea003 s1.L0  L1[0]=2eb003 s1.L1  L2[14]=11c6003 s1.L2g0
  L3[395]=0 arena#454 used -> INVALID at L3  also 0 tpl no
  AT S1E0R par=80f: FAULT translation L3 fst=07 s=0 ptw=0
  model: pool yes, no block
  model: holds no expect 0 shared 0 pend no
  model: priv need yes has yes short 0 live 3 group 2 temp 0 space 1d79000
  => model=invalid table=invalid at=fault table==model yes
```

Gelesen: Wurzel richtig; Seite in Tabelle und MMU nicht abgebildet, das
Modell gibt dem Slot dort nichts (freier Pool) — hier richtig, denn die
Seite war freigegeben. Beim PROC08-Absturz in zViews eigenem Code würde
dieselbe Zeile `model: … block <zViews TPA> … owners <zViews Bit>` und
`holds yes` zeigen; dann entscheiden Walk, `AT` und Summary zwischen A und B.

### Was jede Schirmzeile für A und B bedeutet

Jede Zeile trägt dieselbe Antwort wie ihre serielle Entsprechung (Tabelle
oben); hier nur die Lesart in Kurzform.

| Zeile | Inhalt | Lesart |
|---|---|---|
| `crash-walk slot S el0: ttbr0 R asid N = slot K, exp E rec C same` | aktive Wurzel (`TTBR0_EL1`) gegen erwartete (`exp`, `l0_pa_for`) und eingetragene (`rec`, `Process::l0_pa`) | `DIFFERENT` oder `slot K` ≠ `S` (`empty`, `unknown`): falsche Wurzel aktiv — weder A noch B. `same`: Wurzel richtig. |
| `far V sS: L0[i]=… L1[i]=… L2[i]=… <Art>` und `  L3[j]=D <Art> -> …  also M tpl …` | Software-Walk durch die Tabellen von Slot S, Deskriptor und Tabellenart je Stufe; `also` = Maske der anderen Slots, die dieselbe L3 nennen, `tpl` = die gemeinsame Vorlage nennt sie | `-> mapped L3`: Tabelle bildet ab → spricht für **B**. `-> INVALID at L3`: nicht abgebildet → **A**. `arena#N FREE` oder `BAD TABLE`: Slot zeigt auf freigegebene/fremde Tabelle — Tabellenpflege (A-Seite). `also` ≠ 0 bei privater L3: doppelt vergebene Arena-Tabelle. Zeigt `TTBR0_EL1` auf Slot K, folgt ein zweites Zeilenpaar `far V sK:`. |
| `  AT S1E0R par=P: ok pa=…` / `FAULT translation L3 fst=07 s=0 ptw=0` | was die MMU **jetzt** antwortet | Walk `mapped` + `ok` → **B** (vorübergehend). Walk `mapped` + `FAULT` → **B** (veralteter TLB-/Walk-Cache-Eintrag). Walk `INVALID` + `FAULT` → nicht B. |
| `  model: pool … block A-E mode M own O owners X inh I` | Block der Seite, Modus, Eigentümer-Cookie (`+n further` bei weiteren), Slots dahinter (Maske), geerbte Slots | Code-Seite muss in zViews TPA-Block liegen und `owners` zViews Slot-Bit enthalten; fehlt es, obwohl `own` zViews Basepage ist: Eigentümerauflösung falsch (A im Modell). `no block`: freier Pool-Speicher. |
| `  model: holds … expect D shared D' pend …` | Rechte des Slots, Deskriptor laut Modell (Eigentümer / Nichteigentümer), ausstehende Änderungen | `holds yes`, `expect` gültig, Walk `INVALID` → A im fortschreibenden Teil. `holds no`/`expect 0` für die eigene Code-Seite → A im Modell. `pend yes`: zur Fehlerzeit lagen Änderungen noch nicht in den Tabellen. |
| `  model: priv need … has … short N live X group G temp T space B` | private L3 nötig/vorhanden, Arena-Engpässe, lebende Slots, Speicherraum des Slots (Pexec(4)) | `short` > 0: Arena erschöpft (widerlegt, hier sichtbar). `need yes has no`: private Tabelle fehlt. |
| `  => model=… table=… at=… table==model …` | die drei Antworten nebeneinander | `valid/valid/ok` oder `valid/valid/fault` → **B**. `valid/invalid/…` → **A**, Tabellenpflege. `invalid/invalid/fault` → **A**, Modell. `table==model NO`: L3-Deskriptor ≠ Modell. |
| `  ladder: L1 r sp r priv r far r dsb r dc r vaale1 r vaae1 r vmalle1 r => …` (seit „Stand Leiter“; nur unter der ersten Adresse) | die Leiter: nach jedem Schritt `AT S1E0R` auf die Fehleradresse, Abbruch beim ersten Erfolg; `r` = `ok`, `F<n>` (Übersetzungsfehler Stufe n) oder `x<fst>` (anderer Fehler, FST hex). `sp` = Seite von `SP_EL0` (die `USP`-Zeile des Berichts), `priv` = private Seite `7f000`, `far` = die Fehleradresse danach | Ende siehe Lesetabelle unten. `sp F3` mit `priv ok`: die ganze 2-MiB-Region der Fehleradresse ist betroffen, L0/L1 und die L2-Tabelle sind gesund — nur wenn `USP` in derselben 2-MiB-Region liegt wie `far` (Adressen vergleichen). `ladder: not run, AT ok`: der erste `AT` gelang, nichts lief. |

Lesetabelle der Leiter (aus `docs/analysis/proc08-candidate-b-2026-11-04.md`
Abschnitt 9.1, Tabelle „Erwartete Ausgabe je Kandidat“; R1–R4 wie dort
Abschnitt 7). Gilt für Walk `-> mapped L3` (Summary `valid/valid/fault`), also
den Fall B:

| Ende | Kandidat | Lesart |
|---|---|---|
| `=> repeat` | **R4** nur zeitlich | der zweite `AT` ohne jede Maßnahme gelang: der Zustand hielt nur Mikrosekunden |
| `=> fixed-by sp+priv` | — | nicht in der Erwartungstabelle: die `AT`s auf die Nachbarseiten haben die Übersetzung der Fehleradresse geändert. Kein Kandidat zugeordnet; an den Lead |
| `=> fixed-by dsb` | — | nicht in der Erwartungstabelle (Barriere allein, Kontrolle). Kein Kandidat zugeordnet; an den Lead |
| `=> fixed-by dc` | **R2** Speichersicht | der Walker las den Deskriptor erst nach `dc civac` der Deskriptorzeilen richtig |
| `=> fixed-by vaale1` | — Widerspruch | nur Endstufe invalidiert; ein Blatt kann keinen Übersetzungsfehler erzeugen (`RXCLRD`, D8-7852) — stellt die Lesart des Fehlers selbst in Frage |
| `=> fixed-by vaae1` | **R1** veralteter Zwischeneintrag | alle Ebenen für die Adresse (`RZSXRQ`, D8-7869), nachdem die Endstufe allein nicht half; Eintrag auf L0/L1- oder L2-Ebene nicht unterscheidbar |
| `=> fixed-by vmalle1` | **R3** A72-Eigenheit | erst das Invalidieren von allem half, `vaae1` nicht |
| `=> none` | — | nichts half: Grundannahme falsch, tiefer analysieren (z. B. TCR/Regime) |

Bei Walk `-> INVALID` (Tabelle bildet nicht ab, nicht B) ist `=> none` die
erwartete Antwort: Hardware und Tabelle sind sich einig.

## Stand 2026-11-04 — wieder aufgetreten, Kandidat B

Auf der Karte (Kern `6407f17c…` mit `crash-walk`, serielle Ausgabe aus) beim
Alltagstest erneut abgestuerzt; Foto und Abschrift:
`measurements/measurement-2026-11-04-proc08-recur/RUNINFO.TXT`. Gleiche Stelle wie C1
(pc `1cb497c` = zView text+`2e97c`, erster Befehl nach `Pexec(4)` in
`ldg_load`, X30 `1cb4964`, basepage `01c85000`). Schirmblock:
`=> model=valid table=valid at=fault table==model yes`. Wurzel richtig
(`ttbr0 2f0000 = slot 2, exp/rec gleich`), L3[180] bildet die Seite ab
(`20000001cb4743`, EL0-ausfuehrbar), Modell stimmt (`holds yes`, `pend no`),
aber `AT S1E0R` faellt auf Stufe 3. Nach der Lesart oben: **Kandidat B**, die
MMU arbeitet mit veraltetem Uebersetzungszustand; Kandidat A ist fuer diesen
Fall widerlegt. Naechster Schritt: Mechanismus von B belegen (welcher
Zustand ist veraltet, an welcher Stelle fehlt die Pflege), Bericht vor jedem Fix.

## Stand 2026-11-04 (Mechanismus B)

Bericht `docs/analysis/proc08-candidate-b-2026-11-04.md` (Researcher, nur Lesen; kein Fix).
Anwenderaussage 2026-11-04: der Absturz trat im **ersten** zView-Start nach dem Einschalten
auf, ohne DOOM davor; DOOM ist keine Vorbedingung.

- Gelesen: ein Übersetzungsfehler wird nie zwischengespeichert (ARM ARM `RXCLRD`); der Walker
  hat auf Stufe 3 einen Deskriptor `0` gelesen, also muss er über einen **veralteten
  Zwischeneintrag** (der A72 hat Walk-Caches, TRM 5.6, ASID-zugeordnet) in eine andere
  L3 geführt worden sein oder den Speicherinhalt nicht gesehen haben. Der `AT` fällt nur
  Mikrosekunden nach dem Abruf (nicht „lange danach“): ein zeitlich begrenzter Effekt ist
  nicht ausgeschlossen.
- **Rang 1 (Schluss, Auslöser nicht gefunden):** ein Eintrag der Wurzel des Kindes (Slot 3)
  überlebt den Rückweg. Fenster: `finish_terminate` gibt die privaten Tabellen des Kindes frei,
  **während dessen Wurzel in `TTBR0_EL1` steht** (`kernel/src/arch/aarch64/exceptions.rs:1745@release_slot`,
  der Unwind-Zweig tut es umgekehrt, `:1620@restore_boot_ttbr0`), und `restore_boot_ttbr0`
  schreibt `TTBR0_EL1` und führt `tlbi vmalle1` **ohne `isb` dazwischen** aus
  (`kernel/src/arch/aarch64/user_entry.rs:124-132@tlbi`; ARM ARM `IRBMTM` D8-7700, `IBLDZX`
  D8-7875; der Kern macht es an anderer Stelle richtig, `kernel/src/arch/aarch64/mmu.rs:183@size_asids`).
  Danach folgt vor dem `eret` kein unbedingtes `tlbi` mehr. Ein dort nachgeladener Zwischeneintrag
  „Region → gemeinsame L3“ erklärt Stufe-3-Fehler auf der ersten Seite des Elternprozesses und
  warum nur der Rückweg auffällt. Der Zugriff, der im Fenster nachlädt, ist **nicht gefunden**.
- Rang 2: Walker sieht `arena#361[180]` nicht (Cache-Sicht); Rang 3: A72-Eigenheit
  (SDEN liegt nicht unter `references/`); Rang 4: nur vorübergehend.
- **Nächster Schritt (Entscheidung beim Anwender):** Leiter im Absturzbericht (Debug-Feature,
  Standard aus): zweiter `AT`, `AT` auf Nachbarseiten, dann `dc civac`, `tlbi vaale1is`,
  `tlbi vaae1is`, `tlbi vmalle1is`, nach jedem Schritt `AT`; eine Schirmzeile
  „fixed-by …“. Erwartete Ausgaben je Kandidat in Abschnitt 9.1 des Berichts. Optional
  Breadcrumbs im Rückweg (Abschnitt 9.2).
- Fix-Skizze für Rang 1 (nicht umgesetzt): in `finish_terminate` `restore_boot_ttbr0` vor
  `release_slot`; in `restore_boot_ttbr0` und `switch_ttbr0_kernel` ein `isb` zwischen `msr`
  und `tlbi`. Abnahme nur auf der Pi (QEMU ohne Walk-Cache).

## Stand Fix (2026-11-04, Developer; Anwenderentscheid 2026-11-04)

Umgesetzt ist nur der Fix für Rang 1 (Abschnitt 10 des Berichts), Produktcode,
nicht an ein Feature gebunden. Die Leiter (Abschnitt 9.1) ist auf Anwenderwunsch
**nicht** Teil dieses Schritts (eigene Aufgabe später). Lauf:
`measurements/measurement-2026-11-04-proc08-fix/RUNINFO.TXT`.

Zwei Regeln, an jeder Stelle geprüft (Quellen lokal gelesen, ARM ARM DDI 0487 M.d,
Cortex-A72-TRM 100095_0003_06):

- **R-a (Tabellen erst frei, wenn keine Wurzel sie mehr erreicht):** solange
  `TTBR0_EL1` eine Wurzel nennt, darf der Walker deren Zwischeneinträge jederzeit,
  auch spekulativ, zwischenspeichern (`RSQBCS`, `IRCGCH`, D8-7852); mit ASID aus
  tragen alle Wurzeln ASID 0, und TRM §5.6 (S. 5-282) beschreibt genau den Fehler
  „Tabelle freigegeben und neu belegt, über einen gespeicherten Zwischeneintrag
  erreicht“ und verlangt das Invalidieren, bevor Tabellenspeicher freigegeben wird.
- **R-b (`isb` zwischen `msr ttbr0_el1` und `tlbi`):** ein TTBR-Schreiben wirkt
  für die Übersetzung erst nach einem Context-Synchronization-Event (`IRBMTM`,
  D8-7700); ein TLBI ist erst nach `dsb` + CSE fertig (`IBLDZX`, D8-7875; der A72
  hat kein FEAT_ETS2, `ID_AA64MMFR1_EL1` ist RES0, TRM Tabelle 10-10, S. 10-364),
  bis dahin dürfen spekulative Zugriffe alte Übersetzungsdaten benutzen (`ICPZSM`,
  D8-7875). Ohne `isb` kann ein Walk nach dem `tlbi` noch von der alten Wurzel
  ausgehen und einen Eintrag hinterlassen, den nichts mehr entfernt.

Geänderte Stellen:

| Stelle | Änderung | Regel |
|---|---|---|
| `kernel/src/arch/aarch64/exceptions.rs:1766@restore_boot_ttbr0` (`finish_terminate`) | `restore_boot_ttbr0` **vor** `release_slot` (`kernel/src/arch/aarch64/exceptions.rs:1771@release_slot`), wie der Unwind-Zweig (`kernel/src/arch/aarch64/exceptions.rs:1622@restore_boot_ttbr0`); danach nur noch `install_ttbr0` der Eltern-Wurzel (`kernel/src/arch/aarch64/exceptions.rs:1794@install_ttbr0`) | R-a |
| `kernel/src/arch/aarch64/user_entry.rs:144@ttbr0` (`restore_boot_ttbr0`, ASID aus) | `dsb ishst; msr ttbr0_el1; isb; tlbi vmalle1; dsb ish; isb` | R-b |
| `tos-api/src/bdos/process/mmu.rs:1903@ttbr0` (`switch_ttbr0_kernel`, ASID aus) | dieselbe Folge | R-b |
| `kernel/src/arch/aarch64/mmu.rs:861@ttbr0` (`install_stage2_ttbr1`, Boot) | erst `msr ttbr0_el1, leer; isb`, dann `msr ttbr1_el1` und `tlbi vmalle1; dsb ish; isb` wie bisher | R-b (sonst könnte die Boot-Identität mit ihren UXN-1-GiB-Blöcken nach dem `tlbi` neu geladen werden; bis zum ersten `install_ttbr0` des Starters invalidiert nichts mehr) |

Geprüft, ohne Änderung:

| Stelle | Befund |
|---|---|
| `tos-api/src/bdos/process/mmu.rs:1565@rollover_flush` | hat schon `msr; isb; tlbi vmalle1is; dsb ish; isb` (R-b erfüllt) |
| `tos-api/src/bdos/process/mmu.rs:1688@install_ttbr0`, `tos-api/src/bdos/process/mmu.rs:1528@install_tagged`, `kernel/src/arch/aarch64/user_entry.rs:114@ttbr0` (`restore_boot_ttbr0`, ASID an) | kein `tlbi` nach dem `msr`: R-b greift nicht. `install_ttbr0` verlässt die leere Wurzel, die keinen gültigen Eintrag hat (Fehlereinträge werden nie gespeichert, `RXCLRD`, D8-7852) |
| `kernel/src/arch/aarch64/boot.rs:393@ttbr0_el1` | MMU noch aus (`SCTLR_EL1.M` erst danach): Einträge werden nur bei eingeschalteter Übersetzung gespeichert (`RSQBCS`, `IRCGCH`); `isb` vor dem Einschalten vorhanden |
| `exceptions.rs` `handle_pexec`, `apply_switch`, `restore_kernel_park` | `restore_boot_ttbr0` (jetzt mit `isb`), kein Tabellenfreigeben unter der alten Wurzel |
| Pexec-Ladefehler und `release_unstarted_of`/`_at` (`free_slot` in `pexec.rs`), `accs.rs` | geben Slots frei, deren Wurzel seit `init_slot_tables` nie in `TTBR0_EL1` stand (Zustand `Loaded` bzw. Kernpfad mit leerer Wurzel); die Einträge des Vormieters fielen beim letzten Wurzelwechsel (R-b) |
| `drop_private` / `make_private` (`replace_bbm`) | Break-before-make mit `tlbi` vor dem Freigeben (R-a erfüllt) |

QUIRK-001: Das `kprintln!` zwischen `restore_boot_ttbr0` und `install_ttbr0`, das
der alte Kommentar in `finish_terminate` „load-bearing“ nannte, steht seit
`c31f4637` (2026-06-19) nicht mehr im Code; der Pexec/Pterm-Rückweg lief seitdem in
QEMU raspi4b ohne es (u. a. 810 Modulstarts im Lauf
`measurements/measurement-2026-11-02-proc08/RUNINFO.TXT`). Die Ursache aus
QUIRK-001 (gleiche VA in Eltern und Kind) gibt es seit ARCH03 nicht mehr
(ADR-028). Der Kommentar ist berichtigt.

Ergebnisse siehe RUNINFO. Abnahme bleibt die Pi (QEMU hat keinen Walk-Cache und
kann den Fehler weder zeigen noch seine Abwesenheit belegen).

## Stand Leiter (2026-11-04, Developer)

Stufe 1 aus Abschnitt 9.1 von `docs/analysis/proc08-candidate-b-2026-11-04.md`
(keine Breadcrumbs, keine Stufe 3), im bestehenden Debug-Feature `crash-walk`.
Der Fix `01eb7709` ist unberührt. Lauf:
`measurements/measurement-2026-11-04-proc08-ladder/RUNINFO.TXT`.

- **Ablauf:** `kernel/src/arch/aarch64/crash_walk.rs:138@report` liest wie
  bisher zuerst `TTBR0_EL1` und die `AT`-Proben, dann `collect` (nur Lesen),
  dann die Leiter (`kernel/src/arch/aarch64/crash_walk.rs:152@run_ladder`),
  dann die seriellen Zeilen. Die Leiter läuft auf der ersten untersuchten
  Adresse (`far`; ohne Fehleradresse `pc`) und nur, wenn deren erster `AT`
  fehlschlug; nach jedem Schritt `AT S1E0R` auf die Adresse (`at; isb; mrs
  par_el1`), Abbruch beim ersten Erfolg. Die Schritte wirken aufeinander auf
  (nichts wird zurückgenommen).
- **Aufteilung:** Reihenfolge, Abbruch, Ende und beide Zeilen in tos-api
  (`tos-api/src/bdos/process/crash_diag.rs:343@run_ladder`,
  `tos-api/src/bdos/process/crash_diag.rs:222@Rung`,
  `tos-api/src/bdos/process/crash_diag.rs:325@end`,
  `tos-api/src/bdos/process/crash_diag.rs:405@screen_ladder`,
  `tos-api/src/bdos/process/crash_diag.rs:432@serial_ladder`), die Befehle im
  Kern über die Schnittstelle `tos-api/src/bdos/process/crash_diag.rs:269@LadderHw`
  (`kernel/src/arch/aarch64/crash_walk.rs:171@maintain`). Die Kernadressen der
  Deskriptoren, die der Software-Walk las:
  `tos-api/src/bdos/process/mmu.rs:2089@entry_kva`.

Schritte und Barrieren (Quellen lokal gelesen: ARM ARM DDI 0487 M.d,
`references/arm-arm/DDI0487M_d_a-profile_architecture_reference_manual.pdf`;
Cortex-A72-TRM 100095_0003_06; Volltext auch im Modulkopf von `crash_walk.rs`):

| # | Name | Folge vor dem `AT` | Regel |
|---|---|---|---|
| L1 | `L1` | — (zweiter `AT`) | — |
| L2 | `sp+priv` | `AT S1E0R` auf `SP_EL0` (die `USP` des Berichts) und auf `7f000` (`PRIV_PAGE_VA`) | — |
| L3 | `dsb` | `dsb ish; isb` | Kontrolle: der Barrierenteil von L5–L7 allein |
| L4 | `dc` | `dc civac, <Zeile>` je gelesenem Deskriptor (bis 8: Walk des Slots und ggf. der Wurzel in `TTBR0_EL1`), dann `dsb ish` | C5.3.21 (C5-1128, VA ohne Ausrichtungsvorgabe); Cache-Wartung erst nach `DSB` (Zugriffsart Lesen+Schreiben) fertig, D7.5.9.15 (D7-7667); `DSB` schließt alle vorigen Cache-Wartungen ab, kein späterer Befehl wirkt vorher, B2.6.9 (B2-302/303) |
| L5 | `vaale1` | `tlbi vaale1is, Xt; dsb ish; isb` | VAAL = nur Endstufe, alle ASIDs: `RJXKNN` (D8-7869) |
| L6 | `vaae1` | `tlbi vaae1is, Xt; dsb ish; isb` | VAA = alle Stufen, alle ASIDs: `RZSXRQ` (D8-7869) |
| L7 | `vmalle1` | `tlbi vmalle1is; dsb ish; isb` | VMALL = alles der EL1&0-Stufe 1: `RPMNQC` (D8-7867) |

Für alle TLBI: `IS` = alle TLBs der Inner-Shareable-Domäne (`IDMCXY`, D8-7863);
`Xt` = `(va >> 12)` auf 44 Bit (`IZVNJG`, D8-7863/7864: [43:0] = VA[55:12],
[63:48] RES0 ohne ASID-Argument, [47:44] RES0 ohne FEAT_TTL — optional ab
Armv8.3, A2-128; der A72 implementiert Armv8-A, TRM 1.2.1, S. 1-15); fertig erst
nach `DSB` und einem Context-Synchronization-Event, weil der A72 kein FEAT_ETS2
hat (`IBLDZX`, D8-7875; `ID_AA64MMFR1_EL1` RES0, TRM Tabelle 10-10, S. 10-364);
`ISB` ist ein solches Ereignis (Glossar „Context Synchronization event“, S. 17126).

**Berichtigung zur Analyse 9.1:** dort steht für `vaale1` die Regel `RDRCZZ`; das
ist VAL (mit ASID, D8-7868/7869). `tlbi vaale1is` ist VAAL, Regel `RJXKNN`
(D8-7869). An der Lesart ändert das nichts (beide: nur Endstufe). Die Analyse ist
ein datierter Stand und bleibt unverändert.

- **Zeilen:** seriell nach der `summary`-Zeile der Adresse
  `[crashwalk] far ladder va=0x…: usp=0x… priv=0x7f000 lines=0x…,…; L1 par=0x… sp+priv sp=0x… priv=0x… far par=0x… dsb par=0x… … => fixed-by …`
  (jedes `PAR_EL1` voll); auf dem Schirm nach der Zeile `  => model=…`
  `  ladder: L1 F3 sp F3 priv ok far F3 dsb F3 dc F3 vaale1 F3 vaae1 ok => fixed-by vaae1`.
  Enden: `=> fixed-by <Schritt>`, `=> repeat` (L1 gelang), `=> none`; gelang
  schon der erste `AT`: `ladder: not run, AT ok`. Lesart: Legende und
  Lesetabelle unter „Stand Schirmbericht“. Breite: 86 Spalten für die erwartete
  PROC08-Antwort, höchstens 107 (Host-Test); bei 1920×1080 sind es 240, bei
  640×480 (80 Spalten) wird die Zeile umbrochen — dort passte der Block schon
  vorher nur knapp (Lauf „Stand Schirmbericht“, s7/s8).
- **QEMU raspi4b** (1920×1080, leere Kommandozeile und `serial=on`): Fall `j`
  (Sprung auf eine freigegebene Pool-Seite, Befehlsabbruch wie PROC08) zeigt
  `ladder: L1 F3 sp ok priv ok far F3 dsb F3 dc F3 vaale1 F3 vaae1 F3 vmalle1 F3 => none`
  — erwartet: dort bildet die Tabelle die Seite **nicht** ab (Walk `INVALID`),
  Hardware und Tabelle sind sich einig, kein Schritt kann helfen. Fall `x`
  (Ausführen auf `$700`, `AT` gelingt) zeigt `ladder: not run, AT ok`. Den
  PROC08-Fall (Tabelle gültig, `AT` Fehler) kann QEMU nicht erzeugen: es
  modelliert weder TLB noch Walk-Cache des A72 (F7); die Zweige `fixed-by …`
  und `repeat` sind nur host-getestet
  (`tos-api/tests/proc08_crash_walk.rs:711@the_ladder_ends_name_the_candidates`,
  `tos-api/tests/proc08_crash_walk.rs:765@the_ladder_lines_on_screen_and_serial`).
- **Ohne Feature** ist der Kern bytegleich mit HEAD (rpi4 und qemu-virt,
  release und dev, locdiff 0 Bytes). Abnahme der Aussage bleibt die Pi: der
  nächste PROC08-Absturz mit diesem Kern zeigt die Leiterzeile auf dem Foto.

## Stand 2026-11-04 — Pi mit Fix (Anwender)

Kern `d8ab0c47…` (Fix `01eb7709` + `crash-walk`, ohne Leiter;
`measurements/measurement-2026-11-04-proc08-fix-card/RUNINFO.TXT`). Vor dem Fix
(Kern `6407f17c…`) trat der Absturz beim Anwender mehrmals hintereinander auf.
Mit dem Fix: 20 zView-Starts, Neustart, 20 Starts, Accessories geoeffnet,
20 Starts — kein Absturz; die Pi startet mit der geaenderten Stage-2-Stelle
normal. Anwender: nicht schliessen, als "to be tested" zurueckstellen; das
Release 0.7 ([REL07](REL07.md)) traegt `crash-walk` mit Leiter zu den Testern.

[Gemeinsame Regeln](README.md) gelten.
