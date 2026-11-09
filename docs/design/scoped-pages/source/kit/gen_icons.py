"""Generate the generated block of the prototype kit's icons from the retained Hugeicons and agent logos.

usage: python3 -I gen_icons.py <repo-root> docs/design/scoped-pages/source/kit/icons.js.parts/2-generated.js
"""
import json, re, sys
from pathlib import Path

repo, out = Path(sys.argv[1]), Path(sys.argv[2])

HUGE = {
    "search": "Search01Icon", "plus": "PlusSignIcon", "delete": "Delete02Icon",
    "check": "Tick02Icon", "close": "Cancel01Icon", "alert-circle": "AlertCircleIcon",
    "upload-circle": "UploadCircle01Icon", "layers": "Layers01Icon", "folder": "Folder01Icon",
    "code-folder": "CodeFolderIcon", "square-terminal": "SquareTerminalIcon",
    "loading": "Loading03Icon", "refresh-03": "Refresh03Icon", "refresh-04": "Refresh04Icon",
    "check-circle": "CheckmarkCircle02Icon", "arrow-down": "ArrowDown01Icon",
    "chevron-right": "ArrowRight01Icon", "list-view": "ListViewIcon", "grid-view": "GridViewIcon",
    "libraries": "LibrariesIcon", "compass": "Compass01Icon", "settings": "Settings01Icon",
    "import": "ImportIcon", "unplug": "UnplugIcon", "book": "Book02Icon", "code": "SourceCodeIcon",
    "robot": "Robot01Icon", "pencil-edit": "PencilEdit02Icon", "git-branch": "GitBranchIcon",
}
# The fixture's agents (Universal's members and the workspace directories' first readers): the
# full-color logo where the repo has one, as the app's avatars use, else the monochrome one.
LOGOS = {
    "claude-code": "claude-code-color", "cline": "cline", "codex": "codex-color", "cursor": "cursor",
    "fx": "fx", "kimi-code-cli": "kimi-code-cli-color", "pi": "pi", "warp": "warp", "windsurf": "windsurf",
}

def norm(svg: str) -> str:
    svg = re.sub(r"<title>.*?</title>", "", svg, flags=re.S)
    svg = re.sub(r">\s+<", "><", svg.strip())
    head, rest = svg.split(">", 1)
    for attr in ("width", "height", "style", "color", "xmlns"):
        head = re.sub(rf'\s{attr}="[^"]*"', "", head)
    head = head.replace("<svg", '<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true"', 1)
    return head + ">" + rest

icons = {}
for name, file in HUGE.items():
    icons[name] = norm((repo / "assets/hugeicons" / f"{file}.svg").read_text())
def mono(svg: str) -> str:
    """A monochrome logo paints in the text color, as GPUI tints an SVG icon: black fills (or
    none, which SVG draws black) become currentColor."""
    svg = re.sub(r'fill="(#000000|#000|black)"', 'fill="currentColor"', svg)
    head, rest = svg.split(">", 1)
    if " fill=" not in head:
        head += ' fill="currentColor"'
    return head + ">" + rest

for name, file in LOGOS.items():
    svg = norm((repo / "assets/agent-logos" / f"{file}.svg").read_text())
    icons[name] = svg if file.endswith("-color") else mono(svg)

src = Path(out).read_text() if Path(out).exists() else ""
start, end = "/* BEGIN GENERATED */", "/* END GENERATED */"
gen = start + "\nwindow.ICONS = " + json.dumps(icons, indent=1) + ";\n" + end
if start in src:
    src = src[: src.index(start)] + gen + src[src.index(end) + len(end):]
else:
    src = gen + "\n"
Path(out).write_text(src)
print(f"{len(icons)} icons -> {out}")
