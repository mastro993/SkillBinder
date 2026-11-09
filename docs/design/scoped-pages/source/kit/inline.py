"""Inline local stylesheets and scripts into one self-contained HTML file.

usage: python3 -I inline.py <in.html> <out.html>

Replaces <link rel="stylesheet" href="local.css"> with a minified <style> and
<script src="local.js"></script> with an inline <script> (its leading header
comment dropped). Paths resolve relative to the input file. A file kept as a
folder "<name>.parts/" (to stay under the repository's per-file size limit) is
read as its parts joined in numeric order; the input page may be one too.
Remote URLs are an error (the chat frame allows no external URLs). Warns when the
output exceeds the 512,000-character preview cap.
"""
import re
import sys
from pathlib import Path

LIMIT = 512_000
LINK = re.compile(r"<link\b[^>]*\brel=[\"']stylesheet[\"'][^>]*>", re.I)
SCRIPT = re.compile(r"<script\b[^>]*\bsrc=[\"']([^\"']+)[\"'][^>]*>\s*</script>", re.I)
HREF = re.compile(r"\bhref=[\"']([^\"']+)[\"']", re.I)
HEADER = re.compile(r"\A\s*/\*.*?\*/\s*", re.S)  # a linked script's leading API comment


def minify_css(css: str) -> str:
    """Drop comments and collapse whitespace; keeps the payload small for the chat frame."""
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
    css = re.sub(r"\s+", " ", css)
    return re.sub(r"\s*([{};])\s*", r"\1", css).strip()


def read_source(path: Path) -> str:
    """Read a file, or the parts of "<path>.parts/" joined in numeric order."""
    if path.is_file():
        return path.read_text(encoding="utf-8")
    parts = Path(f"{path}.parts")
    if not parts.is_dir():
        sys.exit(f"inline.py: missing file: {path}")
    order = lambda p: [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", p.name)]
    return "".join(p.read_text(encoding="utf-8") for p in sorted(parts.iterdir(), key=order))


def read_local(base: Path, ref: str) -> str:
    if re.match(r"^[a-z]+:|^//", ref, re.I):
        sys.exit(f"inline.py: remote URL not allowed: {ref}")
    return read_source((base / ref).resolve())


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__.strip())
    src, out = Path(sys.argv[1]), Path(sys.argv[2])
    base = src.resolve().parent
    html = read_source(src)

    def css(m: re.Match) -> str:
        href = HREF.search(m.group(0))
        if not href:
            return m.group(0)
        body = minify_css(read_local(base, href.group(1))).replace("</style", "<\\/style")
        return f"<style>/* {href.group(1)} */\n{body}\n</style>"

    def js(m: re.Match) -> str:
        body = HEADER.sub("", read_local(base, m.group(1)), count=1)
        body = body.replace("</script", "<\\/script")
        return f"<script>/* {m.group(1)} */\n{body}\n</script>"

    html = LINK.sub(css, html)
    html = SCRIPT.sub(js, html)
    out.write_text(html, encoding="utf-8")
    size = len(html)
    note = f"  WARNING: over the {LIMIT:,}-char preview cap" if size > LIMIT else ""
    print(f"{out} {size:,} chars{note}")


if __name__ == "__main__":
    main()
