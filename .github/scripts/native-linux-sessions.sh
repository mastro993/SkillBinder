#!/usr/bin/env bash
set -euo pipefail

unset WAYLAND_DISPLAY
timeout 70 bash .github/scripts/native-smoke-linux.sh \
  target/debug/skillbinder --smoke-test --data-dir "$RUNNER_TEMP/native-x11"

weston --backend=x11 --renderer=pixman --no-config --width=1280 --height=800 \
  --socket=wayland-skillbinder \
  --idle-time=0 --log="$RUNNER_TEMP/weston.log" &
weston_pid=$!
cleanup() {
  status=$?
  kill "$weston_pid" 2>/dev/null || true
  wait "$weston_pid" 2>/dev/null || true
  if [ "$status" -ne 0 ]; then
    cat "$RUNNER_TEMP/weston.log"
  fi
  exit "$status"
}
trap cleanup EXIT
for attempt in $(seq 1 50); do
  test -S "$XDG_RUNTIME_DIR/wayland-skillbinder" && break
  kill -0 "$weston_pid"
  sleep 0.1
done
test -S "$XDG_RUNTIME_DIR/wayland-skillbinder"
env -u DISPLAY WAYLAND_DISPLAY=wayland-skillbinder \
  timeout 70 bash .github/scripts/native-smoke-linux.sh \
  target/debug/skillbinder --smoke-test --data-dir "$RUNNER_TEMP/native-wayland"
