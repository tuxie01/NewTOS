# INPUT04 — IKBD-Dekoder ueberspringt nach 0xF6 sechs statt sieben Datenbytes

Status: **OFFEN** · Prioritaet: **P3** · Typ: Developer — Anwenderentscheidung 2026-10-30, Fund aus [XBIOS04](XBIOS04.md) (`docs/analysis/xbios04-ikbdws-scope-2026-10-30.md`)

## Sachverhalt

- NewTOS: `kernel/src/ikbd.rs:51-55@packet_data_len` liefert fuer das
  Statuspaket-Kopfbyte `0xF6` **6** Datenbytes.
- EmuTOS: `kbdlength_table` beginnt mit **7** fuer `0xF6`
  (`references/emutos/bios/aciavecs.S:435-436`; Index `byte - 0xF6`,
  `:421-430`).
- Folge, falls je ein `0xF6`-Paket den Dekoder erreicht: das 7. Datenbyte wird
  als Scancode gelesen (Phantomtaste oder Break-Code).
- **Ruht heute:** der USB-HID-Weg erzeugt kein `0xF6`-Paket, und `Ikbdws`
  bleibt No-op (XBIOS04 Option 1).

## Vorgabe (Anwender, 2026-10-30)

Muss behoben sein, **bevor** ein `Ikbdws`-Interpreter oder Antwortpakete
(XBIOS04 Option 2/3) entstehen. Dabei auch die uebrigen Laengen gegen
`kbdlength_table` pruefen.

## Abnahme

- Hosttest: Dekoder ueberspringt nach `0xF6` genau 7 Bytes, die Laengen aller
  Kopfbytes `0xF6`–`0xFF` gleich `kbdlength_table`.
- `cargo xtask test` gruen; `mapping.tsv`-Zeile des Dekoders nachgezogen.

[Gemeinsame Regeln](README.md) gelten.
