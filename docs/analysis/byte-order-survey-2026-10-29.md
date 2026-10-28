# Byte-Order-Erhebung ueber alle Ports (SDK04 Erweiterung 2026-10-28)

Stand 2026-10-29, `main` bei c80c04b0. Datierter Schnappschuss, nicht
fortgeschrieben. Auftrag: [SDK04](../../TASK/SDK04.md) Abschnitt „Erweiterung
2026-10-28 (Anwender)“. Erster Fund: [SDK05](../../TASK/SDK05.md).

Kennzeichnung: **READ** = im Quelltext gelesen und zitiert. **DERIVED** =
aus gelesenem Code geschlossen, nicht gemessen. Keine Zeile hier ist
gemessen; die Wirkungen sind Hypothesen, die die Sonden pruefen.

## Abgrenzung (was korrekt ist)

- Mehrzeichen-Literale (`'MagX'`, `'LDGM'`, `'SLIP'`, `'MiNT'`) sind in clang
  auf jedem Ziel big-endian-bewertet (`'ABCD' == 0x41424344`). Sie werden
  mit hexadezimalen Konstanten (`C_MiNT 0x4D694E54L`) und der Kern-Seite
  (`u32::from_be_bytes`, `tos-api/src/cookie.rs:25@NTOS`) verglichen: gleich.
  Treffer dieser Klasse: `sdk/apps/hypview/dl_ledit.c:143@Getcookie`,
  `sdk/apps/highwire/modules/network.src/iconnect/socklib.c:24@find_cookie`,
  `sdk/ldg/ldg-r133/src/mem/ldgmain.c:160@ldg_cookie` u. a. — **kein Fund**
  (READ).
- Zahlenkonstanten (`0x46534D43L`, `C_EdDI`, `TAG_NTOS`, windom `*_COOKIE`)
  und die Jar-Pfade ueber `Ssystem(S_GETCOOKIE)` / `Getcookie`
  (`sdk/libc/newtos-arch/getcookie.c:33@Getcookie`, teradesk
  `sdk/apps/teradesk/library/utility/cookie.c:39@find_cookie`, windom
  `sdk/windom/windom-1.21.3/src/get_cookie.c:49@get_cookie`, LDG-Overlay
  `sdk/ldg/overlay/src/devel/ldgutil.c:86@ldg_cookie`, tosdump
  `tosdump/core/scrninfo.c:373@TD_COOKIE_EDDI`) sind byte-order-neutral
  (READ). Das Jar schreibt der Kern als native u32 (`wr_long`,
  `tos-api/src/bdos/process/loader.rs:1186@from_be_bytes`; apitest liest per
  `RD_U32` nativ) — DERIVED konsistent.
- `htonl`/`ntohl`-Treffer: nur in Highwires mitgelieferten Netzwerk-Headern
  und coremark-POSIX-Portierungen, jeweils korrekt als Netzwerk-Ordnung
  benutzt; kein TOS-Datenbezug. Kein Fund. XBRA: `_XBRA_MAGIC` existiert nur
  im mintlib-Header (`sdk/libc/mintlib/include/mint/xbra.h:22@_XBRA_MAGIC`),
  nirgends in `sdk/apps`, `sdk/examples`, `programs`, `tosdump` benutzt
  (READ, grep leer). Kein Fund.
- Bereits behandelt (Overlay mit Begruendung, Upstream unveraendert):
  zView-Codecs degas/neochrome/godpaint/eureka/img
  (`sdk/zview/overlay/zview/plugins/degas/degasldg.c:12@DEGAS`), hypview
  HYP-Index (`sdk/overlays/hypview/hyp/hyp.c:4@HypLoad`), windom
  Nachrichtenpaare (`sdk/windom/overlay/src/evnt_windom.c:30@evnt_windom`),
  gemlib `a_evnt_timer.c`. BMP/TGA lesen byteweise (`ToL`/`ToS`,
  TGA `cmap_first` in `tgaldg.c`) — host-ordnungsfrei (READ).
- RSC-Dateien: alle auf der Karte laufen ueber `tools/mkrsc.py convert`
  (little-endian NRSC bzw. native RSHDR, `tools/mkrsc.py`), auch
  `ZVIEW.RSC`, `hyp_view.rsc`, `qed.rsc`, `desktop.rsc`. Die eigenen Lader
  (teradesk `rsc_load.c`, windom `rsrc_load.c` (`Fread` in `RsrcLoad`)) lesen den
  konvertierten Kopf host-nativ: konsistent, kein Fund (READ Build-Skripte,
  DERIVED fuer windom).
- DTA im Desktop: `u16::from_le_bytes` in `programs/desktop/src/dir.rs` (`dta[22]`..`dta[29]`)
  passt zum Kern (`tos-api/src/bdos/fs/dir.rs:477@to_le_bytes`) — NewTOS-DTA
  ist little-endian, kein Fund.

## Funde

Klassen: **A** Byte-Union/Puffer als 32-Bit-Kennung; **B** Dateikopf-Magic
als Integer aus dem Puffer gelesen; **C** Struct per `int32`-Cast und
Byte-Index gemischt (Host-Ordnung); **D** gestapelt, nur LP64-Breite
(Kennung ueber `*(long*)`).

| # | Kl. | Programm / Bibliothek | file:line (Anker) | gebaut / verglichen | Wirkung auf LE-AArch64 | erreichbar im NewTOS-Build | Sonde |
|---|-----|-----------------------|--------------------|---------------------|------------------------|---------------------------|-------|
| 1 | A | cflib `getcookie` (Variante 1, `!__PUREC__`) | `sdk/cflib/cflib-21/cookie.c:33@getcookie`, Union `:38-47@as_char` | `union{long; char[4]}` byteweise gefuellt, als `long` an `Getcookie` | Kennung byteverdreht (`_IDT` -> `0x5444495F`); zusaetzlich fuellt der Code nur die unteren 4 der 8 Union-Bytes, die oberen 4 bleiben unbestimmt (LP64-`long`, DERIVED). `getcookie` findet nichts (SDK05 bestaetigt) | Ja: `nm sdk/cflib/cflib.a` zeigt `cookie.o: T getcookie, U Getcookie` (READ); kein Overlay ersetzt `cookie.c` (`sdk/cflib/overlay/` enthaelt nur `fontsel.c`, `inline.rsh`) | siehe S1 |
| 2 | A | cflib `getcookie` (Variante 2, `__PUREC__`) | `sdk/cflib/cflib-21/cookie.c:72@as_char` | gleiche Union | wie 1, aber nur unter `__PUREC__`: im clang-Build nicht uebersetzt | Nein (`#if !defined(__PUREC__)` waehlt Variante 1; READ) | keine |
| 3 | A | Aufrufer von cflib `getcookie` | `sdk/cflib/cflib-21/appinit.c:51@getcookie` (`MiNT`, `MgPC`, `nAES`, `NVDI`), `magxver.c`, `userdef.c` (`Gnva`), `filesel.c` (`FSEL`), `fontsel.c`/Overlay (`xFSL`), `cfmalloc.c` (`MiNT`), alle in `sdk/cflib/cflib-21/` | Literal wird an Fund 1 uebergeben | Jeder cflib-Programmstart: `gl_mint`, `gl_naes`, `gl_nvdi`, `cf_magxPC` bleiben auf dem Vorgabewert, auch wenn das Jar den Cookie haette (DERIVED; `tos-api/src/cookie.rs:25@NTOS` bis `:37@EDDI` definiert `NTOS`, `_MCH`, `_IDT`, `_AKP`, `EdDI` als Konstanten; ob weitere im Jar stehen, nicht geprueft). Sichtbar ist vor allem `_IDT` (Fund 4) | Ja fuer QED (`sdk/apps/qed/src/main.c:242@init_app`), cflib-Testprogramm; QED.NAP ist auf der Karte (`base.img.manifest`: `C:\Apps\QED\QED.NAP`) | S1 |
| 4 | A | QED | `sdk/apps/qed/src/global.c:564@getcookie` (`_IDT`) | Ersatzwert `0x112E` statt `0x112F`, siehe Hinweis oben im Quelltext | Datumsformat des Anwenders weicht ab (DERIVED, SDK05) | Ja (QED.NAP auf der Karte) | S1 |
| 5 | A | QED | `sdk/apps/qed/src/comm.c:62@getcookie` (`DHST`) | `dhst_id`-Suche | `DHST` wird nie gefunden; ohne Cookie im Jar ohnehin 0, also **keine** sichtbare Abweichung (DERIVED) | Ja | keine noetig |
| 6 | B | hypview Referenzdatei | `sdk/apps/hypview/hyp/ref.c:38@ref_load` (Lesen/Vergleich `:44-47`) | `unsigned long magic; Fread(handle,4,&magic); magic != 0x48524546L` ('HREF') | LP64: 4 Dateibytes in die unteren 4 von 8 Bytes, LE ergibt `0x46455248`, obere 4 Bytes unbestimmt: jede `.REF`-Datei wird als „Illegal REF file“ abgelehnt (DERIVED; zwei Fehler, Byte-Ordnung und Breite) | Gebaut von `sdk/apps/build-hypview.sh` (Verzeichnis `hyp` in der Liste, READ); `ref_load` aufgerufen aus `sdk/overlays/hypview/hyp/hyp.c:109@ref_load` und `:480`. **Nicht** auf der Standardkarte (`HYP_VIEW`/`hypview` fehlen in `tools/make-sd-image.sh` und im Manifest, READ) | S2 |
| 7 | B/D | hypview Bubble-Help | `sdk/apps/hypview/bubble/bgh.c:88@BGH_MAGIC` | `*(long *)read == BGH_MAGIC` (`0x23424748L`, '#BGH') | `*(long*)` laedt 8 Bytes, LE-Wert `0x...48474223`: nie gleich, `.BGH`-Hilfe wird nie geladen (DERIVED) | wie 6: gebaut (Verzeichnis `bubble` in der Liste), nicht auf der Karte | S2 |
| 8 | C | zView Farbabbildung | `sdk/zview/z-tools-04805b7/zview/zvdi/color.c:66@remap_color` (`:67-69` Byte-Zugriff), `:83@screen_colortab`, `:112@color_lookup` | `SRGB{satur,red,green,blue}` (Typedef am Anfang von `color.c`) als `int32` verglichen/zusammengesetzt; `remap_color` liest `((uint8*)&value)[1..3]` als R,G,B | Der Code nimmt big-endian-Byteanordnung (`value` = `0xSSRRGGBB`, Byte 1 = R) an. Auf LE liegt in Byte 1 = G, 2 = R, 3 = 0: Rot/Gruen vertauscht, Blau = 0, Saettigung falsch; der Treffer-Vergleich `*(int32*)&screen_colortab[i] == value` (`:83`) trifft nie; `color_lookup` (`:112`) liefert `(idx<<24)|(satur\|R<<8\|G<<16)`-Muster statt `(idx<<24)\|RGB`. Folge: im 4-/8-Bit-Farbmodus falsche Palettenindizes der `cube216`/`graymap`-Tabellen (`sdk/zview/z-tools-04805b7/zview/zvdi/vdi.c:209@color_lookup`) (DERIVED, nicht gemessen) | ZVIEW.NAP ist auf der Karte; Pfad nur bei `depth == 4 \|\| depth == 8` (Abfrage `depth == 4 || depth == 8` in `vdi.c`, READ), und nur wenn `vq_scrninfo` das meldet. Kein Overlay fuer das Verzeichnis `zvdi` (`sdk/zview/overlay/zview/` kennt nur `av_prot.c custom_font.c main.c plugins winimg.c`, READ) | S3 |
| 9 | D | WinDom `vqt_xname` | `sdk/windom/windom-1.21.3/src/vqt_xname.c:87@get_cookie` | `*(long*)vers == _SPD` (`0x5F535044UL`), `vers` ist ein Cookie-Wert (Zeiger) | Zeigerdereferenz 8 Byte statt 4, Byte-Ordnung wie 7; die Zeile steht hinter `vq_vgdos() == _FSM && get_cookie(FSMC)` | Gebaut (libwindom1.a, in ZVIEW gelinkt); Zeile **nicht erreicht**, solange `vq_vgdos()` -2 liefert (`sdk/gemlib/overlay/_gc_vq_vgdos.c:22@vq_vgdos`, READ; Wert der VDI-Seite nicht gemessen) | keine (nur Wache) |

Nicht erreichbar / ohne Build-Rezept (ohne Wirkung, nur der Vollstaendigkeit
halber): `Applications/cflib-git/cookie.c:33` (Upstream-Kopie von Fund 1,
nur Quelle), Highwire (`sdk/apps/highwire/`, kein `build-highwire.sh`;
dessen `getcookie` nehmen numerische Konstanten), cflib `rpl_stat.c`
(`magic == 0x601A` aus 2 Dateibytes; die gesamte Datei steht unter
`#if defined(__PUREC__)…`, `rpl_stat.c` Zeile 5, nicht uebersetzt).

## Karten-Abgleich (Programmerhebung)

Quelle: `measurements/measurement-2026-10-27-mem12/base.img.manifest`
(Standardabbild, `optional_missing none`). Jede Programmdatei der Karte
stammt aus einem Rezept in `tools/make-sd-image.sh`: `REQUIRED`-Liste
(`:60-82`: libc-Tests, ldg, zview, coremark, tosdump, qed, teradesk,
present) oder `build_example` (hello, big, physbase, scrflip, curdir, cmdline,
dtachild, ptermch, ctrlcpar, fppexec, sysinfo, usbsnap, sysacc). Jeder
Eintrag hat also eine lokale Quelle: die Luecke ist **leer by construction**.
Nicht auf der Standardkarte, aber im Baum: hypview, highwire, doom, gemtest,
chaintest, apitest — Fund 6/7 (hypview) ist deshalb nur ueber eine eigene
Karte erreichbar. Die Funde 1-5 und 8 liegen auf Karten-Programmen (QED,
ZVIEW).

## Vorgeschlagene Sonden (nur Vorschlaege, nichts ausgefuehrt)

- **S1** (Fund 1, 3, 4; schon in SDK05): `apitest`-Fall, der die cflib-Union
  woertlich nachbaut und gegen einen Jar-Namen (`NTOS`, `_IDT`) laufen
  laesst — erwartet heute „nicht gefunden“; zusaetzlich die Kennung als
  `unsigned long` ausgeben und mit `0x5F494454` vergleichen (zeigt Ordnung
  *und* die unbestimmten oberen 4 Bytes). QED: `_idt`-Wert aus dem Lauf.
- **S2** (Fund 6, 7): eine `.REF`/`.BGH`-Minimaldatei, `ref_load` bzw.
  `bgh`-Laden in einem Host-Test oder im Hypview-Lauf; erwartet „Illegal REF
  file“. Hostseitig reicht ein Programm, das 4 Bytes `HREF` in eine
  `unsigned long` liest und mit `0x48524546L` vergleicht.
- **S3** (Fund 8): Hosttest mit der echten `color.c` und einer Testpalette:
  `remap_color(0x00FF0000)` muss den Index der roten Palettenfarbe liefern;
  erwartet heute einen anderen Index. Ergaenzend ZVIEW im 8-Bit-Modus mit
  ein Farbverlauf-Bild, Bildschirmabzug.

## Offen / nicht gelesen

- Welcher Farbmodus (4/8 Bit vs. True Color) ZVIEW auf der Standardkarte
  tatsaechlich meldet, steht nicht im gelesenen Code: Erreichbarkeit von
  Fund 8 haengt daran (mit `vq_scrninfo`-Spezifikation
  `docs/api/vdi/eddi-vq_scrninfo.md` pruefen, nicht gelesen).
- Die `reverse`/`z_trail`-Pixelordnung in `vdi.c` des Verzeichnisses `zvdi` stuetzt sich auf die
  vom `vq_scrninfo` gemeldeten Bitpositionen: ob NewTOS dort die LE-Pixelordnung
  des Framebuffers korrekt meldet, ist nicht untersucht (DERIVED: das Muster
  ist eine Moeglichkeit, die Host-Ordnung zu erkennen, kein Fund).
- Nicht durchsucht: Binaerformate in `sdk/libjpeg`, `sdk/libpng`,
  `sdk/giflib`, `sdk/zlib`, `sdk/libexif` (Upstream-Bibliotheken mit eigener
  Endian-Behandlung); nur auf die Muster A/B grepped, ohne Treffer.
