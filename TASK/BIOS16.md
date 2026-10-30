# BIOS16 — Direkt geschriebene Vektorzelle in Seite 0 wird nicht uebernommen

Status: **OFFEN** (entschieden 2026-10-30) · Prioritaet: **P2** · Typ: Developer — Fund aus [BIOS07](BIOS07.md) (2026-10-29)

## Sachverhalt

- Seit BIOS07 sind die Vektorzellen `num <= 0x103` **eine** geteilte Zelle
  (`tos-api/src/bios/vectors.rs`, ADR-031 Regel 3); jede Seite 0 traegt eine
  Kopie. Schreibt ein Programm die Kopie direkt (Super-Zustand, z. B.
  `*(long *)0x400 = handler` fuer `etv_timer`), aendert das die Zelle nicht;
  nur die Seite-0-Warnung (BIOS15) kommt. `Setexc(n,-1)` liefert danach den
  alten Wert, auf EmuTOS den neuen (eine Vektortabelle, direkte Schreibung
  IST der Vektor). Dokumentiert in `docs/COMPATIBILITY.md` („`Setexc` and
  page 0“).
- Nebenbefunde BIOS07: `swv_vec` (`$46E`) startet auf EmuTOS mit `just_rts`
  (`references/emutos/bios/bios.c:363`), auf NewTOS mit 0; veraltete
  Kommentare `sdk/apps/apitest/apitest.c:516@Setexc` („vector shadow holds 64-bit
  values“) und `tos-api/src/bios/tick.rs:53@etv_timer` („shadows the vector“).

## Vorschlag (Lead)

Am Trap-Ende, wo `check_page0` die Kopie mit der Zelle vergleicht und warnt
(BIOS15), den neuen Wert in die geteilte Zelle **uebernehmen** (weiter warnen,
nie aufrufen — ADR-031 bleibt). Dann liefert `Setexc(n,-1)` wie EmuTOS den
direkt geschriebenen Wert. `swv_vec` mit `just_rts` vorbelegen; Kommentare
richtigstellen.

## Abnahme

- Sonde: direkte Schreibung, danach `Setexc(n,-1)` gleich dem Orakel; Warnung
  wie BIOS15; keine neue Abweichung; Hosttests gruen.

[Gemeinsame Regeln](README.md) gelten.

## Entscheidung (Anwender, 2026-10-30)

Wie vorgeschlagen: der direkt in eine Seite-0-Vektorzelle geschriebene Wert
wird am Ende des Systemaufrufs in dieselbe Zelle uebernommen, die `Setexc`
liest und schreibt (wie EmuTOS, `references/emutos/bios/bios.c:1070-1080`);
Warnung einmal nach ADR-031; nie aufrufen. Abnahme: Sonde beidseitig (MEASURED),
direkte Schreibung, danach liefert `Setexc(n,-1)` den geschriebenen Wert.

## Lokalisierung (Lead, 2026-10-30)

- Erkennung heute: `tos-api/src/bdos/process/hooks.rs` (`after_trap` →
  `check_page0`, vergleicht seit BIOS07 die Seite-0-Kopien `$008`–`$408` mit
  ihrer geteilten Zelle und warnt). Uebernahme in die Zelle:
  `tos-api/src/bios/vectors.rs` (`store_shared`, `setexc`).
- EmuTOS: `references/emutos/bios/bios.c:1070-1080` (eine Zelle, kein
  Schatten); `swv_vec` vorbelegt `references/emutos/bios/bios.c:363`.
- Wiedertest: BIOS15-Sonde `hkprobe` (Warnungen einmal je Prozess und Zelle),
  BIOS07-Sonde `sxprobe`, MEM12 (P0PAR), `etv_term`-Upcall
  (`kernel/src/arch/aarch64/exceptions.rs`, uebersprungen nur bei
  `JUST_RTS`/0). **Offene Frage vor dem Fix:** `etv_term` ist ein bedienter
  Haken (per `Setexc` gesetzt wird er beim Prozessende aufgerufen). Ein
  uebernommener Direktwert in derselben Zelle wuerde damit aufgerufen — die
  Entscheidung sagt „nie aufrufen“. Developer prueft, welche Zellen bedient
  sind, und meldet vor der Umsetzung, falls die Uebernahme einen bedienten
  Haken aufrufbar macht.

## Entscheidung (Anwender, 2026-10-30, nach dem Bericht zur offenen Frage)

- **Option B:** direkt geschriebene Werte am Ende des Systemaufrufs fuer alle
  Zellen `$008`–`$407` uebernehmen, die NewTOS nur speichert. `$408`
  (`etv_term`) bleibt: Warnung, `Setexc(0x102,-1)` liefert den per `Setexc`
  gesetzten Wert; als Restabweichung in `docs/COMPATIBILITY.md` (EmuTOS ruft
  ein direkt geschriebenes `etv_term`, `references/emutos/bdos/proc.c:607-608`).
- **Besitz: ja.** Uebernommene Werte bekommen einen Besitzer nach ADR-031
  Regel 3; beim `Pterm` wird der alte Wert zurueckgeschrieben (XBRA-Regel wie
  bei `Setexc`-Haken); `Ptermres` behaelt ihn.
- **Umfang:** `$008`–`$408` wie heute; `etv_xtra` (`$40C`) und Vektoren 0/1
  bleiben draussen, bis gemessen.
- Mit dabei: `swv_vec` mit `just_rts` vorbelegen, die zwei veralteten
  Kommentare.
- **Abnahme:** Sonde beidseitig (MEASURED): direkte Schreibung, `Setexc`-Abfrage
  liefert den geschriebenen Wert; nach `Pterm` auf NewTOS der alte Wert
  zurueck — dass das Zurueckschreiben vom Orakel abweicht, wird dokumentiert.
  Keine neue Abweichung in `bios-core-r1`, `super-r1`, `sxprobe`, `hkprobe`.

## Nachentscheidungen (Anwender, 2026-10-30)

- Bestaetigt: die Uebernahme wird fuer einen Prozess im Super-Zustand
  zusaetzlich **vor** jedem Systemaufruf geprueft (nicht nur am Ende).
- `swv_vec` und die Umbenennung `RESET_STUB_VA` → `RESET_ENTRY_VA`:
  [BIOS17](BIOS17.md). ADR-031-Nachtrag (Uebernahme = erlaubte Reaktion, nie
  Aufruf): Text beim Anwender. tosdiff-Folgeticket: [TOOL26](TOOL26.md).
