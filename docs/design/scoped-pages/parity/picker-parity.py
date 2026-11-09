"""Build spec/picker-parity.json: per state, the crop, ignores, optional ignores and wash samples.

usage: python3 -I picker-parity.py <dumps-dir> <out.json>
<dumps-dir> holds <NN>-<state>-light.json for every state below, each written by
render-ref.sh picker c light <state> <png> --spec selectors/picker.json <dumps-dir>/<NN>-<state>-light.json
"""
import json, os, re, sys
J = sys.argv[1]; OUT = sys.argv[2]
STATES = [("01", "empty"), ("02", "selected-3"), ("03", "all-off"), ("04", "no-agent"), ("05", "no-match"),
          ("06", "already-here"), ("07", "folder-chip"), ("08", "docs-site"), ("09", "docs-site-carry"),
          ("10", "agent"), ("11", "pill-tooltip")]
DIALOG = [56.5, 45.5, 1067, 637]          # inside the dialog's fractional edge (56.4,45.4 1067.2x637.2)
WASH = {"light": "#f2f2f2", "dark": "#080808"}
WASH_SIDEBAR = {"light": "#ededed", "dark": "#111111"}
r2 = lambda v: round(v, 2)
def rect(e, pad=0):
    r = e["rect"]; return [r2(r["x"] - pad), r2(r["y"] - pad), r2(r["w"] + 2 * pad), r2(r["h"] + 2 * pad)]
out = {"window": {"w": 1180, "h": 728, "scale": 2}, "units": "logical px, window top-left; pixdiff --crop/--ignore",
       "dialog": [56.4, 45.4, 1067.2, 637.2], "states": {}}
for n, name in STATES:
    sp = json.load(open(os.path.join(J, f"{n}-{name}-light.json")))["spec"]
    st = {"scenario": name, "png": {t: f"references/picker/{n}-{name}-{t}.png" for t in ("light", "dark")},
          "crops": [DIALOG], "ignore": [], "optional_ignore": {"dashed_pill": [], "check_glyph": []},
          "wash": [{"rect": [1160, 400, 16, 280], "expect": WASH}, {"rect": [1150, 47, 26, 6], "expect": WASH},
                   {"rect": [0, 300, 6, 100], "expect": WASH_SIDEBAR, "note": "sidebar under the scrim; native sidebar bg"}],
          "shadow_band": [232, 683, 892, 40]}
    inp = sp["searchInput"][0]
    if "__ref-focus" in inp["cls"]:
        x = (sp["caret"][0]["rect"]["x"] + sp["caret"][0]["rect"]["w"]) if sp.get("caret") else inp["rect"]["x"] + 29
        st["ignore"].append({"rect": [r2(x - 1.5), r2(inp["rect"]["y"] + 4), 4, 20], "why": "text caret (blinks natively; hidden in the reference)"})
    tips = sp.get("tooltip") or []
    for t in tips:
        r = t["rect"]
        if r["y"] + r["h"] > 682.6 or r["x"] < 56.4 or r["x"] + r["w"] > 1123.6:
            st["crops"].append([r2(r["x"] - 5), r2(r["y"]), r2(r["w"] + 10), r2(r["h"] + 10)])
        st["tooltip"] = {"text": t["text"], "rect": rect(t)}
    if name == "already-here":
        lst = sp["list"][0]["rect"]
        st["ignore"].append({"rect": [r2(lst["x"] + lst["w"] - 14), r2(lst["y"] + 1), 13, r2(lst["h"] - 2)],
                             "why": "list scrollbar (GPUI Kit shows it while the pointer is over the list)"})
    for p in (sp.get("dirPill") or []) + (sp.get("allPill") or []):
        if "is-on" not in p["cls"].split():
            st["optional_ignore"]["dashed_pill"].append(rect(p))
    bottom = sp["list"][0]["rect"]["y"] + sp["list"][0]["rect"]["h"] if sp.get("list") else 0
    for c in sp.get("checkbox") or []:
        r = c["rect"]
        if c["styles"]["background"] == "#171717" and r["y"] + r["h"] <= bottom:  # checked, in view
            st["optional_ignore"]["check_glyph"].append([r2(r["x"] + 2), r2(r["y"] + 2), 12, 12])
    out["states"][f"{n}-{name}"] = st
# One rect per line keeps the file under the repository's 5,000-token limit.
text = json.dumps(out, indent=1)
text = re.sub(r"\[[-\d.,\s]+\]", lambda m: json.dumps(json.loads(m.group(0))), text)
open(OUT, "w").write(text)
for k, v in out["states"].items():
    print(k, "crops", v["crops"], "ignore", [i["rect"] for i in v["ignore"]], "dashed", len(v["optional_ignore"]["dashed_pill"]), "checks", len(v["optional_ignore"]["check_glyph"]), v.get("tooltip", {}).get("text", ""))
