# ZVIEW04 — File > Quit does not quit zView while the browser window is open

Status: PARKED (2026-11-11) | Priority: P2 | Reason: legacy, evidence unverified; receipt ZVIEW04-STATUS-DONE.MD exists | Reopen: a per-ticket reading identifies the commit that carries the work and the owner confirms it

Migrated (owner batch 19, 2026-11-11, to PARKED), evidence: was IMPLEMENTED with a placeholder commit, NOT VERIFIED: no cited file of the ticket/receipt is touched by 6e7dc1c60 (fix(aes): MN_SELECTED carries the menu tree and pa), named by body

Line 3 before the migration:
Status: **OFFEN** · Prioritaet: **P2** · Typ: Defekt, Ursache unbekannt

## Sachverhalt

Anwender auf der echten Pi 4, Kernel `55eddb8f` (= HEAD `8435cd2`),
`ZVIEW.NAP` aus `sdk/zview/build-zview.sh`: Ist das Datei-Browserfenster von
zView offen, beendet *File > Quit* zView nicht. zView beendet sich erst, wenn
vorher das Browserfenster geschlossen wurde.

## Arbeit

Ursache mit Belegen klaeren, bevor gefixt wird: zViews Quit-Pfad lesen
(Menuehandler, WinDom `ApplExit`/`WindClose`/`WM_CLOSED`, Schliesslogik des
Katalog-/Browserfensters) und in QEMU mit AES-Trace und IKBD-Injektor
(`measurements/measurement-2026-10-07-zview02/method`) nachverfolgen, was beim
Quit mit offenem Browser passiert; gegen EmuTOS halten
(`references/emutos/aes`). Entscheiden: NewTOS-AES-Abweichung (dann 1:1 nach
EmuTOS fixen, Spec zuerst), Toolchain/LP64-Problem von zView/WinDom (dann
Overlay, Quellen unberuehrt) oder zViews eigenes Verhalten auch auf dem Atari
(dann berichten, nichts aendern).
