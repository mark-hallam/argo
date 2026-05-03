# Live-mode helpers for gtm_engineer_learning_loop.sh.
# Sourced (not exec'd) — must not mutate caller-controlled traps,
# must not consume positional args. Relies on $ARGO_BIN, $REPO_ROOT,
# $banner / $fail functions being in scope.

# Pick a free TCP port in [7000, 7999) using bash's /dev/tcp probe
# (no python / netcat dep). Returns the port via stdout, or non-zero
# if 50 attempts all collided (vanishingly unlikely).
live_pick_free_port() {
  local port i
  for ((i=0; i<50; i++)); do
    port=$((7000 + RANDOM % 1000))
    # `bash`'s /dev/tcp connect succeeds when something is listening.
    # Failure (no listener) is what we want — the port is free.
    if ! (exec 3<>/dev/tcp/127.0.0.1/"$port") 2>/dev/null; then
      echo "$port"
      return 0
    fi
    exec 3<&- 2>/dev/null || true
  done
  return 1
}

# Block until the gateway answers GET /health, up to ~10s.
live_wait_for_gateway() {
  local port="$1" i
  for ((i=0; i<100; i++)); do
    if curl -fsS "http://127.0.0.1:${port}/health" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.1
  done
  return 1
}

# Start the gateway in REPO_ROOT (so manifests/providers/anthropic.toml
# resolves from cwd) and export $LIVE_GATEWAY_PORT + $LIVE_GATEWAY_PID
# for the caller. --max-requests is a safety net so the gateway
# auto-stops after the demo's traffic.
live_start_gateway() {
  local port
  port=$(live_pick_free_port) \
    || fail "could not find a free TCP port for the gateway"
  # shellcheck disable=SC2034  # consumed by stop_gateway via env
  LIVE_GATEWAY_PORT="$port"
  ( cd "$REPO_ROOT" \
    && "$ARGO_BIN" gateway start --port "$port" --max-requests 50 \
       >/dev/null 2>&1 ) &
  LIVE_GATEWAY_PID=$!
  if ! live_wait_for_gateway "$port"; then
    kill "$LIVE_GATEWAY_PID" 2>/dev/null || true
    fail "gateway did not become healthy on port $port within 10s"
  fi
  echo "  gateway up on port $port (pid $LIVE_GATEWAY_PID)"
}

live_stop_gateway() {
  if [ -n "${LIVE_GATEWAY_PID:-}" ]; then
    kill "$LIVE_GATEWAY_PID" 2>/dev/null || true
    wait "$LIVE_GATEWAY_PID" 2>/dev/null || true
    echo "  gateway pid $LIVE_GATEWAY_PID stopped"
  fi
}

# Drive a single user turn. Args: <session_id> <user_text>.
# Echoes the assistant text on stdout. Fails loud on HTTP/parse error.
# Uses jq to safely encode the user text into a JSON string (handles
# embedded quotes / newlines from the prompt files).
live_turn() {
  local sid="$1" user="$2" body resp text
  body=$(jq -n --arg u "$user" '{user: $u}')
  resp=$(curl -fsS -X POST \
    -H 'Content-Type: application/json' \
    -d "$body" \
    "http://127.0.0.1:${LIVE_GATEWAY_PORT}/sessions/${sid}/runs") \
    || fail "gateway POST /sessions/${sid}/runs failed"
  text=$(printf '%s' "$resp" | jq -r '.text // empty')
  if [ -z "$text" ]; then
    fail "gateway returned no .text field; raw response: $resp"
  fi
  printf '%s' "$text"
}

# Slurp a prompt-template file from demo/data/live/ and substitute
# `{{KEY}}` markers with values from a parallel array of `KEY=value`.
# Args: <template path> [KEY=value ...].
live_render_prompt() {
  local tmpl="$1"; shift
  local out
  out=$(cat "$tmpl") || fail "missing prompt template: $tmpl"
  while [ $# -gt 0 ]; do
    local kv="$1"; shift
    local k="${kv%%=*}" v="${kv#*=}"
    # Python is the most reliable string substitution available
    # cross-platform; sed gets tangled by JSON's `/` and `&`. Falls
    # back to a perl one-liner if python3 is missing.
    if command -v python3 >/dev/null 2>&1; then
      out=$(printf '%s' "$out" | python3 -c "
import sys
print(sys.stdin.read().replace('{{${k}}}', sys.argv[1]), end='')
" "$v")
    elif command -v perl >/dev/null 2>&1; then
      out=$(printf '%s' "$out" \
        | perl -e '$k=shift; $v=shift; local $/; $_=<STDIN>; s/\Q{{$k}}\E/$v/g; print' \
            "$k" "$v")
    else
      fail "live mode needs python3 or perl on PATH for prompt rendering"
    fi
  done
  printf '%s' "$out"
}

# Extract the first JSON object from an LLM response. Strips
# ``` fences if present. Used to defend against the model wrapping
# its output even though the prompt forbids it.
live_extract_json() {
  local raw="$1"
  # Strip code fences (``` or ```json) if present.
  raw=$(printf '%s' "$raw" | sed -E 's/^```[a-zA-Z]*[[:space:]]*//; s/```[[:space:]]*$//')
  # Trim leading non-`{` and trailing non-`}` (defensive).
  printf '%s' "$raw" | python3 -c "
import sys, re
s = sys.stdin.read()
m = re.search(r'\{.*\}', s, re.DOTALL)
print(m.group(0) if m else '', end='')
" 2>/dev/null
}

# Validate that the parsed candidate JSON has every required field
# as a non-empty string. Args: <json>. Fails loud with the offending
# field name on first miss; returns 0 when all pass.
live_validate_candidate() {
  local json="$1" field val
  for field in schema_key skill_name problem reusable_pattern evidence expected_reuse risk; do
    val=$(printf '%s' "$json" | jq -r --arg f "$field" '.[$f] // empty')
    [ -n "$val" ] || fail "live candidate JSON missing required field: $field; got:\n$json"
  done
  local schema_key
  schema_key=$(printf '%s' "$json" | jq -r '.schema_key')
  [ "$schema_key" = "learned_skill_candidates" ] \
    || fail "live candidate schema_key=\"$schema_key\", expected \"learned_skill_candidates\""
}
