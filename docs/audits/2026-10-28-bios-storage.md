# Audit BIOS-Speicher: `Getbpb`, `Rwabs`, `Mediach`, `Drvmap` — 2026-10-28

## 1. Kopf

| | |
|---|---|
| Scope | BIOS trap 13: `Rwabs` (0x04), `Getbpb` (0x07), `Mediach` (0x09), `Drvmap` (0x0A) — erste Triage der DUMPBIOS-Funde fuer diese vier Funktionen und Probe des Audit-Ablaufs (Anwender 2026-10-28) |
| Datum | 2026-10-28 |
| NewTOS | `c3db3747d6822b74d4c8d65e34d99af8b9bf5755`, sauber (einzig die Laufmappe ist neu, nicht committet) |
| EmuTOS | `1a7484aba1bfba23e2c0899e243b6d8177fe41df` |
| ROM | `tools/oracle/etos512de-1a7484ab.img`, sha256 `323c87bb…d2750c` |
| Hatari | 2.6.1 (Aug 15 2025), `tools/oracle/hatari.cfg` sha256 `c8f4998a…f8ae46` |
| Kanarienvogel | ORACLE §6, heute vor dem Audit: `canary tt: OK`, `canary falcontc: OK` |
| Karte | `tools/mkcard.sh`, MBR + FAT16, 64 MiB, image_sha256 `50d3f697…da4e34`, `optional_missing none` |
| Boards | NewTOS: QEMU `raspi4b` (release, `serial=on`); EmuTOS: Hatari Falcon030 und TT, IDE-Master |
| Beleg | `measurements/measurement-2026-10-28-bios-storage-audit/RUNINFO.TXT` |

**Wie gemessen wurde — und was das fuer die Urteile heisst.** `bios-r1.tsv`
enthaelt von den vier Funktionen nur `Drvmap` (Fall 74). `Rwabs`, `Getbpb`
und `Mediach` weist der DUMPBIOS-Lader ab (`tosdump/core/cases.c:3022@call_policy`,
`tosdump/core/cases.c:3109@call_refusals`), und das Orakel hat kein
BIOS-Blockgeraet (`tosdump/cases/COVERAGE-BIOS.md`, Status „later"). tosdump
darf der Auditor nicht aendern. Deshalb lief eine **Ad-hoc-Sonde**
(`measurements/measurement-2026-10-28-bios-storage-audit/method/bstprobe.c`): eine C-Quelle, fuer m68k und AArch64
gebaut, ueber mintlibs `trap_13_*` auf beiden Seiten, auf derselben
FAT16-Karte (Hatari: IDE-Master; QEMU: SD). Beide Seiten deterministisch
(Wiederholung identisch), TT = Falcon. **Das ist Messung, aber kein
tosdiff-Vergleich** — nach den Regeln der Rolle sind alle Funde daraus
PLAUSIBLE, keiner CONFIRMED, und keine Funktion wird dadurch EQUAL.

## 2. Findings

### BIOS-STOR-001 — S2 — PLAUSIBLE (beidseitig per Sonde gemessen, kein tosdiff)

**Der forcechange-Handshake fehlt: Schreiben von logischem Sektor 0.**

- Lokalisierung: LOCALIZED: `rwabs` (`tos-api/src/bios/mod.rs:148@rwabs`);
  beteiligt `mediach` (`tos-api/src/bios/mod.rs:269@mediach`) und
  `bpb::getbpb` (`tos-api/src/bios/bpb.rs:284@getbpb`). Beobachtet ueber
  `Rwabs`- und `Mediach`-Rueckgaben. `cargo xtask deps lrwabs --down` nennt nur
  `protect_wlwwwl` (ext) — die Lokalisierungsbedingung ist damit leer erfuellt,
  siehe BIOS-STOR-009.
- Faelle: W01–W05, X12–X14 (`measurements/measurement-2026-10-28-bios-storage-audit/runs/DIFF-emutos-falcon-vs-newtos.txt`).
- Eingabepruefung: EmuTOS-Seite per `--trace bios` (Aufrufe 42–46, 76–78 in
  `measurements/measurement-2026-10-28-bios-storage-audit/runs/EMUTOS-falcon/trace-bios-probe.txt`); NewTOS-Seite nicht moeglich
  (kein BIOS-Argument-Tracer, Abschnitt 11).
- Gemessen, EmuTOS: `Rwabs(1,buf,1,0,2,0)` liefert **E_CHNG (-14)**; danach
  `Mediach(2)` = 2, `Rwabs` lesend = E_CHNG — **auch mit `RW_NOMEDIACH`** (W04,
  X14) — bis `Getbpb(2)`; danach 0. Mit `RW_NOMEDIACH` (Flag 3) schreibt
  EmuTOS (X12 = 0) und scharfschaltet denselben Zustand (X13 = 2, X14 = -14).
- Gemessen, NewTOS: alle diese Aufrufe 0; `Mediach` immer 0.
- EmuTOS: `references/emutos/bios/blkdev.c:450-453` (forcechange pruefen und
  setzen, vor jedem `RW_NOMEDIACH`-Test), `references/emutos/bios/blkdev.c:474`
  (`blkdev_mediach` sieht forcechange → E_CHNG, Abbruch vor dem Transfer),
  `references/emutos/bios/blkdev.c:802` (`Mediach` → `MEDIACHANGE`),
  `references/emutos/bios/blkdev.c:590` (`Getbpb` loescht).
- Spec: `docs/api/bios/04-rwabs.md` beschreibt forcechange, verschweigt aber
  dass der Schreibaufruf selbst E_CHNG liefert und ohne `RW_NOMEDIACH` nicht
  schreibt (BIOS-STOR-008); `09-mediach.md` fuehrt das Fehlen als
  „Step-2 candidate".
- Fehlerbild: ein Plattenwerkzeug, das den Bootsektor neu schreibt (Label,
  Seriennummer, neue Geometrie), sieht einen anderen Rueckgabewert; vor allem
  meldet NewTOS danach keinen Medienwechsel, also loggt GEMDOS das Laufwerk
  nicht neu ein (EmuTOS: `ckdrv`/`getbcb` fragen `Mediach`). Nach einem
  geometrie-aendernden Bootsektor schriebe die BDOS mit alter Geometrie —
  **geschlossen, nicht gemessen**; wer das misst, misst S1.
- Retest: `deps lrwabs --up` (545 Funktionen, u. a. `getbcb`, `usrio`,
  `flush`, `ckdrv`), `deps mediach --up` (547), `deps getbpb --up` (449);
  Faelle W01–W11, X12–X16; kuenftige DUMPBIOS-Plattenrunde (Abschnitt 5).
- Route: Developer.

### BIOS-STOR-002 — S3 — PLAUSIBLE (Sonde)

**`RW_NOTRANSLATE` adressiert per Laufwerksbuchstabe statt per physischer Einheit.**

- LOCALIZED: `rwabs`, `tos-api/src/bios/mod.rs:173@resolve` und
  `tos-api/src/bios/mod.rs:189@RW_NOTRANSLATE`.
- Faelle P01–P04. EmuTOS: Einheit 2 = ACSI 0 → EUNDEV, Einheit 18 = IDE-Master
  → MBR, letzter Gerätesektor (P03) ESECNF (`>=`-Grenze). NewTOS: „Einheit" 2 =
  C: → MBR, 18 → EUNDEV.
- EmuTOS: `references/emutos/bios/disk.h:30` (`GET_UNITNUM`: 0–1 Floppy, 2–9
  ACSI, 10–17 SCSI, 18–25 IDE, 26– SD/MMC), `references/emutos/bios/blkdev.c:484`.
- Fehlerbild: ein Partitionierer, der `Rwabs(8,…,unit 2)` fuer ACSI 0 ruft,
  liest auf NewTOS die SD-Karte; einer, der die IDE-/SD-Einheiten absucht,
  findet nichts; physischer Zugriff setzt ausserdem ein eingehaengtes
  Laufwerk voraus.
- Spec: `04-rwabs.md` (Pi-Bindung) sagt noch „`RW_NOTRANSLATE` not honoured" —
  veraltet (BIOS-STOR-008). Welche Einheitennummer die Pi-SD tragen soll
  (EmuTOS-Logik: SD/MMC-Bus = 26), ist eine Spec-Entscheidung.
- Retest: P01–P05; `deps lrwabs --up`.
- Route: Researcher (Einheitenzuordnung festlegen), dann Developer.

### BIOS-STOR-003 — S3 — PLAUSIBLE (Sonde)

**`Rwabs` mit Anzahl 0 ueberspringt die Bereichspruefung.**

- LOCALIZED: `rwabs`, `tos-api/src/bios/mod.rs:164@numb` (Rueckkehr vor der
  Pruefung).
- Fall R10: `Rwabs(0,buf,0,-1,2,PSIZE+10)` — EmuTOS ESECNF (-8), NewTOS 0.
  R09 (Anzahl 0 im Bereich) gleich.
- EmuTOS: `references/emutos/bios/blkdev.c:461-463` (Pruefung vor der
  Transferschleife, auch fuer `lcount == 0`).
- Spec: `04-rwabs.md` Edge case 7 („`numb == 0` … returns `E_OK`") ist
  unvollstaendig (BIOS-STOR-008).
- Retest: R08–R10. Route: Developer.

### BIOS-STOR-004 — S3 — PLAUSIBLE (Sonde)

**`Getbpb` nimmt logische Sektoren groesser als das Laufzeit-`max_sect_siz` an.**

- LOCALIZED: `bpb::getbpb`/`parse`, `tos-api/src/bios/bpb.rs:64@MAX_LOGSEC_SIZE`,
  `tos-api/src/bios/bpb.rs:135@MAX_LOGSEC_SIZE`.
- Fall X01 (Bootsektor bps = 1024, spc 4): EmuTOS `Getbpb(2)` = NULL, NewTOS
  BPB `1024,4,4096,16,128,129,273,32676,1`.
- EmuTOS: `max_sect_siz = MAX_LOGSEC_SIZE` gilt nur *voruebergehend* waehrend
  `pun_info_setup` (`references/emutos/bios/blkdev.c:122`); danach
  `max(groesster Boot-recsiz, CONF_LOGSEC_SIZE = 512)`
  (`references/emutos/bios/blkdev.c:159`, `references/emutos/include/config.h:1606`),
  geprueft in `references/emutos/bios/blkdev.c:642`. Der NewTOS-Kommentar
  zitiert Zeile 122 und uebernimmt den Uebergangswert als Dauerwert.
- Folgefrage (nicht gemessen, PLAUSIBLE, im Retest mitpruefen): NewTOS'
  `rwabs` rechnet logische in physische Sektoren nicht um (EmuTOS
  `references/emutos/bios/blkdev.c:456-458`); ein angenommenes BPB mit
  recsiz 1024 adressiert dann 512-Byte-Sektoren. Klaerender Fall: Karte mit
  einer AHDI-/BGM-Partition (recsiz ≥ 1024) beim Booten, `Getbpb` + `Rwabs`
  logisch lesen, auf beiden Seiten.
- Retest: X01; `deps getbpb --up`. Route: Developer.

### BIOS-STOR-005 — S3 — PLAUSIBLE (Sonde)

**`Getbpb` bei `tot16 = tot32 = 0`: NewTOS ersetzt durch die Partitionslaenge.**

- LOCALIZED: `parse`, `tos-api/src/bios/bpb.rs:186@part_sectors`.
- Fall X06: EmuTOS BPB `…,289,0,0` (numcl 0, kein `B_16` — Laufwerk faktisch
  unbrauchbar), NewTOS `…,289,32672,1`.
- EmuTOS: `references/emutos/bios/blkdev.c:730-733` (Kommentar davor: bei
  Festplatten `sec2` als gueltig annehmen, dann numcl 0).
- Ist der Ersatz Absicht, fehlt der `docs/COMPATIBILITY.md`-Eintrag; sonst
  EmuTOS folgen. Retest: X06. Route: Researcher (Absicht klaeren), dann ggf.
  Developer.

### BIOS-STOR-006 — S3 — PLAUSIBLE (Sonde)

**Logisches `Rwabs` nach gescheitertem `Getbpb` liefert Daten statt ESECNF.**

- LOCALIZED: `rwabs`, `tos-api/src/bios/mod.rs:173@resolve` (keine BPB-Pruefung).
- Fall X08 (nach X07, spf = 0, beide `Getbpb` = NULL): EmuTOS ESECNF, NewTOS 0
  mit Daten.
- EmuTOS: `references/emutos/bios/blkdev.c:443-444` (recsiz 0 → ESECNF),
  `references/emutos/bios/blkdev.c:601`, `references/emutos/bios/blkdev.c:740`.
- Spec fuehrt es als „GAP" (`04-rwabs.md`, Pi-Bindung). Retest: X07–X11.
  Route: Developer.

### BIOS-STOR-007 — S4 — PLAUSIBLE (Sonde)

**`Rwabs(…, NULL, n, …, A:/B:)` (FastCopy-Merkmal): EmuTOS 0, NewTOS EUNDEV.**

- Fall F03; die Kontrolle ohne Floppy-Laufwerke (`EMUTOS-falcon-nofloppy`)
  liefert ebenfalls 0 — der Unterschied bleibt also auch bei gleicher
  Hardwarelage. EmuTOS: `references/emutos/bios/blkdev.c:425` (vor jeder
  Laufwerkspruefung). NewTOS: `tos-api/src/bios/mod.rs:173@resolve`.
- Nur fuer Floppy-Kopierer sichtbar (Nicht-Ziel); braucht einen
  `docs/COMPATIBILITY.md`-Eintrag oder Angleichung. Route: Researcher.

### BIOS-STOR-008 — S4 — Spec (gemessen widerlegt bzw. veraltet)

Route: Researcher.

- `docs/api/bios/04-rwabs.md`: Verification criterion „Sector-0 write forces
  media change" und Edge case 2 nennen nicht, dass der Schreibaufruf ohne
  `RW_NOMEDIACH` E_CHNG liefert und nicht schreibt, und dass forcechange auch
  `RW_NOMEDIACH`-Zugriffe sperrt (W01, W04, X14). Edge case 7: Anzahl 0 wird
  trotzdem bereichsgeprueft (R10). Pi-Bindung: Tabelle „`RW_NOTRANSLATE` …
  not honoured" und die Zeilenzitate auf `bios/mod.rs` (66-116, 75-95, 86-90)
  sind veraltet (Code: `tos-api/src/bios/mod.rs:189@RW_NOTRANSLATE`).
- `docs/api/bios/07-getbpb.md`, Pi-Bindung „Gaps": „No sector-size /
  cluster-size / FAT-count validation" und „Zero-FAT→2 quirk not applied"
  sind ueberholt — X02–X05, X07 gleich auf beiden Seiten. Es fehlt dagegen der
  Unterschied Boot- vs. Laufzeit-`max_sect_siz` (BIOS-STOR-004) und
  `tot16 = tot32 = 0` (BIOS-STOR-005).

### BIOS-STOR-009 — S4 — Graph/Mapping

Route: Researcher.

- `docs/analysis/deps/mapping.tsv` Zeilen 274, 275, 279, 280: `blkdev_drvmap`,
  `blkdev_getbpb`, `blkdev_mediach`, `blkdev_rwabs` stehen als „no
  counterpart", obwohl das Verhalten genau dort liegt und in `drvmap`,
  `bpb::getbpb`, `mediach`, `rwabs` nachgebaut ist (n:m-Zuordnung fehlt).
- `edges.tsv` hat keine Kante vom Trap-Eintritt zum Handler hinter dem
  Vektor: `lrwabs`/`getbpb`/`mediach` → `protect_*` endet; nur
  `blkdev_init → blkdev_rwabs` (callback, Installation). `deps lrwabs --down`
  ist deshalb leer bis auf `protect_wlwwwl`, und die Bottom-up-Bedingung fuer
  Lokalisierung ist bei allen Vektor-Funktionen leer erfuellt.
- Die Datei couplings.md in docs/analysis/deps/, auf die die Rollendatei
  verweist, existiert nicht.

### BIOS-STOR-010 — S3 — PLAUSIBLE (nicht gemessen) — NOT LOCALIZED

**Eingehaengte `hdv_rw`/`hdv_bpb`/`hdv_mediach`-Vektoren wirken nicht.**

EmuTOS ruft `Rwabs`/`Getbpb`/`Mediach` ueber die Vektoren `0x476`/`0x472`/
`0x47E` (`docs/api/bios/_state.md`); Programme haengen sich dort ein
(TeraDesk `force_mediach`, Plattencaches). `TASK/MEM12.md` vermerkt, dass
NewTOS sie „ohnehin nie" ruft; ein `docs/COMPATIBILITY.md`-Eintrag fehlt.
Klaerender Fall: Sonde setzt im Super-Zustand einen `hdv_mediach`-Haken, der 2
liefert, ruft `Mediach(2)` und stellt zurueck — auf beiden Seiten. Route:
Researcher (Vertrag festlegen).

## 3. Offene Tickets im Scope

| Ticket | Messung | Ergebnis |
|---|---|---|
| `TASK/FS12.md` (Laufwerksgrenze 16 statt 26) | G05 `Getbpb(16)`, M05 `Mediach(16)`, R14 `Rwabs(…,16,…)` | EQUAL (beide: NULL / EUNDEV / EUNDEV, Q: fehlt beidseitig). Die Grenze selbst ist mit einer Partition nicht beobachtbar; Ticket bleibt offen, braucht > 16 Laufwerke |

## 4. Coverage-Matrix

| EmuTOS | NewTOS | Schicht | Status | Faelle |
|---|---|---|---|---|
| `lrwabs` → `blkdev_rwabs` | `tos-api/src/bios/mod.rs:148@rwabs` | bios | **DIFF** (Sonde; BIOS-STOR-001/002/003/006/007), tosdiff UNMEASURED (Lader verweigert, Orakel ohne Blockgeraet) | R01–R20, P01–P05, W01–W11, X08–X14, F03 |
| `getbpb` → `blkdev_getbpb` | `tos-api/src/bios/bpb.rs:284@getbpb` | bios | **DIFF** (Sonde; BIOS-STOR-004/005), tosdiff UNMEASURED | G01–G07, W06, X01–X07, X10, X15, F02, F05 |
| `mediach` → `blkdev_mediach` | `tos-api/src/bios/mod.rs:269@mediach` | bios | **DIFF** (Sonde; BIOS-STOR-001), tosdiff UNMEASURED | M01–M07, W02/W05/W07/W10, X13/X16, F01/F04/F06 |
| `drvmap` → `blkdev_drvmap` | `tos-api/src/bios/mod.rs:312@drvmap` | bios | **EQUAL** (tosdiff `bios-r1.tsv` Fall 74 auf `c3db3747`, Klasse NONZERO — schwach, siehe Abschnitt 11) | 74; Sonde D01 |

Gleich auf beiden Seiten (Sonde): BPB-Inhalt und Zeigerstabilitaet,
Bereichsgrenzen logisch (letzter Sektor ja, Ende/Ueberlappung/negativ ESECNF),
`lfirst`-Umschaltung, `first` vorzeichenlos (0x8000, -2), Flags 2/4/0x80,
abwesende Laufwerke (EUNDEV bzw. NULL), alle `Getbpb`-Abweisungen ausser X01
und die Sonderfaelle X02/X05, `Mediach` ohne forcechange, Schreiben des
letzten Sektors, Schreiben mit `RW_NOMEDIACH`.

Bug-Dichte: CONFIRMED 0 ÷ 4 gemessene Funktionen = 0 — weil nichts ausser
`Drvmap` unter tosdiff lief. Sondengemessen: 7 Verhaltensabweichungen
(001–007) in 3 von 4 Funktionen.

## 5. Neue Faelle

Kein tosdump-Fall angelegt: jede Zeile wuerde vom Lader abgewiesen
(`tosdump/core/cases.c:3109@call_refusals`), und das Orakel hat keinen
Plattenaufbau. Vorschlag fuer eine DUMPBIOS-Plattenrunde (`bios-r2.tsv`,
Orakel mit IDE-Master = dieselbe Karte, Programm nicht auf ihr), Zeilen in
der Form von `bios-r1.tsv`; erst auf dem Orakel, dann NewTOS:

```
13:7	w:2	PTR_RAM	0:18	Getbpb(2) C: -- BPB capture
13:7	w:15	EXACT	-	Getbpb(15) absent -> 0
13:9	w:2	EXACT	-	Mediach(2) -> 0
13:9	w:15	EXACT	-	Mediach(15) -> EUNDEV
13:4	w:0 b:0 w:1 w:0 w:2 l:0	EXACT	1:512	Rwabs read C: sector 0
13:4	w:0 b:0 w:1 w:-1 w:2 l:<PSIZE-1>	EXACT	1:512	Rwabs last sector
13:4	w:0 b:0 w:1 w:-1 w:2 l:<PSIZE>	EXACT	-	Rwabs past end -> ESECNF
13:4	w:0 b:0 w:0 w:-1 w:2 l:<PSIZE+10>	EXACT	-	Rwabs count 0 past end -> ESECNF (BIOS-STOR-003)
13:4	w:0 b:0 w:1 w:0 w:15 l:0	EXACT	-	Rwabs absent drive -> EUNDEV
13:4	w:8 b:0 w:1 w:0 w:<unit> l:0	EXACT	1:512	Rwabs physical MBR (BIOS-STOR-002; unit per spec decision)
```

Schreibfaelle (BIOS-STOR-001, -004–006) brauchen eine Folge mit
Wiederherstellung (Vorspann/Nachspann), also einen Dumper-Mechanismus, den
`bios-r1` nicht hat; Vorlage sind die Gruppen W und X der Sonde.
Ausserdem fuer `Drvmap` eine relationale Klasse statt NONZERO (Abschnitt 11).

## 6. Gemessen vs. offen

Gemessen (Sonde, beidseitig, deterministisch, TT = Falcon): 87 Aufrufe der
vier Funktionen, inkl. Schreib- und Bootsektor-Faelle; Abbilder danach
verglichen (EmuTOS-Klon bytegleich zur Ausgangskarte, NewTOS nur die zwei
Logdateien) und `fsck_msdos -n` sauber. Gemessen (tosdiff): `Drvmap` Fall 74
auf dem auditierten Commit.

Offen: tosdiff-Urteil fuer `Rwabs`/`Getbpb`/`Mediach` (Werkzeuglage);
NewTOS-seitige Eingabepruefung (kein Tracer); logische Sektoren > 512 beim
Booten (BIOS-STOR-004 Folgefrage); > 16 Laufwerke (FS12); Vektor-Haken
(BIOS-STOR-010); `etv_critic`-Pfad bei echten I/O-Fehlern (QEMU liefert
keine); `RW_NORETRIES` ist bei `RWABS_RETRIES = 1` unbeobachtbar.

## 7. Real-Pi4-Liste

- `Mediach` bei gezogener/neu gesteckter Karte (EMMC2, kein Card-Detect in
  QEMU): Rueckgabe und spaeterer Zugriff.
- `Rwabs`-Fehlercodes bei echten Lese-/Schreibfehlern der Karte (HW03-Pfad)
  und das Verhalten mit laufendem Desktop (EmuTOS: `etv_critic`-Alert).

## 8. Akzeptable Abweichungen

- **`Drvmap` Bits A:/B: und `Mediach(0/1)`**: Geraeteparameter. Beweis durch
  Messung: EmuTOS ohne Floppy-Laufwerke (`--drive-a off --drive-b off`) liefert
  `Mediach(0)`/`Mediach(1)` = EUNDEV wie NewTOS und setzt die Bits nicht
  (`measurements/measurement-2026-10-28-bios-storage-audit/runs/DIFF-controls.txt`). Bit D: ist Hatari's GEMDOS-Programmlaufwerk
  (Messaufbau), nicht EmuTOS.
- Nicht akzeptiert: F03 (BIOS-STOR-007) — bleibt auch ohne Floppy verschieden.

## 9. Out-of-scope observations

- DUMPBIOS auf `c3db3747`: 42 (tt) / 46 (falcontc) von 147 Faellen
  verschieden, vorher 47/51 — nicht verfolgt.
- `cargo xtask build` legt `kernel8.img` im Repository-Wurzelverzeichnis ab
  (CLAUDE.md: wird dort nicht gehalten; gitignored).

## 10. Cleanup

- `AUDIT-TEMP`: keine temporaere Verdrahtung gesetzt; Suche ausserhalb
  `docs/` und `.claude/` leer.
- Kein Produktcode beruehrt; `touch kernel/src/lib.rs tos-api/src/lib.rs` (nur
  mtime, Inhalt unveraendert, `git status` sauber).
- Neu: diese Datei und `measurements/measurement-2026-10-28-bios-storage-audit/`.
- Scratch in der Sitzungs-Scratchpad; alle Kartenklone nach dem Vergleich
  geloescht. Ein Versehen: eine 160-Byte-Hilfsdatei landete in `/tmp/x` und
  wurde sofort geloescht.
- Fremde Aenderungen: keine gesehen.

## 11. Probe des Audit-Ablaufs — was erschwert hat oder unklar war

1. **Drei von vier Funktionen koennen in einem Audit nie tosdiff-gleich
   werden.** Der Dumper existiert, verweigert sie aber per Whitelist; der
   Auditor darf tosdump nicht aendern, das Orakel hat keine Platte. Die
   Rollenregel „kein Dumper → UNMEASURED" passt nicht genau („Dumper da,
   Funktion gesperrt"). Vorschlag: DUMPBIOS-Plattenrunde (Abschnitt 5) als
   Ticket, vor dem naechsten Speicher-Audit.
2. **Plattenfall-Regeln widersprechen dem Werkzeug.** Rolle: GEMDOS-Emulation
   aus, Programm nicht auf dem Testabbild. `tools/hatari-oracle.sh:66@PRG`
   haengt immer einen GEMDOS-Ordner als C: ein und startet von dort. Ich habe
   ein eigenes Skript gebaut: IDE-Master = C:, GEMDOS-Ordner = D: nur fuers
   Programm (BIOS-Aufrufe faengt Hatari nicht ab; die Abbilder zeigen, dass
   EmuTOS selbst gelesen hat), aber Hatari setzt Bit D: in `_drvbits`. Ob ein
   `--ide-master` oder `--drive-a off` auf der Kommandozeile eine
   „Orakel-Aenderung" nach ORACLE §7 ist (COVERAGE-BIOS.md sagt ja), ist
   nirgends entschieden. Vorschlag: ein Plattenprofil in ORACLE §2.
3. **„Programm nie auf dem Testabbild" ist auf NewTOS `raspi4b` unmoeglich**
   (eine SD; `qemu-virt` hat kein C:). Braucht eine Zwei-Partitionen-Karte
   aus `tools/mkcard.sh` oder eine Ausnahme in der Rolle.
4. **Eingabepruefung fuer BIOS nicht erfuellbar.** NewTOS hat keinen Tracer
   mit BIOS-Argumenten: `svc-seq-trace` schreibt nur Funktion und Ergebnis
   (`kernel/src/dbg_qedtrace.rs:397@Bios`), `gemdos-trace` nur GEMDOS. Hatari
   `--trace bios` zeigt `Rwabs` ohne das 6. Argument. Auch der
   DUMPBIOS-Lauf vom 2026-10-27 hat keine Eingabepruefung. Vorschlag: BIOS/
   XBIOS-Argumente im `svc-seq-trace`-Satz.
5. **`$CLAUDE_JOB_DIR` ist in dieser Sitzung leer.** Der Kanarien-Befehl in
   ORACLE §6 wird damit zu `/tmp/canary` (gemeinsames `/tmp`, CLAUDE.md
   verbietet es). Ich habe die Sitzungs-Scratchpad genommen; dort lag schon
   ein Ordner canary einer frueheren Sitzung, `canary.sh` brach (richtig) mit
   „scratch dir exists" ab. Vorschlag: Befehl mit Fallback und eindeutigem
   Namen.
6. **Pre-flight „run a case on NewTOS through `cargo xtask`"**: es gibt keinen
   xtask-Befehl fuer tosdump-Faelle; die Laeufe nutzen Skripte in
   `measurements/*/method/`. Unklar, was gemeint ist.
7. **Abhaengigkeitsgraph traegt die Lokalisierung hier nicht** (BIOS-STOR-009):
   Vektoraufrufe sind keine Kanten, `--down` der Trap-Funktionen ist leer, und
   couplings.md fehlt.
8. **Urteilsklassen**: ein beidseitig gemessener, deterministischer
   Sonden-Unterschied ist nach der Rolle nur PLAUSIBLE, genau wie eine
   Quelltext-Vermutung. Eine Zwischenstufe (z. B. „MEASURED-PROBE") wuerde
   die Triage ehrlicher machen.
9. **`Drvmap` Fall 74, Klasse NONZERO**: besteht fuer jede nichtleere
   Laufwerkskarte (NewTOS 0x4, Orakel 0x7). Vorschlag an den Researcher: eine
   relationale Regel (Bit 2 gesetzt, Bits 16–31 null), am Orakel per
   `tosdiff --selfcheck` geprueft.
