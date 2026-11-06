# V44 — Quittung DONE (2026-11-06)

Die Regression aus V33 kam von den achtwortigen Glyphenzeilen. `fits_narrow` waehlt je Glyphe einwortige Zeilen fuer Zellen bis 64 x 64, groessere behalten die acht Woerter. Fix `6e96e91c`, 5 Tests.
Abnahme `measurements/measurement-2026-11-06-v44-test/RUNINFO.TXT`: Text in QEMU schneller als vor V33 (plain ×0,76, bold ×0,78), Probe und Tabellen bytegleich, AES nur Adresswort, Desktop bytegleich. Offen: Faktor auf der echten Pi, einige Grenzkombinationen ungemessen (Ticket).
