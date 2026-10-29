# SDK04 (Developer-Teil) — Quittung: SDK-Typen fester Breite, REF/zView-Fixes nach Messung, Doku

Datum 2026-10-29 · Ticket [SDK04](SDK04.md) Punkt 2 (Messen/Fixen) und Punkt 3
(Typen) · Eingaben `docs/analysis/lp64-tos-struct-survey-2026-10-28.md`
(Abschnitt 6), `docs/analysis/byte-order-survey-2026-10-29.md` (Funde 6–9,
Sonden S2/S3). Fund C6 hat eine eigene Quittung
([SDK04-C6-STATUS-DONE.MD](SDK04-C6-STATUS-DONE.MD)).

Kein Kern, kein `tos-api`, kein QEMU-Lauf: alle Messungen sind Host-Messungen
am echten Quelltext (daher kein Lauf unter `measurements/`).

## 1. Typen fester Breite (Punkt 3)

**Neu: `sdk/libc/overlay/include/mint/sysvars.h`** (Overlay ueber mintlibs
`<mint/sysvars.h>`; `build-libc.sh` kopiert es in den Baum).

- Jede LONG-/Zeiger-Zelle der Seite 0 ist `volatile __UINT32_TYPE__ *` an
  ihrer Adresse aus `docs/api/bios/_boot-init.md` §7 (`proc_lives`,
  `etv_*`, `memvalid`, `resvalid`, `resvector`, `phystop`, `_membot`,
  `_memtop`, `memval2`, `ramtop`, `_v_bas_ad`, `_vblqueue`, `colorptr`,
  `screenptr`, `_vbclock`, `_frclock`, `_hz_200`, `savptr`, `_sysbase`,
  `_shell_p`, `pun_ptr`, `_p_cookies`). BYTE-/WORD-Zellen unveraendert.
- `OSHEADER` 48 Byte, Zeigerfelder `__UINT32_TYPE__`, mintlibs Feldnamen
  (bestehende Leser von `os_version` uebersetzen unveraendert);
  `_Static_assert` auf alle 14 Offsets und `sizeof == 0x30` gegen
  `docs/api/structures/osheader.md`.
- `struct __post_mortem_dump` (`$384`) mit 32-Bit-Registern, Offsets gegen
  §7a gepinnt, 104 Byte.
- Neu `NEWTOS_COOKIE {u32 tag; u32 value}`, `packed`, Asserts `tag@0`,
  `value@4`, `sizeof == 8` (Schritt 8) gegen `docs/api/bios/_cookie-jar.md`
  „Structure“; Lesehelfer `NEWTOS_COOKIE_JAR` (Jar ueber die 4-Byte-Zelle
  `$5A0`, null-erweitert) und `NEWTOS_SYSBASE` (OSHEADER ueber `$4F2`).
- `HDINFO` (hinter `pun_ptr`) bleibt ungepinnt: `docs/api` hat kein Layout
  fuer `pun_info` — im Kopf vermerkt.

**`sdk/libc/overlay/include/mint/ostruct.h`** (nur Asserts, kein Layout
geaendert, `#if defined(__LP64__)`): `_MD` 32 / `_MPB` 24 Byte mit allen
Offsets (Vertrag O1 = A, `docs/api/bios/00-getmpb.md` V1/V4 — das ist die
SDK-Haelfte von V4; die Kern-Haelfte bleibt bei TASK/BIOS06), `_KBDVECS`
`midivec@0` … `ikbdsys@0x40`, `kbstate`@0x48 (`docs/api/xbios/22-kbdvbase.md`
B1). `_IOREC` nicht dupliziert (hat `iorec-layout`).

**Pruefung:** neuer Hosttest `sysvars-layout` (`sdk/libc/tests/sysvars-layout.sh`
mit `sysvars-layout.c` und `ostruct-layout.c`, fuer das Ziel uebersetzt, nur
`_Static_assert`): Adresse, Breite und Vorzeichenlosigkeit jeder Zelle,
Cookie-Schritt, OSHEADER, die beiden Lesehelfer. Gegen mintlibs
unveraenderten `sysvars.h`: 42 Diagnosen (35 fehlgeschlagene Asserts,
`NEWTOS_*` unbekannt). Mutationsprobe `ostruct.h` (`md_start` als `int`,
`midisys` entfernt): die Asserts `md_length@16`, `md_owner@24`,
`sizeof(_MD)`, `ikbdsys@0x40`, `kbstate@0x48` schlagen an.

**Keine Codegen-Aenderung:** `libc.a` nach dem Neubau byte-gleich mit dem
vorherigen. Jede Uebersetzungseinheit, die `<mint/sysvars.h>` einbindet
(TeraDesk `gettos.c`, WinDom `vq_tos.c`, HypView `init.c`, `present.c`,
`fsbench.c`, DOOM `doomgeneric_gem.c`), wurde in ihrem echten Bau zusaetzlich
mit dem unveraenderten Kopf uebersetzt: alle sechs Objekte byte-gleich
(Gegenprobe mit einer Einheit, die `*_hz_200` direkt liest: verschieden).
TeraDesk, WinDom, HypView, present, fsbench, DOOM, zView, QED bauen.

## 2. Fixes nach Messung (Punkt 2)

**S2 / Fund 6 — HypView `.REF`** (`sdk/apps/hypview/hyp/ref.c`). Sonde = neuer Hosttest
`sdk/overlays/hypview/tests/ref-host.sh`: das echte `ref.c` mit einer
Datei im Speicher statt GEMDOS, eine synthetische Zwei-Modul-REF-Datei und —
falls vorhanden — die echten `references/freemint/doc/programmer/mint-prg.ref`
und `references/freemint/doc/german/mint-de.ref`; Sollwerte die m68k-Lesart
(aus den Dateibytes abgelesen), unter ASan/UBSan.

- **Vorher** (vendored `ref.c`): 18 von 19 Pruefungen falsch, jede Datei
  „ERROR: Illegal REF file. Magic value not found.“ — die Vorhersage der
  Erhebung.
- Zwischenprobe (nur die Magie berichtigt, Scratch): `ref_load` nimmt die
  Datei an, dann falsche Knotennummern (`&ref->start` liegt unter LP64 bei
  Byte 16 statt 8) und UBSan-Abbruch (fehlausgerichtetes `short`) — ein
  reiner Magie-Fix haette die Ablehnung in Fehllesen verwandelt.
- **Patch** `sdk/overlays/hypview/hyp/ref.c` (Patch-Stelle SDK04-S2): Magie,
  `num_entries` (Byte 4), Eintragsbeginn (Byte 8), Modulkopf-Schritt (8
  statt `2*sizeof(long)`), Label- und Alias-Zeilenwort big-endian aus den
  Bytes; Eintragszahlen als vorzeichenbehaftetes 32-Bit-LONG wie auf dem 68k.
  Upstream-Eigenheit bleibt (Alias-„Zeile“ = Typbyte des Folgeeintrags).
- **Nachher:** 25 von 25 Pruefungen richtig. HypView baut (57 Dateien, 4 aus
  dem Overlay), Warnungsmenge unveraendert (8).

**S2 / Fund 7 — HypView `.BGH`** (`sdk/apps/hypview/bubble/bgh.c`): **nur dokumentiert, kein
Patch.** Sonde (Scratch, echtes `bgh.c`, echte `sdk/apps/hypview/hyp_view.bgh` und
`sdk/apps/hypview/bubble/test.bgh`): `*(long *)read = 0x3030302048474223` gegen
`BGH_MAGIC 0x23424748` — kein Treffer, wie vorhergesagt. Wirkung keine:
`BGH_load` wird gelinkt, aber nie gerufen — einziger Aufrufer
`DoInitBubble` steht unter `USE_BUBBLEGEM == YES`, und HypView setzt
`USE_BUBBLEGEM NO` (`dl_user.h`). Ein Fix wuerde toten Code beleben, der
weitere LP64-Stellen hat (Adressmaske `& 0xfffffffeL`, 2-Byte-ausgerichtete
Strukturen mit 8-Byte-Zeigern im In-place-Parser).

**S3 / Fund 8 — zView `remap_color`**
(`sdk/zview/z-tools-04805b7/zview/zvdi/color.c`). Sonde = neuer
Hosttest `sdk/zview/tests/color-host.sh`: das echte `color.c` (mit dem
Integer-Vorspann des zView-Baus) gegen eine 16-Farben-VDI-Palette.

- **Vorher** (unveraendertes `color.c`): 38 von 41 Pruefungen falsch, z. B.
  `remap_color(0xFF0000)` = Gruen-Index, `0x0000FF` = Schwarz,
  `color_lookup` liefert `0xGGRRSS`; der `0xFFFFFFnn`-Direktindex greift nie
  (`~0xFFuL` ist unter LP64 64 Bit).
- **Patch** `sdk/zview/overlay/zview/zvdi/color.c` (Patch-Stelle SDK04-S3,
  CRLF wie das Original): Kanaele per Schieben aus `0x00RRGGBB`,
  Gleichheit und Ergebnis ueber die `SRGB`-Felder statt `*(int32 *)`, Maske
  32 Bit.
- **Nachher:** 41 von 41. zView baut (Overlays 14), `color.c` ohne Warnung,
  Warnungsdateien weiter 11.
- Erreichbarkeit unveraendert latent: der Pfad laeuft nur bei 4/8 Ebenen,
  die VDI hat 16 (ADR-024).

**Fund 9 — WinDom `vqt_xname`:** nur vermerkt, unerreichbar solange
`vq_vgdos()` -2 liefert; kein Patch.

## 3. Doku

- `docs/sdk/05-abi-pitfalls.md` B3: „never dereference a pointer from
  `Kbdvbase`; range-check < 32 MiB“ war ueberholt (KEYS02: eigene RW-Tabelle
  des Prozesses; ARCH03: Nutzeradressen < 2 GiB) — berichtigt, mit Beleg im
  Code (`tos-api/src/xbios/kbdvecs.rs` `kbdvbase`, apitest `el0_readable`,
  DOOM installiert `kbdvec` per Speicherzugriff).
- B14: ein `.NCP` bekommt **keine** „press any key“-Pause; `sh_chgrf`/
  `sh_toalpha` (`kernel/src/shell_loop.rs`) schalten nur in den Textmodus,
  die einzige Warte-auf-Taste ist der Absturzbildschirm
  (`kernel/src/arch/aarch64/crash.rs` `report_and_wait`) — berichtigt.
- B1: Zeile fuer `<mint/sysvars.h>`, OSHEADER nicht mehr „by accident“,
  `_MD`/`_MPB`/`_KBDVECS`/`_IOREC` als gepinnter Vertrag, `HDINFO` offen.
- B2: `NEWTOS_COOKIE`/`NEWTOS_COOKIE_JAR`; Null-Erweiterung der
  Cookie-Werte durch `Getcookie` (Kern schreibt 4 Byte in `long v = 0`).
- B19: REF-Patch als Beispiel, Magie-als-`long`-Muster.
- `docs/sdk/08-porting-checklist.md` (Sysvars, Kbdvbase, Dateimagie),
  `docs/sdk/04-porting-anatomy.md` (HypView `ref.c`, zView 14 Dateien),
  `docs/sdk/13-zview.md` (Overlay-Tabelle, Absatz zu `color.c`).

## Tests

- `sdk/run-host-tests.sh`: **33 passed, 0 failed, 0 skipped** (vorher 30;
  neu `sysvars-layout`, `hypview-ref-host`, `zview-color-host`).
- `cargo xtask test` (voll): **1828 passed, 0 failed, 1 ignored**.
- `cargo xtask doc-check` auf den vier bearbeiteten `docs/sdk/`-Dateien:
  0 Fehler (vorher ebenfalls 0; 34 unverankerte Altzitate auf unberuehrten
  Zeilen).

## Offen / nicht Teil dieser Arbeit

- Nachtrag 2026-10-29 des Tickets: veraltete Kommentare in
  `sdk/gemlib/overlay/_gc_aes.c` und `_gc_vdi.c` (`handle_svc`, „x1..x29“,
  `vdi::dispatch`) — nicht bearbeitet.
- `docs/api/bios/00-getmpb.md` V4 Kern-Haelfte (Asserts am Deskriptor, apitest
  `MDX` ersetzen): TASK/BIOS06.

## Nebenbefunde (nicht angefasst)

- mintlib `set_sysvar_to_long` (in `libc.a`) schreibt auf dem
  `__has_no_ssystem`-Zweig `*(volatile long *)var`, also 8 Byte.
- Kommentare in `sdk/windom/overlay/src/vq_tos.c` („only fields from reseth on
  move“) und DOOM `kbd_probe` („mintlib's OSHEADER … 8 bytes wide“) sprechen
  vom unveraenderten mintlib-Kopf; fuer das SDK seit dieser Arbeit ueberholt.
- `docs/sdk/04-porting-anatomy.md`: die TeraDesk-Patchdatei `cookie.c` (C6)
  fehlt in der Patch-Tabelle.
