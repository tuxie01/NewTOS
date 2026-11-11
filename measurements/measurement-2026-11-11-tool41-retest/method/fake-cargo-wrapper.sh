#!/bin/sh
echo "cargo $*" >> "$CARGO_LOG"
exec /Users/ingouhlemann/.cargo/bin/cargo "$@"
