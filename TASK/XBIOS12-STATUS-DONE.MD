# XBIOS12 — Quittung

Erledigt 2026-10-31. Ohne RTC startet die Uhr beim Baudatum des Kerns um
00:00:00 (`OS_DOSDATE << 16`, dieselbe Quelle wie das OSHEADER-Feld
`os_dosdate`), wie EmuTOS `DEFAULT_DATETIME`
(`references/emutos/bios/ikbd.h:95`, `references/emutos/bios/clock.c:1211`);
gilt fuer XBIOS `Gettime` und GEMDOS `Tgetdate`/`Tgettime`. Der Rueckfall auf
das FAT-Datum von `KERNEL8.IMG` ist entfernt. Danach laeuft die Uhr weiter
(EmuTOS-Pfad mit IKBD-Uhr). Boot beider Boards zeigt das Baudatum; zwei
Bauten mit gleichem `SOURCE_DATE_EPOCH` byte-gleich. Hosttests 1922/0/1.
Beleg `measurements/measurement-2026-10-31-xbios12/RUNINFO.TXT`. Bis
[RTC01](RTC01.md).
