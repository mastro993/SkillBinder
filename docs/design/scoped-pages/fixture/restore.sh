#!/bin/bash
# restore.sh <fixture dir> [problems]: replace <fixture dir>/root with a fresh copy of
# <fixture dir>/seed at the SAME absolute path (the database, records and source cache hold
# absolute paths under root). With "problems", also make the EDITED and FOREIGN copies
# (make-fixture.sh problems). Quit the app first.
set -euo pipefail
fx=$(cd "${1:?usage: restore.sh <fixture dir> [problems]}" && pwd)
[ -d "$fx/seed" ] || { echo "no seed in $fx: build it first (README.md)" >&2; exit 1; }
# Refuse while an app runs on this root. Compare resolved paths: the app may have been launched
# through another spelling of a symlinked path, such as macOS's /var/folders for
# /private/var/folders. A --data-dir holding a space is not recognized.
data=$(cd "$fx" && pwd -P)/root/data
while IFS= read -r cmd; do
  dir=${cmd#*--data-dir }
  dir=${dir%% *}
  if [ -d "$dir" ] && [ "$(cd "$dir" && pwd -P)" = "$data" ]; then
    echo "quit the app running on $fx/root first" >&2; exit 1
  fi
done < <(ps -axo command= | grep -E 'skillbinder .*--data-dir ' || true)
rm -rf "$fx/root"
cp -Rp "$fx/seed" "$fx/root"
[ "${2:-}" != problems ] || "$(dirname "$0")/make-fixture.sh" "$fx/root" problems
echo "restored $fx/root"
