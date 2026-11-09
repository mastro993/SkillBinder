#!/bin/bash
# Render every reference state, light and dark, into ../references/<area>/<NN>-<state>-<theme>.png.
#
# usage: render-all.sh [--check] [area ...]     areas: library scoped-pages picker progress
#   --check  render into a temporary directory instead and compare bytes with the committed
#            PNGs; exit 1 when any differs, is missing, or is not in the list below.
# A state is NN:name, or NN:name:scenario when the scenario has another name ("default" = as
# loaded). The scenarios live in scenarios/<area>.js; the spec files list what each state shows.
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REFS="$HERE/../references"

variant() {
  case $1 in
    library) echo "popover --filters segmented" ;;
    scoped-pages) echo menu ;;
    picker) echo c ;;
    progress) echo inline ;;
    *) echo "render-all: unknown area $1" >&2; exit 2 ;;
  esac
}
states() {
  case $1 in
    library) echo 01:all 02:active 03:inactive 04:updates 05:folder-search 06:no-match \
      07:hover-active 08:hover-inactive 09:hover-place 10:pop-agents-2ws 11:pop-inactive \
      12:pop-docs-site 13:pop-docs-site-on 14:pop-bottom 15:all-agents-off 16:grid 17:grid-pop \
      18:grid-hover-inactive ;;
    scoped-pages) echo 01:ws-list 02:ws-menu-all 03:ws-menu-untick 04:ws-confirm-last \
      05:ws-undo-toast 06:ws-edited-ask 07:ws-empty 08:ws-search 09:agent-claude \
      10:universal-members 11:agent-undo-toast 12:ws-search-none 13:agent-chip-tooltip 14:trash-tooltip ;;
    picker) echo 01:empty 02:selected-3 03:all-off 04:no-agent 05:no-match 06:already-here \
      07:folder-chip 08:docs-site 09:docs-site-carry 10:agent 11:pill-tooltip ;;
    progress) echo 00:idle:default 01:appear-10 02:strip-45 03:adding-50 04:importing-50 \
      05:finishing-100 06:done-2 07:review 08:toast-switched 09:deleting-45 10:toast-deleted ;;
  esac
}

CHECK=0
[[ "${1:-}" != --check ]] || { CHECK=1; shift; }
AREAS=("$@")
[[ ${#AREAS[@]} -gt 0 ]] || AREAS=(library scoped-pages picker progress)
TMP="$(mktemp -d "${TMPDIR:-/tmp}/sbrefs.XXXXXX")"
trap 'rm -rf "$TMP"' EXIT
fail=0
for area in "${AREAS[@]}"; do
  read -r -a args <<<"$(variant "$area")"
  out="$REFS/$area"
  [[ $CHECK == 0 ]] || out="$TMP/$area"
  mkdir -p "$out"
  for st in $(states "$area"); do
    IFS=: read -r n name scenario <<<"$st"
    for theme in light dark; do
      file="$n-$name-$theme.png"
      if ! "$HERE/render-ref.sh" "$area" "${args[0]}" "$theme" "${scenario:-$name}" "$out/$file" \
        "${args[@]:1}" >/dev/null 2>"$TMP/err"; then
        echo "FAIL     $area/$file: $(cat "$TMP/err")"; fail=1; continue
      fi
      if [[ $CHECK == 0 ]]; then echo "rendered $area/$file"
      elif cmp -s "$out/$file" "$REFS/$area/$file"; then echo "same     $area/$file"
      else echo "DIFFERS  $area/$file"; fail=1
      fi
    done
  done
  if [[ $CHECK == 1 ]]; then
    for f in "$REFS/$area"/*.png; do
      [[ -e "$out/$(basename "$f")" ]] || { echo "EXTRA    $area/$(basename "$f") (no state renders it)"; fail=1; }
    done
  fi
done
exit $fail
