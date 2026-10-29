# PTR_RAM-Regel: Funktionsnummer als Zeiger — Regelvorschlag (TOOL11, Arbeitspunkt 1) — 2026-10-28

Rolle: Researcher. Stand: `main` bei 51b24286, EmuTOS-Pin `1a7484ab`
(`references/emutos/`). Dies ist ein Vorschlag; tosdiff aendert der Developer.
Gelesen und gemessen wurde nur lesend; geschrieben wurde allein diese Datei.

## 1. Ergebnis in drei Saetzen

1. Empfehlung: **Option A**, eine feste Untergrenze in `PTR_RAM`:
   `v >= 0x400` (`TD_PTR_LOW`), pro Seite, neben der bestehenden Obergrenze.
   Sie besteht auf allen 25 gepinnten Referenzen (kleinster Orakelwert
   `0x30D4`), laesst `Keytbl` → `0x768` und alle NewTOS-Werte ausser den vier
   Funktionsnummern durch und kippt auf den vorhandenen NewTOS-Dumps genau
   `Iorec(0..2)` und `Bconmap(-2)`.
2. Eine reine Adresspruefung kann nie sagen, dass hinter einem Wert eine
   Struktur liegt; das kann nur der Inhalt. Dafuer gibt es schon den Mechanismus
   `r:<len>` (Abschnitt 5, Option B). Er ist heute nur fuer `Getbpb` tragfaehig:
   fuer `Iorec`/`Bconmap` haengt er an einer ungeklaerten LP64-Layoutfrage, und
   der Laeufer folgt dem Zeiger ohne Obergrenze (Abschnitt 6, Defekt-Kandidat).
3. `Bconmap(-2)` → `0x2C` ist keine zufaellige Abweichung: EmuTOS' eigener
   Zweig ohne `BCONMAP_AVAILABLE` gibt dasselbe zurueck
   (`references/emutos/bios/serport.c:1124`). Die Regel entscheidet nicht, ob das
   akzeptiert wird; der Referenzbau hat `BCONMAP_AVAILABLE`
   (`references/emutos/bios/serport.h:18`, Wert 1 bei `CONF_WITH_SCC`/
   `CONF_WITH_TT_MFP`), also bleibt es eine Abweichung fuer die Triage.

## 2. Alle PTR_RAM-Faelle

Die Tabellen `super-r1`, `xbios-system-r1`, `bios-r2` (ausser Getbpb), alle
AES/VDI/Raster-Tabellen haben keinen `PTR_RAM`-Fall
(`/usr/bin/grep -n PTR_RAM tosdump/cases/*.tsv`). Die Struktur hinter dem Zeiger
steht nur in der Spalte „Struktur“, wo der Rueckgabewert ein Zeiger auf eine ist.
Groessen: m68k-Layout (ILP32) aus dem Pin. Orakelwerte: `decode.py` (lesend) auf
den gepinnten Referenzen; „TT“ = `EMUTOS-1a7484ab-BIOS*`, „F“ = `-falcontc`.

| Tabelle:Fall | Funktion | Struktur hinter dem Wert (EmuTOS) | Groesse | Orakelwert TT / F |
|---|---|---|---|---|
| bios-r1:85, 86 | `Physbase`, `Logbase` | Bildschirmspeicher, keine Struktur: `physbase()` `references/emutos/bios/screen.c:1042`, `logbase()` `:1071` (`v_bas_ad`) | Bildschirm | `0x3DA700` / `0x369F00` |
| bios-r1:98–100; xbios-chardev-r1:21–23 | `Iorec(0/1/2)` | `IOREC` `references/emutos/bios/iorec.h:20-27` (`buf` L, `size`, `head`, `tail`, `low`, `high` W); Geraet 0: `EXT_IOREC` (`in`, `out`, 5 UBYTE) `references/emutos/bios/serport.h:55-65`, Geraet 1/2 plain `IOREC` (`ikbdiorec`, `midiiorec`) — `references/emutos/bios/xbios.c:339-344` | 14 B (1, 2); 33 B + Pad (0) | 0: `0x4A54` / `0x478A`; 1: `0x30D4`; 2: `0x30E2` |
| bios-r1:109; xbios-chardev-r1:34 | `Keytbl(-1,-1,-1)` | `struct keytbl` (`norm`, `shft`, `caps`, `altnorm`, `altshft`, `altcaps`, `dead` je 4 B; `features` W) `references/emutos/bios/ikbd.h:57-85`; Rueckgabe `&current_keytbl` `references/emutos/bios/ikbd.c:121-133` | 30 B | `0x40AE` / `0x40AE` |
| bios-r1:136; xbios-chardev-r1:37 | `Kbdvbase()` | `struct kbdvecs` (9 Funktionszeiger) `references/emutos/include/biosdefs.h:105-116`, Rueckgabe `&kbdvecs` `references/emutos/bios/xbios.c:716-719` | 36 B | `0x30F8` / `0x30F8` |
| bios-r1:184; xbios-chardev-r1:44 | `Bconmap(-2)` | `BCONMAP` (`maptab` L, `maptabsize` W, `mapped_device` W) `references/emutos/bios/serport.h:115-119`, Rueckgabe `&bconmap_root` `references/emutos/bios/serport.c:1092-1093` | 8 B | `0x4A76` / `0x4A76` |
| bios-core-r1:46–57 | `Setexc(n,-1)`, 9 CPU-Vektoren + `etv_*` | keine Struktur: der Wert ist die gelesene Vektorzelle `*(LONG*)(4*num)` `references/emutos/bios/bios.c:1072-1076`, also eine Code-Adresse | — | `0xE00774` … `0xFA002A`, `0xE2A190`, `0xE2FFE4`, `0xE20EA4` (beide Profile gleich) |
| bios-core-r1:60, 61 | `Setexc(0x168/0x169,-1)` | Systemvariable `_p_cookies` (`$5A0`), `ramtop` (`$5A4`); Wert, nicht Struktur — gleiche Zeile `references/emutos/bios/bios.c:1072-1076` | — | `0x8760`; `0x2000000` |
| bios-r2:42, 97, 108 | `Getbpb(2)` | `BPB`, neun WORDs `references/emutos/include/biosdefs.h:67-79` | 18 B | `0x3254`; Capture `r:18.W` bit-gleich in beiden Profilen |

Fazit der Liste: **nur vier Faelle-Gruppen haben eine echte Struktur hinter dem
Zeiger und sind heute ohne Capture** (`Iorec`, `Keytbl`, `Kbdvbase`,
`Bconmap(-2)`); `Getbpb` hat sie bereits (`r:18.W`); `Physbase`/`Logbase`/`Setexc`
liefern Adressen ohne Struktur.

## 3. Was der Orakel- und der NewTOS-Wert sagt (gelesen, nicht geschlossen)

- Der kleinste Orakel-PTR_RAM-Wert ueber alle Referenzen ist `0x30D4`
  (`Iorec(1)`, beide Profile); alle `Setexc`-Werte liegen im ROM (`>= 0xE00000`)
  oder sind `_p_cookies`/`ramtop`. Eine Grenze unterhalb von `0x30D4` besteht das
  Orakel mit Abstand.
- NewTOS-Werte (z. B. Lauf `measurements/measurement-2026-10-28-xbios-system-audit/runs/NEWTOS-bios-r1`, Datei Unterordner cmp, NEWTOS.BIN,
  mit `decode.py` gelesen): `Physbase` `0x3C100000`, `Iorec` `0x0E`, `Keytbl`
  `0x768`, `Kbdvbase` `0x7F010`, `Bconmap(-2)` `0x2C`; `Setexc(0x168)` `0x5F0`;
  `Getbpb` `0x266024`.
- `Keytbl` → `0x768` ist eine **gueltige** Antwort: NewTOS legt `struct keytbl` samt
  Tabellen im Page-0-Bereich des Prozesses an (`tos-api/src/bdos/process/loader.rs:1032@KEYTBL_AREA_VA`,
  ADR-023 in `docs/MEMORY.md`), und der Audit `docs/audits/2026-10-28-xbios-chardev.md`
  (Sonde K01–K03) bestaetigt die Struktur und die Tabellenbytes als gleich dem
  EmuTOS-(US-)Inhalt. Eine Untergrenze darf `0x768` deshalb nicht ausschliessen.
  Gleiches gilt fuer `Setexc(0x168)` → `0x5F0` (Seite 0). Beides *gelesen*
  (Audit, ADR); die Inhaltsgleichheit habe ich nicht neu gemessen.

## 4. Vorschlag (Option A, empfohlen): Untergrenze `0x400`

Regel in `retclass_violation` (`tosdump/host/tosdiff.c:524@retclass_violation`),
neben der Null- und der Obergrenzenpruefung (`:531@TD_RET_PTR_RAM`):

```
v < 0x400  ->  "below 0x400"      (TD_PTR_LOW, wie TD_ADDR_LIMIT in core/record.h)
```

Begruendung aus der Quelle:

- `references/emutos/tosvars.ld:31` setzt `sysvars_start = 0x400`; darunter liegen
  die m68k-Ausnahmevektoren (`0x000–0x37F`) und der Prozessabbild-Bereich
  (`_proc_stk = 0x3cc`, `tosvars.ld:28`). Keine der hier geprueften Funktionen gibt
  einen Zeiger in diesen Bereich zurueck: ein Zeiger auf eine Struktur oder
  einen Puffer liegt darueber (Orakel: `>= 0x30D4`). Die XBIOS-/BIOS-Funktionsnummern
  (`0x00..0xFF`) und alle kleinen Fehlercodes liegen darunter. (Schluss aus
  Quelle plus Orakelwerten, kein Beleg fuer jede denkbare Funktion; die
  Obergrenze ist die Aussage „alle PTR_RAM-Faelle der Tabellen oben“.)
- NewTOS' gueltige Seite-0-Antworten (`0x5F0`, `0x768`) bleiben erlaubt, weil die
  Grenze unter dem Systemvariablenbereich beginnt.

Warum die Regel am Orakel besteht (Schluss aus den dekodierten Werten, ausserdem
laufen lassen): `tosdump/tests/test_selfcheck.sh tosdump/host/build` heute →
`ok (25 references …)`. Alle PTR_RAM-Orakelwerte in der Tabelle von Abschnitt 2
sind `>= 0x30D4`; `Setexc`-Ergebnisse stammen aus dem ROM bzw. sind `0x8760` und
`0x2000000`. Die Regel aendert nur den Fall `v < 0x400`, der in keinem
Orakelwert vorkommt. Der Developer bestaetigt es durch denselben Lauf nach
der Aenderung (Abnahmepunkt 1 des Tickets).

Was sie auf den vorhandenen NewTOS-Dumps **neu** kippt (Skript im Scratchpad, die
Regel auf allen `NEWTOS*.BIN` mit Format 3 und Bereich 1 angewendet, Tabelle je
Dump ueber die Fallzahl erkannt):

| Dump | neu abweichend |
|---|---|
| bios-r1, 10 Laeufe (`measurement-2026-10-27-dumpbios/runs/cmp`, `-10-28-bios-core-audit/runs/NEWTOS-bios-r1`, `-bios-storage-audit/runs/NEWTOS-dumpbios`, `-tool08/runs/r1`, `-tool08/runs-control/r1`, `-tool09/runs/r1-argtrace-head`, `-tool09/runs/r1-c3db-clone`, `-xbios-chardev-audit/runs/NEWTOS-bios-r1`, `-xbios-system-audit/runs/NEWTOS-bios-r1`, jeweils die Datei Unterordner cmp, NEWTOS.BIN) | Fall 98, 99, 100 (`Iorec` = `0x0E`), 184 (`Bconmap(-2)` = `0x2C`) |
| xbios-chardev-r1 (Lauf `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1`, Datei Unterordner cmp, NEWTOS.BIN) | Fall 21, 22, 23 (`Iorec`), 44 (`Bconmap(-2)`) |
| bios-core-r1 (3 Dumps), bios-r2 (3 Dumps), xbios-system-r1 | nichts neu |

`Keytbl` (`0x768`) und `Kbdvbase` (`0x7F010`) bleiben gruen, `Setexc` verhaelt
sich wie bisher (die Fehler dort — NULL, 64-Bit-Platzhalter — fingen schon die
alte Regel; `docs/audits/2026-10-28-bios-core.md` BIOS-CORE-004/-005). Das sind
genau die vier Faelle aus dem Ticket, kein weiterer.

Grenze (ehrlich): die Regel erkennt nur *kleine Konstanten*. Ein NewTOS-Wert wie
`0x1000`, hinter dem nichts steht, bestuende sie. „Keine Struktur dahinter“ in
vollem Umfang prueft nur der Inhalt (Option B).

Aufwand Developer: `retclass_violation` +4 Zeilen, Konstante `TD_PTR_LOW` in
`tosdump/core/record.h` neben `TD_ADDR_LIMIT`, Kommentar im Zweig (dort steht
heute „the oracle returns such pointers“), `docs/TOS-REFERENCE-DUMPER.md` Abschnitt 6
(PTR_RAM-Satz), `test_tosdiff.sh`: ein Fall `PTR_RAM: 0x0E is flagged`, ein
Fall `0x3FF flagged / 0x400 fine`; `test_selfcheck.sh` ohne Aenderung (die
Funktionsnummer-Ablehnung ist ein Test in `test_tosdiff.sh`, die selfcheck-
Pruefung zeigt nur, dass das Orakel besteht). Der Stub gibt `14*0x10000+fn`
(`tosdump/tests/stub_abi.h:145@TD_STUB_TRAP_RET`) und `0xABCD00` zurueck, beides weit ueber `0x400`. Aufwand
klein: unter einer Stunde plus Lauf.

## 5. Alternativen

| Option | Inhalt | Kosten | Besteht Orakel | Faengt |
|---|---|---|---|---|
| A (empfohlen) | Untergrenze `0x400` | ~10 Zeilen C + 2 Tests + Doku | ja (Min. `0x30D4`) | Funktionsnummern, kleine Fehlercodes; nicht `0x1000` |
| A′ | wie A, aber pro Fall (neue Spalte/Retclass `PTR_RAM>=<lo>`) | Tabellenformat, Parser (`tosdump/core/cases.c`), Tests: etwa ½ Tag | ja | wie A, aber je Fall enger (z. B. Seite 0 nur dort erlaubt, wo es Absicht ist) |
| B | `r:<len>` durch den Rueckgabezeiger, EXACT-Vergleich der Struktur | keine Tosdiff-Aenderung; Tabellenzeilen + Referenzen neu (Hatari je Profil) | ja, wenn Offsets/Einheit stimmen | Struktur wirklich da — aber nur layoutneutrale Felder |
| C | nur „v != Aufrufnummer“ ausschliessen | ~6 Zeilen, `opcode` ist im Record | ja | nur genau dieses Muster; schon durch A abgedeckt |
| D | neue Retclass `PTR_STRUCT` mit „Zielbereich lesbar und nicht nur Nullbytes“ | Runner (Lesen unter Obergrenze), Record, Vergleicher, Doku: 1–2 Tage | ja? (nicht geprueft) | Nullbytes, aber nicht falsche Struktur |

Zu **B** im Einzelnen — der Mechanismus ist da: `r:<len>[.B|.W|.L][@<offset>]`
nimmt die Bytes hinter der zurueckgegebenen Adresse in die Capture
(`tosdump/core/cases.c:3630@parse_capture`, nur mit `PTR_RAM` zulaessig;
`docs/TOS-REFERENCE-DUMPER.md` „Capture through the returned pointer“), nicht
auf `!disk`-Tabellen beschraenkt. Wiederverwendbar ohne Code, **wenn** das Layout
auf beiden Seiten gleich ist:

- `Getbpb`: bereits so (`r:18.W`), Orakel und NewTOS gleich.
- `Iorec`: sinnvoll waere `r:10.W@4` (`size`, `head`, `tail`, `low`, `high`;
  Orakel zeigt `ibufsiz` 256/256/128, Wassermarken 64/192 bzw. 32/96, Audit
  XBIOS-CHARDEV-001). Das legt aber fest, dass NewTOS' `IOREC` den Zeiger in 4
  Bytes fuehrt (Offset 4); mintlibs LP64-`IOREC` hat ihn in 8 (Offset 8). Das ist
  die im Audit offene Vertragsfrage des `IOREC`-Abbilds (Researcher-Auftrag
  dort) — ich entscheide sie hier nicht. `head`/`tail` sind `volatile` und aendern
  sich bei Tastendruck; vorsichtshalber nur `size`, `low`, `high` capturen
  (`@4`, `@10`, `@12`, mehrere Zeilen waeren noetig, ein Capture ist
  zusammenhaengend: `r:10.W@4` schliesst `head`/`tail` ein — im Dump ohne
  Eingabe 0, nicht gemessen).
- `Bconmap(-2)`: `r:4.W@4` = `maptabsize` + `mapped_device` (TT 4/6, F 3/7 laut
  Audit); gleiche LP64-Frage (Offset 4 gegen 8).
- `Keytbl`/`Kbdvbase`: die Struktur besteht aus Zeigern (Adressen, nie gleich,
  `docs/analysis/ORACLE.md` §8), nur `features` waere ein Wert; bei LP64 liegt er
  an einem anderen Offset. **Nicht durch `r:` zu fassen**; hier bleibt A.

Fazit: B ist die richtige Antwort fuer `Iorec`/`Bconmap`, sobald NewTOS sie
implementiert und der Vertrag steht (Lead-Entscheid, ggf. COMPATIBILITY-Eintrag);
A schliesst die Luecke jetzt.

## 6. Befund: der Laeufer folgt dem Zeiger ohne Obergrenze (Defekt-Kandidat, nicht behoben)

`tosdump/core/runner_call.c:605@TD_CALL_CAP_RET` liest hinter dem Rueckgabewert,
sobald er `> 0` und gerade ist — **ohne** die Grenzen von `PTR_RAM`. Bei einem
NewTOS-Wert wie dem 64-Bit-Platzhalter `0x4E75…` (BIOS-CORE-005) oder
`>= 0x80000000` waere das ein Datenabbruch im EL0-Dumper, der Lauf ginge verloren,
statt dass tosdiff `not < 0x80000000` meldet. Heute trifft es keinen `r:`-Fall
(nur `Getbpb`). Vor Option B fuer `Iorec`/`Bconmap` muss der Laeufer dieselbe
Spanne (`>= TD_PTR_LOW`, `< TD_ADDR_LIMIT`) einhalten; sonst liest er bei `0x0E`
zwar Seite 0 (harmlos), bei anderen Fehlwerten aber ins Leere. Das ist eine
Developer-Aenderung (`tosdump/core/`), kein Teil dieses Auftrags.

## 7. Nicht belegt / offen

- Kein Beleg, dass jeder moegliche Wert unter `0x400` falsch ist (z. B. eine
  kuenftige Funktion, die auf `0x380`-Strukturen zeigt). Fuer die oben
  gelisteten Faelle gilt es.
- Die Orakelwerte kommen aus den zwei Profilen TT/Falcon-TC; ein ST-Profil
  liegt nicht vor.
- `head`/`tail`-Stabilitaet im Dump ohne Eingabe: nicht gemessen.
- Ob `Bconmap(-2)` → `0x2C` eine akzeptierte Abweichung ist: Triage.

## 8. Out-of-scope

- `docs/api/xbios/2c-bconmap.md` waehlt „single-serial build“; der Referenzbau
  hat `BCONMAP_AVAILABLE` (Audit XBIOS-CHARDEV-005) — Spec-Korrektur ist Sache
  dieses Audits.
- `TASK/TOOL11.md` nennt 18 Referenzen; es sind jetzt 25 (`test_selfcheck.sh`).
