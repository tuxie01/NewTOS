# SDK01 — SDK fuer Fremde benutzbar machen: libc-Luecken und Portierungs-Doku

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: 2a8ef701c | Missing: acceptance

Migrated (owner batch 16, 2026-11-11, to IMPLEMENTED), evidence: commit 2a8ef701c (feat(sdk): take LLVM tools from PATH, Rust only as fallback (SDK01 K)), VERIFIED: diff of 2a8ef701c (feat(sdk): take LLVM tools from PATH, Rust only as fallback ) touches sdk/README.md, sdk/toolchain.sh, names the id by subject, 115 files, legacy line: A–F, H, I, J ERLEDIGT 2026-10-21 (G gestrichen, L erledigt 2026-10-22, K umgeset

Line 3 before the migration:
Status: **A–F, H, I, J ERLEDIGT** 2026-10-21 (G gestrichen; L erledigt 2026-10-22; K umgesetzt, Abnahme Linux/Homebrew offen) · Prioritaet: **P2** · Typ: SDK / Doku — Anwender 2026-10-20 („es gibt Leute, die Apps portieren wollen, dazu braucht es eine Doku“; GitHub-Issue #4)

Grundlage: `docs/analysis/sdk-inventory-2026-10-20.md` (`a36b3280`).

## Pakete (Reihenfolge)

| # | Paket | Groesse | Stand |
|---|---|---|---|
| A | libc-Luecken: `pwdgrp/` in `DIRS` (tmpfile/mkstemp/glob/getlogin …), `__sys_fstat` (ttyname/ctermid), `SIZE_MAX`/`PTRDIFF_MAX`/`SSIZE_MAX` fuer LP64, `ucontext.c`-Testprogramm und Generator-Objekte aus `libc.a`, `.stabs`-Dateien (gets/getwd/siggetmask), `newtos-arch/nf_ops.c` — dazu `tmpfile()` auf NewTOS in QEMU pruefen | S–M | **erledigt** 2026-10-20 (64 → 12 nicht linkende Funktionen; Tester PASS mit allen Apps) |
| B | Doku ehrlich machen: Patch-Aussage („App-Quellen unberuehrt“) in `docs/sdk/README.md`, Kap. 04, 08, 09 §5, `sdk/README.md`, Kopf von `build-qed.sh`/`build-teradesk.sh`; ~12 veraltete Fakten (08, 09, Zahlen in 01/04/07, TeraDesk-`main.c`-Overlay fehlt) | S–M | **erledigt** 2026-10-20 (`57a71b80`, Rest in Skripten/ADR-019 nachgezogen) |
| C | Schnellstart Mac **und** Linux von null: Installation, Bezugsquellen der App-Quellen (freemint/qed `f12055f`, freemint/teradesk `b5532b8`), eigenes Programm ohne Kernelbau auf ein Release-Abbild, Liste nicht linkender Funktionen | M | **erledigt** 2026-10-21 (`docs/sdk/00-quickstart.md`, macOS im frischen GitHub-Klon durchgespielt; Linux ungetestet) |
| D | Compiler-Wrapper `newtos-cc` bzw. `newtos.mk` + Vorlagenprojekt | M | **erledigt** 2026-10-20 (byte-identisch zu den Build-Skripten; Linux ungetestet) |
| E | libm: compiler-rt-Builtins fuer 128-Bit-Gleitkomma und komplexe Multiplikation (`powf`, `remainder` …) | M | **erledigt** 2026-10-21 (libm 21 → 3, libc 12 → 7 nicht linkend) |
| J | `long double` in libc: `vfprintf` `%Lf` (libc mit `-D__NO_LONG_DOUBLE_MATH`) → `qecvt/qfcvt/qgcvt` liefern falsche Ziffern; OpenLibm-ld128-Funktionen (`scalbnl` → `scalbln*`) bauen; `<fenv.h>`-Include ueber `newtos-cc -lm` | M | erledigt (36aacf9a) |
| F | `stat`/`fstat`-Zeitstempel aus der DTA wie Upstream-mintlib | M | **erledigt** 2026-10-21 |
| G | Linux-Weg fuer das SD-Abbild (`make-sd-image.sh` nur macOS) | M | **gestrichen** 2026-10-21 (Anwender: fuer das SDK nicht noetig — `make-sd-image.sh` ist das Testabbild fuer die Arbeit am Kern; Portierer nutzen den plattformneutralen Weg aus dem Schnellstart. Notiz fuer spaeter, falls jemand unter Linux am Kern mitentwickelt) |
| H | **Abnahme durch Portieren:** nach B–D ein bisher nicht portiertes Atari-Programm nur mit der neuen Doku portieren (Anwender 2026-10-20: „weitere Apps kompilieren“); jede Luecke, auf die man stoesst, geht zurueck in Doku/SDK. Kandidaten: Thing, das neuere zView, HighWire (ohne Netz eingeschraenkt), weitere bekannte Atari-Programme | M–L | erledigt (52eeb271): HypView portiert, 15 Dokuluecken (12 behoben, G2/G9/G14 offen); Folgearbeit HypView: AV/BubbleGEM-Nachrichtenzeiger, `.REF`, Bilder |
| K | **Rust raus aus dem C-Weg:** `sdk/toolchain.sh`/`newtos-cc` nehmen zuerst `ld.lld`, `llvm-ar`, `llvm-nm` usw. aus dem PATH (Distributionspakete `clang lld llvm`, macOS `brew install llvm lld`), die gepinnte Rust-Sysroot nur als Rueckfall. Rust brauchen dann nur Kern-Entwickler. Abnahme: dasselbe Programm mit Paket-lld und `rust-lld` gelinkt ist byte-gleich oder erklaerbar verschieden; Bau unter Linux x86_64 (Debian-Container) inkl. Frage, ob clang 14 (LMDE 6) reicht. Anlass: GitHub #4 (ragnar76, LMDE x86_64), Anwender 2026-10-21: „wieso braucht er rust?“ — Anwender 2026-10-22: umsetzen (ragnar76 hat inzwischen gebaut, Rueckmeldung in #4) | M | **umgesetzt 2026-10-22** (`2a8ef701`, byte-gleich 64/64 mit Rust-Rueckfall und ohne rustc; offen: Paket-lld anderer Version, Homebrew, Linux/clang 14 — fehlende Software). Linux-Abnahme geht an ragnar76 (Anwender 2026-10-22, kein Container hier): `sdk/host-report.sh` + `docs/sdk/17-linux-test.md`, Mac-Referenzlauf frisch 24/0/2, 2 min |
| L | **Linux-Infos in die Doku** (`00-quickstart` §1.1/§5.2): unter x86_64 kein `aarch64-linux-gnu-gcc` noetig, Pruefung `clang -print-targets \| grep aarch64`, Fedora-`dnf`-Zeile, apt.llvm.org fuer neueres clang, Befehle im NewTOS-Checkout ausfuehren, QED+TeraDesk (§5.2) als erster Toolchain-Test. Stand in der Antwort auf #4 (Kommentar 6310626343) | S | **erledigt 2026-10-22** (`00-quickstart` §1.1 Fedora-Zeile, kein Cross-gcc, `-print-targets`, apt.llvm.org; §5.2 Toolchain-Test im Checkout) |
| — | spaeter/bei Bedarf: `-Wno-error=int-conversion`-Entscheidung, gemeinsamer LP64-Header, socket/crypt/shadow/argp, C++-Laufzeit | — | — |

[Gemeinsame Regeln](README.md) gelten.

## Befund aus Paket C (2026-10-21)

Die `build-*.sh`-Skripte (libc, gemlib, cflib, libm, apps, vendor-lib …)
rufen `rustc --print sysroot` im aktuellen Verzeichnis auf — ausserhalb des
Checkouts waehlt rustup keine oder die falsche Toolchain (`missing LLVM tool:
/lib/rustlib//bin/llvm-ar`). `newtos-cc` macht es richtig (`cd "$SDK" &&
rustc`). Dokumentiert im Schnellstart („im Checkout ausfuehren“); Fix in ~15
Skripten offen → Paket I (S). **Erledigt 2026-10-21** (`sdk/toolchain.sh`, Ausgaben byte-identisch).
