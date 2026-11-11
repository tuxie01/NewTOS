# DESK44 — `.NTP` laeuft als GEM, soll aber Konsole mit Parametern sein

Status: PARKED (2026-11-11) | Priority: P3 | Reason: legacy, evidence unverified; receipt DESK44-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 15, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 4d16ace00 (feat(desk): U4 - the kernel lets go of desk, drift), named by body

Line 3 before the migration:
Status: **BEHOBEN — HW-Gegenprobe offen** (2026-09-26) · Prioritaet: **P3** · Typ: Belegter Widerspruch im Baum — aus DESK43 U3, Anwenderentscheidung 2026-09-26

Abhaengigkeiten: —

## Der Widerspruch

Der Baum sagt zweierlei ueber dieselbe Endung:

* `kernel/src/desk/ftype.rs:82` bildet `.NTP` auf `Flavor::Params` ab, und
  `main::default_flavor` bildet `Params` auf **`AppFlavor::Ttp`** ab — also auf
  ein **Konsolen**programm mit Parameterzeile.
* `kernel/src/desk/main.rs:1059` und `ftype.rs:13` nennen als Atari-Analogon
  *„`.TTP` / `.GTP`"*, und `isgraf` behandelt `.NTP` heute als **GEM**
  (`#Y`/`.GTP`).

`.TTP` und `.GTP` tragen in EmuTOS **verschiedene** `AF_ISCRYS`
(`references/emutos/desk/deskapp.c:181-188`) — es kann also nur eines von
beiden sein.

## Die Entscheidung (Anwender 2026-09-26)

**`.NTP` ist wie `.TTP`: Konsole, mit Parametern.** Das passt zum Namen der
ADR-018-Familie — `.NCP` Konsole, `.NTP` Konsole *mit Parametern*, genau
analog `.TOS`/`.TTP`.

**Damit ist das heutige GEM-Verhalten der Fehler**, nicht der Eintrag in
`default_flavor`.

## Auftrag

1. `isgraf` (`kernel/src/desk/main.rs`, = `desksupp.c:876` +
   `deskapp.c:181-188`) fuer `.NTP` auf **Konsole** stellen, mit Zitat.
2. Die Doc-Stellen angleichen, die `.GTP` als Analogon nennen — **alle**
   Kopien suchen ([[justification-is-copied-to-several-places]]):
   `ftype.rs:13`, `main.rs:1059`, `docs/MEMORY.md:1039` (ADR-018) und was der
   Sweep sonst findet.
3. `docs/MEMORY.md` ADR-018 traegt die Entscheidung mit Datum.

## Abnahme

Ein `.NTP` startet ueber den Konsolenarm (VT52-Schirm, kein AES-Fenster), mit
der Parameterzeile aus dem Tail-Dialog. Gegenprobe: `.NAP` bleibt GEM,
`.NCP` bleibt Konsole ohne Dialog.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten.

## Abschluss (2026-09-26)

`isgraf` gibt fuer `Flavor::Params` jetzt **`false`** zurueck, `App` allein
`true`. Beleg: `app_parse`s Fallthrough `case 'Y' → case 'G'` setzt
`AF_ISPARM|AF_ISCRYS`, `case 'P' → case 'F'` setzt **nur** `AF_ISPARM`
(`deskapp.c:310-322`, Tabelle `:181-188`); `isgraf` liest genau `AF_ISCRYS`
(`desksupp.c:876`).

**Kein zweiter Codeort noetig** — der Konsolenarm ist der regulaere Weg:
`pro_run` → `shel_write(SHW_EXEC, isgr=0)` → `sh_isgem = 0` → `sh_chgrf` →
`sh_toalpha`, gegen `gemshlib.c:513-523`.

Sweep nach `.GTP`-als-Analogon: **drei** Kopien, alle korrigiert
(`ftype.rs:13`, `main.rs:1059`, `docs/sdk/01-overview.md:47` — letztere sagte
zusaetzlich faelschlich „treated like `.NAP`"). Alle uebrigen `GTP`-Treffer
sind Aussagen **ueber EmuTOS** bzw. die klassische m68k-Familie und bleiben
richtig.

**Abnahme nur zur Haelfte einloesbar, und das ist eine alte Luecke, keine
neue:** `opn_appl` (der Tail-Dialog, `deskinf.c:1253-1271`) ist weiterhin nicht
gebaut. Der Doppelklick startet also mit **leerer** Parameterzeile — jetzt aber
auf dem Textschirm, wo er hingehoert. Gegenprobe im Code:
`inf::default_type_anodes` schrieb schon immer `#P *.NTP`, beide Seiten stimmen
jetzt ueberein.

**HW-Gegenprobe offen:** `.NTP` doppelklicken → `sh_isgem=0` im Serial und
Textschirm statt Desktop; `.NAP` bleibt GEM, `.NCP` bleibt Konsole ohne Dialog.

## Hardware-Runde 2026-09-28 — und was dabei sichtbar wurde

Anwender zu Punkt 4 der Testliste: *„Doppelklick geht ja nur im Desktop und
nicht im Textschirm, und da oeffnet sich kein Argument-Eingabefenster; der
Desktop blitzt kurz auf und das war's."*

**Der Kern der Rueckmeldung ist der fehlende Tail-Dialog** — genau die Haelfte
der Abnahme, die dieses Ticket als „nicht einloesbar" vermerkt hat, weil
`opn_appl` (`deskinf.c:1253-1271`) nicht gebaut ist. Ohne ihn startet ein `.NTP`
mit **leerer** Parameterzeile und ist damit von einem `.NCP` nicht zu
unterscheiden. **Eigenes Ticket: [DESK47](DESK47.md).**

Das kurze Aufblitzen selbst deckt sich mit der Messung aus
[DESK46](DESK46.md): der Konsolenschirm steht **25 ms**, Programmlauf
eingeschlossen. Mit blossem Auge ist daraus nicht zu entscheiden, ob der
Textschirm betreten wurde — fuer `.NCP` ist das per Screendump belegt
(`measurements/measurement-2026-09-27-desk46/screendumps/a3-3-console.png`), fuer `.NTP`
steht derselbe Nachweis aus und ist Teil von DESK47s Abnahme.
