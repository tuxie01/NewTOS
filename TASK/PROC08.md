# PROC08 — Rueckkehr aus `Pexec(4)` findet die Code-Seite des Elternprozesses nicht

Status: **WARTET AUF WIEDERAUFTRETEN** 2026-11-04 (Anwender: Kartenkern mit `crash-walk`, serielle Ausgabe aus; bei einem Absturz Foto des Schirmberichts, siehe "Stand Schirmbericht"; angelegt 2026-10-31) · Prioritaet: **P1** · Typ: Kernfehler — Ursachenforschung zuerst, Fix erst nach Vorlage der Ursache — Fund C1-2 aus [PITEST01](PITEST01.md), Lauf C1

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
  `kernel/src/arch/aarch64/exceptions.rs:2160@handle_pexec`,
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
  `kernel/src/arch/aarch64/crash_walk.rs:94@report` Zeilen `[crashwalk] …`
  auf die serielle Leitung (nicht auf den Bildschirm), vor dem Bericht und
  vor dem Tastenwarten. Der Bericht selbst und das Verhalten bleiben gleich.
  Für die Fehleradresse und, bei einem EL0-Absturz, für die Seite des PC
  (wenn sie eine andere ist): `AT S1E0R` + `PAR_EL1` (was die MMU jetzt
  antwortet; ARM ARM DDI 0487 M.d C5.4.5, D8.2.13, D24.2.141 — Zitate im
  Modulkopf), `TTBR0_EL1` gegen die erwartete Wurzel, ein geprüfter
  Software-Walk durch die Tabellen des Slots
  (`tos-api/src/bdos/process/mmu.rs:2081@walk`: jeder Tabellenzeiger wird
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
  `kernel/src/arch/aarch64/crash_walk.rs:94@report` liest zuerst
  `TTBR0_EL1` und die `AT S1E0R`-Proben, dann sammelt
  `tos-api/src/bdos/process/crash_diag.rs:152@collect` Walk(s) und
  Modellsicht in einen `CrashDiag`. Daraus entstehen die seriellen Zeilen
  (`tos-api/src/bdos/process/crash_diag.rs:302@write_serial`, Text
  unverändert; Host-Test gegen die Zeilen des Laufs vom 2026-11-02) und der
  Schirmblock (`tos-api/src/bdos/process/crash_diag.rs:446@write_screen`),
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

[Gemeinsame Regeln](README.md) gelten.
