# SDK05 — cflib `getcookie`: Cookie-Name per Byte-Union, auf Little-Endian verdreht

Status: **ERLEDIGT** 2026-10-29 ([Quittung](SDK05-STATUS-DONE.MD)); QED-Anzeige unveraendert (siehe Quittung) · Prioritaet: **P1** · Typ: Developer (SDK) — Anwenderentscheidung 2026-10-28, aus `docs/analysis/abi-assumptions-check-2026-10-28.md` §3

## Sachverhalt

- `sdk/cflib/cflib-21/cookie.c:33-49` baut den Cookie-Namen ueber eine Union
  aus `char[4]` und `long`. Auf Little-Endian AArch64 ergibt das den
  byteverdrehten Namen (`_IDT` → `0x5444495F` statt `0x5F494454`); die
  Kernel-Konstanten sind `from_be_bytes` (`tos-api/src/cookie.rs:27@MCH`).
- Folge (abgeleitet, nicht gemessen): `getcookie("_IDT")` aus QED findet
  nichts, QED nimmt den Ersatzwert `0x112E` statt `0x112F` — fuer den Anwender
  sichtbar (Datumsformat).

## Arbeit

1. **Sonde zuerst:** in `apitest` die cflib-Union woertlich mit einem Namen,
   der im Jar steht (`NTOS`, `_IDT`), aufrufen; erwartet heute „nicht
   gefunden“. Dazu QED: welchen `_IDT`-Wert nutzt es (Beleg aus dem Lauf).
2. Fix als benannte Patch-Stelle im cflib-Overlay (Upstream unveraendert),
   danach dieselbe Sonde: gefunden, Wert gleich `Getcookie`.
3. Erhebung derselben Klasse in allen Ports laeuft unter [SDK04](SDK04.md).

## Abnahme

- Sonde vorher/nachher im RUNINFO; QED zeigt `_IDT` = `0x112F`.
- SDK-Hosttests gruen; doc-check 0 Fehler.

## Nachtrag 2026-10-29 (Erhebung Byte-Reihenfolge)

- Bestaetigt (`docs/analysis/byte-order-survey-2026-10-29.md`, Klasse A):
  `cookie.o` mit `getcookie` ist in `sdk/cflib/cflib.a` gebaut, kein Ersatz im
  Overlay. Zusaetzlich fuellt die Union nur 4 der 8 Byte der LP64-`long`; die
  oberen 4 bleiben unbestimmt. Betroffen alle cflib-Programme: QED `_IDT`
  (sichtbar), `DHST`, cflib `appinit`.
