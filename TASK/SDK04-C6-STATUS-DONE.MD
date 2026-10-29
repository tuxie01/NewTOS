# SDK04 (Fund C6) — Quittung: TeraDesk `find_cookie` liest das Jar mit 8-Byte-Schritt

Datum 2026-10-29 · Ticket [SDK04](SDK04.md), Fund C6 (Anwenderentscheidung
2026-10-28: vor ARCH06) · Herkunft
`docs/analysis/lp64-tos-struct-survey-2026-10-28.md` (C6, Messvorschlag M1) ·
Lauf `measurements/measurement-2026-10-29-sdk04-c6/RUNINFO.TXT`

Nur C6. Die uebrigen SDK04-Punkte (HighWire C1–C5, SDK-Typen fester Breite,
Punkt 3) sind nicht Teil dieser Quittung.

## Sonde vorher/nachher

Neue Sonde `sdk/overlays/teradesk/tests/cookie-probe.c` (Konsolenprogramm
TDCOOKIE.NCP aus `C:\AUTO`), gebaut mit
`sdk/overlays/teradesk/tests/build-cookie-probe.sh <out> [cookie.c]`: TeraDesks
`sdk/apps/teradesk/library/utility/cookie.c` wird **unveraendert** mit den CFLAGS von
`sdk/apps/build-teradesk.sh` (`-D_TOS_ONLY`, also der `Setexc`-Zweig) aus einem
Schattenbaum wie dem des TeraDesk-Baus uebersetzt und zur Sonde gelinkt —
standardmaessig die Datei, die auch `DESKTOP.NAP` bekommt (Overlay, sonst
vendored). Die Sonde setzt per `Ssystem(S_SETCOOKIE)` hinter die fuenf
gesetzten Cookies (Slot 0–4) `Z0Z0`=0 (Slot 5), `Z1Z1`=0 (Slot 6),
`Z2Z2`=0x12345678 (Slot 7), `Z3Z3`=0x87654321 (Slot 8), listet das Jar als
32-Bit-Paare, vergleicht `find_cookie` mit `Getcookie` fuer die neun Kennungen
im Jar und drei fehlende (`XXXX`, `_VDO`, `CT60`) und loescht die vier
Cookies danach wieder (`S_DELCOOKIE`).

Ort: `sdk/overlays/teradesk/tests/` neben dem Patch, den sie pruefen, wie
`sdk/cflib/tests/` neben `sdk/cflib/overlay/`. `sdk/apps/teradesk` ist der
nicht versionierte Upstream-Baum und scheidet aus; `sdk/examples/` ist fuer
SDK-Beispiele, nicht fuer Sonden eines Ports. Damit `build-teradesk.sh` die
Sonde nicht in `DESKTOP.NAP` uebersetzt, nimmt es `tests/` beim Ueberlagern
aus (`! -path './tests/*'`).

- **Vorher** (vendored `cookie.c`): `TDCOOKIE: 3/12 agree`. `find_cookie`
  fand nur `Z1Z1` (gerader Slot, Wert 0) — die Vorhersage der Erhebung — und
  lieferte dafuer den Wert `0x123456785A325A32`, also Kennung und Wert von
  Slot 7 (`Getcookie`: 0). Die acht anderen Cookies im Jar: nicht gefunden.
  Uebereinstimmung nur bei den drei fehlenden Kennungen.
- **Nachher** (Overlay): `TDCOOKIE: 12/12 agree` — alle neun gefunden, Wert
  gleich `Getcookie` (`Z3Z3` = `0x0000000087654321`, null-erweitert); die drei
  fehlenden nicht gefunden, Wert unberuehrt.

## Patch (Overlay, Upstream unveraendert)

Neue Overlay-Datei `sdk/overlays/teradesk/library/utility/cookie.c` = die
vendored Datei plus Kopfkommentar (warum, Messung) und eine Patch-Stelle
`NewTOS overlay, patch site SDK04-C6`: im `Setexc`-Zweig von `find_cookie`
ist `cookie` ein `const uint32_t *`; die Schleife vergleicht `cookie[0]` mit
`(uint32_t)name`, geht mit `cookie += 2` (8 Byte) weiter, der Wert ist
`cookie[1]` (null-erweitert wie bei `Getcookie`). Gewaehlt statt „ueber
`Getcookie`“: kleinere Aenderung, die `_TOS_ONLY`-Strecke bleibt der
Upstream-Weg (Jar ueber `Setexc(0x5A0/4, -1)`), der `#if _MINT_`-Zweig und der
`COOKIE`-Typ bleiben unberuehrt. CRLF wie im Original. In `DESKTOP.NAP` wird
aus `ldr x8, [x0], #0x10` jetzt `ldr w8, [x0], #0x8`
(`measurements/measurement-2026-10-29-sdk04-c6/runs/find_cookie-disasm.txt`).
`sdk/apps/build-teradesk.sh`: Kopfkommentar nennt zehn Patch-Dateien und
`tests/`; Ausnahme `tests/` in der Overlay-Suche (keine Zeilenverschiebung).
TeraDesk baut 48/48, 0 offene Symbole, 10 Overlays.

## TeraDesk-Start

Beide Seiten: TeraDesk startet per `#Z` nach der AUTO-Sonde bis zu seinem
Desktop (Menue Desk/File/View/Window/Options, vier Laufwerke, Papierkorb;
`runs/<side>/teradesk.png`), `crash=0 panic=0`. Die beiden Bilder sind
bytegleich: keine sichtbare Wirkung, wie erwartet — `CT60`, `_FLK`, `_VDO`
stehen in keinem Fall im Jar. Die Wirkung kommt erst mit einem `_VDO`
(ARCH06, M2): dann misst ARCH06 den Video-Zweig statt des Schrittfehlers.

## Test (SDK-Hostlauf)

`sdk/overlays/teradesk/tests/cookie-host.sh` + `cookie-host.c`, eingehaengt
in `sdk/run-host-tests.sh` als `td-cookie-host`. Die Uebersetzungseinheit
bindet TeraDesks `library.h` wie der `_TOS_ONLY`-Bau ein, lenkt `Setexc` auf
ein Host-Jar im Drahtformat des Kerns und bindet die zu pruefende `cookie.c`
unveraendert per `#include` ein. 41 Pruefungen: zehn Cookies in geraden und
ungeraden Slots, Wert 0 und != 0, Kennung und Wert mit Bit 31; `value` NULL;
vorzeichenerweiterte Kennung; fehlende Kennung und eine Kennung hinter dem
Terminator (FALSE, Wert unberuehrt); leeres Jar; kein Jar; `Setexc`-Argumente.

- **Mit Patch:** `cookie-host: 0 of 41 checks failed`.
- **Ohne Patch** (`COOKIE_SRC=` die vendored `sdk/apps/teradesk/library/utility/cookie.c`):
  `cookie-host: 30 of 41 checks failed` — dasselbe Muster wie auf dem Ziel,
  nur `Z1Z1` gefunden, Wert `0x123456785A325A32`.

## Doku

- `docs/sdk/05-abi-pitfalls.md` B2: die Aussage „`Setexc` … which is why
  TeraDesk's live path works“ war falsch — der Zeiger stimmte, der
  16-Byte-Schritt blieb. Berichtigt (Symptom „findet nur geraden Slot mit
  Wert 0, liefert den Folgeslot als Wert“, beide Haelften noetig, Verweis auf
  Patch, Hosttest, Lauf).
- `docs/api/bios/_cookie-jar.md` (dieselbe Begruendung stand dort ein zweites
  Mal): Punkt 1 und der Absatz „Consequence today … **never finds a cookie** …
  Not fixed here“ nachgezogen (gemessen, fuer TeraDesk behoben; Aufrufer im
  Bau statt der nicht uebersetzten `#if _MINT_`-Zeilen); dazu „the previous
  slot's value“ berichtigt: das 8-Byte-Wort enthaelt die Kennung und den
  **eigenen** Wert des Slots (so auch das dort zitierte Beispiel).

## Zahlen

- `sdk/run-host-tests.sh`: `host tests: 30 passed, 0 failed, 0 skipped`
  (Basislinie 29 + `td-cookie-host`).
- `cargo xtask test`: 1824 passed, 0 failed, 1 ignored (keine Rust-Datei
  beruehrt).
- QEMU raspi4b, beide Seiten: `crash=0 panic=0`; Kernel unveraendert
  (`kernel8.img` b1dd668a…, Release-Bau von 8258ccac).
- `cargo xtask doc-check` der bearbeiteten Doku: 0 Fehler.

## Dateien

- neu: `sdk/overlays/teradesk/library/utility/cookie.c`,
  `sdk/overlays/teradesk/tests/cookie-probe.c`,
  `sdk/overlays/teradesk/tests/build-cookie-probe.sh` (ausfuehrbar),
  `sdk/overlays/teradesk/tests/cookie-host.sh` (ausfuehrbar),
  `sdk/overlays/teradesk/tests/cookie-host.c`,
  `measurements/measurement-2026-10-29-sdk04-c6/` (RUNINFO.TXT, method/, runs/),
  diese Quittung
- geaendert: `sdk/apps/build-teradesk.sh`, `sdk/run-host-tests.sh`,
  `docs/sdk/05-abi-pitfalls.md`, `docs/api/bios/_cookie-jar.md`
- nicht geaendert: `docs/STATUS.md` (Lead), Statuszeile in `TASK/SDK04.md`,
  TeraDesks Upstream-Baum
