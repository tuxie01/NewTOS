# KEYOWNER — Tasten gehen an den Maus-/Tastatur-Eigentuemer (EmuTOS `gl_mowner`); Ssystem 2020 Eingabemodus

Status: PARKED (2026-11-11) | Priority: P1 | Reason: legacy, evidence unverified; receipt KEYOWNER-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 20, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: f642c797f (fix(aes): keys go to the mouse owner like EmuTOS, Ssystem 20) added the receipt TASK/KEYOWNER-STATUS-DONE.MD itself, the report of this research or measurement ticket (498 files), bookkeeping or bulk commit, not accepted as evidence

Line 3 before the migration:
Status: **ERLEDIGT (QEMU), 2026-10-08** — [Quittung](KEYOWNER-STATUS-DONE.MD) · Prioritaet: **P1** (sichtbar im Demo-Stand: Esc/Return/Pfeile im SYSINFO02-Aufklappfeld) · Typ: EmuTOS-Abweichung (Kernel-AES) + kleine NewTOS-Erweiterung (Ssystem)

Abhaengigkeiten: [SYSINFO02](SYSINFO02.md) (offener Punkt „Esc",
[Beleg](../measurements/measurement-2026-10-08-sysinfo2/RUNINFO.TXT), Abschnitt
„Esc", `diag-*`), INPUT01 (`input=aes`, `kernel/src/input_gate.rs`),
[`docs/api/aes/event.md`](../docs/api/aes/event.md),
[`docs/api/gemdos/154-ssystem.md`](../docs/api/gemdos/154-ssystem.md).
Grundstand: Tag `ofam-2026`; darf die Tastatureingabe in Desktop und Programmen
nicht destabilisieren (Atari-Treffen am Folgetag).

## Befund (aus SYSINFO02)

Tasten gehen an den, der gerade wartet: ein globaler Tastenring, „wer zuerst
pollt, gewinnt" (`tos-api/src/aes/event.rs` `evnt_multi`, `MU_KEYBD`-Arm). Waehrend
SYSACC sein Aufklappfeld verfolgt (`wind_update(BEG_MCTRL)`, wartet in
`evnt_multi`), laeuft EmuDESK (`kernel/src/desk/main.rs`, `MU_KEYBD`) und nimmt
Esc/Return/Pfeile weg.

EmuTOS: `kchange` → `post_keybd(gl_mowner->p_cda, ch)`
(`references/emutos/aes/geminput.c:288-307`), `gl_mowner` = „current
mouse/keybd owner" (`:52`); Eigentuemerwechsel `w_setactive`
(`gemwmlib.c:322-334`, `gemctrl.c:439-442`), `fm_own` (`gemfmlib.c:82-107`),
`chkown` (`geminput.c:310-335`), `mchange` (`:388-389`).

## Auftrag

1. **Spec zuerst**: `docs/api/aes/event.md` — Abschnitt „Wer bekommt eine
   Taste" (EmuTOS-Regel mit Datei:Zeile, beobachtbarer Vertrag, benannte
   NewTOS-Abweichungen).
2. **1:1-Regel umsetzen**: normal = Eigentuemer des obersten Fensters (ohne
   Fenster: Eigentuemer des Desktop-Fensters = Anwendungskette); waehrend
   `BEG_MCTRL` die steuernde Anwendung; Accessory = eigener Eigentuemer,
   Anwendungskette = ein Eigentuemer.
3. **Ssystem 2020** = Eingabemodus (0 = Maus bewegt sich auch waehrend ein
   Programm rechnet / Timer-Pumpe, 1 = `input=aes`); Spec zuerst, Host-Test; der
   bestehende Test, der 2020 als „frei" prueft, rueckt auf die naechste freie
   Nummer (2021). SYSACC zeigt den Wert in der Spalte „Running now" fuer
   „Mouse while busy".

## Pruefung

* Host-Tests rot/gruen: Taste waehrend Accessory-`BEG_MCTRL` geht an das
  Accessory; normales Tippen an den Eigentuemer des obersten Fensters;
  Accessory-Dialog (`form_do`/`form_alert`); Desktop-Kuerzel.
* QEMU raspi4b (`-append serial=on`), `memprot=on`, IKBD-Injektor: SYSINFO02-
  Aufklappfeld — Esc bricht ab, Return/Pfeile wirken. Regression aller
  Tastaturpfade: EmuDESK-Kuerzel (Alt-C u. a., desk3a-Lauf), Ctrl-I-Info-Dialog
  tippen, Umbenennen; TeraDesk-Tasten; QED tippen+speichern (qedw/qedo aus
  `measurements/measurement-2026-10-08-ofam-freeze/method`); LDGDEMO tippen;
  zView Esc (Vollbild); Doom-Tasten ueber kbdvec; EmuCON (Cconin); APITEST.
  Screendumps byte-gleich zu HEAD ausser beabsichtigt.
* Statisch: Host-Tests (Basis 1261/0/1), Release-Builds beider Boards (64/94),
  clippy (76/125), Drift (31), SDK-Host-Tests (19/0/0).

## Paket (nicht kopiert)

Release-rpi4-Kernel + `SYSACC.NAC` mit sha256 + deutsche `LIESMICH.TXT` unter
`measurements/measurement-2026-10-08-keyowner/bin/`. Laufverzeichnis
`measurements/measurement-2026-10-08-keyowner/` (RUNINFO.TXT, method/, serial/,
screendumps/, checks/). Quittung `TASK/KEYOWNER-STATUS-DONE.MD`.

## Arbeitsregel

[Gemeinsame Regeln](README.md) gelten. Keine Stubs; drei Versuche je Fehler;
bei unklarer Semantik oder einer nicht behebbaren Tastatur-Regression STOP und
melden.
