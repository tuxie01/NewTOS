# BIOS15 — Quittung: ADR-031 umgesetzt (Warnung je Haken und Prozess, Besitz, `prt_*`, `Mfpint`/`Xbtimer`-`d0`)

Datum 2026-10-29 · Beleg
[measurements/measurement-2026-10-29-bios15/](../measurements/measurement-2026-10-29-bios15/RUNINFO.TXT)
· Ticket [BIOS15](BIOS15.md) · ADR-031 (`docs/MEMORY.md`)

- **Ein Mechanismus (Arbeit 1):** `tos-api/src/bdos/process/hooks.rs` —
  `Hook` (die nicht bedienten Haken der ADR-Tabelle, 256 Vektoren + 24
  benannte), je Prozess-Slot ein Bit-Satz „schon gewarnt“
  (`tos-api/src/bdos/process/hooks.rs:504@note`), eine Warteschlange von
  `Warning`, die der Kern nach jeder Ausnahme ausgibt
  (`kernel/src/console.rs:407@report_hook_warnings`, gerufen aus
  `rust_sync_handler`; tos-api hat kein Protokoll). Zurueckgesetzt mit dem
  Slot (`free_slot` → `hooks::release_slot`). Die Accessory-`kbdvec`-Warnung
  (`accessory_hook_unwarned`) ist darin aufgegangen; der ADR-Verweis in
  `docs/MEMORY.md` zeigt jetzt auf `note` (Anker war durch die Verschiebung
  ohnehin gebrochen — redaktionell, bitte pruefen). Zeile:
  `[hook] warning: slot <n> (PD …) installed <hook> = <wert> via <stelle>;
  NewTOS never calls it: <grund> (ADR-031)`.
- **Erkennungsstellen:** `Setexc` (`etv_timer`, `etv_critic`, Vektoren
  `< 0x100`, Wert ≠ Startwert; nicht `etv_term`/`etv_xtra`/Systemvariablen;
  `tos-api/src/bios/vectors.rs:209@note_write`); Seite 0 (`hdv_*`, `swv_vec`,
  `exec_os`, `resvector`, `dump_vec`, `vblqueue`, `bell_hook`, `kcl_hook`,
  direkt geschriebene `etv_*`- und CPU-Vektor-Zellen; nur lesen, Vergleich
  mit dem Lader-Wert, `tos-api/src/bdos/process/hooks.rs:298@check_page0`)
  beim Verlassen des Super-Zustands (`set_super`), beim Ende im
  Super-Zustand (`terminate_inner`), nach einem `Setexc` in Seite 0 **und
  nach jedem Trap im Super-Zustand** (`hooks::after_trap`) — Letzteres steht
  nicht woertlich in ADR-031 Regel 4: TeraDesks `force_mediach` stellt
  `hdv_*` vor dem Verlassen zurueck, ohne diesen Punkt bliebe genau der
  Fall der ADR still (Lead/Anwender: bitte bestaetigen, steht so in
  `docs/COMPATIBILITY.md`); `Kbdvbase`-Slots beim ersten Ereignis, das sie
  riefe (IKBD-Byte → `midisys`/`ikbdsys`, Paket → `mousevec`/`statvec`/
  `clockvec`/`joyvec`, Taste → `kbdvec` eines Accessorys; der Dekoder setzt
  nur ein Atom, auch im IRQ; `tos-api/src/xbios/kbdvecs.rs:447@check_events`);
  `Initmous` (Typ 1/2/4, `vec` ≠ 0), `Mfpint`, `Xbtimer`, `vex_butv/motv/
  curv/timv` im Aufruf. Keine Warnung fuer `etv_xtra`, `hdv_init`,
  `hdv_boot`, `prt_*`, `aux_*`; `midivec`/`vkbderr`/`vmiderr` nie (Ereignis
  tritt nicht ein).
- **Besitz (Arbeit 2):** geteilte Zellen = `Setexc`-Schatten bis `0x103` und
  `vex_butv/motv/curv/timv`; je Prozess bis 16 Eintraege (Zelle, zuletzt
  geschrieben, Wert davor; ein Zurueckschreiben des Vorwerts = Aushaengen).
  `Pterm`: Zelle haelt noch den Haken → Vorwert zurueck; anderer Haken oben
  → stehen lassen + `[hook] warning: … ended without unhooking …`; schon
  ausgehaengt → nichts. `Ptermres`: bleibt
  (`tos-api/src/bdos/process/hooks.rs:591@on_terminate`, aus
  `terminate_inner`). Mehr als 16 → Warnung „not tracked“.
- **`prt_stat`/`prt_vec` (Arbeit 3):** der gemeinsame `ret`-Stub
  (`RET_STUB_VA`, EmuTOS `just_rts`, `bios/chardev.c:82-83`) statt 0
  (`tos-api/src/bdos/process/loader.rs:1288@RET_STUB_VA`). `aux_*` bleiben 0 (nicht im
  Ticket).
- **Anwender-Zusatz `Mfpint`/`Xbtimer`:** Orakel gemessen (Sonde
  `tosdump/probes/hkprobe.c`, tt = falcontc bytegleich, zweimal gleich):
  `Mfpint` liefert das MFP-Maskenregister nach dem Setzen (IMRB fuer 0–7,
  IMRA fuer 8–15, nullerweitert; IMRA 0x14, IMRB 0x60 waehrend ein Programm
  laeuft): `Mfpint(5)` 0x60, `Mfpint(0x15)` 0x60, `Mfpint(12)` 0x14,
  `Xbtimer(2,…)` 0x60; `Xbtimer` ohne Timer liefert `control` als WORD:
  `Xbtimer(4,0x1234,…)` 0x1234, `Xbtimer(-1,0x8001,…)` 0x8001 — deckt sich
  mit der Disassembly von `obj/mfp.o` des Pins. NewTOS vorher 0x0d/0x1f,
  nachher gleich dem Orakel (`tos-api/src/xbios/mfp.rs`, Abbild der zwei
  Maskenbytes, `jenabint`-Bit wird gesetzt). Spec neu:
  `docs/api/xbios/0d-mfpint.md`. `Jdisint`/`Jenabint` unveraendert
  (Funktionsnummer) — dort offen notiert.
- **Messung (Arbeit 5):** Sonde P01/P02: `prt_stat` ≠ 0, = `prt_vec`, ein
  Aufruf durch beide kehrt zurueck — EmuTOS ja, NewTOS vorher nein (0),
  nachher ja. Die Sonde ist dauerhaft (`make -C tosdump hkprobe-m68k
  hkprobe-newtos`, `tosdump/probes/run-hkprobe.sh`); `tools/hatari-oracle.sh`
  kennt `HKPROBE.PRG` als Sonde ohne Platte (jedes Profil, nicht
  destruktiv). Abnahme-Log: die Sonde haengt `hdv_mediach` und `etv_timer`
  ein → je genau eine Zeile (dazu je eine fuer die zwei `Mfpint`-Vektoren).
- **Retest:** bios-core-r1 15/44 · 14/44, xbios-chardev-r1 12/23 · 12/23,
  xbios-system-r1 9/43 · 9/43, super-r1 0/9 · 0/9 (tt · falcontc) vorher
  (0d3bb485) und nachher, Falllisten identisch, keine `[hook]`-Zeile.
  TeraDesk (Injektor, Ablauf der MEM12-Abnahme): Start ohne Warnung, erstes
  Shift+Esc genau `hdv_bpb`, `hdv_rw`, `hdv_mediach`, zweites keine, Ctrl+Q →
  Reboot → Reset, kein Absturz. DOOM: „keys via kbdvec“, F10/y ueber den
  Haken, rc 0, keine Warnung. raspi4b bis Desktop in allen Laeufen,
  qemu-virt „desktop up“.
- **Tests:** `cargo xtask test` 1808/0/1 (Basis 1790/0/1; +12 Integration
  `tos-api/tests/bios15_os_hooks.rs`, +3 `hooks`, +2 `loader`, +1 `mfp`;
  `tos-api/tests/xbios_kbdvecs.rs` Accessory-Test auf den neuen Mechanismus
  umgestellt). `make -C tosdump host-test` gruen, SDK-Hosttests 27/0/0. Bau
  rpi4/qemu-virt release je 159 Warnzeilen wie Basis; Clippy kernel (beide
  Boards) und tos-api: dieselbe Warnungsmenge wie Basis.
- **`mapping.tsv`:** `mfpint`, `xbtimer` → `xbios/mfp.rs`; `Initmous` →
  `note_installer` (nur Warnung); `chardev_init` → `populate_sysvars`
  (`prt_*`); `setexc` + `note_write`; `int_acia` → `check_events`;
  MFP-Restzeilen-Notiz angepasst. `--check-mapping` gruen.
- **doc-check:** 0 Fehler in allen bearbeiteten Docs; keine neue
  Anker-Verletzung in Docs, die geaenderte Dateien zitieren (Vergleich
  gegen den HEAD-Stand; `docs/api/xbios/0e-iorec.md:447` nachgezogen).
  `docs/STATUS.md` nicht angefasst (Lead).

## Offen / nicht geprueft

- TeraDesk „Shutdown“ (`lobo`: `Setexc(0x70)` → eine Zeile Vektor 0x1c
  erwartet, `Setexc(0x114)` keine, danach die gewollte Endlosschleife) —
  nicht gefahren; die `Setexc`-Vektorwarnung selbst deckt der Hosttest
  `setexc_warns_once_per_unserved_hook_and_process` ab.
- Echte Pi 4.
- `Setexc(0x40+n,-1)` liefert fuer MFP-Vektoren 0 (Schatten), EmuTOS die
  Adresse — BIOS07.
