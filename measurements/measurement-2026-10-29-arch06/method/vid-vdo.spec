# TeraDesk (arm up:DESKTOP): Options menu -> Video options... dialog. 640x480, 16-px menu rows:
# Options title at x~260 y 9; "Video options..." is the 10th row of the drop-down.
at 600
park
move 260 9
click
wait 150
mark opts
move 300 171
wait 100
nop
mark hover
click
wait 400
# then: leave the dialog unchanged with Return (= OK, the default button); if an alert comes, Return again (its default [1] OK)
wait 200
key 0x1C
wait 300
mark ok
key 0x1C
wait 600
mark alertok
wait 600
nop
mark after
