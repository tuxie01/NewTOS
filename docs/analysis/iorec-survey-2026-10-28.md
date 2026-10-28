# Iorec-Umfrage fuer XBIOS01 E3/E4 (2026-10-28)

Datierter Schnappschuss, kein Live-Dokument. Bezug: `TASK/XBIOS01.md` (E1-E6),
`docs/api/xbios/0e-iorec.md` (E3/E4), Programmliste wie in
`docs/analysis/os-hooks-adr-proposal-2026-10-28.md`.

## Antworten

- **E3: keiner gefunden.** Kein portiertes Programm ruft `Iorec` auf, also pollt
  keines den Ring (`ibufhd`/`ibuftl`/`ibuf`), weder trap-frei noch sonst.
- **E4: keiner gefunden.** Keine Zuweisung an `ibuf` oder `ibufsiz` in irgendeinem
  durchsuchten Programm.

## Durchsucht (gelesen, nicht abgeleitet)

Muster: `Iorec`, `_IOREC`, `ibufhd`, `ibuftl`, `ibufsiz`, `->ibuf`, `.ibuf`,
`xbios(14`, `xbios(0xe`, `trap #14`, `svc #14`; Endungen `.c .h .cpp .S .s .rs .i`.
Wurzeln: `sdk/` (apps, overlays, examples, libc), `programs/`, `tosdump/core`,
`Applications/` (ohne `.claude/worktrees`, dort nur Duplikate). Nicht
durchsucht: `references/freemint` (Vorgabe).

## Evidenztabelle

| Fundstelle | Art | Klasse | Lesart |
|---|---|---|---|
| `sdk/libc/mintlib/include/mint/osbind.h:625@Iorec` | `#define Iorec(ioDEV)` | nur Deklaration | gelesen |
| `sdk/libc/mintlib/include/mint/ostruct.h:575@Iorec` | `_IOREC`-Struct (`ibufsiz`, `ibufhd`, `ibuftl`) | nur Typdefinition | gelesen |
| `sdk/libc/overlay/include/mint/osbind.h:625@Iorec`, `sdk/libc/overlay/include/mint/ostruct.h:575@Iorec` | Kopie der obigen | nur Deklaration | gelesen |
| `Applications/mintlib-src/mintlib-0.60.1/include/mint/osbind.h:1219`, `ostruct.h` (Struct "Structure returned by Iorec") | Upstream-Original | nur Deklaration | gelesen |
| `sdk/libc/mintlib/syscall/syscalls.master:578` | Syscall-Liste | nur Deklaration | gelesen |
| `sdk/examples/kbdvec.c:143@hook`, `sdk/apps/doom/doomgeneric_gem_kbdvec.c:48@kbdvec_hook` | Parameter `iorec` des kbdvec-Hooks, immer 0 | andere (kein Iorec-Aufruf, kein Ringzugriff) | gelesen |
| `tosdump/core/cases.c:3495@Iorec` | Funktionstabelle `0x0E` fuer Dump | andere (misst Rueckgabe, liest keinen Ring) | gelesen |

Aufrufer (Klasse "liest Ring trap-frei", "liest einmal", "nur Flush"): **keine**.
Dass `ostruct.h` ein Feld `_IOREC *iorec` enthaelt (Zeiger-Member in `ostruct.h`), ist Teil von
`struct bconmap`-artigen Typen, kein Nutzer.

## Grenzen (abgeleitet)

- Abgeleitet: Nur Quelltext und Header wurden durchsucht, keine Binaries
  (`.o`, `.prg`). Ein Programm, das Iorec nur ueber hartkodierten Opcode in
  Assembler ruft, waere ueber `trap #14`/`svc #14` gefunden worden; Treffer nur
  in Kommentaren/Bindungen (`sdk/include/newtos.h` Kopfkommentar,
  `programs/newtos-gem/src/xbios.rs` Modulkommentar), kein Iorec-Aufruf.
- Programmquellen, die nicht lokal liegen (z. B. nur als fertige Binaries),
  sind nicht erfasst.

## Nachtrag (Lead, 2026-10-28): Luecke „kein lokaler Quelltext“

Gegen das Manifest des Standardabbilds geprueft
(`measurements/measurement-2026-10-27-mem12-acceptance/cards/std.img.manifest`,
geschrieben von `tools/make-sd-image.sh`): alle 31 ausfuehrbaren Dateien
(`.NAP`, `.NCP`, `.NAC`) werden von `tools/make-sd-image.sh` aus Quellen im
Baum gebaut (`sdk/apps/`, `sdk/examples/`, `sdk/libc/`, `sdk/libm/`,
`sdk/ldg/`, `sdk/zview/`, `tosdump/`, `tools/mk-hello-ncp.py`). Fuer NewTOS ist
die Luecke damit **per Konstruktion leer**: native Programme gibt es nur als
Neubau aus lokaler Quelle (der Lader nimmt nur ELF64, m68k-Binaerprogramme
laufen erst mit dem spaeteren Emulator). Nicht geprueft: die echte Karte des
Anwenders, deren Belegung nicht aus `make-sd-image.sh` kommt; fuer sie gilt
dieselbe Pruefung gegen ihr eigenes Manifest. Wiederverwendbar fuer kuenftige
Erhebungen: `.claude/agents/researcher.md`, Abschnitt zu Programmerhebungen.
