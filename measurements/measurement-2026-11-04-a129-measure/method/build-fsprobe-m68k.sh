#!/usr/bin/env bash
# build-fsprobe-m68k.sh <out.PRG> -- the same fsprobe.c for the m68k oracle, the
# recipe of the sbprobe-m68k target in tosdump/Makefile (M68K_CC, abi/m68k_trap.c).
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd); ROOT=$(cd "$HERE/../../.." && pwd)
. "$ROOT/tools/oracle/oracle.env"
"${ORACLE_CROSS_MINT}gcc" -O2 -Wall -Wextra -Werror -I"$ROOT/tosdump/core" "$HERE/fsprobe.c" "$ROOT/tosdump/abi/m68k_trap.c" -o "$1"
shasum -a 256 "$1"
