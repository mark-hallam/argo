#!/usr/bin/env bash
# Mock `claude` binary used by `claude_code_cli` driver tests.
#
# Behaviour is controlled by env vars so one fixture exercises all paths:
#
#   MOCK_CLAUDE_MODE=ok          → print canonical-shape JSON, exit 0 (default)
#   MOCK_CLAUDE_MODE=brief_shape → print the brief's reference shape, exit 0
#   MOCK_CLAUDE_MODE=error       → print stderr + exit 1
#   MOCK_CLAUDE_MODE=is_error    → print JSON with is_error=true, exit 0
#   MOCK_CLAUDE_MODE=garbage     → print non-JSON to stdout, exit 0
#   MOCK_CLAUDE_MODE=sleep       → sleep MOCK_CLAUDE_SLEEP_SECS, then exit 0
#
# The driver test sets ARGO_CLAUDE_CODE_BINARY=<absolute path to this file>.
#
# Set MOCK_CLAUDE_ARGV_OUT=<path> to capture the spawned argv (one arg
# per line) so tests can assert the driver passed `--model <id>`.

set -u
mode="${MOCK_CLAUDE_MODE:-ok}"

# When MOCK_CLAUDE_ARGV_OUT is set, write the full argv (one arg per
# line) to that path so the driver tests can assert which flags the
# driver passed (e.g. `--model claude-sonnet-4-6`).
if [ -n "${MOCK_CLAUDE_ARGV_OUT:-}" ]; then
  : >"$MOCK_CLAUDE_ARGV_OUT"
  for arg in "$@"; do
    printf '%s\n' "$arg" >>"$MOCK_CLAUDE_ARGV_OUT"
  done
fi

case "$mode" in
  ok)
    printf '{"type":"result","subtype":"success","is_error":false,"result":"hello from mock","stop_reason":"end_turn","usage":{"input_tokens":10,"output_tokens":3},"modelUsage":{"claude-sonnet-4-6":{"inputTokens":10,"outputTokens":3}}}'
    exit 0
    ;;
  brief_shape)
    printf '{"assistant_text":"hello from mock","model":"claude-sonnet-4-6","usage":{"input_tokens":10,"output_tokens":3}}'
    exit 0
    ;;
  error)
    printf 'mock claude: simulated failure\n' >&2
    exit 1
    ;;
  is_error)
    printf '{"is_error":true,"result":"upstream refused"}'
    exit 0
    ;;
  garbage)
    printf 'not json at all'
    exit 0
    ;;
  sleep)
    sleep "${MOCK_CLAUDE_SLEEP_SECS:-10}"
    printf '{"result":"woken","usage":{"input_tokens":1,"output_tokens":1}}'
    exit 0
    ;;
  *)
    printf 'mock claude: unknown MOCK_CLAUDE_MODE=%s\n' "$mode" >&2
    exit 2
    ;;
esac
