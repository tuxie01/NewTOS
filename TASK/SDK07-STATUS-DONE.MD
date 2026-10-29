# SDK07 — Quittung: TeraDesk-Overscan ohne Atari-Hardware gesperrt

Datum 2026-10-29 · Ticket [SDK07](SDK07.md) · Fund aus [ARCH06](ARCH06.md) ·
Lauf `measurements/measurement-2026-10-29-sdk07/RUNINFO.TXT`

## 1. Patch (Patch-Stelle SDK07, `sdk/overlays/teradesk/`)

| Datei | Inhalt |
|---|---|
| `sdk/overlays/teradesk/nohard.h` | neu: `newtos_mch_nohard()` liest `_MCH` per `find_cookie` (Overlay `sdk/overlays/teradesk/library/utility/cookie.c`, SDK04-C6) und vergleicht die unteren 32 Bit mit `0xFFFFFFFF` (`find_cookie` liefert den Wert null-erweitert). Fehlt `_MCH`, gilt wie upstream "Hardware vorhanden" |
| `sdk/overlays/teradesk/video.c` | Overlay der vendorten `sdk/apps/teradesk/video.c`, sonst byte-gleich (CRLF). `get_set_video(0)`, ST-Zweig: bei NOHARD `over = -1L` (die upstream-eigene Marke "kein Overscan"), OVER/Lace-Suche entfaellt; dadurch laesst `voptions()` `VOVERSCN` gesperrt. `get_set_video(1/2)`: der Block `Super(0L)` + Schreiben auf `0xFFFC00` verlangt zusaetzlich `!newtos_mch_nohard()` und laeuft damit auch ohne vorheriges `get_set_video(0)` nie |
| `sdk/apps/build-teradesk.sh` | nur der Kopfkommentar: zwoelf statt zehn Patch-Dateien, `video.c` + `nohard.h` genannt |
| `sdk/overlays/teradesk/tests/nohard-host.c`, `nohard-host.sh` | neuer Hosttest (Bauweise wie `cookie-host`): Overlay-`cookie.c` + `nohard.h` gegen ein Jar im Kernformat |
| `sdk/run-host-tests.sh` | `td-nohard-host` eingehaengt |

Kein Kernel-, tos-api- oder Upstream-Eingriff; `_VDO` bleibt weg.

## 2. Lauf (QEMU raspi4b, Injektor, gleicher Trace-Kernel fuer beide Karten)

- **Vorher** (TeraDesk ohne SDK07-Overlay, `DESKTOP.NAP` `f763e7d9…` =
  byte-gleich mit dem ARCH06-Binary): Overscan anwaehlbar, Klick setzt das
  Kreuz (`objc_change`), Return → `[crash] … Bus Error … far=Some(16776192)`
  (`0xFFFC00`), gleiches pc wie in ARCH06. 1 EXC/PANIC-Zeile.
- **Nachher** (`DESKTOP.NAP` `48dbf1c5…`): Overscan grau; Klick → nur
  `objc_find 13`, kein `objc_change`, Bild unveraendert (0 px). Return → nur
  `Logbase`/`Physbase`, kein GEMDOS `Super` (0x20), kein `Setscreen`, Desktop
  neu aufgebaut, TeraDesk wieder in `evnt_multi`. 0 EXC/PANIC-Zeilen.
- Dialog sonst unveraendert: vorher/nachher unterscheiden sich nur in der
  Beschriftung "Overscan" (148 px, x 221-282 y 275-286, normal → grau);
  Desktop- und Menue-Schirmfotos 0 px Unterschied
  (`measurements/measurement-2026-10-29-sdk07/checks/pxdiff-before-after.txt`).
- `fsck_msdos -n` beider Klone: rc 0; Manifeste: `optional_missing none`.

## 3. Tests

- `sdk/run-host-tests.sh`: 34 passed, 0 failed, 0 skipped (33 + `td-nohard-host`,
  15 Pruefungen). Gegenprobe: ein Vergleich `mch == -1L` faellt mit 2 von 15 durch.
- `cargo xtask test`: 1842 passed, 0 failed, 1 ignored (unveraendert).

## 4. Offen

Echte Pi 4 (Hardware, nicht Sache des Workers). Der Falcon-Zweig (`_VDO`
gesetzt) ist vom Patch nicht beruehrt und schreibt nicht auf `0xFFFC00`.
