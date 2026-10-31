# DOOM01 Ursachenforschung, 2026-10-31

Stand: **Ursache nicht belegt.** Statische Analyse, kein QEMU-Lauf, keine Hardware (Auftrag: nur Bericht).
Gelesen = mit Quellcite; Schluss = Folgerung, so markiert.

## 1. Wo DOOM im Menue die Tasten liest

Gelesen:

- Der Weg im C1-Log ist der kbdvec-Haken: `DOOM: keys via kbdvec -- TOS 4.04, Kbdvbase 0007f010, kbdvec was 0007e048` (`measurements/measurement-2026-10-31-pitest01-c1/serial/screenlog-c1.txt:3123`). `0007e048` ist `KBDVEC_DEFAULT_VA` (`tos-api/src/xbios/kbdvecs.rs:350@route_key`, Stub davor).
- Pro Frame ruft DOOM `vec_poll`: `evnt_multi(MU_KEYBD|MU_TIMER, Timer 0)` in einer Schleife, die AES-Tastenevents liest und verwirft; danach leert es den Hook-Ring (`sdk/apps/doom/doomgeneric_gem.c:1993@vec_poll`). Schlaf zwischen Tics ist `evnt_timer` (`DG_SleepMs`, `sdk/apps/doom/doomgeneric_gem.c:683@DG_SleepMs`). Zeit ist die Seite-0-Zelle `_hz_200` (`:675`).
- Kernel, Tastenbyte: `decode_byte` -> `route_key` (`kernel/src/ikbd.rs:150@decode_byte`); der Besitzer ist der innerste gestartete Prozess, also DOOM, auch wenn gerade ein Zubehoer die CPU hat. Byte in DOOMs 64er Ring; ein wartender Besitzer wird per `wake_for_upcall` bereit gemacht (`tos-api/src/xbios/kbdvecs.rs:350@route_key`, `tos-api/src/bdos/process/sched.rs:904@wake_for_upcall`).
- Zustellung: beim Syscall-Rueckkehr als EL0-Upcall (`kernel/src/arch/aarch64/exceptions.rs:829@deliver_pending_kbdvec`), Hook speichert das Byte und ruft per Tail-Call den Default-Stub, der `Ssystem(2002)` macht und damit `kbd_int` -> `decode_key` (`kernel/src/ikbd.rs:243@decode_key`) -> `console::push_rx`. **Die Ctrl+Alt+Del-Pruefung liegt in `decode_key`** (`kernel/src/ikbd.rs:251@reset_key`). Folgerung: Strg+Alt+Entf nach dem Freeze beweist, dass DOOM weiter Traps macht und die Hook-Kette (Upcall, `Ssystem`) noch laeuft. Ein Haenger im Hook oder im Kern-Warten ohne Pump ist damit ausgeschlossen (deckt sich mit `TASK/DOOM01.md`). Ausnahme: der Reset kommt aus der Absturz-Wartschleife (`drain_hook_bytes`, `kernel/src/arch/aarch64/crash.rs:82@report_and_wait`).
- Der Pump: eine USB-Meldung pro Trap-Pump (`kernel/src/host_env.rs:504@pump_devices`), aufgerufen vom AES-`dsptch` jedes AES-Aufrufs und aus `sleep`/`dsptch` im Leerlauf (`tos-api/src/aes/mod.rs:802@pump_input`, `tos-api/src/bdos/process/sched.rs:844@pump_input`). `pump_input` oeffnet dabei kurz die IRQ-Maske (`kernel/src/host_env.rs:91@pump_input`). Der Timer-IRQ pumpt nur Zeigerbewegung; Tasten bleiben "held" bis zum Trap-Pump (`kernel/src/host_env.rs:549@irq_input_pump`).

## 2. Slot 1 und warum 1 <-> 2 wechselt

Gelesen: Slot 0 = `C:\ACC\SYSACC.NAC`, Slot 1 = `C:\ACC\XSHOT.NAC`, Slot 2 = DOOM (`screenlog-c1.txt:157-159,3115`). XSHOT wartet in `evnt_multi(MU_MESAG|MU_BUTTON|MU_KEYBD)` ohne Timer (`TASK/A117.md:39`); XSHOT liegt auf der Pi-Karte (`measurements/measurement-2026-10-31-pitest01-card/write-a.txt:64`), steht aber in keiner Manifest-/Quelle des Repos (kein lokales `xshot.c`; Fremd-Binary, nur das Ticket zitiert seine Quelle).

Schluss (aus Code): Jeder Eingabeimpuls (Taste, Mausbewegung) setzt im Leerlaufdurchgang `wake(wait_input())` (`tos-api/src/bdos/process/sched.rs:1215@idle_wake_pass`); der Union-Wait von XSHOT deckt `WAKE_INPUT` (`:917@multi_covers`). Am Ende von DOOMs naechstem AES-Aufruf (`dispatch_after_call`, `:1118`) laeuft der bereite XSHOT; er findet kein Ereignis fuer sich, schlaeft und gibt zurueck (`yield_after_completed_call`, `kernel/src/arch/aarch64/exceptions.rs:724@yield_after_completed_call`). So erklaeren sich 309 Hin-und-Rueck-Paare ueber den ganzen DOOM-Lauf (kein Wechsel ohne Eingabe). Die Zuordnung "309 = Eingabeimpulse" ist nicht gezaehlt, nur gefolgert.

## 3. Was bei fehlender serieller Ausgabe anders ist

Gelesen: In dem DOOM-Bereich druckt der Kern nur in `apply_switch`: zwei Zeilen je Wechsel, **genau zwischen** `restore_boot_ttbr0` (leere Wurzel + `tlbi vmalle1` + `dsb ish` + `isb`, `kernel/src/arch/aarch64/user_entry.rs:101@restore_boot_ttbr0`) und `install_ttbr0` (`dsb ish; msr ttbr0_el1; isb`, **kein** TLBI, `tos-api/src/bdos/process/mmu.rs:1688@install_ttbr0`) (`kernel/src/arch/aarch64/exceptions.rs:1286@restore_boot_ttbr0`, `:1290@install_ttbr0`, Zeilen dazwischen `[sched] switch`/`[sched] running`). `serial=on` schaltet nur `write_byte` frei (`kernel/src/console.rs:91@configure_serial_tx`, `SerialConsole`). Ohne `serial=on` entfallen also genau diese Schreibzugriffe:

1. **Zeit.** Eine Wechselzeile kostet mit 115200 Baud (Annahme der Baudrate, nicht gelesen) grob 2-4 ms (Phase B misst 16,6 ms je protokollierter AES-Zeile, `TASK/DOOM01.md`). Ohne Ausgabe ist ein Wechsel Mikrosekunden. Der Kern laeuft ohne `serial=on` also in der Wechselkette um Groessenordnungen schneller als jeder bisher gemessene Lauf.
2. **Reihenfolge/Barriere.** Ohne die Ausgabe liegen TLBI und TTBR0-Install direkt hintereinander. MEMORY.md QUIRK-001 (`docs/MEMORY.md:3286`) nennt die Ausgabe selbst "load-bearing" auf QEMU und nennt sie "harmlos" auf echter HW, belegt das aber nur fuer QEMU. Architektonisch ist die Folge ohne Ausgabe zulaessig (TLB leer vor Install, `sync_now` aendert Eintraege der noch nicht installierten Wurzel); ein Fehler waere nur mit einem Cortex-A72-Befund zu begruenden. **Ungeprueft, kein Beleg.**
3. **Uhr.** `_hz_200` wird nur in die Seite 0 des laufenden Prozesses geschrieben (`tos-api/src/bios/tick.rs:108@publish`, TICK01). Nach einem Wechsel liest DOOM bis zum naechsten Tick einen alten Wert. Mit Ausgabe vergehen Millisekunden je Wechsel (Tick kommt oft dazwischen, auch im Wechselfenster), ohne Ausgabe ist das Fenster haeufig. Die Zeit des Kerns fuer Schlaf/Timer ist ein anderer Zaehler (`monotonic_millis`, `tos-api/src/bdos/process/sched.rs:945@wake_timers`). Folgerung: Zwei Uhren laufen in DOOMs Schleife (Seite-0-Uhr fuer Tics, Kernuhr fuer `evnt_timer`) - ihre Differenz haengt vom Wechselaufwand ab. Ob das mehr als eine kurze Verzoegerung gibt, ist nicht gezeigt (HZPROBE ohne Wechsel bestand: `measurements/measurement-2026-10-31-pitest01-phase-a/RUNINFO.TXT`).

EmuTOS-Vergleich: Es gibt dort eine einzige globale `_hz_200`-Zelle (`bios/vectors.S:503`, laut `tos-api/src/bios/tick.rs` Kopf) und keinen Prozess-Wechsel mit TTBR0; die Wechselkette ist NewTOS-eigen (Kern-intern, nicht Vertrag). Vertrag ist: jeder Prozess liest die laufende Uhr (TICK01).

## 4. Kandidaten, gereiht, mit entscheidender Messung

Beobachtung, die jede Hypothese erfuellen muss: Phase A, Kontrolle: ohne `serial=on` Freeze bei zweitem Pfeil-runter, Strg+Alt+Entf geht. Mit `serial=on` (C1, B) nie. Aus Phase A auch: ein P0PAR-Lauf "froze" (Foto zeigt aber nur den erwarteten "Press any key"-Schirm von Fall w, `measurements/measurement-2026-10-31-pitest01-phase-a/phase-a-results/p0par-case-w-last.jpg`; ob das der Haenger war, ist laut RUNINFO nicht bestaetigt, also **kein** Beleg).

| # | Kandidat | Stuetzt | Schwaeche |
|---|----------|---------|-----------|
| A | Zeitverhalten des Wechsels (Abschnitt 3.1/3.3): schneller Wechsel XSHOT<->DOOM ohne Ausgabe, Uhr/Tick-Fenster | einzige Kern-Differenz im C1-Bereich; Wechsel hangen an Tasten (Foto bestaetigt nur Slot-Namen) | kein Mechanismus gezeigt, der Dauer-Stillstand ergibt |
| B | Barriere/TLB-Reihenfolge am Wechsel (3.2), nur auf A72 sichtbar | QUIRK-001 nennt die Ausgabe selbst Barriere; QEMU kann es nicht zeigen | Folge ist architektonisch zulaessig |
| C | Anderes Zeitverhalten der USB-/Taste-Kette (Held-Meldung, Wiederholung, `poll_repeat` nach `hz_200`) schneller ohne Ausgabe | Freeze beim zweiten Druck: Wiederholung/zweite Meldung | im Code keine Sperre gefunden (`capture_begin` gibt nach Trap-Pump frei) |

Alle drei sind ungeprueft; keiner ist durch Code allein entscheidbar.

Entscheidende Messungen, billigste zuerst, alle auf der Pi (QEMU hat weder A72-Cache/TLB noch USB, und der 2026-10-29-Lauf `noser1` ohne `serial=on` lief ohne Freeze, `measurements/measurement-2026-10-29-doom-freeze/RUNINFO.TXT`):

- **M1 (trennt "Wechsel beteiligt" von "nicht")**: Karte ohne `XSHOT.NAC` und ohne `SYSACC.NAC` (kein Peer, kein Wechsel), `cmdline.txt` leer, DOOM-Menue. Kein Freeze => Wechselkette ist notwendig (A oder B). Freeze => Wechsel ausgeschlossen, Ursache in Tasten-/Uhrpfad (C oder neu).
- **M2 (trennt Zeit von Barriere)**: ein Kern, der die zwei `[sched]`-Zeilen durch eine Spin-Schleife von ~3 ms ersetzt (Produktaenderung, nur nach Freigabe), `serial=on` aus. Kein Freeze => Zeit (A). Freeze => nicht die Verzoegerung allein. Gegenprobe M2b: `serial=on`, aber die zwei `[sched]`-Zeilen entfernt: Freeze => die Zeilen sind die notwendige Bedingung.
- **M3 (Schnappschuss, schon als Arbeit 3 in `TASK/DOOM01.md` vorgesehen)**: bei Strg+Alt+Entf (vor dem Reset) DOOMs `_hz_200` gegen den Kernzaehler, `upcall_depth`, wartender Zustand, Ringe auf den Schirm. Trennt "DOOM schlaeft/waehlt" von "DOOMs Uhr steht" (K6) von "DOOM im Absturz-Warten" (K5), unabhaengig von der Serielle.

Reproduktion in QEMU ohne Serielle: **nicht versucht**, 0 Versuche in diesem Bericht. Gruende: QEMU kennt den USB-Pfad (xHCI/VL805) nicht, die Tasten kommen dort aus dem Trap-Pump-Injektor (`kernel/src/host_env.rs:504@pump_devices`), A72-Barrieren fehlen; die Lage vom 2026-10-29 (11 Laeufe, 0 Haenger, auch ohne Serielle) bleibt. Eine QEMU-Messung, die M1 ersetzt: Anzahl der Wechsel pro Tastendruck zaehlen (Schritt in `tools/qemu-serial-on` mit Log), nur um die Folgerung in Abschnitt 2 zu pruefen, nicht um den Freeze zu finden.

## 5. Ausserhalb des Auftrags

- XSHOT.NAC hat keine Quelle im Repo (nur Ticket-Verweis auf `xshot.c` Zeile 1476 (Fremdquelle)); fuer die Karten-Luecke ("Programme ohne lokale Quelle") ist es der unbelegte Fall.
- `TASK/DOOM01.md` K5/K6 sind weiter offen; M3 entscheidet sie.
