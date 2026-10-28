# SDK04 — LP64: `long`-Zugriffe auf TOS-Strukturen in allen Ports; Typen fester Breite im SDK

Status: **OFFEN** · Prioritaet: **P1** · Typ: Researcher (Erhebung) + Developer (SDK-Typen, Fixes nach Messung) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Dritter Fall derselben Klasse: ein portiertes Programm liest eine
  TOS-Struktur mit `long`, das unter LP64 8 statt 4 Byte breit ist:
  - TeraDesk `hdv_*` und `_sysbase` (TASK/MEM12.md, Bestand der
    Seite-0-Leser);
  - Cookie-Jar mit `long`-Paaren: HighWire
    (`sdk/apps/highwire/src/romvdi.c:36-52@getcookie`) und TeraDesk
    (`docs/analysis/machine-identity-adr-proposal-2026-10-28.md`, Abschnitt 7;
    Wirkung ungeprueft).
- Bekannte Regel: TOS-Strukturen sind auf dem Draht 4-Byte-breit, mintlibs
  Strukturen unter LP64 nicht (BASEPAGE/DTA/AESGLOBAL ueber Overlays
  geloest).

## Arbeit

1. **Erhebung (Researcher):** alle Ports (`sdk/apps/`, `sdk/overlays/`,
   `sdk/examples/`, `programs/`, Bibliotheken unter `sdk/`) auf Zugriffe mit
   `long`/`unsigned long`/Zeigertypen auf TOS-Strukturen: Cookie-Jar
   (`_p_cookies`, `0x5A0`), Systemvariablen (Seite 0, `_sysbase`/OSHEADER),
   Basepage, DTA, weitere (IOREC, MD/MPB, BPB). Tabelle: Programm, Stelle
   (`file:line`), Struktur, Typ, erwartete Wirkung unter LP64 (gelesen vs.
   abgeleitet). Verfahren wie `.claude/agents/researcher.md`
   „Program surveys“.
2. **Messen, wo gefunden (Developer/Tester):** je Fund ein Lauf auf QEMU
   `raspi4b`, der die Wirkung zeigt (z. B. findet HighWire `EdDI`?); Funde ohne
   Wirkung bleiben dokumentiert. Fixes als benannte Patches in
   `sdk/overlays/<prog>/` erst nach Messung.
3. **SDK-Typen fester Breite** fuer diese Strukturen (z. B. Cookie-Eintrag,
   Systemvariablen-Zugriff, OSHEADER), mit Pruefung zur Uebersetzungszeit
   (`sizeof`/`offsetof` gegen `docs/api/`), damit neuer Code den Fehler nicht
   wiederholt; Doku in `docs/sdk/`.

## Abnahme

- Erhebungstabelle mit Belegen; je Fund Messung oder Begruendung.
- SDK-Hosttests gruen mit den neuen Pruefungen; doc-check 0 Fehler.

## Stand 2026-10-28 (Punkt 1 erledigt)

- Erhebung: `docs/analysis/lp64-tos-struct-survey-2026-10-28.md`. Offene Funde:
  Cookie-Jar 7 (davon im Bau erreichbar 1: TeraDesk `find_cookie`, C6), Sysvars/
  OSHEADER 3 ohne Fehlwirkung, Basepage 1 tot, DTA 0, sonstige 0. HighWire
  (C1–C5) hat kein Bau-Rezept und liegt auf keiner Karte.
- C6: TeraDesk liest das Jar mit 16-Byte-Schritt, findet also nur Cookies im
  geraden Slot mit Wert 0; heute ohne sichtbare Wirkung, weil `CT60`, `_FLK`,
  `_VDO` fehlen — latent fuer ein spaeteres `_VDO` (ARCH06).
- **Punkt 2 (Messung):** Vorschlag M1 der Erhebung (AUTO-Sonde mit HighWires
  `getcookie` und TeraDesks `find_cookie` unveraendert gegen `Getcookie`);
  M2 (TeraDesk mit eingeschleustem `_VDO`) gehoert zu ARCH06.
- **Punkt 3 (Typen):** neu noetig Cookie-Eintrag `{u32 tag; u32 value}` mit
  Schritt-8-Pruefung, Seite-0-Zellen als `volatile uint32_t*`-Kopf fuer
  `sysvars.h`, OSHEADER mit 48 Byte und `offsetof`-Pruefungen; niedriger MD/MPB,
  `_IOREC`/`_KBDVECS`. Dazu `docs/sdk/05-abi-pitfalls.md` B2 berichtigen
  (TeraDesks `find_cookie` ist trotz `Setexc` funktionslos) und die
  Null-Erweiterung von Cookie-Werten durch `Getcookie` dokumentieren.

## Anwenderentscheidung 2026-10-28

- Fund C6 (TeraDesk `find_cookie`, `sdk/apps/teradesk/library/utility/cookie.c`)
  wird **vor ARCH06** behoben (benannter Patch in `sdk/overlays/teradesk/`, mit
  Messung vorher/nachher nach Punkt 2), damit die `_VDO`-Messung von ARCH06
  den ST-Zweig misst und nicht den Schrittfehler.

## Erweiterung 2026-10-28 (Anwender)

- Zusaetzlich Byte-Reihenfolge-Annahmen in allen Ports: Unions oder
  Byte-Arrays, die als 32-Bit-Kennung gelesen werden (Cookie-, XBRA-Tags,
  `char[4]` als `long`). Erster Fund: cflib, [SDK05](SDK05.md).

## Nachtrag 2026-10-29 (Zitate in `docs/sdk/05-abi-pitfalls.md` nachgezogen)

- Alle Zeilenzitate verankert; die `Kbdvbase`-Zeile berichtigt (liefert die
  EL0-VA der prozesseigenen Tabelle, der Kern dispatcht `kbdvec` darueber).
- Offen fuer Punkt 3 (Doku): der Folgetext „never dereference a pointer from
  `Kbdvbase`; range-check < 32 MiB“ ist vermutlich ueberholt; die Aussage, ein
  `.NCP` bekomme ein „press any key“-Warten, ist im Code nicht zu finden (nur
  im Absturzbildschirm) — beides gegen die Quelle pruefen.
- Veraltete Kommentare im SDK (Code, Developer): `sdk/gemlib/overlay/_gc_aes.c`
  und `_gc_vdi.c` nennen `handle_svc` (heute `rust_sync_handler`),
  „x1..x29“ (gesichert werden x0..x30) und `vdi::dispatch` (heute
  `dispatch_in`).

## Stand 2026-10-29 (Erhebung Byte-Reihenfolge)

- `docs/analysis/byte-order-survey-2026-10-29.md`: 9 Funde, 7 gebaut, 6
  erreichbar (alles abgeleitet, nicht gemessen):
  - Stellen mit `file:line` im Dokument. Klasse A (Byte-Union als Kennung): cflib `getcookie` → [SDK05](SDK05.md).
  - Klasse B (Dateikopf-Magic als `long`): hypview `.REF` und `.BGH` — Byte-Reihenfolge und 8-Byte-Breite; hypview gebaut,
    nicht auf der Standardkarte.
  - Klasse C: zView `remap_color` nimmt Big-Endian an; im 4-/8-Bit-Farbmodus Rot/Gruen vertauscht, Blau 0 —
    ob ZVIEW auf der Karte diesen Modus nutzt, ist offen.
  - Klasse D: WinDom `vqt_xname`, unerreichbar solange `vq_vgdos()` -2.
- Vorgeschlagene Sonden S1–S3 im Dokument; Fixes erst nach Messung (Punkt 2).
