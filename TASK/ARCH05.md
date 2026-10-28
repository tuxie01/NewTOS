# ARCH05 — ADR: welche Maschine meldet NewTOS (`_MCH`) und was folgt daraus

Status: **ENTWURF LIEGT VOR** 2026-10-28 (`docs/analysis/machine-identity-adr-proposal-2026-10-28.md`), Entscheidungen beim Anwender · Prioritaet: **P2** · Typ: Researcher (ADR-Vorschlag), Entscheidung Anwender — Anwenderentscheidung 2026-10-28, aus [XBIOS10](XBIOS10.md)

## Sachverhalt

- XBIOS-SYS-003 (`NVMaccess` nicht verdrahtet, [XBIOS10](XBIOS10.md)):
  Anwender 2026-10-28 — die NVRAM-Frage folgt daraus, welche Maschine NewTOS
  meldet.
- Geprueft (Lead 2026-10-28): ADR-012 (`docs/MEMORY.md`) haelt nur fest, dass
  NewTOS `_MCH` als eigene Maschinenkennung behaelt. Der Wert `0x0005_0000`
  ist im Code als vorlaeufig markiert
  (`tos-api/src/cookie.rs:223@seed_defaults`: „Values are provisional
  bring-up policy“). Kein ADR leitet daraus maschinenabhaengiges Verhalten ab
  (NVRAM, weitere XBIOS-Funktionen, die EmuTOS je Maschine schaltet).
- Referenzprofil der Spec ist EmuTOS make-512 als Falcon030/TOS4
  (`docs/api/_emutos-config.md`).

## Arbeit

Anwender 2026-10-28: Ausgangslage ist ADR-012 (`_MCH` = eigener
NewTOS-Wert); `0x0005_0000` (laut Anwender die ARAnyM-Kennung) ist die
vorlaeufige Abweichung davon. Kein eigenes Ticket fuer die
Maschinenidentitaet, sie gehoert hierher.

1. **Bestand:** alle Kennungs-Cookies, die NewTOS heute meldet (`_MCH`,
   `_CPU`, `_FPU`, `_VDO`, `_SND`, `_FDC`, `_AKP`, `_IDT`, `NTOS`, `EdDI`, …),
   je mit Wert und `file:line`; daneben, was die Orakel-Profile (`tt`,
   `falcontc`, `disk`) melden (gemessen, nicht aus der Quelle).
2. **Beleg:** welche portierten Programme (`sdk/apps/`, `sdk/overlays/`,
   Portliste) welchen Cookie lesen und was sie je Wert tun (`file:line`).
   Besonders numerische Vergleiche (z. B. `mch >= 2` → TT/Falcon-Hardwarepfade):
   ein eigener Wert darf sie nicht ausloesen. Dazu die Stellen in EmuTOS, die
   Verhalten am Maschinentyp festmachen (NVRAM, RTC, DSP, Videomodi), soweit
   fuer NewTOS beobachtbar.
3. Zeigt der Beleg, dass ein eigener Wert unsicher ist: ADR-012
   **ausdruecklich** ueberarbeiten (Nachtrag mit Begruendung), nie still.
4. **Kennungs-Cookie fuer NewTOS:** Name gegen das bekannte Cookie-Register
   pruefen (Quelle nennen); heute gesetzt ist `NTOS`
   (`tos-api/src/cookie.rs:221@seed_defaults`) — Kollision ja/nein.
5. Cookie-Jar-Faelle gegen das Orakel werden je Profil dokumentierte
   ACCEPTED-Abweichungen (Falltabelle bzw. `docs/COMPATIBILITY.md`, Form im
   Vorschlag); danach folgt XBIOS10 (NVRAM) aus der Entscheidung.
6. ADR-Vorschlag mit Optionen und Folgen; Entscheidung beim Anwender.

## Abnahme

- ADR-Vorschlag mit Quellen; vom Anwender entschieden und als ADR bzw.
  Nachtrag zu ADR-012 in `docs/MEMORY.md` eingetragen (Lead).
- doc-check der geaenderten Doku 0 Fehler.
