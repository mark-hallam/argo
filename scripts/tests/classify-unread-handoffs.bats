#!/usr/bin/env bats

setup() {
  export SCRIPT="$BATS_TEST_DIRNAME/../classify-unread-handoffs.sh"
  export FIXTURE="$BATS_TEST_TMPDIR/repo"
  mkdir -p \
    "$FIXTURE/docs/build-plan" \
    "$FIXTURE/docs/skycot/agents/heartbeats"
  cat >"$FIXTURE/docs/skycot/agents/heartbeats/conductor.json" <<'JSON'
{"last_run_utc":"2026-05-24T10:00:00Z"}
JSON
  cat >"$FIXTURE/docs/build-plan/BUILD_STATUS.md" <<'EOF'
| Brief | Owner | Status | PR | Last update |
|---|---|---|---|---|
| landed-brief | codex | MERGED | #42 | done |
| review-brief | codex | IN_REVIEW | #43 | in review |
| todo-brief | codex | TODO | — | not yet done |
EOF
}

run_classifier() {
  run env ARGO_REPO_ROOT="$FIXTURE" "$SCRIPT"
}

@test "already-landed handoff is recognized from PR and ledger status" {
  cat >"$FIXTURE/docs/skycot/agents/SHARED.md" <<'EOF'
2026-05-24T09:59:59Z [evaluator] old #42 should be ignored
2026-05-24T10:01:00Z [evaluator] PR #42 closed the issue already
2026-05-24T10:02:00Z [auditor] brief review-brief has coverage
EOF

  run_classifier

  [ "$status" -eq 0 ]
  [[ "$output" == *$'already-landed\t2026-05-24T10:01:00Z\t[evaluator]\tPR #42 is MERGED'* ]]
  [[ "$output" == *$'already-landed\t2026-05-24T10:02:00Z\t[auditor]\treview-brief is IN_REVIEW'* ]]
  [[ "$output" != *"old #42 should be ignored"* ]]
}

@test "queue-needed handoff is recognized for missing brief ids" {
  cat >"$FIXTURE/docs/skycot/agents/SHARED.md" <<'EOF'
2026-05-24T10:03:00Z [evaluator] please author brief missing-brief-row for the surfaced gap
EOF

  run_classifier

  [ "$status" -eq 0 ]
  [[ "$output" == *$'queue-needed\t2026-05-24T10:03:00Z\t[evaluator]\tno BUILD_STATUS.md row for missing-brief-row'* ]]
}

@test "operator-only handoff is recognized for outside-repo action" {
  cat >"$FIXTURE/docs/skycot/agents/SHARED.md" <<'EOF'
2026-05-24T10:04:00Z [auditor] operator-only: grant macOS permissions in System Settings
EOF

  run_classifier

  [ "$status" -eq 0 ]
  [[ "$output" == *$'operator-only\t2026-05-24T10:04:00Z\t[auditor]\thandoff points at operator/outside-repo work'* ]]
}

@test "informational handoff is the fallback classification" {
  cat >"$FIXTURE/docs/skycot/agents/SHARED.md" <<'EOF'
2026-05-24T10:05:00Z [marketing] no action needed; noting customer wording preference
EOF

  run_classifier

  [ "$status" -eq 0 ]
  [[ "$output" == *$'informational\t2026-05-24T10:05:00Z\t[marketing]\tno direct ledger action inferred'* ]]
  [[ "$output" == *"summary: informational=1"* ]]
}
