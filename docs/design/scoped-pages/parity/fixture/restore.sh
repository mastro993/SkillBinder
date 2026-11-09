#!/bin/bash
# restore.sh <fixture dir> [problems]: replace <fixture dir>/root with a fresh copy of
# <fixture dir>/seed at the SAME absolute path (the database, records and source cache hold
# absolute paths under root). With "problems", also make the EDITED and FOREIGN copies
# (make-fixture.sh problems). Quit the app first.
set -euo pipefail
fx=$(cd "${1:?usage: restore.sh <fixture dir> [problems]}" && pwd -P)
[ -d "$fx/seed" ] || { echo "no seed in $fx: build it first (README.md)" >&2; exit 1; }
if pgrep -f "skillbinder --mcp --data-dir $fx/root/data" >/dev/null; then
  echo "quit the app running on $fx/root first" >&2; exit 1
fi
rm -rf "$fx/root"
cp -Rp "$fx/seed" "$fx/root"
[ "${2:-}" != problems ] || "$(dirname "$0")/make-fixture.sh" "$fx/root" problems
echo "restored $fx/root"
