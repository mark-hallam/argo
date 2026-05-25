#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  SCRIPT="$REPO_ROOT/scripts/check-unrowed-stream-briefs.sh"
  TMP_ROOT="${BATS_TEST_TMPDIR:-${BATS_TMPDIR:-/tmp}}"
  FIXTURE="$TMP_ROOT/unrowed-streams-$BATS_TEST_NUMBER"
  STREAMS="$FIXTURE/streams"
  LEDGER="$FIXTURE/BUILD_STATUS.md"
  mkdir -p "$STREAMS"
}

teardown() {
  rm -rf "$FIXTURE"
}

write_ledger() {
  cat > "$LEDGER" <<'EOF'
| brief | owner | status | pr | notes |
|---|---|---|---|---|
EOF
  for brief_id in "$@"; do
    printf '| %s | codex | TODO | - | fixture row |\n' "$brief_id" >> "$LEDGER"
  done
}

touch_stream() {
  printf '# %s\n' "$1" > "$STREAMS/$1.md"
}

@test "clean when every stream has a ledger row" {
  touch_stream "codex-clean-brief"
  write_ledger "codex-clean-brief"

  run "$SCRIPT" --streams-dir "$STREAMS" --ledger "$LEDGER"

  [ "$status" -eq 0 ]
  [ "$output" = "unrowed-stream-briefs: ok" ]
}

@test "reports stream ids missing from the ledger" {
  touch_stream "codex-queued-brief"
  touch_stream "codex-unrowed-brief"
  write_ledger "codex-queued-brief"

  run "$SCRIPT" --streams-dir "$STREAMS" --ledger "$LEDGER"

  [ "$status" -eq 1 ]
  [[ "$output" == *"unrowed-stream-briefs:"* ]]
  [[ "$output" == *"codex-unrowed-brief"* ]]
  [[ "$output" != *"codex-queued-brief"* ]]
}

@test "ignores inline allowlisted historical docs" {
  touch_stream "adr-frontend-design-language"
  write_ledger

  run "$SCRIPT" --streams-dir "$STREAMS" --ledger "$LEDGER"

  [ "$status" -eq 0 ]
  [ "$output" = "unrowed-stream-briefs: ok" ]
}
