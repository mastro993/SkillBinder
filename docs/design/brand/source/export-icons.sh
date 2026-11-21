#!/bin/sh
# Renders the app icon from export.html into assets/: icon.icns (macOS), icon.ico (Windows), and
# icon.png (Linux, 256 px). Needs macOS (sips, iconutil) and Google Chrome; set CHROME to another
# Chromium binary. Run it after changing the icon in icons.js and commit the three files.
set -eu
CHROME=${CHROME:-"/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"}
here=$(cd "$(dirname "$0")" && pwd)
assets=$(cd "$here/../../../../assets" && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# render <kind> <weight> <out.png>: one 1024 px capture on a transparent background.
render() {
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor=1 \
    --default-background-color=00000000 --allow-file-access-from-files --window-size=1024,1024 \
    --screenshot="$3" "file://$here/export.html?kind=$1&weight=$2" >/dev/null 2>&1
}
# scale <in.png> <px> <out.png>
scale() { sips -z "$2" "$2" "$1" --out "$3" >/dev/null; }

render mac 1 "$work/mac.png"
render mac 1.6 "$work/mac-small.png"
render bleed 1 "$work/bleed.png"
render bleed 1.6 "$work/bleed-small.png"

# The rune's stroke is thickened wherever the icon is 32 px or smaller.
mkdir "$work/icon.iconset"
for px in 16 32 64 128 256 512 1024; do
  if [ "$px" -le 32 ]; then source="$work/mac-small.png"; else source="$work/mac.png"; fi
  scale "$source" "$px" "$work/mac-$px.png"
done
for size in 16 32 128 256 512; do
  cp "$work/mac-$size.png" "$work/icon.iconset/icon_${size}x${size}.png"
  cp "$work/mac-$((size * 2)).png" "$work/icon.iconset/icon_${size}x${size}@2x.png"
done
iconutil -c icns -o "$assets/icon.icns" "$work/icon.iconset"
cp "$work/mac-256.png" "$assets/icon.png"

for px in 16 24 32 48 64 256; do
  if [ "$px" -le 32 ]; then source="$work/bleed-small.png"; else source="$work/bleed.png"; fi
  scale "$source" "$px" "$work/ico-$px.png"
done
# An ICO file whose entries are PNG images, as Windows Vista and later read them.
python3 -I - "$assets/icon.ico" "$work"/ico-16.png "$work"/ico-24.png "$work"/ico-32.png \
  "$work"/ico-48.png "$work"/ico-64.png "$work"/ico-256.png <<'EOF'
import struct, sys
out, paths = sys.argv[1], sys.argv[2:]
images = [open(path, "rb").read() for path in paths]
sizes = [int.from_bytes(image[16:20], "big") for image in images]
offset = 6 + 16 * len(images)
header = struct.pack("<HHH", 0, 1, len(images))
entries = b""
for size, image in zip(sizes, images):
    side = 0 if size >= 256 else size
    entries += struct.pack("<BBBBHHII", side, side, 0, 0, 1, 32, len(image), offset)
    offset += len(image)
with open(out, "wb") as ico:
    ico.write(header + entries + b"".join(images))
EOF
echo "Wrote $assets/icon.icns, $assets/icon.ico, and $assets/icon.png"
