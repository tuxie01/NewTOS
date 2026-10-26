# MEM11 — Anwenderadressen < 2 GiB nur zufaellig: die Abbildungsgrenze erlaubt 4 GiB

Status: **OFFEN** (Debug-Pruefung erledigt 2026-10-26, Erzwingung offen) · Prioritaet: **P2** · Typ: Kernfehler (Vertragsgrenze nicht erzwungen) — Anwenderbefund 2026-10-26

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
  `tos-api/src/bdos/fs/find.rs:148-153` (`fgetdta`, nur `Space::User` — die
  DTA des Kern-Aufrufers ist eine Kern-VA). Ausgenommen: die `-1`-Abfrage von
  `Malloc`/`Mxalloc` (Groesse, kein Zeiger); `0` (Fehlschlag) besteht.
  Release-Bauten uebersetzen die Pruefung weg.
- [x] Hosttests: `tos-api/tests/mem11_user_ptr_assert.rs` (Grenzfall
  `0x7FFF_FFFF` besteht, `0x8000_0000` loest im Debug-Bau aus; `-1`-Abfragen
  und echte Bloecke ueber die Trap-Eintraege; `Fgetdta` mit `p_xdta` unter /
  auf 2 GiB) und `tos-api/src/gemdos/memory.rs` `mod tests` (die
  `-1`-Ausnahme mit einem Groessenwert >= 2 GiB).
- Hinweis: `Fsetdta` nimmt die DTA des Programms ungeprueft an; ein Programm,
  das eine DTA >= `0x8000_0000` setzt, loest die Pruefung beim naechsten
  `Fgetdta` im Debug-Bau aus (Kernpanik nur im Debug-Bau).

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
