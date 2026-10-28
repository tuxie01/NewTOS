#!/usr/bin/env python3
"""TOOL04: wall time of one command (stdout/stderr discarded). Usage: t.py <label> <cmd...>"""
import subprocess, sys, time
t0 = time.monotonic()
rc = subprocess.run(sys.argv[2:], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode
print(f"{sys.argv[1]}\t{time.monotonic() - t0:.3f}\trc={rc}", flush=True)
