# A141 — Quittung: HypView-Icons beim Bau an die TOS-Palette angepasst (B2)

Datum 2026-11-07 · Beleg
[measurements/measurement-2026-11-07-a141-fix/](../measurements/measurement-2026-11-07-a141-fix/RUNINFO.TXT)
· Ursache [measurements/measurement-2026-11-07-a141/](../measurements/measurement-2026-11-07-a141/RUNINFO.TXT)
· Ticket [A141](A141.md) · GitHub #23

**UNFINISHED heisst hier: die Umsetzung ist fertig und in QEMU geprueft; die
Abnahme durch den Tester und das neue Paket von [REL071](REL071.md) stehen aus.**

- **Ausgangslage:** Die 8-Plane-Icons von `hyp_view.rsc` sind fuer die
  NVDI-Palette gezeichnet; EmuTOS und NewTOS lesen den Plane-Wert als
  Register der Standardpalette (`references/emutos/aes/gemrslib.c:348-396`).
  Entscheidung des Anwenders: B2, Anpassung beim Bau als Teil der Portierung.
- **Patch** (`857cf10d`): `sdk/overlays/hypview/rsc/recolour-cicons.py` (Befehl
  `recolour`), aufgerufen von `sdk/apps/build-hypview.sh`:
  `hyp_view.rsc` -> `hyp_view-recoloured.rsc` -> `tools/mkrsc.py
  convert` -> `hyp_view.rsc`, die beiden letzten im Bauverzeichnis von
  HypView. Upstream unveraendert; Kernel, AES, VDI
  unberuehrt.
- **Abbildung:** Register n -> Pen `REV_MAP_COL[n]` (`vdi_col.c:736-747`) ->
  `nvdi.pal[Pen]` -> naechstes Register der TOS-Standardpalette. Metrik:
  kleinster quadrierter euklidischer Abstand ueber r, g, b in VDI-Einheiten
  0..1000, ungewichtet, nicht quantisiert; bei Gleichstand das kleinste
  Register. Steht im Kopf des Skripts.
- **Paletten mit Herkunft** (`sdk/overlays/hypview/rsc/palettes.sha256`):
  `nvdi.pal` = `references/freemint/xaaes/src.km/pal/nvdi.pal` @ f906416,
  byte-gleich; `tos-default.pal` vom Befehl `tos-pal` aus
  `kernel/src/vdi_backend.rs:56@VIDEL_PALETTE1` und
  `kernel/src/vdi_backend.rs:70@VIDEL_PALETTE2` geschrieben, gleich EmuTOS
  `videl_palette1/2` (`references/emutos/vdi/vdi_col.c:130-199`).
- **Zuordnung:** 256 Register auf 96 Ziele, 75 mit Gleichstand. In den
  Icons 92 Register, 9408 Pixel in 14 Rastern (kein 8-Plane-SELECTED-Bild).
  252 -> 7 (grau 733, Gleichstand mit 20, gleiche Farbe), 250 -> 17 (grau
  933), 93 -> 227 (467,200,0). 13 benutzte Register haben einen Gleichstand
  zwischen verschiedenen Farben (24, 29, 34, 35, 54, 57, 66, 99, 129, 135,
  136, 202, 209; 262 Pixel); das kleinste Register gewinnt, dieselbe Regel
  wie im Sollbild des ersten Laufs.
- **Host-Test:** `tools/xtask/src/rsc_recolour_tests.rs`, eigener Parser
  der klassischen Datei. `recolour_palettes_equal_their_sources`,
  `hypview_rsc_8plane_icons_take_the_nearest_tos_colour` (alles ausser den
  8-Plane-`col_data`/`sel_data` bytegleich, jedes Pixel naechste Farbe, die
  drei Register des Tickets mit ihren Pixelzahlen). Rot mit identischer
  Abbildung (= Upstream-Bytes, also der bisherige Bau), gruen mit Patch.
  `cargo xtask test` voll: 2241 bestanden, 0 fehlgeschlagen, 1 ignoriert.
- **QEMU raspi4b:** HypView mit `cflib.hyp`, Screendump
  `measurements/measurement-2026-11-07-a141-fix/runs/newtos-hypview/shots/hypview.png`. Geaendert sind nur die
  Icon-Pixel der Leiste (3218 Pixel, x 6..464, y 44..64). Die fuenf
  freigegebenen Icons stehen pixelgleich zur Umrechnung auf dem Schirm, die
  neun gesperrten auf der Paritaet, die das Sperrmuster nicht ueberzeichnet.
  Die gebaute Ressource ergibt das Sollbild
  `sheet-nvdi-recoloured-to-default.png` mit 0 verschiedenen Pixeln.
- **Kernelseite unveraendert:** icprobe mit der neuen Ressource, Orakel
  gegen NewTOS: 59 Faelle, 0 verschieden. Gegenprobe gegen den alten
  NewTOS-Lauf: 18 verschieden, genau die 14 8-Plane-Icons und der
  Leistenbaum, alle Masken gleich.
- **Release-Texte:** `docs/release/v0.7.1/README.TXT` und `LIESMICH.TXT`:
  der Punkt steht jetzt unter "Neu in 0.7.1", nicht mehr unter den
  bekannten Einschraenkungen.

Offen: Tester-Lauf, dann Paket neu (REL071). Pi: das Bild der Pi ist das von
QEMU (erster Lauf); nichts Eigenes zu messen. Die anderen HypView-Ressourcen
(de, cs, Skins) gehen nicht auf die Karte und sind nicht angefasst.
