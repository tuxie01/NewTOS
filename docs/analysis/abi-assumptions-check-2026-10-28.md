# ABI-Annahmen: Gegenpruefung gegen Quelle und Messung (2026-10-28)

Stand: `main` bei `a36ea390`. Dateien nur gelesen; nichts gebaut, kein Hatari, kein QEMU.
EmuTOS-Zitate zeilenbasiert gegen den Pin `1a7484aba1bfba23e2c0899e243b6d8177fe41df`
(`docs/analysis/ORACLE.md` §1, `references/emutos`); NewTOS-Zitate als `<pfad>:<zeile>@<symbol>`.
Kennzeichnung je Aussage: **READ** (aus Quelltext gelesen), **MEASURED** (Orakel-/NewTOS-Lauf mit
Fundstelle), **DERIVED** (aus READ geschlossen, nicht beobachtet). Keine Entscheidungen, keine
Empfehlungen; nur was die Belege hergeben und wo sie fehlen.

Nicht durchsucht: das Innere von `references/freemint` (haengt). Programmsuche mit `/usr/bin/grep`
ueber `sdk/apps/`, `sdk/overlays/`, `sdk/examples/`, `programs/`, `Applications/` (ohne
`Applications/**/.claude/worktrees`, das sind Agent-Arbeitsbaeume mit Kopien) sowie `sdk/cflib`,
`sdk/windom`, `sdk/zview`, `sdk/ldg`, `sdk/libc` (Overlays, `newtos-arch`, mintlib-Quelle).

---

## 1. Wo steht „LONG-Ergebnisse sind auf 64 Bit vorzeichenerweitert“, und wer weicht ab

### 1.1 Definitionsstellen — READ

| Stelle | Wortlaut / Inhalt |
|---|---|
| `tos-api/src/trap.rs:6@TrapArgs` | „return value in `x0` (sign-extended `i64`)“ (Modulkopf) |
| `kernel/src/arch/aarch64/exceptions.rs:758@result` | `frame.gpr[0] = result as u64;` — der einzige Schreibpunkt von `x0` fuer den Normalfall |
| `kernel/src/arch/aarch64/exceptions.rs:505@dispatch` | `tos_api::gemdos::dispatch(&args) as i64` — GEMDOS liefert `Long = i32` (`tos-types/src/lib.rs:33@Long`), der Cast `i32 as i64` ist die Vorzeichenerweiterung |
| `kernel/src/arch/aarch64/exceptions.rs:577@Bios` / `kernel/src/arch/aarch64/exceptions.rs:579@Xbios` | BIOS/XBIOS liefern schon `i64`; die Erweiterung geschieht **in den Dispatchern** (1.3) |
| `docs/api/_template.md:90-92` | Vorlage: WORD/LONG „sign-extended into `x0`“; Zeiger „occupies the full 64-bit `x0`, **not** sign-extended from 32 bits“ |
| `docs/sdk/05-abi-pitfalls.md:26` | „`x0`, a sign-extended `i64` of the TOS LONG; BIOS `Getbpb` returns a full 64-bit pointer“ (die dort genannten Zeilen 359-362 und 434 der Datei `exceptions.rs` sind veraltet; der heutige Ort ist `kernel/src/arch/aarch64/exceptions.rs:758@result`) |
| `docs/TOS-REFERENCE-DUMPER.md:267` und `:271-276` | `ret_hi`: auf dem m68k „the sign extension of the low half“; Vergleich immer als 64-Bit-Wert `ret_hi:ret` |
| `docs/TOS-REFERENCE-DUMPER.md:630` | Klasse `EXACT`: „a negative LONG that NewTOS returns zero-extended in `x0` is a difference“ |
| `docs/ARCHITECTURE.md:53` | „sign-extended `i64`; `tos-api/src/trap.rs`“ |
| `docs/api/xbios/_dispatcher.md:203` | „WORD/LONG sign-extended, pointer full-64-bit“ |
| `docs/api/bios/05-setexc.md:262` | Seite-0-Zelle: „old value returned **sign-extended** `as i32 as i64`“ |

`docs/COMPATIBILITY.md` enthaelt **keine** Regel „LONG-Ergebnisse vorzeichenerweitert“. Treffer
`:151` handelt vom GEM-Selektor in `x0` (Eingang), `:476` von `gem_wire_long`
(Parameterfeld). Das ist eine Luecke: die Regel steht in `trap.rs`, `_template.md`, `05-abi-pitfalls.md`,
dem Dumper-Dokument und `ARCHITECTURE.md`, nicht im Kompatibilitaetsvertrag.

Die Regel hat zwei Haelften (`docs/api/_template.md:90-92`): LONG vorzeichenerweitert, Zeiger voll 64 Bit.
Beide sind im Code nur verlaesslich, weil jede Benutzeradresse unter 2 GiB liegt
(`docs/sdk/05-abi-pitfalls.md:67` „Every user address is below 2 GiB (plan invariant I2)“) — dann sind
Vorzeichen- und Nullerweiterung desselben 32-Bit-Zeigers gleich **[DERIVED]**.

### 1.2 SDK-Seite: wie `long` aus `x0` kommt — READ

- `sdk/libc/overlay/include/mint/osbind.h:571@Setexc`: `(void (*)(void))trap_13_wwl(...)` — das
  Makro nimmt die vollen 64 Bit von `x0` und wirft sie auf einen Zeiger; **keine** Verengung auf 32 Bit.
- Dasselbe Muster bei `Getbpb` (`sdk/libc/overlay/include/mint/osbind.h:575@Getbpb`: `(void *)trap_13_ww`) und `Drvmap`/`Kbshift`
  (`(long)trap_13_w`). Der Kopf erklaert es als „the result comes back in x0 (a sign-extended TOS LONG;
  BIOS Getbpb returns a full 64-bit pointer)“ (`sdk/libc/overlay/include/mint/osbind.h:139@x0`, Kommentarblock).
- `sdk/libc/overlay/include/mint/mintbind.h:249@Ssystem`: `trap_1_wwll(0x154, ...)`; das Ergebnis ist
  `long` = 64 Bit aus `x0`.
- Rust: `programs/newtos-gem/src/gemdos.rs:453@dos_getcookie` testet `rc >= 0` und nimmt `rc as u32`.

### 1.3 Dispatcher-Scan — Ergebnisse, die von der Regel abweichen

Gescannt: `tos-api/src/bios/mod.rs`, `tos-api/src/xbios/mod.rs`, `xbios/system.rs`, `xbios/screen.rs`,
`xbios/kbdvecs.rs`, `xbios/keytbl.rs`, `bios/vectors.rs`, `bios/bpb.rs`, `bios/getmpb.rs`,
`gemdos/mod.rs`, `gemdos/time.rs`, `gemdos/ssystem.rs`, Muster `as i64`, `as u32 as i64`, `as u64 as i64`,
`as Long`, `u64::from(`. Alle Treffer sind unten klassifiziert.

**A. Echte Abweichung von „LONG vorzeichenerweitert“ (Wert mit Bit 31 moeglich, Erweiterung null):**

| # | Ergebnis | Stelle | Befund | Beleg |
|---|---|---|---|---|
| A1 | `Rsconf` (XBIOS 0x0F), Pack-Rueckgabe `(ucr<<24)\|(rsr<<16)\|(tsr<<8)` | `tos-api/src/xbios/system.rs:295@old` (`old as i64`, `old: u32`) | nullerweitert; EmuTOS liefert `ULONG` (`references/emutos/bios/serport.c:217`, `:226`), auf dem m68k im `d0` | **MEASURED**: `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt:16` — `EMUTOS.BIN = 0xFFFFFFFF88018100`, `NEWTOS.BIN = 0x0000000088010100`. Die Spec `docs/api/xbios/0f-rsconf.md:80-90` fordert Vorzeichenerweiterung („pending XBIOS06“) — Spec und Code widersprechen sich; das Wort „zero-extended“ steht noch in den Kommentaren von `tos-api/src/xbios/system.rs` (Zeilen 262 und 294) |
| A2 | `Gettime` (XBIOS 0x17), gepacktes `date<<16\|time` | `tos-api/src/xbios/mod.rs:108@GETTIME` (`xbios_gettime() as i64`, `-> u32`: `tos-api/src/gemdos/time.rs:222@xbios_gettime`) | nullerweitert; ab Jahr 2044 ist Bit 31 gesetzt | **MEASURED NewTOS**: `measurements/measurement-2026-10-28-xbios-system-audit/runs/HIGH-NEWTOS-raspi4b/XSHI.LOG:1` — `Settime(8ccf6000) Gettime ret=8ccf6000 hi=00000000 negative-as-long=no`. Orakelseite **nicht gemessen** (`docs/audits/2026-10-28-xbios-system.md:305`: Hatari-RTC nimmt keine Schreibung); dort ist die Erwartung **DERIVED** aus `docs/TOS-REFERENCE-DUMPER.md:267` (m68k-`d0` wird als Vorzeichenerweiterung gelesen) |
| A3 | `Setexc` fuer `num` in `0x100..0x102` (`etv_timer/critic/term`), Abfrage `-1` | `tos-api/src/bios/vectors.rs:45@ETV_DEFAULT_JUST_RTS` (`0x4E75_0000_0000_0000`), `:54@ETV_DEFAULT_CRITIC` (`0x4372_6974_0000_0000`); Rueckgabe `VECTORS[num].load` ohne Verengung: `:104@VECTORS` | liefert ein volles 64-Bit-Muster, das weder eine 32-Bit-LONG-Adresse noch deren Erweiterung ist | **MEASURED**: `measurements/measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-core-r1/tosdiff-newtos-vs-emutos.txt` Faelle 55-57: `NEWTOS.BIN = 0x4E75000000000000, not < 0x80000000` / `0x4372697400000000`; Befund `docs/audits/2026-10-28-bios-core.md` BIOS-CORE-005 |
| A4 | `Iorec` (XBIOS 0x0E) | nicht implementiert: Fangarm `tos-api/src/xbios/mod.rs:138@other` (`other as i64`, `u16`) → 14 | positive Funktionsnummer statt Zeiger/`-1` | **MEASURED**: `measurements/measurement-2026-10-28-xbios-chardev-audit/runs/NEWTOS-bios-r1/tosdiff-newtos-vs-emutos.txt:27-31` (`Iorec(3)`/`Iorec(-1)`: EmuTOS `0xFFFFFFFFFFFFFFFF`, NewTOS `0x000000000000000E`). `Iorec(0..2)` stehen in `tosdump/cases/bios-r1.tsv:98-100@Iorec` als `PTR_RAM`; die Untergrenze `0x400` kam erst danach (`docs/TOS-REFERENCE-DUMPER.md:645-651`, nennt „`Iorec` returning its own `0x0E`“), die Lauf-Diffs oben enthalten diese Zeilen deshalb nicht — der Wert 14 folgt dort aus dem Code **[DERIVED]** |
| A5 | `Bconmap` (XBIOS 0x2C) | `tos-api/src/xbios/system.rs:106@bconmap` (`0x2c`) | feste 0x2C, auch fuer `-1` und `-2` | **MEASURED**: `…/NEWTOS-xbios-chardev-r1/tosdiff-newtos-vs-emutos.txt:18-19` (`Bconmap(-1)`: EmuTOS 6, NewTOS 0x2C). 0x2C ist EmuTOS' eigener Wert fuer „kein BCONMAP“ (`references/emutos/bios/serport.c:1124` `return 0x2c;` unter `#else` zu `BCONMAP_AVAILABLE`), das Orakel ist mit BCONMAP gebaut (liefert 6). Das ist eine Abweichung des Werts, nicht der Erweiterung |

**B. Zeiger — volle 64 Bit per Vertrag (`docs/api/_template.md:91`), kein LONG:**

| Ergebnis | Stelle | Anmerkung |
|---|---|---|
| `Getbpb` | `tos-api/src/bios/bpb.rs:296@getbpb` (`-> i64`); EL0: Benutzeradresse `user_va as i64` (`:317`), EL1-Aufrufer: Kernel-VA `kva as i64` (`:309`) | Benutzeradresse < 2 GiB **[DERIVED]** (I2). **MEASURED** `sx=1` fuer `Getbpb(2)`: `measurements/measurement-2026-10-28-bios-storage-audit/runs/NEWTOS-raspi4b/BSTPROBE.LOG:3` („sx = the native register is the sign-extended LONG“, `measurements/measurement-2026-10-28-bios-storage-audit/RUNINFO.TXT:54`) |
| `Getmpb` | `tos-api/src/bios/getmpb.rs:84@getmpb` (`-> Long`) | gibt 0 zurueck; der Zeiger geht in den MPB, nicht ins Register |
| `Kbdvbase` | `tos-api/src/xbios/kbdvecs.rs:198@kbdvbase` | volles 64-Bit-`u64 as i64`; `USER_VA_BASE = 0` (`tos-api/src/bdos/process/loader.rs:200@USER_VA_BASE`) |
| `Keytbl` | `tos-api/src/xbios/keytbl.rs:185@keytbl` | wie oben |
| `Physbase`/`Logbase` | `tos-api/src/xbios/screen.rs:121@physbase`, `:133@logbase` (`caller_view(..) as i64`) | wie oben |
| `Setexc`, Seite-0-Zellen (`num < 0x200`) | `tos-api/src/bios/vectors.rs:131@read_unaligned` (`as i32 as i64`) | **vorzeichenerweitert**, also regelkonform; Schreibseite schneidet auf 32 Bit: `:121@put_user_u32`, `:134@write_unaligned` (`vector as u32`) |
| GEMDOS `Malloc`/`Mxalloc` | `tos-api/src/bdos/mem/traps.rs:672@malloc` (`-> Long`), Dispatch `kernel/src/arch/aarch64/exceptions.rs:505@dispatch` (`as i64`) | `i32`, also vorzeichenerweitert; richtig nur, solange der Pool < 2 GiB liegt (`docs/sdk/05-abi-pitfalls.md:99`) **[DERIVED]** |
| GEMDOS `Super` | `tos-api/src/bdos/process/superstate.rs:66@super_call` (`-> i64`), Aufruf `kernel/src/arch/aarch64/exceptions.rs:1857@super_call` | volle 64 Bit |
| `Pexec` Modus 5 (Basepage) | `kernel/src/arch/aarch64/exceptions.rs:1273@pd_of` (`as u64`) | volle 64 Bit |

**C. Absichtlich nullerweitert, gleich EmuTOS:** `Pexec`-Rueckgabe des Kindes
`tos-api/src/bdos/process/pterm.rs:126@exit_code_for_caller` (`u64::from(rc as u16)`); EmuTOS speichert
`UWORD rc` in `run->p_dreg[0]` (`references/emutos/bdos/proc.c:601`, `:616`); Entscheidung
`docs/api/_discrepancies.md:101-121` (D-005). Die VDI-`flip_y`-Nullerweiterung ist im Kommentar
`kernel/src/arch/aarch64/exceptions.rs:506-509@flip_y` als `vdi_asm.S:124-125` zitiert (aus dem Kommentar gelesen, **nicht** gegen
`vdi_asm.S` geprueft).

**D. Treffer ohne Abweichung (Wert kann Bit 31 nicht haben, oder gleiches Verhalten wie EmuTOS):**
`tos-api/src/bios/mod.rs:88-99@dispatch` (`Long = i32` → `as i64` vorzeichenerweitert), `tos-api/src/bios/mod.rs:95@TICKCAL` (`TICKCAL_MS = 20`),
`tos-api/src/bios/mod.rs:106@other` und `tos-api/src/xbios/mod.rs:138@other` (Funktionsnummer `u16`, positiv — wie EmuTOS' `bx_ret_exc`,
`references/emutos/bios/vectors.S:1323` / `references/emutos/bios/vectors.S:1334`, `moveq #0,d0; move.w (sp)+,d0`),
`tos-api/src/xbios/mod.rs:80@EUNDEV` (negativ, `as i64` auf `Long` → vorzeichenerweitert),
`tos-api/src/xbios/system.rs:27@RSEED` (24 Bit), `tos-api/src/xbios/system.rs:196@old` und `tos-api/src/xbios/system.rs:233@old` (`old as i16 as i64`, WORD vorzeichenerweitert),
`tos-api/src/xbios/system.rs:272@RS_BAUD` (`u16` 0..15), `tos-api/src/gemdos/time.rs:137@tgetdate`, `tos-api/src/gemdos/time.rs:142@tgettime` (WORD-Werte ≤ 0xFFFF),
`tos-api/src/gemdos/ssystem.rs:371@Long` (siehe 3: Wertform `v as Long`, vorzeichenerweitert),
`tos-api/src/bios/mod.rs:313@drvmap` (`drvbits() as Long`, 32-Bit-Maske).

### 1.4 Offene Fragen aus 1 — ohne Messung

- **Setexc-Vergleich auf `-1`:** `tos-api/src/bios/vectors.rs:100@vector` nimmt `args.arg_u64(1) as i64` und testet
  `vector != -1` auf 64 Bit (`tos-api/src/bios/vectors.rs:105@vector`, `:121@vector`, `:132@vector`). EmuTOS testet ein `LONG`
  (`references/emutos/bios/bios.c:1074`). Ein Aufrufer, der `0x00000000FFFFFFFF` (null-erweitertes
  `-1`) in `x1` legt, wird als Setzen behandelt **[DERIVED]**. Alle gefundenen Aufrufer (Q2) schreiben
  `-1` als `long`/`(void(*)())-1`, also vorzeichenerweitert; es gibt keinen Treffer mit `(unsigned)-1`.
  **Vorgeschlagener Fall:** Sondenschritt analog `sdk/apps/apitest/apitest.c:2242-2252@SHADOW_VEC` (BS13-BS15),
  `SHADOW_VEC` (`0x20`) mit `0x80000001` setzen, mit `-1L` und mit `0xFFFFFFFFL` abfragen, zurueck.
  Als Tabellenzeile geht das nicht: die Aufrufpolitik von `tosdump/core/cases.c` erlaubt bei Setexc nur
  die Abfrage (`tosdump/cases/bios-core-r1.tsv:10@Setexc`).
- **`Gettime` mit Bit 31 am Orakel:** nicht messbar (Hatari-RTC nimmt keine Schreibung, s. A2); die
  Erwartung bleibt abgeleitet.
- **`Getbpb`-Zeigerwert:** das Log druckt nur `nz`/`sx`, nicht den Zeiger; ob er < 2 GiB ist, ist
  nicht gemessen. Vorschlag: `tosdump/cases/bios-r1.tsv` hat bereits `Getbpb`-Faelle; die Klasse
  `PTR_RAM` (`< 0x80000000`, `docs/TOS-REFERENCE-DUMPER.md:636`) prueft es, sobald die Zeile
  `PTR_RAM` statt `NONZERO` nutzt — nicht geprueft, welche Klasse die Zeile heute hat.

---

## 2. Setexc: EmuTOS-Verhalten und wie portierte Programme das Ergebnis nutzen

### 2.1 EmuTOS — READ

`references/emutos/bios/bios.c:1070-1080`:

```
LONG setexc(WORD num, LONG vector)
{
    LONG oldvector;
    LONG *addr = (LONG *) (4L * num);
    oldvector = *addr;
    if(vector != -1) {
        *addr = vector;
    }
    return oldvector;
}
```

- Rueckgabe in jedem Fall der **alte** Zellinhalt `*(LONG*)(4*num)`, 32 Bit (`LONG`) im `d0`.
  `vector == -1` ist nur die Abfrage; jeder andere Wert (auch `0`) wird gespeichert.
- Keine Bereichspruefung auf `num`, kein Sonderfall fuer `etv_*` oder die Systemvariablen; `num` ist
  `WORD`, `4L * num` ist vorzeichenbehaftet (negatives `num` adressiert unterhalb 0).
- Der Rahmen `bios_5` (`references/emutos/bios/bios.c:1083-1088`) ist nur der `DBGBIOS`-Wrapper; Tabelleneintrag
  `VEC(bios_5, setexc)` bei `references/emutos/bios/bios.c:1258`.
- Trap-Bindung: `references/emutos/bios/vectors.S:1323-1334` lehrt `d0` mit `moveq #0,d0; move.w (sp)+,d0`, springt in die
  Tabelle (`jsr (a0)`), das Ergebnis bleibt in `d0`; `d0` ist ein 32-Bit-Register, das `LONG` ist der
  ganze Wert. Der m68k-mintlib-Binder ist `Applications/mintlib-src/mintlib-0.60.1/include/mint/osbind.h:1165`
  (`Setexc(vnum,vptr)`), das Ergebnis geht als `(void (*)(void))` heraus.
- NewTOS-Sicht: siehe 1.3 A3 (ETV-Platzhalter, 64-Bit), 1.3 B (Seite-0-Zellen vorzeichenerweitert,
  Schatten-Slots 64 Bit), 1.4 (Vergleich auf 64 Bit).
- **MEASURED** am Orakel: `tosdump/cases/bios-core-r1.tsv:46-62@Setexc` (Abfrage `-1`, Klasse `PTR_RAM` fuer
  Adressen, `EXACT` fuer `0x103`, `0x108`, `0x16A`), Referenz
  `tosdump/reference/EMUTOS-1a7484ab-BIOS-core.LOG`. NewTOS-Diff:
  `measurements/measurement-2026-10-28-bios-core-audit/runs/NEWTOS-bios-core-r1/tosdiff-newtos-vs-emutos.txt`
  (15 von 44 Faellen verschieden; Setexc-Faelle 46-54 liefern `0`, 55-57 die 64-Bit-Platzhalter, 59
  `memvalid` `0` statt `0x752019F3`; die Seite-0-Faelle 60 `0x5A0` und 61 `0x5A4` bestehen).

### 2.2 Verwendungen in portierten Programmen

Suchbasis: `Setexc`/`setexc` ueber `sdk/apps/`, `sdk/overlays/`, `sdk/examples/`, `programs/`,
`Applications/` (ohne `.claude/worktrees`), `sdk/cflib`, `sdk/windom`, `sdk/zview`, `sdk/ldg`,
`sdk/libc/newtos-arch`, `tosdump`. Kein Treffer in `programs/` (Rust), `sdk/examples/`,
`sdk/cflib`, `sdk/windom`, `sdk/zview`, `sdk/ldg`, `tosdump`. (`setexcl` in QED ist eine Mengenfunktion,
kein Setexc.) Klassen: **A** = mit `-1` verglichen, **B** = `< 0` getestet, **C** = gespeichert und als Zeiger
benutzt, **D** = gespeichert und nur zurueckgeschrieben, **E** = Ergebnis verworfen, **F** = sonstige.

| Klasse | Datei:Zeile | Verwendung |
|---|---|---|
| C | `sdk/apps/teradesk/library/utility/cookie.c:56@Setexc` | `cookie = (COOKIE *)Setexc(0x5A0 / 4, (void (*)())-1);` dann `cookie == NULL`, Schleife `cookie->name != 0`, `cookie++`; Typ `long name; long value;` (`sdk/apps/teradesk/library/xdialog/library.h:205@COOKIE`) — **gebaut** (`-D_TOS_ONLY`, `sdk/apps/build-teradesk.sh`) |
| C | `sdk/apps/highwire/src/romvdi.c:46@Setexc` | `cookiejar = (COOKJAR *)Setexc(0x5A0/4, (void (*)())-1);`, `if (cookiejar)`, `cookiejar[i].id` |
| C | `sdk/apps/highwire/src/AEI.c:72@Setexc` | `long * ptr = (long *)Setexc(0x5A0/4, ...-1);`, `if (!ptr)`, `ptr += 2` |
| C | `sdk/apps/highwire/src/inet.c:215@Setexc` | `jar = (void*)Setexc (0x5A0 /4, ...-1)`, `if (jar) while (jar->cktag)` |
| C | `sdk/apps/highwire/tools/num_char.c:171@Setexc` | wie `romvdi.c` |
| C | `Applications/z-tools/zweather/inet.c:23` | `jar = (void*)Setexc (0x5A0 /4, ...-1)`, `while (jar->cktag)` (ohne NULL-Pruefung) |
| D | `sdk/apps/teradesk/startprg.c:111@old_critic`, `sdk/apps/teradesk/startprg.c:117@old_critic` | `old_critic = Setexc(0x101, new_critic)` und `Setexc(0x101, old_critic)`; `old_critic` nur an diesen zwei Stellen und der Deklaration `sdk/apps/teradesk/startprg.c:55@old_critic` benutzt, **nie aufgerufen** — gebaut |
| D | `Applications/Vanilla-Conquer-Atari-ST/tiberiandawn/atarilib/ikbd.cpp:436`, `:453` | `PrevIKBDVector = Setexc(IKBD_VECTOR_NUMBER, ISR)`, `if (PrevIKBDVector) Setexc(..., PrevIKBDVector)`; nirgends aufgerufen oder verkettet (nur `:42`, `:436`, `:452-456`) |
| D | `Applications/Vanilla-Conquer-Atari-ST/tiberiandawn/atarilib/st_slow_hbl.cpp:36`, `:49` | `Old_Hbl = Setexc(HBL_VECTOR_NUM, ISR)` und Rueckschreiben; nicht aufgerufen |
| E | `sdk/overlays/teradesk/main.c:1754-1756@Setexc` (Vorlage `sdk/apps/teradesk/main.c:1730-1732@Setexc`) | `(void) Setexc(0x070, loopcpu)` u. a.; Ergebnis verworfen (Funktion `lobo`, `!ct60`-Zweig des Abschaltens) |
| F | `sdk/apps/apitest/apitest.c:2235-2255@SETEXC` (Makro `sdk/apps/apitest/apitest.c:532@SETEXC`) | als `long` durchgereicht: `q1 != 0` (Zeile 2239), `q1 == q2` (2240), `prev == old` (2245), `back == old` (2252), Hex-Vergleich mit `RD_U32(SV_PCOOKIES)` (2255). Kein Vergleich mit `-1` auf der Rueckgabe |
| — | `sdk/apps/doom/doomgeneric_gem_kbdvec.h:63@Setexc` | nur Kommentar |

Ergebnis: **Kein** Treffer vergleicht die Rueckgabe mit `-1` (A) oder testet sie auf `< 0` (B). Die
Zeiger-Treffer (C) lesen ausschliesslich `0x5A0/4` und gehen durch die Seite-0-Zelle
(`tos-api/src/bios/vectors.rs:131@read_unaligned`, vorzeichenerweitert, ergibt `0x5F0`). Kein Treffer **ruft** den alten Vektor auf oder
verkettet XBRA; Klasse D speichert nur zum Zurueckschreiben. (Mintlib-Makroform
`(void (*)(void))` `sdk/libc/overlay/include/mint/osbind.h:571@Setexc`; ausgewertet wird sie stets wie oben.)

Auf der Standard-Abbildung (`tools/make-sd-image.sh`): `DESKTOP.NAP` (TeraDesk: `cookie.c`,
`startprg.c`, `main.c`) — `REQUIRED` `tools/make-sd-image.sh:77@DESKTOP`. Nicht auf der Standard-Abbildung:
HighWire (`sdk/apps/build-*.sh` kennt es nicht), `Applications/z-tools/zweather`, Vanilla-Conquer,
`apitest` (`APITEST.NCP`; steht auf der Abnahmekarte
`measurements/measurement-2026-10-27-mem12-acceptance/cards/api.img.manifest`, gebaut von
`sdk/apps/apitest/build-apitest.sh`).

**Karten-Manifest-Abgleich (Luecke „kein lokaler Quelltext“):** `measurements/measurement-2026-10-27-mem12/base.img.manifest`
(Standard-Abbildung, `tools/make-sd-image.sh`): jede Programmdatei dort — `SYSACC.NAC`, `ARGVTEST.NAP`,
`MXPROBE.NCP`, `PRESENT.NAP`, `QED.NAP`, `DESKTOP.NAP`, `BIG.NCP`, `CLOCKTST.NCP`, `CMDLINE.NCP/.NTP`,
`CORE.NCP`, `COREVAL.NCP`, `CTRLCPAR.NCP`, `DTACHILD.NCP`, `DUMPVDI.NAP`, `FPPEXEC.NCP`, `HELLO.NCP`,
`HELLO2.NCP`, `EX_LIB.NLD`, `LDGTEST.NCP`, `LIBTEST.NCP`, `MATHTST.NCP`, `PHYSBASE.NCP`, `PTERMCH.NCP`,
`SCRFLIP.NCP`, `SETJMPT.NCP`, `CURDIR.NCP`, `SYSACC2.NAC`, `SYSINFO.NCP`, `USBSNAP.NCP`, `VDIDRAW.NAP`,
`ZVIEW.NAP` und die Codec-`*.NLD` unter `ZVIEW\CODECS` — entsteht aus `REQUIRED`
(`tools/make-sd-image.sh:60-82@REQUIRED`) oder `build_example` (`tools/make-sd-image.sh:100@build_example`, `sdk/examples/*.c`) bzw.
`tools/mk-hello-ncp.py` (`HELLO.NCP`, `tools/make-sd-image.sh:214@python3`, Erzeugnis aus Python, keine C-Quelle).
Die Luecke ist fuer diese Abbildung durch Konstruktion leer; die Zuordnung der einzelnen
Codec-`*.NLD` zum Rezept `sdk/zview/build-zview.sh` ist hier **nicht** Datei fuer Datei geprueft
**[offen, gering]**. Die Karte `measurements/measurement-2026-10-28-tool10/card.img.manifest` ist eine
Ein-Datei-Karte (`C:\NEWDESK.INF`), kein Programm.

---

## 3. Getcookie: Nullerweiterung im SDK, Kernel-Seite, Verwendung

### 3.1 Wo und warum das SDK nullerweitert

- **READ** `sdk/libc/newtos-arch/getcookie.c:35@v`: `long v = 0;`, dann
  `Ssystem(S_GETCOOKIE, cookie, (long)&v)` (`:37`) und `*p_value = v` (`:45`).
- **READ** Kernel: `tos-api/src/gemdos/ssystem.rs:366@put_u32` schreibt mit `args.space.put_u32(ret, v)`
  **vier Byte** (`v: u32`) nach `*arg2`. Die oberen vier Byte des 8-Byte-`long` bleiben die vorbelegte
  `0` → **Nullerweiterung durch Zusammenspiel von `= 0` und Vier-Byte-Schreiben**; im Code ist kein
  Erweiterungsschritt vorhanden **[READ]**.
- **Begruendung:** Weder im Kommentar von `getcookie.c` (Zeilen 1-26) noch in der Commit-Nachricht des
  einzigen Commits dieser Datei (`git log -S "long v = 0"` → `3a048314`, `feat(sdk): port cflib …`)
  steht ein Grund fuer die Breite oder die Vorbelegung. Der Kommentar nennt nur den Vertrag (`C_FOUND`/
  `C_NOTFOUND`, `*val = 0` bei Fehlschlag). Die Nullerweiterung ist also **ein Nebeneffekt, kein
  dokumentierter Entschluss** **[READ: kein Beleg fuer Absicht]**.
- Dieselbe Eigenschaft in der Querschnittsanalyse: `docs/analysis/lp64-tos-struct-survey-2026-10-28.md:99-104`
  („`v` ist mit `0` vorbelegt, `*p_value = v` nimmt acht Byte“) — **verifiziert** (Zeilen 35/45 und
  `tos-api/src/gemdos/ssystem.rs:366@put_u32` stimmen). `TeraDesks` `find_cookie` hat dieselbe Form (`cvalue = 0`,
  `sdk/apps/teradesk/library/utility/cookie.c:42-47`).

### 3.2 Kernel: zwei Formen — READ

`tos-api/src/gemdos/ssystem.rs:345@s_getcookie`:

- Zeigerform (`arg2 != 0`): `put_u32` nach `*arg2`, Rueckgabe `E_OK` (`:366-368`). Zero-Erweiterung
  entsteht erst beim Aufrufer (3.1).
- Wertform (`arg2 == 0`): `Some(v) => v as Long` (`:371@v`) — `u32 as i32`, daher **vorzeichenerweitert
  in `x0`**; ein Wert mit gesetztem Bit 31 ist von einem Fehlercode nicht zu unterscheiden.
  `programs/newtos-gem/src/gemdos.rs:444-447@cookie` sagt das ausdruecklich („The value form cannot express a
  cookie whose value has bit 31 set“).
- Speicherung: `tos-api/src/cookie.rs:62-63@tag` (`tag: u32`, `value: u32`); Vorbelegung
  `seed_defaults` `tos-api/src/cookie.rs:221-227@seed_defaults`: `NTOS`=Version, `_MCH`=`0x0005_0000`,
  `_IDT`=`0x0000_112F`, `_AKP`=0, `EdDI`=1. **Kein** gesetzter Wert hat Bit 31.
- `_MCH = 0x00050000` ist `MCH_ARANYM` (`references/emutos/include/cookie.h:57`); die Vorlage
  `docs/analysis/machine-identity-adr-proposal-2026-10-28.md:81-84` nennt das ebenfalls
  („vorlaeufige Abweichung“ `tos-api/src/cookie.rs:221-226@seed_defaults`).

### 3.3 Verwendung der Cookie-Werte in portierten Programmen

Alle Treffer von `Getcookie|getcookie|get_cookie|find_cookie|S_GETCOOKIE|dos_getcookie` ausserhalb
HighWire (nicht gebaut):

| Programm | Datei:Zeile | Variablentyp | Auswertung | Cookie | Gebaut/ausgeliefert |
|---|---|---|---|---|---|
| mintlib `si_platform` (in `libc.a`: `unix__sysinfo.o`) | `sdk/libc/mintlib/unix/sysinfo.c:501-517@_mch` | `long _mch`, `short hi` | `hi = (_mch & 0xffff0000) >> 16`, `switch (hi)` mit `case -1`/`0..5` | `_MCH`, `Hades`, `_CF_` | in `libc.a` ja; Aufrufer (`sysinfo`/`uname`-Pfad) nicht verfolgt |
| QED | `sdk/apps/qed/src/global.c:46@_idt`, `sdk/apps/qed/src/global.c:564@getcookie`, `sdk/apps/qed/src/global.c:319@_idt` | `static long _idt` | `(unsigned short)_idt & 0xF00` | `_IDT` | ja (`QED.NAP`) |
| QED | `sdk/apps/qed/src/comm.c:60-63@getcookie` | `long l` | `dhst_id = (short)l` | `DHST` | ja |
| cflib (in QED) | `sdk/cflib/cflib-21/appinit.c:51-57@getcookie`, `sdk/cflib/cflib-21/magxver.c:37@getcookie`, `sdk/cflib/cflib-21/filesel.c:77@getcookie`, `sdk/cflib/cflib-21/userdef.c:692@getcookie`, `sdk/cflib/cflib-21/fontsel.c:195@getcookie` | `long l`/`long v` bzw. `NULL` | Praesenz, `l` weiter nicht verglichen | `MiNT`, `nAES`, `NVDI`, `MgPC`, `MagX`, `FSEL`, `Gnva`, `xFSL` | ja |
| TeraDesk | `sdk/apps/teradesk/video.c:60@vdo`, `sdk/apps/teradesk/video.c:158-160@vdohi`, `sdk/apps/teradesk/video.c:172@vdohi`, `sdk/apps/teradesk/video.c:202@vdohi` | `static long vdo`, `short vdohi` | `vdohi = (_WORD)(vdo >> 16)` (Verengung auf 16 Bit), Vergleiche mit `FAL_VIDEO` usw. | `_VDO` | ja; unter `_TOS_ONLY` nur `Setexc`-Zweig (Q2) |
| TeraDesk | `sdk/apps/teradesk/video.c:213@find_cookie`, `sdk/apps/teradesk/xfilesys.c:1865@find_cookie`, `sdk/overlays/teradesk/main.c:1816-1819@find_cookie`, `sdk/overlays/teradesk/main.c:1861@find_cookie` | `long over` / `NULL` | Praesenz | `OVER`, `Lace`, `_FLK`, `MiNT`, `Gnva`, `nAES`, `MagX`, `CT60` | ja (die `_MINT_`-Zweige sind uebersetzt nicht dabei, `docs/analysis/lp64-tos-struct-survey-2026-10-28.md:90`) |
| HypView | `sdk/apps/hypview/dl_ledit.c:143@Getcookie` | `MAGX_COOKIE *cookie`, Aufruf `Getcookie('MagX',(long *)&cookie)` | als Zeiger dereferenziert | `MagX` | `sdk/apps/build-hypview.sh` vorhanden; nicht auf der Standard-Abbildung |
| HypView | `sdk/apps/hypview/dl_dhst.c:45@Getcookie` | `long value` | nur Praesenz | `DHST` | s. o. |
| DOOM | `sdk/apps/doom/doomgeneric_gem.c:1088@cookie`, `sdk/apps/doom/doomgeneric_gem.c:1096@Getcookie` | `long cookie = 0` | nur Praesenz; „the value is never called“ (`:90`) | `EdDI` | `sdk/apps/build-doom.sh`; nicht auf Standard-Abbildung |
| zView `jpgldg` | `sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:798@mach`, `sdk/zview/overlay/zview/plugins/jpg/jpgldg.c:807-822@Getcookie` | `long mach, cpu` | `mach == 0x50000L`; `cpu &= 0xFFFF`, `cpu != 30` | `_JPD`, `_MCH`, `_CPU` | ja (Codec `JPG.NLD`); der Block laeuft nur bei vorhandenem `_JPD` (Overlay-Kopf `:21-24`), den NewTOS nicht veroeffentlicht |
| LDG | `sdk/ldg/overlay/src/devel/ldgutil.c:86-91@Getcookie` | `long v` | Zeiger-/Praesenz | `LDGM` u. a. | ja |
| WinDom | `sdk/windom/overlay/src/set_cookie.c:75@Getcookie`; `sdk/windom/overlay/src/objc_str3d.c:67@get_cookie` | `long old` / `NULL` | Praesenz, Wert zum Wiederherstellen | beliebig / `MagX` | nicht auf Standard-Abbildung |
| Desktop (Rust) | `programs/desktop/src/redraw.rs:145@dos_getcookie` | `Option<u32>` | `rc as u32` | `_IDT` | ja (NEWDESK) |
| `apitest` | `sdk/apps/apitest/apitest.c:1746@SSYSTEM`, `sdk/apps/apitest/apitest.c:1756@ss_rc` | `ss_val` | `ck_eq`/`ck_hex` | `NTOS` | Abnahmekarte |

**Ergebnis:** Kein gefundener Aufrufer vergleicht einen Cookie-Wert numerisch so, dass sich
Vorzeichen- und Nullerweiterung unterscheiden.
- `>> 16` mit anschliessender Verengung auf 16 Bit (`sdk/apps/teradesk/video.c:160@vdohi`) und
  `(_mch & 0xffff0000) >> 16` in `short` (`sdk/libc/mintlib/unix/sysinfo.c:516@hi`) ergeben fuer `0xFFFFFFFF` in beiden
  Erweiterungen `-1` **[DERIVED]**: im ersten Fall schneidet der `(_WORD)`-Cast, im zweiten wirkt die
  Maske `0xffff0000` (positive 64-Bit-Konstante) auf beide Erweiterungen gleich.
- `mach == 0x50000L` und `cpu & 0xFFFF` (zView) haben kein Bit 31.
- `>=`/`<`-Vergleiche auf Cookie-Werten: **kein Treffer** in den gebauten Programmen.
- Kein Cookie im Seed-Jar hat Bit 31 (3.2). Die Frage stellt sich erst, wenn `_MCH = 0xFFFFFFFF`
  (`MCH_NOHARD`, Q4) oder ein anderer Wert mit Bit 31 gesetzt wird.

**Nebenbefund (DERIVED, ausserhalb des Auftrags):** `sdk/cflib/cflib-21/cookie.c:33-49@as_char` baut den Tag aus
`cast.as_char[0..3]` in einer `union` mit `long as_long` und liest `as_long`. Auf dem Big-Endian-m68k
ist `as_long` der Tag `0x5F494454` (`"_IDT"`), auf Little-Endian-AArch64 liegt `cookie[0]` im niedrigsten
Byte; `as_long` ist dann `0x5444495F`, die Kernel-Konstanten sind `u32::from_be_bytes(*b"_IDT")`
(`tos-api/src/cookie.rs:27@MCH`). Zusaetzlich bleiben die oberen vier Byte der `union` unbelegt. Der
Kernel liest `arg_i32(1)` (nur die unteren 32 Bit, `tos-api/src/gemdos/ssystem.rs:346@tag`). Folge waere: `getcookie("_IDT",…)`
(QED `sdk/apps/qed/src/global.c:564@getcookie`, `getcookie("DHST",…)`) findet nichts und QED nutzt den Rueckfall `0x0000112E`
(`sdk/apps/qed/src/global.c:565@_idt`). Nicht gemessen.
**Vorgeschlagener Sondenschritt:** in `apitest` die cflib-Form woertlich (`union` + `Getcookie`) mit
Tag `NTOS` aufrufen und `C_FOUND` erwarten; Gegenprobe mit vertauschter Byte-Reihenfolge.

### 3.4 Messlage Cookie

- **Keine** Orakelmessung des Cookie-Werts am gepinnten ROM:
  `docs/analysis/machine-identity-adr-proposal-2026-10-28.md:49-52` (kein Cookie-Fall in
  `tosdump/cases/*.tsv`; nur `Setexc(0x168,-1)`, `tosdump/cases/bios-core-r1.tsv:60@Setexc`).
  `_MCH` am Orakel: `0x00020000` (TT), `0x00030000` (Falcon-TC) stammt aus der EdDI-Sonde mit
  EmuTOS 1.4, **nicht** dem Pin (`…machine-identity…:53-59`).
- **Vorgeschlagener Fall (Wertbreite):** Sonde (Muster `apitest` SX09-SX10, `sdk/apps/apitest/apitest.c:1746-1760@ss_rc`):
  `Ssystem(S_SETCOOKIE, 'Z0Z0', 0x80000001)`, danach (a) `Getcookie` mit vorbelegtem `long v = -1L`
  (Pruefung, dass die oberen Bytes gesetzt bleiben, das heisst nicht null werden), (b) `v = 0` vorbelegt,
  (c) Wertform `Ssystem(S_GETCOOKIE, tag, 0)` und Ausgabe von `x0` als 64-Bit. Drei Zeilen Log
  genuegen; ein Tabellenfall (`tosdump`) ist nicht noetig, weil das Orakel keinen `Ssystem` hat
  (FreeMiNT-Erweiterung).

---

## 4. `_MCH = 0xFFFFFFFF` in EmuTOS — READ

- `references/emutos/include/cookie.h:50`:
  `#define MCH_NOHARD  0xffffffffL /* No ST-compatible hardware */`
  (daneben `:63` `#define VDO_NOHARD  0xffffffffL /* No ST-compatible video hardware */`).
- Zuweisung: `references/emutos/bios/machine.c:401`: `    cookie_mch = MCH_NOHARD;` — im `#else`-Zweig
  von `#if CONF_ATARI_HARDWARE` (`machine.c:384` Beginn, `:402` `#endif /* CONF_ATARI_HARDWARE */`)
  innerhalb von `setvalue_mch` (`:382`). Im `#if`-Zweig (`:385-399`) werden `MCH_ARANYM`, `MCH_FALCON`,
  `MCH_TT`, `MCH_MSTE`/`MCH_STE`, `MCH_ST` gesetzt. Der Wert geht danach als `_MCH` ins Jar:
  `references/emutos/bios/machine.c:692-694` (`setvalue_mch(); cookie_add(COOKIE_MCH, cookie_mch);`).
- Wann `CONF_ATARI_HARDWARE` gleich 0 ist: `references/emutos/include/config.h:582-585` (`MACHINE_AMIGA`),
  `:604-606` (`MACHINE_LISA`), `:629-631` (`MACHINE_M548X`, ColdFire); Vorgabe 1 bei `references/emutos/include/config.h:665-667`
  („By default, EmuTOS is built for Atari ST/TT/Falcon compatible hardware“). Im gepinnten Bau
  (`make 512`, `docs/analysis/ORACLE.md` §1) ist `CONF_ATARI_HARDWARE` damit 1 — das Orakel kann
  `0xFFFFFFFF` **nicht** liefern **[DERIVED aus config.h]**. Dieselbe Bedeutung hat der Wert fuer
  `_VDO` (`machine.c:375`, `#else`-Zweig von `:365`).
- Weitere Verwendung von `MCH_NOHARD`/`VDO_NOHARD` im EmuTOS-Quelltext: keine (Suche ueber `*.c`,
  `*.h`, `*.S` in `references/emutos`: nur `references/emutos/include/cookie.h:50`, `:63`, `machine.c:375`, `:401`).
- Die Bedeutung „keine ST-kompatible Hardware“ steht allein im Kommentar `references/emutos/include/cookie.h:50`; der Code
  gibt dem Wert diese Bedeutung nur ueber die Zuweisung im `#else`-Zweig. Ob ein Programm den Wert
  so liest, ist fuer EmuTOS nicht belegbar; auf der Leserseite kennt `sdk/libc/mintlib/unix/sysinfo.c:520@hi`
  `case -1` (`hi == -1`, nur mit `coldfire` zugeordnet, sonst `platform` unveraendert).

---

## 5. Bconmap und MAPTAB

### 5.1 EmuTOS-Layout — READ

`references/emutos/bios/serport.h:102-117` (unter `#if BCONMAP_AVAILABLE`, `serport.h:18`:
`BCONMAP_AVAILABLE (CONF_WITH_SCC || CONF_WITH_TT_MFP)`):

```
typedef struct {        /* one per mappable device */
    LONG (*Bconstat)(void);
    LONG (*Bconin)(void);
    LONG (*Bcostat)(void);
    LONG (*Bconout)(WORD,WORD);
    ULONG (*Rsconf)(WORD,WORD,WORD,WORD,WORD,WORD);
    EXT_IOREC *Iorec;   /* points to IOREC and extended IOREC */
} MAPTAB;
typedef struct {
    MAPTAB *maptab;
    WORD maptabsize;
    WORD mapped_device; /* device currently mapped to device 1 */
} BCONMAP;
#define BCONMAP_START_HANDLE    6
```

Feldreihenfolge `Bconstat, Bconin, Bcostat, Bconout, Rsconf, Iorec`; auf dem m68k je 4 Byte, `MAPTAB`
= 24 Byte **[DERIVED: 6 Zeiger a 4 Byte]**; `BCONMAP` = Zeiger (4) + `WORD` + `WORD` = 8 Byte.
`Bconmap(-2)` liefert `(LONG) &bconmap_root` (`serport.c:1092-1093`), also den **BCONMAP**-Zeiger, nicht
das `MAPTAB` selbst; `MAPTAB` erreicht man ueber `->maptab`.

### 5.2 Befuellung — READ

- Statische Tabellen: `serport.c:92-117` (`maptable[4]`, `maptable_dummy`, `maptable_mfp`
  `{ bconstat1, bconin1, bcostat1, bconout1, rsconf1, &iorec1 }`, `maptable_port_a/_b`
  `{bconstatA…, rsconfA, &iorecA}`, `maptable_mfp_tt`).
- `init_bconmap` `serport.c:954-1004`: dummy-Fuellung (`:962-963`), `maptab`/`maptabsize`/
  `mapped_device` (`:964-966`), je nach Maschine `maptable[0..3]` (`:972-993`); danach Kopie der
  Eintraege des gemappten Geraets in die Tiefspeicher-Vektoren `bconstat_vec[1]`, `bconin_vec[1]`,
  `bcostat_vec[1]`, `bconout_vec[1]`, `rsconfptr`, `rs232iorecptr` (`:997-1003`).
- `bconmap(dev)` `references/emutos/bios/serport.c:1082-1126`: `-1` → gemapptes Geraet; `-2` → `&bconmap_root`; sonst
  Umschalten, dabei werden die alten Vektoren in den alten `MAPTAB`-Eintrag zurueckgesichert
  (`:1103-1110`) und die neuen aus dem Eintrag geholt (`:1114-1121`); ohne `BCONMAP_AVAILABLE` `return 0x2c`
  (`references/emutos/bios/serport.c:1124`).

### 5.3 Wo EmuTOS selbst durch die Zeiger ruft — READ

- `references/emutos/bios/bios.c:925-928` (`Bconstat`), `:969-972` (`Bconin`), `:1002-1005` (`Bconout`),
  `:1154-1157` (`Bcostat`): fuer `handle >= 6` (`map_index = handle - BCONMAP_START_HANDLE`,
  `map_index < maptabsize`) `protect_v(bconmap_root.maptab[map_index].<Feld>)` bzw. `protect_ww(...)`.
  Fuer Geraete `0..7` ohne Mapping: `bconstat_vec[handle]` usw. (`:932`, `:976`, `:1009`).
- `Rsconf` und `Iorec` ueber die kopierten Vektoren: `references/emutos/bios/xbios.c:340`
  (`iorec`: `return (LONG) rs232iorecptr;`) und `:378` (`rsconf`: `(*rsconfptr)(baud, …)`).
  `MAPTAB.Rsconf`/`MAPTAB.Iorec` werden von EmuTOS **nie direkt** aufgerufen; sie wandern nur in
  `rsconfptr`/`rs232iorecptr` (`serport.c:1002-1003`, `:1114-1121`).
- Ausserhalb `bios/bios.c`, `bios/xbios.c` und `bios/serport.c` kommt `MAPTAB`/`maptab` im EmuTOS-Quelltext
  nicht vor (Suche `grep -rn MAPTAB include bios`).
- Messung am Orakel: `Bconmap(-1)` = 6 (TT), `Bconstat(6..9)`-Faelle, `Bcostat(8/9)`:
  `tosdump/cases/bios-core-r1.tsv:20-22@Bconstat`, `tosdump/cases/bios-core-r1.tsv:65-66@Bcostat`; `tosdump/cases/bios-r1.tsv:184@Bconmap` (`Bconmap(-2)`,
  `PTR_RAM`). Was **hinter** dem Zeiger liegt (Layout am Orakel) ist nicht gemessen — die Falltabelle
  nennt die Mitschnittspalte `r:` dafuer (`tosdump/cases/xbios-chardev-r1.tsv:13@Bconmap`, Kommentar).

### 5.4 Liest ein portiertes Programm `maptab` und ruft durch die Zeiger? — Suche

Muster: `Bconmap|MAPTAB|maptab|BCONMAP` (gross/klein) ueber `sdk/apps`, `sdk/overlays`, `sdk/examples`,
`programs`, `sdk/cflib`, `sdk/windom`, `sdk/zview`, `sdk/ldg`, `sdk/gemlib`, `sdk/libc/overlay`,
`sdk/libc/newtos-arch`, `sdk/include`, `tosdump/core`, `Applications` (ohne `.claude`, ohne
`mintlib-src`); zusaetzlich `sdk/libc/mintlib` (ohne Kopf-Dateien) und `Rsconf|Iorec` ueber `sdk`.

- **Kein Programm** liest `maptab` oder ruft durch die Funktionszeiger. Es gibt keinen Treffer fuer
  `Bconmap`, `Rsconf` oder `Iorec` in `sdk/apps`, `sdk/overlays`, `sdk/examples`, `programs`, `sdk/cflib`,
  `sdk/windom`, `sdk/zview`, `sdk/ldg`, `Applications` (nach Entfernen von Fehltreffern auf `RemapTable`
  in Vanilla-Conquer, die das Muster `maptab` ohne Gross-/Kleinunterscheidung traf).
- Treffer nur in Definitionen: `sdk/libc/overlay/include/mint/ostruct.h:527-530@Bconmap` (`BMAP_MAPTAB = -2`),
  `sdk/libc/overlay/include/mint/ostruct.h:679-698@_BCONMAP` (`_MAPTAB`, `_BCONMAP`), `sdk/libc/overlay/include/mint/osbind.h:760@Bconmap` (Binder);
  `tosdump/core/cases.c:3006@CR_BCONMAP`, `tosdump/core/cases.c:3067@CR_BCONMAP`, `tosdump/core/cases.c:3324@CR_BCONMAP` (Dumper-Aufrufpolitik, kein Programm).
- Es gibt kein portiertes Terminalprogramm (keines unter `sdk/apps/`, keines auf der Standard-Abbildung).
  `Applications/thing-neo-codeSVN-r62`, `Applications/z-tools`, `Applications/qed` etc. enthalten kein
  `Bconmap`.
- Karten-Manifest (siehe Q2): die Standard-Abbildung traegt keine Anwendung, die Bconmap benutzt; ein
  Terminalprogramm ist darauf nicht vorhanden.

**Gemessen:** NewTOS `Bconmap` ist ein fester Wert `0x2C` (`system.rs:106@bconmap`, Messung 1.3 A5);
ein Programm, das `Bconmap(-2)` ruft und `->maptab` dereferenziert, bekommt `0x2C` als Zeiger.
**Vorgeschlagener Fall (falls die Frage je akut wird):** Zeile in `tosdump/cases/xbios-chardev-r1.tsv`
fuer `Bconmap(-2)` mit `r:`-Mitschnitt von 8 Byte (`maptab`, `maptabsize`, `mapped_device`) und dem
ersten `MAPTAB` (24 Byte) am Orakel; auf der NewTOS-Seite faellt der Mitschnitt aus, weil kein Zeiger
zurueckkommt.

---

## Ausserhalb des Auftrags (je eine Zeile)

- `docs/sdk/05-abi-pitfalls.md:26` zitiert Zeilen 359-362 und 434 von `exceptions.rs` fuer die Ergebnisregel; die Zeilen
  sind veraltet (heute `kernel/src/arch/aarch64/exceptions.rs:758@result`).
- `tos-api/src/xbios/system.rs` (Zeilen 262 und 294) sagt weiter „zero-extended“ fuer Rsconf, die Spec `0f-rsconf.md:80-90` sagt
  „sign-extended“ (XBIOS07-Korrektur, Code „pending XBIOS06“).
- Die ETV-Platzhalter `tos-api/src/bios/vectors.rs:45@ETV_DEFAULT_JUST_RTS`/`tos-api/src/bios/vectors.rs:54@ETV_DEFAULT_CRITIC` verstossen gegen MEM11s Invariante „jede gelieferte Adresse < 2 GiB“
  (`docs/audits/2026-10-28-bios-core.md`, BIOS-CORE-005); hier nur als Abweichung A3 aufgefuehrt.
