# A111 — STATUS: DONE (2026-10-25, QEMU) — Klickzaehlung und Schreibmarke wie EmuTOS

Punkt 1 (Klickzaehlung) erledigt 2026-10-07 mit [ZVIEW02](ZVIEW02-STATUS-DONE.MD),
`388c6bcd`. Dieser Beleg betrifft **Punkt 2**, die `objc_edit`-Schreibmarke.

**Auftrag:** Wirkung belegen (NewTOS gegen EmuTOS, Pixel), Stelle in EmuTOS,
Fix 1:1 mit rotem Hosttest, Vorher/Nachher in QEMU, Regression an Dialogen mit
Eingabefeld.

## Wirkung (vor dem Fix, HEAD `8da6e860`)

Gleiche Tastenfolge (Alt+C, Ctrl+N „Neuer Ordner“, A, B, Links, Links, Rechts,
Backspace) auf Hatari Falcon (gepinntes `etos512us.img`, Truecolor) und in
QEMU raspi4b; Spalte der Schreibmarke ausgemessen
(`measurements/measurement-2026-10-25-a111/checks/caret-columns.txt`):

| | EmuTOS | NewTOS vorher | NewTOS nachher |
|---|---|---|---|
| Zeilen | 22 = Textzelle −3 … +3 | 16 = nur die Textzelle | 22 = Textzelle −3 … +3 |
| Modus | XOR: (66,69,66) auf Grau (189,186,189), weiss ueber Schwarz | Schwarz, REPLACE | XOR: (66,66,66) auf Grau (189,189,189), weiss ueber Schwarz |
| Index 0 am schwarzen Feldrand | — (EmuTOS-Dialog hat keinen Rahmen) | unsichtbar | weisse Linie durch den Rand |

## Stelle

* EmuTOS `aes/gemobed.c:194-218` `curfld`: `dist == 0` → `gsx_attr(FALSE,
  MD_XOR, BLACK)`, `t.g_y -= 3; t.g_h += 6`, Clip = `t`, `gsx_cline`;
  `dist != 0` → Clip `dist` Spalten breit, `ob_draw(tree, obj, 0)`.
  `aes/gemgraf.c:186-192` `gsx_cline`, `:420-422` Linientyp 7/Breite 1/`0xffff`.
* `aes/gemobed.c:335-463` `ob_edit`: das schliessende `curfld(…, 0)` (`:459-461`)
  laeuft fuer jede Art ausser EDSTART — EDINIT = an (ohne Feldzeichnung),
  EDCHAR = aus (`:374-377`), Spanne neu (`:445-455`), an; EDEND = aus.
  `aes/gemfmlib.c:308/:344` `fm_do` haelt EDINIT/EDEND paarig.
* NewTOS vorher: `edit_field_t` zeichnete bei jeder Art das ganze Feld neu und
  setzte einen `CELL_H`-hohen schwarzen Balken. Die Spec nannte EDEND
  „no-op body“ — falsch, korrigiert.

## Fix — `033da16f`

* `tos-api/src/aes/object.rs`: `curfld()` (beide Zweige, eigener Clip, XOR-Linie),
  `edit_field_t` nach `ob_edit`s Ein/Aus-Protokoll unter einer A65-Mausklammer,
  `edit_char` liefert `no_redraw` und verschiebt `start` beim Ueberschreiben
  eines vollen Feldes (`:407-413`).
* `tos-api/src/aes/fsel.rs`: die eigene Schleife der Dateiauswahl (nicht `fm_do`)
  verliess sich darauf, dass EDINIT das Feld neu zeichnet. Jetzt: Klick ins schon
  aktive Feld beendet die Eingabe vor dem Neustart; Dateiklick beendet die Eingabe,
  setzt den Text, zeichnet FSSELECT, startet neu.
* Spec `docs/api/aes/object-form.md` `objc_edit`: EDINIT ohne Feldzeichnung,
  EDEND kein No-op, neuer Abschnitt „The caret protocol“, Ausdehnung/Farbe/Clip.
* Hosttest `tos-api/tests/aes_a111_caret.rs` (2 Faelle): EDINIT aendert genau die
  22 komplementierten Pixel der Spalte; eine ganze Eingabefolge laesst nach jedem
  Aufruf den sauberen Dialog plus genau eine XOR-Marke stehen, nach EDEND keine.
  **Rot auf `8da6e860`** (beide), gruen mit Fix. Angepasst:
  `aes_ev_rets_state_at_event.rs` `objc_edit_caret_follows_te_just` (LDG-M3 O1)
  grundierte mit ED_END als „nur Feld“ — das schaltet jetzt eine Marke **an**;
  der Test misst jetzt die Pixel, die ED_INIT auf leerem Grund aendert, Ursprung
  `pxl_rect.y − 3`, x-Erwartung unveraendert.

## Pruefung

* QEMU raspi4b, vorher/nachher je Bild Vollbildvergleich (`checks/pixdiff.txt`):
  „Neuer Ordner“ 7 Bilder, Info/Umbenennen 5, Dateiauswahl aus QED 7 (Tippen,
  Dateiklick, Klick ins aktive Feld, Tab, Klick zurueck, Links) — jeweils genau
  22 Pixel in der einen Markenspalte verschieden, sonst nichts; jedes Nachher-Bild
  zeigt genau eine Marke, die alte Stelle ist sauber. 0 EXC/PANIC.
* Builds beide Boards dev+release rc 0, 159 Warnzeilen wie Basis; Clippy
  tos-api 76 → 74 (zwei vorhandene `collapsible_match` in `edit_char` fallen mit
  dem Umbau weg, nicht gezielt), kernel 121 = 121; `cargo xtask test` 159 bin
  1639/0/1 → 160 bin 1641/0/1.

## Offen

* Pi-4-Sichtabnahme (keine Hardware in diesem Lauf).
* TeraDesk nicht gefahren (benutzt dasselbe `objc_edit`).

## Ausserhalb des Auftrags beobachtet (unberuehrt)

* Der „Neuer Ordner“-Dialog des NewTOS-Desktops (`programs/desktop` `dlg.rs`
  `edit_name`, „Name of new folder:“, umrandetes Feld, Kleinbuchstaben) ist nicht
  EmuTOS' ADMKDBOX („NEW FOLDER / Name: ________.___“, Validierung `f`).
* `edit_char` erkennt BACKSPACE/DELETE/ESCAPE am niederwertigen Byte; EmuTOS
  vergleicht das ganze Tastenwort (`0x0E08`/`0x537F`/`0x011B`) — Ctrl+H u. a.
  verhalten sich verschieden.
* `edit_char` nimmt nur 0x20..0xFF als Zeichen; EmuTOS prueft jedes Byte ≠ 0
  gegen die Validierung (`gemobed.c:414-415`).
* `gsx_tblt` schreibt Text immer transparent; ein G_FTEXT/G_TEXT ohne 3D mit
  MD_REPLACE-Text (kein Fuellkasten) laesst bei Textaenderung alte Glyphenpixel
  stehen (EmuTOS malt die Zellen im REPLACE-Modus). Im Hosttest deshalb ein
  G_FBOXTEXT.
