# MEM11 — Anwenderadressen < 2 GiB nur zufaellig: die Abbildungsgrenze erlaubt 4 GiB

Status: **GEPARKT** 2026-10-31 (Anwender: heute nicht erreichbar; der Hosttest, der die Speicheraufteilung unter 2 GiB festhaelt, ist umgesetzt 2026-10-31, siehe *Stand*) · Prioritaet: **P2** · Typ: Kernfehler (Vertragsgrenze nicht erzwungen) — Anwenderbefund 2026-10-26

Prioritaet P2 wie die uebrigen Defekte der Reihe (MEM01–MEM05): der Vertrag
ist verletzbar, aber heute fuehrt kein gefundener Weg tatsaechlich eine
Adresse >= `0x8000_0000` heraus (siehe "Heute erreichbar?"), deshalb nicht P1.

## Sachverhalt

Eine TOS-Adresse ist ein `LONG`, also vorzeichenbehaftet 32 Bit. Eine
Anwenderadresse >= `0x8000_0000` wird beim Verbreitern vorzeichenerweitert und
sieht fuer das Programm wie ein negativer Fehlercode aus. Beispiel: `Malloc`
liefert einen Block bei `0x8000_1000`; der GEMDOS-Rueckgabeweg macht daraus
`(Long as i64)` = `0xFFFF_FFFF_8000_1000`. Ein Aufrufer, der den
Rueckgabewert wie bei TOS ueblich mit `< 0` auf einen Fehler prueft, haelt den
Block fuer einen Fehlercode — obwohl er vergeben und abgebildet ist.

Wo eine Adresse als `LONG` herausgeht (gelesen):

| Weg | Stelle | Breite |
|---|---|---|
| GEMDOS-Rueckgabe allgemein | `kernel/src/arch/aarch64/exceptions.rs:480` (`gemdos::dispatch(&args) as i64`) | `Long` → vorzeichenerweitert in `x0` |
| `Malloc` / `Mxalloc` | `tos-api/src/gemdos/memory.rs:89`, `:69` | `Long` |
| `Fgetdta` | `tos-api/src/bdos/fs/find.rs:142` (`p_xdta` round-trip) | `Long` |
| Basepage-Felder (`p_lowtpa` … `p_xdta`) | `tos-types/src/basepage.rs:97-115` | `Long` |
| MD `m_start`/`m_length` (`Getmpb`) | `tos-api/src/bdos/mem/init.rs:224-236` (`seed_md`) | `Long` |
| `AESGLOBAL` `ap_private`/`ap_3resv` | `docs/COMPATIBILITY.md:1150-1167`, `tos-api/src/aes/appl.rs:172-175` | 4 Byte auch fuer LP64 |
| `Physbase`/`Logbase` | `tos-api/src/xbios/screen.rs:92-138` → `usermem::expose_attr` | XBIOS gibt `i64` zurueck (`tos-api/src/xbios/mod.rs:65`), also im Register nicht erweitert — der Wert landet aber in 32-Bit-Feldern (`v_bas_ad`, ILP32-Bindungen) |

Der Vertrag steht in `docs/COMPATIBILITY.md:1164`: *"A 4-byte pointer suffices
because every user address is below 2 GiB (VA = PA, `docs/MEMORY.md`
ADR-028)"*; ADR-028 begruendet VA = PA ausdruecklich damit, dass jede
Anwenderadresse ein positives 32-Bit-`LONG` bleibt (`docs/MEMORY.md:2669-2670`);
Plan-Invariante I2 (`docs/analysis/arch03-implementation-plan.md:58,279`).

Die Abbildungsgrenze ist aber 4 GiB:

- `tos-api/src/bdos/process/mmu.rs:114-117` — `GIB_COUNT = 4`,
  `USER_VA_LIMIT = 4 GiB`. Die Modulnotiz `:33-34` sagt dazu nur, jede
  Anwenderadresse liege "anyway" unter `0x8000_0000`.
- Verwendet als einzige Obergrenze in `map_shared` (`:809`), `map_pools`
  (`:915`), den L3-/Slot-Pfaden (`:946`, `:962`) und `translate` (`:1402`).
- `usermem::expose_attr` (`tos-api/src/bdos/process/usermem.rs:229,259`)
  veroeffentlicht ein Kernobjekt an seiner PA (`kva_to_user_va`) ueber
  `map_shared` und gibt diese PA ungeprueft als Anwenderadresse zurueck.

## Heute erreichbar? Nein — aber nur durch Platzierung weiter oben

Kein gefundener Weg liefert heute eine Adresse >= `0x8000_0000`, und zwar aus
drei voneinander unabhaengigen Gruenden, von denen keiner an der Stelle sitzt,
an der die Adresse herausgegeben wird:

1. `layout::discover` kappt DRAM auf `min(ttram_start + POOL_MAX_BYTES,
   TOS_MD_MAX_END)` **bevor** Pools, Kern-Heap, Blockcache und (qemu-virt)
   Framebuffer gebildet werden (`kernel/src/bios/machine/layout.rs:120`,
   `:515-530`, `:573-595`). Alles, was `expose_attr` veroeffentlichen kann,
   liegt damit unter `0x7FFF_F000`.
2. `seed_md` lehnt Pools ab, die nicht in ein positives `LONG` passen
   (`tos-api/src/bdos/mem/init.rs:224-236`).
3. Pi-Framebuffer: die VC-Busadresse wird mit `0x3FFF_FFFF` maskiert
   (`kernel/src/drivers/bcm2711/framebuffer.rs:64,561`) → < 1 GiB.

Belegte Speicherkarten: Pi (Firmware 960 MiB) TT-RAM endet bei
`0x39e0_0000`, OS-Reserve bei `0x3c00_0000`
(`measurements/measurement-2026-10-09-version05/serial/boot-serial.log`);
qemu-virt endet bei `0x5100_0000`
(`measurements/measurement-2026-10-07-memprot-fast/serial/series/mf-strict-virt-selftests-serial.log`).
Mehr als 1 GiB ARM-RAM wird auf der Pi nicht verwaltet (Kappung 1).

Der Fehler ist also: die Vertragsgrenze 2 GiB wird nirgends an der
Abbildungs-/Herausgabegrenze erzwungen; sie haelt nur, solange Platzierung und
Kappung weiter oben so bleiben. Eine kuenftige Aenderung (groessere
`POOL_MAX_BYTES`, ein zweiter RAM-Bereich der Firmware, eine Veroeffentlichung
eines Objekts ausserhalb von `dram`, ein anderes Framebuffer-Verfahren)
liefert dann still Adressen, die Programme als Fehler lesen.

## Teilschritt erledigt: Debug-Pruefung am Rueckgabeweg (2026-10-26)

Nur die Pruefung, nicht die Erzwingung — die bleibt offen (Arbeit 1-3):

- [x] `debug_assert` auf Zeigerergebnisse, die als `LONG` herausgehen:
  Helfer `debug_assert_user_ptr(func, addr)` und Grenze `USER_PTR_END =
  0x8000_0000` in `tos-api/src/bdos/process/usermem.rs:57-74`; angewandt in
  `tos-api/src/gemdos/memory.rs` (`pointer_result`, `:116-121`, gerufen aus
  `mxalloc` `:74` und `malloc` `:94`, `:106`) und in
  `tos-api/src/bdos/fs/find.rs` (`fgetdta`, eingegrenzt, siehe den
  naechsten Abschnitt). Ausgenommen: die `-1`-Abfrage von
  `Malloc`/`Mxalloc` (Groesse, kein Zeiger); `0` (Fehlschlag) besteht.
  Release-Bauten uebersetzen die Pruefung weg.
- [x] Hosttests: `tos-api/tests/mem11_user_ptr_assert.rs` (Grenzfall
  `0x7FFF_FFFF` besteht, `0x8000_0000` loest im Debug-Bau aus; `-1`-Abfragen
  und echte Bloecke ueber die Trap-Eintraege; `Fgetdta` mit `p_xdta` unter /
  auf 2 GiB) und `tos-api/src/gemdos/memory.rs` `mod tests` (die
  `-1`-Ausnahme mit einem Groessenwert >= 2 GiB).

## Debug-Pruefungen nur fuer Kerninvarianten (2026-10-26)

Anwenderentscheidung 2026-10-26: *"Debug asserts check kernel invariants,
never user input."* Die erste Fassung pruefte jedes `Fgetdta`-Ergebnis eines
Programms. `Fsetdta` nimmt aber jede Adresse an (EmuTOS `xsetdta`,
`bdos/fsmain.c:403-405`: `run->p_xdta = addr`, keine Pruefung), also konnte
ein Programm mit `Fsetdta(0xDEAD0000)` + `Fgetdta` einen Debug-Kern anhalten.
Gemessen (Debug-Kern von `844a9872`, raspi4b):
`[PANIC] Fgetdta: user pointer 0xdead0000 >= 0x8000_0000 would read as a
negative LONG (MEM11) at tos-api/src/bdos/fs/find.rs:150:9`.

Was jetzt geprueft wird:

| Ergebnis | Herkunft | Debug-Pruefung |
|---|---|---|
| `Malloc`/`Mxalloc`-Block | Kern (Pool) | ja, unveraendert (`tos-api/src/gemdos/memory.rs`, `pointer_result`) |
| `Fgetdta` = Vorgabe-DTA `&p_cmdlin` | Kern (Lader/`standin` setzen `p_xdta`, EmuTOS `bdos/proc.c:375`) | ja, auf der vollen Adresse `pd + 0x80` |
| `Fgetdta` = per `Fsetdta` gesetzte DTA | Programm | nein — Rueckgabe derselben 32 Bit wie EmuTOS `xgetdta` (`bdos/fsmain.c:394-397`) |

Unterscheidung ohne neuen Zustand: `p_xdta` ist genau dann die Vorgabe, wenn
es gleich `(pd + 0x80)` in 32 Bit ist, `pd` = `process::current()` (die
Anwenderadresse der Basepage). `default_dta(pd)` und
`debug_assert_fgetdta(pd, dta)` in `tos-api/src/bdos/fs/find.rs`. Setzt ein
Programm seine DTA selbst auf `&p_cmdlin`, wird diese Kernadresse geprueft —
richtig, sie stammt vom Kern.

## Fall: `Fsetdta(unabgebildet)` + `Fsfirst` — Programm endet, Kern lebt

EmuTOS (Stand `1a7484ab`): `xsfirst` schreibt als Erstes in die DTA
(`bdos/fsdir.c:528-531`, `dt->dt_offset_drive = -1L`), im Supervisor-Modus.
Auf unabgebildetem Speicher ist das ein Busfehler im Trap: `any_vec`
(`bios/vectors.S:152-153`) → `dopanic` (`bios/kprint.c:302`, Bericht, Taste)
→ `kill_program` = `Pterm(-1)` des Aufrufers (`bios/kprint.c:587-594`,
`bios/panicasm.S:96-99`). Das Programm bekommt keinen Fehlercode, es wird
beendet; der Elternprozess sieht `0xFFFF`.

NewTOS (unveraendert, schon so seit ARCH03 3a): `fsfirst` schreibt die DTA
ueber `uaccess` (`sttr`, `put_state`/`makbuf` in
`tos-api/src/bdos/fs/find.rs`); der Fehlzugriff wird ueber die Fixup-Tabelle
abgefangen (`tos-api/src/uaccess.rs`), der Syscall-Schluss beendet den
Aufrufer mit Absturzbericht und `Pterm(-1)`
(`kernel/src/arch/aarch64/exceptions.rs`, `complete_call_tail` →
`uaccess_fault_tail`). Keine Kernpanik, auch im Debug-Bau.

Testfall: `sdk/examples/uakid.c` Fall `D` (neu, neben dem bisherigen `d`):
`Fsetdta(0xDEAD0000)`, `Fgetdta` muss dieselben 32 Bit liefern (sonst
Ende mit 4), dann `Fsfirst("C:\*.*")`; im Standardfallsatz von
`sdk/examples/uacc3a.c` (Plattenfall). Hosttests:
`tos-api/tests/mem11_user_ptr_assert.rs` (Programm-DTA >= 2 GiB kommt
unveraendert zurueck ohne Ausloesen; Vorgabe-DTA auf 2 GiB und Basepage ueber
4 GiB loesen aus). Der Fehlzugriff selbst ist nicht hosttestbar (auf dem Host
gibt es keine Fixup-Behandlung, `tos-api/src/uaccess.rs` Modulnotiz).

Lauf 2026-10-26, Debug-Kerne (`cargo xtask build --board rpi4`, Dev-Profil,
`debug_assertions` an) mit IKBD-Injektor (`tools/inject-arm.py`
`launch:UACC3A`, Leertaste beantwortet den Absturzbericht), QEMU raspi4b
`-append serial=on`, privates 64-MiB-Abbild mit `#Z 01 C:\UACC3A.NAP@`,
Faelle `dD.`:

- vorher (`844a9872`): `d` → `0xFFFF` OK; `D` → Kernpanik in `Fgetdta`
  (siehe oben), Maschine steht.
- nachher: `d` → `0xFFFF` OK; `D`: "UAKID: Fgetdta returned the DTA set",
  Busfehler `far=0xdead0000` im `Fsfirst`, Taste, `Pterm(-1)` →
  `0xFFFF` OK; `.` → 0; "UACC3A: PASS"; danach `[desk] desktop up`.

Offen aus dieser Durchsicht (nicht geaendert, Entscheidung beim Lead): NewTOS
schreibt die DTA in `Fsfirst` erst **nach** der Pfadaufloesung, EmuTOS davor
(`bdos/fsdir.c:531`). Mit einer schlechten DTA und einem Pfad, dessen
Laufwerk/Verzeichnis nicht existiert, beendet EmuTOS das Programm, NewTOS
liefert `EPTHNF`. Ebenso antwortet NewTOS auf eine DTA `0` mit `EFILNF`
ohne Schreibversuch, EmuTOS hat diese Abfrage nicht.

## Arbeit

1. Eine benannte Vertragsgrenze (Anwenderadressen `< 0x8000_0000`) an der
   Abbildungsgrenze erzwingen: `map_shared`/`map_pools`/`expose_attr` lehnen
   jeden Bereich ab, der sie ueberschreitet (Fehler statt Abbildung;
   `expose_attr` → `None`, die Aufrufer melden das wie heute einen
   fehlgeschlagenen Aufruf). Ob `USER_VA_LIMIT` selbst auf 2 GiB sinkt oder
   eine zweite Konstante neben der Tabellenreichweite steht, entscheidet der
   Developer nach Durchsicht aller `USER_VA_LIMIT`-Nutzer (die Tabellen decken
   GiB 0..3 ab; pruefen, ob irgendein Eintrag oberhalb 2 GiB fuer Anwender
   gebraucht wird).
2. Modulnotiz `tos-api/src/bdos/process/mmu.rs:33-34` ("anyway") durch die
   erzwungene Regel ersetzen; `docs/COMPATIBILITY.md:1164` auf die
   erzwingende Stelle verweisen lassen.
3. Keine Aenderung an Kappung, Pools oder Framebuffer — sie bleiben richtig,
   werden aber nicht mehr die einzige Sicherung.

## Abnahme

- Jede Adresse, die ein Programm vom Kern bekommt (Pool-Block,
  Veroeffentlichung, Bildschirmspeicher), ist `< 0x8000_0000`; erzwungen an
  der Abbildungs-/Herausgabegrenze, nicht nur durch Platzierung.
- Hosttest in `tos-api/tests/`: `map_shared` und `expose_attr` mit einem
  Bereich, der `0x8000_0000` beruehrt oder ueberschreitet, werden abgelehnt;
  die letzte zulaessige Seite `[0x7FFF_F000, 0x8000_0000)` wird angenommen
  (Grenzfall).
- Orakelseite: `tosdiff` prueft fuer jedes Feld der Feldklasse "address" in
  jedem Fall auf beiden Seiten `Wert < 0x8000_0000`
  (`docs/TOS-REFERENCE-DUMPER.md`, Feldklassen). Stand 2026-10-26: dafuer muss
  `tosdiff` die Zeigerbreite und Wortreihenfolge jedes Dumps kennen, die das
  Dumpformat nicht traegt — Entscheidung beim Lead. Reichweite dieser Pruefung:
  sie deckt heute nur `ap_3resv` (`global[11..12]`; bei NewTOS eine auf 32 Bit
  gekuerzte Kern-VA, also keine Anwenderadresse) und die MFDB-Zeiger des
  Raster-Dumpers (Adressen im Programmabbild, also Poolspeicher) ab. `Malloc`,
  `Mxalloc`, `Fgetdta` erreicht der Vergleicher erst mit dem GEMDOS-Bereich
  (Aufgabe T7 der Dumper-Spec).
- Bau beider Boards sauber, keine neuen clippy-Warnungen; Tester-Lauf beider
  Boards ohne Regression.

Abhaengigkeiten: — (ADR-028; [ARCH03](ARCH03.md) liefert die Abbildung)

[Gemeinsame Regeln](README.md) gelten.

## Anwenderentscheidung 2026-10-31

Erzwingung geparkt (heute nicht erreichbar). Dazu ein Hosttest, der zusichert,
dass die Speicheraufteilung alle Anwenderadressen unter 2 GiB haelt — eine
Aenderung der Aufteilung soll laut scheitern, statt die Luecke still wieder
zu oeffnen.

## Stand 2026-10-31: Aufteilung unter 2 GiB festgehalten

Umgesetzt nach der Anwenderentscheidung oben; keine Laufzeitaenderung, die
Erzwingung bleibt geparkt. Grenze ist ueberall dieselbe Konstante
`USER_PTR_END` (`tos-api/src/bdos/process/usermem.rs:57@USER_PTR_END`),
keine Kopie der Zahl.

Kern, zur Uebersetzungszeit auf beiden Boards (der Kern laesst sich auf dem
Host nicht binden):

- `TOS_MD_MAX_END <= USER_PTR_END`
  (`kernel/src/bios/machine/layout.rs:376@TOS_MD_MAX_END`).
- Die DRAM-Kappung von `discover` steht jetzt in einer `const fn`
  (`kernel/src/bios/machine/layout.rs:364@ttram_cap_end_for`, aus `discover`
  gerufen `kernel/src/bios/machine/layout.rs:561@ttram_cap_end_for`, gleiche
  Rechnung wie vorher); `ttram_cap_end_for(board::DRAM_START +
  STRAM_SLOT_SIZE) <= USER_PTR_END`
  (`kernel/src/bios/machine/layout.rs:380@ttram_cap_end_for`). Damit sind
  `POOL_MAX_BYTES`, `TOS_MD_MAX_END`, `STRAM_SLOT_SIZE` und
  `board::DRAM_START` gemeinsam gepinnt; alles, was aus DRAM an Programme
  geht (beide Pools, alles, was `usermem::expose` veroeffentlicht: Kernabbild,
  `.dma`, Kern-Heap, Blockcache, qemu-virt-Bildschirmspeicher), liegt unter
  dieser Kappung. Faellt auf qemu-virt auch, wenn jemand das `min` mit
  `TOS_MD_MAX_END` entfernt (`0x4100_0000 + 1 GiB`).
- Pi-Bildschirmspeicher: `BUS_TO_PHYS < USER_PTR_END`
  (`kernel/src/drivers/bcm2711/framebuffer.rs:72@BUS_TO_PHYS`); das Ende des
  Speichers begrenzt der VideoCore-Anteil unter 1 GiB (BCM2711 §1.2.3), nicht
  der Code.

Host, `tos-api/tests/mem11_layout_below_2gib.rs` (3 Tests):

- `limit_is_the_positive_long_range`
  (`tos-api/tests/mem11_layout_below_2gib.rs:44@limit_is_the_positive_long_range`):
  `USER_PTR_END == Long::MAX + 1`, `USER_VA_LIMIT >= USER_PTR_END`.
- `fixed_user_pages_and_pointers_lie_below_2gib`
  (`tos-api/tests/mem11_layout_below_2gib.rs:52@fixed_user_pages_and_pointers_lie_below_2gib`):
  Seite 0, IOREC-, Stub- und private Seite (`IOREC_PAGE_VA`, `STUB_PAGE_VA`,
  `PRIV_PAGE_VA`), die Bereiche `Keytbl`, `Kbdvbase`, `kbdvec`-Stubs,
  Reset-Stub und die herausgegebenen Zeiger (`Iorec` aller drei Geraete,
  `Bconmap`, `ret`-/`kbdvec`-Stub, Reset-Einstieg, Upcall- und
  `G_USERDEF`-Trampolin, `PARMBLK`-Puffer, `os_run`-Zelle).
- `pool_seed_refuses_a_pool_reaching_2gib`
  (`tos-api/tests/mem11_layout_below_2gib.rs:97@pool_seed_refuses_a_pool_reaching_2gib`):
  `bdos::mem::init` nimmt einen Pool bis `USER_PTR_END - 0x1000` (= heutiges
  `TOS_MD_MAX_END`) an und lehnt einen bis `USER_PTR_END` mit
  `AddressOutOfRange` ab (`seed_md`,
  `tos-api/src/bdos/mem/init.rs:226@seed_md`).

Gegenproben (Abschrift, danach von Hand zurueckgestellt): `TOS_MD_MAX_END =
0x8000_1000` → Kernbau scheitert auf rpi4 (eine Zusicherung) und qemu-virt
(beide): `evaluation panicked: MEM11: TOS_MD_MAX_END lets a pool reach
0x8000_0000` bzw. `MEM11: the managed DRAM can reach 0x8000_0000`. Die
`checked_add`-Pruefung in `seed_md` entfernt → `pool_seed_refuses_a_pool_reaching_2gib`
FAILED. Hosttests 1910/0/1 (+3); Clippy Kern 121 / tos-api 74 auf beiden
Boards wie Basis.
