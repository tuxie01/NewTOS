# PROC03 — Desktop/AES als echter EL0-Prozess mit eigenem PD (`p_parent` exakt wie EmuTOS)

Status: DONE (2026-11-11) | Priority: P2 | Commit: a0cf613bb | Receipt: PROC03-STATUS-DONE.MD

Migrated (owner batch 3, 2026-11-11), evidence: receipt TASK/PROC03-STATUS-DONE.MD exists; commit a0cf613bb; TASK/PROC03-STATUS-DONE.MD, commit a0cf613bb (feat(bdos): EmuTOS parent chain with accdesk/EmuCON PDs, Pterm via p_p), fix/feat commits naming the id: ee92d3f93, a0cf613bb (n=2), measurements/measurement-2026-10-06-post-e9/

Line 3 before the migration:
Status: **ERLEDIGT** 2026-10-19 (QEMU) — [Quittung](PROC03-STATUS-DONE.MD) · Prioritaet: **P2** · Typ: Abweichung von EmuTOS — aus [ARCH03](ARCH03.md) Nacharbeit E9 (Punkt 1, Weg B)

Abhaengigkeiten: zuerst Weg A (Stellvertreter-PDs) aus der E9-Nacharbeit; Bezug Memory „Desktop must run in user scope“

> **Neufassung (2026-10-17, Entwurf)** siehe unten — der urspruengliche Text ist ueberholt (DESK43/ADR-026, PPARENT) und enthaelt eine Quellenabweichung (`accdesk`-PD, siehe Befund 1). Er bleibt zur Nachvollziehbarkeit stehen.

## Sachverhalt

EmuTOS setzt `p_parent` nur in `proc_go` (`bdos/proc.c:541`). AUTO- und
AES-Start haben `&initial_basepage` zum Elternprozess (`bios/bios.c:693,862-865`,
`bdos/bdosmain.c:339-342`), Desktop- und `#Z`-Starts den PD des AES
(`aes/gemshlib.c:574-579,645`), Accessories 0 (`util/gemdos.c:34`, `proc.c:368`).
NewTOS hat weder AES-PD noch `initial_basepage`; der Lader schreibt 0
(`loader.rs:841`). Anwenderentscheidung 2026-10-06: jetzt Weg A (zwei
Stellvertreter-PDs), spaeter Weg B = dieses Ticket: Desktop/AES laufen als
echter Prozess, `run` ist dann wirklich der AES-PD.

Beleg: `measurements/measurement-2026-10-06-post-e9/RUNINFO.TXT`,
`TASK/ARCH03-POSTE9-STATUS-UNFINISHED.MD`.

---

## Neufassung (2026-10-17, Entwurf)

Status: **ERLEDIGT** 2026-10-19 (QEMU) — [Quittung](PROC03-STATUS-DONE.MD) · Prioritaet: **P2** · Typ: Abweichung von EmuTOS (beobachtbar + intern, getrennt gekennzeichnet)

Beleg: `measurements/measurement-2026-10-17-proc03/RUNINFO.TXT` (QEMU-Sonde auf HEAD `7d79f4a4`, on und strict); EmuTOS-Pin `1a7484ab`.

### 1. Ausgangslage heute (gelesen / gemessen)

* Der Desktop ist der EL0-Prozess NEWDESK (ADR-026, DESK43 U9/U10). Er, jedes `#Z`-Programm, jedes per `shel_write` gestartete Programm und AUTO-Programme gehen durch `kernel/src/launch.rs` `launch()` → `pexec::set_kernel_parent` (`launch.rs:252`) → `p_parent` = Stellvertreter-PD von `standin::kernel_run()` (`pexec.rs:498-519`): `initial_basepage` vor dem AES, AES-PD nach `enter_aes()` (`kernel/src/lib.rs:518`). Der Lader schreibt beim *Laden* 0 (`loader.rs:894`), gesetzt wird beim Start.
* Accessories: `EnvSpec::Inherit`, `p_parent` bleibt 0 (`kernel/src/accs.rs:201-216`).
* `Pexec`-Kinder aus EL0: `p_parent` = PD des Aufrufers (`exceptions.rs:2155`); aus dem Kernel-EmuCON (Aufrufer `null`): AES-Stellvertreter.
* Die Kette ist im Kernel **nur ein Zeiger**: `Pterm` kehrt ueber `parent_slot` zurueck, **nicht** ueber `p_parent` (`tos-api/src/bdos/process/pterm.rs:246-260`; EmuTOS `run = run->p_parent`, `bdos/proc.c:609`). `RUN` ist im Kernelkontext (Shell-Loop, EmuCON) `null`.
* Stellvertreter-PDs: eine Kernelseite, read-only fuer alle (`standin.rs`); AES-PD `p_hitpa = p+0x100`, `p_tbase = 0`.

### 2. Befund: Korrektur der bisherigen Quellenlesung (gilt auch fuer PPARENT/Spec)

1. **Eltern von Desktop, `#Z`- und Desktop-Programmen ist nicht der AES-PD, sondern ein dritter PD, der `accdesk`-PD.** `gem_main` ruft `aes_run_rom_program(accdesk_start)` (`aes/geminit.c:894`); das ist `Pexec(PE_BASEPAGEFLAGS,…)` + `dos_exec(PE_GOTHENFREE)` aus dem AES-Prozess (`aes/gemshlib.c:570-580`), `proc_go` setzt `p_parent = run` = AES-PD und danach `run` = neuer PD (`bdos/proc.c:541,575`). `accdesk_start` (`aes/gemstart.S:139-160`) ruft `run_accs_and_desktop` (`geminit.c:654`) *in diesem Prozess*; darin laufen `load_accs` (`:687`) und `sh_main` (`:739`) → `sh_ldapp` (`gemshlib.c:592`, `:725`). Also ist `run` bei `dos_exec(PE_LOADGO,…)` (`gemshlib.c:645`) und bei `aes_run_rom_program(deskstart)` (`:606`) der **accdesk-PD**. `run` wird nur in `bdos/bdosmain.c:339`, `proc.c:575`, `proc.c:609` geschrieben (grep ueber `bdos/`, `aes/`). Kette in EmuTOS fuer ein `#Z`-/Desktop-Programm: **Programm → accdesk-PD → AES-PD → `initial_basepage` → 0** (4 Stufen); NewTOS heute: **Programm → AES-Stellvertreter → `initial_basepage` → 0** (3 Stufen; Sonde, Zeilen `[P3 Z] self/parent/grandparent`).
2. Accessories: `Pexec(PE_LOAD)` mit `run` = accdesk-PD → `p_parent` 0 (unveraendert richtig), Umgebung = Kopie von `run->p_env` = accdesk-PD = Kopie der AES-Umgebung (Inhalt gleich wie heute).
3. Der accdesk-PD hat `p_tbase = accdesk_start`, `p_tlen = p_dlen = p_blen = 0` (nur `p_tbase` gesetzt, `gemshlib.c:575`), `p_flags = PF_STANDARD`, `p_hitpa = p+max`, `p_env` = Kopie, `p_uft`/`p_curdir`/`p_curdrv` vom AES-PD geerbt (`proc.c:388-409`). Er endet erst mit dem AES (Aufloesung/Rez-Wechsel).
4. `Mshrink` aendert `p_hitpa` nicht (kein Treffer in `bdos/` ausser `proc.c:374`, `kpgmld.c`); der AES-PD hat in EmuTOS `p_hitpa = p+max`, der Stellvertreter `p+0x100` (Abweichung durch Fehlen, bekannt).
5. EmuCON (`CONSOLE_APP`, `gemshlib.c:614-622`) ist ebenfalls ein `aes_run_rom_program`-Kind des accdesk-PD; Programme, die EmuCON per `Pexec` startet, haben den EmuCON-PD als Eltern.
6. EmuTOS hat **kein** `Pgetpid`/`Pgetppid` (kein Treffer in `bdos/`, `include/`); mintlib faellt auf `_base>>8` und `p_parent>>8` zurueck (`sdk/libc/mintlib/unix/getpid.c`, `getppid.c:22`).

### 3. Beobachtbar (Vertrag) gegen intern

| # | Unterschied | Art | Beleg |
|---|---|---|---|
| B1 | Kettentiefe/-inhalt: kein accdesk-PD zwischen Programm und AES-PD; Felder des AES-PD (`p_hitpa`, `p_tbase`) weichen ab | **beobachtbar** (Programm kann `p_parent` ablaufen) | Sonde; EmuTOS laut §2 Befund 1–3 |
| B2 | `Pexec` aus dem Kernel-EmuCON: Eltern = AES-Stellvertreter statt EmuCON-PD | **beobachtbar** (Kette eines von EmuCON gestarteten Programms) | `exceptions.rs:2155`; `gemshlib.c:614-622` |
| B3 | `p_curdrv`/`p_curdir` je PD fehlen: PD+0x37 liest 0 statt Bootlaufwerk; Kind-`Dsetpath` aendert das Verzeichnis des Elternprozesses | **beobachtbar, mit Wirkung** (Sonde: Eltern `Dgetpath '\SUB'` → nach Kind `''`; EmuTOS: unveraendert, `bdos/fsdir.c:1129-1145`) — **gehoert zu PROC04 Punkt 2**, hier nur als Abhaengigkeit | Sonde Zeilen AUTO 63→74, Z 125→136; `loader.rs:901`, `path.rs:28` |
| B4 | `Pterm` kehrt ueber `parent_slot`, nicht ueber `p_parent` zurueck; ein Programm, das sein eigenes `p_parent` umbiegt, hat keine Wirkung | beobachtbar, aber nur fuer exotische Lader (kein Treffer im SDK: nur mintlib `getppid`, `tfork`, `heap_init`/`crt0` *lesen* `p_parent`) | `pterm.rs:246-260`; `bdos/proc.c:609` |
| B5 | `*os_run` waehrend ein Accessory laeuft: NewTOS = Accessory-PD; EmuTOS = `run` = zuletzt gestarteter Prozess (Vordergrundprogramm bzw. accdesk-PD). Folge: `Malloc`-Besitzer eines Accessory (`bdos/iumem.c:134` `m_own = run`) ist in EmuTOS das Vordergrundprogramm, in NewTOS das Accessory | beobachtbar; EmuTOS-Verhalten ist dort ein bekannter Fallstrick — **Anwenderentscheidung noetig**, ob 1:1 | `accs.rs`; Sonde ACC `os_run PD` = eigener PD |
| B6 | Stellvertreter-PDs read-only, Kind unter `memprot=strict` kann den PD des Elternprozesses nicht lesen (`PROT_P`) | NewTOS-Eigenschaft (Speicherschutz), bewusst | `standin.rs`; S2ALLS |
| I1 | Wo die PDs liegen, Veroeffentlichung, `RUN == null` im Kernelkontext, `kernel_run()` | **intern** | `standin.rs` |
| I2 | AES/Shell-Loop als Kernelkontext statt EL0-Prozess; kein eigener AES-Adressraum | **intern** (kein Programm kann ihn von aussen sehen, solange B1/B2 stimmen) | `lib.rs:518ff` |

Gleich wie EmuTOS (Sonde, on und strict): AUTO-Programm `p_parent` = `initial_basepage` (leere Umgebung `00 00`), `#Z`- und `shel_write`-Programme `p_parent` = AES-Stellvertreter (Umgebung `PATH=…`), Accessory `p_parent` 0 mit AES-Umgebung, `Pexec`-Kind `p_parent` = Aufrufer (auch aus einem Accessory), `shel_envrn` zeigt in die AES-Umgebung, `p_uft` `ff ff fe fd ff ff` an `initial_basepage`.

### 4. Wer koennte B1–B5 merken (SDK gelesen)

* mintlib-Start (`startup/crt0.S:52-56`, `sdk/libc/newtos-arch/heap_init.c:149`, `sdk/crt0.S:63`): nur `p_parent == 0` (Accessory-Test) — erfuellt. `getppid` (`getppid.c:22`): liefert `p_parent>>8`; Wert aendert sich durch B1 (anderer PD), kein Programm im SDK wertet ihn aus.
* `tfork` (`sdk/libc/overlay/mintlib/thread.c:68-77`): liest `b->p_parent` und `parent->p_env` — braucht, dass der Eltern-PD das *Aufrufer*-PD ist (Pexec-Kind: erfuellt), nicht den AES-PD.
* TeraDesk (`sdk/apps/teradesk/environm.c`, `_BasPag->p_env`), QED, cflib, gemlib, windom, ldg, zview: **kein** Lesezugriff auf `p_parent` gefunden (`grep` ueber `sdk/`, ohne Build-/Vendorverzeichnisse; `ldg/…/tsrmain.c` liest `_base`-Felder, `p_parent` nicht gefunden — nicht Zeile fuer Zeile geprueft). Das Wissen "gestartet vom Desktop?" kommt in diesen Programmen aus `appl_init`/`shel_read`, nicht aus `p_parent`. Praktische Wirkung von B1/B2 auf bekannte Programme: **keine nachgewiesen**; sie ist Konformitaet (1:1-Ziel), keine Fehlerbehebung.
* B3 (Verzeichnis) hat dagegen Wirkung: ein Programm, das ein Kind startet, das `chdir` macht (Shells, Launcher, `system()` aus mintlib), sieht sein Verzeichnis verstellt. Das ist PROC04 Punkt 2.

### 5. Umfang (Vorschlag, nach Stufen)

**Stufe 1 — beobachtbare Kette (B1, B2). Spec zuerst.**
* `docs/api/gemdos/process/_parent-and-environment.md` §1/§3/§4 und `docs/COMPATIBILITY.md:716` berichtigen (accdesk-PD); `standin.rs`-Kopf, `launch.rs:246-250`, `accs.rs:205-207`, `pexec.rs:488-495` Kommentare folgen.
* Dritter Stellvertreter `accdesk` auf der Stellvertreterseite: Felder wie `aes_run_rom_program` sie schafft (`p_parent` = AES-PD, `p_tbase` = Platzhalter ≠ 0 fuer `accdesk_start`, `p_flags` = `PF_STANDARD`, Umgebung = Kopie der AES-Umgebung, `p_uft` geerbt). `kernel_run()` ist ab `enter_aes()` der accdesk-PD (= EmuTOS-`run` im Shell-Loop); `shel_envrn`/`ad_envrn` bleiben beim AES-PD (`gemstart.S:78-96`).
* Desktop-/`#Z`-/`shel_write`-Programme und NEWDESK: `p_parent` = accdesk-PD. Accessories unveraendert 0.
* EmuCON: eigener Stellvertreter-PD (`p_parent` = accdesk-PD, Umgebung = Kopie `ad_envrn`), solange der Kernel-EmuCON laeuft ist er `kernel_run()`; `Pexec` aus EmuCON bekommt ihn als Eltern.
* AES-PD-Felder: `p_hitpa`/`p_tbase` so, wie EmuTOS sie ausweist, soweit ohne AES-TPA darstellbar; wo nicht darstellbar, als "Abweichung durch Fehlen" in der Spec stehen lassen (Entscheidung Anwender, Frage 4).
* Abnahme: Sonde `measurements/measurement-2026-10-17-proc03/method/p3probe.c` (nach Erwartung von EmuTOS, siehe 6) PASS in `memprot` on/off/strict; Regressionsreihe wie PPARENT (APITEST, Desktop, TeraDesk, QED, Accessory, TeraDesk→QED, EmuCON-Folge, S2ALL/S2ALLS, VFORKT).

**Stufe 2 — Abhaengigkeit, nicht Teil dieses Tickets: PROC04 Punkt 2 (`p_curdrv`/`p_curdir` je PD, `init_pd_files` zweite Haelfte).** Erst danach sind B3 und das Byte PD+0x37 EmuTOS-gleich; die Stellvertreter-PDs tragen `p_curdrv = Bootlaufwerk` bereits.

**Stufe 3 — "AES-PD als echter Prozess" im Sinn von Weg B (intern, I2). Empfehlung: nicht bauen, solange Stufe 1 und B5 entschieden sind.** Begruendung nur aus dem Beobachtbaren: kein Programm kann unterscheiden, ob accdesk-/AES-PD ein Slot mit eigener TPA oder eine Kernelseite ist, solange (a) die Kette B1/B2 stimmt, (b) `*os_run` waehrend des Kernelkontexts nicht lesbar ist (Kernelkontext fuehrt kein EL0-Programm aus), (c) B5 entschieden ist. **Das ist eine Schlussfolgerung, kein Beleg** — sie hebt sich auf, wenn B5 "wie EmuTOS" entschieden wird: dann braucht `RUN` einen Wert ausserhalb der EL0-Prozesse (Besitzer-/`run`-Modell, wie viele Stellen `current() == null` heute voraussetzen, ist nicht gezaehlt: Analyse noetig).

Nicht im Umfang: AES-Code nach EL0 (ADR-026 verbietet nichts, verlangt es aber nicht; eigener Auftrag), Pi-Abnahme (nur auf Ansage), `Pgetpid`/`Pgetppid` (MiNT, nicht EmuTOS).

### 6. Tests / Abnahmekriterien (Stufe 1)

Erwartung EmuTOS (aus den zitierten Quellen, Kette in Zeigern; `X` = Programm):

| Start | `p_parent`-Kette |
|---|---|
| AUTO | `initial_basepage` → 0 |
| `#Z`, Desktop-Doppelklick, `shel_write` | accdesk → AES-PD → `initial_basepage` → 0 |
| Accessory | 0 |
| `Pexec`-Kind von `X` | `X` → (Kette von `X`) |
| Programm aus EmuCON | EmuCON-PD → accdesk → AES-PD → `initial_basepage` → 0 |

* Hosttests (`tos-api/tests/`): Stellvertreterseite enthaelt 3 (4 mit EmuCON) PDs; Kette `accdesk.p_parent == aes`, `aes.p_parent == initial`, `initial.p_parent == 0`; `kernel_run()`-Folge vor/nach `enter_aes()`; Felder wie oben; Umgebungen (`accdesk` = AES-Kopie).
* QEMU-Sonde (`p3probe.c` mit angepasster Erwartung, Kette bis 4 Stufen ablaufen): `#Z`, `shel_write`-Folgeprogramm, Accessory (+ `Pexec`-Kind), AUTO (+ Kind), EmuCON-Kind (neu: Skript ueber die Konsole), on/off/strict. Rotlauf: dieselbe Sonde auf `7d79f4a4` zeigt 3 Stufen (FAIL der Tiefenpruefung).
* Kein Rueckschritt: Hosttests, Bau 64/94, Clippy 76/125, Drift 0, APITEST 109/0/5 in allen Modi, Desktop mit/ohne Accessory, TeraDesk, QED pixelgleich.

### 7. Offene Fragen an den Anwender

1. Stufe 1 so schneiden (accdesk- und EmuCON-Stellvertreter, Spec-Berichtigung), ja/nein? Die Korrektur der Spec ist unabhaengig davon noetig.
2. B4 (`Pterm` ueber `p_parent` statt `parent_slot`): beobachtbar nur fuer Programme, die ihr eigenes `p_parent` umbiegen; keine im SDK. Parken oder Probe bauen?
3. B5 (`*os_run`/`Malloc`-Besitzer eines Accessory = Vordergrundprogramm): EmuTOS-treu nachbilden oder bewusst bei "Accessory besitzt sein Malloc" bleiben (FreeMiNT-Richtung)? Das entscheidet auch, ob Stufe 3 noetig wird.
4. AES-PD-Felder `p_hitpa`/`p_tbase`: Platzhalterwerte erfinden (Quelle fehlt: `exec_os` hat keine Entsprechung) oder als Abweichung durch Fehlen dokumentiert lassen?

## Anwenderentscheidung (2026-10-18)

Anwender: *„bau es so, dass es wie auch im EmuTOS ist, sonst besteht die
Gefahr, dass wir Probleme bekommen beim Portieren von Apps“*. Damit gilt fuer
alle Fragen aus §7: **EmuTOS-treu**.

1. Stufe 1 (accdesk- und EmuCON-PD, Spec-Berichtigung): ja.
2. B4: `Pterm` kehrt ueber `p_parent` zurueck wie `bdos/proc.c:609`.
3. B5: `run`/`*os_run` und `Malloc`-Besitz eines Accessorys wie EmuTOS
   (`bdos/iumem.c:134` `m_own = run`). Folge (EmuTOS-Fallstrick): Speicher,
   den ein Accessory waehrend eines Programms holt, gehoert dem Programm und
   wird mit dessen `Pterm` frei. Damit wird Stufe 3 (`run` ausserhalb der
   EL0-Prozesse, AES-/accdesk-PD als Besitzer) noetig — erst analysieren.
4. AES-PD-Felder: so, wie EmuTOS sie ausweist; wo ohne AES-TPA nicht
   darstellbar, Analyse vorlegen statt Werte zu erfinden.
5. Stufe 2 (Verzeichnis/Laufwerk je PD, PROC04 Punkt 2) wird mitgezogen —
   einzige Luecke mit gemessener Wirkung, gleiche Begruendung (Portierung).

Reihenfolge (seriell): Spec fuer alles → Stufe 1 + B4 → PROC04 Punkt 2 →
Analyse Stufe 3/B5 → Bau Stufe 3/B5. Jeweils rot/gruen + unabhaengiger Tester.

## Stufe 1 + B4 erledigt (2026-10-18, QEMU)

Bau `31db1d1c`; Belege
`measurements/measurement-2026-10-18-proc03-s1/` (rot/gruen) und
`measurements/measurement-2026-10-18-proc03-s1-test/` (Tester PASS).
Naechstes: PROC04 Punkt 2; B5 wartet auf Anwenderentscheidung (Konflikt mit
ARCH03 E15, `_run-ownership-curdir.md` D-8/D-9).

## Anwenderentscheidung B5 (2026-10-18)

Vorschlag (2) bestaetigt: waehrend ein Accessory laeuft, zeigen `*os_run`
und der `p_parent` eines von ihm per `Pexec` gestarteten Kindes wie EmuTOS
auf `run` (Vordergrund-/accdesk-PD). Speicherbesitz, Verzeichnis, DTA,
Dateien und Speicherschutz bleiben beim Accessory selbst — benannte
Abweichung in `docs/COMPATIBILITY.md` beim Bau (E15 und Speicherschutz
unberuehrt). `_run-ownership-curdir.md` D-7/D-8/D-9 damit entschieden.
