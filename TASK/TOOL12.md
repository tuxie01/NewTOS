# TOOL12 — Abhaengigkeitsgraph Track B: Trap #2 aufloesen, Vektoraufruf-Kanten, Abschlusskriterium

Status: **ERLEDIGT** 2026-10-28 ([Quittung](TOOL12-STATUS-DONE.MD)) · Prioritaet: **P1** · Typ: Developer (Werkzeug) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Der Graph `docs/analysis/deps/edges.tsv` (Generator
  `tools/deps/gen-deps.py`, Abfrage `cargo xtask deps <fn> --down|--up`,
  Commit `329cee07`) hat zwei bekannte Luecken:
  - **Trap #2**: `gem`/`gsx2` haben je eine Kante zu jedem installierten
    Trap-#2-Handler mit Detail `not resolved`; der Schritt je Opcode
    (`crysbind`-Switch der AES, VDI-Tabelle `jmptb1/2`) fehlt
    (`tools/deps/README.md`, Abschnitt „Misses“ der Kantenart `layer`).
  - **Vektoraufrufe**: Funktionen, die EmuTOS ueber einen Systemvektor
    aufruft (z. B. `hdv_rw`/`hdv_bpb`/`hdv_mediach`, installiert in
    `blkdev_init`), haben keine Kante vom Aufrufer zum Handler. `deps lrwabs
    --down` endet deshalb bei `protect_wlwwwl`; die Bottom-up-Bedingung ist
    fuer alle Vektorfunktionen leer erfuellt (BIOS-STOR-009,
    `docs/audits/2026-10-28-bios-storage.md`).
- `docs/analysis/deps/mapping.tsv` fuehrt `blkdev_drvmap`, `blkdev_getbpb`,
  `blkdev_mediach`, `blkdev_rwabs` als „no counterpart“, obwohl `drvmap`,
  `bpb::getbpb`, `mediach`, `rwabs` sie nachbauen (BIOS-STOR-009).
- DUMPGDOS und jede spaetere Lokalisierung brauchen die Kanten.

## Arbeit

1. Trap #2 je Opcode aufloesen: AES ueber den `crysbind`-Switch, VDI ueber
   `jmptb1/2`; Kante Aufrufer → Funktion im Slot, Detail wie die anderen
   Trap-Bindungen. Was nicht konstant aufloesbar ist, steht in `layers.md`,
   nicht geraten.
2. Neue Kantenart (oder Detail) fuer Vektoraufrufe: Installation eines
   Funktionszeigers in einen Systemvektor (`hdv_*`, `etv_*`, `Kbdvbase`-
   Eintraege, Line-A-/Trap-Vektoren soweit im Pin) → Aufrufstellen ueber den
   Vektor. Erkennung aus gimple/Assembler wie bei `callback`; jede Luecke in
   `tools/deps/README.md`.
3. `mapping.tsv`: die vier `blkdev_*`-Zeilen n:m auf die NewTOS-Funktionen;
   alle weiteren Vektorhandler, die die neuen Kanten sichtbar machen, pruefen.
4. Abschlusskriterium belegen: `cargo xtask deps getrec --up` erreicht die
   GEMDOS-Dateiaufrufe ueber die osif-Tabelle (bestehender Hosttest), dazu
   neue Hosttests: `deps lrwabs --down` erreicht `blkdev_rwabs`; ein
   AES-Opcode und ein VDI-Opcode werden je bis zur Funktion aufgeloest.
5. `--check` (bytegleiche Neuerzeugung) besteht nach der Aenderung.

## Abnahme

- Die Hosttests aus Punkt 4 gruen; `gen-deps.py --check` OK.
- `cargo xtask test` gruen (Basislinie 1745/0/1 plus neue); doc-check der
  geaenderten Doku 0 Fehler.
- Bericht: Zahl der Kanten je Art vorher/nachher, verbleibende Luecken.
