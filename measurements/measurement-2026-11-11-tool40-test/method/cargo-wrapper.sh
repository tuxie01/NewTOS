#!/bin/sh
echo "$(date +%s) cargo $*" >> /private/tmp/claude-501/-Volumes-Daten-Development-PiRusTOS/dd3b654c-65c3-4cb0-ad34-cf754ac4504a/scratchpad/tool40-test/cargo.log
exec /Users/ingouhlemann/.cargo/bin/cargo "$@"
