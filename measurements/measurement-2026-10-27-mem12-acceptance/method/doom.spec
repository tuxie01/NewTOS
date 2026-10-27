# MEM12 DOOM (#Z), armed launch:DOOM (as measurement-2026-10-04-keys02-doom): DOOM installs its kbdvec hook in
# Supexec(kbd_install); F10 opens the quit prompt, 'y' answers it -- both only through the hook.
at 3000
key 0x44
wait 300
mark d-f10
key 0x15
wait 400
mark d-y
