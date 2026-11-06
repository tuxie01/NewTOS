# PORT12 — XGDOS portieren: Speedo-Fonts und PDF-Druck

Status: **OFFEN** (2026-11-06) · Prioritaet: **P2** · Typ: Port, Erweiterung; Anwender 2026-11-06: frueher einplanen, vor 1.0, nach [HW08](HW08.md)

## Warum

Erste Druckfunktion (PDF) und Outline-Schriften fuer QED, HighWire, cflib und die GDOS-Tests von Fbench. EmuTOS hat kein GDOS, also ist das eine Erweiterung ueber das Original hinaus; die Aenderung der Roadmap ist die Entscheidung des Anwenders.

## Quelle

XGDOS 1.00, github.com/FrankBRetro/xgdos, Commit `f6c5aeb7`, GPL-2.0-only, Francis C Bullen. Rund 420 KB C, m68k nur in atari.S, traps.S, crt0.S (Upstream, Ordner src); Host-Build vorhanden (`make host-test`). Bisher nur README und Teile der Quellen ueber GitHub gelesen, nichts gebaut. Als eigenes Programm neben dem GPLv2+-Kernel; Upstream bleibt unveraendert, Patches in `sdk/overlays/`.

## Vorwissen

- **Einklinken:** XGDOS setzt `Setexc(34, xg_trap2)` (Upstream main.c, Zeile 221). Bei uns dekodiert der Kernel den VDI-Aufruf selbst (`kernel/src/arch/aarch64/exceptions.rs:564@GEM_SEL_ADDR_QUERY`); ob ein gesetzter Vektor etwas umleiten kann, ist ungeprueft.
- **Empfaenger steht:** Die Fontkette aus [V33](V33.md) (`docs/api/vdi/attributes/_text.md`, D1 `CONTRL[12..15]`, D3 Fonthead LP64). Neu gelinkte Binaries noetig.
- **Adressraum:** ADR-028 gemeinsamer Adressraum, Zeiger bleiben gueltig; Rechte je Prozess, also Ausfuehr- und Schreibrechte auf den XGDOS-Speicher klaeren. XGDOS legt Interna als `u32` ab, gilt nur unter 4 GiB.
- **GDOS-Sonde:** `vq_gdos` antwortet "kein GDOS" (`docs/COMPATIBILITY.md`). Meldet es `_FSM`, schalten QED, cflib, TeraDesk und HighWire sofort bei uns ungetestete Wege ein.
- **Kein Orakel:** EmuTOS hat keine Outline-Fonts. Referenz waere XGDOS selbst auf Hatari; das Weiterreichen an das VDI bleibt am Orakel messbar.

## Stufen

1. **Machbarkeit, ohne Kernelarbeit:** Upstream spd.c, raster.c, text.c, pdf.c mit unserem SDK fuer AArch64 bauen, gegen die Host-Tests laufen lassen, LP64-Probleme auflisten. Ergebnis: Liste, Aufwand, Risiken.
2. **ADR-Vorschlag an den Anwender:** Kernel leitet VDI-Aufrufe an einen eingetragenen EL0-Handler weiter (Empfehlung des Leads, ungeprueft) oder gemlib-Overlay. Bedingungen: ohne GDOS kein messbarer Mehraufwand (`txbench`), kein `todo!`, Rule 2. **Kein Kernelcode vor der Freigabe.**
3. **Port** nach der Entscheidung, Bildschirmtext zuerst, dann PDF.
4. **Apps:** QED-Druck, HighWire, Fbench GDOS-Tests auf neu gelinkten Binaries.

Abnahme je Stufe im Ticket; Pi im Testszenario.

[Gemeinsame Regeln](README.md) gelten.
