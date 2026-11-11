# MEM03 — Der Umgebungsblock eines Kindes ist in EL0 nicht abgebildet; `p_env` traegt eine physische Adresse

Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: 7d19c0559 | Missing: tester acceptance through cargo xtask accept

Migrated (owner batch 17, 2026-11-11, to IMPLEMENTED), evidence: commit 7d19c0559 (fix(libc): printf %f, 64-bit intptr_t and the spawn family under LP64), VERIFIED: 7d19c0559 (fix(libc): printf %f, 64-bit intptr_t and the spawn family u) names the id by body, its message names the ticket-title identifier p_env, touches code (40 files), legacy line: ERLEDIGT 2026-10-05 (mit ARCH03 Stufe 1 `c6280342`, Sonde ENVKID/MEM03 PASS in Q

Line 3 before the migration:
Status: **ERLEDIGT 2026-10-05** (mit ARCH03 Stufe 1 `c6280342`; Sonde ENVKID/MEM03 PASS in QEMU, siehe [ARCH03-1-Quittung](ARCH03-1-STATUS-DONE.MD) und ARCH03-2; Folge PORT09 `861ad038`; eigene Pi-Probe nicht belegt) · Prioritaet: **P2** · Typ: Kernfehler (Lader / Speicherabbildung) — gefunden in [PORT09](PORT09.md)

Abhaengigkeiten: — · **blockiert** [PORT09](PORT09.md) Schritt 2

## Sachverhalt (gemessen 2026-10-02)

`Pexec` aus EL0 mit Umgebung: das Kind liest `p_env` und faellt sofort.
`measurements/measurement-2026-10-02-port09/serial/envkid.log`:
`p_env=0x209b000`, erster Lesezugriff → `Data Abort (lower EL)`, ESR
`0x92000006` (Translation Fault Level 2), `FAR == p_env`; die Maschine kommt
nicht zurueck.

Quelltext (vom Lead nachgelesen):

- `tos-api/src/bdos/process/loader.rs:502-507` — `alloc_env_for` liefert
  `env_phys`; `:675` schreibt sie unveraendert nach `bp.p_env`.
- Der Block wird nie ueber `usermem::map` in den EL0-Raum des Kindes
  gelegt, anders als Malloc/Mxalloc (ADR-021; `usermem.rs` beschreibt genau
  diese Falle). `0x209b000` liegt ausserhalb des 32-MiB-EL0-Fensters.
- `EnvSpec::Inherit` (`env == NULL`, `loader.rs:1210-1225`) liest
  `(*run).p_env` als physische Adresse — muss beim Umstellen mitgehen.
- `selftest_pexec.rs:378-397` prueft den Block nur ueber den Kern-Alias und
  sieht den Fehler deshalb nicht.

## Wen es heute trifft

Jedes `Pexec` aus EL0 mit `EnvSpec::Explicit` oder `Inherit`, sobald das Kind
`p_env` liest. Heute **latent**:

- Desktop, AUTO und Bootlader starten mit `EnvSpec::None` → `p_env = 0`
  (`kernel/src/launch.rs:70-79`). TeraDesk selbst liest `_BasPag->p_env`
  (`sdk/apps/teradesk/environm.c:44,79`) und bekommt daher 0.
- Von TeraDesk gestartete Programme bekommen eine Pool-PA, lesen sie aber
  nicht: unsere libc wertet `p_env` nicht aus
  (`sdk/libc/newtos-arch/cmdargs.c:37-46`).

Sobald die libc `environ`/`ARGV=` auswertet (PORT09), faellt **jedes** von
TeraDesk gestartete libc-Programm beim Start — QED eingeschlossen.

## Arbeit

1. Umgebungsblock in den EL0-Raum des Kindes abbilden (Weg wie ADR-021 /
   `usermem::map`), `p_env` = Nutzer-VA.
2. `Inherit` auf die neue Darstellung umstellen (Quelle ist dann eine VA im
   Raum des Elternprozesses).
3. Lebensdauer/Freigabe mit dem Kind (`owned_by_child`) pruefen.
4. Selftest so aendern, dass er `p_env` aus EL0-Sicht liest.

Abnahme: `measurements/measurement-2026-10-02-port09/method/envpar.c` +
`envkid.c` — das Kind liest seine Umgebung ohne Abort und gibt `PATH=C:\`
und den `ARGV=`-Block aus; Basislinien gehalten.
