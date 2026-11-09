#!/bin/bash
# Render one prototype state in reference mode: the app's window geometry (1180x728 logical at
# 2x, like a Retina gpui MCP screenshot) and the app's color tokens. The same arguments give a
# byte-identical PNG.
#
# usage: render-ref.sh <area> <variant> <theme> <scenario> <out.png> [options]
#   area      library | scoped-pages | picker | progress (renders ../prototypes/<area>.html)
#   variant   id, letter or label: library popover, scoped-pages menu, picker c, progress inline;
#             "-" = the page's first variant
#   theme     light | dark
#   scenario  a name registered in scenarios/<area>.js; "-" or default = as loaded
# options
#   --spec <selectors.json> <out.json>  selectors: ["css", ...] or {"key": "css"}; writes the
#                                       computed-style spec (rects in logical window px)
#   --filters segmented|menu            Library filter style (the page starts on Segmented)
#   --timeout <ms>                      wait for the page's report (default 20000)
# Needs Google Chrome (CHROME overrides its path), node 18+ (NODE) and python3; no packages.
# The bundle is staged in a temporary directory with reference.js and scenarios/<area>.js from
# this folder, so editing either takes effect without touching the bundle.
# Exit 1 when the page reports an error (unknown variant or scenario, a throw, console.error);
# the PNG is still written and shows a red error banner. Exit 2 on bad usage.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
NODE="${NODE:-node}"
W=1180 H=728 SCALE=2

[[ $# -ge 5 ]] || { sed -n '2,22p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }
AREA="$1" VARIANT="$2" THEME="$3" SCENARIO="$4" OUT="$5"
shift 5
SPEC_IN="" SPEC_OUT="" FILTERS="" TIMEOUT=20000
while [[ $# -gt 0 ]]; do
  case "$1" in
    --spec) SPEC_IN="$2" SPEC_OUT="$3"; shift 3 ;;
    --filters) FILTERS="$2"; shift 2 ;;
    --timeout) TIMEOUT="$2"; shift 2 ;;
    *) echo "render-ref: unknown option $1" >&2; exit 2 ;;
  esac
done
[[ "$THEME" == light || "$THEME" == dark ]] || { echo "render-ref: theme must be light or dark" >&2; exit 2; }
[[ "$VARIANT" == - ]] && VARIANT=""
[[ "$SCENARIO" == - || "$SCENARIO" == default ]] && SCENARIO=""
BUNDLE="$HERE/../prototypes/$AREA.html"
[[ -f "$BUNDLE" ]] || { echo "render-ref: no prototype $BUNDLE" >&2; exit 2; }

WORK="$(mktemp -d "${TMPDIR:-/tmp}/sbref.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT
mkdir "$WORK/scenarios"
cp "$HERE/reference.js" "$WORK/reference.js"
[[ ! -f "$HERE/scenarios/$AREA.js" ]] || cp "$HERE/scenarios/$AREA.js" "$WORK/scenarios/$AREA.js"

# Stage the bundle with its inlined reference mode swapped for this folder's reference.js, and
# build the reference-mode URL.
URL="$(python3 -I - "$BUNDLE" "$WORK/$AREA.html" "$VARIANT" "$THEME" "$SCENARIO" "$FILTERS" "$SPEC_IN" <<'PY'
import json, sys
from pathlib import Path
from urllib.parse import urlencode, quote
bundle, staged, variant, theme, scenario, filters, spec_in = sys.argv[1:8]
html = Path(bundle).read_text(encoding="utf-8")
start = html.find("<script>/* kit/reference.js */")
if start < 0:
    sys.exit(f"render-ref: {bundle} holds no inlined kit/reference.js")
end = html.index("</script>", start) + len("</script>")
Path(staged).write_text(html[:start] + '<script src="reference.js"></script>' + html[end:], encoding="utf-8")
q = {"ref": "1", "theme": theme}
for k, v in (("variant", variant), ("filters", filters), ("scenario", scenario)):
    if v:
        q[k] = v
if spec_in:
    q["spec"] = json.dumps(json.loads(Path(spec_in).read_text()), separators=(",", ":"))
print("file://" + quote(staged) + "?" + urlencode(q, quote_via=quote))
PY
)"

"$NODE" "$HERE/ref-chrome.mjs" "$URL" "$WORK/shot.png" "$W" "$H" "$SCALE" "$THEME" "$TIMEOUT" >"$WORK/result.json"
STATUS=0
python3 -I - "$WORK/result.json" "$WORK/shot.png" "$SPEC_OUT" "$W" "$H" "$SCALE" <<'PY' || STATUS=1
import json, struct, sys
from pathlib import Path
res, png, spec_out = sys.argv[1:4]
w, h, scale = (int(v) for v in sys.argv[4:7])
r = json.loads(Path(res).read_text())
vp = r["viewport"]
if (vp["w"], vp["h"], vp["dpr"]) != (w, h, scale):
    r["errors"].append(f"viewport {vp['w']}x{vp['h']}@{vp['dpr']}x, expected {w}x{h}@{scale}x")
pw, ph = struct.unpack(">II", Path(png).read_bytes()[16:24])
if (pw, ph) != (w * scale, h * scale):
    r["errors"].append(f"screenshot {pw}x{ph}, expected {w * scale}x{h * scale}")
print(f"page     {r['page']} variant={r['variant']} filters={r['filters']} theme={r['theme']} "
      f"scenario={r['scenario']} clock={r['clock']}ms pending-timers={r['timersPending']}")
if spec_out:
    meta = {k: r[k] for k in ("page", "variant", "filters", "theme", "scenario")}
    meta.update(window={"w": w, "h": h, "scale": scale}, units="logical px from the window's top-left")
    Path(spec_out).write_text(json.dumps({"meta": meta, "spec": r.get("spec") or {}}, indent=1) + "\n")
    print(f"spec     {spec_out}")
for e in r["errors"]:
    print(f"ERROR    {e}", file=sys.stderr)
sys.exit(1 if r["errors"] else 0)
PY
mkdir -p "$(dirname "$OUT")"
mv -f "$WORK/shot.png" "$OUT"
echo "png      $OUT ($((W * SCALE))x$((H * SCALE)) px = ${W}x${H} logical at ${SCALE}x)"
exit "$STATUS"
