# TOOL04 profile run: summarise `cargo ... --unit-graph -Z unstable-options` JSON
# (stdin) as: count, workspace package, platform, target kind, mode, profile name,
# split_debuginfo. Workspace (path) packages only.
import json, sys, collections
g = json.load(sys.stdin)
c = collections.Counter()
for u in g["units"]:
    pk = u["pkg_id"]
    if not pk.startswith("path+"):
        continue
    name = pk.split("#")[0].rstrip("/").split("/")[-1]
    p = u["profile"]
    c[(name, u["platform"] or "host", "+".join(u["target"]["kind"]), u["mode"],
       p["name"], str(p.get("split_debuginfo")))] += 1
for k, v in sorted(c.items()):
    print(v, *k, sep="\t")
