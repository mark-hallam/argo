#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  SCRIPT="$REPO_ROOT/scripts/check-conductor-singleton.sh"
  TMP_ROOT="${BATS_TEST_TMPDIR:-${BATS_TMPDIR:-/tmp}}"
  HEARTBEAT="$TMP_ROOT/conductor.json"
}

write_heartbeat() {
  local last_run="$1"
  local cadence="${2:-1800}"
  cat > "$HEARTBEAT" <<JSON
{
  "agent": "conductor",
  "last_run_utc": "$last_run",
  "expected_cadence_seconds": $cadence
}
JSON
}

@test "missing heartbeat exits zero with missing reason" {
  run "$SCRIPT" --heartbeat "$HEARTBEAT" --candidate-now "2026-05-24T12:00:00Z"

  [ "$status" -eq 0 ]
  [ "$output" = "missing" ]
}

@test "stale heartbeat exits zero with stale reason" {
  write_heartbeat "2026-05-24T10:00:00Z"

  run "$SCRIPT" --heartbeat "$HEARTBEAT" --candidate-now "2026-05-24T12:00:00Z"

  [ "$status" -eq 0 ]
  [ "$output" = "stale" ]
}

@test "live heartbeat exits nonzero with alive reason" {
  write_heartbeat "2026-05-24T11:45:00Z"

  run "$SCRIPT" --heartbeat "$HEARTBEAT" --candidate-now "2026-05-24T12:00:00Z"

  [ "$status" -eq 1 ]
  [ "$output" = "alive" ]
}

@test "heartbeat newer than candidate exits nonzero with newer reason" {
  write_heartbeat "2026-05-24T12:05:00Z"

  run "$SCRIPT" --heartbeat "$HEARTBEAT" --candidate-now "2026-05-24T12:00:00Z"

  [ "$status" -eq 1 ]
  [ "$output" = "newer" ]
}

@test "malformed heartbeat exits nonzero with invalid reason" {
  printf '{not json\n' > "$HEARTBEAT"

  run "$SCRIPT" --heartbeat "$HEARTBEAT" --candidate-now "2026-05-24T12:00:00Z"

  [ "$status" -eq 1 ]
  [ "$output" = "invalid" ]
}

@test "malformed timestamp exits nonzero with invalid reason" {
  write_heartbeat "not-a-timestamp"

  run "$SCRIPT" --heartbeat "$HEARTBEAT" --candidate-now "2026-05-24T12:00:00Z"

  [ "$status" -eq 1 ]
  [ "$output" = "invalid" ]
}
