# SDK05 — Quittung: cflib `getcookie` baut die Cookie-Kennung big-endian

Datum 2026-10-29 · Ticket [SDK05](SDK05.md) · Herkunft
`docs/analysis/abi-assumptions-check-2026-10-28.md` §3,
`docs/analysis/byte-order-survey-2026-10-29.md` (Klasse A, Fund 1/3/4/5) ·
Lauf `measurements/measurement-2026-10-29-sdk05/RUNINFO.TXT`

## Sonde vorher/nachher

Neue Sonde `sdk/cflib/tests/cookie-probe.c` (Konsolenprogramm CKPROBE.NCP aus
`C:\AUTO`), gebaut mit `sdk/cflib/build-cflib.sh <probe> <out>` — also gegen das
echte `cflib.a`, dessen `cookie.o` auch QED linkt. Sie ruft cflibs
`getcookie(name, &v)` und als Referenz `Getcookie(<Kennung als Zahl>, &v)` fuer
`NTOS`, `_IDT`, `_MCH`, `_AKP`, `EdDI` und `XXXX` (nicht im Jar). Die Sonde
liegt bei cflib statt in `apitest`, weil `apitest` nur `libc.a` linkt und
cflibs `getcookie` dort nur als Nachbau, nicht als Bibliotheksfunktion
erreichbar waere.

- **Vorher** (Upstream-`cookie.c`): `CKPROBE: 1/6 agree` — alle fuenf Jar-Namen
  `getcookie found=0`, `Getcookie found=1`; nur `XXXX` stimmt (beide nicht
  gefunden). Der woertliche Nachbau der Union liefert z. B. `_IDT` ->
  `0x000000005444495F` statt `0x5F494454`. Die oberen 4 Byte waren in diesem
  clang-`-O2`-Bau 0 (32-Bit-`ldr`), im C-Code bleiben sie unbeschrieben.
- **Nachher** (Overlay): `CKPROBE: 6/6 agree` — gefunden, Wert gleich
  `Getcookie` (`_IDT` = `0x0000112F`, `NTOS` = `0x100`, `_MCH` = `0x50000`,
  `_AKP` = 0, `EdDI` = 1); `XXXX` weiter nicht gefunden.

## QED `_idt` vorher/nachher

QED unveraendert, nur neu gegen das jeweilige `cflib.a` gebaut. Beleg ohne
Produktcode: RAM-Abzug (HMP `pmemsave`) bei laufendem QED, TPA ueber
Textbytes + BASEPAGE-Pruefung gefunden, `static long _idt`
(`sdk/apps/qed/src/global.c:564@getcookie`) am `llvm-nm`-Offset gelesen.

- **Vorher:** `_idt = 0x000000000000112e` (QEDs Ersatzwert).
- **Nachher:** `_idt = 0x000000000000112f` (Wert des Kerns,
  `tos-api/src/cookie.rs:224@seed_defaults`).

**Sichtbare Wirkung in QED: keine.** QED liest nur `_idt & 0xF00`
(Datumsreihenfolge, `sdk/apps/qed/src/global.c:315@make_date`) und schreibt
feste Trenner; `0x112E` und `0x112F` haben beide Reihenfolge 1
(TT.MM.JJJJ). Die Ticketannahme „fuer den Anwender sichtbar (Datumsformat)“
(abgeleitet, nicht gemessen) trifft fuer QED nicht zu: der Wert ist jetzt
richtig, die Anzeige bleibt gleich. Beide Bildschirmabzuege zeigen QEDs
Menueleiste.

## Patch (Overlay, Upstream unveraendert)

Neue Overlay-Datei `sdk/cflib/overlay/cookie.c` = unveraenderte
cflib-Datei cookie.c aus `sdk/cflib/cflib-21-ce57817.tar.xz` plus **eine**
benannte Patch-Stelle `NewTOS overlay, patch site SDK05`
(`sdk/cflib/overlay/cookie.c:52@CFLIB_COOKIE_TAG`): unter `__NEWTOS__` (das
Define, mit dem `build-cflib.sh` cflib uebersetzt) baut
`CFLIB_COOKIE_TAG(c)` die Kennung per Schiebeoperationen aus den vier Zeichen
(erstes Zeichen in Bit 31..24, als `unsigned char`, ganze `long` definiert,
Bit 63..32 = 0); der Upstream-Union-Code steht unveraendert im `#else`. Der
`__PUREC__`-Zweig der Datei wird im clang-Bau nicht uebersetzt und ist
unberuehrt. `build-cflib.sh` kopiert das Overlay ueber den Baum; `cflib.a` neu
gebaut (129/129 Dateien, 0 Fehlschlaege). In QED wird aus `ldr w0, [x0]`
jetzt `ldr w8, [x0]; rev w0, w8`
(`measurements/measurement-2026-10-29-sdk05/runs/qed-getcookie-disasm.txt`).
Kopfkommentar von `sdk/cflib/build-cflib.sh` nennt die neue Overlay-Datei;
`docs/sdk/09-port-material.md` fuehrt `cookie.c` neben `fontsel.c` als
Vollkopie, die ein Pin-Wechsel umsetzen muss.

Wirkung ueber QED hinaus: jedes cflib-Programm findet jetzt die Cookies im
Jar. cflibs `appinit` fragt `MiNT`, `MgPC`, `nAES`, `NVDI` — keiner davon
steht heute im Jar (`seed_defaults` setzt nur `NTOS`, `_MCH`, `_IDT`, `_AKP`,
`EdDI`), dort aendert sich also nichts. `DHST` (QED `comm.c`) steht ebenfalls
nicht im Jar.

## Test (SDK-Hostlauf)

`sdk/cflib/tests/cookie-host.sh` + `cookie-host.c` + `cookie-host-intern.h`,
eingehaengt in `sdk/run-host-tests.sh` als `cookie-host`. Uebersetzt das
Overlay mit `-D__NEWTOS__` auf dem Host; der Harness spielt `Getcookie`,
zeichnet die **ganze** uebergebene `long` auf und vergleicht sie 64-bit mit
den Jar-Kennungen; vor jedem Aufruf wird ein Stapelbereich mit `0xA5`
beschmutzt. 26 Pruefungen: fuenf Jar-Namen (Kennung exakt, TRUE, Wert), `XXXX`
(FALSE, Wert 0), `p_value` NULL, ein Name mit Bytes >= 0x80 (keine
Vorzeichenerweiterung).

- **Mit Patch:** `cookie-host: 0 of 26 checks failed`.
- **Ohne Patch** (`COOKIE_SRC=` die Tarball-Datei, im Session-Scratch
  entpackt): `cookie-host: 17 of 26 checks failed`, z. B. `_IDT tag handed to
  Getcookie got 0x000000015444495F want 0x000000005F494454` — verdreht **und**
  mit nicht-nullem oberen Halbwort (die unbestimmten Bytes aus dem Nachtrag).

## Zahlen

- `sdk/run-host-tests.sh`: `host tests: 29 passed, 0 failed, 0 skipped`
  (Basislinie 28 + `cookie-host`).
- `cargo xtask test`: 1824 passed, 0 failed, 1 ignored (keine Rust-Datei
  beruehrt).
- QEMU raspi4b, beide Seiten: `crash=0 panic=0`; Kernel unveraendert
  (`kernel8.img` 3fb96906…, Release-Bau von 1b28ba37).

## Dateien

- neu: `sdk/cflib/overlay/cookie.c`, `sdk/cflib/tests/cookie-probe.c`,
  `sdk/cflib/tests/cookie-host.sh` (ausfuehrbar),
  `sdk/cflib/tests/cookie-host.c`, `sdk/cflib/tests/cookie-host-intern.h`,
  `measurements/measurement-2026-10-29-sdk05/` (RUNINFO.TXT, method/, runs/),
  diese Quittung
- geaendert: `sdk/run-host-tests.sh`, `sdk/cflib/build-cflib.sh`
  (Kopfkommentar), `docs/sdk/09-port-material.md`
- nicht geaendert: `docs/STATUS.md` (Lead), Ticket-Statuszeile in
  `TASK/SDK05.md`, QEDs Quellen
