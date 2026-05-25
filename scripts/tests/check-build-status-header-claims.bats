#!/usr/bin/env bats

setup() {
    tmp="$(mktemp -d)"
    ledger="$tmp/BUILD_STATUS.md"
    script="$BATS_TEST_DIRNAME/../check-build-status-header-claims.sh"
}

teardown() {
    rm -rf "$tmp"
}

write_ledger() {
    cat > "$ledger"
}

@test "clean header reference passes" {
    write_ledger <<'EOF'
# Build status

> Track B shipped: `foo-brief` is MERGED.

| Brief | Owner | Status | PR | Notes |
| foo-brief | codex | MERGED | #1 | shipped |
EOF

    run bash "$script" --ledger "$ledger"

    [ "$status" -eq 0 ]
    [[ "$output" == *"ok build status header claims"* ]]
}

@test "pending ledger insertion fails when row exists" {
    write_ledger <<'EOF'
# Build status

> `foo-brief` is authored and pending ledger insertion this tick.

| Brief | Owner | Status | PR | Notes |
| foo-brief | codex | TODO | — | queued |
EOF

    run bash "$script" --ledger "$ledger"

    [ "$status" -eq 1 ]
    [[ "$output" == *"foo-brief is pending ledger insertion"* ]]
    [[ "$output" == *"status TODO"* ]]
}

@test "stale status keyword mismatch fails" {
    write_ledger <<'EOF'
# Build status

> TODO row `foo-brief` remains queued for the next worker.

| Brief | Owner | Status | PR | Notes |
| foo-brief | codex | MERGED | #2 | shipped |
EOF

    run bash "$script" --ledger "$ledger"

    [ "$status" -eq 1 ]
    [[ "$output" == *"header says foo-brief is TODO"* ]]
    [[ "$output" == *"ledger status is MERGED"* ]]
}

@test "table body text is ignored" {
    write_ledger <<'EOF'
# Build status

> `foo-brief` is MERGED.

| Brief | Owner | Status | PR | Notes |
| foo-brief | codex | MERGED | #2 | stale note says pending ledger insertion |
EOF

    run bash "$script" --ledger "$ledger"

    [ "$status" -eq 0 ]
}

@test "wrapped brief id in header is normalized" {
    write_ledger <<'EOF'
# Build status

> (`mission-real-llm-operator-
> journey-test`) is still described as a TODO row.

| Brief | Owner | Status | PR | Notes |
| mission-real-llm-operator-journey-test | codex | MERGED | #3 | shipped |
EOF

    run bash "$script" --ledger "$ledger"

    [ "$status" -eq 1 ]
    [[ "$output" == *"mission-real-llm-operator-journey-test is TODO"* ]]
}
