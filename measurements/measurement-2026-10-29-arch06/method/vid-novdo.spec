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
# then: tick the Overscan box (enabled in the ST branch) and leave with Return (= OK, the default button)
wait 200
move 201 281
click
wait 150
mark ovs
key 0x1C
wait 400
mark ok
wait 400
nop
mark after
