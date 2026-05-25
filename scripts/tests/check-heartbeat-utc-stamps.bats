#!/usr/bin/env bats

setup() {
    repo="$(mktemp -d)"
    script="$BATS_TEST_DIRNAME/../check-heartbeat-utc-stamps.sh"
    mkdir -p "$repo/docs/skycot/agents/heartbeats"
}

teardown() {
    rm -rf "$repo"
}

write_shared() {
    local line="$1"
    cat > "$repo/docs/skycot/agents/SHARED.md" <<EOF
# Shared cross-agent ledger

---
$line
EOF
}

write_heartbeat() {
    local stamp="$1"
    cat > "$repo/docs/skycot/agents/heartbeats/auditor.json" <<EOF
{
  "agent": "auditor",
  "last_run_utc": "$stamp",
  "expected_cadence_seconds": 28800
}
EOF
}

@test "valid UTC heartbeat and shared entry pass" {
    write_heartbeat "2026-05-24T10:00:00Z"
    write_shared "2026-05-24T10:00:01Z [auditor] valid UTC entry"

    run bash "$script" --root "$repo" --now "2026-05-24T10:02:00Z"

    [ "$status" -eq 0 ]
    [[ "$output" == *"ok heartbeat UTC stamps"* ]]
}

@test "malformed heartbeat timestamp reports exact file and line" {
    write_heartbeat "2026-05-24T25:00:00Z"
    write_shared "2026-05-24T10:00:01Z [auditor] valid UTC entry"

    run bash "$script" --root "$repo" --now "2026-05-24T10:02:00Z"

    [ "$status" -eq 1 ]
    [[ "$output" == *"docs/skycot/agents/heartbeats/auditor.json:3"* ]]
    [[ "$output" == *"not a valid UTC timestamp"* ]]
}

@test "local-offset shared timestamp is rejected" {
    write_heartbeat "2026-05-24T10:00:00Z"
    write_shared "2026-05-24T20:00:01+10:00 [conductor] local offset"

    run bash "$script" --root "$repo" --now "2026-05-24T10:02:00Z"

    [ "$status" -eq 1 ]
    [[ "$output" == *"docs/skycot/agents/SHARED.md:4"* ]]
    [[ "$output" == *"strict UTC YYYY-MM-DDTHH:MM:SSZ"* ]]
}

@test "fake-Z future heartbeat is rejected as probable local time" {
    write_heartbeat "2026-05-24T20:00:00Z"
    write_shared "2026-05-24T10:00:01Z [auditor] valid UTC entry"

    run bash "$script" --root "$repo" --now "2026-05-24T10:02:00Z"

    [ "$status" -eq 1 ]
    [[ "$output" == *"docs/skycot/agents/heartbeats/auditor.json:3"* ]]
    [[ "$output" == *"future-dated"* ]]
    [[ "$output" == *"possible local time stamped with Z"* ]]
}

@test "invalid heartbeat JSON reports exact file and line" {
    cat > "$repo/docs/skycot/agents/heartbeats/auditor.json" <<'EOF'
{
  "agent": "auditor",
  "last_run_utc": "2026-05-24T10:00:00Z",
EOF
    write_shared "2026-05-24T10:00:01Z [auditor] valid UTC entry"

    run bash "$script" --root "$repo" --now "2026-05-24T10:02:00Z"

    [ "$status" -eq 1 ]
    [[ "$output" == *"docs/skycot/agents/heartbeats/auditor.json:4"* ]]
    [[ "$output" == *"invalid json"* ]]
}
