# LDG nach ARCH03 — Entwurf Meilenstein M1 (`ex_lib` als natives `.NLD` laden, `sinus()` rufen)

Stand 2026-10-06 · Researcher-Auftrag · Nur Dokumentation, kein Code, nichts committet, keine Karte/Pi/TT berührt.
Vorgänger: `docs/analysis/zview-ldg-2026-10-04.md` (im Folgenden **ALT**; unverändert gelassen, nur zitiert). Grundlage: `docs/analysis/arch03-implementation-plan.md` (**PLAN**), ADR-025.

Anwenderentscheide 2026-10-06 (vom Lead übergeben): Ziel ist die **LDG-Fassung von zView** in `Applications/z-tools` (nicht th-otto/SLB). Native LDG-Module bekommen die **eigene Endung `.NLD`** (ADR-018-Trennung vom m68k), dafür braucht zView ein **toolchain-seitiges Overlay** für `plugins.c` (Suche nach `*.ldg`); der App-Quelltext bleibt unberührt.

Kennzeichnung: **[gelesen]** = Quelle `file:line` in dieser Sitzung gelesen · **[Schluss]** = aus Gelesenem gefolgert · **[Schätzung]** = Aufwand, kein Beleg · **[offen]** = nicht geklärt / braucht Entscheidung oder Messung.

Pfadkürzel: `LDG` = `Applications/ldg-src/ldg` (SVN r133, gitignoriert, per `curl` geholt: `Applications/ldg-src/SOURCE.txt`) · `DEV` = `LDG/src/devel` · `ZV` = `Applications/z-tools/zview` · `NT` = `tos-api/src/bdos/process` · `KX` = `kernel/src/arch/aarch64/exceptions.rs` · `ET` = `references/emutos` · `FM` = `references/freemint/sys`.

---

## 0. Kurzfassung

1. **Der Original-LDG-Mechanismus läuft auf NewTOS ohne Userspace-Lader und ohne Kernel-Umbau — bis auf eine Lücke, die M1 blockiert.** `Pexec(5)` → `Pexec(3)` → `Pexec(4)` → `Pexec`-Rückkehr ist im Kernel vorhanden und relokiert, synchronisiert den I-Cache und gibt dem Aufrufer das Eigentum am Modulblock (Abschnitt 3). ALT §4.2 (Userspace-ELF-Lader) ist **obsolet** (Abschnitt 1).
2. **Blocker B1 (Toolchain, nicht Kernel):** `ldg_init()` holt die Client-Adresse über `getenv("OFFSETLDG")` (`DEV/ldglib.c:134-137`). Die NewTOS-libc **läuft den Environment-Block nicht in `environ` ab**: `environ` ist für jeden Start das leere Array `{NULL}` (`sdk/libc/newtos-arch/cmdargs.c:37-57,95,246`; der Walk heißt dort „TASK/PORT09 and not built yet“). `getenv` liefert im Modul also `NULL`, `ldg_init` gibt `-1` zurück, das Modul ruft `appl_init()/form_alert()` statt sich zu melden. Ohne Lösung von B1 gibt es kein M1.
3. **Die vier LDG-Quellen sind nicht LP64-/AArch64-tauglich, aber mit kleinen, abgegrenzten Overlays (Muster `sdk/overlays/*`):** `BASEPAGE.p_env` ist im SDK ein festes 32-Bit-Feld (`sdk/libc/overlay/include/mint/basepage.h:30-47`) → `ldgload.c` bekommt harte Int-zu-Zeiger-Fehler; `ldg_cookie` liest `*(long*)0x5a0` roh (`DEV/ldgutil.c:53-69`), auf LP64 mit 32-Bit-Jar-Einträgen falsch; `ldg_cpush` führt ein m68k-Opcode-Array aus (`ldgutil.c:205-225`); der GCC-Build hängt `stdcallgcc.o` (m68k-Assembler) an (`DEV/gcc.mak`, `EXTRAOBJS`).
4. **Kein I-Cache-Handlungsbedarf im Client.** Der Kernel macht `dc cvau`/`ic ivau` nach dem Relokieren (`NT/loader.rs:599`, `NT/mmu.rs:1564-1608`); die EL0-Frage (`SCTLR_EL1.UCI`) aus ALT ist damit gegenstandslos, solange kein Modul selbst Code erzeugt (V0a E1/E3/E13 in `docs/analysis/arch03-stage0a-armarm-verification.md:97-99,181`). Das ist die Pi-Hauptprüfung (Abschnitt 8).
5. **`libshare`: Quelltext nirgends vorhanden** (Abschnitt 5; **ueberholt**, siehe [libshare-2026-10-06.md](libshare-2026-10-06.md)). Für M1 nicht gebraucht (`ex_lib` ruft es nicht). `Applications/ldg-src/ldg/src/share/` ist **nicht** libshare (Falle, Abschnitt 5).
6. **Korrektur an ALT §3.0:** `LDG/src/options.h` und `LDG/src/version.h` **liegen im Baum** (`LDG_NUM 0x0235`, `VERSION "2.35"`: `LDG/src/version.h`). ALT nannte sie fehlend.
7. **Aufwand M1** [Schätzung]: 3–4 Arbeitstage (B1 0,5–1; libldg bauen + Overlays 1; Modul + Probe 0,5–1; Image/ADR 0,5; QEMU-Fehlersuche 1).

---

## 1. Was von ALT gilt, was obsolet ist

| ALT | Aussage | Stand heute |
|---|---|---|
| §3 (Mechanismus, Original) | Ablauf `ldg_open` → `ldg_load` → `Pexec 5/3/4` | **gilt**, unten in Abschnitt 2 neu mit `file:line` belegt (inkl. Korrekturen) |
| §3.4/3.5 (TSR optional, `NOT_SHARED`) | | gilt (Abschnitt 6) |
| §4.1 Zeile „Pexec(3) lädt nicht in den Aufrufer“ | per-Prozess-Adressraum, PD opak | **obsolet**: ADR-025 (now ADR-028), VA = PA, PD ist die echte Basepage-Adresse (`NT/pexec.rs:80-85`, `LoadedChild.handle`) |
| §4.1 „Pexec(5)/(6)/(7) nicht implementiert“ | `EINVFN` | **obsolet**: implementiert (`NT/pexec.rs:171-195`) |
| §4.1 „ELF64 `ET_EXEC` fest gelinkt, keine Relokation“ | | **obsolet**: nur `ET_DYN`, `ET_EXEC` wird abgelehnt (`NT/elf.rs:455-456`), Relokation im Kernel (`NT/loader.rs:579-587`, `NT/reloc.rs`) |
| §4.1 „Fenster-Adressen werden nie wiederverwendet“ (L5) | 28-MiB-Lebenszeitbudget | **obsolet**: es gibt kein Fenster mehr (PLAN Z. 62 „keine Fenstererschöpfung mehr“) |
| §4.1 „Cache-Sync nur im Kernel, EL0 darf nicht“ | `UCI` aus | Befund stimmt, **Folgerung entfällt**: der Kernel synchronisiert beim Laden (Abschnitt 3, Zeile 6) |
| §4.1 „Cookie-Jar 32-Bit-Einträge vs. `long`“ | | **gilt weiter** (Abschnitt 4, G3) |
| §4.2 Nr. 1–5 (Userspace-ELF-Lader, Cache-Sync, eigener Modul-`crt`, `__ldg_lib`-Symbol, eigene Heap-Arena) | | **obsolet.** Das Modul ist ein ganz normales NewTOS-Programm (PIE, libc-`crt0`, `_crtinit` mit `Mshrink`), `ldg_init` bleibt unverändert, der Kernel lädt und relokiert |
| §4.2 Nr. 6 (Endung) | offen | **entschieden: `.NLD`** (Anwender 2026-10-06) |
| §4.2 Nr. 7 (ohne Cookie) | | gilt (Abschnitt 6) |
| §4.2 Nr. 8 (`ldg_cookie` LP64-falsch) | | gilt (G3) |
| §4.3 (MOD01 unabhängig) | | gilt |
| §4.4 L1–L6, §5 Aufwand | | ersetzt durch Abschnitt 4 und 7 dieses Dokuments |

---

## 2. Wie das Original LDG lädt (alles [gelesen], r133)

### 2.1 Client: `ldg_open` (`DEV/ldgapp.c`)

* `ldg_open(lib, gl)`: `ldg_cookie(LDG_COOKIE, &cook)`; mit Cookie, `cook->version >= 0x0200` und gesetztem `ldg_open` → Aufruf des TSR (`ldgapp.c:34-36`). **Sonst `return ldg_load(gl[2], lib)`** (`:37-38`) — der TSR ist **optional**. `gl[2]` ist `aes_global[2]` (AES-Id), wird nur in `ldg->id` abgelegt (`ldgload.c:174`).
* `ldg_find(name, ldg)`: mit TSR → Cookie-Funktion (`:46-49`); sonst lineare `strcmp`-Suche in `ldg->list[]`, rotierend ab `lastpos` (`:54-60`), liefert `list[j].func` (`:58`). Kein Trap, kein Marshalling: der Client ruft den Zeiger direkt auf seinem eigenen Stack (`ldg.h:54-58`).
* `ldg_close`: TSR (`version >= 0x210`) oder `ldg_unload` (`:64-72`).
* `ldg_error`: **ohne Cookie immer `LDG_NO_TSR` (-11)** (`:92-104`); `set_error` schreibt nur in ein vorhandenes Cookie (`DEV/ldgutil.c:78-83`). Folge für die Probe: Fehlerdiagnose nicht über `ldg_error()`, sondern über den `NULL`-Rückgabewert.
* **Keine Bibliothekssuche ohne TSR:** `ldg_load` bekommt den Pfad unverändert. `LDGPATH`/`PATH`/Cookie-Pfad werden erst im TSR (`LDG/src/kernel/ldgcook.c:83-144`, `cdecl_ldg_libpath`) aufgelöst. zView umgeht das, weil es per `chdir` ins Codec-Verzeichnis wechselt und den nackten Dateinamen übergibt (`ZV/plugins.c:61-86,98`).

### 2.2 `ldg_load` (`DEV/ldgload.c:59-195`)

| Z. | Schritt | Wirkung |
|---|---|---|
| 73 | `module = Pexec(5, NULL, "", NULL)` | Dummy-Basepage, nur um `p_env` des Aufrufers zu bekommen |
| 76-96 | `ldg_global_env = module->p_env`; Länge zählen; `mbloc = Malloc(size_env+24)` | Platz für `OFFSETLDG=…` + Kopie der Umgebung |
| 99-106 | `offsetxt = mbloc` oder, bei `Malloc`-Fehler, `smallbloc[24]` | Fehlerfall ist vorgesehen (Umgebung geht dann verloren, `OFFSETLDG` bleibt) |
| 107-108 | `sprintf(offsetxt, "OFFSETLDG=%ld%c%c", (long)&ldg, 0, 0)` | **Adresse einer lokalen Variable `ldg` des Clients** als Text |
| 114-130 | restliche Umgebung dahinterkopieren | |
| 131-136 | `Mfree(module->p_env); Mfree(module)` | Dummy freigeben |
| 137 | `module = Pexec(3, path, NULL, offsetxt)` | **Load, don't go**; Basepage-Zeiger zurück |
| 138 | `Mfree(mbloc)` | |
| 139-142 | `(long)module < 0` → `LDG_ERR_EXEC` | |
| 145-149 | `ldg_cpush()` (außer ColdFire) | Kommentar `:146-147`: „Pexec(3) should flush the caches internally“ |
| 151 | `Pexec(4, NULL, module, NULL)` | **Just go**: Modul läuft bis zum Programmende (synchron) |
| 156 | `ldg_cpush()` | |
| 158 | `Pwait()` | MiNT 0x109; Rückgabe verworfen |
| 160-172 | `ldg == NULL` oder `ldg->magic != LDG_COOKIE` → `LDG_BAD_FORMAT`, `Mfree(p_env)`, `Mfree(module)` | |
| 174-175 | `ldg->id = apid`, `ldg->baspag = module` | |
| 178-193 | ab `vers_ldg >= 0x100`: absoluten Pfad in `ldg->path` | `Dgetdrv`/`Dgetpath` |

`ldg_unload` (`:201-213`): `(*ldg->close)()` falls gesetzt, dann `Mfree(baspag->p_env)`, `Mfree(baspag)`; danach ist `ldg` selbst ungültig (liegt im freigegebenen Modul, Kommentar `:206-210`).

EmuTOS-Gegenstück zu Schritt 137: `Pexec(PE_LOAD)` legt TPA und Umgebung mit **Besitzer = aufrufender Prozess** an (`ET/bdos/proc.c:302-307`: `owner = (flag == PE_LOADGO) ? p : run`), und EmuTOS invalidiert den I-Cache selbst (`ET/bdos/proc.c:352-356`, `invalidate_instruction_cache` über den TEXT-Bereich). FreeMiNT ebenso (`FM/k_exec.c:686-687`, `cpushi(p_tbase, p_tlen)`). `ldg_cpush` ist damit auch auf m68k eine Doppelung (siehe Kommentar `ldgload.c:146`).

### 2.3 Modul-Seite: `ldg_init` (`DEV/ldglib.c:97-142`)

* Das Modul ist ein normales Programm; `main()` ruft `ldg_init(&LibLdg)` und gibt `0` zurück (`LDG/demo/libs/ex_lib.c:65-72`).
* `ldg_init` füllt die **statische** `LDG lib` des Moduls (`ldglib.c:40,102-111`: `magic = 'LDGM'`, `num`, `list`, `infos`, `flags`, `close`, `vers_ldg = LDG_NUM`), nullt `_ldg_adr_regctx[512]` (16 KiB, `:113-122`), setzt unter GCC `lib.flags |= LDG_STDCALL` (`:127-132`), dann
  `env = getenv("OFFSETLDG"); if (env) { offset_pere = (long*)atol(env); *offset_pere = offset; return 0; } else return -1;` (`:134-141`).
* Das Modul **schreibt über den Text-Zeiger direkt in den Stack des Clients**. Das ist der Grund, warum LDG einen gemeinsamen Adressraum voraussetzt (PLAN Z. 37, 69).

### 2.4 Demo (`LDG/demo`)

* `ex_lib.c`: `PROC LibFunc[]` mit `information`, `sinus`, `cosinus` (`:32-38`); `LDGLIB LibLdg[]` ohne Flags (`:40-47`, d. h. **nicht** `LDG_NOT_SHARED`, kein `close`); `double __CDECL sinus(Real *val) { return sin(val->value); }` (`:53-55`). Bindung `demo/libs/exlibbnd.c` (21 Zeilen: `ldg_find("information"|"cosinus"|"sinus", ldg)` in `init_ldg_math`), Header `demo/libs/ldg/ex_lib.h`.
* `demo/client/client.c` ist **windom-basiert** (`#include <windom.h>`, `:11`; `LoadLDG` `:124-137`: `ldg_open(lib, ldg_global)`, `init(ldg)`). windom ist M3 (ALT §5). **Der unveränderte Demo-Client ist daher nicht M1-fähig** (offene Frage 4).
* `ldg_global` ist `aes_global` (`ldg.h:37-39`, nur mit `__GEMLIB__`); im SDK ist das `short aes_global[]` (`sdk/gemlib/overlay/gem.h:27`).

### 2.5 Was der TSR (`ldg.prg`) zusätzlich tut (für Abschnitt 6)

`ldg_open_flag` (`LDG/src/kernel/ldgcook.c:157-299`): Bibliothek schon geladen? (`get_lib`, `:177`) → bei `LDG_NOT_SHARED` verwerfen (`:179`); sonst `ldg_load` (`:191`), unter MiNT/MagiC SHM-Block über `Fcreate("U:\shm\ldgNNN.shm")` + `Fcntl(SHMSETBLK)` (`:193-198`), Client in `usedby[]` (`:286`), Referenzzähler `ndecl[]` (`:287,295`). Der TSR installiert sich per `Ptermres` (`LDG/src/kernel/tsrmain.c:227`).

---

## 3. Schritt-für-Schritt-Abbildung auf NewTOS (Ist-Stand `desk-rebuild`, nach ARCH03)

Bewertung: **ok** = vorhanden und belegt · **Lücke** · **offen** = nur durch Lauf klärbar.

| # | Original-Schritt | NewTOS heute | Befund |
|---|---|---|---|
| 1 | `Pexec(5, NULL, "", NULL)` | `KX:1879` — `path` wird nur für Modus 0/3 gelesen (`loads`), Modus 5 liest keinen String; `tail` `""` → `tlen 0`; `env == 0` → `EnvSpec::Inherit` (`KX:1899-1918`); `create_basepage` (`NT/loader.rs:703`) → PD = Blockadresse, `p_env` = eigener Poolblock, beide Besitzer = Aufrufer (`NT/pexec.rs:205-228`, `load_owner` = `current()`) | **ok** [gelesen] |
| 2 | `module->p_env` im Client lesen | PD liegt im gemeinsamen Raum, Modus-5-Block hat `PRGFLAGS` 0 → `PROT_P` mit Aufrufer als Besitzer → lesbar (`NT/memprot.rs:506-509`) | **ok**; aber **Lücke G2a**: `p_env` ist `__UINT32_TYPE__` (`sdk/libc/overlay/include/mint/basepage.h:44`), `ldgload.c:76,133,162-163,204` weisen es als Zeiger zu → Compilerfehler (Overlay) |
| 3 | `Malloc(size_env+24)` mit offenem Dummy-PD | Das Dummy-TPA ist der **größte freie ST-Block** (`NT/loader.rs:703` Kopfkommentar, `docs/api/gemdos/process/4b-pexec.md` „TPA“) — wie EmuTOS. Ein `Malloc` dazwischen kann scheitern; das Original fängt das ab (`ldgload.c:99-106`) | **ok** [Schluss]; Folge: das Modul bekommt dann evtl. nur `OFFSETLDG=` ohne Client-Umgebung. M1 braucht nur `OFFSETLDG` |
| 4 | `sprintf("OFFSETLDG=%ld")`, `(long)&ldg` | LP64: `long` = 8 Byte, `%ld` korrekt; EL0-Adressen < 2^32 (`basepage.h`-Overlay-Kommentar) → ≤ 10 Ziffern → `OFFSETLDG=` (10) + 10 + 2 NUL = 22 ≤ 24 | **ok** [Schluss] |
| 5 | `Pexec(3, path, NULL, offsetxt)` | `KX:1847` → `pexec::dispatch` → `load(…, launch=false, owned_by_child=false)` (`NT/pexec.rs:184,253`): ELF öffnen (`FileImage::open`, `:273`; **keine Endungsprüfung** [gelesen]: nur `path::findit` → FAT-8.3-Name `tos-api/src/bdos/fs/path.rs:166-169`), Umgebung vor TPA allozieren (`NT/loader.rs:449`), TPA = größter freier Block (`arena::alloc_tpa`), `set_mode(tpa, prot_from_prgflags(prgflags))` (`:472`), **Eigentümer bleibt der Aufrufer** (`owned_by_child == false`, `:473-482`), Segmente streamen (`:564-573`), `reloc::relocate(tpa_bytes, img, base)` (`:587`) — nur `R_AARCH64_RELATIVE`, alles andere → `EPLFMT` (`NT/reloc.rs:1-80`), Basepage schreiben, `sync_code_cache` (`:599`), Slot + eigene Wurzel (`install_slot`) | **ok** — das ist genau, was LDG braucht (PLAN Z. 115). Umgebungsblock aus dem Client wird auf `ENV_BUF_LEN = 512` begrenzt, Überlauf kürzt still (kein Fehler) (`KX:1781-1822`) |
| 6 | `ldg_cpush()` | Entfällt. Kernel hat vorher synchronisiert (`NT/mmu.rs:1564-1608`: `dc cvau`, `dsb ish`, `ic ivau`, `dsb ish`, `isb`). `ldg_cpush` führt nur aus, wenn Cookie `_CPU` > 30 (`ldgutil.c:217-224`); **NewTOS seedet `_CPU` nicht** (Jar: `NTOS _MCH _IDT _AKP EdDI`, `kernel/src/selftest_cookie.rs:63`; `grep` über `tos-api/src`+`kernel/src` ohne Treffer auf `_CPU`) → das m68k-Array würde nie ausgeführt, aber der Code **darf nicht als Daten-Opcodes zur Ausführung gelangen können** | **ok**; Overlay G2c macht `ldg_cpush` zur Leeroperation |
| 7 | `Pexec(4, NULL, module, NULL)` | `go(handle, false)` (`NT/pexec.rs:352-422`): PD muss der Aufrufer besitzen (`region::is_owner`, `:363-371`); das Kind wird **weiterer Besitzer** von Basepage und Umgebung (`:386-398`); `space_pd` = Raum des Aufrufers → **Thread im Speicherraum des Aufrufers** (`:399-410`, FreeMiNT `thread = (mode == 4)`: `FM/k_exec.c:224-231,421`); Start am ELF-Entry auf Stack bei `p_hitpa` (`:430-433`). Trap-Frame des Aufrufers wird gesichert, Aufrufer aus dem kooperativen Lauf genommen, `TTBR0` auf das Kind (`KX:1967-2090`). `p_parent` = Aufrufer (`set_parent`, `:2022`) → nicht-null, daher kein „Accessory“-Zweig im Start (`sdk/libc/newtos-arch/heap_init.c:133`) | **ok** [gelesen]. Der Sonderfall steht ausdrücklich in der Spec: „LDG needs it: the module writes into its caller's stack“ (`docs/api/gemdos/process/4b-pexec.md`, Abschnitt `memprot=strict` und `PE_GO`). Sonde mit `Pexec 3 + 4` existiert: `sdk/examples/mprot3b.c:149-162` (Quittung `TASK/ARCH03-3b-STATUS-UNFINISHED.MD`: QEMU-Serien belegt) |
| 8 | Modul startet: `crt0` → `_crtinit` | `sdk/libc/crt0.S:55-57`: `__libc_crtinit` (`heap_init.c`) → `argv`, **`Mshrink(bp, m)` auf Image + `_stksize` (64 KiB)** (`heap_init.c:190`), Stack auf neuen Top; dann `main`, dann `exit()` (`crt0.S:90`). `_split_mem = 1`/Heap per `GEMDOS Malloc` (`heap_init.c:157`) | **ok**; entspricht mintlib-`_crtinit` (`crtinit.c:154-247`). `Mshrink` prüft in `tos-api` keinen Besitzer (`tos-api/src/bdos/mem/traps.rs:797-847`) |
| 9 | `getenv("OFFSETLDG")` | `environ == {NULL}` für jeden Start (`cmdargs.c:95,246`) → `NULL` | **Blocker B1** (Abschnitt 4) |
| 10 | `*offset_pere = offset` | Modul läuft im Raum des Aufrufers (Modus 4): Schreibzugriff auf den Stack des Clients erlaubt **in jedem Schutzmodus** (Spec s. Zeile 7) | **ok** [gelesen] |
| 11 | `return 0` → `exit` → `Pterm` | `terminate_inner` (`NT/pterm.rs`): `free_all_owned(bp)` = `region::detach_all` (`tos-api/src/bdos/mem/free_all_owned.rs:36-47`) → gibt nur frei, was **niemand sonst** hält; der Modul-TPA hat den Aufrufer als Besitzer → **bleibt stehen** (EmuTOS `free_all_owned` über `m_own`, `ET/bdos/proc.c:92-101`). Modul-eigene `Malloc`s (Besitzer = Modul) werden frei — wie im Original (ALT §3.6). `finish_terminate` stellt den Aufrufer her, Exitcode in `x0`, Slot frei (`KX:1510-1520`) | **ok** [gelesen]; Slot-Verbrauch je geladener Bibliothek nur während des Ladens |
| 12 | `Pwait()` | libc-`Pwait()` = Funktion `0x109` (`sdk/libc/overlay/include/mint/mintbind.h:90-91`), im Kernel nicht belegt → `EINVFN` (`tos-api/src/gemdos/mod.rs:154-156`); `0x4D` ist das NewTOS-Yield (`KX:373`, `:1084`), nicht dasselbe | **ok**: `ldgload.c:158` verwirft die Rückgabe |
| 13 | `ldg->magic`/`ldg == NULL` prüfen, `ldg->id`/`baspag` setzen | reiner Speicherzugriff im gemeinsamen Raum; LP64-Layout von `LDG` auf beiden Seiten gleich, weil beide Seiten mit demselben `ldg.h` unter LP64 übersetzt werden (`ldg.h:64-81`) | **ok** [Schluss] |
| 14 | `ldg_find` → Funktionszeiger aufrufen | Zeiger auf Code im Modulblock; Block gehört dem Aufrufer, `PROT_P`-Besitzer = `rwx` (`UserRwExec`, `memprot.rs:506-509`; Seiten `PXN=1`, `UXN=0`, `NT/mmu.rs:460-473`). Aufruf auf dem Client-Stack, im Client-Kontext | **ok** [gelesen/Schluss] |
| 15 | `ldg_close` → `ldg_unload` | `Mfree(p_env)`, `Mfree(baspag)`; Aufrufer ist Besitzer; `Mfree` einer nie gestarteten PD gibt zusätzlich den Slot frei (`NT/pexec.rs:526-535`) | **ok**; `p_env`-Cast wie Zeile 2 |
| 16 | `ldg_cookie` (jede `ldg_open/find/close/error`-Funktion beginnt damit, `ldgapp.c:34,46,67,95`) | `Supexec(cookieptr)` ist im SDK ein einfacher Aufruf (`sdk/libc/overlay/include/mint/osbind.h:813`); `cookieptr` liest `*(long*)0x5a0` (8 Byte) — im NewTOS-Page-0 sind das 4 Byte `_p_cookies` plus die nächsten 4 Byte (`COOKIE_JAR_VA = 0x5F0`, Einträge zu je zwei **32-Bit**-Wörtern: `NT/loader.rs:1291-1313`) → **falscher Jar-Zeiger, Zugriff auf ungültige Adresse** | **Lücke G3** — schon der erste `ldg_open` stürzt ab, **auch ohne TSR** |

> **Nachtrag 2026-10-06 zu Zeile 11 (TASK/LDG-M3-B2):** „Modul-eigene `Malloc`s (Besitzer = Modul) werden frei — wie im Original … **ok**“ stimmt für den Kernel, aber die Folge für den Modul-Heap fehlt: das `exit()` des Moduls holt per mintlib-`malloc` stdio-Puffer, die Freiliste des Moduls zeigt danach in einen beim `Pterm` freigegebenen Block, und das nächste `malloc` im Modul (vom Client gerufen) liest freien Speicher — Befund B2 (`docs/analysis/ldg-module-malloc-2026-10-06.md`, gemessen `measurements/measurement-2026-10-06-ldg-b2/`). Behoben im SDK durch libshare in jedem `.NLD` (`docs/sdk/10-ldg-modules.md` §6, `measurements/measurement-2026-10-06-libshare/`), nicht im Kernel.

Zusätzlich [gelesen]: `Pexec` prüft **keine Dateiendung** (`NT/pexec.rs:266-275`); `.NLD` wird daher vom Kernel wie jede andere ELF-Datei geladen. Der Desktop klassifiziert unbekannte Endungen als Dokument (`kernel/src/desk/ftype.rs:91-96`, `_ => FileType::Doc`) — ein Doppelklick auf `.NLD` startet nichts.

---

## 4. Lücken (was genau fehlt)

**G1 — Environment-Walk in der libc (Blocker B1).** `getenv`/`environ` kennen den Block in `p_env` nicht.
* Original-Gegenstück: `mintlib/crtinit.c:290-345` `parseargs()` (Walk von `p_env` in `environ`, `ARGV=`-Erweiterung); NewTOS hat nur die Argument-Hälfte nachgebaut (`cmdargs.c:37-57`).
* Zwei Wege [Schluss]: (a) **libc**: `__libc_parseargs` läuft `p_env` ab und füllt `environ` (wirkt für **alle** Programme, mintlib-konform; berührt aber den Start jedes Programms und deckt sich teilweise mit `TASK/PORT09`, das als BLOCKIERT durch MEM03 geführt wird — PLAN Z. 62 nennt MEM03 gelöst, ob PORT09 damit frei ist, ist **[offen]**). (b) **libldg-Overlay**: `ldg_init` liest `OFFSETLDG` selbst aus `_base->p_env`. (b) ist lokal und risikoarm, hält aber `getenv` für alle anderen Programme kaputt. Entscheidung beim Anwender (Frage 1).
* Offene Entwurfsfrage zu (a): Platz für das `char*`-Array (statisch begrenzt wie `arg_vec[ARGV_MAX]` in `cmdargs.c:88-89` vs. `Malloc`); `Malloc` im Modul gehört dem Modul und wird bei dessen `Pterm` frei, ein statisches Array nicht.

**G2 — `libldg` ist nicht gebaut; vier Quellen brauchen Overlays (`sdk/overlays/ldg/` o. ä., Quelle bleibt unberührt).**
* **G2a** `ldgload.c`: `p_env` (`u32`) → Zeiger-Cast `(char*)(uintptr_t)`, an den Stellen `:76,133,162-163,204`. Vorbild: `sdk/overlays/teradesk/environm.c` (`docs/sdk/04-porting-anatomy.md`, Tabelle „int → pointer cast“).
* **G2b** `ldgutil.c`/`ldglib.c`: `ldg_cookie` → `Getcookie` (`sdk/libc/newtos-arch/getcookie.c`, über `Ssystem(S_GETCOOKIE)`), siehe G3.
* **G2c** `ldgutil.c:205-225` `ldg_cpush`: m68k-Opcode-Array → Leeroperation (der Kernel hat beim Laden synchronisiert; PLAN Z. 456 sieht dies schon vor). Das ist der **eine** erlaubte m68k-Asm-Retarget-Fall.
* **G2d** `stdcall.s`/`stdcallgcc.o` (m68k-Assembler, `DEV/gcc.mak` `EXTRAOBJS`): wird für M1 nicht gebaut. `ldg_callback` (`ldg.h:168`) bliebe damit unbelegt. zView und die Codecs rufen es **nicht** (`grep` über `ZV`: nur `ldg_open/find/close/error/init/cookie/global`), `ex_lib` auch nicht. Entscheidung später (Frage 5).
* **G2e** Build: Quellliste = `OBJS = ldgapp.o ldglib.o ldgutil.o ldgload.o` (`DEV/common.mak`); `gcc.mak`/`gccm5475.mak` sind Vorlagen, Zieltriple/Flags nach `sdk/apps/build-qed.sh:80-90` (`-target aarch64-unknown-none-elf -ffreestanding -nostdlib -O2 -fPIE -fno-builtin -nostdinc`). Einbindung in `sdk/lib-stale.sh` (`lib_chain`/`lib_inputs`, Abhängigkeit: libc).
* **G2f** Herkunft/Pinnung: `Applications/` ist gitignoriert (`.gitignore:44`), `ldg-src` ist ein `curl`-Crawl ohne `svn`. Für ein im Repo gepflegtes SDK-Bauteil müsste ein Tarball mit Prüfsumme gepinnt werden (Muster `sdk/gemlib/gemlib-0.44.0.tar.xz`) — Frage 2.

**G3 — Cookie-Zugriff (LP64/32-Bit-Jar).** `Getcookie` liefert den Wert als `long` (`getcookie.c`), die Jar-Werte sind 32 Bit. Ein künftiger `LDGM`-Cookiewert muss ein Zeiger < 4 GiB sein — alle EL0-Adressen sind es. Ohne `LDGM`-Cookie gibt `ldg_cookie` 0 zurück, und `ldg_open/find/close` fallen auf den Direktladeweg (`ldgapp.c:37-38,54-61,71`). zView ruft `ldg_cookie` selbst (`ZV/plugins.c:68`) — über dieselbe libldg, also ebenfalls repariert; ohne Cookie greift dort `LDGPATH`, dann `"C:\\gemsys\\ldg\\"` (`plugins.c:62,70-73`).

**G4 — Endung `.NLD`.**
* Kernel: **nichts nötig** (keine Endungsprüfung, Abschnitt 3).
* Doku: **ADR-018** (`docs/MEMORY.md:1006-…`) kennt `.NAP/.NCP/.NTP/.NAC`; `.NLD` braucht einen Nachtrag (Rolle: native LDG-Bibliothek, nicht startbar; m68k-Analogon `.LDG`, das der spätere Emulator beansprucht). Das Muster „N_P“ trägt `.NLD` nicht (`N`+`LD`) — Textvorschlag für den Anwender, nicht von mir festgelegt (Frage 3).
* Desktop: Standardklassifikation `Doc` genügt (`ftype.rs:96`); eine ausdrückliche Zeile wäre optional.
* Image: `tools/make-sd-image.sh` (Muster `SD_MATHTST_AUTO`, `:299-313`): Dateien in 8.3-Namen (`EX_LIB.NLD`), Verzeichnis frei wählbar.

**G5 — Modul bauen.** Ein `.NLD` ist **dasselbe wie ein Programm**: PIE (`-fPIE`, `-pie -z text`), `sdk/newtos.ld`, `sdk/libc/crt0.S` (setzt die PRGFLAGS-Notiz mit `PF_STANDARD`), Archive `libldg.a`, `gemlib.a` (`appl_init` im Fehlerzweig von `main`), `libm.a` (`sin`, `cos`), `libc.a` — Linkzeile wie `sdk/apps/build-qed.sh:150-160`, nur Ausgabename `.NLD`. `ex_lib.c` selbst bleibt **unverändert** (`#include <gem.h>`/`<ldg.h>`, `__CDECL` ist im SDK leer: `libc/mintlib/include/compiler.h:101-107`). `PROC LibFunc[]` enthält absolute Zeiger in `.data` → genau die `R_AARCH64_RELATIVE`-Einträge, die der Kernel-Lader anwendet; `-z text` verhindert Text-Relokationen.

**G6 — Test-Client (M1-Probe).** Eigener, kleiner Client (Abschnitt 7). Der Demo-Client scheidet aus (windom, Abschnitt 2.4).

**G7 — zView-Overlay `plugins.c` (nicht M1, aber hier festgelegt, damit M1 es nicht verbaut).** Stellen: `ZV/plugins.c:93-96` (`extention` = letzte drei Zeichen des Namens, `strcmp(extention, "ldg")`); alles andere (`ldg_open(de->d_name, ldg_global)` `:98`, `ldg_find("plugin_init")` `:100`) bleibt. Muster: Overlay-Ersatzdatei im Stil `sdk/overlays/<app>/…` (Schattenbaum wie `build-teradesk.sh:104-125` oder Einzeldatei-Substitution wie `build-qed.sh:113-118`). Dazu müssen die Codec-Ausgaben `.NLD` heißen (Build der Codecs ist ohnehin neu: `ZV/plugins/common.mak` ist ein m68k-Cross-Build). Abhängigkeiten der Codecs: `libshare` (Abschnitt 5), Fremdbibliotheken (ALT §1/2).

**Was ausdrücklich keine Lücke ist** (belegt in Abschnitt 3): Laden/Relokieren durch den Kernel, Eigentum/Schutz des Modulblocks, Thread-Semantik von `Pexec 4`, Schreiben in den Client-Stack, `Pterm` lässt den Block stehen, `Mshrink` im Modul-Start, `Pexec 5`, `Mfree` des Moduls, `Pwait`-`EINVFN`.

---

## 5. `libshare`

> **Nachtrag 2026-10-06:** Der libshare-Quelltext ist gefunden (Arnaud Bercegeay, 0.1.1; `references/libshare/`). Abschnitt 5 und Frage 7 sind damit ueberholt, siehe [libshare-2026-10-06.md](libshare-2026-10-06.md).


**Suche (diese Sitzung):** Volltext (`grep -rIl`, ohne Binärdateien) über `Applications/`, `references/`, `docs/`, `sdk/`; Binärsuche (`grep -rla`) über `Applications/{ldg-2,ldg-src,windom-1.21.3,mintlib-src,pml-2.03,cflib-git}`; Dateinamen-Suche (`find -iname '*libshare*'`) über das ganze Volume (Treffer nur Qt-Bibliothek einer fremden App); der tho-otto-Spiegel `/Volumes/Daten/tho-otto-backu` (Dateinamen; `ldg-20171014.tar.xz` Inhaltsliste: `src/share/{internal.c,internal.h,backward.h}`; die zView-Archive dort enthalten Binärpakete). **Ergebnis: kein `libshare`-Quelltext, kein Header `libshare.h`, keine Bibliothek vorhanden.** Treffer auf den Namen nur in den zView-Quellen, `prerequisites.txt` und unseren Dokumenten.

**Falle:** `Applications/ldg-src/ldg/src/share/` ist **nicht** libshare. Die Dateien (`internal.c` Kopf: „La routine - complexe - qui permet d'avoir la liste internes des libraries … utilisé par les outils de maintenance (CPX et showlib)“) sind LDG-Verwaltungshilfen (`ldg_libslist`, `ldg_getlib`).

**Was zView/Codecs aus `libshare` brauchen [gelesen, Aufrufstellen]:**
* `#include <libshare.h>` (`ZV/plugins/common/zview.h:31`), Linkflag `-lshare` (`ZV/plugins/bmp/Makefile:4`), Voraussetzung „libshare 0.1.1“ (`Applications/z-tools/prerequisites.txt:7`).
* **Zwei Symbole, in allen 11 Codecs gleich:** `libshare_init()` als Aufruf in der Plugin-Funktion `init` (= exportiert als `"plugin_init"`) — z. B. `bmp/bmpldg.c:348`, `png/pngldg.c:346` — und `libshare_exit` als **`LDGLIB.close`-Eintrag** (`bmp/bmpldg.c:30`, `png/pngldg.c:35`; alle Codecs: `neochrome/neoldg.c:29`, `eureka/rawldg.c:30`, `tga/tgaldg.c:39`, `jpg/jpgldg.c:44`, `img/imgldg.c:32`, `godpaint/godldg.c:35`, `degas/degasldg.c:29`, `tiff/tiffldg.c:44`, `gif/gifldg.c:33`).
* **Signaturen [Schluss] aus den Typen**, nicht gelesen: `libshare_init` wird als Anweisung ohne Argumente aufgerufen; `libshare_exit` wird einem `void (*close)(void)` zugewiesen (`ldg.h:122`) → `void libshare_exit(void)`.
* **Was die Funktionen tun, ist [offen].** Es gibt keine Quelle, aus der man das ablesen könnte. Vermutungen („Initialisierung gemeinsamer Bibliotheken“) sind Namensraten und werden hier nicht festgehalten. Ein Ersatz darf nicht geschrieben werden, bevor der Quelltext oder eine Spezifikation vorliegt (Frage 7).
* Für **M1 irrelevant**: `ex_lib` bindet `libshare` nicht.

---

## 6. LDG-Seiten: M1 vs. später

| Teil | M1? | Begründung / Beleg |
|---|---|---|
| Client-API `ldg_open/find/close` im Direktladeweg | **ja** | `ldgapp.c:37-38,54-61,71` |
| Modul-API `ldg_init`, `ldg_getpath` | **ja** | `ldglib.c:97-151` |
| `ldg_error` | nur Sichtprobe | ohne Cookie immer `LDG_NO_TSR` (`ldgapp.c:103`) — kein Fehlerkanal |
| TSR `ldg.prg` / Cookie `LDGM` | **nein** | optional (`ldgapp.c:37-38`); `ldg_open_flag` bringt Teilen/Zählen/Pfadsuche (`ldgcook.c:157-299`, `:83-144`), alle zView-Codecs sind `LDG_NOT_SHARED` (`bmp/bmpldg.c:27-31`: „we don't know if the user has ldg.prg … installed“) |
| zView-Rückfall ohne Cookie | **trägt** | `ZV/plugins.c:62-73` (`LDGPATH`, dann `C:\gemsys\ldg\`) |
| `LDG_NOT_SHARED`, `LDG_RESIDENT`, `LDG_LOCKED`-Semantik | nein | nur im TSR ausgewertet (`ldgcook.c:179,248`) |
| `ldg_Malloc/Calloc/Realloc/Free` | nein | gebraucht, sobald ein Modul dem Client Speicher gibt; nutzt `Mxalloc(size, 3)` (`ldgutil.c:150-156`; `3+8+32` nur bei `MiNT`/`MagX`-Cookie, den NewTOS nicht seedet); PLAN Z. 184 ordnet `3+8+32` als `PROT_G` ein |
| `ldg_callback`/`stdcall` | nein | m68k-Assembler (G2d) |
| Cookiewert auf `LDG_INFOS` (Zeiger < 4 GiB, Ziel `PROT_G`/TSR, PLAN Z. 209) | nein | späterer TSR/Cookie-Manager |
| `ldgmgr`, `ldgd`, `ldg.cpx` | nein | `ldgd` verteilt SHM-Anforderungen (MiNT) |
| Offener Punkt für später | — | `TASK/ARCH03-3c-STATUS-UNFINISHED.MD`: „S2ALL/TSRPAR unter Schutz an/strict“ ist als offen geführt — betrifft resident gehaltenen Code (`Ptermres`), also einen späteren TSR |

---

## 7. M1 — Schrittliste mit Arbeitspaketen

**Ziel M1:** Ein natives `EX_LIB.NLD` (unverändertes `LDG/demo/libs/ex_lib.c`) wird über den **echten LDG-Weg** (`ldg_open` → `ldg_load` → `Pexec 5/3/4`, `ldg_find`, Funktionszeigeraufruf, `ldg_close`) geladen; `sinus()` liefert einen Wert, der gegen `sin()` im Client stimmt.

**Abnahme (Vorschlag):**
* `ldg_open("C:\\LDG\\EX_LIB.NLD", gl) != NULL`, `ldg->magic == LDG_COOKIE`, `ldg->num == 3`, `ldg->infos` = „Example of library with floating parameters.“ (`ex_lib.c:44`);
* `ldg_find("information")` liefert den Text `[1][Example of math library!!!][Ok]` (`ex_lib.c:50`);
* `sinus(&r)` mit `r.value = 0.5` stimmt mit `sin(0.5)` des Clients auf kleine Toleranz überein (Gleitkomma nicht über `printf`, sondern wie `sdk/examples/mathtst.c` per Festkomma-Ausgabe — dort aus gutem Grund „so the result does not hang on stdio's float formatting“);
* `ldg_close` → anschließend `Malloc` des vorher belegten Bereichs möglich (Block frei);
* zweites `ldg_open` derselben Datei **ohne** TSR lädt neu (`ldgapp.c:37-38`), gibt zwei verschiedene `LDG*` zurück; beide `ldg_close` fehlerfrei.

**Schritte und Dateien:**

| # | Arbeitspaket | Ebene | Dateien (Vorschlag, noch nicht angelegt) | Voraussetzung |
|---|---|---|---|---|
| S1 | **B1/G1:** Environment-Walk (Weg (a) libc oder (b) libldg-lokal, nach Entscheidung Frage 1). Bei (a): Host-Test im Stil `sdk/libc/tests/cmdargs-host.c` (in `cmdargs.c:100-102` zitiert) | SDK/libc | `sdk/libc/newtos-arch/cmdargs.c` (oder neue Datei daneben), Test unter `sdk/libc/tests/` | Entscheid Q1 |
| S2 | `libldg` bauen: Quelle pinnen/abgleichen (Q2), Overlays G2a/G2b/G2c, Build-Skript, Eintrag in `lib_chain` | SDK | `sdk/ldg/build-ldg.sh`, `sdk/ldg/overlay/{ldgload.c,ldgutil.c}`, Änderung `sdk/lib-stale.sh` | Q2 |
| S3 | Modul `EX_LIB.NLD` bauen (G5): `ex_lib.c` unverändert, `libldg.a gemlib.a libm.a libc.a`, Ausgabe `.NLD` | SDK/Skript | `sdk/apps/build-ldg-demo.sh` (Muster `build-qed.sh`) | S1, S2 |
| S4 | Probe-Client `LDGTEST.NCP`: eigener Code, ruft `ldg_open/find/close` der libldg, bindet `demo/libs/exlibbnd.c` **unverändert** (`init_ldg_math`), `gl[3]`-Feld lokal (kein AES nötig, nur `gl[2]` wird gelesen: `ldgapp.c:38`); Ausgabe per `Cconws` + serielle Verdictzeile; läuft als `\AUTO\*.NCP` (vor dem AES; Konvention aus `memory auto-folder-console-only`/`make-sd-image.sh`) | Test-Probe | `sdk/examples/ldgtest.c` (Muster `mathtst.c`) | S3 |
| S5 | Image: `EX_LIB.NLD` nach `C:\LDG\`, `LDGTEST.NCP` auf `C:` (+ optional `\AUTO\` per Schalter nach Muster `SD_MATHTST_AUTO`). **Nur QEMU-Image**, nicht die echte Karte | Tools | `tools/make-sd-image.sh` | S3, S4 |
| S6 | ADR-018-Nachtrag `.NLD`, kurzer Abschnitt in `docs/sdk/` (LDG-Modul bauen) | Doku | `docs/MEMORY.md`, `docs/sdk/…` | Q3 |
| S7 | Lauf in QEMU raspi4b (`serial=on`), alle drei Schutzmodi `memprot=off/on/strict` (die Spec verlangt Funktion in jedem Modus, Abschnitt 3 Zeile 7/10), Evidenzordner `measurements/measurement-2026-10-…-<ticket>/` | Messung | — | S5 |
| S8 | Pi-Lauf (nur auf Ansage des Anwenders): Abschnitt 8 | Messung (HW) | — | S7 |

**Kernel:** keine Änderung bekannt (Abschnitt 3). Findet S7 etwas, ist das ein eigenes Ticket, kein Teil dieses Entwurfs.

**Nach M1 (Reihenfolge nach ALT §5, angepasst):** Fremdbibliotheken (M2) → windom (M3, danach läuft auch der unveränderte `demo/client`) → zView mit `plugins.c`-Overlay (G7) + `libshare`-Klärung (M4). TSR/Cookie-Manager erst bei Bedarf.

---

## 8. Was nur auf der echten Pi prüfbar ist

* **I-Cache-Kohärenz des frisch geladenen Moduls.** QEMU simuliert keine Caches (Kommentar `NT/loader.rs:594-597`). Prüfung: erster Aufruf von `sinus()` nach `Pexec 3`; dann `ldg_close`, anderes Modul gleicher Größe an derselben Adresse laden (First-Fit gibt den Block wahrscheinlich zurück [Schluss]) und seine Funktion aufrufen — veralteter Code in der I-Cache-Zeile würde sich als falsches Ergebnis zeigen. Quelle der Architekturaussagen: V0a E3 (Folge `DC CVAU; DSB ISH; IC IVAU; DSB ISH; ISB`, kein `IDC/DIC` beim A72: `docs/analysis/arch03-stage0a-armarm-verification.md:99`).
* `sync_code_cache` läuft über den **High-Half-Alias** (`tpa.base_kva`), die Ausführung über die Niedrig-VA (VA = PA); das Kommentar-Argument ist „A72-Caches sind PIPT“ (`NT/loader.rs:594-598`) — **[offen]**, ob das auf der Pi für die I-Cache-Seite trägt; die Messung oben deckt es ab.
* Zusammenspiel mit `asid=on` (Stufe 4, `7cab226`): Modul-Thread und Aufrufer haben **eigene Wurzeln/ASIDs auf denselben Seiten**; Datenkohärenz ist bei PIPT-D-Cache kein Problem [Schluss], TLB-Verhalten bei `Pexec 4`-Wechsel in QEMU nicht aussagekräftig (QUIRK-001: TLBI vor `eret`, `KX:2060-2075`).
* Nicht nötig: `SCTLR_EL1.UCI` (nur relevant, wenn ein Modul selbst Code erzeugt, V0a E13).

---

## 9. Offene Fragen an den Anwender

1. **Environment-Walk (B1):** libc-weit (Weg a: `getenv` funktioniert für alle Programme, berührt jeden Start, Nähe zu `TASK/PORT09`; ist PORT09 nach ARCH03 wieder offen?) oder nur in libldg (Weg b: lokal, `getenv` bleibt sonst leer)? Meine Lesart: (a) ist die richtige Heimat (mintlib-Parität), (b) ist der risikoärmere Notweg.
2. **Quellpinnung `ldg-src`:** `Applications/ldg-src` ist ein `curl`-Crawl von SVN r133 ohne Prüfsumme; für ein SDK-Bauteil ein Tarball mit sha256 unter `sdk/ldg/`? Soll der th-otto-Stand `ldg-20171014` (mit `ldg-cross.patch`, `ldg-pwait.patch`, ungelesen) zum Abgleich herangezogen werden? **Nicht geprüft:** Unterschied r133 ↔ 20171014.
3. **ADR-018-Nachtrag `.NLD`:** Wortlaut (Rolle „native LDG-Bibliothek“, nicht per Doppelklick startbar, m68k-Analogon `.LDG` für den späteren Emulator). Desktop: Standard `Doc` belassen oder explizite Klasse?
4. **Probe-Form:** eigener minimaler Client `LDGTEST.NCP` (headless, serielle Verdictzeile, ohne windom) für M1 — und der **unveränderte `demo/client`** erst nach windom (M3)? Oder ist M1 ohne den Demo-Client nicht „abgenommen“?
5. **`ldg_callback`/`stdcallgcc.o`:** für M1 weglassen (nicht benutzt). Später: AArch64-Kleinstück schreiben (Argumente um ein Register verschieben, `br`), oder das Symbol bewusst als „nicht vorhanden“ führen?
6. **Stack je Modul:** `_crtinit` behält 64 KiB Stack pro geladenem Modul (`heap_init.c:190`, `_stksize` = `MINKEEP`); die Module nutzen ihn nie (Funktionen laufen auf dem Client-Stack). Ein kleinerer `_stksize` im Modul wäre möglich (mintlib erlaubt ihn per Variable). Gewünscht oder Original-Verhalten belassen?
7. **`libshare`:** Quelle liegt nirgends. Woher? Ohne sie sind die 11 Codecs nicht linkbar; ein Ersatz setzt eine Spezifikation voraus (Abschnitt 5).
8. **TSR:** Wird ein NewTOS-`ldg.prg` (LP64-Neubau von `LDG/src/kernel`, m68k-Assembler `cookie.s`, SHM über `Fcntl`) je gebraucht, oder genügt der Direktladeweg dauerhaft (Codecs sind ohnehin `NOT_SHARED`)? Nicht M1.
9. **`PRGFLAGS` der Module:** Standard `PF_STANDARD` ⇒ `PROT_P`, Besitzer = Client. Für mehrere Clients (TSR) müssten Module `PROT_G` tragen (Atari ließ das per `PRGFLAGS.PRG` setzen — `Applications/ldg-2/…\68K\bin\prgflags.ttp`; bei uns `cargo xtask prgflags`, `docs/api/gemdos/process/4b-pexec.md`). Nicht M1.

---

## 10. Ungeprüft und Vorbehalte

* **Nicht gelaufen.** Alles in Abschnitt 3 ist Quelltextlesung, kein Lauf; Zeilen mit [Schluss] sind Folgerungen. Die Probe (S7) ist der Beleg.
* **Nicht gelesen:** `ldg-cross.patch`, `ldg-pwait.patch` (th-otto 2017); `LDG/test/callback`; `LDG/src/kernel/cookie.s` und der TSR im Detail (nur die in Abschnitt 2.5 zitierten Stellen).
* **`path::resolve_parent`** (relative Pfade, aktuelles Verzeichnis des Aufrufers beim `Pexec`-Dateiöffnen): nur `findit` gelesen (`tos-api/src/bdos/fs/path.rs:166-169`); ob ein relativer Name (`ex_lib.nld`, wie zView ihn nach `chdir` übergibt) gegen das **Prozess**-Verzeichnis aufgelöst wird, ist **[offen]**; die Probe sollte einen absoluten Pfad **und** einen relativen testen.
* **`Mshrink`-Rechte** eines „weiteren Besitzers“ unter `memprot=strict`: `mshrink` in `traps.rs:797-847` prüft keinen Besitzer; ob `gemdos/memory.rs` davor eine Prüfung ergänzt, wurde nicht gelesen (Abschnitt 3 Zeile 8 stützt sich auf die tos-api-Ebene). Die Probe in `strict` klärt es.
* `OFFSETLDG`-Länge/`ENV_BUF_LEN = 512`: eine größere Client-Umgebung wird gekürzt, nicht abgelehnt (`KX:1809-1819`) — [gelesen]; ob das je auffällt, hängt von Clients mit großer Umgebung ab.
* Aufwandszahlen sind Schätzungen.

## 11. Out-of-scope-Beobachtungen (je eine Zeile)

* `docs/analysis/zview-ldg-2026-10-04.md` §3.0 nennt `options.h`/`version.h` fehlend und `LDG_NUM` offen — beides durch Abschnitt 0 Nr. 6 überholt (Datei unverändert gelassen, wie angewiesen).
* `TASK/PORT02.md` führt weiterhin „Laderquelltext nicht veröffentlicht“/„`zcodeclib` Unterbau“ (ALT §8) — Ticket bei Gelegenheit nachziehen.
* `docs/sdk/09-port-material.md` Z. 113 („`libshare` … nicht darin gefunden“) stimmt weiter; „`libmem` steckt möglicherweise in `ldg-2`“ ist richtig: `LDG/src/mem` (`mem.ldg`).
* Der Doppelklick-Startpfad auf ein `.NLD` im Desktop ist nicht definiert (landet als `Doc`); `main()` jedes Moduls meldet dann „This program is a LDG library“ per `form_alert` (`ex_lib.c:66-69`), wenn doch eines gestartet würde.
