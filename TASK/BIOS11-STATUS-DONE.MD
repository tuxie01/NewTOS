# BIOS11 — receipt DONE (2026-11-11)

Role: tester
Agent: agent-adb6e13c681799991
Developer: legacy, developer unknown
Self-acceptance: not applicable (ADR-037 point 12); a migrated legacy ticket with no developer recorded
Commit: a32921d5b
Result: PASS

Written by `cargo xtask accept` (ADR-037 point 11). Developer: legacy, developer unknown. Line 3 before: `Status: IMPLEMENTED (2026-11-11) | Priority: P2 | Commit: a32921d5b | Missing: tester acceptance through cargo xtask accept`.

## Note

bios-full run: Bconin EQUAL 16 case-runs but no tosdump case sets conterm (grep tosdump/cases empty), so bit 3 is covered by probe K16 0x02300042 = EmuTOS (measurement-2026-10-29-xbios01 result 3) and host test bconin_keeps_the_shift_state_of_the_key_press_with_conterm_bit_3 ok; full test 2454/0/7; real-Pi4 check open
