#!/usr/bin/env bash
set -euo pipefail

"$@" &
application_pid=$!
(
  sleep 45 &
  delay_pid=$!
  trap 'kill "$delay_pid" 2>/dev/null || true' EXIT
  trap 'exit 0' TERM
  wait "$delay_pid"
  if kill -0 "$application_pid" 2>/dev/null; then
    echo 'Native smoke stalled; collecting thread backtraces.' >&2
    sudo timeout 10 gdb --batch --quiet \
      -ex 'set pagination off' -ex 'thread apply all bt 16' \
      -p "$application_pid" || true
    kill "$application_pid" 2>/dev/null || true
  fi
) &
watchdog_pid=$!
trap 'kill "$watchdog_pid" "$application_pid" 2>/dev/null || true' EXIT
status=0
wait "$application_pid" || status=$?
exit "$status"
