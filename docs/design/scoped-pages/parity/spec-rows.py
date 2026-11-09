"""Print a computed-style dump (render-ref.sh --spec) one element per line, defaults left out.

usage: python3 -I spec-rows.py <dump.json> [key ...]     N=<count> sets the elements per key (default 3)
"""
import json, os, sys
path, *keys = sys.argv[1:]
d = json.load(open(path))["spec"]
LIMIT = int(os.environ.get("N", "3"))
KEEP = ["fontSize","fontWeight","lineHeight","color","padding","margin","gap","borderWidth","borderStyle","borderColor","borderRadius","background","underlay","boxShadow","outline","opacity","effectiveOpacity","width","height","display","position","inset","transform","backgroundImage","letterSpacing"]
def compact(st):
    out = {}
    for k in KEEP:
        if k not in st: continue
        v = st[k]
        if isinstance(v, list) and len(set(map(str, v))) == 1: v = v[0]
        if v in (0, "0px", "none", "normal", "static", 1) and k not in ("opacity","effectiveOpacity","fontWeight"): continue
        if k in ("opacity","effectiveOpacity") and v == 1: continue
        if k == "borderStyle" and v == "none": continue
        if k == "borderColor" and st.get("borderStyle") in ("none", ["none"]*4): continue
        out[k] = v
    return out
for k in keys or d.keys():
    items = d.get(k)
    if not items: print(f"{k}: -"); continue
    if isinstance(items, dict): print(k, items); continue
    for i, e in enumerate(items[:LIMIT]):
        r = e["rect"]
        rs = f"{r['x']},{r['y']} {r['w']}x{r['h']}" if r else "pseudo"
        print(f"{k}[{i}] <{e['tag']} .{e['cls']}> '{e['text'][:40]}' @ {rs} {json.dumps(compact(e['styles']), separators=(',',':'))}")
    if len(items) > LIMIT: print(f"  ... {len(items)} total")
