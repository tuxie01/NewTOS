# SDK06 — `Getcookie()` Zeigerform: Wert vorzeichenerweitert wie die Regel

Status: PARKED (2026-11-11) | Priority: P3 | Reason: owner pre-start check 2026-10-31: no program reads a cookie with bit 31 set | Reopen: a program reads such a cookie

Migrated (owner batch 11, 2026-11-11), evidence: TASK/SDK06.md status line: GEPARKT 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm liest einen Cookie mit gesetztem Bit 31)

Line 3 before the migration:
Status: **GEPARKT** 2026-10-31 (Anwender, Vor-Start-Pruefung: kein Programm liest einen Cookie mit gesetztem Bit 31) · Prioritaet: **P3** · Typ: Developer (SDK) — Anwenderentscheidung 2026-10-28

## Sachverhalt

- Die Zeigerform von `Getcookie()` erweitert mit Nullen als Nebeneffekt:
  `sdk/libc/newtos-arch/getcookie.c:35@Getcookie` belegt `long v = 0` vor, der Kern
  schreibt 4 Byte (`tos-api/src/gemdos/ssystem.rs:421@put_u32`). Die Wertform
  erweitert mit Vorzeichen. Regel: `docs/COMPATIBILITY.md`, „LONG results in
  `x0`“. Kein Nutzer ist betroffen
  (`docs/analysis/abi-assumptions-check-2026-10-28.md` §3).

## Arbeit

1. Zeigerform an die Regel angleichen (Wert als 32-Bit-LONG lesen und
   vorzeichenerweitern), benannte Patch-Stelle im SDK.
2. Test im SDK-Hostlauf mit einem Wert mit Bit 31.

## Abnahme

- SDK-Hosttests gruen; doc-check 0 Fehler.
