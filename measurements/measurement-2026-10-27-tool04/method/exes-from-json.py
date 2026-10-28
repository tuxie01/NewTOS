#!/usr/bin/env python3
"""TOOL04: list the test executables of a `cargo test --no-run --message-format=json` run.

Usage: exes-from-json.py <cargo.json>  > exes.tsv
Columns: kind  target-name  executable  package-dir  fresh
(fresh = cargo's flag: True = not rebuilt by this cargo call)
"""
import json
import sys
from urllib.parse import unquote

for line in open(sys.argv[1]):
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") != "compiler-artifact" or not m.get("executable"):
        continue
    if not m["profile"]["test"]:
        continue
    pid = m["package_id"]  # path+file:///abs/dir#name@ver
    pdir = unquote(pid.split("file://", 1)[1].split("#", 1)[0])
    print("\t".join([m["target"]["kind"][0], m["target"]["name"], m["executable"], pdir, str(m["fresh"])]))
