# A01 — STATUS: DONE

`wind_get(0, WF_WXYWH)` ist geräteabhängig: die Regel **(0, hbox, W, H − hbox)** gilt auf
640×480 und auf 1920×1080, und ein falscher W/H-Wert wird erkannt. **Kein
Codeeingriff** — die Implementierung erfüllte die Regel bereits; ergänzt wurde
nur die fehlende Prüfung. Bearbeitet 2026-09-19, Branch `desk-rebuild`, Basis
HEAD `d7ab92d`, Baum vorher sauber. Gemeinsam mit A01–A04 bearbeitet (eine
Prüfung, vier Unterfunktionen).

## Soll

| | Datei:Zeile |
|---|---|
| EmuTOS, Rechteck bilden | `references/emutos/aes/gemgraf.c:423-424` (`gl_rscreen = (0,0,gl_width,gl_height)`, `gl_rfull = (0,gl_hbox,gl_width,gl_height-gl_hbox)`) |
| EmuTOS, in Handle 0 ablegen | `references/emutos/aes/gemwmlib.c:1294` (`w_setsize(WS_WORK, DESKWH, &gl_rfull)`) |
| EmuTOS, auslesen | `references/emutos/aes/gemwmlib.c:1528-1541`, `:236-262` (`wm_get` → `w_getsize`, kein Rechenweg) |
| EmuTOS, `gl_hbox` nach außen | `references/emutos/aes/gemsuper.c:287-291` (`graf_handle`, `int_out[4]`) |
| Spec | `docs/api/aes/window-graphics.md` § `wind_get` (104), § `wind_create` (Handle 0 = `DESKWH`, `wm_init`) |
| NewTOS | `tos-api/src/aes/window.rs::init_desk_window` (liest `screen_rect()` = Backend-Breite/-Höhe und `desktop_work()` = `menu::desk_top()`), `::wind_get` (`WF_WXYWH` → gespeicherter Slot) |

Gepinnte Quelle: `references/emutos` @ `1a7484aba1bfba23e2c0899e243b6d8177fe41df`.

## Bestand vorher

| Test | Profil | Was er prüft |
|---|---|---|
| `tos-api/tests/aes_reference/cases_r1.rs`, Fall 155 | nur 640×480 (TT) | Zahlenvergleich gegen `EMUTOS14-AES.BIN` — belegt den Wert auf einem Profil, nicht die Ableitung |

Fehlende Eigenschaft: die Regel auf einem **zweiten** Profil, und dass die
Prüfung einen falschen W/H-Wert überhaupt bemerkt.

## Ergänzt

`tos-api/tests/aes_wind_get_desktop_geometry.rs` — metadaten-only Backend mit
umschaltbarer Breite/Höhe; je Profil `wm_init()` (der Einstieg, den
`kernel/src/shell_loop.rs:84` vor jedem Programm ruft), dann `graf_handle`
für `gl_hbox` und `wind_get(0, …)`. Erwartung wird aus Backend-W/H und dem
gemeldeten `gl_hbox` gebildet, nicht aus Konstanten; ein späterer A20-Fix, der
`gl_hbox` aus der Fontmetrik ableitet, bricht den Test nicht, solange
`graf_handle` und die Menüleiste dieselbe Höhe melden (in EmuTOS dieselbe
Variable).

| Test | 640×480 | 1920×1080 |
|---|---|---|
| `a01_wf_wxywh_of_the_desktop_is_the_screen_below_the_menu_row` | (0, 19, 640, 461) — grün | (0, 19, 1920, 1061) — grün |
| `the_desktop_rectangles_follow_the_backend_from_one_profile_to_the_other` | alle vier Felder nach Regel; TT → Pi4 → TT umschalten ändert und stellt wieder her — grün | |
| Negativ `a_wrong_width_is_detected_in_all_four_fields` (Backend 640×480, Erwartung 1920 breit) | alle vier Felder gemeldet — grün | |
| Negativ `a_wrong_height_is_detected_in_all_four_fields` (Erwartung 1080 hoch) | alle vier Felder gemeldet — grün | |
| Negativ `a_wrong_menu_bar_height_is_detected_in_the_two_gl_rfull_fields_only` (`hbox+1`) | | genau WF_WXYWH und WF_FXYWH gemeldet — grün |

**Rot-Gegenprobe** (Scratch, `screen_rect()` vorübergehend auf 640×480
eingefroren, von Hand zurückgesetzt, MD5 von `window.rs` vorher = nachher
`6d301fcd1fc52f23eee98145fdf43677`): `a01_wf_wxywh_of_the_desktop_is_the_screen_below_the_menu_row` fällt bei 1920×1080 mit
`got (0, 19, 640, 461), the rule says (0, 19, 1920, 1061)`. Der Test ist also nicht trivial wahr.

## Beleglage

| | |
|---|---|
| Hosttests | 575 → **583 grün / 0 rot / 1 ignoriert** (+8, alle in der neuen Datei) |
| Bau rpi4 / qemu-virt (frisch, eigener `CARGO_TARGET_DIR`) | beide `rc=0`, tos-api 64, kernel 94 rustc-Warnungen — Sollstand, Delta 0 |
| Clippy beide Boards (`--release`) | tos-api 81, kernel 125 — Sollstand, Delta 0; neue Testdatei 0 Warnungen |
| `git status --short` | nur `?? tos-api/tests/aes_wind_get_desktop_geometry.rs` + die vier Quittungen |

## Offen

- Keine Hardwareabnahme nötig: die Pi-Messung (Fall 155, `AES.LOG`: (0, 19, 1920, 1061)) ist
  mit der Regel bereits erklärt; der Test macht die Erklärung prüfbar.
- A20 (Menüleistenhöhe 19 als Konstante statt aus der Fontmetrik) bleibt offen,
  nicht angefasst.
- Nicht committet; die Arbeit liegt im Baum. Ticket-Kopfzeile unverändert.
