#!/bin/sh
# timeless.sh <file> -- a wfprobe log or serial line without its times: clock
# stamps (h:mm:ss.mmm), every decimal number (the SD report's waits and
# maxima, ms) and the latency histogram counts (" time   <1:N <4:N ...").
sed -E -e 's/[0-9]+:[0-9]{2}:[0-9]{2}\.[0-9]+/T/g' \
       -e 's/[0-9]+\.[0-9]+/X/g' \
       -e '/^ *time +</s/:[0-9]+/:N/g' "$1"
