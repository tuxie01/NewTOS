# DESK43 — Der Desktop muss im User-Space laufen, wie jede andere Anwendung

Status: IMPLEMENTED (2026-11-11) | Priority: P1 | Commit: e074301ab | Missing: Pi counter-check

Migrated (owner batch 15, 2026-11-11, to IMPLEMENTED), evidence: commit e074301ab (feat(desk): drift guards before the EL0 move (DESK43 U5a)), VERIFIED: diff of e074301ab (feat(desk): drift guards before the EL0 move (DESK43 U5a)) touches kernel/src/lib.rs, kernel/Cargo.toml, names the id by subject, 46 files, legacy line: ERLEDIGT 2026-10-16 (U1–U10, U10 `f6f90202`, Tester-Lauf `87436d52` PASS, ADR-02

Line 3 before the migration:
Status: **ERLEDIGT 2026-10-16** (U1–U10; U10 `f6f90202`, Tester-Lauf `87436d52` PASS, ADR-026; Pi 2026-10-16 gebootet; offen nur: Pi-Gegenprobe des EmuCON-Rueckfalls bei defektem NEWDESK) · Prioritaet: **P1** · Typ: **Abweichung von EmuTOS** (nicht blosse Architekturfrage) — Anwenderentscheidung 2026-08-31, geschaerft 2026-09-01, belegt 2026-09-25

Abhaengigkeiten: **[PORT05](PORT05.md) muss vorher stehen** (Begruendung unten).
Plan liegt: `docs/analysis/desktop-userspace-plan.md` (68 KB, 2026-09-01)

## Einordnung des Anwenders 2026-09-25

*„Das ist direkt das naechste nach dem Scrollwheel-Task. Das ist wichtiger als
alles andere."* — damit **P1**, und die Reihenfolge ist: [DESK42](DESK42.md)
(laeuft) → [GUI04](GUI04.md) (Mausrad) → **dieses Ticket**.

**Mit einer Einfuegung, die technisch zwingend ist:** [PORT05](PORT05.md)
gehoert **davor**. Zieht der Desktop nach EL0, startet er Anwendungen ueber
genau den Weg, der laut PORT05 einfriert — heute umgeht er ihn, weil er
kernelseitig startet, und das ist der einzige Grund, warum QED aus NewDesk
laeuft. Der Umzug vor dem Fix wuerde den Desktop handlungsunfaehig machen.
PORT05 ist also keine Konkurrenz zu diesem Ticket, sondern seine
Voraussetzung.

## Warum dieses Ticket ueberhaupt existiert

Die Entscheidung ist **ein Jahr alt in Projektzeit und nie eingeplant worden**,
weil es kein Ticket gab — nur einen Gedaechtniseintrag und einen fertigen Plan.
Der Anwender hat am 2026-09-25 von selbst wieder danach gefragt: *„aber wieso
ist unser Desktop nicht auf einer EL0-Anwendung? Das sollte eigentlich so
sein."*

## Ist-Zustand

`desk::run()` haengt direkt im Boot-Pfad (`kernel/src/lib.rs:329`);
`desk/mod.rs:159` nennt sich selbst *„the desktop's kernel-boot context"*.
Der Desktop ist Kernel-Code mit Oberflaeche, kein Prozess.

Gemessener Ausgangspunkt (2026-09-01): `kernel/src/desk/` = **9 234 Zeilen in
15 Dateien**; nur **sechs** direkte `crate::`-Abhaengigkeiten zum Kernel, aber
rund **40 direkte `tos_api::`-Aufrufe am Trap vorbei** (`aes::object` 9,
`bdos::fs` 9, `vdi::font` 6, `host::host` 6, `aes::window` 6, `bios::drive` 3,
`vdi::mouse` 1). Das sind die Stellen, die der Umzug geradezieht.

## Belegt 2026-09-25: EmuTOS' Desktop laeuft im User-Modus

Der Anwender fragte: *„aber im EmuTOS laeuft er doch auch im User-Space?"* —
und das stimmt, nachgelesen statt angenommen:

* `aes/gemasm.S:77-79` — die AES springt in das TEXT-Segment eines
  Accessories *„in user mode"*.
* `aes/gemstart.S:126` — *„Super() to go back to user mode"*.
* `aes/gemshlib.c:604,634` — beim Start einer Anwendung uebernimmt diese
  **denselben** Deskriptor `rlr`, den der Desktop hatte.

**Damit ist `desk/` auf EL1 eine Abweichung von EmuTOS, keine
Entwurfsfreiheit.** Nach [Don't rationalize EmuTOS deviations] ist die
Vorgabe dann *angleichen*, nicht begruenden — dieses Ticket ist also kein
„waere schoener", sondern eine offene Konformitaetsluecke.

**Ein Unterschied bleibt und bestimmt den Aufwand:** EmuTOS hat **keinen**
Speicherschutz. „User mode" heisst dort nur S-Bit aus, alles liegt in einem
flachen Adressraum. Unser EL0 bringt Modus **und** eigenen Adressraum mit.
Die Richtung ist EmuTOS-konform; der Weg dorthin ist bei uns teurer, weil die
rund 40 Aufrufe am Trap vorbei echte Adressraumgrenzen ueberqueren muessen,
die es dort nicht gibt.

## Was sich damit von selbst aufloest

In EmuTOS **ist** der Desktop der AES-Prozess `rlr`, und eine gestartete
Anwendung uebernimmt **denselben** Deskriptor (`sh_ldapp` ruft
`p_nameit(rlr, fname)`, `aes/gemshlib.c:622-634` — kein `pstart`/`getpd`).
Daher misst EmuTOS `ap_id = 0` fuer die gestartete Anwendung.

Zwei gemessene Abweichungen sind **reine Symptome** davon und duerfen nicht
vorher einzeln „behoben" werden — das waere eine hingeschriebene Zahl statt
nachgebautem Verhalten:
* `ap_id` 1 statt 0 (D-R1-5)
* `ap_private` aus dem Fallback statt der DESKTOP-Ressource (D-R1-6)

Ebenso faellt unser `save_windows`/`restore_windows`-Handoff: EmuTOS'
`sh_ldapp` schliesst **keine** Fenster. Der Handoff existiert nur, weil unser
Desktop strukturell etwas anderes ist.

**Nebeneffekt, der uns nuetzt:** der Umzug beendet den In-Kernel-Zeichenpfad,
wegen dem ein gut aussehender Desktop den EL0-Anwendungspfad bisher **nicht**
validiert.

## Beruehrungspunkt mit [PORT05](PORT05.md)

PORT05 (QED aus TeraDesk gestartet friert ein) ist der erste Fehler, der
**wegen** dieser Struktur auffaellt: der Weg „EL0-Anwendung startet
EL0-Anwendung" ist bei uns nie gegangen worden, weil der Desktop ihn nicht
geht. **PORT05 wartet aber nicht auf dieses Ticket** — dieser Weg muss
funktionieren, ganz gleich wo der Desktop lebt, und vor dem Oktober-Termin ist
der Umzug zu gross. Erst PORT05, dann hier weiter.

## Die wichtigste Randbedingung

**Es ist der dritte Anlauf.** Zwei Rebuilds sind vorher abgedriftet. Jeder
Plan muss beantworten, **was den dritten Abdrift verhindert** — sonst ist er
nicht fertig.

## Auftrag

`docs/analysis/desktop-userspace-plan.md` gegen den heutigen Stand pruefen
(er ist vom 2026-09-01, seitdem sind DESK01–DESK42 gelaufen), dann in
Arbeitseinheiten schneiden, die einzeln abnehmbar sind. Keine Umsetzung, bevor
der Schnitt steht und der Anwender ihn gesehen hat.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Fortschritt

| Einheit | Stand |
|---|---|
| U1 Instrument | **fertig** 2026-09-26 (`c141be2`) — Driftprobe 47, 12 Abweichungen gelistet |
| ACC10 (Vorbedingung) | **fertig** 2026-09-26 (`814de2c`) |
| U2 Zustandskanal | **fertig** 2026-09-26 (`d79e71f`) — Beweispunkt 1 des Plans dabei **widerlegt** |
| U3 Desktop endet (G1) | **fertig** 2026-09-26 — Driftprobe **47 → 38**; HW-Punkte 1+2 offen |
| U4 Kernel loest sich | **fertig** 2026-09-26 — Driftprobe **38 → 32**; zwei Stehenbleiber mit Messbegruendung |
| — Trennlinie — | *alles darueber ohne EL0 und ohne neue Werkzeugkette* |
| U5a Instrumente | **fertig** 2026-10-14 (`4ae60fae`) — Driftprobe v2, Konstantenabgleich, Zitat-Liste, Sequenz-Orakel |
| U6a Rust-EL0-Spike | **fertig** 2026-10-14 (`1257b7cc`) — HELLOGEM.NAP, Lader unveraendert |
| U5b Typen/Konstanten | **fertig** 2026-10-14 (`ac521aeb`) — Driftprobe 31 → 19 |
| U7 Zellmasse | **fertig** 2026-10-14 (`2c7895db`) — Driftprobe 19 → 11, 0 Pixel |
| U8a Entkopplung | **fertig** 2026-10-14 (`87adb580`) — Driftprobe 11 → 4, Kernel meldet „desktop up“ |
| U6b Bindungen + Bau | **fertig** 2026-10-14 (`df11e429`) — `newtos-gem`, `build-programs`, Einbettung mit hartem Fehler |
| U8b Desktop-Bibliothek | **fertig** 2026-10-14 (`d7c5efa2`) — `programs/desktop/`, Umgehung = Compilerfehler |
| U9-0 Voraussetzungen | **fertig** 2026-10-14 (`c04e2a05`) — Registry, PROC05, Shell-Kind-Slot, Injektor |
| U9-1/2 NEWDESK | **fertig** 2026-10-14 (`de03a50f`) — EL0-Desktop eingebettet, per `#Z` lauffaehig |
| U9-3..5 Umstieg | **fertig** 2026-10-15 (`b3b1fc54`) — Standard EL0-NEWDESK, `desk=kernel` als Rueckfall |
| U9-6 Pi-Abnahme | **fertig** 2026-10-16 — echte Pi 4, Kern `f73bb075` (sha256 `2a90feda…`), Anwender: alles ohne Fehler; Nebenfund [DESK62](DESK62.md) (alt) |
| U10 Loeschen | **fertig** 2026-10-16 (`0a3b9be9`) — Kernel ohne Desktop-Abhaengigkeit, `desk=kernel` weg, Ladefehler → EmuCON (D6-d), tote Fenster-API weg; Driftprobe **4 → 0**; offen: ADR, Tester-Lauf, Pi |

## U10 — Loeschen (2026-10-16, `0a3b9be9`)

Beleg: `measurements/measurement-2026-10-16-desk43-u10/RUNINFO.TXT`.

- **Weg:** `kernel/Cargo.toml` haengt nicht mehr an `desktop` und
  `newtos-gem` (neuer xtask-Hosttest `the_kernel_does_not_link_the_desktop`);
  Boot-Argument `desk=kernel` (`bootargs::DESK_KERNEL`), die
  Desktop-Auswahl im `sh_ldapp`, `desktop::reset()` im A106-Neustart;
  in `tos-api` die toten `save_windows`/`restore_windows` (samt Tabelle)
  und `wm_new_keep_desktop` samt den zwei A96-Tests.
- **D6-d nach U10:** ein nicht ladbarer NEWDESK wird einmal versucht, dann
  laeuft EmuCON (Alpha-Modus ueber `sh_chgrf`, Grund auf Bildschirm und
  Seriell); jeder weitere DESKTOP-Durchgang laedt nicht neu (D6-c). In QEMU
  raspi4b gezeigt, inklusive zweitem Durchgang nach `exit`.
- **Bleibt (geprueft, nicht geloescht):** `KERNEL_DTA` (EmuCON und die
  Selbsttests rufen GEMDOS aus EL1), `pd::DESKTOP` als Identitaet des
  Kernel-Kontexts (Name historisch), der Kernel-Park (Bootpfad
  `wait_for_accs`). Begruendungen „Desktop = EL1“ in Kernel, `tos-api`,
  `programs/` und Werkzeugen gegen die Quelle geprueft und berichtigt (G7).
- **Erledigt nachgezogen (2026-10-16, Anwender ja):** ADR-026 in
  `docs/MEMORY.md` (`01555c8d`); `pd::DESKTOP` → `pd::KERNEL_CONTEXT`
  (`d11ed66b`).
- **Tester-Abnahme 2026-10-16 PASS** (`d0e5afa5`,
  `measurements/measurement-2026-10-16-desk43-u10-test/`): Basislinien
  gleich, APITEST 109/0/5 in fuenf Modi, Serien/ACC/EmuCON/Zyklen/INF
  bytegleich zu U9, Rueckfall EmuCON auf beiden Boards mit genau einem
  Ladeversuch.
- **U10 auf echter Pi 4 gebootet 2026-10-16** (Kern aus `970b87d8`): Desktop, Programmstarts, TeraDesk → QED, `desk=kernel` ohne Wirkung — Anwender bestaetigt.
- **Offen:** Pi-Gegenprobe des EmuCON-Rueckfalls (braucht Kern mit defektem NEWDESK); veralteter Abschnitt `docs/api/aes/window-graphics.md`
  ~1224–1240 (beschreibt die in U10 geloeschten `wm_new_keep_desktop`/
  `desktop_resume`).

## Hardware-Abnahme U9-6 (2026-10-16)

Karte: Release-Kern aus `f73bb075` (`kernel8.img`, sha256 beginnt
`2a90feda`), alle libc-Programme neu gelinkt (TeraDesk, QED, Doom, CoreMark,
zView samt Codecs, Ball, Present). Pruefliste = RUNINFO §12 der
`measurements/measurement-2026-10-15-desk43-acceptance/`, gekuerzt auf das ohne
Zusatzprogramme Fahrbare: Boot und EL0-Desktop, Menues, Tastenkuerzel,
Fenster, Programmstart/-ende (QED, Doom, zView, Ball, Present, CoreMark,
EmuCON, TeraDesk → QED → zurueck), SYSACC, Desktop sichern / `.INF` lesen,
libc-Stichproben (Umlenkung, Ctrl-C), Gegenprobe `desk=kernel`.
Anwender: *„rest funktioniert alles"*, Tastenkuerzel ausdruecklich.

Einziger Befund: [DESK62](DESK62.md) — Sanduhr bleibt nach Drop einer Datei
auf eine Desktop-Verknuepfung; bestand schon vor dem Umbau, keine Regression.

Nicht gefahren (Karte hatte die Mittel nicht): Crash-Prompt (CRCHILD),
Panic-/Fehlblob-Abbilder, U9-1-Instrumente (Latenz, Stack-Hochstand),
`memprot=verify` mit APITEST. Sie bleiben fuer U10 bzw. eine eigene Runde.

## Hardware-Abnahme U3/U4 (2026-09-28)

Kern `92ec0ab7`. Anwender: QED per Doppelklick starten und beenden, TeraDesk,
QED **aus TeraDesk heraus**, Fensterrekonstruktion — *„funktionierte schon
laengst"* bzw. am 2026-09-26 bestaetigt. **Alt+Buchstabe: funktioniert** —
damit ist U4s `Keytbl`-Weg (`svc #14` statt `crate::keymap`) auf echter
Hardware belegt, und zwar der Klickweg davor mit.

**Damit sind U1–U4 vollstaendig abgenommen.** Offen bleiben nur U5–U10
(nach dem Termin) und [X03](X03.md) davor.

## Neuschnitt U5–U10 (Pruefung 2026-10-14, Anwender-Freigabe 2026-10-14)

Pruefung gegen den heutigen Code: `docs/analysis/desk43-u5-u10-review-2026-10-14.md`
(gegen Plan v2, `docs/analysis/desktop-userspace-plan-v2.md` §5).

**Ueberholt in diesem Ticket:** die „rund 40 Aufrufe am Trap vorbei“ (heute
keine; Driftprobe 31 = Typ-/Konstanten-/Makro-Importe) und der Absatz zu
D-R1-5/D-R1-6 (`ap_id = 0` und `ap_private` sind erledigt,
`tos-api/src/aes/appl.rs:200-224`, `aes/init.rs:61-65,98`;
`save_windows`/`restore_windows` ohne Produktivaufrufer).

**Freigegeben vom Anwender:**
- **D1** neuer Schnitt, seriell: U5a → U6a → U5b → U7 → U8a → U6b → U8b → U9 → U10;
  ein Quelltext, zwei Linkziele (Kernel-Bibliothek und `.NAP`) ab U8b.
- **D2** Werkzeugkette T1: Rust bleibt (*„ja keep rust“*). Rust-Bibliothek
  `desktop` + duennes Binaer-Crate, `no_std` + `alloc`, gelinkt mit
  `sdk/crt0.S` + `sdk/newtos.ld` ueber `rust-lld`; kein C-Neuschrieb, kein
  `std`-Port. Die Laufzeit-Crate (Trap-Bindungen, `GlobalAlloc` ueber
  `Malloc`-Hunks, `panic_handler`) steht danach auch anderen Rust-Programmen
  offen.

**Empfehlungen, Entscheidung faellig vor der jeweiligen Einheit:** D3
eingebettet im Kernel-Image (U9), D4 Debug-Kanal im Produkt aus + Ersatz fuer
das Orakel `[desk] desktop up …` (U8a), D5 Rueckfall-Flag nur bis Pi-Abnahme
U9, D6 Absturzpolitik wie EmuTOS nach Analyse (U9), D7 PROC03 umformulieren,
D8 Ticket-/Plantexte (dieser Abschnitt).

**D4 entschieden (Anwender 2026-10-14):** Im Produkt keine Debug-Ausgaben des
Desktops (EmuTOS: `KDEBUG` auskompiliert). Die Orakelzeile
`[desk] desktop up …` wird durch eine Kernel-Meldung ersetzt, wenn das
Desktop-Programm sein erstes `evnt_multi` erreicht (gleiches Kriterium, von
aussen beobachtet). Die vier Desktop-Selbsttests (DESK45) werden ein eigenes
Testprogramm. Kein neuer EL0-Debug-Kanal.

**D3 entschieden (Anwender 2026-10-14):** Das Desktop-Programm wird ins
Kernel-Image eingebettet (Registry-Name `NEWDESK`, wie EmuTOS' Desktop im
ROM); `#Z` kann weiterhin einen anderen Desktop von der Karte starten. Der
Kernel-Bau haengt vom Desktop-Bau ab und bricht bei fehlendem Blob hart ab;
je Board ein Blob (X03-Feature durchgereicht).

**D5 entschieden (Anwender 2026-10-14):** U9 kommt mit einem Rueckfall-Schalter
(Desktop im Kernel), der nur bis zur Abnahme auf der echten Pi lebt; U10
entfernt ihn. Vor U9: Researcher-Analyse zu D6 (Absturzpolitik), T10
(Injektor-Bewaffnung) und [PROC05](PROC05.md) (Exitcode im Shell-Start);
der Registry-Vergleich ueber den ganzen 128-Byte-`s_cmd` wird im U9-Auftrag
zuerst behoben.

**U9-Vorbereitung entschieden (Anwender 2026-10-14)**, Grundlage
`docs/analysis/desk43-u9-prep-2026-10-14.md`:
- **D6** wie EmuTOS (`aes/gemshlib.c:600-611`): Desktop endet → bei Fehler
  `wm_new()`, Neustart, keine Meldung. **D6-a** Panic in NEWDESK wartet auf
  eine Taste (Feature nur fuer NEWDESK). **D6-b** kein Schleifenwaechter in U9.
  **D6-c** Ladefehler wird nicht wiederholt. **D6-d** Rueckfall bei
  Ladefehler: Kernel-Desktop solange D5 lebt, danach EmuCON.
- **D5-a** Rueckfallschalter = Boot-Argument `desk=kernel`; **D5-b** Standard
  ab U9 ist der EL0-Desktop.
- **PROC05** korrigiert und als U9-0b eingeplant; **REG** exakter Vergleich bis
  NUL; **T10** Kernel veroeffentlicht den Shell-Kind-Slot (Injektor und
  „desktop up“); **STK** Stackgroesse erst nach Messung.
- Reihenfolge: U9-0a Registry · U9-0b PROC05 · U9-0c Slot + Orakelschluessel ·
  U9-0d Injektor · U9-1 Instrumente · U9-2 NEWDESK + Einbettung · U9-3
  `desk=kernel` · U9-4 Umstieg + D6 · U9-5 Beweislaeufe · U9-6 Pi-Abnahme.
