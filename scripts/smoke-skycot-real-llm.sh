#!/usr/bin/env bash
# SKYCOT real-LLM operator journey test.
#
# Companion to `scripts/smoke-skycot.sh` — same wiring, but drives the
# worker against a REAL provider (Anthropic preferred, OpenAI fallback)
# and asserts the LLM produced coherent, non-mock output. Skips cleanly
# with exit 0 when no provider key is set, so it is safe to wire into
# key-less CI.
#
# Usage:
#   ./scripts/smoke-skycot-real-llm.sh [argo-binary]
#
# Env:
#   ANTHROPIC_API_KEY  prefer Anthropic if set
#   OPENAI_API_KEY     fallback if no Anthropic key
#   ARGO_SMOKE_MODEL   optional override for the model id
#
# Header reports one of three states clearly so operators can see at a
# glance whether real-LLM coverage executed:
#   === RAN (anthropic) ===
#   === RAN (openai)    ===
#   === SKIPPED          ===

set -uo pipefail

ARGO_BIN="${1:-argo}"
SCRIPT_NAME="$(basename "$0")"

if ! command -v "$ARGO_BIN" >/dev/null 2>&1; then
  echo "FAIL: argo binary '$ARGO_BIN' not on PATH"
  echo "      install via the project README; pass the path as arg 1"
  echo "      e.g. $SCRIPT_NAME ~/.local/bin/argo"
  exit 2
fi

# Words we must NOT see in the LLM output — the mock provider in
# `smoke-skycot.sh` produces deterministic stub text containing these.
# Future operators may extend this list as new mock patterns appear.
MOCK_TOKENS=("mock" "placeholder")

# Total wall-clock cap. Real Anthropic / OpenAI runs at --max-steps 5
# for the gtm-engineer vertical land in 30-60s; a 90s cap catches
# stuck inference loops without false-failing the slow tail.
BUDGET_SECONDS=90

# ─── provider selection ───────────────────────────────────────────────
PROVIDER=""
PROVIDER_ID=""
MODEL=""
if [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then
  PROVIDER="anthropic"
  PROVIDER_ID="anthropic"
  MODEL="${ARGO_SMOKE_MODEL:-claude-haiku-4-5}"
elif [[ -n "${OPENAI_API_KEY:-}" ]]; then
  PROVIDER="openai"
  PROVIDER_ID="openai-compatible"
  MODEL="${ARGO_SMOKE_MODEL:-gpt-4o-mini}"
else
  echo
  echo "=== SKIPPED: no provider key set ==="
  echo
  echo "Set ANTHROPIC_API_KEY or OPENAI_API_KEY to exercise the worker"
  echo "against a real provider. Example:"
  echo "  export ANTHROPIC_API_KEY=sk-ant-..."
  echo "  ./scripts/smoke-skycot-real-llm.sh"
  echo
  echo "(This is the safe-no-op path so CI can run this script in"
  echo "key-less environments without failing.)"
  exit 0
fi

# ─── tempdir + helpers (mirror smoke-skycot.sh) ───────────────────────
WORKDIR="$(mktemp -d)"
HOME_DIR="$(mktemp -d)"
trap 'rm -rf "$WORKDIR" "$HOME_DIR"' EXIT

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
VERTICALS_DIR="$SCRIPT_DIR/../verticals"
if [[ ! -d "$VERTICALS_DIR/gtm-engineer" ]]; then
  echo "FAIL: gtm-engineer vertical not found at $VERTICALS_DIR"
  echo "      this script must run from the Argo repo (or have"
  echo "      ARGO_VERTICALS_DIR set to a directory containing"
  echo "      gtm-engineer/)"
  exit 2
fi
export ARGO_HOME="$HOME_DIR"
export ARGO_VERTICALS_DIR="$VERTICALS_DIR"

cd "$WORKDIR"

ok() { echo "  OK: $*"; }
fail() {
  echo "FAIL: $1"
  if [[ -n "${2:-}" ]]; then
    echo "      output:"
    echo "$2" | sed 's/^/        /'
  fi
  exit 1
}
heading() { echo; echo "=== $* ==="; }

# ─── run ──────────────────────────────────────────────────────────────
heading "RAN ($PROVIDER)"
echo "  provider=$PROVIDER_ID model=$MODEL budget=${BUDGET_SECONDS}s"

heading "1. workspace init"
out=$("$ARGO_BIN" workspace init 2>&1) || fail "workspace init" "$out"
ok "workspace initialised under $WORKDIR"

heading "2. vertical loaded"
out=$("$ARGO_BIN" vertical list 2>&1) || fail "vertical list" "$out"
echo "$out" | grep -q "gtm-engineer" || fail "gtm-engineer vertical missing" "$out"
ok "gtm-engineer loaded"

heading "3. queue real-LLM job"
JOB_PROMPT="research the current price of Bauer Vapor X5 inline skates and draft a marketplace listing description"
add_out=$("$ARGO_BIN" work add \
  --vertical gtm-engineer \
  --prompt "$JOB_PROMPT" \
  --json 2>&1) || fail "work add" "$add_out"
ok "job queued"

heading "4. run worker once (real LLM, ${BUDGET_SECONDS}s budget)"
start_ts=$(date +%s)
serve_out=$("$ARGO_BIN" work serve \
  --vertical gtm-engineer \
  --provider "$PROVIDER_ID" \
  --model "$MODEL" \
  --once \
  --max-steps 5 \
  --json 2>&1)
serve_exit=$?
end_ts=$(date +%s)
elapsed=$((end_ts - start_ts))
if [[ $serve_exit -ne 0 ]]; then
  fail "work serve (provider=$PROVIDER_ID model=$MODEL)" "$serve_out"
fi
if [[ $elapsed -gt $BUDGET_SECONDS ]]; then
  fail "BUDGET EXCEEDED: real-LLM run took ${elapsed} seconds (cap ${BUDGET_SECONDS})" "$serve_out"
fi
job_id=$(echo "$serve_out" | grep -oE '"job_id":"[^"]+"' | head -1 | sed 's/.*"\(.*\)"/\1/')
[[ -n "$job_id" ]] || fail "job_id missing from serve output" "$serve_out"
ok "worker ran job $job_id in ${elapsed}s"

heading "5. coherence assertion (non-empty, ≥40 chars, no mock tokens)"
bundle="$WORKDIR/.skycot/outputs/$job_id"
[[ -d "$bundle" ]] || fail "bundle dir missing at $bundle" "$(ls -la "$WORKDIR/.skycot/outputs" 2>&1)"
# The assistant_output.md text is the canonical LLM-generated content
# for the run. action-payload `title` strings are a secondary check.
assistant_path="$bundle/assistant_output.md"
[[ -f "$assistant_path" ]] || fail "assistant_output.md missing in $bundle" "$(ls -la "$bundle" 2>&1)"
assistant_text="$(cat "$assistant_path")"
assistant_len=${#assistant_text}
[[ $assistant_len -ge 40 ]] || fail \
  "assistant_output.md too short (${assistant_len} chars; need ≥40)" \
  "$assistant_text"
lower_text="$(echo "$assistant_text" | tr '[:upper:]' '[:lower:]')"
for token in "${MOCK_TOKENS[@]}"; do
  if echo "$lower_text" | grep -q "$token"; then
    fail "assistant_output.md contains mock-token '$token' — provider may have fallen back to mock" \
      "$assistant_text"
  fi
done
ok "assistant_output.md is ${assistant_len} chars and clean of mock tokens"

heading "6. artifact bundle has non-trivial content (≥1 file >100 bytes)"
big_files=0
while IFS= read -r f; do
  bytes=$(wc -c < "$f" | tr -d ' ')
  if [[ "$bytes" -gt 100 ]]; then
    big_files=$((big_files + 1))
  fi
done < <(find "$bundle" -maxdepth 1 -type f)
[[ "$big_files" -ge 1 ]] || fail \
  "bundle has no files >100 bytes — LLM may have returned empty content" \
  "$(ls -la "$bundle")"
ok "bundle has $big_files file(s) >100 bytes"

heading "7. action ledger landed"
list_out=$("$ARGO_BIN" worker actions list \
  --state all \
  --vertical gtm-engineer \
  --json 2>&1) || fail "actions list" "$list_out"
row_count=$(echo "$list_out" | grep -c '"action_id"')
[[ "$row_count" -ge 1 ]] || fail "expected ≥1 ledger row, got $row_count" "$list_out"
ok "action ledger has $row_count row(s)"

heading "8. cost report (forward-compatible)"
# `work serve --json` does not yet emit `tokens_input` / `tokens_output`
# / `cost_usd`. Wiring those through is a follow-up brief; until then
# this is a forward-compatible probe — if the fields appear later,
# this script picks them up automatically.
tokens_in=$(echo "$serve_out" | grep -oE '"tokens_input"[[:space:]]*:[[:space:]]*[0-9]+' \
  | head -1 | grep -oE '[0-9]+' || true)
tokens_out=$(echo "$serve_out" | grep -oE '"tokens_output"[[:space:]]*:[[:space:]]*[0-9]+' \
  | head -1 | grep -oE '[0-9]+' || true)
cost_usd=$(echo "$serve_out" | grep -oE '"cost_usd"[[:space:]]*:[[:space:]]*[0-9.]+' \
  | head -1 | grep -oE '[0-9.]+' || true)
if [[ -n "$cost_usd" ]]; then
  echo "=== COST: \$${cost_usd} (input=${tokens_in:-?} output=${tokens_out:-?}) ==="
else
  echo "  (cost reporting not yet available in \`work serve --json\` —"
  echo "   follow-up brief to surface tokens_input/tokens_output/cost_usd)"
fi

echo
echo "✓ SKYCOT real-LLM smoke PASSED ($PROVIDER, ${elapsed}s)"
echo "  workspace: $WORKDIR"
echo "  argo home: $HOME_DIR"
echo "  job_id:    $job_id"
echo "  model:     $MODEL"
echo
echo "This proves the agent actually thought — output is coherent and"
echo "non-mock. Next step: read docs/skycot/getting_started.md to drive"
echo "a real worker job in your own workspace."
