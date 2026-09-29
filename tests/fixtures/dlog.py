#!/usr/bin/env python3
"""Synthetic dispatch-log: answers ls/show/cat from dlog.json beside it and logs each call."""
import json, sys
from pathlib import Path

root = Path(__file__).parent
with open(root / "dlog-calls", "a") as f:
    f.write(json.dumps(sys.argv[1:]) + "\n")
data = json.loads((root / "dlog.json").read_text())
args = sys.argv[1:]
if args[0] == "ls":
    if "ls_error" in data:
        sys.stderr.write(data["ls_error"])
        sys.exit(1)
    out = data["ls"]
elif args[0] == "show":
    out = data["show"].get(args[1])
    if out is None:
        sys.stderr.write("dlog: FileNotFoundError: no such dispatch\n")
        sys.exit(1)
elif args[0] == "cat":
    text = data["cat"].get(args[1])
    if text is None:
        sys.stderr.write("dlog: FileNotFoundError: no such blob\n")
        sys.exit(1)
    sys.stdout.write(text)
    sys.exit(0)
else:
    sys.exit(2)
sys.stdout.write(out if isinstance(out, str) else json.dumps(out, ensure_ascii=False, indent=2))
