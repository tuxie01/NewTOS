# `Getmpb` (BIOS 0x00)

> Phase 00 spec (full-contract template). Source: EmuTOS @ `1a7484ab`.
> Verified against `bios/biosmem.c:134` (`getmpb`).

## NewTOS status: **wired at BIOS 0x00** (read-only report; see NewTOS implementation notes)

Historically called exactly once, by GEMDOS init, to discover the ST-RAM
extent. NewTOS initialises its allocator directly from the AArch64 memory
map, so this call is vestigial. Whether to expose a thin MPB-over-free-list
is a `docs/COMPATIBILITY.md` port-vs-reimplement decision, deferred until a
program needs it. The dispatcher's out-of-range fall-through is *not* the
answer — `getmpb` is a real, in-range handler (`bios_vecs[0]`).

> **[BIOS06] 2026-10-28:** Abschnitt "NewTOS-Vertrag" ergaenzt (Layout,
> Inhalt, Pruefung, Abnahme) und "Offene Entscheidungen" (O1-O4). Stand:
> **Vertrag entschieden 2026-10-28 (O1 = A, O2 = (a), O3 = (a) bis zur Orakelmessung, O4 bestaetigt)** — bis dahin ist
> nichts davon umgesetzt. Quellen: `references/emutos/bios/biosmem.c:134-151`
> (gelesen), Kern und SDK-Header (gelesen, Zitate unten), Sonde G00-G06
> (gemessen).

---

## NewTOS-Vertrag [BIOS06]

> **Vertrag vs. Referenz** — `docs/COMPATIBILITY.md` "Native vs m68k struct
> variants" (`docs/COMPATIBILITY.md:1227`): Felder und Bedeutung sind Vertrag,
> die Zeigerbreite folgt der ABI des Aufrufers. Referenz (frei): Adresse und
> Seite des Deskriptors, `themd` bei `$48E`, die Aufteilung in Kern-`MD` und
> Deskriptor. Gelesen = Quelltext/Messung; abgeleitet = Folgerung, so markiert.

### V1 — Layout, das ein NewTOS-Programm sieht (Frage 1)

**EmuTOS (m68k, gelesen):** `MD` = `MD *m_link; UBYTE *m_start; LONG m_length;
PD *m_own` (`references/emutos/include/bdosdefs.h:108-115`), 16 Byte, Offsets
0/4/8/12; `MPB` = drei `MD *` (`include/bdosdefs.h:120-126`), 12 Byte.

**Heute im Kern (gelesen):** `tos_types::MD` = `m_link` (8, Offset 0),
`m_start: Long` (4, Offset 8), `m_length: Long` (4, Offset 12), `m_own` (8,
Offset 16), 24 Byte (`tos-types/src/memory.rs:61@MD`, Tabelle ebd. Modulkopf;
Pruefung `:76`). `MPB` = drei Zeiger, 24 Byte (`tos-types/src/memory.rs:101@MPB`,
Pruefung `:111`).

**Heute im SDK-Header (gelesen):** `_MD` = `struct _md *md_next; long md_start;
long md_length; BASEPAGE *md_owner` (`sdk/libc/overlay/include/mint/ostruct.h:293@_md`),
`_MPB` = drei `_MD *` (`:305@_MPB`). Unter LP64 (`long` = 8 Byte) sind das
`_MD` 32 Byte (Offsets 0/8/16/24) und `_MPB` 24 Byte (0/8/16) — **abgeleitet**
aus den C-Ausrichtungsregeln; gemessen: `sizeof(_MD)=32`, `sizeof(XMPB)=24`
(`measurements/measurement-2026-10-28-bios-core-audit/runs/PROBE-NEWTOS-raspi4b/BCPROB2.LOG`,
G00). Der Header ist byte-gleich zum Upstream-Text
(`Applications/mintlib-src/mintlib-0.60.1/include/mint/ostruct.h:240-255`,
gelesen) — er ist nicht NewTOS-spezifisch.

**Befund (gelesen + gemessen):** `MPB` stimmt auf beiden Seiten (24 Byte,
Offsets 0/8/16). Nur der **MD** weicht ab: der Kern liefert ein Mischformat
(Zeiger 8 Byte, Adresse/Laenge 4 Byte — weder m68k noch LP64), der Header
erwartet LP64. Das Programm liest `md_start` = `m_start | m_length << 32` und
`md_length` = Fuellbytes: gemessen `md_start = 0x00A00000_00400000`,
`md_length = 0` (G03), Rohbytes G05 `00 00 00 00 00 40 00 00 00 A0 00 ...`
sind das Kern-Layout mit den richtigen Werten 0x00400000 / 0x00A00000.
`sdk/apps/apitest/apitest.c:493@MDX` deklariert das Kern-Layout selbst und
prueft `sizeof == 24` (`:507@apitest_assert_mdx`).

**Welche Seite bricht den Vertrag (Folgerung aus COMPATIBILITY:1227):** wenn
der Vertrag "Felder gleich, Zeigerbreite folgt der ABI" lautet, ist das
**Kern-Mischformat** die Abweichung: der Kern hat sein internes
BDOS-`MD` (Felder als `Long`) fuer die Veroeffentlichung wiederverwendet
(`tos-api/src/bios/getmpb.rs:35@ThemdPage`: `md: MD`), ohne dass dieses
Layout je als oeffentlicher Vertrag entschieden wurde. Der unveraenderte
Upstream-Header ist dann die Vertragsseite. Das ist die Folgerung aus der
Regel; die Wahl selbst ist **O1** unten.

**Oeffentliches Layout (O1 = A, Anwender 2026-10-28):**

| Struktur | Offset | Feld | Breite |
|---|---:|---|---:|
| `_MD` | 0 | `md_next` | 8 (Zeiger, EL0-VA) |
| | 8 | `md_start` | 8 (`long`; unter VA = PA ein Wert unter 2 GiB) |
| | 16 | `md_length` | 8 (`long`, Bytes) |
| | 24 | `md_owner` | 8 (Zeiger) |
| | | `sizeof` | 32 |
| `_MPB` | 0 | `mp_free` (`mp_mfl`) | 8 |
| | 8 | `mp_used` (`mp_mal`) | 8 |
| | 16 | `mp_rover` | 8 |
| | | `sizeof` | 24 |

Das interne BDOS-`MD` (24 Byte) bleibt, wie es ist; es ist kein
Programmvertrag (Pools `pmd`/`pmdalt` werden nie veroeffentlicht,
`THEMD` ist nur in `getmpb.rs` benannt — gelesen, `grep THEMD` ueber
`tos-api`/`kernel`: keine weitere Fundstelle).

### V2 — Inhalt, Lebensdauer, Schreibzugriff (Frage 2)

**EmuTOS (gelesen):** `getmpb` fuellt das eine statische `themd` mit
`[membot, memtop)` und setzt `mpb->mp_mfl = &themd`, `mpb->mp_mal = NULL`
(`bios/biosmem.c:144-150`). `mp_rover` bleibt unberuehrt. **Wer es spaeter
liest:** GEMDOS `umem_init` ruft `Getmpb((long)&pmd)` genau einmal und liest
`pmd.mp_mfl->m_start`/`m_length` fuer `start_stram`/`end_stram`
(`bdos/umem.c:545-556`). Danach ist `themd` **lebender GEMDOS-Zustand**:
`pmd.mp_mfl` zeigt auf es, die First-Fit-Zuteilung kuerzt und verlinkt es
(`docs/api/structures/md.md`, "Allocation flow", `bdos/iumem.c:35`). Ein
zweiter `Getmpb`-Aufruf nach dem Start schreibt `themd` neu, waehrend
`pmd.mp_mfl` noch darauf zeigt (abgeleitet aus `bios/biosmem.c:144-147` und
`umem_init`); die Sonde G06 (zweiter Aufruf, gleicher Zeiger) und G03/G04
(`start+length = _memtop`) sind auf EmuTOS deshalb **per Konstruktion** wahr:
das Programm liest, was sein eigener Aufruf gerade geschrieben hat.

**NewTOS (gelesen):** GEMDOS besitzt seine Pools selbst: `bdos::mem::init::init`
seedet `pmd`/`pmdalt` aus der Speicherkarte
(`tos-api/src/bdos/mem/init.rs:192@STRAM_START`; `docs/api/bios/00-getmpb.md`
"Pi / BCM2711 binding"), `Getmpb` wird von GEMDOS nie gerufen. `getmpb` ist
ein **Bericht**: `(start, end) = stram_range()`
(`tos-api/src/bios/getmpb.rs:89@getmpb`; `stram_range` liefert die einmal
beim Pool-Seeding gesetzten Werte, einzige Schreibstelle
`tos-api/src/bdos/mem/init.rs:192@STRAM_START`), schreibt in den statischen `THEMD`
`m_link = NULL`, `m_start = start`, `m_length = end - start`, `m_own = NULL`
(`tos-api/src/bios/getmpb.rs:94-99@m_length`). Daraus folgt der Vertrag:

1. **Inhalt = Schnappschuss des Anfangsumfangs des ST-RAM-Pools**
   (`[membot, memtop)` zur Bootzeit), **nicht** die lebende Freiliste. Ein
   einziger MD, `md_next == NULL`, `md_owner == NULL`; `mp_used == NULL`
   (`mp_mal`) immer; TT-RAM nie enthalten (wie EmuTOS,
   `bios/biosmem.c:144-147`: nur `membot..memtop`). Wer den freien Rest sucht,
   fragt `Malloc(-1)`/`Mxalloc`. Gemessen: `md_start = membot`,
   `start+length = memtop` auf beiden Seiten (G03/G04; NewTOS
   `membot = 0x00400000`, `memtop = 0x00E00000`, G00).
2. **Lebensdauer/Stabilitaet:** ein statischer Deskriptor, dieselbe Adresse bei
   jedem Aufruf (G06 gemessen: `same_free=1`), die Werte aendern sich nach dem
   Boot nie (abgeleitet: einzige Schreibstelle der Quelle ist `init`). Er wird
   bei **jedem** Aufruf mit denselben Werten neu geschrieben und ist deshalb
   harmlos; er ueberlebt jedes Programm (`'static`). Anders als auf EmuTOS
   zerstoert ein zweiter Aufruf **nichts** (Abweichung zu EmuTOS, in
   `docs/api/bios/00-getmpb.md` "NewTOS implementation notes" bereits
   so festgelegt: Verhaltensrichtung "harmlos").
3. **Schreibbarkeit:** der Deskriptor ist fuer das Programm **nur lesbar**
   (FreeMiNT `PROT_PR`, `tos-api/src/bios/getmpb.rs:112@expose`; `UserRoNx`,
   `tos-api/src/bdos/process/usermem.rs:245@flags`). Ein Schreiben ist ein
   Speicherfehler, wie auf EmuTOS (`themd` in geschuetztem Speicher). Der
   **Lesezugriff im Benutzerzustand** ist auf NewTOS erlaubt; auf EmuTOS
   ein Busfehler (`themd` bei `$48E`; `docs/audits/2026-10-28-bios-core.md`
   BIOS-CORE-003 "Messnebenbefund") — ein Programm, das den MD wie ein
   TOS-Programm in `Supexec` liest, funktioniert auf beiden Seiten.
4. **Die Seite:** der `THEMD` besitzt eine eigene 4-KiB-Seite
   (`tos-api/src/bios/getmpb.rs:35@ThemdPage`, Pruefung `tos-api/src/bios/getmpb.rs:48@ThemdPage`); die Veroeffentlichung ist
   je Prozess idempotent (`tos-api/src/bdos/process/usermem.rs:204@expose`). Scheitert sie, bleibt
   `mp_mfl = NULL` (leere Freiliste, `tos-api/src/bios/getmpb.rs:109-114@expose`).
5. **Rueckgabewert:** keiner (EmuTOS `void`); der Kern liefert `0`.
6. **Caller-Puffer:** geschrieben werden **nur** `mp_mfl` (Offset 0) und
   `mp_mal` (Offset 8), je 8 Byte, in dieser Reihenfolge
   (`tos-api/src/bios/getmpb.rs:125-128@put_u64`); `mp_rover` (Offset 16) wird nicht angefasst
   (wie `bios/biosmem.c:149-150`). Gemessen: G02 `mp_rover_kept=1`,
   `mp_used_null=1` auf beiden Seiten. Der Puffer muss mindestens 16 Byte
   schreibbar sein, nicht die vollen 24.

### V3 — Was der Kern prueft (Frage 3, MEM12)

- **gelesen:** `mpb == 0` -> der Aufruf endet still mit `0`, ohne zu schreiben
  (`tos-api/src/bios/getmpb.rs:86@getmpb`). EmuTOS prueft nichts und schreibt auf Adresse 0
  (`bios/biosmem.c:149`, kein Guard). Ob NULL still bleiben oder fehlen soll
  ist **O3**.
- **gelesen:** die Schreibzugriffe laufen ueber `args.space.put_u64`
  (`tos-api/src/bios/getmpb.rs:127@put_u64`), also mit den Rechten des Aufrufers
  (`tos-api/src/uaccess.rs:352@put_u64`; ARCH03 3a). Ist `mp_mfl` oder `mp_mal`
  nicht schreibbar, endet der Aufruf mit `uaccess::FAULTED`
  (`tos-api/src/bios/getmpb.rs:130@FAULTED`, `tos-api/src/uaccess.rs:55@FAULTED`); die Kommentare im
  Quelltext nennen "ends the call and the program" (nicht in diesem Lauf
  verfolgt: wie der Dispatcher `FAULTED` in einen Programmabbruch uebersetzt
  — `docs/api/_super-state.md` S-Abschnitt ist die Stelle, falls noetig).
  Ein Fehlschlag nach dem ersten Schreiben laesst `mp_mfl` bereits gesetzt
  und `mp_mal` unveraendert (abgeleitet aus der Reihenfolge).
- **gelesen:** seitenweise Pruefung gegen MEM12: der Kern liest von Seite 0
  nichts; `Getmpb` liest keine Systemvariable (`stram_range()` ist ein
  Kernel-Atomic, nicht `$432/$436`). Die Systemvariablen `membot`/`memtop`
  in Seite 0 sind eine **Ausgabe** (`docs/api/_super-state.md` Abschnitt 0:
  "the kernel treats page 0 as output only").
- Nicht geprueft wird die Ausrichtung des Puffers (`put_u64` ist
  ausrichtungsfrei, `uaccess.rs:352@put_u64`; abgeleitet: die Kommentare an
  `get_u64` nennen "any alignment").

### V4 — Pruefung zur Uebersetzungszeit (Frage 4, wie IOREC)

Wenn O1 = A: in `ostruct.h` (SDK-Overlay), `#if defined(__LP64__)`:
`_Static_assert(sizeof(_MD) == 32)`, `offsetof(_MD, md_next) == 0`,
`md_start == 8`, `md_length == 16`, `md_owner == 24`, `sizeof(_MPB) == 24`,
`offsetof(_MPB, mp_free) == 0`, `mp_used == 8`, `mp_rover == 16`; im Kern die
gleichen Zahlen als `const _: () = assert!(offset_of!(..) == ..)` an dem
oeffentlichen Deskriptor (und `size_of`). `apitest.c:493@MDX` entfaellt dann
(der Test nimmt `_MD` aus dem Header). Bei O1 = B/C waeren es die Zahlen der
gewaehlten Variante (siehe unten). **Heute nirgends vorhanden** (gelesen:
`ostruct.h` hat keine `_Static_assert`; der Kern prueft nur
`size_of::<MD>() == 2*usize+8`, `tos-types/src/memory.rs:76@size_of`, das das Mischformat sogar
festschreibt).

### V5 — Nutzer (Erhebung)

Siehe `docs/analysis/lp64-tos-struct-survey-2026-10-28.md` (Zeile "MD/MPB")
und die Erhebung dieses Tickets: einziger Leser von `md_start`/`md_length` ist
`sdk/apps/apitest/apitest.c:2176@MDX` (B3, BS03-BS10). `sdk/examples/uakid.c`
ruft `Getmpb(BAD)` nur mit ungueltigem Zeiger (kein Lesen des MD, nicht im
Standard-Image); `tosdump` misst `Getmpb` seit TOOL16 nur in
`tosdump/cases/xbios-ptr-r1.tsv` (in einer `Super`-Klammer, als letzten
Aufruf; `tosdump/core/cases.c:3786@CR_GETMPB`,
`tosdump/cases/COVERAGE-BIOS.md:58@Getmpb`), MPB und MD feldweise mit dem
Layout aus `docs/api/structures/md.md` ("Dumper layout"). Kein
portiertes Programm unter `sdk/apps/`, `sdk/overlays/`, `sdk/examples/`,
`programs/`, `Applications/` ruft `Getmpb` im Quelltext (Binaerdateien:
`sdk/apps/highwire/modules/network.src/socklib/mintlib.lib` enthaelt nur den
Namen, vorgebauter m68k-Bestand, gitignored; kein Aufruf-Nachweis). Der
Fehler trifft heute **kein** portiertes Programm; er trifft jedes kuenftige
Speicher-Infoprogramm und die Sonde G00-G06.

### Offene Entscheidungen (Anwender) [BIOS06]

Keine davon ist entschieden; der Rest dieses Abschnitts ist Folgerung.

**Entscheidungen (Anwender 2026-10-28):** O1 = **A** (LP64-Header-Layout,
eigener oeffentlicher Deskriptor im Kern); O2 = **(a)** statischer
Anfangsschnappschuss; O3 = **(a)** stiller Erfolg bleibt, bis das Verhalten von
EmuTOS bei `mpb == NULL` auf dem Orakel gemessen ist; O4 bestaetigt (Sonde
G00–G06 als dauerhafter Test bis TOOL16). Die Optionen bleiben unten als
Begruendung.

**O1 — Welches Layout ist der Vertrag?**
- **A — LP64-Header-Layout** (32/24 Byte, alle Felder 8 Byte). Der Kern
  veroeffentlicht einen eigenen oeffentlichen Deskriptor (neue Struktur in
  `getmpb.rs`, `THEMD` nutzt sie; das interne BDOS-`MD` bleibt). Der
  Upstream-Header bleibt unveraendert (ADR "Upstream unchanged"); `apitest`
  verliert `MDX`/`MPBX`. Kosten: Kern-Aenderung in `getmpb.rs` + Hosttest;
  folgt dem IOREC-Vorbild F1 ("Kern liefert das Layout des SDK-Headers").
  Ein spaeterer m68k-Thunk braucht dann die Umsetzung 32 -> 16 Byte (wie bei
  OBJECT).
- **B — Kern-Mischformat (24 Byte) behalten, Overlay-Header aendern**
  (`md_start`/`md_length` als 4-Byte-Typen). Kern unveraendert; dafuer weicht
  der Header vom Upstream-Text ab (Patch in `sdk/libc/overlay/`), und
  `md_start` ist kein Zeiger mehr. Kosten: Ausnahme in COMPATIBILITY noetig
  (Begruendung wie `AESGLOBAL`: dort gibt es einen Wortindex-Vertrag; fuer
  `MD` gibt es keinen — gelesen, `docs/COMPATIBILITY.md:1242-1257` — das
  Argument fehlt also).
- **C — m68k-Drahtformat auf jeder ABI** (16/12 Byte, alles 4 Byte). Verlangt
  auch einen 12-Byte-`_MPB` im Header und 4-Byte-Zeiger in `md_next`/`md_owner`;
  bricht den heutigen `_MPB` (24 Byte, in beiden Seiten gleich). Kosten:
  Header-Patch beider Strukturen, Kern schreibt 4-Byte-Zeiger. Nur sinnvoll,
  wenn man Programme gegen den m68k-Header ohne Neukompilation laufen lassen
  wollte (das Ziel ist der m68k-Thunk, nicht jetzt).

**O2 — Inhalt:** (a) statischer Anfangsschnappschuss (heutiger Code,
EmuTOS-gleich in dem, was ein Programm per `Getmpb` bekommt) — keine
Aenderung; (b) die lebende Freiliste/Allokationsliste des BDOS-Pools
veroeffentlichen. **Achtung:** `docs/COMPATIBILITY.md:820` behauptet, ein
Programm finde per `Getmpb` die Bloecke eines TSR auf der Allokationsliste —
der Code (`mp_mal = 0`, ein MD) tut das nicht (gelesen, `tos-api/src/bios/getmpb.rs:128@put_u64`).
Entweder muss COMPATIBILITY korrigiert werden (a) oder (b) gebaut werden (grosse
Aufgabe: BDOS-MDs sind Kern-intern, Seitenrechte, Konsistenz unter dem
Giant-Lock). EmuTOS zeigt nach dem Start ebenfalls nur den einen
Anfangs-MD (`themd`), nicht den lebenden Zustand eines Programms (Folgerung
aus `bios/biosmem.c:144-147`).

**O3 — `mpb == NULL`:** (a) stiller Erfolg ohne Schreiben (heute, `tos-api/src/bios/getmpb.rs:86@getmpb`);
(b) der Aufruf fehlschlagen lassen wie ein nicht schreibbarer Puffer
(`FAULTED`). EmuTOS: undefiniert (schreibt auf Adresse 0). Beobachtbar nur
fuer fehlerhafte Programme; (b) ist die strengere Richtung.

**O4 — Abnahme der Inhalte:** Ticket BIOS06 verlangt tosdiff erst nach
TOOL16; bis dahin die Sonde G00-G06 als dauerhaften Test (beidseitig, Layout
aus O1). Keine Entscheidung noetig, nur Bestaetigung.

---

## EmuTOS source

**Primary location:** `bios/biosmem.c:134` — `void getmpb(MPB *mpb)`
(the exported `getmpb` symbol; `bios/bios.c:895` `bios_0()` is only a
`DBGBIOS` debug wrapper and is compiled out in the default build).

**Helpers called:** none. The function is a straight-line field fill.

**Headers / type definitions used:**
- `include/bdosdefs.h:108` — `MD` (memory descriptor) definition
- `include/bdosdefs.h:120` — `MPB` (memory partition block) definition
- `include/tosvars.h:83` — `extern struct _md themd;`

**System variables / globals read:**
- `membot` (`bios/biosmem.c:145`, TOS var `0x432`) — bottom of TPA (free ST-RAM)
- `memtop` (`bios/biosmem.c:146`, TOS var `0x436`) — top of TPA
- (`end_os`, `0x4fa`, is set to `membot` in `balloc_stram`; `getmpb` reads
  `membot` after all `balloc_stram` calls have settled)

**System variables / globals written:**
- `themd` (`bios/biosmem.c:144-147`) — the single static BIOS `MD`
- `*mpb` (`bios/biosmem.c:149-150`) — caller-supplied struct
- `bmem_allowed = FALSE` (`bios/biosmem.c:137`) **only under `DBG_BALLOC`** —
  the "no more BIOS allocation past this point" guard

**Build configuration affecting this function:**
- `DBG_BALLOC` (default `0`, `bios/biosmem.c:15`) — when set, `getmpb`
  additionally clears `bmem_allowed` so any later `balloc_stram` panics.
  In the default build the body is only the six field stores.

---

## Source extraction summary

`Getmpb` reports the initial free ST-RAM region to GEMDOS as a Memory
Partition Block. It fills a single static memory descriptor `themd`
describing the whole free region `[membot, memtop)`: `m_link = NULL` (only
one descriptor), `m_start = membot`, `m_length = memtop - membot`,
`m_own = NULL` (unowned). It then points the caller's MPB free list at
`themd` (`mp_mfl = &themd`) and empties the allocated list (`mp_mal = NULL`).

Note what it does **not** touch: `mp_rover` (the third MPB field) is left
exactly as the caller supplied it. `getmpb` writes only `mp_mfl` and
`mp_mal`. (`mp_rover` is documented as "roving pointer — no longer used" in
`include/bdosdefs.h:125`.)

`membot`/`memtop` are set by `bmem_init()` (`bios/biosmem.c:51`) at boot:
`membot = end_os` (end of OS static data), `memtop = phystop` (top of
ST-RAM), then adjusted downward/upward by every `balloc_stram()` call during
BIOS init. By the time GEMDOS calls `Getmpb`, they bound the final free
region.

---

## Trap interface

- **Trap number:** 13 (BIOS)
- **Function ID:** 0x00
- **Stack frame on entry (m68k):**
  ```
  +2: mpb (LONG, MPB *)
   0: function_id (word) = 0x0000
  ```
- **Total parameter bytes:** 6 (word ID + LONG pointer)
- **Native (AArch64) ABI:** function ID `0x00` in `x8`; the single pointer
  argument in `x0`. `getmpb` is `void` — the dispatcher leaves whatever is in
  `x0` (classically `d0`); programs must read the populated `*mpb`, never the
  return value.

  | m68k stack | AArch64 reg | Name  | Type / size |
  |------------|-------------|-------|-------------|
  | +2         | `x0`        | `mpb` | `MPB *` (pointer) |

---

## Inputs

| Position | Name  | Type    | Size | Source location      | Notes |
|----------|-------|---------|------|----------------------|-------|
| arg0     | `mpb` | `MPB *` | 4/8  | `bios/biosmem.c:134` | caller-owned, writable 12-byte (m68k) MPB. **Not validated**: `NULL` is dereferenced. |

`getmpb` performs no input validation whatsoever.

---

## Outputs

### Return value

- **Register:** D0 (m68k) / `x0` (native)
- **Type:** none — `getmpb` is declared `void`
- **Encoding:** undefined; the result is delivered entirely through `*mpb`.
- **Source location:** `bios/biosmem.c:134` (no `return` statement)

### Side effects on global state

- `themd` rewritten (`bios/biosmem.c:144-147`): `m_link=NULL`,
  `m_start=membot`, `m_length=memtop-membot`, `m_own=NULL`.
- `*mpb` written (`bios/biosmem.c:149-150`): `mp_mfl=&themd`, `mp_mal=NULL`.
  `mp_rover` is **not** written.
- Under `DBG_BALLOC` only: `bmem_allowed=FALSE`.

The MPB after the call:

| Field      | Value                          | Written? |
|------------|--------------------------------|----------|
| `mp_mfl`   | `&themd`                       | yes      |
| `mp_mal`   | `NULL`                         | yes      |
| `mp_rover` | (caller's prior value)         | **no**   |

The MD `themd` after the call:

| Field      | Value               |
|------------|---------------------|
| `m_link`   | `NULL`              |
| `m_start`  | `membot`            |
| `m_length` | `memtop - membot`   |
| `m_own`    | `NULL`              |

### Side effects on hardware

None.

---

## Error conditions

| Condition | Source location | Returned value |
|-----------|-----------------|----------------|
| (none — the function cannot fail) | `bios/biosmem.c:134` | n/a |

`getmpb` has no error path. A `NULL` or non-writable `mpb` faults; EmuTOS
does not guard it.

**Error paths not in Compendium:** none (Compendium also documents no error).

---

## Verification criteria

- **Nominal:** given a writable `MPB` at `mpb`, after the call
  `mpb->mp_mfl` points at a single `MD` with `m_link == NULL`,
  `m_start == membot`, `m_length == memtop - membot`, `m_own == NULL`, and
  `mpb->mp_mal == NULL`.
- **`mp_rover` untouched:** pre-load `mpb->mp_rover` with a sentinel; after
  the call it is unchanged (`getmpb` writes only `mp_mfl`/`mp_mal`).
- **Error:** none defined. (A conforming NewTOS `Getmpb` need not fault on
  `NULL`; matching EmuTOS's unguarded dereference is not required — it is a
  scarcity-era omission, not observable contract.)

---

## Edge cases discovered in source

1. **Single static MD, shared across calls.** `themd` is one static `MD`
   (`bios/biosmem.c:144`). A second `Getmpb` call after GEMDOS init rewrites
   `themd` from the *current* `membot`/`memtop` and replaces the free list
   with one descriptor — discarding all of GEMDOS's allocation state. EmuTOS
   does not protect against this; the call is meant to run exactly once.

2. **`mp_rover` deliberately preserved.** Unlike `mp_mfl`/`mp_mal`, the
   third MPB field is never written, so the caller's value survives. (Old
   docs correctly flagged this; the source confirms it.)

3. **ST-RAM only.** `Getmpb` reports `[membot, memtop)` = ST-RAM. Alternate
   (TT-)RAM is added to the BDOS pool separately via `Maddalt`
   (`altram_init`, gated by `CONF_WITH_ALT_RAM`), never through `Getmpb`.

4. **The "two-bank MPB" documentation fiction.** Some Atari docs describe
   `Getmpb` returning ST and TT MPBs. EmuTOS fills exactly the one MPB the
   caller points at; there is no second bank.

---

## Dependencies

### Requires (must be initialized / functional)

- `bmem_init()` (`bios/biosmem.c:51`) — sets `membot`/`memtop`/`end_os`.
- All BIOS `balloc_stram()` allocations complete (final `membot`/`memtop`).
- `themd` (static, in BIOS BSS).

### Used by

- GEMDOS `osinit()` — the one caller, at boot, to seed the free-memory pool.

---

## Related data structures

- `docs/api/structures/md.md` — `MD` memory descriptor (16 bytes m68k)
- `docs/api/structures/basepage.md` — TPA context
- `MPB` (`include/bdosdefs.h:120`): three pointer fields `mp_mfl`, `mp_mal`,
  `mp_rover` (12 bytes on m68k). Note the struct name in source is
  **Memory _Partition_ Block**, and `mp_rover` is "no longer used".

---

## Cross-references

### Atari Compendium

- Not present in `references/` (no Compendium PDF). Cross-check deferred.
  Historically documents populate-MPB-and-return-void.

### Atari Profibuch

- Not present in `references/`. Cross-check deferred.

### Atari ST Internals

- Not present in `references/`.

---

## Configuration / version notes

- `DBG_BALLOC` (debug builds only) adds the `bmem_allowed=FALSE` guard.
- Configuration-independent in release builds across all TOS versions.

---

## NewTOS implementation notes

(Empty during Phase 00.)

PHASE-03: NewTOS initialises the GEMDOS allocator directly from the AArch64
device-tree / memory map, so `Getmpb` is vestigial. If exposed at all it
should be a read-only report of the ST-RAM-equivalent pool; calling it
post-init must **not** reproduce EmuTOS's "destroy your free list" behaviour.

---

## Pi / BCM2711 binding

> Re-points the free-RAM discovery from the **`membot`/`memtop` low-memory
> globals** (set by `bmem_init` + `balloc_stram` on ST-RAM) to the **AArch64
> memory map** discovered from the device tree at boot. The MPB/MD layout is
> the contract; `Getmpb` itself is vestigial on the Pi. See
> `docs/HARDWARE-PROFILE.md` §3.6, §3.8.

### The allocator is seeded from the memory map, not from `Getmpb`

NewTOS builds its pools directly: `kernel/src/bios/machine/layout.rs::discover`
computes the ST-RAM (`pmd`) and TT-RAM (`pmdalt`) extents from the DTB memory
node (or the board fallback), and `kernel/src/lib.rs:452-481` hands those ranges
to `bdos::mem::init::init`. GEMDOS never calls `Getmpb` to learn the free region
— the classic "one caller, at boot" role is filled by the memory-map path.

| Atari reference | Pi-4 model today |
|-----------------|------------------|
| `membot`/`memtop` from `bmem_init` + `balloc_stram` (ST-RAM only) | ST-RAM pool `[kernel_end, DRAM_START + 14 MiB)` from the memory map (`layout.rs:310-313`) |
| `themd` single static MD over `[membot, memtop)` | the `pmd` MPB's seed MD, plus a separate `pmdalt` for TT-RAM (`Getmpb` never reported TT-RAM) |
| GEMDOS `osinit` calls `Getmpb` once | GEMDOS pools seeded from the AArch64 map before any trap runs |

### Current-code gaps (Step-2 candidates)

- **Wired since — correction (2026-09-05).** An earlier revision of this
  section said "`Getmpb` (BIOS 0x00) is not wired in the BIOS dispatcher …
  a call hits the BIOS catch-all". That is no longer true: `mod.rs` dispatches
  `GETMPB => getmpb::getmpb(args)` and `tos-api/src/bios/getmpb.rs` fills the
  caller's MPB as a **read-only** report of the ST-RAM pool extent — it does
  *not* rewrite the live free list, so EmuTOS's "second call destroys your
  allocator state" quirk stays rejected. Measured from EL0 on QEMU `raspi4b`:
  `mp_mal == NULL`, `mp_rover` untouched, `mp_mfl` non-NULL (`APITEST` checks
  BS03..BS05 PASS).
- **`mp_mfl` pointed at a kernel address an EL0 program could not read —
  MEASURED 2026-09-05, CLOSED the same day (ADR-023).** The MD the free-list
  head points to is the kernel static `THEMD` (`getmpb.rs`), at a high-half VA
  (`0xFFFF_0000_0000_0000 + physical`,
  `kernel/src/arch/aarch64/link-rpi4.ld:24`) whose pages are EL1-only. An
  application therefore got a free-list head it could not dereference:
  measured `mp_mfl == 0xFFFF00000026D8B8` (`APITEST` check BS06 FAIL; the MD
  field checks BS07..BS10 unrunnable from EL0).

  **Now:** `THEMD` occupies a page of its own and is published read-only —
  FreeMiNT `PROT_PR` (`references/freemint/sys/mint/mem.h:133`) — via
  `bdos::process::usermem::expose`; `mp_mfl` carries the resulting **user
  address** (ADR-023, same mechanism as `Getbpb`,
  `docs/api/bios/07-getbpb.md`; since ARCH03 stage 1 the descriptor's own
  physical address in the shared user address space, VA = PA). An in-kernel
  (EL1) caller still gets the kernel VA. If the publication fails, `mp_mfl` is
  left NULL — an empty free list, which this contract already defines —
  rather than a pointer the caller would fault on. Measured after the change
  on QEMU `raspi4b`: `mp_mfl = 0x00402000`, `APITEST` BS03..BS10 all PASS
  (`m_link == NULL`, `m_own == NULL`, `m_start == membot $432 == 0x400000`,
  `m_length == memtop-membot == 0xA00000`).
