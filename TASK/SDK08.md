# SDK08 — SDK-Paritaetstests scheitern nach dem clang-Update

Status: **OFFEN** · Prioritaet: **P2** · Typ: Developer (Werkzeug/SDK) — Anwenderentscheidung 2026-10-30, Fund aus [COOK02](COOK02.md) (`_IDT`-Lauf)

## Sachverhalt

- `sdk/run-host-tests.sh` steht bei 33/2/0 statt 35/0/0. Es scheitern
  `rt-host` (`sdk/compiler-rt/tests/rt-host.sh`) und `ldbl-host`
  (`sdk/libc/tests/ldbl-host.sh`), nur an der Neubau-Paritaet: 407 von 407
  bzw. 201 von 202 Archivteilen weichen ab. Alle Funktionspruefungen darin
  sind gruen.
- Ursache laut Developer: der Host-clang ist jetzt `Apple clang 21.0.0
  (clang-2100.4.1.1)`; die ausgelieferten Archive `libclang_rt.builtins.a` und
  `libc.a` tragen in `.comment` `clang-2100.3.34.2`.

## Vorgabe (Anwender, 2026-10-30)

- Im Vergleich **nur** den Abschnitt mit der Compiler-Kennung ausblenden
  (`.comment` bzw. `ident`). Code- und Datenabschnitte muessen byte-gleich
  bleiben. Weicht Code oder Daten ab, ist das ein echter Befund: melden, nicht
  wegfiltern.
- Die clang-Version in jedem `RUNINFO.TXT` und im Abbild-Manifest festhalten
  (`tools/sd-manifest.sh`).

## Abnahme

- SDK-Hosttests 35/0/0.
- Gegenprobe: eine absichtlich veraenderte Code-Section faellt im Vergleich
  weiterhin auf.

[Gemeinsame Regeln](README.md) gelten.
