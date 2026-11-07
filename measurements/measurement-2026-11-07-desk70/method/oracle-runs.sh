#!/bin/bash
# oracle-runs.sh <image> <scratch out> <evidence dir> -- the EmuTOS runs of DESK70, one Hatari at a time; every frame in an AVI (--frameskips 0).
IMG=$1; O=$2; E=$3; M=$(cd "$(dirname "$0")" && pwd)
one() { t=$1; a=$2; shift 2
  "$M/hatari-run.sh" "$IMG" "$O/$t" "$M/$a" --frameskips 0 --avirecord --avi-vcodec png --png-level 1 --avi-file "$O/$t.avi" "$@" >/dev/null
  python3 "$M/avi-frames.py" "$O/$t.avi" > "$E/$t.frames.txt"
  mkdir -p "$E/$t"; cp "$O/$t"/shots-*.png "$E/$t/"; cp "$O/$t/actions.log" "$O/$t/disk.sha256" "$E/$t/"; /usr/bin/grep -v 'FIFO read error' "$O/$t/hatari.log" | cut -c1-200 > "$E/$t/hatari.log"; rm -f "$O/$t/disk.img"
  echo "$t done $(date +%H:%M:%S)"
}
one o-m0tos-r1 o-m0tos.act
one o-m0tos-r2 o-m0tos.act
one o-m0ttp o-m0ttp.act
one o-m1tos o-m1tos.act
one o-m2key o-m2key.act
