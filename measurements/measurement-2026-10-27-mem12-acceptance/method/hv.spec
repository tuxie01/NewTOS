# MEM12 HypView (#Z, README.TXT as default document), armed up:HYP_VIEW: two Down-arrow presses in its window --
# each runs KeyboardOnOff twice (dl_routs.c:316-335, window.c:320,512): Ssystem(S_GETBVAL) = EINVFN, then
# Supexec(ChangeSysKeyconfig) writes conterm ($484).
at 600
wait 250
mark h-loaded
key 0x50
wait 200
mark h-key1
key 0x50
wait 200
mark h-key2
