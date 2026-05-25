#!/usr/bin/env bats

setup() {
  export SCRIPT="$BATS_TEST_DIRNAME/../open-pr-overlap-triage.sh"
  export FIXTURE="$BATS_TEST_TMPDIR/fixture"
  export MOCK_BIN="$BATS_TEST_TMPDIR/bin"
  mkdir -p "$FIXTURE" "$MOCK_BIN"
  cat >"$MOCK_BIN/gh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

if [[ "$1 $2" == "pr list" ]]; then
  cat "$OPEN_PR_OVERLAP_FIXTURE/pr-list.json"
  exit 0
fi

if [[ "$1 $2" == "pr view" ]]; then
  cat "$OPEN_PR_OVERLAP_FIXTURE/pr-$3.json"
  exit 0
fi

echo "unexpected gh args: $*" >&2
exit 99
SH
  chmod +x "$MOCK_BIN/gh"
}

write_pr_list() {
  cat >"$FIXTURE/pr-list.json"
}

write_files() {
  local number="$1"
  cat >"$FIXTURE/pr-$number.json"
}

run_script() {
  run env \
    PATH="$MOCK_BIN:$PATH" \
    OPEN_PR_OVERLAP_FIXTURE="$FIXTURE" \
    "$SCRIPT"
}

@test "no code overlap exits zero and ignores docs-only overlap" {
  write_pr_list <<'JSON'
[
  {"number": 1, "title": "docs a", "headRefName": "codex-a"},
  {"number": 2, "title": "docs b", "headRefName": "codex-b"}
]
JSON
  write_files 1 <<'JSON'
{"files":[{"path":"docs/build-plan/BUILD_STATUS.md"},{"path":"scripts/a.sh"}]}
JSON
  write_files 2 <<'JSON'
{"files":[{"path":"docs/build-plan/BUILD_STATUS.md"},{"path":"scripts/b.sh"}]}
JSON

  run_script

  [ "$status" -eq 0 ]
  [[ "$output" == *"OK: no code file overlaps across 2 open PR(s)."* ]]
  [[ "$output" == *"Ignored 1 docs-only overlap(s)."* ]]
  [[ "$output" != *"OVERLAP docs/build-plan/BUILD_STATUS.md"* ]]
}

@test "single shared code path lists involved PR numbers" {
  write_pr_list <<'JSON'
[
  {"number": 7, "title": "first", "headRefName": "codex-first"},
  {"number": 9, "title": "second", "headRefName": "codex-second"}
]
JSON
  write_files 7 <<'JSON'
{"files":[{"path":"crates/argo-cli/src/handlers/gateway/mod.rs"}]}
JSON
  write_files 9 <<'JSON'
{"files":[{"path":"crates/argo-cli/src/handlers/gateway/mod.rs"},{"path":"scripts/other.sh"}]}
JSON

  run_script

  [ "$status" -eq 0 ]
  [[ "$output" == *"Found 1 overlapping code path(s) across open PRs:"* ]]
  [[ "$output" == *"OVERLAP crates/argo-cli/src/handlers/gateway/mod.rs: #7, #9"* ]]
  [[ "$output" == *"RECOMMENDATION crates/argo-cli/src/handlers/gateway/mod.rs:"* ]]
}

@test "multiple shared code paths are reported as separate groups" {
  write_pr_list <<'JSON'
[
  {"number": 1, "title": "alpha", "headRefName": "codex-alpha"},
  {"number": 2, "title": "beta", "headRefName": "codex-beta"},
  {"number": 3, "title": "gamma", "headRefName": "codex-gamma"}
]
JSON
  write_files 1 <<'JSON'
{"files":[{"path":"crates/a/src/lib.rs"},{"path":"scripts/triage.sh"},{"path":"docs/shared.md"}]}
JSON
  write_files 2 <<'JSON'
{"files":[{"path":"crates/a/src/lib.rs"},{"path":"docs/shared.md"}]}
JSON
  write_files 3 <<'JSON'
{"files":[{"path":"crates/a/src/lib.rs"},{"path":"scripts/triage.sh"}]}
JSON

  run_script

  [ "$status" -eq 0 ]
  [[ "$output" == *"Found 2 overlapping code path(s) across open PRs:"* ]]
  [[ "$output" == *"OVERLAP crates/a/src/lib.rs: #1, #2, #3"* ]]
  [[ "$output" == *"OVERLAP scripts/triage.sh: #1, #3"* ]]
  [[ "$output" != *"OVERLAP docs/shared.md"* ]]
}
