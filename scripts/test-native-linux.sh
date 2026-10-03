#!/usr/bin/env bash
# Run inside Xvfb and a D-Bus session; fixtures always use an isolated home.
set -euo pipefail
cd "$(dirname "$0")/.."
evidence="target/native-evidence"
mkdir -p "$evidence"
target/debug/examples/fixture --ready > "$evidence/fixture.log" 2>&1 &
fixture_pid=$!
trap 'kill "$fixture_pid" 2>/dev/null || true' EXIT
window_id=""
for attempt in {1..100}; do
  kill -0 "$fixture_pid"
  window_id=$(xdotool search --onlyvisible --name 'SkillBinder.*isolated fixture' 2>/dev/null | head -1 || true)
  if [ -n "$window_id" ]; then break; fi
  sleep 0.1
done
test -n "$window_id"
# Give the first native frame and the local bootstrap request time to finish.
sleep 2
import -window "$window_id" "$evidence/discovery.png"
# The deterministic fixture geometry makes this a real native navigation smoke test.
xdotool mousemove --window "$window_id" 110 140 click 1
sleep 1
kill -0 "$fixture_pid"
import -window "$window_id" "$evidence/navigation.png"
